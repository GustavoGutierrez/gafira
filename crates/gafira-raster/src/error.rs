//! Why a buffer or a pixel operation was rejected.

use std::fmt;

use gafira_doc::contract::ParamError;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

use crate::PixelFormat;

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
        /// Index of the element in the buffer, or the channel (0 to 3) for a single pixel.
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

/// Why an effect or a transition could not run (02 §7.3, ADR 0012).
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectError {
    /// A buffer or a number was rejected (length, overflow, non-finite).
    Buffer(BufferError),
    /// The parameter values do not fit the schema.
    Param(ParamError),
    /// A buffer has other dimensions than the reference one.
    SizeMismatch {
        /// `to` or `dst`; `from` is the reference.
        field: &'static str,
        /// Width and height of the reference.
        expected: (u32, u32),
        /// Width and height of `field`.
        actual: (u32, u32),
    },
    /// A buffer has another pixel format than the reference one.
    FormatMismatch {
        /// `to` or `dst`; `from` is the reference.
        field: &'static str,
        /// Format of the reference.
        expected: PixelFormat,
        /// Format of `field`.
        actual: PixelFormat,
    },
    /// The operation does not take this pixel format.
    UnsupportedFormat {
        /// The rejected format.
        format: PixelFormat,
    },
}

impl fmt::Display for EffectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buffer(e) => e.fmt(f),
            Self::Param(e) => e.fmt(f),
            Self::SizeMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "{field} is {}x{} but the reference is {}x{}",
                actual.0, actual.1, expected.0, expected.1
            ),
            Self::FormatMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "{field} has format {} but the reference has {}",
                format_name(*actual),
                format_name(*expected)
            ),
            Self::UnsupportedFormat { format } => {
                write!(
                    f,
                    "pixel format {} is not supported here",
                    format_name(*format)
                )
            }
        }
    }
}

impl std::error::Error for EffectError {}

impl From<BufferError> for EffectError {
    fn from(e: BufferError) -> Self {
        Self::Buffer(e)
    }
}

impl From<ParamError> for EffectError {
    fn from(e: ParamError) -> Self {
        Self::Param(e)
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

impl ToMessage for EffectError {
    fn to_message(&self) -> Message {
        match self {
            Self::Buffer(e) => e.to_message(),
            Self::Param(e) => e.to_message(),
            Self::SizeMismatch {
                field,
                expected,
                actual,
            } => msg!(
                "error.raster.size_mismatch",
                field = Text((*field).to_string()),
                expected_width = Count(u64::from(expected.0)),
                expected_height = Count(u64::from(expected.1)),
                actual_width = Count(u64::from(actual.0)),
                actual_height = Count(u64::from(actual.1))
            ),
            Self::FormatMismatch {
                field,
                expected,
                actual,
            } => msg!(
                "error.raster.format_mismatch",
                field = Text((*field).to_string()),
                expected = Text(format_name(*expected).to_string()),
                actual = Text(format_name(*actual).to_string())
            ),
            Self::UnsupportedFormat { format } => msg!(
                "error.raster.unsupported_format",
                format = Text(format_name(*format).to_string())
            ),
        }
    }
}

#[cfg(test)]
mod effect_tests {
    use super::*;

    fn text(m: &Message, name: &str) -> Option<String> {
        m.args
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, a)| match a {
                gafira_doc::msg::MsgArg::Text(t) => Some(t.clone()),
                _ => None,
            })
    }

    // 02:1308: EffectError wraps BufferError and ParamError with From, and to_message delegates.
    #[test]
    fn effect_error_wraps_buffer_and_param_errors_and_delegates() {
        let b = BufferError::NonFinite {
            field: "progress",
            index: 0,
        };
        let e = EffectError::from(b);
        assert_eq!(e, EffectError::Buffer(b));
        assert_eq!(e.to_message(), b.to_message());
        let p = ParamError::OutOfRange { param: "softness" };
        let e = EffectError::from(p);
        assert_eq!(e, EffectError::Param(p));
        assert_eq!(e.to_message(), p.to_message());
        // `?` converts both.
        let via_question = || -> Result<(), EffectError> { Err(b)? };
        assert_eq!(via_question(), Err(EffectError::Buffer(b)));
        let via_question = || -> Result<(), EffectError> { Err(p)? };
        assert_eq!(via_question(), Err(EffectError::Param(p)));
    }

    // P15 item 7: the three new variants have keys, with sizes in Count and formats in Text.
    #[test]
    fn effect_error_new_variants_have_keys_and_arguments() {
        use gafira_doc::msg::MsgArg;
        let m = EffectError::SizeMismatch {
            field: "dst",
            expected: (4, 2),
            actual: (5, 3),
        }
        .to_message();
        assert_eq!(m.key.0, "error.raster.size_mismatch");
        assert_eq!(text(&m, "field").as_deref(), Some("dst"));
        let count = |n: &str| m.args.iter().find(|(k, _)| *k == n).map(|(_, a)| a.clone());
        assert_eq!(count("expected_width"), Some(MsgArg::Count(4)));
        assert_eq!(count("expected_height"), Some(MsgArg::Count(2)));
        assert_eq!(count("actual_width"), Some(MsgArg::Count(5)));
        assert_eq!(count("actual_height"), Some(MsgArg::Count(3)));
        let m = EffectError::FormatMismatch {
            field: "to",
            expected: PixelFormat::Rgba8,
            actual: PixelFormat::RgbaF32,
        }
        .to_message();
        assert_eq!(m.key.0, "error.raster.format_mismatch");
        assert_eq!(text(&m, "field").as_deref(), Some("to"));
        assert_eq!(text(&m, "expected").as_deref(), Some("rgba8"));
        assert_eq!(text(&m, "actual").as_deref(), Some("rgba_f32"));
        let m = EffectError::UnsupportedFormat {
            format: PixelFormat::Alpha8,
        }
        .to_message();
        assert_eq!(m.key.0, "error.raster.unsupported_format");
        assert_eq!(text(&m, "format").as_deref(), Some("alpha8"));
    }

    // 02 §7.3: Display is English text for the log, and the error is a std error.
    #[test]
    fn effect_error_displays_for_the_log() {
        fn assert_error<E: std::error::Error + Clone + PartialEq>() {}
        assert_error::<EffectError>();
        let shown = EffectError::SizeMismatch {
            field: "to",
            expected: (1, 2),
            actual: (3, 4),
        }
        .to_string();
        assert!(
            shown.contains("to") && shown.contains("1x2") && shown.contains("3x4"),
            "{shown}"
        );
        assert!(
            !EffectError::UnsupportedFormat {
                format: PixelFormat::Alpha8
            }
            .to_string()
            .is_empty()
        );
    }
}
