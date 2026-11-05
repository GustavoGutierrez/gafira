//! The page renderer that the exporters reach (02 §4.5, ARQ-42) and the page
//! scope of an export (FD-57, FD-58).
//!
//! One page renders through the graph of `RE1`, the plan of `RE2` and the tile
//! cache of `RE3`, driven by the frame loop of `RE4`: the loop opens the
//! surface, runs the tasks of the plan until the end and reads the surface
//! back. The quality is taken when the renderer is built, because the port
//! carries no quality field (02 §4.5), and it decides two things and only two:
//! the pixel format of the surface and the level of detail the plan resolves
//! (02 §6.2). The graph and the operations are the same for both qualities.
//!
//! The render runs to the end: the port gives [`PageRenderer::render`] no
//! deadline, and an export is not cut by the frame budget of the preview
//! (02 §6.2). The instant still comes from the injected
//! [`Clock`](gafira_ports::system::Clock), never from one the crate owns
//! (ARQ-02, ADR 0016).
//!
//! The page scope of FD-57 maps to the requests of an export and is where a
//! page the document does not hold is rejected, before any render starts
//! (FD-58): `RenderError` has no variant that can carry that, so
//! [`PageRenderer::render`] stays total over [`PageId`] instead.

use std::sync::{Arc, Mutex, PoisonError};

use gafira_color::{Color, Rgba};
use gafira_doc::contract::{CancelToken, NoProgress};
use gafira_doc::visit::{PageView, WalkOrder};
use gafira_doc::{Document, Page};
use gafira_geom::{Affine, IRect};
use gafira_ports::export::{PageRenderRequest, PageRenderer};
use gafira_ports::render::{Deadline, RenderError, RenderOp, RenderQuality, TargetDesc};
use gafira_ports::system::Clock;
use gafira_raster::{BufferError, ImageBuf, PixelBuf, PixelFormat};
use kurbo::{Rect, Shape};

use crate::cache::ContentTileCache;
use crate::frame::{BackendSelector, Degradation, FrameLoop, FrameRequest};
use crate::graph::{GraphError, GraphParams, RenderGraph, RenderGraphBuilder};
use crate::planner::{PageExtent, PlanError, Planner, TileGrid, Viewport};
use crate::pool::InlinePool;

/// The background of a page: white, opaque (FD-55, specs/04 §13).
///
/// The page model carries no background colour yet, so the default of FD-55 is
/// the only one; a page with its own colour gives it to the exporter, which
/// paints it the same way.
pub const PAGE_BACKGROUND: Color = Color::Srgb(Rgba::new(1.0, 1.0, 1.0, 1.0));

/// The renderer of one page for the exporters (02 §4.5, ARQ-42).
///
/// The quality is taken when the renderer is built: the preview answers 8-bit
/// sRGB premultiplied at the zoom of the request, the export answers `f32` in
/// linear light at the level zero (02 §6.2). The two use the same graph and
/// the same operations; only the quality of the content keys and the format of
/// the surface differ.
///
/// The backend, the cache and the clock belong to the caller (P9, P10): the
/// renderer creates none of them.
pub struct GraphPageRenderer {
    document: Document,
    quality: RenderQuality,
    frame: Mutex<FrameLoop>,
    cache: Mutex<ContentTileCache>,
    clock: Box<dyn Clock>,
}

impl GraphPageRenderer {
    /// The renderer of the pages of `document` at `quality`.
    pub fn new(
        document: Document,
        quality: RenderQuality,
        selector: BackendSelector,
        cache: ContentTileCache,
        clock: Box<dyn Clock>,
    ) -> Self {
        Self {
            document,
            quality,
            frame: Mutex::new(FrameLoop::new(selector)),
            cache: Mutex::new(cache),
            clock,
        }
    }

    /// The quality this renderer answers (02 §6.2).
    #[must_use]
    pub fn quality(&self) -> RenderQuality {
        self.quality
    }

    /// The degradation of the session, if the GPU was lost (ARQ-22).
    #[must_use]
    pub fn degradation(&self) -> Option<Degradation> {
        self.frame
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .degradation()
    }

    /// The pixel format of the surface of this quality (02 §6.2).
    fn format(&self) -> PixelFormat {
        format_of(self.quality)
    }

    /// The zoom the plan resolves for this quality (02 §6.2, ARQ-42).
    ///
    /// The preview reads the `scale` of the request as the zoom of the view, so
    /// the level follows it; the export renders at 100 %, the level zero, which
    /// is the full resolution of FD-55.
    fn zoom(&self, scale: f32) -> f64 {
        zoom_of(self.quality, scale)
    }

    /// The size of the page in whole pixels.
    ///
    /// The document validation keeps every side inside `1..=16384` px, so the
    /// failure of the unit math is not reachable for a document that was
    /// validated; it still answers an error instead of panicking.
    fn extent(&self, page: &Page) -> Result<PageExtent, RenderError> {
        let px = page
            .size
            .to_px(self.document.dpi())
            .map_err(|_| non_finite("page.size"))?;
        Ok(PageExtent {
            width: side_px(px.width()),
            height: side_px(px.height()),
        })
    }

    /// The graph of a page at `params` (RE1).
    fn graph(
        &self,
        page: &Page,
        params: GraphParams,
        cancel: &CancelToken,
    ) -> Result<RenderGraph, RenderError> {
        let view = PageView::new(page);
        RenderGraphBuilder::new(&view, params, InlinePool)
            .build(WalkOrder::PreOrder, cancel, &NoProgress)
            .map_err(graph_error)
    }

    /// The plan of the graph over the region of the request (RE2).
    fn plan(
        &self,
        graph: &RenderGraph,
        extent: PageExtent,
        viewport: &Viewport,
        cancel: &CancelToken,
    ) -> Result<crate::planner::RenderPlan, RenderError> {
        Planner::plan(graph, extent, viewport, cancel, &NoProgress).map_err(plan_error)
    }
}

impl PageRenderer for GraphPageRenderer {
    fn render(
        &self,
        request: &PageRenderRequest,
        cancel: &CancelToken,
    ) -> Result<ImageBuf, RenderError> {
        let format = self.format();
        let region = target_region(request.region);
        // AC4: the port gives no variant for a page the document does not
        // hold. The scope mapping is what rejects it (FD-58), so `render` stays
        // total over `PageId` and answers the empty region instead of failing
        // the device or telling the user they cancelled.
        let Some(page) = self.document.page(request.page) else {
            return blank(request.region, format);
        };

        let extent = self.extent(page)?;
        let zoom = self.zoom(request.scale);
        // ARQ-42: the graph and the plan must resolve the same level; the level
        // is part of the content key of a node (02 §6.1).
        let level = TileGrid::new(extent).level_for(zoom);
        let graph = self.graph(
            page,
            GraphParams {
                quality: self.quality,
                lod: level,
                t: request.time,
            },
            cancel,
        )?;
        let viewport = Viewport {
            rect: request.region,
            zoom,
            focus: None,
        };
        let planned = self.plan(&graph, extent, &viewport, cancel)?;
        let mut plan = planned.into_ports_plan();
        // AC7: without transparency the page background goes under the content
        // (FD-55); with it, nothing is painted and the surface keeps its alpha.
        if !request.transparent {
            paint_background(&mut plan, extent);
        }

        let target = TargetDesc {
            width: region.width(),
            height: region.height(),
            format,
        };
        let mut frame = self.frame.lock().unwrap_or_else(PoisonError::into_inner);
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        frame.frame(
            FrameRequest {
                clock: self.clock.as_ref(),
                target,
                // An export is not cut by the frame budget of the preview
                // (02 §6.2): the page renders to the end.
                deadline: Deadline::Unbounded,
                cache: &mut *cache,
                cancel,
                progress: &NoProgress,
            },
            plan,
        )?;
        match frame.read_back(region) {
            Some(image) => image,
            // No frame opened the surface: the render has no pixels to answer.
            None => blank(region, format),
        }
    }
}

/// The pixel format of each quality (02 §6.2).
fn format_of(quality: RenderQuality) -> PixelFormat {
    match quality {
        RenderQuality::Preview => PixelFormat::Rgba8,
        RenderQuality::Export => PixelFormat::RgbaF32,
    }
}

/// The zoom of each quality (02 §6.2): the preview follows the scale of the
/// request and the export renders at 100 %, the full resolution.
fn zoom_of(quality: RenderQuality, scale: f32) -> f64 {
    match quality {
        RenderQuality::Preview => f64::from(scale),
        RenderQuality::Export => 1.0,
    }
}

/// One side of the page in whole pixels, at least one.
fn side_px(px: f64) -> u32 {
    (px.round() as u32).max(1)
}

/// The region of the surface to read back: the output starts at its origin,
/// whatever the page offset of the request.
fn target_region(region: IRect) -> IRect {
    IRect::new(0, 0, whole(region.width()), whole(region.height()))
}

/// A pixel count of the surface as the coordinate type of [`IRect`].
fn whole(px: u32) -> i32 {
    i32::try_from(px).unwrap_or(i32::MAX)
}

/// An empty (fully transparent) image of the region (AC4, AC5).
fn blank(region: IRect, format: PixelFormat) -> Result<ImageBuf, RenderError> {
    let width = region.width();
    let height = region.height();
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or(RenderError::Buffer(BufferError::SizeOverflow {
            width,
            height,
        }))?;
    let data = match format {
        PixelFormat::RgbaF32 => PixelBuf::RgbaF32(vec![0.0; pixels]),
        // The surface of a render is never `Alpha8` (02 §4.6).
        PixelFormat::Rgba8 | PixelFormat::Alpha8 => PixelBuf::Rgba8(vec![0; pixels]),
    };
    ImageBuf::new(width, height, data).map_err(RenderError::Buffer)
}

/// Paints the page background under the content of every tile (FD-55).
///
/// The fill covers the whole page and every task of the plan draws it first;
/// the backend clips it to the tile it draws.
fn paint_background(plan: &mut gafira_ports::render::RenderPlan, extent: PageExtent) {
    let path = Rect::new(0.0, 0.0, f64::from(extent.width), f64::from(extent.height))
        .abs()
        .to_path(0.25);
    let path = Arc::new(path);
    for task in &mut plan.tasks {
        task.ops.insert(
            0,
            RenderOp::FillPath {
                path: Arc::clone(&path),
                color: PAGE_BACKGROUND,
                transform: Affine::IDENTITY,
            },
        );
    }
}

/// A buffer error for a field that is not finite (ARQ-110).
fn non_finite(field: &'static str) -> RenderError {
    RenderError::Buffer(BufferError::NonFinite { field, index: 0 })
}

/// The graph errors as the errors of the port (ARQ-110, ARQ-139).
fn graph_error(error: GraphError) -> RenderError {
    match error {
        GraphError::NonFinite { field } => non_finite(field),
        GraphError::Cancelled(cancelled) => RenderError::Cancelled(cancelled),
    }
}

/// The plan errors as the errors of the port (ARQ-139).
fn plan_error(error: PlanError) -> RenderError {
    match error {
        PlanError::Cancelled(cancelled) => RenderError::Cancelled(cancelled),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gafira_color::{Color, Rgba};
    use gafira_doc::contract::NoProgress;
    use gafira_doc::visit::{PageView, WalkOrder};
    use gafira_doc::{
        BlendMode, Document, Dpi, ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize,
        Placement, ShapeGeometry, ShapeNode,
    };
    use gafira_geom::{Length, Unit};
    use gafira_ports::common::{Bytes, UnixTime};
    use gafira_ports::render::{
        BackendCaps, BackendKind, FrameCx, FrameId, MemoryUsage, RenderBackend, RenderPlan,
        RenderReport, TileCache, TileKey,
    };
    use gafira_time::Flicks;

    use super::*;

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

    /// A page of the given size in pixels, with one filled rectangle.
    fn page(id: PageId, node: NodeId, width: u32, height: u32) -> Page {
        let mut page = Page::new(
            id,
            "P".to_string(),
            PageSize {
                width: Length::new(f64::from(width), Unit::Px),
                height: Length::new(f64::from(height), Unit::Px),
            },
        );
        let base = ElementBase::new(node, Placement::new(0.0, 0.0, 10.0, 10.0));
        let kind = NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(Color::Srgb(Rgba::new(0.2, 0.4, 0.6, 1.0))),
        });
        page.nodes.insert(node, Arc::new(Node::new(base, kind)));
        page.roots = vec![node];
        page
    }

    /// A document of the given page sizes, each page with one filled shape.
    fn document(sizes: &[(u32, u32)]) -> Document {
        let mut next_id = 1u64;
        let mut pages = Vec::with_capacity(sizes.len());
        for (width, height) in sizes {
            let id = PageId::new(next_id);
            next_id += 1;
            let node = NodeId::new(next_id);
            next_id += 1;
            pages.push(Arc::new(page(id, node, *width, *height)));
        }
        Document::from_parts(next_id, Dpi::default(), pages).expect("the document validates")
    }

    /// One request over a region of the page.
    fn request(page: PageId, region: IRect, scale: f32, transparent: bool) -> PageRenderRequest {
        PageRenderRequest {
            page,
            time: Flicks::from_ticks(0),
            scale,
            region,
            transparent,
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

    /// The shapes of the operations of a tile.
    fn shapes(ops: &[RenderOp]) -> Vec<OpShape> {
        ops.iter().map(op_shape).collect()
    }

    /// One plan the backend ran.
    #[derive(Clone, Debug, PartialEq)]
    struct Ran {
        quality: RenderQuality,
        keys: Vec<TileKey>,
        ops: Vec<Vec<OpShape>>,
    }

    /// What the recording backend collected.
    #[derive(Default)]
    struct Record {
        targets: Vec<TargetDesc>,
        ran: Vec<Ran>,
        reads: Vec<IRect>,
        hits: usize,
    }

    /// What the backend saw, one snapshot at a time.
    fn seen(record: &Arc<Mutex<Record>>) -> (Vec<TargetDesc>, Vec<Ran>, Vec<IRect>, usize) {
        let record = record.lock().unwrap_or_else(PoisonError::into_inner);
        (
            record.targets.clone(),
            record.ran.clone(),
            record.reads.clone(),
            record.hits,
        )
    }

    /// A CPU backend that records what it is asked to draw and answers pixels
    /// that carry the level and the content key of the last tile it ran, so two
    /// renders can be told apart by their bytes.
    struct RecordingBackend {
        record: Arc<Mutex<Record>>,
        cancel_at: Option<usize>,
        keep: bool,
        stored: bool,
        last: (u8, u8),
    }

    impl RecordingBackend {
        fn new() -> Self {
            Self {
                record: Arc::new(Mutex::new(Record::default())),
                cancel_at: None,
                keep: false,
                stored: false,
                last: (0, 0),
            }
        }

        /// A backend that cancels its token on the execute call at `index`.
        fn cancelling(mut self, index: usize) -> Self {
            self.cancel_at = Some(index);
            self
        }

        /// A backend that stores the first tile it runs and reads it again on
        /// the next render, so a test can tell the cache is alive (RE3).
        fn keeping(mut self) -> Self {
            self.keep = true;
            self
        }

        /// The record, shared with the test.
        fn record(&self) -> Arc<Mutex<Record>> {
            Arc::clone(&self.record)
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

        fn begin_frame(&mut self, target: &TargetDesc) -> Result<FrameCx, RenderError> {
            self.record
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .targets
                .push(*target);
            Ok(FrameCx { id: FrameId(0) })
        }

        fn execute(
            &mut self,
            _cx: &mut FrameCx,
            plan: &RenderPlan,
            cache: &mut dyn TileCache,
            _deadline: Deadline,
            cancel: &CancelToken,
        ) -> Result<RenderReport, RenderError> {
            let ran = Ran {
                quality: plan.quality,
                keys: plan.tasks.iter().map(|task| task.key).collect(),
                ops: plan.tasks.iter().map(|task| shapes(&task.ops)).collect(),
            };
            let at = {
                let mut record = self.record.lock().unwrap_or_else(PoisonError::into_inner);
                record.ran.push(ran);
                record.ran.len() - 1
            };
            if let Some(task) = plan.tasks.last() {
                self.last = (task.key.level, task.key.content.0 as u8);
            }
            if self.cancel_at == Some(at) {
                cancel.cancel();
            }
            if self.keep
                && let Some(task) = plan.tasks.first()
            {
                if self.stored {
                    if cache.get(&task.key).is_some() {
                        self.record
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .hits += 1;
                    }
                } else if let Ok(tile) = one_pixel() {
                    cache.put(task.key, Arc::new(tile));
                    self.stored = true;
                }
            }
            Ok(RenderReport {
                done: plan.tasks.len(),
                left: 0,
            })
        }

        fn read_back(&mut self, _cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError> {
            self.record
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .reads
                .push(region);
            let (level, key) = self.last;
            let (width, height) = (region.width(), region.height());
            let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.copy_from_slice(&[level, key, 0, 255]);
            }
            ImageBuf::new(width, height, PixelBuf::Rgba8(pixels)).map_err(RenderError::Buffer)
        }

        fn memory_usage(&self) -> MemoryUsage {
            MemoryUsage::default()
        }

        fn trim(&mut self, _budget: Bytes) {}
    }

    /// A transparent tile of one pixel.
    fn one_pixel() -> Result<ImageBuf, RenderError> {
        ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![0; 4])).map_err(RenderError::Buffer)
    }

    /// A renderer over `document`, with the backend's record for the test.
    fn renderer(
        document: Document,
        quality: RenderQuality,
        backend: RecordingBackend,
    ) -> (GraphPageRenderer, Arc<Mutex<Record>>) {
        let record = backend.record();
        let renderer = GraphPageRenderer::new(
            document,
            quality,
            BackendSelector::new(None, Box::new(backend)),
            ContentTileCache::new(Bytes(8 * 1024 * 1024)),
            Box::new(FixedClock),
        );
        (renderer, record)
    }

    /// The graph of the page of a document (RE1), for the expected values.
    fn graph_of(document: &Document, quality: RenderQuality, lod: u8) -> RenderGraph {
        let view = PageView::new(&document.pages()[0]);
        RenderGraphBuilder::new(
            &view,
            GraphParams {
                quality,
                lod,
                t: Flicks::from_ticks(0),
            },
            InlinePool,
        )
        .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
        .expect("the page builds")
    }

    // ARQ-42, AC1: the renderer answers the pixels of the region through the
    // plan of the graph and the tile cache.
    #[test]
    fn arq_42_a_page_renders_through_the_graph_the_plan_and_the_cache() {
        let document = document(&[(200, 150)]);
        let page = document.pages()[0].id;
        let (renderer, record) = renderer(
            document.clone(),
            RenderQuality::Preview,
            RecordingBackend::new().keeping(),
        );
        let request = request(page, IRect::new(0, 0, 200, 150), 1.0, false);

        let image = renderer
            .render(&request, &CancelToken::new())
            .expect("the page renders");
        assert_eq!((image.width(), image.height()), (200, 150));

        // The plan is the one the planner builds from the graph of RE1: same
        // tiles, same operations, with the page background in front.
        let graph = graph_of(&document, RenderQuality::Preview, 0);
        let planned = Planner::plan(
            &graph,
            PageExtent {
                width: 200,
                height: 150,
            },
            &Viewport {
                rect: IRect::new(0, 0, 200, 150),
                zoom: 1.0,
                focus: None,
            },
            &CancelToken::new(),
            &NoProgress,
        )
        .expect("the plan");
        let expected: Vec<TileKey> = planned.tiles().iter().map(|tile| tile.key).collect();
        let (targets, ran, reads, _) = seen(&record);
        let keys: Vec<TileKey> = ran.iter().flat_map(|plan| plan.keys.clone()).collect();
        assert_eq!(keys, expected);
        let ops: Vec<OpShape> = ran
            .iter()
            .flat_map(|plan| plan.ops.iter().flatten().cloned())
            .collect();
        assert_eq!(
            ops.first().cloned(),
            Some(OpShape::FillPath {
                color: PAGE_BACKGROUND,
                transform: Affine::IDENTITY,
            })
        );
        let from_graph = shapes(graph.root_ops().next().expect("a root"));
        assert_eq!(&ops[1..], from_graph.as_slice());
        assert_eq!(
            targets,
            vec![TargetDesc {
                width: 200,
                height: 150,
                format: PixelFormat::Rgba8,
            }]
        );
        assert_eq!(reads, vec![IRect::new(0, 0, 200, 150)]);

        // The tiles of RE3 live in a cache the renderer keeps across renders.
        renderer
            .render(&request, &CancelToken::new())
            .expect("the page renders again");
        let (_, _, _, hits) = seen(&record);
        assert_eq!(hits, 1, "the second render reads the tile of the first");
    }

    // ARQ-42, AC3: the preview is 8-bit sRGB premultiplied at the zoom, the
    // export is f32 in linear light at the level zero.
    #[test]
    fn arq_42_the_preview_is_rgba8_and_the_export_is_rgba_f32() {
        let document = document(&[(256, 256)]);
        let page = document.pages()[0].id;
        for (quality, format, level) in [
            (RenderQuality::Preview, PixelFormat::Rgba8, 1),
            (RenderQuality::Export, PixelFormat::RgbaF32, 0),
        ] {
            let (renderer, record) = renderer(document.clone(), quality, RecordingBackend::new());
            let image = renderer
                .render(
                    &request(page, IRect::new(0, 0, 256, 256), 2.0, false),
                    &CancelToken::new(),
                )
                .expect("the page renders");
            assert_eq!(image.width(), 256);
            let (targets, ran, _, _) = seen(&record);
            assert_eq!(targets[0].format, format, "{quality:?} answers its format");
            assert_eq!(
                ran[0].quality, quality,
                "{quality:?} plans for its own quality"
            );
            assert_eq!(
                ran[0].keys[0].level, level,
                "{quality:?} resolves its own level of detail"
            );
        }
    }

    // FD-57, AC4: `render` stays total over `PageId` because the port fixes no
    // variant for a page the document does not hold.
    #[test]
    fn fd_57_render_is_total_over_the_page_id() {
        let document = document(&[(10, 10)]);
        let (renderer, record) =
            renderer(document, RenderQuality::Preview, RecordingBackend::new());
        let image = renderer
            .render(
                &request(PageId::new(1_000), IRect::new(0, 0, 10, 10), 1.0, false),
                &CancelToken::new(),
            )
            .expect("the render answers for any page id");
        assert_eq!((image.width(), image.height()), (10, 10));
        assert_eq!(*image.data(), PixelBuf::Rgba8(vec![0; 400]));
        let (targets, ran, _, _) = seen(&record);
        assert!(targets.is_empty(), "no frame opens for a missing page");
        assert!(ran.is_empty(), "no plan is built for a missing page");
    }

    // ARQ-43, AC5: a cancelled render answers `Cancelled` and no buffer.
    #[test]
    fn arq_43_a_cancelled_render_answers_cancelled_without_a_buffer() {
        let document = document(&[(300, 300)]);
        let page = document.pages()[0].id;
        let request = request(page, IRect::new(0, 0, 300, 300), 1.0, false);

        // Cancelled before the render starts.
        let (first, _) = renderer(
            document.clone(),
            RenderQuality::Preview,
            RecordingBackend::new(),
        );
        let token = CancelToken::new();
        token.cancel();
        let result = first.render(&request, &token);
        assert!(
            matches!(result, Err(RenderError::Cancelled(_))),
            "a cancelled token answers cancelled, not a buffer: {result:?}"
        );

        // Cancelled between two tiles, by the backend of the first one.
        let (renderer, record) = renderer(
            document,
            RenderQuality::Preview,
            RecordingBackend::new().cancelling(0),
        );
        let token = CancelToken::new();
        let result = renderer.render(&request, &token);
        assert!(
            matches!(result, Err(RenderError::Cancelled(_))),
            "a mid-render cancel answers cancelled, not a half buffer: {result:?}"
        );
        let (_, ran, _, _) = seen(&record);
        assert_eq!(ran.len(), 1, "only the tile the backend ran is drawn");
    }

    // ARQ-02, AC6: two renders of the same page at the same quality are the
    // same bytes.
    #[test]
    fn arq_02_two_renders_of_the_same_page_are_identical() {
        let document = document(&[(200, 150)]);
        let page = document.pages()[0].id;
        for (quality, scale) in [
            (RenderQuality::Preview, 1.0),
            (RenderQuality::Preview, 2.0),
            (RenderQuality::Export, 1.0),
        ] {
            let (renderer, _) = renderer(document.clone(), quality, RecordingBackend::new());
            let request = request(page, IRect::new(0, 0, 200, 150), scale, false);
            let first = renderer
                .render(&request, &CancelToken::new())
                .expect("the first render");
            let second = renderer
                .render(&request, &CancelToken::new())
                .expect("the second render");
            assert_eq!(
                first, second,
                "{quality:?} at {scale} is the same buffer twice"
            );
        }
    }

    // ARQ-42, AC7: `transparent` paints no background and keeps the alpha;
    // without it the page background goes under the content (FD-55).
    #[test]
    fn arq_42_a_transparent_render_paints_no_background() {
        let document = document(&[(100, 80)]);
        let page = document.pages()[0].id;
        for (transparent, painted) in [(true, false), (false, true)] {
            let (renderer, record) = renderer(
                document.clone(),
                RenderQuality::Preview,
                RecordingBackend::new(),
            );
            let region = IRect::new(0, 0, 100, 80);
            renderer
                .render(
                    &request(page, region, 1.0, transparent),
                    &CancelToken::new(),
                )
                .expect("the page renders");
            let (targets, ran, _, _) = seen(&record);
            assert_eq!(
                targets[0].format.channels(),
                4,
                "the surface of a page carries alpha (02 §6.2)"
            );
            let ops = &ran[0].ops[0];
            let background = OpShape::FillPath {
                color: PAGE_BACKGROUND,
                transform: Affine::IDENTITY,
            };
            assert_eq!(
                ops.first().cloned() == Some(background),
                painted,
                "transparent={transparent} paints the background: {painted}"
            );
        }
    }
}
