//! The CPU render backend: the reference implementation of the port (ARQ-113).
//!
//! `CB1` renders one op — `FillPath`, in both formats — through the tile loop
//! of 02 §6.2. The layer stack (`PushLayer`/`PopLayer`, `CB2`), images
//! (`CB4`) and masks, effects and transitions (`CB5`) arrive in their own
//! deliveries, and until then an op of those kinds is **reported**, never
//! dropped: a backend that silently flattened a group would draw the wrong
//! page and say the tile was done.

use std::sync::Arc;

use gafira_color::Color;
use gafira_doc::BlendMode;
use gafira_doc::contract::CancelToken;
use gafira_geom::{Affine, IRect};
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, Deadline, FrameCx, FrameId, MemoryUsage, RenderBackend, RenderError,
    RenderOp, RenderPlan, RenderReport, TargetDesc, TileCache, TileKey, TileTask,
};
use gafira_raster::{ImageBuf, PixelFormat, blend_f32};
use kurbo::BezPath;

use crate::coverage;
use crate::surface::{self, Surface};

/// The CPU rasterizer (02 §2.1, §6.2).
///
/// Single-threaded and clockless (D8, ARQ-02): the deadline belongs to the
/// caller's clock and the cancellation arrives as a [`CancelToken`].
#[derive(Default)]
pub struct CpuBackend {
    /// The open frame's surface, `None` between frames and after a trim.
    surface: Option<Surface>,
    /// The frame the surface belongs to.
    frame: Option<FrameId>,
    /// The id of the next `begin_frame`.
    next_frame: u64,
}

impl CpuBackend {
    /// A backend with no open frame.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The surface of `cx`, or [`RenderError::DeviceLost`] when it is gone.
    fn surface_of(&mut self, cx: &FrameCx) -> Result<&mut Surface, RenderError> {
        match (self.frame, self.surface.as_mut()) {
            (Some(id), Some(surface)) if id == cx.id => Ok(surface),
            _ => Err(RenderError::DeviceLost),
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
        Ok(FrameCx { id })
    }

    fn execute(
        &mut self,
        cx: &mut FrameCx,
        plan: &RenderPlan,
        cache: &mut dyn TileCache,
        _deadline: Deadline,
        _cancel: &CancelToken,
    ) -> Result<RenderReport, RenderError> {
        let planned = plan.tasks.len();
        let format = self.surface_of(cx)?.format();
        for task in &plan.tasks {
            let size = (task.region.width(), task.region.height());
            let tile = match cached_tile(cache, task.key, size, format) {
                Some(tile) => tile,
                None => {
                    let tile = render_tile(task)?;
                    let encoded = surface::encode_tile(&tile, size, format)?;
                    cache.put(task.key, Arc::new(encoded));
                    tile
                }
            };
            self.surface_of(cx)?.write(task.region, &tile, size);
        }
        Ok(RenderReport {
            done: planned,
            left: 0,
        })
    }

    fn read_back(&mut self, cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError> {
        self.surface_of(cx)?.read(region)
    }

    fn memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            host: Bytes(self.surface.as_ref().map_or(0, Surface::bytes)),
            device: Bytes(0),
        }
    }

    fn trim(&mut self, budget: Bytes) {
        if self.memory_usage().host > budget {
            self.surface = None;
            self.frame = None;
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

/// Renders one task into a tile-local premultiplied `f32` buffer.
///
/// The buffer is the tile's own content: the ops are composited in plan order
/// on a transparent backdrop, and the loop writes the result at the task's
/// region (D4, D6).
fn render_tile(task: &TileTask) -> Result<Vec<f32>, RenderError> {
    let size = (task.region.width(), task.region.height());
    let count = usize::try_from(u64::from(size.0) * u64::from(size.1))
        .ok()
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(RenderError::OutOfMemory)?;
    let mut pixels = vec![0.0f32; count];
    if count == 0 {
        return Ok(pixels);
    }
    for op in &task.ops {
        match op {
            RenderOp::FillPath {
                path,
                color,
                transform,
            } => paint_fill(&mut pixels, size, task.region, path, *color, *transform)?,
            RenderOp::PushLayer
            | RenderOp::PopLayer { .. }
            | RenderOp::DrawImage { .. }
            | RenderOp::ApplyMask { .. }
            | RenderOp::ApplyEffect { .. }
            | RenderOp::ApplyTransition { .. } => return Err(not_in_this_delivery()),
        }
    }
    Ok(pixels)
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
/// `PushLayer`/`PopLayer` are `CB2`, `DrawImage` is `CB4`, and the mask, the
/// effect and the transition are `CB5`. The port's error set has no
/// `Unsupported`, so the adapter reports the op as blocked rather than
/// dropping it silently.
fn not_in_this_delivery() -> RenderError {
    RenderError::DriverBlocked
}
