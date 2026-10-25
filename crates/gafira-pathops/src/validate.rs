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
