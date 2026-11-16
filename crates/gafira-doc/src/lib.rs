//! Layer-1 document model for Gafira: `Document -> Page -> Node`.
//!
//! Pure domain code: no I/O, no `unsafe`. Depends only on `gafira-geom`,
//! `gafira-color` and `gafira-time`, the timeline edge of ADR 0007 whose
//! trigger is `Page.duration`; geometry types are always imported through
//! `gafira_geom`.

#![forbid(unsafe_code)]

mod arena;
mod blend;
mod content_hash;
pub mod contract;
mod document;
mod dpi;
mod edit;
mod element;
mod error;
mod ids;
mod json;
mod layout;
pub mod msg;
mod node;
mod page;
mod page_size;
mod placement;
mod project_id;
pub mod timeline;
mod validate;
pub mod visit;

pub use blend::BlendMode;
pub use content_hash::ContentHash;
pub use document::Document;
pub use dpi::Dpi;
pub use edit::{DocMut, EditToken};
pub use element::ElementBase;
pub use error::{NodeField, ValidationError};
pub use ids::{ClipId, IdSpaceExhausted, MarkerId, NodeId, PageId, SequenceId, TrackId};
pub use json::LoadError;
pub use layout::{
    GridSettings, GridStyle, Guide, GuideAxis, GuideSet, Margins, PageField, PageLayoutError,
    check_bleed,
};
pub use node::{
    ExtensionNode, GroupNode, MAX_EXTENSION_DEPTH, Node, NodeKind, ShapeGeometry, ShapeNode,
};
pub use page::Page;
pub use page_size::{Axis, PageSize, PageSizeError};
pub use placement::{Flip, Placement};
pub use project_id::{ProjectId, ProjectIdError};
pub use timeline::{
    AssetEntry, AssetRef, AssetStorage, AssetTable, Clip, ClipSource, Marker, PixelSize, Sequence,
    Speed, TimelineError, Track, TrackKind,
};
