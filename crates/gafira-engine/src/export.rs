//! `ExportService` of 02 §5.4 line 1361 and spec 030: exports the open
//! document through the format registry.
//!
//! The order is licence, document, exporter, validation, then files: an
//! export the licence forbids (02 §4.14), with no document, with an unknown
//! format or with a blocking finding of `Exporter::validate` (FD-58) opens no
//! file. The exporter writes each output to `<file>.tmp` through
//! `FileSystem`; only when it returns `Ok` is every temporary synced and
//! moved over its final name (02 §6.4, ARQ-50). A failed or cancelled export
//! removes its temporaries and leaves no final file.
//!
//! Before any file, the image assets of the pages of the scope are read from
//! the content store and decoded by the `ImageDecoder` port, once per asset,
//! and handed to the renderer (spec 032): the render reads no file. An asset
//! the store does not hold, or one that does not decode, fails the export
//! there.
//!
//! `prepare` does every check on the caller's thread and returns a
//! `PreparedExport` that owns a snapshot of the document and the ports; the
//! host runs it on another thread, with progress and cancellation (spec
//! 035). The persisted export queue, its states and its retries (02 §6.4)
//! are later.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use gafira_doc::contract::{CancelToken, Progress};
use gafira_doc::{AssetRef, Document, NodeKind, PageId};
use gafira_geom::IRect;
use gafira_ports::export::{
    ExportError, ExportJob, ExportReport, ExportRequest, ExportScope, ExportSink, Exporter,
    ImageDecoder, ImagePixels, PageRenderRequest, PageRenderers, decode_image,
};
use gafira_ports::format::FormatRegistry;
use gafira_ports::store::ContentStore;
use gafira_ports::system::{FileSystem, FileWrite, WriteSeek};
use gafira_raster::straight_srgb8;
use gafira_time::Flicks;

use crate::engine::{EngineError, Ports};
use crate::license::LicenseService;
use crate::session::Shared;

/// Exports the open document (02 §5.4 line 1361).
pub struct ExportService {
    shared: Shared,
    license: Arc<LicenseService>,
    exporters: Vec<Arc<dyn Exporter>>,
    pages: Arc<dyn PageRenderers>,
    images: Arc<dyn ImageDecoder>,
    store: Arc<dyn ContentStore>,
    filesystem: Arc<dyn FileSystem>,
}

impl ExportService {
    pub(crate) fn over(
        shared: Shared,
        license: Arc<LicenseService>,
        formats: &FormatRegistry,
        ports: &Ports,
    ) -> Self {
        Self {
            shared,
            license,
            exporters: formats.exporters().cloned().collect(),
            pages: Arc::clone(&ports.pages),
            images: Arc::clone(&ports.images),
            store: Arc::clone(&ports.store),
            filesystem: Arc::clone(&ports.filesystem),
        }
    }

    /// The exporters of the registry, in registration order.
    pub fn exporters(&self) -> impl Iterator<Item = &Arc<dyn Exporter>> {
        self.exporters.iter()
    }

    /// Export the open document as `request` says, to `to`, on the
    /// caller's thread: [`prepare`](Self::prepare), then
    /// [`PreparedExport::run`].
    ///
    /// The first file the exporter writes goes to `to`; a second one to `to`
    /// with `-2` before its extension, and so on (FD-57 (c)). The files
    /// appear only when the whole export succeeded.
    ///
    /// # Errors
    /// Those of [`prepare`](Self::prepare) and of [`PreparedExport::run`].
    pub fn export(
        &self,
        request: &ExportRequest,
        to: &Path,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<ExportReport, EngineError> {
        self.prepare(request.clone(), to.to_path_buf())?
            .run(progress, cancel)
    }

    /// Check an export of the open document and take everything it needs,
    /// without rendering or opening any file (spec 035). The result owns a
    /// snapshot of the document and the ports, so it runs on any thread
    /// while the person keeps editing; their edits do not reach it.
    ///
    /// # Errors
    /// [`EngineError::ExportNotAllowed`] when the licence forbids it,
    /// [`EngineError::NoSession`] with no open document,
    /// [`EngineError::UnknownFormat`] when no exporter has the format id and
    /// [`EngineError::ExportBlocked`] with the blocking findings of
    /// `validate`. Nothing is rendered and no file is opened.
    pub fn prepare(
        &self,
        request: ExportRequest,
        to: PathBuf,
    ) -> Result<PreparedExport, EngineError> {
        let document = self
            .shared
            .lock()
            .as_ref()
            .map(|session| Arc::clone(session.history.document()))
            .ok_or(EngineError::NoSession)?;
        self.prepare_document(document, request, to)
    }

    /// [`prepare`](Self::prepare) over `document` instead of the open one
    /// (spec 104: the pages a bulk run resolves, which never enter the
    /// session). Its images come from the same store.
    ///
    /// # Errors
    /// Those of [`prepare`](Self::prepare) but [`EngineError::NoSession`].
    pub fn prepare_document(
        &self,
        document: Arc<Document>,
        request: ExportRequest,
        to: PathBuf,
    ) -> Result<PreparedExport, EngineError> {
        if !self.license.can_export() {
            return Err(EngineError::ExportNotAllowed);
        }
        let exporter = self
            .exporters
            .iter()
            .find(|exporter| exporter.descriptor().id == request.format_id)
            .ok_or(EngineError::UnknownFormat)?;
        let blocking: Vec<_> = exporter
            .validate(&request)
            .into_iter()
            .filter(|issue| issue.blocks())
            .collect();
        if !blocking.is_empty() {
            return Err(EngineError::ExportBlocked(blocking));
        }
        Ok(PreparedExport {
            exporter: Arc::clone(exporter),
            request,
            to,
            document,
            pages: Arc::clone(&self.pages),
            images: Arc::clone(&self.images),
            store: Arc::clone(&self.store),
            filesystem: Arc::clone(&self.filesystem),
        })
    }
}

impl ExportService {
    /// A small picture of `page` of the open document for the export
    /// dialog (spec 039, 03 §7.5), drawn by the `PageRenderers` of the
    /// exports on the page background or transparent. Its long side is at
    /// most [`THUMBNAIL_SIDE`](gafira_ports::repository::THUMBNAIL_SIDE)
    /// pixels and a small page is not enlarged. Its images come from the
    /// store like an export's; one that cannot be read is left out of the
    /// picture, and the export itself reports it.
    ///
    /// # Errors
    /// [`EngineError::NoSession`] with no open document;
    /// [`EngineError::Export`] when the document has no such page, its size
    /// has no pixels or the render fails.
    pub fn preview(&self, page: PageId, transparent: bool) -> Result<PagePreview, EngineError> {
        self.picture(page, transparent, None)
    }

    /// `page` of `document`, not the open one, drawn as large as fits in
    /// `fit` pixels (spec 104: a row of a bulk run before it runs).
    ///
    /// # Errors
    /// As [`Self::preview`], without a session.
    pub fn preview_document(
        &self,
        document: &Document,
        page: PageId,
        fit: (u32, u32),
    ) -> Result<PagePreview, EngineError> {
        self.picture_of(document, page, false, Some(fit))
    }

    /// `page` drawn as large as fits in `fit` pixels, keeping its proportions and
    /// enlarging a small page (spec 101, presenting on a full screen); its long side is at
    /// most [`PRESENT_MAX_SIDE`].
    ///
    /// # Errors
    /// As [`Self::preview`].
    pub fn present(&self, page: PageId, fit: (u32, u32)) -> Result<PagePreview, EngineError> {
        self.picture(page, false, Some(fit))
    }

    fn picture(
        &self,
        page: PageId,
        transparent: bool,
        fit: Option<(u32, u32)>,
    ) -> Result<PagePreview, EngineError> {
        let document = self
            .shared
            .lock()
            .as_ref()
            .map(|session| Arc::clone(session.history.document()))
            .ok_or(EngineError::NoSession)?;
        self.picture_of(&document, page, transparent, fit)
    }

    fn picture_of(
        &self,
        document: &Document,
        page: PageId,
        transparent: bool,
        fit: Option<(u32, u32)>,
    ) -> Result<PagePreview, EngineError> {
        let invalid = |detail: &str| {
            EngineError::Export(ExportError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                detail,
            )))
        };
        let target = document
            .page(page)
            .ok_or_else(|| invalid("the page is not in the document"))?;
        let px = target
            .size
            .to_px(document.dpi())
            .map_err(|_| invalid("the page size is not finite"))?;
        let sized = match fit {
            None => crate::thumbnail::size(px.width(), px.height()),
            Some(fit) => present_size(px.width(), px.height(), fit),
        };
        let (width, height, scale) = sized.ok_or_else(|| invalid("the page has no pixels"))?;
        let images =
            decode_images(document, &[page], &*self.store, &*self.images).unwrap_or_default();
        let image = self
            .pages
            .for_document(document, images)
            .render(
                &PageRenderRequest {
                    page,
                    time: Flicks::default(),
                    scale: scale as f32,
                    region: IRect::new(0, 0, width as i32, height as i32),
                    transparent,
                },
                &CancelToken::new(),
            )
            .map_err(|error| EngineError::Export(error.into()))?;
        let rgba = straight_srgb8(&image).ok_or_else(|| invalid("the render is a mask"))?;
        Ok(PagePreview {
            width: image.width(),
            height: image.height(),
            rgba,
        })
    }
}

/// The longest side of a page drawn to present it (spec 101).
pub const PRESENT_MAX_SIDE: u32 = 4096;

/// The pixel size and scale of a `width` × `height` px page drawn as large as fits in
/// `fit`, its long side at most [`PRESENT_MAX_SIDE`]; `None` with no size.
fn present_size(width: f64, height: f64, fit: (u32, u32)) -> Option<(u32, u32, f64)> {
    let valid = |side: f64| side.is_finite() && side > 0.0;
    let (fw, fh) = (f64::from(fit.0.max(1)), f64::from(fit.1.max(1)));
    if !valid(width) || !valid(height) {
        return None;
    }
    let most = f64::from(PRESENT_MAX_SIDE);
    let scale = (fw / width).min(fh / height).min(most / width.max(height));
    // In range: each side is at most `most`, which fits a `u32`.
    let px = |length: f64| (length * scale).round().clamp(1.0, most) as u32;
    Some((px(width), px(height), scale))
}

#[cfg(test)]
mod present_tests {
    use super::present_size;

    // S101-02: a page fills the screen keeping its proportions, small ones enlarged,
    // never past the limit.
    #[test]
    fn s101_02_a_page_fits_the_screen() {
        assert_eq!(
            present_size(1920.0, 1080.0, (1920, 1080)),
            Some((1920, 1080, 1.0))
        );
        assert_eq!(
            present_size(1080.0, 1080.0, (1920, 1080)),
            Some((1080, 1080, 1.0))
        );
        assert_eq!(
            present_size(960.0, 540.0, (1920, 1200)),
            Some((1920, 1080, 2.0))
        );
        let (w, h, _) = present_size(100.0, 50.0, (100_000, 100_000)).expect("size");
        assert_eq!((w, h), (4096, 2048));
        assert_eq!(present_size(0.0, 10.0, (100, 100)), None);
    }
}

/// The picture of [`ExportService::preview`]: straight 8-bit sRGB RGBA,
/// row by row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PagePreview {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// An export that passed every check of [`ExportService::prepare`] and owns
/// what it needs: it is `Send + 'static`, so the host runs it off the window
/// thread (spec 035, 02 §6.4).
pub struct PreparedExport {
    request: ExportRequest,
    to: PathBuf,
    document: Arc<Document>,
    exporter: Arc<dyn Exporter>,
    pages: Arc<dyn PageRenderers>,
    images: Arc<dyn ImageDecoder>,
    store: Arc<dyn ContentStore>,
    filesystem: Arc<dyn FileSystem>,
}

impl std::fmt::Debug for PreparedExport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedExport")
            .field("format", &self.request.format_id)
            .field("to", &self.to)
            .finish_non_exhaustive()
    }
}

impl PreparedExport {
    /// The file the export writes first.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.to
    }

    /// The same checked export of the same format and options over another
    /// document, scope and file (spec 104: a bulk run writes many files
    /// with one exporter, off the window thread).
    #[must_use]
    pub fn again(&self, document: Arc<Document>, scope: ExportScope, to: PathBuf) -> Self {
        let mut request = self.request.clone();
        request.scope = scope;
        Self {
            request,
            to,
            document,
            exporter: Arc::clone(&self.exporter),
            pages: Arc::clone(&self.pages),
            images: Arc::clone(&self.images),
            store: Arc::clone(&self.store),
            filesystem: Arc::clone(&self.filesystem),
        }
    }

    /// Decode the images of the scope, render and write the files through
    /// their temporaries. `cancel` is looked at before the images, between
    /// them and by the exporter; a cancelled export leaves no file.
    ///
    /// # Errors
    /// [`EngineError::Export`] with [`ExportError::Store`] when an image
    /// asset of the scope is not in the store (`NotFound` names its hash)
    /// and with [`ExportError::Media`] when it does not decode, before any
    /// file (spec 032); [`EngineError::Export`] with
    /// [`ExportError::Cancelled`] when `cancel` fires; [`EngineError::Export`]
    /// when the exporter fails and [`EngineError::Io`] when a file cannot be
    /// finished. No final file is left then.
    pub fn run(
        self,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<ExportReport, EngineError> {
        let pages: Vec<PageId> = match &self.request.scope {
            ExportScope::Pages(pages) => pages.clone(),
            ExportScope::Nodes { page, .. } => vec![*page],
        };
        let images = decode_images_until(
            &self.document,
            &pages,
            (&*self.store, &*self.images),
            None,
            cancel,
        )
        .map_err(EngineError::Export)?;
        let renderer = self.pages.for_document(&self.document, images);
        let job = ExportJob {
            request: &self.request,
            document: &self.document,
            store: &*self.store,
            pages: renderer.as_ref(),
        };
        let mut sink = FileSink::new(&*self.filesystem, &self.to);
        let outcome = self
            .exporter
            .export(&job, &mut sink, progress, cancel)
            .and_then(|report| cancel.check().map(|()| report).map_err(Into::into));
        match outcome {
            Ok(report) => {
                sink.commit()?;
                Ok(report)
            }
            Err(error) => {
                sink.discard();
                Err(EngineError::Export(error))
            }
        }
    }
}

/// The decoded pixels of every image asset of `pages`, each asset read from
/// `store` and decoded once (spec 032); the thumbnail of a save uses it too
/// (spec 033). Every image of those pages counts, visible or not.
pub(crate) fn decode_images(
    document: &Document,
    pages: &[PageId],
    store: &dyn ContentStore,
    images: &dyn ImageDecoder,
) -> Result<ImagePixels, ExportError> {
    let cancel = CancelToken::new();
    decode_images_until(document, pages, (store, images), Some(SEEN_SMALL), &cancel)
}

/// The longest side of an image with a look in a thumbnail or a preview: a
/// save does not wait for all its pixels (spec 085).
const SEEN_SMALL: u32 = 1024;

/// [`decode_images`], looking at `cancel` before each asset (spec 035); an
/// image with a look no longer than `most`.
fn decode_images_until(
    document: &Document,
    pages: &[PageId],
    (store, images): (&dyn ContentStore, &dyn ImageDecoder),
    most: Option<u32>,
    cancel: &CancelToken,
) -> Result<ImagePixels, ExportError> {
    cancel.check()?;
    let assets: BTreeSet<AssetRef> = pages
        .iter()
        .filter_map(|page| document.page(*page))
        // The image of a page background is decoded too (spec 069).
        .flat_map(|page| {
            page.background
                .as_ref()
                .and_then(gafira_doc::PageBackground::asset)
                .into_iter()
                .chain(page.nodes.values().filter_map(|node| match &node.kind {
                    NodeKind::Image(image) if image.is_plain() => Some(image.asset),
                    NodeKind::Frame(frame) => frame.content.map(|content| content.asset),
                    NodeKind::Qr(qr) => qr.logo.map(|logo| logo.asset),
                    _ => None,
                }))
        })
        .collect();
    let mut decoded = ImagePixels::new();
    for asset in assets {
        cancel.check()?;
        let mut bytes = Vec::new();
        store.open(asset.as_hash())?.read_to_end(&mut bytes)?;
        let pixels = images.decode(&bytes)?;
        decoded.insert(asset, Arc::new(pixels));
    }
    // An image with a look, or cropped, is kept as it looks (specs 085, 088).
    for node in pages
        .iter()
        .filter_map(|page| document.page(*page))
        .flat_map(|page| page.nodes.values())
    {
        let NodeKind::Image(image) = &node.kind else {
            continue;
        };
        let shown = image.shown();
        if image.is_plain() || decoded.contains_key(&shown) {
            continue;
        }
        cancel.check()?;
        decoded.insert(shown, Arc::new(decode_image(image, store, images, most)?));
    }
    // A 3D model is drawn as its camera sees it (spec 100 §8): for an export at four times
    // its box so any export scale keeps its detail (AC-14), for a preview at its box.
    let mut models: BTreeMap<AssetRef, (&gafira_doc::Model3dNode, (u32, u32))> = BTreeMap::new();
    for node in pages
        .iter()
        .filter_map(|page| document.page(*page))
        .flat_map(|page| page.nodes.values())
    {
        let NodeKind::Model3d(model) = &node.kind else {
            continue;
        };
        let p = node.base.placement;
        let (scale, limit) =
            most.map_or((MODEL_EXPORT_SCALE, MODEL_EXPORT_MOST), |most| (1.0, most));
        let size = gafira_ports::export::model_pixels((p.w, p.h), scale, limit);
        // Two boxes showing the same picture share it, drawn for the larger one.
        let entry = models.entry(model.shown()).or_insert((model, size));
        if size.0 * size.1 > entry.1.0 * entry.1.1 {
            entry.1 = size;
        }
    }
    for (shown, (model, size)) in models {
        cancel.check()?;
        let pixels = gafira_ports::export::render_model(model, store, images, size)?;
        decoded.insert(shown, Arc::new(pixels));
    }
    Ok(decoded)
}

/// How much larger than its box a 3D model is drawn for an export (spec 100 AC-14): the
/// largest export scale.
const MODEL_EXPORT_SCALE: f64 = 4.0;
/// The longest side of a 3D model's picture in an export (spec 100 §8.2).
const MODEL_EXPORT_MOST: u32 = 4096;

/// One output in progress: its temporary, its final name and its file.
struct Pending {
    temp: PathBuf,
    path: PathBuf,
    file: Arc<Mutex<Box<dyn FileWrite>>>,
}

/// The sink of one export: every output goes to a temporary first (02 §6.4).
struct FileSink<'a> {
    filesystem: &'a dyn FileSystem,
    target: &'a Path,
    files: Vec<Pending>,
}

impl<'a> FileSink<'a> {
    fn new(filesystem: &'a dyn FileSystem, target: &'a Path) -> Self {
        Self {
            filesystem,
            target,
            files: Vec::new(),
        }
    }

    /// Sync every temporary and move it over its final name. When one step
    /// fails, the temporaries not moved yet are removed.
    fn commit(self) -> Result<(), EngineError> {
        let mut result = Ok(());
        for pending in &self.files {
            let mut file = pending.file.lock().unwrap_or_else(PoisonError::into_inner);
            if let Err(error) = file.flush().and_then(|()| file.sync_all()) {
                result = Err(error);
                break;
            }
        }
        let mut moved = 0;
        if result.is_ok() {
            for pending in &self.files {
                if let Err(error) = self.filesystem.replace(&pending.temp, &pending.path) {
                    result = Err(error);
                    break;
                }
                moved += 1;
            }
        }
        for pending in &self.files[moved..] {
            if result.is_err() {
                let _ = self.filesystem.remove(&pending.temp);
            }
        }
        result.map_err(EngineError::Io)
    }

    /// Remove every temporary: the export failed.
    fn discard(self) {
        for pending in &self.files {
            let _ = self.filesystem.remove(&pending.temp);
        }
    }
}

impl ExportSink for FileSink<'_> {
    fn create(&mut self, _name: &str) -> io::Result<Box<dyn WriteSeek + Send>> {
        let path = numbered(self.target, self.files.len());
        let temp = temporary(&path);
        let file = Arc::new(Mutex::new(self.filesystem.create(&temp)?));
        self.files.push(Pending {
            temp,
            path,
            file: Arc::clone(&file),
        });
        Ok(Box::new(SharedFile(file)))
    }
}

/// The writer an exporter holds; the sink keeps the same file to sync it.
struct SharedFile(Arc<Mutex<Box<dyn FileWrite>>>);

impl SharedFile {
    fn file(&self) -> std::sync::MutexGuard<'_, Box<dyn FileWrite>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Write for SharedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file().flush()
    }
}

impl Seek for SharedFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file().seek(pos)
    }
}

/// The final name of output number `index`: `target` for the first, then
/// `target` with `-2`, `-3`... before its extension (FD-57 (c)).
fn numbered(target: &Path, index: usize) -> PathBuf {
    if index == 0 {
        return target.to_path_buf();
    }
    let stem = target
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let name = match target.extension() {
        Some(ext) => format!("{stem}-{}.{}", index + 1, ext.to_string_lossy()),
        None => format!("{stem}-{}", index + 1),
    };
    target.with_file_name(name)
}

/// The temporary of `path`: `path` with `.tmp` after its whole name (FD-58).
fn temporary(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use gafira_doc::contract::{CancelToken, NoProgress, ParamSchema};
    use gafira_doc::{
        AssetEntry, AssetRef, AssetStorage, Document, ImageNode, NodeKind, PageId, Placement,
    };
    use gafira_doc::{ImageAdjust, ImageLook, ImageShadow, ImageZone, ShadowKind, ZoneKind};
    use gafira_edit::{InsertAsset, InsertNodes, NodeSpec};
    use gafira_geom::IRect;
    use gafira_ports::common::LegalClass;
    use gafira_ports::export::{
        ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
        ExportSink, ExportedFile, Exporter, IssueSeverity, PageRenderRequest, PageRenderer,
        PageRenderers,
    };
    use gafira_ports::format::{FormatDescriptor, FormatKind};
    use gafira_ports::license::{LicenseVerifier, NoLicensing};
    use gafira_ports::render::RenderError;
    use gafira_ports::store::{ContentStore, StoreError};
    use gafira_raster::{ImageBuf, PixelBuf};

    use crate::engine::{EngineError, Gafira, Registries};
    use crate::fakes::*;

    const FAKE: FormatDescriptor = FormatDescriptor {
        id: "fake",
        extensions: &["fake"],
        mime: &[],
        signatures: &[],
        kind: FormatKind::Raster,
        legal: LegalClass::RoyaltyFree,
    };

    /// What the fake exporter does.
    #[derive(Clone, Copy)]
    enum Plan {
        /// Writes `files` outputs and succeeds.
        Write { files: usize },
        /// Writes one output, then fails.
        Fail,
        /// `validate` reports a blocking finding.
        Block,
    }

    /// An exporter that renders the first page of the scope through the job
    /// and writes the bytes `PNG!` per output.
    struct FakeExporter {
        plan: Plan,
        schema: ParamSchema,
        rendered: Mutex<Vec<PageId>>,
    }

    impl FakeExporter {
        fn new(plan: Plan) -> Arc<Self> {
            Arc::new(Self {
                plan,
                schema: ParamSchema::default(),
                rendered: Mutex::new(Vec::new()),
            })
        }
    }

    impl Exporter for FakeExporter {
        fn descriptor(&self) -> &FormatDescriptor {
            &FAKE
        }

        fn capabilities(&self) -> ExportCaps {
            ExportCaps::default()
        }

        fn options_schema(&self) -> &ParamSchema {
            &self.schema
        }

        fn validate(&self, _req: &ExportRequest) -> Vec<ExportIssue> {
            match self.plan {
                Plan::Block => vec![
                    ExportIssue {
                        severity: IssueSeverity::Warning,
                        message: gafira_doc::msg!("export.warning"),
                    },
                    ExportIssue {
                        severity: IssueSeverity::Error,
                        message: gafira_doc::msg!("export.no_pages"),
                    },
                ],
                _ => Vec::new(),
            }
        }

        fn export(
            &self,
            job: &ExportJob<'_>,
            sink: &mut dyn ExportSink,
            _progress: &dyn gafira_doc::contract::Progress,
            cancel: &CancelToken,
        ) -> Result<ExportReport, ExportError> {
            let ExportScope::Pages(pages) = &job.request.scope else {
                return Err(ExportError::Io(std::io::Error::other("no pages")));
            };
            let image = job.pages.render(
                &PageRenderRequest {
                    page: pages[0],
                    time: Default::default(),
                    scale: 1.0,
                    region: IRect::new(0, 0, 1, 1),
                    transparent: false,
                },
                cancel,
            )?;
            assert_eq!(image.width(), 1);
            self.rendered.lock().unwrap().push(pages[0]);
            let files = match self.plan {
                Plan::Write { files } => files,
                Plan::Fail | Plan::Block => 1,
            };
            let mut report = ExportReport::default();
            for n in 0..files {
                let name = format!("out-{n}.fake");
                let mut out = sink.create(&name)?;
                out.write_all(b"PNG!")?;
                report.files.push(ExportedFile { name, bytes: 4 });
            }
            if matches!(self.plan, Plan::Fail) {
                return Err(ExportError::Io(std::io::Error::other("disk full")));
            }
            Ok(report)
        }
    }

    /// Renderers that count the snapshots they were given and answer one
    /// white pixel per page.
    #[derive(Default)]
    struct CountingPages {
        snapshots: Mutex<Vec<usize>>,
        images: Mutex<Vec<gafira_ports::export::ImagePixels>>,
    }

    struct WhitePixel;

    impl PageRenderer for WhitePixel {
        fn render(
            &self,
            _req: &PageRenderRequest,
            _cancel: &CancelToken,
        ) -> Result<ImageBuf, RenderError> {
            Ok(ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![255; 4]))?)
        }
    }

    impl PageRenderers for CountingPages {
        fn for_document(
            &self,
            document: &Document,
            images: gafira_ports::export::ImagePixels,
        ) -> Box<dyn PageRenderer> {
            self.snapshots.lock().unwrap().push(document.pages().len());
            self.images.lock().unwrap().push(images);
            Box::new(WhitePixel)
        }
    }

    struct Setup {
        engine: Gafira,
        fs: MemoryFs,
        pages: Arc<CountingPages>,
        exporter: Arc<FakeExporter>,
    }

    /// An engine with a new document over a memory file system, `license`
    /// and the fake exporter of `plan` in its registry.
    fn setup(plan: Plan, license: Arc<dyn LicenseVerifier>, open: bool) -> Setup {
        let fs = MemoryFs::default();
        let pages = Arc::new(CountingPages::default());
        let exporter = FakeExporter::new(plan);
        let mut ports = six_ports();
        ports.filesystem = Arc::new(fs.clone());
        ports.license = license;
        ports.pages = Arc::clone(&pages) as Arc<dyn PageRenderers>;
        let mut registries = Registries::new();
        registries
            .formats
            .register_exporter(Arc::clone(&exporter) as Arc<dyn Exporter>)
            .expect("the fake registers");
        let engine = Gafira::new(ports, registries, config(), quiet_channel()).expect("an engine");
        if open {
            engine.projects().new_document().expect("a new document");
        }
        fs.take_trace();
        Setup {
            engine,
            fs,
            pages,
            exporter,
        }
    }

    fn first_page(engine: &mut Gafira) -> PageId {
        append_page(engine)
    }

    fn request(format: &str, page: PageId) -> ExportRequest {
        ExportRequest {
            format_id: format.to_owned(),
            scope: ExportScope::Pages(vec![page]),
            options: Default::default(),
        }
    }

    fn export(setup: &Setup, request: &ExportRequest) -> Result<ExportReport, EngineError> {
        setup.engine.export().export(
            request,
            Path::new("/out/poster.fake"),
            &NoProgress,
            &CancelToken::new(),
        )
    }

    fn path(text: &str) -> PathBuf {
        PathBuf::from(text)
    }

    // S30-05: the export runs the exporter of the registry over a snapshot of
    // the open document, with the renderer of the port, and the file appears
    // through its temporary, synced and moved.
    #[test]
    fn s30_05_the_open_document_exports_through_the_registry_and_a_temporary() {
        let mut setup = setup(Plan::Write { files: 1 }, Arc::new(NoLicensing), true);
        let page = first_page(&mut setup.engine);
        setup.fs.take_trace();
        let report = export(&setup, &request("fake", page)).expect("the export runs");
        assert_eq!(report.files.len(), 1);
        assert_eq!(
            setup.fs.file("/out/poster.fake").as_deref(),
            Some(&b"PNG!"[..])
        );
        assert_eq!(setup.fs.file("/out/poster.fake.tmp"), None);
        assert_eq!(*setup.exporter.rendered.lock().unwrap(), [page]);
        assert_eq!(*setup.pages.snapshots.lock().unwrap(), [1]);
        assert_eq!(
            setup.fs.take_trace(),
            [
                FsOp::Create(path("/out/poster.fake.tmp")),
                FsOp::Write(path("/out/poster.fake.tmp"), 4),
                FsOp::SyncAll(path("/out/poster.fake.tmp")),
                FsOp::Replace(path("/out/poster.fake.tmp"), path("/out/poster.fake")),
            ]
        );
    }

    // S30-05, FD-57 (c): a second output takes the name with `-2`.
    #[test]
    fn s30_05_more_outputs_get_numbered_names() {
        let mut setup = setup(Plan::Write { files: 3 }, Arc::new(NoLicensing), true);
        let page = first_page(&mut setup.engine);
        export(&setup, &request("fake", page)).expect("the export runs");
        for name in [
            "/out/poster.fake",
            "/out/poster-2.fake",
            "/out/poster-3.fake",
        ] {
            assert_eq!(setup.fs.file(name).as_deref(), Some(&b"PNG!"[..]), "{name}");
        }
        assert_eq!(
            super::numbered(Path::new("/w/plain"), 1),
            path("/w/plain-2")
        );
    }

    // S30-05: an unknown format, no document or a blocking finding open no
    // file and build no renderer.
    #[test]
    fn s30_05_what_is_refused_before_the_export_opens_no_file() {
        let mut setup = setup(Plan::Write { files: 1 }, Arc::new(NoLicensing), true);
        let page = first_page(&mut setup.engine);
        setup.fs.take_trace();
        assert!(matches!(
            export(&setup, &request("tiff", page)),
            Err(EngineError::UnknownFormat)
        ));

        let blocked = self::setup(Plan::Block, Arc::new(NoLicensing), true);
        match export(&blocked, &request("fake", PageId::new(1))) {
            Err(EngineError::ExportBlocked(issues)) => {
                assert_eq!(issues.len(), 1, "only the error blocks");
                assert_eq!(issues[0].message, gafira_doc::msg!("export.no_pages"));
            }
            other => panic!("blocked, got {other:?}"),
        }

        let closed = self::setup(Plan::Write { files: 1 }, Arc::new(NoLicensing), false);
        assert!(matches!(
            export(&closed, &request("fake", PageId::new(1))),
            Err(EngineError::NoSession)
        ));

        for setup in [&setup, &blocked, &closed] {
            assert!(setup.fs.take_trace().is_empty(), "no file was opened");
            assert!(setup.pages.snapshots.lock().unwrap().is_empty());
        }
    }

    // S30-05, ARQ-50: an exporter that fails after writing leaves no final
    // file and no temporary.
    #[test]
    fn s30_05_a_failed_export_leaves_no_file() {
        let mut setup = setup(Plan::Fail, Arc::new(NoLicensing), true);
        let page = first_page(&mut setup.engine);
        setup.fs.take_trace();
        assert!(matches!(
            export(&setup, &request("fake", page)),
            Err(EngineError::Export(_))
        ));
        assert_eq!(setup.fs.file("/out/poster.fake"), None);
        assert_eq!(setup.fs.file("/out/poster.fake.tmp"), None);
        let trace = setup.fs.take_trace();
        assert!(!trace.iter().any(|op| matches!(op, FsOp::Replace(..))));
        assert_eq!(
            trace.last(),
            Some(&FsOp::Remove(path("/out/poster.fake.tmp")))
        );
    }

    // S30-05, ARQ-50: a temporary that cannot be moved is removed, and the
    // final file does not appear.
    #[test]
    fn s30_05_a_failed_move_removes_the_temporary() {
        let mut setup = setup(Plan::Write { files: 1 }, Arc::new(NoLicensing), true);
        let page = first_page(&mut setup.engine);
        setup.fs.take_trace();
        // Create, write, sync, then the replace at index 3 fails.
        setup.fs.fail_at(&[3]);
        assert!(matches!(
            export(&setup, &request("fake", page)),
            Err(EngineError::Io(_))
        ));
        assert_eq!(setup.fs.file("/out/poster.fake"), None);
        assert_eq!(setup.fs.file("/out/poster.fake.tmp"), None);
    }

    // S30-06: without a licence that allows exporting nothing is opened and
    // nothing is rendered.
    #[test]
    fn s30_06_without_the_licence_the_export_is_refused_first() {
        let mut setup = setup(Plan::Write { files: 1 }, Arc::new(NoLicense), true);
        assert!(!setup.engine.license().can_export());
        let page = first_page(&mut setup.engine);
        setup.fs.take_trace();
        assert!(matches!(
            export(&setup, &request("fake", page)),
            Err(EngineError::ExportNotAllowed)
        ));
        assert!(setup.fs.take_trace().is_empty());
        assert!(setup.pages.snapshots.lock().unwrap().is_empty());
        assert!(setup.exporter.rendered.lock().unwrap().is_empty());
    }

    // S30-05: the service lists the exporters of the registry.
    #[test]
    fn s30_05_the_service_lists_the_registered_exporters() {
        let setup = setup(Plan::Write { files: 1 }, Arc::new(NoLicensing), false);
        let ids: Vec<&str> = setup
            .engine
            .export()
            .exporters()
            .map(|e| e.descriptor().id)
            .collect();
        assert_eq!(ids, ["fake"]);
    }

    /// A decoder that answers an image as wide as the bytes and one pixel
    /// high, and counts its calls; the bytes `bad` do not decode.
    #[derive(Default)]
    struct WidthDecoder {
        calls: Mutex<usize>,
    }

    impl gafira_ports::export::ImageDecoder for WidthDecoder {
        fn decode(&self, bytes: &[u8]) -> Result<ImageBuf, gafira_ports::media::MediaError> {
            *self.calls.lock().unwrap() += 1;
            if bytes == b"bad" {
                return Err(gafira_ports::media::MediaError::Corrupt {
                    detail: "not an image".to_owned(),
                });
            }
            let width = bytes.len() as u32;
            Ok(
                ImageBuf::new(width, 1, PixelBuf::Rgba8(vec![255; 4 * bytes.len()]))
                    .expect("an image"),
            )
        }
    }

    /// An image setup: the engine of [`setup`] over `store` and a
    /// [`WidthDecoder`], with a page that shows `assets`, one image element
    /// per asset, and the asset entries in the document.
    fn image_setup(
        store: &Arc<MemoryStore>,
        assets: &[AssetRef],
    ) -> (Setup, Arc<WidthDecoder>, PageId) {
        let fs = MemoryFs::default();
        let pages = Arc::new(CountingPages::default());
        let decoder = Arc::new(WidthDecoder::default());
        let exporter = FakeExporter::new(Plan::Write { files: 1 });
        let mut ports = six_ports();
        ports.filesystem = Arc::new(fs.clone());
        ports.license = Arc::new(NoLicensing);
        ports.store = Arc::clone(store) as Arc<dyn ContentStore>;
        ports.pages = Arc::clone(&pages) as Arc<dyn PageRenderers>;
        ports.images = Arc::clone(&decoder) as Arc<dyn gafira_ports::export::ImageDecoder>;
        let mut registries = Registries::new();
        registries
            .formats
            .register_exporter(Arc::clone(&exporter) as Arc<dyn Exporter>)
            .expect("the fake registers");
        let mut engine =
            Gafira::new(ports, registries, config(), quiet_channel()).expect("an engine");
        engine.projects().new_document().expect("a new document");
        let page = append_page(&mut engine);
        let mut specs = Vec::new();
        for (at, asset) in assets.iter().enumerate() {
            let entry = AssetEntry {
                name: format!("photo-{at}.png"),
                storage: AssetStorage::Embedded,
            };
            let _ = engine
                .edit()
                .execute(Box::new(InsertAsset::new(*asset, entry)));
            specs.push(NodeSpec::new(
                NodeKind::Image(ImageNode::new(*asset)),
                Placement::new(0.0, 0.0, 4.0, 4.0),
            ));
        }
        engine
            .edit()
            .execute(Box::new(InsertNodes::new(page, None, 0, specs)))
            .expect("the images go in");
        fs.take_trace();
        let setup = Setup {
            engine,
            fs,
            pages,
            exporter,
        };
        (setup, decoder, page)
    }

    /// The asset of `bytes` once `store` holds them.
    fn stored(store: &MemoryStore, bytes: &[u8]) -> AssetRef {
        AssetRef::new(store.put(&mut &bytes[..]).expect("the store takes it"))
    }

    // S32-03: the export hands the renderer the decoded pixels of every image
    // asset of the scope, each asset decoded once.
    #[test]
    fn s32_03_the_export_resolves_and_decodes_the_images_of_the_scope() {
        let store = Arc::new(MemoryStore::default());
        let wide = stored(&store, b"abc");
        let narrow = stored(&store, b"z");
        let (setup, decoder, page) = image_setup(&store, &[wide, narrow, wide]);
        export(&setup, &request("fake", page)).expect("the export runs");
        let given = setup.pages.images.lock().unwrap();
        let [images] = &given[..] else {
            panic!("one snapshot");
        };
        let sizes: Vec<(AssetRef, u32)> = images
            .iter()
            .map(|(asset, image)| (*asset, image.width()))
            .collect();
        let mut expected = vec![(wide, 3), (narrow, 1)];
        expected.sort();
        assert_eq!(sizes, expected);
        assert_eq!(*decoder.calls.lock().unwrap(), 2, "once per asset");
        assert!(setup.fs.file("/out/poster.fake").is_some());
    }

    // S85-02, S85-08: an image with a look goes with its look, its shadow
    // around it, and the same asset without one goes as it is; a mask that is
    // not there fails like an image.
    #[test]
    fn s85_08_the_export_draws_the_look_of_an_image() {
        let store = Arc::new(MemoryStore::default());
        let photo = stored(&store, &[7; 40]);
        let mask = stored(&store, b"mask");
        let look = |mask| ImageLook {
            zones: vec![ImageZone {
                kind: ZoneKind::Area,
                mask,
                inverted: false,
                adjust: ImageAdjust {
                    invert: true,
                    ..ImageAdjust::default()
                },
            }],
            shadow: Some(ImageShadow::new(ShadowKind::Outline)),
            ..ImageLook::default()
        };
        let run = |mask: AssetRef| {
            let (mut setup, _, page) = image_setup(&store, &[photo]);
            let _ = setup.engine.edit().execute(Box::new(InsertAsset::new(
                mask,
                AssetEntry {
                    name: "mask.png".to_owned(),
                    storage: AssetStorage::Embedded,
                },
            )));
            let image = ImageNode::new(photo).looking(look(mask));
            let shown = image.shown();
            setup
                .engine
                .edit()
                .execute(Box::new(InsertNodes::new(
                    page,
                    None,
                    0,
                    vec![NodeSpec::new(
                        NodeKind::Image(image),
                        Placement::new(0.0, 0.0, 4.0, 4.0),
                    )],
                )))
                .expect("the image goes in");
            (export(&setup, &request("fake", page)), setup, shown)
        };
        let (done, setup, shown) = run(mask);
        done.expect("the export runs");
        let given = setup.pages.images.lock().unwrap();
        let [images] = &given[..] else {
            panic!("one snapshot");
        };
        assert_eq!(images.len(), 2);
        assert_eq!(images[&photo].width(), 40);
        let with = &images[&shown];
        assert_eq!((with.width(), with.height()), (42, 3));
        let PixelBuf::Rgba8(bytes) = with.data() else {
            panic!("8 bits");
        };
        // The second row, past the margin: white turned black.
        assert_eq!(bytes[(42 + 1) * 4..][..4], [0, 0, 0, 255]);
        let lost = AssetRef::new(gafira_doc::ContentHash([9; 32]));
        assert!(matches!(
            run(lost).0,
            Err(EngineError::Export(ExportError::Store(_)))
        ));
    }

    // S85-07: the thumbnail of a save works out a look on a small image; the
    // export, on the whole one.
    #[test]
    fn s85_07_a_look_seen_small_is_worked_out_small() {
        let store = Arc::new(MemoryStore::default());
        let photo = stored(&store, &[7; 4000]);
        let (mut setup, decoder, page) = image_setup(&store, &[photo]);
        let mut look = ImageLook::default();
        look.adjust.contrast = 20.0;
        let image = ImageNode::new(photo).looking(look);
        let shown = image.shown();
        setup
            .engine
            .edit()
            .execute(Box::new(InsertNodes::new(
                page,
                None,
                0,
                vec![NodeSpec::new(
                    NodeKind::Image(image),
                    Placement::new(0.0, 0.0, 4.0, 4.0),
                )],
            )))
            .expect("the image goes in");
        let document = setup.engine.edit().document().expect("a document");
        let small = super::decode_images(&document, &[page], &*store, &*decoder).expect("images");
        assert_eq!((small[&shown].width(), small[&photo].width()), (1000, 4000));
        export(&setup, &request("fake", page)).expect("the export runs");
        let given = setup.pages.images.lock().unwrap();
        assert_eq!(given[0][&shown].width(), 4000);
    }

    // S32-04: an image whose bytes are not in the store fails the export with
    // the missing hash, before any file is opened or any renderer is built.
    #[test]
    fn s32_04_a_missing_asset_fails_before_any_file() {
        let store = Arc::new(MemoryStore::default());
        let missing = AssetRef::new(gafira_doc::ContentHash([9; 32]));
        let (setup, _, page) = image_setup(&store, &[missing]);
        match export(&setup, &request("fake", page)) {
            Err(EngineError::Export(ExportError::Store(StoreError::NotFound { hash }))) => {
                assert_eq!(hash, missing.as_hash());
            }
            other => panic!("a missing asset, got {other:?}"),
        }
        assert!(setup.fs.take_trace().is_empty(), "no file was opened");
        assert!(setup.pages.snapshots.lock().unwrap().is_empty());
        assert!(setup.exporter.rendered.lock().unwrap().is_empty());
    }

    // S32-04: an asset that does not decode fails the same way, as a media
    // error.
    #[test]
    fn s32_04_an_asset_that_does_not_decode_fails_before_any_file() {
        let store = Arc::new(MemoryStore::default());
        let bad = stored(&store, b"bad");
        let (setup, _, page) = image_setup(&store, &[bad]);
        assert!(matches!(
            export(&setup, &request("fake", page)),
            Err(EngineError::Export(ExportError::Media(_)))
        ));
        assert!(setup.fs.take_trace().is_empty(), "no file was opened");
        assert!(setup.pages.snapshots.lock().unwrap().is_empty());
    }

    // S35-01: `prepare` checks and refuses like `export`, before any render
    // or file, and what it returns can go to another thread.
    #[test]
    fn s35_01_prepare_checks_first_and_the_job_is_send() {
        fn send_static<T: Send + 'static>(_: &T) {}
        let mut setup = setup(Plan::Write { files: 1 }, Arc::new(NoLicensing), true);
        let page = first_page(&mut setup.engine);
        setup.fs.take_trace();
        let job = setup
            .engine
            .export()
            .prepare(request("fake", page), path("/out/poster.fake"))
            .expect("prepares");
        send_static(&job);
        assert_eq!(job.path(), Path::new("/out/poster.fake"));
        assert!(setup.fs.take_trace().is_empty(), "no file yet");
        assert!(
            setup.pages.snapshots.lock().unwrap().is_empty(),
            "no render yet"
        );

        let refused = self::setup(Plan::Write { files: 1 }, Arc::new(NoLicense), true);
        assert!(matches!(
            refused
                .engine
                .export()
                .prepare(request("fake", page), path("/out/poster.fake")),
            Err(EngineError::ExportNotAllowed)
        ));
        assert!(matches!(
            setup
                .engine
                .export()
                .prepare(request("tiff", page), path("/out/poster.fake")),
            Err(EngineError::UnknownFormat)
        ));
    }

    // S35-02: the job runs over the document as it was when it was
    // prepared; later edits do not reach it. The host runs it on its own
    // thread (S35-01: `Send + 'static`; the engine spawns none, ARQ-02).
    #[test]
    fn s35_02_the_job_runs_over_its_snapshot() {
        let mut setup = setup(Plan::Write { files: 1 }, Arc::new(NoLicensing), true);
        let page = first_page(&mut setup.engine);
        let job = setup
            .engine
            .export()
            .prepare(request("fake", page), path("/out/poster.fake"))
            .expect("prepares");
        append_page(&mut setup.engine);
        let report = job
            .run(&NoProgress, &CancelToken::new())
            .expect("the export runs");
        assert_eq!(report.files.len(), 1);
        assert_eq!(
            setup.fs.file("/out/poster.fake").as_deref(),
            Some(&b"PNG!"[..])
        );
        assert_eq!(
            *setup.pages.snapshots.lock().unwrap(),
            [1],
            "the snapshot of prepare"
        );
    }

    // S35-03: a cancelled job renders nothing and leaves no file, final or
    // temporary.
    #[test]
    fn s35_03_a_cancelled_job_leaves_no_file() {
        let mut setup = setup(Plan::Write { files: 1 }, Arc::new(NoLicensing), true);
        let page = first_page(&mut setup.engine);
        let job = setup
            .engine
            .export()
            .prepare(request("fake", page), path("/out/poster.fake"))
            .expect("prepares");
        let cancel = CancelToken::new();
        cancel.cancel();
        assert!(matches!(
            job.run(&NoProgress, &cancel),
            Err(EngineError::Export(ExportError::Cancelled(_)))
        ));
        assert!(setup.pages.snapshots.lock().unwrap().is_empty());
        assert_eq!(setup.fs.file("/out/poster.fake"), None);
        assert_eq!(setup.fs.file("/out/poster.fake.tmp"), None);
    }

    /// An exporter that cancels its own token once it wrote, as a person
    /// pressing «Cancelar» at the last moment.
    struct CancelAfterWrite;

    impl Exporter for CancelAfterWrite {
        fn descriptor(&self) -> &FormatDescriptor {
            &FAKE
        }

        fn capabilities(&self) -> ExportCaps {
            ExportCaps::default()
        }

        fn options_schema(&self) -> &ParamSchema {
            static SCHEMA: std::sync::OnceLock<ParamSchema> = std::sync::OnceLock::new();
            SCHEMA.get_or_init(ParamSchema::default)
        }

        fn validate(&self, _req: &ExportRequest) -> Vec<ExportIssue> {
            Vec::new()
        }

        fn export(
            &self,
            _job: &ExportJob<'_>,
            sink: &mut dyn ExportSink,
            _progress: &dyn gafira_doc::contract::Progress,
            cancel: &CancelToken,
        ) -> Result<ExportReport, ExportError> {
            sink.create("out.fake")?.write_all(b"PNG!")?;
            cancel.cancel();
            Ok(ExportReport::default())
        }
    }

    // S35-03: a cancel that arrives after the exporter wrote still leaves no
    // file: nothing is moved over the final name.
    #[test]
    fn s35_03_a_cancel_after_the_write_moves_nothing() {
        let fs = MemoryFs::default();
        let mut ports = six_ports();
        ports.filesystem = Arc::new(fs.clone());
        ports.license = Arc::new(NoLicensing);
        let mut registries = Registries::new();
        registries
            .formats
            .register_exporter(Arc::new(CancelAfterWrite) as Arc<dyn Exporter>)
            .expect("registers");
        let mut engine = Gafira::new(ports, registries, config(), quiet_channel()).expect("engine");
        engine.projects().new_document().expect("a document");
        let page = first_page(&mut engine);
        let job = engine
            .export()
            .prepare(request("fake", page), path("/out/poster.fake"))
            .expect("prepares");
        assert!(matches!(
            job.run(&NoProgress, &CancelToken::new()),
            Err(EngineError::Export(ExportError::Cancelled(_)))
        ));
        assert_eq!(fs.file("/out/poster.fake"), None);
        assert_eq!(fs.file("/out/poster.fake.tmp"), None);
    }

    /// Renderers that record each request and answer a buffer of its
    /// region: clear when the background is transparent, white otherwise.
    #[derive(Default)]
    struct RecordingPages {
        requests: Arc<Mutex<Vec<PageRenderRequest>>>,
    }

    struct Recorder(Arc<Mutex<Vec<PageRenderRequest>>>);

    impl PageRenderer for Recorder {
        fn render(
            &self,
            req: &PageRenderRequest,
            _cancel: &CancelToken,
        ) -> Result<ImageBuf, RenderError> {
            self.0.lock().unwrap().push(*req);
            let (width, height) = (req.region.width(), req.region.height());
            let value = if req.transparent { 0 } else { 255 };
            Ok(ImageBuf::new(
                width,
                height,
                PixelBuf::Rgba8(vec![value; (width * height * 4) as usize]),
            )?)
        }
    }

    impl PageRenderers for RecordingPages {
        fn for_document(
            &self,
            _document: &Document,
            _images: gafira_ports::export::ImagePixels,
        ) -> Box<dyn PageRenderer> {
            Box::new(Recorder(Arc::clone(&self.requests)))
        }
    }

    /// An engine over `pages` with a new document and one page of
    /// `width` × `height` px.
    fn preview_engine(pages: Arc<RecordingPages>, width: f64, height: f64) -> (Gafira, PageId) {
        let mut ports = six_ports();
        ports.pages = pages;
        let mut engine =
            Gafira::new(ports, Registries::new(), config(), quiet_channel()).expect("an engine");
        engine.projects().new_document().expect("a document");
        let size = gafira_doc::PageSize {
            width: gafira_geom::Length::new(width, gafira_geom::Unit::Px),
            height: gafira_geom::Length::new(height, gafira_geom::Unit::Px),
        };
        let len = engine.edit().document().expect("a session").pages().len();
        engine
            .edit()
            .execute(Box::new(gafira_edit::InsertPage::new(
                len,
                "P".to_owned(),
                size,
            )))
            .expect("the page is added");
        let page = engine.edit().document().expect("a session").pages()[len].id;
        (engine, page)
    }

    // S39-05: the preview draws the page with its long side at 512 px, the
    // proportion kept, in straight 8-bit RGBA, on the background asked for.
    #[test]
    fn s39_05_the_preview_draws_the_page_at_512_px_on_its_long_side() {
        let pages = Arc::new(RecordingPages::default());
        let (engine, page) = preview_engine(Arc::clone(&pages), 2048.0, 1024.0);
        let preview = engine.export().preview(page, true).expect("a preview");
        assert_eq!((preview.width, preview.height), (512, 256));
        assert_eq!(preview.rgba.len(), 512 * 256 * 4);
        assert!(preview.rgba.iter().all(|byte| *byte == 0), "clear");
        let opaque = engine.export().preview(page, false).expect("a preview");
        assert!(opaque.rgba.iter().all(|byte| *byte == 255), "white");
        let requests = pages.requests.lock().unwrap().clone();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].page, page);
        assert_eq!(requests[0].region, IRect::new(0, 0, 512, 256));
        assert!((requests[0].scale - 0.25).abs() < 1e-6);
        assert!(requests[0].transparent);
        assert!(!requests[1].transparent);
    }

    // S39-05: a small page is not enlarged; no document or a page the
    // document does not hold is an error.
    #[test]
    fn s39_05_a_small_page_keeps_its_size_and_a_missing_page_is_an_error() {
        let pages = Arc::new(RecordingPages::default());
        let (engine, page) = preview_engine(Arc::clone(&pages), 100.0, 300.0);
        let preview = engine.export().preview(page, false).expect("a preview");
        assert_eq!((preview.width, preview.height), (100, 300));
        assert!(engine.export().preview(PageId::new(999), false).is_err());
        engine.projects().close();
        assert!(matches!(
            engine.export().preview(page, false),
            Err(EngineError::NoSession)
        ));
    }
}
