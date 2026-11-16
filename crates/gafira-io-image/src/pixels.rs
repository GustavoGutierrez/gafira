//! The pixels of a render as the straight 8-bit sRGB a file stores
//! (02 §6.2, ARQ-117).

use gafira_color::LinearRgba;
use gafira_raster::{ImageBuf, PixelBuf, unpremultiply_u8};

/// The pixels of `image` as straight (not premultiplied) 8-bit sRGB RGBA, row
/// by row.
///
/// An export render is `RgbaF32` premultiplied in linear light: each pixel
/// is divided by its alpha, encoded to sRGB and rounded, with every channel
/// clamped to `[0, 1]` first. A preview render is `Rgba8` premultiplied sRGB:
/// only the alpha is taken out. A pixel whose alpha rounds to 0 is clear
/// (all four bytes 0). An `Alpha8` mask is not a picture: `None`.
#[must_use]
pub fn straight_srgb8(image: &ImageBuf) -> Option<Vec<u8>> {
    match image.data() {
        PixelBuf::RgbaF32(values) => Some(values.chunks_exact(4).flat_map(from_linear).collect()),
        PixelBuf::Rgba8(values) => Some(values.chunks_exact(4).flat_map(from_srgb8).collect()),
        PixelBuf::Alpha8(_) => None,
    }
}

/// A value in `[0, 1]` as a byte, rounded to the nearest.
fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8
}

/// One premultiplied linear `f32` pixel as straight sRGB bytes.
fn from_linear(pixel: &[f32]) -> [u8; 4] {
    let alpha = pixel[3].clamp(0.0, 1.0);
    if byte(alpha) == 0 {
        return [0; 4];
    }
    let straight = |c: f32| (c / alpha).clamp(0.0, 1.0);
    let encoded = LinearRgba::new(
        straight(pixel[0]),
        straight(pixel[1]),
        straight(pixel[2]),
        alpha,
    )
    .to_srgb();
    [
        byte(encoded.r),
        byte(encoded.g),
        byte(encoded.b),
        byte(alpha),
    ]
}

/// One premultiplied sRGB byte pixel as straight bytes.
fn from_srgb8(pixel: &[u8]) -> [u8; 4] {
    let alpha = pixel[3];
    if alpha == 0 {
        return [0; 4];
    }
    [
        unpremultiply_u8(pixel[0], alpha),
        unpremultiply_u8(pixel[1], alpha),
        unpremultiply_u8(pixel[2], alpha),
        alpha,
    ]
}

#[cfg(test)]
mod tests {
    use gafira_raster::PixelBuf;

    use super::*;

    fn one(data: PixelBuf) -> ImageBuf {
        ImageBuf::new(1, 1, data).expect("one pixel")
    }

    // S30-03: an export pixel is premultiplied linear light; the file gets it
    // straight and sRGB encoded.
    #[test]
    fn s30_03_linear_premultiplied_f32_becomes_straight_srgb8() {
        let white = one(PixelBuf::RgbaF32(vec![1.0, 1.0, 1.0, 1.0]));
        assert_eq!(straight_srgb8(&white).unwrap(), [255, 255, 255, 255]);
        // Linear 0.5 encodes as sRGB 188 (0.7354 · 255 = 187.5, rounded up).
        let grey = one(PixelBuf::RgbaF32(vec![0.5, 0.5, 0.5, 1.0]));
        assert_eq!(straight_srgb8(&grey).unwrap(), [188, 188, 188, 255]);
        // Half covered linear 0.5: premultiplied 0.25, straight 0.5 again.
        let half = one(PixelBuf::RgbaF32(vec![0.25, 0.25, 0.25, 0.5]));
        assert_eq!(straight_srgb8(&half).unwrap(), [188, 188, 188, 128]);
        // Out of range values are clamped; no alpha is a clear pixel.
        let bright = one(PixelBuf::RgbaF32(vec![2.0, -1.0, 0.0, 1.0]));
        assert_eq!(straight_srgb8(&bright).unwrap(), [255, 0, 0, 255]);
        let clear = one(PixelBuf::RgbaF32(vec![0.3, 0.3, 0.3, 0.0]));
        assert_eq!(straight_srgb8(&clear).unwrap(), [0, 0, 0, 0]);
    }

    // S30-03: a preview pixel is premultiplied sRGB already; only the alpha
    // is taken out.
    #[test]
    fn s30_03_premultiplied_srgb8_becomes_straight() {
        let half = one(PixelBuf::Rgba8(vec![100, 50, 0, 128]));
        assert_eq!(straight_srgb8(&half).unwrap(), [199, 100, 0, 128]);
        let opaque = one(PixelBuf::Rgba8(vec![10, 20, 30, 255]));
        assert_eq!(straight_srgb8(&opaque).unwrap(), [10, 20, 30, 255]);
    }

    #[test]
    fn s30_03_a_mask_is_not_a_picture() {
        let mask = one(PixelBuf::Alpha8(vec![255]));
        assert!(straight_srgb8(&mask).is_none());
    }
}
