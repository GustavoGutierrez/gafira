//! Private doubles of the ports for the engine's own tests (design P16).
//!
//! They stay inside the crate until a second crate needs the same doubles.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use gafira_doc::{ContentHash, Document};
use gafira_ports::common::UnixTime;
use gafira_ports::license::{LicenseError, LicenseKey, LicenseStatus, LicenseVerifier};
use gafira_ports::repository::{LoadedProject, ProjectRepository, RepositoryError};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{Clock, FileSystem, FileWrite, Notifier, ReadSeek};

use crate::bus::EventChannel;
use crate::engine::{EngineConfig, Gafira, Ports, Registries};
use crate::event::NoopWaker;

pub(crate) struct NoRepository;

impl ProjectRepository for NoRepository {
    fn load(&self, _path: &Path) -> Result<LoadedProject, RepositoryError> {
        Err(RepositoryError::Io(io::Error::other("unused")))
    }

    fn save(&self, _path: &Path, _document: &Document) -> Result<(), RepositoryError> {
        Err(RepositoryError::Io(io::Error::other("unused")))
    }
}

pub(crate) struct NoStore;

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

pub(crate) struct NoFilesystem;

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

pub(crate) struct NoClock;

impl Clock for NoClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

pub(crate) struct NoNotifier;

impl Notifier for NoNotifier {
    fn notify(&self, _title: &str, _body: &str) {}
}

pub(crate) struct NoLicense;

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

pub(crate) fn config() -> EngineConfig {
    EngineConfig::new("/data/gafira").expect("a valid data directory")
}

pub(crate) fn six_ports() -> Ports {
    Ports::new(
        Arc::new(NoRepository),
        Arc::new(NoStore),
        Arc::new(NoFilesystem),
        Arc::new(NoClock),
        Arc::new(NoNotifier),
        Arc::new(NoLicense),
    )
}

/// A channel nobody wakes, over the zero clock.
pub(crate) fn quiet_channel() -> EventChannel {
    EventChannel::new(Arc::new(NoopWaker), Arc::new(NoClock))
}

/// An engine over the fakes that publishes into `events`.
pub(crate) fn engine_over(events: EventChannel) -> Gafira {
    Gafira::new(six_ports(), Registries::new(), config(), events).expect("an engine")
}
