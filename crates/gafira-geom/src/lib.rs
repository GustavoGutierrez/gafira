//! Layer-0 geometry and unit primitives for Gafira.
//!
//! Pure domain code: no I/O, no `unsafe`, `f64` everywhere.

#![forbid(unsafe_code)]

mod irect;
mod length;
mod size;
mod transform;

pub use irect::IRect;
pub use kurbo::{Affine, Point, Rect, Vec2};
pub use length::{ConversionError, Length, Unit};
pub use size::Size;
pub use transform::{SingularTransform, Transform2D};
