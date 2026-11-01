//! Guide positions and the guide generator (FD-03, FD-04).

use gafira_doc::{Guide, GuideAxis, PageSize};
use gafira_geom::{Length, Rect, Unit};

use crate::LayoutError;
use crate::error::{check_rect, finite};
use crate::rects::convert;

/// A guide at a typed position, and that position shown in the ruler unit.
///
/// The guide keeps `written` as typed: same number, same unit, never rounded
/// (FD-03). The second value is the same distance in `ruler`, converted with
/// `dpi`, for the on-screen readout. A non-finite position or DPI, or a
/// conversion that overflows, is an error and no guide is made.
pub fn place_guide(
    axis: GuideAxis,
    written: Length,
    ruler: Unit,
    dpi: f64,
) -> Result<(Guide, Length), LayoutError> {
    finite("position", written.value())?;
    finite("dpi", dpi)?;
    let shown = convert("position", written, ruler, dpi)?;
    let guide = Guide {
        axis,
        position: written,
        locked: false,
    };
    Ok((guide, Length::new(shown, ruler)))
}

/// What the generator was asked for (FD-04). Plain data: no check until [`generate`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuideSpec {
    /// Columns across the page width; 0 gives no vertical guides.
    pub columns: u32,
    /// Rows down the page height; 0 gives no horizontal guides.
    pub rows: u32,
    /// Space between two columns or rows.
    pub gap: Length,
    /// Distance from each page edge to the first and last column or row.
    pub margin: Length,
}

/// Smallest column or row size, in px; the same minimum as a page side.
const MIN_CELL_PX: f64 = 1.0;

/// Guide positions along one axis of `side` px, sorted, each exact position once.
fn axis_positions(
    field: &'static str,
    n: u32,
    side: f64,
    gap: f64,
    margin: f64,
) -> Result<Vec<f64>, LayoutError> {
    if n == 0 {
        return Ok(Vec::new());
    }
    let count = f64::from(n);
    let usable = finite(field, side - 2.0 * margin)?;
    let gaps = finite(field, (count - 1.0) * gap)?;
    let cell = finite(field, (usable - gaps) / count)?;
    if cell < MIN_CELL_PX {
        return Err(LayoutError::OutOfRange { field });
    }
    let mut at = vec![margin, finite(field, side - margin)?];
    for i in 1..n {
        let i = f64::from(i);
        at.push(finite(field, margin + i * cell + (i - 1.0) * gap)?);
        at.push(finite(field, margin + i * (cell + gap))?);
    }
    at.sort_by(f64::total_cmp);
    at.dedup();
    Ok(at)
}

/// The guides of a column and row layout on a page, in px.
///
/// Per axis with `n >= 1` divisions the usable size is `side - 2 * margin` and
/// each cell is `c = (usable - (n - 1) * gap) / n`, which must be at least 1 px
/// (`c = 1` is valid). The axis gets a guide at each margin and two around each
/// gap, `2n` in all; positions are not rounded, and two guides at the same exact
/// position on an axis are emitted once. Columns use the page width and rows the
/// height; `n = 0` gives no guides on that axis. Output lists the vertical
/// guides first, then the horizontal ones, each group ascending.
///
/// Both page sides must be at most `PageSize::MAX_PX` (16 384 px), which keeps
/// each axis to at most 32 768 guides.
///
/// Positions are in px, whatever the unit of `gap` and `margin`.
///
/// `gap` and `margin` must be finite and `>= 0`; both, `dpi` and the page are
/// checked even when no axis uses them. Page margins are not read. Any error
/// returns no guides.
pub fn generate(spec: &GuideSpec, page: Rect, dpi: f64) -> Result<Vec<Guide>, LayoutError> {
    check_rect("page", page)?;
    finite("dpi", dpi)?;
    finite("gap", spec.gap.value())?;
    finite("margin", spec.margin.value())?;
    let gap = convert("gap", spec.gap, Unit::Px, dpi)?;
    let margin = convert("margin", spec.margin, Unit::Px, dpi)?;
    if gap < 0.0 {
        return Err(LayoutError::OutOfRange { field: "gap" });
    }
    if margin < 0.0 {
        return Err(LayoutError::OutOfRange { field: "margin" });
    }
    // `+ 0.0` turns a stored -0.0 into 0.0.
    let (gap, margin) = (gap + 0.0, margin + 0.0);
    let width = finite("page", page.width())?;
    let height = finite("page", page.height())?;
    // Both sides, even when an axis has `n = 0`; before any column width rule.
    if width > PageSize::MAX_PX || height > PageSize::MAX_PX {
        return Err(LayoutError::OutOfRange { field: "page" });
    }
    let columns = axis_positions("columns", spec.columns, width, gap, margin)?;
    let rows = axis_positions("rows", spec.rows, height, gap, margin)?;
    let guide = |axis| {
        move |position| Guide {
            axis,
            position: Length::new(position, Unit::Px),
            locked: false,
        }
    };
    Ok(columns
        .into_iter()
        .map(guide(GuideAxis::Vertical))
        .chain(rows.into_iter().map(guide(GuideAxis::Horizontal)))
        .collect())
}
