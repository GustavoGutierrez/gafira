//! ARQ-126, FV-20 (d): integrated loudness (BS.1770-4, EBU R128), ARQ-110 and
//! ARQ-111 for the meter. The EBU Tech 3341 signals are synthesised here, the
//! files are not stored.

use gafira_audio::{AudioError, integrated_from_blocks, integrated_loudness};
use gafira_time::SampleRate;

fn rate(hz: u32) -> SampleRate {
    SampleRate::new(hz).expect("a valid rate")
}

/// `secs` of a sine of peak `amp`, `round(secs * hz)` samples.
fn sine(hz: u32, freq: f64, amp: f64, secs: f64) -> Vec<f32> {
    let n = (secs * f64::from(hz)).round() as usize;
    (0..n)
        .map(|i| (amp * (std::f64::consts::TAU * freq * i as f64 / f64::from(hz)).sin()) as f32)
        .collect()
}

fn dbfs(db: f64) -> f64 {
    10_f64.powf(db / 20.0)
}

/// Identical stereo 1 kHz sine pieces `(seconds, LUFS)`: a 1 kHz sine of peak
/// `L` dBFS in both channels reads `L` LUFS (K weighting is +0.691 dB there).
fn stereo_pieces(hz: u32, pieces: &[(f64, f64)]) -> Vec<Vec<f32>> {
    let one: Vec<f32> = pieces
        .iter()
        .flat_map(|&(secs, lufs)| sine(hz, 1000.0, dbfs(lufs), secs))
        .collect();
    vec![one.clone(), one]
}

fn measure(chans: &[Vec<f32>], hz: u32) -> Result<Option<f64>, AudioError> {
    let slices: Vec<&[f32]> = chans.iter().map(Vec::as_slice).collect();
    integrated_loudness(&slices, rate(hz))
}

fn lufs(chans: &[Vec<f32>], hz: u32) -> f64 {
    measure(chans, hz)
        .expect("a valid call")
        .expect("a measure")
}

#[track_caller]
fn assert_close(got: f64, want: f64, tol: f64) {
    assert!((got - want).abs() <= tol, "got {got}, want {want} +-{tol}");
}

// EBU Tech 3341, minimum requirements for integrated loudness, +-0.1 LU.

#[test]
fn arq_126_ebu_3341_case_1_stereo_sine_at_minus_23_lufs() {
    assert_close(
        lufs(&stereo_pieces(48_000, &[(20.0, -23.0)]), 48_000),
        -23.0,
        0.1,
    );
}

#[test]
fn arq_126_ebu_3341_case_2_stereo_sine_at_minus_33_lufs() {
    assert_close(
        lufs(&stereo_pieces(48_000, &[(20.0, -33.0)]), 48_000),
        -33.0,
        0.1,
    );
}

#[test]
fn arq_126_ebu_3341_case_3_quiet_edges_around_a_loud_minute_are_gated_out() {
    let s = stereo_pieces(48_000, &[(10.0, -36.0), (60.0, -23.0), (10.0, -36.0)]);
    assert_close(lufs(&s, 48_000), -23.0, 0.1);
}

#[test]
fn arq_126_ebu_3341_case_4_the_absolute_gate_drops_the_very_quiet_pieces() {
    // Published: -72, -36, -23, -36, -72 LUFS for 10, 10, 60, 10, 10 s.
    let s = stereo_pieces(
        48_000,
        &[
            (10.0, -72.0),
            (10.0, -36.0),
            (60.0, -23.0),
            (10.0, -36.0),
            (10.0, -72.0),
        ],
    );
    assert_close(lufs(&s, 48_000), -23.0, 0.1);
}

#[test]
fn arq_126_ebu_3341_case_5_two_levels_inside_the_relative_gate_are_averaged() {
    // Published: -26, -20, -26 LUFS for 20, 20.1, 20 s.
    let s = stereo_pieces(48_000, &[(20.0, -26.0), (20.1, -20.0), (20.0, -26.0)]);
    assert_close(lufs(&s, 48_000), -23.0, 0.1);
}

#[test]
fn arq_126_a_997_hz_stereo_sine_at_minus_23_dbfs_reads_minus_23_at_44_1_48_and_96_khz() {
    for hz in [44_100, 48_000, 96_000] {
        let one = sine(hz, 997.0, dbfs(-23.0), 10.0);
        assert_close(lufs(&[one.clone(), one], hz), -23.0, 0.1);
    }
}

#[test]
fn arq_126_six_channels_weight_the_surround_pair_and_skip_the_lfe() {
    let hz = 48_000;
    let tone = sine(hz, 997.0, dbfs(-20.0), 5.0);
    let silent = vec![0.0_f32; tone.len()];
    // L, R, C, LFE, Ls, Rs
    let want = [
        Some(-23.0),
        Some(-23.0),
        Some(-23.0),
        None,
        Some(-21.5),
        Some(-21.5),
    ];
    for (index, want) in want.into_iter().enumerate() {
        let chans: Vec<Vec<f32>> = (0..6)
            .map(|i| {
                if i == index {
                    tone.clone()
                } else {
                    silent.clone()
                }
            })
            .collect();
        match (measure(&chans, hz), want) {
            (Ok(Some(got)), Some(want)) => assert_close(got, want, 0.1),
            (Ok(None), None) => {}
            (got, want) => panic!("channel {index}: got {got:?}, want {want:?}"),
        }
    }
}

#[test]
fn arq_126_a_mono_signal_has_weight_one() {
    let one = sine(48_000, 997.0, dbfs(-26.0), 5.0);
    // One channel at -26 dBFS: mean square -29.01 dBFS, K-weighted +0.691, then -0.691.
    assert_close(lufs(&[one], 48_000), -29.01, 0.1);
}

#[test]
fn arq_126_silence_has_no_measure_and_not_minus_infinity() {
    assert_eq!(measure(&vec![vec![0.0; 48_000 * 3]; 2], 48_000), Ok(None));
    assert_eq!(measure(&[vec![0.0; 48_000 * 3]], 44_100), Ok(None));
}

#[test]
fn arq_126_a_signal_under_400_ms_has_no_measure_and_400_ms_has_one() {
    // 399 ms and 400 ms at 48 kHz, and one sample short of 400 ms at 44.1 kHz.
    let cases = [
        (48_000, 19_152, None),
        (48_000, 19_200, Some(())),
        (44_100, 17_639, None),
        (44_100, 17_640, Some(())),
        (96_000, 38_399, None),
        (96_000, 38_400, Some(())),
    ];
    for (hz, n, want) in cases {
        let one: Vec<f32> = sine(hz, 997.0, dbfs(-23.0), 1.0)
            .into_iter()
            .take(n)
            .collect();
        let got = measure(&[one.clone(), one], hz).expect("ok");
        assert_eq!(got.map(|_| ()), want, "{hz} Hz, {n} samples");
    }
}

/// The valid sample rates up to 192 kHz: the divisors of 705 600 000.
fn valid_rates_up_to_192_khz() -> Vec<u32> {
    (1..=192_000_u32)
        .filter(|&hz| SampleRate::new(hz).is_ok())
        .collect()
}

/// A tone at 0.21 of the rate, loud enough to pass the gates, `n` frames.
fn tone(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| (0.2 * (std::f64::consts::TAU * 0.21 * i as f64 + 1.0).sin()) as f32)
        .collect()
}

#[test]
fn arq_126_400_ms_in_frames_has_a_measure_and_one_frame_less_has_none_at_every_rate() {
    // ARQ-126: the block is ceil(0.4 fs) frames; 4 410 frames at 11 025 Hz are
    // 400 ms and are measured, 100 frames at 252 Hz are 396.8 ms and are not.
    assert!(measure(&[tone(4_410)], 11_025).expect("ok").is_some());
    assert_eq!(measure(&[tone(100)], 252), Ok(None));
    for hz in valid_rates_up_to_192_khz() {
        let block = (u64::from(hz) * 4).div_ceil(10) as usize;
        let at = |n: usize| measure(&[tone(n)], hz).expect("a valid call");
        assert!(at(block).is_some(), "{hz} Hz, {block} frames");
        assert_eq!(at(block - 1), None, "{hz} Hz, {} frames", block - 1);
    }
}

#[test]
fn arq_126_a_channel_count_other_than_1_2_or_6_is_an_error() {
    for n in [0_usize, 3, 4, 5, 7] {
        let chans = vec![vec![0.5_f32; 48_000]; n];
        assert_eq!(
            measure(&chans, 48_000),
            Err(AudioError::ChannelCount { channels: n }),
            "{n} channels"
        );
    }
}

#[test]
fn arq_110_a_non_finite_sample_is_an_error_naming_the_samples() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut one = sine(48_000, 997.0, 0.1, 1.0);
        one[777] = bad;
        let r = measure(&[one.clone(), one], 48_000);
        assert_eq!(r, Err(AudioError::NonFinite { field: "samples" }), "{bad}");
    }
}

#[test]
fn arq_110_channels_of_different_lengths_are_an_error() {
    let r = measure(&[vec![0.1; 48_000], vec![0.1; 47_999]], 48_000);
    assert_eq!(r, Err(AudioError::OutOfRange { field: "channels" }));
}

#[test]
fn arq_126_the_k_weighting_at_other_rates_matches_48_khz_within_a_tenth_of_a_db() {
    // The rates of the criterion, plus the others that were checked before;
    // tones 997 Hz, 2.5 kHz and 0.98 of the lower of 16 kHz and Nyquist, those
    // that fit under that ceiling (and the sweep of fractions of it).
    let at = |r: u32, freq: f64| lufs(&[sine(r, freq, 0.2, 1.0)], r);
    for hz in [
        3_360, 8_000, 11_025, 12_000, 16_000, 17_640, 22_050, 24_000, 32_000, 44_100, 88_200,
        96_000, 192_000,
    ] {
        let top = (0.98 * f64::from(hz) / 2.0).min(16_000.0);
        let mut tones = vec![997.0, 2_500.0, top];
        tones.extend([40.0, 100.0, 500.0, 4_000.0, 8_000.0, 12_000.0, 15_000.0]);
        tones.extend([0.25, 0.5, 0.75, 0.9].map(|f| f * top));
        for freq in tones.into_iter().filter(|&f| f <= top) {
            let (got, want) = (at(hz, freq), at(48_000, freq));
            assert!(
                (got - want).abs() <= 0.1,
                "{hz} Hz at {freq} Hz: {got} against {want}"
            );
        }
    }
}

#[test]
fn arq_126_every_sample_rate_is_measured_and_none_gives_an_error() {
    // From 1 Hz up: no floor. The measure is finite whatever the rate.
    for hz in [1, 8, 100, 1_000, 3_200, 3_600] {
        let one = vec![0.1_f32; hz as usize * 2 + 100];
        let got = measure(&[one], hz);
        assert!(
            matches!(got, Ok(Some(v)) if v.is_finite()),
            "{hz} Hz: {got:?}"
        );
    }
}

#[test]
fn arq_111_the_meter_gives_the_same_bytes_twice() {
    let s = stereo_pieces(44_100, &[(3.0, -30.0), (3.0, -20.0)]);
    let (a, b) = (measure(&s, 44_100), measure(&s, 44_100));
    assert_eq!(
        a.map(|v| v.map(f64::to_bits)),
        b.map(|v| v.map(f64::to_bits))
    );
}

// The gating, on block energies (mean square of the K-weighted, channel-weighted block).

fn energy(lufs: f64) -> f64 {
    10_f64.powf((lufs + 0.691) / 10.0)
}

fn loudness_of(e: f64) -> f64 {
    -0.691 + 10.0 * e.log10()
}

#[test]
fn arq_126_a_block_at_exactly_minus_70_lufs_is_under_the_absolute_gate() {
    // Some energy whose loudness is -70.0 to the last bit.
    let mut e = energy(-70.0);
    let mut steps = 0;
    while loudness_of(e) < -70.0 && steps < 1000 {
        e = e.next_up();
        steps += 1;
    }
    while loudness_of(e) > -70.0 && steps < 2000 {
        e = e.next_down();
        steps += 1;
    }
    assert_eq!(loudness_of(e), -70.0, "found an energy at exactly -70 LUFS");
    assert_eq!(integrated_from_blocks(&[e, e, e]), Ok(None), "excluded");
    // The first energy above it is kept.
    let mut up = e;
    while loudness_of(up) <= -70.0 {
        up = up.next_up();
    }
    let got = integrated_from_blocks(&[up, up, up])
        .expect("ok")
        .expect("a measure");
    assert_eq!(got, loudness_of(up));
}

#[test]
fn arq_126_the_relative_gate_is_10_lu_under_the_mean_energy_of_the_blocks_over_minus_70() {
    // 10 blocks at -23, 10 at -37, 15 at -80 and 15 silent ones. The mean
    // energy of the blocks over -70 is 0.52 of the -23 energy: the relative
    // gate is at about -35.8, so the -37 blocks leave and the result is -23.
    // A gate from the mean over all blocks (-40), or from the mean of the
    // loudness values (-40), would keep the -37 blocks.
    let mut blocks = vec![energy(-23.0); 10];
    blocks.extend(vec![energy(-37.0); 10]);
    blocks.extend(vec![energy(-80.0); 15]);
    blocks.extend(vec![0.0; 15]);
    let got = integrated_from_blocks(&blocks)
        .expect("ok")
        .expect("a measure");
    assert_close(got, loudness_of(energy(-23.0)), 1e-9);
}

#[test]
fn arq_126_the_relative_gate_is_exactly_10_lu_under_the_mean() {
    // 10 blocks at -23 and 10 at -23 - x: the mean energy sits 2.7 to 2.8 dB
    // under the loud ones, so the gate is at about -35.78 (x = 12.78).
    let two = |x: f64| {
        let mut blocks = vec![energy(-23.0); 10];
        blocks.extend(vec![energy(-23.0 - x); 10]);
        integrated_from_blocks(&blocks)
            .expect("ok")
            .expect("a measure")
    };
    let mean_of = |x: f64| loudness_of((energy(-23.0) + energy(-23.0 - x)) / 2.0);
    // 12.7 LU under: just above the gate -> both count.
    assert_close(two(12.7), mean_of(12.7), 1e-9);
    // 12.85 LU under: just below it -> the loud ones only. The gap is 0.08 LU
    // wide, so a gate off by 0.1 LU from -10 fails one of the two.
    assert_close(two(12.85), loudness_of(energy(-23.0)), 1e-9);
}

#[test]
fn arq_126_the_relative_gate_counts_only_the_blocks_over_the_absolute_gate() {
    // 10 blocks at -68 and 10 at -75. The mean of the blocks over -70 is -68,
    // so the relative gate is -78; the -75 blocks are under the absolute gate
    // and must not come back in through the relative one.
    let mut blocks = vec![energy(-68.0); 10];
    blocks.extend(vec![energy(-75.0); 10]);
    let got = integrated_from_blocks(&blocks)
        .expect("ok")
        .expect("a measure");
    assert_close(got, -68.0, 1e-9);
}

#[test]
fn arq_110_a_sum_of_energies_that_overflows_is_an_error_not_no_measure() {
    for blocks in [vec![1.7e308, 1.7e308], vec![1e308, 1e308, 1e308]] {
        assert_eq!(
            integrated_from_blocks(&blocks),
            Err(AudioError::NonFinite { field: "energies" }),
            "{blocks:?}"
        );
    }
    assert!(matches!(integrated_from_blocks(&[1e308]), Ok(Some(_))));
}

#[test]
fn arq_126_no_blocks_or_only_silent_ones_have_no_measure() {
    assert_eq!(integrated_from_blocks(&[]), Ok(None));
    assert_eq!(integrated_from_blocks(&[0.0, 0.0]), Ok(None));
}

#[test]
fn arq_110_a_non_finite_or_negative_block_energy_is_an_error() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            integrated_from_blocks(&[1e-3, bad]),
            Err(AudioError::NonFinite { field: "energies" })
        );
    }
    assert_eq!(
        integrated_from_blocks(&[-1e-3]),
        Err(AudioError::OutOfRange { field: "energies" })
    );
}

#[test]
fn arq_102_the_new_errors_give_a_key_with_their_data() {
    use gafira_doc::msg::{Message, MessageKey, MsgArg, ToMessage, is_valid_key};
    let range = AudioError::OutOfRange { field: "channels" };
    let count = AudioError::ChannelCount { channels: 3 };
    assert_eq!(
        range.to_message(),
        Message {
            key: MessageKey("error.audio.out_of_range"),
            args: vec![("field", MsgArg::Text("channels".to_string()))],
        }
    );
    assert_eq!(
        count.to_message(),
        Message {
            key: MessageKey("error.audio.channel_count"),
            args: vec![("channels", MsgArg::Count(3))],
        }
    );
    assert!(is_valid_key(range.to_message().key.0) && is_valid_key(count.to_message().key.0));
}

#[test]
fn arq_126_the_first_and_the_last_block_of_a_signal_count_at_the_exact_boundaries() {
    // 48 kHz: a block is 19 200 frames and the hop 4 800. A burst of 100 ms
    // placed in the last hop is seen only by the last block, which exists from
    // 24 000 frames on; one frame less and there is just the first block, which
    // has no burst. The same burst in the first hop is seen by the first block.
    let hop = 4_800;
    let burst = |from: usize, to: usize, n: usize| -> Vec<f32> {
        (0..n)
            .map(|i| {
                if (from..to).contains(&i) {
                    0.5 * (i as f32 * 0.3).sin()
                } else {
                    0.0
                }
            })
            .collect()
    };
    let n = 19_200 + hop;
    assert_eq!(
        measure(&[burst(n - hop, n, n)], 48_000),
        Ok(Some(lufs(&[burst(n - hop, n, n)], 48_000)))
    );
    assert!(lufs(&[burst(n - hop, n, n)], 48_000) > -70.0);
    assert_eq!(measure(&[burst(n - hop, n - 1, n - 1)], 48_000), Ok(None));
    assert!(
        measure(&[burst(0, hop, 19_200)], 48_000)
            .expect("ok")
            .is_some()
    );
    assert!(
        measure(&[burst(0, hop, 19_200 - 1)], 48_000)
            .expect("ok")
            .is_none()
    );
    // The burst in the first hop is also in the second block's range only if
    // that block starts before it ends: 24 000 frames hold two blocks, the
    // second sees nothing of a burst in [0, hop).
    let two = measure(&[burst(0, hop, n)], 48_000)
        .expect("ok")
        .expect("a measure");
    let one = measure(&[burst(0, hop, 19_200)], 48_000)
        .expect("ok")
        .expect("a measure");
    assert_close(two, one, 1e-9);
}
