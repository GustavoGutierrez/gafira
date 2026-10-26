//! The minimum tolerance of the operations that hang or abort below one:
//! the boolean operations, the simplification and the stroke outline.

use gafira_pathops::{BezPath, FillRule, Path, PathOpError, Point};

fn open_polyline(points: &[(f64, f64)]) -> Path {
    let mut bez = BezPath::new();
    bez.move_to(points[0]);
    for p in &points[1..] {
        bez.line_to(*p);
    }
    Path::new(bez, FillRule::NonZero)
}

fn too_fine(minimum: f64) -> Result<(), PathOpError> {
    Err(PathOpError::ToleranceTooFine { minimum })
}

#[test]
fn a_simplification_tolerance_below_the_minimum_is_rejected_and_never_hangs() {
    let base = 1e7;
    let line = open_polyline(&[
        (base, base),
        (base + 1.0, base + 1.0),
        (base + 2.0, base + 2.0),
    ]);
    let minimum: f64 = 1e-12 * (base + 2.0);

    assert_eq!(line.simplify(1e-9).map(drop), too_fine(minimum));
    assert_eq!(
        line.simplify(minimum.next_down()).map(drop),
        too_fine(minimum)
    );

    let simplified = line.simplify(minimum).unwrap();
    for (x, y) in [
        (base, base),
        (base + 1.0, base + 1.0),
        (base + 2.0, base + 2.0),
    ] {
        let distance = simplified
            .distance_to_perimeter(Point::new(x, y), 1e-12)
            .unwrap();
        assert!(distance <= minimum, "vertex ({x}, {y}) moved {distance}");
    }
}

#[test]
fn the_simplification_accepts_exactly_the_minimum_tolerance() {
    // A cheap case that returns even without the rule: the boundary itself.
    let line = open_polyline(&[(0.0, 0.0), (3.0, 1.0), (-4.0, 2.0)]);
    let minimum: f64 = 1e-12 * 4.0;

    assert_eq!(
        line.simplify(minimum.next_down()).map(drop),
        too_fine(minimum)
    );
    assert!(line.simplify(minimum).is_ok());
}

#[test]
fn the_simplification_of_an_empty_or_zero_path_accepts_any_tolerance() {
    let mut moves = BezPath::new();
    moves.move_to((0.0, 0.0));
    let at_origin = open_polyline(&[(0.0, 0.0), (0.0, 0.0)]);

    for path in [
        Path::new(BezPath::new(), FillRule::NonZero),
        Path::new(moves, FillRule::NonZero),
        at_origin,
    ] {
        assert!(path.simplify(f64::MIN_POSITIVE).is_ok());
    }
}

#[test]
fn a_too_fine_simplification_tolerance_is_reported_after_a_coordinate_beyond_the_limit() {
    let line = open_polyline(&[(0.0, 0.0), (2e9, 0.0), (5.0, 5.0)]);

    assert_eq!(
        line.simplify(1e-15).map(drop),
        Err(PathOpError::CoordinateOutOfRange {
            value: 2e9,
            limit: 1e9
        })
    );
}
