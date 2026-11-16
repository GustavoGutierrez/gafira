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
//! This cut runs the export on the caller's thread, to the end; the
//! persisted export queue, its states and its retries (02 §6.4) are later.

use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::export::{
    ExportJob, ExportReport, ExportRequest, ExportSink, Exporter, PageRenderers,
};
use gafira_ports::format::FormatRegistry;
use gafira_ports::store::ContentStore;
use gafira_ports::system::{FileSystem, FileWrite, WriteSeek};

use crate::engine::{EngineError, Ports};
use crate::license::LicenseService;
use crate::session::Shared;

/// Exports the open document (02 §5.4 line 1361).
pub struct ExportService {
    shared: Shared,
    license: Arc<LicenseService>,
    exporters: Vec<Arc<dyn Exporter>>,
    pages: Arc<dyn PageRenderers>,
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
            store: Arc::clone(&ports.store),
            filesystem: Arc::clone(&ports.filesystem),
        }
    }

    /// The exporters of the registry, in registration order.
    pub fn exporters(&self) -> impl Iterator<Item = &Arc<dyn Exporter>> {
        self.exporters.iter()
    }

    /// Export the open document as `request` says, to `to`.
    ///
    /// The first file the exporter writes goes to `to`; a second one to `to`
    /// with `-2` before its extension, and so on (FD-57 (c)). The files
    /// appear only when the whole export succeeded.
    ///
    /// # Errors
    /// [`EngineError::ExportNotAllowed`] when the licence forbids it,
    /// [`EngineError::NoSession`] with no open document,
    /// [`EngineError::UnknownFormat`] when no exporter has the format id and
    /// [`EngineError::ExportBlocked`] with the blocking findings of
    /// `validate`, all before any file is opened; [`EngineError::Export`]
    /// when the exporter fails and [`EngineError::Io`] when a file cannot be
    /// finished. No final file is left then.
    pub fn export(
        &self,
        request: &ExportRequest,
        to: &Path,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<ExportReport, EngineError> {
        if !self.license.can_export() {
            return Err(EngineError::ExportNotAllowed);
        }
        let document = self
            .shared
            .lock()
            .as_ref()
            .map(|session| Arc::clone(session.history.document()))
            .ok_or(EngineError::NoSession)?;
        let exporter = self
            .exporters
            .iter()
            .find(|exporter| exporter.descriptor().id == request.format_id)
            .ok_or(EngineError::UnknownFormat)?;
        let blocking: Vec<_> = exporter
            .validate(request)
            .into_iter()
            .filter(|issue| issue.blocks())
            .collect();
        if !blocking.is_empty() {
            return Err(EngineError::ExportBlocked(blocking));
        }
        let pages = self.pages.for_document(&document);
        let job = ExportJob {
            request,
            document: &document,
            store: &*self.store,
            pages: pages.as_ref(),
        };
        let mut sink = FileSink::new(&*self.filesystem, to);
        match exporter.export(&job, &mut sink, progress, cancel) {
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
    use gafira_doc::{Document, PageId};
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
        fn for_document(&self, document: &Document) -> Box<dyn PageRenderer> {
            self.snapshots.lock().unwrap().push(document.pages().len());
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
}
