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
