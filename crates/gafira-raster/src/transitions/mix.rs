//! The one place where pixels are mixed (P15 item 8, item 9).

use crate::transfer::transfer;

/// The space in which weighted pixels are mixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MixSpace {
    /// O5 (a): `Rgba8` is decoded to linear light, mixed and encoded; `RgbaF32` is mixed as stored.
    Linear,
    /// O5 (b): `RgbaF32` is encoded to sRGB, mixed and decoded; `Rgba8` is mixed as stored.
    Encoded,
    /// O5 (c): both formats are mixed as stored.
    AsStored,
}

impl MixSpace {
    /// Every answer to O5.
    pub(crate) const ALL: [MixSpace; 3] = [MixSpace::Linear, MixSpace::Encoded, MixSpace::AsStored];
}

/// Provisional until the owner answers O5; the build follows (a).
pub(crate) const MIX_SPACE: MixSpace = MixSpace::Linear;

/// Weighted sum of premultiplied `Rgba8` pixels. The weights sum to 1.
pub(crate) fn weighted_u8(samples: &[([u8; 4], f64)]) -> [u8; 4] {
    debug_assert!(MixSpace::ALL.contains(&MIX_SPACE));
    weighted_u8_in(MIX_SPACE, samples)
}

/// Weighted sum of premultiplied `RgbaF32` pixels. The weights sum to 1.
pub(crate) fn weighted_f32(samples: &[([f32; 4], f64)]) -> [f32; 4] {
    weighted_f32_in(MIX_SPACE, samples)
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

pub(crate) fn weighted_u8_in(space: MixSpace, samples: &[([u8; 4], f64)]) -> [u8; 4] {
    if let Some((px, _)) = samples.iter().find(|(_, w)| *w == 1.0) {
        return *px;
    }
    match space {
        MixSpace::Encoded | MixSpace::AsStored => {
            let mut sum = [0.0f64; 4];
            for (px, w) in samples {
                let px = clean_u8(*px);
                for k in 0..4 {
                    sum[k] += w * f64::from(px[k]);
                }
            }
            sum.map(to_byte)
        }
        MixSpace::Linear => {
            let t = transfer();
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
    }
}

pub(crate) fn weighted_f32_in(space: MixSpace, samples: &[([f32; 4], f64)]) -> [f32; 4] {
    if let Some((px, _)) = samples.iter().find(|(_, w)| *w == 1.0) {
        return *px;
    }
    let mut sum = [0.0f64; 4];
    match space {
        MixSpace::Linear | MixSpace::AsStored => {
            for (px, w) in samples {
                let px = clean_f32(*px);
                for k in 0..4 {
                    sum[k] += w * f64::from(px[k]);
                }
            }
        }
        MixSpace::Encoded => {
            let t = transfer();
            for (px, w) in samples {
                let px = clean_f32(*px);
                let a = px[3];
                if a == 0.0 {
                    continue;
                }
                // Encode the straight colour, premultiply again, mix.
                let enc = t.encode_f32([px[0] / a, px[1] / a, px[2] / a]);
                for k in 0..3 {
                    sum[k] += w * f64::from(enc[k]) * f64::from(a);
                }
                sum[3] += w * f64::from(a);
            }
            let alpha = sum[3];
            if alpha != 0.0 {
                let straight = [0, 1, 2].map(|k| (sum[k] / alpha) as f32);
                let lin = t.decode_f32(straight);
                for k in 0..3 {
                    sum[k] = f64::from(lin[k]) * alpha;
                }
            }
        }
    }
    sum.map(|v| v as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPACES: [MixSpace; 3] = MixSpace::ALL;

    fn bits(p: [f32; 4]) -> [u32; 4] {
        p.map(f32::to_bits)
    }

    // The constant: the build follows O5 (a).
    #[test]
    fn mix_space_is_linear_until_the_owner_answers() {
        assert_eq!(MIX_SPACE, MixSpace::Linear);
        let s = [([0u8, 0, 0, 255], 0.5), ([255, 255, 255, 255], 0.5)];
        assert_eq!(weighted_u8(&s), weighted_u8_in(MixSpace::Linear, &s));
        let s = [([0.0f32, 0.0, 0.0, 1.0], 0.5), ([1.0, 1.0, 1.0, 1.0], 0.5)];
        assert_eq!(
            bits(weighted_f32(&s)),
            bits(weighted_f32_in(MixSpace::Linear, &s))
        );
    }

    // P15 item 8: a sample of weight exactly 1 comes back unconverted in every arm.
    #[test]
    fn a_sample_of_weight_one_is_returned_as_it_is() {
        for space in SPACES {
            for px in [[200u8, 10, 5, 0], [1, 2, 3, 255], [255, 255, 255, 1]] {
                assert_eq!(weighted_u8_in(space, &[([9, 9, 9, 9], 0.0), (px, 1.0)]), px);
                assert_eq!(weighted_u8_in(space, &[(px, 1.0), ([9, 9, 9, 9], 0.0)]), px);
            }
            for px in [
                [-0.0f32, 1.5, 1e-30, 0.0],
                [2.0, -1.0, 0.3, -0.25],
                [0.7, 0.1, 0.1, 1.0],
            ] {
                let got = weighted_f32_in(space, &[(px, 1.0), ([0.5; 4], 0.0)]);
                assert_eq!(bits(got), bits(px), "{space:?}");
            }
        }
    }

    // O5 table, beside the constant: opaque black and white at the midpoint, per space and format.
    // Only these rows change when the owner answers O5.
    #[test]
    fn midpoint_rows_per_space_and_format() {
        let u8_rows: [(MixSpace, u8); 3] = [
            (MixSpace::Linear, 188),
            (MixSpace::Encoded, 128),
            (MixSpace::AsStored, 128),
        ];
        for (space, want) in u8_rows {
            let got = weighted_u8_in(space, &[([0, 0, 0, 255], 0.5), ([255, 255, 255, 255], 0.5)]);
            assert_eq!(got, [want, want, want, 255], "{space:?}");
        }
        let f32_rows: [(MixSpace, f32); 3] = [
            (MixSpace::Linear, 0.5),
            (MixSpace::Encoded, 0.214_041),
            (MixSpace::AsStored, 0.5),
        ];
        for (space, want) in f32_rows {
            let got = weighted_f32_in(
                space,
                &[([0.0, 0.0, 0.0, 1.0], 0.5), ([1.0, 1.0, 1.0, 1.0], 0.5)],
            );
            for c in &got[..3] {
                assert!((c - want).abs() < 1e-5, "{space:?}: {got:?}");
            }
            assert_eq!(got[3], 1.0);
        }
    }

    // P15 item 9: rounding of 8 bits is (v + 0.5).floor(), clamped, in the arms that mix as stored.
    #[test]
    fn rgba8_rounds_half_up_and_clamps() {
        for space in [MixSpace::Encoded, MixSpace::AsStored] {
            let h = |a: [u8; 4], b: [u8; 4]| weighted_u8_in(space, &[(a, 0.5), (b, 0.5)]);
            assert_eq!(h([1, 1, 1, 3], [2, 2, 2, 4]), [2, 2, 2, 4], "{space:?}");
            assert_eq!(h([0, 0, 0, 0], [1, 1, 1, 1]), [1, 1, 1, 1]);
            assert_eq!(h([1, 1, 1, 1], [1, 1, 1, 2]), [1, 1, 1, 2]);
            // 0.4999 stays down; weights that overshoot 1 clamp at 255.
            assert_eq!(
                weighted_u8_in(space, &[([1, 1, 1, 1], 0.4999), ([0, 0, 0, 0], 0.5001)]),
                [0, 0, 0, 0]
            );
            assert_eq!(
                weighted_u8_in(
                    space,
                    &[([255, 255, 255, 255], 0.6), ([255, 255, 255, 255], 0.6)]
                ),
                [255; 4]
            );
        }
        // Linear: alpha rounds the same way; white over nothing at one half is 128.
        let got = weighted_u8_in(
            MixSpace::Linear,
            &[([0, 0, 0, 0], 0.5), ([255, 255, 255, 255], 0.5)],
        );
        assert_eq!(got, [128, 128, 128, 128]);
        assert_eq!(
            weighted_u8_in(
                MixSpace::Linear,
                &[([0, 0, 0, 0], 0.5), ([0, 0, 0, 0], 0.5)]
            ),
            [0; 4]
        );
    }

    // ARQ-117: a pixel with a = 0 adds no colour, whatever bytes it holds, in every arm.
    #[test]
    fn a_transparent_pixel_adds_no_colour() {
        for space in SPACES {
            let got = weighted_u8_in(space, &[([200, 10, 5, 0], 0.5), ([0, 0, 0, 255], 0.5)]);
            assert_eq!(got, [0, 0, 0, 128], "{space:?}");
            let got = weighted_f32_in(
                space,
                &[([9.0, 9.0, 9.0, 0.0], 0.5), ([0.0, 0.0, 0.0, 1.0], 0.5)],
            );
            assert_eq!(bits(got), bits([0.0, 0.0, 0.0, 0.5]), "{space:?}");
        }
    }

    // A constant image stays constant under any weights.
    #[test]
    fn equal_samples_give_that_sample() {
        for space in SPACES {
            for w in [0.1f64, 0.25, 0.7, 0.999] {
                for px in [
                    [100u8, 50, 25, 128],
                    [0, 0, 0, 255],
                    [255, 255, 255, 255],
                    [7, 9, 3, 10],
                ] {
                    assert_eq!(
                        weighted_u8_in(space, &[(px, 1.0 - w), (px, w)]),
                        px,
                        "{space:?} {w}"
                    );
                }
                let px = [0.2f32, 0.1, 0.05, 0.4];
                let got = weighted_f32_in(space, &[(px, 1.0 - w), (px, w)]);
                for k in 0..4 {
                    assert!((got[k] - px[k]).abs() < 1e-6, "{space:?} {got:?}");
                }
            }
        }
    }

    // ARQ-117, ARQ-110: premultiplied input gives c <= a, in every arm; f32 output stays between
    // the samples and finite.
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
                for space in SPACES {
                    let out = weighted_u8_in(space, &[(px(c1, a1), 1.0 - w), (px(c2, a2), w)]);
                    prop_assert!(out[..3].iter().all(|c| *c <= out[3]), "{space:?} {out:?}");
                }
            }

            #[test]
            fn f32_result_is_finite_and_between_the_samples(
                a in prop::array::uniform4(-1.0e30f32..1.0e30),
                b in prop::array::uniform4(-1.0e30f32..1.0e30),
                w in 0.0f64..=1.0,
            ) {
                for space in [MixSpace::Linear, MixSpace::AsStored] {
                    let out = weighted_f32_in(space, &[(a, 1.0 - w), (b, w)]);
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
}
