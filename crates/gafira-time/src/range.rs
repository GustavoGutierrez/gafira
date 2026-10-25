//! Validated half-open time ranges.

use serde::{Deserialize, Serialize};

use crate::{Flicks, TimeError};

/// A half-open interval `[start, end)` with an exact, non-negative duration.
///
/// The end is validated once at construction, so `start + duration` can never
/// overflow through this type. An inverted range is not representable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "TimeRangeWire", into = "TimeRangeWire")]
pub struct TimeRange {
    start: Flicks,
    duration: Flicks,
    end: Flicks,
}

#[derive(Serialize, Deserialize)]
struct TimeRangeWire {
    start: Flicks,
    duration: Flicks,
}

impl TryFrom<TimeRangeWire> for TimeRange {
    type Error = TimeError;

    fn try_from(wire: TimeRangeWire) -> Result<Self, TimeError> {
        Self::new(wire.start, wire.duration)
    }
}

impl From<TimeRange> for TimeRangeWire {
    fn from(range: TimeRange) -> Self {
        Self {
            start: range.start,
            duration: range.duration,
        }
    }
}

impl TimeRange {
    /// Builds a range from a start and a non-negative duration.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::NegativeDuration`] for a negative duration and
    /// [`TimeError::Overflow`] when the end does not fit the signed 64-bit range.
    pub fn new(start: Flicks, duration: Flicks) -> Result<Self, TimeError> {
        if duration.ticks() < 0 {
            return Err(TimeError::NegativeDuration { flicks: duration });
        }
        let end = start.checked_add(duration)?;
        Ok(Self {
            start,
            duration,
            end,
        })
    }

    /// Builds a range from ordered bounds.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InvertedRange`] when `end` precedes `start` and
    /// [`TimeError::Overflow`] when the duration does not fit.
    pub fn from_bounds(start: Flicks, end: Flicks) -> Result<Self, TimeError> {
        if end < start {
            return Err(TimeError::InvertedRange { start, end });
        }
        let duration = end.checked_sub(start)?;
        Ok(Self {
            start,
            duration,
            end,
        })
    }

    /// The start instant.
    pub const fn start(self) -> Flicks {
        self.start
    }

    /// The duration; never negative.
    pub const fn duration(self) -> Flicks {
        self.duration
    }

    /// The end instant; always `start + duration`.
    pub const fn end(self) -> Flicks {
        self.end
    }

    /// Whether the range contains no instant.
    pub const fn is_empty(self) -> bool {
        self.duration.ticks() == 0
    }

    /// Whether `at` lies in `[start, end)`.
    pub fn contains(self, at: Flicks) -> bool {
        self.start <= at && at < self.end
    }

    /// Whether the two ranges share at least one instant.
    pub fn intersects(self, other: Self) -> bool {
        self.intersection(other).is_some()
    }

    /// The common part, or `None` when there is none.
    pub fn intersection(self, other: Self) -> Option<Self> {
        if self.is_empty() || other.is_empty() {
            return None;
        }
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start >= end {
            return None;
        }
        // The span is bounded by the operands' durations, so it cannot overflow.
        let duration = end.checked_sub(start).ok()?;
        Some(Self {
            start,
            duration,
            end,
        })
    }
}
