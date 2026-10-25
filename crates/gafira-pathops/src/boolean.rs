//! The path boolean operations.
//!
//! The four set operations are computed over the Bézier paths themselves, so a
//! curved input stays curved through the operation.

use kurbo::BezPath;
use linesweeper::FillRule as LibFillRule;
use linesweeper::topology::{BinaryWindingNumber, Topology};

use crate::{BooleanOp, FillRule, Path, PathOpError, validate};

/// The smallest tolerance a boolean operation accepts, per unit of the largest
/// absolute coordinate of its operands.
///
/// Below it the boolean library recurses without limit on curved operands: a
/// release build overflows its stack and aborts, a debug build panics. The
/// bound is empirical: no failure was observed at or above it.
const MINIMUM_TOLERANCE_PER_UNIT: f64 = 1e-12;

impl Path {
    /// The set of points covered by either path, or by both.
    ///
    /// The tolerance is the accuracy of the sweep, not a bound on the distance
    /// between the result and the exact answer. See [`Path::boolean_op`] for
    /// what was measured, and for the errors.
    pub fn union(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError> {
        self.boolean_op(other, BooleanOp::Union, tolerance)
    }

    /// The set of points covered by both paths.
    ///
    /// The tolerance is the accuracy of the sweep, not a bound on the distance
    /// between the result and the exact answer. See [`Path::boolean_op`] for
    /// what was measured, and for the errors.
    pub fn intersection(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError> {
        self.boolean_op(other, BooleanOp::Intersection, tolerance)
    }

    /// The set of points covered by the first path and not by the second.
    ///
    /// The tolerance is the accuracy of the sweep, not a bound on the distance
    /// between the result and the exact answer. See [`Path::boolean_op`] for
    /// what was measured, and for the errors.
    pub fn difference(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError> {
        self.boolean_op(other, BooleanOp::Difference, tolerance)
    }

    /// The set of points covered by exactly one of the two paths.
    ///
    /// The tolerance is the accuracy of the sweep, not a bound on the distance
    /// between the result and the exact answer. See [`Path::boolean_op`] for
    /// what was measured, and for the errors.
    pub fn xor(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError> {
        self.boolean_op(other, BooleanOp::Xor, tolerance)
    }

    /// Computes a set operation over the two paths.
    ///
    /// Each path is read with its own fill rule. The result carries the
    /// `NonZero` fill rule. When an operand is empty (no segment, so a moves-only
    /// path too) the result is not computed: it is a clone of the surviving
    /// operand, with that operand's own fill rule, or an empty path.
    ///
    /// # Tolerance
    ///
    /// The tolerance is the accuracy of the sweep that computes the result: a
    /// comparison tolerance along the sweep direction, not a distance in the
    /// plane. The result is not guaranteed to lie within the tolerance of the
    /// exact answer. What was measured, empirically and not proven:
    ///
    /// - for straight-edged operands, and for curves that meet the other boundary
    ///   at an angle, the result matched the exact boundary to about `1e-11`, far
    ///   below the tolerance;
    /// - the worst case is a crossing where a curve runs parallel to the sweep
    ///   direction: there the deviation grew as the square root of the tolerance
    ///   and stayed below `sqrt(2 * R * tolerance)`, where `R` is the largest
    ///   radius of curvature of the operands at a crossing;
    /// - the tolerance with which a curved input was itself produced (for
    ///   example the accuracy of a circle's approximation by cubics) adds to that
    ///   deviation;
    /// - the tolerance is meaningful only well below the thinnest feature of the
    ///   operands.
    ///
    /// # Errors
    ///
    /// In this order, the first match wins: an invalid tolerance, a non-finite
    /// coordinate (first operand, then second), an open subpath (first operand,
    /// then second), and a tolerance below `1e-12` times the largest absolute
    /// coordinate of both operands ([`PathOpError::ToleranceTooFine`]). All the
    /// checks run before the empty-operand rule.
    pub fn boolean_op(
        &self,
        other: &Path,
        op: BooleanOp,
        tolerance: f64,
    ) -> Result<Path, PathOpError> {
        validate::tolerance(tolerance)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Rect, Shape};

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
        Path::new(Rect::new(x0, y0, x1, y1).to_path(0.0), FillRule::NonZero)
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
