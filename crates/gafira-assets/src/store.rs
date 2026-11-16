//! `FsStore`: one file per content, named by its BLAKE3 hash.
//!
//! The content with hash `h` lives at `<root>/<first 2 hex of h>/<hex of h>`.
//! A `put` streams into a temporary file under `<root>/tmp/`, syncs it and
//! renames it into place, so a reader never sees a partial content and a
//! failed `put` leaves nothing behind.

use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gafira_doc::{AssetRef, ContentHash};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;

/// The size of each read while hashing and copying.
const CHUNK: usize = 64 * 1024;

/// Makes temporary names unique within the process.
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// Content stored by hash under a root folder.
#[derive(Clone, Debug)]
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    /// A store rooted at `root`; the folder is created on the first `put`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The root folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the content with `hash` lives.
    fn path_of(&self, hash: ContentHash) -> PathBuf {
        let hex = AssetRef::new(hash).hex();
        self.root.join(&hex[..2]).join(hex)
    }

    /// Copies `src` into `file` and returns its hash.
    fn copy_hashing(src: &mut dyn Read, file: &mut File) -> io::Result<ContentHash> {
        let mut hasher = blake3::Hasher::new();
        let mut buffer = vec![0_u8; CHUNK];
        loop {
            let n = match src.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            hasher.update(&buffer[..n]);
            file.write_all(&buffer[..n])?;
        }
        file.sync_all()?;
        Ok(ContentHash(*hasher.finalize().as_bytes()))
    }
}

/// A temporary file that is removed unless it was moved into place.
struct Temp {
    path: PathBuf,
    kept: bool,
}

impl Drop for Temp {
    fn drop(&mut self) {
        if !self.kept {
            let _ = fs::remove_file(&self.path);
        }
    }
}

impl ContentStore for FsStore {
    fn put(&self, src: &mut dyn Read) -> Result<ContentHash, StoreError> {
        let tmp_dir = self.root.join("tmp");
        fs::create_dir_all(&tmp_dir).map_err(StoreError::Io)?;
        let name = format!(
            "{}-{}.part",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        );
        let mut temp = Temp {
            path: tmp_dir.join(name),
            kept: false,
        };
        let mut file = File::create_new(&temp.path).map_err(StoreError::Io)?;
        let hash = Self::copy_hashing(src, &mut file).map_err(StoreError::Io)?;
        drop(file);

        let target = self.path_of(hash);
        if target.try_exists().map_err(StoreError::Io)? {
            return Ok(hash);
        }
        if let Some(folder) = target.parent() {
            fs::create_dir_all(folder).map_err(StoreError::Io)?;
        }
        match fs::rename(&temp.path, &target) {
            Ok(()) => temp.kept = true,
            // Another put of the same content won the race (Windows refuses
            // to rename over an existing file); the content is there.
            Err(_) if target.try_exists().unwrap_or(false) => return Ok(hash),
            Err(e) => return Err(StoreError::Io(e)),
        }
        sync_folder(target.parent());
        Ok(hash)
    }

    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        match File::open(self.path_of(hash)) {
            Ok(file) => Ok(Box::new(BufReader::new(file))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(StoreError::NotFound { hash }),
            Err(e) => Err(StoreError::Io(e)),
        }
    }

    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError> {
        self.path_of(hash).try_exists().map_err(StoreError::Io)
    }
}

/// Makes the rename durable where the platform can sync a folder. The
/// content is already synced, so a failure here only risks the name.
#[cfg(unix)]
fn sync_folder(folder: Option<&Path>) {
    if let Some(folder) = folder
        && let Ok(handle) = File::open(folder)
    {
        let _ = handle.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_folder(_folder: Option<&Path>) {}
