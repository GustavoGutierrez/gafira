//! ARQ-42, AC2: the preview and the export render the same page through the
//! same graph and the same operations, and only the quality of the content
//! keys changes (02 §6.2).
//!
//! The test goes through the public surface of `gafira-render`: the graph of
//! `RE1`, the plan of `RE2` and the `ports::PageRenderer` that the exporters
//! reach (02 §4.5). The pixels of the two qualities are compared by the
//! golden of ARQ-42, which needs the CPU backend of O3 and does not live here.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_doc::visit::{PageView, WalkOrder};
use gafira_doc::{
    BlendMode, Document, Dpi, ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize,
    Placement, ShapeGeometry, ShapeNode,
};
use gafira_geom::{Affine, IRect, Length, Unit};
use gafira_ports::common::{Bytes, UnixTime};
use gafira_ports::export::{PageRenderRequest, PageRenderer};
use gafira_ports::render::{
    BackendCaps, BackendKind, Deadline, FrameCx, FrameId, MemoryUsage, RenderBackend, RenderError,
    RenderOp, RenderPlan, RenderQuality, RenderReport, TargetDesc, TileCache, TileKey,
};
use gafira_ports::system::Clock;
use gafira_raster::{ImageBuf, PixelBuf, PixelFormat};
use gafira_render::{
    BackendSelector, ContentTileCache, GraphNodeKind, GraphPageRenderer, GraphParams, InlinePool,
    RenderGraph, RenderGraphBuilder,
};
use gafira_time::Flicks;

/// The size of the page of the test, in pixels.
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

/// A page with one filled rectangle and one group around a second rectangle,
/// so the graph holds a source and a composite.
fn page() -> Page {
    let mut page = Page::new(
        PageId::new(1),
        "P".to_string(),
        PageSize {
            width: Length::new(f64::from(PAGE), Unit::Px),
            height: Length::new(f64::from(PAGE), Unit::Px),
        },
    );
    let shape = Node::new(
        ElementBase::new(NodeId::new(2), Placement::new(0.0, 0.0, 10.0, 10.0)),
        NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(Color::Srgb(Rgba::new(0.2, 0.4, 0.6, 1.0))),
            stroke: None,
            corner_radius: 0.0,
            text: None,
        }),
    );
    let group = Node::new(
        ElementBase::new(NodeId::new(3), Placement::new(20.0, 20.0, 20.0, 20.0)),
        NodeKind::Group(gafira_doc::GroupNode {}),
    );
    page.nodes.insert(NodeId::new(2), Arc::new(shape));
    page.nodes.insert(NodeId::new(3), Arc::new(group));
    page.roots = vec![NodeId::new(2), NodeId::new(3)];
    page
}

/// A document of the page of the test.
fn document() -> Document {
    Document::from_parts(4, Dpi::default(), vec![Arc::new(page())]).expect("the document validates")
}

/// The one request of the test: the whole page, no transparency.
fn request() -> PageRenderRequest {
    PageRenderRequest {
        page: PageId::new(1),
        time: Flicks::from_ticks(0),
        scale: 1.0,
        region: IRect::new(0, 0, PAGE as i32, PAGE as i32),
        transparent: false,
    }
}

/// What an operation does, without the curve data of its path.
#[derive(Clone, Debug, PartialEq)]
enum OpShape {
    PushLayer,
    PopLayer { blend: BlendMode, opacity: f32 },
    FillPath { color: Color, transform: Affine },
    DrawImage,
    ApplyMask,
    ApplyEffect,
    ApplyTransition,
}

/// The shape of one operation.
fn op_shape(op: &RenderOp) -> OpShape {
    match op {
        RenderOp::PushLayer => OpShape::PushLayer,
        RenderOp::PopLayer { blend, opacity } => OpShape::PopLayer {
            blend: *blend,
            opacity: *opacity,
        },
        RenderOp::FillPath {
            color, transform, ..
        } => OpShape::FillPath {
            color: *color,
            transform: *transform,
        },
        RenderOp::DrawImage { .. } => OpShape::DrawImage,
        RenderOp::ApplyMask { .. } => OpShape::ApplyMask,
        RenderOp::ApplyEffect { .. } => OpShape::ApplyEffect,
        RenderOp::ApplyTransition { .. } => OpShape::ApplyTransition,
    }
}

/// The shapes of a list of operations.
fn shapes(ops: &[RenderOp]) -> Vec<OpShape> {
    ops.iter().map(op_shape).collect()
}

/// One task the backend ran.
#[derive(Clone, Debug, PartialEq)]
struct Ran {
    quality: RenderQuality,
    key: TileKey,
    ops: Vec<OpShape>,
}

/// A CPU backend that records the tasks it runs and answers a transparent
/// surface of the target.
#[derive(Clone, Default)]
struct RecordingBackend {
    ran: Arc<Mutex<Vec<Ran>>>,
}

impl RecordingBackend {
    /// What the backend ran.
    fn ran(&self) -> Vec<Ran> {
        self.ran
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl RenderBackend for RecordingBackend {
    fn caps(&self) -> BackendCaps {
        BackendCaps {
            kind: BackendKind::Cpu,
            max_texture: 16_384,
            formats: vec![PixelFormat::Rgba8, PixelFormat::RgbaF32],
            gpu_effects: false,
        }
    }

    fn begin_frame(&mut self, _target: &TargetDesc) -> Result<FrameCx, RenderError> {
        Ok(FrameCx { id: FrameId(0) })
    }

    fn execute(
        &mut self,
        _cx: &mut FrameCx,
        plan: &RenderPlan,
        _cache: &mut dyn TileCache,
        _deadline: Deadline,
        cancel: &CancelToken,
    ) -> Result<RenderReport, RenderError> {
        cancel.check().map_err(RenderError::Cancelled)?;
        let mut ran = self.ran.lock().unwrap_or_else(PoisonError::into_inner);
        for task in &plan.tasks {
            ran.push(Ran {
                quality: plan.quality,
                key: task.key,
                ops: shapes(&task.ops),
            });
        }
        Ok(RenderReport {
            done: plan.tasks.len(),
            left: 0,
        })
    }

    fn read_back(&mut self, _cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError> {
        let (width, height) = (region.width(), region.height());
        let pixels = (width as usize) * (height as usize) * 4;
        ImageBuf::new(width, height, PixelBuf::Rgba8(vec![0; pixels])).map_err(RenderError::Buffer)
    }

    fn memory_usage(&self) -> MemoryUsage {
        MemoryUsage::default()
    }

    fn trim(&mut self, _budget: Bytes) {}
}

/// A renderer of the document at `quality`, with its backend's record.
fn renderer(quality: RenderQuality) -> (GraphPageRenderer, RecordingBackend) {
    let backend = RecordingBackend::default();
    let renderer = GraphPageRenderer::new(
        document(),
        quality,
        BackendSelector::new(None, Box::new(backend.clone())),
        ContentTileCache::new(Bytes(1024 * 1024)),
        Box::new(FixedClock),
    );
    (renderer, backend)
}

/// The graph of the page at `quality`, built with the public API of `RE1`.
fn graph(quality: RenderQuality) -> RenderGraph {
    let page = page();
    let view = PageView::new(&page);
    RenderGraphBuilder::new(
        &view,
        GraphParams {
            quality,
            lod: 0,
            t: Flicks::from_ticks(0),
        },
        InlinePool,
    )
    .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
    .expect("the page builds")
}

// ARQ-42, AC2: the preview and the export build the same graph with the same
// operations, and only the quality of the content keys differs.
#[test]
fn arq_42_the_preview_and_the_export_share_the_graph_and_the_operations() {
    let preview = graph(RenderQuality::Preview);
    let export = graph(RenderQuality::Export);

    assert_eq!(preview.nodes().len(), export.nodes().len());
    assert_eq!(preview.nodes().len(), 2, "a shape and a group of the page");
    for (one, other) in preview.nodes().iter().zip(export.nodes()) {
        assert_eq!(one.origin(), other.origin(), "the same document node");
        assert_eq!(one.kind(), other.kind(), "the same kind of node");
        assert!(
            matches!(
                one.kind(),
                GraphNodeKind::Source(_) | GraphNodeKind::Composite
            ),
            "the graph holds the kinds of RE1"
        );
        assert_ne!(one.key(), other.key(), "the quality is in the content key");
    }
    assert_eq!(
        shapes(preview.ops()),
        shapes(export.ops()),
        "the two qualities emit the same operations"
    );
}

// ARQ-42, AC2 (integration): the renderer of each quality hands its backend
// the plan of the same graph, with the same tiles and operations.
#[test]
fn arq_42_the_rendered_plans_differ_only_in_the_content_keys() {
    let (preview_renderer, preview_backend) = renderer(RenderQuality::Preview);
    let (export_renderer, export_backend) = renderer(RenderQuality::Export);

    let request = request();
    let preview_image = preview_renderer
        .render(&request, &CancelToken::new())
        .expect("the preview renders");
    let export_image = export_renderer
        .render(&request, &CancelToken::new())
        .expect("the export renders");
    assert_eq!(
        (preview_image.width(), preview_image.height()),
        (PAGE, PAGE)
    );
    assert_eq!((export_image.width(), export_image.height()), (PAGE, PAGE));

    let preview = preview_backend.ran();
    let export = export_backend.ran();
    assert_eq!(preview.len(), export.len(), "the same number of tiles");
    assert!(!preview.is_empty(), "the page holds a tile");

    for (one, other) in preview.iter().zip(&export) {
        assert_eq!(one.quality, RenderQuality::Preview);
        assert_eq!(other.quality, RenderQuality::Export);
        assert_eq!(
            (one.key.level, one.key.column, one.key.row),
            (other.key.level, other.key.column, other.key.row),
            "the same tile of the same level"
        );
        assert_ne!(
            one.key.content, other.key.content,
            "only the quality of the content key differs"
        );
        assert_eq!(
            one.ops, other.ops,
            "the same operations reach the backend of each quality"
        );
        assert!(
            one.ops.len() > 1,
            "a tile carries the background and the operations of the page's roots"
        );
    }

    // O12: the page has two roots, and the task of a region composes both, so
    // its content key is neither root's key.
    for (quality, task) in [
        (RenderQuality::Preview, &preview[0]),
        (RenderQuality::Export, &export[0]),
    ] {
        let graph = graph(quality);
        assert_eq!(graph.roots().len(), 2, "the page has two roots");
        for root in graph.roots() {
            let root = graph.node(*root).expect("the root is a node");
            assert_ne!(
                task.key.content,
                root.key(),
                "the task composes both roots, not one of them"
            );
        }
    }
}

// ARQ-42, AC1: the engine reaches an exporter through the port (02 §4.5).
#[test]
fn arq_42_the_engine_answers_through_the_page_renderer_port() {
    let (renderer, _) = renderer(RenderQuality::Export);
    let pages: &dyn PageRenderer = &renderer;
    let image = pages
        .render(&request(), &CancelToken::new())
        .expect("the port renders the page");
    assert_eq!((image.width(), image.height()), (PAGE, PAGE));
    assert_eq!(renderer.quality(), RenderQuality::Export);
}
