//! The frame budget of 02 §6.2: 16 ms per UI frame, split into cumulative
//! deadlines from the start of the frame (ARQ-41, P9 of the layer-3 design).
//!
//! The stages are not independent budgets. The 4 ms of egui, the 2 ms of graph
//! and planning and the 8 ms of the visible render are cumulative: the graph is
//! ready before `start + 6 ms`, the visible render ends before `start + 14 ms`,
//! and the 2 ms from `start + 14 ms` to `start + 16 ms` are the margin of the
//! frame, which no stage may spend. A stage that needs the margin defers the
//! work that does not fit to the next frame (02 §6.2).
//!
//! The deadlines come from the injected clock (`ports::Clock`): the crate is
//! pure in the sense of ARQ-02 (ADR 0016) and reads no clock of its own. The
//! 4 ms of egui belong to the UI and are not a stage this crate runs, so the
//! graph includes them in its cumulative offset.

use std::ops::Range;
use std::time::Duration;

use gafira_ports::render::Deadline;
use gafira_ports::system::Clock;

/// The budget of one UI frame (FIJADA, 02 §6.2).
pub const FRAME: Duration = Duration::from_millis(16);

/// The graph and its planning are ready before `start + 6 ms`: the 4 ms of
/// egui plus the 2 ms of the graph and the planning (02 §6.2).
pub const GRAPH_READY: Duration = Duration::from_millis(6);

/// The visible render ends before `start + 14 ms`: its own 8 ms after the
/// graph (02 §6.2).
pub const VISIBLE_DONE: Duration = Duration::from_millis(14);

/// The margin of the frame, `start + 14 ms` to `start + 16 ms`, which no stage
/// may spend (02 §6.2).
pub const MARGIN: Duration = Duration::from_millis(2);

/// A stage `gafira-render` owes a deadline to (02 §6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The construction of the graph and its planning.
    Graph,
    /// The visible render of the plan.
    Visible,
}

impl Stage {
    /// The cumulative offset of the stage's deadline from the start of the
    /// frame (02 §6.2).
    #[must_use]
    pub fn offset(self) -> Duration {
        match self {
            Self::Graph => GRAPH_READY,
            Self::Visible => VISIBLE_DONE,
        }
    }
}

/// The budget of one frame, as cumulative deadlines from its start (02 §6.2).
///
/// The budget carries the instant the frame started and nothing else: every
/// deadline is `start + offset`, so a stage that runs late cannot move the
/// deadlines of the frame or spend the margin.
#[derive(Clone, Copy, Debug)]
pub struct FrameBudget {
    start: Duration,
}

impl FrameBudget {
    /// Starts a frame at the instant of the injected clock (P9, ARQ-02).
    #[must_use]
    pub fn begin(clock: &dyn Clock) -> Self {
        Self {
            start: clock.monotonic(),
        }
    }

    /// The instant the frame started, from the injected clock.
    #[must_use]
    pub fn start(&self) -> Duration {
        self.start
    }

    /// The instant `start + FRAME`, the end of the frame. It is not a stage
    /// deadline: the span before it is the margin (02 §6.2).
    #[must_use]
    pub fn end(&self) -> Duration {
        self.start.saturating_add(FRAME)
    }

    /// The reserved margin of the frame, `start + 14 ms` to `start + 16 ms`
    /// (02 §6.2).
    #[must_use]
    pub fn margin(&self) -> Range<Duration> {
        self.start.saturating_add(VISIBLE_DONE)..self.end()
    }

    /// The deadline of a stage, as a cumulative instant from the start.
    #[must_use]
    pub fn deadline(&self, stage: Stage) -> Deadline {
        Deadline::At(self.start.saturating_add(stage.offset()))
    }

    /// The deadline of the graph and its planning (02 §6.2).
    #[must_use]
    pub fn graph_deadline(&self) -> Deadline {
        self.deadline(Stage::Graph)
    }

    /// The deadline of the visible render (02 §6.2).
    #[must_use]
    pub fn visible_deadline(&self) -> Deadline {
        self.deadline(Stage::Visible)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, PoisonError};

    use gafira_ports::common::UnixTime;
    use proptest::prelude::*;

    use super::*;

    /// A clock whose `monotonic` answers the scripted instants in order and
    /// repeats the last one. It never reads the system time (ARQ-02).
    struct ScriptClock {
        reads: Mutex<Script>,
    }

    /// The instants a [`ScriptClock`] still has to answer.
    struct Script {
        values: Vec<Duration>,
        at: usize,
    }

    impl ScriptClock {
        /// A clock that answers `values`, in order.
        fn new(values: Vec<Duration>) -> Self {
            Self {
                reads: Mutex::new(Script { values, at: 0 }),
            }
        }
    }

    impl Clock for ScriptClock {
        fn now(&self) -> UnixTime {
            UnixTime(0)
        }

        fn monotonic(&self) -> Duration {
            let mut script = self.reads.lock().unwrap_or_else(PoisonError::into_inner);
            let value = script
                .values
                .get(script.at)
                .copied()
                .unwrap_or_else(|| script.values.last().copied().unwrap_or_default());
            script.at += 1;
            value
        }
    }

    /// The budget of a frame that starts at `start`.
    fn budget_at(start: Duration) -> FrameBudget {
        FrameBudget::begin(&ScriptClock::new(vec![start]))
    }

    /// The budget of a frame that starts at `start`, with a clock that
    /// answers `later` when the frame asks again.
    fn budget_of(start: u64, later: u64) -> FrameBudget {
        FrameBudget::begin(&ScriptClock::new(vec![
            Duration::from_nanos(start),
            Duration::from_nanos(later),
        ]))
    }

    /// Milliseconds, as a duration.
    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    // ARQ-41, 02 §6.2: the stages are cumulative deadlines from the start of
    // the frame, and the margin belongs to no stage.
    #[test]
    fn arq_41_the_stages_are_cumulative_deadlines() {
        let budget = budget_at(ms(100));
        assert_eq!(budget.start(), ms(100));
        assert_eq!(budget.deadline(Stage::Graph), Deadline::At(ms(106)));
        assert_eq!(budget.deadline(Stage::Visible), Deadline::At(ms(114)));
        assert_eq!(budget.margin(), ms(114)..ms(116));
        assert_eq!(budget.end(), ms(116));
        // The margin is exactly the span the stages may not spend.
        assert_eq!(VISIBLE_DONE + MARGIN, FRAME);
        for stage in [Stage::Graph, Stage::Visible] {
            let Deadline::At(at) = budget.deadline(stage) else {
                panic!("the deadline of a frame is always bounded");
            };
            assert!(at <= budget.margin().start, "{stage:?} spends the margin");
        }
        // A stage that runs into the margin does not move the deadlines: they
        // are anchored at the start of the frame, not at the instant they are
        // asked for.
        let clock = ScriptClock::new(vec![ms(100), ms(112)]);
        let budget = FrameBudget::begin(&clock);
        assert_eq!(clock.monotonic(), ms(112), "the graph ran into the margin");
        assert_eq!(budget.deadline(Stage::Visible), Deadline::At(ms(114)));
    }

    // ARQ-02, 02 §6.2: the deadline instant comes from the injected clock; the
    // crate reads no clock of its own.
    // ARQ-138: the same for this crate: the deadline comes from the clock the
    // caller injects, and two different clocks give two different deadlines.
    #[test]
    fn arq_02_the_deadline_comes_from_the_injected_clock() {
        for start in [ms(0), ms(7), ms(1_000)] {
            let budget = budget_at(start);
            assert_eq!(
                budget.graph_deadline(),
                Deadline::At(start + GRAPH_READY),
                "start {start:?}"
            );
            assert_eq!(
                budget.visible_deadline(),
                Deadline::At(start + VISIBLE_DONE),
                "start {start:?}"
            );
        }
        // Two frames that start at different instants get different deadlines.
        assert_ne!(
            budget_at(ms(0)).visible_deadline(),
            budget_at(ms(5)).visible_deadline()
        );
    }

    // ARQ-02: the same input gives the same budget and the same deadlines.
    #[test]
    fn arq_02_the_budget_takes_the_same_decisions_twice() {
        let first = budget_at(ms(42));
        let second = budget_at(ms(42));
        assert_eq!(first.start(), second.start());
        for stage in [Stage::Graph, Stage::Visible] {
            assert_eq!(first.deadline(stage), second.deadline(stage));
        }
        assert_eq!(first.margin(), second.margin());
    }

    // ARQ-02, 02 §6.2: the determinism of AC7 as a property: with any input,
    // the budget takes the same decisions twice and the deadlines are the
    // cumulative offsets of the frame, so no hidden state moves them.
    proptest! {
        #[test]
        fn arq_02_the_budget_takes_the_same_decisions_with_any_input(
            start in 0_u64..1_000_000,
            later in 0_u64..1_000_000,
        ) {
            let first = budget_of(start, later);
            let second = budget_of(start, later);

            prop_assert_eq!(first.start(), second.start());
            prop_assert_eq!(first.end(), second.end());
            prop_assert_eq!(first.margin(), second.margin());
            for stage in [Stage::Graph, Stage::Visible] {
                prop_assert_eq!(first.deadline(stage), second.deadline(stage));
                // The deadline of a stage is its cumulative offset from the
                // start of the frame (02 §6.2), whatever the clock answers
                // later.
                prop_assert_eq!(
                    first.deadline(stage),
                    Deadline::At(first.start().saturating_add(stage.offset()))
                );
            }
            // The margin is the span no stage may spend.
            prop_assert_eq!(
                first.margin().start,
                first.start().saturating_add(VISIBLE_DONE)
            );
            prop_assert_eq!(first.margin().end, first.end());
        }
    }
}
