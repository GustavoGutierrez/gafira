//! The public contract of the stroke outlining and the path simplification.

use gafira_pathops::{
    BezPath, Cap, Containment, DEFAULT_TOLERANCE, FillRule, Join, Path, PathOpError, Point,
};
use kurbo::{ParamCurve, Rect, Shape};

fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
    let mut bez = BezPath::new();
    bez.move_to((x0, y0));
    bez.line_to((x1, y1));
    Path::new(bez, FillRule::NonZero)
}

fn open_polyline() -> Path {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((10.0, 10.0));
    Path::new(bez, FillRule::NonZero)
}

fn square() -> Path {
    Path::new(
        Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0),
        FillRule::NonZero,
    )
}

fn style(width: f64) -> gafira_pathops::StrokeStyle {
    gafira_pathops::StrokeStyle {
        width,
        cap: Cap::Round,
        join: Join::Round,
    }
}

fn bounds(path: &Path) -> (f64, f64, f64, f64) {
    let b = path.as_bez_path().bounding_box();
    (b.min_x(), b.min_y(), b.max_x(), b.max_y())
}

#[test]
fn the_outline_of_a_straight_line_is_a_closed_capsule() {
    let outline = line(0.0, 0.0, 100.0, 0.0)
        .stroke_outline(&style(10.0), DEFAULT_TOLERANCE)
        .unwrap();

    assert_eq!(outline.subpath_count(), 1);
    assert!(outline.is_subpath_closed(0));
}

#[test]
fn the_outline_is_wider_than_the_line_by_the_stroke_width() {
    let outline = line(0.0, 0.0, 100.0, 0.0)
        .stroke_outline(&style(10.0), DEFAULT_TOLERANCE)
        .unwrap();

    let (_, y0, _, y1) = bounds(&outline);
    assert!((y0 - -5.0).abs() < 1e-6, "min_y was {y0}");
    assert!((y1 - 5.0).abs() < 1e-6, "max_y was {y1}");
}

#[test]
fn a_wider_stroke_gives_a_wider_outline() {
    let narrow = line(0.0, 0.0, 100.0, 0.0)
        .stroke_outline(&style(4.0), DEFAULT_TOLERANCE)
        .unwrap();
    let wide = line(0.0, 0.0, 100.0, 0.0)
        .stroke_outline(&style(20.0), DEFAULT_TOLERANCE)
        .unwrap();

    let (_, ny0, _, ny1) = bounds(&narrow);
    let (_, wy0, _, wy1) = bounds(&wide);
    assert!(wy1 - wy0 > ny1 - ny0);
}

#[test]
fn the_outline_of_an_open_path_is_closed() {
    let outline = open_polyline()
        .stroke_outline(&style(6.0), DEFAULT_TOLERANCE)
        .unwrap();

    assert!(outline.subpath_count() >= 1);
    for index in 0..outline.subpath_count() {
        assert!(outline.is_subpath_closed(index));
    }
}

#[test]
fn a_redundant_point_is_removed_by_the_simplification() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((20.0, 0.0));
    bez.line_to((20.0, 10.0));
    bez.line_to((0.0, 10.0));
    bez.close_path();
    let path = Path::new(bez, FillRule::NonZero);
    let before = path.as_bez_path().elements().len();

    let simplified = path.simplify(1e-3).unwrap();

    assert!(
        simplified.as_bez_path().elements().len() < before,
        "the simplification removed nothing"
    );
}

#[test]
fn the_simplification_keeps_the_subpath_structure() {
    let mut bez = BezPath::new();
    bez.extend(square().as_bez_path().elements().iter().copied());
    let mut second = BezPath::new();
    second.move_to((20.0, 20.0));
    second.line_to((30.0, 20.0));
    second.line_to((30.0, 30.0));
    second.close_path();
    bez.extend(second.elements().iter().copied());
    let path = Path::new(bez, FillRule::EvenOdd);

    let simplified = path.simplify(1e-3).unwrap();

    assert_eq!(simplified.subpath_count(), 2);
    assert_eq!(simplified.fill_rule(), FillRule::EvenOdd);
}

#[test]
fn an_already_minimal_path_is_unchanged_by_the_simplification() {
    let path = square();

    let simplified = path.simplify(1e-3).unwrap();

    assert_eq!(simplified, path);
}

/// The distance from a point to the closest point of a path.
fn distance_to(path: &Path, point: Point) -> f64 {
    path.distance_to_perimeter(point, 1e-9).unwrap()
}

#[test]
fn a_simplified_polyline_with_a_corner_stays_within_the_tolerance() {
    let polyline = open_polyline();
    let tolerance = 1e-3;

    let simplified = polyline.simplify(tolerance).unwrap();

    // Every point sampled along the original, whatever the segment count of
    // the result.
    for segment in polyline.as_bez_path().segments() {
        for step in 0..=10 {
            let point = segment.eval(f64::from(step) / 10.0);
            let distance = distance_to(&simplified, point);
            assert!(
                distance <= tolerance,
                "{point:?} moved {distance} away from the simplified path"
            );
        }
    }
}

#[test]
fn a_removed_vertex_stays_within_the_tolerance_of_the_output() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((20.0, 0.0));
    bez.line_to((20.0, 10.0));
    bez.line_to((0.0, 10.0));
    bez.close_path();
    let path = Path::new(bez, FillRule::NonZero);

    let simplified = path.simplify(1e-3).unwrap();

    for vertex in [
        (0.0, 0.0),
        (10.0, 0.0),
        (20.0, 0.0),
        (20.0, 10.0),
        (0.0, 10.0),
    ] {
        let distance = distance_to(&simplified, Point::new(vertex.0, vertex.1));
        assert!(distance <= 1e-3, "{vertex:?} is {distance} from the output");
    }
}

#[test]
fn the_outline_of_a_line_is_half_the_stroke_width_away() {
    let outline = line(0.0, 0.0, 100.0, 0.0)
        .stroke_outline(&style(10.0), 1e-3)
        .unwrap();

    let distance = distance_to(&outline, Point::new(50.0, 0.0));

    assert!(
        (distance - 5.0).abs() <= 1e-3,
        "the distance was {distance}"
    );
}

#[test]
fn the_outline_of_a_capsule_spans_the_line_and_the_caps() {
    let outline = line(0.0, 0.0, 100.0, 0.0)
        .stroke_outline(&style(10.0), 1e-3)
        .unwrap();

    let (x0, y0, x1, y1) = bounds(&outline);
    for (value, expected) in [(x0, -5.0), (y0, -5.0), (x1, 105.0), (y1, 5.0)] {
        assert!(
            (value - expected).abs() <= 1e-3,
            "{value} is not {expected}"
        );
    }
}

#[test]
fn every_operation_takes_the_tolerance_as_an_argument() {
    let rect = square();
    let circle = Path::new(
        kurbo::Circle::new((5.0, 5.0), 4.0).to_path(1e-6),
        FillRule::NonZero,
    );
    let point = Point::new(5.0, 5.0);

    for tolerance in [0.5, DEFAULT_TOLERANCE] {
        assert!(rect.union(&circle, tolerance).is_ok());
        assert!(rect.intersection(&circle, tolerance).is_ok());
        assert!(rect.difference(&circle, tolerance).is_ok());
        assert!(rect.xor(&circle, tolerance).is_ok());
        assert!(circle.stroke_outline(&style(2.0), tolerance).is_ok());
        assert!(circle.simplify(tolerance).is_ok());
        assert!(circle.contains(point, tolerance).is_ok());
        assert!(circle.distance_to_perimeter(point, tolerance).is_ok());
    }
}

#[test]
fn an_invalid_tolerance_is_rejected_by_the_simplification() {
    let result = square().simplify(0.0);
    assert!(matches!(result, Err(PathOpError::InvalidTolerance { .. })));
}

fn with_rule(path: Path, rule: FillRule) -> Path {
    Path::new(path.as_bez_path().clone(), rule)
}

#[test]
fn the_outline_carries_non_zero_whatever_the_input_rule() {
    let line = with_rule(line(0.0, 0.0, 100.0, 0.0), FillRule::EvenOdd);

    let outline = line
        .stroke_outline(&style(10.0), DEFAULT_TOLERANCE)
        .unwrap();

    assert_eq!(outline.fill_rule(), FillRule::NonZero);
}

#[test]
fn crossing_strokes_stay_solid() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 10.0));
    bez.move_to((0.0, 10.0));
    bez.line_to((10.0, 0.0));
    let cross = Path::new(bez, FillRule::EvenOdd);

    let outline = cross
        .stroke_outline(&style(2.0), DEFAULT_TOLERANCE)
        .unwrap();

    assert_eq!(
        outline.contains(Point::new(5.0, 5.0), DEFAULT_TOLERANCE),
        Ok(Containment::Inside)
    );
}

#[test]
fn a_self_crossing_polyline_stays_solid() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 10.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((0.0, 10.0));
    let zigzag = Path::new(bez, FillRule::EvenOdd);

    let outline = zigzag
        .stroke_outline(&style(2.0), DEFAULT_TOLERANCE)
        .unwrap();

    assert_eq!(
        outline.contains(Point::new(5.0, 5.0), DEFAULT_TOLERANCE),
        Ok(Containment::Inside)
    );
}
