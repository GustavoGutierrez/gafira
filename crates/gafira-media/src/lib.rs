//! The media engine of Gafira: the keyframe index, the frame cache, the
//! proxies, the playback clock and the video export (02 §6.5).
//!
//! The crate is pure in the sense of ARQ-02: it reads no clock of its own,
//! touches no file system of its own and creates no thread. The time comes
//! from `ports::Clock`, the file system from `ports::FileSystem` and the
//! workers from the pool the caller injects (ADR 0016, ARQ-138).

#![forbid(unsafe_code)]

mod index;
mod medium;
mod ports;

pub use index::{INDEX_BUDGET_PER_MINUTE, IndexReport, KeyframeIndex};
pub use medium::{Medium, SeekOutcome};
pub use ports::{Job, Ports, WorkerPool};
