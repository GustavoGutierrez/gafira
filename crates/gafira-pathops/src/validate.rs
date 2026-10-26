//! The checks on the numbers every operation takes.

use crate::{InvalidNumber, NonFiniteValue, PathOpError};

/// Checks that a number is finite and above zero.
fn positive(value: f64) -> Result<(), InvalidNumber> {
    match NonFiniteValue::classify(value) {
        Some(kind) => Err(InvalidNumber::NonFinite(kind)),
        None if value > 0.0 => Ok(()),
        None => Err(InvalidNumber::NotPositive(value)),
    }
}

/// Checks that the tolerance is a finite positive number.
pub(crate) fn tolerance(tolerance: f64) -> Result<(), PathOpError> {
    positive(tolerance).map_err(|cause| PathOpError::InvalidTolerance { cause })
}

/// Checks that the stroke width is a finite positive number.
pub(crate) fn stroke_width(width: f64) -> Result<(), PathOpError> {
    positive(width).map_err(|cause| PathOpError::InvalidStrokeWidth { cause })
}

/// Checks that the tolerance is at least `per_unit` times the extent.
///
/// The extent is the largest absolute coordinate the operation reads (and, for
/// the stroke outline, half the width). A tolerance equal to the minimum is
/// accepted. The error carries the minimum.
pub(crate) fn not_too_fine(tolerance: f64, per_unit: f64, extent: f64) -> Result<(), PathOpError> {
    let minimum = per_unit * extent;
    if tolerance < minimum {
        return Err(PathOpError::ToleranceTooFine { minimum });
    }
    Ok(())
}
