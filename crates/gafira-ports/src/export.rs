//! Exporters and the export job (02 §4.5, §6.4).
//!
//! An [`Exporter`] lives in an adapter and never sees `gafira-render`
//! (ARQ-06): it asks for pixels through the [`PageRenderer`] of its
//! [`ExportJob`].

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::sync::Arc;

use gafira_doc::contract::{CancelToken, Cancelled, ParamSchema, ParamValues, Progress};
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_doc::{AssetRef, Document, ImageNode, NodeId, PageId};
use gafira_geom::IRect;
use gafira_raster::ImageBuf;
use gafira_raster::look::{apply_look, mask_of, reduced};
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

    /// The drawing of `page` as its operations, in order, in page px (spec
    /// 073): what a vector writer (PDF) keeps as vectors and text. `None`
    /// when this renderer has no such drawing; the writer then draws pixels.
    ///
    /// # Errors
    /// [`RenderError`]; [`RenderError::Cancelled`] when `cancel` fires.
    fn vectors(
        &self,
        _page: gafira_doc::PageId,
        _cancel: &CancelToken,
    ) -> Result<Option<Vec<crate::render::RenderOp>>, RenderError> {
        Ok(None)
    }
}

/// The decoded pixels of the image assets an export draws, by asset
/// (spec 032). Each buffer is 8-bit sRGB premultiplied (ARQ-117).
pub type ImagePixels = BTreeMap<AssetRef, Arc<ImageBuf>>;

/// Builds the [`PageRenderer`] of one snapshot of the document for an export
/// (spec 030).
///
/// The composition root implements it over `gafira-render` and a backend, so
/// the engine hands an [`ExportJob`] its pages without seeing either
/// (ARQ-06).
pub trait PageRenderers: Send + Sync {
    /// A renderer of the pages of `document`, at export quality (02 §6.2),
    /// that draws each image element with the pixels of its asset in
    /// `images` (spec 032). The render reads no file: the engine resolves
    /// and decodes the assets first.
    fn for_document(&self, document: &Document, images: ImagePixels) -> Box<dyn PageRenderer>;
}

/// Decodes the bytes of an image asset into pixels (spec 032).
///
/// Decoding is file-format parsing, so it lives in an adapter
/// (`gafira-io-image`, 02 §2.1); the engine reads the bytes from the
/// [`ContentStore`] and hands them over.
pub trait ImageDecoder: Send + Sync {
    /// The pixels of the encoded image `bytes`, as 8-bit sRGB premultiplied
    /// (ARQ-117).
    ///
    /// # Errors
    /// [`MediaError::Corrupt`] when the bytes are not an image this decoder
    /// reads.
    fn decode(&self, bytes: &[u8]) -> Result<ImageBuf, MediaError>;
}

/// The pixels `image` shows: those of its asset, with its look when it has
/// one (spec 085). A look drawn to be seen small is worked out on an image
/// no longer than `most`, which takes a share of the time.
///
/// # Errors
/// [`ExportError::Store`] when the asset, or the mask of a zone, is not in
/// the store, [`ExportError::Media`] when one does not decode and
/// [`ExportError::Render`] when the look cannot be drawn.
pub fn decode_image(
    image: &ImageNode,
    store: &dyn ContentStore,
    images: &dyn ImageDecoder,
    most: Option<u32>,
) -> Result<ImageBuf, ExportError> {
    let read = |asset: AssetRef| -> Result<ImageBuf, ExportError> {
        let mut bytes = Vec::new();
        io::Read::read_to_end(&mut store.open(asset.as_hash())?, &mut bytes)?;
        Ok(images.decode(&bytes)?)
    };
    let pixels = read(image.asset)?;
    let Some(look) = &image.look else {
        return Ok(pixels);
    };
    let mut masks = BTreeMap::new();
    for zone in &look.zones {
        if let Some(mask) = mask_of(&read(zone.mask)?) {
            masks.insert(zone.mask, mask);
        }
    }
    let (pixels, scale) = most
        .and_then(|most| reduced(&pixels, most))
        .unwrap_or((pixels, 1.0));
    apply_look(look, &pixels, &|asset| masks.get(&asset).cloned(), scale)
        .map_err(|error| ExportError::Render(error.into()))
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
