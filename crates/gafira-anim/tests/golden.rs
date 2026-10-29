//! ARQ-12 cross-platform golden: `tests/golden/sample.json` holds tracks of the
//! four §3.4 types, the sampled instants `t` (flicks) and the expected results
//! stored losslessly as IEEE bit patterns. `f32`, `Vec2` and the easing curves
//! must match bit for bit. `Color` and `Transform2D` match within the §3.4
//! tolerances at interpolated instants, and bit for bit at key instants, outside
//! the key range, under `Hold` and for static tracks (no interpolation runs).
//!
//! Regenerating: this crate is a pure domain crate (ARQ-02), so a test cannot
//! write files or read the environment. The switch is therefore read at compile
//! time (`option_env!`) and the new file is printed, not written. It is only
//! printed when `GAFIRA_BLESS_GOLDEN=1` is set; extract it with:
//!
//! ```text
//! GAFIRA_BLESS_GOLDEN=1 cargo test -p gafira-anim --test golden \
//!   arq_12_golden_print_regenerated -- --nocapture 2>/dev/null \
//!   | sed -n '/^-----BEGIN GOLDEN-----$/,/^-----END GOLDEN-----$/{//!p}' \
//!   > crates/gafira-anim/tests/golden/sample.json
//! ```
//!
//! Never regenerate to silence a failure on another platform: a bit mismatch
//! there is the finding. While CI is disabled (README follow-up 2) this test only
//! runs locally on Linux.

use std::fmt::Write as _;

use gafira_anim::{Animatable, Easing, Interpolate, KeyTrack, Keyframe};
use gafira_color::{Color, ColorSpaceId, DisplayP3Rgba, LinearRgba, Rgba};
use gafira_geom::{Transform2D, Vec2};
use gafira_time::Flicks;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const GOLDEN_TEXT: &str = include_str!("golden/sample.json");
const BLESS: Option<&str> = option_env!("GAFIRA_BLESS_GOLDEN");
const EPS_F32: f64 = 1.0 / 8_388_608.0; // 2^-23

// ---- comparison rules (§3.4) -------------------------------------------

/// The regenerate switch is opt-in: only the exact value `1` turns it on.
fn bless_requested(value: Option<&str>) -> bool {
    value == Some("1")
}

fn bits_equal(got: &[u64], want: &[u64]) -> bool {
    got == want
}

/// Layout: `[space, r, g, b, a]` with the f32 bit patterns widened to u64.
fn color_within(got: &[u64], want: &[u64]) -> bool {
    if got.len() != 5 || want.len() != 5 || got[0] != want[0] || got[4] != want[4] {
        return false;
    }
    (1..4).all(|i| {
        let g = f64::from(f32::from_bits(got[i] as u32));
        let w = f64::from(f32::from_bits(want[i] as u32));
        (g - w).abs() <= 4.0 * EPS_F32 * w.abs().max(1.0)
    })
}

fn transform_within_tol(got: &[u64], want: &[u64], rel: f64) -> bool {
    got.len() == 6
        && want.len() == 6
        && got.iter().zip(want).all(|(g, w)| {
            let (g, w) = (f64::from_bits(*g), f64::from_bits(*w));
            (g - w).abs() <= rel * w.abs().max(1.0)
        })
}

fn transform_within(got: &[u64], want: &[u64]) -> bool {
    transform_within_tol(got, want, 1e-9)
}

// ---- per-type encoding -------------------------------------------------

trait Golden: Interpolate + Serialize + DeserializeOwned {
    const NAME: &'static str;
    fn encode(&self) -> Vec<u64>;
    /// `exact`: the instant involves no interpolation, so the bits must agree.
    fn matches(got: &[u64], want: &[u64], exact: bool) -> bool;
}

impl Golden for f32 {
    const NAME: &'static str = "f32";
    fn encode(&self) -> Vec<u64> {
        vec![u64::from(self.to_bits())]
    }
    fn matches(got: &[u64], want: &[u64], _exact: bool) -> bool {
        bits_equal(got, want)
    }
}

impl Golden for Vec2 {
    const NAME: &'static str = "Vec2";
    fn encode(&self) -> Vec<u64> {
        vec![self.x.to_bits(), self.y.to_bits()]
    }
    fn matches(got: &[u64], want: &[u64], _exact: bool) -> bool {
        bits_equal(got, want)
    }
}

impl Golden for Color {
    const NAME: &'static str = "Color";
    fn encode(&self) -> Vec<u64> {
        let (space, c) = match *self {
            Color::Srgb(v) => (0, [v.r, v.g, v.b, v.a]),
            Color::LinearSrgb(v) => (1, [v.r, v.g, v.b, v.a]),
            Color::DisplayP3(v) => (2, [v.r, v.g, v.b, v.a]),
        };
        let mut out = vec![space];
        out.extend(c.map(|x| u64::from(x.to_bits())));
        out
    }
    fn matches(got: &[u64], want: &[u64], exact: bool) -> bool {
        bits_equal(got, want) || (!exact && color_within(got, want))
    }
}

impl Golden for Transform2D {
    const NAME: &'static str = "Transform2D";
    fn encode(&self) -> Vec<u64> {
        self.coeffs().map(f64::to_bits).to_vec()
    }
    fn matches(got: &[u64], want: &[u64], exact: bool) -> bool {
        bits_equal(got, want) || (!exact && transform_within(got, want))
    }
}

// ---- file model --------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct Sample {
    t: i64,
    bits: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
struct Case<T: Interpolate> {
    name: String,
    track: Animatable<T>,
    samples: Vec<Sample>,
}

#[derive(Serialize, Deserialize)]
struct GoldenFile {
    f32: Vec<Case<f32>>,
    vec2: Vec<Case<Vec2>>,
    color: Vec<Case<Color>>,
    transform: Vec<Case<Transform2D>>,
}

fn load() -> GoldenFile {
    serde_json::from_str(GOLDEN_TEXT).expect("golden file parses")
}

/// One case per line so a review diff points at the case that changed.
fn render(file: &GoldenFile) -> String {
    fn group<T: Golden>(out: &mut String, key: &str, cases: &[Case<T>], last: bool) {
        writeln!(out, "  \"{key}\": [").unwrap();
        for (i, c) in cases.iter().enumerate() {
            let comma = if i + 1 < cases.len() { "," } else { "" };
            writeln!(out, "    {}{comma}", serde_json::to_string(c).unwrap()).unwrap();
        }
        writeln!(out, "  ]{}", if last { "" } else { "," }).unwrap();
    }
    let mut out = String::from("{\n");
    group(&mut out, "f32", &file.f32, false);
    group(&mut out, "vec2", &file.vec2, false);
    group(&mut out, "color", &file.color, false);
    group(&mut out, "transform", &file.transform, true);
    out.push_str("}\n");
    out
}

// ---- authored inputs (used only to regenerate the file) ----------------

fn fl(t: i64) -> Flicks {
    Flicks::from_ticks(t)
}

fn keyed<T: Interpolate>(keys: Vec<(i64, T, Easing)>) -> Animatable<T> {
    let keys = keys
        .into_iter()
        .map(|(at, value, out)| Keyframe {
            at: fl(at),
            value,
            out,
        })
        .collect();
    Animatable::Keyed(KeyTrack::new(keys).expect("valid authored track"))
}

/// Instants around every key (before, at, after), between keys, and the
/// extremes of the `i64` range.
fn instants<T: Interpolate>(track: &Animatable<T>) -> Vec<i64> {
    let mut ts = vec![i64::MIN, i64::MAX, 0];
    if let Animatable::Keyed(k) = track {
        let ats: Vec<i64> = k.keys().iter().map(|k| k.at.ticks()).collect();
        for w in ats.windows(2) {
            let span = i128::from(w[1]) - i128::from(w[0]);
            for q in [1, 2, 4, 8, 16, 128, 255] {
                let t = i128::from(w[0]) + span * q / 256;
                ts.push(i64::try_from(t).expect("between two i64 values"));
            }
        }
        for at in ats {
            ts.extend([
                at.saturating_sub(1_000_000),
                at.saturating_sub(1),
                at,
                at.saturating_add(1),
                at.saturating_add(1_000_000),
            ]);
        }
    }
    ts.sort_unstable();
    ts.dedup();
    ts
}

fn case<T: Golden>(name: &str, track: Animatable<T>) -> Case<T> {
    let samples = instants(&track)
        .into_iter()
        .map(|t| Sample {
            t,
            bits: track.sample(fl(t)).encode(),
        })
        .collect();
    Case {
        name: name.to_owned(),
        track,
        samples,
    }
}

const K: i64 = 1_000_000_000;

fn every_easing() -> [Easing; 6] {
    [
        Easing::Linear,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
        Easing::CubicBezier {
            x1: 0.17,
            y1: -0.6,
            x2: 0.83,
            y2: 1.9,
        },
        Easing::Hold,
    ]
}

fn srgb(c: [f32; 4]) -> Color {
    Rgba::new(c[0], c[1], c[2], c[3]).into()
}

fn lin(c: [f32; 4]) -> Color {
    LinearRgba::new(c[0], c[1], c[2], c[3]).into()
}

fn p3(c: [f32; 4]) -> Color {
    DisplayP3Rgba::new(c[0], c[1], c[2], c[3]).into()
}

fn tf(c: [f64; 6]) -> Transform2D {
    Transform2D::from(c)
}

/// Two-key track with the given curve on the first key; keys 1 s apart.
fn pair<T: Interpolate>(a: T, b: T, out: Easing) -> Animatable<T> {
    keyed(vec![(0, a, out), (K, b, Easing::Linear)])
}

fn authored() -> GoldenFile {
    let mut f32s = vec![case("static", Animatable::Static(1.5_f32))];
    let mut vec2s = vec![case("static", Animatable::Static(Vec2::new(1.5, -2.25)))];
    let mut colors = vec![case(
        "static",
        Animatable::Static(srgb([0.2, 0.4, 0.6, 1.0])),
    )];
    let mut transforms = vec![case(
        "static",
        Animatable::Static(tf([1.0, 0.0, 0.0, 1.0, 5.0, 6.0])),
    )];

    // Every easing variant on a 4-key track (each key uses one curve).
    for (i, e) in every_easing().into_iter().enumerate() {
        f32s.push(case(
            &format!("easing_{i}"),
            keyed(vec![
                (0, 0.0_f32, e),
                (K, 10.0, every_easing()[(i + 1) % 6]),
                (3 * K, -4.5, e),
                (4 * K, 100.25, Easing::Linear),
            ]),
        ));
        vec2s.push(case(
            &format!("easing_{i}"),
            keyed(vec![
                (K, Vec2::new(0.0, 1.0), e),
                (2 * K, Vec2::new(10.0, -3.5), every_easing()[(i + 2) % 6]),
                (5 * K, Vec2::new(-7.25, 0.125), Easing::Linear),
            ]),
        ));
    }
    f32s.push(case(
        "extreme_ats",
        keyed(vec![
            (i64::MIN, -1.0_f32, Easing::Linear),
            (0, 1.0, Easing::EaseInOut),
            (i64::MAX, 3.0, Easing::Hold),
        ]),
    ));

    // Colors: mixed spaces, premultiplied alpha, Q-31 (alpha <= 0), overshoot.
    for (i, e) in every_easing().into_iter().enumerate() {
        colors.push(case(
            &format!("easing_{i}"),
            keyed(vec![
                (0, srgb([1.0, 0.0, 0.0, 1.0]), e),
                (K, p3([0.1, 0.8, 0.3, 0.5]), every_easing()[(i + 3) % 6]),
                (2 * K, lin([0.5, 0.5, 0.5, 0.25]), Easing::Linear),
            ]),
        ));
    }
    colors.push(case(
        "premultiplied_alpha",
        pair(
            srgb([1.0, 0.0, 0.0, 1.0]),
            srgb([0.0, 0.0, 0.0, 0.0]),
            Easing::Linear,
        ),
    ));
    colors.push(case(
        "q31_zero_alpha_both",
        pair(
            srgb([1.0, 0.0, 0.0, 0.0]),
            srgb([0.0, 0.0, 1.0, 0.0]),
            Easing::Linear,
        ),
    ));
    colors.push(case(
        "q31_negative_alpha_extrapolation",
        pair(
            srgb([0.9, 0.2, 0.1, 0.5]),
            srgb([0.1, 0.7, 0.9, 0.0]),
            Easing::CubicBezier {
                x1: 0.25,
                y1: 1.5,
                x2: 0.75,
                y2: 1.5,
            },
        ),
    ));
    colors.push(case(
        "out_of_gamut_no_clamp",
        pair(
            p3([1.0, 0.0, 0.0, 1.0]),
            lin([0.0, 1.0, 0.0, 1.0]),
            Easing::EaseInOut,
        ),
    ));
    // Same-space blends in the two other spaces (the result keeps the space).
    colors.push(case(
        "same_space_display_p3",
        pair(
            p3([0.9, 0.2, 0.1, 1.0]),
            p3([0.1, 0.7, 0.9, 0.6]),
            Easing::Linear,
        ),
    ));
    colors.push(case(
        "same_space_linear_srgb",
        pair(
            lin([0.8, 0.1, 0.3, 0.5]),
            lin([0.2, 0.9, 0.6, 1.0]),
            Easing::EaseInOut,
        ),
    ));
    // Alpha 0.9 -> 0.1: at several instants the f32 formula and an f64 formula
    // narrowed to f32 round differently.
    colors.push(case(
        "alpha_f32_rounding",
        pair(
            srgb([0.3, 0.6, 0.9, 0.9]),
            srgb([0.7, 0.2, 0.4, 0.1]),
            Easing::Linear,
        ),
    ));

    // Transform2D: every easing, then the documented special cases.
    for (i, e) in every_easing().into_iter().enumerate() {
        transforms.push(case(
            &format!("easing_{i}"),
            keyed(vec![
                (0, tf([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]), e),
                (
                    K,
                    tf([0.0, 2.0, -2.0, 0.0, 10.0, -5.0]),
                    every_easing()[(i + 1) % 6],
                ),
                (2 * K, tf([1.5, 0.5, -0.25, 0.75, 1.0, 2.0]), Easing::Linear),
            ]),
        ));
    }
    let id = tf([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let rot = |deg: f64| Transform2D::rotation(deg.to_radians());
    transforms.push(case(
        "rotation_350_to_10",
        pair(rot(350.0), rot(10.0), Easing::Linear),
    ));
    transforms.push(case(
        "q28_degenerate_endpoint",
        pair(
            tf([0.0, 0.0, 0.0, 0.0, 1.0, 2.0]),
            tf([2.0, 0.0, 0.0, 3.0, 5.0, 6.0]),
            Easing::Linear,
        ),
    ));
    transforms.push(case(
        "q28_both_degenerate",
        pair(
            tf([1.0, 2.0, 2.0, 4.0, 1.0, 0.0]),
            tf([0.0, 0.0, 0.0, 0.0, 3.0, 4.0]),
            Easing::Linear,
        ),
    ));
    transforms.push(case(
        "q30_rank1_to_identity",
        pair(tf([1.0, 2.0, 2.0, 4.0, 0.0, 0.0]), id, Easing::Linear),
    ));
    transforms.push(case(
        "q33_rank1_rounding_det",
        pair(tf([0.1, 0.3, 0.7, 2.1, 0.0, 0.0]), id, Easing::Linear),
    ));
    transforms.push(case(
        "q33_det_zero_exact",
        pair(tf([1.1, 2.2, 3.3, 6.6, 0.0, 0.0]), id, Easing::Linear),
    ));
    transforms.push(case(
        "q33_huge_coefficients",
        pair(
            tf([1e200, 1e200, 1e200, 2e200, 0.0, 0.0]),
            id,
            Easing::Linear,
        ),
    ));
    let tiny = 1e-13_f64;
    transforms.push(case(
        "q33_relative_threshold_large",
        pair(
            tf([1e6, 0.0, 2e6 * tiny.cos(), 2e6 * tiny.sin(), 0.0, 0.0]),
            id,
            Easing::Linear,
        ),
    ));
    let small = 1e-7 * 0.3_f64.cos();
    let small_s = 1e-7 * 0.3_f64.sin();
    transforms.push(case(
        "q33_relative_threshold_small",
        pair(
            tf([small, small_s, -small_s, small, 0.0, 0.0]),
            rot(2.0_f64.to_degrees()),
            Easing::Linear,
        ),
    ));
    transforms.push(case(
        "skew",
        pair(id, tf([1.0, 0.0, 3.0, 1.0, 0.0, 0.0]), Easing::EaseInOut),
    ));

    GoldenFile {
        f32: f32s,
        vec2: vec2s,
        color: colors,
        transform: transforms,
    }
}

// ---- checks ------------------------------------------------------------

/// `true` when sampling at `t` runs no interpolation: static track, outside the
/// key range, at a key, or inside a segment whose `out` is `Hold`.
fn is_exact_instant<T: Interpolate>(track: &Animatable<T>, t: i64) -> bool {
    let Animatable::Keyed(k) = track else {
        return true;
    };
    let keys = k.keys();
    let (first, last) = (keys[0].at.ticks(), keys[keys.len() - 1].at.ticks());
    if t <= first || t >= last {
        return true;
    }
    keys.iter().any(|key| key.at.ticks() == t)
        || keys
            .windows(2)
            .any(|w| w[0].at.ticks() < t && t < w[1].at.ticks() && w[0].out == Easing::Hold)
}

fn check<T: Golden>(cases: &[Case<T>]) {
    assert!(!cases.is_empty(), "{}: no cases", T::NAME);
    for c in cases {
        for s in &c.samples {
            let got = c.track.sample(fl(s.t)).encode();
            assert!(
                T::matches(&got, &s.bits, is_exact_instant(&c.track, s.t)),
                "{} case {:?} at t={}: got {:?}, stored {:?}",
                T::NAME,
                c.name,
                s.t,
                got,
                s.bits
            );
        }
    }
}

#[test]
fn arq_12_golden_file_matches_sample_on_this_platform() {
    let file = load();
    check(&file.f32);
    check(&file.vec2);
    check(&file.color);
    check(&file.transform);
}

/// Prints the regenerated file; does nothing unless the switch is on.
#[test]
fn arq_12_golden_print_regenerated() {
    if bless_requested(BLESS) {
        println!(
            "-----BEGIN GOLDEN-----\n{}-----END GOLDEN-----",
            render(&authored())
        );
    }
}

type EasingTest = fn(&Easing) -> bool;

/// Per type: the named cases, every easing as a segment's `out`, Hold, Static,
/// the extreme instants, and samples just before and after every key.
fn covers<T: Interpolate>(kind: &str, cases: &[Case<T>], required: &[&str]) {
    for needed in required {
        assert!(
            cases.iter().any(|c| c.name == *needed),
            "{kind}: missing case {needed}"
        );
    }
    assert!(
        cases
            .iter()
            .any(|c| matches!(c.track, Animatable::Static(_))),
        "{kind}: no static track"
    );
    let ts: Vec<i64> = cases
        .iter()
        .flat_map(|c| c.samples.iter().map(|s| s.t))
        .collect();
    assert!(
        ts.contains(&i64::MIN) && ts.contains(&i64::MAX),
        "{kind}: extreme instants not sampled"
    );
    // The last key's `out` is never used (S4): only segment curves count.
    let mut segment_outs = Vec::new();
    for c in cases {
        let Animatable::Keyed(k) = &c.track else {
            continue;
        };
        let keys = k.keys();
        let sampled = |t: i64| c.samples.iter().any(|s| s.t == t);
        for key in keys {
            let at = key.at.ticks();
            for t in [at.saturating_sub(1), at, at.saturating_add(1)] {
                assert!(sampled(t), "{kind} {}: no sample at {t}", c.name);
            }
        }
        for w in keys.windows(2) {
            let (lo, hi) = (w[0].at.ticks(), w[1].at.ticks());
            assert!(
                c.samples.iter().any(|s| lo < s.t && s.t < hi),
                "{kind} {}: no sample inside the segment at {lo}",
                c.name
            );
            segment_outs.push(w[0].out);
        }
    }
    let variants: [(&str, EasingTest); 6] = [
        ("Linear", |e: &Easing| matches!(e, Easing::Linear)),
        ("EaseIn", |e: &Easing| matches!(e, Easing::EaseIn)),
        ("EaseOut", |e: &Easing| matches!(e, Easing::EaseOut)),
        ("EaseInOut", |e: &Easing| matches!(e, Easing::EaseInOut)),
        ("CubicBezier", |e: &Easing| {
            matches!(e, Easing::CubicBezier { .. })
        }),
        ("Hold", |e: &Easing| matches!(e, Easing::Hold)),
    ];
    for (label, hit) in variants {
        assert!(
            segment_outs.iter().any(hit),
            "{kind}: easing {label} is not used by any segment"
        );
    }
}

fn easing_cases() -> [&'static str; 7] {
    [
        "static", "easing_0", "easing_1", "easing_2", "easing_3", "easing_4", "easing_5",
    ]
}

#[test]
fn arq_12_golden_file_covers_the_required_cases() {
    let file = load();
    let with = |extra: &[&'static str]| {
        let mut v = easing_cases().to_vec();
        v.extend_from_slice(extra);
        v
    };
    covers("f32", &file.f32, &with(&["extreme_ats"]));
    covers("vec2", &file.vec2, &with(&[]));
    covers(
        "color",
        &file.color,
        &with(&[
            "premultiplied_alpha",
            "q31_zero_alpha_both",
            "q31_negative_alpha_extrapolation",
            "out_of_gamut_no_clamp",
            "same_space_display_p3",
            "same_space_linear_srgb",
            "alpha_f32_rounding",
        ]),
    );
    covers(
        "transform",
        &file.transform,
        &with(&[
            "rotation_350_to_10",
            "q28_degenerate_endpoint",
            "q28_both_degenerate",
            "q30_rank1_to_identity",
            "q33_rank1_rounding_det",
            "q33_det_zero_exact",
            "q33_huge_coefficients",
            "q33_relative_threshold_large",
            "q33_relative_threshold_small",
            "skew",
        ]),
    );
    // extreme_ats really has keys at both ends of the i64 range.
    let extreme = file.f32.iter().find(|c| c.name == "extreme_ats").unwrap();
    let Animatable::Keyed(k) = &extreme.track else {
        panic!("extreme_ats must be keyed");
    };
    let ats: Vec<i64> = k.keys().iter().map(|k| k.at.ticks()).collect();
    assert!(ats.contains(&i64::MIN) && ats.contains(&i64::MAX));
}

// ---- the rules themselves ----------------------------------------------

#[test]
fn arq_12_regenerate_switch_is_opt_in() {
    for (value, want) in [
        (None, false),
        (Some(""), false),
        (Some("0"), false),
        (Some("true"), false),
        (Some("1"), true),
    ] {
        assert_eq!(bless_requested(value), want, "{value:?}");
    }
}

#[test]
fn arq_12_f32_and_vec2_compare_bit_for_bit() {
    let one = 1.0_f32.to_bits();
    let next = f32::from_bits(one + 1).to_bits();
    assert!(f32::matches(&[u64::from(one)], &[u64::from(one)], false));
    assert!(!f32::matches(&[u64::from(one)], &[u64::from(next)], false));
    assert!(!f32::matches(
        &[u64::from(0.0_f32.to_bits())],
        &[u64::from((-0.0_f32).to_bits())],
        false
    ));
    let y = 2.0_f64.to_bits();
    assert!(Vec2::matches(
        &[1.0_f64.to_bits(), y],
        &[1.0_f64.to_bits(), y],
        false
    ));
    assert!(!Vec2::matches(
        &[1.0_f64.to_bits(), y],
        &[1.0_f64.to_bits(), y + 1],
        false
    ));
}

#[test]
fn arq_12_color_tolerance_is_4_eps_per_channel_and_alpha_is_exact() {
    let enc = |space: u64, c: [f32; 4]| {
        let mut v = vec![space];
        v.extend(c.map(|x| u64::from(x.to_bits())));
        v
    };
    let want = enc(0, [0.5, 2.0, 0.25, 0.5]);
    // max(1, |c|) scales the allowed error for |c| > 1: 2.0 allows 8 eps.
    let eps = EPS_F32 as f32;
    assert!(Color::matches(
        &enc(0, [0.5 + 3.0 * eps, 2.0 + 7.0 * eps, 0.25, 0.5]),
        &want,
        false
    ));
    assert!(!Color::matches(
        &enc(0, [0.5 + 5.0 * eps, 2.0, 0.25, 0.5]),
        &want,
        false
    ));
    assert!(!Color::matches(
        &enc(0, [0.5, 2.0 + 16.0 * eps, 0.25, 0.5]),
        &want,
        false
    ));
    // Alpha: one ULP is a failure.
    let a1 = f32::from_bits(0.5_f32.to_bits() + 1);
    assert!(!Color::matches(&enc(0, [0.5, 2.0, 0.25, a1]), &want, false));
    // Same values in another space are not the same color.
    assert!(!Color::matches(
        &enc(1, [0.5, 2.0, 0.25, 0.5]),
        &want,
        false
    ));
}

#[test]
fn arq_12_transform_tolerance_is_1e9_relative_to_max_1_and_c() {
    let enc = |c: [f64; 6]| c.map(f64::to_bits).to_vec();
    let want = enc([0.5, 0.0, 0.0, 1000.0, 0.0, 0.0]);
    assert!(Transform2D::matches(
        &enc([0.5 + 5e-10, 0.0, 0.0, 1000.0, 0.0, 0.0]),
        &want,
        false
    ));
    assert!(!Transform2D::matches(
        &enc([0.5 + 1e-8, 0.0, 0.0, 1000.0, 0.0, 0.0]),
        &want,
        false
    ));
    // Large coefficients scale the tolerance: 1e-9 * 1000 = 1e-6.
    assert!(Transform2D::matches(
        &enc([0.5, 0.0, 0.0, 1000.0 + 5e-7, 0.0, 0.0]),
        &want,
        false
    ));
    assert!(!Transform2D::matches(
        &enc([0.5, 0.0, 0.0, 1000.0 + 5e-6, 0.0, 0.0]),
        &want,
        false
    ));
    // A zero coefficient admits up to 1e-9.
    assert!(!Transform2D::matches(
        &enc([0.5, 2e-9, 0.0, 1000.0, 0.0, 0.0]),
        &want,
        false
    ));
}

#[test]
fn arq_12_color_and_transform_are_bit_exact_where_no_interpolation_runs() {
    let enc = |c: [f32; 4]| {
        let mut v = vec![0];
        v.extend(c.map(|x| u64::from(x.to_bits())));
        v
    };
    let want = enc([0.5, 0.25, 0.75, 1.0]);
    let one_ulp = enc([f32::from_bits(0.5_f32.to_bits() + 1), 0.25, 0.75, 1.0]);
    assert!(!Color::matches(&one_ulp, &want, true));
    assert!(Color::matches(&one_ulp, &want, false));
    assert!(Color::matches(&want, &want, true));
    let tw = [1.0_f64, 0.0, 0.0, 1.0, 5.0, 6.0]
        .map(f64::to_bits)
        .to_vec();
    let mut off = tw.clone();
    off[4] += 1;
    assert!(!Transform2D::matches(&off, &tw, true));
    assert!(Transform2D::matches(&off, &tw, false));
    assert!(Transform2D::matches(&tw, &tw, true));
}

#[test]
fn arq_12_exact_instants_are_static_outside_range_keys_and_hold() {
    let hold_then_linear = keyed(vec![
        (0, 0.0_f32, Easing::Hold),
        (10, 1.0, Easing::Linear),
        (20, 2.0, Easing::Linear),
    ]);
    for (t, want) in [
        (i64::MIN, true),
        (0, true),
        (5, true),   // inside the Hold segment
        (10, true),  // a key
        (15, false), // interpolated
        (20, true),
        (i64::MAX, true),
    ] {
        assert_eq!(is_exact_instant(&hold_then_linear, t), want, "t={t}");
    }
    assert!(is_exact_instant(&Animatable::Static(1.0_f32), 7));
}

#[test]
fn arq_12_color_encoding_gives_each_space_its_own_tag() {
    // Guards the encoding: each space must map to its own tag.
    let tags: Vec<u64> = [
        ColorSpaceId::Srgb,
        ColorSpaceId::LinearSrgb,
        ColorSpaceId::DisplayP3,
    ]
    .iter()
    .map(|s| {
        let c = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)).convert(*s);
        c.encode()[0]
    })
    .collect();
    assert_eq!(tags, vec![0, 1, 2]);
}
