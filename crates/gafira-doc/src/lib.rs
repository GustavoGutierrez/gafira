//! Layer-1 document model for Gafira: `Document -> Page -> Node`.
//!
//! Pure domain code: no I/O, no `unsafe`. Depends only on `gafira-geom` and
//! `gafira-color`; geometry types are always imported through `gafira_geom`.

#![forbid(unsafe_code)]

mod blend;
mod dpi;
mod element;
mod ids;
mod node;
mod page;
mod page_size;
mod placement;

pub use blend::BlendMode;
pub use dpi::Dpi;
pub use element::ElementBase;
pub use ids::{NodeId, PageId};
pub use node::{ExtensionNode, GroupNode, Node, NodeKind, ShapeGeometry, ShapeNode};
pub use page::Page;
pub use page_size::{Axis, PageSize, PageSizeError};
pub use placement::{Flip, Placement};
