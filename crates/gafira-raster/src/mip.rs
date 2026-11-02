//! Mip levels (ARQ-121).

use crate::transfer::Transfer;
use crate::{
    BufferError, ImageBuf, ImageView, PixelBuf, PixelData, premultiply_u8, unpremultiply_u8,
};

/// Number of levels: `⌊log2(max(width, height))⌋ + 1`, and 0 for an empty image.
#[must_use]
pub fn mip_level_count(width: u32, height: u32) -> u32 {
    if width == 0 || height == 0 {
        0
    } else {
        u32::BITS - width.max(height).leading_zeros()
    }
}

/// Size of the next level: each side halved, rounded down, minimum 1.
#[must_use]
pub fn mip_size(width: u32, height: u32) -> (u32, u32) {
    ((width / 2).max(1), (height / 2).max(1))
}

/// Sums of the original pixels one level covers, four values per pixel.
///
/// Every level is built from the sums of the level above, never from rounded
/// pixels, so a level equals the mean of the original pixels it covers.
struct Sums {
    width: usize,
    height: usize,
    /// Original columns covered by each column.
    cover_x: Vec<u64>,
    /// Original rows covered by each row.
    cover_y: Vec<u64>,
    values: Vec<[f64; 4]>,
}

impl Sums {
    /// Pixels of the original image, one `[f64; 4]` each.
    fn base(width: usize, height: usize, fetch: impl Fn(usize) -> [f64; 4]) -> Self {
        Self {
            width,
            height,
            cover_x: vec![1; width],
            cover_y: vec![1; height],
            values: (0..width * height).map(fetch).collect(),
        }
    }

    /// The next level. Output `x` takes source `2x` up to `2x + 2`, and the last
    /// output takes everything left, so an odd side loses no pixel.
    fn halve(&self) -> Self {
        let (width, height) = {
            let (w, h) = mip_size(self.width as u32, self.height as u32);
            (w as usize, h as usize)
        };
        let span = |out: usize, side: usize, out_side: usize| {
            (
                2 * out,
                if out + 1 == out_side {
                    side
                } else {
                    2 * out + 2
                },
            )
        };
        let cover = |cover: &[u64], side: usize, out_side: usize| -> Vec<u64> {
            (0..out_side)
                .map(|o| {
                    let (a, b) = span(o, side, out_side);
                    cover[a..b].iter().sum()
                })
                .collect()
        };
        let mut values = Vec::with_capacity(width * height);
        for y in 0..height {
            let (y0, y1) = span(y, self.height, height);
            for x in 0..width {
                let (x0, x1) = span(x, self.width, width);
                let mut acc = [0.0f64; 4];
                for sy in y0..y1 {
                    for v in &self.values[sy * self.width + x0..sy * self.width + x1] {
                        for k in 0..4 {
                            acc[k] += v[k];
                        }
                    }
                }
                values.push(acc);
            }
        }
        Self {
            width,
            height,
            cover_x: cover(&self.cover_x, self.width, width),
            cover_y: cover(&self.cover_y, self.height, height),
            values,
        }
    }

    /// Area in original pixels of output pixel `(x, y)`.
    fn area(&self, x: usize, y: usize) -> f64 {
        (self.cover_x[x] * self.cover_y[y]) as f64
    }

    fn to_rgba8(&self, t: &Transfer) -> Vec<u8> {
        let mut out = vec![0u8; self.width * self.height * 4];
        for y in 0..self.height {
            for x in 0..self.width {
                let o = y * self.width + x;
                let s = self.values[o];
                if s[3] <= 0.0 {
                    continue;
                }
                let a_out = (s[3] / self.area(x, y) + 0.5).floor().clamp(0.0, 255.0) as u8;
                let straight = t.encode([s[0] / s[3], s[1] / s[3], s[2] / s[3]]);
                for k in 0..3 {
                    out[o * 4 + k] = premultiply_u8(straight[k], a_out);
                }
                out[o * 4 + 3] = a_out;
            }
        }
        out
    }

    fn to_alpha8(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.width * self.height];
        for y in 0..self.height {
            for x in 0..self.width {
                let o = y * self.width + x;
                out[o] = (self.values[o][0] / self.area(x, y) + 0.5)
                    .floor()
                    .clamp(0.0, 255.0) as u8;
            }
        }
        out
    }

    fn to_f32(&self) -> Vec<f32> {
        let mut out = vec![0f32; self.width * self.height * 4];
        for y in 0..self.height {
            for x in 0..self.width {
                let o = y * self.width + x;
                for k in 0..4 {
                    out[o * 4 + k] = (self.values[o][k] / self.area(x, y)) as f32;
                }
            }
        }
        out
    }
}

/// Builds every level, level 0 being a copy of `src`. Each level pixel is the mean,
/// in linear light with premultiplied alpha, of the original pixels it covers, all
/// with the same weight. An odd side joins its last row or column to the last box.
/// Levels come from unrounded sums of level 0, so rounding does not build up. A
/// pixel with `a = 0` adds no colour.
///
/// # Errors
///
/// [`BufferError::NonFinite`] when an `f32` pixel is NaN or infinite, also in a
/// pixel with `a = 0`.
pub fn build_mips(src: ImageView<'_>) -> Result<Vec<ImageBuf>, BufferError> {
    if let PixelData::RgbaF32(px) = src.data()
        && let Some(index) = px.iter().position(|v| !v.is_finite())
    {
        return Err(BufferError::NonFinite {
            field: "pixels",
            index,
        });
    }
    let count = mip_level_count(src.width(), src.height());
    let mut levels = Vec::new();
    if count == 0 {
        return Ok(levels);
    }
    let transfer = Transfer::new();
    let (w, h) = (src.width() as usize, src.height() as usize);
    let (mut sums, copy) = match src.data() {
        PixelData::Rgba8(d) => (
            Sums::base(w, h, |i| {
                let p = &d[i * 4..i * 4 + 4];
                let a = p[3];
                if a == 0 {
                    return [0.0; 4];
                }
                let lin = |k: usize| {
                    transfer.decode[usize::from(unpremultiply_u8(p[k], a))] * f64::from(a)
                };
                [lin(0), lin(1), lin(2), f64::from(a)]
            }),
            PixelBuf::Rgba8(d.to_vec()),
        ),
        PixelData::RgbaF32(d) => (
            Sums::base(w, h, |i| {
                let p = &d[i * 4..i * 4 + 4];
                if p[3] == 0.0 {
                    return [0.0; 4];
                }
                [
                    f64::from(p[0]),
                    f64::from(p[1]),
                    f64::from(p[2]),
                    f64::from(p[3]),
                ]
            }),
            PixelBuf::RgbaF32(d.to_vec()),
        ),
        PixelData::Alpha8(d) => (
            Sums::base(w, h, |i| [f64::from(d[i]), 0.0, 0.0, 0.0]),
            PixelBuf::Alpha8(d.to_vec()),
        ),
    };
    levels.push(ImageBuf::new(src.width(), src.height(), copy)?);
    for _ in 1..count {
        sums = sums.halve();
        let data = match src.data() {
            PixelData::Rgba8(_) => PixelBuf::Rgba8(sums.to_rgba8(&transfer)),
            PixelData::RgbaF32(_) => PixelBuf::RgbaF32(sums.to_f32()),
            PixelData::Alpha8(_) => PixelBuf::Alpha8(sums.to_alpha8()),
        };
        levels.push(ImageBuf::new(sums.width as u32, sums.height as u32, data)?);
    }
    Ok(levels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srgb_to_linear(v: f64) -> f64 {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }

    fn linear_to_srgb(v: f64) -> f64 {
        if v <= 0.003_130_8 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    }

    fn rgba8(w: u32, h: u32, px: &[u8]) -> ImageBuf {
        ImageBuf::new(w, h, PixelBuf::Rgba8(px.to_vec())).unwrap()
    }

    fn mips(img: &ImageBuf) -> Vec<ImageBuf> {
        build_mips(img.view()).unwrap()
    }

    fn bytes(img: &ImageBuf) -> &[u8] {
        match img.data() {
            PixelBuf::Rgba8(d) | PixelBuf::Alpha8(d) => d,
            PixelBuf::RgbaF32(_) => panic!("not 8-bit"),
        }
    }

    // ARQ-121: floor(log2(max side)) + 1, at powers of two and one past them.
    #[test]
    fn arq_121_level_count_at_powers_of_two() {
        let cases: [(u32, u32, u32); 17] = [
            (1000, 600, 10),
            (1, 1, 1),
            (2, 1, 2),
            (3, 1, 2),
            (4, 4, 3),
            (1, 1023, 10),
            (1024, 1, 11),
            (1025, 1, 11),
            (1, 2047, 11),
            (2048, 2048, 12),
            (600, 1000, 10),
            (7, 7, 3),
            (8, 8, 4),
            (u32::MAX, 1, 32),
            (0, 0, 0),
            (0, 9, 0),
            (9, 0, 0),
        ];
        for (w, h, n) in cases {
            assert_eq!(mip_level_count(w, h), n, "{w}x{h}");
        }
    }

    // ARQ-121: sides halve rounding down, never below 1, on either axis.
    #[test]
    fn arq_121_mip_size_rounds_down_with_minimum_one() {
        let cases: [((u32, u32), (u32, u32)); 9] = [
            ((1000, 600), (500, 300)),
            ((5, 3), (2, 1)),
            ((3, 3), (1, 1)),
            ((1, 1), (1, 1)),
            ((1, 7), (1, 3)),
            ((7, 1), (3, 1)),
            ((2, 1), (1, 1)),
            ((1, 2), (1, 1)),
            ((u32::MAX, u32::MAX), (2_147_483_647, 2_147_483_647)),
        ];
        for ((w, h), want) in cases {
            assert_eq!(mip_size(w, h), want, "{w}x{h}");
        }
    }

    // ARQ-121: 1000x600 gives 10 levels, the last 1x1, with the stated sizes.
    #[test]
    fn arq_121_chain_of_1000_by_600_has_ten_levels_ending_1x1() {
        let img = rgba8(1000, 600, &vec![0u8; 1000 * 600 * 4]);
        let chain = mips(&img);
        assert_eq!(chain.len(), 10);
        let dims: Vec<_> = chain.iter().map(|l| (l.width(), l.height())).collect();
        assert_eq!(dims[0], (1000, 600));
        assert_eq!(dims[1], (500, 300));
        assert_eq!(dims[9], (1, 1));
        for pair in dims.windows(2) {
            assert_eq!(mip_size(pair[0].0, pair[0].1), pair[1]);
        }
        // Thin images keep the short side at 1 until the long side reaches it.
        let thin = rgba8(5, 1, &[0u8; 20]);
        let dims: Vec<_> = mips(&thin)
            .iter()
            .map(|l| (l.width(), l.height()))
            .collect();
        assert_eq!(dims, [(5, 1), (2, 1), (1, 1)]);
        assert_eq!(mips(&rgba8(1, 1, &[1, 2, 3, 4])).len(), 1);
    }

    // ARQ-121: level 0 is the source; an empty image has no levels.
    #[test]
    fn arq_121_level_zero_is_source_and_empty_image_has_no_levels() {
        let px: Vec<u8> = (0..4 * 6).map(|i| i as u8).collect();
        let img = rgba8(3, 2, &px);
        assert_eq!(mips(&img)[0], img);
        let empty = rgba8(0, 4, &[]);
        assert!(mips(&empty).is_empty());
    }

    // ARQ-121: a constant image is the same at every level.
    #[test]
    fn arq_121_constant_image_is_equal_at_all_levels() {
        for colour in [
            [30u8, 100, 60, 128],
            [0, 0, 0, 255],
            [255, 255, 255, 255],
            [7, 9, 3, 10],
            [0, 0, 0, 0],
        ] {
            let px: Vec<u8> = colour.iter().copied().cycle().take(37 * 21 * 4).collect();
            let chain = mips(&rgba8(37, 21, &px));
            assert_eq!(chain.len(), 6);
            for level in &chain {
                for p in bytes(level).chunks_exact(4) {
                    assert_eq!(
                        p,
                        colour,
                        "colour {colour:?} at {}x{}",
                        level.width(),
                        level.height()
                    );
                }
            }
        }
    }

    // ARQ-121: opaque sRGB 0 and 255 in a checkerboard average to 188 in linear light.
    #[test]
    fn arq_121_checkerboard_level_one_is_188() {
        let (k, w) = ([0u8, 0, 0, 255], [255u8, 255, 255, 255]);
        let px: Vec<u8> = [k, w, w, k].concat();
        let chain = mips(&rgba8(2, 2, &px));
        let got = bytes(&chain[1]);
        assert_eq!(chain[1].width(), 1);
        for c in &got[..3] {
            assert!((i32::from(*c) - 188).abs() <= 1, "{got:?}");
        }
        assert_eq!(got[3], 255);
        // The 2x1 case repeats the row, so it averages the same two pixels.
        let chain = mips(&rgba8(2, 1, &[k, w].concat()));
        assert!((i32::from(bytes(&chain[1])[0]) - 188).abs() <= 1);
    }

    // ARQ-121: a pixel with a = 0 adds no colour, whatever bytes it holds.
    #[test]
    fn arq_121_transparent_pixel_adds_no_colour() {
        let opaque = [255u8, 255, 255, 255];
        let ghost = [200u8, 10, 90, 0];
        let chain = mips(&rgba8(2, 2, &[opaque, ghost, ghost, ghost].concat()));
        // alpha (255 + 2) / 4 = 64; white premultiplied by 64 is 64.
        assert_eq!(bytes(&chain[1]), [64, 64, 64, 64]);
        let chain = mips(&rgba8(2, 2, &[ghost, ghost, ghost, ghost].concat()));
        assert_eq!(bytes(&chain[1]), [0, 0, 0, 0]);
        // Next to opaque black, a bright transparent pixel leaves the colour black.
        let black = [0u8, 0, 0, 255];
        let bright = [255u8, 255, 255, 0];
        let chain = mips(&rgba8(2, 1, &[black, bright].concat()));
        assert_eq!(bytes(&chain[1]), [0, 0, 0, 128]);
    }

    // ARQ-121: the mean is linear and weighted by alpha; compare with an f64 reference.
    #[test]
    fn arq_121_level_one_matches_a_linear_premultiplied_reference() {
        let mut seed = 12345u32;
        let mut next = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 24) as u8
        };
        let mut px = Vec::new();
        for _ in 0..8 * 8 {
            let a = next();
            let straight = [next(), next(), next()];
            px.extend(straight.iter().map(|c| crate::premultiply_u8(*c, a)));
            px.push(a);
        }
        let chain = mips(&rgba8(8, 8, &px));
        let l1 = bytes(&chain[1]);
        assert_eq!((chain[1].width(), chain[1].height()), (4, 4));
        for oy in 0..4usize {
            for ox in 0..4usize {
                let mut sums = [0.0f64; 3];
                let mut asum = 0.0f64;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let i = ((2 * oy + dy) * 8 + 2 * ox + dx) * 4;
                    let a = f64::from(px[i + 3]) / 255.0;
                    asum += a;
                    if px[i + 3] != 0 {
                        for k in 0..3 {
                            let s =
                                f64::from(crate::unpremultiply_u8(px[i + k], px[i + 3])) / 255.0;
                            sums[k] += srgb_to_linear(s) * a;
                        }
                    }
                }
                let o = (oy * 4 + ox) * 4;
                let a_out = ((asum / 4.0) * 255.0).round() as u8;
                assert!(
                    (i32::from(l1[o + 3]) - i32::from(a_out)).abs() <= 1,
                    "alpha at {ox},{oy}"
                );
                if asum > 0.0 {
                    for k in 0..3 {
                        let s = (linear_to_srgb(sums[k] / asum) * 255.0).round() as u8;
                        let want = i32::from(crate::premultiply_u8(s, l1[o + 3]));
                        assert!(
                            (i32::from(l1[o + k]) - want).abs() <= 1,
                            "channel {k} at {ox},{oy}: {} vs {want}",
                            l1[o + k]
                        );
                    }
                }
            }
        }
    }

    // ARQ-121: with unequal alpha the mean follows alpha, so it is not 188.
    #[test]
    fn arq_121_mean_is_weighted_by_alpha() {
        let white = [255u8, 255, 255, 255];
        let half_black = [0u8, 0, 0, 128];
        let chain = mips(&rgba8(2, 1, &[white, half_black].concat()));
        // straight linear = 1 / (1 + 128/255) = 0.6657 -> sRGB 213; alpha = 192.
        let got = bytes(&chain[1]);
        assert_eq!(got[3], 192);
        assert_eq!(got[0], crate::premultiply_u8(213, 192));
    }

    // ARQ-121: 8-bit coverage averages with rounding; f32 averages linear values.
    #[test]
    fn arq_121_alpha8_and_f32_levels() {
        let m = ImageBuf::new(2, 2, PixelBuf::Alpha8(vec![10, 20, 30, 42])).unwrap();
        let chain = mips(&m);
        assert_eq!(chain.len(), 2);
        // 102 / 4 = 25.5 rounds up; 101 / 4 = 25.25 rounds down.
        assert_eq!(bytes(&chain[1]), [26]);
        let m = ImageBuf::new(2, 2, PixelBuf::Alpha8(vec![10, 20, 30, 41])).unwrap();
        assert_eq!(bytes(&mips(&m)[1]), [25]);
        let f = ImageBuf::new(
            2,
            2,
            PixelBuf::RgbaF32(vec![
                1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0,
            ]),
        )
        .unwrap();
        let chain = mips(&f);
        assert_eq!(
            chain[1].data(),
            &PixelBuf::RgbaF32(vec![0.25, 0.25, 0.25, 1.0])
        );
        // Values outside [0, 1] are admitted.
        let f = ImageBuf::new(
            2,
            1,
            PixelBuf::RgbaF32(vec![-1.0, 2.0, 0.0, 1.0, -1.0, 2.0, 0.0, 1.0]),
        )
        .unwrap();
        assert_eq!(
            mips(&f)[1].data(),
            &PixelBuf::RgbaF32(vec![-1.0, 2.0, 0.0, 1.0])
        );
    }

    // ARQ-110: NaN or infinity anywhere, even in a dropped column, is Err.
    #[test]
    fn arq_110_non_finite_pixel_is_err_for_f32() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for at in 0..12 {
                let mut px = vec![0.5f32; 12];
                px[at] = bad;
                let data = PixelData::RgbaF32(&px);
                let view = ImageView::new(3, 1, data).unwrap();
                assert_eq!(
                    build_mips(view).unwrap_err(),
                    BufferError::NonFinite {
                        field: "pixels",
                        index: at
                    },
                    "{bad} at {at}"
                );
            }
        }
    }

    // ARQ-111: same input, same bytes.
    #[test]
    fn arq_111_build_mips_is_deterministic() {
        let px: Vec<u8> = (0..16 * 9 * 4).map(|i| (i * 7 % 251) as u8).collect();
        let img = rgba8(16, 9, &px);
        assert_eq!(mips(&img).len(), 5);
        assert_eq!(mips(&img), mips(&img));
        let f: Vec<f32> = (0..16 * 9 * 4).map(|i| (i % 13) as f32 * 0.07).collect();
        let img = ImageBuf::new(16, 9, PixelBuf::RgbaF32(f)).unwrap();
        assert_eq!(mips(&img), mips(&img));
        let img = ImageBuf::new(16, 9, PixelBuf::Alpha8(px[..144].to_vec())).unwrap();
        assert_eq!(mips(&img), mips(&img));
    }

    fn f32_img(w: u32, h: u32, px: Vec<f32>) -> ImageBuf {
        ImageBuf::new(w, h, PixelBuf::RgbaF32(px)).unwrap()
    }

    fn alpha8(w: u32, h: u32, px: Vec<u8>) -> ImageBuf {
        ImageBuf::new(w, h, PixelBuf::Alpha8(px)).unwrap()
    }

    fn near(got: u8, want: i32, tol: i32) -> bool {
        (i32::from(got) - want).abs() <= tol
    }

    // ARQ-121: an f32 pixel with a = 0 adds no colour, whatever it holds.
    #[test]
    fn arq_121_f32_transparent_pixel_adds_no_colour() {
        let img = f32_img(
            2,
            2,
            vec![
                1.0, 1.0, 1.0, 0.0, 0.1, 0.2, 0.3, 1.0, 0.1, 0.2, 0.3, 1.0, 1.0, 0.0, 0.0, 0.0,
            ],
        );
        let PixelBuf::RgbaF32(got) = mips(&img)[1].data().clone() else {
            panic!("format")
        };
        for (g, w) in got.iter().zip([0.05f32, 0.1, 0.15, 0.5]) {
            assert!((g - w).abs() < 1e-6, "{got:?}");
        }
        let ghost = f32_img(2, 2, [1.0f32, 1.0, 1.0, 0.0].repeat(4));
        assert_eq!(mips(&ghost)[1].data(), &PixelBuf::RgbaF32(vec![0.0; 4]));
        // -0.0 is also zero alpha.
        let neg = f32_img(1, 1, vec![1.0, 1.0, 1.0, -0.0]);
        assert_eq!(mips(&neg).len(), 1);
    }

    // ARQ-121: odd sides keep every pixel; Alpha8 table rows of the spec.
    #[test]
    fn arq_121_odd_side_counts_every_pixel_alpha8() {
        let l1 = |img: &ImageBuf| bytes(&mips(img)[1]).to_vec();
        assert!(near(l1(&alpha8(3, 1, vec![0, 0, 255]))[0], 85, 1));
        assert!(near(l1(&alpha8(1, 3, vec![0, 0, 255]))[0], 85, 1));
        let mut m = vec![0u8; 9];
        for i in [2, 5, 6, 7, 8] {
            m[i] = 255;
        }
        assert!(near(l1(&alpha8(3, 3, m))[0], 142, 1));
        let five = alpha8(5, 1, vec![0, 0, 0, 255, 255]);
        let chain = mips(&five);
        let lv = bytes(&chain[1]);
        assert_eq!((chain[1].width(), lv[0]), (2, 0));
        assert!(near(lv[1], 170, 1), "{lv:?}");
        assert_eq!(chain.len(), 3);
        // The top level is the mean of all five.
        assert!(near(bytes(&chain[2])[0], 102, 1));
    }

    // ARQ-121: opaque Rgba8 3x1 (sRGB 0, 0, 255) gives 156, on both axes.
    #[test]
    fn arq_121_odd_side_counts_every_pixel_rgba8() {
        let (k, w) = ([0u8, 0, 0, 255], [255u8, 255, 255, 255]);
        for (width, height) in [(3, 1), (1, 3)] {
            let chain = mips(&rgba8(width, height, &[k, k, w].concat()));
            let got = bytes(&chain[1]);
            assert!(
                near(got[0], 156, 1) && near(got[1], 156, 1) && near(got[2], 156, 1),
                "{got:?}"
            );
            assert_eq!(got[3], 255);
        }
    }

    // ARQ-121: 1000x600 with 512 columns at 0 and 488 at 255 ends at 185, not 0 or 156.
    #[test]
    fn arq_121_wide_image_top_level_is_the_exact_mean() {
        let mut px = Vec::with_capacity(1000 * 600 * 4);
        for _ in 0..600 {
            for x in 0..1000 {
                let v = if x < 512 { 0 } else { 255 };
                px.extend([v, v, v, 255]);
            }
        }
        let chain = mips(&rgba8(1000, 600, &px));
        let top = bytes(&chain[9]);
        assert!(near(top[0], 185, 2), "{top:?}");
        assert!(near(top[1], 185, 2) && near(top[2], 185, 2) && top[3] == 255);
    }

    // ARQ-121: 1x1 has one level equal to the input, in every format.
    #[test]
    fn arq_121_one_by_one_has_one_level_equal_to_input() {
        for img in [
            rgba8(1, 1, &[10, 20, 30, 200]),
            f32_img(1, 1, vec![0.1, 0.2, 0.3, 0.4]),
            alpha8(1, 1, vec![77]),
        ] {
            assert_eq!(mips(&img), vec![img.clone()]);
        }
    }

    // ARQ-121: a constant image is equal at every level, odd sides included.
    #[test]
    fn arq_121_constant_image_all_formats() {
        for (w, h) in [(37u32, 21u32), (5, 3), (1, 9), (8, 8)] {
            let n = (w * h) as usize;
            let f = f32_img(w, h, [0.125f32, 0.25, 0.5, 0.5].repeat(n));
            let m = alpha8(w, h, vec![77; n]);
            for img in [f, m] {
                let chain = mips(&img);
                assert_eq!(chain.len() as u32, mip_level_count(w, h));
                for level in &chain {
                    let want = match img.data() {
                        PixelBuf::RgbaF32(_) => PixelBuf::RgbaF32(
                            [0.125f32, 0.25, 0.5, 0.5]
                                .repeat((level.width() * level.height()) as usize),
                        ),
                        _ => PixelBuf::Alpha8(vec![77; (level.width() * level.height()) as usize]),
                    };
                    assert_eq!(level.data(), &want, "{w}x{h}");
                }
            }
        }
    }

    // ARQ-121: an image with a zero side has no levels, in every format.
    #[test]
    fn arq_121_zero_side_images_have_no_levels() {
        for (w, h) in [(0u32, 5u32), (5, 0), (0, 0)] {
            assert!(mips(&rgba8(w, h, &[])).is_empty(), "{w}x{h}");
            assert!(mips(&f32_img(w, h, vec![])).is_empty());
            assert!(mips(&alpha8(w, h, vec![])).is_empty());
        }
    }

    // ARQ-110: non-finite colour in an a = 0 pixel is still Err, never skipped.
    #[test]
    fn arq_110_non_finite_colour_in_transparent_pixel_is_err() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for ch in 0..3 {
                let mut px = vec![0.5f32, 0.5, 0.5, 1.0, 0.1, 0.1, 0.1, 0.0];
                px[4 + ch] = bad;
                let view = ImageView::new(2, 1, PixelData::RgbaF32(&px)).unwrap();
                assert_eq!(
                    build_mips(view).unwrap_err(),
                    BufferError::NonFinite {
                        field: "pixels",
                        index: 4 + ch
                    },
                    "{bad} ch {ch}"
                );
            }
        }
    }
}
