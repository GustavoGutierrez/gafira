//! The fake file system of ARQ-112: the port double a test injects in place
//! of the real one (02 §7.2).

use std::io;
use std::path::{Path, PathBuf};

use gafira_ports::system::{FileSystem, FileWrite, ReadSeek};

/// The fake file system. A stub: the media stream fills it.
pub struct FakeFileSystem;

impl FileSystem for FakeFileSystem {
    /// Not filled yet.
    fn open(&self, _path: &Path) -> io::Result<Box<dyn ReadSeek + Send>> {
        panic!("stub: the file system double is not filled yet")
    }

    /// Not filled yet.
    fn create(&self, _path: &Path) -> io::Result<Box<dyn FileWrite>> {
        panic!("stub: the file system double is not filled yet")
    }

    /// Not filled yet.
    fn replace(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        panic!("stub: the file system double is not filled yet")
    }

    /// Not filled yet.
    fn remove(&self, _path: &Path) -> io::Result<()> {
        panic!("stub: the file system double is not filled yet")
    }

    /// Not filled yet.
    fn exists(&self, _path: &Path) -> io::Result<bool> {
        panic!("stub: the file system double is not filled yet")
    }

    /// Not filled yet.
    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        panic!("stub: the file system double is not filled yet")
    }

    /// Not filled yet.
    fn read_dir(&self, _path: &Path) -> io::Result<Vec<PathBuf>> {
        panic!("stub: the file system double is not filled yet")
    }
}
