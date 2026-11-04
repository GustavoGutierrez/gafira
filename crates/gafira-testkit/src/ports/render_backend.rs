//! The fake render backend of ARQ-112: the port double a test injects in
//! place of a real backend (02 §4.6).

use gafira_doc::contract::CancelToken;
use gafira_geom::IRect;
use gafira_ports::common::Bytes;
use gafira_ports::render::{
    BackendCaps, Deadline, FrameCx, MemoryUsage, RenderBackend, RenderError, RenderPlan,
    RenderReport, TargetDesc, TileCache,
};
use gafira_raster::ImageBuf;

/// The fake render backend. A stub: the render stream fills it.
pub struct FakeRenderBackend;

impl RenderBackend for FakeRenderBackend {
    /// Not filled yet.
    fn caps(&self) -> BackendCaps {
        panic!("stub: the render backend double is not filled yet")
    }

    /// Not filled yet.
    fn begin_frame(&mut self, _target: &TargetDesc) -> Result<FrameCx, RenderError> {
        panic!("stub: the render backend double is not filled yet")
    }

    /// Not filled yet.
    fn execute(
        &mut self,
        _cx: &mut FrameCx,
        _plan: &RenderPlan,
        _cache: &mut dyn TileCache,
        _deadline: Deadline,
        _cancel: &CancelToken,
    ) -> Result<RenderReport, RenderError> {
        panic!("stub: the render backend double is not filled yet")
    }

    /// Not filled yet.
    fn read_back(&mut self, _cx: &FrameCx, _region: IRect) -> Result<ImageBuf, RenderError> {
        panic!("stub: the render backend double is not filled yet")
    }

    /// Not filled yet.
    fn memory_usage(&self) -> MemoryUsage {
        panic!("stub: the render backend double is not filled yet")
    }

    /// Not filled yet.
    fn trim(&mut self, _budget: Bytes) {
        panic!("stub: the render backend double is not filled yet")
    }
}
