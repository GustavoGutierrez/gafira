//! Why a path operation could not be completed.

use std::fmt;

/// Which kind of non-finite number a coordinate holds.
///
/// The classification, not the raw `f64`, is what the error carries: a NaN does
/// not equal itself, so an error holding a raw NaN could not be compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NonFiniteValue {
    /// Not a number.
    Nan,
    /// Positive infinity.
    Infinity,
    /// Negative infinity.
    NegativeInfinity,
}

impl fmt::Display for NonFiniteValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nan => write!(f, "NaN"),
            Self::Infinity => write!(f, "infinity"),
            Self::NegativeInfinity => write!(f, "negative infinity"),
        }
    }
}

impl NonFiniteValue {
    /// Classifies a number that is known not to be finite.
    #[must_use]
    pub fn classify(value: f64) -> Option<Self> {
        if value.is_nan() {
            Some(Self::Nan)
        } else if value == f64::INFINITY {
            Some(Self::Infinity)
        } else if value == f64::NEG_INFINITY {
            Some(Self::NegativeInfinity)
        } else {
            None
        }
    }
}

/// The reason a path operation failed.
///
/// The crate checks its own inputs before it computes, so the error names the
/// actual cause instead of forwarding a lower-level error that would mislead.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PathOpError {
    /// A coordinate of the path is not a finite number.
    NonFiniteCoordinate {
        /// Which kind of non-finite number the coordinate holds.
        value: NonFiniteValue,
    },
    /// A subpath of the path is not closed.
    ///
    /// An open subpath has no interior, so it has no defined meaning under a
    /// fill rule. Use [`crate::Path::close_subpaths`] to close it explicitly.
    OpenSubpath {
        /// The index of the open subpath, counted from zero in path order.
        subpath: usize,
    },
    /// The stroke width is not a finite positive number.
    InvalidStrokeWidth {
        /// Which kind of bad number the width holds.
        value: NonFiniteValue,
    },
    /// The tolerance is not a finite positive number.
    InvalidTolerance {
        /// Which kind of bad number the tolerance holds.
        value: NonFiniteValue,
    },
    /// The geometry engine could not complete the operation.
    Geometry(String),
}

impl fmt::Display for PathOpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCoordinate { value } => {
                write!(f, "path has a non-finite coordinate: {value}")
            }
            Self::OpenSubpath { subpath } => {
                write!(f, "subpath {subpath} of the path is not closed")
            }
            Self::InvalidStrokeWidth { value } => {
                write!(f, "stroke width is not a finite positive number: {value}")
            }
            Self::InvalidTolerance { value } => {
                write!(f, "tolerance is not a finite positive number: {value}")
            }
            Self::Geometry(detail) => write!(f, "geometry operation failed: {detail}"),
        }
    }
}

impl std::error::Error for PathOpError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_is_cloneable_comparable_and_an_error() {
        fn assert_contract<E: std::error::Error + fmt::Display + Clone + PartialEq>() {}
        assert_contract::<PathOpError>();
    }

    #[test]
    fn each_variant_names_its_cause() {
        let cases = [
            (
                PathOpError::NonFiniteCoordinate {
                    value: NonFiniteValue::Nan,
                },
                "non-finite",
            ),
            (PathOpError::OpenSubpath { subpath: 2 }, "subpath 2"),
            (
                PathOpError::InvalidStrokeWidth {
                    value: NonFiniteValue::Nan,
                },
                "stroke width",
            ),
            (
                PathOpError::InvalidTolerance {
                    value: NonFiniteValue::Infinity,
                },
                "tolerance",
            ),
            (PathOpError::Geometry("boom".to_string()), "boom"),
        ];
        for (error, expected) in cases {
            let text = error.to_string();
            assert!(
                text.contains(expected),
                "error {text:?} does not mention {expected:?}"
            );
        }
    }
}
