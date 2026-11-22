//! QR codes for Gafira (spec 080, FD-20): the text a code encodes, its
//! matrix and its outlines, for the canvas and every export.
//!
//! Pure engine code: no I/O, no network, no `unsafe`. The same code and
//! style always give the same outlines.

#![forbid(unsafe_code)]

mod check;
mod draw;
mod matrix;
mod parts;
mod payload;
mod shapes;

pub use check::{Warning, contrast, warnings};
pub use draw::{Layer, LogoBox, QrDrawing, draw, swatch};
pub use matrix::Matrix;
pub use parts::{Part, parts};
pub use payload::{ContentError, Field, payload, problems};

/// Why a code cannot be drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QrError {
    /// A field is empty or wrong.
    Content(ContentError),
    /// The content does not fit in a code of its correction.
    TooLong,
}

impl std::fmt::Display for QrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The words of the code are never part of an error (S80-01).
        match self {
            Self::Content(error) => write!(f, "the content is not valid: {error:?}"),
            Self::TooLong => write!(f, "the content does not fit in a QR code"),
        }
    }
}

impl std::error::Error for QrError {}
