//! Layer-2 engine crate `gafira-raster`: pixel buffers, premultiplied alpha,
//! tiles and mip levels. Pure domain code: no I/O, no threads, no clocks.

#![forbid(unsafe_code)]

mod blend;
mod dhash;
pub mod effects;
mod error;
mod mip;
mod palette;
mod pixel;
mod srgb8;
mod tile;
mod transfer;
pub mod transitions;
mod view;

pub use blend::{blend_f32, blend_u8};
pub use dhash::{SIMILAR_DISTANCE, dhash, hamming_distance, is_similar};
pub use error::{BufferError, EffectError};
pub use mip::{build_mips, mip_level_count, mip_size};
pub use palette::{PALETTE_SIZE, PaletteEntry, dominant_colors};
pub use pixel::{premultiply, premultiply_u8, unpremultiply, unpremultiply_u8};
pub use srgb8::straight_srgb8;
pub use tile::{TILE_SIZE, TileGrid, tile_grid};
pub use view::{
    AlphaMask, EffectCx, ImageBuf, ImageView, ImageViewMut, PixelBuf, PixelData, PixelDataMut,
    PixelFormat, TileWindow,
};
