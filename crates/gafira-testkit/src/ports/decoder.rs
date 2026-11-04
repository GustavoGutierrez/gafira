//! The fake decoder of ARQ-112: the port double a test injects in place of a
//! real decoder (02 §4.8).

use gafira_ports::media::{Decoder, Frame, MediaError, Packet};

/// The fake decoder. A stub: the media stream fills it.
pub struct FakeDecoder;

impl Decoder for FakeDecoder {
    /// Not filled yet.
    fn send(&mut self, _packet: Option<&Packet>) -> Result<(), MediaError> {
        panic!("stub: the decoder double is not filled yet")
    }

    /// Not filled yet.
    fn receive(&mut self) -> Result<Option<Frame>, MediaError> {
        panic!("stub: the decoder double is not filled yet")
    }
}
