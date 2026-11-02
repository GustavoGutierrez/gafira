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
    let s = stereo_pieces(
        48_000,
        &[
            (10.0, -72.0),
            (10.0, -40.0),
            (40.0, -23.0),
            (10.0, -40.0),
            (10.0, -72.0),
        ],
    );
    // Expected: the -72 pieces fall under the -70 LUFS absolute gate and the
    // -40 pieces under the relative gate (about -36.8), so -23.0.
    assert_close(lufs(&s, 48_000), -23.0, 0.1);
}

#[test]
fn arq_126_ebu_3341_case_5_two_levels_inside_the_relative_gate_are_averaged() {
    // 20 s at -26 and 10 s at -20: both stay above the relative gate; the mean
    // energy is 10 log10((20 * 10^-2.6 + 10 * 10^-2.0) / 30) = -22.99 LUFS.
    let s = stereo_pieces(48_000, &[(20.0, -26.0), (10.0, -20.0)]);
    assert_close(lufs(&s, 48_000), -22.99, 0.1);
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
    for hz in [44_100, 88_200, 96_000, 192_000, 32_000] {
        for freq in [
            40.0, 100.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 12_000.0, 15_000.0,
        ] {
            let at = |r: u32| lufs(&[sine(r, freq, 0.2, 3.0)], r);
            assert_close(at(hz), at(48_000), 0.1);
        }
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
