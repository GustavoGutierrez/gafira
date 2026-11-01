//! Bleed, safe zone, margin rectangles and overflow (FD-05, FD-09).

use gafira_doc::Margins;
use gafira_geom::{ConversionError, Length, Rect, Unit};

use crate::LayoutError;
use crate::error::{check_rect, finite};

/// Largest bleed, in millimetres (FD-05).
const MAX_BLEED_MM: f64 = 10.0;

pub(crate) fn convert(
    field: &'static str,
    len: Length,
    to: Unit,
    dpi: f64,
) -> Result<f64, LayoutError> {
    len.convert(to, dpi)
        .map(Length::value)
        .map_err(|e| match e {
            ConversionError::InvalidDpi(_) => LayoutError::OutOfRange { field: "dpi" },
            _ => LayoutError::NonFinite { field },
        })
}

/// Checks a computed rectangle: every corner finite, then no axis of `page`
/// with a size above 0 left with a size of 0 or less after rounding.
fn shrunk(field: &'static str, page: Rect, r: Rect) -> Result<Rect, LayoutError> {
    for v in [r.x0, r.y0, r.x1, r.y1] {
        finite(field, v)?;
    }
    if (page.width() > 0.0 && r.x1 <= r.x0) || (page.height() > 0.0 && r.y1 <= r.y0) {
        return Err(LayoutError::NoArea);
    }
    Ok(r)
}

/// The page rectangle grown by the bleed on all four sides.
///
/// The bleed must lie in 0 to 10 mm, both ends included. `dpi` converts the
/// bleed to px.
pub fn bleed_rect(page: Rect, bleed: Length, dpi: f64) -> Result<Rect, LayoutError> {
    check_rect("page", page)?;
    finite("dpi", dpi)?;
    finite("bleed", bleed.value())?;
    let mm = convert("bleed", bleed, Unit::Mm, dpi)?;
    if !(0.0..=MAX_BLEED_MM).contains(&mm) {
        return Err(LayoutError::OutOfRange { field: "bleed" });
    }
    let b = convert("bleed", bleed, Unit::Px, dpi)?;
    check_rect(
        "bleed",
        Rect::new(page.x0 - b, page.y0 - b, page.x1 + b, page.y1 + b),
    )
}

/// The page rectangle shrunk by `percent` of its width left and right, and of
/// its height top and bottom. `percent` is 5.0 for 5 % and must lie in `[0, 50)`.
pub fn safe_rect(page: Rect, percent: f64) -> Result<Rect, LayoutError> {
    check_rect("page", page)?;
    finite("percent", percent)?;
    if !(0.0..50.0).contains(&percent) {
        return Err(LayoutError::OutOfRange { field: "percent" });
    }
    let dx = page.width() * percent / 100.0;
    let dy = page.height() * percent / 100.0;
    shrunk(
        "percent",
        page,
        Rect::new(page.x0 + dx, page.y0 + dy, page.x1 - dx, page.y1 - dy),
    )
}

/// The page rectangle shrunk by each margin. `dpi` converts margins to px.
///
/// Per axis, errors with [`LayoutError::NoArea`] when the two margins add up to
/// more than 0 and reach the size; a sum of 0 keeps the box, also at size 0.
pub fn margin_rect(page: Rect, margins: &Margins, dpi: f64) -> Result<Rect, LayoutError> {
    check_rect("page", page)?;
    finite("dpi", dpi)?;
    let sides = [
        ("margins.top", margins.top),
        ("margins.right", margins.right),
        ("margins.bottom", margins.bottom),
        ("margins.left", margins.left),
    ];
    for (field, len) in sides {
        finite(field, len.value())?;
    }
    let mut px = [0.0; 4];
    for (slot, (field, len)) in px.iter_mut().zip(sides) {
        *slot = convert(field, len, Unit::Px, dpi)?;
        if *slot < 0.0 {
            return Err(LayoutError::OutOfRange { field });
        }
    }
    let [top, right, bottom, left] = px;
    let (width, height) = (
        finite("page", page.width())?,
        finite("page", page.height())?,
    );
    let (across, down) = (
        finite("margins", left + right)?,
        finite("margins", top + bottom)?,
    );
    if (across > 0.0 && across >= width) || (down > 0.0 && down >= height) {
        return Err(LayoutError::NoArea);
    }
    shrunk(
        "margins",
        page,
        Rect::new(
            page.x0 + left,
            page.y0 + top,
            page.x1 - right,
            page.y1 - bottom,
        ),
    )
}

/// How far a box sticks out of a rectangle, per side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overflow {
    /// Excess past the left edge, `>= 0`.
    pub left: f64,
    /// Excess past the top edge, `>= 0`.
    pub top: f64,
    /// Excess past the right edge, `>= 0`.
    pub right: f64,
    /// Excess past the bottom edge, `>= 0`.
    pub bottom: f64,
    /// Whether the box shares no area with the rectangle.
    pub fully_outside: bool,
}

impl Overflow {
    /// Whether any side sticks out (the warning of FD-05 and FD-09).
    #[must_use]
    pub fn exceeds(&self) -> bool {
        self.left > 0.0 || self.top > 0.0 || self.right > 0.0 || self.bottom > 0.0
    }
}

/// The excess of `bounds` past each side of `rect`.
///
/// A box edge on the limit does not overflow. The box is fully outside when it
/// shares no area with `rect`, so a box touching only by an edge is outside.
pub fn overflow(bounds: Rect, rect: Rect) -> Result<Overflow, LayoutError> {
    check_rect("bounds", bounds)?;
    check_rect("rect", rect)?;
    // Check each difference before it is clamped: `-inf` must not turn into 0.
    let edge = |v: f64| finite("overflow", v);
    let (left, top) = (edge(rect.x0 - bounds.x0)?, edge(rect.y0 - bounds.y0)?);
    let (right, bottom) = (edge(bounds.x1 - rect.x1)?, edge(bounds.y1 - rect.y1)?);
    let excess = |v: f64| if v > 0.0 { v } else { 0.0 };
    Ok(Overflow {
        left: excess(left),
        top: excess(top),
        right: excess(right),
        bottom: excess(bottom),
        fully_outside: bounds.x0 >= rect.x1
            || bounds.x1 <= rect.x0
            || bounds.y0 >= rect.y1
            || bounds.y1 <= rect.y0,
    })
}
