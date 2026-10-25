//! Layer-1 document model for Gafira: `Document -> Page -> Node`.
//!
//! Pure domain code: no I/O, no `unsafe`. Depends only on `gafira-geom` and
//! `gafira-color`; geometry types are always imported through `gafira_geom`.

#![forbid(unsafe_code)]

mod arena;
mod blend;
mod document;
mod dpi;
mod edit;
mod element;
mod error;
mod ids;
mod json;
mod node;
mod page;
mod page_size;
mod placement;
mod validate;

pub use blend::BlendMode;
pub use document::Document;
pub use dpi::Dpi;
pub use edit::{DocMut, EditToken};
pub use element::ElementBase;
pub use error::{NodeField, ValidationError};
pub use ids::{IdSpaceExhausted, NodeId, PageId};
pub use json::LoadError;
pub use node::{
    ExtensionNode, GroupNode, MAX_EXTENSION_DEPTH, Node, NodeKind, ShapeGeometry, ShapeNode,
};
pub use page::Page;
pub use page_size::{Axis, PageSize, PageSizeError};
pub use placement::{Flip, Placement};
