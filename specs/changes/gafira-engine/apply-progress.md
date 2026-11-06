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

## Notes — `E3a`

Done: AC1-AC7. `History::newest_touches` and `History::undone_touches` in
`crates/gafira-edit/src/history.rs` (doc comments cite P14); tests in
`crates/gafira-edit/tests/dirty_sets.rs`, each named `acN_...` with `// ARQ-30`. Code commit
`d1d6641`.

RED evidence (both methods stubbed to `None`, tests run, every assertion compares whole
`(newest_touches, undone_touches)` pairs of `DirtySet`s):

| AC | Test | Assertion before the code |
|---|---|---|
| AC1 | `ac1_after_a_changed_execute_the_newest_set_is_the_applied_one` | left `(None, None)`, right `(Some({pages: {PageId(1)}}), None)` |
| AC2 | `ac2_undo_reports_the_undone_entry_and_the_new_top` | left `(None, None)`, right `(Some({PageId(1)}), Some({PageId(2)}))` |
| AC3 | `ac3_redo_reports_the_redone_entry_and_the_next_redo_entry` | left `(None, None)`, right `(Some({PageId(1)}), Some({PageId(2)}))` |
| AC4 | `ac4_a_merged_gesture_reports_the_merged_entry` | left `(None, None)`, right `(Some({PageId(2)}), None)` |
| AC5 | `ac5_an_unchanged_execute_leaves_both_sets` | left `(None, None)`, right `(Some({PageId(1)}), Some({PageId(2)}))` |
| AC5 | `ac5_an_empty_history_reports_nothing` | passed against the `None` stub (see deviations); killed by mutant 3 |
| AC6 | `ac6_a_command_that_fails_to_apply_leaves_both_sets` | left `(None, None)`, right `(Some({PageId(1)}), Some({PageId(2)}))` |
| AC7 | gates at workspace scope | `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` finished with no warning; `cargo test --workspace --locked` 199 suites, 2529 passed, 0 failed |

AC1 uses `InsertPage`, whose set is empty before `apply` (XP4); AC6 uses an `InsertPage` with an
out-of-range index (`IndexOutOfRange { index: 9, len: 1 }`) over a history with both stacks full.

Hand mutants (applied, test failed, reverted; never committed):

| # | Mutant | Killed by | Failure |
|---|---|---|---|
| 1 | swap the stacks (`newest_touches` reads the redo top, `undone_touches` the undo top) | all but `ac5_an_empty_history_reports_nothing`; e.g. `ac1_...` | left `(None, Some({PageId(1)}))`, right `(Some({PageId(1)}), None)` |
| 2 | read the bottom (`first()`) instead of the top | `ac2_...`, `ac3_...`, `ac4_...` | `ac4`: left `(Some({PageId(1)}), None)`, right `(Some({PageId(2)}), None)`; `ac3`: undone left `{PageId(3)}`, right `{PageId(2)}` |
| 3 | `Some(DirtySet::default())` on an empty stack | `ac5_an_empty_history_reports_nothing` (and `ac1` to `ac4`) | left `(Some({}), Some({}))`, right `(None, None)` |

Deviations: the `None`-on-empty half of AC5 cannot fail against a stub that returns `None`; its
RED is mutant 3 instead. Measured lines (`git diff --shortstat cad7d0c..d1d6641`): 2 files, 171
insertions, 1 deletion (estimate 90, 126 at x1.4; the overrun is the test file, 153 lines).

## Slice card — `E3b` session, edits and the document version

Product IDs: 02 §5.1, §5.2 (1317), §5.4 (1357, 1359), §7.3; ARQ-30, ARQ-33, ARQ-102 (`ToMessage`
half only). Design: P12, P14, P15, P16 (the `E3b` parts), row `E3b`, "Critical code" row `E3b`.
Owner decision touched: O2 = (a), 2026-11-06 — `DocVersion` is an engine-owned counter; recorded,
not reopened. `crates/gafira-doc` is not touched.

Scope:

- `DocVersion(u64)`, `Copy + Ord`, owned by `Gafira`, 0 at construction, strictly increasing for
  the engine's lifetime, never reset (P15).
- `Session { history, path, read_only, clean_version, .. }` and `SessionCell =
  Arc<Mutex<Option<Session>>>` shared by `ProjectService` and `EditService` (P15).
- `EditService::{execute, undo, redo, begin_gesture, end_gesture}` over the session's `History`,
  passing `Clock::monotonic()` as `at` (§5.1).
- `ProjectService::new` (empty document via `Document::from_parts`, replaces the session, drops
  the old history) and `ProjectService::is_modified()` (`version != clean_version`).
- `Gafira::projects(&self) -> &ProjectService`, `Gafira::edit(&mut self) -> &mut EditService`.
- `EngineEvent::DocumentChanged { version: DocVersion, dirty: DirtySet }`.
- `EngineError` gains `NoSession`, `Command(CommandError)`, `InvalidDocument(ValidationError)`
  with `From`; becomes `#[non_exhaustive]`; `ToMessage for EngineError` per P12.

Non-goals: `open`, `save` and every file operation (`E3c`); `ReadOnly`, `NoPath`, `Io` (`E3c`);
autosave and `autosave_interval` (`E3d`); `UserError` (`E11`); `ToMessage` for `CommandError` or
`ValidationError` (SP3(b)); any change to `gafira-edit` beyond E3a, or to `gafira-doc`,
`gafira-render`, `gafira-render-cpu`.

Acceptance criteria:

| # | Criterion | Proof |
|---|---|---|
| AC1 | Every bump of the P15 table bumps the version by exactly one and publishes exactly one `DocumentChanged` with that version: `execute` → `Changed` (merged or not), `undo` → `Some`, `redo` → `Some`, a successful `new` | unit tests, one per row, asserting the exact version and the event count |
| AC2 | Nothing in the "never bumps" column bumps or publishes: `execute` → `Unchanged`/`Err`, `undo`/`redo` → `None`/`Err`, `begin_gesture`, `end_gesture` | unit tests asserting version unchanged and zero events drained |
| AC3 | The version is strictly increasing across a mixed sequence (edits, undo, redo, a second `new`) and a second `new` does not reset it | unit test over the whole sequence, comparing each event's version to the previous |
| AC4 | `DocumentChanged.dirty` is `newest_touches()` after `execute`/`redo` and `undone_touches()` after `undo` (P14); after `new` it lists every page and node of the new document | unit tests comparing whole `DirtySet` values |
| AC5 | ARQ-33 through the engine: a gesture of many merging steps leaves one history entry and `undo` returns to the state before the gesture | unit test (`begin_gesture`, N `SetProperty` steps, `end_gesture`, one `undo`) |
| AC6 | History boundary: after `new`, `undo` of an edit made before it is impossible (`None`), and the old history is gone | unit test |
| AC7 | With no session, every `EditService` call returns `EngineError::NoSession` and publishes nothing | unit test per method |
| AC8 | "Modified": false after `new`; true after an edit; still true after undoing back to the `new` content (conservative, P15) | unit test |
| AC9 | `ToMessage`: one key per `EngineError` variant in the area `error.engine.`, `Repository` and `Store` delegate to their own `to_message`, `Command` → `error.engine.command_failed`, `InvalidDocument` → `error.engine.invalid_document`, no `_` arm | fixture per variant asserting the key name (no catalog exists, P12) |
| AC10 | `Gafira` stays `Send + Sync` with the session mutex (XP7) | the existing compile-time assertion still compiles |
| AC11 | Gates at `--workspace` scope | gate output in the notes |

Every test asserts a value that can fail (E1 W1/W2): exact versions, exact event counts, whole
`DirtySet`s.

Hand mutants (critical code, row `E3b`): bump the version on an `Unchanged` execute; do not bump
on `undo`; reset the version to 0 on `new`; publish `newest_touches` after `undo` instead of
`undone_touches`; keep the old history across `new`; publish an event when there is no session;
map `Repository` to an engine key instead of delegating. Record each with its killing test and
failure.

Plan: RED for AC1-AC9, then `DocVersion` and `Session`, then `EditService`, then
`ProjectService::new`/`is_modified`, then the facade accessors and the event, then `ToMessage`,
then the gates; notes in a separate commit after the code. Fakes private to the engine (P16).

Estimate: 420 authored lines, 588 at x1.4. Verify: V1.

## Notes — `E3b`

Done: AC1-AC11. Code commits `5df8a7a` (refactor: the private port fakes move from `engine.rs`'s
test module to `src/fakes.rs`, `#[cfg(test)]`, P16), `dc28f3c` (`session.rs`, `edit.rs`,
`project.rs`, the version, the event field, the new `EngineError` variants) and `3bde884`
(`ToMessage for EngineError`). Tests are unit tests in `src/edit.rs`, `src/project.rs` and
`src/engine.rs`, named `acN_...` with a `// ARQ-30`, `// ARQ-33` or `// ARQ-102` line.

RED evidence. Round 1: the services had their shape with stub bodies (`Shared::changed` bumped and
published nothing, `new_document` installed a session only when none existed, `is_modified` returned
`false`, a missing session was `InvalidConfig { detail: "stub" }`, `to_message` returned
`error.engine.todo`); 15 tests failed, the edit tests mostly in the shared `new` precondition
(`the new`, left `0`, right `1`). Round 2: `Shared::changed` and `new_document` real, the edit side
still not publishing; each test then failed on its own assertion or on the append precondition.

| AC | Test | Assertion before the code |
|---|---|---|
| AC1 | `ac1_a_changed_execute_bumps_once_with_the_newest_set` | left `[]`, right `[DocumentChanged { version: DocVersion(2), dirty: {pages: {PageId(1)}} }]` |
| AC1 | `ac1_a_merged_execute_bumps_once_with_the_merged_set` | `the append` left `0`, right `1`; own assertion: probe X1 |
| AC1, AC4 | `ac1_ac4_undo_bumps_once_with_the_undone_set` | `the append` left `0`, right `1`; own assertion killed by mutants 2 and 4 |
| AC1, AC4 | `ac1_ac4_redo_bumps_once_with_the_redone_set` | `the append` left `0`, right `1`; own assertion killed by mutant 2 |
| AC1, AC4 | `ac1_ac4_new_bumps_once_and_lists_the_whole_empty_document` | round 1: left `[]`, right `[DocumentChanged { version: DocVersion(1), dirty: {} }]` |
| AC2 | `ac2_an_empty_undo_or_redo_neither_bumps_nor_publishes` | left `[]`, right `[DocumentChanged { version: DocVersion(2), .. }]` (the probe bump) |
| AC2 | `ac2_an_unchanged_execute_...`, `ac2_a_failed_execute_...`, `ac2_gestures_neither_bump_nor_publish` | `the append` left `0`, right `1`; own assertions: mutant 1, probes X3 and X2 |
| AC3 | `ac3_the_version_strictly_increases_across_a_second_new` | left `[2]`, right `[1, 2, 3, 4, 5, 6, 7, 8, 9]` |
| AC5 | `ac5_a_gesture_of_many_steps_is_one_entry` | `the append` left `0`, right `1`; own assertion killed by mutant 2 |
| AC6 | `ac6_new_drops_the_history_before_it` | round 1: `the append` left `0`, right `1`; own assertion killed by mutant 5 |
| AC7 | `ac7_without_a_session_every_edit_call_is_refused_and_publishes_nothing` | `call 0: Err(InvalidConfig { detail: "stub" })` |
| AC8 | `ac8_modified_after_an_edit_and_after_undoing_it` | `after an edit` (`is_modified()` was `false`) |
| AC9 | `ac9_every_engine_error_has_its_key_and_the_port_errors_delegate` | left `Message { key: "error.engine.todo" }`, right `Message { key: "error.engine.invalid_config" }` |
| AC10 | `ac5_the_facade_is_send_and_sync_and_keeps_the_ports_it_was_given` | the existing `assert_send_sync::<Gafira>()` still compiles with the session mutex |
| AC11 | gates at workspace scope | `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` no warning; `cargo test --workspace --locked` 199 suites, 2544 passed, 0 failed; `cargo deny check` advisories ok, bans ok, licenses ok, sources ok (the manifest gained a dev-dependency) |

Hand mutants (applied, test failed, reverted; never committed):

| # | Mutant | Killed by | Failure |
|---|---|---|---|
| 1 | bump on an `Unchanged` execute | `ac2_an_unchanged_execute_neither_bumps_nor_publishes` | `nothing published`: left `[DocumentChanged { version: DocVersion(3), dirty: {PageId(1)} }]`, right `[]` |
| 2 | no bump on `undo` | `ac1_ac4_undo_...`, `ac1_ac4_redo_...`, `ac3_...`, `ac5_...` | `undo`: left `[]`, right `[DocumentChanged { version: DocVersion(4), dirty: {PageId(2)} }]`; `ac3`: left `[1, 2, 3, 4, 5, 6, 7]`, right `[1, ..., 9]` |
| 3 | reset the version to 0 on `new_document` | `ac3_the_version_strictly_increases_across_a_second_new` | left `[1, 2, 3, 4, 5, 6, 1, 2, 3]`, right `[1, 2, 3, 4, 5, 6, 7, 8, 9]` |
| 4 | publish `newest_touches` after `undo` | `ac1_ac4_undo_bumps_once_with_the_undone_set` | left `dirty: {PageId(1)}`, right `dirty: {PageId(2)}` (both version 4) |
| 5 | keep the old history across `new_document` | `ac6_new_drops_the_history_before_it` | `assertion failed: engine.edit().undo().expect("an undo").is_none()` |
| 6 | publish an event with no session | `ac7_without_a_session_every_edit_call_is_refused_and_publishes_nothing` | left five `DocumentChanged` (versions 1 to 5, empty sets), right `[]` |
| 7 | map `Repository` to an engine key | `ac9_every_engine_error_has_its_key_and_the_port_errors_delegate` | left `Message { key: "error.engine.repository", args: [] }`, right `Message { key: "error.repository.too_new", args: [found: Count(3), supported: Count(1)] }` |

Extra probes, not on the card's list, run so that every test's own assertion was seen failing:

| # | Probe | Killed by | Failure |
|---|---|---|---|
| X1 | no publish once the history holds two entries | `ac1_a_merged_execute_bumps_once_with_the_merged_set` (and 8 more) | left `[]`, right `[DocumentChanged { version: DocVersion(3), .. }, DocumentChanged { version: DocVersion(4), .. }]` |
| X2 | `begin_gesture` publishes | `ac2_gestures_neither_bump_nor_publish`, `ac5_...` | left `[DocumentChanged { version: DocVersion(3), dirty: {} }]`, right `[]`; `ac5`: left `5`, right `4` |
| X3 | publish on a failed execute | `ac2_a_failed_execute_neither_bumps_nor_publishes` | left `[DocumentChanged { version: DocVersion(3), dirty: {} }]`, right `[]` |

Deviations:

- D1. The `new` of §5.4 and of the card is `ProjectService::new_document`: clippy's
  `new_ret_no_self` (default warn, `-D warnings`) rejects a method `new` that does not return
  `Self`, and the repo forbids `#[allow]`. Reversible rename; the doc comment cites line 1357.
- D2. `Session` holds `history` and `clean_version` only. `path`, `read_only` and the autosave state
  of P15 arrive with `E3c`/`E3d`, which read them; adding them now is dead code under `-D warnings`.
- D3. `is_modified()` is `false` with no session (nothing to lose); no AC or design row states it,
  and `ac8_...` asserts it.
- D4. `undo`/`redo` returning `Err` is not tested through the engine: it needs a compacted entry
  whose revert fails, and `History::new` keeps the 512 MiB budget (P15). The code path is the same
  `?` as `execute`'s `Err`, which `ac2_a_failed_execute_...` covers.
- D5. `new_document` builds a document with no pages (`Document::from_parts(1, Dpi::DEFAULT, [])`),
  so AC4's "every page and node" is proven for the empty document only (`dirty: {}`); a non-empty
  whole-document set first appears with `open` in `E3c`.
- D6. `crates/gafira-engine/Cargo.toml` gains `gafira-geom` as a dev-dependency (the tests build a
  `PageSize`); `Cargo.lock` gains one line. V5 reads the normal tree only, and `gafira-geom` is in
  the engine's row anyway. `crates/gafira-edit` is not touched.
- D7. `InvalidConfig` maps to `error.engine.invalid_config` with no argument (its detail is log
  prose, ARQ-102 forbids prose in a message); `NoSession` maps to `error.engine.no_session`.

Measured lines: `git diff --shortstat 9df403d..3bde884`: 10 files, 865 insertions, 152 deletions.
Without the fakes move (`5df8a7a`, 150/137, moved code): `git diff --shortstat 5df8a7a..3bde884`:
10 files, 717 insertions, 17 deletions, 734 authored lines, about 417 production and 317 test.
Estimate 420 (588 at x1.4): over the x1.4 figure and above the ~700 stop line; the overrun is the
tests (`edit.rs` 225 test lines).

## Slice card — `E3c` open, atomic save and the read-only case

Product IDs: 02 §5.3 (1343), §5.4 (1357), §7.1 (1557), §7.2 steps 1-3 (1568-1570); ARQ-50 (1613),
ARQ-55 (1618). Design: P15 (open, `clean_version`), P16 (`ReadOnly`, `NoPath`, `Io`, fakes), P18,
row `E3c`, "Critical code" row `E3c`. Owner answers, 2026-11-06, already in 02: SP6 (the
repository's `save` leaves the file synced before returning; the engine renames and syncs the
folder), SP7 (a read-only session edits in memory, `save` refuses before any file operation), the
temporary only needs a unique name in the target's folder (engine counter, P18). Follow-up taken
here: E3b-W3 (the whole-document dirty set of `open` on a non-empty document). E3b-S1: `Session`
gains `path`, `read_only` and the project key now.

Scope:

- `ProjectService::open(path)`: `repository.load`, `History::new` (`ValidationError` →
  `InvalidDocument`), swap only on success (P15, P18); bumps the version once and publishes one
  `DocumentChanged` listing every page and node of the loaded document; `read_only` from
  `LoadedProject`.
- `ProjectService::save()`: `NoSession`, then `ReadOnly` (before any port call), then `NoPath`;
  snapshot and version under the lock, lock released; `repository.save(tmp)` then
  `filesystem.replace(tmp, path)`; on failure a best-effort `remove(tmp)` whose error does not
  replace the first; `clean_version` set to the snapshot's version on success only, and only if
  the session is still the same.
- `EngineError` gains `ReadOnly`, `NoPath`, `Io(std::io::Error)` with `From<io::Error>`, each with
  its `error.engine.` key (P12 rule, fixture per variant).
- The `ProjectRepository::save` doc comment in `crates/gafira-ports/src/repository.rs` states the
  SP6 contract (doc only, no signature change).
- Private fakes (P16): an in-memory `FileSystem` that records each operation and can fail the n-th
  one, and an in-memory `ProjectRepository` that writes through that `FileSystem`.

Non-goals: `save_as`, `.gafira.bak`, `recover`, versions (`E4`); autosave (`E3d`); the file lock
(ARQ-57); `UserError` (`E11`); the UI's "Guardar" state (03); any change to `crates/gafira-doc`,
`crates/gafira-render`, `crates/gafira-render-cpu`; any port signature change.

Acceptance criteria:

| # | Criterion | Proof |
|---|---|---|
| AC1 | ARQ-50: for one save, a failure injected at every operation of the fake trace in turn leaves the target with the old bytes or the new bytes in full, never a mix or nothing | unit test looping over every fault index, comparing the target's bytes |
| AC2 | ARQ-50: `replace` runs only after `repository.save` returns `Ok`, and never after an error; the order is exactly `save(tmp)` then `replace(tmp, path)` | unit test on the recorded trace |
| AC3 | On any failure `remove(tmp)` is attempted, the first error is returned even if `remove` fails, and `clean_version` is unchanged | unit tests (failing `save`, failing `replace`, failing `remove` too) |
| AC4 | On success `is_modified()` becomes false at the snapshot's version; an edit after the snapshot keeps it modified | unit test |
| AC5 | ARQ-55 (a): a project loaded with `read_only` accepts `execute`, `undo`, `redo` and the version moves as in a normal session | unit test |
| AC6 | ARQ-55 (b): `save` on a read-only session returns `ReadOnly`, the fakes record no call, the target keeps its bytes | unit test asserting an empty trace |
| AC7 | `save` with no session → `NoSession`; a session from `new_document` (no path) → `NoPath`; both with an empty trace | unit tests |
| AC8 | A failed `open` (load error, invalid document) leaves the old session, its version, its history and its modified state as they were, and publishes nothing | unit tests |
| AC9 | A successful `open` bumps the version once and publishes one `DocumentChanged` whose `dirty` lists every page and node of a NON-empty loaded document; the old history is dropped (E3b-W3) | unit test comparing the whole `DirtySet` |
| AC10 | Two saves in a row use distinct temporary names in the target's folder | unit test on the trace |
| AC11 | `ToMessage` keys for `ReadOnly`, `NoPath`, `Io` (`error.engine.` area, no argument for `Io`) | fixture per variant |
| AC12 | The engine stays pure: every file operation through `FileSystem` or `ProjectRepository`, no `std::fs`, no threads, no `rand` | `git grep` in the notes plus the existing CI steps |
| AC13 | Gates at `--workspace` scope | gate output in the notes |

Every test asserts a value that can fail: exact traces, exact bytes, exact versions.

Hand mutants (critical code, row `E3c`): call `replace` even after `save` failed; `replace` before
`save`; skip `remove(tmp)` on failure; let the `remove` error replace the first error; set
`clean_version` before the write succeeds; check `read_only` after the first port call; swap the
session before `History::new` validates; reuse the same temporary name. Record each with its
killing test and failure.

Plan: fakes first (with their own tests), then RED for AC1-AC11, then `open`, then `save`, then
the errors and keys, then the port doc comment, then the gates; notes in a separate commit.

Estimate: 380 authored lines, 532 at x1.4. Verify: V1.

### Slice card amend — `E3c` split into `E3c1` and `E3c2` (owner, 2026-11-06)

Why: the `open` half with the fault-injecting fakes measured ~500 lines and the forecast for
`save` is ~300, ~800 in all against the ~700 ceiling; the owner chose to split.

- `E3c1` = `ProjectService::open` and the private fakes (in-memory `FileSystem` with trace and
  armed faults, in-memory `ProjectRepository` writing through it). Criteria: AC8, AC9, AC12 (for
  the code it adds), AC13. Hand mutant: session swapped before `History::new` validates. Commit
  `16280b5`.
- `E3c2` = `ProjectService::save`, `Session { path, read_only, project key }`, `EngineError::{
  ReadOnly, NoPath, Io}` and their keys, the `ProjectRepository::save` doc comment (SP6).
  Criteria: AC1-AC7, AC10, AC11, AC12, AC13. Hand mutants: the other seven of the `E3c` list.
  Read-only flag of `open` is kept from `E3c2` on (AC5, AC6).

Everything else in the `E3c` card holds for both halves.

## Notes — `E3c1`

Done: AC8, AC9, AC12 (for the code `E3c1` adds), AC13. Code commit `16280b5`
(`ProjectService::open`, a private `install` shared with `new_document`, `ProjectService::over`
now takes `&Ports`; the private fakes `MemoryFs`, `MemoryFile`, `FsOp`, `MemoryRepository`,
`RepoCall`, `engine_on`, `two_page_document`, `invalid_document` in `src/fakes.rs`). Tests are
unit tests in `src/project.rs` and one fakes test in `src/fakes.rs`.

RED evidence (`open` was first a stub returning `Ok(())`, the tests run against it):

| AC | Test | Assertion before the code |
|---|---|---|
| AC9 | `ac9_open_bumps_once_lists_the_whole_loaded_document_and_drops_the_old_history` | left `[]`, right `[DocumentChanged { version: DocVersion(3), dirty: DirtySet { pages: {PageId(1), PageId(4)}, nodes: {NodeId(2), NodeId(3), NodeId(5)} } }]` |
| AC8 | `ac8_an_open_that_fails_to_load_leaves_the_old_session` | `matches!(error, Err(EngineError::Repository(RepositoryError::Io(_))))` failed on `Ok(())` |
| AC8 | `ac8_an_open_of_an_invalid_document_leaves_the_old_session` | `matches!(error, Err(EngineError::InvalidDocument(_)))` failed on `Ok(())` |
| AC12 | purity grep | `git diff fb3c814..16280b5 -- crates/gafira-engine \| grep '^+' \| grep -cE "std::fs\|std::thread\|std::env\|rand\|thread::spawn"` → `0`; `git grep -nE "std::fs\|std::thread\|std::env\|rand::\|thread::spawn" -- crates/gafira-engine/src` → only the existing doc comment `engine.rs:96` ("`std::env` is banned here"); `open` reads only through `ProjectRepository::load` |
| AC13 | gates at workspace scope, at `16280b5` | `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` no warning; `cargo test --workspace --locked` 199 suites, 2548 passed, 0 failed; no manifest change, so no `cargo deny` |

AC9 runs on a NON-empty document (`two_page_document`: page 1 with nodes 2 and 3, page 4 with
node 5), which closes E3b-W3; it also asserts exactly one `RepoCall::Load(TARGET)`, the loaded
document equal to the fixture, clean after open and no undo across it. AC8 asserts, after the
failed `open`: nothing published, still modified, the old document (one page), its history (an
undo is `Some`), and the version unmoved (the undo publishes version 3 with the appended page).

Hand mutants (applied, test failed, reverted; never committed):

| # | Mutant | Killed by | Failure |
|---|---|---|---|
| 7 | swap the session (`*cell = None` under the lock) before `History::new` validates | `ac8_an_open_of_an_invalid_document_leaves_the_old_session` | panicked `still modified` |

The other seven hand mutants of the `E3c` list belong to `save` (`E3c2`).

Deviations:

- D1. `invalid_document()` (test-only, `src/fakes.rs`): every public constructor of `Document`
  validates, so the fake builds the invalid one with a test command that sets the DPI to 0
  through the `DocMut` it is given, clones `doc.as_doc()`, and returns `Err(NotApplied)`, so
  `History::preview` drops its copy and nothing is installed. `EditToken` is never named. The
  helper asserts the copy fails `validate()`.
- D2. `Session` does not gain `path`, `read_only` and the project key in `E3c1`: nothing reads
  them before `save`, and unread fields are dead code under `-D warnings`. They arrive with
  `E3c2` (the prepared change is kept as `stash@{0}` in the worktree, `session.rs` only). Until
  then `open` ignores `LoadedProject::read_only`.
- D3. The memory fakes are minimal where `E3c` does not need them: `MemoryFile` appends on every
  `write` and its `seek`, `create_dir_all` and `read_dir` return `Unsupported`; `open` and
  `exists` are not traced (the trace is the write side, P16). The `replace` hook for the
  snapshot-version test arrives with `E3c2`.

Measured lines (`git diff --shortstat fb3c814..16280b5`): 3 files, 496 insertions, 10 deletions.
Production: 31 insertions, 6 deletions (`project.rs` 29/5, `engine.rs` 2/1). Test: 465
insertions, 4 deletions (`fakes.rs` 353/4, `project.rs` tests 112/0). The split figures came
from `git diff --numstat` and the hunk headers of `project.rs` against its `#[cfg(test)]` line.

## Notes — `E3c2`

Done: AC1-AC7, AC10, AC11, AC12, AC13. Code commits `5297f6d` (`EngineError::{ReadOnly, NoPath,
Io}`, `From<io::Error>`, their keys), `72d34f6` (`ProjectService::save`, `Session { path,
read_only, project_key }`, `open` keeps `LoadedProject::read_only`, the `before_replace` hook of
`MemoryFs`), `adad43a` (the SP6 doc comment of `ProjectRepository::save`, no signature change).
Tests are unit tests in `src/project.rs` and `src/engine.rs`.

The stash `stash@{0}` from `E3c1` (`session.rs` only) was applied with `git stash pop` as the
starting point and kept as it was; it was stashed again for the length of `5297f6d` (so that
commit carries no unread field) and popped again before `72d34f6`. No stash is left.

RED evidence (AC11: the three variants first mapped to `error.engine.todo`; the rest: `save` first
a stub returning `Ok(())` and `open` passing `read_only: false`, the tests run against them):

| AC | Test | Assertion before the code |
|---|---|---|
| AC1 | `ac1_a_fault_at_every_operation_of_a_save_leaves_the_old_or_the_new_bytes_in_full` | trace left `[]`, right `[Create(tmp-0), Write(tmp-0, 1073), Write(tmp-0, 1073), SyncAll(tmp-0), Replace(tmp-0, TARGET)]` |
| AC2 | `ac2_replace_runs_only_after_the_repository_save_returned_ok` | calls left `[]`, right `[Save("/work/poster.gafira.tmp-0")]` |
| AC3 | `ac3_a_failed_save_removes_the_temporary_returns_the_first_error_and_stays_modified` | `expect_err("a failed save")` on `Ok(())` |
| AC4 | `ac4_a_save_cleans_at_the_snapshot_and_a_later_edit_keeps_it_modified` | `clean after a save` (`is_modified()` was `true`) |
| AC5 | `ac5_a_read_only_session_edits_undoes_and_redoes_like_any_other` | passed at first run: no edit path reads `read_only`, so the stub could not fail it. Its assertion was seen failing with probe X1 below |
| AC6 | `ac6_a_read_only_save_is_refused_before_any_port_call` | `matches!(error, Err(EngineError::ReadOnly))` failed on `Ok(())` |
| AC7 | `ac7_a_save_with_no_session_or_no_path_is_refused_before_any_port_call` | `matches!(error, Err(EngineError::NoSession))` failed on `Ok(())` |
| AC10 | `ac10_two_saves_in_a_row_use_distinct_temporaries_in_the_target_folder` | trace left `[]`, right the two save traces through `tmp-0` then `tmp-1` |
| AC11 | `ac11_read_only_no_path_and_io_have_engine_keys_without_arguments` | `ReadOnly`: left `Message { key: "error.engine.todo" }`, right `Message { key: "error.engine.read_only" }` |
| AC12 | purity grep | `git diff 2f99df8..adad43a -- crates/gafira-engine \| grep '^+' \| grep -nE "std::fs\|std::thread\|std::env\|rand\|thread::spawn"` → one line, the doc comment of `temporary_for` ("no `rand`, P1"); `git grep -nE "std::fs\|std::thread\|std::env\|rand::\|thread::spawn" -- crates/gafira-engine/src` → only `engine.rs:96` (existing doc comment). `save` writes only through `ProjectRepository::save`, `FileSystem::replace` and `FileSystem::remove`; the temporary name comes from an `AtomicU64` counter |
| AC13 | gates at workspace scope, at `72d34f6` and at `adad43a` | `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` no warning; `cargo test --workspace --locked` 199 suites, 2558 passed, 0 failed; at `5297f6d`: 199 suites, 2549 passed. No manifest change, so no `cargo deny` |

AC1/AC2 prove the repository writes through the fake `FileSystem` (E3c1-S2, P16): the exact
trace of a save is the repository's `Create`, two `Write`s and `SyncAll` on the temporary, then the
engine's `Replace`, and AC1 injects a fault at each of those five indices in turn (target: the old
bytes after each fault, the new bytes in full with none). AC2 also asserts, for a fault at each of
the repository's four operations, the exact trace up to the fault followed by `Remove(tmp-0)` and
no `Replace`. AC3 is table-driven: fault at the sync, at the `replace`, at the sync and the
`remove`, at the `replace` and the `remove`; it asserts the port and the text of the first error,
the exact trace ending in `Remove(tmp-0)`, whether the temporary is left behind, still modified and
nothing published. AC4's second half edits through a second `EditService` over the same `Shared`
from the `before_replace` hook, between the snapshot (version 2) and the `replace`; the save then
leaves the session modified. One more test, `a_save_that_finishes_after_the_session_was_replaced_
leaves_the_new_one_alone`, proves the `project_key` rule of P18 (the hook swaps in another session
clean at version 3; it is still clean after the save).

Hand mutants (applied, test failed, reverted; never committed):

| # | Mutant | Killed by | Failure |
|---|---|---|---|
| 1 | `replace` even after the repository's `save` failed | `ac1_...`, `ac2_...`, `ac3_...` | `ac2` fault at 0: left `[Create(tmp-0), Replace(tmp-0, TARGET), Remove(tmp-0)]`, right `[Create(tmp-0), Remove(tmp-0)]`; `ac1` fault at 1: target left `Some([])`, right the old bytes |
| 2 | `replace` before `save` | `ac1_...`, `ac2_...`, `ac3_...`, `ac4_...`, `ac10_...`, the session-replaced test | `a save: Io(Custom { kind: NotFound, error: "/work/poster.gafira.tmp-0" })`; `ac3` `[3]`: left `("filesystem", "/work/poster.gafira.tmp-0")`, right `("repository", "fault at 3")` |
| 3 | skip `remove(tmp)` on failure | `ac2_...`, `ac3_...` | `ac2` fault at 0: left `[Create(tmp-0)]`, right `[Create(tmp-0), Remove(tmp-0)]` |
| 4 | the `remove` error replaces the first error | `ac3_...` | `[3, 4]`: left `("filesystem", "fault at 4")`, right `("repository", "fault at 3")` |
| 5 | `clean_version` set before the write succeeds | `ac3_...` | `[3]: still modified` |
| 6 | `read_only` checked after the first port call | `ac6_...` | `no file operation`: left `[Create(tmp-0), Write(tmp-0, 1073), Write(tmp-0, 1073), SyncAll(tmp-0)]`, right `[]` |
| 8 | the same temporary name for every save | `ac10_...` | left: the second save through `tmp-0`, right: through `tmp-1` |

Mutant 7 (session swapped before `History::new` validates) was `E3c1`'s.

Extra probes, not on the card's list, run so that every new test's own assertion was seen failing:

| # | Probe | Killed by | Failure |
|---|---|---|---|
| X1 | `EditService` refuses every edit of a read-only session with `ReadOnly` | `ac5_a_read_only_session_edits_undoes_and_redoes_like_any_other` | `append_page`: left `None`, right `Some(Changed)` |
| X2 | `clean_version` set on success whatever the session (no `project_key` check) | `a_save_that_finishes_after_the_session_was_replaced_leaves_the_new_one_alone` | `the new session stays clean at version 3` |
| X3 | `clean_version` set to the current version instead of the snapshot's | `ac4_a_save_cleans_at_the_snapshot_and_a_later_edit_keeps_it_modified` | `clean at 2, the document at 3` |

Deviations:

- D1. `project_key` is minted at `new` and `open` as `<clock.now() seconds>-<engine counter>`, as
  P13 states; `E3c2` reads it only to compare (P18), so `ProjectService` now holds the `Clock`
  port. Its format is first read by `E3d` (the recovery folder) and is not tested here.
- D2. The temporary is the target's whole path with `.tmp-<n>` appended (`/work/poster.gafira.tmp-0`),
  which is P18's `<file name>.tmp-<n>` in the target's folder. The counter starts at 0 and is
  separate from the session counter, so the n-th save of an engine uses `tmp-<n-1>`.
- D3. `save` never bumps the version and publishes nothing (P15 table); `ac3_...` and `ac4_...`
  assert it. A successful `save` does not clear any autosave state: there is none until `E3d`.
- D4. AC5 could not be RED against the stub (nothing on the edit path reads `read_only`); probe X1
  shows its assertion can fail.
- D5. `Gafira::projects`'s doc comment now lists `open` and `save` (doc only, `engine.rs`).

Measured lines (`git diff --shortstat 2f99df8..adad43a`): 5 files, 408 insertions, 17 deletions.
Production: 141 insertions, 14 deletions (`project.rs` 100/7, `engine.rs` 24/3, `session.rs`
11/3, `gafira-ports/src/repository.rs` 6/1, doc only). Test: 267 insertions, 3 deletions
(`project.rs` tests 237/3, `engine.rs` tests 16/0, `fakes.rs` 14/0, test-only module). The split
figures came from `git diff --numstat` and the hunk headers of `project.rs` and `engine.rs`
against their `#[cfg(test)]` lines. Forecast ~300; 425 authored lines, under the ~700 stop line.

## Slice card — `E3d` autosave over a snapshot

Product IDs: 02 §7.2 "Autoguardado rotativo" (1573); ARQ-51 (1614) bounded as P13 states, not the
kill test. Design: P13, P16 (`autosave_interval`), P15 ("autosave pending", what never clears
"modified"), row `E3d`, "Critical code" row `E3d`. Owner decision touched: **O5 = (a)**,
2026-11-06 — until a journal port exists, each slot is written as a whole project through
`ProjectRepository::save`; "JSON only" and the 200 ms figure of §7.2 are **not met and not
claimed** here; the journal method in `gafira-ports` is a release prerequisite. SP5 (a real
project id) blocks `E4`, not this delivery: `project_key` from `E3c2` names the folder.

Scope:

- `EngineConfig::autosave_interval: Duration` (default 10 s) and
  `with_autosave_interval(self, Duration) -> Result<Self, EngineError>`: 5 s to 15 s inclusive,
  `InvalidConfig` otherwise.
- `pending_since: Option<Duration>` in the session: set (monotonic) by the first edit, undo or redo
  bump after the last snapshot was taken; not set by `new_document` or `open`; cleared by taking a
  job and by a successful `save`.
- `ProjectService::autosave_due() -> Option<AutosaveJob>`: `Some` exactly when a session is open,
  `pending_since` is set, no job is in flight, and `now - pending_since >= interval`.
- `AutosaveJob { snapshot, version, slot, folder, project_key, repository, filesystem }`, `Send`;
  `run(self) -> AutosaveOutcome`: `create_dir_all(folder)`, `repository.save(tmp)`,
  `filesystem.replace(tmp, slot path)`, best-effort `remove(tmp)` on failure. Path
  `data_dir/recovery/<project_key>/slot-<n>.gafira-journal`, `n` in `0, 1, 2`, temporary
  `slot-<n>.gafira-journal.tmp` in the same folder.
- `ProjectService::autosave_finished(outcome)`: success → slot advances `0 → 1 → 2 → 0`, in-flight
  cleared; failure → slot does not advance, `pending_since = now`; an outcome whose `project_key`
  is not the current session's is ignored.

Non-goals: the journal format, the BLAKE3 checksum, `RecoveryAvailable`, `recover`, slot lookup by
project (`E4`, SP5); the file lock (ARQ-57); a thread or timer in the engine (P1, P8: the host
pumps); `SaveState` events; the kill test of §6.7.

Acceptance criteria:

| # | Criterion | Proof |
|---|---|---|
| AC1 | The interval accepts exactly 5 s and 15 s, rejects 4.999 s and 15.001 s (one step outside each), default 10 s | unit tests on `with_autosave_interval` |
| AC2 | Due at exactly `interval` after the first pending change, not one step before | unit test with the fake `Clock` |
| AC3 | No job without changes: after `new_document`, after `open`, after a successful `save`, and with no session, `autosave_due()` is `None` at any time | unit tests |
| AC4 | The snapshot is the document at job creation: an edit made after `autosave_due()` and before `run` is not in the written slot | unit test reading the slot back through the fake repository |
| AC5 | A change made while a job is in flight stays pending: no second job while in flight; after `autosave_finished(Ok)`, the next job comes one interval after that change | unit test |
| AC6 | Slots rotate `0 → 1 → 2 → 0` on success only; a failed job keeps the slot and the next job writes the same slot | unit test over four successful and one failed job, asserting slot paths from the trace |
| AC7 | After a failure the retry is one interval later, never on the next tick | unit test: `autosave_due()` is `None` just before `now + interval`, `Some` at it |
| AC8 | A stale outcome (session replaced by `new_document` or `open` meanwhile) is ignored: the new session's slot, in-flight mark and pending state are untouched | unit test |
| AC9 | An autosave never clears "modified" (`is_modified()` stays true after a successful job) and does not bump the version or publish | unit test |
| AC10 | `run` writes through the ports only: `create_dir_all` of the slot folder, `save` to the temporary in that folder, `replace` onto the slot; on a failure at any operation the slot holds its previous bytes or the new bytes in full, and `remove(tmp)` is attempted | unit test with the fault-injecting fakes of `E3c1` |
| AC11 | A read-only session still autosaves (slots are not the user's file, P18) | unit test |
| AC12 | `AutosaveJob` is `Send`; the engine spawns nothing, every instant from `Clock` | compile-time assertion and the purity grep |
| AC13 | Gates at `--workspace` scope | gate output in the notes |

Every test asserts a value that can fail: exact instants, exact slot paths, exact bytes.

Hand mutants (critical code, row `E3d`): `>` instead of `>=` at the due boundary; accept 4 s or
16 s; set `pending_since` on `open`/`new_document`; take the snapshot at `run` instead of at
creation; clear `pending_since` on `autosave_finished` instead of when the job is taken; advance
the slot on failure; retry on the next tick after a failure; apply a stale outcome; clear
"modified" on autosave. Record each with its killing test and failure.

Plan: RED for AC1-AC11, then the interval, then the pending rule, then `autosave_due` and the job,
then `run`, then `autosave_finished`, then the gates; notes in a separate commit after the code.

Estimate: 360 authored lines, 504 at x1.4. Verify: V1.

## Notes — `E3d`

Done: AC1-AC13. Code commits `861e0f5` (`EngineConfig::autosave_interval`, default 10 s, and
`with_autosave_interval`, 5 s to 15 s inclusive) and `4cb17f8` (`ProjectService::{autosave_due,
autosave_finished}`, `AutosaveJob`, `AutosaveOutcome`, `Session::autosave` with `pending_since`,
`in_flight` and `slot`, the pending mark on every edit, undo and redo bump, `save` clearing it; in
the private fakes `FsOp::CreateDirAll`, `ManualClock` and `engine_with_clock`). Tests are unit
tests in `src/autosave.rs` (AC2-AC12) and `src/engine.rs` (AC1). Owner decision O5 = (a) holds: a
slot is a whole project written by `ProjectRepository::save`; "JSON only" and the 200 ms figure of
§7.2 are not met and not claimed. The folder is `data_dir/recovery/<project_key>` with the key of
`E3c2` (SP5 stays open for `E4`).

RED evidence (AC1: `with_autosave_interval` first a stub returning `Ok(self)` and the default
`Duration::ZERO`; AC2-AC11: `autosave_due` first returned `None`, `autosave_finished` did nothing,
`run` returned `Ok` without writing, no edit set `pending_since`. A second RED stage, with
`autosave_due` and the pending rule in place but `run` and `autosave_finished` still stubs, is
recorded where it applies):

| AC | Test | Assertion before the code |
|---|---|---|
| AC1 | `e3d_ac1_the_autosave_interval_is_5_to_15_seconds_inclusive_and_10_by_default` | left `0ns`, right `10s`; with the default in place, `4.999s accepted` |
| AC2 | `ac2_due_at_exactly_the_interval_after_the_first_pending_change` | `due at the interval` (`expect` on `None`) |
| AC3 | `ac3_no_job_without_changes` | `a change after the save is due` (the control at the end); second stage: `after a save` |
| AC3 | `ac3_a_save_leaves_pending_a_change_made_while_it_wrote` | `the change made during the save` |
| AC4 | `ac4_the_slot_holds_the_document_as_it_was_when_the_job_was_made` | `a job`; second stage: `the slot: Io(Custom { kind: NotFound, error: "/data/gafira/recovery/1700000000-0/slot-0.gafira-journal" })` |
| AC5 | `ac5_a_change_during_a_job_stays_pending_and_comes_one_interval_after_it` | `the first job`; second stage: `the change during it` |
| AC6 | `ac6_slots_rotate_on_success_only_and_a_failed_job_writes_its_slot_again` | `a job`; second stage: `a replace` |
| AC7 | `ac7_a_failed_job_is_retried_one_interval_later_never_on_the_next_tick` | `a job`; second stage: `the retry` |
| AC8 | `ac8_an_outcome_of_a_replaced_session_is_ignored` | `session 0's job`; second stage: `pending since T0 + 31` |
| AC9 | `ac9_an_autosave_never_clears_modified_never_bumps_and_publishes_nothing` | `a job` |
| AC10 | `ac10_a_fault_at_every_operation_of_a_run_keeps_the_old_slot_and_removes_the_temporary` | `a job`; second stage: trace left `[]`, right `[CreateDirAll(".../1700000000-0"), Create(".../slot-0.gafira-journal.tmp"), Write(.., 1073), Write(.., 1073), SyncAll(..), Replace(".../slot-0.gafira-journal.tmp", ".../slot-0.gafira-journal")]` |
| AC11 | `ac11_a_read_only_session_still_autosaves` | `a job`; second stage: `the slot` left `None`, right the document's bytes |
| AC12 | `ac12_a_job_and_its_outcome_are_send` plus the purity grep | compile-time `assert_send::<AutosaveJob>()` and `::<AutosaveOutcome>()`. `git diff 34837d3..4cb17f8 -- crates/gafira-engine \| grep '^+' \| grep -nE "std::fs\|std::thread\|std::env\|rand\|thread::spawn\|EditToken\|Instant\|SystemTime"` → nothing; `git grep -nE "std::fs\|std::thread\|std::env\|rand::\|thread::spawn\|EditToken" -- crates/gafira-engine/src` → only the existing doc comment `engine.rs:103`. Every instant is `Clock::monotonic()` (`autosave_due`, `autosave_finished`, `EditService`); the job writes only through `FileSystem::create_dir_all`, `ProjectRepository::save`, `FileSystem::replace` and `FileSystem::remove` |
| AC13 | gates at workspace scope, at `861e0f5` and at `4cb17f8` | `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` no warning; `cargo test --workspace --locked` 199 suites, 2571 passed, 0 failed (at `861e0f5`: 2559 passed). No manifest change, so no `cargo deny` |

Every test asserts exact values: instants one nanosecond before and at the boundary, slot paths
`/data/gafira/recovery/1700000000-<session>/slot-<n>.gafira-journal` from the trace (the key's
format of P13 is read here for the first time: `ManualClock::NOW` seconds, then the session
counter), the whole trace of a run, the slot's bytes and the versions. AC6 runs five jobs, the
third failing at its `replace`, and asserts the slots `0, 1, 2, 2, 0` from the `Replace` of each
trace and a `Remove` of `slot-2.gafira-journal.tmp` after the failure. AC8 leaves a succeeding job
of session 0 (`open`) and a failing one of session 1 (`new_document`) and hands both in while
session 2 (`open`) has a job in flight: session 2 is still in flight, then its next job is due at
exactly one interval after its own pending change and writes `.../1700000000-2/slot-1...`. AC10
puts other bytes in slot 0 first, injects a fault at each of the six operations in turn, and asserts
the exact trace up to the fault followed by `Remove(tmp)` and the slot's previous bytes.

Hand mutants (applied, test failed, reverted; never committed):

| # | Mutant | Killed by | Failure |
|---|---|---|---|
| 1 | `>` instead of `>=` at the due boundary (`< interval` became `<= interval`) | `ac2_...` and every test that takes a job at exactly the interval (11 of 12) | `ac2`: `due at the interval` |
| 2 | accept 4 s or 16 s (range `4..=16`, then `5..=16`) | `e3d_ac1_...` | `4.999s accepted`; `15.001s accepted` |
| 3 | set `pending_since` on `open`/`new_document` (in `install`) | `ac3_no_job_without_changes` | `after new_document` |
| 4 | snapshot taken at `run` instead of at creation (the job read the live session) | `ac4_...` | `three pages`: left `Document { next_id: 8, .. }`, right `Document { next_id: 7, .. }` |
| 5 | clear `pending_since` on `autosave_finished` instead of when the job is taken | `ac5_...`, `ac8_...` | `the change during it`; `pending since T0 + 31` |
| 6 | advance the slot on failure | `ac6_...`, `ac7_...` | `ac6`: left `[slot-0, slot-1, slot-2, slot-0, slot-1]`, right `[slot-0, slot-1, slot-2, slot-2, slot-0]`; `ac7` `the same slot`: left `slot-1`, right `slot-0` |
| 7 | retry on the next tick after a failure (`pending_since = now - interval`) | `ac7_...` | `the next tick` |
| 8 | apply a stale outcome (no `project_key` check) | `ac8_...` | `pending since T0 + 31` |
| 9 | clear "modified" on autosave (`clean_version` set on success) | `ac9_...` | `still modified` |

Extra probes, not on the card's list:

| # | Probe | Killed by | Failure |
|---|---|---|---|
| X1 | a successful `save` clears `pending_since` whatever was edited while it wrote | `ac3_a_save_leaves_pending_a_change_made_while_it_wrote` | `the change made during the save` |
| X2 | every bump resets `pending_since` (not only the first) | `ac2_...` | `due at the interval` |

Deviations:

- D1. A successful `save` clears `pending_since` only when no change arrived between its snapshot
  and its end (`version` still the snapshot's). P13 says "a successful `save` clears
  `pending_since`: the content is on disk"; a change made while the file was written is not on
  disk, and clearing it would leave it out of every slot until the next edit, past the ARQ-51
  window. Same rule as `clean_version` in `E3c2`. Test `ac3_a_save_leaves_pending_...`, probe X1.
- D2. On failure `pending_since` becomes `now` even when a change made during the job had set it
  earlier, as P13 states; that change is retried one interval after the failure.
- D3. `AutosaveJob` exposes `version()` (asserted by AC2, AC3, AC5) and keeps its fields
  `pub(crate)`; the slot path is private. `AutosaveOutcome` exposes `error()` for the host's log.
  `autosave_finished` returns nothing.
- D4. `ProjectService::over` takes `&EngineConfig` for the interval and `data_dir`;
  `ProjectService::shared` is `pub(crate)` so a test outside `project.rs` can edit from the
  `before_replace` hook.
- D5. The memory fakes now trace `create_dir_all` (`FsOp::CreateDirAll`); no earlier test calls it,
  so no earlier trace changed. `ManualClock::now` is fixed at `UnixTime(1_700_000_000)`.

Measured lines (`git diff --shortstat 34837d3..4cb17f8`): 7 files, 691 insertions, 21 deletions,
712 authored lines. Production: 246 insertions, 17 deletions (`autosave.rs` 99/0, `project.rs`
77/7, `engine.rs` 31/2, `edit.rs` 20/5, `session.rs` 17/3, `lib.rs` 2/0). Test: 445 insertions, 4
deletions (`autosave.rs` tests 376/0, `engine.rs` tests 25/0, `fakes.rs` 43/4, test-only module,
`project.rs` tests 1/0). The split came from `git diff --numstat` and the hunk headers against each
file's `#[cfg(test)]` line. Estimate 360 (504 at x1.4): over both and just over the ~700 stop line;
production is 263 lines, under the estimate, and the overrun is the tests (`autosave.rs` 376).

## E3d fix round 1 (verify round 1 FAIL, E3d-C1)

Test-only round; no production code changed.

Done:
- E3d-C1: `undo` and `redo` mark the session pending for autosave (design P13 "Timer"). Two tests
  in `autosave.rs`, both `// ARQ-51`: after the edit's job was taken and finished (nothing pending
  at T0 + 60 s), an undo (resp. a redo) alone at T0 + 70 s leaves `autosave_due()` `None` one
  nanosecond before T0 + 80 s and `Some` exactly at T0 + 80 s, with the version of that undo
  (resp. redo).
  - `an_undo_alone_after_the_last_snapshot_makes_a_job_due_one_interval_after_it`
  - `a_redo_alone_after_the_last_snapshot_makes_a_job_due_one_interval_after_it`

Proof in place of RED (the code was already right): the verifier's two probe mutants, applied to
`edit.rs` one at a time, run against the crate's tests, then reverted with `git checkout`
(`git diff` afterwards showed only the new tests).

| probe | mutant | killing test | failure |
|---|---|---|---|
| P-undo | in `undo`, `Self::bumped(..)` replaced by `shared.changed(dirty)` (bump, no pending mark) | `an_undo_alone_after_the_last_snapshot_...` | panic at `expect("one interval after the undo")`; 65 passed, 1 failed |
| P-redo | the same in `redo` | `a_redo_alone_after_the_last_snapshot_...` | panic at `expect("one interval after the redo")`; 65 passed, 1 failed |

Deviations: none.

Measured lines (`git diff --shortstat c2a1faa..e26538c`): 1 file changed, 43 insertions(+).

## Slice card — `E5` memory governor

Product IDs: 02 §6.7 (1515), §4.6 (690); ARQ-44 (1518), ARQ-142 (1225), ARQ-145 (1257); D-03.
Design: P7, P17, row `E5`, "Critical code" row `E5`. Same branch, after `E3d` (owner, 2026-11-06).
Owner requirements for this delivery (2026-11-06): test with a cache that reports `bytes()`
honestly but whose `trim` stops short of the budget, and with one that cannot shrink at all; say
what bounds the retries so the governor can never loop forever; hand mutants on the trim order,
the byte arithmetic with the boundary at exactly the budget, the zero budget, and the
`MemoryPressure` rule.

Scope:

- `Trimmable { fn bytes(&self) -> Bytes; fn trim(&mut self, budget: Bytes); }` in the engine, with
  wrappers over `&mut dyn ports::TileCache` and `&mut dyn ports::RenderBackend` (host bytes). The
  frame cache is wrapped by the composition root until `E8` (no `gafira-media` edge here).
- `CacheKind { Frames, Tiles, Backend }` and `MemoryGovernor::enforce(&mut self, caches: &mut
  [(CacheKind, &mut dyn Trimmable)]) -> GovernorOutcome { before, after, level }`.
- `MAX_TRIM_PASSES = 2`; `PressureLevel { Normal, Critical }`;
  `EngineEvent::MemoryPressure(PressureLevel)`, published only on a change of level.
- The governor is built from `EngineConfig::memory_limit`; `None` refuses (`InvalidConfig`, P5).

Non-goals: owning or storing the caches (the frame loop does); proxies or job deferral on
`Critical` (`E8`, `E7`, UI); the history and the document (they are not trimmable caches here);
VRAM enforcement (SP8); the process test "1 GB limit, +10 %" of §6.7; any change to
`crates/gafira-render`, `crates/gafira-render-cpu`, `crates/gafira-doc`, `crates/gafira-media` or a
port signature.

Acceptance criteria:

| # | Criterion | Proof |
|---|---|---|
| AC1 | Order: whatever order the caller passes, caches are trimmed frames, then tiles, then backend; equal kinds keep the caller's order (stable) | unit test recording the `trim` call sequence from a shuffled slice with two caches of one kind |
| AC2 | Exactly at the budget: `total == limit` calls no `trim`; `total == limit + 1` trims exactly 1 byte from the first cache in order | unit tests on the recorded calls and targets |
| AC3 | Arithmetic: the target passed to each cache is `bytes() - excess` (saturating); the running excess drops by what the cache actually freed; later caches are not trimmed once the excess is 0 | unit test with three honest caches and exact targets |
| AC4 | Saturation: totals near `u64::MAX` saturate instead of overflowing; a cache whose `bytes()` grows after `trim` frees 0, never a negative amount | unit tests |
| AC5 | Zero budget: `limit = 0` trims every cache to 0 in order and ends at `after == 0` with honest caches | unit test |
| AC6 | A lying cache (its `trim` stops short of the target): the governor moves on to the next cache and, if still above the limit, runs a second pass; never more than `MAX_TRIM_PASSES` passes | unit test counting passes and calls |
| AC7 | A cache that cannot shrink at all: `enforce` returns after at most `2 * caches.len()` `trim` calls, with `level == Critical`; repeated `enforce` calls never loop inside one call | unit test with an upper bound on calls |
| AC8 | `MemoryPressure`: `Critical` only when the fresh total is still above the limit after the bound; published once on `Normal → Critical`, not again while it stays `Critical`, once on `Critical → Normal`; never at start | unit test draining `events()` over a sequence of `enforce` calls |
| AC9 | Empty slice: no trim, `Normal`, no event; `memory_limit: None` refuses to build the governor | unit tests |
| AC10 | The second pass runs only when a fresh sum of every `bytes()` (not the running excess) is still above the limit | unit test where the running excess says 0 but a cache grew back |
| AC11 | Gates at `--workspace` scope; the engine stays pure | gate output and the purity grep in the notes |

Every test asserts a value that can fail: exact call sequences, exact targets, exact byte totals,
exact event lists.

Hand mutants (critical code, row `E5`): reverse or caller-order instead of the fixed order; `<`
instead of `<=` at the budget (trims at exactly the budget); target `excess` instead of
`bytes() - excess`; wrapping instead of saturating arithmetic; skip the zero-budget path (treat 0
as "no limit"); third pass or unbounded loop; second pass decided by the running excess; publish
`MemoryPressure` on every `enforce` instead of on a change; `Critical` before the bound is
exhausted. Record each with its killing test and failure.

Plan: test doubles first (honest, lying, unshrinkable, growing), RED for AC1-AC10, then the trait
and wrappers, then the order, then the arithmetic and the bound, then the pressure rule, then the
gates; notes in a separate commit after the code.

Estimate: 320 authored lines, 448 at x1.4. Verify: V1.

## Notes — `E5`

Done: AC1-AC11. Code commits `a0d9599` (`Trimmable`, `TileCacheTrim`, `BackendTrim`,
`CacheKind`, `PressureLevel`, `GovernorOutcome`, `MAX_TRIM_PASSES = 2`, `MemoryGovernor::{new,
limit, enforce}`: the order, the arithmetic, the boundary, the zero budget and the two-pass
bound), `bae33af` (`EngineEvent::MemoryPressure(PressureLevel)`, published through the bus only
on a change of level) and `fc26608` (refactor: a trim pass checks its excess after each cache, so
only the governor's `total <= limit` guard decides whether a pass runs; see hand mutant M3a).
Tests are unit tests in `src/governor.rs` and the variant list of `src/event.rs`.

The bound: `enforce` is a `for` over `0..MAX_TRIM_PASSES` with at most one `trim` per cache per
pass, so one call makes at most `2 * caches.len()` `trim` calls and returns; there is no `loop`,
no `while` and no wait. A cache that cannot shrink is tried again only on the host's next
`enforce` (P17). AC7 proves it with exact call lists over three consecutive calls; hand mutants
M7a (a third pass) and M7b (an unbounded `loop`) are killed.

RED evidence (stage 1: `enforce` returned `{ before: 0, after: 0, level: Normal }` and trimmed
nothing, `new` accepted a missing limit, both wrappers answered `Bytes(0)` and ignored `trim`;
stage 2, for AC8: the trimming in place, the variant added, nothing published):

| AC | Test | Assertion before the code |
|---|---|---|
| AC1 | `ac1_caches_are_trimmed_frames_tiles_backend_and_stable_within_a_kind` | left `[]`, right `[("frames-a", 0), ("frames-b", 0), ("tiles-a", 0), ("tiles-b", 0), ("backend", 0)]` |
| AC2 | `ac2_exactly_at_the_limit_trims_nothing_and_one_byte_above_trims_one_byte` | `limit 100`: left `GovernorOutcome { before: Bytes(0), after: Bytes(0), level: Normal }`, right `{ before: Bytes(100), after: Bytes(100), level: Normal }` (the `limit 99` row expects exactly `[("frames", 59)]`) |
| AC3 | `ac3_each_target_is_the_cache_bytes_minus_the_running_excess` | left `[]`, right `[("frames", 0), ("tiles", 20)]` |
| AC4 | `ac4_a_total_near_u64_max_saturates_instead_of_wrapping` | left `[]`, right `[("frames", 100), ("frames", 90)]` |
| AC4 | `ac4_a_cache_that_grows_after_its_trim_freed_nothing` | left `[]`, right `[("frames", 5), ("tiles", 5), ("frames", 10), ("tiles", 0)]` |
| AC5 | `ac5_a_zero_limit_trims_every_cache_to_zero` | left `[]`, right `[("frames", 0), ("tiles", 0), ("backend", 0)]` |
| AC6 | `ac6_a_cache_that_stops_short_is_finished_by_the_second_pass` | left `[]`, right `[("frames", 30), ("tiles", 0), ("frames", 50)]` |
| AC6 | `ac6_never_more_than_two_passes` | left `[]`, right `[("frames", 50), ("frames", 50)]` |
| AC7 | `ac7_caches_that_cannot_shrink_cost_two_trims_each_per_call` | `call 0`: left `[]`, right `[("frames", 70), ("tiles", 20), ("frames", 70), ("tiles", 20)]` |
| AC8 | `ac8_memory_pressure_is_published_only_on_a_change_of_level` | stage 2, `call 1`: left `[]`, right `[MemoryPressure(Critical)]` |
| AC9 | `ac9_a_config_without_a_memory_limit_refuses_the_governor` | `assertion failed: matches!(refused, Err(EngineError::InvalidConfig { .. }))` |
| AC9 | `ac9_an_empty_slice_trims_nothing_and_stays_normal` | passed against the stub (the stub's answer is the expected one); it kills M3b and M9 |
| AC10 | `ac10_a_cache_that_grew_back_gets_a_second_pass` | left `[]`, right `[("frames", 10), ("frames", 10)]` |
| scope | `scope_the_port_wrappers_report_host_bytes_and_forward_the_target` | left `GovernorOutcome { before: Bytes(0), after: Bytes(0), level: Normal }`, right `{ before: Bytes(80), after: Bytes(50), level: Critical }` |
| AC11 | gates and purity grep | at `a0d9599`, `bae33af` and `fc26608`: `cargo fmt --all --check` clean; `cargo clippy --workspace --all-targets --locked -- -D warnings` no warning; `cargo test --workspace --locked` 199 suites, 2587 passed, 0 failed (2586 at `a0d9599`). `cargo deny check` at `a0d9599` (manifest change): `advisories ok, bans ok, licenses ok, sources ok`. `git diff 69a7d8e..fc26608 -- crates/gafira-engine \| grep '^+' \| grep -nE "std::fs\|std::thread\|std::env\|rand\|thread::spawn\|Instant\|SystemTime\|std::time"` → nothing; `git grep -nE "std::fs\|std::thread\|std::env\|rand::\|thread::spawn" -- crates/gafira-engine/src` → only the existing doc comment `engine.rs:103` |

The doubles (owner's requirement): `Fake::honest` (frees down to the target), `Fake::lying` (honest
`bytes()`, `trim` frees at most a fixed step, so it stops short), `Fake::stuck` (frees nothing),
`Fake::scripted` (`bytes()` answers a fixed list of readings: a cache that grows after its trim or
grows back before the fresh sum). Every test asserts the exact `trim` sequence with names and
targets, the exact `GovernorOutcome`, and for AC8 and AC9 the exact event list drained from
`Gafira::events()` of an engine whose bus the governor publishes into.

Hand mutants (applied by a runner that restores the file in a `finally` and checks the restore;
built once per mutant, the test binary run under `ulimit -v 2000000` and `timeout 30`; `git diff`
empty before every commit):

| # | Mutant | Killed by | Failure |
|---|---|---|---|
| M1 | caller order (no sort) | `ac1`, `ac2`, `ac3`, `ac4` (saturation), `ac5`, `ac6` (stops short), `ac7`, `scope_...` | `ac2` `limit 99`: left `[("tiles", 39)]`, right `[("frames", 59)]` |
| M2 | reversed order (`Reverse(rank)`) | `ac1`, `ac2`, `ac3`, both `ac4`, `ac5`, `ac6` (stops short), `ac7`, `scope_...` | `ac1`: left `[("backend", 0), ("tiles-a", 0), ("tiles-b", 0), ("frames-a", 0), ("frames-b", 0)]` |
| M3a | `<` instead of `<=` in the pass guard (trims at exactly the budget) | `ac1`, `ac2`, `ac3`, `ac5` | `ac1`: a sixth trim `("frames-a", 0)` after the five expected. Before `fc26608` it survived as an equivalent mutant (the pass began with an `excess == 0` check that masked it) |
| M3b | `>=` instead of `>` in the level rule (`Critical` at exactly the budget) | ten tests, among them `ac2`, `ac8`, `ac9` (empty) | `ac10`: left `level: Critical`, right `level: Normal` |
| M4 | target `excess` instead of `bytes() - excess` | ten tests | `ac4` (grows): left `[("frames", 5), ("tiles", 5), ("frames", 5), ("tiles", 5)]`, right `[..., ("frames", 10), ("tiles", 0)]` |
| M5a | `wrapping_add` in the total | `ac4_a_total_near_u64_max_saturates_instead_of_wrapping` | left `[]`, right `[("frames", 100), ("frames", 90)]` |
| M5b | `wrapping_sub` for `freed` | `ac4_a_cache_that_grows_after_its_trim_freed_nothing` | left `[("frames", 5), ("frames", 5), ("tiles", 0)]`, right `[("frames", 5), ("tiles", 5), ("frames", 10), ("tiles", 0)]` |
| M5c | `wrapping_sub` for the target | `ac1`, `ac3`, `ac5`, `ac6` (stops short), `scope_...` | `ac1`: targets `18446744073709551584`, ... instead of `0` |
| M6 | zero budget treated as "no limit" | `ac1`, `ac5` | `ac1`: left `[]`, right the five trims to 0 |
| M7a | a third pass (`0..MAX_TRIM_PASSES + 1`) | `ac6_never_more_than_two_passes`, `ac7`, `scope_...` | left `[("frames", 50), ("frames", 50), ("frames", 50)]`, right two |
| M7b | unbounded `loop` | `ac6_never_more_than_two_passes` (and `ac7`) | the call never returns: `memory allocation of 805306368 bytes failed`, abort (rc -6) under the memory limit |
| M8 | second pass decided by the running excess | `ac10`, `ac4` (both) | `ac10`: left `[("frames", 10)]`, right `[("frames", 10), ("frames", 10)]` |
| M9 | `MemoryPressure` published on every `enforce` | `ac8`, `ac9` (empty) | `ac9`: left `[MemoryPressure(Normal)]`, right `[]` |
| M10 | `Critical` decided after the first pass | `ac6` (stops short), `ac10`, both `ac4` | `ac10`: left `level: Critical`, right `level: Normal` |

Deviations:

- D1. `MemoryGovernor::new(&EngineConfig, EventBus)`: the card fixes the build from
  `memory_limit` and publication through the bus, not the signature; the bus is the Facade's
  (`Gafira::bus().clone()`). `a0d9599` had `new(&EngineConfig)`; `bae33af` added the bus with the
  event. `limit()` is a read accessor (AC9).
- D2. The governor is not held by `Gafira` and has no Facade accessor: P17 says the frame loop
  owns the caches and calls `enforce`; §5.4's accessor list names none. The composition root
  builds it. Confirm in verify if a Facade accessor is wanted.
- D3. The wrapper names `TileCacheTrim` and `BackendTrim` are not fixed by the design.
  `BackendTrim::bytes` is `memory_usage().host` (P17, SP8 (b)); the scope test proves the device
  bytes are not counted.
- D4. Dev-dependency `gafira-raster` (the backend double must name `ImageBuf` in `read_back`);
  allowed by the layer table, normal dependencies unchanged, `cargo deny check` ok.
- D5. `fc26608` is a behaviour-preserving refactor made to kill M3a, not a new rule.
- D6. SP8 (a) stays provisional: `PressureLevel { Normal, Critical }` as P17 states.
- D7. The `EngineEvent` doc now lists the variants still planned "for the later deliveries, up to
  `E10`" instead of "`E3` to `E10`".

Measured (`git diff --shortstat 69a7d8e..fc26608`): 5 files changed, 694 insertions(+), 4
deletions(-). Production about 230 lines (`governor.rs` 219 above `#[cfg(test)]`, `event.rs` the
variant and its doc, `lib.rs` 5, manifest and lock 2), tests about 460 (`governor.rs` 458, the
variant list of `event.rs`). Over the estimate (320, 448 at x1.4) and close to the 700 mark: the
tests are 2/3 of it (the four doubles, the two port doubles of the scope test, and the AC8 table).
