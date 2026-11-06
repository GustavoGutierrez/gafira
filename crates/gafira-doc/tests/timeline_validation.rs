//! The validation the timeline joins with: the three new id spaces are unique
//! in the whole document and below its counter (AC3, ARQ-10). The sources a
//! clip names and the laws of the model arrive with the next commit.
//!
//! Each case breaks exactly one rule and the test pins the error that comes
//! back, so a neighbour that fires instead fails the test.

use gafira_doc::{Document, LoadError, ValidationError};

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
