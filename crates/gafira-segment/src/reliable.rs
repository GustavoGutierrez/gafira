//! Reliability of a result (ARQ-131, D-23, D-44): the foreground covers 1 % to
//! 95 % of the image, and the mean colours of foreground and background, in
//! linear light weighted by the input alpha, are at ΔE76 5 or more.

use gafira_color::{Lab, LinearRgba, delta_e76};
use gafira_doc::contract::Cancelled;
use gafira_raster::{ImageView, PixelData};

use crate::meter::{CHUNK_ROWS, Meter};

/// Smallest ΔE76 between the mean colours that is reliable (PROPUESTA).
const MIN_DELTA_E: f64 = 5.0;

/// The rule on counts and colour distance. `foreground` and `total` count
/// pixels; `delta_e` is `None` when either group has no mean colour. The 1 % and
/// 95 % limits and the colour distance of 5 are inclusive.
pub(crate) fn decide(foreground: u64, total: u64, delta_e: Option<f64>) -> bool {
    let area = foreground * 100;
    area >= total && area <= 95 * total && delta_e.is_some_and(|d| d >= MIN_DELTA_E)
}

/// Per-group sums of the alpha-weighted linear colour.
#[derive(Clone, Copy, Default)]
struct Sums {
    colour: [f64; 3],
    weight: f64,
    pixels: u64,
}

impl Sums {
    fn lab(&self) -> Option<Lab> {
        if self.weight <= 0.0 {
            return None;
        }
        let [r, g, b] = self.colour.map(|v| (v / self.weight) as f32);
        Some(Lab::from_linear_srgb(LinearRgba::new(r, g, b, 1.0)))
    }
}

fn srgb_table() -> [f32; 256] {
    std::array::from_fn(|i| {
        let c = i as f32 / 255.0;
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// Adds the rows `y0..y1` of `img` to the sums: `[foreground, background]`.
/// The colour is read as stored (premultiplied, ARQ-130) and weighted by the
/// alpha; a pixel of alpha 0 adds nothing.
fn accumulate(
    img: &ImageView<'_>,
    mask: &[u8],
    (y0, y1): (usize, usize),
    table: &[f32; 256],
    sums: &mut [Sums; 2],
) {
    let w = img.width() as usize;
    for i in y0 * w..y1 * w {
        let group = usize::from(mask[i] < 128);
        let (linear, alpha) = match img.data() {
            PixelData::Rgba8(d) => (
                [0, 1, 2].map(|c| table[usize::from(d[i * 4 + c])]),
                f32::from(d[i * 4 + 3]) / 255.0,
            ),
            PixelData::RgbaF32(d) => ([d[i * 4], d[i * 4 + 1], d[i * 4 + 2]], d[i * 4 + 3]),
            PixelData::Alpha8(_) => ([0.0; 3], 0.0),
        };
        let s = &mut sums[group];
        s.pixels += 1;
        let weight = f64::from(alpha.max(0.0));
        for (acc, v) in s.colour.iter_mut().zip(linear) {
            *acc += weight * f64::from(v);
        }
        s.weight += weight;
    }
}

/// Mean colour of the foreground (mask alpha >= 128) and of the background, if
/// each has one. Whole image at once; the tests read the colour rule through it.
#[cfg(test)]
pub(crate) fn means(img: &ImageView<'_>, mask: &[u8]) -> [Option<Lab>; 2] {
    let table = srgb_table();
    let mut sums = [Sums::default(); 2];
    accumulate(img, mask, (0, img.height() as usize), &table, &mut sums);
    sums.map(|s| s.lab())
}

/// Whether `mask` is a plausible result for `img`. Reads the image in chunks of
/// rows, the token checked in each.
pub(crate) fn assess(
    img: &ImageView<'_>,
    mask: &[u8],
    meter: &Meter<'_>,
) -> Result<bool, Cancelled> {
    let table = srgb_table();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut sums = [Sums::default(); 2];
    for y0 in (0..h).step_by(CHUNK_ROWS) {
        let y1 = (y0 + CHUNK_ROWS).min(h);
        meter.step(((y1 - y0) * w) as u64)?;
        accumulate(img, mask, (y0, y1), &table, &mut sums);
    }
    let delta = match (sums[0].lab(), sums[1].lab()) {
        (Some(fg), Some(bg)) => Some(delta_e76(fg, bg)),
        _ => None,
    };
    Ok(decide(sums[0].pixels, (w * h) as u64, delta))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gafira_doc::contract::{CancelToken, NoProgress};
    use gafira_raster::PixelData;

    // ARQ-131: 1 % and 95 % are reliable, 0.99 % and 95.01 % are not; dE76 5 is
    // reliable and 4.999 is not; no colour is not reliable.
    // Mutants: either area limit exclusive; the colour threshold strict.
    #[test]
    fn arq_131_rule_bounds_are_inclusive() {
        let rows: [(u64, u64, Option<f64>, bool); 10] = [
            (100, 10_000, Some(9.0), true),
            (99, 10_000, Some(9.0), false),
            (9_500, 10_000, Some(9.0), true),
            (9_501, 10_000, Some(9.0), false),
            (5_000, 10_000, Some(5.0), true),
            (5_000, 10_000, Some(4.999), false),
            (5_000, 10_000, None, false),
            (0, 10_000, Some(50.0), false),
            (10_000, 10_000, Some(50.0), false),
            (50, 5_000, Some(5.0), true),
        ];
        for (fg, total, de, want) in rows {
            assert_eq!(decide(fg, total, de), want, "{fg}/{total} dE {de:?}");
        }
    }

    fn rgba8(px: &[[u8; 4]]) -> Vec<u8> {
        px.iter().flatten().copied().collect()
    }

    fn l_of(lab: Option<Lab>) -> f64 {
        lab.expect("a mean colour").l
    }

    // ARQ-131 / D-44: half black and half white average in linear light: L 76.07.
    // Mutant: average the encoded bytes (L 53).
    #[test]
    fn arq_131_means_are_averaged_in_linear_light() {
        let data = rgba8(&[
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            [10, 10, 10, 255],
            [10, 10, 10, 255],
        ]);
        let img = ImageView::new(4, 1, PixelData::Rgba8(&data)).unwrap();
        let [fg, bg] = means(&img, &[255, 255, 0, 0]);
        assert!((l_of(fg) - 76.07).abs() < 0.01, "L {}", l_of(fg));
        assert!(bg.is_some());
    }

    // ARQ-131: input alpha weights the mean; alpha 0 gives no colour.
    #[test]
    fn arq_131_means_are_weighted_by_input_alpha() {
        let case = |px: &[f32]| {
            let img = ImageView::new(2, 1, PixelData::RgbaF32(px)).unwrap();
            means(&img, &[255, 255])[0]
        };
        // White of alpha 1 and black of alpha 0: L 100.
        let lab = case(&[1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
        assert!((l_of(lab) - 100.0).abs() < 0.01, "L {}", l_of(lab));
        // White of alpha 0.25 and black of alpha 0.75: L 57.08.
        let lab = case(&[1.0, 1.0, 1.0, 0.25, 0.0, 0.0, 0.0, 0.75]);
        assert!((l_of(lab) - 57.08).abs() < 0.01, "L {}", l_of(lab));
        // A group whose pixels all have alpha 0 has no mean colour, and no NaN.
        assert!(case(&[1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]).is_none());
    }

    fn run(img: &ImageView<'_>, mask: &[u8]) -> bool {
        let (p, c) = (NoProgress, CancelToken::new());
        let meter = Meter::new(&p, &c, 1 << 30);
        assess(img, mask, &meter).unwrap()
    }

    // ARQ-131: the mask is binarised at alpha >= 128, as in ARQ-132, and the
    // 1 % bound counts those pixels.
    // Mutant: `> 128`.
    #[test]
    fn arq_131_foreground_is_alpha_128_or_more() {
        let mut data = Vec::new();
        for i in 0..10_000 {
            let v = if i < 100 { 250 } else { 20 };
            data.extend_from_slice(&[v, v, v, 255]);
        }
        let img = ImageView::new(100, 100, PixelData::Rgba8(&data)).unwrap();
        let mask = |n: usize, v: u8| -> Vec<u8> {
            (0..10_000).map(|i| if i < n { v } else { 0 }).collect()
        };
        assert!(run(&img, &mask(100, 128)), "128 counts");
        assert!(!run(&img, &mask(100, 127)), "127 does not");
        assert!(!run(&img, &mask(99, 255)), "99 pixels are 0.99 %");
    }

    // ARQ-131: a uniform image is not reliable whatever the mask; two distinct
    // colours with a mask between them are.
    #[test]
    fn arq_131_uniform_is_unreliable_and_distinct_colours_are_reliable() {
        let uniform = rgba8(&[[90, 90, 90, 255]; 100]);
        let img = ImageView::new(10, 10, PixelData::Rgba8(&uniform)).unwrap();
        let half: Vec<u8> = (0..100).map(|i| if i < 50 { 255 } else { 0 }).collect();
        assert!(!run(&img, &half));
        let two: Vec<[u8; 4]> = (0..100)
            .map(|i| {
                if i < 50 {
                    [200, 30, 30, 255]
                } else {
                    [30, 30, 200, 255]
                }
            })
            .collect();
        let data = rgba8(&two);
        let img = ImageView::new(10, 10, PixelData::Rgba8(&data)).unwrap();
        assert!(run(&img, &half));
        // An empty mask and a full mask leave a group without pixels.
        assert!(!run(&img, &[0; 100]));
        assert!(!run(&img, &[255; 100]));
    }

    // ARQ-112: the token is read while assessing.
    #[test]
    fn arq_112_assess_stops_on_a_cancelled_token() {
        let data = rgba8(&[[1, 2, 3, 255]; 4]);
        let img = ImageView::new(2, 2, PixelData::Rgba8(&data)).unwrap();
        let (p, c) = (NoProgress, CancelToken::new());
        c.cancel();
        let meter = Meter::new(&p, &c, 100);
        assert_eq!(assess(&img, &[255, 0, 0, 0], &meter), Err(Cancelled));
    }
}
