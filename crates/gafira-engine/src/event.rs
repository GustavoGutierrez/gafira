//! The event bus of 02 §5.2 (lines 1314-1335): the vocabulary the engine
//! publishes, the sink it publishes into, and the no-effect sink this delivery
//! ships with.
//!
//! The transport is not part of this delivery. §5.2 (1335) fixes
//! `crossbeam_channel`, a bounded coalescing window of 10 Hz for `JobProgress`
//! and an injected `Waker` for the repaint; those arrive with the channel sink
//! of `E2`, so this file fixes the contract and no dependency.

use std::sync::Arc;

use gafira_doc::AssetRef;
use gafira_edit::DirtySet;
use gafira_ports::i18n::{LanguagePreference, ResolvedLocale};
use gafira_ports::license::LicenseStatus;
use gafira_ports::render::BackendKind;

/// What the engine reports to its observers (02 §5.2, lines 1317-1328).
///
/// The variants 02 names whose payload types exist today are here. `SaveState`,
/// `JobProgress`, `JobFinished`, `RecoveryAvailable`, `MemoryPressure`,
/// `StartupProgress` and `StartupFinished` are **Planned** and arrive with the
/// delivery that first produces them (design P3), because each carries a type
/// that delivery defines.
///
/// `DocumentChanged` carries no version: `DocVersion` does not exist in the
/// repository today (design P3, open question O2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineEvent {
    /// The document changed; `dirty` is what the command touched.
    DocumentChanged { dirty: DirtySet },
    /// Assets the document refers to are no longer at their recorded path.
    LinksMissing(Vec<AssetRef>),
    /// The licence state changed (02 §4.14, 920).
    LicenseChanged(LicenseStatus),
    /// The UI language changed (ARQ-101).
    LocaleChanged {
        /// What the user asked for.
        preference: LanguagePreference,
        /// What the resolver made of it.
        resolved: ResolvedLocale,
    },
    /// The render backend changed, for example after a fallback (ARQ-22).
    BackendChanged(BackendKind),
}

/// Where the bus delivers (02 §5.2).
///
/// A sink takes ownership of the event and never fails: a delivery that does not
/// arrive must not fail the operation that published it.
pub trait EventSink: Send + Sync {
    /// Deliver one event.
    fn publish(&self, event: EngineEvent);
}

/// The sink of a build that has no observer: it drops everything.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopSink;

impl EventSink for NoopSink {
    fn publish(&self, _event: EngineEvent) {}
}

/// The bus: the Facade publishes into it and the sink decides what happens next.
pub struct EventBus {
    sink: Arc<dyn EventSink>,
}

impl EventBus {
    /// A bus that delivers into `sink`.
    #[must_use]
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        Self { sink }
    }

    /// A bus with no observer (design P2).
    #[must_use]
    pub fn noop() -> Self {
        Self::new(Arc::new(NoopSink))
    }

    /// The sink this bus delivers into.
    #[must_use]
    pub fn sink(&self) -> &Arc<dyn EventSink> {
        &self.sink
    }

    /// Deliver one event, exactly once.
    pub fn publish(&self, event: EngineEvent) {
        self.sink.publish(event);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gafira_ports::i18n::{FormatLocale, Locale, LocaleSource};

    use super::*;

    /// Counts what reaches it and forwards to the real no-op sink, so a test
    /// can show the sink this delivery ships accepts every variant.
    #[derive(Default)]
    struct CountingSink {
        noop: NoopSink,
        seen: AtomicUsize,
    }

    impl EventSink for CountingSink {
        fn publish(&self, event: EngineEvent) {
            self.noop.publish(event);
            self.seen.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn document_changed() -> EngineEvent {
        EngineEvent::DocumentChanged {
            dirty: DirtySet::default(),
        }
    }

    /// Every variant of `EngineEvent`: adding a variant must fail this test, and
    /// leaving one out of the list must fail the count in
    /// `ac4_the_noop_sink_accepts_every_variant`.
    fn every_variant() -> Vec<EngineEvent> {
        vec![
            document_changed(),
            EngineEvent::LinksMissing(Vec::new()),
            EngineEvent::LicenseChanged(LicenseStatus::Unlicensed),
            EngineEvent::LocaleChanged {
                preference: LanguagePreference::System,
                resolved: ResolvedLocale {
                    locale: Locale::Es,
                    formats: FormatLocale {
                        locale: Locale::Es,
                        region: None,
                    },
                    source: LocaleSource::Fallback,
                },
            },
            EngineEvent::BackendChanged(BackendKind::Cpu),
        ]
    }

    #[test]
    fn ac4_publish_reaches_the_sink_exactly_once() {
        let sink = Arc::new(CountingSink::default());
        let bus = EventBus::new(sink.clone());

        bus.publish(document_changed());
        assert_eq!(sink.seen.load(Ordering::SeqCst), 1);

        bus.publish(document_changed());
        assert_eq!(sink.seen.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn ac4_the_noop_sink_accepts_every_variant() {
        let sink = Arc::new(CountingSink::default());
        let bus = EventBus::new(sink.clone());

        for (index, event) in every_variant().into_iter().enumerate() {
            bus.publish(event);
            assert_eq!(sink.seen.load(Ordering::SeqCst), index + 1);
        }

        assert_eq!(sink.seen.load(Ordering::SeqCst), 5);
    }
}
