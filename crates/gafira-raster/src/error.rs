//! Why a buffer or a pixel operation was rejected.

use std::fmt;

use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

/// The reason a buffer was rejected (ARQ-117, ARQ-110).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferError {
    /// `width × height × channels` does not fit in `usize`. Nothing was allocated.
    SizeOverflow {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// The buffer length differs from `width × height × channels`.
    LengthMismatch {
        /// Length the dimensions require, in elements.
        expected: usize,
        /// Length the caller gave, in elements.
        actual: usize,
    },
    /// An input or computed `f32` value is NaN or infinite.
    NonFinite {
        /// Name of the offending field.
        field: &'static str,
        /// Index of the element in the buffer.
        index: usize,
    },
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeOverflow { width, height } => {
                write!(f, "buffer size {width}x{height} overflows")
            }
            Self::LengthMismatch { expected, actual } => {
                write!(f, "buffer length {actual} does not match {expected}")
            }
            Self::NonFinite { field, index } => {
                write!(f, "{field}[{index}] is not finite")
            }
        }
    }
}

impl std::error::Error for BufferError {}

impl ToMessage for BufferError {
    fn to_message(&self) -> Message {
        match self {
            Self::SizeOverflow { width, height } => msg!(
                "error.raster.size_overflow",
                width = Count(u64::from(*width)),
                height = Count(u64::from(*height))
            ),
            Self::LengthMismatch { expected, actual } => msg!(
                "error.raster.length_mismatch",
                expected = Count(u64::try_from(*expected).unwrap_or(u64::MAX)),
                actual = Count(u64::try_from(*actual).unwrap_or(u64::MAX))
            ),
            Self::NonFinite { field, index } => msg!(
                "error.raster.non_finite",
                field = Text((*field).to_string()),
                index = Count(u64::try_from(*index).unwrap_or(u64::MAX))
            ),
        }
    }
}
