//! Layer-1 document model for Gafira: `Document -> Page -> Node`.
//!
//! Pure domain code: no I/O, no `unsafe`. Depends only on `gafira-geom` and
//! `gafira-color`; geometry types are always imported through `gafira_geom`.

#![forbid(unsafe_code)]

mod blend;
mod dpi;
mod ids;
mod page_size;
mod placement;

pub use blend::BlendMode;
pub use dpi::Dpi;
pub use ids::{NodeId, PageId};
pub use page_size::{Axis, PageSize, PageSizeError};
pub use placement::{Flip, Placement};
