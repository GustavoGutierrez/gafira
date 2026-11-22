//! The models that work on images (spec 084, D-115): the adapter of
//! `gafira_ports::vision`.
//!
//! A model is data ([`catalog`]); how it is run is a [`runtime::Runtime`]:
//! `tract`, in pure Rust, is always there, and ONNX Runtime, which can use
//! the graphics card, is there once the person installs it. Each model is
//! timed once on each of them, and the fastest is used (S84-04).

#![forbid(unsafe_code)]

pub mod catalog;
mod cutout;
pub mod fetch;
mod inpaint;
mod machine;
#[cfg(feature = "online")]
pub mod online;
pub mod runtime;
mod service;
mod store;
mod upscale;

pub use machine::detect;
pub use service::Vision;
