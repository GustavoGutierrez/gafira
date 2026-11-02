//! ARQ-129 and FV-10 (d): a flick offset becomes samples, rounded half away
//! from 0, in integer arithmetic.

use gafira_audio::flicks_to_samples;
use gafira_time::{FLICKS_PER_SECOND, Flicks, SampleRate};

fn samples(ticks: i64, rate: SampleRate) -> i64 {
    flicks_to_samples(Flicks::from_ticks(ticks), rate)
}

/// Independent oracle: unsigned arithmetic on the magnitude.
fn oracle(ticks: i64, hz: u32) -> i64 {
    let magnitude = u128::from(ticks.unsigned_abs());
    let den = FLICKS_PER_SECOND as u128;
    let rounded = (2 * magnitude * u128::from(hz) + den) / (2 * den);
    let rounded = i64::try_from(rounded).unwrap();
    if ticks < 0 { -rounded } else { rounded }
}

#[test]
fn fv_10_d_one_millisecond_is_44_samples_at_44_1_khz_and_48_at_48_khz() {
    let ms = Flicks::from_millis(1).unwrap().ticks();
    assert_eq!(samples(ms, SampleRate::HZ_44_100), 44);
    assert_eq!(samples(ms, SampleRate::HZ_48_000), 48);
    assert_eq!(samples(ms, SampleRate::HZ_96_000), 96);
    assert_eq!(samples(-ms, SampleRate::HZ_44_100), -44);
    assert_eq!(samples(-ms, SampleRate::HZ_48_000), -48);
}

#[test]
fn arq_129_a_half_sample_rounds_away_from_zero() {
    // (flicks, rate, samples): 14 700 flicks per sample at 48 kHz, 16 000 at 44.1 kHz.
    let table = [
        (7_349, SampleRate::HZ_48_000, 0),
        (7_350, SampleRate::HZ_48_000, 1),
        (-7_349, SampleRate::HZ_48_000, 0),
        (-7_350, SampleRate::HZ_48_000, -1),
        (22_050, SampleRate::HZ_48_000, 2),
        (-22_050, SampleRate::HZ_48_000, -2),
        (14_700, SampleRate::HZ_48_000, 1),
        (-14_700, SampleRate::HZ_48_000, -1),
        (7_999, SampleRate::HZ_44_100, 0),
        (8_000, SampleRate::HZ_44_100, 1),
        (-8_000, SampleRate::HZ_44_100, -1),
        (24_000, SampleRate::HZ_44_100, 2),
        (-24_000, SampleRate::HZ_44_100, -2),
        (0, SampleRate::HZ_48_000, 0),
    ];
    for (ticks, rate, want) in table {
        assert_eq!(
            samples(ticks, rate),
            want,
            "{ticks} flicks at {}",
            rate.hz()
        );
    }
}

#[test]
fn arq_129_the_extremes_of_i64_do_not_overflow() {
    let table = [
        (i64::MAX, SampleRate::HZ_44_100, 576_460_752_303_423),
        (i64::MIN, SampleRate::HZ_44_100, -576_460_752_303_423),
        (i64::MAX, SampleRate::HZ_48_000, 627_440_274_615_971),
        (i64::MIN, SampleRate::HZ_48_000, -627_440_274_615_971),
        (i64::MAX, SampleRate::HZ_96_000, 1_254_880_549_231_942),
        (i64::MIN, SampleRate::HZ_96_000, -1_254_880_549_231_942),
    ];
    for (ticks, rate, want) in table {
        assert_eq!(samples(ticks, rate), want, "{ticks} at {}", rate.hz());
    }
    // One sample per flick's worth of rate: the largest rate keeps the value.
    let flick_rate = SampleRate::new(705_600_000).unwrap();
    assert_eq!(samples(i64::MAX, flick_rate), i64::MAX);
    assert_eq!(samples(i64::MIN, flick_rate), i64::MIN);
}

#[test]
fn arq_129_exact_halves_at_huge_offsets_round_away_from_zero() {
    // 14 700 k + 7 350 is exactly k + 1/2 samples at 48 kHz. An `f64` formula
    // loses the half there, because a flick count above 2^62 is not exact.
    let mut found = 0;
    let mut k: i64 = 500_000_000_000_000;
    while found < 8 {
        let t = 14_700 * k + 7_350;
        if t % 1_024 != 0 {
            assert_eq!(samples(t, SampleRate::HZ_48_000), k + 1, "{t}");
            assert_eq!(samples(-t, SampleRate::HZ_48_000), -(k + 1), "-{t}");
            found += 1;
        }
        k += 1;
    }
}

#[test]
fn arq_129_the_result_matches_the_independent_oracle() {
    let rates = [
        SampleRate::HZ_44_100,
        SampleRate::HZ_48_000,
        SampleRate::HZ_96_000,
    ];
    let mut seed: u64 = 12_345;
    for _ in 0..2_000 {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let ticks = (seed >> 1) as i64 - (seed as i64 >> 40);
        for rate in rates {
            assert_eq!(
                samples(ticks, rate),
                oracle(ticks, rate.hz()),
                "{ticks} {}",
                rate.hz()
            );
            assert_eq!(samples(-ticks, rate), oracle(-ticks, rate.hz()));
        }
    }
}
