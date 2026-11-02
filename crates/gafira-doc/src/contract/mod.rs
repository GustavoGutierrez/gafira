//! Contracts shared by the layer-2 engines (ADR 0005).
//!
//! This module imports only `crate::msg`, `gafira_color`, `serde` and `std`:
//! never `Page`, `Node` or `Document`.

mod describe;

pub use describe::{Describe, Descriptor};

/// How an audio transition blends its two sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCrossfade {
    /// Constant power across the overlap.
    EqualPower,
}
