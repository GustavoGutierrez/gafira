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

#![forbid(unsafe_code)]
