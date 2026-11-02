//! Stereo pan and the equal-power crossfade law (ARQ-124).

use std::f64::consts::{PI, SQRT_2};

use gafira_doc::contract::{AudioCrossfade, Describe, Descriptor, ParamSchema};
use gafira_doc::msg::MessageKey;

use crate::dsp::{Param, Setup, clean_block, clean_input, clean_output};
use crate::error::finite;
use crate::{AudioBlock, AudioError, AudioProcessor, ParamSnapshot};

/// Id of the pan parameter, in `[-1, 1]`.
pub const PARAM_PAN: &str = "pan";

static DESCRIPTOR: Descriptor = Descriptor::new("audio.pan", MessageKey("audio.pan"), 1);

/// Gains `(outgoing, incoming)` of a crossfade at position `p`; `p` is clipped
/// to `[0, 1]`. `EqualPower` gives `cos(π p / 2)` and `sin(π p / 2)`.
///
/// # Errors
///
/// [`AudioError::NonFinite`] naming `p` for NaN or `±∞`.
pub fn crossfade_gains(mode: AudioCrossfade, p: f64) -> Result<(f64, f64), AudioError> {
    let p = finite("p", p)?.clamp(0.0, 1.0);
    match mode {
        AudioCrossfade::EqualPower => {
            let angle = PI * p / 2.0;
            Ok((angle.cos(), angle.sin()))
        }
    }
}

/// Pan law on stereo blocks; any other block is not changed.
#[derive(Debug)]
pub struct Pan {
    setup: Setup,
    pan: Param,
    schema: ParamSchema,
}

impl Pan {
    /// Centre, not prepared.
    #[must_use]
    pub fn new() -> Self {
        let pan = Param::new(PARAM_PAN, -1.0, 1.0, 0.0);
        Self {
            schema: ParamSchema::new(vec![pan.def(MessageKey("audio.pan.pan"), false)]),
            setup: Setup {
                sample_rate: 0,
                max_block: 0,
                channels: 0,
            },
            pan,
        }
    }
}

impl Default for Pan {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Pan {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl AudioProcessor for Pan {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn prepare(&mut self, sample_rate: u32, max_block: usize, channels: u16) {
        self.setup = Setup {
            sample_rate,
            max_block,
            channels,
        };
    }

    fn process(&mut self, block: &mut AudioBlock<'_>, params: &ParamSnapshot) {
        if !self.setup.accepts(block) {
            clean_block(block);
            return;
        }
        let position = self.pan.read(params);
        let (left, right) = if block.channels.len() == 2 {
            // θ = (p + 1) π / 4; ±3 dB law, 0 dB at the centre.
            let theta = (position + 1.0) * PI / 4.0;
            (
                (SQRT_2 * theta.cos()).min(1.0),
                (SQRT_2 * theta.sin()).min(1.0),
            )
        } else {
            (1.0, 1.0)
        };
        for (index, channel) in block.channels.iter_mut().enumerate() {
            let gain = if index == 0 { left } else { right };
            for sample in channel.iter_mut() {
                *sample = clean_output(f64::from(clean_input(*sample)) * gain);
            }
        }
    }

    fn reset(&mut self) {}
}
