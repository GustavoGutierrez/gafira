//! The backend selector: which engine renders the frame, and the degradation
//! to the CPU when the GPU is missing or lost (ARQ-22).
//!
//! The selector picks the GPU when it is given one and the CPU otherwise. On
//! a lost device it drops the GPU and renders on the CPU for the rest of the
//! session, and it reports the switch. It never creates a backend: the
//! composition root hands them in (P10 of the layer-3 design), so the crate
//! stays pure in the sense of ARQ-02 (ADR 0016).

use gafira_doc::contract::CancelToken;
use gafira_geom::IRect;
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, Deadline, FrameCx, MemoryUsage, RenderBackend, RenderError,
    RenderPlan, RenderReport, TargetDesc, TileCache,
};
use gafira_raster::ImageBuf;

/// Why a session left the GPU for the CPU (ARQ-22).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Degradation {
    /// No GPU backend was given when the session started (ARQ-22).
    AdapterMissing,
    /// The device was lost in the middle of a frame (ARQ-22, 02 §6.4).
    DeviceLost,
}

/// The backend a frame renders with, and the degradation to the CPU (ARQ-22).
///
/// The selector picks the GPU when it is given one and the CPU otherwise. On
/// [`RenderError::DeviceLost`] it drops the GPU and renders on the CPU for the
/// rest of the session, and [`degradation`](Self::degradation) reports the
/// switch for as long as the session lasts. The selector never creates a
/// backend: the composition root hands them in (P10).
pub struct BackendSelector {
    gpu: Option<Box<dyn RenderBackend>>,
    cpu: Box<dyn RenderBackend>,
    degradation: Option<Degradation>,
}

impl BackendSelector {
    /// Picks the GPU when it is given one; with none, the session starts on
    /// the CPU and the degradation is reported (ARQ-22).
    #[must_use]
    pub fn new(gpu: Option<Box<dyn RenderBackend>>, cpu: Box<dyn RenderBackend>) -> Self {
        let degradation = gpu.is_none().then_some(Degradation::AdapterMissing);
        Self {
            gpu,
            cpu,
            degradation,
        }
    }

    /// The kind of the backend in use.
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        if self.gpu.is_some() {
            BackendKind::Gpu
        } else {
            BackendKind::Cpu
        }
    }

    /// The degradation of the session, if there was one (ARQ-22).
    #[must_use]
    pub fn degradation(&self) -> Option<Degradation> {
        self.degradation
    }

    /// The backend in use.
    fn active(&self) -> &dyn RenderBackend {
        match &self.gpu {
            Some(gpu) => gpu.as_ref(),
            None => self.cpu.as_ref(),
        }
    }

    /// The backend in use.
    fn active_mut(&mut self) -> &mut dyn RenderBackend {
        match self.gpu.as_mut() {
            Some(gpu) => gpu.as_mut(),
            None => self.cpu.as_mut(),
        }
    }

    /// Drops the GPU for the rest of the session after a device loss (ARQ-22).
    fn degrade(&mut self, error: &RenderError) {
        if matches!(error, RenderError::DeviceLost) && self.gpu.take().is_some() {
            self.degradation = Some(Degradation::DeviceLost);
        }
    }
}

impl RenderBackend for BackendSelector {
    fn caps(&self) -> BackendCaps {
        self.active().caps()
    }

    fn begin_frame(&mut self, target: &TargetDesc) -> Result<FrameCx, RenderError> {
        let result = self.active_mut().begin_frame(target);
        if let Err(error) = &result {
            self.degrade(error);
        }
        result
    }

    fn execute(
        &mut self,
        cx: &mut FrameCx,
        plan: &RenderPlan,
        cache: &mut dyn TileCache,
        deadline: Deadline,
        cancel: &CancelToken,
    ) -> Result<RenderReport, RenderError> {
        let result = self.active_mut().execute(cx, plan, cache, deadline, cancel);
        if let Err(error) = &result {
            self.degrade(error);
        }
        result
    }

    fn read_back(&mut self, cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError> {
        self.active_mut().read_back(cx, region)
    }

    fn memory_usage(&self) -> MemoryUsage {
        self.active().memory_usage()
    }

    fn trim(&mut self, budget: Bytes) {
        self.active_mut().trim(budget);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gafira_color::{Color, Rgba};
    use gafira_doc::contract::NoProgress;
    use gafira_doc::visit::{PageView, WalkOrder};
    use gafira_doc::{
        ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry,
        ShapeNode,
    };
    use gafira_geom::{Length, Unit};
    use gafira_ports::render::{FrameId, RenderQuality};
    use gafira_raster::{PixelBuf, PixelFormat};
    use gafira_time::Flicks;

    use super::*;
    use crate::cache::ContentTileCache;
    use crate::graph::{GraphParams, RenderGraph, RenderGraphBuilder};
    use crate::planner::{PageExtent, Planner, TILE_SIZE, Viewport};
    use crate::pool::InlinePool;

    /// The surface of a 256×256 `Rgba8` frame.
    fn target() -> TargetDesc {
        TargetDesc {
            width: TILE_SIZE,
            height: TILE_SIZE,
            format: PixelFormat::Rgba8,
        }
    }

    /// A transparent image of one pixel.
    fn one_pixel() -> Result<ImageBuf, RenderError> {
        ImageBuf::new(1, 1, PixelBuf::Rgba8(vec![0; 4])).map_err(RenderError::Buffer)
    }

    /// A GPU double that loses the device on the second task of the plan it is
    /// given, after the first one was consumed (ARQ-22, SP11).
    struct LostDevice;

    impl RenderBackend for LostDevice {
        fn caps(&self) -> BackendCaps {
            BackendCaps {
                kind: BackendKind::Gpu,
                max_texture: 8192,
                formats: vec![PixelFormat::Rgba8, PixelFormat::RgbaF32],
                gpu_effects: true,
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
            let mut done = 0;
            for _task in &plan.tasks {
                cancel.check().map_err(RenderError::Cancelled)?;
                if done == 1 {
                    return Err(RenderError::DeviceLost);
                }
                done += 1;
            }
            Ok(RenderReport { done, left: 0 })
        }

        fn read_back(&mut self, _cx: &FrameCx, _region: IRect) -> Result<ImageBuf, RenderError> {
            one_pixel()
        }

        fn memory_usage(&self) -> MemoryUsage {
            MemoryUsage {
                host: Bytes(0),
                device: Bytes(0),
            }
        }

        fn trim(&mut self, _budget: Bytes) {}
    }

    /// A CPU double that counts the tiles it rendered.
    #[derive(Default)]
    struct CountingBackend {
        tiles: Arc<AtomicUsize>,
    }

    impl RenderBackend for CountingBackend {
        fn caps(&self) -> BackendCaps {
            BackendCaps {
                kind: BackendKind::Cpu,
                max_texture: 8192,
                formats: vec![PixelFormat::Rgba8],
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
            let mut done = 0;
            for _task in &plan.tasks {
                cancel.check().map_err(RenderError::Cancelled)?;
                done += 1;
            }
            self.tiles.fetch_add(done, Ordering::Relaxed);
            Ok(RenderReport { done, left: 0 })
        }

        fn read_back(&mut self, _cx: &FrameCx, _region: IRect) -> Result<ImageBuf, RenderError> {
            one_pixel()
        }

        fn memory_usage(&self) -> MemoryUsage {
            MemoryUsage {
                host: Bytes(0),
                device: Bytes(0),
            }
        }

        fn trim(&mut self, _budget: Bytes) {}
    }

    /// A page of the given size, in pixels, with one filled shape.
    fn new_page(width: u32, height: u32) -> Page {
        let mut page = Page::new(
            PageId::new(1),
            "P".to_string(),
            PageSize {
                width: Length::new(f64::from(width), Unit::Px),
                height: Length::new(f64::from(height), Unit::Px),
            },
        );
        let base = ElementBase::new(NodeId::new(1), Placement::new(0.0, 0.0, 10.0, 10.0));
        let kind = NodeKind::Shape(ShapeNode {
            geometry: ShapeGeometry::Rect,
            fill: Some(Color::Srgb(Rgba::new(0.2, 0.4, 0.6, 1.0))),
        });
        page.nodes
            .insert(NodeId::new(1), Arc::new(Node::new(base, kind)));
        page.roots = vec![NodeId::new(1)];
        page
    }

    /// The graph of the page, for the preview.
    fn graph_of(page: &Page) -> RenderGraph {
        let view = PageView::new(page);
        RenderGraphBuilder::new(
            &view,
            GraphParams {
                quality: RenderQuality::Preview,
                lod: 0,
                t: Flicks::from_ticks(0),
            },
            InlinePool,
        )
        .build(WalkOrder::PreOrder, &CancelToken::new(), &NoProgress)
        .expect("the page builds")
    }

    /// The plan of the page at the origin, from its graph.
    fn plan_of(graph: &RenderGraph, size: u32) -> RenderPlan {
        Planner::plan(
            graph,
            PageExtent {
                width: size,
                height: size,
            },
            &Viewport {
                rect: IRect::new(0, 0, size as i32, size as i32),
                zoom: 1.0,
                focus: None,
            },
            &CancelToken::new(),
            &NoProgress,
        )
        .expect("the plan builds")
        .into_ports_plan()
    }

    // ARQ-22: a device lost in the middle of a frame degrades the session to
    // the CPU, reports it, and leaves the document untouched; the next frames
    // render on the CPU.
    #[test]
    fn arq_22_a_lost_device_degrades_the_session_to_the_cpu() {
        let page = new_page(400, 400);
        let graph = graph_of(&page);
        let key = graph.key_of(NodeId::new(1));
        let plan = plan_of(&graph, 400);
        let mut selector = BackendSelector::new(
            Some(Box::new(LostDevice)),
            Box::new(CountingBackend::default()),
        );
        assert_eq!(selector.kind(), BackendKind::Gpu);
        assert_eq!(selector.degradation(), None);

        let mut cache = ContentTileCache::new(Bytes(0));
        let cancel = CancelToken::new();
        let target = target();
        let mut cx = selector
            .begin_frame(&target)
            .expect("the GPU opens the frame");
        // The device is lost on the second task of the plan (ARQ-22, SP11).
        let lost = selector.execute(&mut cx, &plan, &mut cache, Deadline::Unbounded, &cancel);
        assert!(
            matches!(lost, Err(RenderError::DeviceLost)),
            "the frame returns the loss: {lost:?}"
        );
        // The session is on the CPU for good and the switch is reported.
        assert_eq!(selector.degradation(), Some(Degradation::DeviceLost));
        assert_eq!(selector.kind(), BackendKind::Cpu);
        // The document and its graph survived the failure.
        assert_eq!(graph.key_of(NodeId::new(1)), key, "the document is intact");
        assert_eq!(page.nodes.len(), 1, "the document is intact");
        // The next frames render on the CPU, and the degradation lasts.
        for _ in 0..2 {
            let mut cx = selector
                .begin_frame(&target)
                .expect("the CPU opens the frame");
            let next = selector
                .execute(&mut cx, &plan, &mut cache, Deadline::Unbounded, &cancel)
                .expect("the CPU renders the same plan");
            assert_eq!(next.left, 0);
            assert_eq!(selector.degradation(), Some(Degradation::DeviceLost));
            assert_eq!(selector.kind(), BackendKind::Cpu);
        }
    }
}
