//! The one typed error of the crate (ADR 0012).

use std::fmt;

use gafira_doc::contract::Cancelled;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_raster::{BufferError, PixelFormat};

/// Why a segmentation call failed.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegError {
    /// A buffer or a number was rejected: length, overflow, or a non-finite
    /// pixel or hint value (ARQ-110). The `field` names the input at fault.
    Buffer(BufferError),
    /// The image has width or height 0 (ARQ-130).
    EmptyImage {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// The image carries no colour (`Alpha8`).
    UnsupportedFormat {
        /// The rejected format.
        format: PixelFormat,
    },
    /// The token was cancelled; no output was produced (ARQ-112).
    Cancelled(Cancelled),
}

impl fmt::Display for SegError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buffer(e) => e.fmt(f),
            Self::EmptyImage { width, height } => {
                write!(f, "image {width}x{height} has no pixels")
            }
            Self::UnsupportedFormat { format } => {
                write!(f, "pixel format {} carries no colour", format_name(*format))
            }
            Self::Cancelled(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for SegError {}

impl From<BufferError> for SegError {
    fn from(e: BufferError) -> Self {
        Self::Buffer(e)
    }
}

impl From<Cancelled> for SegError {
    fn from(e: Cancelled) -> Self {
        Self::Cancelled(e)
    }
}

/// Stable technical name of a format, for message arguments (ADR 0012).
fn format_name(format: PixelFormat) -> &'static str {
    match format {
        PixelFormat::Rgba8 => "rgba8",
        PixelFormat::RgbaF32 => "rgba_f32",
        PixelFormat::Alpha8 => "alpha8",
    }
}

impl ToMessage for SegError {
    fn to_message(&self) -> Message {
        match self {
            Self::Buffer(e) => e.to_message(),
            Self::EmptyImage { width, height } => msg!(
                "error.segment.empty_image",
                width = Count(u64::from(*width)),
                height = Count(u64::from(*height))
            ),
            Self::UnsupportedFormat { format } => msg!(
                "error.segment.unsupported_format",
                format = Text(format_name(*format).to_string())
            ),
            Self::Cancelled(e) => e.to_message(),
        }
    }
}
