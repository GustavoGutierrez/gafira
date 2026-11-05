//! The doubles the playback tests inject (ARQ-138, ARQ-147).
//!
//! The clock of `tests/support/mod.rs` advances on every reading, so it can
//! not express "the same instant read twice" nor "the clock goes back". The
//! playback tests need both, so they move a clock by hand instead.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use gafira_media::PlaybackSource;
use gafira_ports::common::UnixTime;
use gafira_ports::system::Clock;
use gafira_time::Flicks;

/// A clock the test moves by hand (ARQ-138).
#[derive(Debug, Default)]
pub struct ManualClock {
    nanos: AtomicU64,
}

impl ManualClock {
    /// A clock at zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts the clock at `at`; it may move it back (ARQ-138).
    pub fn set(&self, at: Duration) {
        self.nanos.store(
            u64::try_from(at.as_nanos()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
    }

    /// Moves the clock forward by `step`.
    pub fn advance(&self, step: Duration) {
        self.nanos.fetch_add(
            u64::try_from(step.as_nanos()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
    }
}

impl Clock for ManualClock {
    /// The wall time of the test is always the epoch.
    fn now(&self) -> UnixTime {
        UnixTime(0)
    }

    /// The instant the test set.
    fn monotonic(&self) -> Duration {
        Duration::from_nanos(self.nanos.load(Ordering::Relaxed))
    }
}

/// The frames of a test, one at a time (ARQ-147).
///
/// `moving` advances the clock by a step before every frame, the way a
/// decoder takes time to produce one; `still` leaves the clock where it is.
pub struct Frames<'a> {
    /// The presentation times, in order.
    times: Vec<Flicks>,
    /// The next time to hand over.
    next: usize,
    /// The clock the frames take time from.
    clock: &'a ManualClock,
    /// The time one frame takes.
    step: Duration,
}

impl<'a> Frames<'a> {
    /// The frames `times`, without moving the clock.
    #[must_use]
    pub fn still(clock: &'a ManualClock, times: &[Flicks]) -> Self {
        Self {
            times: times.to_vec(),
            next: 0,
            clock,
            step: Duration::ZERO,
        }
    }

    /// The frames `times`, each one taking `step` of the clock.
    #[must_use]
    pub fn moving(clock: &'a ManualClock, times: &[Flicks], step: Duration) -> Self {
        Self {
            times: times.to_vec(),
            next: 0,
            clock,
            step,
        }
    }
}

impl PlaybackSource for Frames<'_> {
    /// The frames of the test.
    fn frames(&self) -> u32 {
        u32::try_from(self.times.len()).unwrap_or(u32::MAX)
    }

    /// The next presentation time; the clock has moved by the time of one
    /// frame when the caller decides about it.
    fn next(&mut self) -> Option<Flicks> {
        let time = *self.times.get(self.next)?;
        self.next += 1;
        self.clock.advance(self.step);
        Some(time)
    }
}
