//! The JPEG and WebP exporters over the real pipeline: the graph of
//! `gafira-render` on the CPU backend (spec 039, FD-55).

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
use gafira_io_image::{
    JPEG_FORMAT, JpegExporter, PngExporter, QUALITY, SCALE, TRANSPARENT, WEBP_FORMAT, WebpExporter,
};
use gafira_ports::common::{LegalClass, UnixTime};
use gafira_ports::export::{
    ExportError, ExportJob, ExportReport, ExportRequest, ExportScope, ExportSink, Exporter,
    ImagePixels, PageRenderers,
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

impl MemSink {
    fn bytes(&self, at: usize) -> Vec<u8> {
        self.files[at].1.lock().unwrap().get_ref().clone()
    }
}

/// A page of 100×50 with a square of `fill` at (10, 10), 20 px a side.
fn square_document(fill: Color) -> Document {
    let square = Arc::new(Node::new(
        ElementBase::new(NodeId::new(2), Placement::new(10.0, 10.0, 20.0, 20.0)),
        NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(fill),
            stroke: None,
            corner_radius: 0.0,
        }),
    ));
    let mut page = Page::new(
        PageId::new(1),
        "P".to_owned(),
        PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(50.0, Unit::Px),
        },
    );
    page.roots.push(square.base.id);
    page.nodes.insert(square.base.id, square);
    Document::from_parts(3, Dpi::default(), vec![Arc::new(page)]).expect("the document validates")
}

const RED: Color = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));

fn request(format: &str, options: &[(&str, ParamValue)]) -> ExportRequest {
    let mut values = ParamValues::new();
    for (id, value) in options {
        values.set(*id, value.clone());
    }
    ExportRequest {
        format_id: format.to_owned(),
        scope: ExportScope::Pages(vec![PageId::new(1)]),
        options: values,
    }
}

fn export(
    exporter: &dyn Exporter,
    document: &Document,
    request: &ExportRequest,
    sink: &mut MemSink,
) -> Result<ExportReport, ExportError> {
    let renderers = GraphPageRenderers::new(
        Box::new(|| Box::new(CpuBackend::new())),
        Arc::new(FixedClock),
        Arc::new(FontBook::new().with("Inter", gafira_testkit::fonts::INTER.bytes)),
    );
    let pages = renderers.for_document(document, ImagePixels::new());
    let job = ExportJob {
        request,
        document,
        store: &NoStore,
        pages: pages.as_ref(),
    };
    exporter.export(&job, sink, &NoProgress, &CancelToken::new())
}

/// Width, height and RGBA pixels of an encoded image.
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let image = image::load_from_memory(bytes)
        .expect("the image decodes")
        .into_rgba8();
    let (width, height) = image.dimensions();
    (width, height, image.into_raw())
}

/// Width, height and RGBA pixels of a WebP (the `image` of the workspace
/// reads no WebP).
fn decode_webp(bytes: &[u8]) -> (u32, u32, Vec<u8>, bool) {
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes)).expect("a WebP");
    let (width, height) = decoder.dimensions();
    assert!(decoder.has_alpha(), "RGBA");
    let mut pixels = vec![0; decoder.output_buffer_size().expect("a size")];
    decoder.read_image(&mut pixels).expect("the WebP decodes");
    // A lossless WebP holds a `VP8L` chunk.
    (
        width,
        height,
        pixels,
        bytes.get(12..16) == Some(&b"VP8L"[..]),
    )
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
}

/// True when every channel of `got` is within `tolerance` of `want`.
fn near(got: [u8; 4], want: [u8; 4], tolerance: u8) -> bool {
    got.iter()
        .zip(want)
        .all(|(g, w)| g.abs_diff(w) <= tolerance)
}

// S39-01: at 2× the JPG is twice the page, starts with the JPEG signature,
// the square is red where it is and the page white around it.
#[test]
fn s39_01_a_page_exports_as_a_jpeg_at_twice_its_size() {
    let mut sink = MemSink::default();
    let report = export(
        &JpegExporter::new(),
        &square_document(RED),
        &request("jpeg", &[(SCALE, ParamValue::Float(2.0))]),
        &mut sink,
    )
    .expect("the export runs");
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].name, "page.jpg");
    let bytes = sink.bytes(0);
    assert!(bytes.starts_with(&[0xFF, 0xD8, 0xFF]), "the JPEG signature");
    let (width, height, pixels) = decode(&bytes);
    assert_eq!((width, height), (200, 100));
    assert!(near(pixel(&pixels, width, 40, 40), [255, 0, 0, 255], 8));
    assert!(near(
        pixel(&pixels, width, 150, 80),
        [255, 255, 255, 255],
        4
    ));
}

// S39-01: a JPEG has no alpha: a half transparent black square comes out
// grey over white, never dark, and the descriptor says it is royalty free.
#[test]
fn s39_01_a_jpeg_composes_the_page_over_white() {
    let half_black = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 0.5));
    let mut sink = MemSink::default();
    export(
        &JpegExporter::new(),
        &square_document(half_black),
        &request("jpeg", &[(TRANSPARENT, ParamValue::Bool(true))]),
        &mut sink,
    )
    .expect("the export runs");
    let (width, _, pixels) = decode(&sink.bytes(0));
    let grey = pixel(&pixels, width, 20, 20);
    assert!(
        (100..=200).contains(&grey[0]),
        "half black over white is grey: {grey:?}"
    );
    assert!(near(pixel(&pixels, width, 80, 40), [255, 255, 255, 255], 4));
    assert!(!JpegExporter::new().capabilities().transparency);
    assert_eq!(JPEG_FORMAT.legal, LegalClass::RoyaltyFree);
    assert_eq!(JPEG_FORMAT.extensions, ["jpg", "jpeg"]);
}

// S39-01: the quality is 1 to 100, 90 by default; a lower one gives a
// smaller file.
#[test]
fn s39_01_the_jpeg_quality_is_checked_and_used() {
    let exporter = JpegExporter::new();
    let schema = exporter.options_schema();
    assert_eq!(schema.defaults().int(QUALITY), Some(90));
    for bad in [0, 101] {
        let issues = exporter.validate(&request("jpeg", &[(QUALITY, ParamValue::Int(bad))]));
        assert!(issues.iter().any(|issue| issue.blocks()), "quality {bad}");
    }
    assert!(
        exporter
            .validate(&request("jpeg", &[(QUALITY, ParamValue::Int(1))]))
            .is_empty()
    );
    let size = |quality: i64| {
        let mut sink = MemSink::default();
        let document = square_document(RED);
        let request = request(
            "jpeg",
            &[
                (QUALITY, ParamValue::Int(quality)),
                (SCALE, ParamValue::Float(4.0)),
            ],
        );
        export(&exporter, &document, &request, &mut sink).expect("the export runs");
        sink.bytes(0).len()
    };
    assert!(size(10) < size(100), "a lower quality is smaller");
}

// S39-02: a lossless WebP reads back with the pixels of the PNG of the
// same page.
#[test]
fn s39_02_a_lossless_webp_has_the_pixels_of_the_png() {
    let document = square_document(Color::Srgb(Rgba::new(0.2, 0.6, 0.4, 0.7)));
    let options = [(SCALE, ParamValue::Float(2.0))];
    let mut webp = MemSink::default();
    let report = export(
        &WebpExporter::new(),
        &document,
        &request("webp", &options),
        &mut webp,
    )
    .expect("the WebP export runs");
    assert_eq!(report.files[0].name, "page.webp");
    let mut png = MemSink::default();
    export(
        &PngExporter::new(),
        &document,
        &request("png", &options),
        &mut png,
    )
    .expect("the PNG export runs");
    let bytes = webp.bytes(0);
    assert!(bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP");
    let (width, height, pixels, lossless) = decode_webp(&bytes);
    assert!(lossless, "lossless");
    assert_eq!((width, height), (200, 100));
    assert_eq!(pixels, decode(&png.bytes(0)).2, "the same pixels");
    assert_eq!(WEBP_FORMAT.legal, LegalClass::RoyaltyFree);
}

// S39-02: with a transparent background the WebP is clear where there is
// no content.
#[test]
fn s39_02_a_transparent_webp_is_clear_where_there_is_no_content() {
    let mut sink = MemSink::default();
    export(
        &WebpExporter::new(),
        &square_document(RED),
        &request("webp", &[(TRANSPARENT, ParamValue::Bool(true))]),
        &mut sink,
    )
    .expect("the export runs");
    let (width, _, pixels, _) = decode_webp(&sink.bytes(0));
    assert_eq!(pixel(&pixels, width, 20, 20), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, width, 5, 5)[3], 0);
    assert!(WebpExporter::new().capabilities().transparency);
}

/// A page of 100×50 with a red bar of 40×8 centred on (50, 25), turned a
/// quarter turn clockwise: it stands from y 5 to 45 at x 46..54.
fn standing_bar() -> Document {
    let mut at = Placement::new(30.0, 21.0, 40.0, 8.0);
    at.rotation_deg = 90.0;
    let bar = Arc::new(Node::new(
        ElementBase::new(NodeId::new(2), at),
        NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(RED),
            stroke: None,
            corner_radius: 0.0,
        }),
    ));
    let mut page = Page::new(
        PageId::new(1),
        "P".to_owned(),
        PageSize {
            width: Length::new(100.0, Unit::Px),
            height: Length::new(50.0, Unit::Px),
        },
    );
    page.roots.push(bar.base.id);
    page.nodes.insert(bar.base.id, bar);
    Document::from_parts(3, Dpi::default(), vec![Arc::new(page)]).expect("the document validates")
}

// S41-20: the JPG and the WebP draw a turned element where the canvas
// shows it (spec 041).
#[test]
fn s41_20_jpeg_and_webp_draw_a_turned_element() {
    let red = [255, 0, 0, 255];
    let white = [255, 255, 255, 255];
    let mut sink = MemSink::default();
    export(
        &JpegExporter::new(),
        &standing_bar(),
        &request("jpeg", &[]),
        &mut sink,
    )
    .expect("the export runs");
    let (width, _, pixels) = decode(&sink.bytes(0));
    assert!(
        near(pixel(&pixels, width, 50, 8), red, 8),
        "the top of the bar"
    );
    let low = pixel(&pixels, width, 50, 36);
    assert!(near(low, red, 8), "{low:?}");
    assert!(
        near(pixel(&pixels, width, 34, 25), white, 4),
        "its unturned box"
    );
    let mut sink = MemSink::default();
    export(
        &WebpExporter::new(),
        &standing_bar(),
        &request("webp", &[]),
        &mut sink,
    )
    .expect("the export runs");
    let (width, _, pixels, _) = decode_webp(&sink.bytes(0));
    assert_eq!(pixel(&pixels, width, 50, 8), red);
    assert_eq!(pixel(&pixels, width, 34, 25)[3], 255, "opaque white page");
    assert_eq!(pixel(&pixels, width, 34, 25), white);
}
