//! Helpers shared by the processors: the block guard, the parameter reader and
//! the sample cleaning that makes `process` total (ARQ-123).

use gafira_doc::contract::{ParamDef, ParamKind};
use gafira_doc::msg::MessageKey;

use crate::{AudioBlock, ParamSnapshot};

/// The stream shape given to `prepare`; all zeros until then.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Setup {
    pub sample_rate: u32,
    pub max_block: usize,
    pub channels: u16,
}

impl Setup {
    /// True when the block may be processed. A `prepare` with a 0, an empty
    /// block (`frames() == 0`, whatever the `prepare`), a block above
    /// `max_block`, channels of different lengths or a channel count other
    /// than the prepared one make `process` only clean it (ARQ-123). With
    /// `max_block == 0` every non-empty block is above it, and with
    /// `channels == 0` no block has the prepared channel count.
    pub fn accepts(&self, block: &AudioBlock<'_>) -> bool {
        self.sample_rate != 0
            && block.frames() > 0
            && block.channels.len() == usize::from(self.channels)
            && block.is_uniform()
            && block.frames() <= self.max_block
    }
}

/// What `process` does to a block it does not process: NaN, `±∞` and
/// subnormals become 0, every other sample stays as it is. Each channel is
/// cleaned over its own length.
pub(crate) fn clean_block(block: &mut AudioBlock<'_>) {
    for channel in block.channels.iter_mut() {
        for sample in channel.iter_mut() {
            *sample = clean_output(f64::from(clean_input(*sample)));
        }
    }
}

/// A finite sample, or 0 for NaN and `±∞`.
pub(crate) fn clean_input(x: f32) -> f32 {
    if x.is_finite() { x } else { 0.0 }
}

/// A finite, normal sample. NaN gives 0, an overflow saturates at
/// `±f32::MAX` and a subnormal is flushed to 0.
pub(crate) fn clean_output(x: f64) -> f32 {
    let y = x as f32;
    if y.is_nan() || y.is_subnormal() {
        0.0
    } else if y.is_infinite() {
        f32::MAX.copysign(y)
    } else {
        y
    }
}

/// A numeric parameter that keeps its last valid value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Param {
    id: &'static str,
    min: f64,
    max: f64,
    default: f64,
    value: f64,
}

impl Param {
    pub const fn new(id: &'static str, min: f64, max: f64, default: f64) -> Self {
        Self {
            id,
            min,
            max,
            default,
            value: default,
        }
    }

    /// The schema entry of this parameter. The range and the default are the
    /// ones `read` clips to, so they exist once.
    pub fn def(&self, label: MessageKey, animatable: bool) -> ParamDef {
        ParamDef {
            id: self.id,
            label,
            kind: ParamKind::Float {
                min: self.min,
                max: self.max,
                default: self.default,
            },
            animatable,
        }
    }

    /// Reads the snapshot: a finite value is clipped to `[min, max]` and kept;
    /// a missing or non-finite one leaves the last valid value.
    pub fn read(&mut self, snapshot: &ParamSnapshot) -> f64 {
        if let Some(v) = snapshot.get(self.id)
            && v.is_finite()
        {
            self.value = v.clamp(self.min, self.max);
        }
        self.value
    }
}
