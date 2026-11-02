//! Balance de blancos: Temperatura and Matiz, and the eyedropper (FD-33).

use gafira_doc::contract::{Describe, Descriptor, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_geom::IRect;

use super::pointwise::{check_io, check_scale, map_straight, slider};
use super::{Cost, Effect};
use crate::{BufferError, EffectCx, EffectError, ImageView, ImageViewMut};

/// Temperatura and Matiz as sliders, in `[-100, 100]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemperatureTint {
    /// Temperatura.
    pub temperature: f64,
    /// Matiz.
    pub tint: f64,
}

/// The sliders that make the grey `gray` (linear light) neutral.
///
/// # Errors
///
/// [`BufferError::NonFinite`] when a channel is NaN or infinite.
pub fn eyedropper(gray: [f32; 3]) -> Result<TemperatureTint, BufferError> {
    if let Some(index) = gray.iter().position(|v| !v.is_finite()) {
        return Err(BufferError::NonFinite {
            field: "gray",
            index,
        });
    }
    let [r, g, b] = gray.map(f64::from);
    // A channel at 0 (or below) has no quotient: the adjustment does not change.
    if r <= 0.0 || g <= 0.0 || b <= 0.0 {
        return Ok(TemperatureTint {
            temperature: 0.0,
            tint: 0.0,
        });
    }
    Ok(TemperatureTint {
        temperature: (100.0 * (b / r).log2()).clamp(-100.0, 100.0),
        tint: (200.0 * (g / (r * b).sqrt()).log2()).clamp(-100.0, 100.0),
    })
}

/// White balance in linear light.
#[derive(Debug)]
pub struct WhiteBalance {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl WhiteBalance {
    /// Builds the effect.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "effect.adjust.white_balance",
                MessageKey("glossary.effect_white_balance"),
                1,
            ),
            schema: ParamSchema::new(vec![
                slider(
                    "temperature",
                    "effect.adjust.temperature",
                    -100.0,
                    100.0,
                    0.0,
                ),
                slider("tint", "effect.adjust.tint", -100.0, 100.0, 0.0),
            ]),
        }
    }
}

impl Default for WhiteBalance {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for WhiteBalance {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Effect for WhiteBalance {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn roi(&self, _cx: &EffectCx, _p: &ParamValues, out: IRect) -> IRect {
        out
    }

    fn cost_hint(&self, _cx: &EffectCx, _p: &ParamValues, px: u64) -> Cost {
        Cost { units: px }
    }

    fn run_cpu(
        &self,
        cx: &EffectCx,
        p: &ParamValues,
        src: ImageView<'_>,
        dst: ImageViewMut<'_>,
    ) -> Result<(), EffectError> {
        check_scale(cx)?;
        let values = self.schema.resolve(p)?;
        check_io(&src, &dst)?;
        let temperature = values.float("temperature").unwrap_or(0.0);
        let tint = values.float("tint").unwrap_or(0.0);
        let factors = [
            (temperature / 200.0).exp2(),
            (-tint / 200.0).exp2(),
            (-temperature / 200.0).exp2(),
        ];
        map_straight(src, dst, |_, c| {
            if temperature == 0.0 && tint == 0.0 {
                return None;
            }
            Some([c[0] * factors[0], c[1] * factors[1], c[2] * factors[2]])
        })
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::ParamKind;

    use super::super::testing::*;
    use super::*;

    fn wb() -> WhiteBalance {
        WhiteBalance::new()
    }

    // Passes from the start (data only); proved by mutants on the ranges and the id.
    #[test]
    fn fd_33_white_balance_schema_has_two_sliders_of_100() {
        let e = wb();
        assert_eq!(e.descriptor().id, "effect.adjust.white_balance");
        e.params().validate_definition().unwrap();
        let ids: Vec<_> = e.params().params().iter().map(|d| d.id).collect();
        assert_eq!(ids, ["temperature", "tint"]);
        for d in e.params().params() {
            assert!(d.animatable);
            assert_eq!(
                d.kind,
                ParamKind::Float {
                    min: -100.0,
                    max: 100.0,
                    default: 0.0
                }
            );
        }
        assert!(e.gpu().is_none());
    }

    // FD-33 (a)
    #[test]
    fn fd_33_a_white_balance_defaults_are_identity() {
        assert_defaults_are_identity(&wb());
    }

    // FD-33 (e): the factors on linear light, f32, with and without alpha.
    #[test]
    fn fd_33_e_temperature_and_tint_scale_the_linear_channels() {
        let s2 = 2f64.sqrt();
        let cases: [(f64, f64, [f64; 3]); 7] = [
            (100.0, 0.0, [s2, 1.0, 1.0 / s2]),
            (-100.0, 0.0, [1.0 / s2, 1.0, s2]),
            (0.0, 100.0, [1.0, 1.0 / s2, 1.0]),
            (0.0, -100.0, [1.0, s2, 1.0]),
            (50.0, 0.0, [2f64.powf(0.25), 1.0, 2f64.powf(-0.25)]),
            (0.0, 50.0, [1.0, 2f64.powf(-0.25), 1.0]),
            (100.0, -100.0, [s2, s2, 1.0 / s2]),
        ];
        for alpha in [1.0f32, 0.5] {
            let straight = [0.2f32, 0.3, 0.4];
            let src: Vec<f32> = straight.iter().map(|c| c * alpha).chain([alpha]).collect();
            for (t, m, f) in cases {
                let out = runf(
                    &wb(),
                    &cx(1.0),
                    &pv(&[("temperature", t), ("tint", m)]),
                    &src,
                    (1, 1),
                )
                .unwrap();
                for k in 0..3 {
                    let want = f64::from(straight[k]) * f[k] * f64::from(alpha);
                    assert!(
                        (f64::from(out[k]) - want).abs() < 1e-5,
                        "t {t} m {m} a {alpha} ch {k}: {} vs {want}",
                        out[k]
                    );
                }
                assert_eq!(out[3], alpha);
            }
        }
    }

    // FD-33 (e), 8 bits: decode, scale, encode, with the straight colour of a translucent pixel.
    #[test]
    fn fd_33_e_8_bit_pixels_go_through_linear_light() {
        let f = |t: f64, m: f64| {
            [
                2f64.powf(t / 200.0),
                2f64.powf(-m / 200.0),
                2f64.powf(-t / 200.0),
            ]
        };
        for (px, t, m) in [
            ([128u8, 100, 60, 255], 100.0, -40.0),
            ([128, 100, 60, 255], -70.0, 90.0),
            ([200, 10, 250, 255], 33.0, 0.0),
        ] {
            let out = run8(
                &wb(),
                &cx(1.0),
                &pv(&[("temperature", t), ("tint", m)]),
                &px,
                (1, 1),
            )
            .unwrap();
            let k = f(t, m);
            for c in 0..3 {
                assert_eq!(
                    out[c],
                    enc(lin(f64::from(px[c])) * k[c]),
                    "ch {c} t {t} m {m}"
                );
            }
            assert_eq!(out[3], 255);
        }
        // Premultiplied: (64, 50, 30) at alpha 128 has the straight colour (128, 100, 60).
        let out = run8(
            &wb(),
            &cx(1.0),
            &pv(&[("temperature", 60.0), ("tint", 20.0)]),
            &[64, 50, 30, 128],
            (1, 1),
        )
        .unwrap();
        let k = f(60.0, 20.0);
        for (c, straight) in [128.0, 100.0, 60.0].into_iter().enumerate() {
            let byte = u32::from(enc(lin(straight) * k[c]));
            assert_eq!(u32::from(out[c]), (byte * 128 + 127) / 255, "ch {c}");
        }
    }

    // FD-33 (g)
    #[test]
    fn fd_33_g_eyedropper_values_and_clipping() {
        let v = |g: [f32; 3]| eyedropper(g).unwrap();
        let near = |a: f64, b: f64| (a - b).abs() < 1e-5;
        let r = v([1.0, 1.0, 2.0]);
        assert!(near(r.temperature, 100.0) && near(r.tint, -100.0), "{r:?}");
        let r = v([2.0, 1.0, 1.0]);
        assert!(near(r.temperature, -100.0) && near(r.tint, -100.0), "{r:?}");
        let r = v([1.0, 2.0, 1.0]);
        assert!(near(r.temperature, 0.0) && near(r.tint, 100.0), "{r:?}");
        let r = v([0.4, 0.5, 0.45]);
        assert!(near(r.temperature, 100.0 * (0.45f64 / 0.4).log2()), "{r:?}");
        assert!(
            near(r.tint, 200.0 * (0.5f64 / (0.4f64 * 0.45).sqrt()).log2()),
            "{r:?}"
        );
        // Beyond the range: clipped to the ends.
        let r = v([1.0, 1.0, 64.0]);
        assert_eq!((r.temperature, r.tint), (100.0, -100.0));
        let r = v([64.0, 0.01, 1.0]);
        assert_eq!((r.temperature, r.tint), (-100.0, -100.0));
        let r = v([0.01, 64.0, 0.01]);
        assert_eq!((r.temperature, r.tint), (0.0, 100.0));
        let r = v([64.0, 1.0, 0.01]);
        assert_eq!(r.temperature, -100.0);
        assert!(near(r.tint, 200.0 * 1.25f64.log2()), "{r:?}");
    }

    // FD-33 (g), 02 §4.17.4 "canal 0 en un cociente": the adjustment does not change.
    #[test]
    fn fd_33_g_eyedropper_with_a_zero_or_negative_channel_changes_nothing() {
        for g in [
            [0.0f32, 0.5, 0.5],
            [0.5, 0.0, 0.5],
            [0.5, 0.5, 0.0],
            [0.0, 0.0, 0.0],
            [-0.0, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
            [0.5, 0.5, -0.5],
        ] {
            let r = eyedropper(g).unwrap();
            assert_eq!((r.temperature, r.tint), (0.0, 0.0), "{g:?}");
        }
    }

    #[test]
    fn arq_110_eyedropper_rejects_non_finite_channels() {
        for (i, bad) in [
            (0usize, f32::NAN),
            (1, f32::INFINITY),
            (2, f32::NEG_INFINITY),
        ] {
            let mut g = [0.3f32; 3];
            g[i] = bad;
            assert_eq!(
                eyedropper(g),
                Err(BufferError::NonFinite {
                    field: "gray",
                    index: i
                })
            );
        }
    }

    // FD-33 (g): after the eyedropper on a grey inside the range, R, G and B agree <= 1/255.
    #[test]
    fn fd_33_g_the_eyedropper_values_make_the_grey_neutral() {
        for grey in [
            [130u8, 120, 110],
            [100, 100, 100],
            [90, 110, 100],
            [60, 55, 70],
        ] {
            let g = [
                lin(f64::from(grey[0])),
                lin(f64::from(grey[1])),
                lin(f64::from(grey[2])),
            ];
            let tt = eyedropper([g[0] as f32, g[1] as f32, g[2] as f32]).unwrap();
            assert!(
                tt.temperature.abs() < 100.0 && tt.tint.abs() < 100.0,
                "{grey:?} {tt:?}"
            );
            let p = pv(&[("temperature", tt.temperature), ("tint", tt.tint)]);
            let out = run8(
                &wb(),
                &cx(1.0),
                &p,
                &[grey[0], grey[1], grey[2], 255],
                (1, 1),
            )
            .unwrap();
            let hi = out[..3].iter().max().unwrap();
            let lo = out[..3].iter().min().unwrap();
            assert!(hi - lo <= 1, "{grey:?} -> {out:?}");
            let lg = [g[0] as f32, g[1] as f32, g[2] as f32, 1.0];
            let o = runf(&wb(), &cx(1.0), &p, &lg, (1, 1)).unwrap();
            assert!(
                (o[0] - o[1]).abs() < 1e-5 && (o[1] - o[2]).abs() < 1e-5,
                "{o:?}"
            );
        }
    }

    // FD-33 (f)
    #[test]
    fn fd_33_f_white_balance_keeps_alpha() {
        assert_alpha_is_kept(&wb(), 1.0);
    }

    // FD-33 (h), ARQ-110
    #[test]
    fn fd_33_h_bad_parameters_are_err_and_dst_is_untouched() {
        assert_bad_parameters_are_err_and_ends_pass(&wb());
    }

    #[test]
    fn arq_110_non_finite_scale_and_pixels_are_err() {
        assert_non_finite_scale_and_pixels_are_err(&wb());
    }

    // ARQ-110: a result that overflows `f32` is Err before anything is written.
    #[test]
    fn arq_110_overflowing_result_is_err_and_dst_is_untouched() {
        let src = [0.5f32, 0.5, 0.5, 1.0, 3.0e38, 0.5, 0.5, 1.0];
        let (r, dst) = runf_keep(
            &wb(),
            &cx(1.0),
            &pv(&[("temperature", 100.0)]),
            &src,
            (2, 1),
        );
        assert_eq!(
            r,
            Err(EffectError::Buffer(BufferError::NonFinite {
                field: "result",
                index: 4
            }))
        );
        assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
    }

    #[test]
    fn white_balance_rejects_other_formats_and_sizes() {
        assert_format_and_size_errors(&wb());
    }

    #[test]
    fn white_balance_empty_images_are_ok() {
        assert_empty_images_are_ok(&wb());
    }

    // ARQ-111
    #[test]
    fn arq_111_white_balance_is_deterministic() {
        assert_deterministic(&wb(), &pv(&[("temperature", 37.0), ("tint", -81.0)]));
    }

    // ARQ-120: a pointwise effect needs exactly the region it writes. Passes from the start.
    #[test]
    fn white_balance_roi_is_the_output_region() {
        let out = IRect::new(-3, 4, 90, 40);
        for scale in [0.5f32, 1.0, 2.0] {
            assert_eq!(wb().roi(&cx(scale), &ParamValues::new(), out), out);
        }
    }

    // Passes from the start; proved by a mutant on the cost.
    #[test]
    fn white_balance_cost_grows_with_pixels() {
        let e = wb();
        let c = |px| e.cost_hint(&cx(1.0), &ParamValues::new(), px).units;
        assert_eq!(c(0), 0);
        assert!(c(1000) > 0 && c(2000) == 2 * c(1000));
    }
}
