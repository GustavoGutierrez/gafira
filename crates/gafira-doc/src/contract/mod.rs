//! Contracts shared by the layer-2 engines (ADR 0005).
//!
//! This module imports only `crate::msg`, `gafira_color`, `serde` and `std`:
//! never `Page`, `Node` or `Document`.

mod describe;
mod params;

pub use describe::{Describe, Descriptor};
pub use params::{
    ParamChoice, ParamDef, ParamError, ParamKind, ParamSchema, ParamValue, ParamValues,
};

/// How an audio transition blends its two sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCrossfade {
    /// Constant power across the overlap.
    EqualPower,
}
