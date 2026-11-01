//! ARQ-113 / ADR 0010: PNG encoding of goldens.

use gafira_testkit::golden::{Channels, Image, MAX_SIDE, decode_png, encode_png};
use proptest::prelude::*;

/// Chunk types of a PNG, in order.
fn chunk_types(png: &[u8]) -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    let mut at = 8;
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        out.push(png[at + 4..at + 8].try_into().unwrap());
        at += 12 + len;
    }
    out
}

fn image(w: u32, h: u32, ch: Channels, seed: u8) -> Image {
    let n = ch_len(ch) * (w * h) as usize;
    let bytes: Vec<u8> = (0..n)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect();
    Image::new(w, h, ch, &bytes).unwrap()
}

fn ch_len(ch: Channels) -> usize {
    match ch {
        Channels::Gray => 1,
        Channels::Rgba => 4,
    }
}

proptest! {
    // ARQ-113 AC1: every byte returns verbatim, in both layouts.
    #[test]
    fn ac1_roundtrip_is_verbatim(w in 1u32..12, h in 1u32..12, rgba in any::<bool>(), seed in any::<u8>()) {
        let ch = if rgba { Channels::Rgba } else { Channels::Gray };
        let img = image(w, h, ch, seed);
        let back = decode_png(&encode_png(&img).unwrap()).unwrap();
        prop_assert_eq!(back, img);
    }
}

#[test]
fn ac1_premultiplied_bytes_and_zero_alpha_survive() {
    // Premultiplied data may hold colour under alpha 0 and rgb above alpha.
    let px = [200, 100, 50, 0, 255, 255, 255, 128];
    let img = Image::new(2, 1, Channels::Rgba, &px).unwrap();
    let back = decode_png(&encode_png(&img).unwrap()).unwrap();
    assert_eq!(back, img);
}

#[test]
fn ac1_no_colour_chunks_are_written() {
    for ch in [Channels::Gray, Channels::Rgba] {
        let png = encode_png(&image(3, 3, ch, 7)).unwrap();
        for t in chunk_types(&png) {
            assert!(
                [*b"IHDR", *b"IDAT", *b"IEND"].contains(&t),
                "unexpected chunk {t:?}"
            );
        }
    }
}

#[test]
fn ac1_image_rejects_a_wrong_length() {
    assert!(Image::new(2, 2, Channels::Rgba, &[0; 15]).is_err());
    assert!(Image::new(2, 2, Channels::Gray, &[0; 5]).is_err());
}

#[test]
fn ac5_size_limit_is_256_on_each_axis() {
    // 256x256 is accepted, one more on either axis is rejected.
    let ok = image(MAX_SIDE, MAX_SIDE, Channels::Gray, 0);
    assert!(encode_png(&ok).is_ok());
    for (w, h) in [
        (MAX_SIDE + 1, 1),
        (1, MAX_SIDE + 1),
        (MAX_SIDE + 1, MAX_SIDE + 1),
    ] {
        let big = Image::new(w, h, Channels::Gray, &vec![0; (w * h) as usize]).unwrap();
        assert!(encode_png(&big).is_err(), "{w}x{h} must be rejected");
    }
}

#[test]
fn ac5_decode_rejects_an_oversized_png() {
    // Build the oversized PNG with the same crate, bypassing the module's check.
    for (w, h) in [(MAX_SIDE + 1, 1), (1, MAX_SIDE + 1)] {
        let mut buf = Vec::new();
        let mut enc = png::Encoder::new(&mut buf, w, h);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .unwrap()
            .write_image_data(&vec![0; (w * h) as usize])
            .unwrap();
        assert!(decode_png(&buf).is_err(), "{w}x{h} must be rejected");
    }
    let mut buf = Vec::new();
    let mut enc = png::Encoder::new(&mut buf, MAX_SIDE, MAX_SIDE);
    enc.set_color(png::ColorType::Grayscale);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .unwrap()
        .write_image_data(&vec![0; (MAX_SIDE * MAX_SIDE) as usize])
        .unwrap();
    assert!(decode_png(&buf).is_ok());
}
