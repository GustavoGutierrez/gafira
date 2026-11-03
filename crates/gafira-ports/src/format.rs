//! Importers and the format registry (02 §4.5).

use std::fmt;
use std::io;

use gafira_doc::contract::{CancelToken, Cancelled, ParamValues, Progress};
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_doc::{ContentHash, Node, NodeId, Page};

use crate::common::{Confidence, LegalClass};
use crate::store::{ContentStore, StoreError};
use crate::system::ReadSeek;

/// What a format holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormatKind {
    Raster,
    Vector,
    Document,
    Video,
    Audio,
    Captions,
    Project,
}

/// Magic bytes at a fixed offset of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteSignature {
    pub offset: usize,
    pub bytes: &'static [u8],
}

impl ByteSignature {
    /// True when `head[offset..offset + bytes.len()] == bytes`.
    ///
    /// A `head` too short, an overflowing offset or empty `bytes` gives
    /// `false`.
    #[must_use]
    pub fn matches(&self, head: &[u8]) -> bool {
        let Some(end) = self.offset.checked_add(self.bytes.len()) else {
            return false;
        };
        !self.bytes.is_empty() && head.get(self.offset..end) == Some(self.bytes)
    }
}

/// Identity and recognition data of a file format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatDescriptor {
    pub id: &'static str,
    /// Lower-case extensions without the dot.
    pub extensions: &'static [&'static str],
    pub mime: &'static [&'static str],
    pub signatures: &'static [ByteSignature],
    pub kind: FormatKind,
    pub legal: LegalClass,
}

impl FormatDescriptor {
    /// [`Confidence::High`] when any signature matches `head`, otherwise
    /// [`Confidence::None`].
    #[must_use]
    pub fn sniff_signatures(&self, head: &[u8]) -> Confidence {
        if self.signatures.iter().any(|s| s.matches(head)) {
            Confidence::High
        } else {
            Confidence::None
        }
    }
}

/// Options of one import.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportOptions {
    pub params: ParamValues,
}

/// What an importer may use, and what it reports.
pub struct ImportCx<'a> {
    pub store: &'a dyn ContentStore,
    pub progress: &'a dyn Progress,
    pub cancel: &'a CancelToken,
    /// Non-fatal findings of the current attempt.
    pub warnings: Vec<Message>,
}

/// Content stored by an importer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedAsset {
    pub hash: ContentHash,
    pub mime: String,
}

/// The result of an import.
#[derive(Clone, Debug, PartialEq)]
pub enum Imported {
    Asset(ImportedAsset),
    Nodes {
        roots: Vec<NodeId>,
        nodes: Vec<Node>,
    },
    Pages(Vec<Page>),
}

/// Why an import failed.
#[derive(Debug)]
pub enum ImportError {
    /// No importer recognises the file.
    Unsupported,
    /// The file is damaged. `detail` is library text, never a message argument.
    Corrupt {
        detail: String,
    },
    Store(StoreError),
    Io(io::Error),
    Cancelled(Cancelled),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("unsupported format"),
            Self::Corrupt { detail } => write!(f, "corrupt file: {detail}"),
            Self::Store(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "import I/O error: {e}"),
            Self::Cancelled(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Unsupported | Self::Corrupt { .. } => None,
            Self::Store(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::Cancelled(e) => Some(e),
        }
    }
}

impl ToMessage for ImportError {
    fn to_message(&self) -> Message {
        match self {
            Self::Unsupported => msg!("error.import.unsupported"),
            Self::Corrupt { .. } => msg!("error.import.corrupt"),
            Self::Store(e) => e.to_message(),
            Self::Io(_) => msg!("error.import.io"),
            Self::Cancelled(e) => e.to_message(),
        }
    }
}

/// Reads one file format into the document model.
pub trait Importer: Send + Sync {
    fn descriptor(&self) -> &FormatDescriptor;

    /// How sure this importer is about `head`, the first bytes of the file.
    fn sniff(&self, head: &[u8]) -> Confidence;

    /// Reads `src` from its start.
    ///
    /// # Errors
    /// [`ImportError`]; [`ImportError::Cancelled`] stops the whole import.
    fn import(
        &self,
        src: &mut dyn ReadSeek,
        opt: &ImportOptions,
        cx: &mut ImportCx<'_>,
    ) -> Result<Imported, ImportError>;
}
