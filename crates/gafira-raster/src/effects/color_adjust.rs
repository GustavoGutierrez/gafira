//! Color: Tono, Saturación, Vitalidad and Invertir (FD-33).

use gafira_doc::contract::{Describe, Descriptor, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_geom::IRect;

use super::pointwise::{check_io, check_scale, map_straight, slider};
use super::{Cost, Effect};
use crate::transfer::Transfer;
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// The colour group, in linear light.
#[derive(Debug)]
pub struct ColorAdjust {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl ColorAdjust {
    /// Builds the effect.
    #[must_use]
    pub fn new() -> Self {
        Self {
            descriptor: Descriptor::new(
                "effect.adjust.color",
                MessageKey("glossary.effect_color"),
                1,
            ),
            schema: ParamSchema::new(vec![
                slider("hue", "effect.adjust.hue", -180.0, 180.0, 0.0),
                slider("saturation", "effect.adjust.saturation", -100.0, 100.0, 0.0),
                slider("vibrance", "effect.adjust.vibrance", -100.0, 100.0, 0.0),
                ParamDef {
                    id: "invert",
                    label: MessageKey("effect.adjust.invert"),
                    kind: ParamKind::Bool { default: false },
                    animatable: false,
                },
            ]),
        }
    }
}

impl Default for ColorAdjust {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for ColorAdjust {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

pub(super) type Matrix = [[f64; 3]; 3];

/// W3C *Filter Effects* `hue-rotate(degrees)`.
pub(super) fn hue_matrix(degrees: f64) -> Matrix {
    let (s, k) = degrees.to_radians().sin_cos();
    [
        [
            0.213 + k * 0.787 - s * 0.213,
            0.715 - k * 0.715 - s * 0.715,
            0.072 - k * 0.072 + s * 0.928,
        ],
        [
            0.213 - k * 0.213 + s * 0.143,
            0.715 + k * 0.285 + s * 0.140,
            0.072 - k * 0.072 - s * 0.283,
        ],
        [
            0.213 - k * 0.213 - s * 0.787,
            0.715 - k * 0.715 + s * 0.715,
            0.072 + k * 0.928 + s * 0.072,
        ],
    ]
}

/// W3C *Filter Effects* `saturate(s)`.
pub(super) fn saturate_matrix(s: f64) -> Matrix {
    [
        [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
        [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
        [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
    ]
}

pub(super) fn apply(m: &Matrix, c: [f64; 3]) -> [f64; 3] {
    m.map(|r| r[0] * c[0] + r[1] * c[1] + r[2] * c[2])
}

impl Effect for ColorAdjust {
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
        let hue = values.float("hue").unwrap_or(0.0);
        let saturation = values.float("saturation").unwrap_or(0.0) / 100.0;
        let vibrance = values.float("vibrance").unwrap_or(0.0) / 100.0;
        let invert = values.bool("invert").unwrap_or(false);
        let rotation = hue_matrix(hue);
        let t = Transfer::new();
        map_straight(src, dst, |_, c| {
            let mut c = c;
            let mut changed = false;
            if hue != 0.0 {
                c = apply(&rotation, c);
                changed = true;
            }
            if saturation != 0.0 {
                c = apply(&saturate_matrix(1.0 + saturation), c);
                changed = true;
            }
            if vibrance != 0.0 {
                let max = c[0].max(c[1]).max(c[2]);
                let min = c[0].min(c[1]).min(c[2]);
                c = apply(&saturate_matrix(1.0 + vibrance * (1.0 - (max - min))), c);
                changed = true;
            }
            if invert {
                c = c.map(|x| t.decode_f64(1.0 - t.encode_f64(x).clamp(0.0, 1.0)));
                changed = true;
            }
            changed.then_some(c)
        })
    }
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::{ParamError, ParamValue};

    use super::super::testing::*;
    use super::*;

    fn color() -> ColorAdjust {
        ColorAdjust::new()
    }

    /// W3C *Filter Effects* `hue-rotate` then `saturate`, written again from the spec, in `f64`.
    fn matrix_hue(c: [f64; 3], degrees: f64) -> [f64; 3] {
        let (s, k) = degrees.to_radians().sin_cos();
        let m = [
            [
                0.213 + k * 0.787 - s * 0.213,
                0.715 - k * 0.715 - s * 0.715,
                0.072 - k * 0.072 + s * 0.928,
            ],
            [
                0.213 - k * 0.213 + s * 0.143,
                0.715 + k * 0.285 + s * 0.140,
                0.072 - k * 0.072 - s * 0.283,
            ],
            [
                0.213 - k * 0.213 - s * 0.787,
                0.715 - k * 0.715 + s * 0.715,
                0.072 + k * 0.928 + s * 0.072,
            ],
        ];
        m.map(|r| r[0] * c[0] + r[1] * c[1] + r[2] * c[2])
    }

    fn matrix_saturate(c: [f64; 3], s: f64) -> [f64; 3] {
        let m = [
            [0.213 + 0.787 * s, 0.715 - 0.715 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 + 0.285 * s, 0.072 - 0.072 * s],
            [0.213 - 0.213 * s, 0.715 - 0.715 * s, 0.072 + 0.928 * s],
        ];
        m.map(|r| r[0] * c[0] + r[1] * c[1] + r[2] * c[2])
    }

    /// The Color group of FD-33 from its table, for hue in degrees, saturation and vibrance
    /// divided by 100.
    fn reference(c: [f64; 3], hue: f64, sat: f64, vib: f64, invert: bool) -> [f64; 3] {
        let mut c = c;
        if hue != 0.0 {
            c = matrix_hue(c, hue);
        }
        if sat != 0.0 {
            c = matrix_saturate(c, 1.0 + sat);
        }
        if vib != 0.0 {
            let max = c[0].max(c[1]).max(c[2]);
            let min = c[0].min(c[1]).min(c[2]);
            c = matrix_saturate(c, 1.0 + vib * (1.0 - (max - min)));
        }
        if invert {
            c = c.map(|x| dec_f(1.0 - enc_f(x).clamp(0.0, 1.0)));
        }
        c
    }

    fn params(hue: f64, sat: f64, vib: f64, invert: bool) -> ParamValues {
        let mut p = pv(&[("hue", hue), ("saturation", sat), ("vibrance", vib)]);
        p.set("invert", ParamValue::Bool(invert));
        p
    }

    fn one(c: [f64; 3], p: &ParamValues) -> [f64; 3] {
        let src = [c[0] as f32, c[1] as f32, c[2] as f32, 1.0];
        let o = runf(&color(), &cx(1.0), p, &src, (1, 1)).unwrap();
        [f64::from(o[0]), f64::from(o[1]), f64::from(o[2])]
    }

    // Passes from the start (data only); proved by mutants on the ranges, defaults and ids.
    #[test]
    fn fd_33_color_schema_has_three_sliders_and_a_switch() {
        let e = color();
        assert_eq!(e.descriptor().id, "effect.adjust.color");
        e.params().validate_definition().unwrap();
        let ids: Vec<_> = e.params().params().iter().map(|d| d.id).collect();
        assert_eq!(ids, ["hue", "saturation", "vibrance", "invert"]);
        let kind = |id: &str| e.params().get(id).unwrap().kind.clone();
        assert_eq!(
            kind("hue"),
            ParamKind::Float {
                min: -180.0,
                max: 180.0,
                default: 0.0
            }
        );
        for id in ["saturation", "vibrance"] {
            assert_eq!(
                kind(id),
                ParamKind::Float {
                    min: -100.0,
                    max: 100.0,
                    default: 0.0
                }
            );
        }
        assert_eq!(kind("invert"), ParamKind::Bool { default: false });
        assert!(e.gpu().is_none());
    }

    // FD-33 (a)
    #[test]
    fn fd_33_a_color_defaults_are_identity() {
        assert_defaults_are_identity(&color());
        let off = params(0.0, 0.0, 0.0, false);
        let s8 = img_u8(6, 6, 8);
        assert_eq!(run8(&color(), &cx(1.0), &off, &s8, (6, 6)).unwrap(), s8);
    }

    // FD-33 (e): Saturación -100 makes R = G = B, at the W3C luminance weights.
    #[test]
    fn fd_33_e_saturation_minus_100_gives_equal_channels() {
        let o = one([0.8, 0.3, 0.1], &pv(&[("saturation", -100.0)]));
        let y = 0.213 * 0.8 + 0.715 * 0.3 + 0.072 * 0.1;
        for ch in o {
            assert!((ch - y).abs() < 1e-6, "{o:?}");
        }
        let px = run8(
            &color(),
            &cx(1.0),
            &pv(&[("saturation", -100.0)]),
            &[200, 60, 20, 255],
            (1, 1),
        )
        .unwrap();
        assert!(px[0] == px[1] && px[1] == px[2], "{px:?}");
    }

    // FD-33: Tono, with values from the W3C matrix computed by hand.
    #[test]
    fn fd_33_color_hue_rotates_the_linear_channels() {
        // Red at 90 degrees: cos 0, sin 1.
        let o = one([1.0, 0.0, 0.0], &pv(&[("hue", 90.0)]));
        let want = [0.213 - 0.213, 0.213 + 0.143, 0.213 - 0.787];
        for k in 0..3 {
            assert!((o[k] - want[k]).abs() < 1e-6, "{o:?} vs {want:?}");
        }
        // 180 degrees on a grey leaves it grey; a half turn both ways is the same colour.
        let g = one([0.4, 0.4, 0.4], &pv(&[("hue", 180.0)]));
        assert!(g.iter().all(|x| (x - 0.4).abs() < 1e-6), "{g:?}");
        let (a, b) = (
            one([0.7, 0.2, 0.4], &pv(&[("hue", 180.0)])),
            one([0.7, 0.2, 0.4], &pv(&[("hue", -180.0)])),
        );
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-5, "{a:?} {b:?}");
        }
        // The sign is the direction: +30 and -30 differ.
        let (p, m) = (
            one([0.7, 0.2, 0.4], &pv(&[("hue", 30.0)])),
            one([0.7, 0.2, 0.4], &pv(&[("hue", -30.0)])),
        );
        assert!((p[1] - m[1]).abs() > 1e-3);
    }

    // FD-33: Vitalidad is Saturación with s = 1 + k (1 - (max - min)).
    #[test]
    fn fd_33_color_vibrance_saturates_dull_colours_more() {
        let dull = [0.5, 0.4, 0.4];
        let vivid = [1.0, 0.0, 0.0];
        let v = |c, k| one(c, &pv(&[("vibrance", k)]));
        // Spread 0.1 gives s = 1 + 0.9 k; spread 1 gives s = 1: a fully spread colour stays.
        let want = matrix_saturate(dull, 1.0 + 0.9);
        let got = v(dull, 100.0);
        for k in 0..3 {
            assert!((got[k] - want[k]).abs() < 1e-6, "{got:?} {want:?}");
        }
        let got = v(vivid, 100.0);
        for k in 0..3 {
            assert!((got[k] - vivid[k]).abs() < 1e-6, "{got:?}");
        }
        // Negative values desaturate.
        let got = v(dull, -100.0);
        let want = matrix_saturate(dull, 1.0 - 0.9);
        for k in 0..3 {
            assert!((got[k] - want[k]).abs() < 1e-6, "{got:?} {want:?}");
        }
    }

    // FD-33 (f): Invertir twice returns the image, <= 1/255; v' = 1 - v on the encoded value.
    #[test]
    fn fd_33_f_invert_twice_is_the_original_and_flips_v() {
        let on = params(0.0, 0.0, 0.0, true);
        let o = one([dec_f(0.3); 3], &on);
        assert!((enc_f(o[0]) - 0.7).abs() < 1e-5, "{o:?}");
        let s8 = img_u8(12, 9, 6);
        let once = run8(&color(), &cx(1.0), &on, &s8, (12, 9)).unwrap();
        assert_ne!(once, s8);
        let twice = run8(&color(), &cx(1.0), &on, &once, (12, 9)).unwrap();
        for (a, b) in s8.iter().zip(&twice) {
            assert!(a.abs_diff(*b) <= 1, "{a} vs {b}");
        }
        let px = run8(&color(), &cx(1.0), &on, &[0, 128, 255, 255], (1, 1)).unwrap();
        assert_eq!(px, [255, 127, 0, 255]);
        let sf = img_f32(12, 9, 6);
        let a = runf(&color(), &cx(1.0), &on, &sf, (12, 9)).unwrap();
        let b = runf(&color(), &cx(1.0), &on, &a, (12, 9)).unwrap();
        for (p, q) in sf.chunks(4).zip(b.chunks(4)) {
            if p[3] != 0.0 {
                // Values beyond [0, 1] are clipped by the curve, so compare the clipped input.
                for k in 0..3 {
                    let want =
                        (f64::from(p[k]) / f64::from(p[3])).clamp(0.0, 1.0) * f64::from(p[3]);
                    assert!((f64::from(q[k]) - want).abs() < 1e-5, "{p:?} {q:?}");
                }
            }
        }
    }

    // FD-33 formulas against the reference, f32.
    #[test]
    fn fd_33_color_matches_the_reference_in_f32() {
        let pixels = [
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.8, 0.3, 0.1],
            [0.1, 0.6, 0.9],
            [0.02, 0.02, 0.5],
            [1.0, 0.0, 0.0],
            [1.6, 0.3, 0.2],
        ];
        let settings = [
            (45.0, 0.0, 0.0, false),
            (0.0, 0.5, 0.0, false),
            (0.0, 0.0, -0.7, false),
            (-120.0, 0.3, 0.6, false),
            (90.0, -0.4, 0.5, true),
            (180.0, 1.0, 1.0, true),
            (-180.0, -1.0, -1.0, false),
            (0.0, 0.0, 0.0, true),
        ];
        for (h, s, v, i) in settings {
            let p = params(h, s * 100.0, v * 100.0, i);
            for c in pixels {
                let got = one(c, &p);
                let want = reference(c, h, s, v, i);
                for k in 0..3 {
                    let tol = 1e-5 * want[k].abs().max(1.0);
                    assert!(
                        (got[k] - want[k]).abs() < tol,
                        "{c:?} {h} {s} {v} {i} ch {k}: {} vs {}",
                        got[k],
                        want[k]
                    );
                }
            }
        }
    }

    // FD-33, 8 bits: the reference within 1/255.
    #[test]
    fn fd_33_color_matches_the_reference_in_8_bits() {
        for (h, s, v, i) in [
            (60.0, 0.0, 0.0, false),
            (0.0, -0.5, 0.0, false),
            (0.0, 0.0, 0.8, false),
            (-100.0, 0.4, 0.3, true),
        ] {
            let p = params(h, s * 100.0, v * 100.0, i);
            for px in [[10u8, 100, 200, 255], [255, 30, 0, 255], [90, 70, 40, 200]] {
                let a = f64::from(px[3]);
                let straight = [0, 1, 2].map(|k| (f64::from(px[k]) * 255.0 / a).round().min(255.0));
                let want = reference(straight.map(lin), h, s, v, i);
                let out = run8(&color(), &cx(1.0), &p, &px, (1, 1)).unwrap();
                for k in 0..3 {
                    let w = (f64::from(enc(want[k])) * a / 255.0).round();
                    assert!(
                        (f64::from(out[k]) - w).abs() <= 1.0,
                        "{px:?} ch {k}: {} vs {w}",
                        out[k]
                    );
                }
                assert_eq!(out[3], px[3]);
            }
        }
    }

    // FD-33 (f)
    #[test]
    fn fd_33_f_color_keeps_alpha() {
        assert_alpha_is_kept(
            &color(),
            1.0,
            &[params(0.0, 0.0, 0.0, true), params(90.0, 40.0, 40.0, true)],
        );
    }

    // FD-33 (h), ARQ-110
    #[test]
    fn fd_33_h_color_bad_parameters_are_err_and_dst_is_untouched() {
        assert_bad_parameters_are_err_and_ends_pass(&color());
        // A switch given a number is a wrong type, not a silent default.
        let mut p = ParamValues::new();
        p.set("invert", ParamValue::Float(1.0));
        let (r, dst) = run8_keep(&color(), &cx(1.0), &p, &img_u8(2, 2, 1), (2, 2));
        assert_eq!(
            r,
            Err(EffectError::Param(ParamError::WrongType {
                param: "invert"
            }))
        );
        assert!(dst.iter().all(|b| *b == SENTINEL));
    }

    #[test]
    fn arq_110_color_non_finite_scale_and_pixels_are_err() {
        assert_non_finite_scale_and_pixels_are_err(&color());
    }

    // ARQ-110: a result that overflows `f32` is Err before anything is written.
    #[test]
    fn arq_110_color_overflowing_result_is_err_and_dst_is_untouched() {
        // Saturation +100 scales a channel near f32::MAX by 1.787, past the range of f32.
        let src = [0.5f32, 0.5, 0.5, 1.0, 3.0e38, 0.0, 0.0, 1.0];
        let (r, dst) = runf_keep(
            &color(),
            &cx(1.0),
            &pv(&[("saturation", 100.0)]),
            &src,
            (2, 1),
        );
        assert!(matches!(r, Err(EffectError::Buffer(_))), "{r:?}");
        assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
    }

    #[test]
    fn color_rejects_other_formats_and_sizes() {
        assert_format_and_size_errors(&color());
    }

    #[test]
    fn color_empty_images_are_ok() {
        assert_empty_images_are_ok(&color());
    }

    // ARQ-111
    #[test]
    fn arq_111_color_is_deterministic() {
        assert_deterministic(&color(), &params(33.0, -20.0, 70.0, true));
    }

    // ARQ-120: a pointwise effect needs exactly the region it writes. Passes from the start.
    #[test]
    fn color_roi_is_the_output_region() {
        let out = IRect::new(-3, 4, 90, 40);
        for scale in [0.5f32, 1.0, 2.0] {
            assert_eq!(color().roi(&cx(scale), &ParamValues::new(), out), out);
        }
    }

    // Passes from the start; proved by a mutant on the cost.
    #[test]
    fn color_cost_grows_with_pixels() {
        let e = color();
        let c = |px| e.cost_hint(&cx(1.0), &ParamValues::new(), px).units;
        assert_eq!(c(0), 0);
        assert!(c(1000) > 0 && c(2000) == 2 * c(1000));
    }

    // ARQ-110: an id the schema does not know is never read (killed mutant: the effect behaves
    // differently when one is present).
    #[test]
    fn arq_110_color_unknown_ids_are_ignored() {
        assert_unknown_ids_are_ignored(&color());
    }

    // ARQ-110: roi and cost_hint do not panic at the extremes (killed mutants: an unchecked
    // multiply of `px`, an assert on a finite scale).
    #[test]
    fn arq_110_color_roi_and_cost_hint_survive_the_extremes() {
        assert_roi_and_cost_extremes(&color());
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(48))]

        // ARQ-110: finite input inside the domain gives finite output.
        #[test]
        fn arq_110_color_finite_input_gives_finite_output(
            px in proptest::collection::vec(-1.0e6f32..1.0e6, 4 * 12),
            fractions in proptest::collection::vec(0.0f64..=1.0, 1..40),
            scale in 0.01f32..8.0,
        ) {
            finite_output_case(&color(), &px, &fractions, scale)?;
        }
    }
}
