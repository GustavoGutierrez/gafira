//! Layer-1 document model for Gafira: `Document -> Page -> Node`.
//!
//! Pure domain code: no I/O, no `unsafe`. Depends only on `gafira-geom` and
//! `gafira-color`; geometry types are always imported through `gafira_geom`.

#![forbid(unsafe_code)]
