//! The PDF exporter over the real pipeline, read back by a PDF parser
//! (spec 030, FD-56).

use std::io::{self, Cursor};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{CancelToken, NoProgress, ParamValue, ParamValues};
use gafira_doc::{
    Document, Dpi, ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeGeometry, ShapeNode,
};
use gafira_geom::{Length, Unit};
use gafira_io_pdf::{FILE_NAME, PdfExporter, SCALE, page_points};
use gafira_ports::common::UnixTime;
use gafira_ports::export::{
    ExportError, ExportJob, ExportReport, ExportRequest, ExportScope, ExportSink, Exporter,
    PageRenderers,
};
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{Clock, ReadSeek, WriteSeek};
use gafira_render::{FontBook, GraphPageRenderers};
use gafira_render_cpu::CpuBackend;

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

struct NoStore;

impl ContentStore for NoStore {
    fn put(&self, _src: &mut dyn io::Read) -> Result<gafira_doc::ContentHash, StoreError> {
        Err(StoreError::Io(io::Error::other("unused")))
    }
    fn open(&self, hash: gafira_doc::ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::NotFound { hash })
    }
    fn contains(&self, _hash: gafira_doc::ContentHash) -> Result<bool, StoreError> {
        Ok(false)
    }
}

/// One file in memory, shared by the sink and its writer.
type MemFile = Arc<Mutex<Cursor<Vec<u8>>>>;

#[derive(Default)]
struct MemSink {
    files: Vec<(String, MemFile)>,
}

struct Shared(MemFile);

impl io::Write for Shared {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Seek for Shared {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.0.lock().unwrap().seek(pos)
    }
}

impl ExportSink for MemSink {
    fn create(&mut self, name: &str) -> io::Result<Box<dyn WriteSeek + Send>> {
        let file = Arc::new(Mutex::new(Cursor::new(Vec::new())));
        self.files.push((name.to_owned(), Arc::clone(&file)));
        Ok(Box::new(Shared(file)))
    }
}

fn page(id: u64, width: f64, height: f64, square: Option<u64>) -> Arc<Page> {
    let mut page = Page::new(
        PageId::new(id),
        "P".to_owned(),
        PageSize {
            width: Length::new(width, Unit::Px),
            height: Length::new(height, Unit::Px),
        },
    );
    if let Some(node) = square {
        let id = NodeId::new(node);
        let base = ElementBase::new(id, Placement::new(10.0, 10.0, 20.0, 20.0));
        let kind = NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Ellipse,
            fill: Some(Color::Srgb(Rgba::new(0.0, 0.5, 1.0, 0.5))),
        });
        page.nodes.insert(id, Arc::new(Node::new(base, kind)));
        page.roots.push(id);
    }
    Arc::new(page)
}

fn document() -> Document {
    Document::from_parts(
        4,
        Dpi::default(),
        vec![page(1, 100.0, 50.0, Some(3)), page(2, 200.0, 100.0, None)],
    )
    .expect("the document validates")
}

fn request(pages: &[u64], scale: f64) -> ExportRequest {
    let mut options = ParamValues::new();
    options.set(SCALE, ParamValue::Float(scale));
    ExportRequest {
        format_id: "pdf".to_owned(),
        scope: ExportScope::Pages(pages.iter().map(|p| PageId::new(*p)).collect()),
        options,
    }
}

fn export(
    document: &Document,
    request: &ExportRequest,
    sink: &mut MemSink,
) -> Result<ExportReport, ExportError> {
    let renderers = GraphPageRenderers::new(
        Box::new(|| Box::new(CpuBackend::new())),
        Arc::new(FixedClock),
        Arc::new(FontBook::new()),
    );
    let pages = renderers.for_document(document);
    let job = ExportJob {
        request,
        document,
        store: &NoStore,
        pages: pages.as_ref(),
    };
    PdfExporter::new().export(&job, sink, &NoProgress, &CancelToken::new())
}

// S30-04: a document of two pages is one PDF of two pages, each the size of
// its page in points, that a reader parses.
#[test]
fn s30_04_the_document_exports_as_one_pdf_page_per_page() {
    let mut sink = MemSink::default();
    let report = export(&document(), &request(&[1, 2], 2.0), &mut sink).expect("the export");
    assert_eq!(sink.files.len(), 1);
    assert_eq!(sink.files[0].0, FILE_NAME);
    let bytes = sink.files[0].1.lock().unwrap().get_ref().clone();
    assert_eq!(report.files[0].bytes, bytes.len() as u64);
    assert!(bytes.starts_with(b"%PDF-"));
    let pdf = hayro_syntax::Pdf::new(bytes).expect("a reader parses the PDF");
    let pages = pdf.pages();
    assert_eq!(pages.len(), 2);
    // 96 dpi: 100 × 50 px are 75 × 37.5 pt, 200 × 100 px are 150 × 75 pt.
    let sizes: Vec<(f32, f32)> = pages.iter().map(|p| p.base_dimensions()).collect();
    assert_eq!(sizes, [(75.0, 37.5), (150.0, 75.0)]);
}

// S30-04: the size in points follows the dpi of the document.
#[test]
fn s30_04_page_points_follow_the_dpi() {
    let page = page(1, 300.0, 150.0, None);
    assert_eq!(page_points(&page, Dpi::default()).unwrap(), (225.0, 112.5));
    let print = Dpi::new(300);
    assert_eq!(page_points(&page, print).unwrap(), (72.0, 36.0));
}

// FD-58: a resolution out of range or a selection block the export; a page
// outside the document writes nothing.
#[test]
fn s30_04_what_blocks_the_export_writes_nothing() {
    let exporter = PdfExporter::new();
    assert!(exporter.validate(&request(&[1, 2], 2.0)).is_empty());
    assert!(
        exporter
            .validate(&request(&[1], 9.0))
            .iter()
            .any(|issue| issue.blocks())
    );
    assert!(
        exporter
            .validate(&ExportRequest {
                format_id: "pdf".to_owned(),
                scope: ExportScope::Nodes {
                    page: PageId::new(1),
                    nodes: vec![NodeId::new(3)],
                },
                options: ParamValues::new(),
            })
            .iter()
            .any(|issue| issue.blocks())
    );
    let mut sink = MemSink::default();
    assert!(export(&document(), &request(&[1, 9], 1.0), &mut sink).is_err());
    assert!(sink.files.is_empty());
}
