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
//! `ApplyMask` multiplies the innermost layer by its coverage and
//! `ApplyTransition` replaces it with the transition of the layer and the
//! pixels it will be composited onto (02 §4.6). Both are ops **on** the layer,
//! so both are refused when no layer is open.
//!
//! The plan's stack is balanced (02 §6.1): every pop closes the layer its push
//! opened. A pop with no layer open, and a layer left open at the end of the
//! tile, are plans this backend cannot execute; the port carries no
//! plan-validation error, so both are refused with the one it has for a plan
//! that cannot run, [`RenderError::DriverBlocked`], instead of drawn with a
//! guess.

use gafira_doc::BlendMode;
use gafira_doc::contract::ParamValues;
use gafira_ports::render::RenderError;
use gafira_raster::transitions::Transition;
use gafira_raster::{
    AlphaMask, BufferError, EffectCx, EffectError, ImageView, ImageViewMut, PixelData,
    PixelDataMut, blend_f32,
};

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

    /// Multiplies the innermost layer by `mask`'s coverage (02 §4.6).
    ///
    /// The mask is 8-bit and single-channel, one mask pixel per layer pixel,
    /// anchored at the layer's top-left with no rescale; the coverage multiplies
    /// **all four** premultiplied channels, so the layer stays premultiplied
    /// (ARQ-117). Where the mask does not reach the layer keeps the value it
    /// had, which is what makes a mask smaller than the layer mask only its own
    /// rectangle.
    ///
    /// # Errors
    /// [`RenderError::DriverBlocked`] when no layer is open: 02 §4.6 puts an
    /// `ApplyMask` outside a `PushLayer`/`PopLayer` pair outside the plan's
    /// domain, and this backend refuses what it cannot define.
    pub(crate) fn apply_mask(
        &mut self,
        mask: &AlphaMask,
        size: (u32, u32),
    ) -> Result<(), RenderError> {
        let Some(layer) = self.open.last_mut() else {
            return Err(RenderError::DriverBlocked);
        };
        let stride = usize::try_from(size.0).unwrap_or(0);
        let columns = usize::try_from(mask.width()).unwrap_or(0).min(stride);
        let rows = usize::try_from(mask.height())
            .unwrap_or(0)
            .min(usize::try_from(size.1).unwrap_or(0));
        let mask_stride = usize::try_from(mask.width()).unwrap_or(0);
        for row in 0..rows {
            for column in 0..columns {
                let coverage = f32::from(mask.data()[row * mask_stride + column]) / 255.0;
                let at = (row * stride + column) * 4;
                for channel in 0..4 {
                    layer[at + channel] *= coverage;
                }
            }
        }
        Ok(())
    }

    /// Replaces the innermost layer with the transition of its background and
    /// its content (02 §4.6).
    ///
    /// `to` is the layer's content — what was drawn since the `PushLayer` — and
    /// `from` is its background: the pixels the `PopLayer` will composite onto,
    /// which is the parent's buffer, or the tile's own pixels for the outermost
    /// layer. No snapshot is needed: while a layer is open nothing writes to its
    /// parent, so the parent **at the op** is the background **at the push**.
    /// Both inputs have the region and the format of the destination, as
    /// ARQ-118 requires; this backend accumulates in premultiplied `f32` (O11),
    /// so the format is the same in both qualities.
    ///
    /// # Errors
    /// [`RenderError::DriverBlocked`] when no layer is open: without a `PopLayer`
    /// there is no background to be `from`. [`RenderError::Effect`] naming
    /// `progress` when it is not finite, and the transition's own [`EffectError`]
    /// otherwise. A finite `progress` outside `[0, 1]` is clipped (ARQ-118).
    pub(crate) fn apply_transition(
        &mut self,
        transition: &dyn Transition,
        params: &ParamValues,
        progress: f32,
        tile: &mut [f32],
        size: (u32, u32),
    ) -> Result<(), RenderError> {
        if self.open.is_empty() {
            return Err(RenderError::DriverBlocked);
        }
        if !progress.is_finite() {
            return Err(RenderError::Effect(EffectError::Buffer(
                BufferError::NonFinite {
                    field: "progress",
                    index: 0,
                },
            )));
        }
        // ARQ-118: a finite progress outside the range is clipped, not refused.
        let progress = progress.clamp(0.0, 1.0);
        let depth = self.open.len();
        let (parents, innermost) = self.open.split_at_mut(depth - 1);
        let to = innermost[0].as_slice();
        let from = match parents.last() {
            Some(parent) => parent.as_slice(),
            None => &tile[..],
        };
        let mut dst = vec![0.0f32; to.len()];
        {
            // A transition runs on whole frames and does not read `cx.tile`
            // (02 §4.13), and its op carries no scale.
            let cx = EffectCx::whole(1.0);
            transition
                .run_cpu(
                    &cx,
                    params,
                    progress,
                    ImageView::new(size.0, size.1, PixelData::RgbaF32(from))?,
                    ImageView::new(size.0, size.1, PixelData::RgbaF32(to))?,
                    ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(&mut dst))?,
                )
                .map_err(RenderError::Effect)?;
        }
        // The op replaces the layer's content with the result; what is drawn
        // after it belongs to neither input (02 §4.6).
        if let Some(layer) = self.open.last_mut() {
            layer.copy_from_slice(&dst);
        }
        Ok(())
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
