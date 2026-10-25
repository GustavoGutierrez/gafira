//! The public contract of the geometric hit testing.

use gafira_pathops::{BezPath, Containment, DEFAULT_TOLERANCE, FillRule, Path, PathOpError, Point};
use kurbo::{Circle, Rect, Shape};

fn square() -> Path {
    Path::new(
        Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0),
        FillRule::NonZero,
    )
}

fn circle(radius: f64) -> Path {
    Path::new(
        Circle::new((0.0, 0.0), radius).to_path(1e-6),
        FillRule::NonZero,
    )
}

/// Two nested squares, both wound in the same direction.
fn nested_squares() -> Path {
    let mut bez = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0);
    bez.extend(
        Rect::new(3.0, 3.0, 7.0, 7.0)
            .to_path(0.0)
            .elements()
            .iter()
            .copied(),
    );
    Path::new(bez, FillRule::NonZero)
}

#[test]
fn a_point_inside_a_rectangle_is_inside() {
    let answer = square()
        .contains(Point::new(5.0, 5.0), DEFAULT_TOLERANCE)
        .unwrap();
    assert_eq!(answer, Containment::Inside);
}

#[test]
fn a_point_outside_a_rectangle_is_outside() {
    let answer = square()
        .contains(Point::new(15.0, 5.0), DEFAULT_TOLERANCE)
        .unwrap();
    assert_eq!(answer, Containment::Outside);
}

#[test]
fn a_point_on_the_perimeter_is_on_the_boundary() {
    let answer = square()
        .contains(Point::new(0.0, 5.0), DEFAULT_TOLERANCE)
        .unwrap();
    assert_eq!(answer, Containment::OnBoundary);
}

#[test]
fn the_fill_rule_changes_the_answer() {
    let path = nested_squares();
    let point = Point::new(5.0, 5.0);

    let even_odd = Path::new(path.as_bez_path().clone(), FillRule::EvenOdd)
        .contains(point, DEFAULT_TOLERANCE)
        .unwrap();
    let non_zero = path.contains(point, DEFAULT_TOLERANCE).unwrap();

    assert_eq!(even_odd, Containment::Outside);
    assert_eq!(non_zero, Containment::Inside);
}

#[test]
fn the_distance_to_the_perimeter_of_a_circle_is_the_gap() {
    let distance = circle(10.0)
        .distance_to_perimeter(Point::new(12.0, 0.0), DEFAULT_TOLERANCE)
        .unwrap();
    assert!((distance - 2.0).abs() < 1e-3, "the distance was {distance}");
}

#[test]
fn the_distance_from_the_centre_of_a_circle_is_the_radius() {
    let distance = circle(10.0)
        .distance_to_perimeter(Point::new(0.0, 0.0), DEFAULT_TOLERANCE)
        .unwrap();
    assert!(
        (distance - 10.0).abs() < 1e-3,
        "the distance was {distance}"
    );
}

#[test]
fn the_distance_to_the_perimeter_of_a_rectangle() {
    let distance = square()
        .distance_to_perimeter(Point::new(5.0, 5.0), DEFAULT_TOLERANCE)
        .unwrap();
    assert!((distance - 5.0).abs() < 1e-3, "the distance was {distance}");
}

#[test]
fn an_invalid_tolerance_is_rejected_by_the_containment_test() {
    let result = square().contains(Point::new(5.0, 5.0), 0.0);
    assert!(matches!(result, Err(PathOpError::InvalidTolerance { .. })));
}

#[test]
fn an_invalid_tolerance_is_rejected_by_the_distance_test() {
    let result = square().distance_to_perimeter(Point::new(5.0, 5.0), 0.0);
    assert!(matches!(result, Err(PathOpError::InvalidTolerance { .. })));
}

#[test]
fn a_path_with_a_non_finite_coordinate_is_rejected() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((f64::NAN, 1.0));
    bez.line_to((1.0, 1.0));
    bez.close_path();
    let path = Path::new(bez, FillRule::NonZero);

    let result = path.contains(Point::new(0.5, 0.5), DEFAULT_TOLERANCE);

    assert!(matches!(
        result,
        Err(PathOpError::NonFiniteCoordinate { .. })
    ));
}

#[test]
fn an_empty_path_covers_no_point() {
    let path = Path::new(BezPath::new(), FillRule::NonZero);
    let answer = path
        .contains(Point::new(0.0, 0.0), DEFAULT_TOLERANCE)
        .unwrap();
    assert_eq!(answer, Containment::Outside);
}
