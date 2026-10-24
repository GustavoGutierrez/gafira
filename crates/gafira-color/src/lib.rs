//! Layer-0 color vocabulary and analytic conversions for Gafira.
//!
//! Pure domain code: no I/O, no `unsafe`. Channels are stored as `f32`;
//! conversions widen to `f64`, compute, and narrow once.

#![forbid(unsafe_code)]

#[cfg(test)]
mod math;
mod rgb;
mod space;

pub use rgb::{DisplayP3Rgba, LinearRgba, Rgba};
pub use space::ColorSpaceId;
