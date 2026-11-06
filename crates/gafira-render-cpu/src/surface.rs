//! The frame surface: linear light, `f32`, premultiplied (ARQ-117, O11).
//!
//! One surface pixel per page pixel (D4), covering the page rectangle that
//! starts at the target's `origin`. Both qualities accumulate here in `f32`
//! and the target's format decides only what `read_back` answers, so the
//! preview quantises **once**, when the surface is handed back as `Rgba8`
//! (O11). There is no second accumulation path.

use gafira_color::{LinearRgba, Rgba};
use gafira_geom::IRect;
use gafira_ports::common::Bytes;
use gafira_ports::render::{RenderError, TargetDesc};
use gafira_raster::{
    BufferError, EffectError, ImageBuf, PixelBuf, PixelFormat, premultiply_u8, unpremultiply_u8,
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
    /// What `read_back` answers: `Rgba8` or `RgbaF32`.
    format: PixelFormat,
    /// `width × height × 4` linear-light premultiplied values (ARQ-117).
    pixels: Vec<f32>,
}

impl Surface {
    /// Opens a surface for `target`.
    ///
    /// # Errors
    /// [`RenderError::Effect`] for a format that is not a render target
    /// (`Alpha8`), and [`RenderError::OutOfMemory`] for a surface over
    /// [`MAX_SIDE`] or over [`SURFACE_BUDGET`].
    pub(crate) fn new(target: &TargetDesc) -> Result<Self, RenderError> {
        if !matches!(target.format, PixelFormat::Rgba8 | PixelFormat::RgbaF32) {
            // The port's error set has no `Unsupported` variant; the buffer
            // vocabulary already names this case (`EffectError::UnsupportedFormat`).
            return Err(RenderError::Effect(EffectError::UnsupportedFormat {
                format: target.format,
            }));
        }
        if target.width > MAX_SIDE || target.height > MAX_SIDE {
            return Err(RenderError::OutOfMemory);
        }
        let pixels = usize::try_from(u64::from(target.width) * u64::from(target.height))
            .ok()
            .and_then(|count| count.checked_mul(4))
            .ok_or(RenderError::OutOfMemory)?;
        let bytes = u64::try_from(pixels)
            .ok()
            .and_then(|len| len.checked_mul(4))
            .ok_or(RenderError::OutOfMemory)?;
        if Bytes(bytes) > SURFACE_BUDGET {
            return Err(RenderError::OutOfMemory);
        }
        Ok(Self {
            width: target.width,
            height: target.height,
            origin: target.origin,
            format: target.format,
            pixels: vec![0.0; pixels],
        })
    }

    /// The format `read_back` answers.
    pub(crate) const fn format(&self) -> PixelFormat {
        self.format
    }

    /// The page-space rectangle the surface covers.
    pub(crate) fn page_rect(&self) -> IRect {
        let x1 = i32::try_from(self.width).map_or(i32::MAX, |w| self.origin.0.saturating_add(w));
        let y1 = i32::try_from(self.height).map_or(i32::MAX, |h| self.origin.1.saturating_add(h));
        IRect::new(self.origin.0, self.origin.1, x1, y1)
    }

    /// The bytes of the pixel buffer.
    pub(crate) fn bytes(&self) -> u64 {
        u64::try_from(self.pixels.len() * 4).unwrap_or(u64::MAX)
    }

    /// Writes one tile into the surface, clipped to the target (D4).
    ///
    /// `region` is page space (O13), the same space as [`Self::read`] and as
    /// `TileTask.region`; the surface maps it into its own storage through
    /// [`Self::source_index`]. `tile` is `size`-sized, tile-local and
    /// premultiplied; the part of `region` that falls outside the surface is
    /// dropped, so a clip that only partly overlaps the target loses nothing
    /// inside it.
    pub(crate) fn write(&mut self, region: IRect, tile: &[f32], size: (u32, u32)) {
        let clip = region.intersect(self.page_rect());
        if clip.is_empty() {
            return;
        }
        let Ok(span) = usize::try_from(clip.width()) else {
            return;
        };
        let span = span * 4;
        let Ok(tile_width) = usize::try_from(size.0) else {
            return;
        };
        for y in clip.y0..clip.y1 {
            let (Ok(source_row), Ok(source_column)) = (
                usize::try_from(y - region.y0),
                usize::try_from(clip.x0 - region.x0),
            ) else {
                continue;
            };
            let Some(target_start) = self.source_index(clip.x0, y) else {
                continue;
            };
            let source_start = (source_row * tile_width + source_column) * 4;
            let (Some(source_end), Some(target_end)) = (
                source_start.checked_add(span),
                target_start.checked_add(span),
            ) else {
                continue;
            };
            if source_end <= tile.len() && target_end <= self.pixels.len() {
                self.pixels[target_start..target_end]
                    .copy_from_slice(&tile[source_start..source_end]);
            }
        }
    }

    /// Reads `region` back in the target's format.
    ///
    /// `region` is page space (O13), the same space as [`Self::write`] and as
    /// `TileTask.region`; the surface maps it into its own storage through
    /// [`Self::source_index`]. The answer is `region`-sized; the part of
    /// `region` outside the surface is transparent black, which is the pixel
    /// the page has there.
    ///
    /// # Errors
    /// [`RenderError::OutOfMemory`] when the region cannot be allocated.
    pub(crate) fn read(&self, region: IRect) -> Result<ImageBuf, RenderError> {
        let (width, height) = (region.width(), region.height());
        let count = usize::try_from(u64::from(width) * u64::from(height))
            .ok()
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(RenderError::OutOfMemory)?;
        let clip = region.intersect(self.page_rect());
        match self.format {
            PixelFormat::RgbaF32 => {
                let mut out = vec![0.0f32; count];
                for (x, y) in pixels_of(clip) {
                    let (Some(source), Some(target)) =
                        (self.source_index(x, y), output_index(region, width, x, y))
                    else {
                        continue;
                    };
                    out[target..target + 4].copy_from_slice(&self.pixels[source..source + 4]);
                }
                ImageBuf::new(width, height, PixelBuf::RgbaF32(out)).map_err(RenderError::Buffer)
            }
            PixelFormat::Rgba8 => {
                let mut out = vec![0u8; count];
                for (x, y) in pixels_of(clip) {
                    let (Some(source), Some(target)) =
                        (self.source_index(x, y), output_index(region, width, x, y))
                    else {
                        continue;
                    };
                    let pixel = [
                        self.pixels[source],
                        self.pixels[source + 1],
                        self.pixels[source + 2],
                        self.pixels[source + 3],
                    ];
                    out[target..target + 4].copy_from_slice(&quantise_srgb8(pixel));
                }
                ImageBuf::new(width, height, PixelBuf::Rgba8(out)).map_err(RenderError::Buffer)
            }
            // `new` refuses every other format, so no surface can be in one.
            PixelFormat::Alpha8 => Err(RenderError::Buffer(BufferError::LengthMismatch {
                expected: count,
                actual: 0,
            })),
        }
    }

    /// The index of page pixel `(x, y)` in `pixels`, if it is inside.
    fn source_index(&self, x: i32, y: i32) -> Option<usize> {
        let Ok(width) = usize::try_from(self.width) else {
            return None;
        };
        let column = usize::try_from(x - self.origin.0).ok()?;
        let row = usize::try_from(y - self.origin.1).ok()?;
        if column >= width || row >= usize::try_from(self.height).ok()? {
            return None;
        }
        Some((row * width + column) * 4)
    }
}

/// Every pixel of `region`, row by row.
fn pixels_of(region: IRect) -> impl Iterator<Item = (i32, i32)> {
    (region.y0..region.y1).flat_map(move |y| (region.x0..region.x1).map(move |x| (x, y)))
}

/// The index of page pixel `(x, y)` in a `region`-sized, `width`-wide buffer.
fn output_index(region: IRect, width: u32, x: i32, y: i32) -> Option<usize> {
    let read_width = usize::try_from(width).ok()?;
    let column = usize::try_from(x - region.x0).ok()?;
    let row = usize::try_from(y - region.y0).ok()?;
    Some((row * read_width + column) * 4)
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

/// Encodes a tile-local premultiplied `f32` tile in `format`, for the cache.
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

/// Decodes a cached tile into tile-local premultiplied `f32` pixels.
///
/// # Errors
/// [`RenderError::Effect`] for an `Alpha8` tile, which is never a render target.
pub(crate) fn decode_tile(image: &ImageBuf) -> Result<Vec<f32>, RenderError> {
    match image.data() {
        PixelBuf::RgbaF32(data) => Ok(data.clone()),
        PixelBuf::Rgba8(data) => {
            let mut out = vec![0.0f32; data.len()];
            for (index, pixel) in data.chunks_exact(4).enumerate() {
                out[index * 4..index * 4 + 4]
                    .copy_from_slice(&decode_srgb8([pixel[0], pixel[1], pixel[2], pixel[3]]));
            }
            Ok(out)
        }
        PixelBuf::Alpha8(_) => Err(not_rgba()),
    }
}

/// Decodes one premultiplied sRGB 8-bit pixel into premultiplied linear `f32`.
///
/// The inverse of [`quantise_srgb8`], used when a cached tile is read back
/// into the `f32` surface.
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
fn not_rgba() -> RenderError {
    RenderError::Effect(EffectError::UnsupportedFormat {
        format: PixelFormat::Alpha8,
    })
}

/// Rounds a `[0, 1]` channel to an 8-bit value, halves up.
fn round_u8(value: f32) -> u8 {
    let scaled = (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor();
    u8::try_from(scaled.clamp(0.0, 255.0) as u32).unwrap_or(u8::MAX)
}
