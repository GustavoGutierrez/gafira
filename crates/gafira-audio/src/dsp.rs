//! Helpers shared by the processors: the block guard, the parameter reader and
//! the sample cleaning that makes `process` total (ARQ-123).

use crate::{AudioBlock, ParamSnapshot};

/// The stream shape given to `prepare`; all zeros until then.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Setup {
    pub sample_rate: u32,
    pub max_block: usize,
    pub channels: u16,
}

impl Setup {
    /// True when the block may be processed. A `prepare` with a 0, a block
    /// above `max_block` or channels of different lengths leave it intact.
    /// With `max_block == 0` every non-empty block is above it.
    pub fn accepts(&self, block: &AudioBlock<'_>) -> bool {
        self.sample_rate != 0
            && self.channels != 0
            && block.is_uniform()
            && block.frames() <= self.max_block
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
    value: f64,
}

impl Param {
    pub const fn new(id: &'static str, min: f64, max: f64, default: f64) -> Self {
        Self {
            id,
            min,
            max,
            value: default,
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
