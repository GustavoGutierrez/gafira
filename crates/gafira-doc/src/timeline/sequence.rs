//! The sequence: a timeline with a rate, a pixel size, a sample rate and a
//! ordered list of tracks whose index decides the stacking (FV-01, P11).
//!
//! A sequence stores no length of its own: [`Sequence::duration`] is the end
//! of its last clip, so the bounds a clip answers to are its track's order,
//! the non-overlap rule of P11 and the frame grid.

use gafira_time::{Flicks, FrameRate, SampleRate};
use serde::{Deserialize, Serialize};

use super::error::TimelineError;
use super::track::{Marker, Track, TrackKind};
use super::{Clip, Speed};
use crate::{ClipId, MarkerId, SequenceId, TrackId};

/// The pixel size of a sequence, whole pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PixelSize {
    /// Whole pixels of the sequence's width.
    pub width: u32,
    /// Whole pixels of the sequence's height.
    pub height: u32,
}

impl PixelSize {
    /// Builds the size; [`Sequence::new`] rejects a zero side.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// One sequence: a rate, a size, a sample rate, the ordered lanes and the
/// markers (FV-01, 02 §3.1).
///
/// Every field is written in the wire form, defaults included (ADR 0006
/// rule 3), and the laws that make the value a timeline are re-applied to a
/// value that came from bytes ([`Sequence::check_laws`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub(crate) id: SequenceId,
    pub(crate) fps: FrameRate,
    pub(crate) size: PixelSize,
    pub(crate) sample_rate: SampleRate,
    /// The tracks in stacking order: index 0 is the bottom, later tracks
    /// paint over earlier ones (P11).
    pub(crate) tracks: Vec<Track>,
    pub(crate) markers: Vec<Marker>,
}

/// How many audio tracks one sequence can hold (FV-01): the sixteenth enters,
/// the seventeenth is refused.
pub const AUDIO_TRACK_CAP: usize = 16;

/// The place a source window takes at one speed, on a sequence's grid
/// ([`Sequence::place_span_for`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaceSpan {
    /// The place in the sequence: a whole number of frames, at least one.
    pub place: Flicks,
    /// The source time the place does not reach; signed, see the method.
    pub remainder: Flicks,
}

impl Sequence {
    /// Builds a sequence; a zero size side is refused (AC1).
    ///
    /// # Errors
    ///
    /// [`TimelineError::EmptyPixelSize`] when either side is zero: a sequence
    /// without pixels serves nothing.
    pub fn new(
        id: SequenceId,
        fps: FrameRate,
        size: PixelSize,
        sample_rate: SampleRate,
    ) -> Result<Self, TimelineError> {
        if size.width == 0 || size.height == 0 {
            return Err(TimelineError::EmptyPixelSize {
                sequence: id,
                width: size.width,
                height: size.height,
            });
        }
        Ok(Self {
            id,
            fps,
            size,
            sample_rate,
            tracks: Vec::new(),
            markers: Vec::new(),
        })
    }

    /// The sequence's id, unique inside its document (ARQ-10).
    pub const fn id(&self) -> SequenceId {
        self.id
    }

    /// The sequence's frame rate, exact and never rounded (V-2).
    pub const fn fps(&self) -> FrameRate {
        self.fps
    }

    /// The sequence's pixel size.
    pub const fn size(&self) -> PixelSize {
        self.size
    }

    /// The sequence's sample rate.
    pub const fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// The tracks of the sequence, in stacking order (index 0 the bottom, P11).
    #[must_use]
    pub const fn tracks(&self) -> &Vec<Track> {
        &self.tracks
    }

    /// The markers of the sequence.
    #[must_use]
    pub const fn markers(&self) -> &Vec<Marker> {
        &self.markers
    }

    /// The exact duration of one frame at this sequence's rate: a whole
    /// number of flicks by the rate's own law. At 23,976 (24 000/1 001) the
    /// duration is exactly 29 429 400 flicks per frame, never rounded to a
    /// millisecond or anything else (V-2, FV-03).
    #[must_use]
    pub fn frame(&self) -> Flicks {
        self.fps.flicks_per_frame()
    }

    /// The end of the last clip of the sequence, over every lane; zero when
    /// the sequence is empty. No stored length: it follows from the clips
    /// (P4 of the design).
    #[must_use]
    pub fn duration(&self) -> Flicks {
        let mut longest = Flicks::ZERO;
        for track in &self.tracks {
            let span = track.span();
            if span > longest {
                longest = span;
            }
        }
        longest
    }

    /// Adds one track at the end of the stacking order.
    ///
    /// The audio cap of FV-01 counts the sixteenth audio track as the last
    /// one: the seventeenth audio track is refused with a message
    /// ([`TimelineError::TooManyAudioTracks`]). Every other kind is
    /// unlimited in this cut.
    ///
    /// # Errors
    ///
    /// [`TimelineError::TooManyAudioTracks`] when this track would be the
    /// seventeenth audio lane.
    pub fn add_track(&mut self, id: TrackId, kind: TrackKind) -> Result<(), TimelineError> {
        if kind == TrackKind::Audio {
            let audio = self
                .tracks
                .iter()
                .filter(|track| track.kind == TrackKind::Audio)
                .count();
            if audio >= AUDIO_TRACK_CAP {
                return Err(TimelineError::TooManyAudioTracks { sequence: self.id });
            }
        }
        self.tracks.push(Track::new(id, kind));
        Ok(())
    }

    /// Adds a marker to the end of the marker list.
    pub fn add_marker(&mut self, marker: Marker) {
        self.markers.push(marker);
    }

    /// The marker with this id.
    #[must_use]
    pub fn marker(&self, id: MarkerId) -> Option<&Marker> {
        self.markers.iter().find(|m| m.id == id)
    }

    /// The track with this id.
    #[must_use]
    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|t| t.id == id)
    }

    /// The track at this stacking index: index 0 the bottom (P11).
    #[must_use]
    pub fn track_at(&self, index: usize) -> Option<&Track> {
        self.tracks.get(index)
    }

    /// The mutable track with this id.
    ///
    /// The sequence is plain data that its holder owns (the document keeps it
    /// behind the edit view of ARQ-30, and a retimed copy belongs to whoever
    /// asked for it), so this accessor is the model's own and not a way around
    /// the token: the token guards the document, not the value.
    pub fn track_mut_by_id(&mut self, id: TrackId) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|track| track.id == id)
    }

    /// The clip with this id, over every lane of the sequence.
    #[must_use]
    pub fn clip(&self, id: ClipId) -> Option<&Clip> {
        self.tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .find(|clip| clip.id == id)
    }

    /// The clip each lane shows at the instant, one per track, in the
    /// stacking order of [`Sequence::tracks`] (P7, P11's readability of the
    /// resolver query). A lane that shows nothing contributes no entry, so
    /// the caller sees `None` where no clip is on the lane.
    #[must_use]
    pub fn active_clips(&self, at: Flicks) -> Vec<(usize, &Clip)> {
        let mut out = Vec::new();
        for (index, track) in self.tracks.iter().enumerate() {
            if let Some(clip) = track.clip_at(at) {
                out.push((index, clip));
            }
        }
        out
    }

    /// Applies the model's laws to a sequence that already exists, which is
    /// the shape a sequence takes when it comes from bytes: the cap of the
    /// audio tracks, then, lane by lane, the cut on the grid, the span of at
    /// least one frame, the clip inside its sequence and inside its source,
    /// the speed that divides and the one strip per lane at any instant
    /// (FV-01, P11, FV-03).
    ///
    /// These are the laws [`Track::insert`] upholds while a sequence is
    /// built. A value read from the wire went through no insertion, so the
    /// document applies them here instead of trusting the bytes (ARQ-13).
    ///
    /// # Errors
    ///
    /// [`TimelineError::TooManyAudioTracks`], [`TimelineError::BeforeSequence`],
    /// [`TimelineError::OffGrid`], [`TimelineError::SpanTooShort`],
    /// [`TimelineError::SourceBeforeStart`], [`TimelineError::InvalidSpeed`]
    /// or [`TimelineError::Overlap`], as the first law the value breaks.
    pub fn check_laws(&self) -> Result<(), TimelineError> {
        let audio = self
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio)
            .count();
        if audio > AUDIO_TRACK_CAP {
            return Err(TimelineError::TooManyAudioTracks { sequence: self.id });
        }
        let frame = self.frame();
        for track in &self.tracks {
            for clip in track.clips() {
                track.check_clip(clip, frame)?;
                let speed = clip.speed();
                if speed.num() == 0 || speed.den() == 0 {
                    return Err(TimelineError::InvalidSpeed {
                        num: speed.num(),
                        den: speed.den(),
                    });
                }
            }
            // The lane keeps its clips sorted by place and none of them meets
            // another: the rule of P11, read over the list the bytes gave.
            for pair in track.clips().windows(2) {
                if pair[1].place().start() < pair[0].place().end() {
                    return Err(TimelineError::Overlap {
                        track: track.id(),
                        at: pair[1].place().start().ticks(),
                    });
                }
            }
        }
        Ok(())
    }

    /// The place a clip takes for a window of one source at one speed, on this
    /// sequence's frame grid (FV-03, design TL2: "the place of the clip is the
    /// source duration over the speed, rounded to the grid, and the source
    /// offset absorbs the remainder").
    ///
    /// `place` is a whole number of frames, at least one, and is the value
    /// [`crate::Track::insert`] accepts; `remainder` is the source time the
    /// place does not reach (`source − speed × place`), signed: positive when
    /// the grid rounded the place down and the window keeps frames unused,
    /// negative when the law of at least one frame asks more source than the
    /// window holds. The source bound that would refuse the second case needs
    /// the import metadata (Q5), so the value is reported instead of refused.
    #[must_use]
    pub fn place_span_for(&self, source: Flicks, speed: Speed) -> PlaceSpan {
        let frame = self.frame().ticks();
        let exact = speed.sequence_span(source).ticks();
        // `snap_nearest` answers a frame index; the place is counted in
        // flicks, and at least one frame is the law's own floor.
        let place = (snap_nearest(exact, frame) * frame).max(frame);
        let used = speed.source_span(Flicks::from_ticks(place)).ticks();
        PlaceSpan {
            place: Flicks::from_ticks(place),
            remainder: Flicks::from_ticks(source.ticks() - used),
        }
    }

    /// The retime of FV-01's acceptance (c): a NEW sequence at `new_rate`
    /// whose cuts land on the nearest frame of the new grid, each clip
    /// keeping at least one frame, the later clips pushed right when a span
    /// stays below one frame, and not one clip lost (FV-01 (c)).
    ///
    /// The ties of the nearest frame break towards the later frame, and the
    /// computation is pure integer arithmetic on flicks: two calls with the
    /// same inputs give the same sequence, and `self` stays untouched
    /// because only a command may write (ARQ-30; the commands change is the
    /// one that stores the result).
    #[must_use]
    pub fn retimed(&self, new_rate: FrameRate) -> Sequence {
        let frame = new_rate.flicks_per_frame().ticks();
        let mut tracks = Vec::with_capacity(self.tracks.len());
        for track in &self.tracks {
            let mut clips = Vec::with_capacity(track.clips.len());
            let mut last_end = 0i64;
            for clip in &track.clips {
                // `snap_nearest` answers a frame index; the place is counted
                // in flicks, so the index is multiplied by the frame.
                let start = snap_nearest(clip.place.start().ticks(), frame) * frame;
                let duration = snap_nearest(clip.place.duration().ticks(), frame) * frame;
                // A span below one frame keeps one frame and pushes what
                // follows it right; nothing is lost (FV-01 (c)).
                let duration = duration.max(frame);
                let start = start.max(last_end);
                let end = start.saturating_add(duration);
                last_end = end;
                let mut moved = clip.clone();
                moved.place = gafira_time::TimeRange::new(
                    Flicks::from_ticks(start),
                    Flicks::from_ticks(duration),
                )
                .expect("the retimed place keeps the range's own law (start, positive span)");
                clips.push(moved);
            }
            tracks.push(Track {
                id: track.id,
                kind: track.kind,
                ripple: track.ripple,
                muted: track.muted,
                locked: track.locked,
                solo: track.solo,
                clips,
            });
        }
        Sequence {
            id: self.id,
            fps: new_rate,
            size: self.size,
            sample_rate: self.sample_rate,
            tracks,
            markers: self.markers.clone(),
        }
    }
}

/// The nearest frame of the grid for `ticks`, ties breaking towards the later
/// frame (FV-01 (c)). `ticks` is at least zero, `frame` at least one.
fn snap_nearest(ticks: i64, frame: i64) -> i64 {
    debug_assert!(ticks >= 0 && frame > 0, "the grid snaps non-negative ticks");
    let q = ticks / frame;
    let r = ticks % frame;
    if r * 2 >= frame { q + 1 } else { q }
}
