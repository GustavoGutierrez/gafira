//! Gafira design tokens and embedded fonts (02 §2.1, §8.2; 03 §3).
//!
//! A layer-4 adapter. It loads `assets/theme/gafira.tokens.json`, compiled in
//! with `include_str!`, resolves its `{references}`, validates it (schema,
//! both modes, WCAG contrast of the declared pairs, reduced motion) and
//! exposes the result as neutral types: no egui, so `gafira-ui` translates
//! them to its style in one place (ARQ-60). Token values exist only in the
//! JSON. Inter and Manrope are embedded with their OFL text (DIS-05).
//!
//! The crate is pure in the sense of ARQ-02: it reads no file, clock or
//! environment at run time.
#![forbid(unsafe_code)]

mod color;
mod error;
mod fonts;
mod library;
mod tokens;

pub use color::{Rgb8, contrast_ratio};
pub use error::ThemeError;
pub use fonts::{EmbeddedFont, font, fonts};
pub use library::{LibraryFace, face_key, library, library_families, nearest_face};
pub use tokens::{
    BorderTokens, Brand, CheckedPair, Durations, ModeTokens, MotionTokens, REQUIRED_MODE_TOKENS,
    REQUIRED_TEXT_STYLES, RadiusTokens, SUPPORTED_SCHEMA, Shadow, SizeSteps, SizeTokens,
    SpaceTokens, SplashTokens, TOKENS_JSON, TextStyle, ThemeTokens, TypeTokens,
};
