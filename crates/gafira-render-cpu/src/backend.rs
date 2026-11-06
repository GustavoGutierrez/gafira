//! The CPU render backend: the reference implementation of the port (ARQ-113).
//!
//! `CB1` renders `FillPath` in both formats through the tile loop of 02 §6.2,
//! and `CB2` composites it through the layer stack of 02 §6.1: `PushLayer`
//! opens a tile-sized buffer, the ops inside it are drawn there, and `PopLayer`
//! composites that buffer onto its parent with the pop's blend and opacity.
//! `CB4` resamples `DrawImage` into the same buffer with a bilinear filter
//! ([`crate::image`]). `CB5` masks the open layer with an 8-bit coverage
//! ([`crate::layer`]), grows the tile by an effect's input region and runs the
//! effect once over it ([`crate::effect`]), and blends the layer with its
//! background through a transition. An op of a kind this delivery does not
//! render is **reported**, never dropped: a backend that silently flattened a
//! group would draw the wrong page and say the tile was done.

use std::sync::Arc;

use gafira_color::Color;
use gafira_doc::BlendMode;
use gafira_doc::contract::{CancelToken, NoProgress, Progress, checkpoint};
use gafira_geom::{Affine, IRect};
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, Deadline, FrameCx, FrameId, MemoryUsage, RenderBackend, RenderError,
    RenderOp, RenderPlan, RenderReport, TargetDesc, TileCache, TileKey, TileTask,
};
use gafira_raster::{ImageBuf, PixelFormat, blend_f32};
use kurbo::BezPath;

use crate::coverage;
use crate::effect;
use crate::image as sampling;
use crate::layer::Layers;
use crate::surface::{self, Surface};

/// The CPU rasterizer (02 §2.1, §6.2).
///
/// Single-threaded and clockless (D8, ARQ-02): the deadline belongs to the
/// caller's clock and the cancellation arrives as a [`CancelToken`].
pub struct CpuBackend {
    /// The open frame's surface, `None` between frames and after a trim.
    surface: Option<Surface>,
    /// The frame the surface belongs to.
    frame: Option<FrameId>,
    /// The id of the next `begin_frame`.
    next_frame: u64,
    /// Where `execute` reports the fraction of the plan it finished (ARQ-139).
    progress: Arc<dyn Progress>,
    /// The last fraction reported in this frame, so it never decreases.
    last_fraction: f32,
    /// The bytes the caller's cache held at the last observation (ARQ-143).
    cache_bytes: u64,
    /// The layers of the tile being drawn, and the buffers their pops returned.
    ///
    /// The pool lives on the backend, so a pop's buffer is reused by the next
    /// layer of the same tile and by the tiles and frames that follow; `trim`
    /// is the only thing that drops it (ARQ-143).
    layers: Layers,
}

impl CpuBackend {
    /// A backend with no open frame that reports no progress.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A backend that reports the fraction of the plan it finished to
    /// `progress` (ARQ-139).
    ///
    /// The port's `execute` takes no `Progress`, so the sink is bound to the
    /// backend; [`NoProgress`] is the default.
    #[must_use]
    pub fn with_progress(mut self, progress: Arc<dyn Progress>) -> Self {
        self.progress = progress;
        self
    }

    /// Runs the plan's tasks through the cache and the surface.
    fn run(
        &mut self,
        cx: &FrameCx,
        plan: &RenderPlan,
        cache: &mut dyn TileCache,
        cancel: &CancelToken,
    ) -> Result<RenderReport, RenderError> {
        let planned = plan.tasks.len();
        let format = self.surface_of(cx)?.format();
        // A plan with nothing to do is complete by definition (ARQ-139).
        if planned == 0 {
            return self.finish(cancel);
        }
        for (index, task) in plan.tasks.iter().enumerate() {
            // The token is checked before every tile (02 §6.3, ARQ-139).
            cancel.check()?;
            let size = (task.region.width(), task.region.height());
            let page = self.surface_of(cx)?.page_rect();
            let tile = match cached_tile(cache, task.key, size, format) {
                Some(tile) => tile,
                None => {
                    let tile = render_tile(task, page, &mut self.layers, cancel)?;
                    let encoded = surface::encode_tile(&tile, size, format)?;
                    cache.put(task.key, Arc::new(encoded));
                    tile
                }
            };
            self.surface_of(cx)?.write(task.region, &tile, size);
            // The fraction is the tiles done over the tiles planned, and it
            // reaches `1.0` only with the last tile of the plan (ARQ-139).
            self.report(cancel, (index + 1, planned))?;
        }
        Ok(RenderReport {
            done: planned,
            left: 0,
        })
    }

    /// Reports the fraction of `(done, planned)`, never below the last one.
    fn report(
        &mut self,
        cancel: &CancelToken,
        (done, planned): (usize, usize),
    ) -> Result<(), RenderError> {
        let planned = planned.max(1);
        let fraction = (done as f32 / planned as f32).clamp(0.0, 1.0);
        self.last_fraction = self.last_fraction.max(fraction);
        checkpoint(cancel, self.progress.as_ref(), self.last_fraction)?;
        Ok(())
    }

    /// Reports `1.0` for a plan that ended without a tile.
    fn finish(&mut self, cancel: &CancelToken) -> Result<RenderReport, RenderError> {
        self.last_fraction = 1.0;
        checkpoint(cancel, self.progress.as_ref(), 1.0)?;
        Ok(RenderReport { done: 0, left: 0 })
    }

    /// The surface of `cx`, or [`RenderError::DeviceLost`] when it is gone.
    fn surface_of(&mut self, cx: &FrameCx) -> Result<&mut Surface, RenderError> {
        match (self.frame, self.surface.as_mut()) {
            (Some(id), Some(surface)) if id == cx.id => Ok(surface),
            _ => Err(RenderError::DeviceLost),
        }
    }
}

impl Default for CpuBackend {
    fn default() -> Self {
        Self {
            surface: None,
            frame: None,
            next_frame: 0,
            progress: Arc::new(NoProgress),
            last_fraction: 0.0,
            cache_bytes: 0,
            layers: Layers::new(),
        }
    }
}

impl RenderBackend for CpuBackend {
    fn caps(&self) -> BackendCaps {
        BackendCaps {
            kind: BackendKind::Cpu,
            max_texture: crate::surface::MAX_SIDE,
            formats: vec![PixelFormat::Rgba8, PixelFormat::RgbaF32],
            gpu_effects: false,
        }
    }

    fn begin_frame(&mut self, target: &TargetDesc) -> Result<FrameCx, RenderError> {
        let surface = Surface::new(target)?;
        let id = FrameId(self.next_frame);
        self.next_frame = self.next_frame.saturating_add(1);
        self.frame = Some(id);
        self.surface = Some(surface);
        self.last_fraction = 0.0;
        Ok(FrameCx { id })
    }

    fn execute(
        &mut self,
        cx: &mut FrameCx,
        plan: &RenderPlan,
        cache: &mut dyn TileCache,
        _deadline: Deadline,
        cancel: &CancelToken,
    ) -> Result<RenderReport, RenderError> {
        // The deadline is the caller's: this backend reads no clock (D8,
        // ARQ-02), and `FrameLoop` checks it before every tile.
        let outcome = self.run(cx, plan, cache, cancel);
        // The cache belongs to the caller, so its size is observed, not owned.
        self.cache_bytes = cache.bytes().0;
        match outcome {
            Ok(report) => Ok(report),
            Err(error) => {
                // A tile that failed may have left a layer open: the buffers go
                // back to the pool, the depth restarts with the next tile.
                self.layers.reset();
                if matches!(&error, RenderError::Cancelled(_)) {
                    // A cancelled frame is abandoned, so no partial surface is
                    // returned and the next frame opens its own (ARQ-139).
                    self.surface = None;
                    self.frame = None;
                }
                Err(error)
            }
        }
    }

    fn read_back(&mut self, cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError> {
        self.surface_of(cx)?.read(region)
    }

    fn memory_usage(&self) -> MemoryUsage {
        let surface = self.surface.as_ref().map_or(0, Surface::bytes);
        MemoryUsage {
            host: Bytes(surface + self.cache_bytes + self.layers.bytes()),
            device: Bytes(0),
        }
    }

    fn trim(&mut self, budget: Bytes) {
        // The surface and the layer pool are this backend's buffers; the
        // cache's bytes are the caller's and only its accounting is dropped
        // (ARQ-44, ARQ-143).
        if self.memory_usage().host > budget {
            self.surface = None;
            self.frame = None;
            self.cache_bytes = 0;
            self.layers = Layers::new();
        }
    }
}

/// The cached tile of `key`, when it is reusable as it stands.
///
/// `TileKey` carries no format (D5), so a caller may hand one cache to a
/// preview and an export frame: a tile whose sizes or format differ from this
/// frame is a miss, and the loop writes the right one back.
fn cached_tile(
    cache: &dyn TileCache,
    key: TileKey,
    size: (u32, u32),
    format: PixelFormat,
) -> Option<Vec<f32>> {
    let tile = cache.get(&key)?;
    if tile.width() != size.0 || tile.height() != size.1 || surface::format_of(&tile) != format {
        return None;
    }
    surface::decode_tile(&tile).ok()
}

/// Renders one task and answers the tile's own crop, premultiplied `f32`.
///
/// The ops are composited in plan order on a transparent backdrop and the loop
/// writes the result at the task's region (D4, D6). The buffer is the tile
/// **grown** by every effect's input region (`CB5`, ARQ-120): an effect has to
/// read pixels the tile does not hold, so the whole tile is drawn over that
/// larger rectangle and the tile keeps its own crop of it.
fn render_tile(
    task: &TileTask,
    page: IRect,
    layers: &mut Layers,
    cancel: &CancelToken,
) -> Result<Vec<f32>, RenderError> {
    let drawn = effect::grown_rect(task, page);
    let size = (drawn.width(), drawn.height());
    let count = usize::try_from(u64::from(size.0) * u64::from(size.1))
        .ok()
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(RenderError::OutOfMemory)?;
    // A tile that failed before left its layers open; their buffers are
    // reusable, the depth is not.
    layers.reset();
    if count == 0 {
        return Ok(Vec::new());
    }
    let mut pixels = vec![0.0f32; count];
    for op in &task.ops {
        match op {
            RenderOp::PushLayer => layers.push(count),
            RenderOp::PopLayer { blend, opacity } => layers.pop(&mut pixels, *blend, *opacity)?,
            RenderOp::FillPath {
                path,
                color,
                transform,
            } => {
                // D6: an op whose page-space bounds miss the drawn rectangle is
                // skipped before the path is converted. The box is the
                // transformed control-point box, so the cull never drops a
                // contributing op — and the rectangle is the grown one, because
                // an op outside the tile may still feed an effect's `roi`.
                if coverage::meets(coverage::page_bounds(path, *transform), drawn) {
                    paint_fill(
                        layers.target(&mut pixels),
                        size,
                        drawn,
                        path,
                        *color,
                        *transform,
                    )?;
                }
            }
            RenderOp::DrawImage { image, transform } => {
                // D6: an image whose page bounds miss the drawn rectangle
                // contributes no pixel, so it is dropped before a texel is
                // sampled.
                if coverage::meets(sampling::page_bounds(image, *transform), drawn) {
                    sampling::draw(image, *transform, layers.target(&mut pixels), size, drawn)?;
                }
            }
            RenderOp::ApplyMask { mask } => layers.apply_mask(mask, size)?,
            RenderOp::ApplyEffect {
                effect,
                params,
                scale,
            } => effect::run(
                effect.as_ref(),
                params,
                *scale,
                layers.target(&mut pixels),
                drawn,
                page,
                cancel,
            )?,
            RenderOp::ApplyTransition { .. } => return Err(not_in_this_delivery()),
        }
    }
    // The plan's stack is balanced (02 §6.1): a layer left open would have to
    // be flattened by a guess, so the tile is refused instead.
    if !layers.is_empty() {
        return Err(RenderError::DriverBlocked);
    }
    // The tile keeps its own crop of the grown buffer (ARQ-120).
    crop(&pixels, size, drawn, task.region)
}

/// The tile's own rectangle out of the (possibly grown) buffer.
///
/// The tile is what the cache stores and what the loop writes, so the grown
/// rectangle never leaves this function.
fn crop(
    pixels: &[f32],
    size: (u32, u32),
    drawn: IRect,
    tile: IRect,
) -> Result<Vec<f32>, RenderError> {
    let width = usize::try_from(tile.width()).unwrap_or(0);
    let height = usize::try_from(tile.height()).unwrap_or(0);
    let count = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(RenderError::OutOfMemory)?;
    let stride = usize::try_from(size.0).unwrap_or(0);
    let mut out = vec![0.0f32; count];
    // A tile that leaves the page keeps nothing there: the pixels it covers
    // outside the page are the page's transparent black.
    let clip = tile.intersect(drawn);
    for y in clip.y0..clip.y1 {
        for x in clip.x0..clip.x1 {
            let (Ok(row), Ok(column)) =
                (usize::try_from(y - drawn.y0), usize::try_from(x - drawn.x0))
            else {
                continue;
            };
            let (Ok(target_row), Ok(target_column)) =
                (usize::try_from(y - tile.y0), usize::try_from(x - tile.x0))
            else {
                continue;
            };
            let Some(source) = (row * stride + column).checked_mul(4) else {
                continue;
            };
            let Some(target) = (target_row * width + target_column).checked_mul(4) else {
                continue;
            };
            if source + 4 <= pixels.len() && target + 4 <= out.len() {
                out[target..target + 4].copy_from_slice(&pixels[source..source + 4]);
            }
        }
    }
    Ok(out)
}

/// Composites one fill into the tile (ARQ-117, ARQ-119).
///
/// The coverage is the **source alpha** of the fill: the colour is
/// premultiplied by the colour's alpha times the coverage, and `blend_f32`
/// composites it over the tile's backdrop. The blend is `gafira-raster`'s and
/// is never redefined here (ARQ-119).
fn paint_fill(
    pixels: &mut [f32],
    size: (u32, u32),
    region: IRect,
    path: &BezPath,
    color: Color,
    transform: Affine,
) -> Result<(), RenderError> {
    let Some(mask) = coverage::mask(path, transform, size, (region.x0, region.y0)) else {
        return Ok(());
    };
    let linear = color.to_linear_srgb();
    for (index, coverage) in mask.data().iter().enumerate() {
        if *coverage == 0 {
            continue;
        }
        let alpha = (linear.a * (f32::from(*coverage) / 255.0)).clamp(0.0, 1.0);
        let source = [linear.r * alpha, linear.g * alpha, linear.b * alpha, alpha];
        let at = index * 4;
        let backdrop = [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]];
        pixels[at..at + 4].copy_from_slice(&blend_f32(BlendMode::Normal, backdrop, source)?);
    }
    Ok(())
}

/// The error of an op this delivery does not render yet.
///
/// The mask, the effect and the transition are `CB5`. The port's error set has
/// no `Unsupported`, so the adapter reports the op as blocked rather than
/// dropping it silently.
fn not_in_this_delivery() -> RenderError {
    RenderError::DriverBlocked
}
