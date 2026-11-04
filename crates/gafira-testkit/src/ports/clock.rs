//! The fake clock of ARQ-112: the port double a test injects in place of the
//! system clock (02 §6.4).

use std::time::Duration;

use gafira_ports::common::UnixTime;
use gafira_ports::system::Clock;

/// The fake clock. A stub: the media stream fills it.
pub struct FakeClock;

impl Clock for FakeClock {
    /// Not filled yet.
    fn now(&self) -> UnixTime {
        panic!("stub: the clock double is not filled yet")
    }

    /// Not filled yet.
    fn monotonic(&self) -> Duration {
        panic!("stub: the clock double is not filled yet")
    }
}
