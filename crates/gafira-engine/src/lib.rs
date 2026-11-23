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
//! `session`; `settings` (spec 009) holds the user's settings `license` (spec 010)
//! the licence state and `assets` (spec 011) the imports. The other services arrive in the later deliveries of
//! `specs/done/legacy/changes/gafira-engine/design.md`.

#![forbid(unsafe_code)]

mod assets;
mod autosave;
mod bus;
mod edit;
mod engine;
mod event;
mod export;
#[cfg(test)]
mod fakes;
mod governor;
mod license;
mod lock;
mod project;
mod session;
mod settings;
mod thumbnail;
mod versions;
pub mod vision;

pub use assets::AssetService;
pub use autosave::{AutosaveJob, AutosaveOutcome};
pub use bus::{ChannelSink, EventChannel, JOB_PROGRESS_WINDOW};
pub use edit::EditService;
pub use engine::{
    EngineConfig, EngineError, ExportPorts, Gafira, Ports, Registries, SettingsPorts, SystemPorts,
};
pub use event::{
    EngineEvent, EventBus, EventSink, JobId, NoopSink, NoopWaker, RecoveryEntry, Waker,
};
pub use export::{ExportService, PagePreview, PreparedExport};
pub use governor::{
    BackendTrim, CacheKind, GovernorOutcome, MAX_TRIM_PASSES, MemoryGovernor, PressureLevel,
    TileCacheTrim, Trimmable,
};
pub use license::LicenseService;
pub use lock::FileLock;
pub use project::{ProjectService, SaveJob, SaveOutcome, SaveRequest};
pub use session::DocVersion;
pub use settings::SettingsService;
