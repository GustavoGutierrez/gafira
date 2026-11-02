//! The dominant colours of an image (ARQ-122, FD-26).

use gafira_color::{Color, Lab, LinearRgba, Rgba, delta_e2000};

use crate::transfer::Transfer;
use crate::{BufferError, EffectError, ImageView, PixelData, PixelFormat, unpremultiply_u8};

/// The most colours a palette holds (ARQ-122).
pub const PALETTE_SIZE: usize = 5;

/// Two bins closer than this in dE2000 are one colour (FD-26 looks for a match within 15).
const MERGE_DELTA_E: f64 = 10.0;

/// 5 bits per channel.
const BINS: usize = 1 << 15;

/// The bin of a straight sRGB byte colour.
fn bin_of(rgb: [u8; 3]) -> usize {
    (usize::from(rgb[0] >> 3) << 10) | (usize::from(rgb[1] >> 3) << 5) | usize::from(rgb[2] >> 3)
}

/// An area and the sum of its linear colours.
#[derive(Clone, Copy, Default)]
struct Bin {
    pixels: u64,
    sum: [f64; 3],
}

impl Bin {
    fn add(&mut self, linear: [f64; 3]) {
        self.pixels += 1;
        for (s, l) in self.sum.iter_mut().zip(linear) {
            *s += l;
        }
    }

    fn merge(&mut self, other: &Bin) {
        self.pixels += other.pixels;
        for (s, o) in self.sum.iter_mut().zip(other.sum) {
            *s += o;
        }
    }

    fn mean(&self) -> [f32; 3] {
        let n = self.pixels as f64;
        self.sum.map(|s| (s / n) as f32)
    }

    fn lab(&self) -> Lab {
        let m = self.mean();
        Lab::from_linear_srgb(LinearRgba::new(m[0], m[1], m[2], 1.0))
    }

    fn srgb(&self, t: &Transfer) -> Color {
        let m = self.mean();
        let e = t.encode_unit(m.map(f64::from));
        Color::Srgb(Rgba::new(e[0] as f32, e[1] as f32, e[2] as f32, 1.0))
    }
}

/// One dominant colour and the area it covers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaletteEntry {
    /// The mean colour of the pixels it covers, straight and opaque, in sRGB.
    pub color: Color,
    /// How many pixels it covers.
    pub pixels: u64,
}

/// Up to [`PALETTE_SIZE`] dominant colours, the largest area first. Pixels with alpha 0 are
/// ignored; any other alpha counts as one pixel of its straight colour.
///
/// # Errors
///
/// [`EffectError::UnsupportedFormat`] for `Alpha8`, [`EffectError::Buffer`] for a non-finite
/// `f32` pixel.
pub fn dominant_colors(image: ImageView<'_>) -> Result<Vec<PaletteEntry>, EffectError> {
    let format = image.data().format();
    if format == PixelFormat::Alpha8 {
        return Err(EffectError::UnsupportedFormat { format });
    }
    // Each pixel goes to one of 32 768 bins (5 bits per sRGB channel), where its area and its
    // linear colour add up.
    let mut bins = vec![Bin::default(); BINS];
    let t = Transfer::new();
    match image.data() {
        PixelData::Rgba8(s) => {
            for px in s.chunks_exact(4).filter(|px| px[3] != 0) {
                let straight = [0, 1, 2].map(|k| unpremultiply_u8(px[k], px[3]));
                bins[bin_of(straight)].add(straight.map(|b| t.decode[usize::from(b)]));
            }
        }
        PixelData::RgbaF32(s) => {
            if let Some(index) = s.iter().position(|v| !v.is_finite()) {
                return Err(BufferError::NonFinite {
                    field: "src",
                    index,
                }
                .into());
            }
            for px in s.chunks_exact(4).filter(|px| px[3] != 0.0) {
                let a = f64::from(px[3]);
                let lin = [0, 1, 2].map(|k| (f64::from(px[k]) / a).clamp(0.0, 1.0));
                let bytes = t.encode(lin);
                bins[bin_of(bytes)].add(lin);
            }
        }
        PixelData::Alpha8(_) => {}
    }
    // The fullest bins first, the lower bin number first among equals.
    let mut order: Vec<usize> = (0..BINS).filter(|i| bins[*i].pixels > 0).collect();
    order.sort_by_key(|i| (std::cmp::Reverse(bins[*i].pixels), *i));
    let mut clusters: Vec<Bin> = Vec::new();
    for i in order {
        let b = bins[i];
        let lab = b.lab();
        let nearest = clusters
            .iter()
            .enumerate()
            .map(|(k, c)| (k, delta_e2000(c.lab(), lab)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match nearest {
            Some((k, d)) if d < MERGE_DELTA_E => clusters[k].merge(&b),
            _ if clusters.len() < PALETTE_SIZE => clusters.push(b),
            // Out of room: a colour far from all five is left out.
            _ => {}
        }
    }
    // Stable, so equal areas keep the order they were found in.
    clusters.sort_by_key(|c| std::cmp::Reverse(c.pixels));
    Ok(clusters
        .iter()
        .map(|c| PaletteEntry {
            color: c.srgb(&t),
            pixels: c.pixels,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab_of(c: Color) -> Lab {
        Lab::from_linear_srgb(c.to_linear_srgb())
    }

    fn lab_u8(rgb: [u8; 3]) -> Lab {
        let l = |v: u8| {
            let c = Color::Srgb(Rgba::new(f32::from(v) / 255.0, 0.0, 0.0, 1.0));
            c.to_linear_srgb().r
        };
        Lab::from_linear_srgb(LinearRgba::new(l(rgb[0]), l(rgb[1]), l(rgb[2]), 1.0))
    }

    fn de(c: Color, rgb: [u8; 3]) -> f64 {
        delta_e2000(lab_of(c), lab_u8(rgb))
    }

    /// An image of horizontal runs: `(colour, pixels)` in order, one row.
    fn runs(parts: &[([u8; 4], usize)]) -> (Vec<u8>, u32) {
        let mut v = Vec::new();
        let mut n = 0;
        for (c, k) in parts {
            for _ in 0..*k {
                v.extend(*c);
            }
            n += k;
        }
        (v, n as u32)
    }

    fn pal(bytes: &[u8], w: u32) -> Vec<PaletteEntry> {
        dominant_colors(ImageView::new(w, 1, PixelData::Rgba8(bytes)).unwrap()).unwrap()
    }

    const RED: [u8; 3] = [220, 30, 30];
    const GREEN: [u8; 3] = [30, 160, 60];
    const BLUE: [u8; 3] = [40, 60, 210];
    const YELLOW: [u8; 3] = [240, 220, 40];
    const GREY: [u8; 3] = [120, 120, 120];
    const PURPLE: [u8; 3] = [150, 40, 170];
    const CYAN: [u8; 3] = [40, 200, 210];

    fn op(c: [u8; 3]) -> [u8; 4] {
        [c[0], c[1], c[2], 255]
    }

    // ARQ-122: five flat colours with known areas come back in order of area, each within
    // dE2000 < 1, with their exact pixel counts.
    #[test]
    fn arq_122_five_flat_colours_in_order_of_area() {
        // Shuffled on purpose: the order of the result is by area, not by position.
        let (bytes, w) = runs(&[
            (op(GREEN), 250),
            (op(BLUE), 120),
            (op(RED), 400),
            (op(GREY), 80),
            (op(YELLOW), 150),
        ]);
        let p = pal(&bytes, w);
        assert_eq!(p.len(), 5);
        let want = [
            (RED, 400),
            (GREEN, 250),
            (YELLOW, 150),
            (BLUE, 120),
            (GREY, 80),
        ];
        for (e, (rgb, n)) in p.iter().zip(want) {
            assert!(de(e.color, rgb) < 1.0, "{rgb:?}: {:?}", e.color);
            assert_eq!(e.pixels, n);
            assert_eq!(e.color.alpha(), 1.0);
        }
    }

    // ARQ-122: two colours give two entries; one colour gives one.
    #[test]
    fn arq_122_two_colours_give_two_entries() {
        let (bytes, w) = runs(&[(op(BLUE), 30), (op(RED), 70)]);
        let p = pal(&bytes, w);
        assert_eq!(p.len(), 2);
        assert!(de(p[0].color, RED) < 1.0 && p[0].pixels == 70);
        assert!(de(p[1].color, BLUE) < 1.0 && p[1].pixels == 30);
        let (bytes, w) = runs(&[(op(PURPLE), 1)]);
        assert_eq!(pal(&bytes, w).len(), 1);
    }

    // ARQ-122: a transparent image gives an empty list, and so does an empty image.
    #[test]
    fn arq_122_transparent_and_empty_images_give_an_empty_list() {
        let (bytes, w) = runs(&[([0, 0, 0, 0], 20)]);
        assert!(pal(&bytes, w).is_empty());
        // Colour held by a transparent pixel is not a colour.
        let (bytes, w) = runs(&[([200, 10, 10, 0], 20)]);
        assert!(pal(&bytes, w).is_empty());
        let none = dominant_colors(ImageView::new(0, 0, PixelData::Rgba8(&[])).unwrap()).unwrap();
        assert!(none.is_empty());
        let none = dominant_colors(ImageView::new(0, 7, PixelData::RgbaF32(&[])).unwrap()).unwrap();
        assert!(none.is_empty());
    }

    // ARQ-122: pixels with alpha 0 are ignored, whatever area they take.
    #[test]
    fn arq_122_alpha_zero_pixels_do_not_count() {
        let (bytes, w) = runs(&[([9, 200, 9, 0], 500), (op(BLUE), 10), (op(YELLOW), 4)]);
        let p = pal(&bytes, w);
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].pixels, p[1].pixels), (10, 4));
        assert!(de(p[0].color, BLUE) < 1.0 && de(p[1].color, YELLOW) < 1.0);
    }

    // ARQ-122: a translucent pixel counts as one pixel of its straight colour.
    #[test]
    fn arq_122_translucent_pixels_count_with_their_straight_colour() {
        // Premultiplied (110, 15, 15) at alpha 128 is straight (219, 30, 30).
        let (bytes, w) = runs(&[([110, 15, 15, 128], 40), (op(GREEN), 10)]);
        let p = pal(&bytes, w);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].pixels, 40);
        assert!(de(p[0].color, [219, 30, 30]) < 1.0, "{:?}", p[0].color);
    }

    // ARQ-122: no more than five entries; the largest areas win and the rest are left out.
    #[test]
    fn arq_122_at_most_five_entries_the_largest_areas() {
        let (bytes, w) = runs(&[
            (op(CYAN), 3),
            (op(RED), 90),
            (op(PURPLE), 5),
            (op(GREEN), 70),
            (op(BLUE), 50),
            (op(YELLOW), 30),
            (op(GREY), 10),
        ]);
        let p = pal(&bytes, w);
        assert_eq!(p.len(), 5);
        assert_eq!(PALETTE_SIZE, 5);
        let want = [(RED, 90), (GREEN, 70), (BLUE, 50), (YELLOW, 30), (GREY, 10)];
        for (e, (rgb, n)) in p.iter().zip(want) {
            assert!(de(e.color, rgb) < 1.0, "{rgb:?}");
            assert_eq!(e.pixels, n);
        }
    }

    // ARQ-122: noise around a colour is one colour, not several.
    #[test]
    fn arq_122_noise_around_a_colour_is_one_entry() {
        let mut v = Vec::new();
        for i in 0..400u32 {
            let j = |s: u32| ((i.wrapping_mul(2_654_435_761) >> s) % 7) as i32 - 3;
            let c = |base: i32, s| (base + j(s)).clamp(0, 255) as u8;
            if i % 2 == 0 {
                v.extend([c(200, 3), c(40, 9), c(40, 15), 255]);
            } else {
                v.extend([c(30, 3), c(60, 9), c(200, 15), 255]);
            }
        }
        let p = pal(&v, 400);
        assert_eq!(p.len(), 2, "{p:?}");
        assert_eq!(p[0].pixels + p[1].pixels, 400);
        assert!(p.iter().all(|e| e.pixels == 200));
        let reds: Vec<_> = p
            .iter()
            .filter(|e| de(e.color, [200, 40, 40]) < 3.0)
            .collect();
        assert_eq!(reds.len(), 1);
    }

    // ARQ-122, FD-26: the same in `f32` (linear, premultiplied) and a result in sRGB.
    #[test]
    fn arq_122_rgba_f32_gives_the_same_palette() {
        let to_f = |rgb: [u8; 3], a: f32| {
            let l = |v: u8| {
                Color::Srgb(Rgba::new(f32::from(v) / 255.0, 0.0, 0.0, 1.0))
                    .to_linear_srgb()
                    .r
            };
            [l(rgb[0]) * a, l(rgb[1]) * a, l(rgb[2]) * a, a]
        };
        let mut px = Vec::new();
        for (rgb, n, a) in [(RED, 12, 1.0f32), (BLUE, 7, 0.5), (GREEN, 3, 1.0)] {
            for _ in 0..n {
                px.extend(to_f(rgb, a));
            }
        }
        px.extend([0.5f32, 0.2, 0.1, 0.0]);
        let p = dominant_colors(ImageView::new(23, 1, PixelData::RgbaF32(&px)).unwrap()).unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p.iter().map(|e| e.pixels).collect::<Vec<_>>(), [12, 7, 3]);
        for (e, rgb) in p.iter().zip([RED, BLUE, GREEN]) {
            assert!(de(e.color, rgb) < 1.0, "{rgb:?} {:?}", e.color);
            assert!(matches!(e.color, Color::Srgb(_)));
        }
    }

    // ARQ-122, 4.17.4: values beyond [0, 1] in `f32` are cut, and the sum cannot overflow.
    #[test]
    fn arq_122_f32_values_beyond_the_range_are_clipped() {
        let px = [3.0e38f32, -3.0e38, 0.5, 1.0];
        let p = dominant_colors(ImageView::new(1, 1, PixelData::RgbaF32(&px)).unwrap()).unwrap();
        assert_eq!(p.len(), 1);
        let Color::Srgb(c) = p[0].color else { panic!() };
        assert!(c.r == 1.0 && c.g == 0.0 && c.b > 0.0 && c.b < 1.0, "{c:?}");
    }

    // ARQ-122: colours that differ only in blue are two colours.
    #[test]
    fn arq_122_colours_that_differ_in_one_channel_are_apart() {
        let (bytes, w) = runs(&[([100, 100, 40, 255], 20), ([100, 100, 220, 255], 10)]);
        let p = pal(&bytes, w);
        assert_eq!(p.len(), 2);
        assert!(de(p[0].color, [100, 100, 40]) < 1.0 && de(p[1].color, [100, 100, 220]) < 1.0);
    }

    // ARQ-122: the order is by the area of the whole colour, noise included, not by the
    // fullest single bin.
    #[test]
    fn arq_122_a_noisy_large_colour_comes_before_a_small_flat_one() {
        let mut v = Vec::new();
        for _ in 0..50 {
            v.extend(op(GREEN));
        }
        for i in 0..150u32 {
            let j = |s: u32| ((i.wrapping_mul(2_654_435_761) >> s) % 25) as i32 - 12;
            let c = |base: i32, s| (base + j(s)).clamp(0, 255) as u8;
            v.extend([c(40, 3), c(60, 9), c(210, 15), 255]);
        }
        let p = pal(&v, 200);
        assert_eq!(p.len(), 2, "{p:?}");
        assert_eq!((p[0].pixels, p[1].pixels), (150, 50));
        assert!(de(p[0].color, BLUE) < 4.0 && de(p[1].color, GREEN) < 1.0);
    }

    // ARQ-122, 4.17.4: a value far beyond 1 in `f32` is a full channel, so a bright red next
    // to it is the same colour.
    #[test]
    fn arq_122_f32_values_beyond_the_range_do_not_split_a_colour() {
        let px = [1.0e30f32, 0.0, 0.0, 1.0, 0.8, 0.0, 0.0, 1.0];
        let p = dominant_colors(ImageView::new(2, 1, PixelData::RgbaF32(&px)).unwrap()).unwrap();
        assert_eq!(p.len(), 1, "{p:?}");
        assert_eq!(p[0].pixels, 2);
    }

    // ARQ-110 and formats.
    #[test]
    fn arq_110_palette_rejects_non_finite_pixels_and_alpha8() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let px = [0.1f32, 0.2, 0.3, 1.0, 0.1, bad, 0.3, 1.0];
            assert_eq!(
                dominant_colors(ImageView::new(2, 1, PixelData::RgbaF32(&px)).unwrap()),
                Err(EffectError::Buffer(BufferError::NonFinite {
                    field: "src",
                    index: 5
                }))
            );
        }
        let a8 = [1u8, 2, 3, 4];
        assert_eq!(
            dominant_colors(ImageView::new(2, 2, PixelData::Alpha8(&a8)).unwrap()),
            Err(EffectError::UnsupportedFormat {
                format: PixelFormat::Alpha8
            })
        );
    }

    // ARQ-111: the same image gives the same palette, bit for bit.
    #[test]
    fn arq_111_palette_is_deterministic() {
        let (bytes, w) = runs(&[
            (op(RED), 33),
            (op(GREEN), 33),
            (op(BLUE), 33),
            (op(GREY), 33),
        ]);
        let a = pal(&bytes, w);
        let b = pal(&bytes, w);
        assert_eq!(a, b);
        // Equal areas come in the order of their bins (red, green, blue of 5 bits each).
        assert_eq!(a.len(), 4);
        for (e, rgb) in a.iter().zip([GREEN, BLUE, GREY, RED]) {
            assert!(de(e.color, rgb) < 1.0, "{rgb:?}");
        }
    }
}
