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
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "{field} is not finite"),
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
