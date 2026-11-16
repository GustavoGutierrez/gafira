//! Wall time from the system clock and monotonic time from `Instant`.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use gafira_ports::common::UnixTime;
use gafira_ports::system::Clock;

/// The system clock; `monotonic` counts from the moment it was built.
pub struct SystemClock {
    start: Instant,
}

impl SystemClock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> UnixTime {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        UnixTime(i64::try_from(secs).unwrap_or(i64::MAX))
    }

    fn monotonic(&self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s18_02_wall_time_is_after_2026_and_monotonic_never_goes_back() {
        let clock = SystemClock::new();
        assert!(clock.now().0 > 1_767_225_600, "after 2026-01-01");
        let a = clock.monotonic();
        let b = clock.monotonic();
        assert!(b >= a);
    }
}
