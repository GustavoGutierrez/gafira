//! A clip: a rectangle of one source placed in one sequence, with the speed
//! relation between the two (FV-01, FV-03).

use gafira_time::{Flicks, TimeRange};

use serde::{Deserialize, Serialize};

use super::error::TimelineError;
use crate::{AssetRef, ClipId, NodeId, PageId};

/// The ratio between source time and sequence time: `speed = source / sequence`
/// (FV-03's relation `place = source / speed` written per channel of time).
///
/// Positive and reduced to lowest terms at construction, so the same ratio has
/// one form and comparisons and hashes are exact. A zero numerator or
/// denominator is refused: it divides nothing (FV-03).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Speed {
    num: u32,
    den: u32,
}

impl Speed {
    /// 1/1, the identity speed: every source frame lands where it was recorded.
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// Validates and reduces a speed ratio in lowest terms.
    ///
    /// # Errors
    ///
    /// [`TimelineError::InvalidSpeed`] when the numerator or the denominator
    /// is zero (FV-03).
    pub fn new(num: u32, den: u32) -> Result<Self, TimelineError> {
        if num == 0 || den == 0 {
            return Err(TimelineError::InvalidSpeed { num, den });
        }
        let g = gcd(num, den);
        Ok(Self {
            num: num / g,
            den: den / g,
        })
    }

    /// Numerator, in lowest terms.
    pub const fn num(self) -> u32 {
        self.num
    }

    /// Denominator, in lowest terms.
    pub const fn den(self) -> u32 {
        self.den
    }

    /// The span of sequence time a source span plays at this speed:
    /// `source × den / num`, floored to whole flicks and never below zero.
    /// Not rounded to any grid here: rounding to the sequence's grid is
    /// [`crate::timeline::Sequence::place_span_for`]'s job.
    #[must_use]
    pub fn sequence_span(self, source: Flicks) -> Flicks {
        let span = i128::from(source.ticks()) * i128::from(self.den) / i128::from(self.num);
        Flicks::from_ticks(span as i64)
    }

    /// The span of source time a place of the sequence consumes at this speed:
    /// the exact inverse of [`Speed::sequence_span`].
    #[must_use]
    pub fn source_span(self, sequence: Flicks) -> Flicks {
        let span = i128::from(sequence.ticks()) * i128::from(self.num) / i128::from(self.den);
        Flicks::from_ticks(span as i64)
    }
}

/// Greatest common divisor of two non-zero values.
fn gcd(a: u32, b: u32) -> u32 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// Where a clip's content comes from (02 §3.1).
///
/// The identity is by content, never by path (P9): a `Media` or `Audio` source
/// names an asset by its hash, so replacing the file on disk does not move any
/// clip, and a `Scene` names a page by its id. The wire form names the tag in
/// `snake_case` and then the value: `{"scene":1}`, `{"media":"<hash>"}`,
/// `{"nodes":[2, 3]}` or `{"audio":"<hash>"}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClipSource {
    /// A page, rendered as the clip's content.
    Scene(PageId),
    /// A media asset inside the project's asset table.
    Media(AssetRef),
    /// A set of nodes of the document, composited in place.
    Nodes(Vec<NodeId>),
    /// An audio asset inside the project's asset table.
    Audio(AssetRef),
}

/// One rectangle of one source over one sequence (02 §3.1).
///
/// The laws it obeys live in [`crate::timeline::Track::insert`] and [`crate::timeline::Sequence`]: its cuts land
/// on whole frames of the sequence's rate, its span is at least one frame,
/// half-open by [`TimeRange`], and it starts at or after the start of its
/// source (`source_in` is at least zero). The animation of the volume arrives
/// with the commands change; today it is a bare value (Q3).
///
/// Every field is written in the wire form, defaults included (ADR 0006
/// rule 3); the laws are re-applied to a value that came from bytes
/// ([`Sequence::check_laws`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub(crate) id: ClipId,
    pub(crate) source: ClipSource,
    pub(crate) place: TimeRange,
    pub(crate) source_in: Flicks,
    pub(crate) speed: Speed,
    pub(crate) volume: f32,
}

impl Clip {
    /// Builds the value itself; the placement laws are [`crate::timeline::Track::insert`]'s.
    ///
    /// `volume` follows Q3: a bare `0.0..=1.0` value until the animation
    /// change arrives; the commands clamp.
    #[must_use]
    pub fn new(
        id: ClipId,
        source: ClipSource,
        place: TimeRange,
        source_in: Flicks,
        speed: Speed,
        volume: f32,
    ) -> Self {
        Self {
            id,
            source,
            place,
            source_in,
            speed,
            volume,
        }
    }

    /// The clip's id, unique inside its document (ARQ-10).
    pub const fn id(&self) -> ClipId {
        self.id
    }

    /// Where the clip's content comes from.
    pub const fn source(&self) -> &ClipSource {
        &self.source
    }

    /// The [start, end) range the clip occupies in its sequence.
    pub const fn place(&self) -> TimeRange {
        self.place
    }

    /// The offset in the source the clip starts at, in flicks, at least zero.
    pub const fn source_in(&self) -> Flicks {
        self.source_in
    }

    /// The speed relation between source and sequence time.
    pub const fn speed(&self) -> Speed {
        self.speed
    }

    /// The clip's volume, a bare value until the animation change (Q3).
    pub const fn volume(&self) -> f32 {
        self.volume
    }

    /// The span of source time this clip's place consumes at its speed: the
    /// exact inverse of [`Speed::sequence_span`].
    #[must_use]
    pub fn source_span(&self) -> Flicks {
        self.speed.source_span(self.place.duration())
    }

    /// The source time an instant of the sequence plays, in `i128` with the
    /// floor of the design: `source_in + (at − place.start) × speed`.
    ///
    /// At the identity speed the value is exact, not a rounded division
    /// (FV-03). `at` must lie inside the clip's place; [`crate::timeline::Track::clip_at`]
    /// upholds that, its callers never see the outside.
    #[must_use]
    pub fn source_time(&self, at: Flicks) -> Flicks {
        let span = i128::from(at.ticks()) - i128::from(self.place.start().ticks());
        let scaled = i128::from(self.source_in.ticks())
            + span * i128::from(self.speed.num) / i128::from(self.speed.den);
        Flicks::from_ticks(scaled as i64)
    }
}
