//! The one typed error of the crate.

use std::fmt;

use gafira_doc::NodeId;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_geom::Rect;

/// Why a layout call failed. `field` names the input at fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// An input, or an intermediate result, is NaN or infinite.
    NonFinite {
        /// The offending input.
        field: &'static str,
    },
    /// A box has `x1 < x0` or `y1 < y0`.
    InvalidBox {
        /// The offending input.
        field: &'static str,
    },
    /// A finite input lies outside its allowed range.
    OutOfRange {
        /// The offending input.
        field: &'static str,
    },
    /// The margins leave a rectangle with no area.
    NoArea,
    /// The node, or one of its children, is not in the page.
    UnknownNode(NodeId),
    /// A group contains itself.
    Cycle(NodeId),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field } => write!(f, "{field} is not finite"),
            Self::InvalidBox { field } => write!(f, "{field} has a negative size"),
            Self::OutOfRange { field } => write!(f, "{field} is out of range"),
            Self::NoArea => f.write_str("the margins leave no area"),
            Self::UnknownNode(id) => write!(f, "node {id} is not in the page"),
            Self::Cycle(id) => write!(f, "group {id} contains itself"),
        }
    }
}

impl std::error::Error for LayoutError {}

impl ToMessage for LayoutError {
    fn to_message(&self) -> Message {
        match self {
            Self::NonFinite { field } => {
                msg!("error.layout.non_finite", field = Text(field.to_string()))
            }
            Self::InvalidBox { field } => {
                msg!("error.layout.invalid_box", field = Text(field.to_string()))
            }
            Self::OutOfRange { field } => {
                msg!("error.layout.out_of_range", field = Text(field.to_string()))
            }
            Self::NoArea => msg!("error.layout.no_area"),
            Self::UnknownNode(id) => msg!("error.layout.unknown_node", node = Text(id.to_string())),
            Self::Cycle(id) => msg!("error.layout.cycle", node = Text(id.to_string())),
        }
    }
}

/// `Ok(value)` when finite.
pub(crate) fn finite(field: &'static str, value: f64) -> Result<f64, LayoutError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(LayoutError::NonFinite { field })
    }
}

/// Checks that the four coordinates are finite, then that the box is not inverted.
pub(crate) fn check_rect(field: &'static str, r: Rect) -> Result<Rect, LayoutError> {
    for v in [r.x0, r.y0, r.x1, r.y1] {
        finite(field, v)?;
    }
    if r.x1 < r.x0 || r.y1 < r.y0 {
        return Err(LayoutError::InvalidBox { field });
    }
    Ok(r)
}
