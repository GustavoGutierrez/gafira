//! AVIF (spec 087, D-118): its container is read here, its frames by
//! `re_rav1d`, and it is written with `ravif`. [`register`] teaches `image`
//! to read it, so every place that reads an image reads it too.

mod container;
mod decode;
mod export;

use std::io::Read;

use image::error::{DecodingError, ImageFormatHint, LimitError, LimitErrorKind};
use image::{ColorType, ImageDecoder, ImageError, ImageResult};

pub use export::{AVIF_FORMAT, AVIF_QUALITY, AvifExporter};

/// The most pixels of an image.
const MOST_PIXELS: u64 = 200_000_000;

/// Teaches `image` to read AVIF; more than once changes nothing.
pub fn register() {
    use image::hooks::{decoding_hook_registered, register_decoding_hook};
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if decoding_hook_registered("avif".as_ref()) {
            return;
        }
        register_decoding_hook(
            "avif".into(),
            Box::new(|mut reader| {
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes)?;
                Ok(Box::new(Decoder::new(&bytes)?))
            }),
        );
        // The size of the first box may be any.
        const MASK: [u8; 12] = [0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255];
        for signature in [
            b"\0\0\0\0ftypavif",
            b"\0\0\0\0ftypavis",
            b"\0\0\0\0ftypmif1",
        ] {
            image::hooks::register_format_detection_hook("avif".into(), signature, Some(&MASK));
        }
    });
}

fn bad(what: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> ImageError {
    let hint = ImageFormatHint::Name("AVIF".to_owned());
    ImageError::Decoding(DecodingError::new(hint, what))
}

/// An AVIF read by `image`: its frames are not decoded until its pixels are
/// asked for.
struct Decoder {
    avif: container::Avif,
    size: (u32, u32),
}

impl Decoder {
    fn new(bytes: &[u8]) -> ImageResult<Self> {
        let avif = container::read(bytes).map_err(|e| bad(e.0))?;
        let size = avif.size().ok_or_else(|| bad("no size"))?;
        if size.0 == 0 || size.1 == 0 {
            return Err(bad("no pixels"));
        }
        if u64::from(size.0) * u64::from(size.1) > MOST_PIXELS {
            let limit = LimitError::from_kind(LimitErrorKind::DimensionError);
            return Err(ImageError::Limits(limit));
        }
        Ok(Self { avif, size })
    }
}

impl ImageDecoder for Decoder {
    fn dimensions(&self) -> (u32, u32) {
        self.size
    }

    fn color_type(&self) -> ColorType {
        ColorType::Rgba8
    }

    fn read_image(self, buf: &mut [u8]) -> ImageResult<()>
    where
        Self: Sized,
    {
        // The decoder is a translation of C: what it does not expect may
        // panic, and a broken file must not close Gafira.
        let decode = std::panic::AssertUnwindSafe(|| decode::rgba(&self.avif));
        let (size, rgba) = std::panic::catch_unwind(decode)
            .unwrap_or(Err(container::Bad("the decoder failed")))
            .map_err(|e| bad(e.0))?;
        if size != self.size || rgba.len() != buf.len() {
            return Err(bad("not the size it says"));
        }
        buf.copy_from_slice(&rgba);
        Ok(())
    }

    fn read_image_boxed(self: Box<Self>, buf: &mut [u8]) -> ImageResult<()> {
        (*self).read_image(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const ALPHA: &[u8] = include_bytes!("../../tests/avif/alpha.avif");
    const TURNED: &[u8] = include_bytes!("../../tests/avif/turned.avif");

    fn open(bytes: &[u8]) -> ImageResult<image::DynamicImage> {
        register();
        image::load_from_memory(bytes)
    }

    // S87-01: cut anywhere, or with any byte changed, it is an error or an
    // image, and never a failure.
    #[test]
    fn s87_01_cut_short() {
        for file in [ALPHA, TURNED] {
            assert!(open(file).is_ok());
            for n in 0..file.len().min(700) {
                let _ = open(&file[..n]);
            }
        }
        assert!(open(b"\0\0\0\x18ftypavif\0\0\0\0mif1avif").is_err());
    }

    fn scene(w: u32, h: u32) -> Vec<u8> {
        (0..w * h)
            .flat_map(|i| {
                let (x, y) = (i % w, i / w);
                // Below 256 for what is asked.
                [
                    (x * 2) as u8,
                    (y * 3) as u8,
                    90,
                    if x < w / 2 { 255 } else { 120 },
                ]
            })
            .collect()
    }

    fn written(w: u32, h: u32, quality: &'static str) -> Vec<u8> {
        use gafira_doc::contract::{ParamValue, ParamValues};
        let mut options = ParamValues::new();
        options.set(AVIF_QUALITY, ParamValue::Choice(quality.into()));
        export::encode(w, h, scene(w, h), &options).expect("an AVIF")
    }

    // S87-04: what is written is read back, its opacity too; a lower quality
    // is a smaller file.
    #[test]
    fn s87_04_written_and_read_back() {
        for (w, h) in [(96, 64), (33, 17), (1, 1)] {
            let file = written(w, h, "high");
            let back = open(&file).expect("it reads").into_rgba8();
            assert_eq!(back.dimensions(), (w, h));
            let off: u64 = back
                .as_raw()
                .iter()
                .zip(scene(w, h))
                .map(|(a, b)| u64::from(a.abs_diff(b)))
                .sum();
            let mean = off as f64 / f64::from(w * h * 4);
            assert!(mean < 3.0, "{w} {h} {mean}");
        }
        assert!(written(96, 64, "low").len() < written(96, 64, "high").len());
    }

    fn boxed(kind: &[u8; 4], inside: &[u8]) -> Vec<u8> {
        let mut out = u32::try_from(inside.len() + 8)
            .unwrap()
            .to_be_bytes()
            .to_vec();
        out.extend(kind);
        out.extend(inside);
        out
    }

    /// A mosaic of two pieces side by side, cut to `size`.
    fn mosaic(left: &[u8], right: &[u8], size: (u16, u16)) -> Vec<u8> {
        let full = |kind: &[u8; 4], version: u8, inside: &[u8]| {
            boxed(kind, &[&[version, 0, 0, 0], inside].concat())
        };
        let mut grid = vec![0, 0, 0, 1];
        grid.extend(size.0.to_be_bytes());
        grid.extend(size.1.to_be_bytes());
        let parts: [&[u8]; 3] = [&grid, left, right];
        // One extent each, in the box of data.
        let mut iloc = vec![0x44, 0x00, 0, 3];
        let mut at = 0_u32;
        for (n, part) in parts.iter().enumerate() {
            let length = u32::try_from(part.len()).unwrap();
            iloc.extend([0, n as u8 + 1, 0, 1, 0, 0, 0, 1]);
            iloc.extend(at.to_be_bytes());
            iloc.extend(length.to_be_bytes());
            at += length;
        }
        let infe =
            |id: u8, kind: &[u8; 4]| full(b"infe", 2, &[&[0, id, 0, 0][..], kind, &[0]].concat());
        let iinf = [
            vec![0, 3],
            infe(1, b"grid"),
            infe(2, b"av01"),
            infe(3, b"av01"),
        ]
        .concat();
        let ispe = full(
            b"ispe",
            0,
            &[
                0,
                0,
                size.0.to_be_bytes()[0],
                size.0.to_be_bytes()[1],
                0,
                0,
                size.1.to_be_bytes()[0],
                size.1.to_be_bytes()[1],
            ],
        );
        let ipma = full(b"ipma", 0, &[0, 0, 0, 1, 0, 1, 1, 1]);
        let iref = full(b"iref", 0, &boxed(b"dimg", &[0, 1, 0, 2, 0, 2, 0, 3]));
        let meta = [
            full(b"pitm", 0, &[0, 1]),
            full(b"iloc", 1, &iloc),
            full(b"iinf", 0, &iinf),
            boxed(b"iprp", &[boxed(b"ipco", &ispe), ipma].concat()),
            iref,
            boxed(b"idat", &parts.concat()),
        ]
        .concat();
        [
            boxed(b"ftyp", b"avif\0\0\0\0avifmif1"),
            full(b"meta", 0, &meta),
        ]
        .concat()
    }

    // S87-02: the pieces of a mosaic are put side by side, and cut; one that
    // has fewer than it says is an error.
    #[test]
    fn s87_02_a_mosaic() {
        let tile = |file: &[u8]| container::read(file).expect("an AVIF").color.tiles[0].clone();
        let flat = |c: [u8; 3]| {
            let pixels = [c[0], c[1], c[2], 255].repeat(64 * 64);
            let none = gafira_doc::contract::ParamValues::new();
            tile(&export::encode(64, 64, pixels, &none).expect("an AVIF"))
        };
        let (red, blue) = (flat([220, 20, 20]), flat([20, 20, 220]));
        let back = open(&mosaic(&red, &blue, (100, 60)))
            .expect("it reads")
            .into_rgba8();
        assert_eq!(back.dimensions(), (100, 60));
        let near = |x, y, want: [u8; 3]| {
            let p = back.get_pixel(x, y).0;
            assert!(
                p.iter().zip(want).all(|(a, b)| a.abs_diff(b) < 12),
                "{x},{y} {p:?}"
            );
        };
        near(5, 5, [220, 20, 20]);
        near(60, 55, [220, 20, 20]);
        near(70, 30, [20, 20, 220]);
        near(99, 59, [20, 20, 220]);
        assert!(open(&mosaic(&red, &blue, (200, 60))).is_err());
        assert!(open(&mosaic(&red, b"what", (100, 60))).is_err());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(300))]
        #[test]
        fn s87_01_changed(at in 0_usize..870, to in any::<u8>(), more in 0_usize..870, other in any::<u8>()) {
            let mut file = ALPHA.to_vec();
            let n = file.len();
            file[at % n] = to;
            file[more % n] = other;
            let _ = open(&file);
        }
    }
}
