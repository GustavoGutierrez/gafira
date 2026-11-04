//! The fake muxer of ARQ-112: the port double a test injects in place of a
//! real muxer (02 §4.8).

use gafira_ports::media::{MediaError, MuxReport, Muxer, Packet, StreamIdx, StreamInfo};

/// The fake muxer. A stub: the media stream fills it.
pub struct FakeMuxer;

impl Muxer for FakeMuxer {
    /// Not filled yet.
    fn add_stream(&mut self, _stream: &StreamInfo) -> Result<StreamIdx, MediaError> {
        panic!("stub: the muxer double is not filled yet")
    }

    /// Not filled yet.
    fn write_packet(&mut self, _packet: &Packet) -> Result<(), MediaError> {
        panic!("stub: the muxer double is not filled yet")
    }

    /// Not filled yet.
    fn finish(self: Box<Self>) -> Result<MuxReport, MediaError> {
        panic!("stub: the muxer double is not filled yet")
    }
}
