//! The playback clock and the frame dropping of the preview (ARQ-147).
//!
//! The master clock is the audio clock: the video never blocks it (02 §6.5,
//! FV-11). The time comes from `ports::Clock`, so the crate keeps no clock of
//! its own (ARQ-02, ARQ-138). A video frame whose presentation time is 40 ms
//! or more behind the master is dropped and the next one is presented; a
//! frame 39 ms late is presented (02 §6.5, ARQ-147). The drift between the
//! audio and the video in the preview never passes that same tolerance.

use std::time::Duration;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::media::MediaError;
use gafira_ports::system::Clock;
use gafira_time::{FLICKS_PER_MILLISECOND, FLICKS_PER_SECOND, Flicks, FrameRate, TimeRange};

/// The drift the preview tolerates between the audio and the video: 40 ms
/// (02 §6.5, ARQ-147).
///
/// The tolerance applies to the frame whose presentation has already passed:
/// a frame 39 ms late is presented, and one 40 ms late or later is dropped
/// and the next one is presented (02 §6.5, FV-11 (d)).
pub const SYNC_TOLERANCE: Flicks = Flicks::from_ticks(40 * FLICKS_PER_MILLISECOND);

/// The playback speeds of FV-11.
///
/// The three speeds scale the presentation time while the master stays the
/// audio clock (FV-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speed {
    /// 0,5×.
    Half,
    /// 1×.
    Normal,
    /// 2×.
    Double,
}

impl Speed {
    /// The master time that the audio time `elapsed` covers at this speed
    /// (FV-11).
    ///
    /// Half a tick of a 0,5× playback is rounded down: a tick is 1/705 600 000
    /// of a second, so the loss stays below the resolution of the medium.
    fn scale(self, elapsed: Flicks) -> Flicks {
        let ticks = elapsed.ticks();
        match self {
            Self::Half => Flicks::from_ticks(ticks / 2),
            Self::Normal => elapsed,
            Self::Double => Flicks::from_ticks(ticks.saturating_mul(2)),
        }
    }
}

/// The direction of a frame step (FV-11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepDirection {
    /// `←`: one frame towards the start of the range.
    Back,
    /// `→`: one frame towards the end of the range.
    Forward,
}

/// What the synchronisation decides about one video frame (ARQ-147).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameDecision {
    /// The frame is presented: it is not late, or it is 39 ms late at most.
    Present,
    /// The frame is dropped and the next one is presented: it is 40 ms late
    /// or later.
    Drop,
}

/// The frames a playback presents, one at a time (ARQ-147).
///
/// The crate decodes nothing: the caller hands the presentation times over as
/// its decoder produces them, and the clock decides about each one. The plan
/// is the number of frames the playback will present, which is the total the
/// progress counts (ARQ-139).
pub trait PlaybackSource {
    /// The frames the playback will present.
    fn frames(&self) -> u32;

    /// The presentation time of the next frame, or `None` when the source has
    /// no more frames.
    fn next(&mut self) -> Option<Flicks>;
}

/// What one playback did (ARQ-147).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayReport {
    /// The frames presented.
    pub presented: u32,
    /// The frames dropped because they were late.
    pub dropped: u32,
    /// The speed of the playback.
    pub speed: Speed,
    /// The time the playback took, from the injected clock.
    pub elapsed: Duration,
}

/// One presented frame and the master reading it was decided at.
#[derive(Clone, Copy, Debug)]
struct Presentation {
    /// The presentation time of the frame.
    frame: Flicks,
    /// The master reading of the decision.
    master: Flicks,
}

/// The playback clock of the preview (ARQ-147).
///
/// The master position is the audio clock, scaled by the speed and kept
/// inside the range. The video reads it and drops the frames it cannot reach
/// in time; it never holds the master back (02 §6.5, FV-11).
pub struct PlaybackClock<'a> {
    /// The clock the master reads, injected by the caller (ARQ-138).
    clock: &'a dyn Clock,
    /// The speed of the playback (FV-11).
    speed: Speed,
    /// The range being played.
    range: TimeRange,
    /// The frame rate of the step (FV-11).
    rate: FrameRate,
    /// The clock reading the position was anchored at.
    origin: Duration,
    /// The master position at `origin`.
    anchor: Flicks,
    /// The last reading: the monotonic guard of ARQ-138.
    last: Flicks,
    /// The last presented frame, or `None` before the first one.
    presented: Option<Presentation>,
}

impl<'a> PlaybackClock<'a> {
    /// A playback of `range` at `speed`, stepping whole frames of `rate`.
    ///
    /// The clock supplies the time and the crate uses no clock of its own
    /// (ARQ-02, ARQ-138). The playback starts at the start of the range.
    #[must_use]
    pub fn new(clock: &'a dyn Clock, range: TimeRange, speed: Speed, rate: FrameRate) -> Self {
        let anchor = range.start();
        Self {
            clock,
            speed,
            range,
            rate,
            origin: clock.monotonic(),
            anchor,
            last: anchor,
            presented: None,
        }
    }

    /// The speed of the playback.
    #[must_use]
    pub const fn speed(&self) -> Speed {
        self.speed
    }

    /// The range being played.
    #[must_use]
    pub const fn range(&self) -> TimeRange {
        self.range
    }

    /// The frame rate of the step.
    #[must_use]
    pub const fn rate(&self) -> FrameRate {
        self.rate
    }

    /// The master position now: the audio clock (02 §6.5, ARQ-147).
    ///
    /// The position is the anchor plus the time the clock advanced since the
    /// anchor, scaled by the speed, kept inside the range and never below the
    /// last reading. Two readings of the same instant are therefore equal and
    /// a clock that goes back never makes the playback go back (ARQ-138).
    pub fn master(&mut self) -> Flicks {
        let elapsed = from_duration(self.clock.monotonic().saturating_sub(self.origin));
        let advanced = self
            .anchor
            .checked_add(self.speed.scale(elapsed))
            .unwrap_or(self.range.end());
        let position = advanced.max(self.range.start()).min(self.range.end());
        self.last = self.last.max(position);
        self.last
    }

    /// The drift of the last presented frame behind the master (ARQ-147).
    ///
    /// The drift is measured when the frame is decided, so it does not grow
    /// with the time: a frame 39 ms late reports 39 ms. It is 0 before the
    /// first frame and when that frame was ahead of the master. A presented
    /// frame is never 40 ms late or later, so the drift of the preview stays
    /// below the tolerance of §6.5.
    #[must_use]
    pub fn drift(&self) -> Flicks {
        self.presented.map_or(Flicks::ZERO, |presentation| {
            presentation
                .master
                .checked_sub(presentation.frame)
                .unwrap_or(Flicks::ZERO)
                .max(Flicks::ZERO)
        })
    }

    /// Decides one video frame (02 §6.5, ARQ-147).
    ///
    /// A frame 39 ms late or less is presented. One 40 ms late or later is
    /// dropped, and the next frame is the one presented. A frame ahead of the
    /// master is presented: it is not late, and the video never blocks the
    /// audio to wait for it (02 §6.5).
    pub fn decide(&mut self, frame: Flicks) -> FrameDecision {
        let master = self.master();
        let late = master.checked_sub(frame).unwrap_or(Flicks::ZERO);
        if late >= SYNC_TOLERANCE {
            return FrameDecision::Drop;
        }
        self.presented = Some(Presentation { frame, master });
        FrameDecision::Present
    }

    /// True when the playback reached the end of the range (ARQ-147).
    ///
    /// A range of zero length is over at once: there is nothing to play, and
    /// it neither panics nor drops a frame.
    pub fn is_finished(&mut self) -> bool {
        self.range.is_empty() || self.master() >= self.range.end()
    }

    /// Moves the master one whole frame in `direction` (FV-11).
    ///
    /// The step follows the direction and stays inside the range: backwards
    /// it stops at the start and forwards at the end. The playback continues
    /// from the frame stepped to, and that frame counts as presented at its
    /// own time, so the drift of a step is 0.
    pub fn step(&mut self, direction: StepDirection) -> Flicks {
        let current = self.master();
        let one = self.rate.flicks_per_frame();
        let target = match direction {
            StepDirection::Back => current
                .checked_sub(one)
                .unwrap_or(self.range.start())
                .max(self.range.start()),
            StepDirection::Forward => current
                .checked_add(one)
                .unwrap_or(self.range.end())
                .min(self.range.end()),
        };
        self.origin = self.clock.monotonic();
        self.anchor = target;
        self.last = target;
        self.presented = Some(Presentation {
            frame: target,
            master: target,
        });
        target
    }

    /// Plays the range from the frames of `source` (ARQ-147, ARQ-139).
    ///
    /// Each frame is decided against the audio clock: the late ones are
    /// dropped and the others presented, and the playback never waits for a
    /// frame, so the audio is never cut (02 §6.5, FV-11 (c)). The playback is
    /// a long operation: it checks the cancel token once per frame and
    /// reports the frames done over the frames planned, and the fraction
    /// reaches `1.0` only on a normal end (ARQ-139). It stops at the end of
    /// the range or when the source runs out, whichever comes first.
    ///
    /// # Errors
    ///
    /// [`MediaError::Cancelled`] when the caller cancels the playback, which
    /// then reports no more progress and never reaches `1.0` (ARQ-139).
    pub fn play(
        &mut self,
        source: &mut dyn PlaybackSource,
        cancel: &CancelToken,
        progress: &dyn Progress,
    ) -> Result<PlayReport, MediaError> {
        let started = self.clock.monotonic();
        let planned = source.frames();
        // The end of the playback is one more unit of work, so the fraction
        // of the frames done never reaches 1.0 before the range is over
        // (ARQ-139).
        let total = planned.saturating_add(1);
        let mut presented = 0_u32;
        let mut dropped = 0_u32;
        while presented.saturating_add(dropped) < planned && !self.is_finished() {
            cancel.check()?;
            let Some(frame) = source.next() else {
                break;
            };
            match self.decide(frame) {
                FrameDecision::Present => presented = presented.saturating_add(1),
                FrameDecision::Drop => dropped = dropped.saturating_add(1),
            }
            progress.report(fraction(presented.saturating_add(dropped), total));
        }
        // The last check is before the end is reported, so a cancel after the
        // last frame never reaches 1.0 (ARQ-139).
        cancel.check()?;
        progress.report(1.0);
        let elapsed = self.clock.monotonic().saturating_sub(started);
        Ok(PlayReport {
            presented,
            dropped,
            speed: self.speed,
            elapsed,
        })
    }
}

/// The flicks of a reading of the clock (ARQ-138).
///
/// A reading is a `Duration` of the injected clock and a presentation time is
/// a count of flicks, so the two are converted exactly: no float takes part.
fn from_duration(duration: Duration) -> Flicks {
    let nanos = i128::try_from(duration.as_nanos()).unwrap_or(i128::MAX);
    let ticks = nanos.saturating_mul(i128::from(FLICKS_PER_SECOND)) / 1_000_000_000;
    Flicks::from_ticks(i64::try_from(ticks).unwrap_or(i64::MAX))
}

/// The fraction of the frames done over the frames planned (ARQ-139).
fn fraction(done: u32, planned: u32) -> f32 {
    done as f32 / planned.max(1) as f32
}
