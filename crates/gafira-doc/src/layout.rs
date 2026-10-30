//! Page layout data: margins, guides and grid settings.
//!
//! Plain data, like the rest of the model: nothing here validates, and values
//! are stored unchanged. `gafira-layout` operates on these types.

use std::fmt;

use gafira_geom::{Length, Unit};
use serde::{Deserialize, Serialize};

use crate::Dpi;

/// A numeric page-layout field, named in [`PageLayoutError`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageField {
    /// `bleed`.
    Bleed,
    /// `margins.top`.
    MarginTop,
    /// `margins.right`.
    MarginRight,
    /// `margins.bottom`.
    MarginBottom,
    /// `margins.left`.
    MarginLeft,
    /// The `position` of a guide.
    GuidePosition,
    /// `grid.step`.
    GridStep,
    /// `grid.subdivisions`.
    GridSubdivisions,
}

impl fmt::Display for PageField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bleed => "bleed",
            Self::MarginTop => "margin top",
            Self::MarginRight => "margin right",
            Self::MarginBottom => "margin bottom",
            Self::MarginLeft => "margin left",
            Self::GuidePosition => "guide position",
            Self::GridStep => "grid step",
            Self::GridSubdivisions => "grid subdivisions",
        })
    }
}

/// Why page-layout data failed its range check. Nothing is ever clamped.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageLayoutError {
    /// The value is NaN or infinite, or cannot be converted to millimetres.
    NonFinite {
        /// The failing field.
        field: PageField,
    },
    /// The value is finite but outside its range.
    OutOfRange {
        /// The failing field.
        field: PageField,
        /// The value as checked: millimetres for the bleed, the stored number
        /// for the others.
        value: f64,
    },
}

impl fmt::Display for PageLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "{field} is not finite"),
            Self::OutOfRange { field, value } => write!(f, "{field} is {value}, out of range"),
        }
    }
}

impl std::error::Error for PageLayoutError {}

/// The four page margins.
///
/// `linked` is a stored flag: when true the editor keeps the four sides equal.
/// The sides themselves are always all written.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Margins {
    /// Top margin.
    pub top: Length,
    /// Right margin.
    pub right: Length,
    /// Bottom margin.
    pub bottom: Length,
    /// Left margin.
    pub left: Length,
    /// Whether the four sides are edited together. Reads true when absent.
    #[serde(default = "default_linked")]
    pub linked: bool,
}

impl Default for Margins {
    /// 0 px on every side, linked.
    fn default() -> Self {
        let zero = Length::new(0.0, Unit::Px);
        Self {
            top: zero,
            right: zero,
            bottom: zero,
            left: zero,
            linked: true,
        }
    }
}

/// The direction of a guide line.
///
/// Not `Axis`, which names a page side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideAxis {
    /// A vertical line; `position` is measured from the left edge.
    Vertical,
    /// A horizontal line; `position` is measured from the top edge.
    Horizontal,
}

/// One guide line, stored as written: unrounded, in the unit it was typed in.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    /// Line direction.
    pub axis: GuideAxis,
    /// Distance from the page's top-left corner. May lie outside the page.
    pub position: Length,
    /// Whether the guide is protected from dragging. Reads false when absent.
    #[serde(default)]
    pub locked: bool,
}

/// The guides of one page, in the order they were added. Duplicates are kept.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GuideSet(pub Vec<Guide>);

/// How grid lines are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridStyle {
    /// Continuous lines.
    #[default]
    Lines,
    /// Dots at the intersections.
    Dots,
}

/// The background grid of a page. Never exported.
///
/// Every field reads its [`Default`] value when absent (ADR 0006).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GridSettings {
    /// Whether the grid is drawn.
    pub visible: bool,
    /// Distance between major lines, in its own unit.
    pub step: Length,
    /// Divisions of each step.
    pub subdivisions: u8,
    /// Lines or dots.
    pub style: GridStyle,
    /// Whether snapping attracts to the grid.
    pub snap: bool,
}

impl Default for GridSettings {
    /// Hidden, 10 px step, 1 subdivision, lines, snapping on.
    fn default() -> Self {
        Self {
            visible: false,
            step: Length::new(10.0, Unit::Px),
            subdivisions: 1,
            style: GridStyle::Lines,
            snap: true,
        }
    }
}

fn default_linked() -> bool {
    true
}

/// The default page bleed: 0 mm.
pub(crate) fn default_bleed() -> Length {
    Length::new(0.0, Unit::Mm)
}

/// Largest page bleed, in millimetres.
const MAX_BLEED_MM: f64 = 10.0;
/// Smallest and largest grid step, in the unit of the step.
const MIN_STEP: f64 = 1.0;
const MAX_STEP: f64 = 500.0;
/// Fewest and most grid subdivisions.
const MIN_SUBDIVISIONS: u8 = 1;
const MAX_SUBDIVISIONS: u8 = 10;

/// `value` when finite, else the non-finite error of `field`.
fn finite(field: PageField, value: f64) -> Result<f64, PageLayoutError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(PageLayoutError::NonFinite { field })
    }
}

/// `Ok` when `value` is within `lo..=hi`, else the range error of `field`.
fn within(field: PageField, value: f64, lo: f64, hi: f64) -> Result<(), PageLayoutError> {
    if (lo..=hi).contains(&value) {
        Ok(())
    } else {
        Err(PageLayoutError::OutOfRange { field, value })
    }
}

/// Checks a page bleed: 0 to 10 mm at `dpi`, both ends included.
pub fn check_bleed(bleed: Length, dpi: Dpi) -> Result<(), PageLayoutError> {
    let field = PageField::Bleed;
    let mm = bleed
        .convert(Unit::Mm, f64::from(dpi.get()))
        .map_err(|_| PageLayoutError::NonFinite { field })?
        .value();
    within(field, mm, 0.0, MAX_BLEED_MM)
}

impl Margins {
    /// Checks the four sides (top, right, bottom, left): finite and not
    /// negative, in their own unit. There is no upper bound.
    pub fn check(&self) -> Result<(), PageLayoutError> {
        let sides = [
            (PageField::MarginTop, self.top),
            (PageField::MarginRight, self.right),
            (PageField::MarginBottom, self.bottom),
            (PageField::MarginLeft, self.left),
        ];
        for (field, side) in sides {
            let value = finite(field, side.value())?;
            within(field, value, 0.0, f64::INFINITY)?;
        }
        Ok(())
    }
}

impl GuideSet {
    /// Checks every guide position (in order): finite. Any finite value may
    /// lie outside the page.
    pub fn check(&self) -> Result<(), PageLayoutError> {
        for guide in &self.0 {
            finite(PageField::GuidePosition, guide.position.value())?;
        }
        Ok(())
    }
}

impl GridSettings {
    /// Checks the step (1 to 500 in its own unit) and then the subdivisions
    /// (1 to 10), both ends included.
    pub fn check(&self) -> Result<(), PageLayoutError> {
        let step = finite(PageField::GridStep, self.step.value())?;
        within(PageField::GridStep, step, MIN_STEP, MAX_STEP)?;
        within(
            PageField::GridSubdivisions,
            f64::from(self.subdivisions),
            f64::from(MIN_SUBDIVISIONS),
            f64::from(MAX_SUBDIVISIONS),
        )
    }
}
