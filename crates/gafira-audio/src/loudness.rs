//! Integrated loudness, true peak and the Nivelar gain (ARQ-126, ARQ-127).
//!
//! These are analysis functions over whole signals; none of them takes a
//! cancellation token (ARQ-112, D-43). They take planar slices: 1, 2 or 6
//! channels, the six in the order L, R, C, LFE, Ls, Rs.

use std::sync::OnceLock;

use gafira_time::SampleRate;

use crate::AudioError;
use crate::dsp::bessel_i0;
use crate::error::finite;

/// Absolute gate of BS.1770-4, in LUFS. A block must be louder than this.
const ABSOLUTE_GATE: f64 = -70.0;
/// The relative gate sits this many LU under the mean of the blocks that
/// passed the absolute gate.
const RELATIVE_GATE: f64 = -10.0;
/// Centre of the head-related shelf of the K weighting, in Hz.
const SHELF_HZ: f64 = 1_681.974_450_955_533;
/// Weight of the surround channels (Ls, Rs).
const SURROUND_WEIGHT: f64 = 1.41;

/// A second-order section in transposed direct form II, in `f64`.
#[derive(Clone, Copy)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

impl Biquad {
    fn step(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

/// The K weighting is always run at this rate or above, so that the bilinear
/// transform keeps the shelf close to its analogue shape (ARQ-126).
const WEIGHTING_MIN_HZ: u32 = 32_000;
/// Half length, in input samples, of the interpolation filter that brings a
/// lower rate up to [`WEIGHTING_MIN_HZ`]: 216 taps per output sample.
const UP_HALF: usize = 108;
/// Kaiser parameter of that filter (about 70 dB of stopband).
const UP_BETA: f64 = 7.0;
/// Entries per input sample of its kernel table, read by linear interpolation.
const UP_RESOLUTION: usize = 1_024;

/// Integer factor that takes `hz` to [`WEIGHTING_MIN_HZ`] or above.
fn upsampling(hz: u32) -> usize {
    WEIGHTING_MIN_HZ.div_ceil(hz).max(1) as usize
}

/// `sinc(t) × kaiser(t / UP_HALF)` at `k / UP_RESOLUTION`; it is even and
/// zero past `UP_HALF`. Cuts at half the input rate.
fn up_kernel() -> &'static [f64] {
    static TABLE: OnceLock<Vec<f64>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let norm = bessel_i0(UP_BETA);
        (0..=UP_HALF * UP_RESOLUTION + 1)
            .map(|k| {
                let t = k as f64 / UP_RESOLUTION as f64;
                let u = (t / UP_HALF as f64).min(1.0);
                let sinc = if k == 0 {
                    1.0
                } else {
                    (std::f64::consts::PI * t).sin() / (std::f64::consts::PI * t)
                };
                sinc * bessel_i0(UP_BETA * (1.0 - u * u).sqrt()) / norm
            })
            .collect()
    })
}

/// The signal `phase / factor` of a sample after `x[n]`, interpolated from
/// the samples around it (zero outside the signal).
fn upsampled(samples: &[f32], n: usize, phase: usize, factor: usize) -> f64 {
    if phase == 0 {
        return f64::from(samples[n]);
    }
    let table = up_kernel();
    let frac = phase as f64 / factor as f64;
    let mut sum = 0.0;
    for j in 0..2 * UP_HALF {
        // x[n + j + 1 - HALF], zero outside the signal.
        let Some(i) = (n + j + 1)
            .checked_sub(UP_HALF)
            .filter(|&i| i < samples.len())
        else {
            continue;
        };
        let at = (frac + (UP_HALF - 1) as f64 - j as f64).abs() * UP_RESOLUTION as f64;
        let k = at as usize;
        let w = at - k as f64;
        sum += f64::from(samples[i]) * (table[k] * (1.0 - w) + table[k + 1] * w);
    }
    sum
}

/// The two stages of the K weighting at `hz`: the head-related high shelf and
/// the RLB high pass. The analogue prototype of BS.1770-4 gives the published
/// 48 kHz coefficients through the bilinear transform and any other rate its
/// own set.
fn k_weighting(hz: u32) -> [Biquad; 2] {
    let fs = f64::from(hz);
    let (f0, gain_db, q) = (SHELF_HZ, 3.999_843_853_973_347, 0.707_175_236_955_419_6);
    let k = (std::f64::consts::PI * f0 / fs).tan();
    let vh = 10_f64.powf(gain_db / 20.0);
    let vb = vh.powf(0.499_666_774_154_541_6);
    let a0 = 1.0 + k / q + k * k;
    let shelf = Biquad {
        b: [
            (vh + vb * k / q + k * k) / a0,
            2.0 * (k * k - vh) / a0,
            (vh - vb * k / q + k * k) / a0,
        ],
        a: [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
        z: [0.0; 2],
    };
    let (f0, q) = (38.135_470_876_024_44, 0.500_327_037_323_877_3);
    let k = (std::f64::consts::PI * f0 / fs).tan();
    let a0 = 1.0 + k / q + k * k;
    let high_pass = Biquad {
        b: [1.0, -2.0, 1.0],
        a: [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
        z: [0.0; 2],
    };
    [shelf, high_pass]
}

/// Channel weights for a layout, or the error for a count other than 1, 2, 6.
fn weights(channels: usize) -> Result<&'static [f64], AudioError> {
    match channels {
        1 => Ok(&[1.0]),
        2 => Ok(&[1.0, 1.0]),
        6 => Ok(&[1.0, 1.0, 1.0, 0.0, SURROUND_WEIGHT, SURROUND_WEIGHT]),
        n => Err(AudioError::ChannelCount { channels: n }),
    }
}

/// Checks the layout, the lengths and every sample before anything is computed
/// (ARQ-110). Returns the frame count.
fn check(channels: &[&[f32]]) -> Result<usize, AudioError> {
    weights(channels.len())?;
    let frames = channels[0].len();
    if channels.iter().any(|c| c.len() != frames) {
        return Err(AudioError::OutOfRange { field: "channels" });
    }
    if channels.iter().any(|c| c.iter().any(|x| !x.is_finite())) {
        return Err(AudioError::NonFinite { field: "samples" });
    }
    Ok(frames)
}

/// Loudness of a mean-square energy, as BS.1770-4 defines it.
fn loudness_of(energy: f64) -> f64 {
    -0.691 + 10.0 * energy.log10()
}

/// Integrated loudness in LUFS (ITU-R BS.1770-4, EBU R128), or `None` for
/// "no measure": silence, or a signal with no complete 400 ms block.
///
/// Blocks last 400 ms and start every 100 ms (75 % overlap); a block counts
/// only when it fits in the signal. A rate whose 100 ms is not a whole number
/// of samples rounds it to the nearest sample, and the block is 4 of them.
/// Every `SampleRate` is accepted. Under 32 kHz the signal is interpolated up
/// by an integer factor to 32 kHz or more before the K weighting, so the
/// weighting follows the 48 kHz one at every rate; the blocks stay of 400 ms
/// of the original signal.
///
/// # Errors
///
/// [`AudioError::ChannelCount`] for a count other than 1, 2 or 6,
/// [`AudioError::OutOfRange`] naming `channels` when their lengths differ and
/// [`AudioError::NonFinite`] naming `samples` for NaN or `±∞`.
pub fn integrated_loudness(
    channels: &[&[f32]],
    rate: SampleRate,
) -> Result<Option<f64>, AudioError> {
    let frames = check(channels)?;
    let weight = weights(channels.len())?;
    let factor = upsampling(rate.hz());
    let step = ((u64::from(rate.hz()) + 5) / 10).max(1) as usize;
    let chunks = frames / step;
    if chunks < 4 {
        return Ok(None);
    }
    // Weighted energy of each 100 ms chunk (of the interpolated signal); a
    // block is 4 chunks.
    let mut chunk_energy = vec![0.0_f64; chunks];
    for (samples, &w) in channels.iter().zip(weight) {
        if w == 0.0 {
            continue;
        }
        let mut filter = k_weighting(rate.hz() * factor as u32);
        for n in 0..chunks * step {
            for phase in 0..factor {
                let shelved = filter[0].step(upsampled(samples, n, phase, factor));
                let y = filter[1].step(shelved);
                chunk_energy[n / step] += w * y * y;
            }
        }
    }
    let block = (4 * step * factor) as f64;
    let blocks: Vec<f64> = chunk_energy
        .windows(4)
        .map(|w| (w[0] + w[1] + w[2] + w[3]) / block)
        .collect();
    integrated_from_blocks(&blocks)
}

/// The gated integrated loudness of block energies: the channel-weighted mean
/// square of each K-weighted 400 ms block. A block over the absolute gate of
/// -70 LUFS counts; the mean energy of those gives the relative gate, 10 LU
/// under it; the result is the loudness of the mean energy of the blocks over
/// both. A block exactly at a gate is under it (BS.1770-4 compares with `>`).
///
/// # Errors
///
/// [`AudioError::NonFinite`] naming `energies` for NaN or `±∞`, or when their
/// sum overflows, and
/// [`AudioError::OutOfRange`] naming `energies` for a negative one.
pub fn integrated_from_blocks(energies: &[f64]) -> Result<Option<f64>, AudioError> {
    for &e in energies {
        if finite("energies", e)? < 0.0 {
            return Err(AudioError::OutOfRange { field: "energies" });
        }
    }
    let passed = louder_than(energies, ABSOLUTE_GATE);
    let Some(mean) = mean(&passed)? else {
        return Ok(None);
    };
    let kept = louder_than(&passed, loudness_of(mean) + RELATIVE_GATE);
    Ok(self::mean(&kept)?.map(loudness_of))
}

/// The energies whose loudness is above `gate`.
fn louder_than(energies: &[f64], gate: f64) -> Vec<f64> {
    energies
        .iter()
        .copied()
        .filter(|&e| loudness_of(e) > gate)
        .collect()
}

/// Mean of the energies, if there are any. A sum that overflows is an error,
/// not a mean that would gate everything out (ARQ-110).
fn mean(energies: &[f64]) -> Result<Option<f64>, AudioError> {
    if energies.is_empty() {
        return Ok(None);
    }
    let sum = finite("energies", energies.iter().sum::<f64>())?;
    Ok(Some(sum / energies.len() as f64))
}

/// Half the taps of one interpolation phase (24 taps per phase).
const PEAK_HALF_TAPS: usize = 12;
/// Kaiser parameter of the interpolation window.
const PEAK_KAISER_BETA: f64 = 8.0;
/// The ceiling Nivelar keeps the true peak under, in dBTP.
const PEAK_CEILING_DBTP: f64 = -1.0;

/// Windowed-sinc taps that read the signal `phase / factor` of a sample after
/// `x[n]`, for `x[n - HALF + 1 ..= n + HALF]`; their sum is 1.
fn interpolation_taps(phase: usize, factor: usize) -> Vec<f64> {
    let frac = phase as f64 / factor as f64;
    let half = PEAK_HALF_TAPS as f64;
    let norm = bessel_i0(PEAK_KAISER_BETA);
    let mut taps: Vec<f64> = (0..2 * PEAK_HALF_TAPS)
        .map(|j| {
            let t = j as f64 - half + 1.0 - frac;
            let sinc = (std::f64::consts::PI * t).sin() / (std::f64::consts::PI * t);
            let window = bessel_i0(PEAK_KAISER_BETA * (1.0 - (t / half).powi(2)).sqrt()) / norm;
            sinc * window
        })
        .collect();
    let sum: f64 = taps.iter().sum();
    for tap in &mut taps {
        *tap /= sum;
    }
    taps
}

/// Oversampling factor of BS.1770-4 Annex 2: 4 up to 48 kHz, 2 up to 96 kHz,
/// none above.
fn oversampling(hz: u32) -> usize {
    match hz {
        0..=48_000 => 4,
        48_001..=96_000 => 2,
        _ => 1,
    }
}

/// True peak of the whole signal, linear, over all channels (LFE included):
/// the largest absolute value of the signal oversampled as BS.1770-4 Annex 2
/// asks: x4 up to 48 kHz (the rule of ARQ-127), x2 up to 96 kHz and none
/// above. The signal is taken as zero outside its ends.
///
/// # Errors
///
/// As [`integrated_loudness`].
pub fn true_peak(channels: &[&[f32]], rate: SampleRate) -> Result<f64, AudioError> {
    let frames = check(channels)?;
    let factor = oversampling(rate.hz());
    let taps: Vec<Vec<f64>> = (1..factor).map(|p| interpolation_taps(p, factor)).collect();
    let mut peak = 0.0_f64;
    for samples in channels {
        for (n, &x) in samples.iter().enumerate() {
            peak = peak.max(f64::from(x).abs());
            for phase in &taps {
                let mut y = 0.0;
                for (j, &tap) in phase.iter().enumerate() {
                    // x[n + j - HALF + 1], zero outside the signal.
                    let at = (n + j + 1).checked_sub(PEAK_HALF_TAPS);
                    if let Some(v) = at.filter(|&i| i < frames).map(|i| samples[i]) {
                        y += tap * f64::from(v);
                    }
                }
                peak = peak.max(y.abs());
            }
        }
    }
    Ok(peak)
}

/// The gain Nivelar applies and whether it reached the target (ARQ-127).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leveling {
    /// Constant gain to apply to the whole signal, in dB.
    pub gain_db: f64,
    /// True when the target was reached. The ceiling equal to the request
    /// reaches it.
    pub reached: bool,
}

/// The Nivelar gain from the measures: the request is `target − measured`,
/// the ceiling is `−1 dBTP − peak`, and the gain is the smaller of the two.
/// Without a measure the gain is 0 and the target is not reached; without a
/// peak there is no ceiling.
///
/// # Errors
///
/// [`AudioError::NonFinite`] naming `target`, `measured` or `peak`, or `gain`
/// when the request overflows.
pub fn plan_leveling(
    target_lufs: f64,
    measured_lufs: Option<f64>,
    true_peak_dbtp: Option<f64>,
) -> Result<Leveling, AudioError> {
    finite("target", target_lufs)?;
    if let Some(m) = measured_lufs {
        finite("measured", m)?;
    }
    if let Some(p) = true_peak_dbtp {
        finite("peak", p)?;
    }
    let Some(measured) = measured_lufs else {
        return Ok(Leveling {
            gain_db: 0.0,
            reached: false,
        });
    };
    let request = finite("gain", target_lufs - measured)?;
    let Some(peak) = true_peak_dbtp else {
        return Ok(Leveling {
            gain_db: request,
            reached: true,
        });
    };
    let ceiling = finite("gain", PEAK_CEILING_DBTP - peak)?;
    Ok(Leveling {
        gain_db: request.min(ceiling),
        reached: request <= ceiling,
    })
}

/// Measures the signal and plans its Nivelar gain.
///
/// # Errors
///
/// As [`integrated_loudness`], and [`AudioError::NonFinite`] naming `target`.
pub fn level(
    channels: &[&[f32]],
    rate: SampleRate,
    target_lufs: f64,
) -> Result<Leveling, AudioError> {
    finite("target", target_lufs)?;
    let measured = integrated_loudness(channels, rate)?;
    let peak = true_peak(channels, rate)?;
    let peak_db = (peak > 0.0).then(|| 20.0 * peak.log10());
    plan_leveling(target_lufs, measured, peak_db)
}
