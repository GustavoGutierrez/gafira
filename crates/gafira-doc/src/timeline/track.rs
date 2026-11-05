//! A lane of the sequence: one kind, its clips in place order, and the
//! law that keeps one strip on it at every instant (P11, FV-01).

use gafira_time::Flicks;
use serde::{Deserialize, Serialize};

use super::Clip;
use super::error::TimelineError;
use crate::{ClipId, MarkerId, TrackId};

/// The four kinds the timeline knows (FV-01); no other kind exists.
///
/// The five lanes of the interface are a mapping from these values plus the
/// track's label; the model itself knows only the kind and the place in the
/// [`Sequence::tracks`] order, which alone decides the pixel stacking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TrackKind {
    /// The principal video track.
    Video,
    /// A video track placed over the principal one.
    Overlay,
    /// An audio lane.
    Audio,
    /// A subtitle lane.
    Caption,
}

impl TrackKind {
    /// `true` for the kinds whose default `ripple` is on (FV-01): the video
    /// track ripples by default, the others do not.
    #[must_use]
    pub const fn ripples_by_default(self) -> bool {
        matches!(self, Self::Video)
    }
}

/// One track of one sequence: a list of clips, sorted by place, disjoint.
///
/// The index inside [`Sequence::tracks`], not any order of insertion, is the
/// stacking rule of P11: index 0 is the bottom and later tracks paint over
/// earlier ones, the same back-to-front convention [`crate::Page::roots`]
/// uses.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub(crate) id: TrackId,
    pub(crate) kind: TrackKind,
    pub(crate) ripple: bool,
    pub(crate) muted: bool,
    pub(crate) locked: bool,
    pub(crate) solo: bool,
    /// The clips of this lane, sorted by `place.start`, disjoint (P11).
    pub(crate) clips: Vec<Clip>,
}

impl Track {
    /// Builds an empty track with the default flags of FV-01: nothing is muted,
    /// locked or solo, and only a video track ripples (P5).
    #[must_use]
    pub fn new(id: TrackId, kind: TrackKind) -> Self {
        Self {
            id,
            kind,
            ripple: kind.ripples_by_default(),
            muted: false,
            locked: false,
            solo: false,
            clips: Vec::new(),
        }
    }

    /// The track's id, unique inside its document (ARQ-10).
    pub const fn id(&self) -> TrackId {
        self.id
    }

    /// The track's kind.
    pub const fn kind(&self) -> TrackKind {
        self.kind
    }

    /// Whether a change at this track pushes the later clips of the same
    /// lane right (FV-01: the video track ripples by default).
    pub const fn ripple(&self) -> bool {
        self.ripple
    }

    /// Whether the track is muted.
    pub const fn muted(&self) -> bool {
        self.muted
    }

    /// Whether the track is locked against edits.
    pub const fn locked(&self) -> bool {
        self.locked
    }

    /// Whether the track is soloed.
    pub const fn solo(&self) -> bool {
        self.solo
    }

    /// The clips of this lane, in place order (sorted by start, disjoint).
    #[must_use]
    pub const fn clips(&self) -> &Vec<Clip> {
        &self.clips
    }

    /// The span of time this lane covers: the end of its last clip, or zero.
    #[must_use]
    pub fn span(&self) -> Flicks {
        match self.clips.last() {
            Some(last) => last.place.end(),
            None => Flicks::ZERO,
        }
    }

    /// Validates a clip's own laws against this sequence's frame grid: the cut
    /// is a whole number of frames (V-2), the span is at least one frame, and
    /// the source window does not start before the source does (FV-01). The
    /// half-open `end` of the place makes the frame at `place.start +
    /// place.duration` the first frame the clip does not own.
    ///
    /// # Errors
    ///
    /// [`TimelineError::OffGrid`], [`TimelineError::SpanTooShort`] or
    /// [`TimelineError::SourceBeforeStart`], as the first broken law.
    pub fn check_clip(&self, clip: &Clip, frame: Flicks) -> Result<(), TimelineError> {
        if clip.place.start().ticks() % frame.ticks() != 0 {
            return Err(TimelineError::OffGrid {
                clip: clip.id,
                at: clip.place.start().ticks(),
            });
        }
        if clip.place.duration().ticks() % frame.ticks() != 0 || clip.place.duration() < frame {
            return Err(TimelineError::SpanTooShort {
                span: clip.place.duration().ticks(),
            });
        }
        if clip.source_in < Flicks::ZERO {
            return Err(TimelineError::SourceBeforeStart {
                clip: clip.id,
                source_in: clip.source_in.ticks(),
            });
        }
        Ok(())
    }

    /// Validates the P11 law against the clip's place: no instant of it may
    /// overlap the clips already on this lane, and the lane's contents stay
    /// sorted by place.
    pub fn check_overlaps(&self, clip: &Clip) -> Result<(), TimelineError> {
        let idx = self
            .clips
            .partition_point(|c| c.place.end() <= clip.place.start());
        if let Some(left) = self.clips.get(idx.wrapping_sub(1))
            && left.place.end() > clip.place.start()
        {
            return Err(TimelineError::Overlap {
                track: self.id,
                at: clip.place.start().ticks(),
            });
        }
        if let Some(right) = self.clips.get(idx)
            && right.place.start() < clip.place.end()
        {
            return Err(TimelineError::Overlap {
                track: self.id,
                at: right.place.start().ticks(),
            });
        }
        Ok(())
    }

    /// Places a clip on this lane, upholding the placement laws first (the
    /// grid of the sequence, at least one frame, the source's start inside the
    /// source) and the P11 law of one strip per lane at any instant (FV-01).
    /// The clips stay sorted by start; nothing is rounded.
    ///
    /// # Errors
    ///
    /// The first law the design's P4 or P11 breaks: [`TimelineError::OffGrid`],
    /// [`TimelineError::SpanTooShort`], [`TimelineError::SourceBeforeStart`]
    /// or [`TimelineError::Overlap`].
    pub fn insert(&mut self, clip: Clip, frame: Flicks) -> Result<(), TimelineError> {
        self.check_clip(&clip, frame)?;
        self.check_overlaps(&clip)?;
        let idx = self
            .clips
            .partition_point(|c| c.place.end() <= clip.place.start());
        // Never below the first's index: `idx` was computed on the pre-insert
        // list and equals the slot the clip takes.
        self.clips.insert(idx, clip);
        Ok(())
    }

    /// The mutable clip with this id, for the edit view only.
    pub(crate) fn clip_mut_by_id(&mut self, id: ClipId) -> Option<&mut Clip> {
        self.clips.iter_mut().find(|clip| clip.id == id)
    }

    /// Removes a clip from this lane by its id, returning it. Removing is not
    /// a placement law: nothing is relocated here, that is a command's job.
    pub fn remove(&mut self, id: ClipId) -> Vec<Clip> {
        let idx = self.clips.iter().position(|c| c.id == id);
        match idx {
            Some(idx) => vec![self.clips.remove(idx)],
            None => Vec::new(),
        }
    }

    /// The clip of this lane that covers the instant, or `None` when the lane
    /// shows nothing there (P7, the model's answer the resolver asks).
    ///
    /// Half-open at both ends: the instant `place.end` belongs to the next
    /// clip, not to this one, so a cut belongs exactly where the second clip
    /// starts.
    #[must_use]
    pub fn clip_at(&self, at: Flicks) -> Option<&Clip> {
        let idx = self.clips.partition_point(|c| c.place.start() <= at);
        let candidate = idx.checked_sub(1)?;
        let clip = self.clips.get(candidate)?;
        if clip.place.end() > at {
            Some(clip)
        } else {
            None
        }
    }
}

/// A marker on the sequence: a named point in time with a color (FV-01).
///
/// Markers have no span and no law on their instant in this cut beyond being
/// at least zero; a marker that lands between two frames is not a cut, and
/// the laws that move it (`M`, the Imán) arrive with the commands change.
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub(crate) id: MarkerId,
    /// The instant the marker points at; at least zero.
    pub(crate) at: Flicks,
    /// The name FV-01 writes into the marker.
    pub(crate) name: String,
    /// The marker's color.
    pub(crate) color: gafira_color::Color,
}

impl Marker {
    /// Builds the marker; moving it is a command's job, not a law of the
    /// model.
    #[must_use]
    pub const fn new(id: MarkerId, at: Flicks, name: String, color: gafira_color::Color) -> Self {
        Self {
            id,
            at,
            name,
            color,
        }
    }

    /// The marker's id, unique inside its document (ARQ-10).
    pub const fn id(&self) -> MarkerId {
        self.id
    }

    /// The instant the marker sits at.
    pub const fn at(&self) -> Flicks {
        self.at
    }

    /// The marker's name (FV-01).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The marker's color (FV-01).
    pub const fn color(&self) -> &gafira_color::Color {
        &self.color
    }
}
