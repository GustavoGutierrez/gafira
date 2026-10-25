//! The public contract of the stroke outlining and the path simplification.

use gafira_pathops::{
    BezPath, Cap, Containment, DEFAULT_TOLERANCE, FillRule, Join, Path, PathOpError, Point,
};
use kurbo::{Rect, Shape};

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

#[test]
fn the_simplification_keeps_the_path_within_the_tolerance() {
    let mut bez = BezPath::new();
    bez.move_to((0.0, 0.0));
    bez.line_to((10.0, 0.0));
    bez.line_to((20.0, 0.5));
    bez.line_to((30.0, 0.0));
    bez.line_to((30.0, 10.0));
    bez.line_to((0.0, 10.0));
    bez.close_path();
    let path = Path::new(bez, FillRule::NonZero);
    let tolerance = 1.0;

    let simplified = path.simplify(tolerance).unwrap();

    // Every corner of the original must stay within the tolerance of the
    // simplified outline.
    for corner in [
        (0.0, 0.0),
        (10.0, 0.0),
        (20.0, 0.5),
        (30.0, 0.0),
        (30.0, 10.0),
    ] {
        let (_, _, _, _) = bounds(&simplified);
        let (_, _, _, _) = bounds(&path);
        let simplified_box = simplified.as_bez_path().bounding_box();
        let original_box = path.as_bez_path().bounding_box();
        // The corner must remain inside the original box grown by the
        // tolerance and inside the simplified box grown by the tolerance.
        assert!(
            corner.0 >= original_box.min_x() - tolerance
                && corner.0 <= original_box.max_x() + tolerance
                && corner.1 >= original_box.min_y() - tolerance
                && corner.1 <= original_box.max_y() + tolerance,
            "the corner {corner:?} left the original box"
        );
        assert!(
            corner.0 >= simplified_box.min_x() - tolerance
                && corner.0 <= simplified_box.max_x() + tolerance
                && corner.1 >= simplified_box.min_y() - tolerance
                && corner.1 <= simplified_box.max_y() + tolerance,
            "the corner {corner:?} left the simplified box"
        );
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
