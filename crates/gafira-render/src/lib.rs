//! Layer-3 render engine crate `gafira-render`: the content key of every node
//! of the render graph (02 §4.17.7, §6.1).
//!
//! The crate is pure in the sense of ARQ-02 (ADR 0016): it opens no file,
//! opens no socket, spawns no thread and reads no clock.

#![forbid(unsafe_code)]

mod key;

pub use key::{DOMAIN, KeyParam, NodeType, canonical_description, content_key};
