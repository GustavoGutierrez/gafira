//! Error type for every fallible operation in the crate.

use std::fmt;

use crate::Flicks;

/// Why a time operation failed.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeError {
    /// The result did not fit the signed 64-bit flick range.
    Overflow,
    /// A frame-rate denominator was zero.
    ZeroDenominator,
    /// A frame-rate numerator was zero.
    ZeroFrameRate,
    /// A frame rate was below one frame per second.
    FrameRateBelowOneFps {
        /// Numerator of the rejected rate.
        num: u32,
        /// Denominator of the rejected rate.
        den: u32,
    },
    /// One frame is not a whole number of flicks at the rejected rate.
    FrameRateNotIntegral {
        /// Numerator of the rejected rate.
        num: u32,
        /// Denominator of the rejected rate.
        den: u32,
    },
    /// A sample rate was zero.
    ZeroSampleRate,
    /// One sample is not a whole number of flicks at the rejected rate.
    SampleRateNotIntegral {
        /// The rejected rate, in hertz.
        hz: u32,
    },
    /// A time range was given a negative duration.
    NegativeDuration {
        /// The rejected duration.
        flicks: Flicks,
    },
    /// A time range was built from bounds whose end precedes its start.
    InvertedRange {
        /// The given start.
        start: Flicks,
        /// The given end.
        end: Flicks,
    },
    /// Drop-frame timecode was requested for a rate that does not support it.
    DropFrameUnsupported {
        /// Numerator of the rate.
        num: u32,
        /// Denominator of the rate.
        den: u32,
    },
    /// The label names a frame number that drop-frame timecode skips.
    DroppedFrameLabel,
    /// The timecode text does not have the `HH:MM:SS:FF` shape.
    MalformedTimecode,
    /// A timecode field is out of range for its position.
    InvalidTimecodeField,
    /// The frames separator contradicts the timecode mode.
    WrongTimecodeSeparator,
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => f.write_str("time value overflowed the signed 64-bit range"),
            Self::ZeroDenominator => f.write_str("frame-rate denominator must not be zero"),
            Self::ZeroFrameRate => f.write_str("frame-rate numerator must not be zero"),
            Self::FrameRateBelowOneFps { num, den } => {
                write!(f, "frame rate {num}/{den} is below one frame per second")
            }
            Self::FrameRateNotIntegral { num, den } => {
                write!(
                    f,
                    "one frame at {num}/{den} fps is not a whole number of flicks"
                )
            }
            Self::ZeroSampleRate => f.write_str("sample rate must not be zero"),
            Self::SampleRateNotIntegral { hz } => {
                write!(f, "one sample at {hz} Hz is not a whole number of flicks")
            }
            Self::NegativeDuration { flicks } => {
                write!(f, "time range duration {} is negative", flicks.ticks())
            }
            Self::InvertedRange { start, end } => write!(
                f,
                "time range end {} precedes its start {}",
                end.ticks(),
                start.ticks()
            ),
            Self::DropFrameUnsupported { num, den } => {
                write!(f, "drop-frame timecode is not defined for {num}/{den} fps")
            }
            Self::DroppedFrameLabel => {
                f.write_str("timecode label names a frame number skipped by drop-frame")
            }
            Self::MalformedTimecode => {
                f.write_str("timecode text is not a well-formed HH:MM:SS:FF label")
            }
            Self::InvalidTimecodeField => f.write_str("timecode field is out of range"),
            Self::WrongTimecodeSeparator => {
                f.write_str("timecode frames separator contradicts the labeling mode")
            }
        }
    }
}

impl std::error::Error for TimeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_displays_a_message() {
        let errors = [
            TimeError::Overflow,
            TimeError::ZeroDenominator,
            TimeError::ZeroFrameRate,
            TimeError::FrameRateBelowOneFps { num: 1, den: 2 },
            TimeError::FrameRateNotIntegral { num: 11, den: 1 },
            TimeError::ZeroSampleRate,
            TimeError::SampleRateNotIntegral { hz: 11 },
            TimeError::NegativeDuration {
                flicks: Flicks::from_ticks(-1),
            },
            TimeError::InvertedRange {
                start: Flicks::from_ticks(1),
                end: Flicks::ZERO,
            },
            TimeError::DropFrameUnsupported { num: 24, den: 1 },
            TimeError::DroppedFrameLabel,
            TimeError::MalformedTimecode,
            TimeError::InvalidTimecodeField,
            TimeError::WrongTimecodeSeparator,
        ];
        for error in errors {
            assert!(!error.to_string().is_empty(), "{error:?} displayed nothing");
        }
    }
}
