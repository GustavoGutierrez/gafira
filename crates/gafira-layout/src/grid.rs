//! Visible grid lines for a view and zoom (FD-07).

use gafira_doc::GridSettings;
use gafira_geom::{Rect, Unit};

use crate::LayoutError;
use crate::error::{check_rect, finite};
use crate::rects::convert;

/// One grid line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLine {
    /// Distance from the page's top-left corner, in px.
    pub position: f64,
    /// Whether the line belongs to the major grid.
    pub major: bool,
}

/// The lines to draw: `vertical` lines have an x position, `horizontal` a y position.
#[derive(Clone, Debug, PartialEq)]
pub struct GridLines {
    /// Lines at fixed x, ascending.
    pub vertical: Vec<GridLine>,
    /// Lines at fixed y, ascending.
    pub horizontal: Vec<GridLine>,
}

/// Zoom range, as a ratio: 5 % to 3200 % (FD-07).
const ZOOM_RANGE: std::ops::RangeInclusive<f64> = 0.05..=32.0;
/// Step range, in the unit of the step (FD-07).
const STEP_RANGE: std::ops::RangeInclusive<f64> = 1.0..=500.0;
/// Subdivision range (FD-07).
const SUBDIVISION_RANGE: std::ops::RangeInclusive<u8> = 1..=10;
/// Below this zoom only the major grid is shown.
const MINOR_MIN_ZOOM: f64 = 0.25;
/// A level closer than this on screen, in px, is not shown.
const MIN_SPACING_PX: f64 = 6.0;

/// Largest view on screen, in px (width, height): the FD-07 (c) domain.
const MAX_VIEW_PX: (f64, f64) = (3840.0, 2160.0);
/// Screen px added to each limit, so a view computed as `screen / zoom` still fits.
const VIEW_EPSILON_PX: f64 = 1e-6;

/// The lines `k * unit / div` inside `lo..=hi`, both ends included; every `every`-th is major.
fn axis_lines(
    field: &'static str,
    (lo, hi): (f64, f64),
    unit: f64,
    div: f64,
    every: i64,
) -> Result<Vec<GridLine>, LayoutError> {
    let spacing = finite(field, unit / div)?;
    // The quotients round, so each bound is widened by one line and `contains` decides.
    let first = finite(field, (lo / spacing).floor())? as i64;
    let last = finite(field, (hi / spacing).ceil())? as i64;
    let mut lines = Vec::new();
    for k in first..=last {
        let major = k % every == 0;
        // A major line is `j * unit` exactly; multiplying before dividing can miss it by an ulp.
        let position = if major {
            finite(field, (k / every) as f64 * unit)?
        } else {
            finite(field, k as f64 * unit / div)?
        };
        if (lo..=hi).contains(&position) {
            lines.push(GridLine { position, major });
        }
    }
    Ok(lines)
}

/// The grid lines to draw over `view`, a region of the page in px.
///
/// `zoom` is a ratio in `[0.05, 32]`. Lines sit at `k * step` (the step
/// converted to px with `dpi`) and, when the minor grid shows, at
/// `k * step / subdivisions`; the page corner is the origin. Below a zoom of
/// 0.25 only the major grid shows; from 0.25 the minor one shows too. A level
/// with a screen spacing under 6 px is hidden. When the major level does not
/// fit either, the lines are the smallest `step * 2^k` with a spacing of at
/// least 6 px, all major.
///
/// `view` may extend past the page on any side: the lines go on at negative
/// `k`, and a line on a whole multiple of the step used is major, also for
/// negative `k`. Positions are in page px whatever the unit of the step.
///
/// `view` may be at most 3840 x 2160 px on screen (`width * zoom`, `height * zoom`),
/// compared per axis with a tolerance of 1e-6 px, which absorbs the rounding of a
/// view computed as `screen / zoom`. Both edges of `view` are inside it.
///
/// `grid.step` must lie in 1 to 500 in its own unit and `grid.subdivisions` in
/// 1 to 10; `visible`, `style` and `snap` are not read. Any error returns no lines.
pub fn grid_lines(
    grid: &GridSettings,
    view: Rect,
    zoom: f64,
    dpi: f64,
) -> Result<GridLines, LayoutError> {
    check_rect("view", view)?;
    finite("zoom", zoom)?;
    finite("dpi", dpi)?;
    finite("step", grid.step.value())?;
    if dpi <= 0.0 {
        return Err(LayoutError::OutOfRange { field: "dpi" });
    }
    if !ZOOM_RANGE.contains(&zoom) {
        return Err(LayoutError::OutOfRange { field: "zoom" });
    }
    // Screen size per axis: the FD-07 (c) domain. A product that overflows is non-finite.
    let on_screen_w = finite("view", view.width() * zoom)?;
    let on_screen_h = finite("view", view.height() * zoom)?;
    if on_screen_w > MAX_VIEW_PX.0 + VIEW_EPSILON_PX
        || on_screen_h > MAX_VIEW_PX.1 + VIEW_EPSILON_PX
    {
        return Err(LayoutError::OutOfRange { field: "view" });
    }
    if !STEP_RANGE.contains(&grid.step.value()) {
        return Err(LayoutError::OutOfRange { field: "step" });
    }
    if !SUBDIVISION_RANGE.contains(&grid.subdivisions) {
        return Err(LayoutError::OutOfRange {
            field: "subdivisions",
        });
    }
    let step = convert("step", grid.step, Unit::Px, dpi)?;
    let subdivisions = f64::from(grid.subdivisions);
    let major_on_screen = finite("step", step * zoom)?;
    if major_on_screen <= 0.0 {
        return Err(LayoutError::OutOfRange { field: "step" });
    }
    // (line spacing numerator, divisor, major every n lines)
    let (unit, div, every) = if major_on_screen < MIN_SPACING_PX {
        let mut multiple = 1.0;
        while major_on_screen * multiple < MIN_SPACING_PX {
            multiple *= 2.0;
        }
        (finite("step", step * multiple)?, 1.0, 1)
    } else if zoom >= MINOR_MIN_ZOOM && major_on_screen / subdivisions >= MIN_SPACING_PX {
        (step, subdivisions, i64::from(grid.subdivisions))
    } else {
        (step, 1.0, 1)
    };
    Ok(GridLines {
        vertical: axis_lines("view", (view.x0, view.x1), unit, div, every)?,
        horizontal: axis_lines("view", (view.y0, view.y1), unit, div, every)?,
    })
}
