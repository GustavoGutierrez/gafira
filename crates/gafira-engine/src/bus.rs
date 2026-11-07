//! The transport of the bus: 02 §5.2 (line 1335).
//!
//! `ChannelSink` publishes into an unbounded `crossbeam-channel`, because the
//! spec fixes that "state events are never dropped" and a bounded channel would
//! make the publisher wait for the UI. `JobProgress` is the one exception: it
//! coalesces to 10 Hz, keeps only the latest fraction of the window, and the
//! pending one is delivered by the next state event or by the host's
//! [`EventBus::flush`].
//!
//! The pair is built by the composition root before the engine (design P11).
//!
//! Every delivery wakes the injected [`Waker`], which is what asks the UI for a
//! repaint. A coalesced event does not wake anyone: nothing reached the queue.

use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender, unbounded};

use gafira_ports::system::Clock;

use crate::event::{EngineEvent, EventBus, EventSink, Waker};

/// The coalescing window of `JobProgress`: 10 Hz (02 §5.2 line 1335).
pub const JOB_PROGRESS_WINDOW: std::time::Duration = std::time::Duration::from_millis(100);

/// The pair the composition root builds before the engine exists (design P11,
/// 02 §5.2 line 1331): the bus the publishers hold and the receiver the
/// observer drains, both over one unbounded channel.
///
/// The root builds it with the same `Clock` it puts in `Ports`, hands
/// [`EventChannel::receiver`] to the startup screen and [`EventChannel::bus`]
/// to the startup sequence, then consumes it in `Gafira::new`.
pub struct EventChannel {
    bus: EventBus,
    receiver: Receiver<EngineEvent>,
}

impl EventChannel {
    /// A channel whose deliveries wake `waker`, with `JobProgress` coalesced on
    /// the time `clock` reads.
    #[must_use]
    pub fn new(waker: Arc<dyn Waker>, clock: Arc<dyn Clock>) -> Self {
        let (sender, receiver) = unbounded();
        let sink = Arc::new(ChannelSink::new(sender, waker, clock));
        Self {
            bus: EventBus::new(sink),
            receiver,
        }
    }

    /// A handle that publishes into this channel; every clone feeds the same
    /// queue.
    #[must_use]
    pub fn bus(&self) -> EventBus {
        self.bus.clone()
    }

    /// A handle that drains this channel (02 §5.4 line 1365).
    ///
    /// One drainer at a time: a `crossbeam` receiver is multi-consumer, so two
    /// drainers would split the stream. The startup screen drains until
    /// `StartupFinished`, then the UI drains `Gafira::events`. The engine cannot
    /// enforce this (P11).
    #[must_use]
    pub fn receiver(&self) -> Receiver<EngineEvent> {
        self.receiver.clone()
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

    /// Run `f` over the window state and the sender under one lock, ignoring
    /// lock poisoning: a panic in another thread must not lose the queue.
    ///
    /// `f` returns how many events it delivered; the waker is called that many
    /// times after the lock is released, so a waker can never block a
    /// publisher. An unbounded `send` never blocks (design P11, XP6).
    fn deliver(&self, f: impl FnOnce(&mut Window, &Sender<EngineEvent>) -> usize) {
        let delivered = {
            let mut guard = match self.window.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            f(&mut guard, &self.sender)
        };
        for _ in 0..delivered {
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

impl Window {
    fn take_pending(&mut self) -> Option<EngineEvent> {
        self.pending.take()
    }
}

/// Send one event; `1` when it reached the queue, `0` when the receiver is gone
/// (the error carries the event back and there is nobody to wake).
fn send(sender: &Sender<EngineEvent>, event: EngineEvent) -> usize {
    usize::from(sender.send(event).is_ok())
}

impl EventSink for ChannelSink {
    fn publish(&self, event: EngineEvent) {
        let is_progress = matches!(event, EngineEvent::JobProgress { .. });
        let now = self.clock.monotonic();
        self.deliver(|window, sender| {
            if is_progress {
                if Self::window_is_open(now, window.last_progress) {
                    window.last_progress = Some(now);
                    // The newer fraction supersedes any pending older one.
                    window.pending = None;
                    return send(sender, event);
                }
                // Inside the window: keep only the latest fraction. It leaves
                // on the next event of any other kind, or on the host's flush.
                window.pending = Some(event);
                return 0;
            }
            // A state event: never dropped, never coalesced. It carries the
            // pending progress out first, and a progress that opens the window
            // discards the pending one, so the observer never sees progress go
            // backwards and always ends on the newest fraction.
            let carried = window
                .take_pending()
                .map_or(0, |pending| send(sender, pending));
            carried + send(sender, event)
        });
    }

    fn flush(&self) {
        let now = self.clock.monotonic();
        self.deliver(|window, sender| {
            if !Self::window_is_open(now, window.last_progress) {
                return 0;
            }
            // The window moves only when something leaves (E2-S1).
            match window.take_pending() {
                Some(pending) => {
                    window.last_progress = Some(now);
                    send(sender, pending)
                }
                None => 0,
            }
        });
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
            version: crate::session::DocVersion::default(),
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

    /// Fractions the observer received, in order.
    fn fractions(receiver: &Receiver<EngineEvent>) -> Vec<f32> {
        receiver
            .try_iter()
            .filter_map(|event| match event {
                EngineEvent::JobProgress { fraction, .. } => Some(fraction),
                _ => None,
            })
            .collect()
    }

    /// AC3: a progress that opens the window supersedes the pending one; the
    /// observer ends on the newest fraction after a state event.
    #[test]
    fn ac3_a_new_window_discards_the_stale_pending_progress_before_a_state_event() {
        let (sink, receiver, clock, _waker) = sink();
        sink.publish(progress(0.1));
        sink.publish(progress(0.9));
        clock.advance(JOB_PROGRESS_WINDOW);
        sink.publish(progress(0.2));
        sink.publish(changed());
        assert_eq!(fractions(&receiver), vec![0.1, 0.2]);
    }

    /// AC3: same, ending with a flush.
    #[test]
    fn ac3_a_new_window_discards_the_stale_pending_progress_before_a_flush() {
        let (sink, receiver, clock, _waker) = sink();
        sink.publish(progress(0.1));
        sink.publish(progress(0.9));
        clock.advance(JOB_PROGRESS_WINDOW);
        sink.publish(progress(0.2));
        clock.advance(JOB_PROGRESS_WINDOW);
        sink.flush();
        assert_eq!(fractions(&receiver), vec![0.1, 0.2]);
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

    /// E2-S1: a flush with nothing pending does not move the window, so the next
    /// progress after 100 ms is delivered, not coalesced.
    #[test]
    fn s1_an_idle_flush_does_not_hold_back_the_next_progress() {
        let (sink, receiver, clock, _waker) = sink();

        sink.publish(progress(0.1));
        for _ in 0..7 {
            clock.advance(Duration::from_millis(16));
            sink.flush();
        }
        // 112 ms since the last delivered progress.
        sink.publish(progress(0.9));

        assert_eq!(receiver.try_recv().expect("first"), progress(0.1));
        assert_eq!(receiver.try_recv().expect("next"), progress(0.9));
    }

    /// E2-S2: two publishers over one channel keep the order: the pending
    /// progress of one leaves before the state event of the other.
    #[test]
    fn s2_two_bus_clones_keep_the_pending_progress_before_the_state_event() {
        let clock = TestClock::new();
        let channel = EventChannel::new(Arc::new(NoopWaker), clock);
        let (one, other, receiver) = (channel.bus(), channel.bus(), channel.receiver());

        one.publish(progress(0.2));
        one.publish(progress(0.6));
        other.publish(changed());

        assert_eq!(receiver.try_recv().expect("first"), progress(0.2));
        assert_eq!(receiver.try_recv().expect("pending"), progress(0.6));
        assert_eq!(receiver.try_recv().expect("state"), changed());
    }

    /// E2-S2: the waker runs after the sink released its lock, so a waker that
    /// reaches back into the sink cannot deadlock.
    #[test]
    fn s2_the_waker_is_called_after_the_lock_is_released() {
        struct ProbeWaker {
            sink: std::sync::OnceLock<Arc<ChannelSink>>,
            lock_free: AtomicUsize,
            lock_held: AtomicUsize,
        }
        impl Waker for ProbeWaker {
            fn wake(&self) {
                let free = self
                    .sink
                    .get()
                    .is_some_and(|sink| sink.window.try_lock().is_ok());
                let counter = if free {
                    &self.lock_free
                } else {
                    &self.lock_held
                };
                counter.fetch_add(1, Ordering::SeqCst);
            }
        }

        let (sender, _receiver) = unbounded();
        let probe = Arc::new(ProbeWaker {
            sink: std::sync::OnceLock::new(),
            lock_free: AtomicUsize::new(0),
            lock_held: AtomicUsize::new(0),
        });
        let sink = Arc::new(ChannelSink::new(sender, probe.clone(), TestClock::new()));
        assert!(probe.sink.set(sink.clone()).is_ok());

        sink.publish(progress(0.1));
        sink.publish(progress(0.2));
        sink.publish(changed());

        assert_eq!(
            probe.lock_held.load(Ordering::SeqCst),
            0,
            "woke under the lock"
        );
        assert_eq!(
            probe.lock_free.load(Ordering::SeqCst),
            3,
            "pending, state, first"
        );
    }

    /// E2-S2 / XP6: a send into a channel whose receiver is gone returns an
    /// error at once; the sink must not panic and must not wake anyone.
    #[test]
    fn s2_publishing_after_the_receiver_dropped_does_not_panic() {
        let (sink, receiver, _clock, waker) = sink();
        drop(receiver);

        sink.publish(changed());
        sink.publish(progress(0.1));
        sink.publish(progress(0.2));
        sink.flush();

        assert_eq!(waker.count(), 0, "nothing was delivered");
    }
}
