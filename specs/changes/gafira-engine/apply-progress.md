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
