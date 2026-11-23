//! Rulers and page guides on the canvas (FD-03, 03 §2.2 «Ver»; spec 044):
//! where a guide lies on the screen, which one the pointer holds, the
//! guide set after creating, moving or deleting one, and the ticks of a
//! ruler. Pure functions, so they are tested without a window.

use egui::{Pos2, Rect};
use gafira_doc::{Guide, GuideAxis, GuideSet, Page, PageId};
use gafira_geom::{Length, Unit};

/// The thickness of a ruler, in screen px.
pub(crate) const RULER: f32 = 20.0;

/// How near the pointer must be to a guide to hold it, in screen px.
pub(crate) const GUIDE_REACH: f32 = 4.0;

/// The least space between two numbered ticks of a ruler, in screen px.
const TICK_SPACING: f64 = 64.0;

/// Where `guide` lies in px of page; `None` if it cannot be converted.
pub(crate) fn guide_px(guide: &Guide, dpi: f64) -> Option<f64> {
    guide.position.to_px(dpi).ok().filter(|px| px.is_finite())
}

/// The guide of `page` (drawn at `page_rect` with `zoom`) under `pointer`:
/// the nearest within [`GUIDE_REACH`]. A vertical guide is held along the
/// height of its page, a horizontal one along the whole canvas row.
/// Locked guides are not held (FD-03).
pub(crate) fn hit(
    page: &Page,
    dpi: f64,
    page_rect: Rect,
    zoom: f32,
    pointer: Pos2,
) -> Option<usize> {
    page.guides
        .0
        .iter()
        .enumerate()
        .filter(|(_, guide)| !guide.locked)
        .filter_map(|(index, guide)| {
            let px = guide_px(guide, dpi)? as f32 * zoom;
            let distance = match guide.axis {
                GuideAxis::Vertical => {
                    if !page_rect.y_range().contains(pointer.y) {
                        return None;
                    }
                    (pointer.x - (page_rect.min.x + px)).abs()
                }
                GuideAxis::Horizontal => (pointer.y - (page_rect.min.y + px)).abs(),
            };
            (distance <= GUIDE_REACH).then_some((index, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
}

/// Where `pointer` lies on `axis` of a page drawn at `page_rect`, in px
/// of page: x for a vertical guide, y for a horizontal one.
pub(crate) fn page_coordinate(axis: GuideAxis, page_rect: Rect, zoom: f32, pointer: Pos2) -> f64 {
    let offset = match axis {
        GuideAxis::Vertical => pointer.x - page_rect.min.x,
        GuideAxis::Horizontal => pointer.y - page_rect.min.y,
    };
    f64::from(offset) / f64::from(zoom)
}

/// `guides` with a new guide on `axis` at `px` (written in px, FD-03),
/// added last.
pub(crate) fn added(guides: &GuideSet, axis: GuideAxis, px: f64) -> GuideSet {
    let mut out = guides.clone();
    out.0.push(guide(axis, px, false));
    out
}

/// `guides` with guide `index` moved to `px`; it keeps its axis and lock.
pub(crate) fn moved(guides: &GuideSet, index: usize, px: f64) -> GuideSet {
    let mut out = guides.clone();
    if let Some(at) = out.0.get_mut(index) {
        *at = guide(at.axis, px, at.locked);
    }
    out
}

/// `guides` without guide `index`; the others keep their order.
pub(crate) fn removed(guides: &GuideSet, index: usize) -> GuideSet {
    let mut out = guides.clone();
    if index < out.0.len() {
        out.0.remove(index);
    }
    out
}

/// The strip of the ruler a guide of `axis` comes from and goes back to:
/// the top ruler for a horizontal guide, the left one for a vertical one,
/// inside the canvas `area`.
pub(crate) fn strip(axis: GuideAxis, area: Rect) -> Rect {
    let size = match axis {
        GuideAxis::Horizontal => egui::vec2(area.width(), RULER),
        GuideAxis::Vertical => egui::vec2(RULER, area.height()),
    };
    Rect::from_min_size(area.min, size)
}

/// Whether a guide of `axis` let go at `at` goes back to its ruler: on
/// the ruler or past it, out of the canvas `area` (above the top ruler for
/// a horizontal guide, left of the left one for a vertical guide), so an
/// overshoot deletes it too instead of leaving it off the page.
pub(crate) fn back_to_ruler(axis: GuideAxis, area: Rect, at: Pos2) -> bool {
    match axis {
        GuideAxis::Horizontal => at.y <= area.min.y + RULER,
        GuideAxis::Vertical => at.x <= area.min.x + RULER,
    }
}

/// The page whose row holds `y` (the pages stack vertically), else the
/// nearest one.
pub(crate) fn page_at(pages: &[(PageId, Rect)], y: f32) -> Option<PageId> {
    let gap = |rect: &Rect| {
        if y < rect.min.y {
            rect.min.y - y
        } else {
            (y - rect.max.y).max(0.0)
        }
    };
    pages
        .iter()
        .min_by(|a, b| gap(&a.1).total_cmp(&gap(&b.1)))
        .map(|(page, _)| *page)
}

/// The ruler step in units of the ruler when one unit is `scale` screen
/// px: 1, 2 or 5 times a power of ten, at least `finest`, the smallest at
/// least [`TICK_SPACING`] screen px long.
fn step(scale: f64, finest: f64) -> f64 {
    let least = TICK_SPACING / scale;
    let mut decade = finest;
    // The bound keeps a broken zoom from spinning.
    for _ in 0..32 {
        if let Some(factor) = [1.0, 2.0, 5.0].into_iter().find(|f| f * decade >= least) {
            return factor * decade;
        }
        decade *= 10.0;
    }
    decade
}

/// The numbered ticks of a ruler: `(screen position, value in px of
/// page)` for each multiple of the step between `from` and `to` (screen
/// px), with the page's corner at `origin`. The step is 1, 2 or 5 times a
/// power of ten, the smallest at least [`TICK_SPACING`] apart on screen.
#[cfg(test)]
pub(crate) fn ticks(origin: f32, zoom: f32, from: f32, to: f32) -> Vec<(f32, i64)> {
    ticks_in(origin, f64::from(zoom), (from, to), 1.0)
        .into_iter()
        .map(|(at, value)| (at, value.round() as i64))
        .collect()
}

/// The numbered ticks of a ruler that counts in a unit `scale` screen px
/// long (spec 076: the page's own unit, px, in, mm or cm), steps of at
/// least `finest` units: `(screen position, value in units)`.
pub(crate) fn ticks_in(
    origin: f32,
    scale: f64,
    (from, to): (f32, f32),
    finest: f64,
) -> Vec<(f32, f64)> {
    if !(scale.is_finite() && scale > 0.0) {
        return Vec::new();
    }
    let step = step(scale, finest);
    let value = |screen: f32| f64::from(screen - origin) / scale;
    let first = (value(from) / step).ceil() as i64;
    let last = (value(to) / step).floor() as i64;
    (first..=last)
        .map(|k| {
            let at = k as f64 * step;
            (origin + (at * scale) as f32, at)
        })
        .collect()
}

/// The least space between two minor ticks of a ruler, in screen px.
const MINOR_SPACING: f64 = 5.0;

/// The minor ticks of a ruler between `from` and `to` (screen px), with
/// the page's corner at `origin` (S49-02): each numbered step split in 10,
/// 5 or 2, the most that keeps [`MINOR_SPACING`] between ticks. Each one
/// with whether it is the middle of its step (drawn longer). None falls on
/// a numbered tick.
#[cfg(test)]
pub(crate) fn minor_ticks(origin: f32, zoom: f32, from: f32, to: f32) -> Vec<(f32, bool)> {
    minor_ticks_in(origin, f64::from(zoom), (from, to), 1.0)
}

/// [`minor_ticks`] of a ruler that counts in a unit `scale` screen px long.
pub(crate) fn minor_ticks_in(
    origin: f32,
    scale: f64,
    (from, to): (f32, f32),
    finest: f64,
) -> Vec<(f32, bool)> {
    if !(scale.is_finite() && scale > 0.0) {
        return Vec::new();
    }
    let major = step(scale, finest);
    let Some(parts) = [10_i64, 5, 2]
        .into_iter()
        .find(|n| major / *n as f64 * scale >= MINOR_SPACING)
    else {
        return Vec::new();
    };
    let minor = major / parts as f64;
    let value = |screen: f32| f64::from(screen - origin) / scale;
    let first = (value(from) / minor).ceil() as i64;
    let last = (value(to) / minor).floor() as i64;
    (first..=last)
        .filter(|k| k.rem_euclid(parts) != 0)
        .map(|k| {
            let at = k as f64 * minor;
            let middle = parts % 2 == 0 && k.rem_euclid(parts) == parts / 2;
            (origin + (at * scale) as f32, middle)
        })
        .collect()
}

/// The finest step of a ruler in `unit`: whole px and points, tenths of
/// the rest (half a millimetre is too fine to read, so mm also steps by
/// whole units).
pub(crate) fn finest(unit: Unit) -> f64 {
    match unit {
        Unit::Px | Unit::Pt | Unit::Mm => 1.0,
        Unit::Cm | Unit::In => 0.1,
    }
}

/// Where a guide is, in `unit`, for its label while it is dragged: whole
/// px and points, a tenth of a millimetre, thousandths of an inch or a
/// centimetre («1.666»).
pub(crate) fn position_label(value: f64, unit: Unit) -> String {
    let decimals = match unit {
        Unit::Px | Unit::Pt => 0,
        Unit::Mm => 1,
        Unit::Cm | Unit::In => 3,
    };
    let text = format!("{value:.decimals$}");
    let text = if decimals > 0 {
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        text
    };
    if text == "-0" { "0".to_owned() } else { text }
}

/// A ruler's number: whole, or with the decimals the step needs.
pub(crate) fn tick_label(value: f64) -> String {
    let text = format!("{:.2}", value);
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// The extent of `boxes` along a ruler (S49-04): from the least to the
/// greatest x for the top ruler, y for the left one. `None` with no box.
pub(crate) fn band(boxes: &[Rect], axis: GuideAxis) -> Option<(f32, f32)> {
    let range = |r: &Rect| match axis {
        GuideAxis::Horizontal => (r.min.x, r.max.x),
        GuideAxis::Vertical => (r.min.y, r.max.y),
    };
    boxes
        .iter()
        .map(range)
        .reduce(|(a0, a1), (b0, b1)| (a0.min(b0), a1.max(b1)))
}

/// A guide of `axis` at `px`, written in px.
fn guide(axis: GuideAxis, px: f64, locked: bool) -> Guide {
    Guide {
        axis,
        position: Length::new(px, Unit::Px),
        locked,
    }
}

#[cfg(test)]
mod tests;
