//! The input validation every operation shares: non-finite coordinates, open
//! subpaths, and tolerances or stroke widths that are not usable.

use gafira_pathops::{
    BezPath, Cap, DEFAULT_TOLERANCE, FillRule, InvalidNumber, Join, NonFiniteValue, Path,
    PathOpError, Point, StrokeStyle,
};
use kurbo::{Rect, Shape};

fn rect() -> Path {
    Path::new(
        Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0),
        FillRule::NonZero,
    )
}

/// A closed triangle whose second point has the given x coordinate.
fn closed_with_x(x: f64) -> Path {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((x, 1.0));
    bez.line_to((1.0, 1.0));
    bez.close_path();
    Path::new(bez, FillRule::NonZero)
}

/// An open polyline whose second point has the given x coordinate.
fn open_with_x(x: f64) -> Path {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((x, 1.0));
    bez.line_to((1.0, 1.0));
    Path::new(bez, FillRule::NonZero)
}

fn style(width: f64) -> StrokeStyle {
    StrokeStyle {
        width,
        cap: Cap::Round,
        join: Join::Round,
    }
}

const NON_FINITE: [(f64, NonFiniteValue); 3] = [
    (f64::NAN, NonFiniteValue::Nan),
    (f64::INFINITY, NonFiniteValue::Infinity),
    (f64::NEG_INFINITY, NonFiniteValue::NegativeInfinity),
];

/// The outcome of every operation that takes one path, a tolerance and a
/// stroke width.
fn single_path_calls(
    path: &Path,
    tolerance: f64,
    width: f64,
) -> Vec<(&'static str, Result<(), PathOpError>)> {
    let point = Point::new(0.5, 0.5);
    vec![
        (
            "stroke_outline",
            path.stroke_outline(&style(width), tolerance).map(drop),
        ),
        ("simplify", path.simplify(tolerance).map(drop)),
        ("contains", path.contains(point, tolerance).map(drop)),
        (
            "distance_to_perimeter",
            path.distance_to_perimeter(point, tolerance).map(drop),
        ),
    ]
}

/// The outcome of the four boolean operations over `a` and `b`.
fn boolean_calls(
    a: &Path,
    b: &Path,
    tolerance: f64,
) -> Vec<(&'static str, Result<(), PathOpError>)> {
    vec![
        ("union", a.union(b, tolerance).map(drop)),
        ("intersection", a.intersection(b, tolerance).map(drop)),
        ("difference", a.difference(b, tolerance).map(drop)),
        ("xor", a.xor(b, tolerance).map(drop)),
    ]
}

#[test]
fn a_non_finite_coordinate_is_rejected_by_every_operation() {
    for (x, kind) in NON_FINITE {
        let bad = closed_with_x(x);
        let good = rect();
        let expected = Err(PathOpError::NonFiniteCoordinate { value: kind });

        let mut calls = boolean_calls(&bad, &good, DEFAULT_TOLERANCE);
        calls.extend(boolean_calls(&good, &bad, DEFAULT_TOLERANCE));
        calls.extend(single_path_calls(&bad, DEFAULT_TOLERANCE, 2.0));
        for (name, outcome) in calls {
            assert_eq!(outcome, expected, "{name} with {kind}");
        }
    }
}

#[test]
fn the_simplification_of_a_non_finite_path_returns_an_error() {
    let polyline = open_with_x(f64::NAN);

    assert_eq!(
        polyline.simplify(1e-3).map(drop),
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Nan
        })
    );
}

#[test]
fn the_stroke_of_a_non_finite_path_returns_an_error() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((1.0, f64::INFINITY));
    let line = Path::new(bez, FillRule::NonZero);

    assert_eq!(
        line.stroke_outline(&style(2.0), 1e-3).map(drop),
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Infinity
        })
    );
}

#[test]
fn a_non_finite_coordinate_is_reported_before_an_open_subpath() {
    let open_and_nan = open_with_x(f64::NAN);

    assert_eq!(
        open_and_nan.union(&rect(), DEFAULT_TOLERANCE).map(drop),
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Nan
        })
    );
}

#[test]
fn a_non_finite_second_operand_is_reported_before_an_open_first_one() {
    let open = open_with_x(0.5);
    let infinite = closed_with_x(f64::INFINITY);

    assert_eq!(
        open.intersection(&infinite, DEFAULT_TOLERANCE).map(drop),
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Infinity
        })
    );
}

#[test]
fn an_invalid_tolerance_is_reported_before_the_path_defects() {
    let defective = open_with_x(f64::NAN);

    assert_eq!(
        defective.union(&rect(), 0.0).map(drop),
        Err(PathOpError::InvalidTolerance {
            cause: InvalidNumber::NotPositive(0.0)
        })
    );
}

#[test]
fn an_invalid_width_is_reported_before_the_path_defects() {
    let defective = open_with_x(f64::NAN);

    assert_eq!(
        defective.stroke_outline(&style(0.0), 1e-3).map(drop),
        Err(PathOpError::InvalidStrokeWidth {
            cause: InvalidNumber::NotPositive(0.0)
        })
    );
}

/// Whether the outcome is the error that says `expected` is not positive.
fn is_not_positive(outcome: &Result<(), PathOpError>, tolerance: bool, expected: f64) -> bool {
    let cause = match outcome {
        Err(PathOpError::InvalidTolerance { cause }) if tolerance => cause,
        Err(PathOpError::InvalidStrokeWidth { cause }) if !tolerance => cause,
        _ => return false,
    };
    matches!(cause, InvalidNumber::NotPositive(v) if v.to_bits() == expected.to_bits())
}

#[test]
fn a_non_positive_tolerance_and_width_are_named_as_such() {
    for value in [0.0, -0.0, -1.0] {
        let mut calls = boolean_calls(&rect(), &rect(), value);
        calls.extend(single_path_calls(&rect(), value, 2.0));
        for (name, outcome) in calls {
            assert!(
                is_not_positive(&outcome, true, value),
                "{name} with tolerance {value}: {outcome:?}"
            );
            let message = outcome.unwrap_err().to_string();
            assert!(!message.contains("NaN"), "{name}: {message}");
            assert!(message.contains("not positive"), "{name}: {message}");
            assert!(message.contains(&value.to_string()), "{name}: {message}");
        }

        let outcome = rect().stroke_outline(&style(value), 1e-3).map(drop);
        assert!(
            is_not_positive(&outcome, false, value),
            "width {value}: {outcome:?}"
        );
        let message = outcome.unwrap_err().to_string();
        assert!(!message.contains("NaN"), "width {value}: {message}");
        assert!(message.contains("not positive"), "width {value}: {message}");
        assert!(
            message.contains(&value.to_string()),
            "width {value}: {message}"
        );
    }
}

#[test]
fn a_non_finite_tolerance_and_width_are_named_by_kind() {
    for (value, kind) in NON_FINITE {
        let mut calls = boolean_calls(&rect(), &rect(), value);
        calls.extend(single_path_calls(&rect(), value, 2.0));
        for (name, outcome) in calls {
            assert_eq!(
                outcome,
                Err(PathOpError::InvalidTolerance {
                    cause: InvalidNumber::NonFinite(kind)
                }),
                "{name} with tolerance {kind}"
            );
            let message = outcome.unwrap_err().to_string();
            assert!(message.contains(&kind.to_string()), "{name}: {message}");
        }

        assert_eq!(
            rect().stroke_outline(&style(value), 1e-3).map(drop),
            Err(PathOpError::InvalidStrokeWidth {
                cause: InvalidNumber::NonFinite(kind)
            })
        );
    }
}

#[test]
fn an_error_leaves_no_partial_result_when_the_second_operand_fails() {
    let a = rect();
    let b = closed_with_x(f64::NAN);
    // A NaN is not equal to itself, so the broken operand is compared by text.
    let (a_before, b_before) = (a.clone(), format!("{b:?}"));

    let result = a.union(&b, DEFAULT_TOLERANCE);

    assert_eq!(
        result,
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Nan
        })
    );
    assert_eq!(a, a_before);
    assert_eq!(format!("{b:?}"), b_before);
}

/// A closed triangle followed by two segments drawn after the close, with no new
/// move: the segments form an open subpath.
fn closed_then_drawn_on() -> Path {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((10.0, 10.0));
    bez.close_path();
    bez.line_to((5.0, 20.0));
    bez.line_to((0.0, 20.0));
    Path::new(bez, FillRule::NonZero)
}

#[test]
fn drawing_after_a_close_without_a_move_is_an_open_subpath() {
    let path = closed_then_drawn_on();
    let expected = Err(PathOpError::OpenSubpath { subpath: 1 });

    assert_eq!(path.validate_closed(), expected);
    assert_eq!(
        path.contains(Point::new(1.0, 1.0), DEFAULT_TOLERANCE)
            .map(drop),
        expected
    );
    let mut calls = boolean_calls(&path, &rect(), DEFAULT_TOLERANCE);
    calls.extend(boolean_calls(&rect(), &path, DEFAULT_TOLERANCE));
    for (name, outcome) in calls {
        assert_eq!(outcome, expected, "{name}");
    }
}

#[test]
fn the_stroke_and_the_distance_accept_drawing_after_a_close() {
    let path = closed_then_drawn_on();

    assert!(path.stroke_outline(&style(2.0), DEFAULT_TOLERANCE).is_ok());
    assert!(path.simplify(DEFAULT_TOLERANCE).is_ok());
    assert!(
        path.distance_to_perimeter(Point::new(1.0, 1.0), DEFAULT_TOLERANCE)
            .is_ok()
    );
}

#[test]
fn closing_the_subpaths_closes_the_tail_drawn_after_a_close() {
    let closed = closed_then_drawn_on().close_subpaths();

    assert_eq!(closed.validate_closed(), Ok(()));
    assert_eq!(closed.subpath_count(), 2);
}
