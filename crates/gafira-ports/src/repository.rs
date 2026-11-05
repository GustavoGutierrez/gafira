//! Project repositories (02 §5.3). `ProjectRepository` works on `Document`
//! until `Project` lands (02 §3.1, S15).

use std::fmt;
use std::io;
use std::path::Path;

use gafira_doc::Document;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

/// A document read from storage.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedProject {
    /// The document.
    pub document: Document,
    /// True when it must not be saved over: its schema is newer (ARQ-55).
    pub read_only: bool,
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

/// Loads and saves projects (`.gafira`). Adapters implement it (`gafira-format`).
pub trait ProjectRepository: Send + Sync {
    /// Reads the project at `path`.
    ///
    /// # Errors
    /// [`RepositoryError::Corrupt`] when it cannot be read,
    /// [`RepositoryError::Io`] when the storage fails.
    /// A newer schema is not an error: it loads with `read_only` set (ARQ-55).
    fn load(&self, path: &Path) -> Result<LoadedProject, RepositoryError>;

    /// Writes `document` at `path`.
    ///
    /// # Errors
    /// [`RepositoryError::TooNew`] when `path` holds a newer schema that must
    /// not be overwritten, [`RepositoryError::Io`] when the storage fails.
    fn save(&self, path: &Path, document: &Document) -> Result<(), RepositoryError>;
}
