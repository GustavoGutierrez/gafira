//! Spec 074: the SVG exporter through the whole drawing path.

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
use gafira_io_svg::{SvgExporter, TEXT_AS_PATHS, TRANSPARENT};
use gafira_ports::common::UnixTime;
use gafira_ports::export::{
    ExportJob, ExportRequest, ExportScope, ExportSink, Exporter, ImagePixels, PageRenderers,
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

/// A page with a circle and the words «Hola mundo» in Inter.
fn document() -> Document {
    let mut page = Page::new(
        PageId::new(1),
        "P".to_owned(),
        PageSize {
            width: Length::new(300.0, Unit::Px),
            height: Length::new(120.0, Unit::Px),
        },
    );
    let circle = NodeId::new(2);
    let mut shape = ShapeNode::new(ShapeGeometry::Ellipse);
    shape.fill = Some(Color::Srgb(Rgba::new(0.0, 0.5, 1.0, 1.0)));
    page.nodes.insert(
        circle,
        Arc::new(Node::new(
            ElementBase::new(circle, Placement::new(10.0, 10.0, 20.0, 20.0)),
            NodeKind::Shape(shape),
        )),
    );
    page.roots.push(circle);
    let style = gafira_doc::SpanStyle {
        font: "Inter".to_owned(),
        size: 32.0,
        color: Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 1.0)),
        bold: false,
        italic: false,
        gradient: None,
    };
    let text = NodeId::new(3);
    page.nodes.insert(
        text,
        Arc::new(Node::new(
            ElementBase::new(text, Placement::new(10.0, 40.0, 280.0, 60.0)),
            NodeKind::Text(gafira_doc::TextNode::plain("Hola mundo", &style)),
        )),
    );
    page.roots.push(text);
    Document::from_parts(4, Dpi::default(), vec![Arc::new(page)]).expect("valid")
}

/// The SVG of the page with `options`.
fn svg_of(options: &[(&'static str, bool)]) -> String {
    svg_of_document(&document(), options)
}

fn svg_of_document(document: &Document, options: &[(&'static str, bool)]) -> String {
    let mut values = ParamValues::new();
    for (id, on) in options {
        values.set(*id, ParamValue::Bool(*on));
    }
    let request = ExportRequest {
        format_id: "svg".to_owned(),
        scope: ExportScope::Pages(vec![PageId::new(1)]),
        options: values,
    };
    let renderers = GraphPageRenderers::new(
        Box::new(|| Box::new(CpuBackend::new())),
        Arc::new(FixedClock),
        Arc::new(FontBook::new().with("Inter", gafira_testkit::fonts::INTER.bytes)),
    );
    let pages = renderers.for_document(document, ImagePixels::new());
    let job = ExportJob {
        request: &request,
        document,
        store: &NoStore,
        pages: pages.as_ref(),
    };
    let mut sink = MemSink::default();
    let report = SvgExporter::new()
        .export(&job, &mut sink, &NoProgress, &CancelToken::new())
        .expect("the export");
    assert_eq!(report.files[0].name, "page.svg");
    let bytes = sink.files[0].1.lock().unwrap().get_ref().clone();
    String::from_utf8(bytes).expect("utf-8")
}

// S74-01: an SVG of vectors: the text is real text in its embedded font,
// the circle a path; an SVG reader opens it.
#[test]
fn s74_01_the_page_as_vectors_and_text() {
    let svg = svg_of(&[]);
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("@font-face"), "the font is embedded");
    assert!(
        svg.contains(">Hola<") || svg.contains(">H<"),
        "the words are text"
    );
    assert!(svg.contains("<path"), "the shapes are paths");
    assert!(svg.contains("fill=\"#ffffff\""), "on the white of the page");
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default());
    assert!(tree.is_ok(), "an SVG reader opens it");
}

// S74-02: «Texto como trazados» draws the text as paths; «Fondo
// transparente» leaves out the white of the page.
#[test]
fn s74_02_outlines_and_transparency() {
    let outlines = svg_of(&[(TEXT_AS_PATHS, true)]);
    assert!(!outlines.contains("<text"));
    assert!(!outlines.contains("@font-face"));
    let clear = svg_of(&[(TRANSPARENT, true)]);
    assert!(!clear.contains("fill=\"#ffffff\""));
}

// S80-12: a QR code goes to SVG as vectors with no outside references, and
// the SVG, drawn and read, gives back what it says.
#[test]
fn s80_12_a_qr_code_reads_back_from_the_svg() {
    use gafira_doc::{QrBody, QrContent, QrEyeFrame, QrFill, QrNode};
    use resvg::{tiny_skia, usvg};
    let text = "https://example.com/gafira";
    let grey = |v: f32| Color::Srgb(Rgba::new(v, v, v, 1.0));
    let url = text.to_owned();
    let mut qr = QrNode::new(QrContent::Url { url }, grey(0.0), grey(1.0));
    qr.style.body = QrBody::Fluid;
    qr.style.eye_frame = QrEyeFrame::Leaf;
    qr.style.fill = QrFill::Gradient(gafira_doc::Gradient {
        kind: gafira_doc::GradientKind::Radial,
        stops: [(0.0, 0.0), (1.0, 0.3)]
            .map(|(offset, v)| gafira_doc::GradientStop {
                offset,
                color: grey(v),
            })
            .to_vec(),
    });
    let mut page = Page::new(
        PageId::new(1),
        "P".to_owned(),
        PageSize {
            width: Length::new(400.0, Unit::Px),
            height: Length::new(400.0, Unit::Px),
        },
    );
    let id = NodeId::new(2);
    let base = ElementBase::new(id, Placement::new(50.0, 50.0, 300.0, 300.0));
    page.nodes.insert(
        id,
        Arc::new(Node::new(base, NodeKind::Qr(Box::new(qr.clone())))),
    );
    page.roots.push(id);
    let document = Document::from_parts(3, Dpi::default(), vec![Arc::new(page)]).expect("valid");
    let svg = svg_of_document(&document, &[]);
    assert!(!svg.contains("<image"), "vectors, not pixels");
    assert!(!svg.contains("href=\"http"), "no outside references");
    let tree = usvg::Tree::from_str(&svg, &usvg::Options::default()).expect("an SVG");
    let mut pixmap = tiny_skia::Pixmap::new(400, 400).expect("a size");
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let drawing = gafira_qr::draw(&qr).expect("drawn");
    let (k, (x0, y0)) = drawing.fit(300.0, 300.0).expect("a box");
    let found = gafira_testkit::qr::read(drawing.size as usize, |x, y| {
        let at = |from: f64, v: f64| ((50.0 + from + v * k).max(0.0) as u32).min(399);
        pixmap.data()[((at(y0, y) * 400 + at(x0, x)) * 4) as usize + 1]
    });
    assert_eq!(found.as_deref(), Some(text));
}
