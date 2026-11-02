//! Color: edición por colores detectados (FD-33): up to eight samples, each moving only the
//! colours near it in Tono and Saturación.

use gafira_color::{Color, Lab, LinearRgba, Rgba, delta_e2000};
use gafira_doc::contract::{Describe, Descriptor, ParamDef, ParamKind, ParamSchema, ParamValues};
use gafira_doc::msg::MessageKey;
use gafira_geom::IRect;

use super::color_adjust::{apply, hue_matrix, saturate_matrix};
use super::pointwise::{check_io, check_scale, map_straight, slider, smoothstep};
use super::{Cost, Effect};
use crate::{EffectCx, EffectError, ImageView, ImageViewMut};

/// How many samples the effect holds (FD-33).
pub const DETECTED_SLOTS: usize = 8;

const COLOR: [&str; 8] = [
    "color_1", "color_2", "color_3", "color_4", "color_5", "color_6", "color_7", "color_8",
];
const HUE: [&str; 8] = [
    "hue_1", "hue_2", "hue_3", "hue_4", "hue_5", "hue_6", "hue_7", "hue_8",
];
const SATURATION: [&str; 8] = [
    "saturation_1",
    "saturation_2",
    "saturation_3",
    "saturation_4",
    "saturation_5",
    "saturation_6",
    "saturation_7",
    "saturation_8",
];
const TOLERANCE: [&str; 8] = [
    "tolerance_1",
    "tolerance_2",
    "tolerance_3",
    "tolerance_4",
    "tolerance_5",
    "tolerance_6",
    "tolerance_7",
    "tolerance_8",
];
const COLOR_LABEL: [&str; 8] = [
    "effect.adjust.detected_color_1",
    "effect.adjust.detected_color_2",
    "effect.adjust.detected_color_3",
    "effect.adjust.detected_color_4",
    "effect.adjust.detected_color_5",
    "effect.adjust.detected_color_6",
    "effect.adjust.detected_color_7",
    "effect.adjust.detected_color_8",
];
const HUE_LABEL: [&str; 8] = [
    "effect.adjust.detected_hue_1",
    "effect.adjust.detected_hue_2",
    "effect.adjust.detected_hue_3",
    "effect.adjust.detected_hue_4",
    "effect.adjust.detected_hue_5",
    "effect.adjust.detected_hue_6",
    "effect.adjust.detected_hue_7",
    "effect.adjust.detected_hue_8",
];
const SATURATION_LABEL: [&str; 8] = [
    "effect.adjust.detected_saturation_1",
    "effect.adjust.detected_saturation_2",
    "effect.adjust.detected_saturation_3",
    "effect.adjust.detected_saturation_4",
    "effect.adjust.detected_saturation_5",
    "effect.adjust.detected_saturation_6",
    "effect.adjust.detected_saturation_7",
    "effect.adjust.detected_saturation_8",
];
const TOLERANCE_LABEL: [&str; 8] = [
    "effect.adjust.detected_tolerance_1",
    "effect.adjust.detected_tolerance_2",
    "effect.adjust.detected_tolerance_3",
    "effect.adjust.detected_tolerance_4",
    "effect.adjust.detected_tolerance_5",
    "effect.adjust.detected_tolerance_6",
    "effect.adjust.detected_tolerance_7",
    "effect.adjust.detected_tolerance_8",
];

/// Tolerance of a sample until the person changes it, in dE2000 (FD-26 matches within 15).
const DEFAULT_TOLERANCE: f64 = 15.0;

/// The colour a sample holds until the person picks one.
const DEFAULT_SAMPLE: Color = Color::Srgb(Rgba {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
});

/// Detected colours: each sample moves Tono and Saturación of the colours within its tolerance.
#[derive(Debug)]
pub struct DetectedColors {
    descriptor: Descriptor,
    schema: ParamSchema,
}

impl DetectedColors {
    /// Builds the effect.
    #[must_use]
    pub fn new() -> Self {
        let mut params = Vec::with_capacity(4 * DETECTED_SLOTS);
        for n in 0..DETECTED_SLOTS {
            params.push(ParamDef {
                id: COLOR[n],
                label: MessageKey(COLOR_LABEL[n]),
                kind: ParamKind::Color {
                    default: DEFAULT_SAMPLE,
                },
                animatable: true,
            });
            params.push(slider(HUE[n], HUE_LABEL[n], -180.0, 180.0, 0.0));
            params.push(slider(
                SATURATION[n],
                SATURATION_LABEL[n],
                -100.0,
                100.0,
                0.0,
            ));
            params.push(slider(
                TOLERANCE[n],
                TOLERANCE_LABEL[n],
                0.0,
                100.0,
                DEFAULT_TOLERANCE,
            ));
        }
        Self {
            descriptor: Descriptor::new(
                "effect.adjust.detected_colors",
                MessageKey("glossary.effect_detected_colors"),
                1,
            ),
            schema: ParamSchema::new(params),
        }
    }
}

impl Default for DetectedColors {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for DetectedColors {
    fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }
}

impl Effect for DetectedColors {
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
        // Only the samples that move something count.
        let samples: Vec<Sample> = (0..DETECTED_SLOTS)
            .filter_map(|n| {
                let hue = values.float(HUE[n]).unwrap_or(0.0);
                let saturation = values.float(SATURATION[n]).unwrap_or(0.0) / 100.0;
                if hue == 0.0 && saturation == 0.0 {
                    return None;
                }
                let c = values
                    .color(COLOR[n])
                    .unwrap_or(DEFAULT_SAMPLE)
                    .to_linear_srgb();
                Some(Sample {
                    lab: lab_of([c.r, c.g, c.b]),
                    hue,
                    saturation,
                    tolerance: values.float(TOLERANCE[n]).unwrap_or(DEFAULT_TOLERANCE),
                })
            })
            .collect();
        map_straight(src, dst, |_, c| {
            if samples.is_empty() {
                return None;
            }
            // The weights come from the colour as it was, not as earlier samples left it.
            let lab = lab_of(c.map(|x| x as f32));
            let mut out = c;
            let mut changed = false;
            for s in &samples {
                let weight =
                    1.0 - smoothstep(s.tolerance / 2.0, s.tolerance, delta_e2000(lab, s.lab));
                if weight <= 0.0 {
                    continue;
                }
                if s.hue != 0.0 {
                    out = apply(&hue_matrix(s.hue * weight), out);
                }
                if s.saturation != 0.0 {
                    out = apply(&saturate_matrix(1.0 + s.saturation * weight), out);
                }
                changed = true;
            }
            changed.then_some(out)
        })
    }
}

/// A sample in Lab, with what it does and how far it reaches.
struct Sample {
    lab: Lab,
    hue: f64,
    saturation: f64,
    tolerance: f64,
}

/// Lab of a linear colour, with the channels cut to `[0, 1]` (a colour beyond the gamut has no
/// distance worth measuring, and `cbrt` of a negative would flip the sign).
fn lab_of(c: [f32; 3]) -> Lab {
    let [r, g, b] = c.map(|x| x.clamp(0.0, 1.0));
    Lab::from_linear_srgb(LinearRgba::new(r, g, b, 1.0))
}

#[cfg(test)]
mod tests {
    use gafira_doc::contract::{ParamError, ParamValue};

    use super::super::ColorAdjust;
    use super::super::testing::*;
    use super::*;

    fn det() -> DetectedColors {
        DetectedColors::new()
    }

    fn sample(r: f32, g: f32, b: f32) -> ParamValue {
        ParamValue::Color(Color::LinearSrgb(LinearRgba::new(r, g, b, 1.0)))
    }

    /// Slot `n` (0-based) set to a sample, a hue, a saturation and a tolerance.
    fn slot(p: &mut ParamValues, n: usize, c: [f32; 3], hue: f64, sat: f64, tol: f64) {
        p.set(COLOR[n], sample(c[0], c[1], c[2]));
        p.set(HUE[n], ParamValue::Float(hue));
        p.set(SATURATION[n], ParamValue::Float(sat));
        p.set(TOLERANCE[n], ParamValue::Float(tol));
    }

    fn lab(c: [f32; 3]) -> Lab {
        Lab::from_linear_srgb(LinearRgba::new(c[0], c[1], c[2], 1.0))
    }

    fn one(c: [f32; 3], p: &ParamValues) -> [f32; 3] {
        let o = runf(&det(), &cx(1.0), p, &[c[0], c[1], c[2], 1.0], (1, 1)).unwrap();
        [o[0], o[1], o[2]]
    }

    /// What Color gives with the hue and saturation scaled by a weight.
    fn color_with(c: [f32; 3], hue: f64, sat: f64) -> [f32; 3] {
        let o = runf(
            &ColorAdjust::new(),
            &cx(1.0),
            &pv(&[("hue", hue), ("saturation", sat)]),
            &[c[0], c[1], c[2], 1.0],
            (1, 1),
        )
        .unwrap();
        [o[0], o[1], o[2]]
    }

    const SAMPLE: [f32; 3] = [0.6, 0.2, 0.2];
    const OTHER: [f32; 3] = [0.2, 0.5, 0.7];

    /// A colour a fraction `t` of the way from the sample to another one.
    fn mix(t: f32) -> [f32; 3] {
        [0, 1, 2].map(|k| SAMPLE[k] * (1.0 - t) + OTHER[k] * t)
    }

    // Passes from the start (data only); proved by mutants on ranges, defaults and ids.
    #[test]
    fn fd_33_detected_schema_has_eight_samples_of_four_parameters() {
        let e = det();
        assert_eq!(e.descriptor().id, "effect.adjust.detected_colors");
        e.params().validate_definition().unwrap();
        assert_eq!(e.params().params().len(), 32);
        for n in 0..8 {
            let at = |i: usize| &e.params().params()[4 * n + i];
            assert_eq!(at(0).id, format!("color_{}", n + 1));
            assert_eq!(at(1).id, format!("hue_{}", n + 1));
            assert_eq!(at(2).id, format!("saturation_{}", n + 1));
            assert_eq!(at(3).id, format!("tolerance_{}", n + 1));
            assert!(
                matches!(at(0).kind, ParamKind::Color { default } if default == DEFAULT_SAMPLE)
            );
            assert_eq!(
                at(1).kind,
                ParamKind::Float {
                    min: -180.0,
                    max: 180.0,
                    default: 0.0
                }
            );
            assert_eq!(
                at(2).kind,
                ParamKind::Float {
                    min: -100.0,
                    max: 100.0,
                    default: 0.0
                }
            );
            assert_eq!(
                at(3).kind,
                ParamKind::Float {
                    min: 0.0,
                    max: 100.0,
                    default: 15.0
                }
            );
        }
        assert_eq!(DETECTED_SLOTS, 8);
        assert!(e.gpu().is_none());
    }

    // FD-33 (a)
    #[test]
    fn fd_33_a_detected_defaults_are_identity() {
        assert_defaults_are_identity(&det());
    }

    // FD-33: a colour on the sample moves in full: Tono and Saturación as in Color.
    #[test]
    fn fd_33_detected_the_sample_colour_moves_like_color() {
        let mut p = ParamValues::new();
        slot(&mut p, 0, SAMPLE, 70.0, -40.0, 15.0);
        let got = one(SAMPLE, &p);
        let want = color_with(SAMPLE, 70.0, -40.0);
        for k in 0..3 {
            assert!((got[k] - want[k]).abs() < 1e-6, "{got:?} vs {want:?}");
        }
        assert_ne!(got, SAMPLE);
        // Hue alone and saturation alone.
        let mut p = ParamValues::new();
        slot(&mut p, 0, SAMPLE, 0.0, 60.0, 15.0);
        assert_eq!(one(SAMPLE, &p), color_with(SAMPLE, 0.0, 60.0));
        let mut p = ParamValues::new();
        slot(&mut p, 0, SAMPLE, -120.0, 0.0, 15.0);
        assert_eq!(one(SAMPLE, &p), color_with(SAMPLE, -120.0, 0.0));
    }

    // FD-33: weight 1 - w(tau / 2, tau, dE2000); the weight scales Tono and Saturación.
    #[test]
    fn fd_33_detected_weight_follows_delta_e_and_tolerance() {
        let (hue, sat) = (90.0, 80.0);
        for tol in [8.0, 20.0, 45.0] {
            let mut p = ParamValues::new();
            slot(&mut p, 0, SAMPLE, hue, sat, tol);
            let (mut full, mut part, mut none) = (0, 0, 0);
            for i in 0..=40 {
                let c = mix(i as f32 / 40.0);
                let d = delta_e2000(lab(c), lab(SAMPLE));
                let w = 1.0 - smooth(tol / 2.0, tol, d);
                match w {
                    w if w >= 1.0 => full += 1,
                    w if w <= 0.0 => none += 1,
                    _ => part += 1,
                }
                let got = one(c, &p);
                let want = if w <= 0.0 {
                    c
                } else {
                    color_with(c, hue * w, sat * w)
                };
                for k in 0..3 {
                    assert!(
                        (got[k] - want[k]).abs() < 1e-5,
                        "tol {tol} t {i}/40 dE {d} w {w}: {got:?} vs {want:?}"
                    );
                }
                if w <= 0.0 {
                    assert_eq!(
                        got.map(f32::to_bits),
                        c.map(f32::to_bits),
                        "outside is untouched"
                    );
                }
            }
            assert!(
                full > 0 && part > 2 && none > 0,
                "tol {tol}: {full} {part} {none}"
            );
        }
    }

    // FD-33: tolerance 0 selects nothing (no NaN from the empty smoothstep); more tolerance
    // reaches more colours.
    #[test]
    fn fd_33_detected_tolerance_zero_selects_nothing_and_more_reaches_further() {
        let mut p = ParamValues::new();
        slot(&mut p, 0, SAMPLE, 90.0, 90.0, 0.0);
        for t in [0.0, 0.1, 0.5, 1.0] {
            let c = mix(t);
            assert_eq!(one(c, &p).map(f32::to_bits), c.map(f32::to_bits), "t {t}");
        }
        let changed = |tol: f64| {
            let mut p = ParamValues::new();
            slot(&mut p, 0, SAMPLE, 90.0, 90.0, tol);
            (0..=40)
                .filter(|i| one(mix(*i as f32 / 40.0), &p) != mix(*i as f32 / 40.0))
                .count()
        };
        let counts: Vec<usize> = [0.0, 5.0, 10.0, 30.0, 100.0].map(changed).to_vec();
        assert_eq!(counts[0], 0);
        assert!(counts.windows(2).all(|w| w[0] <= w[1]), "{counts:?}");
        assert!(counts[4] > counts[1], "{counts:?}");
    }

    // FD-33: up to eight samples, each its own: a colour near one is moved by that one only.
    #[test]
    fn fd_33_detected_eight_samples_each_move_their_own_colours() {
        let colours: Vec<[f32; 3]> = vec![
            [0.9, 0.05, 0.05],
            [0.05, 0.6, 0.05],
            [0.05, 0.05, 0.8],
            [0.9, 0.7, 0.05],
            [0.7, 0.05, 0.6],
            [0.05, 0.6, 0.6],
            [0.9, 0.35, 0.05],
            [0.35, 0.2, 0.5],
        ];
        let mut p = ParamValues::new();
        for (n, c) in colours.iter().enumerate() {
            slot(
                &mut p,
                n,
                *c,
                20.0 + 15.0 * n as f64,
                10.0 * n as f64 - 30.0,
                6.0,
            );
        }
        for (n, c) in colours.iter().enumerate() {
            let near_others = colours
                .iter()
                .enumerate()
                .any(|(m, o)| m != n && delta_e2000(lab(*c), lab(*o)) < 12.0);
            assert!(!near_others, "test colours must be apart");
            let got = one(*c, &p);
            let want = color_with(*c, 20.0 + 15.0 * n as f64, 10.0 * n as f64 - 30.0);
            for k in 0..3 {
                assert!(
                    (got[k] - want[k]).abs() < 1e-5,
                    "sample {n}: {got:?} vs {want:?}"
                );
            }
        }
    }

    // FD-33: a colour inside two tolerances is moved by both, the weights from the original.
    #[test]
    fn fd_33_detected_overlapping_samples_apply_in_order() {
        let mid = mix(0.1);
        let mut p = ParamValues::new();
        slot(&mut p, 0, SAMPLE, 40.0, 0.0, 60.0);
        slot(&mut p, 1, mid, 0.0, 50.0, 60.0);
        let w0 = 1.0 - smooth(30.0, 60.0, delta_e2000(lab(mid), lab(SAMPLE)));
        let w1 = 1.0 - smooth(30.0, 60.0, delta_e2000(lab(mid), lab(mid)));
        assert!(w0 > 0.0 && w1 == 1.0);
        let got = one(mid, &p);
        // Hue first, then saturation, both through Color.
        let step1 = color_with(mid, 40.0 * w0, 0.0);
        let want = color_with(step1, 0.0, 50.0);
        for k in 0..3 {
            assert!((got[k] - want[k]).abs() < 1e-5, "{got:?} vs {want:?}");
        }
    }

    // FD-33, 8 bits: the sample colour gives the bytes Color gives.
    #[test]
    fn fd_33_detected_8_bit_pixels_match_color() {
        let mut p = ParamValues::new();
        let sample8 = Color::Srgb(Rgba::new(200.0 / 255.0, 90.0 / 255.0, 60.0 / 255.0, 1.0));
        p.set(COLOR[2], ParamValue::Color(sample8));
        p.set(HUE[2], ParamValue::Float(100.0));
        p.set(SATURATION[2], ParamValue::Float(-30.0));
        let px = [200u8, 90, 60, 255];
        let got = run8(&det(), &cx(1.0), &p, &px, (1, 1)).unwrap();
        let want = run8(
            &ColorAdjust::new(),
            &cx(1.0),
            &pv(&[("hue", 100.0), ("saturation", -30.0)]),
            &px,
            (1, 1),
        )
        .unwrap();
        assert_eq!(got, want);
        assert_ne!(got, px);
        // Far from the sample the bytes stay.
        let far = [20u8, 60, 220, 255];
        assert_eq!(run8(&det(), &cx(1.0), &p, &far, (1, 1)).unwrap(), far);
    }

    // FD-33 (f)
    #[test]
    fn fd_33_f_detected_keeps_alpha() {
        let mut on = ParamValues::new();
        slot(&mut on, 0, [0.5, 0.5, 0.5], 90.0, 100.0, 100.0);
        assert_alpha_is_kept(&det(), 1.0, &[on]);
    }

    // FD-33 (h), ARQ-110
    #[test]
    fn fd_33_h_detected_bad_parameters_are_err_and_dst_is_untouched() {
        assert_bad_parameters_are_err_and_ends_pass(&det());
        for (value, want) in [
            (
                ParamValue::Color(Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0))),
                ParamError::NonFinite { param: "color_3" },
            ),
            (
                ParamValue::Float(1.0),
                ParamError::WrongType { param: "color_3" },
            ),
        ] {
            let mut p = ParamValues::new();
            p.set("color_3", value);
            let (r, dst) = run8_keep(&det(), &cx(1.0), &p, &img_u8(2, 2, 1), (2, 2));
            assert_eq!(r, Err(EffectError::Param(want)));
            assert!(dst.iter().all(|b| *b == SENTINEL));
        }
    }

    #[test]
    fn arq_110_detected_non_finite_scale_and_pixels_are_err() {
        assert_non_finite_scale_and_pixels_are_err(&det());
    }

    // ARQ-110: a result that overflows `f32` is Err before anything is written.
    #[test]
    fn arq_110_detected_overflowing_result_is_err_and_dst_is_untouched() {
        let mut p = ParamValues::new();
        slot(&mut p, 0, [1.0, 0.0, 0.0], 0.0, 100.0, 100.0);
        // The first pixel is on the sample; saturation 2 scales a channel near f32::MAX by 1.787.
        let src = [1.0f32, 0.0, 0.0, 1.0, 3.0e38, 0.0, 0.0, 1.0];
        let (r, dst) = runf_keep(&det(), &cx(1.0), &p, &src, (2, 1));
        assert!(matches!(r, Err(EffectError::Buffer(_))), "{r:?}");
        assert!(dst.iter().all(|b| b.to_bits() == SENTINEL_F32));
    }

    #[test]
    fn detected_rejects_other_formats_and_sizes() {
        assert_format_and_size_errors(&det());
    }

    #[test]
    fn detected_empty_images_are_ok() {
        assert_empty_images_are_ok(&det());
    }

    // ARQ-111
    #[test]
    fn arq_111_detected_is_deterministic() {
        let mut p = ParamValues::new();
        slot(&mut p, 0, [0.4, 0.4, 0.4], 50.0, 50.0, 40.0);
        slot(&mut p, 5, [0.1, 0.2, 0.7], -50.0, 20.0, 30.0);
        assert_deterministic(&det(), &p);
    }

    // ARQ-120: a pointwise effect needs exactly the region it writes. Passes from the start.
    #[test]
    fn detected_roi_is_the_output_region() {
        let out = IRect::new(-3, 4, 90, 40);
        for scale in [0.5f32, 1.0, 2.0] {
            assert_eq!(det().roi(&cx(scale), &ParamValues::new(), out), out);
        }
    }

    // Passes from the start; proved by a mutant on the cost.
    #[test]
    fn detected_cost_grows_with_pixels() {
        let e = det();
        let c = |px| e.cost_hint(&cx(1.0), &ParamValues::new(), px).units;
        assert_eq!(c(0), 0);
        assert!(c(1000) > 0 && c(2000) == 2 * c(1000));
    }
}
