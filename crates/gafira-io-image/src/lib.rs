//! Layer-4 adapter `gafira-io-image` (02 §2.1): the raster exporters.
//!
//! This first cut writes PNG (FD-55, spec 030). The exporter never sees
//! `gafira-render` (ARQ-06): it asks the `PageRenderer` of its `ExportJob`
//! for the pixels of each page.

#![forbid(unsafe_code)]

mod pixels;
mod png_export;

pub use pixels::straight_srgb8;
pub use png_export::{MAX_SIDE, PNG_FORMAT, PngExporter, SCALE, TRANSPARENT, page_pixels};
