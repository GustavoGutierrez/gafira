//! The one place where pixels are mixed (P15 item 8, item 9).

use crate::transfer::Transfer;

/// Weighted sum of premultiplied `Rgba8` pixels, mixed in linear light (D-48). The weights sum
/// to 1.
pub(crate) fn weighted_u8(t: &Transfer, samples: &[([u8; 4], f64)]) -> [u8; 4] {
    if let Some((px, _)) = samples.iter().find(|(_, w)| *w == 1.0) {
        return *px;
    }
    // Linear premultiplied colour and alpha, both in [0, 1].
    let mut colour = [0.0f64; 3];
    let mut alpha = 0.0f64;
    for (px, w) in samples {
        let px = clean_u8(*px);
        if px[3] == 0 {
            continue;
        }
        let a = f64::from(px[3]) / 255.0;
        alpha += w * a;
        for k in 0..3 {
            let straight = crate::unpremultiply_u8(px[k], px[3]);
            colour[k] += w * t.decode[usize::from(straight)] * a;
        }
    }
    if alpha <= 0.0 {
        return [0; 4];
    }
    let encoded = t.encode_unit(colour.map(|c| c / alpha));
    let mut out = [0u8; 4];
    for k in 0..3 {
        out[k] = to_byte(255.0 * alpha * encoded[k]);
    }
    out[3] = to_byte(255.0 * alpha);
    out
}

/// Weighted sum of premultiplied `RgbaF32` pixels, mixed as stored (D-48). The weights sum to 1.
pub(crate) fn weighted_f32(_t: &Transfer, samples: &[([f32; 4], f64)]) -> [f32; 4] {
    if let Some((px, _)) = samples.iter().find(|(_, w)| *w == 1.0) {
        return *px;
    }
    let mut sum = [0.0f64; 4];
    for (px, w) in samples {
        let px = clean_f32(*px);
        for k in 0..4 {
            sum[k] += w * f64::from(px[k]);
        }
    }
    // A convex sum cannot leave the `f32` range, but rounding in `f64` may touch its edge.
    sum.map(|v| v.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32)
}

/// A pixel with `a = 0` adds no colour, whatever bytes it holds.
fn clean_u8(p: [u8; 4]) -> [u8; 4] {
    if p[3] == 0 { [0; 4] } else { p }
}

fn clean_f32(p: [f32; 4]) -> [f32; 4] {
    if p[3] == 0.0 { [0.0; 4] } else { p }
}

/// Rounds to nearest, halves up, clamped to a byte (the rule of `blend_u8`).
fn to_byte(v: f64) -> u8 {
    (v + 0.5).floor().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(p: [f32; 4]) -> [u32; 4] {
        p.map(f32::to_bits)
    }

    // P15 item 8: a sample of weight exactly 1 comes back unconverted.
    #[test]
    fn a_sample_of_weight_one_is_returned_as_it_is() {
        for px in [[200u8, 10, 5, 0], [1, 2, 3, 255], [255, 255, 255, 1]] {
            assert_eq!(
                weighted_u8(&Transfer::new(), &[([9, 9, 9, 9], 0.0), (px, 1.0)]),
                px
            );
            assert_eq!(
                weighted_u8(&Transfer::new(), &[(px, 1.0), ([9, 9, 9, 9], 0.0)]),
                px
            );
        }
        for px in [
            [-0.0f32, 1.5, 1e-30, 0.0],
            [2.0, -1.0, 0.3, -0.25],
            [0.7, 0.1, 0.1, 1.0],
        ] {
            let got = weighted_f32(&Transfer::new(), &[(px, 1.0), ([0.5; 4], 0.0)]);
            assert_eq!(bits(got), bits(px));
        }
    }

    // D-48 (O5), ARQ-118 table: opaque black and white at the midpoint. `Rgba8` mixes in linear
    // light; `RgbaF32` mixes as stored. These rows are the guard of the decision.
    #[test]
    fn midpoint_rows_per_format() {
        let got = weighted_u8(
            &Transfer::new(),
            &[([0, 0, 0, 255], 0.5), ([255, 255, 255, 255], 0.5)],
        );
        assert_eq!(got, [188, 188, 188, 255]);
        let got = weighted_f32(
            &Transfer::new(),
            &[([0.0, 0.0, 0.0, 1.0], 0.5), ([1.0, 1.0, 1.0, 1.0], 0.5)],
        );
        for c in &got[..3] {
            assert!((c - 0.5).abs() < 1e-5, "{got:?}");
        }
        assert_eq!(got[3], 1.0);
    }

    // P15 item 9: alpha rounds to nearest, halves up; white over nothing at one half is 128.
    #[test]
    fn rgba8_alpha_rounds_half_up() {
        let got = weighted_u8(
            &Transfer::new(),
            &[([0, 0, 0, 0], 0.5), ([255, 255, 255, 255], 0.5)],
        );
        assert_eq!(got, [128, 128, 128, 128]);
        assert_eq!(
            weighted_u8(
                &Transfer::new(),
                &[([0, 0, 0, 0], 0.5), ([0, 0, 0, 0], 0.5)]
            ),
            [0; 4]
        );
    }

    // ARQ-117: a pixel with a = 0 adds no colour, whatever bytes it holds.
    #[test]
    fn a_transparent_pixel_adds_no_colour() {
        let got = weighted_u8(
            &Transfer::new(),
            &[([200, 10, 5, 0], 0.5), ([0, 0, 0, 255], 0.5)],
        );
        assert_eq!(got, [0, 0, 0, 128]);
        let got = weighted_f32(
            &Transfer::new(),
            &[([9.0, 9.0, 9.0, 0.0], 0.5), ([0.0, 0.0, 0.0, 1.0], 0.5)],
        );
        assert_eq!(bits(got), bits([0.0, 0.0, 0.0, 0.5]));
    }

    // A constant image stays constant under any weights.
    #[test]
    fn equal_samples_give_that_sample() {
        for w in [0.1f64, 0.25, 0.7, 0.999] {
            for px in [
                [100u8, 50, 25, 128],
                [0, 0, 0, 255],
                [255, 255, 255, 255],
                [7, 9, 3, 10],
            ] {
                assert_eq!(
                    weighted_u8(&Transfer::new(), &[(px, 1.0 - w), (px, w)]),
                    px,
                    "{w}"
                );
            }
            let px = [0.2f32, 0.1, 0.05, 0.4];
            let got = weighted_f32(&Transfer::new(), &[(px, 1.0 - w), (px, w)]);
            for k in 0..4 {
                assert!((got[k] - px[k]).abs() < 1e-6, "{got:?}");
            }
        }
    }

    // 02 §4.17.4: channels below 0 or above 1 are admitted; mixing a pixel with itself keeps it.
    #[test]
    fn out_of_range_colours_keep_their_sign() {
        let px = [-0.25f32, 1.5, -2.0, 1.0];
        for w in [0.3f64, 0.5, 0.9] {
            let got = weighted_f32(&Transfer::new(), &[(px, 1.0 - w), (px, w)]);
            for k in 0..4 {
                assert!((got[k] - px[k]).abs() < 1e-5, "w={w}: {got:?}");
            }
        }
    }

    // ARQ-110: finite pixels give finite output, also an out-of-range colour under a tiny alpha.
    #[test]
    fn f32_stays_finite_on_finite_pixels() {
        let rows: [([f32; 4], [f32; 4]); 4] = [
            ([1.0e30, 0.0, 0.0, 1.0e-40], [1.5e38, 0.0, 0.0, 1.0e-3]),
            (
                [f32::MAX, -f32::MAX, f32::MAX, 1.0e-45],
                [f32::MAX, f32::MAX, -f32::MAX, 1.0],
            ),
            (
                [1.0e30, 1.0e30, 1.0e30, 1.0e-30],
                [-1.0e30, 0.0, 1.0e30, 2.0e-30],
            ),
            ([0.5, 0.5, 0.5, -1.0e-30], [0.5, 0.5, 0.5, 1.0e-30]),
        ];
        for (a, b) in rows {
            for w in [0.5f64, 0.25, 1.0e-9, 0.999_999] {
                let out = weighted_f32(&Transfer::new(), &[(a, 1.0 - w), (b, w)]);
                assert!(
                    out.iter().all(|v| v.is_finite()),
                    "{a:?} {b:?} w={w}: {out:?}"
                );
            }
        }
    }

    // ARQ-117, ARQ-110: premultiplied input gives c <= a; f32 output stays between the samples
    // and finite.
    mod property {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn rgba8_result_is_premultiplied_valid(
                c1 in prop::array::uniform3(any::<u8>()), a1 in any::<u8>(),
                c2 in prop::array::uniform3(any::<u8>()), a2 in any::<u8>(),
                w in 0.0f64..=1.0,
            ) {
                let px = |c: [u8; 3], a: u8| [
                    crate::premultiply_u8(c[0], a), crate::premultiply_u8(c[1], a),
                    crate::premultiply_u8(c[2], a), a,
                ];
                let out = weighted_u8(&Transfer::new(), &[(px(c1, a1), 1.0 - w), (px(c2, a2), w)]);
                prop_assert!(out[..3].iter().all(|c| *c <= out[3]), "{out:?}");
            }

            #[test]
            fn f32_result_is_finite(
                a in prop::array::uniform4(prop_oneof![-1.0e38f32..1.0e38, -1.0e-30f32..1.0e-30, -2.0f32..2.0]),
                b in prop::array::uniform4(prop_oneof![-1.0e38f32..1.0e38, -1.0e-30f32..1.0e-30, -2.0f32..2.0]),
                w in 0.0f64..=1.0,
            ) {
                let out = weighted_f32(&Transfer::new(), &[(a, 1.0 - w), (b, w)]);
                prop_assert!(out.iter().all(|v| v.is_finite()), "{out:?}");
            }

            #[test]
            fn f32_result_is_finite_and_between_the_samples(
                a in prop::array::uniform4(-1.0e30f32..1.0e30),
                b in prop::array::uniform4(-1.0e30f32..1.0e30),
                w in 0.0f64..=1.0,
            ) {
                let out = weighted_f32(&Transfer::new(), &[(a, 1.0 - w), (b, w)]);
                for k in 0..4 {
                    let ak = if a[3] == 0.0 && k < 3 { 0.0 } else { a[k] };
                    let bk = if b[3] == 0.0 && k < 3 { 0.0 } else { b[k] };
                    prop_assert!(out[k].is_finite());
                    prop_assert!(out[k] >= ak.min(bk) - 1e-6 * ak.abs().max(bk.abs()) && out[k] <= ak.max(bk) + 1e-6 * ak.abs().max(bk.abs()), "{out:?}");
                }
            }
        }
    }
}
