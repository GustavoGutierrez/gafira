//! The video exporter: the pipeline, the audio track, the cancellation, the
//! progress and the original medium (FV-12, FV-24, ARQ-43).

mod exporter_support;

use std::path::PathBuf;

use exporter_support::{
    CountingRenderer, DeafRenderer, Journal, LogCodec, MemSink, NoStore, Probe, ProgressProbe,
    RecordingContainer, RecordingSourceFactory, SinkProbe, SourcePlan, Step, silent,
};
use gafira_doc::contract::{CancelToken, ParamValue, ParamValues, Progress};
use gafira_doc::{Document, PageId};
use gafira_geom::IRect;
use gafira_media::{
    ExportMedium, OPTION_AUDIO_CODEC, OPTION_CODEC, OPTION_END_MS, OPTION_FPS, OPTION_HEIGHT,
    OPTION_START_MS, OPTION_WIDTH, SYNC_TOLERANCE, VideoExporter,
};
use gafira_ports::export::{
    ExportError, ExportJob, ExportReport, ExportRequest, ExportScope, Exporter, PageRenderer,
};
use gafira_ports::media::{CodecId, MediaError, StreamIdx};
use gafira_ports::render::Priority;
use gafira_testkit::progress::check_progress_contract;
use gafira_time::{Flicks, FrameRate};

/// The page every test exports.
fn page() -> PageId {
    PageId::new(7)
}

/// The options of an export that can run.
fn values(start_ms: i64, end_ms: i64, fps: &str) -> ParamValues {
    let mut values = ParamValues::new();
    values.set(OPTION_CODEC, ParamValue::Choice("h264".into()));
    values.set(OPTION_AUDIO_CODEC, ParamValue::Choice("opus".into()));
    values.set(OPTION_FPS, ParamValue::Choice(fps.into()));
    values.set(OPTION_WIDTH, ParamValue::Int(64));
    values.set(OPTION_HEIGHT, ParamValue::Int(36));
    values.set(OPTION_START_MS, ParamValue::Int(start_ms));
    values.set(OPTION_END_MS, ParamValue::Int(end_ms));
    values
}

/// The request of an export over the single page of the test.
fn request(values: ParamValues) -> ExportRequest {
    ExportRequest {
        format_id: "video.mp4".into(),
        scope: ExportScope::Pages(vec![page()]),
        options: values,
    }
}

/// The frames of a range at `rate`, as the source would decode them.
fn timeline(start_ms: i64, end_ms: i64, rate: FrameRate) -> Vec<Flicks> {
    let start = Flicks::from_millis(start_ms).expect("the start fits");
    let end = Flicks::from_millis(end_ms).expect("the end fits");
    let frame = rate.flicks_per_frame().ticks();
    let count = (end.ticks() - start.ticks()) / frame;
    (0..count)
        .map(|index| Flicks::from_ticks(start.ticks() + index * frame))
        .collect()
}

/// Runs one export through the job the port fixes.
fn run(
    exporter: &VideoExporter<'_>,
    request: &ExportRequest,
    renderer: &dyn PageRenderer,
    sink: &mut MemSink,
    progress: &dyn Progress,
    cancel: &CancelToken,
) -> Result<ExportReport, ExportError> {
    let document = Document::default();
    let job = ExportJob {
        request,
        document: &document,
        store: &NoStore,
        pages: renderer,
    };
    exporter.export(&job, sink, progress, cancel)
}

// FV-12: the pipeline is decode → compose → encode → mux in that order, and
// the number of encoded frames is the frames of the range at the export rate.
#[test]
fn fv_12_the_pipeline_orders_the_stages_and_encodes_the_frames_of_the_range() {
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe.clone());
    let video = LogCodec::video(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 2_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let sink_probe = SinkProbe::new();
    let mut sink = MemSink::new(sink_probe.clone());
    let request = request(values(0, 2_000, "25"));

    let report = run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::new(),
        &CancelToken::new(),
    )
    .expect("the export completes");

    assert_eq!(
        journal.steps().get(..4),
        Some([Step::Decode, Step::Compose, Step::Encode, Step::Mux].as_slice()),
        "the pipeline is decode, compose, encode, mux in that order"
    );
    assert_eq!(
        journal.count(Step::Encode),
        50,
        "the frames of a range of 2 s at 25 fps are encoded"
    );
    assert_eq!(
        journal.count(Step::Mux),
        50,
        "every encoded frame reaches the muxer"
    );
    assert_eq!(sink_probe.names(), vec!["reel.mp4".to_string()]);
    assert_eq!(report.files.len(), 1, "the export writes one file");
    assert!(
        report.files[0].bytes > 0,
        "the muxed packets reach the output file"
    );
}

// FV-12 (b): the audio track is muxed in and the A/V difference of the result
// stays below the 40 ms of the preview tolerance.
#[test]
fn fv_12_the_audio_track_is_muxed_and_the_av_difference_is_below_40_ms() {
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe.clone());
    let video = LogCodec::video(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(500, 2_500, FrameRate::FPS_25),
            audio: (0..20).map(silent).collect(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let mut sink = MemSink::new(SinkProbe::new());
    let request = request(values(500, 2_500, "25"));

    run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::new(),
        &CancelToken::new(),
    )
    .expect("the export completes");

    assert_eq!(
        probe.streams().len(),
        2,
        "the muxer is given the video stream and the audio track"
    );
    let video_packets = probe.stream_packets(StreamIdx(0));
    let audio_packets = probe.stream_packets(StreamIdx(1));
    assert!(
        !audio_packets.is_empty(),
        "the audio track carries packets, not an empty stream FV-12"
    );
    let first_video = video_packets
        .first()
        .expect("the video has a first packet")
        .pts
        .ticks();
    let first_audio = audio_packets
        .first()
        .expect("the audio has a first packet")
        .pts
        .ticks();
    let drift = (first_audio - first_video).abs();
    assert!(
        drift < SYNC_TOLERANCE.ticks(),
        "the A/V difference is below 40 ms: {drift} ticks against {}",
        SYNC_TOLERANCE.ticks()
    );
    assert_eq!(
        first_audio,
        Flicks::from_millis(500)
            .expect("the range start fits")
            .ticks(),
        "the audio track is anchored to the start of the range"
    );
}

// FV-12 (d): a cancelled export leaves no partial final file, and the same
// exporter runs the next job (the queue stays usable).
#[test]
fn fv_12_a_cancelled_export_leaves_no_partial_file_and_the_next_one_runs() {
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe.clone());
    let video = LogCodec::video(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 2_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let sink_probe = SinkProbe::new();
    let request = request(values(0, 2_000, "25"));

    let token = CancelToken::new();
    let mut sink = MemSink::new(sink_probe.clone());
    let cancelled = run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::cancelling(token.clone(), 3),
        &token,
    );
    assert!(
        matches!(cancelled, Err(ExportError::Cancelled(_))),
        "a cancelled export answers the cancelled error: {cancelled:?}"
    );
    assert_eq!(
        sink_probe.names(),
        Vec::<String>::new(),
        "a cancelled export creates no final file"
    );

    // The queue stays usable: the same exporter runs the next job.
    let mut sink = MemSink::new(sink_probe.clone());
    let report = run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::new(),
        &CancelToken::new(),
    )
    .expect("the next export of the queue runs");
    assert_eq!(report.files.len(), 1);
    assert_eq!(
        sink_probe.names(),
        vec!["reel.mp4".to_string()],
        "the next export writes its file"
    );
}

// ARQ-43, ARQ-139: the export checks the token per frame, reports a fraction
// that only reaches 1.0 on success, and runs at Background priority.
#[test]
fn arq_139_the_export_honours_the_progress_contract_at_background_priority() {
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe.clone());
    let video = LogCodec::video(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 2_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );

    let contract = check_progress_contract(0.02, |cancel, progress| {
        let request = request(values(0, 2_000, "25"));
        let mut sink = MemSink::new(SinkProbe::new());
        match run(&exporter, &request, &renderer, &mut sink, progress, cancel) {
            Ok(_) => Ok(false),
            Err(ExportError::Cancelled(_)) => Ok(true),
            Err(error) => Err(error),
        }
    });
    match contract {
        Ok(verdict) => assert_eq!(verdict, Ok(()), "the export follows the progress contract"),
        Err(error) => panic!("the export failed: {error}"),
    }
    assert_eq!(
        exporter.priority(),
        Priority::Background,
        "the export queue runs the job at Background priority"
    );
}

// ARQ-139: the exporter checks the cancel token per frame itself, even when
// a port does not look at it.
#[test]
fn arq_139_the_export_checks_the_cancel_token_per_frame() {
    let journal = Journal::new();
    let probe = Probe::new();
    let container = RecordingContainer::new(journal.clone(), probe);
    let video = LogCodec::video(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 2_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let renderer = DeafRenderer;

    let contract = check_progress_contract(0.02, |cancel, progress| {
        let request = request(values(0, 2_000, "25"));
        let mut sink = MemSink::new(SinkProbe::new());
        match run(&exporter, &request, &renderer, &mut sink, progress, cancel) {
            Ok(_) => Ok(false),
            Err(ExportError::Cancelled(_)) => Ok(true),
            Err(error) => Err(error),
        }
    });
    match contract {
        Ok(verdict) => assert_eq!(
            verdict,
            Ok(()),
            "the export checks the token per frame, not through the renderer"
        ),
        Err(error) => panic!("the export failed: {error}"),
    }
}

// FV-22 (b): an export reads the original medium, never the light copy.
#[test]
fn fv_12_an_export_reads_the_original_and_never_the_copy() {
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe.clone());
    let video = LogCodec::video(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 1_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let probe = source.probe();
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov").with_copy("copy.mjpeg"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let mut sink = MemSink::new(SinkProbe::new());
    let request = request(values(0, 1_000, "25"));

    run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::new(),
        &CancelToken::new(),
    )
    .expect("the export completes");

    assert_eq!(
        probe.opened(),
        vec![PathBuf::from("original.mov")],
        "the export reads the original and never the light copy"
    );
}

// FV-12, design ME5: the composition goes through the page renderer port and
// is proved against the testkit double.
#[test]
fn fv_12_the_composition_goes_through_the_page_renderer() {
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe.clone());
    let video = LogCodec::video(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 1_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let mut sink = MemSink::new(SinkProbe::new());
    let request = request(values(0, 1_000, "25"));

    run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::new(),
        &CancelToken::new(),
    )
    .expect("the export completes");

    assert_eq!(
        renderer.count(),
        25,
        "every frame of the range is composed through PageRenderer"
    );
    let last = renderer.last().expect("a frame was composed");
    assert_eq!(
        last.page,
        page(),
        "the renderer composes the page of the scope"
    );
    assert_eq!(
        last.region,
        IRect::new(0, 0, 64, 36),
        "FV-24: the resolution option reaches the renderer"
    );
    assert_eq!(
        journal.count(Step::Compose),
        25,
        "the composition is a stage of the pipeline"
    );
}

// ARQ-43, FV-12: a lost or unavailable encoder fails with the error the port
// fixes and leaves no partial file.
#[test]
fn arq_43_a_lost_or_unavailable_encoder_fails_without_a_file() {
    // (a) the encoder cannot be created.
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe.clone());
    let video = LogCodec::unavailable(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 1_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let sink_probe = SinkProbe::new();
    let mut sink = MemSink::new(sink_probe.clone());
    let request = request(values(0, 1_000, "25"));
    let unavailable = run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::new(),
        &CancelToken::new(),
    );
    assert!(
        matches!(
            unavailable,
            Err(ExportError::Media(MediaError::SystemCodecUnavailable))
        ),
        "an unavailable encoder fails with the error of the port: {unavailable:?}"
    );
    assert!(
        sink_probe.names().is_empty(),
        "an unavailable encoder leaves no file"
    );

    // (b) the encoder is lost in the middle of the export.
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe);
    let video = LogCodec::lost(journal.clone(), 3);
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 1_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let sink_probe = SinkProbe::new();
    let mut sink = MemSink::new(sink_probe.clone());
    let lost = run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::new(),
        &CancelToken::new(),
    );
    assert!(
        matches!(lost, Err(ExportError::Media(MediaError::Corrupt { .. }))),
        "a lost encoder fails with the error of the port: {lost:?}"
    );
    assert!(
        sink_probe.names().is_empty(),
        "a lost encoder leaves no partial file"
    );
}

// FV-24: the codec option is checked before the export starts.
#[test]
fn fv_24_an_unavailable_codec_is_rejected_before_the_export_starts() {
    let journal = Journal::new();
    let probe = Probe::new();
    let renderer = CountingRenderer::new(journal.clone());
    let container = RecordingContainer::new(journal.clone(), probe);
    let video = LogCodec::video(journal.clone());
    let audio = LogCodec::audio(journal.clone());
    let source = RecordingSourceFactory::new(
        journal.clone(),
        SourcePlan {
            video: timeline(0, 1_000, FrameRate::FPS_25),
            audio: Vec::new(),
        },
    );
    let exporter = VideoExporter::new(
        ExportMedium::new("original.mov"),
        Box::new(source),
        &container,
        &video,
        &audio,
        "reel.mp4",
    );
    let mut options = values(0, 1_000, "25");
    options.set(OPTION_CODEC, ParamValue::Choice("av1".into()));
    let request = request(options);

    let issues = exporter.validate(&request);
    assert!(
        issues.iter().any(|issue| issue.blocks()),
        "FV-24: a codec the adapter cannot write blocks the export"
    );
    let sink_probe = SinkProbe::new();
    let mut sink = MemSink::new(sink_probe.clone());
    let result = run(
        &exporter,
        &request,
        &renderer,
        &mut sink,
        &ProgressProbe::new(),
        &CancelToken::new(),
    );
    assert!(
        matches!(
            result,
            Err(ExportError::Media(MediaError::UnsupportedCodec {
                codec: CodecId::Av1
            }))
        ),
        "the export refuses the codec before writing: {result:?}"
    );
    assert!(
        sink_probe.names().is_empty(),
        "a rejected codec leaves no partial file"
    );
}
