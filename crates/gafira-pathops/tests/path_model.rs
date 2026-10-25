//! The public contract of the path model.
//!
//! Every test here goes through the public interface of the crate, so the
//! contract is what a consumer sees, not the internals.

use gafira_pathops::{BezPath, FillRule, NonFiniteValue, Path, PathOpError};
use kurbo::{Rect, Shape};

fn closed_square() -> BezPath {
    Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0)
}

fn open_triangle() -> BezPath {
    let mut p = BezPath::new();
    p.move_to((0.0, 0.0));
    p.line_to((5.0, 0.0));
    p.line_to((2.5, 5.0));
    p
}

fn two_subpaths() -> BezPath {
    let mut bez = closed_square();
    bez.extend(open_triangle().elements().iter().copied());
    bez
}

#[test]
fn a_path_carries_its_subpaths_and_fill_rule() {
    let path = Path::new(two_subpaths(), FillRule::NonZero);

    assert_eq!(path.subpath_count(), 2);
    assert!(path.is_subpath_closed(0));
    assert!(!path.is_subpath_closed(1));
    assert_eq!(path.fill_rule(), FillRule::NonZero);
}

#[test]
fn an_out_of_range_subpath_is_not_closed() {
    let path = Path::new(closed_square(), FillRule::EvenOdd);
    assert!(!path.is_subpath_closed(7));
}

#[test]
fn an_empty_path_is_empty() {
    let path = Path::new(BezPath::new(), FillRule::EvenOdd);
    assert!(path.is_empty());
    assert_eq!(path.subpath_count(), 0);
}

#[test]
fn closing_subpaths_closes_the_open_one() {
    let path = Path::new(two_subpaths(), FillRule::EvenOdd);

    let closed = path.close_subpaths();
    assert_eq!(closed.subpath_count(), 2);
    assert!(closed.is_subpath_closed(0));
    assert!(closed.is_subpath_closed(1));
    assert_eq!(closed.fill_rule(), FillRule::EvenOdd);
}

#[test]
fn closing_an_already_closed_path_changes_nothing() {
    let path = Path::new(closed_square(), FillRule::EvenOdd);
    assert_eq!(path.close_subpaths(), path);
}

#[test]
fn closing_does_not_add_elements_to_a_closed_subpath() {
    let path = Path::new(two_subpaths(), FillRule::EvenOdd);

    let closed = path.close_subpaths();

    // The closed square keeps its five elements (a move, three lines and a
    // close) and the open triangle gets exactly one close, so the result has
    // nine elements and no two closes in a row. Closing adds nothing to a
    // subpath that is already closed.
    let elements = closed.as_bez_path().elements();
    assert_eq!(
        elements.len(),
        9,
        "the closed path has {} elements",
        elements.len()
    );
    assert!(
        !elements
            .windows(2)
            .any(|pair| matches!(pair, [kurbo::PathEl::ClosePath, kurbo::PathEl::ClosePath])),
        "closing added a redundant close"
    );
}

#[test]
fn closing_a_path_with_two_open_subpaths_closes_both() {
    let mut bez = open_triangle();
    bez.extend(open_triangle().elements().iter().copied());
    let path = Path::new(bez, FillRule::EvenOdd);
    assert_eq!(path.subpath_count(), 2);
    assert!(!path.is_subpath_closed(0));
    assert!(!path.is_subpath_closed(1));

    let closed = path.close_subpaths();

    assert_eq!(closed.subpath_count(), 2);
    assert!(closed.is_subpath_closed(0));
    assert!(closed.is_subpath_closed(1));
}

#[test]
fn a_non_finite_coordinate_is_rejected() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((f64::NAN, 1.0));
    bez.line_to((1.0, 1.0));
    bez.close_path();
    let path = Path::new(bez, FillRule::EvenOdd);

    assert_eq!(
        path.validate_finite(),
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Nan,
        })
    );
}

#[test]
fn an_infinite_coordinate_is_rejected() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((f64::INFINITY, 1.0));
    bez.line_to((1.0, 1.0));
    bez.close_path();
    let path = Path::new(bez, FillRule::EvenOdd);

    assert_eq!(
        path.validate_finite(),
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Infinity,
        })
    );
}

#[test]
fn a_finite_path_passes_the_finite_check() {
    let path = Path::new(closed_square(), FillRule::EvenOdd);
    assert_eq!(path.validate_finite(), Ok(()));
}

#[test]
fn an_open_subpath_is_rejected_with_its_index() {
    let path = Path::new(open_triangle(), FillRule::EvenOdd);
    assert_eq!(
        path.validate_closed(),
        Err(PathOpError::OpenSubpath { subpath: 0 })
    );
}

#[test]
fn the_second_open_subpath_is_the_one_reported() {
    let path = Path::new(two_subpaths(), FillRule::EvenOdd);
    assert_eq!(
        path.validate_closed(),
        Err(PathOpError::OpenSubpath { subpath: 1 })
    );
}

#[test]
fn a_closed_path_passes_the_closed_check() {
    let path = Path::new(closed_square(), FillRule::EvenOdd);
    assert_eq!(path.validate_closed(), Ok(()));
}

#[test]
fn the_fill_rule_set_is_exactly_two_variants() {
    // The match is exhaustive on purpose: adding a variant breaks the build,
    // which is what pins the set at exactly two.
    let names: Vec<&str> = [FillRule::EvenOdd, FillRule::NonZero]
        .iter()
        .map(|rule| match rule {
            FillRule::EvenOdd => "even-odd",
            FillRule::NonZero => "non-zero",
        })
        .collect();
    assert_eq!(names, ["even-odd", "non-zero"]);
}

#[test]
fn the_points_are_the_geom_points() {
    // The path crate exposes no point of its own: a point it hands out is a
    // `gafira_geom::Point`, so no conversion is needed by a caller.
    fn takes_a_geom_point(_p: gafira_geom::Point) {}
    let point: gafira_geom::Point = gafira_pathops::Point::new(1.0, 2.0);
    takes_a_geom_point(point);

    // And the crate's re-export is the same type.
    let same: gafira_pathops::Point = gafira_geom::Point::new(1.0, 2.0);
    assert_eq!(point, same);
}

#[test]
fn a_non_finite_value_is_classified_by_kind() {
    assert_eq!(
        gafira_pathops::NonFiniteValue::classify(f64::NAN),
        Some(gafira_pathops::NonFiniteValue::Nan)
    );
    assert_eq!(
        gafira_pathops::NonFiniteValue::classify(f64::INFINITY),
        Some(gafira_pathops::NonFiniteValue::Infinity)
    );
    assert_eq!(
        gafira_pathops::NonFiniteValue::classify(f64::NEG_INFINITY),
        Some(gafira_pathops::NonFiniteValue::NegativeInfinity)
    );
    assert_eq!(gafira_pathops::NonFiniteValue::classify(1.5), None);
}

#[test]
fn a_path_of_moves_only_is_empty_and_has_no_open_subpath() {
    let mut bez = BezPath::new();
    bez.move_to((1.0, 1.0));
    bez.move_to((4.0, 4.0));
    let path = Path::new(bez, FillRule::NonZero);

    assert!(path.is_empty());
    assert_eq!(path.validate_closed(), Ok(()));
}

#[test]
fn a_path_with_a_segment_is_not_empty() {
    assert!(!Path::new(closed_square(), FillRule::NonZero).is_empty());
    assert!(!Path::new(open_triangle(), FillRule::NonZero).is_empty());
}
