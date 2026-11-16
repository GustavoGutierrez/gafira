//! File system, clock and notifier ports (02 §6.4, §7.2).

use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use gafira_doc::ProjectId;

use crate::common::UnixTime;

/// A readable, seekable source. Every `Read + Seek` type is one.
pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek + ?Sized> ReadSeek for T {}

/// A writable, seekable sink. Every `Write + Seek` type is one.
pub trait WriteSeek: Write + Seek {}

impl<T: Write + Seek + ?Sized> WriteSeek for T {}

/// A file opened for writing.
pub trait FileWrite: Write + Seek + Send {
    /// Flushes the data and metadata to the device.
    ///
    /// # Errors
    /// Any I/O error of the sync.
    fn sync_all(&mut self) -> std::io::Result<()>;
}

/// The file operations the engine needs, so tests can inject a double (ARQ-50).
pub trait FileSystem: Send + Sync {
    /// Opens `path` for reading.
    ///
    /// # Errors
    /// Any I/O error of the open.
    fn open(&self, path: &Path) -> std::io::Result<Box<dyn ReadSeek + Send>>;

    /// Creates or truncates `path` for writing.
    ///
    /// # Errors
    /// Any I/O error of the create.
    fn create(&self, path: &Path) -> std::io::Result<Box<dyn FileWrite>>;

    /// Moves `from` over `to` atomically, per OS (02 §7.2 step 3), and syncs
    /// the directory.
    ///
    /// # Errors
    /// Any I/O error of the rename or the directory sync.
    fn replace(&self, from: &Path, to: &Path) -> std::io::Result<()>;

    /// Removes the file at `path`.
    ///
    /// # Errors
    /// Any I/O error of the removal.
    fn remove(&self, path: &Path) -> std::io::Result<()>;

    /// True when `path` exists.
    ///
    /// # Errors
    /// Any I/O error of the lookup.
    fn exists(&self, path: &Path) -> std::io::Result<bool>;

    /// Creates `path` and every missing parent.
    ///
    /// # Errors
    /// Any I/O error of the create.
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;

    /// The entries of the directory `path`.
    ///
    /// # Errors
    /// Any I/O error of the listing.
    fn read_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>>;
}

/// Wall and monotonic time, injectable (ARQ-93).
pub trait Clock: Send + Sync {
    /// Seconds since the Unix epoch.
    fn now(&self) -> UnixTime;

    /// Time since an arbitrary fixed start; never goes back.
    fn monotonic(&self) -> Duration;
}

/// Mints project identities (D-63, spec 012), so the engine needs no
/// randomness of its own (ARQ-02). The platform adapter returns a random
/// RFC 9562 version-4 UUID; it never returns [`ProjectId::NIL`].
pub trait IdSource: Send + Sync {
    /// A fresh identity, distinct from every earlier one.
    fn project_id(&self) -> ProjectId;
}

/// Shows a message outside the app, best effort. The text is already
/// translated (02 §6.4).
pub trait Notifier: Send + Sync {
    fn notify(&self, title: &str, body: &str);
}
