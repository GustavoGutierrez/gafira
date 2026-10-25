//! Layer-2 path operations for Gafira.
//!
//! Pure engine code: no I/O, no `unsafe`, `f64` everywhere. The crate provides
//! path boolean operations, stroke outlining, path simplification and geometric
//! hit testing over the Bézier paths that `gafira-geom` speaks.

#![forbid(unsafe_code)]

mod boolean;
mod error;
mod hittest;
mod path;
mod simplify;
mod stroke;
mod validate;

pub use error::{InvalidNumber, NonFiniteValue, PathOpError};
pub use hittest::Containment;
pub use path::Path;
pub use stroke::StrokeStyle;

pub use gafira_geom::{Affine, Point, Rect, Vec2};
pub use kurbo::{BezPath, Cap, Join, PathEl};

/// The rule that decides which points of a path the path covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FillRule {
    /// A point is inside when the number of crossings of the path is odd.
    EvenOdd,
    /// A point is inside when the winding number of the path around it is not
    /// zero.
    NonZero,
}

/// The four set operations over two paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BooleanOp {
    /// A point is in the union when it is in either operand.
    Union,
    /// A point is in the intersection when it is in both operands.
    Intersection,
    /// A point is in the difference when it is in the first operand and not in
    /// the second.
    Difference,
    /// A point is in the exclusive-or when it is in one operand but not the
    /// other.
    Xor,
}

/// The default tolerance, in the unit of the paths (pixels).
///
/// Every operation takes the tolerance explicitly or uses this constant; the
/// crate never mixes tolerances inside a single call. For the stroke outline
/// and the simplification it is the distance the result may move from the exact
/// answer; the arcs of round caps and round joins are fitted at this distance
/// too, so a finer tolerance gives more curve segments there. For the boolean operations it is the accuracy of the sweep, not a
/// bound on the distance to the exact result: measured, and not proven, the
/// result matches the exact boundary far below the tolerance for straight edges,
/// and for a curve that runs parallel to the sweep direction at a crossing the
/// deviation grows as the square root of the tolerance (see
/// [`Path::boolean_op`]).
pub const DEFAULT_TOLERANCE: f64 = 1e-3;
