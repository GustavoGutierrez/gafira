//! The frame surface: the format's own storage, one pixel per page pixel
//! (ARQ-117, O11, O14).
//!
//! One surface pixel per page pixel (D4), covering the page rectangle that
//! starts at the target's `origin`. The target's format decides the storage:
//! an `RgbaF32` surface holds linear-light premultiplied floats, and an
//! `Rgba8` surface holds the premultiplied sRGB bytes [`encode_tile`] produced
//! for the cache. The write is a copy of that encoded tile and the read is a
//! copy of the surface, so the sRGB curve runs **once**, where it already ran
//! (`spike-frame-cost.md`, O14). There is no second accumulation path.

use gafira_color::{LinearRgba, Rgba};
use gafira_geom::IRect;
use gafira_ports::common::Bytes;
use gafira_ports::render::{RenderError, TargetDesc};
use gafira_raster::{
    EffectError, ImageBuf, PixelBuf, PixelFormat, premultiply_u8, unpremultiply_u8,
};

/// The largest surface side this delivery allocates.
///
/// FD-55 allows 16 384 px per side, and the page-sized surface of D4 cannot
/// hold it: 4 GiB in `RgbaF32`, 2.9× the 1.5 GB cap of `D-03` (XP4). 8 192 px
/// keeps every surface under the cap; the tiled or streaming path that FD-55
/// also names is what would lift it, and it is not part of this delivery.
pub(crate) const MAX_SIDE: u32 = 8192;

/// The memory cap the surface must stay inside (`D-03`, ARQ-44).
pub(crate) const SURFACE_BUDGET: Bytes = Bytes(1_500_000_000);

/// The surface of one frame (D4, O11).
pub(crate) struct Surface {
    width: u32,
    height: u32,
    origin: (i32, i32),
    /// The page's extent, in page pixels (O18): what an effect's window is
    /// measured against. The surface is a window onto the page, so this is not
    /// [`Self::surface_rect`].
    page: (u32, u32),
    /// What the surface holds, and therefore what `read_back` answers.
    pixels: Pixels,
}

/// The surface's storage, one variant per render target (O14).
enum Pixels {
    /// `width × height × 4` linear-light premultiplied values (ARQ-117).
    Linear(Vec<f32>),
    /// `width × height × 4` premultiplied sRGB bytes: the cache's encoding,
    /// stored as it stands (O14).
    Srgb8(Vec<u8>),
}

impl Surface {
    /// Opens a surface for `target`.
    ///
    /// `budget` is the cap the storage is compared against before it is
    /// allocated: one byte per channel for `Rgba8`, four for `RgbaF32`, which
    /// is what `O17` compares the effect's peak against.
    ///
    /// # Errors
    /// [`RenderError::Effect`] for a format that is not a render target
    /// (`Alpha8`), and [`RenderError::OutOfMemory`] for a surface over
    /// [`MAX_SIDE`] or over `budget`.
    pub(crate) fn new(target: &TargetDesc, budget: Bytes) -> Result<Self, RenderError> {
        if target.width > MAX_SIDE || target.height > MAX_SIDE {
            return Err(RenderError::OutOfMemory);
        }
        let channels = usize::try_from(u64::from(target.width) * u64::from(target.height))
            .ok()
            .and_then(|count| count.checked_mul(4))
            .ok_or(RenderError::OutOfMemory)?;
        let per_channel = match target.format {
            PixelFormat::Rgba8 => 1u64,
            PixelFormat::RgbaF32 => 4,
            PixelFormat::Alpha8 => return Err(not_rgba()),
        };
        let bytes = u64::try_from(channels)
            .ok()
            .and_then(|len| len.checked_mul(per_channel))
            .ok_or(RenderError::OutOfMemory)?;
        if Bytes(bytes) > budget {
            return Err(RenderError::OutOfMemory);
        }
        let pixels = match target.format {
            PixelFormat::Rgba8 => Pixels::Srgb8(vec![0u8; channels]),
            PixelFormat::RgbaF32 => Pixels::Linear(vec![0.0f32; channels]),
            // Refused above.
            PixelFormat::Alpha8 => return Err(not_rgba()),
        };
        Ok(Self {
            width: target.width,
            height: target.height,
            origin: target.origin,
            page: target.page,
            pixels,
        })
    }

    /// The format `read_back` answers.
    pub(crate) const fn format(&self) -> PixelFormat {
        match &self.pixels {
            Pixels::Linear(_) => PixelFormat::RgbaF32,
            Pixels::Srgb8(_) => PixelFormat::Rgba8,
        }
    }

    /// The page-space rectangle the surface covers.
    ///
    /// This is the window, not the page (ARQ-149, O18): a request off the page
    /// origin answers the region it asked for, inside a page that may be larger.
    pub(crate) fn surface_rect(&self) -> IRect {
        let x1 = i32::try_from(self.width).map_or(i32::MAX, |w| self.origin.0.saturating_add(w));
        let y1 = i32::try_from(self.height).map_or(i32::MAX, |h| self.origin.1.saturating_add(h));
        IRect::new(self.origin.0, self.origin.1, x1, y1)
    }

    /// The page's extent, in page pixels (O18).
    ///
    /// The effect's window is measured in page space, so a surface that is a
    /// sub-region of the page still names the page's extent.
    pub(crate) const fn page_extent(&self) -> (u32, u32) {
        self.page
    }

    /// The bytes the surface holds: one per channel for `Rgba8`, four for
    /// `RgbaF32` (ARQ-44, O14).
    pub(crate) fn bytes(&self) -> u64 {
        match &self.pixels {
            Pixels::Srgb8(pixels) => u64::try_from(pixels.len()).unwrap_or(u64::MAX),
            Pixels::Linear(pixels) => u64::try_from(pixels.len())
                .unwrap_or(u64::MAX)
                .saturating_mul(4),
        }
    }

    /// Writes one encoded tile into the surface, clipped to the target (D4).
    ///
    /// `region` is page space (O13), the same space as [`Self::read`] and as
    /// `TileTask.region`; the surface maps it into its own storage through
    /// [`pixel_index`]. `tile` is the tile [`encode_tile`] produced for the
    /// frame's format, tile-local and premultiplied; the part of `region` that
    /// falls outside the surface is dropped, so a clip that only partly
    /// overlaps the target loses nothing inside it.
    pub(crate) fn write(&mut self, region: IRect, tile: &ImageBuf) {
        let clip = region.intersect(self.surface_rect());
        if clip.is_empty() {
            return;
        }
        let source_width = tile.width();
        match (&mut self.pixels, tile.data()) {
            (Pixels::Linear(target), PixelBuf::RgbaF32(source)) => write_rows(
                target,
                source,
                self.origin,
                self.width,
                region,
                clip,
                source_width,
            ),
            (Pixels::Srgb8(target), PixelBuf::Rgba8(source)) => write_rows(
                target,
                source,
                self.origin,
                self.width,
                region,
                clip,
                source_width,
            ),
            // The loop encodes in the frame's format, so a tile of any other
            // format is not one this backend writes.
            _ => {}
        }
    }

    /// Reads `region` back in the target's format.
    ///
    /// `region` is page space (O13), the same space as [`Self::write`] and as
    /// `TileTask.region`; the surface maps it into its own storage through
    /// [`pixel_index`]. The answer is `region`-sized; the part of `region`
    /// outside the surface is transparent black, which is the pixel the page
    /// has there.
    ///
    /// # Errors
    /// [`RenderError::OutOfMemory`] when the region cannot be allocated.
    pub(crate) fn read(&self, region: IRect) -> Result<ImageBuf, RenderError> {
        let (width, height) = (region.width(), region.height());
        let count = usize::try_from(u64::from(width) * u64::from(height))
            .ok()
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(RenderError::OutOfMemory)?;
        let clip = region.intersect(self.surface_rect());
        match &self.pixels {
            Pixels::Linear(pixels) => {
                let mut out = vec![0.0f32; count];
                read_rows(&mut out, pixels, self.origin, self.width, region, clip);
                ImageBuf::new(width, height, PixelBuf::RgbaF32(out)).map_err(RenderError::Buffer)
            }
            Pixels::Srgb8(pixels) => {
                let mut out = vec![0u8; count];
                read_rows(&mut out, pixels, self.origin, self.width, region, clip);
                ImageBuf::new(width, height, PixelBuf::Rgba8(out)).map_err(RenderError::Buffer)
            }
        }
    }
}

/// Copies the clipped rows of a tile into the surface's storage.
fn write_rows<T: Copy>(
    target: &mut [T],
    source: &[T],
    origin: (i32, i32),
    width: u32,
    region: IRect,
    clip: IRect,
    source_width: u32,
) {
    let Ok(span) = usize::try_from(clip.width()) else {
        return;
    };
    let span = span * 4;
    let Ok(source_width) = usize::try_from(source_width) else {
        return;
    };
    for y in clip.y0..clip.y1 {
        let (Ok(source_row), Ok(source_column)) = (
            usize::try_from(y - region.y0),
            usize::try_from(clip.x0 - region.x0),
        ) else {
            continue;
        };
        let Some(target_start) = pixel_index(origin, width, clip.x0, y) else {
            continue;
        };
        let source_start = (source_row * source_width + source_column) * 4;
        copy_span(target, source, target_start, source_start, span);
    }
}

/// Copies the clipped rows of the surface into a `region`-sized output.
fn read_rows<T: Copy>(
    out: &mut [T],
    pixels: &[T],
    origin: (i32, i32),
    width: u32,
    region: IRect,
    clip: IRect,
) {
    let Ok(span) = usize::try_from(clip.width()) else {
        return;
    };
    let span = span * 4;
    for y in clip.y0..clip.y1 {
        let (Some(from), Some(at)) = (
            pixel_index(origin, width, clip.x0, y),
            pixel_index((region.x0, region.y0), region.width(), clip.x0, y),
        ) else {
            continue;
        };
        copy_span(out, pixels, at, from, span);
    }
}

/// The index of page pixel `(x, y)` in a `width`-wide buffer whose origin is
/// `origin`: four channels per pixel, for either storage. Callers clip first,
/// so this converts and checks the bounds of the arithmetic, not the pixel.
fn pixel_index(origin: (i32, i32), width: u32, x: i32, y: i32) -> Option<usize> {
    let width = usize::try_from(width).ok()?;
    let column = usize::try_from(x - origin.0).ok()?;
    let row = usize::try_from(y - origin.1).ok()?;
    row.checked_mul(width)?.checked_add(column)?.checked_mul(4)
}

/// Copies `span` channels from `source` into `target`, or nothing when either
/// slice is too short.
fn copy_span<T: Copy>(target: &mut [T], source: &[T], at: usize, from: usize, span: usize) {
    let (Some(target_end), Some(source_end)) = (at.checked_add(span), from.checked_add(span))
    else {
        return;
    };
    if target_end <= target.len() && source_end <= source.len() {
        target[at..target_end].copy_from_slice(&source[from..source_end]);
    }
}

/// Encodes one premultiplied linear pixel as premultiplied sRGB 8-bit.
///
/// The order is the one ARQ-117 and D9 fix: unpremultiply in linear light,
/// encode with the sRGB curve, and premultiply in 8 bits with
/// `(c × a + 127) / 255` on integer division.
pub(crate) fn quantise_srgb8(pixel: [f32; 4]) -> [u8; 4] {
    let alpha = pixel[3].clamp(0.0, 1.0);
    let a = round_u8(alpha);
    if a == 0 {
        return [0, 0, 0, 0];
    }
    let straight = [pixel[0] / alpha, pixel[1] / alpha, pixel[2] / alpha];
    let encoded = LinearRgba::new(straight[0], straight[1], straight[2], alpha).to_srgb();
    [
        premultiply_u8(round_u8(encoded.r), a),
        premultiply_u8(round_u8(encoded.g), a),
        premultiply_u8(round_u8(encoded.b), a),
        a,
    ]
}

/// The format of a tile or a surface image.
pub(crate) fn format_of(image: &ImageBuf) -> PixelFormat {
    match image.data() {
        PixelBuf::Rgba8(_) => PixelFormat::Rgba8,
        PixelBuf::RgbaF32(_) => PixelFormat::RgbaF32,
        PixelBuf::Alpha8(_) => PixelFormat::Alpha8,
    }
}

/// Encodes a tile-local premultiplied `f32` tile in `format`: the bytes the
/// cache holds and, for `Rgba8`, the bytes the surface stores (O14).
///
/// # Errors
/// [`RenderError::Effect`] for a format that is not a tile format, and
/// [`RenderError::Buffer`] when the buffer does not match `size`.
pub(crate) fn encode_tile(
    pixels: &[f32],
    size: (u32, u32),
    format: PixelFormat,
) -> Result<ImageBuf, RenderError> {
    match format {
        PixelFormat::RgbaF32 => ImageBuf::new(size.0, size.1, PixelBuf::RgbaF32(pixels.to_vec()))
            .map_err(RenderError::Buffer),
        PixelFormat::Rgba8 => {
            let mut out = vec![0u8; pixels.len()];
            for (index, pixel) in pixels.chunks_exact(4).enumerate() {
                out[index * 4..index * 4 + 4]
                    .copy_from_slice(&quantise_srgb8([pixel[0], pixel[1], pixel[2], pixel[3]]));
            }
            ImageBuf::new(size.0, size.1, PixelBuf::Rgba8(out)).map_err(RenderError::Buffer)
        }
        PixelFormat::Alpha8 => Err(not_rgba()),
    }
}

/// Decodes one premultiplied sRGB 8-bit pixel into premultiplied linear `f32`.
///
/// The inverse of [`quantise_srgb8`], used when an `Rgba8` image is drawn into
/// a linear buffer.
pub(crate) fn decode_srgb8(pixel: [u8; 4]) -> [f32; 4] {
    let alpha = f32::from(pixel[3]) / 255.0;
    if alpha == 0.0 {
        return [0.0; 4];
    }
    let straight = [
        f32::from(unpremultiply_u8(pixel[0], pixel[3])) / 255.0,
        f32::from(unpremultiply_u8(pixel[1], pixel[3])) / 255.0,
        f32::from(unpremultiply_u8(pixel[2], pixel[3])) / 255.0,
    ];
    let linear = Rgba::new(straight[0], straight[1], straight[2], alpha).to_linear();
    [linear.r * alpha, linear.g * alpha, linear.b * alpha, alpha]
}

/// The error of a buffer that is not an RGBA render target.
pub(crate) fn not_rgba() -> RenderError {
    RenderError::Effect(EffectError::UnsupportedFormat {
        format: PixelFormat::Alpha8,
    })
}

/// Rounds a `[0, 1]` channel to an 8-bit value, halves up.
fn round_u8(value: f32) -> u8 {
    let scaled = (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor();
    u8::try_from(scaled.clamp(0.0, 255.0) as u32).unwrap_or(u8::MAX)
}
