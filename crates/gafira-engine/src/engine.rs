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
/// The `UserError` mapping with message keys of ADR 0012 (ARQ-102) is
/// **Planned** for `E2`: the `Display` text below is a plain sentence and no
/// caller may match on it.
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
}

impl Gafira {
    /// Compose an engine from its ports, its registries and its configuration
    /// (02 §5.4, line 1356).
    ///
    /// The bus starts with the no-op sink; `E2` replaces it with the channel
    /// sink without changing this signature (design P2).
    pub fn new(
        ports: Ports,
        registries: Registries,
        config: EngineConfig,
    ) -> Result<Self, EngineError> {
        Ok(Self {
            ports,
            registries,
            config,
            bus: EventBus::noop(),
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

    /// Publish one event to the observers of this engine.
    pub fn publish(&self, event: EngineEvent) {
        self.bus.publish(event);
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::time::Duration;

    use gafira_doc::{ContentHash, Document};
    use gafira_ports::common::UnixTime;
    use gafira_ports::license::{LicenseError, LicenseKey, LicenseStatus};
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

    fn ports() -> Ports {
        Ports::new(
            Arc::new(NoRepository),
            Arc::new(NoStore),
            Arc::new(NoFilesystem),
            Arc::new(NoClock),
            Arc::new(NoNotifier),
            Arc::new(NoLicense),
        )
    }

    fn config() -> EngineConfig {
        EngineConfig::new("/data/gafira").expect("a valid data directory")
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
    fn ac5_the_facade_is_send_and_sync_and_keeps_its_parts() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Gafira>();

        let engine = Gafira::new(ports(), Registries::new(), config()).expect("an engine");
        assert_eq!(engine.config().data_dir(), Path::new("/data/gafira"));
        assert_eq!(engine.registries().formats.importers().count(), 0);
    }

    #[test]
    fn ac5_publishing_through_the_facade_reaches_the_bus() {
        let engine = Gafira::new(ports(), Registries::new(), config()).expect("an engine");
        engine.publish(EngineEvent::LinksMissing(Vec::new()));
    }
}
