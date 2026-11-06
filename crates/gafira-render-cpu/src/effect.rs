//! The effect pass of one tile (`CB5`, ARQ-120, 02 §4.6).
//!
//! An effect is a filter over one input, and `Effect::roi` says which input
//! region it needs to produce its output. A tile alone does not hold that
//! region, so the tile is **grown** by every effect's `roi` before anything is
//! drawn, the plan's ops are rendered over the grown rectangle, each effect runs
//! **once** over it, and the tile keeps its own crop (`design.md`, "Critical
//! code"). The growth is clipped to the page, so an effect cannot ask for a
//! buffer the destination does not have.
//!
//! The effect runs in place, through a destination buffer of the same size, and
//! its result is committed only when it succeeded **and** the token is still
//! live: `Effect::run_cpu` receives no `CancelToken` (02 §4.6), so the
//! checkpoint is between the run and the commit (ARQ-112). A cancelled pass
//! therefore leaves a layer with what it had before the op.
//!
//! The destination belongs to the backend, not to the op (`O17`): one scratch
//! buffer, reused by every effect of the frame and compared against
//! [`SURFACE_BUDGET`] **before** it grows, because `trim` runs between frames
//! and cannot be the lever for a buffer that lives inside one `execute`.
//!
//! The buffers are premultiplied linear `f32` ([`crate::layer`], O11), which is
//! the format the effect is handed in both qualities: the target's format
//! decides only what `read_back` answers.

use gafira_doc::contract::{CancelToken, ParamValues};
use gafira_geom::IRect;
use gafira_ports::render::{RenderError, RenderOp, TileTask};
use gafira_raster::effects::Effect;
use gafira_raster::{EffectCx, ImageView, ImageViewMut, PixelData, PixelDataMut, TileWindow};

use crate::surface::SURFACE_BUDGET;

/// The rectangle the tile's ops are rendered over: the tile grown by every
/// effect's input region, and clipped to the **surface** (ARQ-120).
///
/// The clipping rectangle is the surface and not the page: an effect's input
/// can only be what the backend holds (O18), so growth past the surface would
/// ask for pixels no buffer has.
///
/// The growth composes, in plan order: each effect's `roi` is asked against the
/// buffer the previous ops produced, and the buffer extends to include the
/// input that `roi` answers. A second effect therefore measures its input over
/// what the first one grew, not over the bare tile — measuring every `roi`
/// against the tile leaves a chain short of the input its last effect needs.
pub(crate) fn grown_rect(task: &TileTask, surface: IRect, page: (u32, u32)) -> IRect {
    let tile = task.region.intersect(surface);
    let mut grown = tile;
    for op in &task.ops {
        if let RenderOp::ApplyEffect {
            effect,
            params,
            scale,
        } = op
        {
            // An effect replaces the buffer it filters, so what it must produce
            // is the buffer as it stands at this op. Asking `roi` over `grown`
            // is asking it over the buffer the effect will read.
            let input = effect.roi(&cx(*scale, grown, page), params, grown);
            grown = union(grown, input.intersect(surface));
        }
    }
    grown
}

/// One effect pass over the grown buffer: what the effect filters, and the
/// backend's scratch it writes into.
pub(crate) struct Pass<'a> {
    /// The resolved parameters (`p` is resolved, ARQ-110).
    pub params: &'a ParamValues,
    /// The scale `roi` was asked with (ARQ-120).
    pub scale: f32,
    /// The buffer the effect filters and, on success, replaces.
    pub target: &'a mut [f32],
    /// The backend's one destination buffer, reused across ops and tiles (O17).
    pub scratch: &'a mut Vec<f32>,
    /// The bytes the frame already holds: the surface, the layer pool and the
    /// layers open at this op.
    pub held: u64,
    /// The grown rectangle `target` covers.
    pub grown: IRect,
    /// The page's extent, in page pixels: the image the window describes (O18).
    pub page: (u32, u32),
}

/// Runs `effect` once over the grown tile and commits its result to `target`.
///
/// The destination is [`Pass::scratch`], grown only after the frame's peak —
/// [`Pass::held`] plus this buffer — is checked against [`SURFACE_BUDGET`]
/// (`O17`).
///
/// # Errors
/// [`RenderError::OutOfMemory`] when the destination would take the frame's
/// peak over the budget, before it is allocated; [`RenderError::Buffer`] when a
/// buffer does not fit the region; [`RenderError::Effect`] when the effect
/// rejects its input; and [`RenderError::Cancelled`] when the token is dead
/// after the run, in which case `target` is left as it was.
pub(crate) fn run(
    effect: &dyn Effect,
    pass: Pass<'_>,
    cancel: &CancelToken,
) -> Result<(), RenderError> {
    let Pass {
        params,
        scale,
        target,
        scratch,
        held,
        grown,
        page,
    } = pass;
    // The buffer is the grown rectangle's pixels, so its size is the region's.
    let size = (grown.width(), grown.height());
    reserve_scratch(scratch, target.len(), held)?;
    let dst = &mut scratch[..target.len()];
    // The destination is reused, so nothing of a previous op or tile may reach
    // this one: the effect starts from transparent black, as a fresh buffer
    // would (ARQ-111).
    dst.fill(0.0);
    effect
        .run_cpu(
            &cx(scale, grown, page),
            params,
            ImageView::new(size.0, size.1, PixelData::RgbaF32(target))?,
            ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(dst))?,
        )
        .map_err(RenderError::Effect)?;
    // The effect takes no token, so this is the checkpoint of the grown pass:
    // the result reaches the layer only when the frame is still alive.
    cancel.check()?;
    target.copy_from_slice(&scratch[..target.len()]);
    Ok(())
}

/// Grows `scratch` to `pixels`, refusing the frame's peak **before** it is
/// allocated (`O17`, `D-03`).
///
/// `held` is what the frame already holds: the surface, the layer pool and the
/// layers open at this op. The scratch is one buffer shared by every effect of
/// the frame, so growing it replaces the previous size rather than adding to
/// it, and a buffer already big enough costs nothing.
///
/// # Errors
/// [`RenderError::OutOfMemory`] when `held` plus the new buffer would pass
/// [`SURFACE_BUDGET`], or when the size cannot be expressed.
pub(crate) fn reserve_scratch(
    scratch: &mut Vec<f32>,
    pixels: usize,
    held: u64,
) -> Result<(), RenderError> {
    if scratch.len() >= pixels {
        return Ok(());
    }
    let bytes = u64::try_from(pixels)
        .ok()
        .and_then(|count| count.checked_mul(4))
        .ok_or(RenderError::OutOfMemory)?;
    if !fits(held, bytes) {
        return Err(RenderError::OutOfMemory);
    }
    scratch.resize(pixels, 0.0);
    Ok(())
}

/// True when a frame holding `held` bytes can add `scratch` bytes and stay
/// inside the cap `Surface::new` applies (`D-03`, `O17`).
fn fits(held: u64, scratch: u64) -> bool {
    held.saturating_add(scratch) <= SURFACE_BUDGET.0
}

/// The context an effect answers its `roi` and runs in.
///
/// `src` is a cut of the page, so the window describes where the cut sits in
/// the page (ARQ-120, O18): `x` and `y` are its origin in page pixels and
/// `image_width`/`image_height` are the **page's** extent, never the surface's.
/// A tile of this backend is one page pixel per buffer pixel (D4) and the ops
/// carry page coordinates (ARQ-149), so a region's own corner is already the
/// page pixel the window asks for — subtracting the surface's origin would
/// re-express it in the window's space instead.
fn cx(scale: f32, region: IRect, page: (u32, u32)) -> EffectCx {
    EffectCx {
        scale,
        tile: Some(TileWindow {
            image_width: page.0,
            image_height: page.1,
            x: u32::try_from(region.x0).unwrap_or(0),
            y: u32::try_from(region.y0).unwrap_or(0),
        }),
    }
}

/// The smallest rectangle holding both inputs, saturating at the `i32` edges.
fn union(a: IRect, b: IRect) -> IRect {
    IRect::new(
        a.x0.min(b.x0),
        a.y0.min(b.y0),
        a.x1.max(b.x1),
        a.y1.max(b.y1),
    )
}
