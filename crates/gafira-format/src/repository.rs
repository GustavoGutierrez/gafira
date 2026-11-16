//! `ProjectRepository` over real files (02 §5.3, §7.2 steps 1-2).

use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;

use gafira_doc::{ContentHash, Document};
use gafira_ports::repository::{
    LoadedProject, ProjectRepository, RepositoryError, Thumbnail, Version,
};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::ReadSeek;

use crate::{Limits, read_gaf, write_gaf};

/// Reads and writes `.gaf` files. The bytes of the embedded assets come
/// from `store` on save and go into it on load (spec 026).
#[derive(Clone)]
pub struct GafRepository {
    limits: Limits,
    store: Arc<dyn ContentStore>,
}

impl GafRepository {
    #[must_use]
    pub fn new(limits: Limits, store: Arc<dyn ContentStore>) -> Self {
        Self { limits, store }
    }
}

impl fmt::Debug for GafRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GafRepository")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

/// The default limits and no content store: it saves and opens only
/// documents without embedded assets.
impl Default for GafRepository {
    fn default() -> Self {
        Self::new(Limits::default(), Arc::new(NoStore))
    }
}

/// A store that holds nothing and accepts nothing.
struct NoStore;

impl ContentStore for NoStore {
    fn put(&self, _src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        Err(StoreError::Io(io::Error::new(
            io::ErrorKind::Unsupported,
            "this repository has no content store",
        )))
    }

    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::NotFound { hash })
    }

    fn contains(&self, _hash: ContentHash) -> Result<bool, StoreError> {
        Ok(false)
    }
}

impl ProjectRepository for GafRepository {
    fn load(&self, path: &Path) -> Result<LoadedProject, RepositoryError> {
        let file = File::open(path).map_err(RepositoryError::Io)?;
        read_gaf(BufReader::new(file), self.limits, self.store.as_ref())
    }

    /// [`GafRepository::save_with_thumbnail`] without a thumbnail.
    fn save(
        &self,
        path: &Path,
        document: &Document,
        versions: &[Version],
    ) -> Result<(), RepositoryError> {
        self.save_with_thumbnail(path, document, versions, None)
    }

    /// Writes the whole file at `path` (the engine's temporary), with the
    /// thumbnail in `thumbs/` (spec 033), and syncs it before returning
    /// (SP6); the rename over the target is the engine's.
    fn save_with_thumbnail(
        &self,
        path: &Path,
        document: &Document,
        versions: &[Version],
        thumbnail: Option<&Thumbnail>,
    ) -> Result<(), RepositoryError> {
        let file = File::create(path).map_err(RepositoryError::Io)?;
        let mut sink = BufWriter::new(file);
        write_gaf(
            &mut sink,
            document,
            versions,
            thumbnail,
            self.store.as_ref(),
        )?;
        let file = sink
            .into_inner()
            .map_err(|e| RepositoryError::Io(e.into_error()))?;
        file.sync_all().map_err(RepositoryError::Io)
    }
}
