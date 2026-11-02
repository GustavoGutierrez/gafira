//! ARQ-125, ARQ-123, ARQ-111: the state of `Eq3Band` (biquad histories).

use std::f64::consts::PI;

use gafira_audio::{AudioBlock, AudioProcessor, Eq3Band, ParamSnapshot};

fn snapshot(low: f64, mid: f64, high: f64) -> ParamSnapshot {
    let mut s = ParamSnapshot::new();
    s.set("low", low);
    s.set("mid", mid);
    s.set("high", high);
    s
}

fn run(p: &mut dyn AudioProcessor, chans: &mut [Vec<f32>], params: &ParamSnapshot) {
    let mut slices: Vec<&mut [f32]> = chans.iter_mut().map(Vec::as_mut_slice).collect();
    p.process(&mut AudioBlock::new(&mut slices), params);
}

fn bits(chans: &[Vec<f32>]) -> Vec<Vec<u32>> {
    chans
        .iter()
        .map(|c| c.iter().map(|f| f.to_bits()).collect())
        .collect()
}

fn eq(rate: u32, max: usize, channels: u16) -> Eq3Band {
    let mut e = Eq3Band::new();
    e.prepare(rate, max, channels);
    e
}

fn noise(seed: u64, n: usize) -> Vec<f32> {
    let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let unit = (state >> 40) as f32 / (1u64 << 24) as f32;
            (unit - 0.5) * 8.0 + 0.001
        })
        .collect()
}

fn impulse(n: usize) -> Vec<Vec<f32>> {
    let mut v = vec![0.0_f32; n];
    v[0] = 1.0;
    vec![v]
}

const GAINS: (f64, f64, f64) = (9.0, -12.0, 15.0);

#[test]
fn arq_111_reset_clears_the_history_of_every_band() {
    let (l, m, h) = GAINS;
    let mut used = eq(48_000, 512, 1);
    let mut block = vec![noise(1, 512)];
    run(&mut used, &mut block, &snapshot(l, m, h));
    used.reset();
    let mut after = impulse(512);
    run(&mut used, &mut after, &snapshot(l, m, h));
    let mut fresh_out = impulse(512);
    run(&mut eq(48_000, 512, 1), &mut fresh_out, &snapshot(l, m, h));
    assert_eq!(bits(&after), bits(&fresh_out));
}

#[test]
fn arq_111_prepare_clears_the_history_of_every_band() {
    let (l, m, h) = GAINS;
    let mut used = eq(48_000, 512, 1);
    let mut block = vec![noise(2, 512)];
    run(&mut used, &mut block, &snapshot(l, m, h));
    used.prepare(48_000, 512, 1);
    let mut after = impulse(512);
    run(&mut used, &mut after, &snapshot(l, m, h));
    let mut fresh_out = impulse(512);
    run(&mut eq(48_000, 512, 1), &mut fresh_out, &snapshot(l, m, h));
    assert_eq!(bits(&after), bits(&fresh_out));
}

#[test]
fn arq_125_each_channel_has_its_own_history() {
    let (l, m, h) = GAINS;
    let params = snapshot(l, m, h);
    let (left, right) = (noise(3, 700), noise(4, 700));
    let mut stereo = vec![left.clone(), right.clone()];
    let mut both = eq(44_100, 1024, 2);
    run(&mut both, &mut stereo, &params);
    for (c, input) in [left, right].into_iter().enumerate() {
        let mut mono = vec![input];
        run(&mut eq(44_100, 1024, 1), &mut mono, &params);
        assert_eq!(bits(&stereo[c..=c]), bits(&mono), "channel {c}");
    }
    // The same on a second block: the histories moved on per channel.
    let (left, right) = (noise(5, 300), vec![0.0_f32; 300]);
    let mut stereo = vec![left, right];
    run(&mut both, &mut stereo, &params);
    let mut quiet = eq(44_100, 1024, 1);
    let mut warm = vec![noise(4, 700)];
    run(&mut quiet, &mut warm, &params);
    let mut silent = vec![vec![0.0_f32; 300]];
    run(&mut quiet, &mut silent, &params);
    assert_eq!(bits(&stereo[1..2]), bits(&silent));
}

/// The cookbook biquads in `f64`, direct form I, one channel. A band at 0 dB
/// (or at or above `fs / 2`) passes the sample and its histories follow it.
struct Reference {
    fs: f64,
    hist: [[f64; 4]; 3],
}

impl Reference {
    fn new(fs: f64) -> Self {
        Self {
            fs,
            hist: [[0.0; 4]; 3],
        }
    }

    fn coefficients(&self, band: usize, db: f64) -> Option<[f64; 5]> {
        let f0 = [100.0, 1000.0, 8000.0][band];
        if db == 0.0 || f0 >= self.fs / 2.0 {
            return None;
        }
        let a = 10_f64.powf(db / 40.0);
        let w0 = 2.0 * PI * f0 / self.fs;
        let (cos, sin) = (w0.cos(), w0.sin());
        let (b, den) = match band {
            1 => {
                let alpha = sin / 2.0;
                (
                    [1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a],
                    [1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a],
                )
            }
            _ => {
                let t = 2.0 * a.sqrt() * (sin / 2.0 * 2_f64.sqrt());
                if band == 0 {
                    (
                        [
                            a * ((a + 1.0) - (a - 1.0) * cos + t),
                            2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                            a * ((a + 1.0) - (a - 1.0) * cos - t),
                        ],
                        [
                            (a + 1.0) + (a - 1.0) * cos + t,
                            -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                            (a + 1.0) + (a - 1.0) * cos - t,
                        ],
                    )
                } else {
                    (
                        [
                            a * ((a + 1.0) + (a - 1.0) * cos + t),
                            -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                            a * ((a + 1.0) + (a - 1.0) * cos - t),
                        ],
                        [
                            (a + 1.0) - (a - 1.0) * cos + t,
                            2.0 * ((a - 1.0) - (a + 1.0) * cos),
                            (a + 1.0) - (a - 1.0) * cos - t,
                        ],
                    )
                }
            }
        };
        Some([
            b[0] / den[0],
            b[1] / den[0],
            b[2] / den[0],
            den[1] / den[0],
            den[2] / den[0],
        ])
    }

    fn process(&mut self, input: &[f32], gains: [f64; 3]) -> Vec<f64> {
        let coeffs: Vec<Option<[f64; 5]>> =
            (0..3).map(|k| self.coefficients(k, gains[k])).collect();
        input
            .iter()
            .map(|&x| {
                let mut v = f64::from(x);
                for (k, c) in coeffs.iter().enumerate() {
                    let [x1, x2, y1, y2] = self.hist[k];
                    let y = match c {
                        Some([b0, b1, b2, a1, a2]) => {
                            b0 * v + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2
                        }
                        None => v,
                    };
                    self.hist[k] = [v, x1, y, y1];
                    v = y;
                }
                v
            })
            .collect()
    }
}

fn close(got: &[f32], want: &[f64], context: &str) {
    for (i, (&g, &w)) in got.iter().zip(want).enumerate() {
        assert!(
            (f64::from(g) - w).abs() <= 1e-4 * w.abs().max(1.0),
            "{context} sample {i}: got {g}, want {w}"
        );
    }
}

#[test]
fn arq_125_a_gain_change_applies_from_the_next_block_and_keeps_the_history() {
    // Settings that run through 0 dB, a change of one band and several at once.
    let steps = [
        [0.0, 0.0, 0.0],
        [0.0, 12.0, 0.0],
        [6.0, -6.0, 15.0],
        [6.0, -6.0, -9.0],
        [0.0, 0.0, 0.0],
        [-15.0, 15.0, 0.0],
    ];
    for fs in [44_100_u32, 96_000] {
        let mut e = eq(fs, 512, 1);
        let mut reference = Reference::new(f64::from(fs));
        for (n, gains) in steps.into_iter().enumerate() {
            let input = noise(10 + n as u64, 300);
            let mut block = vec![input.clone()];
            run(&mut e, &mut block, &snapshot(gains[0], gains[1], gains[2]));
            let want = reference.process(&input, gains);
            close(&block[0], &want, &format!("fs {fs} step {n}"));
        }
    }
}

#[test]
fn arq_123_the_history_is_flushed_below_the_smallest_normal_sample() {
    // After an impulse the mid band rings down. Whatever is left in the
    // history below `f32::MIN_POSITIVE` is silence: from the sample where the
    // ring-down is below it (plus 100 samples, because a flushed sample bends
    // the tail a little), a probe of tiny normal samples must come out as it
    // does from a fresh processor, however long the silence before it.
    let gains = [0.0, 15.0, 0.0];
    let params = snapshot(gains[0], gains[1], gains[2]);
    let mut impulse_in = vec![0.0_f32; 4000];
    impulse_in[0] = 1.0;
    let ring = Reference::new(44_100.0).process(&impulse_in, gains);
    let below = f64::from(f32::MIN_POSITIVE);
    let quiet_from = ring.iter().rposition(|y| y.abs() >= below).unwrap() + 1;
    assert!(quiet_from < 3_000, "the ring-down must end inside the test");
    let probe = vec![vec![4.0 * f32::MIN_POSITIVE; 16]];
    let mut fresh = probe.clone();
    run(&mut eq(44_100, 4096, 1), &mut fresh, &params);
    for silence in quiet_from + 100..quiet_from + 300 {
        let mut e = eq(44_100, 4096, 1);
        let mut rung = impulse(silence);
        run(&mut e, &mut rung, &params);
        let mut after = probe.clone();
        run(&mut e, &mut after, &params);
        assert_eq!(bits(&after), bits(&fresh), "silence {silence}");
    }
}

#[test]
fn arq_125_a_band_omitted_at_the_new_rate_does_nothing_after_prepare() {
    // The high shelf (8 kHz) is designed at 48 kHz, then `prepare` moves to
    // 16 kHz, where it sits at `fs / 2` and is omitted: the value stays the
    // last valid one, the design is redone, and the output equals the input.
    let mut e = eq(48_000, 256, 1);
    let mut block = vec![noise(5, 256)];
    run(&mut e, &mut block, &snapshot(0.0, 0.0, 15.0));
    e.prepare(16_000, 256, 1);
    let input = noise(6, 256);
    let mut out = vec![input.clone()];
    run(&mut e, &mut out, &ParamSnapshot::new());
    assert_eq!(bits(&out), bits(&[input]));
}
