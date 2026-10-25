//! Layer-0 exact time for Gafira.
//!
//! Time is a signed count of flicks (1/705,600,000 s). The crate is pure: it
//! reads no clock, performs no I/O and depends on no other `gafira-*` crate.

#![forbid(unsafe_code)]
