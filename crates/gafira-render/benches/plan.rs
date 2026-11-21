//! The plan benchmarks of ARQ-40 (02 §6.7 and the XP3 of the design).
//!
//! The crate has no clock (ARQ-02, ADR 0016), so the bench never reads the
//! wall clock. It measures the plan — the tiles it holds and the operations
//! it carries — over the zoom range of 02 §6.1, and the only time source in
//! the bench is the `Clock` double it injects, whose instant stamps the
//! sweep.

use std::sync::Arc;
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_doc::visit::{PageView, WalkOrder};
use gafira_doc::{
    ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry,
    ShapeNode,
};
use gafira_geom::{IRect, Length, Unit};
use gafira_ports::common::UnixTime;
use gafira_ports::render::RenderQuality;
use gafira_ports::system::Clock;
use gafira_render::{
    GraphParams, InlinePool, PageExtent, Planner, RenderGraph, RenderGraphBuilder, Viewport,
};
use gafira_time::Flicks;

/// The zooms of the sweep: the range of 02 §6.1, from 5 % to 3200 %.
const ZOOMS: [f64; 7] = [0.05, 0.5, 1.0, 2.0, 8.0, 32.0, 64.0];

/// The `Clock` double of the bench: an instant that the sweep advances by the
/// stabilization window of 02 §6.1, never the system clock.
struct SweepClock {
    at: Duration,
}

impl Clock for SweepClock {
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    fn monotonic(&self) -> Duration {
        self.at
    }
}

/// A page of 1920 by 1080 pixels with one filled shape.
fn page() -> Page {
    let mut page = Page::new(
        PageId::new(1),
        "P".to_string(),
        PageSize {
            width: Length::new(1920.0, Unit::Px),
            height: Length::new(1080.0, Unit::Px),
        },
    );
    let base = ElementBase::new(NodeId::new(1), Placement::new(0.0, 0.0, 100.0, 100.0));
    let kind = NodeKind::Shape(ShapeNode {
        geometry: ShapeGeometry::Rect,
        fill: Some(Color::Srgb(Rgba::new(0.2, 0.4, 0.6, 1.0))),
        stroke: None,
        corner_radius: 0.0,
        text: None,
        gradient: None,
        effect: None,
    });
    page.nodes
        .insert(NodeId::new(1), Arc::new(Node::new(base, kind)));
    page.roots = vec![NodeId::new(1)];
    page
}

/// The graph of the page.
fn graph() -> RenderGraph {
    let page = page();
    let view = PageView::new(&page);
    let params = GraphParams {
        quality: RenderQuality::Preview,
        lod: 0,
        t: Flicks::from_ticks(0),
    };
    RenderGraphBuilder::new(&view, params, InlinePool)
        .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
        .expect("the page builds")
}

/// `bench_plan_zoom`: the tiles the plan holds at every zoom of the range.
///
/// The measure is the plan itself: the number of tiles and the number of
/// operations they carry. XP3 asks for the count of the visible tiles over
/// the zoom range; the wall clock never enters the measure.
fn main() {
    let extent = PageExtent {
        width: 1920,
        height: 1080,
    };
    let graph = graph();
    let mut clock = SweepClock { at: Duration::ZERO };
    for zoom in ZOOMS {
        let viewport = Viewport {
            rect: IRect::new(0, 0, extent.width as i32, extent.height as i32),
            zoom,
            focus: None,
        };
        let plan = Planner::plan(&graph, extent, &viewport, &CancelToken::new(), &NoProgress)
            .expect("the plan of the page");
        let ops: usize = plan.tiles().iter().map(|tile| tile.ops.len()).sum();
        println!(
            "bench_plan_zoom_{zoom}: level {}, tiles {}, ops {}, at {:?}",
            plan.level(),
            plan.tiles().len(),
            ops,
            clock.monotonic()
        );
        clock.at += Duration::from_millis(150);
    }
}
