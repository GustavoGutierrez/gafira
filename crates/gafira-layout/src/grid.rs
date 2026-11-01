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

/// The line level that shows at a zoom: lines `k * unit / div`, every `every`-th major.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Level {
    pub(crate) unit: f64,
    pub(crate) div: f64,
    pub(crate) every: i64,
}

impl Level {
    /// The distance between two neighbouring lines.
    pub(crate) fn spacing(self, field: &'static str) -> Result<f64, LayoutError> {
        finite(field, self.unit / self.div)
    }

    /// Where line `k` sits, and whether it is major.
    ///
    /// A major line is `j * unit` exactly; multiplying before dividing can miss it by an ulp.
    /// `k` is integer-valued, also negative.
    pub(crate) fn line(self, field: &'static str, k: f64) -> Result<GridLine, LayoutError> {
        let every = self.every as f64;
        let major = k % every == 0.0;
        let position = if major {
            finite(field, (k / every) * self.unit)?
        } else {
            finite(field, k * self.unit / self.div)?
        };
        Ok(GridLine { position, major })
    }
}

/// The lines `k * unit / div` inside `lo..=hi`, both ends included; every `every`-th is major.
fn axis_lines(
    field: &'static str,
    (lo, hi): (f64, f64),
    level: Level,
) -> Result<Vec<GridLine>, LayoutError> {
    let spacing = level.spacing(field)?;
    // The quotients round, so each bound is widened by one line and `contains` decides.
    let first = finite(field, (lo / spacing).floor())? as i64;
    let last = finite(field, (hi / spacing).ceil())? as i64;
    let mut lines = Vec::new();
    for k in first..=last {
        let line = level.line(field, k as f64)?;
        if (lo..=hi).contains(&line.position) {
            lines.push(line);
        }
    }
    Ok(lines)
}

/// The level shown at `zoom` for `grid`, after the step and subdivision ranges.
///
/// `dpi` and `zoom` must already be finite, `dpi > 0` and `zoom` in range. The
/// level is the one `grid_lines` draws, so snapping and drawing never differ.
pub(crate) fn level(grid: &GridSettings, zoom: f64, dpi: f64) -> Result<Level, LayoutError> {
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
    Ok(if major_on_screen < MIN_SPACING_PX {
        let mut multiple = 1.0;
        while major_on_screen * multiple < MIN_SPACING_PX {
            multiple *= 2.0;
        }
        Level {
            unit: finite("step", step * multiple)?,
            div: 1.0,
            every: 1,
        }
    } else if zoom >= MINOR_MIN_ZOOM && major_on_screen / subdivisions >= MIN_SPACING_PX {
        Level {
            unit: step,
            div: subdivisions,
            every: i64::from(grid.subdivisions),
        }
    } else {
        Level {
            unit: step,
            div: 1.0,
            every: 1,
        }
    })
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
    let level = level(grid, zoom, dpi)?;
    Ok(GridLines {
        vertical: axis_lines("view", (view.x0, view.x1), level)?,
        horizontal: axis_lines("view", (view.y0, view.y1), level)?,
    })
}
