//! Property tests for the stroke outlining and the path simplification.

use gafira_pathops::{
    BezPath, Cap, DEFAULT_TOLERANCE, FillRule, Join, Path, PathOpError, StrokeStyle,
};
use kurbo::{ParamCurve, ParamCurveNearest, Point, Shape};
use proptest::prelude::*;

/// Builds a path from a polyline of the given points.
fn polyline(points: &[(f64, f64)]) -> Path {
    let mut bez = BezPath::new();
    for (index, (x, y)) in points.iter().enumerate() {
        if index == 0 {
            bez.move_to((*x, *y));
        } else {
            bez.line_to((*x, *y));
        }
    }
    Path::new(bez, FillRule::NonZero)
}

/// The distance from a point to the closest point of a path.
fn distance_to_path(path: &Path, point: Point) -> f64 {
    let mut best = f64::INFINITY;
    for segment in path.as_bez_path().segments() {
        let near = segment.nearest(point, 1e-6);
        best = best.min(segment.eval(near.t).distance(point));
    }
    best
}

/// The points a polyline visits.
fn polyline_strategy() -> impl Strategy<Value = Vec<(f64, f64)>> {
    prop::collection::vec((0.0..200.0, 0.0..200.0), 2..12)
}

proptest! {
    /// The simplified path keeps the subpath structure of the original.
    #[test]
    fn the_simplification_keeps_the_subpath_structure(
        points in polyline_strategy(),
        tolerance in 1e-4..1.0,
    ) {
        let path = polyline(&points);

        let simplified = path.simplify(tolerance).unwrap();

        prop_assert_eq!(simplified.subpath_count(), path.subpath_count());
    }

    /// Every point of the original path stays within the tolerance of the
    /// simplified path.
    #[test]
    fn the_simplification_stays_within_the_tolerance(
        points in polyline_strategy(),
        tolerance in 1e-2..1.0,
    ) {
        let path = polyline(&points);

        let simplified = path.simplify(tolerance).unwrap();

        for (x, y) in &points {
            let distance = distance_to_path(&simplified, Point::new(*x, *y));
            prop_assert!(
                distance <= tolerance + 1e-6,
                "a point moved {distance} away, over the tolerance {tolerance}"
            );
        }
    }

    /// The stroke outline covers the path: the outline reaches half the stroke
    /// width beyond the path on every side.
    #[test]
    fn the_outline_covers_the_path(
        points in prop::collection::vec((0.0..200.0, 0.0..200.0), 2..6),
        width in 1.0..20.0,
    ) {
        let path = polyline(&points);
        let style = StrokeStyle {
            width,
            cap: Cap::Round,
            join: Join::Round,
        };

        let outline = path.stroke_outline(&style, DEFAULT_TOLERANCE).unwrap();

        // A path with no segment covers nothing, so its outline is empty.
        if outline.is_empty() {
            return Ok(());
        }

        let path_box = path.as_bez_path().bounding_box();
        let outline_box = outline.as_bez_path().bounding_box();
        let half = width / 2.0;
        // A round cap reaches half the width past the end of the path, so the
        // outline reaches at least that far on every side.
        prop_assert!(
            outline_box.min_x() <= path_box.min_x() - half + 1e-2
                && outline_box.min_y() <= path_box.min_y() - half + 1e-2
                && outline_box.max_x() >= path_box.max_x() + half - 1e-2
                && outline_box.max_y() >= path_box.max_y() + half - 1e-2,
            "the outline {outline_box:?} does not cover the path {path_box:?} by {half}"
        );
    }

    /// A stroke of an invalid width never produces an outline.
    #[test]
    fn an_invalid_stroke_width_produces_no_outline(
        points in polyline_strategy(),
        width in prop::sample::select(vec![0.0, -1.0, f64::NAN, f64::INFINITY]),
    ) {
        let path = polyline(&points);
        let style = StrokeStyle {
            width,
            cap: Cap::Round,
            join: Join::Round,
        };

        let result = path.stroke_outline(&style, DEFAULT_TOLERANCE);

        prop_assert!(matches!(result, Err(PathOpError::InvalidStrokeWidth { .. })), "the invalid width was not rejected");
    }
}
