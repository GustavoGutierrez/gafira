//! S87: AVIF and WebP read and written.

use gafira_io_image::RasterDecoder;
use gafira_ports::export::ImageDecoder;
use gafira_raster::{ImageBuf, straight_srgb8};

/// What the files were made from.
fn scene(x: u32, y: u32) -> [u8; 3] {
    [(x * 4) as u8, (y * 5) as u8, (255 - x * 2 - y) as u8]
}

fn open(name: &str) -> ImageBuf {
    let path = format!("{}/tests/avif/{name}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(path).expect("the file");
    RasterDecoder::new().decode(&bytes).expect(name)
}

fn mean(image: &ImageBuf, want: impl Fn(u32, u32) -> [u8; 3]) -> f64 {
    let data = straight_srgb8(image).expect("pixels");
    let w = image.width();
    let sum: u64 = data
        .chunks(4)
        .enumerate()
        .map(|(i, p)| {
            let want = want(i as u32 % w, i as u32 / w);
            (0..3)
                .map(|c| u64::from(p[c].abs_diff(want[c])))
                .sum::<u64>()
        })
        .sum();
    sum as f64 / (data.len() / 4 * 3) as f64
}

// S87-02: each kind is what it was made from.
#[test]
fn s87_02_each_kind_is_read() {
    for name in ["rgb420.avif", "rgb444.avif", "limited.avif"] {
        let image = open(name);
        assert_eq!((image.width(), image.height()), (64, 48), "{name}");
        let off = mean(&image, scene);
        assert!(off < 3.0, "{name} {off}");
    }
    let grey = open("grey.avif");
    let light = |x, y| {
        let [r, g, b] = scene(x, y).map(f64::from);
        [(0.299 * r + 0.587 * g + 0.114 * b) as u8; 3]
    };
    assert!(mean(&grey, light) < 3.0);
    // A quarter of a turn clockwise.
    let turned = open("turned.avif");
    assert_eq!((turned.width(), turned.height()), (48, 64));
    let off = mean(&turned, |x, y| scene(y, 47 - x));
    assert!(off < 3.0, "{off}");
    let alpha = open("alpha.avif");
    let data = straight_srgb8(&alpha).expect("pixels");
    let a = |x: usize, y: usize| data[(y * 64 + x) * 4 + 3];
    assert!(a(0, 2) < 6 && a(32, 2).abs_diff(128) < 6 && a(60, 40) > 250);
    let small = open("small.webp");
    assert_eq!((small.width(), small.height()), (32, 24));
    assert!(mean(&small, |x, y| scene(x * 2, y * 2)) < 8.0);
}

// S87-02: the size is told without reading the pixels.
#[test]
fn s87_02_the_size() {
    gafira_io_image::register();
    let bytes = include_bytes!("avif/turned.avif");
    let reader = image::ImageReader::new(std::io::Cursor::new(&bytes[..]))
        .with_guessed_format()
        .expect("a format");
    assert_eq!(reader.into_dimensions().expect("a size"), (48, 64));
}
