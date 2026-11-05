//! The content-addressed store port (02 §5.3).

use std::fmt;
use std::io;

use gafira_doc::ContentHash;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

use crate::system::ReadSeek;

/// Why a [`ContentStore`] call failed.
#[derive(Debug)]
pub enum StoreError {
    /// No content has this hash.
    NotFound { hash: ContentHash },
    /// The backing storage failed.
    Io(io::Error),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { .. } => f.write_str("content not found"),
            Self::Io(e) => write!(f, "store I/O error: {e}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotFound { .. } => None,
            Self::Io(e) => Some(e),
        }
    }
}

impl ToMessage for StoreError {
    fn to_message(&self) -> Message {
        match self {
            Self::NotFound { hash } => msg!("error.store.not_found", hash = Text(hex(hash))),
            Self::Io(_) => msg!("error.store.io"),
        }
    }
}

fn hex(hash: &ContentHash) -> String {
    hash.0.iter().map(|b| format!("{b:02x}")).collect()
}

/// Content stored by hash. Adapters implement it (`gafira-assets`).
pub trait ContentStore: Send + Sync {
    /// Stores everything `src` yields and returns its hash.
    ///
    /// # Errors
    /// [`StoreError::Io`] when reading `src` or writing the store fails.
    fn put(&self, src: &mut dyn io::Read) -> Result<ContentHash, StoreError>;

    /// Opens the content with this hash.
    ///
    /// # Errors
    /// [`StoreError::NotFound`] when the store has no such content.
    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError>;

    /// True when the store has the content.
    ///
    /// # Errors
    /// [`StoreError::Io`] when the store cannot be queried.
    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError>;
}
