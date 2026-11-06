//! Image sampling: `RenderOp::DrawImage` (CB4, 02 §6.1; ARQ-121, ARQ-117).
//!
//! An image reaches the plan as an `ImageBuf` and an affine transform from
//! image space to **page** space; the op carries no opacity and no filter flag,
//! so the filter is this delivery's decision and it is bilinear (ARQ-121). The
//! result is composited into the tile-local premultiplied `f32` buffer the
//! fills draw into, with the same `blend_f32` and the same `Normal` blend, so a
//! group's backdrop and an image agree on the arithmetic (O11, ARQ-119).
//!
//! Sampling happens in **linear light, premultiplied**: the tile's own space,
//! and the only space where an edge against transparency can be interpolated —
//! interpolating straight-alpha sRGB would drag the edge towards black. Each
//! tap is therefore converted from its own texel, in the order ARQ-117 fixes,
//! **before** any weight is applied: an `Rgba8` texel is unpremultiplied,
//! decoded with the sRGB curve and premultiplied again, and an `RgbaF32` texel
//! is taken as it stands because it already is linear premultiplied. Reading
//! the 8-bit value in any other order — converting the premultiplied number, or
//! interpolating first — draws a plausible page with the wrong colours, which
//! is why the conversion is the critical code of this delivery. The price, not
//! hidden: no converted copy of the image is kept, so a texel converts once per
//! tap and an 8-bit source pays the sRGB curve up to four times per output
//! pixel. `CB6` measures frames; until then no document produces this op
//! (ARQ-134).
//!
//! The inverse of the transform maps the centre of a page pixel — offset by the
//! tile's origin, because the ops carry page coordinates (ARQ-149) — into image
//! space, where texel `i` covers `[i, i+1)` and its centre sits at `i + 0.5`.
//! A sample point outside the image is not covered by it: the destination is
//! clipped to the image's own quad (in image space, one range test per axis),
//! not to its bounding box, so a rotated image does not smear its edge colour
//! into the corners of its box. A *tap* outside the texel grid is clamped to
//! the edge texel, so a magnified edge repeats its last texel instead of fading
//! into transparency.

use gafira_doc::BlendMode;
use gafira_geom::{Affine, IRect, Point, Rect};
use gafira_ports::render::RenderError;
use gafira_raster::{ImageBuf, PixelBuf, blend_f32};

use crate::coverage;
use crate::surface::{decode_srgb8, not_rgba};

/// The page-space box of `image` under `transform`, from its four corners.
///
/// A transformed rectangle's box is the minimal box holding its four mapped
/// corners, which is what a cull may use (D6); the image is the rectangle
/// `(0, 0)-(width, height)` in its own space.
pub(crate) fn page_bounds(image: &ImageBuf, transform: Affine) -> Rect {
    coverage::transformed_bounds(
        Rect::new(
            0.0,
            0.0,
            f64::from(image.width()),
            f64::from(image.height()),
        ),
        transform,
    )
}

/// Draws `image` into one tile (ARQ-121, ARQ-117).
///
/// `pixels` is `size`-sized, tile-local and premultiplied; `region` is the
/// tile's page-space rectangle (ARQ-149), so page pixel
/// `(region.x0 + column, region.y0 + row)` lands at the buffer's
/// `(column, row)` — the same placement the coverage of a fill uses.
///
/// # Errors
/// [`RenderError::Effect`] for an `Alpha8` image, which has no colour to draw,
/// and [`RenderError::Buffer`] when a texel or the composite is not finite.
pub(crate) fn draw(
    image: &ImageBuf,
    transform: Affine,
    pixels: &mut [f32],
    size: (u32, u32),
    region: IRect,
) -> Result<(), RenderError> {
    let source = Source::of(image)?;
    let inverse = transform.inverse();
    // A transform with no area has no sample point inside the image, and one
    // that is not finite cannot place a sample at all: both draw nothing.
    if !inverse.is_finite() {
        return Ok(());
    }
    let bounds = (f64::from(image.width()), f64::from(image.height()));
    let width = usize::try_from(size.0).unwrap_or(0);
    for row in 0..size.1 {
        for column in 0..size.0 {
            let page = Point::new(
                f64::from(region.x0) + f64::from(column) + 0.5,
                f64::from(region.y0) + f64::from(row) + 0.5,
            );
            let sample = inverse * page;
            if !(0.0..bounds.0).contains(&sample.x) || !(0.0..bounds.1).contains(&sample.y) {
                continue;
            }
            let at = (usize::try_from(row).unwrap_or(0) * width
                + usize::try_from(column).unwrap_or(0))
                * 4;
            if at + 4 > pixels.len() {
                continue;
            }
            let backdrop = [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]];
            let out = blend_f32(BlendMode::Normal, backdrop, source.sample(sample))?;
            pixels[at..at + 4].copy_from_slice(&out);
        }
    }
    Ok(())
}

/// One image's texels, in the space the tile composites in.
enum Source<'a> {
    /// Premultiplied sRGB bytes: converted per texel, in ARQ-117's order.
    Srgb8 {
        data: &'a [u8],
        grid: (usize, usize),
    },
    /// Linear-light premultiplied values, taken as they stand (ARQ-117).
    Linear32 {
        data: &'a [f32],
        grid: (usize, usize),
    },
}

impl<'a> Source<'a> {
    /// Reads `image` as a sampling source.
    ///
    /// # Errors
    /// [`RenderError::Effect`] for an `Alpha8` image, which carries no colour.
    fn of(image: &'a ImageBuf) -> Result<Self, RenderError> {
        let grid = (
            usize::try_from(image.width()).unwrap_or(0),
            usize::try_from(image.height()).unwrap_or(0),
        );
        match image.data() {
            PixelBuf::Rgba8(data) => Ok(Self::Srgb8 { data, grid }),
            PixelBuf::RgbaF32(data) => Ok(Self::Linear32 { data, grid }),
            PixelBuf::Alpha8(_) => Err(not_rgba()),
        }
    }

    /// The bilinear sample at the image-space point `sample` (ARQ-121).
    ///
    /// The caller guarantees the point is inside the image, so the taps bracket
    /// it; they are clamped all the same, because the probe of an edge pixel
    /// reaches half a texel outside the grid.
    fn sample(&self, sample: Point) -> [f32; 4] {
        let (columns, rows) = self.grid();
        let (left, right, across) = taps(sample.x - 0.5, columns);
        let (top, bottom, down) = taps(sample.y - 0.5, rows);
        let upper = mix(self.texel(left, top), self.texel(right, top), across);
        let lower = mix(self.texel(left, bottom), self.texel(right, bottom), across);
        mix(upper, lower, down)
    }

    /// The texel at `(column, row)`, in linear light and premultiplied.
    fn texel(&self, column: usize, row: usize) -> [f32; 4] {
        let at = (row * self.grid().0 + column) * 4;
        match self {
            // ARQ-117: unpremultiply, decode with the sRGB curve, premultiply.
            Self::Srgb8 { data, .. } => {
                decode_srgb8([data[at], data[at + 1], data[at + 2], data[at + 3]])
            }
            Self::Linear32 { data, .. } => data[at..at + 4].try_into().unwrap_or([0.0; 4]),
        }
    }

    /// The columns and rows of the texel grid.
    const fn grid(&self) -> (usize, usize) {
        match self {
            Self::Srgb8 { grid, .. } | Self::Linear32 { grid, .. } => *grid,
        }
    }
}

/// The two texel indices the sample coordinate `t` falls between, and the
/// weight of the second, clamped to the grid (ARQ-121).
///
/// A coordinate outside the grid resolves to the edge texel on both sides, so
/// the pair repeats it and the weight cannot move the value.
fn taps(t: f64, count: usize) -> (usize, usize, f32) {
    let base = t.floor();
    let weight = (t - base) as f32;
    (
        clamp_index(base, count),
        clamp_index(base + 1.0, count),
        weight,
    )
}

/// `index` clamped into `[0, count)`.
fn clamp_index(index: f64, count: usize) -> usize {
    let last = i64::try_from(count).unwrap_or(0) - 1;
    let clamped = (index as i64).clamp(0, last.max(0));
    usize::try_from(clamped).unwrap_or(0)
}

/// `a + f × (b − a)`: exact when both taps are the same texel, which is what
/// keeps a clamped edge and a one-texel image free of filter artefacts.
fn mix(a: [f32; 4], b: [f32; 4], f: f32) -> [f32; 4] {
    [
        a[0] + f * (b[0] - a[0]),
        a[1] + f * (b[1] - a[1]),
        a[2] + f * (b[2] - a[2]),
        a[3] + f * (b[3] - a[3]),
    ]
}
