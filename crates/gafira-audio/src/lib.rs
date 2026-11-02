//! Layer-2 engine crate `gafira-audio`: the `AudioProcessor` contract, gain,
//! fades, pan, the processor chain, the mix and flick offsets (02 §4.9).

#![forbid(unsafe_code)]

mod block;
mod dsp;
mod error;
mod gain;
mod processor;

pub use block::{AudioBlock, MAX_PARAMS, ParamSnapshot};
pub use error::AudioError;
pub use gain::{Gain, PARAM_VOLUME, db_to_gain, percent_to_gain};
pub use processor::AudioProcessor;
