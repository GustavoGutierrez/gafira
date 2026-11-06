# Design: `gafira-engine` (layer 3)

The engine is the application layer of `specs/02-arquitectura-software.md` §2.1: `Session`, the
Facade services of §5.4, the event bus of §5.2, the export queue, autosave, recovery, versions,
the memory limits, job management and cancellation, and `LicenseService`. This design fixes the
delivery table over that Facade and lands the first delivery, `E1`, a scaffold: the crate, its
V5 row, its `clippy.toml` copy, the `Gafira` type with its ports injected, and the bus with a
no-op sink. No service is functional after `E1`.

Evidence: `AGENTS.md`, `specs/02-arquitectura-software.md` (§2.1 line 103, §2.2 lines 127-155,
§4.5, §4.6, §4.11, §4.14, §4.16, §5.1 lines 1299-1312, §5.2 lines 1314-1335, §5.4 lines
1351-1365, §5.5 lines 1369-1386, §6.3 line 1428, §6.4 lines 1436-1442, §6.7 line 1507, §7.1
line 1571, §7.2 step 3, §13, §14.1 line 2160, ARQ-05, ARQ-06, ARQ-20, ARQ-25, ARQ-30, ARQ-31,
ARQ-44, ARQ-101, ARQ-112, ARQ-138, ARQ-139, ARQ-145, ARQ-148), `specs/00-decisiones.md` (D-03,
D-08, D-12, D-36, D-55), `specs/01-producto-y-calidad.md` (RF-05, RF-65, RF-66, RNF-04),
`specs/04-...` (FD-57, FD-58, FD-64), `specs/05-...` (FV-12, FV-23, FV-24), ADR 0008, 0012,
0013, 0016, 0017, `.github/workflows/ci.yml` (V5, V12, the `forbid(unsafe_code)` step, the
ARQ-107 step), the root `Cargo.toml`, and the code of `crates/gafira-ports`
(`repository.rs:68`, `store.rs:53`, `system.rs:29`, `system.rs:75`, `system.rs:85`,
`license.rs:108`, `i18n.rs:140-156`), `crates/gafira-edit` (`dirty.rs:12`) and `crates/gafira-doc`
(`timeline/asset.rs:23`).

Precedent followed: `specs/history/2026-11-04-layer-3/design.md` (its scope tables, delivery
table, "why each mark holds", decision records and critical-code table) and its `SC1`, which is
the same shape as `E1` here.

## Quick path

1. Run `E1` alone: the crate, the V5 row, the `clippy.toml` copy, `Cargo.lock`, `Gafira`, `Ports`,
   `Registries`, `EngineConfig`, `EngineError`, the bus and its no-op sink.
2. Run `E2` next: the sink over a channel, `events()` and the coalescing of `JobProgress`.
3. Run `E3` and `E5` beside each other; everything else depends on at least one of them.
4. Verify each delivery against its own Slice card at depth V1; the gates and the hand mutants of
   the critical code run on every commit (ADR 0017).

## Dependency state

State at `develop` `2b949be`. Layers 0 to 2 are closed and archived, `gafira-ports` is archived,
and `gafira-render`/`gafira-media` are merged. `.odd/state.json` has no active outcome and no
change folder names the engine. The two active change folders, `gafira-render-cpu` and
`gafira-timeline`, do not touch it.

| Delivery | State | Blocked by |
|---|---|---|
| `E1` | Ready | Nothing |
| `E2` | Blocked | `E1` |
| `E3` | Blocked | `E1` |
| `E5` | Blocked | `E1` |
| `E4`, `E6`, `E7`, `E8`, `E9`, `E10` | Blocked | `E2`, `E3` and, for `E7`, `E5` |

## Scope of the cut

### In

| Area | Requirement IDs | Where |
|---|---|---|
| Crate, V5 row, `clippy.toml` copy, workspace dependency, `#![forbid(unsafe_code)]` | ARQ-01, ARQ-02, ARQ-03, ARQ-05, ARQ-06, ARQ-70, ARQ-71 | `E1` |
| `Gafira` with `Ports` and `Registries` injected; `EngineConfig`; `EngineError` | 02 §5.4 (1354-1356), §2.1 (103); ARQ-06 | `E1` |
| `EngineEvent`, `EventSink`, `NoopSink`, `EventBus` | 02 §5.2 (1314-1335); ARQ-68, ARQ-101 | `E1` |
| Sink over a channel, `events()`, `Waker`, 10 Hz coalescing of `JobProgress` | 02 §5.2 (1335), §13; ARQ-112 | `E2` |
| `EditService::{execute, undo, redo, begin_gesture, end_gesture}`, `Session`, `DocumentChanged` | 02 §5.1 (1299-1312), §5.4 (1359); ARQ-30, ARQ-33 | `E3` |
| `ProjectService::{new, open, save}`, autosave over a snapshot | 02 §5.4 (1357), §7.1 (1571), §7.2 | `E3` |
| `ProjectService::{save_as, recover, save_version, list_versions, restore_version}` | 02 §5.4 (1358), §7.2; ARQ-55 | `E4` |
| `MemoryGovernor`: `bytes()`/`trim(budget)` over the caches, `MemoryPressure` | 02 §6.7 (1507), §4.6 (690); ARQ-44, ARQ-145; D-03 | `E5` |
| `AssetService::{import, relocate}`, `LinksMissing` | 02 §5.4 (1360), §4.11; ARQ-149 | `E6` |
| `ExportService::{enqueue, cancel, list}`, the persisted queue and its state machine | 02 §6.4 (1436-1442), §5.4 (1361); FV-23; ARQ-148 | `E7` |
| `MediaService::{play, pause, seek, scrub, proxies}` | 02 §5.4 (1362), §6.5; FV-11 | `E8` |
| `LicenseService::{status, activate, validate, deactivate}`, `LicenseChanged` | 02 §5.4 (1363), §13, §4.14 (900-920); D-12, D-55 | `E9` |
| `SettingsService::{get, set, set_language}`, `LocaleChanged` | 02 §5.4 (1364), §14.1 (2160); ARQ-101; RF-65, RF-66, D-08 | `E10` |

### Out, and why

| Out | Reason | Requirement IDs |
|---|---|---|
| `gafira-ui`, `gafira-platform`, `gafira-i18n`, `gafira-theme`, `gafira-license`, `gafira-format` | Layer 4 adapters (02 §2.1). The engine reaches them through ports only (ARQ-06). | ARQ-06, ARQ-75, ARQ-100 |
| A channel dependency in `E1` | `E1` fixes the bus contract, not its transport. §5.2 (1335) fixes `crossbeam_channel` for the delivery to the UI; that dependency arrives in `E2` with `events()` and the coalescing rule, so `E1` adds no unused edge. | ARQ-68 |
| The persisted export queue in `E1` | It needs `ExportRequest` serialization, the state machine and the retry rule of §6.4 (1438-1441); it is `E7`. | FV-23, ARQ-148 |
| `SelectionService`, `IdentityService` | §5.3 (1342) names them but §5.4 gives them no accessor and `gafira-edit` has no `Selection` yet. Open question O3. | ARQ-30 |
| `MemoryGovernor` in `E1` | It trims caches that do not exist in the engine yet; `E5` owns it. | ARQ-44, ARQ-145 |
| The `UserError` mapping of ADR 0012 | `E1` has no user-facing failure with a message key. The mapping arrives with the first service that fails, in `E2`. | ARQ-102 |

### What is reversible

Reversible: the bus transport (`E1` fixes `EventSink`; swapping the channel for another transport
in `E2` touches no caller), the shape of `Ports` (adding a port is a field and a constructor
parameter), and every event variant (`E1` has no consumer of the enum yet, so the later deliveries
extend it freely).

**Not cheaply reversible**: the `clippy.toml` copy. V12 requires it to stay byte-identical to
`crates/gafira-geom/clippy.toml`, so the engine inherits the domain prohibitions, and relaxing one
of them for the engine means changing the canonical copy and every copy at once (P1).

## Delivery table

Forecast is authored lines, with the x1.4 planning figure in brackets. Each delivery passes
`cargo fmt --check`, its scoped `clippy`/`test` or the `--workspace` scope when it touches
`Cargo.toml`, `Cargo.lock` or `ci.yml` (ADR 0017).

| Delivery | Scope | Specs covered | Depends on | Mode | Est. lines |
|---|---|---|---|---|---|
| `E1` | Crate, `clippy.toml` copy, root `Cargo.toml`, `ci.yml` V5 row, `Cargo.lock`; `Gafira`, `Ports`, `Registries`, `EngineConfig`, `EngineError`; `EngineEvent`, `EventSink`, `NoopSink`, `EventBus` | 02 §2.1, §5.2, §5.4; ARQ-01, ARQ-02, ARQ-03, ARQ-05, ARQ-06, ARQ-70, ARQ-71 | None | serial | 230 (322) |
| `E2` | Sink over a channel, `events() -> &Receiver<EngineEvent>`, `Waker`, 10 Hz coalescing, `UserError` mapping | 02 §5.2 (1335), §13; ARQ-68, ARQ-101, ARQ-102, ARQ-112 | `E1` | serial | 320 (448) |
| `E3` | `Session`, `EditService`, `ProjectService::{new, open, save}`, autosave over a snapshot, `DocumentChanged` | 02 §5.1, §5.4, §7.1, §7.2; ARQ-30, ARQ-33, ARQ-55 | `E1` | serial | 480 (672) |
| `E5` | `MemoryGovernor` over `bytes()`/`trim(budget)`, the trim order, `MemoryPressure` | 02 §6.7, §4.6; ARQ-44, ARQ-145; D-03 | `E1` | parallel to `E3` | 300 (420) |
| `E4` | `save_as`, `recover`, the versions folder and its list/restore | 02 §5.4 (1358), §7.2; ARQ-50, ARQ-55 | `E3` | serial | 420 (588) |
| `E6` | `AssetService::{import, relocate}` through `ContentStore`, `LinksMissing` | 02 §5.4 (1360), §4.11; ARQ-149 | `E3` | serial | 380 (532) |
| `E7` | `ExportService::{enqueue, cancel, list}`, the persisted queue, the state machine, the retries | 02 §6.4, §5.4 (1361); FV-23, FV-24; ARQ-43, ARQ-148 | `E5` | serial | 500 (700) |
| `E8` | `MediaService::{play, pause, seek, scrub, proxies}` over `gafira-media` | 02 §5.4 (1362), §6.5; FV-11 | `E2` | serial | 400 (560) |
| `E9` | `LicenseService` and `LicenseChanged` | 02 §5.4 (1363), §13, §4.14; D-12, D-55 | `E3` | parallel to `E10` | 300 (420) |
| `E10` | `SettingsService`, `set_language`, `LocaleChanged` | 02 §5.4 (1364), §14; ARQ-101; D-08 | `E3` | parallel to `E9` | 320 (448) |

Total: 3 650 authored lines (x1.4: 5 110). The largest delivery is `E7` at 500 lines (x1.4: 700).

### Why each mark holds

| Mark | Ground |
|---|---|
| `E1` serial, first | It edits every shared file: the root `Cargo.toml`, `Cargo.lock`, the V5 table in `ci.yml`, and a new `crates/gafira-engine/`. It is the only delivery that can add the crate the rest import. |
| `E1` before everything | The V5 step and the V12 step fail on a crate that has a row without a `clippy.toml` copy, and the `forbid(unsafe_code)` step reads `crates/*/src/lib.rs`, so the crate cannot exist half-registered. |
| `E2` after `E1` | It implements the sink `E1` declares and needs `Bridge` the bus field; no other delivery needs the channel. |
| `E5` parallel to `E3` | `E5` owns `governor.rs` and `E3` owns `session.rs` and `edit.rs`; they meet at `ports::TileCache` and `ports::FileCache`, which already exist. Both edit `src/lib.rs`, so they are only parallel inside one worktree, never as two branches (`E1` fixes that: see "Shared files"). |
| `E4`, `E6`, `E9`, `E10` after `E3` | All four begin from a loaded document or an open session, which is what `E3` produces. |
| `E7` after `E5` | The queue asks the governor for a budget before each job and reports `JobProgress`; `E5` is what answers. |
| `E8` after `E2` | `MediaService` publishes playback events, and the sink that delivers them lands in `E2`. |

## Shared files

| File | Touched by | Resolution at merge |
|---|---|---|
| `crates/gafira-engine/src/lib.rs` | every delivery | One writer at a time: the module list is edited only by the delivery that opens the module, and streams run in one worktree rather than two branches. |
| `Cargo.toml` (root) | `E1` | `E1` adds `gafira-engine = { path = "crates/gafira-engine" }` to `[workspace.dependencies]` and nothing else. |
| `Cargo.lock` | `E1`, and any delivery that adds a dependency | Never hand-merge: take `develop`'s file and run `cargo check --workspace --all-targets` to regenerate, then the gates with `--locked`. |
| `.github/workflows/ci.yml` | `E1` | `E1` adds the V5 row and changes nothing else. |
| `crates/gafira-engine/clippy.toml` | `E1` | New file, byte-identical to `crates/gafira-geom/clippy.toml` (V12). |
| `.odd/state.json` | the coordinator | One entry for the change, updated at boundaries. |

## Decisions

### P1. The engine takes the `clippy.toml` copy and is pure in the ARQ-02 sense

Options: (a) the engine carries the copy and inherits the prohibitions; (b) it carries no copy and
no V5 row; (c) the copy exists but is relaxed for layer 3.

Decision: (a). Reason: (b) is not available — the V5 step requires a row for every crate with a
`clippy.toml`, and V12 requires the copy for every crate in the table, so a row implies the copy.
(c) forks the lint set and adds CI surface, and ADR 0016 already decided the same question for the
two other layer-3 crates. The engine therefore opens no file (`ports::FileSystem`), reads no clock
(`ports::Clock`), reads no environment and spawns no thread. Its consequence for the export queue
is P8.

Reversible: only with a new ADR and every copy changed in one commit (V12).

### P2. `E1` fixes the bus contract, not its transport

`EventSink` is a trait with one method, `publish(&self, event: EngineEvent)`; `NoopSink` is the
zero-effect implementation and the default of `Gafira`. §5.2 (1335) fixes `crossbeam_channel` for
the delivery to the UI, with a `Waker` for the repaint; that is `E2`, together with
`events() -> &Receiver<EngineEvent>` and the 10 Hz coalescing of `JobProgress`.

Reason: the transport is a dependency with a licence and a version, and `E1` has no consumer that
needs it. Keeping the contract above the transport also lets a test publish into a counting sink,
which is what proves the bus delegates.

Reversible: yes. The channel sink implements the trait `E1` declares, so `E2` adds a file and
changes no caller.

### P3. `EngineEvent` in `E1` carries only the variants whose payloads exist today

`DocumentChanged { dirty: DirtySet }`, `LinksMissing(Vec<AssetRef>)`,
`LicenseChanged(LicenseStatus)`, `LocaleChanged { preference, resolved }` and
`BackendChanged(BackendKind)` are in `E1`: every payload type exists in `gafira-edit`
(`dirty.rs:12`), `gafira-doc` (`timeline/asset.rs:23`) or `gafira-ports` (`render.rs:135`,
`license.rs:22`, `i18n.rs:21`, `i18n.rs:118`).

`SaveState`, `JobProgress`, `JobFinished`, `RecoveryAvailable`, `MemoryPressure`,
`StartupProgress` and `StartupFinished` are **Planned**, not omitted: each arrives with the delivery
that first produces it (`E2`, `E4`, `E5`, `E7`), because each carries vocabulary that delivery
defines (`JobId`, `JobResult`, `SaveState`, `RecoveryEntry`, the pressure level, `StartupStep`).

Deviation from §5.2 recorded: `DocumentChanged` carries no `version`, because `DocVersion` does not
exist in the repository (`grep` finds no type of that name in `crates/`). Open question O2 fixes
where it comes from. The enum has no consumer, so adding the field in `E3` is not a breaking change.

Reversible: yes, an enum without consumers is one delivery away from any shape.

### P4. The ports are injected as one `Ports` value, and the registries as one `Registries` value

`Gafira::new(ports, registries, cfg)` is the signature §5.4 fixes (1356). `Ports` holds the traits
the engine consumes; `Registries` holds the `gafira-ports` registries the composition root fills
before the engine starts (`FormatRegistry`, `CodecRegistry`, and the port registries as they are
needed). The engine never creates a port or a registry entry: ARQ-06 and ARQ-31 make the
composition root the only place that wires adapters.

`E1` injects `repository`, `store`, `filesystem`, `clock`, `notifier` and `license`. `translator`
arrives with `E10`, because `LocaleChanged` is the first event that needs a resolved locale rather
than a translated string.

Reversible: yes; a port is a field and a constructor parameter, and `Ports::new` takes what the
caller has.

### P5. `EngineConfig` takes the data directory and the limits; it reads neither RAM nor the environment

`EngineConfig { data_dir: PathBuf, memory_limit: Option<Bytes>, export_jobs: u8 }`. The defaults of
02 §6.7 (1465-1466: 50 % of physical RAM, 70 % of VRAM) and of §6.3 (1428: one export job at a
time) are the caller's to compute and pass, because reading them is `gafira-platform`'s job (layer
4, §2.1:115) and `std::env` is banned in this crate (P1). `memory_limit: None` means "no limit
declared", not "unlimited by default at runtime": `E5` refuses to start the governor without one.

`EngineConfig::new(data_dir)` rejects an empty path with `EngineError::InvalidConfig`. That
validation is the only behaviour `E1` has beyond composition, and it is the critical code of the
delivery.

Reversible: yes, the struct is not serialized and has no consumer outside the engine.

### P6. `EngineError` starts with the port errors `E1` can produce

`EngineError { InvalidConfig { detail: &'static str }, Repository(RepositoryError), Store(StoreError) }`.
The `UserError` mapping with message keys of ADR 0012 (ARQ-102) arrives in `E2`, when the first
user-facing failure exists; `Display` in `E1` is a plain sentence in English and is marked
**Planned** to become a key, which is why no caller may match on its text.

Reversible: yes, no persisted data and no message key depend on it.

### P7. The memory governor lives in the engine and the caches only report

`MemoryGovernor` is `E5`. ARQ-44 (1516) makes every trimmable cache expose `bytes()` and
`trim(budget)`; ARQ-145 (1257) makes the frame cache the first one trimmed; D-03 fixes the process
figures. The governor therefore owns the order and the arithmetic, and the caches own only their
own accounting. Putting the order inside a cache would give each crate a different idea of the
budget.

Reversible: the trim order is a governor decision, so a later delivery can move a cache in or out
without touching a crate's API.

### P8. The export queue is pumped by the host, not by threads of the engine

§6.4 (1436-1442) fixes the persisted queue, the state machine, the temp-file-and-rename write and
the two automatic retries on transient errors. §6.3 (1428) fixes one job at a time on the render
pool at `Background` priority. With P1 the engine creates no thread, so `ExportService` drives the
job on the caller's tick: `enqueue` records and persists the `ExportRequest`, the host asks
"is there work?" and advances one step, and cancellation travels as a `CancelToken` (ADR 0005)
with the 100 ms bound of ARQ-43. Through `ports::FileSystem`, never `std::fs`.

Reversible: yes for the API (the queue is data plus a state machine), not for the purity decision:
putting real threads inside the engine is O1.

### P9. One service per module; `Gafira` only composes

`lib.rs` declares one module per service and the Facade holds the ports, the registries, the config
and the bus, and forwards. §5.5 (1375) forbids the `Editor` god class; the accessors of §5.4
(1357-1365) are the whole public surface of `Gafira`, so a service never reaches another service's
state except through a port.

Reversible: yes, moving a method from the Facade to a service is not a breaking change while the
accessor stays.

### P10. No T4 in this change; hand mutants on the critical code of each delivery

Options: (a) `mutation: off`, hand mutants only; (b) `mutation: archive`, T4 at close.

Decision: (a), following the precedent of `O10` of the render change and the owner's instruction
for this block. Reason: the assemblies of this change are composition and state machines with
tests at every branch, and a whole-crate mutation run of `gafira-engine` would mostly mutate
`derive` output and doc comments. The cost is stated once: a wrong behaviour that the tests do not
exercise survives, and the hand mutants are the only defence.

Reversible: yes; T4 is a command over the same tree.

## Critical code

Hand mutants apply to all of these.

| Delivery | Critical | Not critical |
|---|---|---|
| `E1` | The `data_dir` validation of `EngineConfig`, the bus delegation to its sink, the `Ports` wiring (a missing port must not compile, not default) | Doc comments, `Display` text |
| `E2` | The coalescing window, the `Waker` call, the channel capacity choice, the ordering of the drain | The error mapping text |
| `E3` | The history ownership, the `DocumentChanged` payload, the autosave snapshot selection, the save-through-rename order | Doc comments |
| `E5` | The trim order, the byte arithmetic, the boundary at exactly the budget, the zero budget | Doc comments |
| `E4` | The recover case, the version naming, the restore order, the read-only case (ARQ-55) | Doc comments |
| `E6` | The embed-versus-link decision, the store hash, the relocate path rewrite | Doc comments |
| `E7` | The state machine, the persistence boundary, the retry count, the temp-file rename, the cancellation checkpoints | Doc comments |
| `E8` | The seek boundaries, the proxy request, the playback state transitions | Doc comments |
| `E9` | The `can_export` mapping of D-55, the offline grace boundary, the redaction of the key | Doc comments |
| `E10` | The preference resolution order (manual, system, fallback), the persisted key | Doc comments |

## Open questions (owner)

| # | Question | Options and tradeoffs | Recommendation |
|---|---|---|---|
| O1 | With P1 the engine cannot create threads, so the export queue must be pumped by the host. Is that acceptable for `E7`, or does the owner want an exception to the `clippy.toml` copy for `gafira-engine`? | (a) Host-pumped: the engine stays pure and V12 holds; the composition root owns the worker, and the queue is deterministic and testable without a clock. (b) Exception: an engine-owned pool is simpler to call, but it forks the lint set, needs a new ADR and V12 has to be taught the exception. (c) A pool injected as a trait in `gafira-ports`: the engine calls `spawn` on a trait it does not own, which keeps the lint and gives the engine a scheduler; it is one more port to design. | (a) for `E7`, and (c) when a delivery needs the engine to schedule work rather than advance it. |
| O2 | `EngineEvent::DocumentChanged` carries a document version in §5.2 (1317) and `DocVersion` exists in no crate. Where does it come from? | (a) A counter owned by the engine, bumped by `EditService` on every applied command: no crate outside the engine changes, and the version is a session fact. (b) `gafira-doc` gains `DocVersion` in the document: every consumer sees it, but it changes the persisted schema while D-36 keeps schema 1 open. (c) `gafira-edit`'s `History` exposes a revision counter: closest to the truth (the history is what advances), and a change in a layer-1 crate that is in flight nowhere. | (a) for `E3`; if a later delivery needs the version outside the engine, (c) is the migration. |
| O3 | `SelectionService` and `SelectionSnapshot` are named in §5.3 (1342) and §5.2 (1318) and `gafira-edit` has no `Selection` at all. | (a) The engine defines `SelectionSnapshot` (ids plus flags) and `gafira-edit` gains `Selection` later in its own change. (b) `gafira-edit` gains `Selection` first, and the engine only carries it. (c) Drop both from the Facade until a UI needs them. | (a) now, (b) as the follow-up change, since hit testing and the tools of the UI are what need the model. |
| O4 | `crossbeam-channel` becomes a direct dependency of the engine in `E2`. Its licence is MIT OR Apache-2.0, both allowed by `deny.toml`, and it is already in `Cargo.lock` through another crate. | (a) Take it: it is what §5.2 (1335) names, and using another channel would contradict the spec. (b) Use `std::sync::mpsc`: no new dependency, but no `select`, no bounded capacity choice and no coalescing window with a timeout. | (a), confirmed by `cargo deny check` on the three targets in `E2`. |

## Needs explorer

| # | Claim | Check | Blocks |
|---|---|---|---|
| XP1 | **Resolved in `E1`.** The V5 step and the V12 copy comparison accept a new row whose list is the whole transitive `gafira-*` tree (the step compares every transitive dependency, not only the direct ones); V5 and V12 exit 0 on the engine row. The ARQ-107 step does **not** leave layer 3 alone: it iterates **every** crate of the V5 table, applies its dependency check to all of them, and skips the `Locale` file search only for the names on its exemption list. That list holds two crates, `gafira-ports` and `gafira-engine`, because layer 3 resolves the locale by design (02 §14.1, ADR 0016). Membership is by name, so any future layer-3 crate that carries a `Locale` type must be added to that list deliberately rather than assumed exempt. | The V5, V12 and ARQ-107 shell snippets run locally against the new row: all three exit 0, and ARQ-107 prints `gafira-ports: layer 3, exempt from the Locale search` and `gafira-engine: layer 3, exempt from the Locale search`. | — |
| XP2 | ARQ-05 (76) requires `cargo public-api` in CI for `gafira-doc`, `gafira-ports` and `gafira-engine`, and the current `ci.yml` has no such step for any of the three. | `grep -n "public-api" .github/workflows/ci.yml` and `docs/adr/`. | `E1` records it; not fixed here |
| XP3 | A `Gafira` that owns `Arc<dyn Trait>` ports and an `Arc<dyn EventSink>` bus is `Send + Sync`, which the UI needs to hold it across frames. | A compile-time assertion in a test: `fn assert_send_sync<T: Send + Sync>() {}` applied to `Gafira`. | `E1` |

## Needs specifier

| # | ID | Request | Blocks |
|---|---|---|---|
| SP1 | 02 §5.4 (1356) | `new` returns `Result<Self, EngineError>` but §5.4 never says what can fail at construction. This design narrows it to `InvalidConfig` and the port errors; confirm or extend. | `E1` |
| SP2 | 02 §5.2 (1317-1328) | The payload types the spec names for the bus (`SaveState`, `JobResult`, `RecoveryEntry`, `Level`, `StartupStep`) belong to no crate. Confirm that the engine defines them, which P3 assumes. | `E2`, `E4`, `E5`, `E7` |

## Next

`E1` is the only delivery of this block: the crate, its registration in CI, and the Facade with the
bus. `E2` (the sink over a channel) and `E3` (the session and `EditService`) are what make the
engine do anything, and they are the next two in the queue.
