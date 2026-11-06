//! Layer-3 application crate `gafira-engine`: the Facade of 02 §5.4, the event
//! bus of §5.2, the session, the services, the memory limits and the export
//! queue (§2.1 line 103).
//!
//! The crate is pure in the sense of ARQ-02 (ADR 0016): it opens no file, opens
//! no socket, spawns no thread and reads no clock of its own. The file system
//! comes from `ports::FileSystem`, the instant from `ports::Clock` and the work
//! from the caller (ARQ-138).
//!
//! The ports are injected, the Facade composes, and each service lives in its
//! own module (design P9): `project` and `edit` share the session of
//! `session`. The other services arrive in the later deliveries of
//! `specs/changes/gafira-engine/design.md`.

#![forbid(unsafe_code)]

mod bus;
mod edit;
mod engine;
mod event;
#[cfg(test)]
mod fakes;
mod project;
mod session;

pub use bus::{ChannelSink, EventChannel, JOB_PROGRESS_WINDOW};
pub use edit::EditService;
pub use engine::{EngineConfig, EngineError, Gafira, Ports, Registries};
pub use event::{EngineEvent, EventBus, EventSink, JobId, NoopSink, NoopWaker, Waker};
pub use project::ProjectService;
pub use session::DocVersion;
