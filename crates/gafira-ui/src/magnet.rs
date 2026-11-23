//! The magnet (Imán) on the canvas and the arrow-key nudge (FD-06, D-16,
//! D-28, 03 §9.1, §9.2; spec 044).
//!
//! `gafira-layout::snap` resolves each axis; this module feeds it what the
//! editor has: the box that moves, the page, the other elements that
//! attract (`gafira_layout::attractors`) and the page guides, with the
//! threshold in screen px so it feels the same at any zoom. Pure functions,
//! so they are tested without a window.

use egui::Key;
use gafira_doc::{NodeId, Page, Placement};
use gafira_geom::Rect;
use gafira_layout::{SnapInput, snap};

use crate::canvas::{self, Handle};
use crate::rotation;

/// How near a target must be to attract, in screen px (FD-06: 6 px by
/// default, D-28).
pub(crate) const THRESHOLD_PX: f64 = 6.0;

/// How far an arrow moves the selection, in px of page; with Shift, the
/// second (03 §9.2: «Mover 1 px · 10 px»).
pub(crate) const NUDGE: f64 = 1.0;
pub(crate) const NUDGE_SHIFT: f64 = 10.0;

/// The outcome of the magnet: how far to move on each axis, in px of
/// page, and where the target line it snapped to lies (`x` a vertical
/// line, `y` a horizontal one), for the smart guides the canvas draws.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Snap {
    pub(crate) dx: f64,
    pub(crate) dy: f64,
    pub(crate) x: Option<f64>,
    pub(crate) y: Option<f64>,
}

/// What attracts on one page.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Field<'a> {
    pub(crate) page: &'a Page,
    /// The page size in px.
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) dpi: f64,
    /// Screen px per px of page.
    pub(crate) zoom: f64,
    /// Ver → Guías: hidden guides do not attract (FD-06).
    pub(crate) guides: bool,
    /// The magnet is on and Ctrl (Cmd on macOS) is not held (FD-06, D-90).
    pub(crate) active: bool,
    /// What moves: it does not attract itself (FD-06 «Quién no atrae»).
    pub(crate) moving: &'a [NodeId],
}

/// Snaps the box `moving` (px of page) on each axis by its edges and
/// centre, to the nearest guide, page edge or centre, or edge or centre of
/// another element within [`THRESHOLD_PX`] of screen. Anything the layout
/// refuses (a broken page, a zoom out of range) snaps nothing.
pub(crate) fn snap_box(field: &Field<'_>, moving: Rect) -> Snap {
    if !field.active {
        return Snap::default();
    }
    let Ok(elements) = gafira_layout::attractors(field.page, field.moving) else {
        return Snap::default();
    };
    let input = SnapInput {
        moving,
        page: Rect::new(0.0, 0.0, field.width, field.height),
        // Márgenes have no interface yet; they would attract here.
        margins: None,
        elements: &elements,
        guides: &field.page.guides.0,
        guides_visible: field.guides,
        grid: &field.page.grid,
        active: true,
        zoom: field.zoom.clamp(0.05, 32.0),
        threshold: THRESHOLD_PX,
        dpi: field.dpi,
    };
    snap(&input).map_or_else(
        |_| Snap::default(),
        |result| Snap {
            dx: result.x.offset,
            dy: result.y.offset,
            x: result.x.target.map(|t| t.coordinate),
            y: result.y.target.map(|t| t.coordinate),
        },
    )
}

/// Snaps the edges a resize handle drags: `x` and `y` are where they are
/// now (px of page), `None` for an axis the handle does not move.
pub(crate) fn snap_edges(field: &Field<'_>, x: Option<f64>, y: Option<f64>) -> Snap {
    // A box of no size at the edges: its edges and centre are the point.
    let (at_x, at_y) = (x.unwrap_or(0.0), y.unwrap_or(0.0));
    let snap = snap_box(field, Rect::new(at_x, at_y, at_x, at_y));
    Snap {
        dx: if x.is_some() { snap.dx } else { 0.0 },
        dy: if y.is_some() { snap.dy } else { 0.0 },
        x: snap.x.filter(|_| x.is_some()),
        y: snap.y.filter(|_| y.is_some()),
    }
}

/// The box of `nodes` on `page` aligned to the axes, a turned one by the
/// box of its turned outline (D-28); `None` if there is none.
pub(crate) fn selection_box(page: &Page, nodes: &[NodeId]) -> Option<Rect> {
    let mut boxes = Vec::with_capacity(nodes.len());
    for node in nodes {
        boxes.push(gafira_layout::node_bounds(page, *node).ok()?);
    }
    boxes.into_iter().reduce(|all, one| all.union(one))
}

/// `resized` (what the handle made of `origin`) with the edges the handle
/// drags snapped (spec 044). Only an upright element snaps: a turned one
/// resizes in its own axes, which are not the page's (spec 041).
pub(crate) fn snap_resize(
    field: &Field<'_>,
    origin: &Placement,
    handle: Handle,
    resized: Placement,
) -> (Placement, Snap) {
    if !rotation::upright(origin) {
        return (resized, Snap::default());
    }
    let edge = |side: i8, lo: f64, size: f64| match side {
        -1 => Some(lo),
        1 => Some(lo + size),
        _ => None,
    };
    let snap = snap_edges(
        field,
        edge(handle.x, resized.x, resized.w),
        edge(handle.y, resized.y, resized.h),
    );
    let mut to = resized;
    // The dragged edge moves; the opposite one stays. A snap that would
    // leave a side under 1 px (`canvas::resized`'s minimum) is dropped.
    let mut kept = snap;
    match handle.x {
        -1 if resized.w - snap.dx >= canvas::MIN_SIDE => {
            to.x += snap.dx;
            to.w -= snap.dx;
        }
        1 if resized.w + snap.dx >= canvas::MIN_SIDE => to.w += snap.dx,
        _ => (kept.dx, kept.x) = (0.0, None),
    }
    match handle.y {
        -1 if resized.h - snap.dy >= canvas::MIN_SIDE => {
            to.y += snap.dy;
            to.h -= snap.dy;
        }
        1 if resized.h + snap.dy >= canvas::MIN_SIDE => to.h += snap.dy,
        _ => (kept.dy, kept.y) = (0.0, None),
    }
    (to, kept)
}

/// `kept` (what a corner that keeps the proportions made of `origin`,
/// spec 045) with one dragged edge snapped: the one nearer its target, x
/// on a tie. The other side follows the proportions of `origin` from the
/// fixed corner, so the ratio holds. Only an upright element snaps.
pub(crate) fn snap_resize_kept(
    field: &Field<'_>,
    origin: &Placement,
    handle: Handle,
    kept: Placement,
) -> (Placement, Snap) {
    let none = (kept, Snap::default());
    if !rotation::upright(origin) || origin.w <= 0.0 || origin.h <= 0.0 {
        return none;
    }
    let edge = |side: i8, lo: f64, size: f64| if side == 1 { lo + size } else { lo };
    let snap = snap_edges(
        field,
        Some(edge(handle.x, kept.x, kept.w)),
        Some(edge(handle.y, kept.y, kept.h)),
    );
    // The driving edge: the one that snapped, the nearer if both did.
    let by_x = match (snap.x, snap.y) {
        (Some(_), Some(_)) => snap.dx.abs() <= snap.dy.abs(),
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => return none,
    };
    let scale = if by_x {
        (kept.w + f64::from(handle.x) * snap.dx) / origin.w
    } else {
        (kept.h + f64::from(handle.y) * snap.dy) / origin.h
    };
    let Some(to) = crate::resize::scaled(origin, handle, scale) else {
        return none;
    };
    let shown = if by_x {
        Snap {
            dx: snap.dx,
            x: snap.x,
            ..Snap::default()
        }
    } else {
        Snap {
            dy: snap.dy,
            y: snap.y,
            ..Snap::default()
        }
    };
    (to, shown)
}

/// `resized` (what a handle made of `origin` from its centre, D-90) with
/// the dragged edges snapped and the opposite ones mirrored about the
/// centre. Free, each dragged edge snaps on its axis and a snap that,
/// mirrored, would leave a side under 1 px is dropped. With `keep` (a
/// corner that keeps the proportions) only the edge nearer its target
/// snaps, `x` on a tie, and the other side follows the proportions of
/// `origin` from the centre. Only an upright element snaps (spec 041).
pub(crate) fn snap_resize_centred(
    field: &Field<'_>,
    origin: &Placement,
    handle: Handle,
    resized: Placement,
    keep: bool,
) -> (Placement, Snap) {
    let none = (resized, Snap::default());
    if !rotation::upright(origin) {
        return none;
    }
    let edge = |side: i8, lo: f64, size: f64| match side {
        -1 => Some(lo),
        1 => Some(lo + size),
        _ => None,
    };
    let snap = snap_edges(
        field,
        edge(handle.x, resized.x, resized.w),
        edge(handle.y, resized.y, resized.h),
    );
    // A dragged edge moving `d` makes its side `2d` longer on the side the
    // handle points to, as both edges move.
    let grown = |size: f64, side: i8, d: f64| size + 2.0 * f64::from(side) * d;
    if keep && crate::resize::is_corner(handle) && origin.w > 0.0 && origin.h > 0.0 {
        let by_x = match (snap.x, snap.y) {
            (Some(_), Some(_)) => snap.dx.abs() <= snap.dy.abs(),
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return none,
        };
        let scale = if by_x {
            grown(resized.w, handle.x, snap.dx) / origin.w
        } else {
            grown(resized.h, handle.y, snap.dy) / origin.h
        };
        let Some(to) = crate::resize::scaled_centred(origin, scale) else {
            return none;
        };
        let shown = if by_x {
            Snap {
                dx: snap.dx,
                x: snap.x,
                ..Snap::default()
            }
        } else {
            Snap {
                dy: snap.dy,
                y: snap.y,
                ..Snap::default()
            }
        };
        return (to, shown);
    }
    let mut kept = snap;
    let mut w = resized.w;
    let mut h = resized.h;
    match grown(resized.w, handle.x, snap.dx) {
        size if snap.x.is_some() && size >= canvas::MIN_SIDE => w = size,
        _ => (kept.dx, kept.x) = (0.0, None),
    }
    match grown(resized.h, handle.y, snap.dy) {
        size if snap.y.is_some() && size >= canvas::MIN_SIDE => h = size,
        _ => (kept.dy, kept.y) = (0.0, None),
    }
    (crate::resize::centred(&resized, w, h), kept)
}

/// How far an arrow key moves the selection, in px of page; Shift moves
/// [`NUDGE_SHIFT`]. Any other key is `None`.
pub(crate) fn nudge(key: Key, shift: bool) -> Option<(f64, f64)> {
    let step = if shift { NUDGE_SHIFT } else { NUDGE };
    match key {
        Key::ArrowLeft => Some((-step, 0.0)),
        Key::ArrowRight => Some((step, 0.0)),
        Key::ArrowUp => Some((0.0, -step)),
        Key::ArrowDown => Some((0.0, step)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
