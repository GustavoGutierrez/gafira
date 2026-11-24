//! Resizing with the handles (03 §9.1, D-89, D-90; specs 045, 047): the
//! corners keep the element's proportions unless Shift frees them, Alt
//! resizes from the centre, the cursor each handle shows, and the live
//! size label. Pure functions, so they are tested without a window.

use egui::{CursorIcon, Modifiers, Pos2, Rect, Vec2};
use gafira_doc::Placement;
use gafira_geom::{Affine, Point};

use crate::canvas::{self, Handle};
use crate::rotation;

/// The gap between the pointer and the size label, in screen px.
pub(crate) const LABEL_GAP: f32 = 16.0;

/// `origin` resized by dragging `handle` `offset` screen px at `zoom`.
/// With `keep`, a corner keeps the proportions of `origin`: the side that
/// grows more (relatively) drives and the other follows; the opposite
/// corner stays and neither side goes under 1 px, so dragging past the
/// opposite corner stops at the smallest box of the same proportions. An
/// edge, or a corner without `keep`, is [`canvas::resized`]. A turned
/// element resizes in its own axes (spec 041).
pub(crate) fn resized(
    origin: &Placement,
    handle: Handle,
    offset: Vec2,
    zoom: f32,
    keep: bool,
) -> Placement {
    if !keep || !is_corner(handle) || origin.w <= 0.0 || origin.h <= 0.0 {
        return canvas::resized(origin, handle, offset, zoom);
    }
    let Some(local) = local_drag(origin, offset, zoom) else {
        return *origin;
    };
    let wide = (origin.w + f64::from(handle.x) * local.x) / origin.w;
    let tall = (origin.h + f64::from(handle.y) * local.y) / origin.h;
    let least = (canvas::MIN_SIDE / origin.w).max(canvas::MIN_SIDE / origin.h);
    scaled(origin, handle, wide.max(tall).max(least)).unwrap_or(*origin)
}

/// What the keys held during a drag of the selection do (D-89, D-90).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DragKeys {
    /// Shift: a corner frees its proportions, a turn snaps to 15°.
    pub(crate) shift: bool,
    /// Alt: a resize goes from the centre.
    pub(crate) centre: bool,
    /// Ctrl (Cmd on macOS): the magnet lets go.
    pub(crate) free: bool,
}

/// The [`DragKeys`] of `modifiers`: only the state of the keys counts,
/// so a held Ctrl fires no shortcut. `command` is Ctrl, or Cmd on macOS.
pub(crate) fn drag_keys(modifiers: Modifiers) -> DragKeys {
    DragKeys {
        shift: modifiers.shift,
        centre: modifiers.alt,
        free: modifiers.command,
    }
}

/// `origin` resized from its centre by dragging `handle` `offset` screen
/// px at `zoom` (D-90): the centre stays on the page and the opposite side
/// moves as much the other way. With `keep`, a corner keeps the
/// proportions, the side that grows more driving, as [`resized`] does.
/// No side goes under 1 px: past the centre the box stays at the smallest
/// size about the centre, it does not flip. A turned element resizes in
/// its own axes (spec 041).
pub(crate) fn resized_centred(
    origin: &Placement,
    handle: Handle,
    offset: Vec2,
    zoom: f32,
    keep: bool,
) -> Placement {
    let Some(local) = local_drag(origin, offset, zoom) else {
        return *origin;
    };
    // Both sides move, so a side changes twice the drag.
    let grow_w = 2.0 * f64::from(handle.x) * local.x;
    let grow_h = 2.0 * f64::from(handle.y) * local.y;
    if keep && is_corner(handle) && origin.w > 0.0 && origin.h > 0.0 {
        let wide = (origin.w + grow_w) / origin.w;
        let tall = (origin.h + grow_h) / origin.h;
        let least = (canvas::MIN_SIDE / origin.w).max(canvas::MIN_SIDE / origin.h);
        return scaled_centred(origin, wide.max(tall).max(least)).unwrap_or(*origin);
    }
    let w = match handle.x {
        0 => origin.w,
        _ => (origin.w + grow_w).max(canvas::MIN_SIDE),
    };
    let h = match handle.y {
        0 => origin.h,
        _ => (origin.h + grow_h).max(canvas::MIN_SIDE),
    };
    centred(origin, w, h)
}

/// `origin` scaled by `scale` about its centre; `None` if a side would go
/// under 1 px.
pub(crate) fn scaled_centred(origin: &Placement, scale: f64) -> Option<Placement> {
    let (w, h) = (origin.w * scale, origin.h * scale);
    if !(w.is_finite() && h.is_finite()) || w < canvas::MIN_SIDE || h < canvas::MIN_SIDE {
        return None;
    }
    Some(centred(origin, w, h))
}

/// `origin` with a `w` × `h` box about the same centre. The placement
/// turns, shears and flips about the centre of its box, so this keeps the
/// centre on the page for a turned element too.
pub(crate) fn centred(origin: &Placement, w: f64, h: f64) -> Placement {
    let mut to = *origin;
    to.x = origin.x + (origin.w - w) / 2.0;
    to.y = origin.y + (origin.h - h) / 2.0;
    to.w = w;
    to.h = h;
    to
}

/// The drag `offset` (screen px at `zoom`) in the element's own axes: the
/// page's for an upright one; `None` for a transform that cannot be
/// undone.
fn local_drag(origin: &Placement, offset: Vec2, zoom: f32) -> Option<Point> {
    let page = Point::new(f64::from(offset.x / zoom), f64::from(offset.y / zoom));
    if rotation::upright(origin) {
        return Some(page);
    }
    let [a, b, c, d, ..] = origin.to_transform().as_coeffs();
    let linear = Affine::new([a, b, c, d, 0.0, 0.0]);
    if !linear.determinant().is_finite() || linear.determinant().abs() < 1e-12 {
        return None;
    }
    Some(linear.inverse() * page)
}

/// Whether `handle` is a corner: the ones that keep the proportions.
pub(crate) fn is_corner(handle: Handle) -> bool {
    handle.x != 0 && handle.y != 0
}

/// `origin` scaled by `scale` about the corner opposite `handle`, in its
/// own axes; `None` if a side would go under 1 px.
pub(crate) fn scaled(origin: &Placement, handle: Handle, scale: f64) -> Option<Placement> {
    let (w, h) = (origin.w * scale, origin.h * scale);
    if !(w.is_finite() && h.is_finite()) || w < canvas::MIN_SIDE || h < canvas::MIN_SIDE {
        return None;
    }
    // The new local box, with the corner opposite the handle where it was.
    let left = if handle.x == 1 { 0.0 } else { origin.w - w };
    let top = if handle.y == 1 { 0.0 } else { origin.h - h };
    let mut to = *origin;
    to.w = w;
    to.h = h;
    if rotation::upright(origin) {
        to.x = origin.x + left;
        to.y = origin.y + top;
    } else {
        // The turn is about the centre, so the new centre goes where the
        // old transform puts the middle of the new local box.
        let centre = origin.to_transform() * Point::new(left + w / 2.0, top + h / 2.0);
        to.x = centre.x - w / 2.0;
        to.y = centre.y - h / 2.0;
    }
    Some(to)
}

/// Where `handle` points on the screen under `transform` (the element's
/// local box to the screen): its local direction `(x, y)` through the
/// turn, flip and shear, not the box's size, so a corner is diagonal
/// whatever the proportions.
pub(crate) fn outward(transform: Affine, handle: Handle) -> Vec2 {
    let [a, b, c, d, ..] = transform.as_coeffs();
    let along =
        Affine::new([a, b, c, d, 0.0, 0.0]) * Point::new(f64::from(handle.x), f64::from(handle.y));
    Vec2::new(along.x as f32, along.y as f32)
}

/// The resize cursor of `handle`: the double-headed arrow whose axis is
/// nearest the handle's direction on the screen, in 45° sectors.
pub(crate) fn cursor(transform: Affine, handle: Handle) -> CursorIcon {
    let along = outward(transform, handle);
    // Screen y grows down; the angle is counted up from the right.
    let degrees = f64::from(-along.y).atan2(f64::from(along.x)).to_degrees();
    match ((degrees / 45.0).round() as i64).rem_euclid(4) {
        0 => CursorIcon::ResizeHorizontal,
        1 => CursorIcon::ResizeNeSw,
        2 => CursorIcon::ResizeVertical,
        _ => CursorIcon::ResizeNwSe,
    }
}

/// A size in px as the Inspector shows it: at most one decimal, none when
/// it is whole («320», «180.5»).
pub(crate) fn number(value: f64) -> String {
    // `+ 0.0` turns a -0.0 into 0.0.
    let tenths = (value * 10.0).round() / 10.0 + 0.0;
    if tenths.fract() == 0.0 {
        format!("{tenths:.0}")
    } else {
        format!("{tenths:.1}")
    }
}

/// «320 × 180»: the live size of a resize.
pub(crate) fn size_text(w: f64, h: f64) -> String {
    format!("{} × {}", number(w), number(h))
}

/// «+20 × −11»: how much each side changed, with its sign.
pub(crate) fn delta_text(dw: f64, dh: f64) -> String {
    let signed = |v: f64| {
        let text = number(v.abs());
        if text == "0" {
            text
        } else if v > 0.0 {
            format!("+{text}")
        } else {
            // An en dash: the interface font has no minus sign (spec 098).
            format!("–{text}")
        }
    };
    format!("{} × {}", signed(dw), signed(dh))
}

/// Where the size label of `size` goes: `LABEL_GAP` px from `pointer`
/// along `outward`, so it sits outside the element; on an axis where that
/// would leave `view`, on the other side of the pointer.
pub(crate) fn label_rect(pointer: Pos2, outward: Vec2, size: Vec2, view: Rect) -> Rect {
    // On one axis: after the pointer when `ahead`, before it otherwise;
    // flipped when that leaves the view.
    let place = |at: f32, ahead: bool, length: f32, lo: f32, hi: f32| {
        let after = at + LABEL_GAP;
        let before = at - LABEL_GAP - length;
        match ahead {
            true if after + length > hi => before,
            false if before < lo => after,
            true => after,
            false => before,
        }
    };
    let x = place(pointer.x, outward.x >= 0.0, size.x, view.min.x, view.max.x);
    let y = place(pointer.y, outward.y >= 0.0, size.y, view.min.y, view.max.y);
    Rect::from_min_size(Pos2::new(x, y), size)
}

#[cfg(test)]
mod tests;
