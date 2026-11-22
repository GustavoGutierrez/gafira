//! S80-12: a QR code through the whole export path (graph, planner, CPU
//! backend), read back by a QR reader.

use std::sync::Arc;
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::CancelToken;
use gafira_doc::timeline::{AssetEntry, AssetRef, AssetStorage, AssetTable};
use gafira_doc::{
    ContentHash, Document, Dpi, ElementBase, Gradient, GradientKind, GradientStop, Node, NodeId,
    NodeKind, Page, PageId, PageSize, Placement, QrBody, QrContent, QrEyeFrame, QrFill, QrLogo,
    QrNode,
};
use gafira_geom::{IRect, Length, Unit};
use gafira_ports::common::{Bytes, UnixTime};
use gafira_ports::export::{ImagePixels, PageRenderRequest, PageRenderer};
use gafira_ports::render::RenderQuality;
use gafira_ports::system::Clock;
use gafira_raster::{ImageBuf, PixelBuf};
use gafira_render::{BackendSelector, ContentTileCache, GraphPageRenderer};
use gafira_render_cpu::CpuBackend;

const SIDE: u32 = 400;
const LOGO: AssetRef = AssetRef::new(ContentHash([9; 32]));
const TEXT: &str = "https://example.com/gafira";

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::Srgb(Rgba::new(r, g, b, 1.0))
}

fn code() -> QrNode {
    let url = TEXT.to_owned();
    QrNode::new(
        QrContent::Url { url },
        rgb(0.0, 0.0, 0.0),
        rgb(1.0, 1.0, 1.0),
    )
}

fn render(qr: QrNode, placement: Placement) -> Vec<u8> {
    let size = PageSize {
        width: Length::new(f64::from(SIDE), Unit::Px),
        height: Length::new(f64::from(SIDE), Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), "S80".to_string(), size);
    let node = Node::new(
        ElementBase::new(NodeId::new(2), placement),
        NodeKind::Qr(Box::new(qr)),
    );
    page.roots.push(node.base.id);
    page.nodes.insert(node.base.id, Arc::new(node));
    let mut assets = AssetTable::new();
    assets.insert(
        LOGO,
        AssetEntry {
            name: "red.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    let document =
        Document::from_parts_with_assets(3, Dpi::default(), vec![Arc::new(page)], assets)
            .expect("valid");
    let red = [255_u8, 0, 0, 255].repeat(16);
    let image = ImageBuf::new(4, 4, PixelBuf::Rgba8(red)).expect("image");
    let images = ImagePixels::from([(LOGO, Arc::new(image))]);
    let renderer = GraphPageRenderer::new(
        document,
        RenderQuality::Preview,
        BackendSelector::new(None, Box::new(CpuBackend::new())),
        ContentTileCache::new(Bytes(4 * 1024 * 1024)),
        Box::new(FixedClock),
    )
    .with_images(images);
    let request = PageRenderRequest {
        page: PageId::new(1),
        time: Default::default(),
        scale: 1.0,
        region: IRect::new(0, 0, SIDE as i32, SIDE as i32),
        transparent: true,
    };
    let image = renderer
        .render(&request, &CancelToken::new())
        .expect("renders");
    match image.data() {
        PixelBuf::Rgba8(bytes) => bytes.clone(),
        other => panic!("the preview answers Rgba8, not {other:?}"),
    }
}

fn at(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * SIDE + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// The text of `qr` drawn in `placement` over a white page.
fn read(pixels: &[u8], qr: &QrNode, placement: Placement) -> Option<String> {
    let drawing = gafira_qr::draw(qr).expect("drawn");
    let (k, (x0, y0)) = drawing.fit(placement.w, placement.h).expect("a box");
    let to_page = placement.to_transform();
    gafira_testkit::qr::read(drawing.size as usize, |x, y| {
        let p = to_page * kurbo::Point::new(x0 + x * k, y0 + y * k);
        let clamp = |v: f64| (v.max(0.0) as u32).min(SIDE - 1);
        let p = at(pixels, clamp(p.x), clamp(p.y));
        let grey = (u32::from(p[0]) * 3 + u32::from(p[1]) * 6 + u32::from(p[2])) / 10;
        (grey + 255 - u32::from(p[3])).min(255) as u8
    })
}

fn whole() -> Placement {
    Placement::new(0.0, 0.0, f64::from(SIDE), f64::from(SIDE))
}

// S80-12: a plain code, and one turned and in a long box, are read back.
#[test]
fn s80_12_the_export_reads_back() {
    assert_eq!(
        read(&render(code(), whole()), &code(), whole()).as_deref(),
        Some(TEXT)
    );
    let mut turned = Placement::new(20.0, 70.0, 360.0, 260.0);
    turned.rotation_deg = 90.0;
    assert_eq!(
        read(&render(code(), turned), &code(), turned).as_deref(),
        Some(TEXT)
    );
}

// S80-12: shapes, a gradient and a logo are read back; the logo shows.
#[test]
fn s80_12_a_styled_code_reads_back() {
    let mut qr = code();
    qr.style.body = QrBody::FluidCorners;
    qr.style.eye_frame = QrEyeFrame::Drop;
    qr.style.fill = QrFill::Gradient(Gradient {
        kind: GradientKind::Linear { angle_deg: 90.0 },
        stops: [(0.0, rgb(0.0, 0.0, 0.0)), (1.0, rgb(0.0, 0.2, 0.6))]
            .map(|(offset, color)| GradientStop { offset, color })
            .to_vec(),
    });
    qr.logo = Some(QrLogo {
        asset: LOGO,
        natural: (4, 4),
        share: 0.2,
        clear_behind: true,
    });
    let pixels = render(qr.clone(), whole());
    assert_eq!(
        at(&pixels, SIDE / 2, SIDE / 2),
        [255, 0, 0, 255],
        "the logo"
    );
    assert_eq!(read(&pixels, &qr, whole()).as_deref(), Some(TEXT));
}

// S80-07, S80-12: without a background only the code is drawn; a code half
// written draws nothing.
#[test]
fn s80_12_transparent_and_unfinished() {
    let mut qr = code();
    qr.style.background = None;
    let pixels = render(qr.clone(), whole());
    assert_eq!(at(&pixels, 2, 2)[3], 0, "the margin");
    assert_eq!(read(&pixels, &qr, whole()).as_deref(), Some(TEXT));
    qr.content = QrContent::Url { url: String::new() };
    assert!(render(qr, whole()).iter().all(|b| *b == 0));
}
