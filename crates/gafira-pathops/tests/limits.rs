//! The coordinate limit: every operation rejects a path with a coordinate whose
//! absolute value is above `COORDINATE_LIMIT`, before it computes anything.

use gafira_pathops::{
    BezPath, COORDINATE_LIMIT, Cap, DEFAULT_TOLERANCE, FillRule, Join, NonFiniteValue, Path,
    PathOpError, Point, StrokeStyle,
};
use kurbo::{Rect, Shape};

fn rect() -> Path {
    Path::new(
        Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0),
        FillRule::NonZero,
    )
}

fn style() -> StrokeStyle {
    StrokeStyle {
        width: 10.0,
        cap: Cap::Round,
        join: Join::Round,
    }
}

/// Where the offending point sits in the path.
#[derive(Clone, Copy, Debug)]
enum Slot {
    EndPoint,
    ControlPoint,
    Move,
}

/// A closed path that is valid except for the point `(x, y)`, placed in `slot`.
fn path_with(slot: Slot, x: f64, y: f64) -> Path {
    let mut bez = BezPath::new();
    match slot {
        Slot::EndPoint => {
            bez.move_to((0.0, 0.0));
            bez.line_to((x, y));
            bez.line_to((10.0, 10.0));
        }
        Slot::ControlPoint => {
            bez.move_to((0.0, 0.0));
            bez.curve_to((x, y), (5.0, 5.0), (10.0, 0.0));
            bez.line_to((10.0, 10.0));
        }
        Slot::Move => {
            bez.move_to((x, y));
            bez.line_to((10.0, 0.0));
            bez.line_to((10.0, 10.0));
        }
    }
    bez.close_path();
    Path::new(bez, FillRule::NonZero)
}

const SLOTS: [Slot; 3] = [Slot::EndPoint, Slot::ControlPoint, Slot::Move];

/// Every operation over `bad` (in both operand positions for the booleans), by
/// name.
fn every_operation(bad: &Path, point: Point) -> Vec<(&'static str, Result<(), PathOpError>)> {
    let t = DEFAULT_TOLERANCE;
    let good = rect();
    vec![
        ("union a", bad.union(&good, t).map(drop)),
        ("union b", good.union(bad, t).map(drop)),
        ("intersection a", bad.intersection(&good, t).map(drop)),
        ("intersection b", good.intersection(bad, t).map(drop)),
        ("difference a", bad.difference(&good, t).map(drop)),
        ("difference b", good.difference(bad, t).map(drop)),
        ("xor a", bad.xor(&good, t).map(drop)),
        ("xor b", good.xor(bad, t).map(drop)),
        ("stroke_outline", bad.stroke_outline(&style(), t).map(drop)),
        ("simplify", bad.simplify(t).map(drop)),
        ("contains", bad.contains(point, t).map(drop)),
        (
            "distance_to_perimeter",
            bad.distance_to_perimeter(point, t).map(drop),
        ),
    ]
}

/// Whether the outcome is the out-of-range error for exactly `value`.
fn is_out_of_range(outcome: &Result<(), PathOpError>, value: f64) -> bool {
    matches!(
        outcome,
        Err(PathOpError::CoordinateOutOfRange { value: v, limit })
            if v.to_bits() == value.to_bits() && *limit == COORDINATE_LIMIT
    )
}

#[test]
fn the_limit_is_one_billion() {
    assert_eq!(COORDINATE_LIMIT, 1e9);
}

#[test]
fn a_coordinate_of_exactly_the_limit_is_accepted() {
    let big = Path::new(
        Rect::new(-1e9, -1e9, 1e9, 1e9).to_path(0.0),
        FillRule::NonZero,
    );
    let t = DEFAULT_TOLERANCE;
    // The boolean minimum at the limit is `1e-11 x 1e9`, the others use `t`.
    let boolean_t = 1e-11 * 1e9;

    assert!(big.union(&rect(), boolean_t).is_ok());
    assert!(rect().union(&big, boolean_t).is_ok());
    assert!(big.intersection(&rect(), boolean_t).is_ok());
    assert!(rect().intersection(&big, boolean_t).is_ok());
    assert!(big.difference(&rect(), boolean_t).is_ok());
    assert!(rect().difference(&big, boolean_t).is_ok());
    assert!(big.xor(&rect(), boolean_t).is_ok());
    assert!(rect().xor(&big, boolean_t).is_ok());
    assert!(big.stroke_outline(&style(), t).is_ok());
    assert!(big.simplify(t).is_ok());
    assert!(big.contains(Point::new(0.0, 0.0), t).is_ok());
    assert!(big.distance_to_perimeter(Point::new(0.0, 0.0), t).is_ok());
}

#[test]
fn a_coordinate_beyond_the_limit_is_rejected_by_every_operation() {
    for slot in SLOTS {
        for sign in [1.0, -1.0] {
            let value = sign * COORDINATE_LIMIT.next_up();
            for (x, y) in [(value, 1.0), (1.0, value)] {
                let bad = path_with(slot, x, y);
                for (name, outcome) in every_operation(&bad, Point::new(0.0, 0.0)) {
                    assert!(
                        is_out_of_range(&outcome, value),
                        "{name} with {slot:?} at ({x}, {y}): {outcome:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_first_offending_coordinate_in_path_order_is_reported() {
    // x before y within a point, and the earlier point before the later one.
    let both = path_with(Slot::EndPoint, 3e9, 4e9);
    let later = {
        let mut bez = BezPath::new();
        bez.move_to((0.0, 0.0));
        bez.line_to((5e9, 0.0));
        bez.line_to((6e9, 0.0));
        bez.close_path();
        Path::new(bez, FillRule::NonZero)
    };

    assert!(is_out_of_range(
        &both.simplify(DEFAULT_TOLERANCE).map(drop),
        3e9
    ));
    assert!(is_out_of_range(
        &later.simplify(DEFAULT_TOLERANCE).map(drop),
        5e9
    ));
}

#[test]
fn the_overflow_cases_return_an_error_instead_of_hanging() {
    let mut polyline = BezPath::new();
    polyline.move_to((0.0, 0.0));
    polyline.line_to((1e154, 1e154));
    polyline.line_to((2e154, 0.0));
    let mut line = BezPath::new();
    line.move_to((0.0, 0.0));
    line.line_to((1e155, 1e155));

    let simplified = Path::new(polyline, FillRule::NonZero).simplify(1e-3);
    let distance =
        Path::new(line, FillRule::NonZero).distance_to_perimeter(Point::new(1.0, 0.0), 1e-3);

    assert!(is_out_of_range(&simplified.map(drop), 1e154));
    assert!(is_out_of_range(&distance.map(drop), 1e155));
}

#[test]
fn a_non_finite_coordinate_is_reported_before_one_beyond_the_limit() {
    let mut polyline = BezPath::new();
    polyline.move_to((0.0, 0.0));
    polyline.line_to((1e10, 0.0));
    polyline.line_to((f64::NAN, 0.0));
    let first = path_with(Slot::EndPoint, 1e10, 1.0);
    let second = path_with(Slot::EndPoint, f64::INFINITY, 1.0);

    assert_eq!(
        Path::new(polyline, FillRule::NonZero)
            .simplify(DEFAULT_TOLERANCE)
            .map(drop),
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Nan
        })
    );
    assert_eq!(
        first.union(&second, DEFAULT_TOLERANCE).map(drop),
        Err(PathOpError::NonFiniteCoordinate {
            value: NonFiniteValue::Infinity
        })
    );
}

#[test]
fn a_coordinate_beyond_the_limit_is_reported_before_an_open_subpath() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((1e10, 0.0));
    bez.line_to((5.0, 5.0));
    let open = Path::new(bez, FillRule::NonZero);

    assert!(is_out_of_range(
        &open.union(&rect(), DEFAULT_TOLERANCE).map(drop),
        1e10
    ));
}

#[test]
fn a_moves_only_operand_beyond_the_limit_is_rejected() {
    let mut bez = BezPath::new();
    bez.move_to((2e9, 0.0));
    let moves = Path::new(bez, FillRule::NonZero);

    assert!(is_out_of_range(
        &moves.union(&rect(), DEFAULT_TOLERANCE).map(drop),
        2e9
    ));
    assert!(is_out_of_range(
        &rect().union(&moves, DEFAULT_TOLERANCE).map(drop),
        2e9
    ));
}
