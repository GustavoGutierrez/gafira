//! The path simplification.
//!
//! Simplifying a path removes the parts of it that carry no shape, within a
//! tolerance that bounds how far the result may move from the original.

use kurbo::simplify::{SimplifyOptions, simplify_bezpath};

use crate::{Path, PathOpError, validate};

/// The smallest tolerance the simplification accepts, per unit of the largest
/// absolute coordinate of the path.
///
/// Below it the simplification can run without end, on curved paths and, a
/// little further down, on straight ones. The bound is empirical: no hang was
/// observed at or above it.
const MINIMUM_TOLERANCE_PER_UNIT: f64 = 1e-12;

impl Path {
    /// The path with its redundant parts removed.
    ///
    /// The simplified path stays within `tolerance` of the original. The
    /// guarantee is about shape, not about the segment count: straight segments
    /// may be replaced by fitted curves, and the result may hold more segments
    /// than the original. Every subpath of the original appears in the result,
    /// in the same order, with the same fill rule. A closed path of straight
    /// segments whose every vertex lies farther than `tolerance` from the line
    /// through its neighbours comes back equal in value.
    ///
    /// # Errors
    ///
    /// A path with a coordinate whose absolute value is above
    /// [`crate::COORDINATE_LIMIT`] (`1e9`) is rejected with
    /// [`PathOpError::CoordinateOutOfRange`]. A tolerance below `1e-12 * m`,
    /// where `m` is the largest absolute coordinate of the path, is rejected
    /// with [`PathOpError::ToleranceTooFine`]; a tolerance equal to the minimum
    /// is accepted, and an empty path has no minimum.
    pub fn simplify(&self, tolerance: f64) -> Result<Path, PathOpError> {
        validate::tolerance(tolerance)?;
        self.validate_finite()?;
        self.validate_in_range()?;
        validate::not_too_fine(
            tolerance,
            MINIMUM_TOLERANCE_PER_UNIT,
            self.max_abs_coordinate(),
        )?;

        let simplified = simplify_bezpath(
            self.as_bez_path().elements().to_vec(),
            tolerance,
            &SimplifyOptions::default(),
        );

        Ok(Path::new(simplified, self.fill_rule()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FillRule;
    use kurbo::{BezPath, Rect, Shape};

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
