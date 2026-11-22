//! S66-04, S66-05: a frame through the whole export path (graph, planner,
//! CPU backend): its photo shows inside its shape and nothing outside it.

use std::sync::Arc;
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::CancelToken;
use gafira_doc::timeline::{AssetEntry, AssetRef, AssetStorage, AssetTable};
use gafira_doc::{
    ContentHash, Document, Dpi, ElementBase, FrameContent, FrameNode, Node, NodeId, NodeKind, Page,
    PageId, PageSize, Placement, ShapeGeometry, ShapeStroke,
};
use gafira_geom::{IRect, Length, Unit};
use gafira_ports::common::{Bytes, UnixTime};
use gafira_ports::export::{ImagePixels, PageRenderRequest, PageRenderer};
use gafira_ports::render::RenderQuality;
use gafira_ports::system::Clock;
use gafira_raster::{ImageBuf, PixelBuf};
use gafira_render::{BackendSelector, ContentTileCache, GraphPageRenderer};
use gafira_render_cpu::CpuBackend;

const SIDE: u32 = 100;
const PHOTO: AssetRef = AssetRef::new(ContentHash([9; 32]));

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

/// A page with one round frame over the whole page, holding `content`.
fn document(content: Option<FrameContent>, stroke: Option<ShapeStroke>) -> Document {
    let mut page = Page::new(
        PageId::new(1),
        "S66".to_string(),
        PageSize {
            width: Length::new(f64::from(SIDE), Unit::Px),
            height: Length::new(f64::from(SIDE), Unit::Px),
        },
    );
    let mut frame = FrameNode::new(ShapeGeometry::Ellipse);
    frame.content = content;
    frame.stroke = stroke;
    let node = Node::new(
        ElementBase::new(
            NodeId::new(2),
            Placement::new(0.0, 0.0, f64::from(SIDE), f64::from(SIDE)),
        ),
        NodeKind::Frame(frame),
    );
    page.roots.push(node.base.id);
    page.nodes.insert(node.base.id, Arc::new(node));
    let mut assets = AssetTable::new();
    assets.insert(
        PHOTO,
        AssetEntry {
            name: "red.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    Document::from_parts_with_assets(3, Dpi::default(), vec![Arc::new(page)], assets)
        .expect("valid")
}

/// A 4 × 4 opaque red photo.
fn red() -> ImagePixels {
    let pixels = [255_u8, 0, 0, 255].repeat(16);
    let image = ImageBuf::new(4, 4, PixelBuf::Rgba8(pixels)).expect("image");
    ImagePixels::from([(PHOTO, Arc::new(image))])
}

/// The page rendered over a transparent background, as 8-bit premultiplied.
fn render(document: Document) -> Vec<u8> {
    let renderer = GraphPageRenderer::new(
        document,
        RenderQuality::Preview,
        BackendSelector::new(None, Box::new(CpuBackend::new())),
        ContentTileCache::new(Bytes(4 * 1024 * 1024)),
        Box::new(FixedClock),
    )
    .with_images(red());
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

fn photo() -> Option<FrameContent> {
    Some(FrameContent {
        asset: PHOTO,
        natural: (4, 4),
    })
}

// S66-05: outside the circle nothing is drawn, even inside the box.
#[test]
fn s66_05_outside_the_shape_is_transparent() {
    let pixels = render(document(photo(), None));
    for (x, y) in [(2, 2), (97, 2), (2, 97), (97, 97)] {
        assert_eq!(at(&pixels, x, y)[3], 0, "corner ({x}, {y})");
    }
}

// S66-05: inside the circle the photo shows.
#[test]
fn s66_05_inside_shows_the_image() {
    let pixels = render(document(photo(), None));
    for (x, y) in [(50, 50), (50, 10), (10, 50)] {
        assert_eq!(at(&pixels, x, y), [255, 0, 0, 255], "({x}, {y})");
    }
}

// FD-18 c: an empty frame is not exported as a visible box; its outline is.
#[test]
fn s66_05_an_empty_frame_shows_only_its_outline() {
    let empty = render(document(None, None));
    assert!(empty.iter().all(|&b| b == 0), "nothing at all");
    let black = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0));
    let outlined = render(document(None, Some(ShapeStroke::new(black, 4.0))));
    assert_eq!(at(&outlined, 50, 50)[3], 0, "the inside stays empty");
    assert!(at(&outlined, 50, 1)[3] > 0, "the outline at the top");
}
