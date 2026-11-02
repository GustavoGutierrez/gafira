//! Luz: Brillo, Contraste, Luces, Sombras, Blancos and Negros (FD-33).

use gafira_doc::contract::{Describe, Descriptor, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_geom::IRect;

use super::pointwise::{check_io, check_scale, map_straight, slider, smoothstep};
use super::{Cost, Effect};
use crate::transfer::Transfer;
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// The light group, in linear light.
#[derive(Debug)]
pub struct Light {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl Light {
    /// Builds the effect.
    #[must_use]
    pub fn new() -> Self {
        let s = |id, label| slider(id, label, -100.0, 100.0, 0.0);
        Self {
            descriptor: Descriptor::new(
                "effect.adjust.light",
                MessageKey("glossary.effect_light"),
                1,
            ),
            schema: ParamSchema::new(vec![
                s("brightness", "effect.adjust.brightness"),
                s("contrast", "effect.adjust.contrast"),
                s("highlights", "effect.adjust.highlights"),
                s("shadows", "effect.adjust.shadows"),
                s("whites", "effect.adjust.whites"),
                s("blacks", "effect.adjust.blacks"),
            ]),
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Light {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Effect for Light {
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
        let k = |id: &str| values.float(id).unwrap_or(0.0) / 100.0;
        let (brightness, contrast, highlights, shadows, whites, blacks) = (
            k("brightness"),
            k("contrast"),
            k("highlights"),
            k("shadows"),
            k("whites"),
            k("blacks"),
        );
        let gain = brightness.exp2();
        let gamma = contrast.exp2();
        let white = 1.0 - 0.25 * whites;
        let black = -0.25 * blacks;
        let t = Transfer::new();
        // The curves work on the encoded value clipped to [0, 1] (FD-33 notation `v`).
        let encoded = |x: f64| t.encode_f64(x).clamp(0.0, 1.0);
        map_straight(src, dst, |_, c| {
            let mut c = c;
            let mut changed = false;
            if brightness != 0.0 {
                c = c.map(|x| x * gain);
                changed = true;
            }
            if contrast != 0.0 {
                c = c.map(|x| {
                    let v = encoded(x);
                    let (hi, lo) = (v.powf(gamma), (1.0 - v).powf(gamma));
                    t.decode_f64(hi / (hi + lo))
                });
                changed = true;
            }
            if highlights != 0.0 || shadows != 0.0 {
                let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
                let vy = encoded(y);
                let exponent = highlights * smoothstep(0.5, 1.0, vy)
                    + shadows * (1.0 - smoothstep(0.0, 0.5, vy));
                let f = exponent.exp2();
                c = c.map(|x| x * f);
                changed = true;
            }
            if whites != 0.0 || blacks != 0.0 {
                c = c.map(|x| t.decode_f64((encoded(x) - black) / (white - black)));
                changed = true;
            }
            changed.then_some(c)
        })
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::ParamKind;

    use super::super::testing::*;
    use super::*;

    const IDS: [&str; 6] = [
        "brightness",
        "contrast",
        "highlights",
        "shadows",
        "whites",
        "blacks",
    ];

    fn light() -> Light {
        Light::new()
    }

    /// FD-33 formulas of the Luz group, written again from the table in `f64`: the reference.
    /// `k` is the six sliders divided by 100, in `IDS` order.
    fn reference(c: [f64; 3], k: [f64; 6]) -> [f64; 3] {
        let [br, co, hi, sh, wh, bl] = k;
        let mut c = c.map(|x| x * 2f64.powf(br));
        if co != 0.0 {
            let g = 2f64.powf(co);
            c = c.map(|x| {
                let v = enc_f(x).clamp(0.0, 1.0);
                dec_f(v.powf(g) / (v.powf(g) + (1.0 - v).powf(g)))
            });
        }
        let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let vy = enc_f(y).clamp(0.0, 1.0);
        let e = hi * smooth(0.5, 1.0, vy) + sh * (1.0 - smooth(0.0, 0.5, vy));
        c = c.map(|x| x * 2f64.powf(e));
        if wh == 0.0 && bl == 0.0 {
            return c;
        }
        let b = 1.0 - 0.25 * wh;
        let n = -0.25 * bl;
        c.map(|x| dec_f((enc_f(x).clamp(0.0, 1.0) - n) / (b - n)))
    }

    fn params_of(k: [f64; 6]) -> ParamValues {
        let items: Vec<(&str, f64)> = IDS.iter().copied().zip(k.map(|x| x * 100.0)).collect();
        pv(&items)
    }

    fn one_pixel_f32(c: [f64; 3], k: [f64; 6]) -> [f32; 3] {
        let src = [c[0] as f32, c[1] as f32, c[2] as f32, 1.0];
        let out = runf(&light(), &cx(1.0), &params_of(k), &src, (1, 1)).unwrap();
        [out[0], out[1], out[2]]
    }

    fn only(i: usize, v: f64) -> [f64; 6] {
        let mut k = [0.0; 6];
        k[i] = v;
        k
    }

    // Passes from the start (data only); proved by mutants on the ranges, defaults and ids.
    #[test]
    fn fd_33_light_schema_has_six_sliders_of_100() {
        let e = light();
        assert_eq!(e.descriptor().id, "effect.adjust.light");
        e.params().validate_definition().unwrap();
        let ids: Vec<_> = e.params().params().iter().map(|d| d.id).collect();
        assert_eq!(ids, IDS);
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
    fn fd_33_a_light_defaults_are_identity() {
        assert_defaults_are_identity(&light());
    }

    // FD-33 (e): the analytic values of the table, +-1e-5, on encoded values.
    #[test]
    fn fd_33_e_light_analytic_values() {
        let near = |a: f64, b: f64, what: &str| assert!((a - b).abs() < 1e-5, "{what}: {a} vs {b}");
        // Brillo +100 takes 0.25 to 0.5 (linear light).
        let o = one_pixel_f32([0.25; 3], only(0, 1.0));
        near(f64::from(o[0]), 0.5, "brightness +100");
        // Contraste leaves v = 0.5 where it is, with any value.
        for v in [-100.0, -63.0, -1.0, 1.0, 37.0, 100.0] {
            let o = one_pixel_f32([dec_f(0.5); 3], only(1, v / 100.0));
            near(enc_f(f64::from(o[1])), 0.5, "contrast fixed point");
        }
        // Contraste +100 takes v = 0.75 to 0.9.
        let o = one_pixel_f32([dec_f(0.75); 3], only(1, 1.0));
        near(enc_f(f64::from(o[2])), 0.9, "contrast +100");
        // Blancos +100 takes v = 0.75 to 1.
        let o = one_pixel_f32([dec_f(0.75); 3], only(4, 1.0));
        near(enc_f(f64::from(o[0])), 1.0, "whites +100");
    }

    // FD-33 formulas: Luces/Sombras at known points, on grey pixels where v_Y = v.
    #[test]
    fn fd_33_light_highlights_and_shadows_weights() {
        let grey = |v: f64, k: [f64; 6]| f64::from(one_pixel_f32([dec_f(v); 3], k)[0]);
        let hi = only(2, 1.0);
        let sh = only(3, 1.0);
        // Highlights: nothing up to v = 0.5, x2 at v = 1, the smoothstep half way (x sqrt 2).
        assert_eq!(grey(0.5, hi), grey(0.5, [0.0; 6]));
        assert!((grey(1.0, hi) - 2.0).abs() < 1e-5);
        assert!((grey(0.75, hi) / dec_f(0.75) - 2f64.sqrt()).abs() < 1e-5);
        // Shadows: x2 at v = 0, nothing from v = 0.5, x sqrt 2 at v = 0.25.
        assert!((grey(0.25, sh) / dec_f(0.25) - 2f64.sqrt()).abs() < 1e-5);
        assert_eq!(grey(0.5, sh), grey(0.5, [0.0; 6]));
        assert_eq!(grey(0.9, sh), grey(0.9, [0.0; 6]));
        // Negative values darken in the same bands.
        assert!((grey(1.0, only(2, -1.0)) - 0.5).abs() < 1e-5);
        assert!((grey(0.25, only(3, -1.0)) / dec_f(0.25) - 1.0 / 2f64.sqrt()).abs() < 1e-5);
    }

    // FD-33: Blancos and Negros move the ends of v.
    #[test]
    fn fd_33_light_whites_and_blacks_move_the_ends() {
        let at = |v: f64, k: [f64; 6]| enc_f(f64::from(one_pixel_f32([dec_f(v); 3], k)[1]));
        let near = |a: f64, b: f64| assert!((a - b).abs() < 1e-5, "{a} vs {b}");
        near(at(0.75, only(4, 1.0)), 1.0);
        near(at(0.5, only(4, -1.0)), 0.5 / 1.25);
        near(at(0.0, only(5, 1.0)), 0.25 / 1.25);
        near(at(0.25, only(5, -1.0)), 0.0);
        near(
            at(0.5, [0.0, 0.0, 0.0, 0.0, 0.5, 0.5]),
            (0.5 + 0.125) / (0.875 + 0.125),
        );
        // Beyond 1 and below 0 is kept in f32 (02 4.17.4).
        assert!(at(0.9, only(4, 1.0)) > 1.0);
        assert!(at(0.1, only(5, -1.0)) < 0.0);
    }

    // 02 4.17.4: Contraste clips its input to [0, 1]; Brillo does not clip.
    #[test]
    fn fd_33_light_contrast_clips_and_brightness_does_not() {
        let o = one_pixel_f32([2.0, -0.5, 0.5], only(1, 0.3));
        assert!((o[0] - 1.0).abs() < 1e-6, "{o:?}");
        assert!(o[1].abs() < 1e-6, "{o:?}");
        let o = one_pixel_f32([2.0, 0.5, 0.5], only(0, 1.0));
        assert!((o[0] - 4.0).abs() < 1e-5, "{o:?}");
    }

    // FD-33: all six sliders against the reference, f32, +-1e-5 relative to max(1, |value|).
    #[test]
    fn fd_33_light_matches_the_reference_in_f32() {
        let pixels = [
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.2, 0.5, 0.8],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.05, 0.02, 0.4],
            [0.7, 0.6, 0.1],
            [0.004, 0.2, 0.9],
            [1.7, 0.3, 0.01],
        ];
        let settings: [[f64; 6]; 9] = [
            [0.3, 0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, -0.6, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.7, -0.4, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, -0.8, 0.5],
            [0.2, 0.5, 0.3, 0.9, 0.4, -0.7],
            [-0.5, -0.5, -0.5, -0.5, -0.5, -0.5],
            [1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
            [-1.0, -1.0, -1.0, -1.0, -1.0, -1.0],
            [0.0, 0.0, -1.0, 1.0, 0.25, 0.0],
        ];
        for k in settings {
            for c in pixels {
                let got = one_pixel_f32(c, k);
                let want = reference(c, k);
                for ch in 0..3 {
                    let tol = 1e-5 * want[ch].abs().max(1.0);
                    assert!(
                        (f64::from(got[ch]) - want[ch]).abs() < tol,
                        "{c:?} {k:?} ch {ch}: {} vs {}",
                        got[ch],
                        want[ch]
                    );
                }
            }
        }
    }

    // FD-33, 8 bits: the reference within 1/255, premultiplied pixels included.
    #[test]
    fn fd_33_light_matches_the_reference_in_8_bits() {
        let k = [0.2, 0.5, 0.3, 0.9, 0.4, -0.7];
        for k in [k, only(0, 0.6), only(1, -0.8), only(3, 0.9), only(5, 1.0)] {
            for px in [
                [10u8, 100, 200, 255],
                [255, 255, 255, 255],
                [0, 0, 0, 255],
                [90, 70, 40, 200],
            ] {
                let a = f64::from(px[3]);
                let straight: Vec<f64> = (0..3)
                    .map(|i| (f64::from(px[i]) * 255.0 / a).round().min(255.0))
                    .collect();
                let want = reference([lin(straight[0]), lin(straight[1]), lin(straight[2])], k);
                let out = run8(&light(), &cx(1.0), &params_of(k), &px, (1, 1)).unwrap();
                for ch in 0..3 {
                    let w = (f64::from(enc(want[ch])) * a / 255.0).round();
                    assert!(
                        (f64::from(out[ch]) - w).abs() <= 1.0,
                        "{px:?} {k:?} ch {ch}: {} vs {w}",
                        out[ch]
                    );
                }
                assert_eq!(out[3], px[3]);
            }
        }
    }

    // FD-33 (f)
    #[test]
    fn fd_33_f_light_keeps_alpha() {
        assert_alpha_is_kept(&light(), 1.0, &[]);
    }

    // FD-33 (h), ARQ-110
    #[test]
    fn fd_33_h_light_bad_parameters_are_err_and_dst_is_untouched() {
        assert_bad_parameters_are_err_and_ends_pass(&light());
    }

    #[test]
    fn arq_110_light_non_finite_scale_and_pixels_are_err() {
        assert_non_finite_scale_and_pixels_are_err(&light());
    }

    // ARQ-110: a result that overflows `f32` is Err before anything is written.
    #[test]
    fn arq_110_light_overflowing_result_is_err_and_dst_is_untouched() {
        let src = [0.5f32, 0.5, 0.5, 1.0, 3.0e38, 0.5, 0.5, 1.0];
        let (r, dst) = runf_keep(
            &light(),
            &cx(1.0),
            &pv(&[("brightness", 100.0)]),
            &src,
            (2, 1),
        );
        assert!(matches!(r, Err(EffectError::Buffer(_))), "{r:?}");
        assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
    }

    #[test]
    fn light_rejects_other_formats_and_sizes() {
        assert_format_and_size_errors(&light());
    }

    #[test]
    fn light_empty_images_are_ok() {
        assert_empty_images_are_ok(&light());
    }

    // ARQ-111
    #[test]
    fn arq_111_light_is_deterministic() {
        assert_deterministic(&light(), &params_of([0.2, 0.5, 0.3, 0.9, 0.4, -0.7]));
    }

    // ARQ-120: a pointwise effect needs exactly the region it writes. Passes from the start.
    #[test]
    fn light_roi_is_the_output_region() {
        let out = IRect::new(-3, 4, 90, 40);
        for scale in [0.5f32, 1.0, 2.0] {
            assert_eq!(light().roi(&cx(scale), &ParamValues::new(), out), out);
        }
    }

    // Passes from the start; proved by a mutant on the cost.
    #[test]
    fn light_cost_grows_with_pixels() {
        let e = light();
        let c = |px| e.cost_hint(&cx(1.0), &ParamValues::new(), px).units;
        assert_eq!(c(0), 0);
        assert!(c(1000) > 0 && c(2000) == 2 * c(1000));
    }

    // ARQ-110: an id the schema does not know is never read (killed mutant: the effect behaves
    // differently when one is present).
    #[test]
    fn arq_110_light_unknown_ids_are_ignored() {
        assert_unknown_ids_are_ignored(&light());
    }

    // ARQ-110: roi and cost_hint do not panic at the extremes (killed mutants: an unchecked
    // multiply of `px`, an assert on a finite scale).
    #[test]
    fn arq_110_light_roi_and_cost_hint_survive_the_extremes() {
        assert_roi_and_cost_extremes(&light());
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(48))]

        // ARQ-110: finite input inside the domain gives finite output.
        #[test]
        fn arq_110_light_finite_input_gives_finite_output(
            px in proptest::collection::vec(-1.0e6f32..1.0e6, 4 * 12),
            fractions in proptest::collection::vec(0.0f64..=1.0, 1..40),
            scale in 0.01f32..8.0,
        ) {
            finite_output_case(&light(), &px, &fractions, scale)?;
        }
    }
}
