//! The decoder of the image assets an export draws (spec 032, ARQ-66).

use gafira_ports::export::ImageDecoder;
use gafira_ports::media::MediaError;
use gafira_raster::{ImageBuf, PixelBuf, premultiply_u8};

/// Decodes PNG and JPEG with `image` (only its `png` and `jpeg` features,
/// ARQ-66) into 8-bit sRGB premultiplied pixels (ARQ-117).
///
/// The format is read from the bytes, not from a name. The allocation limits
/// are the defaults of `image`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RasterDecoder;

impl RasterDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ImageDecoder for RasterDecoder {
    fn decode(&self, bytes: &[u8]) -> Result<ImageBuf, MediaError> {
        let corrupt = |detail: String| MediaError::Corrupt { detail };
        let decoded = image::load_from_memory(bytes).map_err(|e| corrupt(e.to_string()))?;
        let rgba = decoded.into_rgba8();
        let (width, height) = rgba.dimensions();
        let mut pixels = rgba.into_raw();
        for pixel in pixels.chunks_exact_mut(4) {
            let alpha = pixel[3];
            for channel in &mut pixel[..3] {
                *channel = premultiply_u8(*channel, alpha);
            }
        }
        ImageBuf::new(width, height, PixelBuf::Rgba8(pixels)).map_err(|e| corrupt(e.to_string()))
    }
}
