//! The transport of the bus: 02 §5.2 (line 1335).
//!
//! `ChannelSink` publishes into an unbounded `crossbeam-channel`, because the
//! spec fixes that "state events are never dropped" and a bounded channel would
//! make the publisher wait for the UI. `JobProgress` is the one exception: it
//! coalesces to 10 Hz, keeps only the latest fraction of the window, and the
//! pending one is delivered by the next state event or by the host's
//! [`EventBus::flush`].
//!
//! Every delivery wakes the injected [`Waker`], which is what asks the UI for a
//! repaint. A coalesced event does not wake anyone: nothing reached the queue.

use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender, unbounded};

use gafira_ports::system::Clock;

use crate::event::{EngineEvent, EventBus, EventSink, Waker};

/// The coalescing window of `JobProgress`: 10 Hz (02 §5.2 line 1335).
pub const JOB_PROGRESS_WINDOW: std::time::Duration = std::time::Duration::from_millis(100);

impl EventBus {
    /// A bus that delivers into an unbounded channel and wakes `waker`.
    ///
    /// The receiver is the one §5.4 line 1365 asks the Facade for: the
    /// composition root creates the channel first, hands the receiver to the
    /// UI (or to the splash screen, §5.2 line 1331) and keeps the bus here.
    #[must_use]
    pub fn channel(waker: Arc<dyn Waker>, clock: Arc<dyn Clock>) -> (Self, Receiver<EngineEvent>) {
        let (sender, receiver) = unbounded();
        let sink = Arc::new(ChannelSink::new(sender, waker, clock));
        (Self::new(sink), receiver)
    }
}

/// What the sink remembers between calls.
#[derive(Default)]
struct Window {
    last_progress: Option<std::time::Duration>,
    pending: Option<EngineEvent>,
}

/// The channel sink of `E2`: unbounded delivery, one waker call per event and a
/// coalescing window for `JobProgress`.
pub struct ChannelSink {
    sender: Sender<EngineEvent>,
    waker: Arc<dyn Waker>,
    clock: Arc<dyn Clock>,
    window: Mutex<Window>,
}

impl ChannelSink {
    /// A sink that delivers into `sender`, wakes `waker` and reads the time from
    /// `clock`. The clock is the injected one: this crate never reads its own.
    #[must_use]
    pub fn new(sender: Sender<EngineEvent>, waker: Arc<dyn Waker>, clock: Arc<dyn Clock>) -> Self {
        Self {
            sender,
            waker,
            clock,
            window: Mutex::new(Window::default()),
        }
    }

    /// Run `f` over the window state, ignoring lock poisoning: a panic in
    /// another thread must not lose the queue.
    fn with_window<T>(&self, f: impl FnOnce(&mut Window) -> T) -> T {
        let mut guard = match self.window.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        f(&mut guard)
    }

    /// Deliver one event and wake the observer.
    fn send(&self, event: EngineEvent) {
        if self.sender.send(event).is_ok() {
            self.waker.wake();
        }
    }

    /// Whether the coalescing window is open at `now`.
    fn window_is_open(now: std::time::Duration, last: Option<std::time::Duration>) -> bool {
        match last {
            Some(last) => now.saturating_sub(last) >= JOB_PROGRESS_WINDOW,
            None => true,
        }
    }
}

impl EventSink for ChannelSink {
    fn publish(&self, event: EngineEvent) {
        if matches!(event, EngineEvent::JobProgress { .. }) {
            let now = self.clock.monotonic();
            let open = self.with_window(|window| {
                let open = Self::window_is_open(now, window.last_progress);
                if open {
                    window.last_progress = Some(now);
                }
                open
            });
            if open {
                self.send(event);
            } else {
                // Inside the window: keep only the latest fraction. It leaves on
                // the next event of any other kind, or on the host's flush.
                self.with_window(|window| window.pending = Some(event));
            }
            return;
        }

        // A state event: never dropped, never coalesced. It carries the pending
        // progress out first, so the observer never sees progress go backwards.
        if let Some(pending) = self.with_window(Window::take_pending) {
            self.send(pending);
        }
        self.send(event);
    }

    fn flush(&self) {
        let now = self.clock.monotonic();
        let open = self.with_window(|window| Self::window_is_open(now, window.last_progress));
        if !open {
            return;
        }
        let pending = self.with_window(|window| {
            window.last_progress = Some(now);
            window.take_pending()
        });
        if let Some(pending) = pending {
            self.send(pending);
        }
    }
}

impl Window {
    fn take_pending(&mut self) -> Option<EngineEvent> {
        self.pending.take()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use gafira_ports::common::UnixTime;

    use super::*;
    use crate::event::{JobId, NoopWaker};

    #[derive(Default)]
    struct TestClock {
        now: Mutex<Duration>,
    }

    impl TestClock {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn advance(&self, by: Duration) {
            let mut now = match self.now.lock() {
                Ok(now) => now,
                Err(poisoned) => poisoned.into_inner(),
            };
            *now += by;
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> UnixTime {
            UnixTime(0)
        }

        fn monotonic(&self) -> Duration {
            match self.now.lock() {
                Ok(now) => *now,
                Err(poisoned) => *poisoned.into_inner(),
            }
        }
    }

    #[derive(Default)]
    struct CountingWaker {
        wakes: AtomicUsize,
    }

    impl CountingWaker {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn count(&self) -> usize {
            self.wakes.load(Ordering::SeqCst)
        }
    }

    impl Waker for CountingWaker {
        fn wake(&self) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn progress(fraction: f32) -> EngineEvent {
        EngineEvent::JobProgress {
            job: JobId::new(1),
            fraction,
            eta: None,
        }
    }

    fn changed() -> EngineEvent {
        EngineEvent::DocumentChanged {
            dirty: gafira_edit::DirtySet::default(),
        }
    }

    fn sink() -> (
        Arc<ChannelSink>,
        Receiver<EngineEvent>,
        Arc<TestClock>,
        Arc<CountingWaker>,
    ) {
        let (sender, receiver) = unbounded();
        let clock = TestClock::new();
        let waker = CountingWaker::new();
        let sink = Arc::new(ChannelSink::new(sender, waker.clone(), clock.clone()));
        (sink, receiver, clock, waker)
    }

    /// AC3: five updates inside one window deliver once, and the last fraction
    /// is the one that arrives when the window opens again.
    #[test]
    fn ac3_job_progress_coalesces_to_the_window_and_keeps_the_latest() {
        let (sink, receiver, clock, waker) = sink();

        for step in 1..=5 {
            sink.publish(progress(step as f32 / 10.0));
        }
        assert_eq!(receiver.len(), 1, "one delivery per window");
        assert_eq!(waker.count(), 1, "a coalesced event wakes nobody");

        clock.advance(JOB_PROGRESS_WINDOW);
        sink.flush();

        assert_eq!(receiver.len(), 2, "the flush delivers the pending one");
        let first = receiver.try_recv().expect("the first progress");
        let second = receiver.try_recv().expect("the pending progress");
        assert_eq!(first, progress(0.1));
        assert_eq!(second, progress(0.5), "the latest fraction is the one kept");
        assert_eq!(waker.count(), 2);
        assert!(receiver.try_recv().is_err(), "nothing else is queued");
    }

    /// AC3: a state event carries the pending progress out before itself.
    #[test]
    fn ac3_a_state_event_flushes_the_pending_progress_first() {
        let (sink, receiver, _clock, _waker) = sink();

        sink.publish(progress(0.2));
        sink.publish(progress(0.6));
        sink.publish(changed());

        assert_eq!(receiver.len(), 3);
        assert_eq!(receiver.try_recv().expect("first"), progress(0.2));
        assert_eq!(receiver.try_recv().expect("pending"), progress(0.6));
        assert_eq!(receiver.try_recv().expect("state"), changed());
    }

    /// AC3: a flush inside the window delivers nothing.
    #[test]
    fn ac3_a_flush_inside_the_window_delivers_nothing() {
        let (sink, receiver, clock, waker) = sink();

        sink.publish(progress(0.3));
        sink.publish(progress(0.7));
        clock.advance(JOB_PROGRESS_WINDOW / 2);
        sink.flush();

        assert_eq!(receiver.len(), 1);
        assert_eq!(waker.count(), 1);
    }

    /// AC4: state events are never coalesced.
    #[test]
    fn ac4_ten_state_events_produce_ten_deliveries() {
        let (sink, receiver, _clock, waker) = sink();

        for _ in 0..10 {
            sink.publish(changed());
        }

        assert_eq!(receiver.len(), 10);
        assert_eq!(waker.count(), 10);
    }

    /// AC2: the waker is called once per delivered event, and never for a
    /// coalesced one.
    #[test]
    fn ac2_the_waker_is_called_once_per_delivery() {
        let (sink, _receiver, _clock, waker) = sink();

        sink.publish(changed());
        sink.publish(progress(0.1));
        sink.publish(progress(0.2));

        assert_eq!(waker.count(), 2, "the coalesced one wakes nobody");
    }

    /// A sink with no observer at all still accepts every event.
    #[test]
    fn ac4_the_noop_waker_is_accepted() {
        let (sender, receiver) = unbounded();
        let clock = TestClock::new();
        let sink = ChannelSink::new(sender, Arc::new(NoopWaker), clock);
        sink.publish(changed());
        assert_eq!(receiver.len(), 1);
    }
}
