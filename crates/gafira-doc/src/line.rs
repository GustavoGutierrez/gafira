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

/// One of the five points of a shape the magnet of a line end prefers
/// (spec 058); an end can also stick to any point of the outline.
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

    /// Where the point lies in the unit box of a shape.
    #[must_use]
    pub fn unit(self) -> [f64; 2] {
        match self {
            Self::Top => [0.5, 0.0],
            Self::Right => [1.0, 0.5],
            Self::Bottom => [0.5, 1.0],
            Self::Left => [0.0, 0.5],
            Self::Center => [0.5, 0.5],
        }
    }
}

/// The shape and the point a line end sticks to: any point of its outline
/// or one of its five points, in the unit box of the shape, so it moves,
/// turns and scales with it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    pub node: NodeId,
    pub at: [f64; 2],
}

impl Attachment {
    /// Stuck to one of the five points of `node`.
    #[must_use]
    pub fn anchor(node: NodeId, anchor: Anchor) -> Self {
        Self {
            node,
            at: anchor.unit(),
        }
    }
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

    /// Where the point `at` of the unit box of `node` lies on `page`, in
    /// page px: turned and flipped with the node. `None` when the node is
    /// not on the page or is itself a line.
    #[must_use]
    pub fn anchor_point(page: &Page, node: NodeId, at: [f64; 2]) -> Option<Point> {
        let target = page.node(node)?;
        if matches!(target.kind, crate::NodeKind::Line(_)) {
            return None;
        }
        let p = &target.base.placement;
        Some(p.to_transform() * Point::new(at[0] * p.w, at[1] * p.h))
    }

    /// Both ends in page px for a line placed at `placement` on `page`:
    /// a connected end at its shape's point, else at its point in the box.
    #[must_use]
    pub fn ends(&self, page: &Page, placement: &Placement) -> [Point; 2] {
        let transform = placement.to_transform();
        [self.start, self.end].map(|end| {
            end.attached
                .and_then(|a| Self::anchor_point(page, a.node, a.at))
                .unwrap_or_else(|| {
                    let [x, y] = end.at;
                    transform * Point::new(x * placement.w, y * placement.h)
                })
        })
    }
}
