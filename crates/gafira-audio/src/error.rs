//! The one typed error of the crate (ARQ-110, ARQ-102).

use std::fmt;

use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

/// Why a call with a `Result` failed. `field` names the input at fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioError {
    /// An input, or an intermediate result, is NaN or infinite.
    NonFinite {
        /// The offending input.
        field: &'static str,
    },
    /// A finite input outside its documented range, or a block shape the
    /// call cannot use (channels of different lengths).
    OutOfRange {
        /// The offending input.
        field: &'static str,
    },
    /// A loudness or peak measure takes 1, 2 or 6 channels (ARQ-126).
    ChannelCount {
        /// How many channels the call got.
        channels: usize,
    },
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "{field} is not finite"),
            Self::OutOfRange { field } => write!(f, "{field} is out of range"),
            Self::ChannelCount { channels } => {
                write!(f, "{channels} channels: only 1, 2 or 6 are supported")
            }
        }
    }
}

impl std::error::Error for AudioError {}

impl ToMessage for AudioError {
    fn to_message(&self) -> Message {
        match self {
            Self::NonFinite { field } => {
                msg!("error.audio.non_finite", field = Text(field.to_string()))
            }
            Self::OutOfRange { field } => {
                msg!("error.audio.out_of_range", field = Text(field.to_string()))
            }
            Self::ChannelCount { channels } => {
                msg!(
                    "error.audio.channel_count",
                    channels = Count(*channels as u64)
                )
            }
        }
    }
}

/// `Ok(value)` when finite.
pub(crate) fn finite(field: &'static str, value: f64) -> Result<f64, AudioError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AudioError::NonFinite { field })
    }
}
