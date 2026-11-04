//! The file system the tests of the proxy store inject (ARQ-50, ARQ-138).
//!
//! The file system of `tests/support/mod.rs` refuses to write: it proves that
//! opening a medium writes no file (ARQ-144). The store of the light copies
//! writes by design, so it needs a double that keeps the files it creates and
//! the ones it moves.

use std::collections::BTreeMap;
use std::io::{self, Cursor, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use gafira_ports::system::{FileSystem, FileWrite, ReadSeek};

/// The file system of a test: it holds the files of the test and records the
/// calls it receives.
#[derive(Debug, Default)]
pub struct FakeFileSystem {
    files: Arc<Mutex<BTreeMap<PathBuf, Vec<u8>>>>,
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

    /// The files the file system holds now, in order.
    #[must_use]
    pub fn paths(&self) -> Vec<PathBuf> {
        self.files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .keys()
            .cloned()
            .collect()
    }

    /// The bytes of a file of the test.
    #[must_use]
    pub fn read(&self, path: &Path) -> Option<Vec<u8>> {
        self.files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
            .cloned()
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

    /// Creates or truncates a file of the test.
    fn create(&self, path: &Path) -> io::Result<Box<dyn FileWrite>> {
        self.record("create");
        self.files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path.to_path_buf(), Vec::new());
        Ok(Box::new(FakeFile {
            files: Arc::clone(&self.files),
            path: path.to_path_buf(),
            position: 0,
        }))
    }

    /// Moves `from` over `to`, as the store does at the end of a generation.
    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.record("replace");
        let mut files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        let bytes = files.remove(from).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "the test holds no such file")
        })?;
        files.insert(to.to_path_buf(), bytes);
        Ok(())
    }

    /// Removes a file of the test. A missing file is not an error.
    fn remove(&self, path: &Path) -> io::Result<()> {
        self.record("remove");
        self.files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(path);
        Ok(())
    }

    /// Whether the test holds the file.
    fn exists(&self, path: &Path) -> io::Result<bool> {
        self.record("exists");
        let files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(files.contains_key(path))
    }

    /// Creates a directory: the test has no directory of its own.
    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        self.record("create_dir_all");
        Ok(())
    }

    /// The entries of the directory `path`: the files it holds.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        self.record("read_dir");
        let files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(files
            .keys()
            .filter(|file| file.parent() == Some(path))
            .cloned()
            .collect())
    }
}

/// A file of the test opened for writing.
struct FakeFile {
    files: Arc<Mutex<BTreeMap<PathBuf, Vec<u8>>>>,
    path: PathBuf,
    position: u64,
}

impl Write for FakeFile {
    /// Writes `buf` at the position of the file, growing it if needed.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut files = self.files.lock().unwrap_or_else(PoisonError::into_inner);
        let bytes = files.entry(self.path.clone()).or_default();
        let start = usize::try_from(self.position).unwrap_or(0);
        let end = start.saturating_add(buf.len());
        if bytes.len() < end {
            bytes.resize(end, 0);
        }
        if let Some(slot) = bytes.get_mut(start..end) {
            slot.copy_from_slice(buf);
        }
        self.position = u64::try_from(end).unwrap_or(u64::MAX);
        Ok(buf.len())
    }

    /// Nothing is buffered.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for FakeFile {
    /// Moves the position of the file.
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let length = self
            .files
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&self.path)
            .map_or(0, |bytes| u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        let next = match position {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => length.saturating_add_signed(offset),
            SeekFrom::Current(offset) => self.position.saturating_add_signed(offset),
        };
        self.position = next;
        Ok(next)
    }
}

impl FileWrite for FakeFile {
    /// Nothing is buffered, so there is nothing to flush to the device.
    fn sync_all(&mut self) -> io::Result<()> {
        Ok(())
    }
}
