//! The frame loop: the backend selector with its degradation to the CPU
//! (ARQ-22) and the progressive refinement of 02 §6.2, with the cancellation
//! and the progress of 02 §6.3 (ARQ-43).
//!
//! The loop drives one task of the plan per [`RenderBackend::execute`] call, so
//! it checks the cancel token and reports the progress at every tile boundary,
//! and it stops at the deadline of the frame. A frame that does not finish is
//! not an error (02 §6.2): the loop returns the tiles it left and the mip the
//! caller presents, and the next frame refines the plan. The caller never
//! waits.
//!
//! The crate stays pure in the sense of ARQ-02 (ADR 0016): the instant comes
//! from the injected [`Clock`](gafira_ports::system::Clock) and the backends
//! come from the caller. The selector never creates a backend (P10 of the
//! layer-3 design).

use std::time::Duration;

use gafira_doc::contract::{CancelToken, Progress, checkpoint};
use gafira_geom::IRect;
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, Deadline, FrameCx, MemoryUsage, RenderBackend, RenderError,
    RenderPlan, RenderReport, TargetDesc, TileCache,
};
use gafira_ports::system::Clock;
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

/// What a frame presents and what is left for the next one (02 §6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameOutcome {
    /// The tiles the frame finished, and the ones it left.
    pub report: RenderReport,
    /// The level of the mip the frame presents: the level of the plan when it
    /// finished, and the best mip available when the deadline passed
    /// (02 §6.2).
    pub mip: u8,
    /// True when the plan still has tiles for the next frame (02 §6.2).
    pub refine: bool,
}

/// True when `now` is at or past `deadline` (02 §6.2): the equality counts as
/// passed, so a frame that ends exactly at its deadline is incomplete, and
/// [`Deadline::Unbounded`] never passes.
///
/// The port owns the arithmetic ([`Deadline::has_passed`]); the loop enforces
/// it before every tile, so the rule has a named seam next to the loop that
/// applies it.
#[must_use]
fn deadline_passed(deadline: Deadline, now: Duration) -> bool {
    deadline.has_passed(now)
}

/// Runs one plan per frame: the deadline of the frame, the cancel token per
/// tile and the progress of the units done over the units planned (02 §6.2,
/// §6.3, ARQ-43).
///
/// The loop never waits for the backend: when the deadline passes it returns
/// the report with the tiles it left, and the caller presents
/// [`FrameOutcome::mip`] and refines on the next frame (02 §6.2).
pub struct FrameLoop {
    selector: BackendSelector,
    /// The level of the last plan that finished: the best mip available.
    presented: Option<u8>,
    /// The surface the last frame opened, for the read back of
    /// [`FrameLoop::read_back`] (02 §4.6).
    surface: Option<FrameCx>,
}

/// What one frame needs from the caller (02 §6.2, §6.3).
pub struct FrameRequest<'a> {
    /// The injected clock the loop reads the deadline against (P9, ARQ-02).
    pub clock: &'a dyn Clock,
    /// The surface of the frame.
    pub target: TargetDesc,
    /// The deadline of the frame. [`Deadline::Unbounded`] never passes, so an
    /// export frame runs to the end (02 §6.2).
    pub deadline: Deadline,
    /// The cache the backend reads and writes.
    pub cache: &'a mut dyn TileCache,
    /// The token the loop checks once per frame and before every tile
    /// (02 §6.3).
    pub cancel: &'a CancelToken,
    /// Receives the units done over the units planned (ARQ-43).
    pub progress: &'a dyn Progress,
}

impl FrameLoop {
    /// A loop over the backends of the selector (P10).
    #[must_use]
    pub fn new(selector: BackendSelector) -> Self {
        Self {
            selector,
            presented: None,
            surface: None,
        }
    }

    /// The selector, for the caller that inspects the backend in use (P10).
    #[must_use]
    pub fn selector(&self) -> &BackendSelector {
        &self.selector
    }

    /// The kind of the backend in use.
    #[must_use]
    pub fn kind(&self) -> BackendKind {
        self.selector.kind()
    }

    /// The degradation of the session, if there was one (ARQ-22).
    #[must_use]
    pub fn degradation(&self) -> Option<Degradation> {
        self.selector.degradation()
    }

    /// The level of the last plan that finished: the best mip available.
    #[must_use]
    pub fn presented(&self) -> Option<u8> {
        self.presented
    }

    /// Renders `plan` until it ends or the request's deadline passes.
    ///
    /// A late frame is not an error (02 §6.2): the report carries the tiles it
    /// left, and the outcome presents the best mip available and asks for a
    /// refinement on the next frame. The cancel token is checked once per
    /// frame and before every tile, and the progress is the units done over
    /// the units planned (02 §6.3, ARQ-43).
    ///
    /// # Errors
    /// [`RenderError::Cancelled`] when the request's token is cancelled, and
    /// the error of the backend when it fails. A lost device moves the session
    /// to the CPU for good (ARQ-22).
    pub fn frame(
        &mut self,
        request: FrameRequest<'_>,
        plan: RenderPlan,
    ) -> Result<FrameOutcome, RenderError> {
        let FrameRequest {
            clock,
            target,
            deadline,
            cache,
            cancel,
            progress,
        } = request;
        let quality = plan.quality;
        let level = plan.tasks.first().map_or(0, |task| task.key.level);
        let planned = plan.tasks.len();
        let mut cx = self.selector.begin_frame(&target)?;
        // The surface stays open after the frame so a caller can read its
        // pixels back (02 §4.6, RE5).
        self.surface = Some(cx);
        let mut done = 0usize;
        // The cancel token is checked once per frame (02 §6.3).
        cancel.check().map_err(RenderError::Cancelled)?;
        for task in plan.tasks {
            // The deadline is cumulative and passes on equality (02 §6.2).
            if deadline_passed(deadline, clock.monotonic()) {
                break;
            }
            // The cancel token is checked again before every tile (02 §6.3).
            cancel.check().map_err(RenderError::Cancelled)?;
            let single = RenderPlan {
                quality,
                tasks: vec![task],
            };
            let report = self
                .selector
                .execute(&mut cx, &single, cache, deadline, cancel)?;
            // A backend that stops inside a tile reports it as left: the tile
            // is not done, and the next frame repeats it (02 §6.2).
            if !report.is_complete() {
                break;
            }
            done += 1;
            checkpoint(cancel, progress, fraction(done, planned))?;
        }
        let left = planned.saturating_sub(done);
        if left == 0 {
            self.presented = Some(level);
            if planned == 0 {
                checkpoint(cancel, progress, 1.0)?;
            }
        }
        Ok(FrameOutcome {
            report: RenderReport { done, left },
            mip: self.presented.unwrap_or(level),
            refine: left > 0,
        })
    }

    /// Reads the region of the surface the last frame opened (02 §4.6).
    ///
    /// The loop only schedules; a caller that needs the pixels asks for them
    /// here. `None` when no frame has run yet, because there is no surface to
    /// read back.
    #[must_use]
    pub fn read_back(&mut self, region: IRect) -> Option<Result<ImageBuf, RenderError>> {
        let cx = self.surface?;
        Some(self.selector.read_back(&cx, region))
    }
}

/// The units done over the units planned (ARQ-43). A plan with no tile is
/// complete by definition.
fn fraction(done: usize, planned: usize) -> f32 {
    if planned == 0 {
        1.0
    } else {
        done as f32 / planned as f32
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::Duration;

    use gafira_color::{Color, Rgba};
    use gafira_doc::contract::NoProgress;
    use gafira_doc::visit::{PageView, WalkOrder};
    use gafira_doc::{
        ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ShapeGeometry,
        ShapeNode,
    };
    use gafira_geom::{Length, Unit};
    use gafira_ports::common::UnixTime;
    use gafira_ports::render::{ContentKey, FrameId, Priority, RenderQuality, TileKey, TileTask};
    use gafira_raster::{PixelBuf, PixelFormat};
    use gafira_testkit::ports::render_backend::FakeRenderBackend;
    use gafira_testkit::progress::{ProgressLog, check_progress_contract};
    use gafira_time::Flicks;

    use super::*;
    use crate::budget::FrameBudget;
    use crate::cache::ContentTileCache;
    use crate::graph::{GraphParams, RenderGraph, RenderGraphBuilder};
    use crate::planner::{PageExtent, Planner, TILE_SIZE, Viewport};
    use crate::pool::InlinePool;

    /// A clock stuck at one instant; it never reads the system time (ARQ-02).
    struct FixedClock(Duration);

    impl Clock for FixedClock {
        fn now(&self) -> UnixTime {
            UnixTime(0)
        }

        fn monotonic(&self) -> Duration {
            self.0
        }
    }

    /// A clock whose `monotonic` answers the scripted instants in order and
    /// repeats the last one; it never reads the system time (ARQ-02).
    struct ScriptClock {
        reads: Mutex<Script>,
    }

    /// The instants a [`ScriptClock`] still has to answer.
    struct Script {
        values: Vec<Duration>,
        at: usize,
    }

    impl ScriptClock {
        /// A clock that answers `values`, in order.
        fn new(values: Vec<Duration>) -> Self {
            Self {
                reads: Mutex::new(Script { values, at: 0 }),
            }
        }
    }

    impl Clock for ScriptClock {
        fn now(&self) -> UnixTime {
            UnixTime(0)
        }

        fn monotonic(&self) -> Duration {
            let mut script = self.reads.lock().unwrap_or_else(PoisonError::into_inner);
            let value = script
                .values
                .get(script.at)
                .copied()
                .unwrap_or_else(|| script.values.last().copied().unwrap_or_default());
            script.at += 1;
            value
        }
    }

    /// The surface of a 256×256 `Rgba8` frame.
    fn target() -> TargetDesc {
        TargetDesc {
            width: TILE_SIZE,
            height: TILE_SIZE,
            origin: (0, 0),
            page: (TILE_SIZE, TILE_SIZE),
            format: PixelFormat::Rgba8,
        }
    }

    /// A plan of `count` tiles at `level`, in the priority order of 02 §6.3.
    fn plan(level: u8, count: u32) -> RenderPlan {
        RenderPlan {
            quality: RenderQuality::Preview,
            tasks: (0..count)
                .map(|column| TileTask {
                    key: TileKey {
                        content: ContentKey(1),
                        level,
                        column,
                        row: 0,
                    },
                    priority: Priority::Visible,
                    region: IRect::new(0, 0, 256, 256),
                    ops: Vec::new(),
                })
                .collect(),
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

    /// A CPU double that counts the tiles it rendered and ignores the cancel
    /// token on purpose, so the test proves the loop checks it at every tile
    /// (02 §6.3).
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
            _cancel: &CancelToken,
        ) -> Result<RenderReport, RenderError> {
            let done = plan.tasks.len();
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

    /// A backend that stops inside every tile and reports it as left, as a
    /// backend out of time does (02 §6.2).
    #[derive(Default)]
    struct StopsInsideTile {
        calls: Arc<AtomicUsize>,
    }

    impl RenderBackend for StopsInsideTile {
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
            _cancel: &CancelToken,
        ) -> Result<RenderReport, RenderError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(RenderReport {
                done: 0,
                left: plan.tasks.len(),
            })
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

    /// A progress that cancels the token on its first report (ARQ-43).
    struct CancelNow<'a>(&'a CancelToken);

    impl Progress for CancelNow<'_> {
        fn report(&self, _fraction: f32) {
            self.0.cancel();
        }
    }

    /// A loop that renders on the CPU.
    fn cpu_loop() -> FrameLoop {
        FrameLoop::new(BackendSelector::new(None, Box::new(FakeRenderBackend)))
    }

    /// The request of a frame over the fake target.
    fn request<'a>(
        clock: &'a dyn Clock,
        deadline: Deadline,
        cache: &'a mut ContentTileCache,
        cancel: &'a CancelToken,
        progress: &'a dyn Progress,
    ) -> FrameRequest<'a> {
        FrameRequest {
            clock,
            target: target(),
            deadline,
            cache,
            cancel,
            progress,
        }
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
            stroke: None,
            corner_radius: 0.0,
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

    // ARQ-22: a session that starts without a GPU adapter degrades to the
    // CPU, reports the missing adapter for the rest of the session, and
    // leaves the document untouched.
    #[test]
    fn arq_22_a_missing_adapter_degrades_the_session_to_the_cpu() {
        let page = new_page(400, 400);
        let graph = graph_of(&page);
        let key = graph.key_of(NodeId::new(1));
        let plan = plan_of(&graph, 400);
        let mut selector = BackendSelector::new(None, Box::new(CountingBackend::default()));

        assert_eq!(
            selector.kind(),
            BackendKind::Cpu,
            "a session without a GPU adapter starts on the CPU"
        );
        assert_eq!(
            selector.degradation(),
            Some(Degradation::AdapterMissing),
            "the missing adapter is reported"
        );

        let mut cache = ContentTileCache::new(Bytes(0));
        let cancel = CancelToken::new();
        let target = target();
        // The frames render on the CPU and the degradation lasts for the
        // rest of the session.
        for _ in 0..2 {
            let mut cx = selector
                .begin_frame(&target)
                .expect("the CPU opens the frame");
            let report = selector
                .execute(&mut cx, &plan, &mut cache, Deadline::Unbounded, &cancel)
                .expect("the CPU renders the same plan");
            assert_eq!(report.left, 0);
            assert_eq!(selector.kind(), BackendKind::Cpu);
            assert_eq!(
                selector.degradation(),
                Some(Degradation::AdapterMissing),
                "the degradation lasts for the rest of the session"
            );
            // The document and its graph survived.
            assert_eq!(graph.key_of(NodeId::new(1)), key, "the document is intact");
            assert_eq!(page.nodes.len(), 1, "the document is intact");
        }
    }

    // ARQ-41, 02 §6.2: a frame that ends exactly at the deadline is incomplete
    // with tiles left, one a nanosecond before is complete, and `Unbounded`
    // never passes.
    #[test]
    fn arq_41_the_left_tiles_are_reported_at_the_exact_deadline() {
        let deadline = FrameBudget::begin(&FixedClock(Duration::ZERO)).visible_deadline();
        // Before the second tile the clock reads exactly the deadline.
        let mut cache = ContentTileCache::new(Bytes(0));
        let at_deadline = ScriptClock::new(vec![Duration::ZERO, Duration::from_millis(14)]);
        let outcome = cpu_loop()
            .frame(
                request(
                    &at_deadline,
                    deadline,
                    &mut cache,
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(0, 2),
            )
            .expect("a late frame is a report, not an error");
        assert_eq!(outcome.report, RenderReport { done: 1, left: 1 });
        assert!(outcome.refine);
        // A nanosecond before the deadline the second tile runs.
        let mut cache = ContentTileCache::new(Bytes(0));
        let before = ScriptClock::new(vec![Duration::ZERO, Duration::from_nanos(13_999_999)]);
        let outcome = cpu_loop()
            .frame(
                request(
                    &before,
                    deadline,
                    &mut cache,
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(0, 2),
            )
            .expect("the frame renders");
        assert_eq!(outcome.report, RenderReport { done: 2, left: 0 });
        assert!(!outcome.refine);
        // `Unbounded` never passes: the frame finishes past 16 ms.
        let mut cache = ContentTileCache::new(Bytes(0));
        let past = ScriptClock::new(vec![Duration::from_millis(16), Duration::from_millis(16)]);
        let outcome = cpu_loop()
            .frame(
                request(
                    &past,
                    Deadline::Unbounded,
                    &mut cache,
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(0, 2),
            )
            .expect("an unbounded frame renders");
        assert_eq!(outcome.report, RenderReport { done: 2, left: 0 });
    }

    // ARQ-41, 02 §6.2: a late frame presents the best mip available, asks for
    // the refinement, and the next frame continues the plan.
    #[test]
    fn arq_41_a_late_frame_presents_the_best_mip_and_refines() {
        let deadline = Deadline::At(Duration::from_millis(14));
        let clock = FixedClock(Duration::ZERO);
        let mut cache = ContentTileCache::new(Bytes(0));
        let mut frame_loop = cpu_loop();
        // The level 0 finishes: it is the mip available.
        let first = frame_loop
            .frame(
                request(
                    &clock,
                    deadline,
                    &mut cache,
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(0, 2),
            )
            .expect("the frame renders");
        assert_eq!(first.report, RenderReport { done: 2, left: 0 });
        assert_eq!(first.mip, 0);
        assert!(!first.refine);
        // The level 1 does not finish: present the level 0 and refine.
        let late = ScriptClock::new(vec![Duration::ZERO, Duration::from_millis(14)]);
        let second = frame_loop
            .frame(
                request(
                    &late,
                    deadline,
                    &mut cache,
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(1, 2),
            )
            .expect("a late frame is a report");
        assert_eq!(second.report, RenderReport { done: 1, left: 1 });
        assert_eq!(second.mip, 0, "the best mip available is the last one done");
        assert!(second.refine);
        // A deadline already past returns at once with every tile left: the
        // caller never waits.
        let past = ScriptClock::new(vec![Duration::from_millis(14)]);
        let none = frame_loop
            .frame(
                request(
                    &past,
                    deadline,
                    &mut cache,
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(2, 3),
            )
            .expect("a late frame is a report");
        assert_eq!(none.report, RenderReport { done: 0, left: 3 });
        assert_eq!(none.mip, 0);
        assert!(none.refine);
        // The next frame continues the plan and finishes it: the mip advances.
        let third = frame_loop
            .frame(
                request(
                    &clock,
                    deadline,
                    &mut cache,
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(1, 2),
            )
            .expect("the refinement ends");
        assert_eq!(third.report, RenderReport { done: 2, left: 0 });
        assert_eq!(third.mip, 1);
        assert!(!third.refine);
    }

    // ARQ-41, 02 §6.2: a tile the backend leaves inside `execute` is not
    // counted as done, and the frame reports it as left instead of looping.
    #[test]
    fn arq_41_a_tile_the_backend_leaves_is_not_counted() {
        let backend = StopsInsideTile::default();
        let calls = Arc::clone(&backend.calls);
        let mut frame_loop = FrameLoop::new(BackendSelector::new(None, Box::new(backend)));
        let outcome = frame_loop
            .frame(
                request(
                    &FixedClock(Duration::ZERO),
                    Deadline::Unbounded,
                    &mut ContentTileCache::new(Bytes(0)),
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(0, 3),
            )
            .expect("a backend out of time is not an error");
        assert_eq!(outcome.report, RenderReport { done: 0, left: 3 });
        assert!(outcome.refine);
        assert_eq!(
            calls.load(Ordering::Relaxed),
            1,
            "the loop stops at the first tile it cannot finish"
        );
    }

    // ARQ-43: the progress fraction is the units done over the units planned.
    #[test]
    fn arq_43_the_progress_is_the_units_done_over_the_units_planned() {
        let log = ProgressLog::new();
        let outcome = cpu_loop()
            .frame(
                request(
                    &FixedClock(Duration::ZERO),
                    Deadline::Unbounded,
                    &mut ContentTileCache::new(Bytes(0)),
                    &CancelToken::new(),
                    &log,
                ),
                plan(0, 4),
            )
            .expect("the frame renders");
        assert_eq!(outcome.report, RenderReport { done: 4, left: 0 });
        assert_eq!(log.fractions(), vec![0.25, 0.5, 0.75, 1.0]);
        // An empty plan is complete and reports its completion once.
        let empty = ProgressLog::new();
        cpu_loop()
            .frame(
                request(
                    &FixedClock(Duration::ZERO),
                    Deadline::Unbounded,
                    &mut ContentTileCache::new(Bytes(0)),
                    &CancelToken::new(),
                    &empty,
                ),
                plan(0, 0),
            )
            .expect("an empty frame renders");
        assert_eq!(empty.fractions(), vec![1.0]);
    }

    // ARQ-43: the frame follows the cancellation contract of ARQ-112 at every
    // tile boundary.
    #[test]
    fn arq_43_the_frame_checks_the_cancel_per_tile() {
        let result =
            check_progress_contract(0.25, |token, progress| -> Result<bool, RenderError> {
                let outcome = cpu_loop().frame(
                    request(
                        &FixedClock(Duration::ZERO),
                        Deadline::Unbounded,
                        &mut ContentTileCache::new(Bytes(0)),
                        token,
                        progress,
                    ),
                    plan(0, 4),
                );
                match outcome {
                    Ok(_) => Ok(false),
                    Err(RenderError::Cancelled(_)) => Ok(true),
                    Err(error) => Err(error),
                }
            });
        match result {
            Ok(Ok(())) => {}
            other => panic!("the progress contract fails: {other:?}"),
        }
    }

    // ARQ-43: the cancel stops the frame between tiles instead of finishing
    // the plan.
    #[test]
    fn arq_43_the_cancel_stops_the_frame_between_tiles() {
        let backend = CountingBackend::default();
        let tiles = Arc::clone(&backend.tiles);
        let mut frame_loop = FrameLoop::new(BackendSelector::new(None, Box::new(backend)));
        let cancel = CancelToken::new();
        let result = frame_loop.frame(
            request(
                &FixedClock(Duration::ZERO),
                Deadline::Unbounded,
                &mut ContentTileCache::new(Bytes(0)),
                &cancel,
                &CancelNow(&cancel),
            ),
            plan(0, 4),
        );
        assert!(
            matches!(result, Err(RenderError::Cancelled(_))),
            "the frame returns the cancel: {result:?}"
        );
        assert_eq!(
            tiles.load(Ordering::Relaxed),
            1,
            "the frame stops at the tile after the cancel"
        );
    }

    // ARQ-02: the same input takes the same decisions, frame after frame.
    #[test]
    fn arq_02_the_same_frame_takes_the_same_decisions() {
        let deadline = Deadline::At(Duration::from_millis(14));
        let run = || {
            let clock = ScriptClock::new(vec![Duration::ZERO, Duration::from_millis(14)]);
            cpu_loop()
                .frame(
                    request(
                        &clock,
                        deadline,
                        &mut ContentTileCache::new(Bytes(0)),
                        &CancelToken::new(),
                        &NoProgress,
                    ),
                    plan(0, 3),
                )
                .expect("a late frame is a report")
        };
        assert_eq!(run(), run());
    }

    // ARQ-42, 02 §4.6: the surface of the last frame reads back, and a loop
    // that ran no frame has none to answer.
    #[test]
    fn arq_42_the_surface_of_the_last_frame_reads_back() {
        let mut loop_ = cpu_loop();
        let region = IRect::new(0, 0, TILE_SIZE as i32, TILE_SIZE as i32);
        assert!(
            loop_.read_back(region).is_none(),
            "no frame ran, so there is no surface"
        );

        let outcome = loop_
            .frame(
                request(
                    &FixedClock(Duration::ZERO),
                    Deadline::Unbounded,
                    &mut ContentTileCache::new(Bytes(0)),
                    &CancelToken::new(),
                    &NoProgress,
                ),
                plan(0, 1),
            )
            .expect("the frame renders");
        assert_eq!(outcome.report, RenderReport { done: 1, left: 0 });

        let image = loop_
            .read_back(region)
            .expect("the frame opened a surface")
            .expect("the region reads back");
        assert_eq!((image.width(), image.height()), (TILE_SIZE, TILE_SIZE));
    }
}
