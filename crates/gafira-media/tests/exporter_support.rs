//! The doubles the exporter tests inject (FV-12, ARQ-139).
//!
//! Every stage of the pipeline records itself in one [`Journal`], so a test
//! can prove the order decode → compose → encode → mux. The composition
//! delegates to the testkit `FakePageRenderer`, so the delivery is proved
//! against the port double and does not depend on `gafira-render` (design
//! `ME5`).

use std::io::{self, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use gafira_doc::contract::{CancelToken, Describe, Descriptor, Progress};
use gafira_doc::msg::MessageKey;
use gafira_ports::common::{Bytes, Confidence, LegalClass};
use gafira_ports::export::{ExportSink, PageRenderRequest, PageRenderer};
use gafira_ports::media::{
    AudioBuf, Codec, CodecCaps, CodecId, CodecParams, Container, Decoder, Demuxer, Encoder,
    EncoderConfig, Frame, MediaError, MuxConfig, MuxReport, Muxer, Packet, StreamIdx, StreamInfo,
};
use gafira_ports::render::RenderError;
use gafira_ports::store::{ContentStore, StoreError};
use gafira_ports::system::{ReadSeek, WriteSeek};
use gafira_raster::{ImageBuf, PixelBuf};
use gafira_testkit::ports::decoder::FakeDecoder;
use gafira_testkit::ports::page_renderer::FakePageRenderer;
use gafira_time::{Flicks, SampleRate};

/// One stage of the export pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The source handed a decoded video frame over.
    Decode,
    /// The page renderer composed the frame.
    Compose,
    /// The encoder turned the frame into a packet.
    Encode,
    /// The muxer wrote the packet.
    Mux,
}

/// The order of the stages of an export.
#[derive(Clone, Default)]
pub struct Journal {
    steps: Arc<Mutex<Vec<Step>>>,
}

impl Journal {
    /// An empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one step.
    fn record(&self, step: Step) {
        self.steps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(step);
    }

    /// Every step, in order.
    #[must_use]
    pub fn steps(&self) -> Vec<Step> {
        self.steps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// How many times a step happened.
    #[must_use]
    pub fn count(&self, step: Step) -> usize {
        self.steps()
            .iter()
            .filter(|recorded| **recorded == step)
            .count()
    }
}

/// What the muxer was given.
#[derive(Clone, Default)]
pub struct Probe {
    streams: Arc<Mutex<Vec<StreamInfo>>>,
    packets: Arc<Mutex<Vec<Packet>>>,
}

impl Probe {
    /// An empty recorder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The streams the muxer was given.
    #[must_use]
    pub fn streams(&self) -> Vec<StreamInfo> {
        self.streams
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The packets the muxer was given.
    #[must_use]
    pub fn packets(&self) -> Vec<Packet> {
        self.packets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The packets of one stream.
    #[must_use]
    pub fn stream_packets(&self, stream: StreamIdx) -> Vec<Packet> {
        self.packets()
            .into_iter()
            .filter(|packet| packet.stream == stream)
            .collect()
    }
}

/// The files the sink was asked to create.
#[derive(Clone, Default)]
pub struct SinkProbe {
    names: Arc<Mutex<Vec<String>>>,
}

impl SinkProbe {
    /// An empty probe.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The names given to `create`, in order.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The export sink of a test: it records the names it is asked for.
pub struct MemSink {
    probe: SinkProbe,
}

impl MemSink {
    /// A sink that accepts every file.
    #[must_use]
    pub fn new(probe: SinkProbe) -> Self {
        Self { probe }
    }
}

impl ExportSink for MemSink {
    fn create(&mut self, name: &str) -> io::Result<Box<dyn WriteSeek + Send>> {
        self.probe
            .names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(name.to_string());
        Ok(Box::new(Cursor::new(Vec::new())))
    }
}

/// The page renderer of a test: it counts, records and delegates to the
/// testkit double (design `ME5`, AC6).
pub struct CountingRenderer {
    journal: Journal,
    count: Arc<AtomicU32>,
    last: Arc<Mutex<Option<PageRenderRequest>>>,
}

impl CountingRenderer {
    /// A renderer that records into `journal`.
    #[must_use]
    pub fn new(journal: Journal) -> Self {
        Self {
            journal,
            count: Arc::new(AtomicU32::new(0)),
            last: Arc::new(Mutex::new(None)),
        }
    }

    /// How many frames it composed.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }

    /// The last request it composed.
    #[must_use]
    pub fn last(&self) -> Option<PageRenderRequest> {
        *self.last.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PageRenderer for CountingRenderer {
    fn render(
        &self,
        request: &PageRenderRequest,
        cancel: &CancelToken,
    ) -> Result<ImageBuf, RenderError> {
        cancel.check().map_err(RenderError::Cancelled)?;
        self.journal.record(Step::Compose);
        self.count.fetch_add(1, Ordering::Relaxed);
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = Some(*request);
        // The testkit double: the delivery is proved against the port, not
        // against `gafira-render` (design `ME5`, AC6).
        FakePageRenderer.render(request, cancel)
    }
}

/// A renderer that ignores the cancel token, so only the exporter's own
/// checkpoint can react to a cancellation (ARQ-139).
pub struct DeafRenderer;

impl PageRenderer for DeafRenderer {
    fn render(
        &self,
        request: &PageRenderRequest,
        _cancel: &CancelToken,
    ) -> Result<ImageBuf, RenderError> {
        let width = request.region.width();
        let height = request.region.height();
        let len = (width as usize) * (height as usize) * 4;
        ImageBuf::new(width, height, PixelBuf::Rgba8(vec![0; len])).map_err(RenderError::Buffer)
    }
}

/// The frames and the audio a test source yields.
#[derive(Clone, Default)]
pub struct SourcePlan {
    /// The presentation times of the video frames.
    pub video: Vec<Flicks>,
    /// The audio buffers, in their own presentation order.
    pub audio: Vec<AudioBuf>,
}

/// What the source of a test was asked to read.
#[derive(Clone)]
pub struct SourceProbe {
    journal: Journal,
    opened: Arc<Mutex<Vec<PathBuf>>>,
}

impl SourceProbe {
    /// The files the source was asked to open, in order.
    #[must_use]
    pub fn opened(&self) -> Vec<PathBuf> {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The source factory of a test: every export gets a fresh source.
pub struct RecordingSourceFactory {
    probe: SourceProbe,
    plan: SourcePlan,
}

impl RecordingSourceFactory {
    /// A factory over `plan`, recording into `journal`.
    #[must_use]
    pub fn new(journal: Journal, plan: SourcePlan) -> Self {
        Self {
            probe: SourceProbe {
                journal,
                opened: Arc::new(Mutex::new(Vec::new())),
            },
            plan,
        }
    }

    /// The probe of the sources it creates.
    #[must_use]
    pub fn probe(&self) -> SourceProbe {
        self.probe.clone()
    }
}

impl gafira_media::ExportSourceFactory for RecordingSourceFactory {
    fn source(&self) -> Box<dyn gafira_media::ExportSource> {
        Box::new(RecordingSource {
            journal: self.probe.journal.clone(),
            opened: Arc::clone(&self.probe.opened),
            plan: self.plan.clone(),
            video: 0,
            audio: 0,
        })
    }
}

/// One source of a test.
struct RecordingSource {
    journal: Journal,
    opened: Arc<Mutex<Vec<PathBuf>>>,
    plan: SourcePlan,
    video: usize,
    audio: usize,
}

impl gafira_media::ExportSource for RecordingSource {
    fn open(&mut self, path: &Path) -> Result<(), MediaError> {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(path.to_path_buf());
        Ok(())
    }

    fn next_video(&mut self) -> Result<Option<Flicks>, MediaError> {
        let Some(time) = self.plan.video.get(self.video).copied() else {
            return Ok(None);
        };
        self.video += 1;
        self.journal.record(Step::Decode);
        Ok(Some(time))
    }

    fn next_audio(&mut self) -> Result<Option<AudioBuf>, MediaError> {
        let Some(buffer) = self.plan.audio.get(self.audio).cloned() else {
            return Ok(None);
        };
        self.audio += 1;
        Ok(Some(buffer))
    }
}

/// The descriptor of every double of the test.
static DESCRIPTOR: Descriptor = Descriptor::new("test.exporter", MessageKey("test.exporter"), 1);

/// The codec of a test: it turns every frame into one packet, or fails.
pub struct LogCodec {
    journal: Journal,
    codec: CodecId,
    fail_request: bool,
    fail_send: Option<u32>,
}

impl LogCodec {
    /// A video codec that encodes.
    #[must_use]
    pub fn video(journal: Journal) -> Self {
        Self {
            journal,
            codec: CodecId::H264,
            fail_request: false,
            fail_send: None,
        }
    }

    /// An audio codec that encodes.
    #[must_use]
    pub fn audio(journal: Journal) -> Self {
        Self {
            journal,
            codec: CodecId::Opus,
            fail_request: false,
            fail_send: None,
        }
    }

    /// A codec whose encoder cannot be created.
    #[must_use]
    pub fn unavailable(journal: Journal) -> Self {
        Self {
            journal,
            codec: CodecId::H264,
            fail_request: true,
            fail_send: None,
        }
    }

    /// A codec that dies after `after` frames.
    #[must_use]
    pub fn lost(journal: Journal, after: u32) -> Self {
        Self {
            journal,
            codec: CodecId::H264,
            fail_request: false,
            fail_send: Some(after),
        }
    }
}

impl Describe for LogCodec {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl Codec for LogCodec {
    fn codec_id(&self) -> CodecId {
        self.codec
    }

    fn caps(&self) -> CodecCaps {
        CodecCaps {
            decode: true,
            encode: true,
            hardware: false,
            legal: LegalClass::RoyaltyFree,
        }
    }

    fn decoder(&self, _params: &CodecParams) -> Result<Box<dyn Decoder>, MediaError> {
        Ok(Box::new(FakeDecoder))
    }

    fn encoder(&self, _config: &EncoderConfig) -> Result<Box<dyn Encoder>, MediaError> {
        if self.fail_request {
            return Err(MediaError::SystemCodecUnavailable);
        }
        Ok(Box::new(LogEncoder {
            journal: self.journal.clone(),
            sent: 0,
            fail_send: self.fail_send,
            pending: None,
        }))
    }
}

/// The encoder of a test: one packet per frame.
struct LogEncoder {
    journal: Journal,
    sent: u32,
    fail_send: Option<u32>,
    pending: Option<Packet>,
}

impl Encoder for LogEncoder {
    fn send(&mut self, frame: Option<&Frame>) -> Result<(), MediaError> {
        let Some(frame) = frame else {
            return Ok(());
        };
        if self.fail_send == Some(self.sent) {
            return Err(MediaError::Corrupt {
                detail: "the encoder was lost".into(),
            });
        }
        self.journal.record(Step::Encode);
        let pts = match frame {
            Frame::Video(video) => video.pts,
            Frame::Audio(audio) => audio.pts(),
        };
        self.pending = Some(Packet {
            stream: StreamIdx(0),
            pts,
            dts: pts,
            keyframe: true,
            data: vec![0, 0],
        });
        self.sent = self.sent.saturating_add(1);
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<Packet>, MediaError> {
        Ok(self.pending.take())
    }
}

/// The container of a test: it hands out a recording muxer.
pub struct RecordingContainer {
    journal: Journal,
    probe: Probe,
}

impl RecordingContainer {
    /// A container over `probe`, recording into `journal`.
    #[must_use]
    pub fn new(journal: Journal, probe: Probe) -> Self {
        Self { journal, probe }
    }
}

impl Describe for RecordingContainer {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl Container for RecordingContainer {
    fn probe(&self, _head: &[u8]) -> Confidence {
        Confidence::High
    }

    fn open(&self, _src: Box<dyn ReadSeek + Send>) -> Result<Box<dyn Demuxer>, MediaError> {
        Err(MediaError::Corrupt {
            detail: "the test does not demux".into(),
        })
    }

    fn create(
        &self,
        dst: Box<dyn WriteSeek + Send>,
        _config: &MuxConfig,
    ) -> Result<Box<dyn Muxer>, MediaError> {
        Ok(Box::new(RecordingMuxer {
            journal: self.journal.clone(),
            probe: self.probe.clone(),
            out: dst,
        }))
    }
}

/// The muxer of a test: it logs, records and writes one byte per packet.
struct RecordingMuxer {
    journal: Journal,
    probe: Probe,
    out: Box<dyn WriteSeek + Send>,
}

impl Muxer for RecordingMuxer {
    fn add_stream(&mut self, stream: &StreamInfo) -> Result<StreamIdx, MediaError> {
        let mut streams = self
            .probe
            .streams
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let index = StreamIdx(u32::try_from(streams.len()).unwrap_or(u32::MAX));
        streams.push(stream.clone());
        Ok(index)
    }

    fn write_packet(&mut self, packet: &Packet) -> Result<(), MediaError> {
        self.journal.record(Step::Mux);
        self.probe
            .packets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(packet.clone());
        self.out.write_all(b"PK").map_err(MediaError::Io)?;
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<MuxReport, MediaError> {
        let packets = self
            .probe
            .packets
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len();
        let packets = u64::try_from(packets).unwrap_or(u64::MAX);
        Ok(MuxReport {
            size: Bytes(packets.saturating_mul(2)),
            packets,
        })
    }
}

/// The progress of a test: it logs and can cancel the token.
pub struct ProgressProbe {
    token: Option<CancelToken>,
    after: u32,
    reports: AtomicU32,
    fractions: Mutex<Vec<f32>>,
}

impl ProgressProbe {
    /// A progress that never cancels.
    #[must_use]
    pub fn new() -> Self {
        Self {
            token: None,
            after: 0,
            reports: AtomicU32::new(0),
            fractions: Mutex::new(Vec::new()),
        }
    }

    /// A progress that cancels `token` on the `after`-th report.
    #[must_use]
    pub fn cancelling(token: CancelToken, after: u32) -> Self {
        Self {
            token: Some(token),
            after,
            reports: AtomicU32::new(0),
            fractions: Mutex::new(Vec::new()),
        }
    }
}

impl Default for ProgressProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl Progress for ProgressProbe {
    fn report(&self, fraction: f32) {
        self.fractions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(fraction);
        let done = self
            .reports
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        if let Some(token) = &self.token
            && done == self.after
        {
            token.cancel();
        }
    }
}

/// A store the export never reads (ARQ-148).
pub struct NoStore;

impl ContentStore for NoStore {
    fn put(&self, _src: &mut dyn Read) -> Result<gafira_doc::ContentHash, StoreError> {
        Err(StoreError::Io(io::Error::other("no store")))
    }

    fn open(&self, hash: gafira_doc::ContentHash) -> Result<Box<dyn ReadSeek + Send>, StoreError> {
        Err(StoreError::NotFound { hash })
    }

    fn contains(&self, _hash: gafira_doc::ContentHash) -> Result<bool, StoreError> {
        Ok(false)
    }
}

/// An audio buffer of 100 ms at 48 kHz, silent.
#[must_use]
pub fn silent(pts_ms: i64) -> AudioBuf {
    let pts = Flicks::from_millis(pts_ms).expect("the time fits");
    let samples = vec![0.0_f32; 4_800];
    AudioBuf::new(pts, SampleRate::HZ_48_000, vec![samples.clone(), samples])
        .expect("the channels are equal")
}
