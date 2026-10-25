//! The public contract of the path boolean operations.

use gafira_pathops::{
    BezPath, Containment, DEFAULT_TOLERANCE, FillRule, NonFiniteValue, Path, PathOpError, Point,
};
use kurbo::{Circle, Rect, Shape};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
    Path::new(Rect::new(x0, y0, x1, y1).to_path(0.0), FillRule::NonZero)
}

fn bounds(path: &Path) -> (f64, f64, f64, f64) {
    let b = path.as_bez_path().bounding_box();
    (b.min_x(), b.min_y(), b.max_x(), b.max_y())
}

#[test]
fn the_union_of_two_overlapping_rectangles_spans_both() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(5.0, 0.0, 15.0, 10.0);

    let union = a.union(&b, DEFAULT_TOLERANCE).unwrap();

    assert_eq!(union.subpath_count(), 1);
    let (x0, y0, x1, y1) = bounds(&union);
    assert!((x0 - 0.0).abs() < 1e-6, "min_x was {x0}");
    assert!((y0 - 0.0).abs() < 1e-6, "min_y was {y0}");
    assert!((x1 - 15.0).abs() < 1e-6, "max_x was {x1}");
    assert!((y1 - 10.0).abs() < 1e-6, "max_y was {y1}");
}

#[test]
fn the_intersection_of_two_overlapping_rectangles_spans_the_overlap() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(5.0, 0.0, 15.0, 10.0);

    let intersection = a.intersection(&b, DEFAULT_TOLERANCE).unwrap();

    assert_eq!(intersection.subpath_count(), 1);
    let (x0, y0, x1, y1) = bounds(&intersection);
    assert!((x0 - 5.0).abs() < 1e-6, "min_x was {x0}");
    assert!((y0 - 0.0).abs() < 1e-6, "min_y was {y0}");
    assert!((x1 - 10.0).abs() < 1e-6, "max_x was {x1}");
    assert!((y1 - 10.0).abs() < 1e-6, "max_y was {y1}");
}

#[test]
fn the_difference_removes_the_second_shape() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(5.0, 0.0, 15.0, 10.0);

    let difference = a.difference(&b, DEFAULT_TOLERANCE).unwrap();

    assert_eq!(difference.subpath_count(), 1);
    let (x0, y0, x1, y1) = bounds(&difference);
    assert!((x0 - 0.0).abs() < 1e-6, "min_x was {x0}");
    assert!((y0 - 0.0).abs() < 1e-6, "min_y was {y0}");
    assert!((x1 - 5.0).abs() < 1e-6, "max_x was {x1}");
    assert!((y1 - 10.0).abs() < 1e-6, "max_y was {y1}");
}

#[test]
fn the_exclusive_or_is_symmetric() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(5.0, 0.0, 15.0, 10.0);

    let ab = a.xor(&b, DEFAULT_TOLERANCE).unwrap();
    let ba = b.xor(&a, DEFAULT_TOLERANCE).unwrap();

    assert_eq!(ab, ba);
}

#[test]
fn the_exclusive_or_covers_both_operands() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(5.0, 0.0, 15.0, 10.0);

    let xor = a.xor(&b, DEFAULT_TOLERANCE).unwrap();
    let union = a.union(&b, DEFAULT_TOLERANCE).unwrap();

    let (x0, _, x1, _) = bounds(&xor);
    let (ux0, _, ux1, _) = bounds(&union);
    assert!((x0 - ux0).abs() < 1e-6);
    assert!((x1 - ux1).abs() < 1e-6);
}

#[test]
fn curves_stay_curves_through_a_boolean_operation() {
    let circle = Path::new(
        Circle::new((0.0, 0.0), 10.0).to_path(1e-6),
        FillRule::NonZero,
    );
    let square = rect(-20.0, -20.0, 20.0, 20.0);

    let intersection = circle.intersection(&square, DEFAULT_TOLERANCE).unwrap();

    let cubics = intersection
        .as_bez_path()
        .elements()
        .iter()
        .filter(|el| matches!(el, kurbo::PathEl::CurveTo(..)))
        .count();
    assert!(
        cubics >= 1,
        "the intersection of a circle with a rectangle lost its curves"
    );
}

/// An empty path: no element at all.
fn empty(rule: FillRule) -> Path {
    Path::new(BezPath::new(), rule)
}

/// A path holding only moves: it has no segment, so it is empty too.
fn moves_only(points: &[(f64, f64)]) -> Path {
    let mut bez = BezPath::new();
    for point in points {
        bez.move_to(*point);
    }
    Path::new(bez, FillRule::NonZero)
}

/// An outer square with a same-wound inner square, read with `EvenOdd`: the
/// inner square is a hole.
fn ring() -> Path {
    let mut bez = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0);
    bez.extend(
        Rect::new(3.0, 3.0, 7.0, 7.0)
            .to_path(0.0)
            .elements()
            .iter()
            .copied(),
    );
    Path::new(bez, FillRule::EvenOdd)
}

fn circle() -> Path {
    Path::new(
        Circle::new((0.0, 0.0), 10.0).to_path(1e-6),
        FillRule::NonZero,
    )
}

fn assert_no_element(path: &Path) {
    assert!(
        path.as_bez_path().elements().is_empty(),
        "expected an empty path, got {path:?}"
    );
}

#[test]
fn the_union_with_an_empty_path_returns_a_clone() {
    let r = rect(0.0, 0.0, 10.0, 10.0);
    let c = circle();
    let e = empty(FillRule::NonZero);

    assert_eq!(r.union(&e, DEFAULT_TOLERANCE), Ok(r.clone()));
    assert_eq!(e.union(&r, DEFAULT_TOLERANCE), Ok(r.clone()));
    assert_eq!(c.union(&e, DEFAULT_TOLERANCE), Ok(c.clone()));
    assert_eq!(e.union(&c, DEFAULT_TOLERANCE), Ok(c.clone()));
}

#[test]
fn the_surviving_operand_keeps_its_own_fill_rule() {
    let g = ring();
    let e = empty(FillRule::NonZero);
    let hole = Point::new(5.0, 5.0);

    for result in [
        e.union(&g, DEFAULT_TOLERANCE),
        g.union(&e, DEFAULT_TOLERANCE),
        e.xor(&g, DEFAULT_TOLERANCE),
        g.difference(&e, DEFAULT_TOLERANCE),
    ] {
        let result = result.unwrap();
        assert_eq!(result, g);
        assert_eq!(result.fill_rule(), FillRule::EvenOdd);
        assert_eq!(
            result.contains(hole, DEFAULT_TOLERANCE),
            Ok(Containment::Outside)
        );
    }
}

#[test]
fn a_moves_only_path_is_empty_in_either_operand_position() {
    let m = moves_only(&[(1.0, 1.0), (4.0, 4.0)]);
    let m2 = moves_only(&[(2.0, 2.0)]);
    let g = ring();

    assert_eq!(m.union(&g, DEFAULT_TOLERANCE), Ok(g.clone()));
    assert_eq!(g.union(&m, DEFAULT_TOLERANCE), Ok(g.clone()));
    assert_eq!(m.xor(&g, DEFAULT_TOLERANCE), Ok(g.clone()));
    assert_eq!(g.difference(&m, DEFAULT_TOLERANCE), Ok(g.clone()));
    assert_no_element(&m.difference(&g, DEFAULT_TOLERANCE).unwrap());
    assert_no_element(&m.intersection(&g, DEFAULT_TOLERANCE).unwrap());
    assert_no_element(&g.intersection(&m, DEFAULT_TOLERANCE).unwrap());
    assert_no_element(&m.union(&m2, DEFAULT_TOLERANCE).unwrap());
}

#[test]
fn a_moves_only_path_covers_no_point() {
    let m = moves_only(&[(1.0, 1.0), (4.0, 4.0)]);

    assert_eq!(
        m.contains(Point::new(1.0, 1.0), DEFAULT_TOLERANCE),
        Ok(Containment::Outside)
    );
}

#[test]
fn difference_and_xor_with_an_empty_path_return_a_clone() {
    let r = rect(0.0, 0.0, 10.0, 10.0);
    let e = empty(FillRule::NonZero);

    assert_eq!(r.difference(&e, DEFAULT_TOLERANCE), Ok(r.clone()));
    assert_eq!(r.xor(&e, DEFAULT_TOLERANCE), Ok(r.clone()));
    assert_eq!(e.xor(&r, DEFAULT_TOLERANCE), Ok(r.clone()));
}

#[test]
fn intersection_with_an_empty_path_is_empty() {
    let r = rect(0.0, 0.0, 10.0, 10.0);
    let e = empty(FillRule::NonZero);

    assert_no_element(&r.intersection(&e, DEFAULT_TOLERANCE).unwrap());
    assert_no_element(&e.intersection(&r, DEFAULT_TOLERANCE).unwrap());
    assert_no_element(&e.difference(&r, DEFAULT_TOLERANCE).unwrap());
}

#[test]
fn every_operation_on_two_empty_paths_is_empty() {
    let e = empty(FillRule::NonZero);

    for result in [
        e.union(&e, DEFAULT_TOLERANCE),
        e.intersection(&e, DEFAULT_TOLERANCE),
        e.difference(&e, DEFAULT_TOLERANCE),
        e.xor(&e, DEFAULT_TOLERANCE),
    ] {
        assert_no_element(&result.unwrap());
    }
}

#[test]
fn an_empty_operand_with_mixed_rules_keeps_the_survivors_rule() {
    let r = rect(0.0, 0.0, 10.0, 10.0);
    let g = ring();
    let e_even_odd = empty(FillRule::EvenOdd);
    let e_non_zero = empty(FillRule::NonZero);

    assert_eq!(e_even_odd.union(&r, DEFAULT_TOLERANCE), Ok(r.clone()));
    assert_eq!(r.xor(&e_even_odd, DEFAULT_TOLERANCE), Ok(r.clone()));
    assert_eq!(r.difference(&e_even_odd, DEFAULT_TOLERANCE), Ok(r.clone()));
    assert_eq!(e_non_zero.union(&g, DEFAULT_TOLERANCE), Ok(g.clone()));
    assert_eq!(g.xor(&e_non_zero, DEFAULT_TOLERANCE), Ok(g.clone()));
}

#[test]
fn validation_runs_before_the_empty_operand_rule() {
    let r = rect(0.0, 0.0, 10.0, 10.0);
    let e = empty(FillRule::NonZero);

    assert!(matches!(
        r.union(&e, 0.0),
        Err(PathOpError::InvalidTolerance { .. })
    ));
}

#[test]
fn a_non_finite_coordinate_is_rejected_before_the_operation() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((f64::NAN, 1.0));
    bez.line_to((1.0, 1.0));
    bez.close_path();
    let broken = Path::new(bez, FillRule::NonZero);
    let square = rect(0.0, 0.0, 10.0, 10.0);

    let result = broken.union(&square, DEFAULT_TOLERANCE);

    assert!(matches!(
        result,
        Err(PathOpError::NonFiniteCoordinate { .. })
    ));
}

#[test]
fn an_open_subpath_is_rejected_before_the_operation() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((5.0, 0.0));
    bez.line_to((2.5, 5.0));
    let open = Path::new(bez, FillRule::NonZero);
    let square = rect(0.0, 0.0, 10.0, 10.0);

    let result = open.union(&square, DEFAULT_TOLERANCE);

    assert_eq!(result, Err(PathOpError::OpenSubpath { subpath: 0 }));
}

#[test]
fn a_self_intersecting_path_is_accepted() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 10.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((0.0, 10.0));
    bez.close_path();
    let bowtie = Path::new(bez, FillRule::EvenOdd);
    let square = rect(0.0, 0.0, 20.0, 20.0);

    let result = bowtie.union(&square, DEFAULT_TOLERANCE);

    assert!(result.is_ok(), "a self-intersecting path was rejected");
}

#[test]
fn a_zero_length_segment_is_accepted() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((0.0, 0.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((10.0, 10.0));
    bez.line_to((0.0, 0.0));
    bez.close_path();
    let path = Path::new(bez, FillRule::NonZero);
    let square = rect(0.0, 0.0, 10.0, 10.0);

    let result = path.intersection(&square, DEFAULT_TOLERANCE);

    assert!(result.is_ok(), "a zero-length segment was rejected");
}

#[test]
fn an_invalid_tolerance_is_rejected() {
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        // The rejection happens before any geometry runs, so the empty paths
        // are enough to observe it.
        let result = Path::new(BezPath::new(), FillRule::NonZero)
            .union(&Path::new(BezPath::new(), FillRule::NonZero), tolerance);
        let expected = match NonFiniteValue::classify(tolerance) {
            Some(value) => value,
            None => NonFiniteValue::Nan,
        };
        assert_eq!(
            result,
            Err(PathOpError::InvalidTolerance { value: expected }),
            "tolerance {tolerance} was not rejected"
        );
    }
}

#[test]
fn an_invalid_tolerance_is_rejected_before_the_geometry_runs() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(5.0, 0.0, 15.0, 10.0);
    let _ = (&a, &b);

    let result = a.union(&b, 0.0);

    assert_eq!(
        result,
        Err(PathOpError::InvalidTolerance {
            value: NonFiniteValue::Nan,
        })
    );
}

#[test]
fn the_same_inputs_give_the_same_result_every_time() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(5.0, 0.0, 15.0, 10.0);

    let first = a.union(&b, DEFAULT_TOLERANCE).unwrap();
    for _ in 0..99 {
        assert_eq!(a.union(&b, DEFAULT_TOLERANCE).unwrap(), first);
    }
}

#[test]
fn the_fill_rule_is_carried_into_the_result() {
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(5.0, 0.0, 15.0, 10.0);

    let union = a.union(&b, DEFAULT_TOLERANCE).unwrap();

    assert_eq!(union.fill_rule(), FillRule::NonZero);
}
