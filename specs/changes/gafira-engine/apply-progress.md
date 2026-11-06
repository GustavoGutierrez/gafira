# `gafira-engine` — delivery log

Flow: full | level: L3 | pack: new capability | mutation: off | tier: hand mutants only | verify: V1 | decided-by: owner | reason: a new layer-3 crate with the Facade contract and the event bus; `mutation: off` follows O10 of the render change and the owner's instruction for this block.

Design: `specs/changes/gafira-engine/design.md`.

## Slice card — `E1` scaffold

Product IDs: none. This delivery adds no user-visible capability; it registers the crate and
fixes the two contracts the rest of the change builds on.

Scope:

- `crates/gafira-engine/` with `#![forbid(unsafe_code)]`, no `std::fs`, no threads, no clock of
  its own (design P1, ARQ-02, ADR 0016).
- `crates/gafira-engine/clippy.toml`, byte-identical to `crates/gafira-geom/clippy.toml` (V12).
- One row in the V5 table of `.github/workflows/ci.yml`:
  `gafira-engine gafira-ports gafira-doc gafira-edit gafira-geom gafira-color gafira-time
  gafira-raster` — the whole transitive tree, which is what the V5 step compares (design P1, ARQ-01).
- `gafira-engine = { path = "crates/gafira-engine" }` in `[workspace.dependencies]` of the root
  `Cargo.toml`, and the regenerated `Cargo.lock`.
- `Gafira` (the Facade of 02 §5.4) with `Ports`, `Registries`, `EngineConfig` and `EngineError`.
- `EngineEvent` with the five variants whose payloads exist today (design P3), `EventSink`,
  `NoopSink` and `EventBus`.

Non-goals: no service is functional, no channel dependency, no persisted queue, no governor, no
`UserError` mapping (design P2, P6, P7, P8; deliveries `E2` to `E10`).

Acceptance criteria:

| # | Criterion | Proof |
|---|---|---|
| AC1 | The crate builds with `#![forbid(unsafe_code)]` and imports no `std::fs`, `std::net`, thread or clock API | `cargo clippy -p gafira-engine --all-targets --locked -- -D warnings`; the `clippy.toml` copy rejects the calls at lint time |
| AC2 | The V5 and V12 steps accept the new row and the new copy | the two shell snippets of the workflow run locally against the new row |
| AC3 | `EngineConfig::new` rejects an empty data directory with `InvalidConfig`, and keeps the limits it is given | unit test |
| AC4 | `publish` reaches the sink exactly once per event, and the no-op sink accepts every variant | unit test with a counting sink |
| AC5 | `Gafira` is `Send + Sync` and keeps the ports it was given | unit test with a compile-time assertion |
| AC6 | The dependency tree holds no `gafira-i18n` and no ICU crate, which is the part of the ARQ-107 rule that reaches layer 3 | `cargo tree -p gafira-engine -e normal,build --target all` |

Plan: RED for AC3 and AC4, then the scaffold, then the registration files, then the gates at
`--workspace` scope because the commit touches `Cargo.lock` and `ci.yml` (ADR 0017). Hand mutants:
drop the empty-path check in `EngineConfig::new` (AC3 must fail), and make `publish` skip the sink
(AC4 must fail).

Estimate: 230 authored lines, 322 at x1.4. Verify: V1, gates and the criterion map.

## Notes

### Hand mutants of `E1` (applied, watched fail, reverted; none committed)

| # | Mutation | Killing test | Failure message |
|---|---|---|---|
| H1 | `EngineConfig::new`: the `if data_dir.as_os_str().is_empty() { return Err(EngineError::InvalidConfig { .. }); }` block deleted, so an empty path is accepted | `engine::tests::ac3_an_empty_data_directory_is_rejected` | `panicked at crates/gafira-engine/src/engine.rs:377: an empty data directory must be rejected: EngineConfig { data_dir: "", memory_limit: None, export_jobs: 1 }` |
| H2 | `EventBus::publish` body replaced by `let _ = event;`, so the sink is never called | `event::tests::ac4_publish_reaches_the_sink_exactly_once` | `assertion 'left == right' failed: left: 0, right: 1` at `crates/gafira-engine/src/event.rs:157` |

Both were applied to the source, the named test was run and failed with the message above,
and the file was restored with `git checkout --`; `cargo test -p gafira-engine --locked`
was green again afterwards. Nothing from a mutant reached a commit.

### Fix round after verify `V1` (round 1)

- **W1, AC5.** `ac5_the_facade_is_send_and_sync_and_keeps_the_ports_it_was_given` now builds the
  six trait-object `Arc`s itself, hands clones to `Gafira::new` and asserts `Arc::ptr_eq` for
  `repository`, `store`, `filesystem`, `clock`, `notifier` and `license` against them. It still
  asserts `Send + Sync`, the data directory and the empty format registry.
  `ac5_publishing_through_the_facade_reaches_the_bus` is **deleted**: `Gafira::new` fixes
  `EventBus::noop()` and `E1` injects no sink, so the test could not observe delivery and could
  only fail by not compiling. AC4's delegation is proved on `EventBus` directly; `E2` restores a
  facade publish test with the channel sink and a real observer.
- **W2, AC4.** `ac4_the_noop_sink_accepts_every_variant` now publishes all five variants through a
  counting sink that forwards each one to the real `NoopSink`, and asserts one delivery per event
  and five in total. `LocaleChanged` is included, built from `LanguagePreference::System` and a
  `ResolvedLocale`. The count is what makes the test fail if a variant is dropped.
- **W3.** The hand-mutant record is the table above.
- **S2.** `.odd/state.json` `E1-AC2` now names the three consumers of the V5 table (V5, V12,
  ARQ-107) and the policy run that exercised them with the engine row present.
- **S3.** `EngineEvent` is `#[non_exhaustive]`, with the reason in its doc comment. No in-tree
  matcher needed a wildcard.
- **C1.** The workflow half arrived with the `develop` merge (`5e582e1`); the design half is the
  corrected `XP1`, which no longer claims the ARQ-107 step leaves layer 3 alone.

Measured size of the round's code commits: `git diff --shortstat e7bb63f..HEAD -- crates/` =
2 files changed, 76 insertions, 25 deletions (the deleted test accounts for most of the
deletions).


## Slice card — `E2` bus transport

Product IDs: none. This delivery makes the bus of `E1` observable: the transport, the receiver and
the coalescing window. No service is functional yet.

Deliveries `E2` to `E10` in `E1`'s non-goals are unaffected; `E2` is the first of them.

Scope:

- `EngineEvent::JobProgress { job, fraction, eta }` and the `JobId` newtype (the vocabulary §5.2
  line 1322 needs and no crate owns; design P3 assigns it to this delivery).
- `Waker` trait — one method, `wake()` — and `NoopWaker`; the engine calls it on every delivery
  (02 §5.2 line 1335: an injected waker that asks the UI for a repaint).
- `EventSink::flush` with a default no-op, so `E1`'s sinks keep compiling.
- `ChannelSink`: an unbounded `crossbeam_channel` for state events, the `Waker` call and the 10 Hz
  coalescing of `JobProgress`.
- `EventBus::channel(waker, clock) -> (EventBus, Receiver<EngineEvent>)` and `EventBus::flush`.
- `Gafira::new(ports, registries, config, waker)`, `Gafira::events() -> &Receiver<EngineEvent>` and
  `Gafira::flush_events()`.
- `crossbeam-channel` in the root `[workspace.dependencies]`, in the engine manifest and in
  `Cargo.lock`; `cargo deny check` on the three targets.

Non-goals: no `SaveState`, no `JobFinished`, no `RecoveryAvailable` (they belong to `E4`, `E7` and
the governor), no `UserError` mapping beyond what `E1` shipped, no service.

Acceptance criteria:

| # | Criterion | Proof |
|---|---|---|
| AC1 | `Gafira::publish` reaches the observer through the channel, which is what **W5** asked for: gutting the method body must fail a test | unit test: publish through the facade, receive the event from `events()`, assert the waker count |
| AC2 | The waker is called exactly once per delivered event and never for a coalesced one | unit test with a counting waker |
| AC3 | `JobProgress` is coalesced to 10 Hz and the last one always arrives | unit test with the injected clock: five in one window, then a flush; and a non-progress event flushing the pending one |
| AC4 | State events are never dropped or coalesced | unit test: ten `DocumentChanged` produce ten deliveries |
| AC5 | `docs`-only commits skip cargo; the code commit runs the `--workspace` scope | gate output in the commit notes |
| AC6 | The new dependency is allowed by `deny.toml` and by §6 of the spec | `cargo deny check` on the three targets |

Plan: RED for AC1 (publish through the facade with nothing behind it), AC2, AC3 and AC4, then the
sink, then the facade wiring, then the gates. Hand mutants: make `publish` skip the bus (AC1 must
fail); call the waker on a coalesced event (AC2 must fail); drop the window check (AC3 must fail);
route state events through the coalescer (AC4 must fail).

Estimate: 320 authored lines, 448 at x1.4. Verify: V1, gates and the criterion map.

## Notes — `E2`

- **W5 is closed.** `w5_publishing_through_the_facade_reaches_the_observer` publishes through the
  facade, drains `Gafira::events()` and asserts exactly one delivery. Gutting `Gafira::publish` now
  fails a test (mutant M1 below).
- `Gafira::new` gains the `waker` argument. `§5.4` line 1356 does not list it and P2 of the design
  said the transport would arrive "without changing this signature"; it could not, because an
  optional bus cannot honour `events()`. The deviation is recorded in the method's doc comment.
- `EngineEvent` loses `Eq`, which `JobProgress`'s `f32` cannot satisfy; `PartialEq` stays.
- `JobId(u64)` and `JobProgress { job, fraction, eta }` join `event.rs`: the first **Planned**
  variant of design P3 to land. The remaining seven still belong to their deliveries.
- `EventSink` gains `flush` with a default no-op, so `NoopSink` and the `E1` tests compiled without
  a change.
- The coalescing clock is `ports::Clock`, not a clock of its own (design P1): the tests drive a
  `TestClock` and the window is deterministic.

Hand mutants: each was applied, the named test failed, the file was restored, and
`cargo test -p gafira-engine --locked` was green afterwards. Nothing from a mutant reached a commit.

| # | Mutant | Killed by |
|---|---|---|
| M1 | `Gafira::publish` body replaced by `let _ = event;` | `w5_publishing_through_the_facade_reaches_the_observer` |
| M2 | a coalesced `JobProgress` also calls `waker.wake()` | `ac2_the_waker_is_called_once_per_delivery`, plus two `ac3` tests |
| M3 | `ChannelSink::window_is_open` returns `true` | the three `ac3` tests |
| M4 | state events routed through the coalescer | `ac4_ten_state_events_produce_ten_deliveries`, plus two more |

## E2 fix round

Commits: the channel and facade change (F1, F2, F6), then the sink change (F3, F4, F5). Gates per
code commit: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D
warnings`, `cargo test --workspace --locked`, all green. No manifest changed, so no `cargo deny`.

RED evidence:

| Item | Failing test and assertion before the code |
|---|---|
| F1 | `p11_an_event_published_before_the_engine_exists_is_drained_first`: left `LinksMissing([])`, right `BackendChanged(Cpu)` (stub `Gafira::new` that ignored the channel) |
| F2 | `w5_publishing_through_the_facade_reaches_the_observer_and_wakes_it`: "one wake", left 0, right 1 (same stub) |
| F5 | `s1_an_idle_flush_does_not_hold_back_the_next_progress`: the second `try_recv` was empty (old `flush` moved the window with nothing pending) |
| F3, F4, F6 | no RED: F3 and F4 are characterization tests of behaviour the old code already had (the waker already ran outside the lock and a dropped receiver was already handled); they are pinned by hand mutants 3, 4 and 5. F6 is docs only. |

Hand mutants (applied, test failed, reverted; never committed):

| # | Mutant | Killed by | Failure |
|---|---|---|---|
| 1 | `Gafira::new` builds its bus with a no-op waker instead of the channel's | `w5_publishing_through_the_facade_reaches_the_observer_and_wakes_it`, `p11_an_event_published_before...` | `try_recv` expects on the bus/waker path fail; wake count 0 |
| 2 | `flush` moves the window with nothing pending | `s1_an_idle_flush_does_not_hold_back_the_next_progress` | next `try_recv` empty |
| 3 | `LocaleChanged` dropped from the every-variant list | `ac4_every_variant_reaches_the_sink_once` | left 5, right 6 |
| 4a | waker called while the lock is held | `s2_the_waker_is_called_after_the_lock_is_released` | `lock_held` left 3, right 0 |
| 4b | a coalesced `JobProgress` wakes | `ac2_the_waker_is_called_once_per_delivery`, two `ac3` tests, `s2_...` pair | left 3, right 2 |
| 5 | `send` result unwrapped | `s2_publishing_after_the_receiver_dropped_does_not_panic` | panic at the `unwrap` |

Deviations:

- F3 uses a counting sink (the bus forwards to it), not "the real `NoopSink`", because a
  `NoopSink` cannot be counted; the assertion is one count per variant and the exact total of six.
- Observed and left alone: a progress that opens the window does not clear an older pending one,
  so a stale fraction can still leave later. No finding names it; it belongs in a follow-up.

## E2 fix round 2

Item E2-C2 (CRITICAL, verify round 2): a `JobProgress` that opened a new coalescing window left
an older pending progress in place, so the observer received 0.1, 0.2, 0.9 and ended on a stale
fraction (AC3). Fix: when a progress opens the window, the pending one is discarded (superseded)
before sending; the comment on the state-event path now says so. This closes the stale-fraction
deviation recorded at the end of the E2 fix round. The single-slot design is unchanged (E2-W7
stays a follow-up).

Commit: `fix(gafira-engine): a progress that opens the window supersedes the pending one`. Gates
green: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D
warnings`, `cargo test --workspace --locked`.

RED evidence (new tests run against the code at `93c0be8`, fix removed, tests kept):

| Test | Assertion before the fix |
|---|---|
| `ac3_a_new_window_discards_the_stale_pending_progress_before_a_state_event` | `fractions` left `[0.1, 0.2, 0.9]`, right `[0.1, 0.2]` |
| `ac3_a_new_window_discards_the_stale_pending_progress_before_a_flush` | `fractions` left `[0.1, 0.2, 0.9]`, right `[0.1, 0.2]` |

Hand mutants (applied, test failed, reverted; never committed):

| # | Mutant | Killed by | Failure |
|---|---|---|---|
| a | a progress that opens the window keeps the stale pending one | both `ac3_a_new_window_...` tests | left `[0.1, 0.2, 0.9]` |
| b | the pending one is sent after the new one instead of being discarded | both `ac3_a_new_window_...` tests | left `[0.1, 0.2, 0.9]` |

Deviations: none. The tests and fix came from an interrupted earlier run of this round. They were
kept as they were, after RED was shown again against `93c0be8`.

## Slice card — `E3a` history dirty sets

Product IDs: 02 §5.1, §4.3; ARQ-30 (design P14, row `E3a`). Branch `feat/gafira-engine-e3`, cut
from `feat/gafira-engine-e2` at `6f7525b` after `E2` PASS (round 3); `E3a` to `E3d` and `E5` run
serially on it (owner, 2026-11-06). Flow line of the change applies (header of this file).

Scope:

- `History::newest_touches(&self) -> Option<DirtySet>`: `touches()` of the newest undo entry.
- `History::undone_touches(&self) -> Option<DirtySet>`: `touches()` of the newest redo entry.
- Both additive, read-only, in `crates/gafira-edit/src/history.rs`; doc comments cite P14.

Non-goals: no engine code (that is `E3b`); no change to `execute`, `undo` or `redo` signatures; no
change to any `Command` impl; no change to `crates/gafira-doc`, `crates/gafira-render`,
`crates/gafira-render-cpu`.

Acceptance criteria:

| # | Criterion | Proof |
|---|---|---|
| AC1 | After an `execute` that returns `Changed`, `newest_touches()` equals the `DirtySet` the command reports once applied | unit test with a command whose set is only known after `apply` (e.g. a reparent or an insert) |
| AC2 | After `undo` returns `Some`, `undone_touches()` equals the set of the entry just undone, and `newest_touches()` is the set of the entry now on top (or `None`) | unit test with two distinct entries |
| AC3 | After `redo` returns `Some`, `newest_touches()` is the redone entry's set and `undone_touches()` moves to the next redo entry (or `None`) | unit test |
| AC4 | Inside one merged gesture, `newest_touches()` reports the merged entry's set (`SetProperty`, the only merging command, XP4) | unit test with a gesture of several merged steps |
| AC5 | Both return `None` on an empty stack; an `execute` that returns `Unchanged` leaves both unchanged | unit test |
| AC6 | The accessors only read installed entries (XP4: before `apply` some commands report an empty or partial set) | unit test: a command that fails to apply leaves `newest_touches()` as it was |
| AC7 | Gates at `--workspace` scope (every crate above layer 1 compiles against `gafira-edit`) | gate output in the commit notes |

Every test asserts a value that can fail (E1 W1/W2): compare full `DirtySet` values, never only
`is_some()`.

Hand mutants (critical code, design "Critical code" row `E3a`): swap the stacks (`newest_touches`
reads the redo top, `undone_touches` the undo top); read the bottom instead of the top; return
`Some(DirtySet::default())` on an empty stack. Record each with its killing test and failure.

Plan: RED for AC1-AC6 in `crates/gafira-edit` tests, then the two methods, then the gates; notes
in a separate commit after the code.

Estimate: 90 authored lines, 126 at x1.4. Verify: V1.

Owner decisions touched: none (O2 and O5 belong to `E3b` and `E3d`).
