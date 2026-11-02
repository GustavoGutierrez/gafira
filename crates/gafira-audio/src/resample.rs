//! Sample-rate conversion with a speed factor (ARQ-128).
//!
//! The resampler is a Kaiser-windowed sinc interpolator whose cutoff sits at
//! the lower of the two Nyquist frequencies. A clip played at `speed` is read
//! as if its rate were `from × speed`; the output has the rate `to`, so the
//! "lower Nyquist" of ARQ-128 is `min(from × speed, to) / 2` in output Hz.
//! The filter is centred (zero phase): output sample `j` is the signal at
//! input position `j × from × speed / to`, so the declared latency, as a shift
//! in time, is 0. Long operations take no cancellation token here (ARQ-112,
//! D-43); a caller that wants to cancel feeds [`Resampler::process`] in blocks.

use std::sync::OnceLock;

use gafira_time::SampleRate;

use crate::AudioError;
use crate::dsp::{bessel_i0, clean_output};
use crate::error::finite;

/// The smallest speed (FV-04), valid.
const MIN_SPEED: f64 = 0.1;
/// The greatest speed (FV-04), valid.
const MAX_SPEED: f64 = 10.0;
/// Where the passband ends and the stopband starts, as a share of the lower
/// Nyquist: 0.9 and 1.0 (ARQ-128). The cutoff is half way.
const CUTOFF: f64 = 0.95;
/// Half the length of the kernel in lobes of the sinc (`2 × cutoff × H`).
/// With a Kaiser window of beta 9.6 it gives a transition band of 0.1 of the
/// lower Nyquist and a stopband of about 96 dB.
const HALF_LOBES: f64 = 60.0;
/// Kaiser parameter for a 96 dB stopband.
const KAISER_BETA: f64 = 9.6;
/// Table entries per lobe; the kernel is read by linear interpolation.
const TABLE_RESOLUTION: usize = 2048;

/// `speed` finite and in `[0.1, 10]`, both ends valid.
fn check_speed(speed: f64) -> Result<(), AudioError> {
    finite("speed", speed)?;
    if (MIN_SPEED..=MAX_SPEED).contains(&speed) {
        Ok(())
    } else {
        Err(AudioError::OutOfRange { field: "speed" })
    }
}

/// Output length for `frames` input frames: `round(n × to / (from × speed))`,
/// evaluated in `f64` in that order and rounded half away from 0 (ARQ-128).
///
/// # Errors
///
/// [`AudioError::NonFinite`] naming `speed` for NaN or `±∞`,
/// [`AudioError::OutOfRange`] naming `speed` outside `[0.1, 10]`, or naming
/// `samples` when the length does not fit.
pub fn resampled_len(
    frames: usize,
    from: SampleRate,
    to: SampleRate,
    speed: f64,
) -> Result<usize, AudioError> {
    check_speed(speed)?;
    let n = (frames as f64 * f64::from(to.hz()) / (f64::from(from.hz()) * speed)).round();
    if n < 2_f64.powi(63) {
        Ok(n as usize)
    } else {
        Err(AudioError::OutOfRange { field: "samples" })
    }
}

/// `sinc(x) × kaiser(x / HALF_LOBES)` at `k / TABLE_RESOLUTION`, for
/// `k = 0 ..= HALF_LOBES × TABLE_RESOLUTION + 2`. The kernel is even.
fn kernel() -> &'static [f64] {
    static TABLE: OnceLock<Vec<f64>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let norm = bessel_i0(KAISER_BETA);
        let len = HALF_LOBES as usize * TABLE_RESOLUTION + 3;
        (0..len)
            .map(|k| {
                let x = k as f64 / TABLE_RESOLUTION as f64;
                let u = (x / HALF_LOBES).min(1.0);
                let sinc = if k == 0 {
                    1.0
                } else {
                    (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
                };
                sinc * bessel_i0(KAISER_BETA * (1.0 - u * u).sqrt()) / norm
            })
            .collect()
    })
}

/// A constant-ratio resampler. Feed it chunks with [`process`](Self::process)
/// and end the run with [`finish`](Self::finish); the bytes do not depend on
/// where the chunks end. The ratio is fixed when it is built.
#[derive(Debug)]
pub struct Resampler {
    from: SampleRate,
    to: SampleRate,
    speed: f64,
    /// Input frames per second of the clip once the speed applies.
    input_rate: f64,
    /// Twice the cutoff, in cycles per input sample.
    two_fc: f64,
    /// Half the kernel, in input samples.
    half: f64,
    /// The input samples still needed; `buf[0]` is input frame `base`.
    buf: Vec<f32>,
    base: u64,
    /// Input frames received in this run.
    total: u64,
    /// The next output frame.
    next_out: u64,
}

impl Resampler {
    /// Builds a resampler for `from` to `to` at `speed`.
    ///
    /// # Errors
    ///
    /// As [`resampled_len`] for the speed.
    pub fn new(from: SampleRate, to: SampleRate, speed: f64) -> Result<Self, AudioError> {
        check_speed(speed)?;
        let input_rate = f64::from(from.hz()) * speed;
        let ratio = f64::from(to.hz()) / input_rate;
        let two_fc = 2.0 * CUTOFF * 0.5 * ratio.min(1.0);
        Ok(Self {
            from,
            to,
            speed,
            input_rate,
            two_fc,
            half: HALF_LOBES / two_fc,
            buf: Vec::new(),
            base: 0,
            total: 0,
            next_out: 0,
        })
    }

    /// The delay in time the resampler adds, in output samples: 0, the filter
    /// is centred. (It does look ahead by about [`HALF_LOBES`] lobes of input,
    /// which `finish` pays at the end of the run.)
    #[must_use]
    pub fn latency(&self) -> u32 {
        0
    }

    /// Input position of output frame `j`.
    fn position(&self, j: u64) -> f64 {
        j as f64 * self.input_rate / f64::from(self.to.hz())
    }

    /// Output frame `j`, reading zeros past the end of the input.
    fn output(&self, j: u64) -> f32 {
        let pos = self.position(j);
        let table = kernel();
        let lo = (pos - self.half).ceil() as i64;
        let hi = (pos + self.half).floor() as i64;
        let mut acc = 0.0;
        for i in lo.max(self.base as i64)..=hi.min(self.total as i64 - 1) {
            let t = (i as f64 - pos).abs() * self.two_fc * TABLE_RESOLUTION as f64;
            let k = t as usize;
            let frac = t - k as f64;
            let weight = table[k] + frac * (table[k + 1] - table[k]);
            acc += weight * f64::from(self.buf[(i - self.base as i64) as usize]);
        }
        clean_output(acc * self.two_fc)
    }

    /// Drops the input no later output reads.
    fn trim(&mut self) {
        let keep_from = (self.position(self.next_out) - self.half).ceil().max(0.0) as u64;
        let drop = keep_from.clamp(self.base, self.total) - self.base;
        self.buf.drain(..drop as usize);
        self.base += drop;
    }

    /// Takes a chunk of the input and appends to `out` every output frame
    /// that has all its input.
    ///
    /// # Errors
    ///
    /// [`AudioError::NonFinite`] naming `samples` for NaN or `±∞`; `out` and
    /// the state stay as they were.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) -> Result<(), AudioError> {
        if input.iter().any(|x| !x.is_finite()) {
            return Err(AudioError::NonFinite { field: "samples" });
        }
        self.buf.extend_from_slice(input);
        self.total += input.len() as u64;
        // Output `j` needs input up to `floor(position + half)`.
        while (self.position(self.next_out) + self.half).floor() < self.total as f64 {
            out.push(self.output(self.next_out));
            self.next_out += 1;
        }
        self.trim();
        Ok(())
    }

    /// Ends the run: appends the frames still owed, up to the length of
    /// [`resampled_len`] for everything received, reading zeros past the end,
    /// and returns to the state of a new resampler.
    pub fn finish(&mut self, out: &mut Vec<f32>) {
        let frames = usize::try_from(self.total).unwrap_or(usize::MAX);
        let len = resampled_len(frames, self.from, self.to, self.speed).unwrap_or(0) as u64;
        while self.next_out < len {
            out.push(self.output(self.next_out));
            self.next_out += 1;
        }
        self.reset();
    }

    /// Forgets the run: the next input is the start of a new one.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.base = 0;
        self.total = 0;
        self.next_out = 0;
    }
}

/// Resamples a whole signal from `from` to `to` at `speed`.
///
/// # Errors
///
/// As [`resampled_len`] and [`Resampler::process`]; [`AudioError::OutOfRange`]
/// naming `samples` when the output cannot be allocated.
pub fn resample(
    input: &[f32],
    from: SampleRate,
    to: SampleRate,
    speed: f64,
) -> Result<Vec<f32>, AudioError> {
    let len = resampled_len(input.len(), from, to, speed)?;
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| AudioError::OutOfRange { field: "samples" })?;
    let mut resampler = Resampler::new(from, to, speed)?;
    resampler.process(input, &mut out)?;
    resampler.finish(&mut out);
    Ok(out)
}
