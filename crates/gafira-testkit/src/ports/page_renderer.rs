//! The fake page renderer of ARQ-112: the port double a test injects in place
//! of `gafira-render` (02 §4.5).
//!
//! The double answers a transparent image of the size asked for. The only
//! error it returns is the cancellation of the token.

use gafira_doc::contract::CancelToken;
use gafira_ports::export::{PageRenderRequest, PageRenderer};
use gafira_ports::render::RenderError;
use gafira_raster::{ImageBuf, PixelBuf};

/// The fake page renderer: it answers a transparent image of the size asked.
pub struct FakePageRenderer;

impl PageRenderer for FakePageRenderer {
    /// Answers a transparent image of the size of the region.
    fn render(
        &self,
        request: &PageRenderRequest,
        cancel: &CancelToken,
    ) -> Result<ImageBuf, RenderError> {
        cancel.check().map_err(RenderError::Cancelled)?;
        let width = request.region.width();
        let height = request.region.height();
        let len = (width as usize) * (height as usize) * 4;
        ImageBuf::new(width, height, PixelBuf::Rgba8(vec![0; len])).map_err(RenderError::Buffer)
    }
}
