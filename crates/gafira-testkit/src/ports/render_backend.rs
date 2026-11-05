//! The fake render backend of ARQ-112: the port double a test injects in place
//! of a real engine (02 §4.6).
//!
//! The double is a CPU backend that runs the whole plan and answers a
//! transparent image. It never fails on its own: the only error it returns is
//! the cancellation of the token.

use gafira_doc::contract::CancelToken;
use gafira_geom::IRect;
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, BackendKind, Deadline, FrameCx, FrameId, MemoryUsage, RenderBackend, RenderError,
    RenderPlan, RenderReport, TargetDesc, TileCache,
};
use gafira_raster::{ImageBuf, PixelBuf, PixelFormat};

/// The fake render backend: a CPU backend that runs every task of the plan.
pub struct FakeRenderBackend;

impl RenderBackend for FakeRenderBackend {
    /// A CPU backend with the formats of §6.2 and no device effects.
    fn caps(&self) -> BackendCaps {
        BackendCaps {
            kind: BackendKind::Cpu,
            max_texture: 8192,
            formats: vec![PixelFormat::Rgba8, PixelFormat::RgbaF32],
            gpu_effects: false,
        }
    }

    /// Opens the frame. The double always answers the same handle.
    fn begin_frame(&mut self, _target: &TargetDesc) -> Result<FrameCx, RenderError> {
        Ok(FrameCx { id: FrameId(0) })
    }

    /// Runs every task of the plan, unless the token is cancelled.
    ///
    /// The deadline is not read: the double has no clock, and the caller that
    /// owns one decides when the plan is cut (ARQ-143).
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
        Ok(RenderReport { done, left: 0 })
    }

    /// Reads the region back as a transparent image.
    fn read_back(&mut self, _cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError> {
        transparent(region.width(), region.height())
    }

    /// The double holds nothing.
    fn memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            host: Bytes(0),
            device: Bytes(0),
        }
    }

    /// The double holds nothing to free.
    fn trim(&mut self, _budget: Bytes) {}
}

/// A transparent image of the given size.
fn transparent(width: u32, height: u32) -> Result<ImageBuf, RenderError> {
    let len = (width as usize) * (height as usize) * 4;
    ImageBuf::new(width, height, PixelBuf::Rgba8(vec![0; len])).map_err(RenderError::Buffer)
}
