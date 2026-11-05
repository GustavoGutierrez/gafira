//! The laws of the timeline as errors: a misplacement is reported, never
//! silently kept, because a model without its laws is data that breaks the
//! first time someone persists it.

use std::fmt;

use crate::{ClipId, SequenceId, TrackId};

/// A timeline law the model refuses to break.
///
/// The enum is `#[non_exhaustive]`: extending it is not a breaking change.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineError {
    /// A cut is not a whole number of frames of its sequence's rate. The
    /// value is the offending tick; it is reported, never rounded (V-2).
    OffGrid { clip: ClipId, at: i64 },
    /// A clip is shorter than one frame of its sequence (FV-01): every
    /// clip is a whole number of frames, at least one. The value is the
    /// offending span in ticks, for diagnostics.
    SpanTooShort { span: i64 },
    /// A clip's window in its source starts before the source does (FV-01):
    /// the source offset is at least zero.
    SourceBeforeStart { clip: ClipId, source_in: i64 },
    /// Two clips of one lane meet at an instant (P11): a lane holds one
    /// strip at a time, so the cut at `at` is refused.
    Overlap {
        track: TrackId,
        /// The cut that overlaps, for diagnostics.
        at: i64,
    },
    /// The sequence would take a seventeenth audio track (FV-01): sixteen.
    TooManyAudioTracks { sequence: SequenceId },
    /// A speed that divides nothing: its numerator or denominator is zero
    /// (FV-03).
    InvalidSpeed { num: u32, den: u32 },
    /// A size side is zero: a sequence without pixels serves nothing.
    EmptyPixelSize {
        sequence: SequenceId,
        width: u32,
        height: u32,
    },
}

impl fmt::Display for TimelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OffGrid { clip, at } => {
                write!(f, "clip {clip}: the cut at {at} is not a whole frame")
            }
            Self::SpanTooShort { span } => {
                write!(f, "a span of {span} ticks is shorter than one frame")
            }
            Self::SourceBeforeStart { clip, source_in } => {
                write!(
                    f,
                    "clip {clip}: source offset {source_in} is before the source start"
                )
            }
            Self::Overlap { track, at } => {
                write!(f, "track {track}: two clips meet at the instant {at}")
            }
            Self::TooManyAudioTracks { sequence } => {
                write!(
                    f,
                    "sequence {sequence}: a seventeenth audio track does not fit (sixteen is the cap)"
                )
            }
            Self::InvalidSpeed { num, den } => {
                write!(
                    f,
                    "speed {num}/{den}: a zero numerator or denominator divides nothing"
                )
            }
            Self::EmptyPixelSize {
                sequence,
                width,
                height,
            } => {
                write!(
                    f,
                    "sequence {sequence}: size {width}x{height} has a zero side"
                )
            }
        }
    }
}

impl std::error::Error for TimelineError {}

#[cfg(test)]
mod tests {
    use super::*;

    // The error displays its rule and gives no hole for a wrong cut to hide in.
    #[test]
    fn every_variant_displays() {
        let cases = [
            TimelineError::OffGrid {
                clip: ClipId::new(7),
                at: 9,
            },
            TimelineError::SpanTooShort { span: 9 },
            TimelineError::SourceBeforeStart {
                clip: ClipId::new(7),
                source_in: -3,
            },
            TimelineError::Overlap {
                track: TrackId::new(2),
                at: 11,
            },
            TimelineError::TooManyAudioTracks {
                sequence: SequenceId::new(1),
            },
            TimelineError::InvalidSpeed { num: 0, den: 4 },
            TimelineError::EmptyPixelSize {
                sequence: SequenceId::new(1),
                width: 0,
                height: 100,
            },
        ];
        let expected = [
            "clip 7: the cut at 9 is not a whole frame",
            "a span of 9 ticks is shorter than one frame",
            "clip 7: source offset -3 is before the source start",
            "track 2: two clips meet at the instant 11",
            "sequence 1: a seventeenth audio track does not fit (sixteen is the cap)",
            "speed 0/4: a zero numerator or denominator divides nothing",
            "sequence 1: size 0x100 has a zero side",
        ];
        for (error, want) in cases.iter().zip(expected) {
            assert_eq!(&error.to_string(), want, "{error:?}");
        }
    }
}
