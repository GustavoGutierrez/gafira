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
//! The buffers are premultiplied linear `f32` ([`crate::layer`], O11), which is
//! the format the effect is handed in both qualities: the target's format
//! decides only what `read_back` answers.

use gafira_doc::contract::{CancelToken, ParamValues};
use gafira_geom::IRect;
use gafira_ports::render::{RenderError, RenderOp, TileTask};
use gafira_raster::effects::Effect;
use gafira_raster::{EffectCx, ImageView, ImageViewMut, PixelData, PixelDataMut, TileWindow};

/// The rectangle the tile's ops are rendered over: the tile grown by every
/// effect's input region, and clipped to the page (ARQ-120).
pub(crate) fn grown_rect(task: &TileTask, page: IRect) -> IRect {
    let tile = task.region.intersect(page);
    let mut grown = tile;
    for op in &task.ops {
        if let RenderOp::ApplyEffect {
            effect,
            params,
            scale,
        } = op
        {
            // `out` is the tile's region: an effect's output replaces the layer
            // it runs on, and the tile is the part of it that reaches the page.
            let input = effect.roi(&cx(*scale, tile, page), params, tile);
            grown = union(grown, input.intersect(page));
        }
    }
    grown
}

/// Runs `effect` once over the grown tile and commits its result to `target`.
///
/// # Errors
/// [`RenderError::Buffer`] when a buffer does not fit the region,
/// [`RenderError::Effect`] when the effect rejects its input, and
/// [`RenderError::Cancelled`] when the token is dead after the run, in which
/// case `target` is left as it was.
pub(crate) fn run(
    effect: &dyn Effect,
    params: &ParamValues,
    scale: f32,
    target: &mut [f32],
    grown: IRect,
    page: IRect,
    cancel: &CancelToken,
) -> Result<(), RenderError> {
    // The buffer is the grown rectangle's pixels, so its size is the region's.
    let size = (grown.width(), grown.height());
    let mut dst = vec![0.0f32; target.len()];
    effect
        .run_cpu(
            &cx(scale, grown, page),
            params,
            ImageView::new(size.0, size.1, PixelData::RgbaF32(target))?,
            ImageViewMut::new(size.0, size.1, PixelDataMut::RgbaF32(&mut dst))?,
        )
        .map_err(RenderError::Effect)?;
    // The effect takes no token, so this is the checkpoint of the grown pass:
    // the result reaches the layer only when the frame is still alive.
    cancel.check()?;
    target.copy_from_slice(&dst);
    Ok(())
}

/// The context an effect answers its `roi` and runs in.
///
/// `src` is a cut of the page, so the window describes where the cut sits in
/// the page (ARQ-120): `x` and `y` are its origin in page pixels. A tile of
/// this backend is one page pixel per buffer pixel, so the offsets are the
/// region's own.
fn cx(scale: f32, region: IRect, page: IRect) -> EffectCx {
    let offset = |low: i32, origin: i32| u32::try_from(low.saturating_sub(origin)).unwrap_or(0);
    EffectCx {
        scale,
        tile: Some(TileWindow {
            image_width: page.width(),
            image_height: page.height(),
            x: offset(region.x0, page.x0),
            y: offset(region.y0, page.y0),
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
