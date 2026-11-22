//! Layer-4 adapter `gafira-io-image` (02 §2.1): the raster exporters.
//!
//! It writes PNG (FD-55, spec 030), JPEG and lossless WebP (spec 039), one
//! file per page, and decodes the PNG and JPEG assets an export draws
//! (spec 032). The exporters never see `gafira-render` (ARQ-06): they ask
//! the `PageRenderer` of their `ExportJob` for the pixels of each page.

#![forbid(unsafe_code)]

mod decode;
mod jpeg_export;
mod png_export;
mod raster;
mod webp_export;

pub use decode::RasterDecoder;
/// Moved to `gafira-raster` (spec 033); kept here for the exporters.
pub use gafira_raster::straight_srgb8;
pub use jpeg_export::{JPEG_FORMAT, JpegExporter, QUALITY};
pub use png_export::{PNG_FORMAT, PNG_QUALITY, PngExporter};
pub use raster::{MAX_SIDE, SCALE, TRANSPARENT, page_pixels};
pub use webp_export::{WEBP_FORMAT, WEBP_QUALITY, WebpExporter};
