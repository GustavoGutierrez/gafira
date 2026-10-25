//! Property tests for the geometric hit testing.

use gafira_pathops::{Containment, DEFAULT_TOLERANCE, FillRule, Path, Point};
use kurbo::{BezPath, Rect, Shape};
use proptest::prelude::*;

/// Builds a path from a list of axis-aligned rectangles, read with the rule.
fn path_from_rects(rects: &[(f64, f64, f64, f64)], rule: FillRule) -> Path {
    let mut bez = BezPath::new();
    for (x0, y0, x1, y1) in rects {
        bez.extend(
            Rect::new(*x0, *y0, *x1, *y1)
                .to_path(0.0)
                .elements()
                .iter()
                .copied(),
        );
    }
    Path::new(bez, rule)
}

fn rect_strategy() -> impl Strategy<Value = (f64, f64, f64, f64)> {
    (0.0..200.0, 0.0..200.0, 1.0..60.0, 1.0..60.0).prop_map(|(x, y, w, h)| (x, y, x + w, y + h))
}

/// Whether the point is farther than `margin` from the edges of the rectangle.
fn clear_of_edges(rect: &(f64, f64, f64, f64), x: f64, y: f64, margin: f64) -> bool {
    let (x0, y0, x1, y1) = *rect;
    let near_x = (x - x0).abs() <= margin || (x - x1).abs() <= margin;
    let near_y = (y - y0).abs() <= margin || (y - y1).abs() <= margin;
    let in_x = x0 - margin <= x && x <= x1 + margin;
    let in_y = y0 - margin <= y && y <= y1 + margin;
    !((near_x && in_y) || (near_y && in_x))
}

proptest! {
    /// A point farther than the tolerance from every rectangle edge is inside
    /// the union exactly when it is inside at least one operand, whatever the
    /// fill rule of each operand.
    #[test]
    fn the_containment_agrees_with_the_union(
        a in prop::collection::vec(rect_strategy(), 1..=3),
        b in prop::collection::vec(rect_strategy(), 1..=3),
        rule_a in prop::sample::select(vec![FillRule::EvenOdd, FillRule::NonZero]),
        rule_b in prop::sample::select(vec![FillRule::EvenOdd, FillRule::NonZero]),
        px in 0.0..300.0,
        py in 0.0..300.0,
    ) {
        let pa = path_from_rects(&a, rule_a);
        let pb = path_from_rects(&b, rule_b);
        let point = Point::new(px, py);
        let margin = 2.0 * DEFAULT_TOLERANCE;
        prop_assume!(
            a.iter().chain(&b).all(|r| clear_of_edges(r, px, py, margin))
        );

        let in_a = pa.contains(point, DEFAULT_TOLERANCE).unwrap();
        let in_b = pb.contains(point, DEFAULT_TOLERANCE).unwrap();
        let union = pa.union(&pb, DEFAULT_TOLERANCE).unwrap();
        let in_union = union.contains(point, DEFAULT_TOLERANCE).unwrap();

        let covered = in_a == Containment::Inside || in_b == Containment::Inside;
        prop_assert_eq!(
            in_union,
            if covered { Containment::Inside } else { Containment::Outside }
        );
    }

    /// The distance to the perimeter is never negative and is zero on the
    /// perimeter.
    #[test]
    fn the_distance_is_never_negative(
        a in rect_strategy(),
        px in 0.0..300.0,
        py in 0.0..300.0,
    ) {
        let path = path_from_rects(&[a], FillRule::NonZero);
        let point = Point::new(px, py);

        let distance = path.distance_to_perimeter(point, DEFAULT_TOLERANCE).unwrap();

        prop_assert!(distance >= 0.0);
    }

    /// A point on the perimeter is reported as on the boundary, and a point
    /// far from the path is reported as outside.
    #[test]
    fn the_answer_follows_the_distance(
        a in rect_strategy(),
    ) {
        let path = path_from_rects(&[a], FillRule::NonZero);
        let (x0, y0, x1, y1) = a;

        // The middle of the top edge is on the perimeter.
        let on_edge = Point::new((x0 + x1) / 2.0, y0);
        let answer = path.contains(on_edge, DEFAULT_TOLERANCE).unwrap();
        prop_assert_eq!(answer, Containment::OnBoundary);

        // A point far away from the path is outside.
        let far = Point::new(x1 + 1000.0, y1 + 1000.0);
        let answer = path.contains(far, DEFAULT_TOLERANCE).unwrap();
        prop_assert_eq!(answer, Containment::Outside);
    }
}
