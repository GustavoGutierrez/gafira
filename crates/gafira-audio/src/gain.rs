//! Gain: dB and percent maths, and the ramped `Gain` processor (ARQ-124).

use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::MessageKey;

use crate::dsp::{Param, Setup, clean_input, clean_output};
use crate::error::finite;
use crate::{AudioBlock, AudioError, AudioProcessor, ParamSnapshot};

/// Id of the volume parameter, in percent (FV-10: 0 to 200).
pub const PARAM_VOLUME: &str = "volume";

static DESCRIPTOR: Descriptor = Descriptor::new("audio.gain", MessageKey("audio.gain"), 1);

/// Linear gain of `db` decibels: `10^(db / 20)`. `−∞` dB is 0.
///
/// # Errors
///
/// [`AudioError::NonFinite`] naming `db` for NaN or `+∞`, or when the result
/// overflows.
pub fn db_to_gain(db: f64) -> Result<f64, AudioError> {
    if db == f64::NEG_INFINITY {
        return Ok(0.0);
    }
    finite("db", db)?;
    finite("db", 10_f64.powf(db / 20.0))
}

/// Linear gain of a volume in percent: 200 % is 2.0.
///
/// # Errors
///
/// [`AudioError::NonFinite`] naming `percent` for NaN or `±∞`.
pub fn percent_to_gain(percent: f64) -> Result<f64, AudioError> {
    finite("percent", percent).map(|p| p / 100.0)
}

/// Volume with a 10 ms linear ramp between values.
#[derive(Debug)]
pub struct Gain {
    setup: Setup,
    volume: Param,
    /// Gain applied to the next sample.
    current: f64,
    target: f64,
    step: f64,
    /// Samples left in the running ramp.
    remaining: u32,
    /// Length of a ramp: 10 ms.
    ramp_len: u32,
}

/// 10 ms (PROPUESTA, ARQ-124) in whole samples: `sample_rate / 100`, at least 1.
/// Exact for 44.1, 48 and 96 kHz.
fn ramp_samples(sample_rate: u32) -> u32 {
    (sample_rate / 100).max(1)
}

impl Gain {
    /// Unity gain, not prepared.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            setup: Setup {
                sample_rate: 0,
                max_block: 0,
                channels: 0,
            },
            volume: Param::new(PARAM_VOLUME, 0.0, 200.0, 100.0),
            current: 1.0,
            target: 1.0,
            step: 0.0,
            remaining: 0,
            ramp_len: 1,
        }
    }
}

impl Default for Gain {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Gain {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl AudioProcessor for Gain {
    fn prepare(&mut self, sample_rate: u32, max_block: usize, channels: u16) {
        self.setup = Setup {
            sample_rate,
            max_block,
            channels,
        };
        self.ramp_len = ramp_samples(sample_rate);
        self.reset();
    }

    fn process(&mut self, block: &mut AudioBlock<'_>, params: &ParamSnapshot) {
        if !self.setup.accepts(block) {
            return;
        }
        let target = self.volume.read(params) / 100.0;
        if target.to_bits() != self.target.to_bits() {
            self.target = target;
            self.remaining = self.ramp_len;
            self.step = (target - self.current) / f64::from(self.ramp_len);
        }
        for i in 0..block.frames() {
            if self.remaining > 0 {
                self.current += self.step;
                self.remaining -= 1;
                if self.remaining == 0 {
                    self.current = self.target;
                }
            }
            for channel in block.channels.iter_mut() {
                channel[i] = clean_output(f64::from(clean_input(channel[i])) * self.current);
            }
        }
    }

    fn reset(&mut self) {
        self.current = self.target;
        self.remaining = 0;
        self.step = 0.0;
    }
}
