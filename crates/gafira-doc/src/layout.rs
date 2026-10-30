//! Page layout data: margins, guides and grid settings.
//!
//! Plain data, like the rest of the model: nothing here validates, and values
//! are stored unchanged. `gafira-layout` operates on these types.

use gafira_geom::{Length, Unit};
use serde::{Deserialize, Serialize};

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
    /// Whether the four sides are edited together.
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
    /// Whether the guide is protected from dragging.
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
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

/// The default page bleed: 0 mm.
pub(crate) fn default_bleed() -> Length {
    Length::new(0.0, Unit::Mm)
}
