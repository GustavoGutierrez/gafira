//! Mip levels (ARQ-121).

use gafira_color::{AnalyticTransform, ColorSpaceId, ColorTransform, PixelLayout};

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

/// The four source indices of one output pixel on one axis. A side of 1 repeats.
fn pair(out: usize, side: usize) -> [usize; 2] {
    [(2 * out).min(side - 1), (2 * out + 1).min(side - 1)]
}

/// Visits every output pixel with the four source pixel indices of its 2×2 box.
fn for_each_box(
    (sw, sh): (usize, usize),
    (dw, dh): (usize, usize),
    mut f: impl FnMut(usize, [usize; 4]),
) {
    for y in 0..dh {
        let ys = pair(y, sh);
        for x in 0..dw {
            let xs = pair(x, sw);
            f(
                y * dw + x,
                [
                    ys[0] * sw + xs[0],
                    ys[0] * sw + xs[1],
                    ys[1] * sw + xs[0],
                    ys[1] * sw + xs[1],
                ],
            );
        }
    }
}

/// sRGB to linear light and back, through the layer-0 transform.
struct Transfer {
    decode: [f64; 256],
    to_srgb: AnalyticTransform,
}

impl Transfer {
    fn new() -> Self {
        let to_linear = AnalyticTransform::new(ColorSpaceId::Srgb, ColorSpaceId::LinearSrgb);
        let mut decode = [0.0; 256];
        for (v, slot) in (0u8..=255).zip(&mut decode) {
            let c = f32::from(v) / 255.0;
            let mut px = [c, c, c, 1.0];
            // A four-channel buffer is always whole, so this cannot fail.
            let _ = to_linear.apply_f32(&mut px, PixelLayout::Rgba);
            *slot = f64::from(px[0]);
        }
        Self {
            decode,
            to_srgb: AnalyticTransform::new(ColorSpaceId::LinearSrgb, ColorSpaceId::Srgb),
        }
    }

    fn encode(&self, linear: [f64; 3]) -> [u8; 3] {
        let mut px = [linear[0] as f32, linear[1] as f32, linear[2] as f32, 1.0];
        let _ = self.to_srgb.apply_f32(&mut px, PixelLayout::Rgba);
        let q = |v: f32| (f64::from(v).clamp(0.0, 1.0) * 255.0 + 0.5).floor() as u8;
        [q(px[0]), q(px[1]), q(px[2])]
    }
}

fn reduce_rgba8(src: &[u8], s: (usize, usize), d: (usize, usize), t: &Transfer) -> Vec<u8> {
    let mut out = vec![0u8; d.0 * d.1 * 4];
    for_each_box(s, d, |o, idx| {
        let mut sum = [0.0f64; 3];
        let mut alpha_sum = 0u32;
        for i in idx {
            let p = &src[i * 4..i * 4 + 4];
            let a = p[3];
            if a == 0 {
                continue;
            }
            alpha_sum += u32::from(a);
            for k in 0..3 {
                let straight = unpremultiply_u8(p[k], a);
                sum[k] += t.decode[usize::from(straight)] * f64::from(a);
            }
        }
        if alpha_sum == 0 {
            return;
        }
        let w = f64::from(alpha_sum);
        let straight = t.encode([sum[0] / w, sum[1] / w, sum[2] / w]);
        let a_out = u8::try_from((alpha_sum + 2) / 4).unwrap_or(u8::MAX);
        for k in 0..3 {
            out[o * 4 + k] = premultiply_u8(straight[k], a_out);
        }
        out[o * 4 + 3] = a_out;
    });
    out
}

fn reduce_alpha8(src: &[u8], s: (usize, usize), d: (usize, usize)) -> Vec<u8> {
    let mut out = vec![0u8; d.0 * d.1];
    for_each_box(s, d, |o, idx| {
        let sum: u32 = idx.iter().map(|&i| u32::from(src[i])).sum();
        out[o] = u8::try_from((sum + 2) / 4).unwrap_or(u8::MAX);
    });
    out
}

fn reduce_f32(src: &[f32], s: (usize, usize), d: (usize, usize)) -> Vec<f32> {
    let mut out = vec![0f32; d.0 * d.1 * 4];
    for_each_box(s, d, |o, idx| {
        for k in 0..4 {
            let sum: f64 = idx.iter().map(|&i| f64::from(src[i * 4 + k])).sum();
            out[o * 4 + k] = (sum / 4.0) as f32;
        }
    });
    out
}

/// Builds every level, level 0 being a copy of `src`. Averages 2×2 boxes in linear
/// light with premultiplied alpha. An odd side drops its last row or column.
///
/// # Errors
///
/// [`BufferError::NonFinite`] when an `f32` pixel is NaN or infinite.
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
    let mut current = match src.data() {
        PixelData::Rgba8(d) => PixelBuf::Rgba8(d.to_vec()),
        PixelData::RgbaF32(d) => PixelBuf::RgbaF32(d.to_vec()),
        PixelData::Alpha8(d) => PixelBuf::Alpha8(d.to_vec()),
    };
    let (mut w, mut h) = (src.width(), src.height());
    levels.push(ImageBuf::new(w, h, current.clone())?);
    for _ in 1..count {
        let (nw, nh) = mip_size(w, h);
        let sd = (w as usize, h as usize);
        let dd = (nw as usize, nh as usize);
        current = match &current {
            PixelBuf::Rgba8(d) => PixelBuf::Rgba8(reduce_rgba8(d, sd, dd, &transfer)),
            PixelBuf::RgbaF32(d) => PixelBuf::RgbaF32(reduce_f32(d, sd, dd)),
            PixelBuf::Alpha8(d) => PixelBuf::Alpha8(reduce_alpha8(d, sd, dd)),
        };
        (w, h) = (nw, nh);
        levels.push(ImageBuf::new(w, h, current.clone())?);
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

    // ARQ-121: an odd side drops its last row or column.
    #[test]
    fn arq_121_odd_side_uses_the_first_even_part() {
        let a = [10u8, 10, 10, 255];
        let b = [10u8, 10, 10, 255];
        let junk = [250u8, 250, 250, 255];
        let chain = mips(&rgba8(3, 1, &[a, b, junk].concat()));
        assert_eq!(bytes(&chain[1]), a);
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
}
