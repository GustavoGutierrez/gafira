//! Layer-3 port traits and registries of Gafira (02 §2.1, §4).
//!
//! Traits and plain data only: adapters implement them, the engine consumes
//! them. No I/O, no global state. Each module is reached by its path.

#![forbid(unsafe_code)]

pub mod assets;
pub mod assist;
pub mod common;
pub mod export;
pub mod format;
pub mod i18n;
pub mod license;
pub mod media;
pub mod registry;
pub mod render;
pub mod repository;
pub mod settings;
pub mod store;
pub mod system;
