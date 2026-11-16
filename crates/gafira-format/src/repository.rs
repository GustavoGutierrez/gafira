//! `ProjectRepository` over real files (02 §5.3, §7.2 steps 1-2).

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use gafira_doc::Document;
use gafira_ports::repository::{LoadedProject, ProjectRepository, RepositoryError};

use crate::{Limits, read_gaf, write_gaf};

/// Reads and writes `.gaf` files.
#[derive(Clone, Copy, Debug, Default)]
pub struct GafRepository {
    limits: Limits,
}

impl GafRepository {
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Self { limits }
    }
}

impl ProjectRepository for GafRepository {
    fn load(&self, path: &Path) -> Result<LoadedProject, RepositoryError> {
        let file = File::open(path).map_err(RepositoryError::Io)?;
        read_gaf(BufReader::new(file), self.limits)
    }

    /// Writes the whole file at `path` (the engine's temporary) and syncs it
    /// before returning (SP6); the rename over the target is the engine's.
    fn save(&self, path: &Path, document: &Document) -> Result<(), RepositoryError> {
        let file = File::create(path).map_err(RepositoryError::Io)?;
        let mut sink = BufWriter::new(file);
        write_gaf(&mut sink, document)?;
        let file = sink
            .into_inner()
            .map_err(|e| RepositoryError::Io(e.into_error()))?;
        file.sync_all().map_err(RepositoryError::Io)
    }
}
