//! The fake page renderer of ARQ-112: the port double a test injects in place
//! of `gafira-render` (02 §4.5).

use gafira_doc::contract::CancelToken;
use gafira_ports::export::{PageRenderRequest, PageRenderer};
use gafira_ports::render::RenderError;
use gafira_raster::ImageBuf;

/// The fake page renderer. A stub: the render stream fills it.
pub struct FakePageRenderer;

impl PageRenderer for FakePageRenderer {
    /// Not filled yet.
    fn render(
        &self,
        _request: &PageRenderRequest,
        _cancel: &CancelToken,
    ) -> Result<ImageBuf, RenderError> {
        panic!("stub: the page renderer double is not filled yet")
    }
}
