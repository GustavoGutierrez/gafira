//! The path simplification.
//!
//! Simplifying a path removes the parts of it that carry no shape, within a
//! tolerance that bounds how far the result may move from the original.

use kurbo::simplify::{SimplifyOptions, simplify_bezpath};

use crate::{Path, PathOpError, validate};

impl Path {
    /// The path with its redundant parts removed.
    ///
    /// The simplified path stays within `tolerance` of the original and never
    /// holds more segments than it did. Every subpath of the original appears
    /// in the result, in the same order.
    pub fn simplify(&self, tolerance: f64) -> Result<Path, PathOpError> {
        validate::tolerance(tolerance)?;
        self.validate_finite()?;

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
