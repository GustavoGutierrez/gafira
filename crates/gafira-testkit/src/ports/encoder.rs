//! The fake encoder of ARQ-112: the port double a test injects in place of a
//! real encoder (02 §4.8).

use gafira_ports::media::{Encoder, Frame, MediaError, Packet};

/// The fake encoder. A stub: the media stream fills it.
pub struct FakeEncoder;

impl Encoder for FakeEncoder {
    /// Not filled yet.
    fn send(&mut self, _frame: Option<&Frame>) -> Result<(), MediaError> {
        panic!("stub: the encoder double is not filled yet")
    }

    /// Not filled yet.
    fn receive(&mut self) -> Result<Option<Packet>, MediaError> {
        panic!("stub: the encoder double is not filled yet")
    }
}
