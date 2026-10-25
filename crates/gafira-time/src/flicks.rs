//! Exact rational time in flicks.

use serde::{Deserialize, Serialize};

use crate::TimeError;

/// Flicks per second: 705,600,000.
pub const FLICKS_PER_SECOND: i64 = 705_600_000;
/// Flicks per millisecond: 705,600.
pub const FLICKS_PER_MILLISECOND: i64 = 705_600;

/// A signed count of flicks (1/705,600,000 s).
///
/// The value is exact: no floating-point arithmetic participates in storage or
/// in the conversions defined by this crate. Arithmetic is checked; nothing
/// wraps or saturates silently.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Flicks(i64);

impl Flicks {
    /// Zero time.
    pub const ZERO: Self = Self(0);

    /// Builds a value from a raw tick count.
    pub const fn from_ticks(ticks: i64) -> Self {
        Self(ticks)
    }

    /// The raw tick count.
    pub const fn ticks(self) -> i64 {
        self.0
    }

    /// Exact conversion from whole seconds.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] when the result does not fit the signed
    /// 64-bit flick range.
    pub fn from_seconds(seconds: i64) -> Result<Self, TimeError> {
        seconds
            .checked_mul(FLICKS_PER_SECOND)
            .map(Self)
            .ok_or(TimeError::Overflow)
    }

    /// Exact conversion from whole milliseconds.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] when the result does not fit the signed
    /// 64-bit flick range.
    pub fn from_millis(millis: i64) -> Result<Self, TimeError> {
        millis
            .checked_mul(FLICKS_PER_MILLISECOND)
            .map(Self)
            .ok_or(TimeError::Overflow)
    }

    /// Whole seconds, floored towards negative infinity.
    pub fn seconds_floor(self) -> i64 {
        self.0.div_euclid(FLICKS_PER_SECOND)
    }

    /// Seconds as `f64`.
    ///
    /// Lossy and intended for display only: use [`Flicks::ticks`] or an exact
    /// conversion for persistence and comparison.
    pub fn as_seconds_f64(self) -> f64 {
        self.0 as f64 / FLICKS_PER_SECOND as f64
    }

    /// Checked addition.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] when the sum does not fit.
    pub fn checked_add(self, other: Self) -> Result<Self, TimeError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(TimeError::Overflow)
    }

    /// Checked subtraction.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] when the difference does not fit.
    pub fn checked_sub(self, other: Self) -> Result<Self, TimeError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(TimeError::Overflow)
    }

    /// Checked multiplication by an integer factor.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] when the product does not fit.
    pub fn checked_mul(self, factor: i64) -> Result<Self, TimeError> {
        self.0
            .checked_mul(factor)
            .map(Self)
            .ok_or(TimeError::Overflow)
    }

    /// Checked negation.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] for the minimum representable value.
    pub fn checked_neg(self) -> Result<Self, TimeError> {
        self.0.checked_neg().map(Self).ok_or(TimeError::Overflow)
    }
}
