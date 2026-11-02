//! The `AudioProcessor` trait (02 §4.9).

use gafira_doc::contract::Describe;

use crate::{AudioBlock, ParamSnapshot};

/// One stage of a track's chain.
///
/// `process` is total (ARQ-123): it never returns an error, never panics and
/// never allocates. A NaN or infinite sample counts as 0, a non-finite
/// parameter is ignored (the last valid one stays), an out-of-range parameter
/// is clipped, and no output sample is non-finite or subnormal. If `prepare`
/// got a 0, the block is larger than `max_block`, its channels differ in
/// length or their number is not the prepared one, `process` does not process
/// the block and its state does not change: it only sets NaN, `±∞` and
/// subnormals to 0.
pub trait AudioProcessor: Describe + Send {
    /// Sets the stream shape and clears the processing state.
    fn prepare(&mut self, sample_rate: u32, max_block: usize, channels: u16);
    /// Processes one block in place.
    fn process(&mut self, block: &mut AudioBlock<'_>, params: &ParamSnapshot);
    /// Delay in samples that the processor adds.
    fn latency(&self) -> u32 {
        0
    }
    /// Samples the output keeps ringing after the input ends.
    fn tail(&self) -> u32 {
        0
    }
    /// Clears the processing state; parameters and stream shape stay.
    fn reset(&mut self);
}
