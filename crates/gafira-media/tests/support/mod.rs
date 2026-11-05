//! The doubles the tests of the crate inject (ARQ-112, ARQ-138).
//!
//! The media doubles of the testkit cover the ports of the media engine;
//! the file system, the pool and the clock are here, because the testkit
//! keeps the file system and the clock as stubs.

use std::collections::BTreeMap;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use gafira_media::{Job, WorkerPool};
use gafira_ports::common::UnixTime;
use gafira_ports::system::{Clock, FileSystem, FileWrite, ReadSeek};
use gafira_testkit::ports::container::{FakeContainer, FakePacket};

/// The path the medium of a test lives in.
pub const MEDIUM: &str = "medium.json";

/// The reference material of the specs: 120 frames of 6 s at 20 fps, with a
/// keyframe every 24 frames.
pub fn reference_packets() -> Vec<FakePacket> {
    (0..120)
        .map(|frame| {
            let ms = frame * 50;
            if frame % 24 == 0 {
                FakePacket::Keyframe(ms)
            } else {
                FakePacket::Frame(ms)
            }
        })
        .collect()
}

/// The bytes of the reference medium.
pub fn reference_medium() -> Vec<u8> {
    FakeContainer::medium(1920, 1080, 20, 6_000, &reference_packets())
}

/// The file system of a test: it holds the files of the test and records
/// every call it receives.
#[derive(Debug, Default)]
pub struct FakeFileSystem {
    files: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    calls: Mutex<Vec<String>>,
}

impl FakeFileSystem {
    /// A file system with no file.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A file system with one file.
    #[must_use]
    pub fn with_file(path: &str, bytes: Vec<u8>) -> Self {
        let file_system = Self::new();
        file_system
            .files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(PathBuf::from(path), bytes);
        file_system
    }

    /// The calls the file system received, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Records one call.
    fn record(&self, call: &str) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call.to_string());
    }
}

impl FileSystem for FakeFileSystem {
    /// Opens a file of the test.
    fn open(&self, path: &Path) -> io::Result<Box<dyn ReadSeek + Send>> {
        self.record("open");
        let files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        let bytes = files.get(path).cloned().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "the test holds no such file")
        })?;
        Ok(Box::new(Cursor::new(bytes)))
    }

    /// The double refuses to write: the media engine writes no file.
    fn create(&self, _path: &Path) -> io::Result<Box<dyn FileWrite>> {
        self.record("create");
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "the double of the testkit writes no file",
        ))
    }

    /// The double refuses to move a file.
    fn replace(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        self.record("replace");
        Ok(())
    }

    /// The double refuses to remove a file.
    fn remove(&self, _path: &Path) -> io::Result<()> {
        self.record("remove");
        Ok(())
    }

    /// Whether the test holds the file.
    fn exists(&self, path: &Path) -> io::Result<bool> {
        self.record("exists");
        let files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(files.contains_key(path))
    }

    /// The double refuses to make a directory.
    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        self.record("create_dir_all");
        Ok(())
    }

    /// The entries of the directory: the test holds one file per path.
    fn read_dir(&self, _path: &Path) -> io::Result<Vec<PathBuf>> {
        self.record("read_dir");
        let files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(files.keys().cloned().collect())
    }
}

/// The pool of a test: it runs the jobs in the reverse order, so a result
/// that depends on the order the jobs finish in fails (ARQ-140).
#[derive(Clone, Copy, Debug)]
pub struct FakePool {
    workers: usize,
}

impl FakePool {
    /// A pool with this many workers.
    #[must_use]
    pub fn new(workers: usize) -> Self {
        Self { workers }
    }
}

impl WorkerPool for FakePool {
    /// The workers of the pool.
    fn workers(&self) -> usize {
        self.workers
    }

    /// Runs every job, the last one first.
    fn execute_all(&self, mut jobs: Vec<Job>) {
        jobs.reverse();
        for job in jobs {
            job();
        }
    }
}

/// The clock of a test: it advances by a fixed step on every reading.
#[derive(Debug)]
pub struct FakeClock {
    nanos: AtomicU64,
    step: u64,
}

impl FakeClock {
    /// A clock that advances `step` on every reading.
    #[must_use]
    pub fn new(step: Duration) -> Self {
        Self {
            nanos: AtomicU64::new(0),
            step: u64::try_from(step.as_nanos()).unwrap_or(u64::MAX),
        }
    }
}

impl Clock for FakeClock {
    /// The wall time of the test is always the epoch.
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    /// The time the test has advanced.
    fn monotonic(&self) -> Duration {
        Duration::from_nanos(self.nanos.fetch_add(self.step, Ordering::Relaxed))
    }
}
