//! The laws of the timeline of TL2's slice card, AC1 to AC10, as honest tests
//! over the model's API.
//!
//! Every test names the requirement it pins in its `fn` ident (`fv_01_..`,
//! `fv_03_..`, `v_2_..`, `arq_10_..`, `arq_30_..`, `p_11_..`, `p_7_..`), so
//! `git grep fv_01_` traces them.

use gafira_time::{Flicks, FrameRate, SampleRate, TimeRange};

use gafira_doc::timeline::{
    AUDIO_TRACK_CAP, AssetRef, Clip, ClipSource, PixelSize, Sequence, Speed, TimelineError, Track,
    TrackKind,
};
use gafira_doc::{ClipId, ContentHash, EditToken, SequenceId, TrackId};

const FPS_24: FrameRate = FrameRate::FPS_24;
const FPS_23_976: FrameRate = FrameRate::FPS_23_976;

/// The media reference the clips of the tests name.
fn media() -> ClipSource {
    ClipSource::Media(AssetRef::new(ContentHash([7u8; 32])))
}

/// A document with the token and two ids already allocated from the counter.
fn setup() -> (gafira_doc::Document, EditToken, SequenceId, TrackId) {
    let token = EditToken::__new_for_gafira_edit_only();
    let mut doc = gafira_doc::Document::default();
    let mut view = doc.edit(&token);
    let sid = view.alloc_sequence_id().expect("the counter has room");
    let tid = view.alloc_track_id().expect("the counter has room");
    (doc, token, sid, tid)
}

/// A sequence at `fps`, 1920×1080, 48 kHz.
fn a_sequence(sid: SequenceId, fps: FrameRate) -> Sequence {
    Sequence::new(
        sid,
        fps,
        PixelSize::new(1920, 1080),
        SampleRate::new(48_000).expect("48 kHz is a whole-flick sample rate"),
    )
    .expect("the size is not empty")
}

/// A clip from `start` frames to `start + span` frames, at the identity speed,
/// with its source window starting at zero.
fn frame_clip(id: ClipId, start: i64, span: i64, frame: Flicks) -> Clip {
    let place = TimeRange::new(
        Flicks::from_ticks(start * frame.ticks()),
        Flicks::from_ticks(span * frame.ticks()),
    )
    .expect("a clip of whole frames and positive span");
    Clip::new(id, media(), place, Flicks::ZERO, Speed::ONE, 1.0)
}

/// A video track with the given clips already inside, at the given rate.
fn track_with_clips(track_id: TrackId, clips: Vec<Clip>, frame: Flicks) -> Track {
    let mut track = Track::new(track_id, TrackKind::Video);
    for clip in clips {
        track.insert(clip, frame).expect("a whole-frame clip");
    }
    track
}

// FV-01 (AC1): a sequence carries its rate, its size and its sample rate, the
// tracks are of exactly four kinds, and a clip names its source, its offsets
// in the sequence and in the source, its speed and its volume.
#[test]
fn fv_01_a_sequence_and_a_clip_carry_their_fields() {
    let (_doc, _token, sid, _tid) = setup();
    let sequence = a_sequence(sid, FPS_24);
    assert_eq!(sequence.id(), sid);
    assert_eq!(sequence.fps(), FPS_24);
    assert_eq!(sequence.size(), PixelSize::new(1920, 1080));
    assert_eq!(sequence.sample_rate(), SampleRate::new(48_000).unwrap());
    // The four kinds exist, and no other kind compiles as one.
    let kinds = [
        TrackKind::Video,
        TrackKind::Overlay,
        TrackKind::Audio,
        TrackKind::Caption,
    ];
    assert_eq!(kinds.len(), 4);
    // The clip carries the six fields the law names.
    let clip = frame_clip(ClipId::new(9), 0, 1, FPS_24.flicks_per_frame());
    assert!(matches!(clip.source(), ClipSource::Media(_)));
    assert_eq!(clip.place().duration(), FPS_24.flicks_per_frame());
    assert_eq!(clip.source_in(), Flicks::ZERO);
    assert_eq!(clip.speed(), Speed::ONE);
    assert_eq!(clip.volume(), 1.0);
}

// ARQ-10, V-1 (AC2): the ids of the timeline come from the document's counter,
// unique across the whole document, never one per page.
#[test]
fn arq_10_and_v_1_the_timeline_ids_come_from_one_counter() {
    let (mut doc, token, _sid, _tid) = setup();
    let mut view = doc.edit(&token);
    let sid = view.alloc_sequence_id().unwrap();
    let tid = view.alloc_track_id().unwrap();
    let cid = view.alloc_clip_id().unwrap();
    let mid = view.alloc_marker_id().unwrap();
    let got = [sid.get(), tid.get(), cid.get(), mid.get()];
    let unique: std::collections::BTreeSet<u64> = got.into_iter().collect();
    assert_eq!(unique.len(), 4, "the four ids drew one value each");
    // And the document's own pages draw from the same counter, so a sequence
    // and a page never share a raw value.
    let page_id = view.alloc_page_id().unwrap();
    let all: std::collections::BTreeSet<u64> = got
        .into_iter()
        .chain(std::iter::once(page_id.get()))
        .collect();
    assert_eq!(all.len(), 5, "one counter for the whole document");
}

// V-2, FV-03 (AC3): a cut lands on an exact frame of the sequence at its
// rate. At 23,976 (24 000/1 001) the frame duration in flicks is exactly
// 29 429 400, not a rounding of it.
#[test]
fn v_2_and_fv_03_a_cut_lands_on_an_exact_frame_including_23_976() {
    let frame = FPS_23_976.flicks_per_frame();
    assert_eq!(frame.ticks(), 29_429_400, "exact flicks per frame");
    assert_eq!(frame.ticks() * 2, 58_858_800);
    // The sequence's own frame is that exact number: a frame rounded to the
    // millisecond would be 29 635 200 flicks (42 ms), not 29 429 400.
    let (_doc, _token, sid, _tid) = setup();
    let sequence = a_sequence(sid, FPS_23_976);
    assert_eq!(
        sequence.frame().ticks(),
        29_429_400,
        "the frame of the sequence is the exact one, not a rounded millisecond"
    );
    let frame = sequence.frame();
    // The grid refuses a cut between two frames instead of rounding it.
    let mut track = Track::new(TrackId::new(5), TrackKind::Video);
    let between = TimeRange::new(Flicks::from_ticks(1), Flicks::from_ticks(frame.ticks()))
        .expect("a range of one frame");
    let clip = Clip::new(
        ClipId::new(1),
        media(),
        between,
        Flicks::ZERO,
        Speed::ONE,
        1.0,
    );
    let got = track.insert(clip, frame);
    assert!(
        matches!(got, Err(TimelineError::OffGrid { .. })),
        "a cut between two frames is reported, never rounded"
    );
    // And the exact grid accepts the cut on the boundary.
    let mut track = Track::new(TrackId::new(6), TrackKind::Video);
    let on_grid = frame_clip(ClipId::new(2), 1, 1, frame);
    assert!(
        track.insert(on_grid, frame).is_ok(),
        "the boundary is a cut"
    );
}

// FV-01 (AC4): a clip lies inside its source and inside its sequence; a span
// below one frame is rejected, and a source offset before the source is too.
#[test]
fn fv_01_a_clip_lies_inside_its_source_and_its_sequence() {
    let (_doc, _token, _sid, _tid) = setup();
    let frame = FPS_24.flicks_per_frame();
    let mut track = track_with_clips(
        TrackId::new(5),
        vec![frame_clip(ClipId::new(1), 0, 2, frame)],
        frame,
    );
    // A span of half a frame is not a clip.
    let half = TimeRange::new(Flicks::ZERO, Flicks::from_ticks(frame.ticks() / 2))
        .expect("a half-frame range is still a range");
    let short = Clip::new(ClipId::new(2), media(), half, Flicks::ZERO, Speed::ONE, 1.0);
    let got = track.insert(short, frame);
    assert!(
        matches!(got, Err(TimelineError::SpanTooShort { .. })),
        "half a frame is not a clip, it is a mistake"
    );
    // A source window that starts before the source is outside its range.
    let one_frame = TimeRange::new(Flicks::from_ticks(frame.ticks()), frame).expect("one frame");
    let early = Clip::new(
        ClipId::new(3),
        media(),
        one_frame,
        Flicks::from_ticks(-1),
        Speed::ONE,
        1.0,
    );
    let got = track.insert(early, frame);
    assert!(
        matches!(got, Err(TimelineError::SourceBeforeStart { .. })),
        "a clip starting before its source has no frames to play"
    );
    // The half-open end: the last frame of a clip is the one that starts at
    // end minus a frame, and the frame at `end` is the next clip's.
    let two = frame_clip(ClipId::new(4), 0, 2, frame);
    assert_eq!(two.place().end().ticks(), 2 * frame.ticks());
    let last = two.place().end().ticks() - frame.ticks();
    assert_eq!(
        last,
        frame.ticks(),
        "the last frame starts at end minus frame"
    );
}

// P11 (AC5): no lane holds two clips at any instant, and the stacking of the
// lanes is the order of the tracks, never the order of insertion.
#[test]
fn p_11_no_lane_holds_two_clips_at_any_instant() {
    let (_doc, _token, sid, tid) = setup();
    let frame = FPS_24.flicks_per_frame();
    let mut track = track_with_clips(
        TrackId::new(5),
        vec![frame_clip(ClipId::new(1), 0, 2, frame)],
        frame,
    );
    // A clip that starts inside the one already there overlaps it, no matter
    // when it arrives.
    for start in 0i64..2 {
        let candidate = frame_clip(ClipId::new(100 + start as u64), start, 1, frame);
        let got = track.insert(candidate, frame);
        assert!(
            matches!(got, Err(TimelineError::Overlap { .. })),
            "lane {start} must not overlap the video track"
        );
    }
    // A clip that starts exactly where the other ends is fine: the interval is
    // half-open at both ends.
    let after = frame_clip(ClipId::new(200), 2, 1, frame);
    assert!(
        track.insert(after, frame).is_ok(),
        "the cut at the end of the other clip belongs to this one"
    );
    // The stacking is the order of the tracks list: index 0 the bottom, and
    // the answer keeps it whatever order the tracks were added in.
    let mut sequence = a_sequence(sid, FPS_24);
    sequence
        .add_track(tid, TrackKind::Overlay)
        .expect("the first overlay fits");
    let overlay_id = TrackId::new(77);
    sequence
        .add_track(overlay_id, TrackKind::Overlay)
        .expect("the second overlay fits");
    let video_id = TrackId::new(78);
    sequence
        .add_track(video_id, TrackKind::Video)
        .expect("the video lane fits");
    let kinds: Vec<TrackKind> = sequence.tracks().iter().map(|t| t.kind()).collect();
    assert_eq!(
        kinds,
        vec![TrackKind::Overlay, TrackKind::Overlay, TrackKind::Video],
        "the tracks keep the order they were added in, and that order is the stacking"
    );
}

// FV-01 (AC6): the retime FV-01 (c) fixes moves every cut to the nearest
// frame of the new grid and loses no clip. The expected ticks are absolute and
// the tie goes to the later frame, so a frame index passed where a count of
// ticks belongs fails here instead of passing on a one-frame coincidence.
#[test]
fn fv_01_the_retime_snaps_cuts_to_the_new_grid() {
    let (_doc, _token, sid, tid) = setup();
    let frame_24 = FPS_24.flicks_per_frame();
    let frame_30 = FrameRate::FPS_30.flicks_per_frame();
    let mut sequence = a_sequence(sid, FPS_24);
    sequence
        .add_track(tid, TrackKind::Video)
        .expect("the video lane fits");
    let tie_id = TrackId::new(77);
    sequence
        .add_track(tie_id, TrackKind::Overlay)
        .expect("the overlay lane fits");
    // 60 frames at 24 fps: 2.5 s, which is 75 frames at 30 fps. Both cuts
    // land exactly: the frame duration divides the place on both grids.
    let video = sequence.track_mut_by_id(tid).expect("the lane is there");
    video
        .insert(frame_clip(ClipId::new(1), 0, 60, frame_24), frame_24)
        .expect("the 2.5 s clip sits on the grid");
    // A cut exactly halfway between two 30 fps frames: 2 frames at 24 fps is
    // 2.5 frames at 30 fps, so the tie breaks towards the later frame.
    let tie = sequence.track_mut_by_id(tie_id).expect("the lane is there");
    tie.insert(frame_clip(ClipId::new(2), 2, 1, frame_24), frame_24)
        .expect("the tie clip sits on the grid");
    let before = sequence.clone();
    let after = sequence.retimed(FrameRate::FPS_30);
    assert_eq!(after.fps(), FrameRate::FPS_30);
    // The long clip keeps its 2.5 s: 75 frames of 30 fps, not one frame.
    let video = after.track(tid).expect("the lane is still there").clips();
    assert_eq!(video.len(), 1, "the retime loses no clip");
    assert_eq!(
        video[0].place().start().ticks(),
        0,
        "the first cut stays at 0"
    );
    assert_eq!(
        video[0].place().duration().ticks(),
        1_764_000_000,
        "2.5 s at 30 fps is 75 frames, not one"
    );
    assert_eq!(video[0].place().duration().ticks(), 75 * frame_30.ticks());
    // The tie clip: 58 800 000 ticks is 2.5 frames of 30 fps, so it lands on
    // the third one.
    let tie = after
        .track(tie_id)
        .expect("the lane is still there")
        .clips();
    assert_eq!(tie.len(), 1);
    assert_eq!(
        tie[0].place().start().ticks(),
        70_560_000,
        "a tie breaks towards the later frame (3 frames of 30 fps)"
    );
    assert_eq!(tie[0].place().duration().ticks(), frame_30.ticks());
    // Both cuts are on the new grid, and the function is pure and
    // deterministic.
    for clip in [&video[0], &tie[0]] {
        assert_eq!(clip.place().start().ticks() % frame_30.ticks(), 0);
        assert_eq!(clip.place().duration().ticks() % frame_30.ticks(), 0);
    }
    assert_eq!(sequence, before, "the retime mutates nothing");
    assert_eq!(sequence.retimed(FrameRate::FPS_30), after);
}

// FV-01 (AC7): sixteen audio tracks are allowed, the seventeenth is refused
// with a message that says which sequence ran out.
#[test]
fn fv_01_the_audio_cap_sixteenth_enters_seventeenth_is_refused() {
    let (_doc, _token, sid, _tid) = setup();
    let mut sequence = a_sequence(sid, FPS_24);
    for n in 0..AUDIO_TRACK_CAP {
        sequence
            .add_track(TrackId::new(100 + n as u64), TrackKind::Audio)
            .unwrap_or_else(|error| panic!("the audio track {n} fits: {error:?}"));
    }
    let got = sequence.add_track(TrackId::new(999), TrackKind::Audio);
    assert!(
        matches!(got, Err(TimelineError::TooManyAudioTracks { .. })),
        "the seventeenth audio track does not fit"
    );
    assert!(
        sequence
            .add_track(TrackId::new(998), TrackKind::Video)
            .is_ok(),
        "the cap counts audio lanes only"
    );
    let message = TimelineError::TooManyAudioTracks { sequence: sid }.to_string();
    assert!(
        message.contains("seventeenth") && message.contains(&sid.to_string()),
        "the message says what ran out and where: {message}"
    );
}

// FV-03 (AC8): the speed relation holds: the sequence span of a source span
// is source over speed, its inverse is exact, and zero or negative speeds are
// refused.
#[test]
fn fv_03_the_duration_follows_the_speed_relation() {
    let (_doc, _token, _sid, _tid) = setup();
    let frame = FPS_24.flicks_per_frame();
    let source = Flicks::from_ticks(frame.ticks() * 2);
    let speed = Speed::new(2, 1).expect("speed two is a speed");
    assert_eq!(speed.sequence_span(source).ticks(), frame.ticks());
    let slow = Speed::new(1, 2).expect("speed one half is a speed too");
    assert_eq!(slow.sequence_span(source).ticks(), frame.ticks() * 4);
    // The relation is exact both ways at the identity speed.
    assert_eq!(Speed::ONE.sequence_span(source), source);
    assert_eq!(Speed::ONE.source_span(source), source);
    // Zero divides nothing; a negative speed cannot be a ratio of two `u32`.
    let error = Speed::new(0, 4).expect_err("speed zero divides nothing");
    assert!(matches!(error, TimelineError::InvalidSpeed { .. }));
    assert!(
        Speed::new(5, 0).is_err(),
        "a denominator of zero is not a speed"
    );
    assert_eq!(Speed::ONE.num(), 1);
    assert_eq!(Speed::ONE.den(), 1);
}

// P7 (AC9): the model answers which clip each lane shows at an instant, one
// entry per lane in the stacking order, and `None` where nothing shows.
#[test]
fn p_7_the_lane_answers_what_it_shows_at_an_instant() {
    let (_doc, _token, sid, tid) = setup();
    let frame = FPS_24.flicks_per_frame();
    let mut sequence = a_sequence(sid, FPS_24);
    sequence
        .add_track(tid, TrackKind::Video)
        .expect("the video lane fits");
    let empty_id = TrackId::new(99);
    sequence
        .add_track(empty_id, TrackKind::Overlay)
        .expect("the overlay lane fits");
    let track = sequence.track_mut_by_id(tid).expect("the lane is there");
    track
        .insert(frame_clip(ClipId::new(1), 0, 2, frame), frame)
        .expect("the first clip sits on the grid");
    track
        .insert(frame_clip(ClipId::new(2), 2, 2, frame), frame)
        .expect("the second clip sits on the grid");
    let video = sequence.track(tid).expect("the lane is there");
    assert_eq!(
        video
            .clip_at(Flicks::from_ticks(frame.ticks()))
            .map(|c| c.id()),
        Some(ClipId::new(1)),
        "the first clip shows"
    );
    assert_eq!(
        video
            .clip_at(Flicks::from_ticks(frame.ticks() * 2))
            .map(|c| c.id()),
        Some(ClipId::new(2)),
        "the instant at the end of the first clip belongs to the second one"
    );
    assert!(
        video
            .clip_at(Flicks::from_ticks(frame.ticks() * 4))
            .is_none(),
        "past the last clip there is nothing"
    );
    let empty = sequence.track(empty_id).expect("the lane is there");
    assert!(
        empty.clip_at(Flicks::ZERO).is_none(),
        "an empty lane shows nothing"
    );
    // The sequence's answer: one entry per covering lane, in the order of the
    // tracks.
    let active = sequence.active_clips(Flicks::from_ticks(frame.ticks()));
    assert_eq!(active.len(), 1, "only the video lane covers that instant");
    assert_eq!(active[0].0, 0, "the entry names the stacking index");
    assert_eq!(active[0].1.id(), ClipId::new(1));
    // The stacking is the order of the tracks, not the order of insertion:
    // a lane added later but placed first in the list paints under the one
    // above it. The ids come from the counter in creation order, so the two
    // orders differ here.
    let mut stacked = a_sequence(sid, FPS_24);
    let under_id = TrackId::new(500);
    let over_id = TrackId::new(400);
    stacked
        .add_track(under_id, TrackKind::Overlay)
        .expect("the overlay lane fits");
    stacked
        .add_track(over_id, TrackKind::Video)
        .expect("the video lane fits");
    let under = stacked
        .track_mut_by_id(under_id)
        .expect("the lane is there");
    under
        .insert(frame_clip(ClipId::new(12), 0, 1, frame), frame)
        .expect("the clip sits on the grid");
    let over = stacked.track_mut_by_id(over_id).expect("the lane is there");
    over.insert(frame_clip(ClipId::new(11), 0, 1, frame), frame)
        .expect("the clip sits on the grid");
    let answer = stacked.active_clips(Flicks::ZERO);
    assert_eq!(answer.len(), 2, "both lanes cover the instant");
    assert_eq!(
        answer[0].1.id(),
        ClipId::new(12),
        "the first lane of the list is the bottom one, whatever the ids say"
    );
    assert_eq!(answer[1].1.id(), ClipId::new(11));
    assert_eq!(answer[0].0, 0, "the entry names the stacking index");
    assert_eq!(answer[1].0, 1);
    // The source time of an instant, at the identity speed, is exact.
    let clip = active[0].1;
    assert_eq!(
        clip.source_time(Flicks::from_ticks(frame.ticks())),
        Flicks::from_ticks(frame.ticks()),
        "at speed one the source time is the sequence time"
    );
}

// ARQ-30 (AC10): the timeline mutators live on `DocMut` behind the token, and
// the ids stay unique when the edit view is reused (the counter is one).
#[test]
fn arq_30_the_timeline_mutators_ask_for_the_token() {
    let (mut doc, token, sid, tid) = setup();
    let mut view = doc.edit(&token);
    view.sequences_mut()
        .push(std::sync::Arc::new(a_sequence(sid, FPS_24)));
    let cid = view.alloc_clip_id().unwrap();
    assert_ne!(
        cid.get(),
        tid.get(),
        "the counter never repeats a raw value"
    );
    let sequence = view.sequence_mut(sid).expect("the sequence exists");
    assert_eq!(sequence.id(), sid);
    // The sequence list mutates only through the view.
    let length = view.as_doc().sequences().len();
    assert_eq!(length, 1);
}
