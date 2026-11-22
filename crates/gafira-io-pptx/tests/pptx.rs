//! Spec 075: the PowerPoint exporter writes an editable presentation.

use std::io::{self, Cursor, Read};
use std::sync::{Arc, Mutex};

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{CancelToken, NoProgress, ParamValues};
use gafira_doc::timeline::{AssetEntry, AssetRef, AssetStorage, AssetTable};
use gafira_doc::{
    ContentHash, Document, Dpi, ElementBase, GroupNode, ImageNode, LineCap, LineEnd, LineNode,
    Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry, ShapeNode, TextNode,
};
use gafira_geom::{Length, Unit};
use gafira_io_pptx::{FILE_NAME, PptxExporter};
use gafira_ports::export::{
    ExportError, ExportJob, ExportRequest, ExportScope, ExportSink, Exporter, PageRenderRequest,
    PageRenderer,
};
use gafira_ports::render::RenderError;
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{ReadSeek, WriteSeek};
use gafira_raster::ImageBuf;

const PHOTO: AssetRef = AssetRef::new(ContentHash([7; 32]));

/// A store holding one small PNG under [PHOTO].
struct OnePhoto(Vec<u8>);

impl ContentStore for OnePhoto {
    fn put(&self, _src: &mut dyn io::Read) -> Result<ContentHash, StoreError> {
        Err(StoreError::Io(io::Error::other("unused")))
    }
    fn open(&self, hash: ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        if hash == PHOTO.as_hash() {
            Ok(Box::new(Cursor::new(self.0.clone())))
        } else {
            Err(StoreError::NotFound { hash })
        }
    }
    fn contains(&self, hash: ContentHash) -> Result<bool, StoreError> {
        Ok(hash == PHOTO.as_hash())
    }
}

/// The PPTX exporter draws no pixels.
struct NoPixels;

impl PageRenderer for NoPixels {
    fn render(&self, _r: &PageRenderRequest, _c: &CancelToken) -> Result<ImageBuf, RenderError> {
        Err(RenderError::DeviceLost)
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

fn node(id: u64, placement: Placement, kind: NodeKind) -> Node {
    Node::new(ElementBase::new(NodeId::new(id), placement), kind)
}

/// A page with a circle, a text, a photo and a line, the circle and text in
/// a group.
fn document() -> Document {
    let mut page = Page::new(
        PageId::new(1),
        "P".to_owned(),
        PageSize {
            width: Length::new(1280.0, Unit::Px),
            height: Length::new(720.0, Unit::Px),
        },
    );
    let mut circle = ShapeNode::new(ShapeGeometry::Ellipse);
    circle.fill = Some(Color::Srgb(Rgba::new(0.25, 0.75, 0.66, 1.0)));
    let style = gafira_doc::SpanStyle {
        font: "Inter".to_owned(),
        size: 48.0,
        color: Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 1.0)),
        bold: true,
        italic: false,
    };
    let nodes = [
        node(
            2,
            Placement::new(100.0, 100.0, 200.0, 200.0),
            NodeKind::Shape(circle),
        ),
        node(
            3,
            Placement::new(100.0, 320.0, 600.0, 80.0),
            NodeKind::Text(TextNode::plain("Hola & mundo", &style)),
        ),
        node(
            4,
            Placement::new(800.0, 100.0, 300.0, 200.0),
            NodeKind::Image(ImageNode::new(PHOTO)),
        ),
        node(
            5,
            Placement::new(100.0, 600.0, 1000.0, 50.0),
            NodeKind::Line(LineNode {
                start: LineEnd::new([0.0, 0.5], LineCap::None),
                end: LineEnd::new([1.0, 0.5], LineCap::Arrow),
                color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
                width: 4.0,
            }),
        ),
    ];
    for n in nodes {
        page.nodes.insert(n.base.id, Arc::new(n));
    }
    let mut group = node(
        6,
        Placement::new(100.0, 100.0, 600.0, 300.0),
        NodeKind::Group(GroupNode {}),
    );
    group.children = vec![NodeId::new(2), NodeId::new(3)];
    page.nodes.insert(group.base.id, Arc::new(group));
    page.roots = vec![NodeId::new(6), NodeId::new(4), NodeId::new(5)];
    let mut assets = AssetTable::new();
    assets.insert(
        PHOTO,
        AssetEntry {
            name: "foto.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    Document::from_parts_with_assets(7, Dpi::default(), vec![Arc::new(page)], assets)
        .expect("valid")
}

fn png() -> Vec<u8> {
    let mut out = Vec::new();
    image::RgbaImage::from_pixel(8, 6, image::Rgba([200, 30, 30, 255]))
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .expect("png");
    out
}

fn pptx() -> Vec<u8> {
    pptx_of(&document())
}

fn pptx_of(document: &Document) -> Vec<u8> {
    let request = ExportRequest {
        format_id: "pptx".to_owned(),
        scope: ExportScope::Pages(vec![PageId::new(1)]),
        options: ParamValues::new(),
    };
    let store = OnePhoto(png());
    let job = ExportJob {
        request: &request,
        document,
        store: &store,
        pages: &NoPixels,
    };
    let mut sink = MemSink::default();
    let report: Result<_, ExportError> =
        PptxExporter::new().export(&job, &mut sink, &NoProgress, &CancelToken::new());
    let report = report.expect("the export");
    assert_eq!(report.files[0].name, FILE_NAME);
    sink.files[0].1.lock().unwrap().get_ref().clone()
}

// S75-01: the package holds the parts of a presentation, every XML part
// reads, and the slide has the text as text in its font, the shapes, the
// group, the photo and the line.
#[test]
fn s75_01_an_editable_presentation() {
    let bytes = pptx();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).expect("a zip");
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).expect("entry").name().to_owned())
        .collect();
    for part in [
        "[Content_Types].xml",
        "_rels/.rels",
        "ppt/presentation.xml",
        "ppt/slides/slide1.xml",
        "ppt/slideMasters/slideMaster1.xml",
        "ppt/slideLayouts/slideLayout1.xml",
        "ppt/theme/theme1.xml",
        "ppt/media/image1.png",
    ] {
        assert!(names.iter().any(|n| n == part), "{part}");
    }
    let mut slide = String::new();
    for name in &names {
        if !name.ends_with(".xml") && !name.ends_with(".rels") {
            continue;
        }
        let mut text = String::new();
        zip.by_name(name)
            .expect("part")
            .read_to_string(&mut text)
            .expect("utf-8");
        roxmltree::Document::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        if name == "ppt/slides/slide1.xml" {
            slide = text;
        }
    }
    assert!(
        slide.contains("<a:t>Hola &amp; mundo</a:t>"),
        "the words as text"
    );
    assert!(slide.contains("typeface=\"Inter\""), "in their font");
    assert!(slide.contains("b=\"1\""), "bold");
    assert!(slide.contains("prst=\"ellipse\""), "the circle");
    assert!(slide.contains("<p:grpSp>"), "the group");
    assert!(slide.contains("<p:pic>"), "the photo");
    assert!(
        slide.contains("<p:cxnSp>") && slide.contains("tailEnd type=\"triangle\""),
        "the arrow"
    );
}

// S80-12: a QR code goes as its shapes, all in one square, and its logo.
#[test]
fn s80_12_a_qr_code_goes_as_shapes() {
    use gafira_doc::{QrContent, QrLogo, QrNode};
    let grey = |v: f32| Color::Srgb(Rgba::new(v, v, v, 1.0));
    let url = "https://example.com".to_owned();
    let mut qr = QrNode::new(QrContent::Url { url }, grey(0.0), grey(1.0));
    qr.logo = Some(QrLogo {
        asset: PHOTO,
        natural: (8, 6),
        share: 0.2,
        clear_behind: true,
    });
    let base = document();
    let mut page = (*base.pages()[0]).clone();
    let code = node(
        20,
        Placement::new(100.0, 100.0, 300.0, 300.0),
        NodeKind::Qr(Box::new(qr)),
    );
    page.nodes.clear();
    page.roots = vec![code.base.id];
    page.nodes.insert(code.base.id, Arc::new(code));
    let mut assets = AssetTable::new();
    assets.insert(
        PHOTO,
        AssetEntry {
            name: "foto.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    let document =
        Document::from_parts_with_assets(21, Dpi::default(), vec![Arc::new(page)], assets)
            .expect("valid");
    let mut zip = zip::ZipArchive::new(Cursor::new(pptx_of(&document))).expect("a zip");
    let mut slide = String::new();
    zip.by_name("ppt/slides/slide1.xml")
        .expect("the slide")
        .read_to_string(&mut slide)
        .expect("utf-8");
    roxmltree::Document::parse(&slide).expect("XML");
    assert!(slide.contains("prst=\"rect\""), "the background");
    assert_eq!(slide.matches("<a:custGeom>").count(), 3, "body and eyes");
    assert_eq!(slide.matches("<p:pic>").count(), 1, "the logo");
}

// S81-13: a table goes as a PowerPoint table.
#[test]
fn s81_13_a_table_goes_as_a_table() {
    use gafira_doc::{CellMerge, CellStyle, ShapeStroke, TableNode};
    let ink = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
    let mut table = TableNode::new(2, 3, (100.0, 40.0), &CellStyle::new("Inter", 16.0, ink));
    table.cells[0].text = "Uno & dos".to_owned();
    table.cells[0].style.borders.right = Some(ShapeStroke::new(ink, 2.0));
    table.merges.push(CellMerge {
        row: 1,
        col: 0,
        rows: 1,
        cols: 2,
    });
    table.header = true;
    let base = document();
    let mut page = (*base.pages()[0]).clone();
    let code = node(
        20,
        Placement::new(100.0, 100.0, 300.0, 80.0),
        NodeKind::Table(Box::new(table)),
    );
    page.nodes.clear();
    page.roots = vec![code.base.id];
    page.nodes.insert(code.base.id, Arc::new(code));
    let document = Document::from_parts(21, Dpi::default(), vec![Arc::new(page)]).expect("valid");
    let mut zip = zip::ZipArchive::new(Cursor::new(pptx_of(&document))).expect("a zip");
    let mut slide = String::new();
    zip.by_name("ppt/slides/slide1.xml")
        .expect("the slide")
        .read_to_string(&mut slide)
        .expect("utf-8");
    roxmltree::Document::parse(&slide).expect("XML");
    for piece in [
        "<a:tbl><a:tblPr firstRow=\"1\"/>",
        "gridSpan=\"2\"",
        "hMerge=\"1\"",
        "<a:t>Uno &amp; dos</a:t>",
        "<a:txBody>",
    ] {
        assert!(slide.contains(piece), "{piece}");
    }
    assert_eq!(slide.matches("<a:gridCol ").count(), 3);
    assert_eq!(slide.matches("<a:tr ").count(), 2);
    assert_eq!(
        slide.matches("<a:tc").count() - slide.matches("<a:tcPr").count(),
        6
    );
    assert!(!slide.contains("<p:txBody>"), "the text is of the table");
}

// S83-09: a chart goes as shapes and texts that can be edited.
#[test]
fn s83_09_a_chart_goes_as_shapes_and_texts() {
    use gafira_doc::{ChartKind, ChartNode, ChartText};
    let ink = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
    let cells = ["Equipo", "Total", "Ventas & más", "75", "Otros", "25"];
    let text = ChartText {
        font: "Inter".to_owned(),
        size: 14.0,
        color: ink,
        bold: false,
        italic: false,
    };
    let data = (2, cells.map(str::to_owned).to_vec());
    let mut chart = ChartNode::new(ChartKind::Pie, data, vec![ink], text);
    chart.names = true;
    let base = document();
    let mut page = (*base.pages()[0]).clone();
    let code = node(
        20,
        Placement::new(100.0, 100.0, 400.0, 300.0),
        NodeKind::Chart(Box::new(chart)),
    );
    page.nodes.clear();
    page.roots = vec![code.base.id];
    page.nodes.insert(code.base.id, Arc::new(code));
    let document = Document::from_parts(21, Dpi::default(), vec![Arc::new(page)]).expect("valid");
    let mut zip = zip::ZipArchive::new(Cursor::new(pptx_of(&document))).expect("a zip");
    let mut slide = String::new();
    zip.by_name("ppt/slides/slide1.xml")
        .expect("the slide")
        .read_to_string(&mut slide)
        .expect("utf-8");
    roxmltree::Document::parse(&slide).expect("XML");
    assert_eq!(slide.matches("<a:custGeom>").count(), 2, "two slices");
    assert!(slide.contains("<a:t>Ventas &amp; más</a:t>"), "{slide}");
    assert!(!slide.contains("<p:pic>"), "nothing is pixels");
}

// S85-08: an image with a look goes as a picture of it, its box grown by
// what its shadow takes.
#[test]
fn s85_08_an_image_goes_with_its_look() {
    let mut shadow = gafira_doc::ImageShadow::new(gafira_doc::ShadowKind::Outline);
    shadow.size = 0.16;
    let look = gafira_doc::ImageLook {
        adjust: gafira_doc::ImageAdjust {
            saturation: -100.0,
            ..gafira_doc::ImageAdjust::default()
        },
        shadow: Some(shadow),
        ..gafira_doc::ImageLook::default()
    };
    let plain = document();
    let mut page = (*plain.pages()[0]).clone();
    let mut photo = (*page.nodes[&NodeId::new(4)]).clone();
    photo.kind = NodeKind::Image(ImageNode::new(PHOTO).looking(look));
    page.nodes.insert(NodeId::new(4), Arc::new(photo));
    let mut assets = AssetTable::new();
    assets.insert(
        PHOTO,
        AssetEntry {
            name: "foto.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    let document =
        Document::from_parts_with_assets(7, Dpi::default(), vec![Arc::new(page)], assets)
            .expect("valid");
    let mut zip = zip::ZipArchive::new(Cursor::new(pptx_of(&document))).expect("a zip");
    let mut png = Vec::new();
    zip.by_name("ppt/media/image1.png")
        .expect("the picture")
        .read_to_end(&mut png)
        .expect("read");
    let picture = image::load_from_memory(&png).expect("png").into_rgba8();
    // 8 by 6 and a margin of one pixel.
    assert_eq!(picture.dimensions(), (10, 8));
    let p = picture.get_pixel(5, 4).0;
    assert!(p[0] == p[1] && p[1] == p[2] && p[3] == 255, "{p:?}");
    let mut slide = String::new();
    zip.by_name("ppt/slides/slide1.xml")
        .expect("the slide")
        .read_to_string(&mut slide)
        .expect("utf-8");
    let slide = roxmltree::Document::parse(&slide).expect("xml");
    let picture = slide
        .descendants()
        .find(|n| n.has_tag_name("pic") || n.tag_name().name() == "pic")
        .expect("the picture");
    let number = |tag: &str, key: &str| -> f64 {
        picture
            .descendants()
            .find(|n| n.tag_name().name() == tag)
            .and_then(|n| n.attribute(key))
            .and_then(|v| v.parse().ok())
            .expect("a number")
    };
    // The box is 300 by 200: a pixel of margin is 37.5 by 33.3.
    let (cx, cy) = (number("ext", "cx"), number("ext", "cy"));
    assert!(
        (cx / cy - 375.0 / (200.0 * 8.0 / 6.0)).abs() < 1e-3,
        "{cx} {cy}"
    );
    assert!((number("off", "x") / cx - 762.5 / 375.0).abs() < 1e-3);
}

// S89-03: the effects PowerPoint has go with the words, which stay words.
#[test]
fn s89_03_a_text_goes_with_its_effects() {
    use gafira_doc::{TextEffect, TextEffectKind};
    let slide = |kinds: &[TextEffectKind]| {
        let plain = document();
        let mut page = (*plain.pages()[0]).clone();
        let mut node = (*page.nodes[&NodeId::new(3)]).clone();
        let NodeKind::Text(text) = &mut node.kind else {
            panic!("a text")
        };
        text.effects = kinds.iter().map(|kind| TextEffect::new(*kind)).collect();
        page.nodes.insert(NodeId::new(3), Arc::new(node));
        let mut assets = AssetTable::new();
        assets.insert(
            PHOTO,
            AssetEntry {
                name: "foto.png".to_owned(),
                storage: AssetStorage::Embedded,
            },
        );
        let pages = vec![Arc::new(page)];
        let document =
            Document::from_parts_with_assets(7, Dpi::default(), pages, assets).expect("valid");
        let mut zip = zip::ZipArchive::new(Cursor::new(pptx_of(&document))).expect("a zip");
        let mut slide = String::new();
        zip.by_name("ppt/slides/slide1.xml")
            .expect("the slide")
            .read_to_string(&mut slide)
            .expect("utf-8");
        roxmltree::Document::parse(&slide).expect("xml");
        assert!(slide.contains("<a:t>Hola &amp; mundo</a:t>"));
        slide
    };
    use TextEffectKind as K;
    assert!(slide(&[K::Drop]).contains("<a:outerShdw"));
    assert!(slide(&[K::Neon]).contains("<a:glow"));
    assert!(slide(&[K::Background]).contains("<a:highlight>"));
    let hollow = slide(&[K::Hollow]);
    assert!(hollow.contains("<a:noFill/><a:latin") && hollow.contains("<a:ln w="));
    let both = slide(&[K::Outline, K::Glow, K::Drop]);
    let at = |what: &str| both.find(what).expect(what);
    assert!(at("<a:glow") < at("<a:outerShdw") && at("<a:ln w=") < at("<a:effectLst>"));
    assert!(!slide(&[]).contains("effectLst"));
}
