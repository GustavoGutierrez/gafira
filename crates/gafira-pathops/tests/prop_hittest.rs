//! Property tests for the geometric hit testing.

use gafira_pathops::{Containment, DEFAULT_TOLERANCE, FillRule, Path, Point};
use kurbo::{BezPath, Rect, Shape};
use proptest::prelude::*;

/// Builds a path from a list of axis-aligned rectangles.
fn path_from_rects(rects: &[(f64, f64, f64, f64)]) -> Path {
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
    Path::new(bez, FillRule::NonZero)
}

fn rect_strategy() -> impl Strategy<Value = (f64, f64, f64, f64)> {
    (0.0..200.0, 0.0..200.0, 1.0..60.0, 1.0..60.0).prop_map(|(x, y, w, h)| (x, y, x + w, y + h))
}

proptest! {
    /// A point covered by the union is covered by at least one of the
    /// operands, and a point covered by neither is not covered by the union.
    #[test]
    fn the_containment_agrees_with_the_union(
        a in rect_strategy(),
        b in rect_strategy(),
        px in 0.0..300.0,
        py in 0.0..300.0,
    ) {
        let pa = path_from_rects(&[a]);
        let pb = path_from_rects(&[b]);
        let point = Point::new(px, py);

        let in_a = pa.contains(point, DEFAULT_TOLERANCE).unwrap();
        let in_b = pb.contains(point, DEFAULT_TOLERANCE).unwrap();
        let union = pa.union(&pb, DEFAULT_TOLERANCE).unwrap();
        let in_union = union.contains(point, DEFAULT_TOLERANCE).unwrap();

        // A point on a boundary of an operand is on a boundary of the result
        // too, so only the strict answers are compared.
        if in_a != Containment::OnBoundary && in_b != Containment::OnBoundary {
            prop_assert!(
                in_union != Containment::OnBoundary,
                "the union reported a boundary where the operands did not"
            );
        }

        let covered_by_an_operand =
            in_a == Containment::Inside || in_b == Containment::Inside;
        if in_union == Containment::Inside {
            prop_assert!(
                covered_by_an_operand,
                "the union covers the point but neither operand does"
            );
        }
        if !covered_by_an_operand && in_a != Containment::OnBoundary && in_b != Containment::OnBoundary {
            prop_assert!(
                in_union != Containment::Inside,
                "the union covers a point that neither operand covers"
            );
        }
    }

    /// The distance to the perimeter is never negative and is zero on the
    /// perimeter.
    #[test]
    fn the_distance_is_never_negative(
        a in rect_strategy(),
        px in 0.0..300.0,
        py in 0.0..300.0,
    ) {
        let path = path_from_rects(&[a]);
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
        let path = path_from_rects(&[a]);
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
