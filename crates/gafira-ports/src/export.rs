//! Exporters and the export job (02 §4.5, §6.4).
//!
//! An [`Exporter`] lives in an adapter and never sees `gafira-render`
//! (ARQ-06): it asks for pixels through the [`PageRenderer`] of its
//! [`ExportJob`].

use std::fmt;
use std::io;

use gafira_doc::contract::{CancelToken, Cancelled, ParamSchema, ParamValues, Progress};
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_doc::{Document, NodeId, PageId};
use gafira_geom::IRect;
use gafira_raster::ImageBuf;
use gafira_time::Flicks;

use crate::format::FormatDescriptor;
use crate::media::MediaError;
use crate::render::RenderError;
use crate::store::{ContentStore, StoreError};
use crate::system::WriteSeek;

/// What an exporter can write. `Default` is all false.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExportCaps {
    pub transparency: bool,
    pub multipage: bool,
    pub animation: bool,
    pub cmyk: bool,
    pub bleed: bool,
    pub notes: bool,
}

impl ExportCaps {
    /// True when every flag set in `required` is set in `self`.
    #[must_use]
    pub fn contains(self, required: Self) -> bool {
        (!required.transparency || self.transparency)
            && (!required.multipage || self.multipage)
            && (!required.animation || self.animation)
            && (!required.cmyk || self.cmyk)
            && (!required.bleed || self.bleed)
            && (!required.notes || self.notes)
    }
}

/// What to export. Persisted in the export queue (02 §6.4).
///
/// «Exportar proyecto editable con recursos» is a save, not an export (D-55):
/// it does not go through an [`Exporter`] and `LicenseStatus::can_export` does
/// not gate it.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportRequest {
    /// Id of the exporter's [`FormatDescriptor`].
    pub format_id: String,
    pub scope: ExportScope,
    pub options: ParamValues,
}

/// The part of the document to export; ranges are resolved by the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportScope {
    Pages(Vec<PageId>),
    Nodes { page: PageId, nodes: Vec<NodeId> },
}

/// How serious an [`ExportIssue`] is (FD-58).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IssueSeverity {
    /// Stops the export (a missing asset, an invalid size).
    Error,
    /// Shown, and the export may go on.
    Warning,
}

/// One finding of [`Exporter::validate`].
#[derive(Clone, Debug, PartialEq)]
pub struct ExportIssue {
    pub severity: IssueSeverity,
    /// What the user reads; names the resource it is about.
    pub message: Message,
}

impl ExportIssue {
    /// True when the export must not start (FD-58).
    #[must_use]
    pub fn blocks(&self) -> bool {
        self.severity == IssueSeverity::Error
    }
}

/// One file an export wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedFile {
    /// The name given to [`ExportSink::create`].
    pub name: String,
    pub bytes: u64,
}

/// What an export produced.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExportReport {
    pub files: Vec<ExportedFile>,
    /// Non-fatal findings met while writing.
    pub warnings: Vec<Message>,
}

/// One run of an [`ExportRequest`]. Borrows its inputs and is never persisted.
pub struct ExportJob<'a> {
    pub request: &'a ExportRequest,
    pub document: &'a Document,
    pub store: &'a dyn ContentStore,
    pub pages: &'a dyn PageRenderer,
}

/// Where an exporter writes. Temp file and rename live in its implementation
/// (02 §6.4).
pub trait ExportSink {
    /// Opens the output called `name`.
    ///
    /// # Errors
    /// The I/O error of the underlying file system.
    fn create(&mut self, name: &str) -> io::Result<Box<dyn WriteSeek + Send>>;
}

/// The pixels of one page. `gafira-render` implements it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageRenderRequest {
    pub page: PageId,
    pub time: Flicks,
    pub scale: f32,
    pub region: IRect,
    pub transparent: bool,
}

/// Renders a page for an exporter.
pub trait PageRenderer: Send + Sync {
    /// # Errors
    /// [`RenderError`]; [`RenderError::Cancelled`] when `cancel` fires.
    fn render(
        &self,
        req: &PageRenderRequest,
        cancel: &CancelToken,
    ) -> Result<ImageBuf, RenderError>;
}

/// Why an export failed.
#[derive(Debug)]
pub enum ExportError {
    Render(RenderError),
    Media(MediaError),
    Store(StoreError),
    Io(io::Error),
    Cancelled(Cancelled),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Render(e) => write!(f, "{e}"),
            Self::Media(e) => write!(f, "{e}"),
            Self::Store(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "export I/O error: {e}"),
            Self::Cancelled(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Render(e) => Some(e),
            Self::Media(e) => Some(e),
            Self::Store(e) => Some(e),
            Self::Io(e) => Some(e),
            Self::Cancelled(e) => Some(e),
        }
    }
}

impl From<RenderError> for ExportError {
    fn from(e: RenderError) -> Self {
        Self::Render(e)
    }
}

impl From<MediaError> for ExportError {
    fn from(e: MediaError) -> Self {
        Self::Media(e)
    }
}

impl From<StoreError> for ExportError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

impl From<io::Error> for ExportError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<Cancelled> for ExportError {
    fn from(e: Cancelled) -> Self {
        Self::Cancelled(e)
    }
}

impl ToMessage for ExportError {
    fn to_message(&self) -> Message {
        match self {
            Self::Render(e) => e.to_message(),
            Self::Media(e) => e.to_message(),
            Self::Store(e) => e.to_message(),
            Self::Io(_) => msg!("error.export.io"),
            Self::Cancelled(e) => e.to_message(),
        }
    }
}

/// Writes one file format from the document model.
pub trait Exporter: Send + Sync {
    fn descriptor(&self) -> &FormatDescriptor;

    /// What this exporter can write.
    fn capabilities(&self) -> ExportCaps;

    /// Builds the export options panel.
    fn options_schema(&self) -> &ParamSchema;

    /// Findings to show before the export is queued (FD-58).
    fn validate(&self, req: &ExportRequest) -> Vec<ExportIssue>;

    /// Writes the job through `sink`.
    ///
    /// # Errors
    /// [`ExportError`]; [`ExportError::Cancelled`] when `cancel` fires.
    fn export(
        &self,
        job: &ExportJob<'_>,
        sink: &mut dyn ExportSink,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<ExportReport, ExportError>;
}
