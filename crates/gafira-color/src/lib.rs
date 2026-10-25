//! Layer-0 color vocabulary and analytic conversions for Gafira.
//!
//! Pure domain code: no I/O, no `unsafe`. Channels are stored as `f32`;
//! conversions widen to `f64`, compute, and narrow once.

#![forbid(unsafe_code)]

mod color;
mod math;
mod rgb;
mod space;
mod transform;

pub use color::Color;
pub use rgb::{DisplayP3Rgba, LinearRgba, Rgba};
pub use space::ColorSpaceId;
pub use transform::{AnalyticTransform, ColorTransform, PixelBufferError, PixelLayout};
