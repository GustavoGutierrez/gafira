//! Layer-2 engine crate `gafira-audio`: the `AudioProcessor` contract, gain,
//! fades, pan, the three-band equaliser, the processor chain, the mix and flick offsets (02 §4.9).

#![forbid(unsafe_code)]

mod block;
mod chain;
mod dsp;
mod eq;
mod error;
mod fade;
mod gain;
mod loudness;
mod mix;
mod offset;
mod pan;
mod processor;

pub use block::{AudioBlock, MAX_PARAMS, ParamSnapshot};
pub use chain::ProcessorChain;
pub use eq::{Eq3Band, PARAM_HIGH, PARAM_LOW, PARAM_MID};
pub use error::AudioError;
pub use fade::{Fade, FadeDirection, PARAM_DURATION};
pub use gain::{Gain, PARAM_VOLUME, db_to_gain, percent_to_gain};
pub use loudness::{
    Leveling, integrated_from_blocks, integrated_loudness, level, plan_leveling, true_peak,
};
pub use mix::{MixTrack, mix};
pub use offset::flicks_to_samples;
pub use pan::{PARAM_PAN, Pan, crossfade_gains};
pub use processor::AudioProcessor;
