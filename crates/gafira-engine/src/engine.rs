//! The Facade of 02 §5.4 (lines 1351-1365) and the values it is built from.
//!
//! `Gafira` composes; it does not implement anything (design P9). The ports come
//! from the composition root (ARQ-06), the registries are the ones the adapters
//! filled before the engine started (ARQ-31), and the configuration comes from
//! the caller, never from the environment (design P5).

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use gafira_ports::common::Bytes;
use gafira_ports::format::FormatRegistry;
use gafira_ports::i18n::LocaleResolver;
use gafira_ports::license::{LicenseError, LicenseVerifier};
use gafira_ports::media::CodecRegistry;
use gafira_ports::repository::{ProjectRepository, RepositoryError};
use gafira_ports::settings::{SettingsError, SettingsRepository};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{Clock, FileSystem, Notifier};

use crossbeam_channel::Receiver;
use gafira_doc::ValidationError;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_edit::CommandError;

use crate::bus::EventChannel;
use crate::edit::EditService;
use crate::event::{EngineEvent, EventBus};
use crate::license::LicenseService;
use crate::project::ProjectService;
use crate::session::Shared;
use crate::settings::SettingsService;

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
    /// Where the settings persist and how their language resolves (spec 009).
    pub settings: SettingsPorts,
}

/// The two ports `SettingsService` needs, grouped so [`Ports::new`] stays
/// within clippy's seven arguments (spec 009).
pub struct SettingsPorts {
    /// Loads and saves the user's settings (`ports::SettingsRepository`).
    pub repository: Arc<dyn SettingsRepository>,
    /// Maps the language preference to a locale (`gafira-i18n`, 02 §14.4).
    pub resolver: Arc<dyn LocaleResolver>,
}

impl SettingsPorts {
    /// The settings store and the locale resolver.
    #[must_use]
    pub fn new(repository: Arc<dyn SettingsRepository>, resolver: Arc<dyn LocaleResolver>) -> Self {
        Self {
            repository,
            resolver,
        }
    }
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
        settings: SettingsPorts,
    ) -> Self {
        Self {
            repository,
            store,
            filesystem,
            clock,
            notifier,
            license,
            settings,
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

/// The autosave interval unless the caller sets another (02 §7.2).
const DEFAULT_AUTOSAVE_INTERVAL: Duration = Duration::from_secs(10);
/// The shortest and the longest autosave interval, both allowed (02 §7.2).
const AUTOSAVE_INTERVALS: std::ops::RangeInclusive<Duration> =
    Duration::from_secs(5)..=Duration::from_secs(15);

/// What the caller fixes before the engine starts (design P5).
///
/// The limits arrive from the caller because reading RAM or the environment is
/// `gafira-platform`'s job (§2.1 line 115) and `std::env` is banned here (P1).
#[derive(Debug)]
pub struct EngineConfig {
    data_dir: PathBuf,
    memory_limit: Option<Bytes>,
    export_jobs: u8,
    autosave_interval: Duration,
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
            autosave_interval: DEFAULT_AUTOSAVE_INTERVAL,
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

    /// How long a change waits before it is autosaved (02 §7.2, design P16).
    #[must_use]
    pub fn autosave_interval(&self) -> Duration {
        self.autosave_interval
    }

    /// Set the autosave interval: 5 s to 15 s, both included (02 §7.2,
    /// FIJADA; design P16).
    ///
    /// Returns [`EngineError::InvalidConfig`] for any other duration.
    pub fn with_autosave_interval(mut self, interval: Duration) -> Result<Self, EngineError> {
        if !AUTOSAVE_INTERVALS.contains(&interval) {
            return Err(EngineError::InvalidConfig {
                detail: "the autosave interval is outside 5 s to 15 s",
            });
        }
        self.autosave_interval = interval;
        Ok(self)
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

/// What the engine can fail with (design P6, P16).
///
/// Every variant has a message key in the area `error.engine.`, and the
/// wrapped port enums delegate to their own key (ARQ-102, ADR 0012, ADR 0014;
/// design P12). The `Display` text is for the local log only. `E3c` to `E10`
/// add variants, so the enum is `#[non_exhaustive]`.
#[non_exhaustive]
#[derive(Debug)]
pub enum EngineError {
    /// The configuration cannot produce a working engine.
    InvalidConfig {
        /// What is wrong, in one sentence, for the log.
        detail: &'static str,
    },
    /// The project repository port failed.
    Repository(RepositoryError),
    /// The content store port failed.
    Store(StoreError),
    /// An edit or undo arrived with no open document (design P15).
    NoSession,
    /// A command could not be applied or reverted (ARQ-30).
    Command(CommandError),
    /// A document the engine built or received is not valid.
    InvalidDocument(ValidationError),
    /// `save` on a session whose file has a newer schema (ARQ-55): edits
    /// stay in memory and only `save_as` (`E4`) keeps them.
    ReadOnly,
    /// `save` on a session that was never saved (`new`); its exit is
    /// `save_as` (`E4`).
    NoPath,
    /// A `FileSystem` call of the engine failed (design P16).
    Io(std::io::Error),
    /// The settings port failed (spec 009).
    Settings(SettingsError),
    /// The licence port failed or refused (spec 010).
    License(LicenseError),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig { detail } => write!(f, "invalid engine configuration: {detail}"),
            Self::Repository(e) => write!(f, "repository error: {e}"),
            Self::Store(e) => write!(f, "content store error: {e}"),
            Self::NoSession => f.write_str("no document is open"),
            Self::Command(e) => write!(f, "command failed: {e}"),
            Self::InvalidDocument(e) => write!(f, "invalid document: {e}"),
            Self::ReadOnly => f.write_str("the project is read-only"),
            Self::NoPath => f.write_str("the project has no file yet"),
            Self::Io(e) => write!(f, "file system error: {e}"),
            Self::Settings(e) => write!(f, "settings error: {e}"),
            Self::License(e) => write!(f, "licence error: {e}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl ToMessage for EngineError {
    /// `CommandError` and `ValidationError` have no `ToMessage` yet, so their
    /// variants get an engine key without delegation (design P12, SP3(b)).
    /// `InvalidConfig` takes no argument: its detail is prose for the log, and
    /// neither does `Io`: an `io::Error` carries only prose (ARQ-102).
    fn to_message(&self) -> Message {
        match self {
            Self::InvalidConfig { .. } => msg!("error.engine.invalid_config"),
            Self::Repository(e) => e.to_message(),
            Self::Store(e) => e.to_message(),
            Self::NoSession => msg!("error.engine.no_session"),
            Self::Command(_) => msg!("error.engine.command_failed"),
            Self::InvalidDocument(_) => msg!("error.engine.invalid_document"),
            Self::ReadOnly => msg!("error.engine.read_only"),
            Self::NoPath => msg!("error.engine.no_path"),
            Self::Io(_) => msg!("error.engine.io"),
            Self::Settings(e) => e.to_message(),
            Self::License(e) => e.to_message(),
        }
    }
}

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

impl From<CommandError> for EngineError {
    fn from(e: CommandError) -> Self {
        Self::Command(e)
    }
}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<LicenseError> for EngineError {
    fn from(e: LicenseError) -> Self {
        Self::License(e)
    }
}

impl From<SettingsError> for EngineError {
    fn from(e: SettingsError) -> Self {
        Self::Settings(e)
    }
}

impl From<ValidationError> for EngineError {
    fn from(e: ValidationError) -> Self {
        Self::InvalidDocument(e)
    }
}

/// The application Facade of 02 §5.4: what the UI and the CLI talk to.
///
/// Each service accessor of §5.4 (1357-1365) arrives with its delivery. The
/// document version is the engine's (owner decision O2): both services share
/// it with the session, and it lives as long as `Gafira`.
pub struct Gafira {
    ports: Ports,
    registries: Registries,
    config: EngineConfig,
    bus: EventBus,
    events: Receiver<EngineEvent>,
    projects: ProjectService,
    edit: EditService,
    settings: SettingsService,
    license: LicenseService,
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
        let bus = events.bus();
        let shared = Shared::new(bus.clone());
        let edit = EditService::new(shared.clone(), Arc::clone(&ports.clock));
        let projects = ProjectService::over(shared, &ports, &config);
        let settings = SettingsService::over(&ports.settings, bus.clone());
        let license = LicenseService::over(
            Arc::clone(&ports.license),
            Arc::clone(&ports.clock),
            bus.clone(),
        );
        Ok(Self {
            ports,
            registries,
            config,
            bus,
            events: events.receiver(),
            projects,
            edit,
            settings,
            license,
        })
    }

    /// The session's lifecycle (02 §5.4 line 1357): `new_document`, `open`,
    /// `save`, `is_modified`, `autosave_due` and `autosave_finished` so far.
    #[must_use]
    pub fn projects(&self) -> &ProjectService {
        &self.projects
    }

    /// The edits, undo, redo and gestures (02 §5.4 line 1359).
    pub fn edit(&mut self) -> &mut EditService {
        &mut self.edit
    }

    /// The settings and the interface language (02 §5.4 line 1364, spec 009).
    /// Nothing is loaded until [`SettingsService::start`].
    #[must_use]
    pub fn settings(&self) -> &SettingsService {
        &self.settings
    }

    /// The licence state and its actions (02 §5.4 line 1363, spec 010).
    /// Whether a licence is required at all is the injected verifier's
    /// choice (D-58).
    #[must_use]
    pub fn license(&self) -> &LicenseService {
        &self.license
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::event::Waker;
    use crate::fakes::*;
    use gafira_doc::ContentHash;
    use gafira_ports::render::BackendKind;

    use super::*;

    #[derive(Default)]
    struct CountingWaker {
        wakes: AtomicUsize,
    }

    impl Waker for CountingWaker {
        fn wake(&self) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
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

    // ARQ-102
    #[test]
    fn ac9_every_engine_error_has_its_key_and_the_port_errors_delegate() {
        let too_new = || RepositoryError::TooNew {
            found: 3,
            supported: 1,
        };
        let not_found = || StoreError::NotFound {
            hash: ContentHash([7; 32]),
        };
        let cases = [
            (
                EngineError::InvalidConfig { detail: "x" },
                msg!("error.engine.invalid_config"),
            ),
            (EngineError::Repository(too_new()), too_new().to_message()),
            (EngineError::Store(not_found()), not_found().to_message()),
            (EngineError::NoSession, msg!("error.engine.no_session")),
            (
                EngineError::Command(CommandError::NotApplied),
                msg!("error.engine.command_failed"),
            ),
            (
                EngineError::InvalidDocument(ValidationError::InvalidDpi { dpi: 0 }),
                msg!("error.engine.invalid_document"),
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_message(), expected, "{error:?}");
        }
        assert_eq!(
            EngineError::Repository(too_new()).to_message().key.0,
            "error.repository.too_new"
        );
    }

    // ARQ-55, ARQ-102
    #[test]
    fn ac11_read_only_no_path_and_io_have_engine_keys_without_arguments() {
        let cases = [
            (EngineError::ReadOnly, msg!("error.engine.read_only")),
            (EngineError::NoPath, msg!("error.engine.no_path")),
            (
                EngineError::from(std::io::Error::other("disk full")),
                msg!("error.engine.io"),
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_message(), expected, "{error:?}");
        }
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

    // E3d AC1, ARQ-51
    #[test]
    fn e3d_ac1_the_autosave_interval_is_5_to_15_seconds_inclusive_and_10_by_default() {
        assert_eq!(config().autosave_interval(), Duration::from_secs(10));
        let cases = [
            (Duration::from_millis(4_999), false),
            (Duration::from_secs(5), true),
            (Duration::from_secs(15), true),
            (Duration::from_millis(15_001), false),
        ];
        for (interval, accepted) in cases {
            let result = config().with_autosave_interval(interval);
            match result {
                Ok(config) => {
                    assert!(accepted, "{interval:?} accepted");
                    assert_eq!(config.autosave_interval(), interval);
                }
                Err(error) => {
                    assert!(!accepted, "{interval:?} rejected");
                    assert!(matches!(error, EngineError::InvalidConfig { .. }));
                }
            }
        }
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
                no_settings(),
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
