//! The path boolean operations.
//!
//! The four set operations are computed over the Bézier paths themselves, so a
//! curved input stays curved through the operation.

use kurbo::BezPath;
use linesweeper::FillRule as LibFillRule;
use linesweeper::topology::{BinaryWindingNumber, Topology};

use crate::{BooleanOp, FillRule, NonFiniteValue, Path, PathOpError};

/// The smallest tolerance a boolean operation accepts, per unit of the largest
/// absolute coordinate of its operands.
///
/// Below it the boolean library recurses without limit on curved operands: a
/// release build overflows its stack and aborts, a debug build panics. The
/// bound is empirical: no failure was observed at or above it.
const MINIMUM_TOLERANCE_PER_UNIT: f64 = 1e-12;

impl Path {
    /// The set of points covered by both paths.
    ///
    /// The tolerance bounds the deviation between the result and the exact
    /// geometric answer.
    pub fn union(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError> {
        self.boolean_op(other, BooleanOp::Union, tolerance)
    }

    /// The set of points covered by both paths.
    ///
    /// The tolerance bounds the deviation between the result and the exact
    /// geometric answer.
    pub fn intersection(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError> {
        self.boolean_op(other, BooleanOp::Intersection, tolerance)
    }

    /// The set of points covered by the first path and not by the second.
    ///
    /// The tolerance bounds the deviation between the result and the exact
    /// geometric answer.
    pub fn difference(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError> {
        self.boolean_op(other, BooleanOp::Difference, tolerance)
    }

    /// The set of points covered by exactly one of the two paths.
    ///
    /// The tolerance bounds the deviation between the result and the exact
    /// geometric answer.
    pub fn xor(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError> {
        self.boolean_op(other, BooleanOp::Xor, tolerance)
    }

    /// Computes a set operation over the two paths.
    ///
    /// Each path is read with its own fill rule. The result carries the
    /// `NonZero` fill rule.
    pub fn boolean_op(
        &self,
        other: &Path,
        op: BooleanOp,
        tolerance: f64,
    ) -> Result<Path, PathOpError> {
        validate_tolerance(tolerance)?;
        self.validate_finite()?;
        other.validate_finite()?;
        self.validate_closed()?;
        other.validate_closed()?;

        let minimum =
            MINIMUM_TOLERANCE_PER_UNIT * self.max_abs_coordinate().max(other.max_abs_coordinate());
        if tolerance < minimum {
            return Err(PathOpError::ToleranceTooFine { minimum });
        }

        if let Some(result) = self.empty_operand_result(other, op) {
            return Ok(result);
        }

        let rule_a = lib_fill_rule(self.fill_rule());
        let rule_b = lib_fill_rule(other.fill_rule());

        let topology = Topology::<BinaryWindingNumber>::from_paths_binary(
            self.as_bez_path(),
            other.as_bez_path(),
            tolerance,
        )
        .map_err(|error| PathOpError::Geometry(error.to_string()))?;
        let contours = topology.contours(|winding| {
            let in_a = rule_a.is_inside(winding.shape_a);
            let in_b = rule_b.is_inside(winding.shape_b);
            match op {
                BooleanOp::Union => in_a || in_b,
                BooleanOp::Intersection => in_a && in_b,
                BooleanOp::Difference => in_a && !in_b,
                BooleanOp::Xor => in_a != in_b,
            }
        });

        let mut bez = BezPath::new();
        for contour in contours.contours() {
            bez.extend(contour.path.elements().iter().copied());
        }
        Ok(Path::new(bez, FillRule::NonZero))
    }
}

impl Path {
    /// The result of the operation when at least one operand is empty, or
    /// `None` when both hold a segment.
    ///
    /// The result is decided without a sweep: a clone of the surviving operand,
    /// with its own fill rule, or an empty path.
    fn empty_operand_result(&self, other: &Path, op: BooleanOp) -> Option<Path> {
        let nothing = || Path::new(BezPath::new(), FillRule::NonZero);
        match (self.is_empty(), other.is_empty(), op) {
            (false, false, _) => None,
            (true, true, _)
            | (true, false, BooleanOp::Intersection | BooleanOp::Difference)
            | (false, true, BooleanOp::Intersection) => Some(nothing()),
            (true, false, BooleanOp::Union | BooleanOp::Xor) => Some(other.clone()),
            (false, true, _) => Some(self.clone()),
        }
    }
}

/// The library's name for a fill rule.
fn lib_fill_rule(rule: FillRule) -> LibFillRule {
    match rule {
        FillRule::EvenOdd => LibFillRule::EvenOdd,
        FillRule::NonZero => LibFillRule::NonZero,
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
    use kurbo::{Rect, Shape};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
        Path::new(Rect::new(x0, y0, x1, y1).to_path(0.0), FillRule::NonZero)
    }

    #[test]
    fn a_valid_tolerance_passes_the_check() {
        assert_eq!(validate_tolerance(1e-3), Ok(()));
    }

    #[test]
    fn an_invalid_tolerance_is_rejected() {
        for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let expected = match NonFiniteValue::classify(tolerance) {
                Some(value) => value,
                None => NonFiniteValue::Nan,
            };
            assert_eq!(
                validate_tolerance(tolerance),
                Err(PathOpError::InvalidTolerance { value: expected })
            );
        }
    }

    #[test]
    fn the_operation_maps_to_the_library_operation() {
        let a = rect(0.0, 0.0, 10.0, 10.0);
        let b = rect(5.0, 0.0, 15.0, 10.0);

        let union = a.boolean_op(&b, BooleanOp::Union, 1e-3).unwrap();
        let direct = a.union(&b, 1e-3).unwrap();
        assert_eq!(union, direct);
    }
}
