//! The wire form of the timeline of TL3's slice card: the ids as plain
//! numbers and the enums tagged (AC1, ARQ-10), the root that writes
//! `sequences` always and reads it as empty from older bytes (AC2, ARQ-13), the
//! asset table keyed by the lowercase hex hash (AC5, P9) and the canonical
//! bytes of a document with a clip of each source kind (AC7, ARQ-53).
//!
//! The tests name the requirement they pin in their `fn` ident, so
//! `git grep arq_53_` finds them.

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_geom::{Length, Unit};

use gafira_doc::timeline::{
    AssetEntry, AssetRef, AssetStorage, Clip, ClipSource, Marker, PixelSize, Sequence, Speed,
    TrackKind,
};
use gafira_doc::{
    ClipId, ContentHash, Document, Dpi, EditToken, ElementBase, GroupNode, MarkerId, Node, NodeId,
    NodeKind, Page, PageId, PageSize, Placement, SequenceId, TrackId,
};
use gafira_time::{Flicks, FrameRate, SampleRate, TimeRange};

/// One frame at the sequence's 24 fps, in flicks: exact, never rounded (V-2).
const FRAME: i64 = 29_400_000;

/// The hash of the media clip's content: 32 bytes of `0xAB`, which the wire
/// form spells as sixty-four lowercase `ab` pairs.
const MEDIA_HASH: [u8; 32] = [0xAB; 32];
/// The hash of the audio clip's content: sixty-four lowercase `2`s.
const AUDIO_HASH: [u8; 32] = [0x22; 32];

/// The hash of [`MEDIA_HASH`] in its wire spelling (§7.1).
const MEDIA_HEX: &str = "abababababababababababababababababababababababababababababababab";
/// The hash of [`AUDIO_HASH`] in its wire spelling (§7.1).
const AUDIO_HEX: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// The page the `Scene` clip shows: one group node, `2`, as its only root.
fn page() -> Arc<Page> {
    let size = PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), "Cover".to_string(), size);
    let base = ElementBase::new(NodeId::new(2), Placement::new(0.0, 0.0, 1.0, 1.0));
    let node = Node::new(base, NodeKind::Group(GroupNode {}));
    page.roots.push(NodeId::new(2));
    page.nodes.insert(NodeId::new(2), Arc::new(node));
    Arc::new(page)
}

/// A clip of `source` from frame `start` for `frames`, at `speed`.
fn clip(id: u64, source: ClipSource, start: i64, frames: i64, speed: Speed, volume: f32) -> Clip {
    let place = TimeRange::new(
        Flicks::from_ticks(start * FRAME),
        Flicks::from_ticks(frames * FRAME),
    )
    .expect("a clip of whole frames and a positive span");
    Clip::new(ClipId::new(id), source, place, Flicks::ZERO, speed, volume)
}

/// The document of the timeline golden: one page, one sequence whose video
/// lane carries a clip of each source kind, the four track kinds, a marker and
/// two assets — one embedded, one linked.
fn timeline_document() -> Document {
    let mut doc = Document::from_parts(13, Dpi::default(), vec![page()])
        .expect("the page and the counter are valid");
    let token = EditToken::__new_for_gafira_edit_only();
    let mut view = doc.edit(&token);

    let mut sequence = Sequence::new(
        SequenceId::new(3),
        FrameRate::FPS_24,
        PixelSize::new(1920, 1080),
        SampleRate::new(48_000).expect("48 kHz is a whole-flick sample rate"),
    )
    .expect("the size has no zero side");

    for (id, kind) in [
        (4, TrackKind::Video),
        (8, TrackKind::Overlay),
        (9, TrackKind::Audio),
        (11, TrackKind::Caption),
    ] {
        sequence
            .add_track(TrackId::new(id), kind)
            .expect("the audio cap is not reached");
    }

    let frame = sequence.frame();
    let video = sequence
        .track_mut_by_id(TrackId::new(4))
        .expect("the video lane was added");
    for (id, source, start, frames, speed, volume) in [
        (5, ClipSource::Scene(PageId::new(1)), 0, 1, Speed::ONE, 1.0),
        (
            6,
            ClipSource::Media(AssetRef::new(ContentHash(MEDIA_HASH))),
            1,
            2,
            Speed::new(2, 1).expect("2/1 divides"),
            0.5,
        ),
        (
            7,
            ClipSource::Nodes(vec![NodeId::new(2)]),
            3,
            1,
            Speed::ONE,
            1.0,
        ),
    ] {
        video
            .insert(clip(id, source, start, frames, speed, volume), frame)
            .expect("the clips are on the grid and do not overlap");
    }
    let audio = sequence
        .track_mut_by_id(TrackId::new(9))
        .expect("the audio lane was added");
    audio
        .insert(
            clip(
                10,
                ClipSource::Audio(AssetRef::new(ContentHash(AUDIO_HASH))),
                0,
                2,
                Speed::ONE,
                0.75,
            ),
            frame,
        )
        .expect("the clip is on the grid");

    sequence.add_marker(Marker::new(
        MarkerId::new(12),
        Flicks::from_ticks(FRAME / 2),
        "Beat".to_string(),
        Color::Srgb(Rgba::new(1.0, 0.5, 0.25, 1.0)),
    ));

    view.sequences_mut().push(Arc::new(sequence));
    view.assets_mut().insert(
        AssetRef::new(ContentHash(MEDIA_HASH)),
        AssetEntry {
            name: "clip.mp4".to_string(),
            storage: AssetStorage::Embedded,
        },
    );
    view.assets_mut().insert(
        AssetRef::new(ContentHash(AUDIO_HASH)),
        AssetEntry {
            name: "score.wav".to_string(),
            storage: AssetStorage::Linked,
        },
    );
    doc
}

fn resave(text: &str) -> String {
    Document::from_json(text.as_bytes())
        .expect("the bytes are a valid document")
        .to_json()
        .expect("a document writes as JSON")
}

// ARQ-10 (AC1): the ids of the sequence, the track, the clip and the marker
// are plain numbers in the wire form, the enums carry their tag, and every
// field of the model is written (ADR 0006 rule 3).
#[test]
fn arq_10_the_timeline_ids_are_plain_numbers_and_every_field_is_written() {
    let json = timeline_document().to_json().unwrap();
    for id in [
        r#""sequences":[{"id":3,"#,
        r#"{"id":4,"kind":"video""#,
        r#"{"id":5,"source":{"scene":1}"#,
        r#"{"id":12,"at":14700000,"name":"Beat""#,
    ] {
        assert!(json.contains(id), "{id} is not in {json}");
    }
    // Every field of a track, a clip and a marker is written, with its
    // canonical value, even when it is a default.
    for field in [
        r#""kind":"video","ripple":true,"muted":false,"locked":false,"solo":false"#,
        r#""place":{"start":0,"duration":29400000},"source_in":0,"speed":{"num":1,"den":1},"volume":1.0"#,
        r#""kind":"overlay","ripple":false"#,
        r#""markers":[{"id":12,"at":14700000,"name":"Beat","color":{"space":"srgb","r":1.0,"g":0.5,"b":0.25,"a":1.0}}]"#,
    ] {
        assert!(json.contains(field), "{field} is not in {json}");
    }
    // The four source kinds carry their tag, and the sequence carries its
    // exact rates and its size.
    for source in [
        r#""source":{"media":"abab"#,
        r#""source":{"nodes":[2]}"#,
        r#""source":{"audio":"2222"#,
    ] {
        assert!(json.contains(source), "{source} is not in {json}");
    }
    assert!(json.contains(r#""fps":{"num":24,"den":1}"#), "{json}");
    assert!(json.contains(r#""sample_rate":{"hz":48000}"#), "{json}");
    assert!(
        json.contains(r#""size":{"width":1920,"height":1080}"#),
        "{json}"
    );
}

// ARQ-13 (AC2): the root writes `sequences` always, a document that predates
// the field reads it as empty, and its error path is unchanged (ADR 0006
// rule 1).
#[test]
fn arq_13_a_document_that_predates_sequences_reads_them_as_empty() {
    let old = include_str!("golden/pre_layout/minimal.json");
    assert!(!old.contains("sequences"), "the fixture predates the field");
    let doc = Document::from_json(old.as_bytes()).expect("old bytes still load");
    assert!(doc.sequences().is_empty(), "the field reads as empty");
    assert!(doc.assets().is_empty(), "the table reads as empty");
    let json = doc.to_json().unwrap();
    assert!(
        json.contains(r#""sequences":[],"assets":{}"#),
        "the saved bytes write both fields: {json}"
    );
    // The error path of `Document::from_json` is unchanged: the schema version
    // still decides first, and malformed bytes are still JSON errors.
    let future = old.replace(r#""schema_version":1"#, r#""schema_version":2"#);
    assert!(matches!(
        Document::from_json(future.as_bytes()),
        Err(gafira_doc::LoadError::Invalid(
            gafira_doc::ValidationError::UnsupportedSchemaVersion { found: 2 }
        ))
    ));
    assert!(matches!(
        Document::from_json(br#"{"schema_version":1,"next_id":3,"pages":[],"sequences":[{"#),
        Err(gafira_doc::LoadError::Json(_))
    ));
}

// ARQ-10, ARQ-13 (AC1, AC2): the timeline survives the document's round trip
// byte for byte.
#[test]
fn arq_10_the_timeline_round_trips_through_the_document() {
    let doc = timeline_document();
    let json = doc.to_json().unwrap();
    let back = Document::from_json(json.as_bytes()).expect("the timeline loads");
    assert_eq!(back, doc);
    assert_eq!(back.to_json().unwrap(), json);
    assert_eq!(back.sequences().len(), 1);
    assert_eq!(back.sequences()[0].as_ref(), doc.sequences()[0].as_ref());
    assert_eq!(back.assets().len(), 2);
}

// P9 (AC5): the asset table is keyed by `AssetRef`, the sixty-four character
// lowercase hex hash of the content, and each entry carries its name, its hash
// and its storage.
#[test]
fn p9_the_asset_table_is_keyed_by_the_lowercase_hex_hash() {
    let json = timeline_document().to_json().unwrap();
    assert!(
        json.contains(&format!(
            r#""assets":{{"{AUDIO_HEX}":{{"name":"score.wav","storage":"linked"}},"{MEDIA_HEX}":{{"name":"clip.mp4","storage":"embedded"}}}}"#
        )),
        "the table is keyed by the hex hash: {json}"
    );
    // A key in another spelling is the same hash and is written back in the
    // canonical one.
    let upper = json.replace(MEDIA_HEX, &MEDIA_HEX.to_uppercase());
    assert_ne!(upper, json);
    assert_eq!(resave(&upper), json);
    // The key is the hash itself: a short key names no content and the bytes
    // are a JSON error, not a document that loads.
    for bad in ["1111", &MEDIA_HEX[..63], &format!("{MEDIA_HEX}0")] {
        let text = json.replace(MEDIA_HEX, bad);
        assert!(
            matches!(
                Document::from_json(text.as_bytes()),
                Err(gafira_doc::LoadError::Json(_))
            ),
            "{bad}"
        );
    }
    // The entry the table holds is keyed by the reference the clip names.
    let doc = Document::from_json(json.as_bytes()).unwrap();
    let reference = AssetRef::new(ContentHash(MEDIA_HASH));
    assert_eq!(
        doc.asset(&reference).map(|e| e.name.as_str()),
        Some("clip.mp4")
    );
    assert_eq!(
        doc.asset(&reference).map(|e| e.storage),
        Some(AssetStorage::Embedded)
    );
    assert_eq!(reference.hex(), MEDIA_HEX);
}

// ARQ-53 (AC7): the golden of the timeline is the canonical bytes of the
// document that holds a clip of each source kind, and it re-saves byte for
// byte through the validating path.
#[test]
fn arq_53_the_timeline_golden_is_the_canonical_bytes() {
    let golden = include_str!("golden/timeline.json");
    assert_eq!(timeline_document().to_json().unwrap(), golden);
    assert_eq!(resave(golden), golden);
    assert_eq!(
        Document::from_json(golden.as_bytes()).unwrap(),
        timeline_document()
    );
    assert!(!golden.contains(['\n', '\r']), "the golden is one line");
}

// ADR 0006 rules 1 and 3 (AC2, AC7): the seven byte sets of the document —
// the three canonical goldens and the four old input fixtures — still hold.
// The old bytes load with an empty timeline and re-save in the canonical form,
// so a document written before this delivery still opens; the canonical bytes
// round-trip byte for byte in `tests/golden.rs`.
#[test]
fn adr_0006_the_old_bytes_read_the_timeline_as_empty() {
    for (name, old) in [
        (
            "extension_noncanonical",
            include_str!("golden/extension_noncanonical.json"),
        ),
        (
            "pre_layout/minimal",
            include_str!("golden/pre_layout/minimal.json"),
        ),
        (
            "pre_layout/full",
            include_str!("golden/pre_layout/full.json"),
        ),
        (
            "pre_layout/extension",
            include_str!("golden/pre_layout/extension.json"),
        ),
    ] {
        assert!(!old.contains("sequences"), "{name} predates the field");
        let doc = Document::from_json(old.as_bytes()).expect("{name} still loads");
        assert!(
            doc.sequences().is_empty(),
            "{name} reads the timeline as empty"
        );
        assert!(doc.assets().is_empty(), "{name} reads the table as empty");
        let saved = doc.to_json().unwrap();
        assert!(
            saved.contains(r#","sequences":[],"assets":{}}"#),
            "{name} writes both fields: {saved}"
        );
        assert_eq!(resave(&saved), saved, "{name} saves canonically");
    }
    // Two of the old byte sets land on a canonical golden with no layout
    // question in between, so the whole document is pinned, not just the two
    // fields (the third, `pre_layout/full.json`, differs in page 1's layout
    // and is pinned key by key in `tests/layout.rs`).
    assert_eq!(
        resave(include_str!("golden/extension_noncanonical.json")),
        include_str!("golden/extension.json")
    );
    assert_eq!(
        resave(include_str!("golden/pre_layout/minimal.json")),
        include_str!("golden/minimal.json")
    );
    assert_eq!(
        resave(include_str!("golden/pre_layout/extension.json")),
        include_str!("golden/extension.json")
    );
}
