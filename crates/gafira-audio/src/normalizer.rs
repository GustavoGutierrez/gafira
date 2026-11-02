//! `LoudnessNormalizer`: the Nivelar gain (ARQ-127).

use gafira_doc::contract::{Describe, Descriptor, ParamSchema};
use gafira_doc::msg::MessageKey;

use crate::dsp::{Param, Setup, clean_block, clean_input, clean_output};
use crate::{AudioBlock, AudioProcessor, ParamSnapshot};

/// Id of the gain parameter, in dB.
pub const PARAM_GAIN: &str = "gain";

static DESCRIPTOR: Descriptor = Descriptor::new(
    "audio.loudness_normalizer",
    MessageKey("audio.loudness_normalizer"),
    1,
);

/// Applies the Nivelar gain: one constant gain in dB, aside from the clip's
/// Volume (D-31). It reads only `gain`; the Volume parameter does not reach
/// it, and a gain above +6 dB is not limited to the 200 % of Volume.
///
/// A new value applies from the first sample of the next processed block, with
/// no ramp: Nivelar sets the gain once for the whole signal (ARQ-127).
#[derive(Debug)]
pub struct LoudnessNormalizer {
    setup: Setup,
    gain: Param,
    schema: ParamSchema,
}

impl LoudnessNormalizer {
    /// Gain 0 dB, not prepared.
    #[must_use]
    pub fn new() -> Self {
        let gain = Param::new(PARAM_GAIN, -f64::MAX, f64::MAX, 0.0);
        Self {
            schema: ParamSchema::new(vec![
                gain.def(MessageKey("audio.loudness_normalizer.gain"), false),
            ]),
            setup: Setup::default(),
            gain,
        }
    }
}

impl Default for LoudnessNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for LoudnessNormalizer {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

/// `10^(db / 20)`. A gain that overflows to `+∞` still leaves a finite output:
/// `clean_output` saturates a product and turns `0 × ∞` into 0.
fn linear(db: f64) -> f64 {
    10_f64.powf(db / 20.0)
}

impl AudioProcessor for LoudnessNormalizer {
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
        let gain = linear(self.gain.read(params));
        for channel in block.channels.iter_mut() {
            for sample in channel.iter_mut() {
                *sample = clean_output(f64::from(clean_input(*sample)) * gain);
            }
        }
    }

    fn reset(&mut self) {}
}
