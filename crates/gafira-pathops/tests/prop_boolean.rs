//! Property tests for the path boolean operations.
//!
//! The properties are the algebraic identities of set difference, checked on
//! generated paths rather than on fixed examples.

use gafira_pathops::{Containment, DEFAULT_TOLERANCE, FillRule, Path, Point};
use kurbo::{BezPath as KurbBezPath, Rect, Shape};
use proptest::prelude::*;

/// Builds a path from a list of axis-aligned rectangles, each one a subpath.
fn path_from_rects(rects: &[(f64, f64, f64, f64)]) -> Path {
    let mut bez = KurbBezPath::new();
    for (x0, y0, x1, y1) in rects {
        let sub = Rect::new(*x0, *y0, *x1, *y1).to_path(0.0);
        bez.extend(sub.elements().iter().copied());
    }
    Path::new(bez, FillRule::NonZero)
}

/// Builds a path from rectangles read with the given rule.
fn ruled_path(rects: &[(f64, f64, f64, f64)], rule: FillRule) -> Path {
    Path::new(path_from_rects(rects).as_bez_path().clone(), rule)
}

fn rule_strategy() -> impl Strategy<Value = FillRule> {
    prop::sample::select(vec![FillRule::EvenOdd, FillRule::NonZero])
}

/// One to three rectangles, which may overlap.
fn rects_strategy() -> impl Strategy<Value = Vec<(f64, f64, f64, f64)>> {
    prop::collection::vec(rect_strategy(), 1..=3)
}

/// Whether the oracle says the rectangles cover the point under the rule.
fn oracle_covers(rects: &[(f64, f64, f64, f64)], rule: FillRule, x: f64, y: f64) -> bool {
    let count = rects
        .iter()
        .filter(|(x0, y0, x1, y1)| *x0 < x && x < *x1 && *y0 < y && y < *y1)
        .count();
    match rule {
        FillRule::EvenOdd => count % 2 == 1,
        FillRule::NonZero => count > 0,
    }
}

/// Whether the point is farther than `margin` from every rectangle edge.
fn clear_of_edges(rects: &[(f64, f64, f64, f64)], x: f64, y: f64, margin: f64) -> bool {
    rects.iter().all(|(x0, y0, x1, y1)| {
        let near_x = (x - x0).abs() <= margin || (x - x1).abs() <= margin;
        let near_y = (y - y0).abs() <= margin || (y - y1).abs() <= margin;
        let in_x = *x0 - margin <= x && x <= *x1 + margin;
        let in_y = *y0 - margin <= y && y <= *y1 + margin;
        !((near_x && in_y) || (near_y && in_x))
    })
}

/// A rectangle well away from the origin, so a difference is never empty by
/// accident.
fn rect_strategy() -> impl Strategy<Value = (f64, f64, f64, f64)> {
    (0.0..500.0, 0.0..500.0, 1.0..100.0, 1.0..100.0).prop_map(|(x, y, w, h)| (x, y, x + w, y + h))
}

proptest! {
    /// Each operation covers a point exactly when the definition says, from
    /// the answers of the two operands, each read with its own fill rule.
    #[test]
    fn the_point_by_point_definition_holds_for_rectangles(
        a in rects_strategy(),
        b in rects_strategy(),
        rule_a in rule_strategy(),
        rule_b in rule_strategy(),
        x in 0.0..600.0,
        y in 0.0..600.0,
    ) {
        prop_assume!(clear_of_edges(&a, x, y, 2.0 * DEFAULT_TOLERANCE));
        prop_assume!(clear_of_edges(&b, x, y, 2.0 * DEFAULT_TOLERANCE));
        let pa = ruled_path(&a, rule_a);
        let pb = ruled_path(&b, rule_b);
        let in_a = oracle_covers(&a, rule_a, x, y);
        let in_b = oracle_covers(&b, rule_b, x, y);
        let point = Point::new(x, y);
        let covers = |path: &Path| {
            path.contains(point, DEFAULT_TOLERANCE).unwrap() == Containment::Inside
        };

        let union = pa.union(&pb, DEFAULT_TOLERANCE).unwrap();
        let intersection = pa.intersection(&pb, DEFAULT_TOLERANCE).unwrap();
        let difference = pa.difference(&pb, DEFAULT_TOLERANCE).unwrap();
        let reverse = pb.difference(&pa, DEFAULT_TOLERANCE).unwrap();
        let xor = pa.xor(&pb, DEFAULT_TOLERANCE).unwrap();

        prop_assert_eq!(covers(&union), in_a || in_b);
        prop_assert_eq!(covers(&intersection), in_a && in_b);
        prop_assert_eq!(covers(&difference), in_a && !in_b);
        prop_assert_eq!(covers(&reverse), in_b && !in_a);
        prop_assert_eq!(covers(&xor), in_a != in_b);
        prop_assert!(!(covers(&difference) && covers(&reverse)));
        for result in [&union, &intersection, &difference, &reverse, &xor] {
            prop_assert_eq!(result.fill_rule(), FillRule::NonZero);
        }
    }

    /// The bounding box of `A \ B` is contained in the bounding box of `A`.
    #[test]
    fn the_difference_is_contained_in_its_first_operand(
        a in rect_strategy(),
        b in rect_strategy(),
    ) {
        let pa = path_from_rects(&[a]);
        let pb = path_from_rects(&[b]);

        let difference = pa.difference(&pb, DEFAULT_TOLERANCE).unwrap();

        // An empty difference covers no point, so it has no meaningful
        // bounding box to compare: an empty path reports the origin box.
        if !difference.is_empty() {
            let (ax0, ay0, ax1, ay1) = bounds(&pa);
            let (dx0, dy0, dx1, dy1) = bounds(&difference);
            prop_assert!(dx0 >= ax0 - 1e-6, "min_x {dx0} < {ax0}");
            prop_assert!(dy0 >= ay0 - 1e-6, "min_y {dy0} < {ay0}");
            prop_assert!(dx1 <= ax1 + 1e-6, "max_x {dx1} > {ax1}");
            prop_assert!(dy1 <= ay1 + 1e-6, "max_y {dy1} > {ay1}");
        }
    }

    /// The bounding box of the exclusive-or equals the bounding box of the
    /// union.
    #[test]
    fn the_exclusive_or_spans_the_same_box_as_the_union(
        a in rect_strategy(),
        b in rect_strategy(),
    ) {
        let pa = path_from_rects(&[a]);
        let pb = path_from_rects(&[b]);

        let xor = pa.xor(&pb, DEFAULT_TOLERANCE).unwrap();
        let union = pa.union(&pb, DEFAULT_TOLERANCE).unwrap();

        let (xo0, yo0, xo1, yo1) = bounds(&xor);
        let (uo0, uo0y, uo1, uo1y) = bounds(&union);
        prop_assert!((xo0 - uo0).abs() < 1e-6);
        prop_assert!((yo0 - uo0y).abs() < 1e-6);
        prop_assert!((xo1 - uo1).abs() < 1e-6);
        prop_assert!((yo1 - uo1y).abs() < 1e-6);
    }

    /// The exclusive-or is symmetric.
    #[test]
    fn the_exclusive_or_is_symmetric(
        a in rect_strategy(),
        b in rect_strategy(),
    ) {
        let pa = path_from_rects(&[a]);
        let pb = path_from_rects(&[b]);

        prop_assert_eq!(pa.xor(&pb, DEFAULT_TOLERANCE).unwrap(),
                        pb.xor(&pa, DEFAULT_TOLERANCE).unwrap());
    }

    /// The intersection is contained in both operands.
    #[test]
    fn the_intersection_is_contained_in_both_operands(
        a in rect_strategy(),
        b in rect_strategy(),
    ) {
        let pa = path_from_rects(&[a]);
        let pb = path_from_rects(&[b]);

        let intersection = pa.intersection(&pb, DEFAULT_TOLERANCE).unwrap();

        // An empty intersection covers no point, so it has no meaningful
        // bounding box to compare.
        if !intersection.is_empty() {
            let (ax0, ay0, ax1, ay1) = bounds(&pa);
            let (bx0, by0, bx1, by1) = bounds(&pb);
            let (ix0, iy0, ix1, iy1) = bounds(&intersection);
            prop_assert!(ix0 >= ax0.max(bx0) - 1e-6);
            prop_assert!(iy0 >= ay0.max(by0) - 1e-6);
            prop_assert!(ix1 <= ax1.min(bx1) + 1e-6);
            prop_assert!(iy1 <= ay1.min(by1) + 1e-6);
        }
    }

    /// The same inputs give the same result on every run.
    #[test]
    fn the_operation_is_deterministic(
        a in rect_strategy(),
        b in rect_strategy(),
    ) {
        let pa = path_from_rects(&[a]);
        let pb = path_from_rects(&[b]);

        let first = pa.union(&pb, DEFAULT_TOLERANCE).unwrap();
        for _ in 0..4 {
            prop_assert_eq!(pa.union(&pb, DEFAULT_TOLERANCE).unwrap(), first.clone());
        }
    }

    /// A union never loses a curve: a circular operand keeps its cubic
    /// segments.
    #[test]
    fn a_curve_survives_the_union(
        cx in 0.0..100.0,
        cy in 0.0..100.0,
        r in 5.0..50.0,
    ) {
        use kurbo::Circle;
        let circle = Path::new(
            Circle::new((cx, cy), r).to_path(1e-6),
            FillRule::NonZero,
        );
        // The square overlaps the circle instead of containing it: a circle
        // fully inside the square makes the union the square, which has no
        // cubic segment to keep.
        let square = path_from_rects(&[(cx, cy, cx + 2.0 * r, cy + 2.0 * r)]);

        let union = circle.union(&square, DEFAULT_TOLERANCE).unwrap();

        let cubics = union
            .as_bez_path()
            .elements()
            .iter()
            .filter(|el| matches!(el, kurbo::PathEl::CurveTo(..)))
            .count();
        prop_assert!(cubics >= 1, "the union lost every cubic segment");
    }
}

/// The bounding box of a path.
fn bounds(path: &Path) -> (f64, f64, f64, f64) {
    let b = path.as_bez_path().bounding_box();
    (b.min_x(), b.min_y(), b.max_x(), b.max_y())
}
