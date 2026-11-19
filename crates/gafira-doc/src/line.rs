//! Lines that connect to shapes (spec 058, D-98): two ends, each with a cap
//! and, when connected, the shape and the point it sticks to.

use gafira_color::Color;
use gafira_geom::Point;
use serde::{Deserialize, Serialize};

use crate::{NodeId, Page, Placement};

/// What a line draws at one of its ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineCap {
    #[default]
    None,
    /// An open arrowhead.
    Arrow,
    /// A hollow circle.
    Circle,
    /// A hollow diamond.
    Diamond,
    /// A filled diamond.
    DiamondFilled,
    /// A hollow square.
    Square,
}

/// One of the five points of a shape a line end connects to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    Top,
    Right,
    Bottom,
    Left,
    Center,
}

impl Anchor {
    /// The five points, in the order they are tried.
    pub const ALL: [Self; 5] = [
        Self::Top,
        Self::Right,
        Self::Bottom,
        Self::Left,
        Self::Center,
    ];

    /// Where the point lies in a box of `w` by `h`, from its top-left corner.
    #[must_use]
    pub fn local(self, w: f64, h: f64) -> Point {
        let (x, y) = match self {
            Self::Top => (0.5, 0.0),
            Self::Right => (1.0, 0.5),
            Self::Bottom => (0.5, 1.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
        };
        Point::new(x * w, y * h)
    }
}

/// The shape and the point a line end sticks to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub node: NodeId,
    pub anchor: Anchor,
}

/// One end of a line.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineEnd {
    /// Where the end lies in the unit box of the line's placement, used
    /// when it is not connected or its shape is gone.
    pub at: [f64; 2],
    #[serde(default)]
    pub cap: LineCap,
    /// The shape and point it sticks to, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attached: Option<Attachment>,
}

impl LineEnd {
    /// A loose end at `at` with `cap`.
    #[must_use]
    pub const fn new(at: [f64; 2], cap: LineCap) -> Self {
        Self {
            at,
            cap,
            attached: None,
        }
    }
}

/// A straight line between two ends (spec 058).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineNode {
    pub start: LineEnd,
    pub end: LineEnd,
    pub color: Color,
    /// The width in document px, `0..=MAX_WIDTH`.
    pub width: f64,
}

impl LineNode {
    /// The widest line a document holds, in px.
    pub const MAX_WIDTH: f64 = 1000.0;

    /// Where the point `anchor` of `node` lies on `page`, in page px: its
    /// place in the node's box, turned and flipped with it. `None` when the
    /// node is not on the page or is itself a line.
    #[must_use]
    pub fn anchor_point(page: &Page, node: NodeId, anchor: Anchor) -> Option<Point> {
        let target = page.node(node)?;
        if matches!(target.kind, crate::NodeKind::Line(_)) {
            return None;
        }
        let at = &target.base.placement;
        Some(at.to_transform() * anchor.local(at.w, at.h))
    }

    /// Both ends in page px for a line placed at `placement` on `page`:
    /// a connected end at its shape's point, else at its point in the box.
    #[must_use]
    pub fn ends(&self, page: &Page, placement: &Placement) -> [Point; 2] {
        let transform = placement.to_transform();
        [self.start, self.end].map(|end| {
            end.attached
                .and_then(|a| Self::anchor_point(page, a.node, a.anchor))
                .unwrap_or_else(|| {
                    let [x, y] = end.at;
                    transform * Point::new(x * placement.w, y * placement.h)
                })
        })
    }
}
