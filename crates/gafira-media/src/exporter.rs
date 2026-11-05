//! The video exporter of the media stream (FV-12, FV-24, ARQ-43).
//!
//! The exporter implements [`Exporter`] and drives the pipeline
//! decode → compose → encode → mux over a time range: it asks the source for
//! the decoded frames, composes each one through the page renderer the job
//! carries (ARQ-06), encodes it through an [`Encoder`] and writes the packets
//! to a [`Muxer`] (02 §4.5, ARQ-148).
//!
//! The codecs and the container writers are layer 4: this module never
//! encodes a format of its own (D-02, ARQ-77). It receives the codecs and the
//! container of the caller, owns the loop, the order, the progress, the
//! cancellation and the audio track, and reads its options from the job's
//! `ParamValues` in memory (ARQ-148).
//!
//! The export is a long operation: it runs at [`Priority::Background`] (02
//! §6.3), checks the cancel token once per frame and once per audio buffer,
//! and reports the frames done over the frames planned plus the finish, so
//! the fraction never reaches `1.0` before the file is written (ARQ-139).
//! The final file is created only once the mux has finished, so a cancelled
//! export leaves no partial final file (FV-12 (d), 02 §6.4).

use std::io::{self, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use gafira_doc::PageId;
use gafira_doc::contract::{
    CancelToken, ParamChoice, ParamDef, ParamKind, ParamSchema, ParamValues, Progress,
};
use gafira_doc::msg;
use gafira_doc::msg::{MessageKey, ToMessage};
use gafira_geom::IRect;
use gafira_ports::common::LegalClass;
use gafira_ports::export::{
    ExportCaps, ExportError, ExportIssue, ExportJob, ExportReport, ExportRequest, ExportScope,
    ExportSink, ExportedFile, Exporter, IssueSeverity, PageRenderRequest,
};
use gafira_ports::format::{FormatDescriptor, FormatKind};
use gafira_ports::media::{
    AudioBuf, Codec, CodecId, CodecParams, Container, Encoder, EncoderConfig, Frame, MediaError,
    MuxConfig, Muxer, StreamIdx, StreamInfo, VideoFrame,
};
use gafira_ports::render::{Priority, RenderError};
use gafira_time::{Flicks, FrameRate, SampleRate, TimeError, TimeRange};

/// The id of the export frame rate option (FV-24).
pub const OPTION_FPS: &str = "fps";
/// The id of the video codec option (FV-24).
pub const OPTION_CODEC: &str = "codec";
/// The id of the video bit rate option, in bits per second (FV-24).
pub const OPTION_BITRATE: &str = "bitrate";
/// The id of the output width option (FV-24).
pub const OPTION_WIDTH: &str = "width";
/// The id of the output height option (FV-24).
pub const OPTION_HEIGHT: &str = "height";
/// The id of the audio codec option (FV-24).
pub const OPTION_AUDIO_CODEC: &str = "audio_codec";
/// The id of the audio bit rate option, in bits per second (FV-24).
pub const OPTION_AUDIO_BITRATE: &str = "audio_bitrate";
/// The id of the audio sample rate option (FV-24).
pub const OPTION_SAMPLE_RATE: &str = "sample_rate";
/// The id of the audio channel count option (FV-24).
pub const OPTION_CHANNELS: &str = "channels";
/// The id of the start of the exported range, in milliseconds (FV-12).
pub const OPTION_START_MS: &str = "start_ms";
/// The id of the end of the exported range, in milliseconds (FV-12).
pub const OPTION_END_MS: &str = "end_ms";

/// The video codecs the exporter can ask for by id (FV-24, D-02).
static VIDEO_CODECS: &[ParamChoice] = &[
    ParamChoice {
        id: "av1",
        label: MessageKey("export.codec.av1"),
    },
    ParamChoice {
        id: "h264",
        label: MessageKey("export.codec.h264"),
    },
    ParamChoice {
        id: "vp9",
        label: MessageKey("export.codec.vp9"),
    },
];

/// The audio codecs the exporter can ask for by id (FV-24, D-02).
static AUDIO_CODECS: &[ParamChoice] = &[
    ParamChoice {
        id: "opus",
        label: MessageKey("export.codec.opus"),
    },
    ParamChoice {
        id: "aac",
        label: MessageKey("export.codec.aac"),
    },
    ParamChoice {
        id: "flac",
        label: MessageKey("export.codec.flac"),
    },
    ParamChoice {
        id: "vorbis",
        label: MessageKey("export.codec.vorbis"),
    },
];

/// The export frame rates of FV-24, from 23,976 to 60.
static FRAME_RATES: &[ParamChoice] = &[
    ParamChoice {
        id: "23.976",
        label: MessageKey("export.fps.23_976"),
    },
    ParamChoice {
        id: "24",
        label: MessageKey("export.fps.24"),
    },
    ParamChoice {
        id: "25",
        label: MessageKey("export.fps.25"),
    },
    ParamChoice {
        id: "29.97",
        label: MessageKey("export.fps.29_97"),
    },
    ParamChoice {
        id: "30",
        label: MessageKey("export.fps.30"),
    },
    ParamChoice {
        id: "48",
        label: MessageKey("export.fps.48"),
    },
    ParamChoice {
        id: "50",
        label: MessageKey("export.fps.50"),
    },
    ParamChoice {
        id: "59.94",
        label: MessageKey("export.fps.59_94"),
    },
    ParamChoice {
        id: "60",
        label: MessageKey("export.fps.60"),
    },
];

/// The audio sample rates of FV-24.
static SAMPLE_RATES: &[ParamChoice] = &[
    ParamChoice {
        id: "44100",
        label: MessageKey("export.sample_rate.44100"),
    },
    ParamChoice {
        id: "48000",
        label: MessageKey("export.sample_rate.48000"),
    },
];

/// The format descriptor of the video export (02 §4.5).
pub const VIDEO_FORMAT: FormatDescriptor = FormatDescriptor {
    id: "video.mp4",
    extensions: &["mp4"],
    mime: &["video/mp4"],
    signatures: &[],
    kind: FormatKind::Video,
    legal: LegalClass::RoyaltyFree,
};

/// The medium an export reads (FV-22 (b), ARQ-146).
///
/// The export always reads the original file. The light copy the cache may
/// hold is carried for the caller, never read here: a copy has another
/// resolution than the original, so an export never uses one (FV-22 (b),
/// 02 §6.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportMedium {
    /// The original file of the medium.
    original: PathBuf,
    /// The light copy of the medium, when the cache holds one.
    copy: Option<PathBuf>,
}

impl ExportMedium {
    /// The original file of the medium.
    #[must_use]
    pub fn new(original: impl Into<PathBuf>) -> Self {
        Self {
            original: original.into(),
            copy: None,
        }
    }

    /// The same medium, with the light copy the cache holds.
    #[must_use]
    pub fn with_copy(mut self, copy: impl Into<PathBuf>) -> Self {
        self.copy = Some(copy.into());
        self
    }

    /// The original file of the medium.
    #[must_use]
    pub fn original(&self) -> &Path {
        &self.original
    }

    /// The light copy of the medium, when the cache holds one.
    #[must_use]
    pub fn copy(&self) -> Option<&Path> {
        self.copy.as_deref()
    }

    /// The file the export reads: the original, never a light copy
    /// (FV-22 (b), ARQ-146).
    #[must_use]
    pub fn read_path(&self) -> &Path {
        &self.original
    }
}

/// What an export decodes (FV-12, 02 §6.5).
///
/// The crate decodes nothing itself: the caller opens the medium and hands
/// the decoded frames over, as the proxy producer (ARQ-146) and the playback
/// source (ARQ-147) do. One video time is one frame, so the cancel token has
/// a checkpoint per frame (ARQ-139).
pub trait ExportSource: Send {
    /// Opens the file the export reads.
    ///
    /// # Errors
    /// [`MediaError`] when the medium cannot be opened.
    fn open(&mut self, path: &Path) -> Result<(), MediaError>;

    /// The presentation time of the next decoded video frame, or `None` at
    /// the end of the source.
    ///
    /// # Errors
    /// [`MediaError`] when the decode fails.
    fn next_video(&mut self) -> Result<Option<Flicks>, MediaError>;

    /// The next decoded audio buffer, or `None` when the audio is over.
    ///
    /// # Errors
    /// [`MediaError`] when the decode fails.
    fn next_audio(&mut self) -> Result<Option<AudioBuf>, MediaError>;
}

/// Creates the source of one export (FV-12).
///
/// The exporter holds the factory, not the source: `Exporter::export` takes
/// `&self`, so every run gets its own source and no run depends on the state
/// of the one before (ARQ-140).
pub trait ExportSourceFactory: Send + Sync {
    /// A fresh source over the medium of the export.
    fn source(&self) -> Box<dyn ExportSource>;
}

/// The video exporter of `gafira-media` (FV-12, ARQ-148).
///
/// It receives the medium, the source factory, the container and the two
/// codecs of the caller, and writes the file called `output` through the
/// `ExportSink` of the job. The layer-4 adapters supply the codecs and the
/// container: this type never encodes or muxes a format of its own
/// (D-02, ARQ-77).
pub struct VideoExporter<'a> {
    /// The identity of the format.
    descriptor: FormatDescriptor,
    /// The options panel of FV-24.
    schema: ParamSchema,
    /// The medium the export reads.
    medium: ExportMedium,
    /// The decoded frames of the export.
    source: Box<dyn ExportSourceFactory>,
    /// The container writer of the output.
    container: &'a dyn Container,
    /// The video codec.
    video: &'a dyn Codec,
    /// The audio codec.
    audio: &'a dyn Codec,
    /// The name of the output file.
    output: String,
}

impl<'a> VideoExporter<'a> {
    /// A video exporter with the ports of the caller.
    #[must_use]
    pub fn new(
        medium: ExportMedium,
        source: Box<dyn ExportSourceFactory>,
        container: &'a dyn Container,
        video: &'a dyn Codec,
        audio: &'a dyn Codec,
        output: impl Into<String>,
    ) -> Self {
        Self {
            descriptor: VIDEO_FORMAT,
            schema: schema(),
            medium,
            source,
            container,
            video,
            audio,
            output: output.into(),
        }
    }

    /// The priority the export queue runs the job at: [`Priority::Background`]
    /// (02 §6.3, ARQ-43).
    #[must_use]
    pub const fn priority(&self) -> Priority {
        Priority::Background
    }

    /// The medium the export reads.
    #[must_use]
    pub const fn medium(&self) -> &ExportMedium {
        &self.medium
    }

    /// The options of one job, or the error the export starts with.
    ///
    /// The options are read from the job's `ParamValues` in memory and never
    /// serialized (ARQ-148). An option the request lacks takes its default,
    /// and the codec it asks for must be the one the injected codec writes.
    fn options(&self, request: &ExportRequest) -> Result<ExportOptions, ExportError> {
        let values = self
            .schema
            .resolve(&request.options)
            .map_err(|_| invalid_input("the export options do not fit the schema"))?;
        ExportOptions::parse(&values, self.video, self.audio)
    }

    /// The page the export renders.
    ///
    /// The timeline is not part of this cut (design `O1`), so the export runs
    /// over the first page of the request's scope and the range says which
    /// part of it is exported (FV-12).
    fn page(request: &ExportRequest) -> Result<PageId, ExportError> {
        match &request.scope {
            ExportScope::Pages(pages) => pages
                .first()
                .copied()
                .ok_or_else(|| invalid_input("the export scope holds no page")),
            ExportScope::Nodes { .. } => {
                Err(invalid_input("a video export needs pages, not nodes"))
            }
        }
    }

    /// The info of the video stream (02 §4.8).
    fn video_stream(&self, options: &ExportOptions) -> StreamInfo {
        StreamInfo {
            index: StreamIdx(0),
            codec: options.codec,
            params: CodecParams::Video {
                width: options.width,
                height: options.height,
                frame_rate: options.rate,
            },
            duration: Some(options.range.duration()),
        }
    }

    /// The info of the audio stream (02 §4.8).
    fn audio_stream(&self, options: &ExportOptions) -> StreamInfo {
        StreamInfo {
            index: StreamIdx(1),
            codec: options.audio_codec,
            params: CodecParams::Audio {
                sample_rate: options.sample_rate,
                channels: options.channels,
            },
            duration: Some(options.range.duration()),
        }
    }

    /// The video encoder of the job.
    fn video_encoder(&self, options: &ExportOptions) -> Result<Box<dyn Encoder>, ExportError> {
        let config = EncoderConfig {
            codec: options.codec,
            params: CodecParams::Video {
                width: options.width,
                height: options.height,
                frame_rate: options.rate,
            },
            bitrate: options.bitrate,
        };
        Ok(self.video.encoder(&config)?)
    }

    /// The audio encoder of the job.
    fn audio_encoder(&self, options: &ExportOptions) -> Result<Box<dyn Encoder>, ExportError> {
        let config = EncoderConfig {
            codec: options.audio_codec,
            params: CodecParams::Audio {
                sample_rate: options.sample_rate,
                channels: options.channels,
            },
            bitrate: options.audio_bitrate,
        };
        Ok(self.audio.encoder(&config)?)
    }
}

impl Exporter for VideoExporter<'_> {
    fn descriptor(&self) -> &FormatDescriptor {
        &self.descriptor
    }

    fn capabilities(&self) -> ExportCaps {
        ExportCaps {
            animation: true,
            ..ExportCaps::default()
        }
    }

    fn options_schema(&self) -> &ParamSchema {
        &self.schema
    }

    fn validate(&self, request: &ExportRequest) -> Vec<ExportIssue> {
        if let Err(error) = Self::page(request) {
            return vec![issue(error)];
        }
        match self.options(request) {
            Ok(_) => Vec::new(),
            Err(error) => vec![issue(error)],
        }
    }

    fn export(
        &self,
        job: &ExportJob<'_>,
        sink: &mut dyn ExportSink,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<ExportReport, ExportError> {
        let options = self.options(job.request)?;
        let page = Self::page(job.request)?;
        // The output lives in memory until the mux finishes, so the run
        // creates the final name only once the result is complete (FV-12 (d),
        // 02 §6.4). A cancelled export leaves no partial final file.
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedOutput::new(Arc::clone(&output));
        let mut muxer = self
            .container
            .create(Box::new(writer), &MuxConfig::default())?;
        let video_index = muxer.add_stream(&self.video_stream(&options))?;
        let mut video_encoder = self.video_encoder(&options)?;
        let mut source = self.source.source();
        // The export opens the original, never a light copy (FV-22 (b)).
        source.open(self.medium.read_path()).map_err(media_error)?;
        let frames = options.frames();
        let planned = frames.saturating_add(1);
        // The video track is anchored to the start of the range, as the audio
        // is: the first video frame of the range is presented at the start
        // of the range, so the A/V difference of the result stays below the
        // 40 ms of FV-12 (b) whatever the times of the source are.
        let mut video_origin: Option<Flicks> = None;
        for index in 0..frames {
            cancel.check().map_err(ExportError::Cancelled)?;
            let Some(time) = source.next_video().map_err(media_error)? else {
                break;
            };
            let image = job
                .pages
                .render(
                    &PageRenderRequest {
                        page,
                        time,
                        scale: 1.0,
                        region: options.region(),
                        transparent: false,
                    },
                    cancel,
                )
                .map_err(render_error)?;
            let first = *video_origin.get_or_insert(time);
            let pts = Flicks::from_ticks(
                options
                    .range
                    .start()
                    .ticks()
                    .saturating_add(time.ticks().saturating_sub(first.ticks())),
            );
            video_encoder.send(Some(&Frame::Video(VideoFrame { pts, image })))?;
            if let Some(mut packet) = video_encoder.receive()? {
                packet.stream = video_index;
                muxer.write_packet(&packet)?;
            }
            progress.report(fraction(index.saturating_add(1), planned));
        }
        // The decode is over: flush the encoder so its last packets reach the
        // muxer before the file is finished (FV-12).
        drain(&mut *video_encoder, video_index, &mut *muxer)?;
        self.write_audio(&mut *source, &mut *muxer, &options, cancel)?;
        muxer.finish()?;
        // The last checkpoint sits before the final name is created, so a
        // cancel after the last frame never reaches 1.0 (ARQ-139).
        cancel.check().map_err(ExportError::Cancelled)?;
        let bytes = export_bytes(&output);
        let mut file = sink.create(&self.output)?;
        file.write_all(&bytes)?;
        file.flush()?;
        progress.report(1.0);
        Ok(ExportReport {
            files: vec![ExportedFile {
                name: self.output.clone(),
                bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            }],
            warnings: Vec::new(),
        })
    }
}

impl VideoExporter<'_> {
    /// Writes the audio track of the range (FV-12 (b)).
    ///
    /// The audio is anchored to the start of the range, and so is the video:
    /// the first audio packet and the first video packet share their
    /// presentation time, so the A/V difference of the result is 0 and stays
    /// below the 40 ms of FV-12 (b) whatever the times of the source are.
    /// The audio the range does not cover is dropped.
    ///
    /// # Errors
    /// [`MediaError`] when the encoder or the muxer fails, and
    /// [`ExportError::Cancelled`] when the caller cancels.
    fn write_audio(
        &self,
        source: &mut dyn ExportSource,
        muxer: &mut dyn Muxer,
        options: &ExportOptions,
        cancel: &CancelToken,
    ) -> Result<(), ExportError> {
        let mut audio: Option<(StreamIdx, Box<dyn Encoder>)> = None;
        let mut origin: Option<Flicks> = None;
        while let Some(buffer) = source.next_audio().map_err(media_error)? {
            cancel.check().map_err(ExportError::Cancelled)?;
            let first = *origin.get_or_insert(buffer.pts());
            let time = Flicks::from_ticks(
                options
                    .range
                    .start()
                    .ticks()
                    .saturating_add(buffer.pts().ticks().saturating_sub(first.ticks())),
            );
            if !options.range.is_empty() && time >= options.range.end() {
                break;
            }
            if audio.is_none() {
                let index = muxer.add_stream(&self.audio_stream(options))?;
                audio = Some((index, self.audio_encoder(options)?));
            }
            let Some((index, encoder)) = audio.as_mut() else {
                break;
            };
            let aligned = AudioBuf::new(time, buffer.sample_rate(), buffer.channels().to_vec())?;
            encoder.send(Some(&Frame::Audio(aligned)))?;
            if let Some(mut packet) = encoder.receive()? {
                packet.stream = *index;
                muxer.write_packet(&packet)?;
            }
        }
        if let Some((index, encoder)) = audio.as_mut() {
            drain(&mut **encoder, *index, muxer)?;
        }
        Ok(())
    }
}

/// Feeds `None` to `encoder` and muxes what it yields.
///
/// # Errors
/// [`MediaError`] when the encoder or the muxer fails.
fn drain(
    encoder: &mut dyn Encoder,
    index: StreamIdx,
    muxer: &mut dyn Muxer,
) -> Result<(), ExportError> {
    encoder.send(None)?;
    while let Some(mut packet) = encoder.receive()? {
        packet.stream = index;
        muxer.write_packet(&packet)?;
    }
    Ok(())
}

/// The issue of an export the exporter rejects before it starts (FD-58).
fn issue(error: ExportError) -> ExportIssue {
    let message = match error {
        ExportError::Media(MediaError::UnsupportedCodec { .. }) => {
            msg!("error.export.codec_unavailable")
        }
        ExportError::Media(error) => error.to_message(),
        _ => msg!("error.export.options"),
    };
    ExportIssue {
        severity: IssueSeverity::Error,
        message,
    }
}

/// The options of one video export, parsed from the job's `ParamValues`
/// (FV-24, ARQ-148).
#[derive(Clone, Copy, Debug, PartialEq)]
struct ExportOptions {
    codec: CodecId,
    audio_codec: CodecId,
    bitrate: Option<u32>,
    audio_bitrate: Option<u32>,
    rate: FrameRate,
    width: u32,
    height: u32,
    sample_rate: SampleRate,
    channels: u16,
    range: TimeRange,
}

impl ExportOptions {
    /// Reads every option, filling the defaults the schema already applied.
    ///
    /// # Errors
    /// [`MediaError::UnsupportedCodec`] when the requested codec is not the
    /// one the injected codec writes, and the I/O error of an option or a
    /// range the schema cannot express.
    fn parse(
        values: &ParamValues,
        video: &dyn Codec,
        audio: &dyn Codec,
    ) -> Result<Self, ExportError> {
        let codec = video_codec(values.choice(OPTION_CODEC).unwrap_or_default())
            .ok_or_else(|| invalid_input("the video codec option is unknown"))?;
        if video.codec_id() != codec {
            return Err(ExportError::Media(MediaError::UnsupportedCodec { codec }));
        }
        let audio_codec = audio_codec(values.choice(OPTION_AUDIO_CODEC).unwrap_or_default())
            .ok_or_else(|| invalid_input("the audio codec option is unknown"))?;
        if audio.codec_id() != audio_codec {
            return Err(ExportError::Media(MediaError::UnsupportedCodec {
                codec: audio_codec,
            }));
        }
        let rate = frame_rate(values.choice(OPTION_FPS).unwrap_or_default())
            .ok_or_else(|| invalid_input("the frame rate option is unknown"))?;
        let width = positive(values.int(OPTION_WIDTH), "the export width")?;
        let height = positive(values.int(OPTION_HEIGHT), "the export height")?;
        if width % 2 != 0 || height % 2 != 0 {
            return Err(invalid_input("the resolution must be a multiple of 2"));
        }
        let sample_rate = sample_rate(values.choice(OPTION_SAMPLE_RATE).unwrap_or_default())
            .ok_or_else(|| invalid_input("the sample rate option is unknown"))?;
        let channels = u16::try_from(values.int(OPTION_CHANNELS).unwrap_or(2))
            .map_err(|_| invalid_input("the channel count does not fit"))?;
        let start =
            Flicks::from_millis(values.int(OPTION_START_MS).unwrap_or(0)).map_err(time_error)?;
        let end =
            Flicks::from_millis(values.int(OPTION_END_MS).unwrap_or(0)).map_err(time_error)?;
        let range = TimeRange::from_bounds(start, end).map_err(time_error)?;
        Ok(Self {
            codec,
            audio_codec,
            bitrate: bitrate(values.int(OPTION_BITRATE)),
            audio_bitrate: bitrate(values.int(OPTION_AUDIO_BITRATE)),
            rate,
            width,
            height,
            sample_rate,
            channels,
            range,
        })
    }

    /// The frames of the range at the export rate (FV-12).
    ///
    /// The whole frames that fit in the range are the frames of the export;
    /// a partial tail is not encoded (the fps conversion of FV-24 belongs to
    /// the compositor).
    fn frames(&self) -> u32 {
        let ticks = self.rate.flicks_per_frame().ticks();
        if ticks <= 0 {
            return 0;
        }
        u32::try_from(self.range.duration().ticks() / ticks).unwrap_or(u32::MAX)
    }

    /// The region of one composed frame.
    fn region(&self) -> IRect {
        IRect::new(
            0,
            0,
            i32::try_from(self.width).unwrap_or(i32::MAX),
            i32::try_from(self.height).unwrap_or(i32::MAX),
        )
    }
}

/// The options panel of the video export (FV-24).
fn schema() -> ParamSchema {
    ParamSchema::new(vec![
        ParamDef {
            id: OPTION_CODEC,
            label: MessageKey("export.codec"),
            kind: ParamKind::Choice {
                options: VIDEO_CODECS,
                default: "av1",
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_BITRATE,
            label: MessageKey("export.bitrate"),
            kind: ParamKind::Int {
                min: 0,
                max: 200_000_000,
                default: 8_000_000,
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_FPS,
            label: MessageKey("export.fps"),
            kind: ParamKind::Choice {
                options: FRAME_RATES,
                default: "30",
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_WIDTH,
            label: MessageKey("export.width"),
            kind: ParamKind::Int {
                min: 2,
                max: 8192,
                default: 1920,
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_HEIGHT,
            label: MessageKey("export.height"),
            kind: ParamKind::Int {
                min: 2,
                max: 8192,
                default: 1080,
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_AUDIO_CODEC,
            label: MessageKey("export.audio_codec"),
            kind: ParamKind::Choice {
                options: AUDIO_CODECS,
                default: "opus",
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_AUDIO_BITRATE,
            label: MessageKey("export.audio_bitrate"),
            kind: ParamKind::Int {
                min: 0,
                max: 320_000,
                default: 128_000,
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_SAMPLE_RATE,
            label: MessageKey("export.sample_rate"),
            kind: ParamKind::Choice {
                options: SAMPLE_RATES,
                default: "48000",
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_CHANNELS,
            label: MessageKey("export.channels"),
            kind: ParamKind::Int {
                min: 1,
                max: 8,
                default: 2,
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_START_MS,
            label: MessageKey("export.start"),
            kind: ParamKind::Int {
                min: 0,
                max: i64::from(i32::MAX),
                default: 0,
            },
            animatable: false,
        },
        ParamDef {
            id: OPTION_END_MS,
            label: MessageKey("export.end"),
            kind: ParamKind::Int {
                min: 0,
                max: i64::from(i32::MAX),
                default: 0,
            },
            animatable: false,
        },
    ])
}

/// The video codec called `id` (FV-24).
fn video_codec(id: &str) -> Option<CodecId> {
    match id {
        "av1" => Some(CodecId::Av1),
        "h264" => Some(CodecId::H264),
        "vp9" => Some(CodecId::Vp9),
        _ => None,
    }
}

/// The audio codec called `id` (FV-24).
fn audio_codec(id: &str) -> Option<CodecId> {
    match id {
        "opus" => Some(CodecId::Opus),
        "aac" => Some(CodecId::Aac),
        "flac" => Some(CodecId::Flac),
        "vorbis" => Some(CodecId::Vorbis),
        _ => None,
    }
}

/// The frame rate called `id` (FV-24).
fn frame_rate(id: &str) -> Option<FrameRate> {
    match id {
        "23.976" => Some(FrameRate::FPS_23_976),
        "24" => Some(FrameRate::FPS_24),
        "25" => Some(FrameRate::FPS_25),
        "29.97" => Some(FrameRate::FPS_29_97),
        "30" => Some(FrameRate::FPS_30),
        "48" => Some(FrameRate::FPS_48),
        "50" => Some(FrameRate::FPS_50),
        "59.94" => Some(FrameRate::FPS_59_94),
        "60" => Some(FrameRate::FPS_60),
        _ => None,
    }
}

/// The sample rate called `id` (FV-24).
fn sample_rate(id: &str) -> Option<SampleRate> {
    match id {
        "44100" => Some(SampleRate::HZ_44_100),
        "48000" => Some(SampleRate::HZ_48_000),
        _ => None,
    }
}

/// A positive whole option, or the invalid input error naming it.
fn positive(value: Option<i64>, what: &str) -> Result<u32, ExportError> {
    match value.and_then(|v| u32::try_from(v).ok()) {
        Some(v) if v > 0 => Ok(v),
        _ => Err(invalid_input(what)),
    }
}

/// A bit rate option; `0` leaves it to the codec.
fn bitrate(value: Option<i64>) -> Option<u32> {
    match value.and_then(|v| u32::try_from(v).ok()) {
        Some(0) | None => None,
        Some(v) => Some(v),
    }
}

/// The error of an option or a range that does not fit.
fn invalid_input(what: &str) -> ExportError {
    ExportError::Io(io::Error::new(
        io::ErrorKind::InvalidInput,
        what.to_string(),
    ))
}

/// The cancelled export of a render that saw the token fire.
///
/// `Exporter::export` answers [`ExportError::Cancelled`] when the caller
/// cancels, also when the renderer notices first (02 §4.5).
fn render_error(error: RenderError) -> ExportError {
    match error {
        RenderError::Cancelled(cancelled) => ExportError::Cancelled(cancelled),
        other => ExportError::Render(other),
    }
}

/// The cancelled export of a decode that saw the token fire.
fn media_error(error: MediaError) -> ExportError {
    match error {
        MediaError::Cancelled(cancelled) => ExportError::Cancelled(cancelled),
        other => ExportError::Media(other),
    }
}

/// The export error of a time conversion that does not fit.
fn time_error(error: TimeError) -> ExportError {
    ExportError::Io(io::Error::new(
        io::ErrorKind::InvalidInput,
        error.to_string(),
    ))
}

/// The fraction of the units done over the units planned (ARQ-139).
///
/// The units planned are the frames plus the finish of the file, so the
/// fraction stays below `1.0` until the file is written.
fn fraction(done: u32, planned: u32) -> f32 {
    done as f32 / planned.max(1) as f32
}

/// The bytes the mux wrote, once the mux is over.
fn export_bytes(output: &Mutex<Vec<u8>>) -> Vec<u8> {
    output
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// The output file while the mux writes it.
///
/// The muxer takes a `WriteSeek`, so the bytes are collected here and handed
/// to the sink only once the mux has finished (FV-12 (d)). The real temp file
/// and its rename belong to the layer-4 `ExportSink`, whose contract puts
/// them there (02 §6.4).
struct SharedOutput {
    data: Arc<Mutex<Vec<u8>>>,
    position: usize,
}

impl SharedOutput {
    /// A writer over `data`, at its start.
    fn new(data: Arc<Mutex<Vec<u8>>>) -> Self {
        Self { data, position: 0 }
    }
}

impl Write for SharedOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut data = lock(&self.data);
        let end = self
            .position
            .checked_add(buf.len())
            .ok_or_else(|| io::Error::other("the export output overflows"))?;
        if data.len() < end {
            data.resize(end, 0);
        }
        data[self.position..end].copy_from_slice(buf);
        self.position = end;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for SharedOutput {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let len = i64::try_from(lock(&self.data).len())
            .map_err(|_| io::Error::other("the export output overflows"))?;
        let position = i64::try_from(self.position)
            .map_err(|_| io::Error::other("the export output overflows"))?;
        let target = match from {
            SeekFrom::Start(offset) => {
                i64::try_from(offset).map_err(|_| io::Error::other("the seek offset overflows"))?
            }
            SeekFrom::Current(offset) => position.saturating_add(offset),
            SeekFrom::End(offset) => len.saturating_add(offset),
        };
        self.position = usize::try_from(target)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a negative seek"))?;
        u64::try_from(self.position).map_err(|_| io::Error::other("the seek offset overflows"))
    }
}

/// Locks a mutex, ignoring poisoning: the data it guards is plain.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
