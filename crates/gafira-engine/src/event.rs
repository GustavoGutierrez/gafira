//! The event bus contract of 02 §5.2 (lines 1314-1335): the vocabulary the
//! engine publishes, the sink it publishes into, and the waker that tells the
//! observer something arrived.
//!
//! The transport itself — the unbounded channel and the 10 Hz coalescing window
//! — lives in [`crate::bus::ChannelSink`]; this module keeps the contract, so a
//! sink can be a channel, a test counter or nothing at all.

use std::sync::Arc;
use std::time::Duration;

use gafira_doc::AssetRef;
use gafira_edit::DirtySet;
use gafira_ports::i18n::{LanguagePreference, ResolvedLocale};
use gafira_ports::license::LicenseStatus;
use gafira_ports::render::BackendKind;

/// The identifier of a queued job (02 §5.2 line 1322, §6.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

impl JobId {
    /// The id of a job the queue assigned.
    #[must_use]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// The number this id carries.
    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

/// What the engine reports to its observers (02 §5.2, lines 1317-1328).
///
/// The variants 02 names arrive with the delivery that first produces them
/// (design P3). `E1` shipped the five whose payload types exist in the layers
/// below; `E2` adds `JobProgress` with its `JobId`, because the coalescing rule
/// of §5.2 line 1335 is what this delivery builds. `SaveState`, `JobFinished`,
/// `RecoveryAvailable`, `MemoryPressure`, `StartupProgress`, `StartupFinished`
/// and `SelectionChanged` are **Planned** for `E3` to `E10`.
///
/// `DocumentChanged` carries no version yet: `DocVersion` does not exist in the
/// repository (design P3, open question O2). `E2` drops the `Eq` bound, which
/// `JobProgress`'s `f32` cannot satisfy.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    /// The document changed; `dirty` is what the command touched.
    DocumentChanged { dirty: DirtySet },
    /// Assets the document refers to are no longer at their recorded path.
    LinksMissing(Vec<AssetRef>),
    /// The licence state changed (02 §4.14, line 920).
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
    /// A queued job advanced; coalesced to 10 Hz (02 §5.2 line 1335).
    JobProgress {
        /// Which job advanced.
        job: JobId,
        /// How much of it is done, in `[0, 1]`.
        fraction: f32,
        /// The estimate of what is left, when the job can produce one.
        eta: Option<Duration>,
    },
}

/// Tells the observer that an event is waiting for it (02 §5.2 line 1335).
///
/// The engine never repaints anything itself: it calls `wake` and the adapter
/// that owns the UI decides what that means.
pub trait Waker: Send + Sync {
    /// Ask the observer to look at the queue.
    fn wake(&self);
}

/// The waker of a build with no observer.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopWaker;

impl Waker for NoopWaker {
    fn wake(&self) {}
}

/// Where the bus delivers (02 §5.2).
///
/// A sink takes ownership of the event and never fails: a delivery that does
/// not arrive must not fail the operation that published it.
pub trait EventSink: Send + Sync {
    /// Deliver one event.
    fn publish(&self, event: EngineEvent);

    /// Deliver whatever the sink is holding back, when its window allows it.
    ///
    /// The default does nothing; the coalescing sink of `E2` overrides it.
    fn flush(&self) {}
}

/// The sink of a build that has no observer: it drops everything.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopSink;

impl EventSink for NoopSink {
    fn publish(&self, _event: EngineEvent) {}
}

/// The bus: the Facade publishes into it and the sink decides what happens next.
///
/// Cloning is cheap (one `Arc`) and every clone feeds the same sink, which is
/// what lets the startup sequence publish from another thread (design P11).
#[derive(Clone)]
pub struct EventBus {
    sink: Arc<dyn EventSink>,
}

impl EventBus {
    /// A bus that delivers into `sink`.
    #[must_use]
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        Self { sink }
    }

    /// A bus with no observer.
    #[must_use]
    pub fn noop() -> Self {
        Self::new(Arc::new(NoopSink))
    }

    /// The sink this bus delivers into.
    #[must_use]
    pub fn sink(&self) -> &Arc<dyn EventSink> {
        &self.sink
    }

    /// Deliver one event, exactly once, unless the sink coalesces it.
    pub fn publish(&self, event: EngineEvent) {
        self.sink.publish(event);
    }

    /// Let the sink release what its window held back.
    pub fn flush(&self) {
        self.sink.flush();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gafira_ports::i18n::{FormatLocale, Locale, LocaleSource};

    use super::*;

    #[derive(Default)]
    struct CountingSink {
        seen: AtomicUsize,
    }

    impl EventSink for CountingSink {
        fn publish(&self, _event: EngineEvent) {
            self.seen.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn document_changed() -> EngineEvent {
        EngineEvent::DocumentChanged {
            dirty: DirtySet::default(),
        }
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

    /// E2-W1: every variant of the vocabulary goes through the bus to its sink,
    /// one count each. A new variant must be added to this list.
    #[test]
    fn ac4_every_variant_reaches_the_sink_once() {
        const VARIANTS: usize = 6;
        let sink = Arc::new(CountingSink::default());
        let bus = EventBus::new(sink.clone());
        let every_variant = [
            document_changed(),
            EngineEvent::LinksMissing(Vec::new()),
            EngineEvent::LicenseChanged(LicenseStatus::Unlicensed),
            EngineEvent::LocaleChanged {
                preference: LanguagePreference::En,
                resolved: ResolvedLocale {
                    locale: Locale::En,
                    formats: FormatLocale {
                        locale: Locale::En,
                        region: None,
                    },
                    source: LocaleSource::Manual,
                },
            },
            EngineEvent::BackendChanged(BackendKind::Cpu),
            EngineEvent::JobProgress {
                job: JobId::new(1),
                fraction: 0.5,
                eta: None,
            },
        ];

        for (published, event) in every_variant.into_iter().enumerate() {
            bus.publish(event);
            assert_eq!(sink.seen.load(Ordering::SeqCst), published + 1);
        }
        assert_eq!(sink.seen.load(Ordering::SeqCst), VARIANTS);
    }

    #[test]
    fn ac4_the_noop_sink_accepts_a_flush() {
        EventBus::noop().flush();
    }

    #[test]
    fn ac4_a_job_id_round_trips_its_number() {
        assert_eq!(JobId::new(7).get(), 7);
        assert_eq!(JobId(7), JobId::new(7));
    }
}
