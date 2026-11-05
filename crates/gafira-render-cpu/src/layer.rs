//! The layer the ops of a tile are drawn into (`CB2`, 02 §6.1).
//!
//! `PushLayer` opens a buffer the size of the tile, the ops between it and its
//! `PopLayer` are drawn there, and the pop composites that buffer onto its
//! parent with the pop's blend and opacity, **once** (ARQ-119). The layer's
//! bounds are the task's: `PushLayer` carries no rectangle, so the buffer is
//! the whole tile and nothing finer clips the group (02 §6.1, ARQ-149).
//!
//! The buffer is linear-light premultiplied `f32`, the same type as the
//! surface (O11), so a group costs memory per nesting level and nothing in
//! conversion. There is no 8-bit layer path.
//!
//! The sixteen formulas belong to `gafira-raster` (ARQ-119). This module only
//! chooses the operands: the layer's buffer is the **source** and the parent's
//! pixels are the **backdrop**.

use gafira_doc::BlendMode;
use gafira_ports::render::RenderError;
use gafira_raster::blend_f32;

/// The open layer of one tile, if any.
#[derive(Default)]
pub(crate) struct Layer {
    open: Option<Vec<f32>>,
}

impl Layer {
    /// A tile with no open layer.
    pub(crate) const fn new() -> Self {
        Self { open: None }
    }

    /// The buffer the next op draws into: the open layer, or the tile's own.
    pub(crate) fn target<'a>(&'a mut self, tile: &'a mut [f32]) -> &'a mut [f32] {
        match self.open.as_mut() {
            Some(buffer) => buffer,
            None => tile,
        }
    }

    /// Opens a layer with a transparent buffer of `pixels` premultiplied values.
    pub(crate) fn push(&mut self, pixels: usize) {
        self.open = Some(vec![0.0; pixels]);
    }

    /// Composites the open layer onto `tile` with `blend` and `opacity`.
    pub(crate) fn pop(
        &mut self,
        tile: &mut [f32],
        blend: BlendMode,
        opacity: f32,
    ) -> Result<(), RenderError> {
        let Some(source) = self.open.take() else {
            return Ok(());
        };
        composite(tile, &source, blend, opacity)
    }
}

/// Composites one layer onto its parent, applying the opacity once.
///
/// The opacity multiplies the whole premultiplied layer, which is the same as
/// compositing the layer and then scaling the result: it is applied to the
/// composite, never to each child as it is drawn (ARQ-119, 02 §6.1).
fn composite(
    parent: &mut [f32],
    source: &[f32],
    blend: BlendMode,
    opacity: f32,
) -> Result<(), RenderError> {
    let opacity = opacity.clamp(0.0, 1.0);
    for (parent_pixel, layer) in parent.chunks_exact_mut(4).zip(source.chunks_exact(4)) {
        let source = [
            layer[0] * opacity,
            layer[1] * opacity,
            layer[2] * opacity,
            layer[3] * opacity,
        ];
        let backdrop = [
            parent_pixel[0],
            parent_pixel[1],
            parent_pixel[2],
            parent_pixel[3],
        ];
        let out = blend_f32(blend, backdrop, source).map_err(RenderError::Buffer)?;
        parent_pixel.copy_from_slice(&out);
    }
    Ok(())
}
