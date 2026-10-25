//! Property tests: curve and gamut round trips, symmetry, composition and serde round trips.

use gafira_color::{
    AnalyticTransform, Color, ColorSpaceId, ColorTransform, DisplayP3Rgba, LinearRgba, PixelLayout,
    Rgba,
};
use proptest::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Any finite `f32`, including `-0.0` and subnormals.
fn finite() -> impl Strategy<Value = f32> {
    use proptest::num::f32::{NEGATIVE, NORMAL, POSITIVE, SUBNORMAL, ZERO};
    POSITIVE | NEGATIVE | NORMAL | SUBNORMAL | ZERO
}

/// Any bit pattern, including NaN payloads and infinities.
fn any_bits() -> impl Strategy<Value = f32> {
    any::<u32>().prop_map(f32::from_bits)
}

/// A channel in `[0, 1]` that also lands on zero, near zero and near both curve thresholds.
fn unit() -> impl Strategy<Value = f32> {
    prop_oneof![
        6 => 0.0_f32..=1.0,
        2 => 0.0_f32..=1e-3,
        1 => proptest::num::f32::SUBNORMAL.prop_map(f32::abs),
        1 => Just(0.0_f32),
        1 => Just(-0.0_f32),
        1 => 0.04045_f32 - 1e-3..=0.04045 + 1e-3,
        1 => 0.0031308_f32 - 1e-4..=0.0031308 + 1e-4,
    ]
}

/// A channel in `[-4, 4]` that also lands on both zeros, near zero and near both thresholds.
fn extended() -> impl Strategy<Value = f32> {
    prop_oneof![
        6 => -4.0_f32..=4.0,
        2 => -1e-3_f32..=1e-3,
        1 => proptest::num::f32::POSITIVE | proptest::num::f32::NEGATIVE | proptest::num::f32::SUBNORMAL,
        1 => Just(0.0_f32),
        1 => Just(-0.0_f32),
        1 => (0.04045_f32 - 1e-3..=0.04045 + 1e-3, any::<bool>())
            .prop_map(|(v, negative)| if negative { -v } else { v }),
        1 => (0.0031308_f32 - 1e-4..=0.0031308 + 1e-4, any::<bool>())
            .prop_map(|(v, negative)| if negative { -v } else { v }),
    ]
}

fn space() -> impl Strategy<Value = ColorSpaceId> {
    proptest::sample::select(ColorSpaceId::ALL.to_vec())
}

fn color(space: ColorSpaceId, [r, g, b, a]: [f32; 4]) -> Color {
    match space {
        ColorSpaceId::Srgb => Color::Srgb(Rgba::new(r, g, b, a)),
        ColorSpaceId::LinearSrgb => Color::LinearSrgb(LinearRgba::new(r, g, b, a)),
        ColorSpaceId::DisplayP3 => Color::DisplayP3(DisplayP3Rgba::new(r, g, b, a)),
    }
}

fn channels(c: Color) -> [f32; 4] {
    match c {
        Color::Srgb(v) => [v.r, v.g, v.b, v.a],
        Color::LinearSrgb(v) => [v.r, v.g, v.b, v.a],
        Color::DisplayP3(v) => [v.r, v.g, v.b, v.a],
    }
}

fn bits4(v: [f32; 4]) -> [u32; 4] {
    v.map(f32::to_bits)
}

/// `|actual - expected| <= tol * max(1, |expected|)`, evaluated in `f64`.
fn close(actual: f32, expected: f32, tol: f64) -> bool {
    let expected = f64::from(expected);
    (f64::from(actual) - expected).abs() <= tol * expected.abs().max(1.0)
}

fn assert_colors_close(actual: Color, expected: Color, tol: f64) -> Result<(), TestCaseError> {
    let (a, e) = (channels(actual), channels(expected));
    for i in 0..3 {
        prop_assert!(
            close(a[i], e[i], tol),
            "channel {i}: {actual:?} vs {expected:?}"
        );
    }
    Ok(())
}

fn round_trip<T: Serialize + DeserializeOwned>(value: &T) -> T {
    let json = serde_json::to_string(value).expect("finite values serialize");
    serde_json::from_str(&json).expect("serialized value deserializes")
}

fn decode(x: f32) -> f32 {
    Rgba::new(x, x, x, 1.0).to_linear().r
}

fn encode(x: f32) -> f32 {
    LinearRgba::new(x, x, x, 1.0).to_srgb().r
}

/// The published encode constants leave a genuine downward step of about 2.85e-8
/// just above 0.0031308, so strict monotonicity of `encode` cannot hold there.
const ENCODE_STEP: f32 = 5e-8;

proptest! {
    // P1: sRGB -> linear -> sRGB.
    #[test]
    fn srgb_linear_srgb_round_trip_in_unit_range(r in unit(), g in unit(), b in unit()) {
        let back = Rgba::new(r, g, b, 1.0).to_linear().to_srgb();
        for (got, want) in [(back.r, r), (back.g, g), (back.b, b)] {
            prop_assert!(close(got, want, 2e-6), "{got} vs {want}");
        }
    }

    #[test]
    fn srgb_linear_srgb_round_trip_in_extended_range(r in extended(), g in extended(), b in extended()) {
        let back = Rgba::new(r, g, b, 1.0).to_linear().to_srgb();
        for (got, want) in [(back.r, r), (back.g, g), (back.b, b)] {
            prop_assert!(close(got, want, 2e-6), "{got} vs {want}");
        }
    }

    // P2: linear -> sRGB -> linear.
    #[test]
    fn linear_srgb_linear_round_trip_in_unit_range(r in unit(), g in unit(), b in unit()) {
        let back = LinearRgba::new(r, g, b, 1.0).to_srgb().to_linear();
        for (got, want) in [(back.r, r), (back.g, g), (back.b, b)] {
            prop_assert!(close(got, want, 2e-6), "{got} vs {want}");
        }
    }

    #[test]
    fn linear_srgb_linear_round_trip_in_extended_range(r in extended(), g in extended(), b in extended()) {
        let back = LinearRgba::new(r, g, b, 1.0).to_srgb().to_linear();
        for (got, want) in [(back.r, r), (back.g, g), (back.b, b)] {
            prop_assert!(close(got, want, 2e-6), "{got} vs {want}");
        }
    }

    // P3: monotonic curves.
    #[test]
    fn decode_is_monotonic(x in extended(), y in extended()) {
        let (lo, hi) = if x <= y { (x, y) } else { (y, x) };
        prop_assert!(decode(lo) <= decode(hi), "{lo} {hi}");
    }

    #[test]
    fn encode_is_monotonic_up_to_the_published_step(x in extended(), y in extended()) {
        let (lo, hi) = if x <= y { (x, y) } else { (y, x) };
        prop_assert!(encode(lo) <= encode(hi) + ENCODE_STEP, "{lo} {hi}");
    }

    // P6: the buffer API is bit-identical to the per-pixel typed API.
    #[test]
    fn apply_f32_agrees_with_the_typed_api(
        from in space(), to in space(), rgba_layout in any::<bool>(),
        pixels in proptest::collection::vec([any_bits(), any_bits(), any_bits(), any_bits()], 0..8),
    ) {
        let layout = if rgba_layout { PixelLayout::Rgba } else { PixelLayout::Rgb };
        let mut buffer: Vec<f32> = pixels.iter().flat_map(|p| p[..layout.channels()].to_vec()).collect();
        prop_assert_eq!(AnalyticTransform::new(from, to).apply_f32(&mut buffer, layout), Ok(()));
        for (got, p) in buffer.chunks_exact(layout.channels()).zip(&pixels) {
            let want = channels(color(from, *p).convert(to));
            prop_assert_eq!(bits4(std::array::from_fn(|i| *got.get(i).unwrap_or(&p[i]))), bits4([want[0], want[1], want[2], if rgba_layout { want[3] } else { p[3] }]));
        }
    }

    // P4: alpha is bit-preserved by every conversion.
    #[test]
    fn alpha_survives_every_conversion(
        from in space(), to in space(), r in extended(), g in extended(), b in extended(), a in any_bits(),
    ) {
        let out = color(from, [r, g, b, a]).convert(to);
        prop_assert_eq!(out.alpha().to_bits(), a.to_bits());
        let rgba = Rgba::new(r, g, b, a);
        prop_assert_eq!(rgba.to_linear().a.to_bits(), a.to_bits());
        let linear = LinearRgba::new(r, g, b, a);
        prop_assert_eq!(linear.to_srgb().a.to_bits(), a.to_bits());
        prop_assert_eq!(linear.to_display_p3().a.to_bits(), a.to_bits());
        prop_assert_eq!(DisplayP3Rgba::new(r, g, b, a).to_linear_srgb().a.to_bits(), a.to_bits());
    }

    // P5: odd symmetry, bit for bit.
    #[test]
    fn decode_is_odd_symmetric(x in finite()) {
        prop_assert_eq!(decode(-x).to_bits(), (-decode(x)).to_bits());
    }

    #[test]
    fn encode_is_odd_symmetric(x in finite()) {
        prop_assert_eq!(encode(-x).to_bits(), (-encode(x)).to_bits());
    }

    // Round trips through Display-P3.
    #[test]
    fn display_p3_round_trip_in_unit_range(from in space(), r in unit(), g in unit(), b in unit()) {
        let original = color(from, [r, g, b, 1.0]);
        let back = original.convert(ColorSpaceId::DisplayP3).convert(from);
        assert_colors_close(back, original, 1e-5)?;
    }

    #[test]
    fn display_p3_round_trip_in_extended_range(from in space(), r in extended(), g in extended(), b in extended()) {
        let original = color(from, [r, g, b, 1.0]);
        let back = original.convert(ColorSpaceId::DisplayP3).convert(from);
        assert_colors_close(back, original, 1e-4)?;
    }

    // P8: composition, for every ordered triple.
    #[test]
    fn composition_in_unit_range(a in space(), b in space(), c in space(), r in unit(), g in unit(), bl in unit()) {
        let start = color(a, [r, g, bl, 1.0]);
        assert_colors_close(start.convert(b).convert(c), start.convert(c), 1e-5)?;
    }

    #[test]
    fn composition_in_extended_range(a in space(), b in space(), c in space(), r in extended(), g in extended(), bl in extended()) {
        let start = color(a, [r, g, bl, 1.0]);
        assert_colors_close(start.convert(b).convert(c), start.convert(c), 1e-4)?;
    }

    // Same-space conversion and non-finite inputs.
    #[test]
    fn same_space_conversion_is_bit_identical(s in space(), r in any_bits(), g in any_bits(), b in any_bits(), a in any_bits()) {
        let original = color(s, [r, g, b, a]);
        prop_assert_eq!(bits4(channels(original.convert(s))), bits4([r, g, b, a]));
    }

    #[test]
    fn non_finite_channels_never_panic(
        from in space(), to in space(), r in any_bits(), g in any_bits(), b in any_bits(), a in any_bits(),
    ) {
        let original = color(from, [r, g, b, a]);
        let _ = serde_json::to_string(&original);
        let out = original.convert(to);
        prop_assert_eq!(out.alpha().to_bits(), a.to_bits());
        let _ = original.to_linear_srgb();
    }

    // P7: serde round trips are bit-identical for every finite f32.
    #[test]
    fn channel_types_json_round_trip_is_bit_identical(r in finite(), g in finite(), b in finite(), a in finite()) {
        let want = bits4([r, g, b, a]);
        let srgb = round_trip(&Rgba::new(r, g, b, a));
        prop_assert_eq!(bits4([srgb.r, srgb.g, srgb.b, srgb.a]), want);
        let linear = round_trip(&LinearRgba::new(r, g, b, a));
        prop_assert_eq!(bits4([linear.r, linear.g, linear.b, linear.a]), want);
        let p3 = round_trip(&DisplayP3Rgba::new(r, g, b, a));
        prop_assert_eq!(bits4([p3.r, p3.g, p3.b, p3.a]), want);
    }

    #[test]
    fn color_json_round_trip_is_bit_identical(s in space(), r in finite(), g in finite(), b in finite(), a in finite()) {
        let original = color(s, [r, g, b, a]);
        let back = round_trip(&original);
        prop_assert_eq!(back.space(), s);
        prop_assert_eq!(bits4(channels(back)), bits4([r, g, b, a]));
    }

    #[test]
    fn color_space_id_json_round_trip(s in space()) {
        prop_assert_eq!(round_trip(&s), s);
    }
}

#[test]
fn strategies_reach_negative_zero_and_subnormals() {
    // Guards the generators: without them the bit-identity claims would be vacuous.
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    fn reaches<S: Strategy<Value = f32>>(strategy: S) -> (bool, bool, bool) {
        let mut runner = TestRunner::deterministic();
        let (mut neg_zero, mut subnormal, mut negative_subnormal) = (false, false, false);
        for _ in 0..20_000 {
            let v = strategy.new_tree(&mut runner).unwrap().current();
            assert!(v.is_finite());
            neg_zero |= v.to_bits() == (-0.0_f32).to_bits();
            subnormal |= v != 0.0 && !v.is_normal();
            negative_subnormal |= v < 0.0 && !v.is_normal();
        }
        (neg_zero, subnormal, negative_subnormal)
    }

    let (neg_zero, subnormal, _) = reaches(finite());
    assert!(neg_zero, "finite(): -0.0 never generated");
    assert!(subnormal, "finite(): subnormal never generated");

    let (neg_zero, subnormal, negative_subnormal) = reaches(extended());
    assert!(neg_zero, "extended(): -0.0 never generated");
    assert!(subnormal, "extended(): subnormal never generated");
    assert!(
        negative_subnormal,
        "extended(): negative subnormal never generated"
    );

    let (neg_zero, subnormal, negative_subnormal) = reaches(unit());
    assert!(neg_zero, "unit(): -0.0 never generated");
    assert!(subnormal, "unit(): subnormal never generated");
    assert!(!negative_subnormal, "unit(): must stay inside [0, 1]");
}

#[test]
fn near_threshold_samples_are_generated() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let strategy = unit();
    let (mut near_decode, mut near_encode, mut near_zero) = (false, false, false);
    for _ in 0..20_000 {
        let v = strategy.new_tree(&mut runner).unwrap().current();
        assert!((0.0..=1.0).contains(&v), "{v}");
        near_decode |= (v - 0.04045).abs() < 1e-5;
        near_encode |= (v - 0.0031308).abs() < 1e-6;
        near_zero |= v != 0.0 && v < 1e-9;
    }
    assert!(
        near_decode && near_encode && near_zero,
        "{near_decode} {near_encode} {near_zero}"
    );
}

#[test]
fn encode_really_has_the_published_downward_step() {
    // Keeps ENCODE_STEP honest: the slack is needed, and it is not larger than the gap.
    let mut x = 0.0031308_f32 - 1e-8;
    let mut worst = 0.0_f32;
    let mut previous = encode(x);
    for _ in 0..4_000 {
        x = x.next_up();
        let current = encode(x);
        worst = worst.max(previous - current);
        previous = current;
    }
    assert!(
        worst > 0.0,
        "no downward step found: the slack would be vacuous"
    );
    assert!(
        worst < ENCODE_STEP,
        "step {worst} exceeds the documented slack"
    );
}
