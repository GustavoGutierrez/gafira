//! Layer-0 exact time for Gafira.
//!
//! Time is a signed count of flicks (1/705,600,000 s). The crate is pure: it
//! reads no clock, performs no I/O and depends on no other `gafira-*` crate.

#![forbid(unsafe_code)]

mod error;
mod flicks;

pub use error::TimeError;
pub use flicks::{FLICKS_PER_MILLISECOND, FLICKS_PER_SECOND, Flicks};
