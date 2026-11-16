//! Project repositories (02 §5.3). `ProjectRepository` works on `Document`
//! until `Project` lands (02 §3.1, S15). A project carries its versions
//! (02 §7.1 «Versiones», D-67, spec 031).

use std::fmt;
use std::io;
use std::path::Path;
use std::sync::Arc;

use gafira_doc::Document;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

use crate::common::UnixTime;

/// Names one version inside a project file (02 §7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionId(pub u64);

/// Who made a version (02 §7.1): the person, or a save.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionKind {
    /// Made with «Guardar versión»; never removed automatically.
    Named,
    /// Made by a save; only the newest ones are kept.
    Auto,
}

/// What `versions/index.json` says of one version (02 §7.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionInfo {
    pub id: VersionId,
    /// The person's name for it; empty for an automatic one.
    pub name: String,
    pub kind: VersionKind,
    /// When it was made.
    pub created: UnixTime,
    /// The version of the app that made it.
    pub app: String,
    /// The schema of its document.
    pub schema: u32,
}

/// One version: what it is and the document of that moment.
#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    pub info: VersionInfo,
    pub document: Arc<Document>,
}

/// A document read from storage, with its versions.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedProject {
    /// The document.
    pub document: Document,
    /// True when it must not be saved over: its schema is newer (ARQ-55).
    pub read_only: bool,
    /// The versions the file holds, in the order it lists them (spec 031).
    pub versions: Vec<Version>,
}

/// Why a [`ProjectRepository`] call failed.
#[derive(Debug)]
pub enum RepositoryError {
    /// The stored project cannot be read. `detail` is library text.
    Corrupt { detail: String },
    /// The stored schema is newer than this build supports.
    TooNew { found: u32, supported: u32 },
    /// The backing storage failed.
    Io(io::Error),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Corrupt { detail } => write!(f, "corrupt project: {detail}"),
            Self::TooNew { found, supported } => {
                write!(f, "schema {found} is newer than supported {supported}")
            }
            Self::Io(e) => write!(f, "repository I/O error: {e}"),
        }
    }
}

impl std::error::Error for RepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Corrupt { .. } | Self::TooNew { .. } => None,
            Self::Io(e) => Some(e),
        }
    }
}

impl ToMessage for RepositoryError {
    fn to_message(&self) -> Message {
        match self {
            Self::Corrupt { .. } => msg!("error.repository.corrupt"),
            Self::TooNew { found, supported } => msg!(
                "error.repository.too_new",
                found = Count(u64::from(*found)),
                supported = Count(u64::from(*supported))
            ),
            Self::Io(_) => msg!("error.repository.io"),
        }
    }
}

/// Loads and saves projects (`.gaf`). Adapters implement it (`gafira-format`).
pub trait ProjectRepository: Send + Sync {
    /// Reads the project at `path`.
    ///
    /// # Errors
    /// [`RepositoryError::Corrupt`] when it cannot be read,
    /// [`RepositoryError::Io`] when the storage fails.
    /// A newer schema is not an error: it loads with `read_only` set (ARQ-55).
    fn load(&self, path: &Path) -> Result<LoadedProject, RepositoryError>;

    /// Writes `document` and its `versions` at `path` and leaves the file
    /// synced (`fsync`) before it returns `Ok` (02 §7.2 step 2, ARQ-50).
    /// The file holds exactly `versions`, in that order: the engine decides
    /// which ones it keeps (spec 031).
    ///
    /// The engine passes a temporary path in the target's folder and, only
    /// after this returns `Ok`, renames it over the target and syncs the
    /// folder with `FileSystem::replace`; the rename is not the adapter's.
    ///
    /// # Errors
    /// [`RepositoryError::TooNew`] when `path` holds a newer schema that must
    /// not be overwritten, [`RepositoryError::Io`] when the storage fails.
    fn save(
        &self,
        path: &Path,
        document: &Document,
        versions: &[Version],
    ) -> Result<(), RepositoryError>;
}
