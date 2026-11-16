//! The decoder of the image assets an export draws (spec 032).

use std::io::Cursor;

use gafira_io_image::RasterDecoder;
use gafira_ports::export::ImageDecoder;
use gafira_ports::media::MediaError;
use gafira_raster::PixelBuf;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

fn encoded(image: &RgbaImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let result = match format {
        // JPEG has no alpha: write the colour channels only.
        ImageFormat::Jpeg => DynamicImage::ImageRgba8(image.clone())
            .into_rgb8()
            .write_to(&mut bytes, format),
        _ => image.write_to(&mut bytes, format),
    };
    result.expect("the image encodes");
    bytes.into_inner()
}

// S32-02: a PNG decodes to premultiplied 8-bit pixels of its size.
#[test]
fn s32_02_a_png_decodes_to_premultiplied_pixels() {
    let mut image = RgbaImage::from_pixel(3, 2, Rgba([255, 0, 0, 255]));
    image.put_pixel(1, 0, Rgba([200, 100, 50, 128]));
    let decoded = RasterDecoder::new()
        .decode(&encoded(&image, ImageFormat::Png))
        .expect("the PNG decodes");
    assert_eq!((decoded.width(), decoded.height()), (3, 2));
    let PixelBuf::Rgba8(pixels) = decoded.data() else {
        panic!("8-bit pixels");
    };
    assert_eq!(&pixels[..4], [255, 0, 0, 255]);
    assert_eq!(&pixels[4..8], [100, 50, 25, 128]);
}

// S32-02: a JPEG decodes to opaque pixels of its size.
#[test]
fn s32_02_a_jpeg_decodes_to_opaque_pixels() {
    let image = RgbaImage::from_pixel(16, 8, Rgba([0, 0, 255, 255]));
    let decoded = RasterDecoder::new()
        .decode(&encoded(&image, ImageFormat::Jpeg))
        .expect("the JPEG decodes");
    assert_eq!((decoded.width(), decoded.height()), (16, 8));
    let PixelBuf::Rgba8(pixels) = decoded.data() else {
        panic!("8-bit pixels");
    };
    assert!(pixels.chunks_exact(4).all(|p| p[3] == 255 && p[2] > 200));
}

// S32-02: bytes that are not an image are a corrupt asset.
#[test]
fn s32_02_bytes_that_are_not_an_image_are_corrupt() {
    assert!(matches!(
        RasterDecoder::new().decode(b"\x89PNG not really"),
        Err(MediaError::Corrupt { .. })
    ));
}
