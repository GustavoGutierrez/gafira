//! The magnet: per-axis snapping of a moving box (FD-06, D-16, D-28).

use gafira_doc::{GridSettings, Guide, GuideAxis};
use gafira_geom::{Rect, Unit};

use crate::LayoutError;
use crate::error::{check_rect, finite};
use crate::grid::{Level, level};
use crate::rects::convert;

/// What a snap target is. Declared in priority order: the first wins a tie (D-16).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapKind {
    /// A guide.
    Guide,
    /// An edge of the margin rectangle.
    Margin,
    /// An edge or the centre of the page.
    Page,
    /// An edge or the centre of an element.
    Element,
    /// A grid line.
    Grid,
}

/// The target a reference snapped to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapTarget {
    /// Where the target sits on its axis, in px of page.
    pub coordinate: f64,
    /// What the target is.
    pub kind: SnapKind,
}

/// The outcome on one axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisSnap {
    /// How far the box moves: `target - reference`, or 0 without a target.
    pub offset: f64,
    /// The chosen target, if any.
    pub target: Option<SnapTarget>,
}

/// The outcome on both axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapResult {
    /// The horizontal axis.
    pub x: AxisSnap,
    /// The vertical axis.
    pub y: AxisSnap,
}

/// Everything [`snap`] reads. Plain data: nothing is checked until the call.
#[derive(Clone, Copy, Debug)]
pub struct SnapInput<'a> {
    /// The box being moved, in px of page.
    pub moving: Rect,
    /// The page box.
    pub page: Rect,
    /// The margin rectangle, when the page has one.
    pub margins: Option<Rect>,
    /// The boxes of the elements that attract (see `attractors`).
    pub elements: &'a [Rect],
    /// The page guides.
    pub guides: &'a [Guide],
    /// Whether the guides attract.
    pub guides_visible: bool,
    /// The grid of the page.
    pub grid: &'a GridSettings,
    /// Whether the magnet is on (and `Alt` is not held).
    pub active: bool,
    /// Screen px per page unit.
    pub zoom: f64,
    /// Screen px.
    pub threshold: f64,
    /// Dots per inch.
    pub dpi: f64,
}

/// Zoom range, as a ratio: 5 % to 3200 % (FD-06, FD-07).
const ZOOM_RANGE: std::ops::RangeInclusive<f64> = 0.05..=32.0;

/// One candidate: a target reached from one reference of the moving box.
#[derive(Clone, Copy)]
struct Candidate {
    distance: f64,
    kind: SnapKind,
    coordinate: f64,
    reference: usize,
    offset: f64,
}

impl Candidate {
    /// Smaller distance, then higher priority, then smaller coordinate, then earlier reference.
    /// All comparisons are exact.
    fn beats(&self, other: &Candidate) -> bool {
        if self.distance != other.distance {
            return self.distance < other.distance;
        }
        if self.kind != other.kind {
            return self.kind < other.kind;
        }
        if self.coordinate != other.coordinate {
            return self.coordinate < other.coordinate;
        }
        self.reference < other.reference
    }
}

/// The edges and the centre of a box along one axis, checking the centre.
fn span(field: &'static str, lo: f64, hi: f64) -> Result<[f64; 3], LayoutError> {
    Ok([lo, finite(field, (lo + hi) / 2.0)?, hi])
}

/// Checks all the numbers of the input, then the boxes, then the DPI and the ranges.
fn check(input: &SnapInput<'_>) -> Result<(), LayoutError> {
    let all_finite = |field: &'static str, r: Rect| -> Result<(), LayoutError> {
        for v in [r.x0, r.y0, r.x1, r.y1] {
            finite(field, v)?;
        }
        Ok(())
    };
    all_finite("moving", input.moving)?;
    all_finite("page", input.page)?;
    if let Some(m) = input.margins {
        all_finite("margins", m)?;
    }
    for e in input.elements {
        all_finite("element", *e)?;
    }
    for g in input.guides {
        finite("guide", g.position.value())?;
    }
    finite("step", input.grid.step.value())?;
    finite("zoom", input.zoom)?;
    finite("threshold", input.threshold)?;
    finite("dpi", input.dpi)?;
    check_rect("moving", input.moving)?;
    check_rect("page", input.page)?;
    if let Some(m) = input.margins {
        check_rect("margins", m)?;
    }
    for e in input.elements {
        check_rect("element", *e)?;
    }
    if input.dpi <= 0.0 {
        return Err(LayoutError::OutOfRange { field: "dpi" });
    }
    if input.threshold <= 0.0 {
        return Err(LayoutError::OutOfRange { field: "threshold" });
    }
    if !ZOOM_RANGE.contains(&input.zoom) {
        return Err(LayoutError::OutOfRange { field: "zoom" });
    }
    Ok(())
}

/// The line of `level` nearest to `at`; of two equally near lines, the smaller.
///
/// The quotient `at / spacing` rounds, so the two lines around `at` are looked for
/// among four neighbours. No loop depends on the size of `at`.
fn grid_target(level: Level, at: f64) -> Result<f64, LayoutError> {
    let spacing = level.spacing("step")?;
    let k = finite("step", at / spacing)?.floor();
    let mut best: Option<(f64, f64)> = None;
    for k in [k - 1.0, k, k + 1.0, k + 2.0] {
        let line = level.line("step", k)?.position;
        let distance = finite("step", (line - at).abs())?;
        if best.is_none_or(|(d, p)| distance < d || (distance == d && line < p)) {
            best = Some((distance, line));
        }
    }
    best.map(|(_, line)| line)
        .ok_or(LayoutError::NonFinite { field: "step" })
}

/// The best candidate of one axis, if any lies within `limit`.
fn resolve(
    references: [f64; 3],
    targets: &[(f64, SnapKind)],
    limit: f64,
) -> Result<AxisSnap, LayoutError> {
    let mut best: Option<Candidate> = None;
    for (reference, &at) in references.iter().enumerate() {
        for &(coordinate, kind) in targets {
            let offset = finite("distance", coordinate - at)?;
            let candidate = Candidate {
                distance: offset.abs(),
                kind,
                coordinate,
                reference,
                offset,
            };
            if candidate.distance <= limit && best.is_none_or(|b| candidate.beats(&b)) {
                best = Some(candidate);
            }
        }
    }
    // `+ 0.0` turns a -0.0 offset into 0.0.
    Ok(best.map_or(
        AxisSnap {
            offset: 0.0,
            target: None,
        },
        |c| AxisSnap {
            offset: c.offset + 0.0,
            target: Some(SnapTarget {
                coordinate: c.coordinate,
                kind: c.kind,
            }),
        },
    ))
}

/// Snaps each axis of the moving box to the nearest target within the threshold.
///
/// Each axis is resolved on its own: the left edge, centre and right edge of
/// `moving` (top, middle and bottom on `y`) are compared with the targets of
/// that axis. The nearest target with `|target - reference| <= threshold /
/// zoom` wins; at an exact tie the kind decides (guide, margin, page,
/// element), then the smaller coordinate, then the reference (left before
/// centre before right). All comparisons are exact `f64`.
///
/// Targets of an axis: the guides of that axis in px (when `guides_visible`),
/// the two edges of `margins`, the two edges and the centre of `page` and of
/// each box of `elements`, and, when the grid is visible and set to snap, the
/// drawn grid line nearest to each reference (see [`crate::grid::grid_lines`]).
/// An inactive magnet (`active` false) gives the offset 0 and no target, after
/// the same checks. A guide that does not attract is still checked.
///
/// `threshold` must be finite and above 0 (the 2 to 16 range belongs to the
/// settings), `zoom` in `[0.05, 32]`, `dpi` above 0 and the grid step and
/// subdivisions in the ranges of `grid_lines`. The order of checks is: all
/// numbers, then the boxes, then the DPI, then the other ranges, then the
/// computed values (`threshold / zoom`, every centre, every guide in px and
/// every distance). Any error returns no result.
pub fn snap(input: &SnapInput<'_>) -> Result<SnapResult, LayoutError> {
    check(input)?;
    let level = level(input.grid, input.zoom, input.dpi)?;
    let limit = finite("threshold", input.threshold / input.zoom)?;
    let moving_x = span("moving", input.moving.x0, input.moving.x1)?;
    let moving_y = span("moving", input.moving.y0, input.moving.y1)?;
    let page_x = span("page", input.page.x0, input.page.x1)?;
    let page_y = span("page", input.page.y0, input.page.y1)?;
    let mut xs: Vec<(f64, SnapKind)> = Vec::new();
    let mut ys: Vec<(f64, SnapKind)> = Vec::new();
    for g in input.guides {
        let at = convert("guide", g.position, Unit::Px, input.dpi)?;
        if input.guides_visible {
            match g.axis {
                GuideAxis::Vertical => xs.push((at, SnapKind::Guide)),
                GuideAxis::Horizontal => ys.push((at, SnapKind::Guide)),
            }
        }
    }
    if let Some(m) = input.margins {
        xs.extend([(m.x0, SnapKind::Margin), (m.x1, SnapKind::Margin)]);
        ys.extend([(m.y0, SnapKind::Margin), (m.y1, SnapKind::Margin)]);
    }
    xs.extend(page_x.map(|v| (v, SnapKind::Page)));
    ys.extend(page_y.map(|v| (v, SnapKind::Page)));
    for e in input.elements {
        xs.extend(span("element", e.x0, e.x1)?.map(|v| (v, SnapKind::Element)));
        ys.extend(span("element", e.y0, e.y1)?.map(|v| (v, SnapKind::Element)));
    }
    if !input.active {
        let none = AxisSnap {
            offset: 0.0,
            target: None,
        };
        return Ok(SnapResult { x: none, y: none });
    }
    if input.grid.visible && input.grid.snap {
        // Each reference gets its nearest drawn line, which competes like any target.
        for at in moving_x {
            xs.push((grid_target(level, at)?, SnapKind::Grid));
        }
        for at in moving_y {
            ys.push((grid_target(level, at)?, SnapKind::Grid));
        }
    }
    Ok(SnapResult {
        x: resolve(moving_x, &xs, limit)?,
        y: resolve(moving_y, &ys, limit)?,
    })
}
