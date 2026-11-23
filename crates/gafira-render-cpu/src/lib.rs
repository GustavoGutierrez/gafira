//! The CPU rasterizer of Gafira: the reference implementation of
//! `ports::RenderBackend` (02 §2.1, §6.2).
//!
//! This is a layer-4 adapter. It takes a `ports::RenderPlan` and a
//! `ports::TileCache`, and it produces pixels in the format the target asks
//! for: `Rgba8` sRGB premultiplied for the preview and `RgbaF32` linear
//! premultiplied for the export. It is the reference backend and the golden
//! oracle of the other backends (ARQ-113).
//!
//! Coverage comes from `tiny-skia` behind one private module; the pixels and
//! the sixteen blend modes come from `gafira-raster`, which is used and never
//! redefined (ARQ-119). The crate is pure in the sense of ARQ-02: it reads no
//! clock, touches no file system and creates no thread; the deadline arrives
//! as `ports::Deadline` and the cancellation as `gafira_doc::contract::CancelToken`.
//!
//! A tile accumulates in linear `f32` and the surface stores the frame's
//! format: linear `f32` for the export, the premultiplied sRGB bytes the cache
//! already encoded for the preview, so the curve runs once and `read_back` is
//! a copy (O11, O14).
//!
//! The surface is page-sized (D4): 1080p and 4K fit inside the 1.5 GB cap of
//! `D-03`, and FD-55's 16 384 px side does not (4 GiB in `RgbaF32`, measured in
//! `spike.md`). That side is a stated limit of this delivery; the tiled or
//! streaming path FD-55 names is what would lift it.

#![forbid(unsafe_code)]

mod backend;
mod coverage;
mod effect;
mod image;
mod layer;
mod surface;
#[cfg(test)]
mod tests;

pub use backend::CpuBackend;
