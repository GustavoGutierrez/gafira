//! Layer-2 layout primitives for Gafira.
//!
//! Pure functions over rectangles: no I/O, no `unsafe`, no serde, and nothing
//! here mutates a document. Every public function checks its numbers first and
//! returns `Err` for the whole call (ARQ-110).

#![forbid(unsafe_code)]

mod attractors;
mod bounds;
mod error;
pub mod grid;
pub mod guides;
mod rects;
pub mod snap;

pub use attractors::attractors;
pub use bounds::node_bounds;
pub use error::LayoutError;
pub use grid::{GridLine, GridLines, grid_lines};
pub use guides::{GuideSpec, generate, place_guide};
pub use rects::{Overflow, bleed_rect, margin_rect, overflow, safe_rect};
pub use snap::{AxisSnap, SnapInput, SnapKind, SnapResult, SnapTarget, snap, snap_rotation};

pub use gafira_doc::{GridSettings, GridStyle, Guide, GuideAxis};
pub use gafira_geom::{Length, Rect, Unit};
