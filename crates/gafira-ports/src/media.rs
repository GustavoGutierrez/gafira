//! Containers and codecs (02 §4.8).

use std::fmt;
use std::io;
use std::sync::Arc;

use gafira_doc::contract::{Cancelled, Describe};
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};
use gafira_raster::ImageBuf;
use gafira_time::{Flicks, FrameRate, SampleRate};

use crate::common::{Bytes, Confidence, LegalClass};
use crate::registry::{Registry, RegistryError};
use crate::system::{ReadSeek, WriteSeek};

/// Why a media call failed.
#[derive(Debug)]
pub enum MediaError {
    /// No codec of this id can do what was asked.
    UnsupportedCodec { codec: CodecId },
    /// The stream is damaged. `detail` is library text, for `Display` only.
    Corrupt { detail: String },
    /// The operating system has no API for this codec.
    SystemCodecUnavailable,
    /// A planar audio channel has another length than the first one.
    ChannelMismatch {
        expected: usize,
        actual: usize,
        channel: usize,
    },
    /// Reading or writing failed.
    Io(io::Error),
    /// The caller cancelled the job.
    Cancelled(Cancelled),
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCodec { codec } => write!(f, "unsupported codec `{}`", codec.as_str()),
            Self::Corrupt { detail } => write!(f, "corrupt media: {detail}"),
            Self::SystemCodecUnavailable => f.write_str("system codec unavailable"),
            Self::ChannelMismatch {
                expected,
                actual,
                channel,
            } => write!(
                f,
                "channel {channel} has {actual} samples, expected {expected}"
            ),
            Self::Io(e) => write!(f, "media I/O error: {e}"),
            Self::Cancelled(_) => f.write_str("cancelled"),
        }
    }
}

impl std::error::Error for MediaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl ToMessage for MediaError {
    fn to_message(&self) -> Message {
        match self {
            Self::UnsupportedCodec { codec } => {
                msg!(
                    "error.media.unsupported_codec",
                    codec = Text(codec.as_str().into())
                )
            }
            Self::Corrupt { .. } => msg!("error.media.corrupt"),
            Self::SystemCodecUnavailable => msg!("error.media.system_codec_unavailable"),
            Self::ChannelMismatch {
                expected,
                actual,
                channel,
            } => msg!(
                "error.media.channel_mismatch",
                expected = Count(*expected as u64),
                actual = Count(*actual as u64),
                channel = Count(*channel as u64),
            ),
            Self::Io(_) => msg!("error.media.io"),
            Self::Cancelled(c) => c.to_message(),
        }
    }
}

/// A codec identifier. `Hevc` exists only to warn that it is unsupported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodecId {
    Av1,
    Vp9,
    H264,
    Hevc,
    Opus,
    Aac,
    Flac,
    Vorbis,
    Mp3,
    Pcm,
    Mjpeg,
}

impl CodecId {
    /// A stable lower-case name, for messages.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Av1 => "av1",
            Self::Vp9 => "vp9",
            Self::H264 => "h264",
            Self::Hevc => "hevc",
            Self::Opus => "opus",
            Self::Aac => "aac",
            Self::Flac => "flac",
            Self::Vorbis => "vorbis",
            Self::Mp3 => "mp3",
            Self::Pcm => "pcm",
            Self::Mjpeg => "mjpeg",
        }
    }
}

/// What a codec can do and under which legal class (D-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodecCaps {
    pub decode: bool,
    pub encode: bool,
    pub hardware: bool,
    pub legal: LegalClass,
}

/// The index of a stream inside a container.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamIdx(pub u32);

/// How a decoder or encoder is set up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CodecParams {
    Video {
        width: u32,
        height: u32,
        frame_rate: FrameRate,
    },
    Audio {
        sample_rate: SampleRate,
        channels: u16,
    },
}

/// One stream of a container.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamInfo {
    pub index: StreamIdx,
    pub codec: CodecId,
    pub params: CodecParams,
    pub duration: Option<Flicks>,
}

/// Settings for a new encoder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncoderConfig {
    pub codec: CodecId,
    pub params: CodecParams,
    /// Target bit rate in bits per second; `None` leaves it to the codec.
    pub bitrate: Option<u32>,
}

/// Settings for a new muxer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MuxConfig {
    pub streams: Vec<StreamInfo>,
}

/// What a muxer wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MuxReport {
    pub size: Bytes,
    pub packets: u64,
}

/// Where a seek lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeekMode {
    /// The previous keyframe.
    Keyframe,
    /// The requested time, decoding forward from the keyframe.
    Exact,
}

/// A compressed unit of one stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub stream: StreamIdx,
    pub pts: Flicks,
    pub dts: Flicks,
    pub keyframe: bool,
    pub data: Vec<u8>,
}

/// A decoded video frame, as RGBA.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoFrame {
    pub pts: Flicks,
    pub image: ImageBuf,
}

/// Decoded planar audio: every channel has the same length.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioBuf {
    pts: Flicks,
    sample_rate: SampleRate,
    channels: Vec<Vec<f32>>,
}

impl AudioBuf {
    /// Builds a buffer from planar channels. No channels give an empty buffer.
    ///
    /// # Errors
    /// [`MediaError::ChannelMismatch`] naming the first channel whose length
    /// differs from the first channel's.
    pub fn new(
        pts: Flicks,
        sample_rate: SampleRate,
        channels: Vec<Vec<f32>>,
    ) -> Result<Self, MediaError> {
        if let Some(first) = channels.first() {
            let expected = first.len();
            if let Some((channel, bad)) = channels
                .iter()
                .enumerate()
                .find(|(_, c)| c.len() != expected)
            {
                return Err(MediaError::ChannelMismatch {
                    expected,
                    actual: bad.len(),
                    channel,
                });
            }
        }
        Ok(Self {
            pts,
            sample_rate,
            channels,
        })
    }

    /// Presentation time of the first sample.
    #[must_use]
    pub const fn pts(&self) -> Flicks {
        self.pts
    }

    /// Samples per second.
    #[must_use]
    pub const fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// The planar channels, all of one length.
    #[must_use]
    pub fn channels(&self) -> &[Vec<f32>] {
        &self.channels
    }

    /// Samples per channel; 0 with no channels.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }
}

/// What a decoder yields and an encoder takes.
#[derive(Clone, Debug, PartialEq)]
pub enum Frame {
    Video(VideoFrame),
    Audio(AudioBuf),
}

/// A container format (Strategy).
pub trait Container: Describe + Send + Sync {
    /// How sure this container is about `head`.
    fn probe(&self, head: &[u8]) -> Confidence;

    /// Opens `src` for reading.
    ///
    /// # Errors
    /// [`MediaError`] when the source is not this container or is damaged.
    fn open(&self, src: Box<dyn ReadSeek + Send>) -> Result<Box<dyn Demuxer>, MediaError>;

    /// Starts writing a container to `dst`.
    ///
    /// # Errors
    /// [`MediaError`] when the configuration or the sink is refused.
    fn create(
        &self,
        dst: Box<dyn WriteSeek + Send>,
        cfg: &MuxConfig,
    ) -> Result<Box<dyn Muxer>, MediaError>;
}

/// Reads packets from a container.
pub trait Demuxer: Send {
    /// The streams of the container.
    fn streams(&self) -> &[StreamInfo];

    /// The next packet, or `None` at the end.
    ///
    /// # Errors
    /// [`MediaError`] when reading fails.
    fn read_packet(&mut self) -> Result<Option<Packet>, MediaError>;

    /// Moves to `t` as `mode` says.
    ///
    /// # Errors
    /// [`MediaError`] when the container cannot seek there.
    fn seek(&mut self, t: Flicks, mode: SeekMode) -> Result<(), MediaError>;
}

/// Writes packets to a container.
pub trait Muxer: Send {
    /// Adds a stream.
    ///
    /// # Errors
    /// [`MediaError`] when the container refuses the stream.
    fn add_stream(&mut self, s: &StreamInfo) -> Result<StreamIdx, MediaError>;

    /// Writes one packet.
    ///
    /// # Errors
    /// [`MediaError`] when writing fails.
    fn write_packet(&mut self, p: &Packet) -> Result<(), MediaError>;

    /// Finishes the file.
    ///
    /// # Errors
    /// [`MediaError`] when the trailer cannot be written.
    fn finish(self: Box<Self>) -> Result<MuxReport, MediaError>;
}

/// A codec: creates compatible decoders and encoders (Abstract Factory).
pub trait Codec: Describe + Send + Sync {
    /// The codec this implements.
    fn codec_id(&self) -> CodecId;

    /// What it can do.
    fn caps(&self) -> CodecCaps;

    /// A decoder for `p`.
    ///
    /// # Errors
    /// [`MediaError`] when the parameters are refused.
    fn decoder(&self, p: &CodecParams) -> Result<Box<dyn Decoder>, MediaError>;

    /// An encoder for `c`.
    ///
    /// # Errors
    /// [`MediaError`] when the configuration is refused.
    fn encoder(&self, c: &EncoderConfig) -> Result<Box<dyn Encoder>, MediaError>;
}

/// Turns packets into frames.
pub trait Decoder: Send {
    /// Feeds a packet; `None` flushes.
    ///
    /// # Errors
    /// [`MediaError`] when the packet cannot be decoded.
    fn send(&mut self, p: Option<&Packet>) -> Result<(), MediaError>;

    /// The next frame, if one is ready.
    ///
    /// # Errors
    /// [`MediaError`] when decoding fails.
    fn receive(&mut self) -> Result<Option<Frame>, MediaError>;
}

/// Turns frames into packets.
pub trait Encoder: Send {
    /// Feeds a frame; `None` flushes.
    ///
    /// # Errors
    /// [`MediaError`] when the frame is refused.
    fn send(&mut self, f: Option<&Frame>) -> Result<(), MediaError>;

    /// The next packet, if one is ready.
    ///
    /// # Errors
    /// [`MediaError`] when encoding fails.
    fn receive(&mut self) -> Result<Option<Packet>, MediaError>;
}

/// Codecs by id, with the decoder policy of D-02.
#[derive(Default)]
pub struct CodecRegistry {
    codecs: Registry<dyn Codec>,
}

impl CodecRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a codec under its descriptor id.
    ///
    /// # Errors
    /// [`RegistryError::DuplicateId`] when the id is taken.
    pub fn register(&mut self, codec: Arc<dyn Codec>) -> Result<(), RegistryError> {
        self.codecs.register(codec)
    }

    /// The codec registered under `id`.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Arc<dyn Codec>> {
        self.codecs.get(id)
    }

    /// The codecs in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Codec>> {
        self.codecs.iter()
    }

    /// Number of codecs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.codecs.len()
    }

    /// True when no codec is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.codecs.is_empty()
    }

    /// The codec to decode `id` with: hardware first, then `OsLicensed`, then
    /// `RoyaltyFree`; registration order on a tie. A codec that does not
    /// decode, has another id or is `Encumbered` is never chosen (D-02).
    #[must_use]
    pub fn best_decoder(&self, id: CodecId) -> Option<&Arc<dyn Codec>> {
        self.codecs
            .iter()
            .filter(|c| {
                let caps = c.caps();
                c.codec_id() == id && caps.decode && caps.legal != LegalClass::Encumbered
            })
            .min_by_key(|c| rank(c.caps()))
    }
}

/// Lower is better: hardware, then `OsLicensed`, then `RoyaltyFree`.
fn rank(caps: CodecCaps) -> u8 {
    if caps.hardware {
        0
    } else if caps.legal == LegalClass::OsLicensed {
        1
    } else {
        2
    }
}
