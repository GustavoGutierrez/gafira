//! Layer-2 text engine for Gafira: shaping, bidi, line breaking and box layout.
//!
//! Fonts arrive as bytes. The output has no pixels (ARQ-116). Every public
//! function checks its numbers first and returns `Err` for the whole call
//! (ARQ-110).

#![forbid(unsafe_code)]

mod error;
mod layout;

pub use error::TextError;
pub use layout::{TextLayout, TextStyle, layout_box};
