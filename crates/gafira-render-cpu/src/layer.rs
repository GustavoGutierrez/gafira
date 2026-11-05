//! The layers the ops of a tile are drawn into (`CB2`, 02 §6.1).
//!
//! `PushLayer` opens a buffer the size of the tile, the ops between it and its
//! `PopLayer` are drawn there, and the pop composites that buffer onto its
//! parent — the layer below it, or the tile's own pixels — with the pop's blend
//! and opacity, **once** (ARQ-119). The layer's bounds are the task's:
//! `PushLayer` carries no rectangle, so the buffer is the whole tile and
//! nothing finer clips the group (02 §6.1, ARQ-149).
//!
//! The buffers are linear-light premultiplied `f32`, the same type as the
//! surface (O11), so a group costs memory per nesting level and nothing in
//! conversion. There is no 8-bit layer path.
//!
//! The sixteen formulas belong to `gafira-raster` (ARQ-119). This module only
//! chooses the operands: the layer's buffer is the **source** and the parent's
//! pixels are the **backdrop**.
//!
//! The buffers are pooled rather than allocated per layer: a pop returns its
//! buffer to the pool and the next push takes it back. The pool keeps every
//! buffer a pop returns, so its size converges to the deepest stack the frame
//! drew and never shrinks on its own; `trim` is what drops it (ARQ-143). That
//! growth policy is not critical code (design, "Critical code").
//!
//! The plan's stack is balanced (02 §6.1): every pop closes the layer its push
//! opened. A pop with no layer open, and a layer left open at the end of the
//! tile, are plans this backend cannot execute; the port carries no
//! plan-validation error, so both are refused with the one it has for a plan
//! that cannot run, [`RenderError::DriverBlocked`], instead of drawn with a
//! guess.

use gafira_doc::BlendMode;
use gafira_ports::render::RenderError;
use gafira_raster::blend_f32;

/// The layers of one tile, outermost first, and the buffers a pop returned.
#[derive(Default)]
pub(crate) struct Layers {
    open: Vec<Vec<f32>>,
    pool: Vec<Vec<f32>>,
}

impl Layers {
    /// A tile with no open layer and an empty pool.
    pub(crate) const fn new() -> Self {
        Self {
            open: Vec::new(),
            pool: Vec::new(),
        }
    }

    /// True when every layer the plan pushed has been popped.
    pub(crate) fn is_empty(&self) -> bool {
        self.open.is_empty()
    }

    /// The bytes the open layers and the pool hold (ARQ-143).
    pub(crate) fn bytes(&self) -> u64 {
        let pixels: usize = self.open.iter().chain(self.pool.iter()).map(Vec::len).sum();
        u64::try_from(pixels)
            .ok()
            .and_then(|count| count.checked_mul(4))
            .unwrap_or(u64::MAX)
    }

    /// Returns every open layer to the pool.
    ///
    /// A tile that ended with a layer open, because its plan was unbalanced or
    /// its render failed, leaves nothing behind for the next tile: the buffers
    /// are reusable, the depth is reset.
    pub(crate) fn reset(&mut self) {
        let open = std::mem::take(&mut self.open);
        self.pool.extend(open);
    }

    /// The buffer the next op draws into: the innermost open layer, or the
    /// tile's own pixels when there is none.
    pub(crate) fn target<'a>(&'a mut self, tile: &'a mut [f32]) -> &'a mut [f32] {
        match self.open.last_mut() {
            Some(buffer) => buffer,
            None => tile,
        }
    }

    /// Opens a layer with a transparent buffer of `pixels` premultiplied values.
    pub(crate) fn push(&mut self, pixels: usize) {
        let mut buffer = self.pool.pop().unwrap_or_default();
        buffer.resize(pixels, 0.0);
        // A pooled buffer still holds the layer that returned it, so it is
        // cleared to transparent here: nothing of a previous layer may reach
        // the new one (ARQ-117). A fresh buffer is transparent already.
        buffer.fill(0.0);
        self.open.push(buffer);
    }

    /// Composites the innermost layer onto its parent with `blend` and `opacity`.
    ///
    /// # Errors
    /// [`RenderError::DriverBlocked`] when no layer is open, and
    /// [`RenderError::Buffer`] when the composite produces a non-finite value.
    pub(crate) fn pop(
        &mut self,
        tile: &mut [f32],
        blend: BlendMode,
        opacity: f32,
    ) -> Result<(), RenderError> {
        let Some(source) = self.open.pop() else {
            return Err(RenderError::DriverBlocked);
        };
        let outcome = match self.open.last_mut() {
            Some(parent) => composite(parent, &source, blend, opacity),
            None => composite(tile, &source, blend, opacity),
        };
        // The buffer goes back to the pool whether the composite worked or not.
        self.pool.push(source);
        outcome
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
