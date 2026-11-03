//! Clip fade: a linear gain ramp (ARQ-124, FV-10).

use gafira_doc::contract::{Describe, Descriptor, ParamSchema};
use gafira_doc::msg::MessageKey;

use crate::dsp::{Param, Setup, clean_block, clean_input, clean_output};
use crate::{AudioBlock, AudioProcessor, ParamSnapshot};

/// Id of the fade length parameter, in seconds.
pub const PARAM_DURATION: &str = "duration";

static DESCRIPTOR: Descriptor = Descriptor::new("audio.fade", MessageKey("audio.fade"), 1);

/// Which way the gain moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FadeDirection {
    /// Gain rises from 0 to 1.
    In,
    /// Gain falls from 1 to 0.
    Out,
}

/// A linear gain ramp over `duration` seconds, starting at the first sample
/// after `prepare` or `reset`; it holds its end value afterwards. A duration
/// of 0 changes nothing.
#[derive(Debug)]
pub struct Fade {
    direction: FadeDirection,
    setup: Setup,
    duration: Param,
    position: u64,
    /// Ramp length in samples, read at the first accepted block after
    /// `prepare` or `reset`.
    length: Option<u64>,
    schema: ParamSchema,
}

impl Fade {
    /// A fade with duration 0 until the snapshot says otherwise.
    #[must_use]
    pub fn new(direction: FadeDirection) -> Self {
        let duration = Param::new(PARAM_DURATION, 0.0, f64::MAX, 0.0);
        Self {
            schema: ParamSchema::new(vec![duration.def(MessageKey("audio.fade.duration"), false)]),
            direction,
            setup: Setup {
                sample_rate: 0,
                max_block: 0,
                channels: 0,
            },
            duration,
            position: 0,
            length: None,
        }
    }
}

impl Describe for Fade {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl AudioProcessor for Fade {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn prepare(&mut self, sample_rate: u32, max_block: usize, channels: u16) {
        self.setup = Setup {
            sample_rate,
            max_block,
            channels,
        };
        self.reset();
    }

    fn process(&mut self, block: &mut AudioBlock<'_>, params: &ParamSnapshot) {
        if !self.setup.accepts(block) {
            clean_block(block);
            return;
        }
        let seconds = self.duration.read(params);
        // The length is latched with the first block of the ramp, so a later
        // change waits for the next `reset` and the gain stays continuous.
        // `as` saturates: a huge duration is a fade that never ends.
        let length = *self
            .length
            .get_or_insert_with(|| (seconds * f64::from(self.setup.sample_rate)).round() as u64);
        for i in 0..block.frames() {
            let gain = if length == 0 {
                1.0
            } else {
                let rise = (self.position as f64 / length as f64).min(1.0);
                match self.direction {
                    FadeDirection::In => rise,
                    FadeDirection::Out => 1.0 - rise,
                }
            };
            self.position = self.position.saturating_add(1);
            for channel in block.channels.iter_mut() {
                channel[i] = clean_output(f64::from(clean_input(channel[i])) * gain);
            }
        }
    }

    fn reset(&mut self) {
        self.position = 0;
        self.length = None;
    }
}
