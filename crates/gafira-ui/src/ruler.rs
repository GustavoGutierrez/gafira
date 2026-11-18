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

/// The ruler step in px of page at `zoom`: 1, 2 or 5 times a power of
/// ten, the smallest at least [`TICK_SPACING`] screen px long.
fn step(zoom: f64) -> f64 {
    let least = TICK_SPACING / zoom;
    let mut decade = 1.0_f64;
    // At the smallest zoom (5 %) this ends at 2000; the bound keeps a
    // broken zoom from spinning.
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
pub(crate) fn ticks(origin: f32, zoom: f32, from: f32, to: f32) -> Vec<(f32, i64)> {
    let zoom = f64::from(zoom);
    if !(zoom.is_finite() && zoom > 0.0) {
        return Vec::new();
    }
    let step = step(zoom);
    let value = |screen: f32| f64::from(screen - origin) / zoom;
    let first = (value(from) / step).ceil() as i64;
    let last = (value(to) / step).floor() as i64;
    (first..=last)
        .map(|k| {
            let at = k as f64 * step;
            (origin + (at * zoom) as f32, at as i64)
        })
        .collect()
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
