//! The validation the timeline joins with: the new id spaces (AC3, ARQ-10),
//! the sources a clip names (AC4, P9) and the laws of the model applied to a
//! document that is loaded (AC6, ARQ-13). A document that breaks one fails to
//! load, so it never reaches the editor.
//!
//! Each case breaks exactly one rule and the test pins the error that comes
//! back, so a neighbour that fires instead fails the test.

use gafira_doc::timeline::TimelineError;
use gafira_doc::{ClipId, Document, LoadError, PageId, SequenceId, TrackId, ValidationError};

/// One frame at 24 fps, in flicks: exact, never rounded (V-2).
const FRAME: i64 = 29_400_000;

/// The page the `Scene` clips of the battery show.
const PAGE: &str =
    r#"{"id":1,"size":{"width":{"value":1,"unit":"px"},"height":{"value":1,"unit":"px"}}}"#;

/// A document root with the given sequences text and counter.
fn document(next_id: u64, sequences: &str) -> String {
    format!(
        r#"{{"schema_version":1,"next_id":{next_id},"pages":[{PAGE}],"sequences":[{sequences}],"assets":{{}}}}"#
    )
}

/// One sequence with the given tracks and markers text.
fn sequence(id: u64, tracks: &str, markers: &str) -> String {
    format!(
        r#"{{"id":{id},"fps":{{"num":24,"den":1}},"size":{{"width":1920,"height":1080}},"sample_rate":{{"hz":48000}},"tracks":[{tracks}],"markers":[{markers}]}}"#
    )
}

/// One track with the given clips text.
fn track(id: u64, kind: &str, clips: &str) -> String {
    format!(
        r#"{{"id":{id},"kind":"{kind}","ripple":false,"muted":false,"locked":false,"solo":false,"clips":[{clips}]}}"#
    )
}

/// One clip: `source` as wire text, a half-open place at `start` for
/// `duration` ticks, `source_in` and the speed `num/1`.
fn clip(id: u64, source: &str, start: i64, duration: i64, source_in: i64, speed: i64) -> String {
    format!(
        r#"{{"id":{id},"source":{source},"place":{{"start":{start},"duration":{duration}}},"source_in":{source_in},"speed":{{"num":{speed},"den":1}},"volume":1.0}}"#
    )
}

/// A one-frame clip of the page at `start_frames`, at the identity speed.
fn frame_clip(id: u64, start_frames: i64) -> String {
    clip(id, r#"{"scene":1}"#, start_frames * FRAME, FRAME, 0, 1)
}

/// A sequence whose one video track holds the given clips.
fn with_clips(clips: &str) -> String {
    document(100, &sequence(2, &track(3, "video", clips), ""))
}

fn load(text: &str) -> Result<Document, LoadError> {
    Document::from_json(text.as_bytes())
}

/// The document rule the load refuses with; a document that loads is a bug.
fn refused(text: &str, label: &str) -> ValidationError {
    match load(text) {
        Err(LoadError::Invalid(error)) => error,
        Err(other) => panic!("{label}: expected a document rule, got {other}"),
        Ok(_) => panic!("{label}: the document loaded and it must not"),
    }
}

/// The law of the timeline the load refuses with: the shape a break of FV-01,
/// FV-03 or P11 takes.
fn law(text: &str, label: &str) -> TimelineError {
    match refused(text, label) {
        ValidationError::TimelineLaw { error, .. } => error,
        other => panic!("{label}: expected a law of the timeline, got {other}"),
    }
}

// ARQ-10 (AC3): the ids of the sequences, the tracks, the clips and the
// markers are unique in the whole document and below `next_id` (P2 extended).
#[test]
fn arq_10_the_timeline_ids_are_unique_in_the_whole_document() {
    // One id shared by two sequences.
    let sequences = format!(
        "{},{}",
        sequence(2, "", ""),
        sequence(2, &track(3, "video", ""), "")
    );
    assert_eq!(
        refused(&document(100, &sequences), "a sequence id used twice"),
        ValidationError::DuplicateId { id: 2 }
    );
    // One id shared by two tracks.
    let tracks = format!("{},{}", track(3, "video", ""), track(3, "overlay", ""));
    assert_eq!(
        refused(
            &document(100, &sequence(2, &tracks, "")),
            "a track id used twice"
        ),
        ValidationError::DuplicateId { id: 3 }
    );
    // One id shared by two clips.
    let clips = format!("{},{}", frame_clip(4, 0), frame_clip(4, 1));
    assert_eq!(
        refused(&with_clips(&clips), "a clip id used twice"),
        ValidationError::DuplicateId { id: 4 }
    );
    // One id shared by two markers.
    let marker = |at: i64| {
        format!(
            r#"{{"id":5,"at":{at},"name":"m","color":{{"space":"srgb","r":1.0,"g":1.0,"b":1.0,"a":1.0}}}}"#
        )
    };
    let markers = format!("{},{}", marker(0), marker(FRAME));
    assert_eq!(
        refused(
            &document(100, &sequence(2, "", &markers)),
            "a marker id used twice"
        ),
        ValidationError::DuplicateId { id: 5 }
    );
    // The counter is the document's, so a timeline id that shares a raw value
    // with a page is one id used twice.
    assert_eq!(
        refused(
            &with_clips(&frame_clip(1, 0)),
            "a clip id equal to the page"
        ),
        ValidationError::DuplicateId { id: 1 }
    );
}

// ARQ-10 (AC3): every timeline id stays below the document's counter, which
// the pages already obey.
#[test]
fn arq_10_a_timeline_id_at_the_counter_is_refused() {
    let clips = frame_clip(4, 0);
    let at_the_counter = document(4, &sequence(2, &track(3, "video", &clips), ""));
    assert_eq!(
        refused(&at_the_counter, "a clip id at the counter"),
        ValidationError::NextIdNotAboveMax {
            next_id: 4,
            max_id: 4
        }
    );
    assert!(load(&document(100, &sequence(2, &track(3, "video", &clips), ""))).is_ok());
}

// P9 (AC4): a `Scene` clip names an existing page, and a `Media` or `Audio`
// clip names an entry of the asset table.
#[test]
fn p9_a_clip_names_a_page_and_an_asset_the_document_holds() {
    let gone = clip(4, r#"{"scene":9}"#, 0, FRAME, 0, 1);
    assert_eq!(
        refused(&with_clips(&gone), "a scene clip with no page"),
        ValidationError::DanglingScene {
            clip: ClipId::new(4),
            page: PageId::new(9)
        }
    );

    let hash = "a".repeat(64);
    let media = format!(r#"{{"media":"{hash}"}}"#);
    assert_eq!(
        refused(
            &with_clips(&clip(4, &media, 0, FRAME, 0, 1)),
            "a media clip with no asset"
        ),
        ValidationError::DanglingAsset {
            clip: ClipId::new(4)
        }
    );
    let audio = format!(r#"{{"audio":"{hash}"}}"#);
    assert_eq!(
        refused(
            &document(
                100,
                &sequence(2, &track(3, "audio", &clip(4, &audio, 0, FRAME, 0, 1)), "")
            ),
            "an audio clip with no asset"
        ),
        ValidationError::DanglingAsset {
            clip: ClipId::new(4)
        }
    );

    // The same clip loads once the table holds the entry, and a scene clip
    // that names the page loads.
    let with_asset = format!(
        r#"{{"schema_version":1,"next_id":100,"pages":[{PAGE}],"sequences":[{}],"assets":{{"{hash}":{{"name":"clip.mp4","storage":"embedded"}}}}}}"#,
        sequence(2, &track(3, "video", &clip(4, &media, 0, FRAME, 0, 1)), "")
    );
    assert!(load(&with_asset).is_ok(), "{with_asset}");
    assert!(load(&with_clips(&frame_clip(4, 0))).is_ok());
}

// ARQ-13 (AC6): the laws of the model hold on the document that is loaded,
// and a document that breaks one fails to load instead of reaching the editor.
#[test]
fn arq_13_a_document_that_breaks_a_law_does_not_load() {
    // The cut on the grid.
    let off_grid = clip(4, r#"{"scene":1}"#, 1, FRAME, 0, 1);
    assert_eq!(
        law(&with_clips(&off_grid), "a cut between two frames"),
        TimelineError::OffGrid {
            clip: ClipId::new(4),
            at: 1
        }
    );

    // The span: whole frames, at least one.
    let half_frame = clip(4, r#"{"scene":1}"#, 0, FRAME / 2, 0, 1);
    assert_eq!(
        law(&with_clips(&half_frame), "a span below one frame"),
        TimelineError::SpanTooShort { span: FRAME / 2 }
    );
    let empty = clip(4, r#"{"scene":1}"#, 0, 0, 0, 1);
    assert_eq!(
        law(&with_clips(&empty), "an empty clip"),
        TimelineError::SpanTooShort { span: 0 }
    );

    // The clip inside its sequence and inside its source.
    let before_zero = clip(4, r#"{"scene":1}"#, -FRAME, FRAME, 0, 1);
    assert_eq!(
        law(
            &with_clips(&before_zero),
            "a clip before the sequence starts"
        ),
        TimelineError::BeforeSequence {
            clip: ClipId::new(4),
            at: -FRAME
        }
    );
    let before_source = clip(4, r#"{"scene":1}"#, 0, FRAME, -1, 1);
    assert_eq!(
        law(&with_clips(&before_source), "a clip before its source"),
        TimelineError::SourceBeforeStart {
            clip: ClipId::new(4),
            source_in: -1
        }
    );

    // The lane: one strip at an instant, sorted by place.
    let meeting = format!("{},{}", frame_clip(4, 0), frame_clip(5, 0));
    assert_eq!(
        law(&with_clips(&meeting), "two clips at one instant"),
        TimelineError::Overlap {
            track: TrackId::new(3),
            at: 0
        }
    );
    let unsorted = format!("{},{}", frame_clip(4, 2), frame_clip(5, 0));
    assert_eq!(
        law(&with_clips(&unsorted), "a lane out of place order"),
        TimelineError::Overlap {
            track: TrackId::new(3),
            at: 0
        }
    );

    // The speed: a zero numerator or denominator divides nothing.
    let zero_speed = clip(4, r#"{"scene":1}"#, 0, FRAME, 0, 0);
    assert_eq!(
        law(&with_clips(&zero_speed), "a zero speed"),
        TimelineError::InvalidSpeed { num: 0, den: 1 }
    );
    let zero_denominator = zero_speed.replace(r#""num":0,"den":1"#, r#""num":1,"den":0"#);
    assert_eq!(
        law(&with_clips(&zero_denominator), "a zero denominator"),
        TimelineError::InvalidSpeed { num: 1, den: 0 }
    );

    // The audio cap: sixteen, and the seventeenth is refused.
    let lanes = |count: u64| -> String {
        let tracks: Vec<String> = (0..count).map(|i| track(4 + i, "audio", "")).collect();
        document(100, &sequence(2, &tracks.join(","), ""))
    };
    assert!(load(&lanes(16)).is_ok(), "the sixteenth audio track enters");
    assert_eq!(
        law(&lanes(17), "the seventeenth audio track"),
        TimelineError::TooManyAudioTracks {
            sequence: SequenceId::new(2)
        }
    );

    // The document the delivery pins as canonical satisfies every law.
    assert!(load(include_str!("golden/timeline.json")).is_ok());
}

// ARQ-13 (AC6): the laws are applied to every sequence of the document, not
// only to the first one.
#[test]
fn arq_13_every_sequence_of_the_document_is_checked() {
    let good = sequence(2, &track(3, "video", &frame_clip(4, 0)), "");
    let bad = sequence(
        5,
        &track(6, "video", &clip(7, r#"{"scene":1}"#, 1, FRAME, 0, 1)),
        "",
    );
    assert_eq!(
        refused(
            &document(100, &format!("{good},{bad}")),
            "the second sequence is checked"
        ),
        ValidationError::TimelineLaw {
            sequence: SequenceId::new(5),
            error: TimelineError::OffGrid {
                clip: ClipId::new(7),
                at: 1
            }
        }
    );
}

// ARQ-13 (AC4, AC6): the sources are checked over every sequence and track,
// not only the first one.
#[test]
fn arq_13_every_clip_of_the_document_names_its_source() {
    let good = sequence(2, &track(3, "video", &frame_clip(4, 0)), "");
    let bad = sequence(
        5,
        &track(6, "overlay", &clip(7, r#"{"scene":9}"#, 0, FRAME, 0, 1)),
        "",
    );
    assert_eq!(
        refused(
            &document(100, &format!("{good},{bad}")),
            "the second sequence names a page that is gone"
        ),
        ValidationError::DanglingScene {
            clip: ClipId::new(7),
            page: PageId::new(9)
        }
    );
}
