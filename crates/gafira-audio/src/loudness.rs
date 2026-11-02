//! Integrated loudness (ARQ-126).
//!
//! These are analysis functions over whole signals; none of them takes a
//! cancellation token (ARQ-112, D-43). They take planar slices: 1, 2 or 6
//! channels, the six in the order L, R, C, LFE, Ls, Rs.

use gafira_time::SampleRate;

use crate::AudioError;
use crate::error::finite;

/// Absolute gate of BS.1770-4, in LUFS. A block must be louder than this.
const ABSOLUTE_GATE: f64 = -70.0;
/// The relative gate sits this many LU under the mean of the blocks that
/// passed the absolute gate.
const RELATIVE_GATE: f64 = -10.0;
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

/// The two stages of the K weighting at `hz`: the head-related high shelf and
/// the RLB high pass. The analogue prototype of BS.1770-4 gives the published
/// 48 kHz coefficients through the bilinear transform and any other rate its
/// own set.
fn k_weighting(hz: u32) -> [Biquad; 2] {
    let fs = f64::from(hz);
    let (f0, gain_db, q) = (
        1_681.974_450_955_533,
        3.999_843_853_973_347,
        0.707_175_236_955_419_6,
    );
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
    let step = ((u64::from(rate.hz()) + 5) / 10).max(1) as usize;
    let chunks = frames / step;
    if chunks < 4 {
        return Ok(None);
    }
    // Weighted energy of each 100 ms chunk; a block is 4 chunks.
    let mut chunk_energy = vec![0.0_f64; chunks];
    for (samples, &w) in channels.iter().zip(weight) {
        if w == 0.0 {
            continue;
        }
        let mut filter = k_weighting(rate.hz());
        for (i, &x) in samples[..chunks * step].iter().enumerate() {
            let shelved = filter[0].step(f64::from(x));
            let y = filter[1].step(shelved);
            chunk_energy[i / step] += w * y * y;
        }
    }
    let block = (4 * step) as f64;
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
/// [`AudioError::NonFinite`] naming `energies` for NaN or `±∞`, and
/// [`AudioError::OutOfRange`] naming `energies` for a negative one.
pub fn integrated_from_blocks(energies: &[f64]) -> Result<Option<f64>, AudioError> {
    for &e in energies {
        if finite("energies", e)? < 0.0 {
            return Err(AudioError::OutOfRange { field: "energies" });
        }
    }
    let passed = louder_than(energies, ABSOLUTE_GATE);
    let Some(mean) = mean(&passed) else {
        return Ok(None);
    };
    let kept = louder_than(&passed, loudness_of(mean) + RELATIVE_GATE);
    Ok(self::mean(&kept).map(loudness_of))
}

/// The energies whose loudness is above `gate`.
fn louder_than(energies: &[f64], gate: f64) -> Vec<f64> {
    energies
        .iter()
        .copied()
        .filter(|&e| loudness_of(e) > gate)
        .collect()
}

/// Mean of the energies, if there are any.
fn mean(energies: &[f64]) -> Option<f64> {
    (!energies.is_empty()).then(|| energies.iter().sum::<f64>() / energies.len() as f64)
}
