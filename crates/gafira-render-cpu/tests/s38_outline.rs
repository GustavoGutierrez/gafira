//! Spec 038: the outline and the corner radius of a shape reach the pixels
//! through the graph of `gafira-render` and the CPU backend, the path the
//! PNG and PDF exports and the thumbnails take.

use std::sync::Arc;
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::CancelToken;
use gafira_doc::{
    Document, Dpi, ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeGeometry, ShapeNode, ShapeStroke,
};
use gafira_geom::{IRect, Length, Unit};
use gafira_ports::common::{Bytes, UnixTime};
use gafira_ports::export::{PageRenderRequest, PageRenderer};
use gafira_ports::render::RenderQuality;
use gafira_ports::system::Clock;
use gafira_raster::PixelBuf;
use gafira_render::{BackendSelector, ContentTileCache, GraphPageRenderer};
use gafira_render_cpu::CpuBackend;

/// The side of the square page, in px.
const PAGE: u32 = 64;

/// A clock stuck at one instant; it never reads the system time (ARQ-02).
struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// A red rectangle in the box `(16, 16, 32, 32)` with a blue outline of 4 px
/// and the given corner radius, drawn on a transparent page.
fn pixels(radius: f64) -> Vec<u8> {
    let mut shape = ShapeNode::new(ShapeGeometry::Rect);
    shape.fill = Some(Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0)));
    shape.stroke = Some(ShapeStroke::new(
        Color::Srgb(Rgba::new(0.0, 0.0, 1.0, 1.0)),
        4.0,
    ));
    shape.corner_radius = radius;
    let base = ElementBase::new(NodeId::new(2), Placement::new(16.0, 16.0, 32.0, 32.0));
    let mut page = Page::new(
        PageId::new(1),
        "S38".to_string(),
        PageSize {
            width: Length::new(f64::from(PAGE), Unit::Px),
            height: Length::new(f64::from(PAGE), Unit::Px),
        },
    );
    page.nodes.insert(
        NodeId::new(2),
        Arc::new(Node::new(base, NodeKind::Shape(shape))),
    );
    page.roots = vec![NodeId::new(2)];
    let document = Document::from_parts(3, Dpi::default(), vec![Arc::new(page)])
        .expect("the document validates");
    let renderer = GraphPageRenderer::new(
        document,
        RenderQuality::Preview,
        BackendSelector::new(None, Box::new(CpuBackend::new())),
        ContentTileCache::new(Bytes(1024 * 1024)),
        Box::new(FixedClock),
    );
    let request = PageRenderRequest {
        page: PageId::new(1),
        time: Default::default(),
        scale: 1.0,
        region: IRect::new(0, 0, PAGE as i32, PAGE as i32),
        transparent: true,
    };
    let image = renderer
        .render(&request, &CancelToken::new())
        .expect("the page renders");
    match image.data() {
        PixelBuf::Rgba8(bytes) => bytes.clone(),
        other => panic!("the preview answers Rgba8, not {other:?}"),
    }
}

/// The pixel at `(x, y)`.
fn at(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * PAGE + x) * 4) as usize;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

// S38-04: the outline covers 2 px on each side of the edge, over the fill.
#[test]
fn s38_04_the_outline_is_painted_centred_on_the_edge_over_the_fill() {
    let square = pixels(0.0);
    assert_eq!(at(&square, 32, 32), RED, "the fill inside");
    assert_eq!(at(&square, 14, 32), BLUE, "the outline outside the box");
    assert_eq!(at(&square, 17, 32), BLUE, "the outline inside the box");
    assert_eq!(at(&square, 19, 32), RED, "past the outline");
    assert_eq!(at(&square, 12, 32), CLEAR, "beyond the outline");
    assert_eq!(at(&square, 17, 17), BLUE, "a square corner");
}

// S38-05: a rounded rectangle leaves its corners empty and its outline
// follows the curve.
#[test]
fn s38_05_the_rounded_corners_are_empty_and_outlined() {
    let round = pixels(12.0);
    assert_eq!(at(&round, 17, 17), CLEAR, "outside the rounded corner");
    assert_eq!(at(&round, 32, 32), RED, "the fill inside");
    assert_eq!(at(&round, 32, 15), BLUE, "the outline on a straight side");
    // On the diagonal of the corner circle (centre (28, 28), radius 12).
    assert_eq!(at(&round, 19, 19), BLUE, "the outline on the curve");
}
