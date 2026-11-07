//! The Facade of 02 §5.4 (lines 1351-1365) and the values it is built from.
//!
//! `Gafira` composes; it does not implement anything (design P9). The ports come
//! from the composition root (ARQ-06), the registries are the ones the adapters
//! filled before the engine started (ARQ-31), and the configuration comes from
//! the caller, never from the environment (design P5).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gafira_ports::common::Bytes;
use gafira_ports::format::FormatRegistry;
use gafira_ports::license::LicenseVerifier;
use gafira_ports::media::CodecRegistry;
use gafira_ports::repository::{ProjectRepository, RepositoryError};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{Clock, FileSystem, Notifier};

use crossbeam_channel::Receiver;

use crate::bus::EventChannel;
use crate::event::{EngineEvent, EventBus};

/// The port implementations the composition root injects (ARQ-06, 02 §5.4).
///
/// The engine never creates a port: a missing one is a compile error at the
/// call site, never a silent default (design P4).
pub struct Ports {
    /// Loads and saves the project document (02 §4, `ports::ProjectRepository`).
    pub repository: Arc<dyn ProjectRepository>,
    /// The content-addressed store the assets live in (`ports::ContentStore`).
    pub store: Arc<dyn ContentStore>,
    /// Every file operation, so the engine opens nothing itself (ARQ-50).
    pub filesystem: Arc<dyn FileSystem>,
    /// Wall and monotonic time (ARQ-138, ARQ-93).
    pub clock: Arc<dyn Clock>,
    /// Best-effort out-of-app notices (02 §6.4).
    pub notifier: Arc<dyn Notifier>,
    /// The licence decision of 02 §4.14.
    pub license: Arc<dyn LicenseVerifier>,
}

impl Ports {
    /// The ports the engine consumes in this delivery.
    #[must_use]
    pub fn new(
        repository: Arc<dyn ProjectRepository>,
        store: Arc<dyn ContentStore>,
        filesystem: Arc<dyn FileSystem>,
        clock: Arc<dyn Clock>,
        notifier: Arc<dyn Notifier>,
        license: Arc<dyn LicenseVerifier>,
    ) -> Self {
        Self {
            repository,
            store,
            filesystem,
            clock,
            notifier,
            license,
        }
    }
}

/// The registries the adapters populate before the engine starts (ARQ-31).
///
/// They carry no global state: the composition root owns them and hands them
/// over, which is what makes a second engine in the same process possible.
#[derive(Default)]
pub struct Registries {
    /// Importers and exporters, filled by the format adapters (02 §4.5).
    pub formats: FormatRegistry,
    /// Codecs and containers, filled by the media adapters (02 §4.8).
    pub codecs: CodecRegistry,
}

impl Registries {
    /// Empty registries; the adapters fill them before `Gafira::new`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// What the caller fixes before the engine starts (design P5).
///
/// The limits arrive from the caller because reading RAM or the environment is
/// `gafira-platform`'s job (§2.1 line 115) and `std::env` is banned here (P1).
#[derive(Debug)]
pub struct EngineConfig {
    data_dir: PathBuf,
    memory_limit: Option<Bytes>,
    export_jobs: u8,
}

impl EngineConfig {
    /// A configuration for `data_dir`, with one export job at a time (02 §6.3,
    /// line 1428) and no memory limit declared.
    ///
    /// Returns [`EngineError::InvalidConfig`] when `data_dir` is empty: an
    /// engine without a data directory cannot persist anything, and defaulting
    /// it to the current directory would hide the mistake.
    pub fn new(data_dir: impl Into<PathBuf>) -> Result<Self, EngineError> {
        let data_dir = data_dir.into();
        if data_dir.as_os_str().is_empty() {
            return Err(EngineError::InvalidConfig {
                detail: "the data directory is empty",
            });
        }
        Ok(Self {
            data_dir,
            memory_limit: None,
            export_jobs: 1,
        })
    }

    /// The directory the engine persists into.
    #[must_use]
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// The memory ceiling this run honours, if the caller declared one.
    ///
    /// `None` means the caller did not declare a limit, not that the process is
    /// unbounded: the governor of `E5` refuses to start without one (design P7).
    #[must_use]
    pub fn memory_limit(&self) -> Option<Bytes> {
        self.memory_limit
    }

    /// How many export jobs may run at once (02 §6.3).
    #[must_use]
    pub fn export_jobs(&self) -> u8 {
        self.export_jobs
    }

    /// Set the memory ceiling for this run.
    #[must_use]
    pub fn with_memory_limit(mut self, limit: Bytes) -> Self {
        self.memory_limit = Some(limit);
        self
    }

    /// Set how many export jobs may run at once.
    #[must_use]
    pub fn with_export_jobs(mut self, jobs: u8) -> Self {
        self.export_jobs = jobs;
        self
    }
}

/// What the engine can fail with in this delivery (design P6).
///
/// The message-key mapping of ADR 0012 (ARQ-102) is **Planned**: `ToMessage`
/// for this enum arrives with the first failing service, `E3b`, and the
/// `UserError` struct is `E11`, after SP3 (design P12). Until then the
/// `Display` text below is a plain sentence and no caller may match on it.
#[derive(Debug)]
pub enum EngineError {
    /// The configuration cannot produce a working engine.
    InvalidConfig {
        /// What is wrong, in one sentence.
        detail: &'static str,
    },
    /// The project repository port failed.
    Repository(RepositoryError),
    /// The content store port failed.
    Store(StoreError),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig { detail } => write!(f, "invalid engine configuration: {detail}"),
            Self::Repository(e) => write!(f, "repository error: {e}"),
            Self::Store(e) => write!(f, "content store error: {e}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<RepositoryError> for EngineError {
    fn from(e: RepositoryError) -> Self {
        Self::Repository(e)
    }
}

impl From<StoreError> for EngineError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

/// The application Facade of 02 §5.4: what the UI and the CLI talk to.
///
/// `E1` fixes the shape only. Each service accessor of §5.4 (1357-1365) arrives
/// with its delivery; until then `Gafira` exposes its parts and nothing else.
pub struct Gafira {
    ports: Ports,
    registries: Registries,
    config: EngineConfig,
    bus: EventBus,
    events: Receiver<EngineEvent>,
}

impl Gafira {
    /// Compose an engine from its ports, its registries, its configuration and
    /// the event channel the composition root built first (design P11,
    /// 02 §5.2 line 1331).
    ///
    /// §5.4 line 1356 lists three parameters; §5.2 needs the fourth, because the
    /// startup sequence publishes before the engine exists (SP4). The bus is
    /// the channel's bus and [`Gafira::events`] is its receiver.
    pub fn new(
        ports: Ports,
        registries: Registries,
        config: EngineConfig,
        events: EventChannel,
    ) -> Result<Self, EngineError> {
        Ok(Self {
            ports,
            registries,
            config,
            bus: events.bus(),
            events: events.receiver(),
        })
    }

    /// The configuration this engine was built with.
    #[must_use]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// The ports this engine was built with.
    #[must_use]
    pub fn ports(&self) -> &Ports {
        &self.ports
    }

    /// The registries this engine was built with.
    #[must_use]
    pub fn registries(&self) -> &Registries {
        &self.registries
    }

    /// The event bus (02 §5.2).
    #[must_use]
    pub fn bus(&self) -> &EventBus {
        &self.bus
    }

    /// The queue the observer drains, at the start of each of its frames
    /// (02 §5.2 line 1335, §5.4 line 1365).
    #[must_use]
    pub fn events(&self) -> &Receiver<EngineEvent> {
        &self.events
    }

    /// Let the bus release the `JobProgress` its window held back.
    pub fn flush_events(&self) {
        self.bus.flush();
    }

    /// Publish one event to the observers of this engine.
    pub fn publish(&self, event: EngineEvent) {
        self.bus.publish(event);
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::time::Duration;

    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::event::{NoopWaker, Waker};
    use gafira_doc::{ContentHash, Document};
    use gafira_ports::common::UnixTime;
    use gafira_ports::license::{LicenseError, LicenseKey, LicenseStatus};
    use gafira_ports::render::BackendKind;
    use gafira_ports::store::*;
    use gafira_ports::system::*;

    use super::*;

    struct NoRepository;

    impl ProjectRepository for NoRepository {
        fn load(
            &self,
            _path: &Path,
        ) -> Result<gafira_ports::repository::LoadedProject, RepositoryError> {
            Err(RepositoryError::Io(io::Error::other("unused")))
        }

        fn save(&self, _path: &Path, _document: &Document) -> Result<(), RepositoryError> {
            Err(RepositoryError::Io(io::Error::other("unused")))
        }
    }

    struct NoStore;

    impl ContentStore for NoStore {
        fn put(&self, _src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
            Err(StoreError::Io(io::Error::other("unused")))
        }

        fn open(&self, _hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
            Err(StoreError::Io(io::Error::other("unused")))
        }

        fn contains(&self, _hash: ContentHash) -> Result<bool, StoreError> {
            Err(StoreError::Io(io::Error::other("unused")))
        }
    }

    struct NoFilesystem;

    impl FileSystem for NoFilesystem {
        fn open(&self, _path: &Path) -> io::Result<Box<dyn ReadSeek + Send>> {
            Err(io::Error::other("unused"))
        }

        fn create(&self, _path: &Path) -> io::Result<Box<dyn FileWrite>> {
            Err(io::Error::other("unused"))
        }

        fn replace(&self, _from: &Path, _to: &Path) -> io::Result<()> {
            Err(io::Error::other("unused"))
        }

        fn remove(&self, _path: &Path) -> io::Result<()> {
            Err(io::Error::other("unused"))
        }

        fn exists(&self, _path: &Path) -> io::Result<bool> {
            Err(io::Error::other("unused"))
        }

        fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
            Err(io::Error::other("unused"))
        }

        fn read_dir(&self, _path: &Path) -> io::Result<Vec<PathBuf>> {
            Err(io::Error::other("unused"))
        }
    }

    struct NoClock;

    impl Clock for NoClock {
        fn now(&self) -> UnixTime {
            UnixTime(0)
        }

        fn monotonic(&self) -> Duration {
            Duration::ZERO
        }
    }

    struct NoNotifier;

    impl Notifier for NoNotifier {
        fn notify(&self, _title: &str, _body: &str) {}
    }

    struct NoLicense;

    impl LicenseVerifier for NoLicense {
        fn status(&self, _now: UnixTime) -> LicenseStatus {
            LicenseStatus::Unlicensed
        }

        fn activate(
            &self,
            _key: &LicenseKey,
            _label: Option<&str>,
        ) -> Result<LicenseStatus, LicenseError> {
            Err(LicenseError::Storage)
        }

        fn validate(&self, _now: UnixTime) -> Result<LicenseStatus, LicenseError> {
            Err(LicenseError::Storage)
        }

        fn deactivate(&self) -> Result<(), LicenseError> {
            Err(LicenseError::Storage)
        }
    }

    fn config() -> EngineConfig {
        EngineConfig::new("/data/gafira").expect("a valid data directory")
    }

    fn six_ports() -> Ports {
        Ports::new(
            Arc::new(NoRepository),
            Arc::new(NoStore),
            Arc::new(NoFilesystem),
            Arc::new(NoClock),
            Arc::new(NoNotifier),
            Arc::new(NoLicense),
        )
    }

    #[derive(Default)]
    struct CountingWaker {
        wakes: AtomicUsize,
    }

    impl Waker for CountingWaker {
        fn wake(&self) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn quiet_channel() -> EventChannel {
        EventChannel::new(Arc::new(NoopWaker), Arc::new(NoClock))
    }

    fn engine_over(events: EventChannel) -> Gafira {
        Gafira::new(six_ports(), Registries::new(), config(), events).expect("an engine")
    }

    /// W5 and E2-C1: publishing through the facade reaches the observer exactly
    /// once and wakes the waker the root built the channel with, once.
    #[test]
    fn w5_publishing_through_the_facade_reaches_the_observer_and_wakes_it() {
        let waker = Arc::new(CountingWaker::default());
        let engine = engine_over(EventChannel::new(waker.clone(), Arc::new(NoClock)));

        engine.publish(EngineEvent::LinksMissing(Vec::new()));

        assert_eq!(
            engine.events().try_recv().expect("the published event"),
            EngineEvent::LinksMissing(Vec::new())
        );
        assert!(engine.events().try_recv().is_err(), "exactly one delivery");
        assert_eq!(waker.wakes.load(Ordering::SeqCst), 1, "one wake");
    }

    /// P11: what the startup sequence published through the channel before the
    /// engine existed is the first thing the engine's `events()` drains.
    #[test]
    fn p11_an_event_published_before_the_engine_exists_is_drained_first() {
        let channel = quiet_channel();
        channel
            .bus()
            .publish(EngineEvent::BackendChanged(BackendKind::Cpu));

        let engine = engine_over(channel);
        engine.publish(EngineEvent::LinksMissing(Vec::new()));

        assert_eq!(
            engine.events().try_recv().expect("the early event"),
            EngineEvent::BackendChanged(BackendKind::Cpu)
        );
        assert_eq!(
            engine.events().try_recv().expect("the later event"),
            EngineEvent::LinksMissing(Vec::new())
        );
    }

    #[test]
    fn ac3_an_empty_data_directory_is_rejected() {
        let error = EngineConfig::new("").expect_err("an empty data directory must be rejected");
        assert!(matches!(error, EngineError::InvalidConfig { .. }));
    }

    #[test]
    fn ac3_the_limits_the_caller_gives_are_kept() {
        let config = config()
            .with_memory_limit(Bytes(1_073_741_824))
            .with_export_jobs(2);
        assert_eq!(config.memory_limit(), Some(Bytes(1_073_741_824)));
        assert_eq!(config.export_jobs(), 2);
        assert_eq!(config.data_dir(), Path::new("/data/gafira"));
    }

    #[test]
    fn ac3_a_fresh_configuration_runs_one_export_job_and_declares_no_limit() {
        let config = config();
        assert_eq!(config.memory_limit(), None);
        assert_eq!(config.export_jobs(), 1);
    }

    #[test]
    fn ac5_the_facade_is_send_and_sync_and_keeps_the_ports_it_was_given() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Gafira>();

        // The exact values handed over below: the facade must hold these
        // allocations and not a copy or a replacement (design P4).
        let repository: Arc<dyn ProjectRepository> = Arc::new(NoRepository);
        let store: Arc<dyn ContentStore> = Arc::new(NoStore);
        let filesystem: Arc<dyn FileSystem> = Arc::new(NoFilesystem);
        let clock: Arc<dyn Clock> = Arc::new(NoClock);
        let notifier: Arc<dyn Notifier> = Arc::new(NoNotifier);
        let license: Arc<dyn LicenseVerifier> = Arc::new(NoLicense);

        let engine = Gafira::new(
            Ports::new(
                Arc::clone(&repository),
                Arc::clone(&store),
                Arc::clone(&filesystem),
                Arc::clone(&clock),
                Arc::clone(&notifier),
                Arc::clone(&license),
            ),
            Registries::new(),
            config(),
            quiet_channel(),
        )
        .expect("an engine");

        let ports = engine.ports();
        assert!(Arc::ptr_eq(&ports.repository, &repository));
        assert!(Arc::ptr_eq(&ports.store, &store));
        assert!(Arc::ptr_eq(&ports.filesystem, &filesystem));
        assert!(Arc::ptr_eq(&ports.clock, &clock));
        assert!(Arc::ptr_eq(&ports.notifier, &notifier));
        assert!(Arc::ptr_eq(&ports.license, &license));

        assert_eq!(engine.config().data_dir(), Path::new("/data/gafira"));
        assert_eq!(engine.registries().formats.importers().count(), 0);
    }
}
