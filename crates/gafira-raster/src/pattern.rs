//! The patterns of page backgrounds (spec 069) as vector shapes, drawn the
//! same by the export and by the editor canvas, sharp at any zoom.

use gafira_doc::PatternKind;
use kurbo::{BezPath, Circle, Point, Rect, Shape, Vec2};

/// The repeat of a background pattern, in px of page (spec 069).
pub const PATTERN_CELL: f64 = 32.0;

/// The drawn part of `kind` over a `width` × `height` page, one repeat every
/// `cell` px, as vector shapes (spec 069): every subpath is convex and they
/// all turn the same way, so a non-zero fill draws their union and a
/// renderer can draw each one as a convex polygon. Sharp at any zoom; what
/// reaches past the page is cut by whoever draws it.
#[must_use]
pub fn pattern_path(kind: PatternKind, width: f64, height: f64, cell: f64) -> BezPath {
    let cell = cell.max(2.0);
    let mut out = BezPath::new();
    let across = (width / cell).ceil() as i64 + 1;
    let down = (height / cell).ceil() as i64 + 1;
    let quad = |out: &mut BezPath, points: [Point; 4]| {
        out.move_to(points[0]);
        for p in &points[1..] {
            out.line_to(*p);
        }
        out.close_path();
    };
    match kind {
        PatternKind::Dots => {
            for j in 0..down {
                for i in 0..across {
                    let centre = Point::new((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
                    out.extend(Circle::new(centre, cell * 0.12).path_elements(0.1));
                }
            }
        }
        PatternKind::Grid => {
            let t = cell * 0.025;
            for i in 0..=across {
                let x = i as f64 * cell;
                out.extend(Rect::new(x - t, 0.0, x + t, height).path_elements(0.1));
            }
            for j in 0..=down {
                let y = j as f64 * cell;
                out.extend(Rect::new(0.0, y - t, width, y + t).path_elements(0.1));
            }
        }
        PatternKind::Stripes => {
            // Bands where x + y is within an eighth of a repeat of the
            // middle of the repeat.
            let reach = width + height + 2.0 * cell;
            let half = cell * 0.125;
            let bands = (reach / cell).ceil() as i64 + 1;
            for k in -1..bands {
                let middle = k as f64 * cell + cell / 2.0;
                let (a, b) = (middle - half, middle + half);
                let (top, bottom) = (-cell, height + cell);
                quad(
                    &mut out,
                    [
                        Point::new(a - top, top),
                        Point::new(b - top, top),
                        Point::new(b - bottom, bottom),
                        Point::new(a - bottom, bottom),
                    ],
                );
            }
        }
        PatternKind::Checks => {
            for j in 0..down {
                for i in 0..across {
                    if (i + j) % 2 == 0 {
                        let (x, y) = (i as f64 * cell, j as f64 * cell);
                        out.extend(Rect::new(x, y, x + cell, y + cell).path_elements(0.1));
                    }
                }
            }
        }
        PatternKind::Zigzag => {
            // A line up and down twice a repeat across the middle of each
            // row, as one thick segment per slope.
            let t = cell * 0.05;
            for j in 0..down {
                let base = j as f64 * cell + cell / 2.0;
                let (high, low) = (base - cell * 0.125, base + cell * 0.125);
                // Four slopes a repeat, past both sides of the page.
                for k in -1..(4 * across + 4) {
                    let x0 = k as f64 * cell / 4.0;
                    let x1 = x0 + cell / 4.0;
                    let (y0, y1) = if k % 2 == 0 { (high, low) } else { (low, high) };
                    quad(
                        &mut out,
                        [
                            Point::new(x0, y0 - t),
                            Point::new(x1, y1 - t),
                            Point::new(x1, y1 + t),
                            Point::new(x0, y0 + t),
                        ],
                    );
                }
            }
        }
        // Rows half a repeat high and bricks a repeat wide, or twice that
        // for the large ones; every other row shifted half a brick.
        PatternKind::Bricks => bricks(&mut out, (width, height), cell / 2.0, cell, t_of(cell)),
        PatternKind::BricksLarge => bricks(&mut out, (width, height), cell, cell * 2.0, t_of(cell)),
        // A dense grid of thinner lines (quadrille).
        PatternKind::GridFine => {
            let spacing = cell * 0.4;
            let t = cell * 0.015;
            let columns = (width / spacing).ceil() as i64;
            let rows = (height / spacing).ceil() as i64;
            for i in 0..=columns {
                let x = i as f64 * spacing;
                out.extend(Rect::new(x - t, 0.0, x + t, height).path_elements(0.1));
            }
            for j in 0..=rows {
                let y = j as f64 * spacing;
                out.extend(Rect::new(0.0, y - t, width, y + t).path_elements(0.1));
            }
        }
        PatternKind::GridDiagonal => {
            lines(&mut out, (width, height), 0.0, cell, cell);
            lines(&mut out, (width, height), 90.0, cell, cell);
            lines(
                &mut out,
                (width, height),
                45.0,
                cell * 2.0 * std::f64::consts::FRAC_1_SQRT_2,
                cell,
            );
            lines(
                &mut out,
                (width, height),
                -45.0,
                cell * 2.0 * std::f64::consts::FRAC_1_SQRT_2,
                cell,
            );
        }
        PatternKind::Hexagons => {
            // Pointy-top honeycomb: each hexagon's six edges.
            let r = cell * 0.6;
            let (dx, dy) = (3.0_f64.sqrt() * r, 1.5 * r);
            let cols = (width / dx).ceil() as i64 + 2;
            let rows = (height / dy).ceil() as i64 + 2;
            for j in -1..rows {
                for i in -1..cols {
                    let shift = if j % 2 == 0 { 0.0 } else { dx / 2.0 };
                    let centre = Point::new(i as f64 * dx + shift, j as f64 * dy);
                    let corner = |k: i64| {
                        let a = (-90.0 + 60.0 * k as f64).to_radians();
                        centre + Vec2::new(a.cos(), a.sin()) * r
                    };
                    for k in 0..6 {
                        segment(&mut out, corner(k), corner(k + 1), t_of(cell));
                    }
                }
            }
        }
        PatternKind::Isometric => {
            lines(&mut out, (width, height), 30.0, cell / 2.0, cell);
            lines(&mut out, (width, height), -30.0, cell / 2.0, cell);
        }
        PatternKind::Diamonds => {
            lines(
                &mut out,
                (width, height),
                45.0,
                cell * std::f64::consts::FRAC_1_SQRT_2,
                cell,
            );
            lines(
                &mut out,
                (width, height),
                -45.0,
                cell * std::f64::consts::FRAC_1_SQRT_2,
                cell,
            );
        }
        PatternKind::Triangles => {
            // Equilateral triangles with upright sides.
            let side = cell * 3.0_f64.sqrt() / 2.0;
            lines(&mut out, (width, height), 90.0, side, cell);
            lines(&mut out, (width, height), 30.0, side, cell);
            lines(&mut out, (width, height), -30.0, side, cell);
        }
        PatternKind::Crosses => {
            let (arm, t) = (cell * 0.18, cell * 0.05);
            for j in 0..down {
                for i in 0..across {
                    let (cx, cy) = ((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
                    out.extend(Rect::new(cx - arm, cy - t, cx + arm, cy + t).path_elements(0.1));
                    out.extend(Rect::new(cx - t, cy - arm, cx + t, cy + arm).path_elements(0.1));
                }
            }
        }
    }
    out
}

/// Bricks over a `size` page: rows `row` high, bricks `brick` wide, every
/// other row shifted half a brick, lines `t` each side.
fn bricks(out: &mut BezPath, (width, height): (f64, f64), row: f64, brick: f64, t: f64) {
    let rows = (height / row).ceil() as i64 + 1;
    let across = (width / brick).ceil() as i64 + 1;
    for j in 0..=rows {
        let y = j as f64 * row;
        out.extend(Rect::new(0.0, y - t, width, y + t).path_elements(0.1));
        let shift = if j % 2 == 0 { 0.0 } else { brick / 2.0 };
        for i in -1..=across {
            let x = i as f64 * brick + shift;
            out.extend(Rect::new(x - t, y, x + t, y + row).path_elements(0.1));
        }
    }
}

/// Half the width of a pattern line, for a repeat of `cell`.
fn t_of(cell: f64) -> f64 {
    cell * 0.025
}

/// A line from `a` to `b`, `t` each side of it, as one convex quad.
fn segment(out: &mut BezPath, a: Point, b: Point, t: f64) {
    let along = b - a;
    let length = along.hypot();
    if length <= 0.0 {
        return;
    }
    let n = Vec2::new(-along.y, along.x) / length * t;
    out.move_to(a + n);
    out.line_to(b + n);
    out.line_to(b - n);
    out.line_to(a - n);
    out.close_path();
}

/// Parallel lines at `angle_deg` from the horizontal, `spacing` apart,
/// across a `size` page, each `cell`'s line width.
fn lines(out: &mut BezPath, (width, height): (f64, f64), angle_deg: f64, spacing: f64, cell: f64) {
    let (sin, cos) = angle_deg.to_radians().sin_cos();
    let along = Vec2::new(cos, sin);
    let normal = Vec2::new(-sin, cos);
    let reach = width.hypot(height) + spacing;
    let count = (reach / spacing).ceil() as i64;
    for k in -count..=count {
        let middle = Point::new(width / 2.0, height / 2.0) + normal * (k as f64 * spacing);
        segment(
            out,
            middle - along * reach,
            middle + along * reach,
            t_of(cell),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // S69-02: the vector pattern covers the same places as the pixels: a
    // dot in the middle of each repeat, a square on every other cell.
    #[test]
    fn s69_02_vector_patterns_cover_their_shapes() {
        use kurbo::Shape as _;
        let inside =
            |kind, x, y| pattern_path(kind, 80.0, 80.0, 40.0).winding(Point::new(x, y)) != 0;
        assert!(inside(PatternKind::Dots, 20.0, 20.0) && inside(PatternKind::Dots, 60.0, 60.0));
        assert!(!inside(PatternKind::Dots, 2.0, 2.0));
        assert!(inside(PatternKind::Grid, 0.5, 20.0) && !inside(PatternKind::Grid, 20.0, 20.0));
        assert!(
            inside(PatternKind::Checks, 10.0, 10.0) && !inside(PatternKind::Checks, 50.0, 10.0)
        );
        assert!(
            inside(PatternKind::Crosses, 20.0, 20.0) && !inside(PatternKind::Crosses, 2.0, 2.0)
        );
        assert!(
            inside(PatternKind::Stripes, 10.0, 10.0) && !inside(PatternKind::Stripes, 0.5, 0.5)
        );
        assert!(
            inside(PatternKind::Zigzag, 0.5, 15.0),
            "the line starts at its peak"
        );
        assert!(!inside(PatternKind::Zigzag, 0.5, 35.0));
        // The paper patterns draw lines, not areas: the middle of a cell
        // is empty, a line is drawn.
        for kind in [
            PatternKind::Bricks,
            PatternKind::GridDiagonal,
            PatternKind::Hexagons,
            PatternKind::Isometric,
            PatternKind::Diamonds,
            PatternKind::Triangles,
            PatternKind::BricksLarge,
            PatternKind::GridFine,
        ] {
            let path = pattern_path(kind, 200.0, 200.0, 40.0);
            assert!(!path.elements().is_empty(), "{kind:?}");
            let covered = (0..200)
                .flat_map(|x| (0..200).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    path.winding(Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5)) != 0
                })
                .count();
            assert!(
                (1_000..20_000).contains(&covered),
                "{kind:?}: thin lines, {covered} px"
            );
        }
        assert!(inside(PatternKind::Bricks, 20.0, 0.5), "a row line");
        assert!(
            inside(PatternKind::Diamonds, 20.0, 20.0) || inside(PatternKind::Diamonds, 0.0, 0.0)
        );
    }

    // Owner rule: every pattern fills the whole page, to its four edges
    // and corners, whatever its size.
    #[test]
    fn s69_02_every_pattern_reaches_every_edge() {
        let (w, h, cell) = (300.0, 200.0, 32.0);
        for kind in PatternKind::ALL {
            let path = pattern_path(kind, w, h, cell);
            // In each band of two repeats along every edge, some point of
            // the band is drawn.
            let drawn = |x0: f64, y0: f64, x1: f64, y1: f64| {
                let step = 2.0;
                let mut y = y0;
                while y < y1 {
                    let mut x = x0;
                    while x < x1 {
                        if path.winding(Point::new(x, y)) != 0 {
                            return true;
                        }
                        x += step;
                    }
                    y += step;
                }
                false
            };
            let band = cell * 2.0;
            assert!(drawn(w - band, 0.0, w, h), "{kind:?}: right edge");
            assert!(drawn(0.0, h - band, w, h), "{kind:?}: bottom edge");
            assert!(drawn(0.0, 0.0, band, h), "{kind:?}: left edge");
            assert!(drawn(0.0, 0.0, w, band), "{kind:?}: top edge");
            assert!(
                drawn(w - band, h - band, w, h),
                "{kind:?}: bottom right corner"
            );
        }
    }
}
