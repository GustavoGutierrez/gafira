//! Public-API tests for frame rates, sample rates and their flicks conversions.

use gafira_time::{FLICKS_PER_SECOND, Flicks, FrameRate, SampleRate, TimeError};
use proptest::prelude::*;

const STANDARD_RATES: [(FrameRate, i64); 10] = [
    (FrameRate::FPS_23_976, 29_429_400),
    (FrameRate::FPS_24, 29_400_000),
    (FrameRate::FPS_25, 28_224_000),
    (FrameRate::FPS_29_97, 23_543_520),
    (FrameRate::FPS_30, 23_520_000),
    (FrameRate::FPS_48, 14_700_000),
    (FrameRate::FPS_50, 14_112_000),
    (FrameRate::FPS_59_94, 11_771_760),
    (FrameRate::FPS_60, 11_760_000),
    (FrameRate::FPS_120, 5_880_000),
];

#[test]
fn standard_frame_rates_are_accepted_with_exact_durations() {
    for (rate, expected) in STANDARD_RATES {
        assert_eq!(rate.flicks_per_frame().ticks(), expected, "{rate:?}");
    }
}

#[test]
fn standard_frame_rates_construct_from_their_fraction() {
    for (num, den) in [
        (24_000, 1_001),
        (24, 1),
        (25, 1),
        (30_000, 1_001),
        (30, 1),
        (48, 1),
        (50, 1),
        (60_000, 1_001),
        (60, 1),
        (120, 1),
    ] {
        assert!(FrameRate::new(num, den).is_ok(), "{num}/{den}");
    }
}

#[test]
fn ntsc_frame_durations_are_exact() {
    assert_eq!(FrameRate::FPS_23_976.flicks_per_frame().ticks(), 29_429_400);
    assert_eq!(FrameRate::FPS_29_97.flicks_per_frame().ticks(), 23_543_520);
    assert_eq!(FrameRate::FPS_59_94.flicks_per_frame().ticks(), 11_771_760);
}

#[test]
fn non_integral_frame_durations_are_rejected() {
    assert!(matches!(
        FrameRate::new(11, 1),
        Err(TimeError::FrameRateNotIntegral { .. })
    ));
    assert!(matches!(
        FrameRate::new(2398, 100),
        Err(TimeError::FrameRateNotIntegral { .. })
    ));
}

#[test]
fn zero_rates_are_rejected() {
    assert_eq!(FrameRate::new(1, 0), Err(TimeError::ZeroDenominator));
    assert_eq!(FrameRate::new(24, 0), Err(TimeError::ZeroDenominator));
    assert_eq!(FrameRate::new(0, 1), Err(TimeError::ZeroFrameRate));
    assert!(FrameRate::new(0, 0).is_err());
}

#[test]
fn rates_below_one_frame_per_second_are_rejected() {
    assert!(matches!(
        FrameRate::new(1, 2),
        Err(TimeError::FrameRateBelowOneFps { num: 1, den: 2 })
    ));
    assert!(matches!(
        FrameRate::new(24, 25),
        Err(TimeError::FrameRateBelowOneFps { .. })
    ));
}

#[test]
fn equivalent_fractions_normalize() {
    let rate = FrameRate::new(48, 2).unwrap();
    assert_eq!(rate, FrameRate::FPS_24);
    assert_eq!(rate.num(), 24);
    assert_eq!(rate.den(), 1);
    assert_eq!(
        rate.flicks_per_frame(),
        FrameRate::FPS_24.flicks_per_frame()
    );
    assert_eq!(FrameRate::new(60_000, 2_002).unwrap(), FrameRate::FPS_29_97);
}

#[test]
fn nominal_frame_rate_rounds_to_nearest_with_ties_up() {
    assert_eq!(FrameRate::FPS_23_976.nominal_fps(), 24);
    assert_eq!(FrameRate::FPS_29_97.nominal_fps(), 30);
    assert_eq!(FrameRate::FPS_59_94.nominal_fps(), 60);
    assert_eq!(FrameRate::new(49, 2).unwrap().nominal_fps(), 25);
    assert_eq!(FrameRate::FPS_24.nominal_fps(), 24);
}

#[test]
fn fps_f64_is_the_lossy_display_value() {
    let fps = FrameRate::FPS_29_97.fps_f64();
    assert!((fps - 29.970_029_970_029_97).abs() < 1e-12);
}

#[test]
fn frames_convert_to_flicks_exactly() {
    assert_eq!(
        Flicks::from_frames(5, FrameRate::FPS_24).unwrap().ticks(),
        147_000_000
    );
    assert_eq!(
        Flicks::from_frames(0, FrameRate::FPS_24).unwrap(),
        Flicks::ZERO
    );
    assert_eq!(
        Flicks::from_frames(-1, FrameRate::FPS_24).unwrap().ticks(),
        -29_400_000
    );
}

#[test]
fn frame_conversion_overflow_is_an_error() {
    assert_eq!(
        Flicks::from_frames(i64::MAX, FrameRate::FPS_24),
        Err(TimeError::Overflow)
    );
    assert_eq!(
        Flicks::from_frames(i64::MIN, FrameRate::FPS_24),
        Err(TimeError::Overflow)
    );
}

#[test]
fn instants_floor_to_the_containing_frame() {
    let d = 29_400_000;
    assert_eq!(
        Flicks::from_ticks(d * 5).to_frames_floor(FrameRate::FPS_24),
        5
    );
    assert_eq!(
        Flicks::from_ticks(d * 6 - 1).to_frames_floor(FrameRate::FPS_24),
        5
    );
    assert_eq!(
        Flicks::from_ticks(d * 6).to_frames_floor(FrameRate::FPS_24),
        6
    );
    assert_eq!(
        Flicks::from_ticks(-1).to_frames_floor(FrameRate::FPS_24),
        -1
    );
    assert_eq!(
        Flicks::from_ticks(-d).to_frames_floor(FrameRate::FPS_24),
        -1
    );
    assert_eq!(
        Flicks::from_ticks(-d - 1).to_frames_floor(FrameRate::FPS_24),
        -2
    );
}

#[test]
fn frame_boundaries_are_exact_multiples() {
    let d = 29_400_000;
    assert!(Flicks::ZERO.is_frame_boundary(FrameRate::FPS_24));
    assert!(Flicks::from_ticks(d).is_frame_boundary(FrameRate::FPS_24));
    assert!(Flicks::from_ticks(-d).is_frame_boundary(FrameRate::FPS_24));
    assert!(Flicks::from_ticks(d * 6).is_frame_boundary(FrameRate::FPS_24));
    assert!(!Flicks::from_ticks(d - 1).is_frame_boundary(FrameRate::FPS_24));
    assert!(!Flicks::from_ticks(d + 1).is_frame_boundary(FrameRate::FPS_24));
    assert!(!Flicks::from_ticks(1).is_frame_boundary(FrameRate::FPS_24));
}

#[test]
fn cross_rate_conversion_floors_instead_of_rounding() {
    // One 24 fps frame is 29,400,000 flicks; one 25 fps frame is 28,224,000.
    let instant = Flicks::from_ticks(29_400_000);
    assert_eq!(instant.to_frames_floor(FrameRate::FPS_25), 1);
    assert!(!instant.is_frame_boundary(FrameRate::FPS_25));
}

#[test]
fn standard_sample_rates_are_accepted_with_exact_durations() {
    assert_eq!(SampleRate::HZ_44_100.flicks_per_sample().ticks(), 16_000);
    assert_eq!(SampleRate::HZ_48_000.flicks_per_sample().ticks(), 14_700);
    assert_eq!(SampleRate::HZ_96_000.flicks_per_sample().ticks(), 7_350);
}

#[test]
fn sample_rates_are_validated() {
    assert_eq!(SampleRate::new(0), Err(TimeError::ZeroSampleRate));
    assert!(matches!(
        SampleRate::new(11),
        Err(TimeError::SampleRateNotIntegral { hz: 11 })
    ));
    assert!(matches!(
        SampleRate::new(44_101),
        Err(TimeError::SampleRateNotIntegral { .. })
    ));
    assert!(SampleRate::new(11_025).is_ok());
    assert!(SampleRate::new(192_000).is_ok());
    assert_eq!(SampleRate::new(48_000).unwrap().hz(), 48_000);
}

#[test]
fn samples_convert_to_flicks_exactly() {
    let d = 14_700;
    assert_eq!(
        Flicks::from_samples(1, SampleRate::HZ_48_000)
            .unwrap()
            .ticks(),
        d
    );
    assert_eq!(
        Flicks::from_samples(0, SampleRate::HZ_48_000).unwrap(),
        Flicks::ZERO
    );
    assert_eq!(
        Flicks::from_samples(-1, SampleRate::HZ_48_000)
            .unwrap()
            .ticks(),
        -d
    );
    assert_eq!(
        Flicks::from_samples(i64::MAX, SampleRate::HZ_48_000),
        Err(TimeError::Overflow)
    );
}

#[test]
fn instants_floor_to_the_containing_sample() {
    let d = 14_700;
    assert_eq!(
        Flicks::from_ticks(d - 1).to_samples_floor(SampleRate::HZ_48_000),
        0
    );
    assert_eq!(
        Flicks::from_ticks(d).to_samples_floor(SampleRate::HZ_48_000),
        1
    );
    assert_eq!(
        Flicks::from_ticks(-1).to_samples_floor(SampleRate::HZ_48_000),
        -1
    );
    assert_eq!(
        Flicks::from_ticks(-d).to_samples_floor(SampleRate::HZ_48_000),
        -1
    );
    assert_eq!(
        Flicks::from_ticks(-d - 1).to_samples_floor(SampleRate::HZ_48_000),
        -2
    );
}

#[test]
fn frame_rate_serde_uses_the_normalized_fraction() {
    assert_eq!(
        serde_json::to_string(&FrameRate::FPS_29_97).unwrap(),
        r#"{"num":30000,"den":1001}"#
    );
    assert_eq!(
        serde_json::to_string(&FrameRate::new(48, 2).unwrap()).unwrap(),
        r#"{"num":24,"den":1}"#
    );
    assert_eq!(
        serde_json::from_str::<FrameRate>(r#"{"num":30000,"den":1001}"#).unwrap(),
        FrameRate::FPS_29_97
    );
}

#[test]
fn sample_rate_serde_uses_hertz() {
    assert_eq!(
        serde_json::to_string(&SampleRate::HZ_48_000).unwrap(),
        r#"{"hz":48000}"#
    );
    assert_eq!(
        serde_json::from_str::<SampleRate>(r#"{"hz":48000}"#).unwrap(),
        SampleRate::HZ_48_000
    );
}

#[test]
fn serde_rejects_invalid_rates() {
    for json in [
        r#"{"num":1,"den":0}"#,
        r#"{"num":0,"den":1}"#,
        r#"{"num":11,"den":1}"#,
        r#"{"num":1,"den":2}"#,
    ] {
        assert!(
            serde_json::from_str::<FrameRate>(json).is_err(),
            "accepted {json}"
        );
    }
    assert!(serde_json::from_str::<SampleRate>(r#"{"hz":0}"#).is_err());
    assert!(serde_json::from_str::<SampleRate>(r#"{"hz":11}"#).is_err());
}

fn every_tested_rate() -> Vec<FrameRate> {
    vec![
        FrameRate::FPS_23_976,
        FrameRate::FPS_24,
        FrameRate::FPS_25,
        FrameRate::FPS_29_97,
        FrameRate::FPS_30,
        FrameRate::FPS_59_94,
        FrameRate::FPS_120,
        FrameRate::new(245, 3).unwrap(),
    ]
}

#[test]
fn frames_round_trip_at_every_rate() {
    for rate in every_tested_rate() {
        for frames in [-1_000_000, -86_400, -24, -1, 0, 1, 23, 24, 1_001, 123_456] {
            let flicks = Flicks::from_frames(frames, rate).unwrap();
            assert_eq!(flicks.to_frames_floor(rate), frames, "{rate:?} {frames}");
            assert!(flicks.is_frame_boundary(rate), "{rate:?} {frames}");
        }
    }
}

#[test]
fn samples_round_trip_at_every_standard_rate() {
    for hz in [44_100, 48_000, 96_000] {
        let rate = SampleRate::new(hz).unwrap();
        for samples in [-1_000_000, -48_000, -1, 0, 1, 44_099, 48_000, 123_456] {
            let flicks = Flicks::from_samples(samples, rate).unwrap();
            assert_eq!(flicks.to_samples_floor(rate), samples, "{hz} {samples}");
        }
    }
}

proptest! {
    #[test]
    fn frames_round_trip_when_representable(frames in any::<i64>()) {
        let d = 29_400_000;
        let expected = frames.checked_mul(d).map(|ticks| ticks / d);
        let actual = Flicks::from_frames(frames, FrameRate::FPS_24)
            .ok()
            .map(|flicks| flicks.to_frames_floor(FrameRate::FPS_24));
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn to_frames_floor_matches_euclidean_division(value in any::<i64>()) {
        let d = 29_400_000;
        prop_assert_eq!(
            Flicks::from_ticks(value).to_frames_floor(FrameRate::FPS_24),
            value.div_euclid(d)
        );
    }

    #[test]
    fn frame_boundary_matches_the_remainder(value in any::<i64>()) {
        let d = 29_400_000;
        prop_assert_eq!(
            Flicks::from_ticks(value).is_frame_boundary(FrameRate::FPS_24),
            value.rem_euclid(d) == 0
        );
    }

    #[test]
    fn samples_round_trip_when_representable(samples in any::<i64>()) {
        let d = 14_700;
        let expected = samples.checked_mul(d).map(|ticks| ticks / d);
        let actual = Flicks::from_samples(samples, SampleRate::HZ_48_000)
            .ok()
            .map(|flicks| flicks.to_samples_floor(SampleRate::HZ_48_000));
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn accepted_frame_rates_round_trip(num in 1u32..100_000, den in 1u32..1_000) {
        if let Ok(rate) = FrameRate::new(num, den) {
            let json = serde_json::to_string(&rate).unwrap();
            prop_assert_eq!(serde_json::from_str::<FrameRate>(&json).unwrap(), rate);
        }
    }

    #[test]
    fn accepted_sample_rates_round_trip(hz in 1u32..200_000) {
        if let Ok(rate) = SampleRate::new(hz) {
            let json = serde_json::to_string(&rate).unwrap();
            prop_assert_eq!(serde_json::from_str::<SampleRate>(&json).unwrap(), rate);
        }
    }

    #[test]
    fn accepted_frame_rates_have_exact_frame_durations(num in 1u32..100_000, den in 1u32..1_000) {
        if let Ok(rate) = FrameRate::new(num, den) {
            let duration = rate.flicks_per_frame().ticks();
            prop_assert!(duration > 0);
            prop_assert_eq!(
                i128::from(duration) * i128::from(rate.num()),
                i128::from(FLICKS_PER_SECOND) * i128::from(rate.den())
            );
        }
    }
}
