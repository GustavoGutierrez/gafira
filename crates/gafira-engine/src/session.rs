//! The open document and the version observers see it by (design P15).
//!
//! The session owns the `History`, the only place a command runs (ARQ-30).
//! `ProjectService` and `EditService` share it through one [`SessionCell`], so
//! `Gafira::projects(&self)` and `Gafira::edit(&mut self)` of 02 §5.4 both
//! hold. The version is not the session's: the engine owns it, so replacing
//! the session never resets it (owner decision O2).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use gafira_edit::{DirtySet, History};

use crate::event::{EngineEvent, EventBus};

/// The number of a document state an observer saw (02 §5.2 line 1317).
///
/// 0 when the engine is built, then one more on every state change of the
/// P15 table, for the whole life of the engine: `new` bumps it instead of
/// resetting it, so a cache keyed by version never confuses two documents,
/// and an undo bumps it too, because a version names a state the observer
/// saw, not a content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocVersion(pub(crate) u64);

impl DocVersion {
    /// The number this version carries.
    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

/// The open document: its history and the version it was last clean at.
///
/// The path, the read-only flag and the autosave state of P15 arrive with
/// the deliveries that read them (`E3c`, `E3d`).
pub(crate) struct Session {
    /// The document and its undo stack, with the default budget (§5.1).
    pub(crate) history: History,
    /// The version of the last `new`, `open` or `save` (P15, "modified").
    pub(crate) clean_version: DocVersion,
}

/// The session both services share; `None` until the first `new` or `open`.
pub(crate) type SessionCell = Arc<Mutex<Option<Session>>>;

/// What every service that changes the document holds: the session, the
/// engine's version counter and the bus the change is published on.
#[derive(Clone)]
pub(crate) struct Shared {
    session: SessionCell,
    version: Arc<AtomicU64>,
    bus: EventBus,
}

impl Shared {
    /// No session and version 0, publishing on `bus`.
    pub(crate) fn new(bus: EventBus) -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            version: Arc::new(AtomicU64::new(0)),
            bus,
        }
    }

    /// The session, ignoring lock poisoning as `bus.rs` does: a panic in
    /// another thread must not lose the open document.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Option<Session>> {
        match self.session.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// The version of the newest published state.
    pub(crate) fn version(&self) -> DocVersion {
        DocVersion(self.version.load(Ordering::SeqCst))
    }

    /// Bump the version by one and publish exactly one `DocumentChanged` with
    /// it and `dirty`. Callers hold the session lock, so the versions reach
    /// the bus in order.
    pub(crate) fn changed(&self, dirty: DirtySet) -> DocVersion {
        let version = DocVersion(self.version.fetch_add(1, Ordering::SeqCst) + 1);
        self.bus
            .publish(EngineEvent::DocumentChanged { version, dirty });
        version
    }
}
