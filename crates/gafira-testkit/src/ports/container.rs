//! The fake container of ARQ-112: the port double a test injects in place of
//! a real container (02 §4.8).
//!
//! The double reads its medium from the source it opens. A medium is a
//! JSON document that lists the streams of the container and its packets:
//!
//! ```json
//! {"streams":[{"width":1920,"height":1080,"fps":20,"duration_ms":6000}],
//!  "packets":[{"keyframe":true,"ms":0}]}
//! ```
//!
//! The times are in milliseconds, the unit the time type of the ports reads.

use std::io::Read;

use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::MessageKey;
use gafira_ports::common::Confidence;
use gafira_ports::media::{
    CodecId, CodecParams, Container, Demuxer, MediaError, MuxConfig, Muxer, Packet, SeekMode,
    StreamIdx, StreamInfo,
};
use gafira_ports::system::{ReadSeek, WriteSeek};
use gafira_time::{Flicks, FrameRate};
use serde_json::{Value, json};

use super::muxer::RecordingMuxer;

/// The descriptor of the fake container.
static DESCRIPTOR: Descriptor = Descriptor::new("test.container", MessageKey("test.container"), 1);

/// One packet of a fake medium.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FakePacket {
    /// A packet that is not a keyframe, at its presentation time.
    Frame(i64),
    /// A keyframe packet, at its presentation time.
    Keyframe(i64),
}

/// The fake container: a container over a medium written as JSON.
pub struct FakeContainer;

impl FakeContainer {
    /// The bytes of a fake medium: one video stream and these packets.
    ///
    /// The times are in milliseconds.
    #[must_use]
    pub fn medium(
        width: u32,
        height: u32,
        fps: u32,
        duration_ms: i64,
        packets: &[FakePacket],
    ) -> Vec<u8> {
        let packets: Vec<Value> = packets
            .iter()
            .map(|packet| match packet {
                FakePacket::Frame(ms) => json!({"keyframe": false, "ms": ms}),
                FakePacket::Keyframe(ms) => json!({"keyframe": true, "ms": ms}),
            })
            .collect();
        let medium = json!({
            "streams": [{
                "width": width,
                "height": height,
                "fps": fps,
                "duration_ms": duration_ms,
            }],
            "packets": packets,
        });
        serde_json::to_vec(&medium).expect("a medium serializes")
    }
}

impl Describe for FakeContainer {
    /// The descriptor of the double.
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl Container for FakeContainer {
    /// The double is sure of every head it reads.
    fn probe(&self, _head: &[u8]) -> Confidence {
        Confidence::High
    }

    /// Opens the medium the source holds.
    fn open(&self, mut src: Box<dyn ReadSeek + Send>) -> Result<Box<dyn Demuxer>, MediaError> {
        let mut bytes = Vec::new();
        src.read_to_end(&mut bytes).map_err(MediaError::Io)?;
        let demuxer = FakeDemuxer::parse(&bytes)?;
        Ok(Box::new(demuxer))
    }

    /// Starts a muxing double.
    fn create(
        &self,
        _dst: Box<dyn WriteSeek + Send>,
        _config: &MuxConfig,
    ) -> Result<Box<dyn Muxer>, MediaError> {
        Ok(Box::new(RecordingMuxer::default()))
    }
}

/// The demuxer of the fake container: it plays back the packets of the
/// medium it read.
#[derive(Clone, Debug, Default)]
pub struct FakeDemuxer {
    streams: Vec<StreamInfo>,
    packets: Vec<Packet>,
    cursor: usize,
}

impl FakeDemuxer {
    /// A demuxer over these streams and these packets.
    #[must_use]
    pub fn new(streams: Vec<StreamInfo>, packets: Vec<Packet>) -> Self {
        Self {
            streams,
            packets,
            cursor: 0,
        }
    }

    /// The demuxer of the medium in these bytes.
    fn parse(bytes: &[u8]) -> Result<Self, MediaError> {
        let medium: Value = serde_json::from_slice(bytes).map_err(|e| corrupt(e.to_string()))?;
        let streams = medium["streams"]
            .as_array()
            .ok_or_else(|| corrupt("the medium has no streams"))?;
        let streams = streams
            .iter()
            .enumerate()
            .map(|(index, stream)| stream_info(index, stream))
            .collect::<Result<Vec<_>, MediaError>>()?;
        let packets = medium["packets"]
            .as_array()
            .ok_or_else(|| corrupt("the medium has no packets"))?;
        let packets = packets
            .iter()
            .map(packet)
            .collect::<Result<Vec<_>, MediaError>>()?;
        Ok(Self::new(streams, packets))
    }
}

impl Demuxer for FakeDemuxer {
    /// The streams of the medium.
    fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }

    /// The next packet of the medium.
    fn read_packet(&mut self) -> Result<Option<Packet>, MediaError> {
        let packet = self.packets.get(self.cursor).cloned();
        if packet.is_some() {
            self.cursor += 1;
        }
        Ok(packet)
    }

    /// Moves to `time`: the keyframe that covers it, or the frame at it.
    fn seek(&mut self, time: Flicks, mode: SeekMode) -> Result<(), MediaError> {
        self.cursor = match mode {
            SeekMode::Keyframe => self
                .packets
                .iter()
                .rposition(|packet| packet.keyframe && packet.pts <= time)
                .or_else(|| self.packets.iter().position(|packet| packet.keyframe))
                .unwrap_or(0),
            SeekMode::Exact => self
                .packets
                .iter()
                .position(|packet| packet.pts >= time)
                .unwrap_or(self.packets.len()),
        };
        Ok(())
    }
}

/// The stream of the medium at `index`.
fn stream_info(index: usize, stream: &Value) -> Result<StreamInfo, MediaError> {
    let width = u32_field(stream, "width")?;
    let height = u32_field(stream, "height")?;
    let fps = u32_field(stream, "fps")?;
    let duration = i64_field(stream, "duration_ms")?;
    Ok(StreamInfo {
        index: StreamIdx(u32::try_from(index).map_err(|_| corrupt("too many streams"))?),
        codec: CodecId::H264,
        params: CodecParams::Video {
            width,
            height,
            frame_rate: FrameRate::new(fps, 1).map_err(|e| corrupt(e.to_string()))?,
        },
        duration: Some(Flicks::from_millis(duration).map_err(|e| corrupt(e.to_string()))?),
    })
}

/// One packet of the medium.
fn packet(value: &Value) -> Result<Packet, MediaError> {
    let ms = i64_field(value, "ms")?;
    let keyframe = value["keyframe"]
        .as_bool()
        .ok_or_else(|| corrupt("a packet has no keyframe flag"))?;
    let pts = Flicks::from_millis(ms).map_err(|e| corrupt(e.to_string()))?;
    Ok(Packet {
        stream: StreamIdx(0),
        pts,
        dts: pts,
        keyframe,
        data: Vec::new(),
    })
}

/// A whole number of a JSON field.
fn u32_field(value: &Value, name: &str) -> Result<u32, MediaError> {
    let number = value[name]
        .as_u64()
        .ok_or_else(|| corrupt(format!("a field has no {name}")))?;
    u32::try_from(number).map_err(|_| corrupt(format!("the {name} does not fit")))
}

/// A whole number of a JSON field.
fn i64_field(value: &Value, name: &str) -> Result<i64, MediaError> {
    value[name]
        .as_i64()
        .ok_or_else(|| corrupt(format!("a field has no {name}")))
}

/// The error of a medium that cannot be read.
fn corrupt(detail: impl Into<String>) -> MediaError {
    MediaError::Corrupt {
        detail: detail.into(),
    }
}
