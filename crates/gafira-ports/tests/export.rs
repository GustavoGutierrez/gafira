//! `Exporter`, `ExportCaps`, `ExportIssue`, `PageRenderer`, `ExportError` and the
//! exporter side of `FormatRegistry` (design P4, P6, P8).

use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use gafira_doc::contract::{
    CancelToken, Cancelled, NoProgress, ParamSchema, ParamValues, Progress,
};
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_doc::{ContentHash, Document, NodeId, PageId, msg};
use gafira_geom::IRect;
use gafira_ports::common::{Confidence, LegalClass};
use gafira_ports::export::{
    ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
    ExportSink, ExportedFile, Exporter, IssueSeverity, PageRenderRequest, PageRenderer,
};
use gafira_ports::format::{
    FormatDescriptor, FormatKind, FormatRegistry, ImportCx, ImportError, ImportOptions, Imported,
    Importer,
};
use gafira_ports::media::MediaError;
use gafira_ports::registry::RegistryError;
use gafira_ports::render::RenderError;
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{ReadSeek, WriteSeek};
use gafira_raster::{ImageBuf, PixelBuf};
use gafira_time::Flicks;

// ---- ExportCaps::contains (P8) ----

type Flag = fn(&mut ExportCaps);

const FLAGS: [(&str, Flag); 6] = [
    ("transparency", |c| c.transparency = true),
    ("multipage", |c| c.multipage = true),
    ("animation", |c| c.animation = true),
    ("cmyk", |c| c.cmyk = true),
    ("bleed", |c| c.bleed = true),
    ("notes", |c| c.notes = true),
];

fn only(flag: Flag) -> ExportCaps {
    let mut caps = ExportCaps::default();
    flag(&mut caps);
    caps
}

fn all() -> ExportCaps {
    let mut caps = ExportCaps::default();
    for (_, flag) in FLAGS {
        flag(&mut caps);
    }
    caps
}

#[test]
fn p8_caps_default_is_all_false() {
    assert_eq!(
        ExportCaps::default(),
        ExportCaps {
            transparency: false,
            multipage: false,
            animation: false,
            cmyk: false,
            bleed: false,
            notes: false,
        }
    );
}

// P8: each of the 6 flags alone, against every flag alone.
#[test]
fn p8_contains_each_flag_alone() {
    for (i, (have_name, have)) in FLAGS.iter().enumerate() {
        for (j, (need_name, need)) in FLAGS.iter().enumerate() {
            assert_eq!(
                only(*have).contains(only(*need)),
                i == j,
                "has {have_name}, requires {need_name}"
            );
        }
    }
}

#[test]
fn p8_contains_needs_every_required_flag() {
    for (name, flag) in FLAGS {
        // Nothing required: always true.
        assert!(
            ExportCaps::default().contains(ExportCaps::default()),
            "{name}"
        );
        assert!(only(flag).contains(ExportCaps::default()), "{name}");
        // Everything held holds each flag alone and all together.
        assert!(all().contains(only(flag)), "{name}");
        assert!(all().contains(all()), "{name}");
        // Missing a flag fails; it is not symmetric (a superset of `self` fails).
        assert!(!ExportCaps::default().contains(only(flag)), "{name}");
        assert!(!only(flag).contains(all()), "{name}");
    }
    let transparency_and_notes = ExportCaps {
        transparency: true,
        notes: true,
        ..ExportCaps::default()
    };
    assert!(!only(FLAGS[0].1).contains(transparency_and_notes));
    assert!(!only(FLAGS[5].1).contains(transparency_and_notes));
    assert!(transparency_and_notes.contains(only(FLAGS[0].1)));
}

// ---- ExportIssue::blocks (FD-58) ----

#[test]
fn fd_58_error_blocks_and_warning_does_not() {
    let issue = |severity| ExportIssue {
        severity,
        message: msg!("export.issue.test"),
    };
    assert!(issue(IssueSeverity::Error).blocks());
    assert!(!issue(IssueSeverity::Warning).blocks());
}

// ---- ExportScope and ExportRequest (FD-57) ----

#[test]
fn fd_57_scope_pages_and_nodes_are_distinct_values() {
    let page = PageId::new(1);
    let pages = ExportScope::Pages(vec![page, PageId::new(2)]);
    let nodes = ExportScope::Nodes {
        page,
        nodes: vec![NodeId::new(5)],
    };
    assert_ne!(pages, nodes);
    assert_eq!(pages, pages.clone());
    let request = ExportRequest {
        format_id: "png".into(),
        scope: nodes,
        options: ParamValues::default(),
    };
    assert_eq!(request, request.clone());
}

// ---- ExportError (P6, ARQ-102) ----

#[test]
fn arq_102_export_error_messages() {
    let hash = ContentHash([0xAB; 32]);
    let io = ExportError::Io(io::Error::other("secret detail")).to_message();
    assert!(is_valid_key(io.key.0), "{:?}", io.key);
    assert_eq!(io.key, MessageKey("error.export.io"));
    assert!(io.args.is_empty(), "library text never becomes an argument");

    // Wrapped errors delegate their message.
    let render = ExportError::Render(RenderError::DeviceLost).to_message();
    assert_eq!(render, RenderError::DeviceLost.to_message());
    assert_eq!(render.key, MessageKey("error.render.device_lost"));

    let media = ExportError::Media(MediaError::SystemCodecUnavailable).to_message();
    assert_eq!(media, MediaError::SystemCodecUnavailable.to_message());

    let store = ExportError::Store(StoreError::NotFound { hash }).to_message();
    assert_eq!(store, StoreError::NotFound { hash }.to_message());
    assert_eq!(store.args, vec![("hash", MsgArg::Text("ab".repeat(32)))]);

    let cancelled = ExportError::Cancelled(Cancelled).to_message();
    assert_eq!(cancelled, Cancelled.to_message());
    assert_eq!(cancelled.key, MessageKey("error.job.cancelled"));
}

// ADR 0012 rule 4 via ADR 0014: `?` converts every wrapped error.
#[test]
fn adr_0014_export_error_from_each_wrapped_error() {
    let e: ExportError = Cancelled.into();
    assert!(matches!(e, ExportError::Cancelled(_)), "{e:?}");
    let e: ExportError = RenderError::OutOfMemory.into();
    assert!(
        matches!(e, ExportError::Render(RenderError::OutOfMemory)),
        "{e:?}"
    );
    let e: ExportError = MediaError::SystemCodecUnavailable.into();
    assert!(matches!(e, ExportError::Media(_)), "{e:?}");
    let e: ExportError = StoreError::NotFound {
        hash: ContentHash([0; 32]),
    }
    .into();
    assert!(matches!(e, ExportError::Store(_)), "{e:?}");
    let e: ExportError = io::Error::other("x").into();
    assert!(matches!(e, ExportError::Io(_)), "{e:?}");
}

// ---- doubles ----

struct Fake {
    d: FormatDescriptor,
    caps: ExportCaps,
    schema: ParamSchema,
}

impl Exporter for Fake {
    fn descriptor(&self) -> &FormatDescriptor {
        &self.d
    }
    fn capabilities(&self) -> ExportCaps {
        self.caps
    }
    fn options_schema(&self) -> &ParamSchema {
        &self.schema
    }
    fn validate(&self, _req: &ExportRequest) -> Vec<ExportIssue> {
        Vec::new()
    }
    fn export(
        &self,
        job: &ExportJob<'_>,
        sink: &mut dyn ExportSink,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<ExportReport, ExportError> {
        // The only way to pixels is the injected `PageRenderer` (02 §4.5, ARQ-06).
        let ExportScope::Pages(pages) = &job.request.scope else {
            return Ok(ExportReport::default());
        };
        let mut report = ExportReport::default();
        for (i, page) in pages.iter().enumerate() {
            cancel.check()?;
            let image = job.pages.render(
                &PageRenderRequest {
                    page: *page,
                    time: Flicks::from_ticks(0),
                    scale: 1.0,
                    region: IRect::new(0, 0, 1, 1),
                    transparent: self.caps.transparency,
                },
                cancel,
            )?;
            let name = format!("page-{}.{}", i + 1, self.d.id);
            let mut out = sink.create(&name)?;
            out.write_all(b"IMG")?;
            report.files.push(ExportedFile { name, bytes: 3 });
            let _ = image;
            progress.report(1.0);
        }
        Ok(report)
    }
}

fn descriptor(id: &'static str) -> FormatDescriptor {
    FormatDescriptor {
        id,
        extensions: &[],
        mime: &[],
        signatures: &[],
        kind: FormatKind::Raster,
        legal: LegalClass::RoyaltyFree,
    }
}

fn exporter(id: &'static str, caps: ExportCaps) -> Arc<dyn Exporter> {
    Arc::new(Fake {
        d: descriptor(id),
        caps,
        schema: ParamSchema::default(),
    })
}

struct Reader {
    d: FormatDescriptor,
}

impl Importer for Reader {
    fn descriptor(&self) -> &FormatDescriptor {
        &self.d
    }
    fn sniff(&self, _head: &[u8]) -> Confidence {
        Confidence::None
    }
    fn import(
        &self,
        _src: &mut dyn ReadSeek,
        _opt: &ImportOptions,
        _cx: &mut ImportCx<'_>,
    ) -> Result<Imported, ImportError> {
        Err(ImportError::Unsupported)
    }
}

fn importer(id: &'static str) -> Arc<dyn Importer> {
    Arc::new(Reader { d: descriptor(id) })
}

fn ids(reg: &FormatRegistry) -> Vec<&'static str> {
    reg.exporters().map(|e| e.descriptor().id).collect()
}

fn with(reg: &FormatRegistry, required: ExportCaps) -> Vec<&'static str> {
    reg.exporters_with(required)
        .map(|e| e.descriptor().id)
        .collect()
}

// ---- register_exporter (P8) ----

#[test]
fn arq_20_exporter_sink_and_renderer_are_object_safe() {
    fn _exporter(_: &dyn Exporter) {}
    fn _sink(_: &mut dyn ExportSink) {}
    fn _renderer(_: &dyn PageRenderer) {}
    let reg = FormatRegistry::new();
    assert_eq!(reg.exporters().count(), 0);
}

#[test]
fn p8_duplicate_exporter_id_is_rejected_and_changes_nothing() {
    let mut reg = FormatRegistry::new();
    reg.register_exporter(exporter("png", ExportCaps::default()))
        .unwrap();
    reg.register_exporter(exporter("pdf", all())).unwrap();
    let err = reg.register_exporter(exporter("png", all())).unwrap_err();
    assert!(
        matches!(err, RegistryError::DuplicateId { id: "png" }),
        "{err:?}"
    );
    assert_eq!(ids(&reg), ["png", "pdf"]);
    // The old one stays: it is not replaced by the repeated id.
    assert_eq!(
        reg.exporters().next().unwrap().capabilities(),
        ExportCaps::default()
    );
}

// P8: `.gaf` is both an importer and an exporter.
#[test]
fn p8_same_id_as_importer_and_exporter_is_valid() {
    let mut reg = FormatRegistry::new();
    reg.register_importer(importer("gafira")).unwrap();
    reg.register_exporter(exporter("gafira", all())).unwrap();
    assert_eq!(ids(&reg), ["gafira"]);
    assert_eq!(reg.importers().count(), 1);

    // The other order, and each list still rejects its own repeat.
    let mut reg = FormatRegistry::new();
    reg.register_exporter(exporter("gafira", all())).unwrap();
    reg.register_importer(importer("gafira")).unwrap();
    assert!(reg.register_importer(importer("gafira")).is_err());
    assert!(reg.register_exporter(exporter("gafira", all())).is_err());
    assert_eq!(reg.importers().count(), 1);
    assert_eq!(ids(&reg), ["gafira"]);
}

// ---- exporters_with (P8) ----

#[test]
fn p8_exporters_keep_registration_order() {
    let mut reg = FormatRegistry::new();
    for id in ["zeta", "alpha", "mid"] {
        reg.register_exporter(exporter(id, all())).unwrap();
    }
    assert_eq!(ids(&reg), ["zeta", "alpha", "mid"]);
}

#[test]
fn p8_exporters_with_filters_by_capabilities_in_registration_order() {
    let mut reg = FormatRegistry::new();
    reg.register_exporter(exporter("zeta", all())).unwrap();
    reg.register_exporter(exporter("plain", ExportCaps::default()))
        .unwrap();
    reg.register_exporter(exporter("alpha", only(FLAGS[0].1)))
        .unwrap();
    reg.register_exporter(exporter("mid", all())).unwrap();

    // Nothing required: all, in registration order (not id order).
    assert_eq!(
        with(&reg, ExportCaps::default()),
        ["zeta", "plain", "alpha", "mid"]
    );
    // Transparency: those holding it, in registration order.
    assert_eq!(with(&reg, only(FLAGS[0].1)), ["zeta", "alpha", "mid"]);
    // Each flag alone is honoured, not just the first.
    for (name, flag) in &FLAGS[1..] {
        assert_eq!(with(&reg, only(*flag)), ["zeta", "mid"], "{name}");
    }
    // A subset is not enough.
    assert_eq!(
        with(
            &reg,
            ExportCaps {
                transparency: true,
                cmyk: true,
                ..ExportCaps::default()
            }
        ),
        ["zeta", "mid"]
    );
}

// ---- PageRenderer reaches an exporter through the job (02 §4.5, ARQ-06) ----

struct Pages {
    asked: Mutex<Vec<(PageId, bool)>>,
}

impl PageRenderer for Pages {
    fn render(
        &self,
        req: &PageRenderRequest,
        cancel: &CancelToken,
    ) -> Result<ImageBuf, RenderError> {
        cancel.check()?;
        self.asked.lock().unwrap().push((req.page, req.transparent));
        Ok(ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![0, 0, 0, 255]))?)
    }
}

struct NoStore;

impl ContentStore for NoStore {
    fn put(&self, _src: &mut dyn Read) -> Result<ContentHash, StoreError> {
        Err(StoreError::Io(io::Error::other("unused")))
    }
    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::NotFound { hash })
    }
    fn contains(&self, _hash: ContentHash) -> Result<bool, StoreError> {
        Ok(false)
    }
}

#[derive(Default)]
struct MemSink {
    names: Vec<String>,
    fail: bool,
}

impl ExportSink for MemSink {
    fn create(&mut self, name: &str) -> io::Result<Box<dyn WriteSeek + Send>> {
        if self.fail {
            return Err(io::Error::other("disk full"));
        }
        self.names.push(name.into());
        Ok(Box::new(Cursor::new(Vec::new())))
    }
}

fn request(pages: &[u64]) -> ExportRequest {
    ExportRequest {
        format_id: "png".into(),
        scope: ExportScope::Pages(pages.iter().map(|p| PageId::new(*p)).collect()),
        options: ParamValues::default(),
    }
}

fn run(
    sink: &mut MemSink,
    cancel: &CancelToken,
) -> (Result<ExportReport, ExportError>, Vec<(PageId, bool)>) {
    let mut reg = FormatRegistry::new();
    reg.register_exporter(exporter("png", only(FLAGS[0].1)))
        .unwrap();
    let request = request(&[7, 9]);
    let document = Document::default();
    let pages = Pages {
        asked: Mutex::default(),
    };
    let job = ExportJob {
        request: &request,
        document: &document,
        store: &NoStore,
        pages: &pages,
    };
    let exporter = reg.exporters().next().unwrap();
    let r = exporter.export(&job, sink, &NoProgress, cancel);
    let asked = pages.asked.lock().unwrap().clone();
    (r, asked)
}

// 02 §4.5: an exporter gets each page as an `ImageBuf` from `ExportJob::pages`.
#[test]
fn arq_06_exporter_gets_pages_through_the_page_renderer() {
    let mut sink = MemSink::default();
    let (r, asked) = run(&mut sink, &CancelToken::new());
    let report = r.unwrap();
    assert_eq!(asked, [(PageId::new(7), true), (PageId::new(9), true)]);
    assert_eq!(sink.names, ["page-1.png", "page-2.png"]);
    assert_eq!(report.files.len(), 2);
    assert_eq!(report.files[0].name, "page-1.png");
}

#[test]
fn arq_06_renderer_and_sink_errors_reach_the_caller_as_export_errors() {
    let cancel = CancelToken::new();
    cancel.cancel();
    let (r, asked) = run(&mut MemSink::default(), &cancel);
    assert!(matches!(r, Err(ExportError::Cancelled(_))), "{r:?}");
    assert!(asked.is_empty());

    let (r, _) = run(
        &mut MemSink {
            fail: true,
            ..MemSink::default()
        },
        &CancelToken::new(),
    );
    assert!(matches!(r, Err(ExportError::Io(_))), "{r:?}");
}

// `Seek` is part of the sink's writer: an exporter may patch a header.
#[test]
fn p4_sink_writer_can_seek_back() {
    let mut sink = MemSink::default();
    let mut out = sink.create("x").unwrap();
    out.write_all(b"abcd").unwrap();
    out.seek(SeekFrom::Start(1)).unwrap();
    out.write_all(b"Z").unwrap();
    assert_eq!(out.stream_position().unwrap(), 2);
}
