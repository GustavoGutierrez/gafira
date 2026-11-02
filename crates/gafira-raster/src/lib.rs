//! Layer-2 engine crate `gafira-raster`: pixel buffers, premultiplied alpha,
//! tiles and mip levels. Pure domain code: no I/O, no threads, no clocks.

#![forbid(unsafe_code)]

mod error;
mod pixel;
mod view;

pub use error::BufferError;
pub use pixel::{premultiply, premultiply_u8, unpremultiply, unpremultiply_u8};
pub use view::{
    AlphaMask, EffectCx, ImageBuf, ImageView, ImageViewMut, PixelBuf, PixelData, PixelDataMut,
    PixelFormat,
};
