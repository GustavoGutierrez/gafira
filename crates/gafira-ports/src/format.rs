//! Importers, exporters and the format registry (02 §4.5).

use std::fmt;
use std::io::{self, SeekFrom};
use std::path::Path;
use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Cancelled, ParamValues, Progress};
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_doc::{ContentHash, Node, NodeId, Page};

use crate::common::{Confidence, LegalClass};
use crate::export::{ExportCaps, Exporter};
use crate::registry::RegistryError;
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

impl From<Cancelled> for ImportError {
    fn from(e: Cancelled) -> Self {
        Self::Cancelled(e)
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

/// The importers and exporters of the application, each list in registration
/// order.
///
/// Built by value at the composition root (ARQ-31). An importer and an
/// exporter may share an id (`.gafira` does both); an id is unique within its
/// own list.
#[derive(Default)]
pub struct FormatRegistry {
    importers: Vec<Arc<dyn Importer>>,
    exporters: Vec<Arc<dyn Exporter>>,
}

impl FormatRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an importer under its descriptor id.
    ///
    /// # Errors
    /// [`RegistryError::DuplicateId`] when another importer has the id; the
    /// registry is left unchanged.
    pub fn register_importer(&mut self, importer: Arc<dyn Importer>) -> Result<(), RegistryError> {
        let id = importer.descriptor().id;
        if self.importers.iter().any(|i| i.descriptor().id == id) {
            return Err(RegistryError::DuplicateId { id });
        }
        self.importers.push(importer);
        Ok(())
    }

    /// Adds an exporter under its descriptor id.
    ///
    /// # Errors
    /// [`RegistryError::DuplicateId`] when another exporter has the id; the
    /// registry is left unchanged. An importer with the same id does not
    /// count.
    pub fn register_exporter(&mut self, exporter: Arc<dyn Exporter>) -> Result<(), RegistryError> {
        let id = exporter.descriptor().id;
        if self.exporters.iter().any(|e| e.descriptor().id == id) {
            return Err(RegistryError::DuplicateId { id });
        }
        self.exporters.push(exporter);
        Ok(())
    }

    /// The exporters in registration order.
    pub fn exporters(&self) -> impl Iterator<Item = &Arc<dyn Exporter>> {
        self.exporters.iter()
    }

    /// The exporters whose capabilities contain `required`, in registration
    /// order.
    pub fn exporters_with(&self, required: ExportCaps) -> impl Iterator<Item = &Arc<dyn Exporter>> {
        self.exporters
            .iter()
            .filter(move |e| e.capabilities().contains(required))
    }

    /// The importers in registration order.
    pub fn importers(&self) -> impl Iterator<Item = &Arc<dyn Importer>> {
        self.importers.iter()
    }

    /// The importers to try for `path`, most likely first.
    ///
    /// Those listing the file's extension (last one, ASCII lower case) and
    /// recognising `head` come first. When none does, every importer that
    /// recognises `head` is a candidate, so a PNG named `.jpg` still opens.
    /// Highest confidence first; ties keep registration order.
    #[must_use]
    pub fn resolve_import(&self, path: &Path, head: &[u8]) -> Vec<Arc<dyn Importer>> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        // (confidence, listed for the extension, importer), sniffed once.
        let mut known: Vec<(Confidence, bool, &Arc<dyn Importer>)> = self
            .importers
            .iter()
            .map(|i| {
                let listed = ext
                    .as_deref()
                    .is_some_and(|e| i.descriptor().extensions.contains(&e));
                (i.sniff(head), listed, i)
            })
            .filter(|(confidence, _, _)| *confidence != Confidence::None)
            .collect();
        if known.iter().any(|(_, listed, _)| *listed) {
            known.retain(|(_, listed, _)| *listed);
        }
        known.sort_by_key(|(confidence, _, _)| std::cmp::Reverse(*confidence));
        known.into_iter().map(|(_, _, i)| Arc::clone(i)).collect()
    }

    /// Imports `src`, trying the candidates of [`Self::resolve_import`] in
    /// order.
    ///
    /// Each try starts at byte 0 of `src` and with the warnings `cx` held on
    /// entry. The first `Ok` is the result and `Cancelled` stops at once. When
    /// every importer fails the first error is returned, since it comes from
    /// the most likely format. On every `Err` the warnings of `cx` are those
    /// it held on entry: nothing from a failed try stays (02 §4.5 rule 5).
    ///
    /// # Errors
    /// [`ImportError::Unsupported`] without candidates, [`ImportError::Io`]
    /// when `src` cannot rewind, otherwise the first importer error.
    pub fn import(
        &self,
        path: &Path,
        head: &[u8],
        src: &mut dyn ReadSeek,
        opt: &ImportOptions,
        cx: &mut ImportCx<'_>,
    ) -> Result<Imported, ImportError> {
        let kept = cx.warnings.len();
        let result = self.try_each(path, head, src, opt, cx, kept);
        if result.is_err() {
            cx.warnings.truncate(kept);
        }
        result
    }

    fn try_each(
        &self,
        path: &Path,
        head: &[u8],
        src: &mut dyn ReadSeek,
        opt: &ImportOptions,
        cx: &mut ImportCx<'_>,
        kept: usize,
    ) -> Result<Imported, ImportError> {
        let mut first_error = None;
        for importer in self.resolve_import(path, head) {
            src.seek(SeekFrom::Start(0)).map_err(ImportError::Io)?;
            cx.warnings.truncate(kept);
            match importer.import(src, opt, cx) {
                Ok(imported) => return Ok(imported),
                Err(e @ ImportError::Cancelled(_)) => return Err(e),
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        Err(first_error.unwrap_or(ImportError::Unsupported))
    }
}
