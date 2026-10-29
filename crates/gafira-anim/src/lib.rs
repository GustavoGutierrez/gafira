//! Layer-1 animation for Gafira: easing curves, keyframe tracks and sampling.
//!
//! The crate is pure: it reads no clock, performs no I/O and depends only on
//! lower-layer `gafira-*` crates.

#![forbid(unsafe_code)]

mod easing;
mod interpolate;

pub use easing::Easing;
pub use interpolate::Interpolate;
