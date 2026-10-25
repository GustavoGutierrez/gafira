//! The stroke outlining.
//!
//! A stroke covers an area around a path. The crate returns that area as a
//! path, so a caller can fill it, intersect it or measure it.

use kurbo::{Arc, BezPath, Cap, CubicBez, Join, ParamCurve, PathEl, Point, Stroke, StrokeOpts};

use crate::{FillRule, Path, PathOpError, validate};

/// The visual style of a stroke.
///
/// The style carries the width of the stroke and the way its ends and corners
/// are shaped. Dashes are out of scope for this crate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    /// The width of the stroke, in the unit of the path.
    ///
    /// It must be a finite positive number.
    pub width: f64,
    /// How the ends of an open subpath are shaped.
    pub cap: Cap,
    /// How the corners between two segments are shaped.
    pub join: Join,
}

impl Path {
    /// The outline of the area this path covers when stroked.
    ///
    /// The outline is always closed: a stroke encloses an area, even when the
    /// stroked path is open. The outline carries the `NonZero` fill rule,
    /// whatever rule the input carries, so a region the stroke covers more than
    /// once is covered by the outline and never a hole. The tolerance bounds the
    /// deviation between the outline and the exact stroke, including the arcs of
    /// round caps and round joins, whatever the width.
    pub fn stroke_outline(&self, style: &StrokeStyle, tolerance: f64) -> Result<Path, PathOpError> {
        validate::tolerance(tolerance)?;
        validate::stroke_width(style.width)?;
        self.validate_finite()?;

        let kurbo_style = Stroke::new(style.width)
            .with_caps(style.cap)
            .with_join(style.join);

        let outline = kurbo::stroke(
            self.as_bez_path().elements().iter().copied(),
            &kurbo_style,
            &StrokeOpts::default(),
            tolerance,
        );

        let refined = refine_round_arcs(&outline, self.as_bez_path(), style.width / 2.0, tolerance);

        Ok(Path::new(close_all(refined), FillRule::NonZero))
    }
}

/// Replaces the arcs the stroker draws for round caps and round joins by arcs
/// that lie within the tolerance.
///
/// The stroker fits those arcs on a unit circle with a fixed internal tolerance
/// and then scales them by half the width, so their error grows with the width
/// and ignores the tolerance of the caller. An arc of a round cap or a round
/// join is a cubic whose end points and middle point lie at half the width from
/// a vertex of the stroked path; each such cubic is replaced by an arc of the
/// same circle, sweeping the same angle, fitted at the tolerance of the caller.
fn refine_round_arcs(
    outline: &BezPath,
    centreline: &BezPath,
    radius: f64,
    tolerance: f64,
) -> BezPath {
    let vertices: Vec<Point> = centreline
        .elements()
        .iter()
        .filter_map(|el| el.end_point())
        .collect();
    let slack = radius * 1e-3;
    let on_circle = |c: Point, p: Point| (c.distance(p) - radius).abs() <= slack;

    let mut refined = BezPath::new();
    let mut current = Point::ORIGIN;
    for el in outline.elements() {
        if let PathEl::CurveTo(c1, c2, end) = *el {
            let cubic = CubicBez::new(current, c1, c2, end);
            let mid = cubic.eval(0.5);
            let replaced = vertices
                .iter()
                .copied()
                .filter(|c| on_circle(*c, current) && on_circle(*c, end) && on_circle(*c, mid))
                .find_map(|c| arc_sweep(c, current, mid, end).map(|sweep| (c, sweep)));
            if let Some((centre, sweep)) = replaced {
                let start_angle = (current - centre).atan2();
                let arc = Arc::new(centre, (radius, radius), start_angle, sweep, 0.0);
                let mut pieces: Vec<[Point; 3]> = Vec::new();
                arc.to_cubic_beziers(tolerance, |p1, p2, p3| pieces.push([p1, p2, p3]));
                if let Some(last) = pieces.last_mut() {
                    last[2] = end;
                }
                for [p1, p2, p3] in pieces {
                    refined.curve_to(p1, p2, p3);
                }
                current = end;
                continue;
            }
        }
        refined.push(*el);
        match el {
            PathEl::ClosePath => {}
            other => {
                if let Some(p) = other.end_point() {
                    current = p;
                }
            }
        }
    }
    refined
}

/// The signed angle swept from `start` to `end` around `centre`, in the
/// direction that passes through `mid`.
fn arc_sweep(centre: Point, start: Point, mid: Point, end: Point) -> Option<f64> {
    use std::f64::consts::{PI, TAU};
    let a0 = (start - centre).atan2();
    let am = (mid - centre).atan2();
    let a1 = (end - centre).atan2();
    let towards_mid = (am - a0 + PI).rem_euclid(TAU) - PI;
    if towards_mid == 0.0 {
        None
    } else if towards_mid > 0.0 {
        Some((a1 - a0).rem_euclid(TAU))
    } else {
        Some(-(a0 - a1).rem_euclid(TAU))
    }
}

/// Closes every subpath of the outline.
fn close_all(bez: BezPath) -> BezPath {
    let mut closed = BezPath::new();
    let mut open = false;
    for el in bez.elements() {
        match el {
            kurbo::PathEl::MoveTo(p) => {
                if open {
                    closed.close_path();
                }
                open = false;
                closed.move_to(*p);
            }
            kurbo::PathEl::ClosePath => {
                closed.close_path();
                open = false;
            }
            other => {
                open = true;
                closed.push(*other);
            }
        }
    }
    if open {
        closed.close_path();
    }
    closed
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Rect, Shape};

    #[test]
    fn the_outline_of_an_open_line_is_closed() {
        let mut bez = BezPath::new();
        bez.move_to((0.0, 0.0));
        bez.line_to((10.0, 0.0));
        let path = Path::new(bez, FillRule::NonZero);

        let outline = path
            .stroke_outline(
                &StrokeStyle {
                    width: 2.0,
                    cap: Cap::Butt,
                    join: Join::Miter,
                },
                1e-3,
            )
            .unwrap();

        assert!(outline.is_subpath_closed(0));
    }

    #[test]
    fn the_outline_of_a_rectangle_is_closed() {
        let path = Path::new(
            Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0),
            FillRule::NonZero,
        );

        let outline = path
            .stroke_outline(
                &StrokeStyle {
                    width: 2.0,
                    cap: Cap::Round,
                    join: Join::Round,
                },
                1e-3,
            )
            .unwrap();

        // A stroked closed shape has an outer boundary and an inner one.
        assert_eq!(outline.subpath_count(), 2);
        assert!(outline.is_subpath_closed(0));
        assert!(outline.is_subpath_closed(1));
    }
}
