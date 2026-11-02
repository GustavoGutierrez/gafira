//! The one typed error of the crate.

use std::fmt;

use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

/// Why a text call failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextError {
    /// An input, or an intermediate result, is NaN or infinite.
    NonFinite {
        /// The offending input.
        field: &'static str,
    },
    /// A finite input lies outside its allowed range.
    OutOfRange {
        /// The offending input.
        field: &'static str,
    },
    /// The font at this position of the stack cannot be read.
    InvalidFont {
        /// Position in the font stack.
        index: usize,
    },
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "{field} is not finite"),
            Self::OutOfRange { field } => write!(f, "{field} is out of range"),
            Self::InvalidFont { index } => write!(f, "font {index} cannot be read"),
        }
    }
}

impl std::error::Error for TextError {}

impl ToMessage for TextError {
    fn to_message(&self) -> Message {
        match self {
            Self::NonFinite { field } => {
                msg!("error.text.non_finite", field = Text(field.to_string()))
            }
            Self::OutOfRange { field } => {
                msg!("error.text.out_of_range", field = Text(field.to_string()))
            }
            Self::InvalidFont { index } => {
                msg!("error.text.invalid_font", index = Text(index.to_string()))
            }
        }
    }
}
