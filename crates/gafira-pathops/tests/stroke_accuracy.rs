//! The accuracy of the round caps and the round joins of the stroke outline.
//!
//! The outline MUST lie within the tolerance of the exact stroke. For a straight
//! centreline the exact boundary of a round cap or of a round join is the set of
//! points at half the width from the centreline, so the test samples the whole
//! outline and measures that distance.

use gafira_pathops::{BezPath, Cap, FillRule, Join, Path, Point, StrokeStyle};
use kurbo::{ParamCurve, Shape};

const WIDTHS: [f64; 5] = [0.1, 1.0, 10.0, 100.0, 1000.0];
const TOLERANCES: [f64; 5] = [1e-1, 1e-2, 1e-3, 1e-4, 1e-5];
const SAMPLES_PER_SEGMENT: usize = 16;

/// The distance from a point to a straight segment.
fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.hypot2()).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

/// The distance from a point to the straight segments of a path.
fn distance_to_lines(p: Point, centreline: &[(Point, Point)]) -> f64 {
    centreline
        .iter()
        .map(|(a, b)| distance_to_segment(p, *a, *b))
        .fold(f64::INFINITY, f64::min)
}

/// The contour of the outline with the largest bounding box.
///
/// The inner contour of a closed stroke is kurbo's inner join, which cuts across
/// the interior of the stroke. It is not a boundary of the covered area, so only
/// the outer contour is compared with the exact offset.
fn outer_contour(outline: &Path) -> BezPath {
    let mut contours: Vec<BezPath> = Vec::new();
    for el in outline.as_bez_path().elements() {
        if matches!(el, kurbo::PathEl::MoveTo(_)) {
            contours.push(BezPath::new());
        }
        if let Some(last) = contours.last_mut() {
            last.push(*el);
        }
    }
    let area = |c: &BezPath| c.bounding_box().area();
    contours
        .into_iter()
        .max_by(|a, b| area(a).total_cmp(&area(b)))
        .unwrap_or_default()
}

/// The worst deviation of the outer contour from the exact boundary at half the
/// width.
fn worst_deviation(outline: &Path, centreline: &[(Point, Point)], width: f64) -> f64 {
    let mut worst = 0.0_f64;
    for segment in outer_contour(outline).segments() {
        for k in 0..=SAMPLES_PER_SEGMENT {
            let p = segment.eval(k as f64 / SAMPLES_PER_SEGMENT as f64);
            let deviation = (distance_to_lines(p, centreline) - width / 2.0).abs();
            worst = worst.max(deviation);
        }
    }
    worst
}

fn round_style(width: f64) -> StrokeStyle {
    StrokeStyle {
        width,
        cap: Cap::Round,
        join: Join::Round,
    }
}

#[test]
fn a_round_cap_lies_within_the_tolerance_of_the_exact_cap() {
    for width in WIDTHS {
        for tolerance in TOLERANCES {
            let mut bez = BezPath::new();
            bez.move_to((0.0, 0.0));
            bez.line_to((100.0, 0.0));
            let line = Path::new(bez, FillRule::NonZero);
            let centreline = [(Point::new(0.0, 0.0), Point::new(100.0, 0.0))];

            let outline = line.stroke_outline(&round_style(width), tolerance).unwrap();

            let worst = worst_deviation(&outline, &centreline, width);
            assert!(
                worst <= tolerance + 1e-10,
                "width {width}, tolerance {tolerance}: the cap deviates by {worst}"
            );
        }
    }
}

/// A closed polygon wider than the stroke, as a path and as its straight sides.
fn polygon(vertices: &[(f64, f64)]) -> (Path, Vec<(Point, Point)>) {
    let mut bez = BezPath::new();
    bez.move_to(vertices[0]);
    for v in &vertices[1..] {
        bez.line_to(*v);
    }
    bez.close_path();
    let points: Vec<Point> = vertices.iter().map(|v| Point::new(v.0, v.1)).collect();
    let sides = (0..points.len())
        .map(|i| (points[i], points[(i + 1) % points.len()]))
        .collect();
    (Path::new(bez, FillRule::NonZero), sides)
}

#[test]
fn a_round_join_lies_within_the_tolerance_of_the_exact_join() {
    for width in WIDTHS {
        for tolerance in TOLERANCES {
            // Closed polygons wider than the stroke: the outer contour of the
            // outline is the exact offset of the polygon. The square turns by
            // 90 degrees at each corner, the triangle by about 120.
            let side = 3.0 * width;
            let shapes = [
                polygon(&[(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)]),
                polygon(&[(0.0, 0.0), (side, 0.0), (side / 2.0, side)]),
            ];
            for (shape, centreline) in shapes {
                let outline = shape
                    .stroke_outline(&round_style(width), tolerance)
                    .unwrap();

                let worst = worst_deviation(&outline, &centreline, width);
                assert!(
                    worst <= tolerance + 1e-10,
                    "width {width}, tolerance {tolerance}: the join deviates by {worst}"
                );
            }
        }
    }
}
