//! Layer-2 engine crate `gafira-raster`: pixel buffers, premultiplied alpha,
//! tiles and mip levels. Pure domain code: no I/O, no threads, no clocks.

#![forbid(unsafe_code)]

mod blend;
mod error;
mod mip;
mod pixel;
mod tile;
mod transfer;
pub mod transitions;
mod view;

pub use blend::{blend_f32, blend_u8};
pub use error::{BufferError, EffectError};
pub use mip::{build_mips, mip_level_count, mip_size};
pub use pixel::{premultiply, premultiply_u8, unpremultiply, unpremultiply_u8};
pub use tile::{TILE_SIZE, TileGrid, tile_grid};
pub use view::{
    AlphaMask, EffectCx, ImageBuf, ImageView, ImageViewMut, PixelBuf, PixelData, PixelDataMut,
    PixelFormat,
};
