//! S65-04: «Brillo» and «Destello de lente» through the whole export path
//! (graph, planner, CPU backend) on a black page.

use std::sync::Arc;
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::CancelToken;
use gafira_doc::{
    Document, Dpi, ElementBase, Light, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ShapeEffect, ShapeGeometry, ShapeNode,
};
use gafira_geom::{IRect, Length, Unit};
use gafira_ports::common::{Bytes, UnixTime};
use gafira_ports::export::{PageRenderRequest, PageRenderer};
use gafira_ports::render::RenderQuality;
use gafira_ports::system::Clock;
use gafira_raster::PixelBuf;
use gafira_render::{BackendSelector, ContentTileCache, GraphPageRenderer};
use gafira_render_cpu::CpuBackend;

const W: u32 = 480;
const H: u32 = 270;

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }
}

fn node(id: u64, rect: [f64; 4], shape: ShapeNode) -> Node {
    Node::new(
        ElementBase::new(
            NodeId::new(id),
            Placement::new(rect[0], rect[1], rect[2], rect[3]),
        ),
        NodeKind::Shape(shape),
    )
}

/// A black page with a sparkle on the left and a lens flare top centre.
fn document() -> Document {
    let mut page = Page::new(
        PageId::new(1),
        "S65".to_string(),
        PageSize {
            width: Length::new(f64::from(W), Unit::Px),
            height: Length::new(f64::from(H), Unit::Px),
        },
    );
    let mut black = ShapeNode::new(ShapeGeometry::Rect);
    black.fill = Some(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)));
    let mut sparkle = ShapeNode::new(ShapeGeometry::Ellipse);
    sparkle.effect = Some(ShapeEffect::Sparkle(Light::SPARKLE));
    let mut flare = ShapeNode::new(ShapeGeometry::Ellipse);
    flare.effect = Some(ShapeEffect::LensFlare(Light::LENS_FLARE));
    let nodes = [
        node(2, [0.0, 0.0, f64::from(W), f64::from(H)], black),
        node(3, [40.0, 60.0, 80.0, 80.0], sparkle),
        node(4, [200.0, 40.0, 40.0, 40.0], flare),
    ];
    for n in nodes {
        page.roots.push(n.base.id);
        page.nodes.insert(n.base.id, Arc::new(n));
    }
    Document::from_parts(5, Dpi::default(), vec![Arc::new(page)]).expect("valid")
}

fn render() -> Vec<u8> {
    let renderer = GraphPageRenderer::new(
        document(),
        RenderQuality::Preview,
        BackendSelector::new(None, Box::new(CpuBackend::new())),
        ContentTileCache::new(Bytes(4 * 1024 * 1024)),
        Box::new(FixedClock),
    );
    let request = PageRenderRequest {
        page: PageId::new(1),
        time: Default::default(),
        scale: 1.0,
        region: IRect::new(0, 0, W as i32, H as i32),
        transparent: false,
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
    let i = ((y * W + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

#[test]
fn s65_04_lights_render_on_a_black_page() {
    let pixels = render();
    // The core of the sparkle is white; far from both lights stays dark.
    assert!(at(&pixels, 80, 100)[..3].iter().all(|&c| c > 240));
    assert!(at(&pixels, 470, 10)[..3].iter().all(|&c| c < 40));
    // The flare reaches the far side of the centre of the page, along the
    // line from the light (220, 60) through the centre (240, 135).
    let ghost = at(&pixels, 249, 169);
    let off = at(&pixels, 120, 240);
    assert!(
        u32::from(ghost[0]) + u32::from(ghost[1]) + u32::from(ghost[2])
            > u32::from(off[0]) + u32::from(off[1]) + u32::from(off[2]) + 15,
        "ghost {ghost:?}, off {off:?}"
    );
}
