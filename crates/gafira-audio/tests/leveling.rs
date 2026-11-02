//! ARQ-127, FV-10 (b), FV-20 (d): true peak and the Nivelar gain. ARQ-110 and
//! ARQ-111 for both.

use gafira_audio::{
    AudioBlock, AudioError, AudioProcessor, LoudnessNormalizer, PARAM_GAIN, ParamSnapshot,
    integrated_loudness, level, plan_leveling, true_peak,
};
use gafira_time::SampleRate;

fn rate(hz: u32) -> SampleRate {
    SampleRate::new(hz).expect("a valid rate")
}

fn sine(hz: u32, freq: f64, amp: f64, secs: f64) -> Vec<f32> {
    let n = (secs * f64::from(hz)).round() as usize;
    (0..n)
        .map(|i| (amp * (std::f64::consts::TAU * freq * i as f64 / f64::from(hz)).sin()) as f32)
        .collect()
}

fn dbfs(db: f64) -> f64 {
    10_f64.powf(db / 20.0)
}

fn db(linear: f64) -> f64 {
    20.0 * linear.log10()
}

fn peak(chans: &[Vec<f32>], hz: u32) -> Result<f64, AudioError> {
    let slices: Vec<&[f32]> = chans.iter().map(Vec::as_slice).collect();
    true_peak(&slices, rate(hz))
}

fn lufs(chans: &[Vec<f32>], hz: u32) -> Option<f64> {
    let slices: Vec<&[f32]> = chans.iter().map(Vec::as_slice).collect();
    integrated_loudness(&slices, rate(hz)).expect("a valid call")
}

fn plan(chans: &[Vec<f32>], hz: u32, target: f64) -> gafira_audio::Leveling {
    let slices: Vec<&[f32]> = chans.iter().map(Vec::as_slice).collect();
    level(&slices, rate(hz), target).expect("a valid call")
}

/// The signal through `LoudnessNormalizer` with `gain_db`, in blocks of 512
/// frames (the last one shorter), as the engine would run it.
fn applied(chans: &[Vec<f32>], hz: u32, gain_db: f64) -> Vec<Vec<f32>> {
    const BLOCK: usize = 512;
    let mut normalizer = LoudnessNormalizer::new();
    normalizer.prepare(hz, BLOCK, chans.len() as u16);
    let mut params = ParamSnapshot::new();
    params.set(PARAM_GAIN, gain_db);
    let mut out = chans.to_vec();
    let frames = out[0].len();
    for from in (0..frames).step_by(BLOCK) {
        let to = (from + BLOCK).min(frames);
        let mut slices: Vec<&mut [f32]> = out.iter_mut().map(|c| &mut c[from..to]).collect();
        normalizer.process(&mut AudioBlock::new(&mut slices), &params);
    }
    out
}

#[track_caller]
fn assert_close(got: f64, want: f64, tol: f64) {
    assert!((got - want).abs() <= tol, "got {got}, want {want} +-{tol}");
}

// True peak.

#[test]
fn arq_127_a_sine_at_a_quarter_of_the_rate_between_its_samples_peaks_at_full_scale() {
    // Also at 96 and 192 kHz, where the factor is 4 too (the chosen reading of
    // Annex 2, which asks less). fs/4 sines whose crest falls half, a quarter and three quarters of a
    // sample after a sample (phases 45, 67.5 and 22.5 degrees): the samples
    // reach 0.7071, 0.9239 and 0.9239 of the crest, which is 1.0. x2 would see
    // only the first, x4 sees the three. The ends fade over 2 000 samples,
    // because a signal that starts abruptly really overshoots there (0.1 dB).
    use std::f64::consts::PI;
    for hz in [8_000, 32_000, 44_100, 48_000, 96_000, 192_000] {
        for phase in [PI / 4.0, 3.0 * PI / 8.0, PI / 8.0] {
            let n = hz as usize;
            let one: Vec<f32> = (0..n)
                .map(|i| {
                    let fade = (i.min(n - 1 - i) as f64 / 2_000.0).min(1.0);
                    (fade * (PI / 2.0 * i as f64 + phase).sin()) as f32
                })
                .collect();
            let got = peak(&[one], hz).expect("a valid call");
            assert!(
                db(got).abs() <= 0.01,
                "{hz} Hz, phase {phase}: {got} is {} dBFS",
                db(got)
            );
        }
    }
}

#[test]
fn arq_127_the_true_peak_is_never_under_the_sample_peak_and_sees_every_channel() {
    for hz in [8_000, 44_100, 48_000, 96_000, 192_000] {
        let a = sine(hz, 1_234.5, 0.3, 0.5);
        let b = sine(hz, 4_321.0, 0.6, 0.5);
        let c: Vec<f32> = vec![0.0; a.len()];
        let lfe = sine(hz, 60.0, 0.45, 0.5);
        let sample_peak = |chans: &[&Vec<f32>]| {
            chans
                .iter()
                .flat_map(|c| c.iter())
                .fold(0.0_f64, |m, &x| m.max(f64::from(x).abs()))
        };
        let six = [a.clone(), b.clone(), c.clone(), lfe.clone(), c.clone(), c];
        let got = peak(&six, hz).expect("six channels");
        assert!(
            got >= sample_peak(&[&a, &b, &lfe]),
            "{hz} Hz: {got} under the sample peak"
        );
        // The 0.6 channel wins; the LFE (0.45) is part of the set, not ignored.
        let only_lfe = [
            vec![0.0; a.len()],
            vec![0.0; a.len()],
            vec![0.0; a.len()],
            lfe.clone(),
            vec![0.0; a.len()],
            vec![0.0; a.len()],
        ];
        let got = peak(&only_lfe, hz).expect("six channels");
        assert!(got >= 0.449, "{hz} Hz: the LFE gave {got}");
    }
}

#[test]
fn arq_127_a_lone_sample_peaks_at_itself() {
    let mut one = vec![0.0_f32; 4_800];
    one[2_400] = -0.5;
    let got = peak(&[one], 48_000).expect("a valid call");
    assert_close(got, 0.5, 1e-6);
}

#[test]
fn arq_127_silence_and_an_empty_signal_have_a_peak_of_zero() {
    assert_eq!(peak(&[vec![0.0; 1000]], 48_000), Ok(0.0));
    assert_eq!(peak(&[Vec::new(), Vec::new()], 48_000), Ok(0.0));
}

#[test]
fn arq_110_the_peak_takes_the_same_channel_counts_and_checks_the_same_samples() {
    for n in [0_usize, 3, 4, 5, 7] {
        assert_eq!(
            peak(&vec![vec![0.5_f32; 480]; n], 48_000),
            Err(AudioError::ChannelCount { channels: n })
        );
    }
    let mut bad = vec![0.1_f32; 480];
    bad[3] = f32::NAN;
    assert_eq!(
        peak(&[bad], 48_000),
        Err(AudioError::NonFinite { field: "samples" })
    );
    assert_eq!(
        peak(&[vec![0.1; 480], vec![0.1; 479]], 48_000),
        Err(AudioError::OutOfRange { field: "channels" })
    );
}

#[test]
fn arq_111_the_peak_gives_the_same_bytes_twice() {
    let s = [
        sine(44_100, 3_000.0, 0.7, 0.3),
        sine(44_100, 5_000.0, 0.2, 0.3),
    ];
    assert_eq!(
        peak(&s, 44_100).map(f64::to_bits),
        peak(&s, 44_100).map(f64::to_bits)
    );
}

// The gain.

#[test]
fn arq_127_the_applied_gain_is_the_smaller_of_the_request_and_the_ceiling() {
    // -30 LUFS, -3 dBTP, target -14: request +16, ceiling -1 + 3 = +2.
    let p = plan_leveling(-14.0, Some(-30.0), Some(-3.0)).expect("finite");
    assert_eq!((p.gain_db, p.reached), (2.0, false));
    // -20 LUFS, -10 dBTP: request +6, ceiling +9.
    let p = plan_leveling(-14.0, Some(-20.0), Some(-10.0)).expect("finite");
    assert_eq!((p.gain_db, p.reached), (6.0, true));
    // Attenuation: -14 LUFS to -23 is -9, far under the ceiling.
    let p = plan_leveling(-23.0, Some(-14.0), Some(-30.0)).expect("finite");
    assert_eq!((p.gain_db, p.reached), (-9.0, true));
    // A peak over full scale gives a negative ceiling: -1 - 2 = -3.
    let p = plan_leveling(-14.0, Some(-19.0), Some(2.0)).expect("finite");
    assert_eq!((p.gain_db, p.reached), (-3.0, false));
}

#[test]
fn arq_127_a_request_equal_to_the_ceiling_reaches_the_target() {
    // -20 LUFS, -7 dBTP: request +6, ceiling -1 + 7 = +6.
    let p = plan_leveling(-14.0, Some(-20.0), Some(-7.0)).expect("finite");
    assert_eq!((p.gain_db, p.reached), (6.0, true));
}

#[test]
fn arq_127_without_a_measure_the_gain_is_zero_and_the_target_is_not_reached() {
    for peak in [None, Some(-3.0)] {
        let p = plan_leveling(-14.0, None, peak).expect("finite");
        assert_eq!((p.gain_db, p.reached), (0.0, false));
    }
}

#[test]
fn arq_110_the_plan_names_the_non_finite_input() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            plan_leveling(bad, Some(-20.0), Some(-3.0)),
            Err(AudioError::NonFinite { field: "target" })
        );
        assert_eq!(
            plan_leveling(-14.0, Some(bad), Some(-3.0)),
            Err(AudioError::NonFinite { field: "measured" })
        );
        assert_eq!(
            plan_leveling(-14.0, Some(-20.0), Some(bad)),
            Err(AudioError::NonFinite { field: "peak" })
        );
        // Checked even when the measure is missing.
        assert_eq!(
            plan_leveling(-14.0, None, Some(bad)),
            Err(AudioError::NonFinite { field: "peak" })
        );
    }
}

#[test]
fn arq_110_a_request_that_overflows_is_an_error() {
    assert_eq!(
        plan_leveling(f64::MAX, Some(-f64::MAX), None),
        Err(AudioError::NonFinite { field: "gain" })
    );
}

// Measure and plan together, with the gain applied by scaling the samples.

fn stereo_sine(hz: u32, lufs: f64, secs: f64) -> Vec<Vec<f32>> {
    let one = sine(hz, 997.0, dbfs(lufs), secs);
    vec![one.clone(), one]
}

#[test]
fn fv_10_b_a_signal_at_minus_20_lufs_with_headroom_levels_to_minus_14() {
    let s = stereo_sine(48_000, -20.0, 10.0);
    let p = plan(&s, 48_000, -14.0);
    assert!(p.reached);
    let out = lufs(&applied(&s, 48_000, p.gain_db), 48_000).expect("a measure");
    assert_close(out, -14.0, 0.5);
}

#[test]
fn fv_10_b_a_quiet_signal_with_a_high_peak_stops_at_minus_1_dbtp_and_says_so() {
    // -30 LUFS with one sample at -3 dBFS.
    let mut s = stereo_sine(48_000, -30.0, 10.0);
    s[0][100_000] = dbfs(-3.0) as f32;
    let p = plan(&s, 48_000, -14.0);
    assert!(!p.reached);
    assert_close(p.gain_db, 2.0, 0.01);
    let out = applied(&s, 48_000, p.gain_db);
    assert!(db(peak(&out, 48_000).expect("ok")) <= -1.0);
    assert_close(lufs(&out, 48_000).expect("a measure"), -28.0, 0.5);
}

#[test]
fn arq_127_silence_and_a_signal_under_400_ms_have_gain_zero_and_do_not_reach() {
    for s in [
        vec![vec![0.0_f32; 48_000]; 2],
        stereo_sine(48_000, -20.0, 0.399),
    ] {
        let p = plan(&s, 48_000, -14.0);
        assert_eq!((p.gain_db, p.reached), (0.0, false));
    }
}

#[test]
fn arq_110_level_checks_the_target_and_the_signal() {
    let s = [vec![0.1_f32; 48_000]];
    let slices: Vec<&[f32]> = s.iter().map(Vec::as_slice).collect();
    assert_eq!(
        level(&slices, rate(48_000), f64::NAN),
        Err(AudioError::NonFinite { field: "target" })
    );
    let three = vec![vec![0.1_f32; 480]; 3];
    let slices: Vec<&[f32]> = three.iter().map(Vec::as_slice).collect();
    assert_eq!(
        level(&slices, rate(48_000), -14.0),
        Err(AudioError::ChannelCount { channels: 3 })
    );
}

#[test]
fn arq_111_level_gives_the_same_bytes_twice() {
    let s = stereo_sine(44_100, -26.0, 3.0);
    assert_eq!(plan(&s, 44_100, -16.0), plan(&s, 44_100, -16.0));
}

/// Filtered noise: a one-pole low pass over a xorshift sequence, a different
/// colour and level for every `seed`.
fn noise(seed: u64, frames: usize, amp: f64, pole: f64) -> Vec<f32> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut low = 0.0_f64;
    (0..frames)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let white = (state >> 11) as f64 / (1_u64 << 53) as f64 * 2.0 - 1.0;
            low = pole * low + (1.0 - pole) * white;
            (amp * low) as f32
        })
        .collect()
}

#[test]
fn arq_127_the_leveled_peak_is_at_most_minus_1_dbtp_with_no_tolerance() {
    // 120 stereo signals; the target is far above every signal, so the ceiling
    // is the binding side. Gain from `level`, applied by `LoudnessNormalizer`,
    // peak measured again.
    for seed in 1..=120_u64 {
        let amp = 0.2 + 0.7 * (seed % 7) as f64 / 7.0;
        let pole = 0.5 + 0.45 * (seed % 5) as f64 / 5.0;
        let s = vec![
            noise(seed, 24_000, amp, pole),
            noise(seed + 1_000, 24_000, amp * 0.6, pole),
        ];
        let p = plan(&s, 48_000, 0.0);
        assert!(!p.reached, "seed {seed}: the ceiling binds");
        let got = db(peak(&applied(&s, 48_000, p.gain_db), 48_000).expect("ok"));
        assert!(got <= -1.0, "seed {seed}: {got} dBTP");
    }
}
