//! Path coverage: a `kurbo` path becomes a `tiny_skia` mask (D2, O6).
//!
//! `tiny-skia` is coverage **only** (ARQ-119): it turns the path into 8-bit
//! anti-aliased coverage, and `gafira-raster` does the compositing. The module
//! is the one place the crate names `tiny-skia`, so replacing the coverage
//! source touches nothing else (D2, "What is reversible").
//!
//! The mask is **tile-local**: the op's transform carries the page-space
//! placement, and the tile origin is pre-multiplied into it (D6), so the
//! coverage covers exactly the tile's window and no page-sized path is ever
//! scanned.

use gafira_geom::Affine;
use kurbo::{BezPath, PathEl};

/// The coverage of `path` on a `size`-pixel tile whose top-left page pixel is
/// `origin` (page pixels, ARQ-149).
///
/// `None` when the path has no element to fill: an empty path covers nothing
/// and tiny-skia refuses to fill it anyway.
pub(crate) fn mask(
    path: &BezPath,
    transform: Affine,
    size: (u32, u32),
    origin: (i32, i32),
) -> Option<tiny_skia::Mask> {
    let path = to_tiny(path)?;
    let mut mask = tiny_skia::Mask::new(size.0, size.1)?;
    mask.fill_path(
        &path,
        tiny_skia::FillRule::Winding,
        true,
        tile_transform(transform, origin),
    );
    Some(mask)
}

/// The `tiny_skia` transform of one op, pre-translated into tile coordinates.
///
/// `kurbo`'s coefficients are `[m11, m12, m21, m22, dx, dy]` with
/// `x' = m11·x + m21·y + dx`; `tiny_skia`'s row is `(sx, ky, kx, sy, tx, ty)`
/// with `x' = sx·x + kx·y + tx`, so the order is the same and only the
/// translation moves: the tile's origin is subtracted because the mask's pixel
/// `(0, 0)` is the tile's top-left page pixel.
fn tile_transform(transform: Affine, origin: (i32, i32)) -> tiny_skia::Transform {
    let [m11, m12, m21, m22, dx, dy] = transform.as_coeffs();
    tiny_skia::Transform::from_row(
        m11 as f32,
        m12 as f32,
        m21 as f32,
        m22 as f32,
        (dx - f64::from(origin.0)) as f32,
        (dy - f64::from(origin.1)) as f32,
    )
}

/// One `kurbo` path in the `tiny_skia` vocabulary. Both have the four drawing
/// verbs and no arc (D2).
fn to_tiny(path: &BezPath) -> Option<tiny_skia::Path> {
    let mut builder = tiny_skia::PathBuilder::new();
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => builder.move_to(p.x as f32, p.y as f32),
            PathEl::LineTo(p) => builder.line_to(p.x as f32, p.y as f32),
            PathEl::QuadTo(a, b) => builder.quad_to(a.x as f32, a.y as f32, b.x as f32, b.y as f32),
            PathEl::CurveTo(a, b, c) => builder.cubic_to(
                a.x as f32, a.y as f32, b.x as f32, b.y as f32, c.x as f32, c.y as f32,
            ),
            PathEl::ClosePath => builder.close(),
        }
    }
    builder.finish()
}
