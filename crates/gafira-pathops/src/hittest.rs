//! The geometric hit testing.
//!
//! The containment test uses the winding number of the path around the point,
//! so it answers with the same fill rule the boolean operations use.

use kurbo::{ParamCurve, ParamCurveNearest, Point, Shape};

use crate::{FillRule, NonFiniteValue, Path, PathOpError};

/// Where a point sits relative to a path.
///
/// The boundary is an answer of its own, so a caller can tell an edge hit
/// from a fill hit without guessing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Containment {
    /// The path covers the point.
    Inside,
    /// The path does not cover the point.
    Outside,
    /// The point lies on the perimeter of the path.
    OnBoundary,
}

impl Path {
    /// Whether the path covers the point, under the fill rule of the path.
    ///
    /// A point that lies on the perimeter is reported as
    /// [`Containment::OnBoundary`], so a caller can tell an edge hit from a
    /// fill hit.
    pub fn contains(&self, point: Point, tolerance: f64) -> Result<Containment, PathOpError> {
        validate_tolerance(tolerance)?;
        self.validate_finite()?;
        self.validate_closed()?;

        if self.is_empty() {
            return Ok(Containment::Outside);
        }

        let distance = self.distance_to_perimeter(point, tolerance)?;
        if distance <= tolerance {
            return Ok(Containment::OnBoundary);
        }

        let winding = self.as_bez_path().winding(point);
        let inside = match self.fill_rule() {
            FillRule::EvenOdd => winding % 2 != 0,
            FillRule::NonZero => winding != 0,
        };
        Ok(if inside {
            Containment::Inside
        } else {
            Containment::Outside
        })
    }

    /// The distance from the point to the perimeter of the path.
    ///
    /// The distance is zero on the perimeter and grows as the point moves
    /// away, in either direction. A caller that hit tests a stroke uses it to
    /// decide whether the point is within the stroke's width.
    pub fn distance_to_perimeter(&self, point: Point, tolerance: f64) -> Result<f64, PathOpError> {
        validate_tolerance(tolerance)?;
        self.validate_finite()?;

        let mut closest = f64::INFINITY;
        for segment in self.as_bez_path().path_segments(tolerance) {
            let near = segment.nearest(point, tolerance);
            let distance = segment.eval(near.t).distance(point);
            if distance < closest {
                closest = distance;
            }
        }
        Ok(closest)
    }
}

/// Checks that the tolerance is a finite positive number.
fn validate_tolerance(tolerance: f64) -> Result<(), PathOpError> {
    if tolerance.is_finite() && tolerance > 0.0 {
        Ok(())
    } else if let Some(value) = NonFiniteValue::classify(tolerance) {
        Err(PathOpError::InvalidTolerance { value })
    } else {
        Err(PathOpError::InvalidTolerance {
            value: NonFiniteValue::Nan,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Circle, Rect};

    #[test]
    fn a_valid_tolerance_passes_the_check() {
        assert_eq!(validate_tolerance(1e-3), Ok(()));
    }

    #[test]
    fn an_invalid_tolerance_is_rejected() {
        assert!(matches!(
            validate_tolerance(0.0),
            Err(PathOpError::InvalidTolerance { .. })
        ));
    }

    #[test]
    fn the_winding_number_of_a_square_is_one_inside() {
        let path = Path::new(
            Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0),
            FillRule::NonZero,
        );
        assert_eq!(path.as_bez_path().winding(Point::new(5.0, 5.0)), 1);
        assert_eq!(path.as_bez_path().winding(Point::new(15.0, 5.0)), 0);
    }

    #[test]
    fn the_distance_to_an_empty_path_is_not_finite() {
        let path = Path::new(kurbo::BezPath::new(), FillRule::NonZero);
        let distance = path
            .distance_to_perimeter(Point::new(0.0, 0.0), 1e-3)
            .unwrap();
        assert!(distance.is_infinite());
    }

    #[test]
    fn the_distance_to_a_circle_is_the_gap() {
        let path = Path::new(
            Circle::new((0.0, 0.0), 10.0).to_path(1e-6),
            FillRule::NonZero,
        );
        let distance = path
            .distance_to_perimeter(Point::new(12.0, 0.0), 1e-3)
            .unwrap();
        assert!((distance - 2.0).abs() < 1e-3, "the distance was {distance}");
    }
}
