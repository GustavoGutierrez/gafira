//! Gaussian blur of one plane: direct separable kernel, tap-outer loops, truncated at 3σ (P11).

/// Largest kernel half width in taps: 3σ of blur 100 (radius 100, σ 50, 150 taps) at the
/// maximum zoom 32 of FD-08. A wider radius is cut here, in `half_width`, so that `roi`, cost
/// and pixels agree and a huge `cx.scale` cannot ask for a huge allocation.
pub(crate) const MAX_HALF_WIDTH: usize = 4800;

/// Taps on each side of the centre for a CSS-style `radius_px` (σ = radius / 2), 3σ rounded up.
/// No radius, a negative one, NaN, or one whose square underflows to 0 give 0: the identity.
pub(crate) fn half_width(radius_px: f64) -> usize {
    if radius_px.is_nan() || radius_px <= 0.0 || radius_px * radius_px <= 0.0 {
        return 0;
    }
    let taps = (1.5 * radius_px).ceil();
    if taps >= MAX_HALF_WIDTH as f64 {
        MAX_HALF_WIDTH
    } else {
        taps as usize
    }
}

/// Below this σ the kernel is the exact-variance three-tap one.
const SMALL_SIGMA: f64 = 0.6;

/// The `2k + 1` weights of the Gaussian of σ = `radius_px` / 2, cut at `k` and summing to 1.
fn kernel(radius_px: f64, k: usize) -> Vec<f32> {
    let sigma = radius_px / 2.0;
    if sigma < SMALL_SIGMA {
        // Integer taps of a sampled Gaussian lose variance here. Three taps with the exact
        // variance σ² keep the response at σ; the outer taps are 0, so `half_width` stands.
        let side = (sigma * sigma / 2.0) as f32;
        let mut ker = vec![0.0f32; 2 * k + 1];
        ker[k - 1] = side;
        ker[k + 1] = side;
        ker[k] = (1.0 - sigma * sigma) as f32;
        return ker;
    }
    let raw: Vec<f64> = (0..=2 * k)
        .map(|t| {
            let d = t as f64 - k as f64;
            (-(d * d) / (2.0 * sigma * sigma)).exp()
        })
        .collect();
    let total: f64 = raw.iter().sum();
    raw.iter().map(|w| (w / total) as f32).collect()
}

/// Blurs `src` (`w` × `h`, zero outside) and returns the plane whose pixel `(i, j)` is the
/// blur at `(i + ox, j + oy)` of `src`. `radius_px` is the CSS radius, already scaled.
///
/// Two passes of tap-outer loops over whole rows, so no allocation per pixel. The additions
/// of a pixel always run in tap order, so a crop of the input gives the same bits.
pub(crate) fn blur_shifted(
    src: &[f32],
    w: usize,
    h: usize,
    ox: i64,
    oy: i64,
    radius_px: f64,
) -> Vec<f32> {
    let mut out = vec![0.0f32; w * h];
    if w == 0 || h == 0 {
        return out;
    }
    let k = half_width(radius_px);
    let (wi, hi) = (w as i64, h as i64);
    if k == 0 {
        // The identity, read at the shifted origin.
        for j in 0..hi {
            let y = j + oy;
            if !(0..hi).contains(&y) {
                continue;
            }
            let lo = (-ox).clamp(0, wi);
            let hi_x = (wi - ox).clamp(0, wi);
            if lo < hi_x {
                let s = &src[y as usize * w..][(lo + ox) as usize..(hi_x + ox) as usize];
                out[j as usize * w..][lo as usize..hi_x as usize].copy_from_slice(s);
            }
        }
        return out;
    }
    let ker = kernel(radius_px, k);
    // Horizontal pass: rows[y][i] = sum_d ker[d] * src[y][i + ox + d].
    let mut rows = vec![0.0f32; w * h];
    for y in 0..h {
        let line = &src[y * w..(y + 1) * w];
        let acc = &mut rows[y * w..(y + 1) * w];
        for (t, &weight) in ker.iter().enumerate() {
            let shift = ox + t as i64 - k as i64;
            let lo = (-shift).clamp(0, wi);
            let hi_x = (wi - shift).clamp(0, wi);
            if lo >= hi_x {
                continue;
            }
            let taps = &line[(lo + shift) as usize..(hi_x + shift) as usize];
            for (a, &v) in acc[lo as usize..hi_x as usize].iter_mut().zip(taps) {
                *a += weight * v;
            }
        }
    }
    // Vertical pass: out[j] = sum_d ker[d] * rows[j + oy + d].
    for j in 0..hi {
        let acc = &mut out[j as usize * w..(j as usize + 1) * w];
        for (t, &weight) in ker.iter().enumerate() {
            let y = j + oy + t as i64 - k as i64;
            if !(0..hi).contains(&y) {
                continue;
            }
            let line = &rows[y as usize * w..(y as usize + 1) * w];
            for (a, &v) in acc.iter_mut().zip(line) {
                *a += weight * v;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(v: &[f32]) -> Vec<u32> {
        v.iter().map(|x| x.to_bits()).collect()
    }

    /// A deterministic plane with every kind of value: zeros, ones, fractions.
    fn plane(w: usize, h: usize, seed: u32) -> Vec<f32> {
        (0..w * h)
            .map(|i| {
                let x = (i as u32).wrapping_mul(2_654_435_761).wrapping_add(seed);
                match x >> 29 {
                    0 => 0.0,
                    1 => 1.0,
                    n => f32::from((x >> 8) as u8) / 255.0 * (n as f32) / 7.0,
                }
            })
            .collect()
    }

    /// The Gaussian of σ = radius / 2 in `f64`, cut at `ceil(3σ)` and normalised, written
    /// from the spec with its own structure (ARQ-120, P11).
    fn reference(src: &[f32], w: usize, h: usize, radius: f64) -> Vec<f64> {
        let sigma = radius / 2.0;
        let k = (3.0 * sigma).ceil() as i64;
        let g: Vec<f64> = (-k..=k)
            .map(|d| (-((d * d) as f64) / (2.0 * sigma * sigma)).exp())
            .collect();
        let total: f64 = g.iter().sum();
        let at = |x: i64, y: i64, v: &[f64]| -> f64 {
            if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
                0.0
            } else {
                v[y as usize * w + x as usize]
            }
        };
        let input: Vec<f64> = src.iter().map(|&v| f64::from(v)).collect();
        let mut rows = vec![0.0; w * h];
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                rows[y as usize * w + x as usize] = (-k..=k)
                    .map(|d| g[(d + k) as usize] / total * at(x + d, y, &input))
                    .sum();
            }
        }
        let mut out = vec![0.0; w * h];
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                out[y as usize * w + x as usize] = (-k..=k)
                    .map(|d| g[(d + k) as usize] / total * at(x, y + d, &rows))
                    .sum();
            }
        }
        out
    }

    /// σ of the response of a unit impulse at the centre, along x and along y.
    fn measured_sigma(size: usize, radius: f64) -> (f64, f64) {
        let mut p = vec![0.0f32; size * size];
        let c = size / 2;
        p[c * size + c] = 1.0;
        let out = blur_shifted(&p, size, size, 0, 0, radius);
        let (mut mass, mut vx, mut vy) = (0.0, 0.0, 0.0);
        for y in 0..size {
            for x in 0..size {
                let v = f64::from(out[y * size + x]);
                mass += v;
                vx += v * (x as f64 - c as f64).powi(2);
                vy += v * (y as f64 - c as f64).powi(2);
            }
        }
        ((vx / mass).sqrt(), (vy / mass).sqrt())
    }

    // ARQ-120: an impulse gives a Gaussian response with sigma = radius / 2, measured within 2 %.
    #[test]
    fn arq_120_impulse_response_has_sigma_of_half_the_radius() {
        for (size, radius) in [(61, 10.0), (101, 16.0), (201, 40.0), (121, 7.0)] {
            let (sx, sy) = measured_sigma(size, radius);
            let want = radius / 2.0;
            for s in [sx, sy] {
                assert!(
                    (s - want).abs() <= 0.02 * want,
                    "radius {radius}: sigma {s}, want {want}"
                );
            }
        }
    }

    // ARQ-120: the blur equals the truncated, normalised Gaussian, outside is transparent black.
    #[test]
    fn arq_120_matches_the_reference_gaussian_with_transparent_outside() {
        for (w, h, radius) in [(40, 30, 10.0), (17, 9, 6.0), (25, 25, 2.5)] {
            let src = plane(w, h, 7);
            let got = blur_shifted(&src, w, h, 0, 0, radius);
            let want = reference(&src, w, h, radius);
            for (i, (g, r)) in got.iter().zip(&want).enumerate() {
                assert!(
                    (f64::from(*g) - r).abs() <= 1e-5,
                    "{w}x{h} r{radius} px {i}: {g} vs {r}"
                );
            }
        }
    }

    // ARQ-120: a constant opaque image keeps its value beyond 3 sigma of the edge, and
    // its edge falls off to transparent black, not to the edge colour.
    #[test]
    fn arq_120_constant_image_keeps_its_value_inside_and_fades_at_the_edge() {
        let (w, h) = (80usize, 80usize);
        let src = vec![1.0f32; w * h];
        let out = blur_shifted(&src, w, h, 0, 0, 10.0); // sigma 5, 3 sigma 15
        for y in 15..=h - 16 {
            for x in 15..=w - 16 {
                assert!((out[y * w + x] - 1.0).abs() <= 1.0 / 255.0, "({x},{y})");
            }
        }
        let edge = out[40 * w];
        assert!((0.5..0.6).contains(&edge), "edge {edge}");
        let corner = out[0];
        assert!((0.25..0.36).contains(&corner), "corner {corner}");
    }

    // ARQ-120: radius 0 is the identity, bit for bit; a radius too small to matter is the
    // identity to within the variance it keeps (σ² of 2.5e-25).
    #[test]
    fn arq_120_radius_zero_is_the_identity() {
        let (w, h) = (13, 7);
        let src = plane(w, h, 3);
        assert_eq!(bits(&blur_shifted(&src, w, h, 0, 0, 0.0)), bits(&src));
        assert_eq!(bits(&blur_shifted(&src, w, h, 0, 0, -3.0)), bits(&src));
        let tiny = blur_shifted(&src, w, h, 0, 0, 1e-12);
        assert!(tiny.iter().zip(&src).all(|(a, b)| (a - b).abs() <= 1e-6));
    }

    // ARQ-120, FD-35: output (i, j) reads the blur at (i + ox, j + oy), also beyond the image,
    // where the response still reaches. Bit-identical to blurring a zero-padded plane.
    #[test]
    fn arq_120_shifted_origin_equals_a_padded_plane() {
        let (w, h, m) = (11usize, 8usize, 40usize);
        let src = plane(w, h, 11);
        for radius in [0.0, 6.0] {
            for (ox, oy) in [
                (0i64, 0i64),
                (-3, 2),
                (4, -5),
                (-9, 9),
                (30, -30),
                (-200, 5),
            ] {
                let got = blur_shifted(&src, w, h, ox, oy, radius);
                let (bw, bh) = (w + 2 * m, h + 2 * m);
                let mut big = vec![0.0f32; bw * bh];
                for y in 0..h {
                    big[(y + m) * bw + m..(y + m) * bw + m + w]
                        .copy_from_slice(&src[y * w..(y + 1) * w]);
                }
                let full = blur_shifted(&big, bw, bh, 0, 0, radius);
                for j in 0..h as i64 {
                    for i in 0..w as i64 {
                        let (x, y) = (i + ox + m as i64, j + oy + m as i64);
                        let want = if x < 0 || y < 0 || x >= bw as i64 || y >= bh as i64 {
                            0.0
                        } else {
                            full[y as usize * bw + x as usize]
                        };
                        assert_eq!(
                            got[j as usize * w + i as usize].to_bits(),
                            want.to_bits(),
                            "r{radius} o({ox},{oy}) px({i},{j})"
                        );
                    }
                }
            }
        }
    }

    // ARQ-120: a tile computed from its region of interest equals the whole image, bit for bit.
    #[test]
    fn arq_120_a_tile_with_its_roi_equals_the_whole_image() {
        let (w, h) = (70usize, 50usize);
        let src = plane(w, h, 5);
        for radius in [0.0, 5.0, 9.0] {
            let whole = blur_shifted(&src, w, h, 0, 0, radius);
            let k = half_width(radius);
            // Tile [20, 45) x [15, 35); its roi grows k on each side and is cut to the image.
            let (x0, x1, y0, y1) = (20usize, 45usize, 15usize, 35usize);
            let (rx0, rx1) = (x0.saturating_sub(k), (x1 + k).min(w));
            let (ry0, ry1) = (y0.saturating_sub(k), (y1 + k).min(h));
            let (cw, ch) = (rx1 - rx0, ry1 - ry0);
            let mut crop = Vec::new();
            for y in ry0..ry1 {
                crop.extend_from_slice(&src[y * w + rx0..y * w + rx1]);
            }
            let tile = blur_shifted(&crop, cw, ch, 0, 0, radius);
            for y in y0..y1 {
                for x in x0..x1 {
                    assert_eq!(
                        tile[(y - ry0) * cw + (x - rx0)].to_bits(),
                        whole[y * w + x].to_bits(),
                        "r{radius} ({x},{y})"
                    );
                }
            }
        }
    }

    // ARQ-120: the half width is 3 sigma rounded up, and has a ceiling.
    #[test]
    fn arq_120_half_width_is_three_sigma_rounded_up() {
        for (radius, want) in [
            (0.0, 0),
            (-1.0, 0),
            (f64::NAN, 0),
            (10.0, 15),
            (10.1, 16),
            (1.0, 2),
            (100.0, 150),
            (f64::MAX, MAX_HALF_WIDTH),
            (f64::INFINITY, MAX_HALF_WIDTH),
        ] {
            assert_eq!(half_width(radius), want, "radius {radius}");
        }
    }

    // ARQ-120: sigma holds within 2 % also when radius x scale is small, below sigma 0.6
    // where a sampled Gaussian loses variance.
    #[test]
    fn arq_120_sigma_holds_for_small_radius_times_scale() {
        for radius in [0.25, 0.5, 0.75, 1.0, 1.16, 1.2, 1.25, 1.5] {
            let (sx, sy) = measured_sigma(21, radius);
            let want = radius / 2.0;
            for s in [sx, sy] {
                assert!(
                    (s - want).abs() <= 0.02 * want,
                    "radius {radius}: sigma {s}, want {want}"
                );
            }
        }
    }

    // ARQ-110, ARQ-120: a radius whose square underflows to 0 is the identity, never NaN.
    #[test]
    fn arq_110_a_radius_whose_square_underflows_is_the_identity() {
        assert_eq!(half_width(1e-170), 0);
        assert_eq!(half_width(5e-324), 0);
        let (w, h) = (9, 5);
        let src = plane(w, h, 2);
        for radius in [1e-170, 5e-324] {
            assert_eq!(bits(&blur_shifted(&src, w, h, 0, 0, radius)), bits(&src));
        }
    }

    // ARQ-120: the cap is 1.5 x blur 100 x the maximum zoom 32 (FD-08), and sigma holds there.
    #[test]
    fn arq_120_the_cap_covers_blur_100_at_zoom_32() {
        assert_eq!(half_width(f64::MAX), 4800);
        assert_eq!(half_width(3200.0), 4800);
        // One row: the vertical pass reads only the centre tap, so sigma is along x.
        let w = 2 * 4800 + 1;
        let mut p = vec![0.0f32; w];
        p[4800] = 1.0;
        let out = blur_shifted(&p, w, 1, 0, 0, 3200.0);
        let mass: f64 = out.iter().map(|&v| f64::from(v)).sum();
        let var: f64 = out
            .iter()
            .enumerate()
            .map(|(x, &v)| f64::from(v) * (x as f64 - 4800.0).powi(2))
            .sum::<f64>()
            / mass;
        let sigma = var.sqrt();
        assert!((sigma - 1600.0).abs() <= 32.0, "sigma {sigma}");
    }
}
