//! ARQ-125, FV-20 (c): `Eq3Band`, three RBJ biquads at 100 Hz (low shelf,
//! `S = 1`), 1 kHz (peak, `Q = 1`) and 8 kHz (high shelf, `S = 1`), +-15 dB.

use std::f64::consts::{FRAC_1_SQRT_2, PI};
use std::ops::{Add, Div, Mul};

use gafira_audio::{AudioBlock, AudioProcessor, Eq3Band, ParamSnapshot};

const BANDS: [(&str, f64); 3] = [("low", 100.0), ("mid", 1000.0), ("high", 8000.0)];
const RATES: [u32; 3] = [44_100, 48_000, 96_000];

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

fn eq(rate: u32, channels: u16) -> Eq3Band {
    let mut e = Eq3Band::new();
    e.prepare(rate, 4096, channels);
    e
}

/// Deterministic noise in `[-4, 4]`, never subnormal.
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

/// The impulse response of a mono processor, `n` samples, in blocks of 4096.
fn impulse_response(p: &mut dyn AudioProcessor, params: &ParamSnapshot, n: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let len = 4096.min(n - out.len());
        let mut block = vec![vec![0.0_f32; len]];
        if out.is_empty() {
            block[0][0] = 1.0;
        }
        run(p, &mut block, params);
        out.extend_from_slice(&block[0]);
    }
    out
}

#[derive(Clone, Copy)]
struct C {
    re: f64,
    im: f64,
}

impl C {
    fn real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }
}

impl Add for C {
    type Output = C;
    fn add(self, o: C) -> C {
        C {
            re: self.re + o.re,
            im: self.im + o.im,
        }
    }
}

impl Mul for C {
    type Output = C;
    fn mul(self, o: C) -> C {
        C {
            re: self.re * o.re - self.im * o.im,
            im: self.re * o.im + self.im * o.re,
        }
    }
}

impl Div for C {
    type Output = C;
    fn div(self, o: C) -> C {
        let d = o.re * o.re + o.im * o.im;
        C {
            re: (self.re * o.re + self.im * o.im) / d,
            im: (self.im * o.re - self.re * o.im) / d,
        }
    }
}

/// `|H(e^jw)|` in dB at `f` Hz, from the analogue prototypes of the Cookbook
/// through the bilinear transform with `f0` pre-warped. A band at or above
/// `fs / 2` is omitted and 0 dB is unity.
fn theory_db(fs: f64, gains: [f64; 3], f: f64) -> f64 {
    let mut total = C::real(1.0);
    for (k, &(_, f0)) in BANDS.iter().enumerate() {
        if gains[k] == 0.0 || f0 >= fs / 2.0 {
            continue;
        }
        let a = 10_f64.powf(gains[k] / 40.0);
        let s = C {
            re: 0.0,
            im: (PI * f / fs).tan() / (PI * f0 / fs).tan(),
        };
        let s2 = s * s;
        let one = C::real(1.0);
        let (a_c, a_inv) = (C::real(a), C::real(1.0 / a));
        // Shelf slope S = 1 gives 1/Q = sqrt(2); the shelf term is sqrt(A)/Q.
        let shelf = C::real(a.sqrt() / FRAC_1_SQRT_2) * s;
        let h = match k {
            0 => a_c * (s2 + shelf + a_c) / (a_c * s2 + shelf + one),
            // Peak with Q = 1.
            1 => (s2 + a_c * s + one) / (s2 + a_inv * s + one),
            _ => a_c * (a_c * s2 + shelf + one) / (s2 + shelf + a_c),
        };
        total = total * h;
    }
    20.0 * total.re.hypot(total.im).log10()
}

/// `|H|` in dB at `f` Hz from an impulse response (a direct DFT bin).
fn measured_db(h: &[f32], fs: f64, f: f64) -> f64 {
    let w = 2.0 * PI * f / fs;
    let (c, s) = (w.cos(), -w.sin());
    let (mut rr, mut ri) = (1.0_f64, 0.0_f64);
    let (mut re, mut im) = (0.0_f64, 0.0_f64);
    for (n, &v) in h.iter().enumerate() {
        re += f64::from(v) * rr;
        im += f64::from(v) * ri;
        (rr, ri) = (rr * c - ri * s, rr * s + ri * c);
        if n % 1024 == 1023 {
            let m = rr.hypot(ri);
            (rr, ri) = (rr / m, ri / m);
        }
    }
    20.0 * re.hypot(im).log10()
}

fn log_freqs(n: usize, fs: f64) -> Vec<f64> {
    (0..n)
        .map(|i| 20.0 * (1000.0_f64).powf(i as f64 / (n - 1) as f64))
        .filter(|&f| f < fs / 2.0)
        .collect()
}

/// Every setting of the response tests: each band alone at +-15 and 0 dB,
/// and mixed settings.
fn settings() -> Vec<[f64; 3]> {
    let mut v = vec![[0.0; 3]];
    for k in 0..3 {
        for g in [15.0, -15.0] {
            let mut s = [0.0; 3];
            s[k] = g;
            v.push(s);
        }
    }
    v.push([15.0, -15.0, 15.0]);
    v.push([-15.0, 15.0, -15.0]);
    v.push([-7.0, 9.0, -3.0]);
    v
}

#[test]
fn arq_125_the_impulse_response_matches_h_within_half_a_db_from_20_hz_to_20_khz() {
    for fs in RATES {
        for gains in settings() {
            let mut e = eq(fs, 1);
            let h = impulse_response(&mut e, &snapshot(gains[0], gains[1], gains[2]), 1 << 14);
            for f in log_freqs(60, f64::from(fs)).into_iter().chain([20_000.0]) {
                let want = theory_db(f64::from(fs), gains, f);
                let got = measured_db(&h, f64::from(fs), f);
                assert!(
                    (got - want).abs() <= 0.5,
                    "fs {fs} gains {gains:?} f {f:.1}: got {got:.3} dB, want {want:.3} dB"
                );
            }
        }
    }
}

#[test]
fn fv_20_c_a_sine_sweep_follows_the_theoretical_curve() {
    // 1 s of settling and 1 s of measurement: whole cycles at integer Hz.
    let fs = 48_000_u32;
    let gains = [9.0, -12.0, 15.0];
    for f in [
        20_u32, 40, 100, 160, 250, 500, 800, 1000, 1600, 3000, 5000, 8000, 11_000, 14_000, 17_000,
        20_000,
    ] {
        let mut e = eq(fs, 1);
        let params = snapshot(gains[0], gains[1], gains[2]);
        let w = 2.0 * PI * f64::from(f) / f64::from(fs);
        let (mut sin_sum, mut cos_sum) = (0.0_f64, 0.0_f64);
        for second in 0..2_usize {
            for chunk in 0..(fs as usize / 4096 + 1) {
                let base = second * fs as usize + chunk * 4096;
                let len = if chunk == fs as usize / 4096 {
                    fs as usize % 4096
                } else {
                    4096
                };
                let mut block = vec![
                    (0..len)
                        .map(|i| (0.5 * (w * (base + i) as f64).sin()) as f32)
                        .collect::<Vec<f32>>(),
                ];
                run(&mut e, &mut block, &params);
                if second == 1 {
                    for (i, &y) in block[0].iter().enumerate() {
                        let t = w * (base + i) as f64;
                        sin_sum += f64::from(y) * t.sin();
                        cos_sum += f64::from(y) * t.cos();
                    }
                }
            }
        }
        let amplitude = 2.0 * sin_sum.hypot(cos_sum) / f64::from(fs);
        let got = 20.0 * (amplitude / 0.5).log10();
        let want = theory_db(f64::from(fs), gains, f64::from(f));
        assert!(
            (got - want).abs() <= 0.5,
            "f {f}: got {got:.3} dB, want {want:.3} dB"
        );
    }
}

#[test]
fn arq_125_with_every_band_at_0_db_the_output_is_bit_identical() {
    for fs in RATES {
        let mut e = eq(fs, 2);
        for round in 0..6_u64 {
            let len = [1, 7, 512, 4096, 100, 333][round as usize];
            let input = vec![noise(2 * round, len), noise(2 * round + 1, len)];
            let mut block = input.clone();
            run(&mut e, &mut block, &snapshot(0.0, 0.0, 0.0));
            assert_eq!(bits(&block), bits(&input), "fs {fs} round {round}");
        }
        // A huge sample followed by small ones: a filter that only looks like
        // the identity loses the small ones to cancellation.
        let spiky = vec![vec![1e30_f32, 1.0, 0.5, -3e-20, 7.0, -1e30, 1e-3, 0.25]; 2];
        let mut block = spiky.clone();
        run(&mut e, &mut block, &snapshot(0.0, 0.0, 0.0));
        assert_eq!(bits(&block), bits(&spiky), "fs {fs} spiky");
    }
}

#[test]
fn arq_125_every_pole_is_inside_the_unit_circle() {
    // The impulse response of a stable filter dies away.
    let extremes: Vec<[f64; 3]> = (0..8)
        .map(|m| [0, 1, 2].map(|k| if m >> k & 1 == 1 { 15.0 } else { -15.0 }))
        .collect();
    for fs in [8_000, 16_001, 22_050, 44_100, 48_000, 96_000, 192_000] {
        for gains in &extremes {
            let mut e = eq(fs, 1);
            let h = impulse_response(&mut e, &snapshot(gains[0], gains[1], gains[2]), 1 << 16);
            assert!(h.iter().all(|x| x.is_finite()), "fs {fs} {gains:?}");
            let window =
                |from: usize, to: usize| h[from..to].iter().fold(0.0_f32, |m, x| m.max(x.abs()));
            let (early, late) = (window(1 << 14, 1 << 15), window(1 << 15, 1 << 16));
            // A pole on or outside the circle keeps `late` at `early`. Near
            // Nyquist a pole at 0.9998 still halves it in 16 384 samples.
            assert!(
                late <= 1e-9 || late < 0.5 * early,
                "fs {fs} {gains:?}: {early} then {late}"
            );
        }
    }
}

#[test]
fn arq_125_a_band_at_or_above_half_the_sample_rate_is_omitted() {
    // At 16 000 Hz the 8 kHz band is at fs / 2; at 2 000 Hz the 1 kHz one is.
    let run_pair = |fs: u32, a: [f64; 3], b: [f64; 3]| {
        let (mut x, mut y) = (eq(fs, 1), eq(fs, 1));
        let input = vec![noise(7, 2048)];
        let (mut out_a, mut out_b) = (input.clone(), input);
        run(&mut x, &mut out_a, &snapshot(a[0], a[1], a[2]));
        run(&mut y, &mut out_b, &snapshot(b[0], b[1], b[2]));
        bits(&out_a) == bits(&out_b)
    };
    assert!(run_pair(16_000, [6.0, -4.0, 15.0], [6.0, -4.0, 0.0]));
    assert!(!run_pair(16_001, [6.0, -4.0, 15.0], [6.0, -4.0, 0.0]));
    assert!(run_pair(2_000, [6.0, -9.0, 15.0], [6.0, 0.0, 0.0]));
    assert!(!run_pair(2_001, [6.0, -9.0, 0.0], [6.0, 0.0, 0.0]));
    assert!(!run_pair(2_000, [6.0, 0.0, 0.0], [0.0, 0.0, 0.0]));
}
