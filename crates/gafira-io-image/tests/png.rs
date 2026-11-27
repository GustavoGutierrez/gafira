//! The PNG exporter over the real pipeline: the graph of `gafira-render` on
//! the CPU backend (spec 030, FD-55).

use std::io::{self, Cursor};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{CancelToken, NoProgress, ParamValue, ParamValues};
use gafira_doc::{
    AssetEntry, AssetRef, AssetStorage, AssetTable, ContentHash, Document, Dpi, ElementBase,
    ImageNode, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry, ShapeNode,
    SpanStyle, TextNode,
};
use gafira_geom::{Length, Unit};
use gafira_io_image::{MAX_SIDE, PngExporter, RasterDecoder, SCALE, TRANSPARENT, page_pixels};
use gafira_ports::common::UnixTime;
use gafira_ports::export::{
    ExportError, ExportJob, ExportReport, ExportRequest, ExportScope, ExportSink, Exporter,
    ImageDecoder, ImagePixels, PageRenderers,
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

/// A sink in memory: the files by name.
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

const RED: Color = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));

fn node(id: u64, placement: Placement, kind: NodeKind) -> Arc<Node> {
    Arc::new(Node::new(
        ElementBase::new(NodeId::new(id), placement),
        kind,
    ))
}

fn page(id: u64, width: f64, height: f64, nodes: Vec<Arc<Node>>) -> Arc<Page> {
    let mut page = Page::new(
        PageId::new(id),
        "P".to_owned(),
        PageSize {
            width: Length::new(width, Unit::Px),
            height: Length::new(height, Unit::Px),
        },
    );
    for node in nodes {
        page.roots.push(node.base.id);
        page.nodes.insert(node.base.id, node);
    }
    Arc::new(page)
}

/// A page of 100×50 with a red square at (10, 10), 20 px a side.
fn square_document() -> Document {
    let square = node(
        2,
        Placement::new(10.0, 10.0, 20.0, 20.0),
        NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(RED),
            stroke: None,
            corner_radius: 0.0,
            text: None,
            gradient: None,
            effect: None,
            drawn_with: None,
        }),
    );
    Document::from_parts(3, Dpi::default(), vec![page(1, 100.0, 50.0, vec![square])])
        .expect("the document validates")
}

fn renderers() -> GraphPageRenderers {
    GraphPageRenderers::new(
        Box::new(|| Box::new(CpuBackend::new())),
        Arc::new(FixedClock),
        Arc::new(FontBook::new().with("Inter", gafira_testkit::fonts::INTER.bytes)),
    )
}

fn request(pages: &[u64], scale: f64, transparent: bool) -> ExportRequest {
    let mut options = ParamValues::new();
    options.set(SCALE, ParamValue::Float(scale));
    options.set(TRANSPARENT, ParamValue::Bool(transparent));
    ExportRequest {
        format_id: "png".to_owned(),
        scope: ExportScope::Pages(pages.iter().map(|p| PageId::new(*p)).collect()),
        options,
    }
}

fn export(
    document: &Document,
    request: &ExportRequest,
    sink: &mut MemSink,
) -> Result<ExportReport, ExportError> {
    export_with(document, ImagePixels::new(), request, sink)
}

fn export_with(
    document: &Document,
    images: ImagePixels,
    request: &ExportRequest,
    sink: &mut MemSink,
) -> Result<ExportReport, ExportError> {
    let renderers = renderers();
    let pages = renderers.for_document(document, images);
    let job = ExportJob {
        request,
        document,
        store: &NoStore,
        pages: pages.as_ref(),
    };
    PngExporter::new().export(&job, sink, &NoProgress, &CancelToken::new())
}

/// The decoded PNG: width, height, RGBA bytes and whether it carries `sRGB`.
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>, bool) {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("a PNG");
    let mut pixels = vec![0; reader.output_buffer_size().expect("a size")];
    let frame = reader.next_frame(&mut pixels).expect("one frame");
    let srgb = reader.info().srgb.is_some();
    pixels.truncate(frame.buffer_size());
    (frame.width, frame.height, pixels, srgb)
}

fn pixel(pixels: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
}

// S30-03: at 2× the PNG is twice the page, the square is red where it is,
// the page is white around it and the file says it is sRGB.
#[test]
fn s30_03_a_page_exports_as_a_png_at_twice_its_size() {
    let mut sink = MemSink::default();
    let report =
        export(&square_document(), &request(&[1], 2.0, false), &mut sink).expect("the export runs");
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].name, "page.png");
    let bytes = sink.bytes(0);
    assert_eq!(report.files[0].bytes, bytes.len() as u64);
    let (width, height, pixels, srgb) = decode(&bytes);
    assert_eq!((width, height), (200, 100));
    assert!(srgb, "the sRGB chunk is there");
    // The square covers (20, 20)..(60, 60) at 2×.
    assert_eq!(pixel(&pixels, width, 40, 40), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, width, 21, 21), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, width, 18, 18), [255, 255, 255, 255]);
    assert_eq!(pixel(&pixels, width, 62, 40), [255, 255, 255, 255]);
    assert_eq!(pixel(&pixels, width, 199, 99), [255, 255, 255, 255]);
}

// S30-03, FD-55 (b): with a transparent background the page around the
// content has alpha 0.
#[test]
fn s30_03_a_transparent_png_is_clear_where_there_is_no_content() {
    let mut sink = MemSink::default();
    export(&square_document(), &request(&[1], 1.0, true), &mut sink).expect("the export runs");
    let (width, height, pixels, _) = decode(&sink.bytes(0));
    assert_eq!((width, height), (100, 50));
    assert_eq!(pixel(&pixels, width, 20, 20), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, width, 5, 5)[3], 0);
    assert_eq!(pixel(&pixels, width, 90, 40)[3], 0);
}

// S30-01: a text paints dark pixels inside its box and none outside it.
#[test]
fn s30_01_a_text_paints_inside_its_box_and_nowhere_else() {
    let style = SpanStyle {
        font: "Inter".to_owned(),
        size: 24.0,
        color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
        bold: false,
        italic: false,
        gradient: None,
    };
    let text = node(
        2,
        Placement::new(20.0, 10.0, 160.0, 40.0),
        NodeKind::Text(TextNode::plain("Hola", &style)),
    );
    let document = Document::from_parts(3, Dpi::default(), vec![page(1, 200.0, 60.0, vec![text])])
        .expect("the document validates");
    let mut sink = MemSink::default();
    export(&document, &request(&[1], 1.0, false), &mut sink).expect("the export runs");
    let (width, height, pixels, _) = decode(&sink.bytes(0));
    let mut inside = 0;
    for y in 0..height {
        for x in 0..width {
            let dark = pixel(&pixels, width, x, y)[0] < 128;
            let in_box = (20..180).contains(&x) && (10..50).contains(&y);
            if dark {
                assert!(in_box, "a dark pixel at ({x}, {y}) is outside the box");
                inside += 1;
            }
        }
    }
    assert!(inside > 50, "the glyphs paint: {inside} dark pixels");
}

// S30-03: two pages give two files, in the order of the scope.
#[test]
fn s30_03_each_page_of_the_scope_is_one_file() {
    let document = Document::from_parts(
        3,
        Dpi::default(),
        vec![page(1, 10.0, 10.0, vec![]), page(2, 30.0, 20.0, vec![])],
    )
    .expect("the document validates");
    let mut sink = MemSink::default();
    let report = export(&document, &request(&[2, 1], 1.0, false), &mut sink).expect("the export");
    let names: Vec<&str> = report.files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["page-1.png", "page-2.png"]);
    assert_eq!(decode(&sink.bytes(0)).0, 30, "the first file is page 2");
    assert_eq!(decode(&sink.bytes(1)).0, 10);
}

// S30-03, D-71: a side above 16 384 px is refused before any pixel.
#[test]
fn s30_03_a_side_above_the_limit_is_an_error() {
    let wide = page(1, 9000.0, 10.0, vec![]);
    assert_eq!(page_pixels(&wide, Dpi::default(), 1.0).unwrap(), (9000, 10));
    assert!(page_pixels(&wide, Dpi::default(), 2.0).is_err());
    let edge = page(1, f64::from(MAX_SIDE), 1.0, vec![]);
    assert_eq!(
        page_pixels(&edge, Dpi::default(), 1.0).unwrap(),
        (MAX_SIDE, 1)
    );
    assert!(page_pixels(&edge, Dpi::default(), 1.0001).is_err());
    let tiny = page(1, 1.0, 1.0, vec![]);
    assert_eq!(page_pixels(&tiny, Dpi::default(), 0.5).unwrap(), (1, 1));
    let document =
        Document::from_parts(2, Dpi::default(), vec![wide]).expect("the document validates");
    let mut sink = MemSink::default();
    assert!(export(&document, &request(&[1], 2.0, false), &mut sink).is_err());
    assert!(sink.files.is_empty(), "nothing was written");
}

// FD-58: a scale out of range, no page or a selection block the export.
#[test]
fn s30_03_validate_names_what_blocks_the_export() {
    let exporter = PngExporter::new();
    assert!(exporter.validate(&request(&[1], 2.0, false)).is_empty());
    for bad in [
        request(&[1], 8.0, false),
        request(&[], 1.0, false),
        ExportRequest {
            format_id: "png".to_owned(),
            scope: ExportScope::Nodes {
                page: PageId::new(1),
                nodes: vec![NodeId::new(2)],
            },
            options: ParamValues::new(),
        },
    ] {
        let issues = exporter.validate(&bad);
        assert!(issues.iter().any(|issue| issue.blocks()), "{bad:?}");
    }
}

// A page the document does not hold is an error, not an empty file.
#[test]
fn s30_03_a_page_outside_the_document_is_an_error() {
    let mut sink = MemSink::default();
    assert!(export(&square_document(), &request(&[9], 1.0, false), &mut sink).is_err());
    assert!(sink.files.is_empty());
}

/// The asset of the image tests.
const PHOTO: AssetRef = AssetRef::new(ContentHash([7; 32]));

/// A page of 100×50 with an image of [`PHOTO`] in the box (10, 10, 40, 20)
/// at `opacity`.
fn image_document(opacity: f32) -> Document {
    let mut base = ElementBase::new(NodeId::new(2), Placement::new(10.0, 10.0, 40.0, 20.0));
    base.opacity = opacity;
    let image = Arc::new(Node::new(base, NodeKind::Image(ImageNode::new(PHOTO))));
    let mut assets = AssetTable::new();
    assets.insert(
        PHOTO,
        AssetEntry {
            name: "photo.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    Document::from_parts_with_assets(
        3,
        Dpi::default(),
        vec![page(1, 100.0, 50.0, vec![image])],
        assets,
    )
    .expect("the document validates")
}

/// The pixels of [`PHOTO`]: a PNG of 2×1, red then blue, through the decoder
/// the app composes.
fn photo() -> ImagePixels {
    let mut source = image::RgbaImage::new(2, 1);
    source.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
    source.put_pixel(1, 0, image::Rgba([0, 0, 255, 255]));
    let mut bytes = Cursor::new(Vec::new());
    source
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("the PNG encodes");
    let pixels = RasterDecoder::new()
        .decode(bytes.get_ref())
        .expect("the PNG decodes");
    ImagePixels::from([(PHOTO, Arc::new(pixels))])
}

// S32-05: the image is stretched to its box: red in its left half, blue in
// its right half, white around it.
#[test]
fn s32_05_a_png_has_the_image_pixels_in_its_box() {
    let mut sink = MemSink::default();
    export_with(
        &image_document(1.0),
        photo(),
        &request(&[1], 1.0, false),
        &mut sink,
    )
    .expect("the export runs");
    let (width, height, pixels, _) = decode(&sink.bytes(0));
    assert_eq!((width, height), (100, 50));
    // Bilinear sampling (ARQ-121) mixes the two texels only around the middle
    // of the box: each half keeps its colour up to 10 px from its edge.
    assert_eq!(pixel(&pixels, width, 12, 12), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, width, 18, 25), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, width, 42, 25), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, width, 48, 28), [0, 0, 255, 255]);
    for (x, y) in [(8, 20), (52, 20), (30, 8), (30, 31), (90, 45)] {
        assert_eq!(
            pixel(&pixels, width, x, y),
            [255, 255, 255, 255],
            "({x}, {y})"
        );
    }
}

// S32-05: at 2× the image follows its box, and at half opacity it mixes with
// the white page in linear light (linear 0.5 is sRGB 188).
#[test]
fn s32_05_the_image_takes_the_scale_and_the_opacity_of_its_element() {
    let mut sink = MemSink::default();
    export_with(
        &image_document(0.5),
        photo(),
        &request(&[1], 2.0, false),
        &mut sink,
    )
    .expect("the export runs");
    let (width, height, pixels, _) = decode(&sink.bytes(0));
    assert_eq!((width, height), (200, 100));
    let near = |got: [u8; 4], want: [u8; 4]| {
        got.iter()
            .zip(want)
            .all(|(g, w)| (i16::from(*g) - i16::from(w)).abs() <= 1)
    };
    let red = pixel(&pixels, width, 30, 40);
    assert!(near(red, [255, 188, 188, 255]), "{red:?}");
    let blue = pixel(&pixels, width, 90, 40);
    assert!(near(blue, [188, 188, 255, 255]), "{blue:?}");
    assert_eq!(pixel(&pixels, width, 110, 40), [255, 255, 255, 255]);
}

/// The PNG of [`image_document`] with its image turned by `rotation` and
/// mirrored left to right when `flip`.
fn turned_png(rotation: f64, flip: bool) -> (u32, Vec<u8>) {
    let mut document = image_document(1.0);
    let mut page = document.page(PageId::new(1)).expect("the page").clone();
    let mut node = (**page.nodes.get(&NodeId::new(2)).expect("the image")).clone();
    node.base.placement.rotation_deg = rotation;
    node.base.placement.flip.x = flip;
    page.nodes.insert(NodeId::new(2), Arc::new(node));
    document = Document::from_parts_with_assets(
        3,
        Dpi::default(),
        vec![Arc::new(page)],
        document.assets().clone(),
    )
    .expect("the document validates");
    let mut sink = MemSink::default();
    export_with(&document, photo(), &request(&[1], 1.0, false), &mut sink)
        .expect("the export runs");
    let (width, _, pixels, _) = decode(&sink.bytes(0));
    (width, pixels)
}

// S41-20: the export draws an element turned about its centre and mirrored
// in its box, the transform the canvas shows (spec 041).
#[test]
fn s41_20_a_png_draws_the_image_turned_and_flipped() {
    const RED: [u8; 4] = [255, 0, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const WHITE: [u8; 4] = [255, 255, 255, 255];
    let (width, flipped) = turned_png(0.0, true);
    assert_eq!(
        pixel(&flipped, width, 12, 12),
        BLUE,
        "mirrored: blue on the left"
    );
    assert_eq!(pixel(&flipped, width, 48, 28), RED);
    // A quarter turn clockwise about (30, 20): the box stands from y 0 to
    // 40 at x 20..40, its left half (red) on top.
    let (width, turned) = turned_png(90.0, false);
    assert_eq!(pixel(&turned, width, 30, 4), RED, "the left half on top");
    assert_eq!(pixel(&turned, width, 30, 36), BLUE);
    assert_eq!(
        pixel(&turned, width, 12, 20),
        WHITE,
        "the unturned box is empty"
    );
    assert_eq!(pixel(&turned, width, 30, 45), WHITE);
    // Turned and flipped, the right half (blue) is on top.
    let (width, both) = turned_png(90.0, true);
    assert_eq!(pixel(&both, width, 30, 4), BLUE);
    assert_eq!(pixel(&both, width, 30, 36), RED);
}
