//! The path simplification.
//!
//! Simplifying a path removes the parts of it that carry no shape, within a
//! tolerance that bounds how far the result may move from the original.

use kurbo::simplify::{SimplifyOptions, simplify_bezpath};

use crate::{NonFiniteValue, Path, PathOpError};

impl Path {
    /// The path with its redundant parts removed.
    ///
    /// The simplified path stays within `tolerance` of the original and never
    /// holds more segments than it did. Every subpath of the original appears
    /// in the result, in the same order.
    pub fn simplify(&self, tolerance: f64) -> Result<Path, PathOpError> {
        validate_tolerance(tolerance)?;
        self.validate_finite()?;

        let simplified = simplify_bezpath(
            self.as_bez_path().elements().to_vec(),
            tolerance,
            &SimplifyOptions::default(),
        );

        Ok(Path::new(simplified, self.fill_rule()))
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
    use crate::FillRule;
    use kurbo::{BezPath, Rect, Shape};

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
    fn the_simplification_keeps_the_subpaths() {
        let mut bez = Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.0);
        let mut second = BezPath::new();
        second.move_to((20.0, 20.0));
        second.line_to((30.0, 20.0));
        second.line_to((30.0, 30.0));
        second.close_path();
        bez.extend(second.elements().iter().copied());
        let path = Path::new(bez, FillRule::NonZero);

        let simplified = path.simplify(1e-3).unwrap();

        assert_eq!(simplified.subpath_count(), 2);
    }
}
