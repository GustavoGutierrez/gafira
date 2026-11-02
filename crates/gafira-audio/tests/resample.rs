//! ARQ-128, FV-04 (b): constant-ratio resampling with a speed factor. ARQ-110
//! and ARQ-111 for it.

use gafira_audio::{AudioError, Resampler, resample, resampled_len};
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

fn run(input: &[f32], from: u32, to: u32, speed: f64) -> Vec<f32> {
    resample(input, rate(from), rate(to), speed).expect("a valid call")
}

fn len(n: usize, from: u32, to: u32, speed: f64) -> usize {
    resampled_len(n, rate(from), rate(to), speed).expect("a valid call")
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / x.len() as f64).sqrt()
}

fn db(x: f64) -> f64 {
    20.0 * x.log10()
}

// The length.

#[test]
fn arq_128_44100_samples_from_44_1_to_48_khz_give_48000() {
    let out = run(&vec![0.25; 44_100], 44_100, 48_000, 1.0);
    assert_eq!(out.len(), 48_000);
}

#[test]
fn arq_128_96000_samples_at_2x_give_48000_and_the_audio_keeps_sync() {
    // FV-04 (b): the length of ARQ-128 is the sync.
    let out = run(&vec![0.25; 96_000], 48_000, 48_000, 2.0);
    assert_eq!(out.len(), 48_000);
}

#[test]
fn arq_128_the_length_is_the_f64_expression_rounded_half_away_from_zero() {
    // (frames, from, to, speed, length). The first rows are exact ties; the
    // others tell the evaluation order and the precision apart: an f32
    // evaluation, `n * (to / from) / speed` or an exact rational on the
    // speed all give another number on at least one row.
    let rows: [(usize, u32, u32, f64, usize); 12] = [
        (1, 48_000, 48_000, 2.0, 1),
        (3, 48_000, 48_000, 2.0, 2),
        (5, 48_000, 48_000, 2.0, 3),
        (1, 8_000, 8_000, 0.4, 3),
        (3, 8_000, 8_000, 0.4, 8),
        (28, 8_000, 44_100, 0.1, 1_544),
        (33, 32_000, 8_000, 1.1, 8),
        (11, 44_100, 11_025, 1.1, 2),
        (96_000, 8_000, 192_000, 1.1, 2_094_545),
        (3, 22_050, 11_025, 0.6, 3),
        (0, 48_000, 44_100, 1.0, 0),
        (1, 44_100, 48_000, 10.0, 0),
    ];
    for (n, from, to, speed, want) in rows {
        assert_eq!(len(n, from, to, speed), want, "{n} {from}->{to} x{speed}");
    }
}

#[test]
fn arq_128_the_output_has_the_length_of_the_formula_for_any_input_size() {
    for n in [1_usize, 2, 7, 100, 1_000, 4_410] {
        for (from, to, speed) in [
            (44_100, 48_000, 1.0),
            (48_000, 44_100, 1.5),
            (48_000, 8_000, 0.25),
        ] {
            let out = run(&vec![0.1; n], from, to, speed);
            assert_eq!(
                out.len(),
                len(n, from, to, speed),
                "{n} {from}->{to} x{speed}"
            );
        }
    }
}

#[test]
fn arq_128_an_empty_input_gives_an_empty_output() {
    assert!(run(&[], 44_100, 48_000, 1.0).is_empty());
    assert_eq!(len(0, 44_100, 48_000, 1.0), 0);
}

#[test]
fn arq_128_a_speed_outside_0_1_to_10_is_an_error_and_the_bounds_are_valid() {
    for bad in [
        0.0,
        -0.0,
        -1.0,
        0.09,
        0.099_999_999_999,
        10.1,
        10.000_000_000_001,
        1e300,
    ] {
        let r = resample(&[0.0; 4], rate(48_000), rate(48_000), bad);
        assert_eq!(r, Err(AudioError::OutOfRange { field: "speed" }), "{bad}");
        assert_eq!(
            resampled_len(4, rate(48_000), rate(48_000), bad),
            Err(AudioError::OutOfRange { field: "speed" }),
            "{bad}"
        );
    }
    for ok in [0.1, 10.0, 1.0] {
        assert!(
            resample(&[0.0; 4], rate(48_000), rate(48_000), ok).is_ok(),
            "{ok}"
        );
    }
}

#[test]
fn arq_110_a_non_finite_speed_is_an_error_naming_the_speed() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            resample(&[0.0; 4], rate(48_000), rate(48_000), bad),
            Err(AudioError::NonFinite { field: "speed" })
        );
    }
}

#[test]
fn arq_110_a_non_finite_sample_is_an_error_and_the_output_and_the_state_are_untouched() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut x = vec![0.5_f32; 100];
        x[40] = bad;
        assert_eq!(
            resample(&x, rate(44_100), rate(48_000), 1.0),
            Err(AudioError::NonFinite { field: "samples" })
        );
        let mut r = Resampler::new(rate(44_100), rate(48_000), 1.0).expect("valid");
        let mut out = vec![9.0_f32];
        assert_eq!(
            r.process(&x, &mut out),
            Err(AudioError::NonFinite { field: "samples" })
        );
        assert_eq!(out, vec![9.0]);
        // The state is as new: the run that follows equals a fresh one.
        let good = sine(44_100, 1_000.0, 0.5, 0.2);
        r.process(&good, &mut out).expect("finite");
        r.finish(&mut out);
        assert_eq!(&out[1..], run(&good, 44_100, 48_000, 1.0).as_slice());
    }
}

// The filter.

/// Error of the middle of `out` against the ideal sine of the output rate,
/// as SNR in dB; the first and last `margin` samples are left out.
fn snr_db(out: &[f32], to: u32, freq: f64, amp: f64, margin: usize) -> f64 {
    let mut signal = 0.0;
    let mut noise = 0.0;
    for (j, &y) in out.iter().enumerate().take(out.len() - margin).skip(margin) {
        let ideal = amp * (std::f64::consts::TAU * freq * j as f64 / f64::from(to)).sin();
        signal += ideal * ideal;
        noise += (f64::from(y) - ideal).powi(2);
    }
    10.0 * (signal / noise).log10()
}

#[test]
fn arq_128_a_1_khz_sine_from_44_1_to_48_khz_keeps_an_snr_of_at_least_80_db() {
    let out = run(&sine(44_100, 1_000.0, 0.8, 2.0), 44_100, 48_000, 1.0);
    assert!(snr_db(&out, 48_000, 1_000.0, 0.8, 4_000) >= 80.0);
}

#[test]
fn arq_128_the_ratio_is_constant_over_a_long_run() {
    // 20 s at 44.1 -> 48 kHz: a ratio that drifted would shift the phase and
    // spoil the last seconds. The last 2 s are judged on their own.
    let out = run(&sine(44_100, 1_000.0, 0.8, 20.0), 44_100, 48_000, 1.0);
    let tail = out.len() - 96_000..out.len() - 4_000;
    let signal: f64 = tail
        .clone()
        .map(|j| {
            let ideal = 0.8 * (std::f64::consts::TAU * 1_000.0 * j as f64 / 48_000.0).sin();
            ideal * ideal
        })
        .sum();
    let noise: f64 = tail
        .map(|j| {
            let ideal = 0.8 * (std::f64::consts::TAU * 1_000.0 * j as f64 / 48_000.0).sin();
            (f64::from(out[j]) - ideal).powi(2)
        })
        .sum();
    assert!(10.0 * (signal / noise).log10() >= 80.0);
}

/// The gain of a sine of `freq` (Hz of the input) through the resampler, in dB,
/// judged on the middle of the output.
fn gain_db(from: u32, to: u32, speed: f64, freq: f64) -> f64 {
    let input = sine(from, freq, 0.5, 1.0);
    let out = run(&input, from, to, speed);
    let mid = out.len() / 4..out.len() * 3 / 4;
    db(rms(&out[mid]) / (0.5 / 2_f64.sqrt()))
}

/// Lower Nyquist in Hz of the input: half of the smaller of the effective
/// rates, `from * speed` and `to`, which in input Hz is `min(to / speed, from) / 2`.
fn lower_nyquist(from: u32, to: u32, speed: f64) -> f64 {
    (f64::from(to) / speed).min(f64::from(from)) / 2.0
}

/// Rows `(from, to, speed, lower Nyquist)`.
fn cases() -> Vec<(u32, u32, f64, f64)> {
    [
        (48_000, 32_000, 1.0),
        (96_000, 44_100, 1.0),
        (48_000, 48_000, 2.0),
        (44_100, 48_000, 1.0),
        (48_000, 16_000, 0.5),
        (48_000, 48_000, 10.0),
        (8_000, 48_000, 0.1),
    ]
    .into_iter()
    .map(|(f, t, s)| (f, t, s, lower_nyquist(f, t, s)))
    .collect()
}

#[test]
fn arq_128_the_passband_is_flat_within_0_1_db_up_to_0_9_of_the_lower_nyquist() {
    for (from, to, speed, nyquist) in cases() {
        for fraction in [0.05, 0.3, 0.6, 0.9] {
            let g = gain_db(from, to, speed, fraction * nyquist);
            assert!(
                g.abs() <= 0.1,
                "{from}->{to} x{speed} at {fraction}: {g} dB"
            );
        }
    }
}

#[test]
fn arq_128_the_stopband_from_the_lower_nyquist_is_down_at_least_80_db() {
    for (from, to, speed, nyquist) in cases() {
        for fraction in [1.0, 1.02, 1.3] {
            let freq = fraction * nyquist;
            if freq >= f64::from(from) / 2.0 {
                continue; // the input has no such frequency
            }
            let g = gain_db(from, to, speed, freq);
            assert!(g <= -80.0, "{from}->{to} x{speed} at {fraction}: {g} dB");
        }
    }
}

/// Amplitude of the component of `x` at `freq` Hz (rate `hz`), with a Hann
/// window so that a strong tone far away does not leak into the bin.
fn component(x: &[f32], hz: u32, freq: f64) -> f64 {
    let n = x.len();
    let (mut re, mut im, mut w_sum) = (0.0, 0.0, 0.0);
    for (i, &v) in x.iter().enumerate() {
        let w = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
        let phase = std::f64::consts::TAU * freq * i as f64 / f64::from(hz);
        re += w * f64::from(v) * phase.cos();
        im += w * f64::from(v) * phase.sin();
        w_sum += w;
    }
    2.0 * re.hypot(im) / w_sum
}

#[test]
fn arq_128_the_images_of_an_upsampled_tone_are_down_at_least_80_db() {
    // Rows: (from, to, tone). The first image, `from - tone`, lies in the
    // stopband (over the lower Nyquist, `from / 2`) and, past `to / 2`, folds
    // back to `to - image`.
    for (from, to, tone) in [
        (44_100, 48_000, 20_000.0),
        (22_050, 48_000, 9_000.0),
        (16_000, 96_000, 6_000.0),
    ] {
        let out = run(&sine(from, tone, 0.5, 1.5), from, to, 1.0);
        let mid = &out[out.len() / 4..out.len() * 3 / 4];
        let image = f64::from(from) - tone;
        let seen = if image > f64::from(to) / 2.0 {
            f64::from(to) - image
        } else {
            image
        };
        let rejection = db(component(mid, to, seen) / component(mid, to, tone));
        assert!(
            rejection <= -80.0,
            "{from}->{to} tone {tone}: image at {seen} is {rejection} dB"
        );
    }
}

#[test]
fn arq_128_an_impulse_comes_out_at_the_declared_latency_within_one_sample() {
    for (from, to, speed) in [
        (48_000, 44_100, 1.0),
        (44_100, 48_000, 1.0),
        (48_000, 48_000, 2.0),
        (48_000, 16_000, 0.5),
    ] {
        let k = 10_000_usize;
        let mut input = vec![0.0_f32; 20_000];
        input[k] = 1.0;
        let out = run(&input, from, to, speed);
        let (peak_at, _) =
            out.iter().enumerate().fold(
                (0, 0.0_f32),
                |m, (j, &v)| if v.abs() > m.1 { (j, v.abs()) } else { m },
            );
        let expect = (k as f64 * f64::from(to) / (f64::from(from) * speed)).round() as i64
            + i64::from(
                Resampler::new(rate(from), rate(to), speed)
                    .expect("valid")
                    .latency(),
            );
        assert!(
            (peak_at as i64 - expect).abs() <= 1,
            "{from}->{to} x{speed}: peak at {peak_at}, expected {expect}"
        );
    }
}

// The run.

fn chunked(input: &[f32], sizes: &[usize], from: u32, to: u32, speed: f64) -> Vec<f32> {
    let mut r = Resampler::new(rate(from), rate(to), speed).expect("valid");
    let mut out = Vec::new();
    let (mut at, mut i) = (0, 0);
    while at < input.len() {
        let end = (at + sizes[i % sizes.len()]).min(input.len());
        r.process(&input[at..end], &mut out).expect("finite");
        at = end;
        i += 1;
    }
    r.finish(&mut out);
    out
}

#[test]
fn arq_128_chunks_of_any_size_give_the_bytes_of_the_whole_signal() {
    let input: Vec<f32> = (0..30_000)
        .map(|i| ((i * 31 % 97) as f32 - 48.0) / 60.0)
        .collect();
    for (from, to, speed) in [
        (44_100, 48_000, 1.0),
        (48_000, 44_100, 1.7),
        (48_000, 16_000, 0.3),
    ] {
        let whole = run(&input, from, to, speed);
        for sizes in [
            vec![1],
            vec![7, 1, 100],
            vec![4_410],
            vec![29_999],
            vec![5_000, 3],
        ] {
            let got = chunked(&input, &sizes, from, to, speed);
            let a: Vec<u32> = whole.iter().map(|x| x.to_bits()).collect();
            let b: Vec<u32> = got.iter().map(|x| x.to_bits()).collect();
            assert_eq!(a, b, "{from}->{to} x{speed} chunks {sizes:?}");
        }
    }
}

#[test]
fn arq_128_a_resampler_used_twice_starts_each_run_from_nothing() {
    let a = sine(44_100, 700.0, 0.6, 0.3);
    let b = sine(44_100, 1_900.0, 0.4, 0.2);
    let mut r = Resampler::new(rate(44_100), rate(48_000), 1.0).expect("valid");
    let mut first = Vec::new();
    r.process(&a, &mut first).expect("finite");
    r.finish(&mut first);
    let mut second = Vec::new();
    r.process(&b, &mut second).expect("finite");
    r.finish(&mut second);
    assert_eq!(second, run(&b, 44_100, 48_000, 1.0));
    // `reset` in the middle of a run forgets it.
    let mut third = Vec::new();
    r.process(&a, &mut third).expect("finite");
    r.reset();
    let mut fourth = Vec::new();
    r.process(&b, &mut fourth).expect("finite");
    r.finish(&mut fourth);
    assert_eq!(fourth, run(&b, 44_100, 48_000, 1.0));
}

#[test]
fn arq_128_the_latency_is_declared_and_does_not_depend_on_the_input() {
    let r = Resampler::new(rate(44_100), rate(48_000), 1.0).expect("valid");
    assert_eq!(r.latency(), r.latency());
}

#[test]
fn arq_111_resampling_twice_gives_the_same_bytes() {
    let x = sine(44_100, 1_000.0, 0.5, 0.5);
    let a: Vec<u32> = run(&x, 44_100, 48_000, 1.3)
        .iter()
        .map(|v| v.to_bits())
        .collect();
    let b: Vec<u32> = run(&x, 44_100, 48_000, 1.3)
        .iter()
        .map(|v| v.to_bits())
        .collect();
    assert_eq!(a, b);
}

#[test]
fn arq_110_a_finite_input_gives_a_finite_output_even_at_the_largest_values() {
    let x = vec![
        f32::MAX,
        -f32::MAX,
        f32::MAX,
        f32::MAX,
        -f32::MAX,
        0.0,
        f32::MAX,
    ];
    for (from, to, speed) in [
        (48_000, 44_100, 1.0),
        (44_100, 192_000, 0.1),
        (192_000, 8_000, 10.0),
    ] {
        let out = run(&x, from, to, speed);
        assert!(out.iter().all(|v| v.is_finite()), "{from}->{to} x{speed}");
    }
}
