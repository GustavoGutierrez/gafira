//! The fake muxer of ARQ-112: the port double a test injects in place of a
//! real muxer (02 §4.8).

use gafira_ports::common::Bytes;
use gafira_ports::media::{MediaError, MuxReport, Muxer, Packet, StreamIdx, StreamInfo};

/// The fake muxer: the empty muxer, which writes nothing.
///
/// The muxer that records what it is given is [`RecordingMuxer`].
pub struct FakeMuxer;

impl Muxer for FakeMuxer {
    /// The empty muxer takes the stream and keeps nothing.
    fn add_stream(&mut self, _stream: &StreamInfo) -> Result<StreamIdx, MediaError> {
        Ok(StreamIdx(0))
    }

    /// The empty muxer takes the packet and keeps nothing.
    fn write_packet(&mut self, _packet: &Packet) -> Result<(), MediaError> {
        Ok(())
    }

    /// The empty muxer wrote nothing.
    fn finish(self: Box<Self>) -> Result<MuxReport, MediaError> {
        Ok(MuxReport {
            size: Bytes(0),
            packets: 0,
        })
    }
}

/// The muxer that records the streams and the packets it is given.
#[derive(Clone, Debug, Default)]
pub struct RecordingMuxer {
    streams: Vec<StreamInfo>,
    packets: Vec<Packet>,
}

impl RecordingMuxer {
    /// A muxer that has recorded nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The streams the muxer was given.
    #[must_use]
    pub fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    /// The packets the muxer was given.
    #[must_use]
    pub fn packets(&self) -> &[Packet] {
        &self.packets
    }
}

impl Muxer for RecordingMuxer {
    /// Records the stream and gives back its index.
    fn add_stream(&mut self, stream: &StreamInfo) -> Result<StreamIdx, MediaError> {
        let index = StreamIdx(u32::try_from(self.streams.len()).unwrap_or(u32::MAX));
        self.streams.push(stream.clone());
        Ok(index)
    }

    /// Records the packet.
    fn write_packet(&mut self, packet: &Packet) -> Result<(), MediaError> {
        self.packets.push(packet.clone());
        Ok(())
    }

    /// Reports what the muxer wrote.
    fn finish(self: Box<Self>) -> Result<MuxReport, MediaError> {
        let packets = u64::try_from(self.packets.len()).unwrap_or(u64::MAX);
        let size = self
            .packets
            .iter()
            .map(|packet| u64::try_from(packet.data.len()).unwrap_or(u64::MAX))
            .sum();
        Ok(MuxReport {
            size: Bytes(size),
            packets,
        })
    }
}
