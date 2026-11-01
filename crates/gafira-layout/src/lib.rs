//! Layer-2 layout primitives for Gafira.
//!
//! Pure functions over rectangles: no I/O, no `unsafe`, no serde, and nothing
//! here mutates a document. Every public function checks its numbers first and
//! returns `Err` for the whole call (ARQ-110).

#![forbid(unsafe_code)]

mod bounds;
mod error;
mod rects;

pub use bounds::node_bounds;
pub use error::LayoutError;
pub use rects::{Overflow, bleed_rect, margin_rect, overflow, safe_rect};

pub use gafira_geom::{Length, Rect, Unit};
