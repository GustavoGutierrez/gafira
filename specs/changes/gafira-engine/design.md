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

**Amend of 2026-11-06** (after the `E2` round-1 verify, `verify-report.md` last section): P2 and
P6 are amended; P11 to P18 are new; O2 is answered by the owner; `E3` is split into `E3a` to
`E3d`; `E3` and `E5` move to one serial branch; the `UserError` mapping leaves `E2`. Additional
evidence read for the amend: 02 §5.2 (1331, 1335), §5.4 (1356), §6.7 (1515), §7.2 (1563-1570),
§7.3 (1590-1591), ARQ-30, ARQ-33, ARQ-44, ARQ-50, ARQ-51, ARQ-55, ARQ-68, ARQ-102, ARQ-142,
ARQ-145, D-03; and the code of `crates/gafira-engine` (`engine.rs:215-228` `Gafira::new`,
`bus.rs:27-33` `EventBus::channel`, `bus.rs:94-135` `publish`/`flush`, `event.rs:117`
`EventBus`), `crates/gafira-edit` (`history.rs:94` `execute`, `history.rs:219` `undo`,
`history.rs:251` `redo`, `command.rs:71` `touches`, `reparent.rs:63-69`, `error.rs:12`
`CommandError`), `crates/gafira-ports` (`repository.rs:68-82`, `system.rs:29-71`,
`render.rs:112-121` `TileCache`, `render.rs:168` `MemoryUsage`, `render.rs:315`
`RenderBackend::trim`, `common.rs:25` `Bytes`), `crates/gafira-doc` (`document.rs:18`,
`error.rs:70` `ValidationError`), `crates/gafira-media` (`cache.rs:20`, `cache.rs:321`,
`tests/support/mod.rs:45,172`) and `crates/gafira-testkit` (`src/ports/file_system.rs`,
`src/ports/clock.rs`).

## Quick path

1. Run `E1` alone: the crate, the V5 row, the `clippy.toml` copy, `Cargo.lock`, `Gafira`, `Ports`,
   `Registries`, `EngineConfig`, `EngineError`, the bus and its no-op sink. **Done** (merged).
2. Run `E2` next: the sink over a channel, `events()` and the coalescing of `JobProgress`. Round 1
   FAILed; its fix round follows P11 ("E2 fix round" below).
3. After `E2` passes, cut one branch `feat/gafira-engine-e3` from `E2`'s HEAD and run, in order,
   `E3a`, `E3b`, `E3c`, `E3d` and `E5` on it (owner, 2026-11-06). Nothing runs in parallel there.
4. Verify each delivery against its own Slice card at depth V1; the gates and the hand mutants of
   the critical code run on every commit (ADR 0017).

## Dependency state

State at `feat/gafira-engine-e2` `b5111f6` (base `develop` `0bc90a9`). `E1` is merged into
`develop`. `E2` has a round-1 FAIL (`verify-report.md`, E2-C1) and owes a fix round. The two
active change folders, `gafira-render-cpu` and `gafira-timeline`, do not touch the engine, and
neither has an open delivery in `crates/gafira-edit` (the timeline commands are deferred there,
`specs/changes/gafira-timeline/design.md:87`), which is what lets `E3a` edit that crate.

| Delivery | State | Blocked by |
|---|---|---|
| `E1` | Done | — |
| `E2` | Fix round owed | its own round-1 findings |
| `E3a`, `E3b`, `E3c`, `E3d` | Blocked | `E2` PASS; then each on the previous one, on `feat/gafira-engine-e3` |
| `E5` | Blocked | `E2` PASS and `E3d`, on the same branch (owner order) |
| `E4`, `E6`, `E9`, `E10` | Blocked | `E3d` |
| `E7` | Blocked | `E5` |
| `E8` | Blocked | `E2` |
| `E11` (`UserError`) | Blocked | `E3b` and SP3 |

## Scope of the cut

### In

| Area | Requirement IDs | Where |
|---|---|---|
| Crate, V5 row, `clippy.toml` copy, workspace dependency, `#![forbid(unsafe_code)]` | ARQ-01, ARQ-02, ARQ-03, ARQ-05, ARQ-06, ARQ-70, ARQ-71 | `E1` |
| `Gafira` with `Ports` and `Registries` injected; `EngineConfig`; `EngineError` | 02 §5.4 (1354-1356), §2.1 (103); ARQ-06 | `E1` |
| `EngineEvent`, `EventSink`, `NoopSink`, `EventBus` | 02 §5.2 (1314-1335); ARQ-68, ARQ-101 | `E1` |
| Sink over a channel, `events()`, `Waker`, 10 Hz coalescing of `JobProgress`; the channel built by the composition root (P11) | 02 §5.2 (1331, 1335), §13; ARQ-112 | `E2` |
| `History` reports the `DirtySet` of the entry it installed (P14) | 02 §5.1, §4.3; ARQ-30 | `E3a` |
| `EditService::{execute, undo, redo, begin_gesture, end_gesture}`, `Session`, `DocVersion`, `DocumentChanged { version, dirty }`, `ProjectService::new`, `ToMessage for EngineError` | 02 §5.1 (1299-1312), §5.2 (1317), §5.4 (1359), §7.3 (1591); ARQ-30, ARQ-33, ARQ-102 (the `ToMessage` half) | `E3b` |
| `ProjectService::{open, save}`, the atomic save steps 1-3, the read-only case | 02 §5.4 (1357), §7.1 (1571), §7.2; ARQ-50, ARQ-55 | `E3c` |
| Autosave over a snapshot: interval, "only if there are changes", slot rotation, host-run job | 02 §7.2 (1567); ARQ-51 (bounded, P13) | `E3d` |
| `UserError { kind, message, detail, actions }` | 02 §7.3 (1591); ARQ-102 | `E11`, after SP3 |
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
| The `UserError` mapping of ADR 0012 | `E1` and `E2` have no user-facing failure with a message key. Amended (E2-W4, P12): `ToMessage for EngineError` arrives with the first failing service, `E3b`; the `UserError` struct waits for SP3 and is `E11`. | ARQ-102 |
| The `.gafira.bak` generation (02 §7.2 step 4) in `E3c` | `ports::FileSystem` has no copy or link (`system.rs:29-71`); keeping one generation needs either a byte copy through `open`/`create` or a port change. `E4` decides it with `save_as`. | ARQ-50 |
| The JSON-only journal format, its BLAKE3 checksum and the 200 ms figure | No port writes a journal (`repository.rs:68-82` only saves a whole `Document`). `E3d` writes an interim slot (P13, O5). | ARQ-51, ARQ-52 |
| A host VRAM limit in `E5` | 02 §6.7 (1466) gives a separate VRAM limit; `E5` governs host bytes only (P17, SP8). | ARQ-44 |

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
| `E2` | Sink over a channel, `EventChannel` built by the composition root and handed to `Gafira::new` (P11), `events() -> &Receiver<EngineEvent>`, `Waker`, 10 Hz coalescing | 02 §5.2 (1331, 1335), §13; ARQ-112 (the bus part) | `E1` | serial | 320 (448); measured 558 before the fix round (E2-W2) |
| `E3a` | `gafira-edit`: `History::newest_touches()` and `History::undone_touches()`, additive (P14) | 02 §5.1, §4.3; ARQ-30 | `E2` | serial | 90 (126) |
| `E3b` | `DocVersion`, `Session`, `SessionCell`, `EditService::{execute, undo, redo, begin_gesture, end_gesture}`, `ProjectService::new`, `DocumentChanged { version, dirty }`, new `EngineError` variants, `ToMessage for EngineError` | 02 §5.1, §5.2 (1317), §5.4 (1357, 1359), §7.3; ARQ-30, ARQ-33, ARQ-102 (`ToMessage` half) | `E3a` | serial | 420 (588) |
| `E3c` | `ProjectService::{open, save}`, atomic save steps 1-3 through the ports, read-only refusal, private fakes with fault injection | 02 §5.4 (1357), §7.2 (1565-1568); ARQ-50, ARQ-55 | `E3b` | serial | 380 (532) |
| `E3d` | `EngineConfig::autosave_interval`, `ProjectService::{autosave_due, autosave_finished}`, `AutosaveJob::run`, slot rotation, the interim slot write (P13) | 02 §7.2 (1567); ARQ-51 (bounded) | `E3c` | serial | 360 (504) |
| `E5` | `MemoryGovernor` over the engine trait `Trimmable`, the trim order, the two-pass bound, `MemoryPressure(PressureLevel)` | 02 §6.7 (1515), §4.6 (690); ARQ-44, ARQ-142, ARQ-145; D-03 | `E2`, `E3d` (owner order, same branch) | serial | 320 (448) |
| `E4` | `save_as`, `recover`, `.gafira.bak`, `SaveState`, the versions folder and its list/restore | 02 §5.4 (1358), §7.2; ARQ-50, ARQ-52, ARQ-55 | `E3d` | serial | 420 (588) |
| `E6` | `AssetService::{import, relocate}` through `ContentStore`, `LinksMissing` | 02 §5.4 (1360), §4.11; ARQ-149 | `E3d` | serial | 380 (532) |
| `E7` | `ExportService::{enqueue, cancel, list}`, the persisted queue, the state machine, the retries | 02 §6.4, §5.4 (1361); FV-23, FV-24; ARQ-43, ARQ-148 | `E5` | serial | 500 (700) |
| `E8` | `MediaService::{play, pause, seek, scrub, proxies}` over `gafira-media` | 02 §5.4 (1362), §6.5; FV-11 | `E2` | serial | 400 (560) |
| `E9` | `LicenseService` and `LicenseChanged` | 02 §5.4 (1363), §13, §4.14; D-12, D-55 | `E3d` | parallel to `E10` | 300 (420) |
| `E10` | `SettingsService`, `set_language`, `LocaleChanged` | 02 §5.4 (1364), §14; ARQ-101; D-08 | `E3d` | parallel to `E9` | 320 (448) |
| `E11` | `UserError` and the mapping from `EngineError` (P12) | 02 §7.3 (1591); ARQ-102 | `E3b`, SP3 | serial | 200 (280), provisional until SP3 |

Total: 4 640 authored lines (x1.4: 6 496). The largest delivery is `E7` at 500 lines (x1.4: 700).
The `E2` fix round is not a row: it is about 120 lines (P11, "E2 fix round" below).

### Why each mark holds

| Mark | Ground |
|---|---|
| `E1` serial, first | It edits every shared file: the root `Cargo.toml`, `Cargo.lock`, the V5 table in `ci.yml`, and a new `crates/gafira-engine/`. It is the only delivery that can add the crate the rest import. |
| `E1` before everything | The V5 step and the V12 step fail on a crate that has a row without a `clippy.toml` copy, and the `forbid(unsafe_code)` step reads `crates/*/src/lib.rs`, so the crate cannot exist half-registered. |
| `E2` after `E1` | It implements the sink `E1` declares and replaces the bus field; no other delivery needs the channel. |
| `E3a` to `E3d` serial, in that order | `E3a` changes a public API of `gafira-edit` that `E3b` calls; `E3b` creates `session.rs` and `edit.rs` and the `SessionCell` that `E3c` and `E3d` extend; `E3c` and `E3d` both edit `project.rs` and `EngineConfig`. Each shares a file or an API with the next. |
| `E3`-series and `E5` on one branch, `E5` last | Owner decision of 2026-11-06. It also holds on its own: `E5` publishes `MemoryPressure` through the cloneable `EventBus` of P11 and edits `src/lib.rs` and `event.rs`, which every `E3` delivery edits. (Supersedes the former "`E5` parallel to `E3`" mark.) |
| `E4`, `E6`, `E9`, `E10` after `E3d` | All four begin from a loaded document or an open session; `E4` also extends the save and the autosave of `E3c`/`E3d`. |
| `E11` after `E3b` and SP3 | It maps `EngineError`, which reaches its shape in `E3b`, and its fields are not specified yet (SP3). |
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
| `crates/gafira-edit/src/history.rs` | `E3a` | Additive methods only; no in-flight change edits the crate (see "Dependency state"). Gates at `--workspace` scope, because every crate above layer 1 compiles against it. |
| `crates/gafira-engine/src/event.rs`, `bus.rs` | `E2` fix round, `E3b` (`DocumentChanged.version`), `E5` (`MemoryPressure`) | Serial on one branch; each adds a variant or a field, none reorders. |
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

**Amended 2026-11-06 (E2-W3).** The promise "without changing this signature" did not hold and
is withdrawn: `Gafira::new` takes the channel the composition root built (P11). `EventSink` and
`NoopSink` stay as `E1` fixed them.

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

**Amended 2026-11-06.** O2 is answered (a): `DocVersion` is an engine-owned counter and
`DocumentChanged` gains `version: DocVersion` in `E3b` (P15). `gafira-doc`'s schema is not
touched.

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

**Amended 2026-11-06 (E2-W4).** `E2` has no failing service, so nothing of ARQ-102 lands there.
The mapping is split by P12: `ToMessage for EngineError` in `E3b`, `UserError` in `E11`. The
variants added in `E3` are in P16.

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

### P11. The composition root builds the channel and hands it to `Gafira::new` (E2-W3)

Fact: 02 §5.2 (1331) has the composition root create the channel **before** the engine, give the
receiver to the startup screen and the sender to the startup sequence, which runs off the main
thread, so `StartupProgress` (ARQ-68) reaches the bar from the first step. `E2`'s
`Gafira::new(ports, registries, config, waker)` (`engine.rs:215-228`) creates the channel itself,
so nothing can publish before the engine exists, and `bus.rs:27-33` already documents the
opposite.

Options: (a) keep `E2`'s shape and amend §5.2 through the specifier: the startup sequence would
publish into a second queue, which is what §5.2 rules out; (b) the root builds the bus and the
receiver separately and passes both: a bus and a receiver of two different channels still
compile; (c) one `EventChannel` value that owns the pair, built by the root and consumed by
`Gafira::new`.

Decision: (c).

- `EventChannel::new(waker: Arc<dyn Waker>, clock: Arc<dyn Clock>) -> EventChannel` replaces
  `EventBus::channel`, so there is one way to build the pair.
- `EventChannel::bus(&self) -> EventBus` and `EventChannel::receiver(&self) ->
  Receiver<EngineEvent>` hand out clones; `EventBus` derives `Clone` (it is one `Arc`).
- `Gafira::new(ports, registries, config, events: EventChannel) -> Result<Self, EngineError>`.
- The root builds the channel with the same `Clock` it puts in `Ports`, gives `receiver()` to the
  startup screen and `bus()` to the startup sequence, then calls `Gafira::new(.., channel)`.
- One drainer at a time: a `crossbeam` receiver is multi-consumer, so two drainers would split the
  stream. The startup screen drains until `StartupFinished`, then the UI drains `events()`. The
  engine cannot enforce this; the doc comment of `receiver()` states it.

Consequence for E2-S2: with the startup sequence on another thread there are now two publishers.
`ChannelSink` therefore takes the window decision and the `send` under the same lock, and calls
the waker after it releases the lock (an unbounded `send` does not block; XP6). Consequence for
E2-S1: `flush` moves the window only when it delivers a pending event.

The §5.4 (1356) summary signature has three parameters; §5.2 (1331) needs a fourth. That is an
inconsistency inside 02 (SP4), not a design deviation.

Reversible: yes. `Gafira::new` has no caller outside the engine's tests.

### P12. `ToMessage for EngineError` in `E3b`; `UserError` in `E11`, after SP3 (E2-W4)

Options: (a) the whole ARQ-102 mapping in `E3b`; (b) `ToMessage` in `E3b`, `UserError` after the
specifier answers; (c) all of it in a later delivery.

Decision: (b). `ToMessage` is fully specified (02 §7.3 line 1591, ADR 0012, ADR 0014): a key with
parameters per variant, no `_` arm, delegation for wrapped enums. `E3b` gives every `EngineError`
variant a key in the area `error.engine.`, delegates `Repository` and `Store` to their own
`to_message` (`repository.rs:54`), and has a fixture per variant. `UserError { kind, message,
detail, actions }` is not specified enough to build: `kind` has no values, `detail` has no type
while ARQ-102 forbids visible `String` fields, `actions` has no source, and nothing says whether
the engine or the UI builds it (SP3). It becomes `E11`.

Two wrapped enums do not implement `ToMessage` today: `gafira-edit`'s `CommandError`
(`error.rs:12`) and `gafira-doc`'s `ValidationError` (`error.rs:70`). `EngineError::Command` and
`EngineError::InvalidDocument` therefore get engine keys without delegation
(`error.engine.command_failed`, `error.engine.invalid_document`). That is an ARQ-102 compliance gap
of those crates, reported as SP3(b), not fixed here.

The "key exists in both catalogs" half of ARQ-102 cannot run: no catalog crate exists
(`gafira-i18n` is layer 4 and not built). `E3b`'s fixture asserts the key name per variant only.

Reversible: yes; message keys are not persisted.

### P13. `E3d` autosave: the engine schedules, the host runs the job, the slot is interim

Facts: §7.2 (1567) asks for a journal of JSON only in
`datos_app/recovery/<project_id>/slot-{0,1,2}.gafira-journal`, under 200 ms for 20 pages, on the
I/O thread over an `Arc` snapshot. No port writes a journal: `ProjectRepository::save(&Path,
&Document)` (`repository.rs:82`) writes a whole project, and `FileSystem` has `create`,
`sync_all`, `replace` and `remove` (`system.rs:20-53`). `Document` has no project id
(`document.rs:18`). The engine spawns no thread (P1).

Options: (a) interim: write the snapshot through `repository.save` to the slot path, through a
temporary file and `filesystem.replace`; (b) add a journal method to `gafira-ports` now; (c) `E3d`
owns only the timing rule and writes nothing until `E4`.

Decision: (a), **provisional** pending O5. (b) designs a format (JSON-only pages, references into
the `ContentStore`, a BLAKE3 checksum) inside an archived crate with no format spec behind it. (c)
leaves a scheduler that no test can see write, and gives `E4`'s `recover` nothing to read. (a) uses
only existing ports, gives `E4` a slot it can `repository.load`, and the later format swap touches
only `AutosaveJob::run`.

Cost, stated once: a slot holds a whole project under a `.gafira-journal` name; "JSON only" and the
200 ms figure are **not met and not claimed** by `E3d`. Recovery slots are short-lived, so a later
format change needs no migration of them (`E4` decides what an unreadable slot means).

Mechanics:

- Path: `data_dir/recovery/<project_key>/slot-<n>.gafira-journal`, `n` in `0, 1, 2`, temporary
  file `slot-<n>.gafira-journal.tmp` in the same folder. One job per session at a time, so the
  fixed temporary name cannot collide; a stale one from a crash is truncated by `create`
  (`system.rs:40`).
- `project_key`: minted by the engine at `new` and `open` as `<clock.now() seconds>-<engine
  counter>`, stable for the session. A real project id is SP5.
- Timer: `pending_since: Option<Duration>` (monotonic) is set by the first edit, undo or redo
  bump after the last snapshot was taken; the bumps of `new` and `open` do not set it (an
  untouched document needs no slot). `ProjectService::autosave_due()` reads `clock.monotonic()` and returns
  `Some(AutosaveJob)` exactly when a session is open, `pending_since` is set, no job is in flight,
  and `now - pending_since >= interval` (inclusive). Taking the job clears `pending_since`.
- `AutosaveJob { snapshot: Arc<Document>, version, slot, folder, project_key, repository,
  filesystem }` is `Send`; `run(self) -> AutosaveOutcome` does `create_dir_all`, `repository.save`
  to the temporary path, `filesystem.replace` onto the slot, and on failure a best-effort `remove`
  of the temporary file. The host runs it on its I/O thread or inline; the engine never spawns
  (P1, P8).
- `ProjectService::autosave_finished(outcome)`: on success the slot advances `0 -> 1 -> 2 -> 0` and
  the in-flight mark clears; on failure the slot does not advance and `pending_since` becomes
  `now`, so a failing disk is retried once per interval, never once per tick. An outcome whose
  `project_key` is not the current session's is ignored (the session was replaced meanwhile).
- A successful `save` clears `pending_since`: the content is on disk.

What bounds ARQ-51 in `E3d`: the engine is host-pumped, so the claim it proves is the scheduling
invariant, not the process kill test. If the host calls `autosave_due` at least every `p` and a
job completes in `w`, the newest change not yet in a slot is at most `interval + p + w` old. A
keystroke is lost only when the process dies inside that window. The 500-kill test of §6.7 and
ARQ-51 itself belong to the binary that pumps the engine; `E3d` does not close them.

Reversible: yes for the scheduling API; the slot content is the provisional part.

### P14. `History` reports the dirty set of the entry it installed (`E3a`)

Fact: the engine cannot read `cmd.touches()` around `History::execute`. `touches` is defined only
after a successful `apply` (`reparent.rs:63-69`), and `execute` takes the command by value
(`history.rs:94`) and may merge it into the newest entry, so no handle remains afterwards.
`undo` and `redo` return only the label (`history.rs:219`, `history.rs:251`).

Options: (a) the engine over-approximates, every page and node, on every edit: that gives up the
"re-rasterize only the node's tiles" technique of §6.7 (recolor < 50 ms) and the dirty pages of
the future journal; (b) `gafira-edit` gains an API that returns the entry's set; (c) the engine
wraps each command to record its set: it breaks `merge`, which recovers the concrete type through
`Any` (`command.rs`, `merge` doc).

Decision: (b), additive:

- `History::newest_touches(&self) -> Option<DirtySet>`: `touches()` of the newest undo entry.
- `History::undone_touches(&self) -> Option<DirtySet>`: `touches()` of the newest redo entry.

The engine reads `newest_touches` after an `execute` that returns `Changed` and after a `redo`
that returns `Some`, and `undone_touches` after an `undo` that returns `Some`. Inside a merged
gesture the newest entry reports the union of the gesture, a superset of the last step; consumers
over-invalidate within one gesture, which is safe. XP4 checks that every command's `touches` is
defined after `merge` and after `revert`.

Rejected as well: changing the return types of `undo`/`redo` (breaks the call sites in
`gafira-edit/tests/history.rs` and `gesture.rs` for no gain over two read-only methods).

Reversible: yes, two additive methods.

### P15. `DocVersion`, the session, the history boundary and what "dirty" means (`E3b`, `E3c`)

**Version (O2 = (a), owner).** `DocVersion(u64)`, `Copy + Ord`, owned by `Gafira` (not by the
session), starting at 0 when the engine is built and strictly increasing for the engine's
lifetime.

| Bumps (one `DocumentChanged` each) | Never bumps |
|---|---|
| `execute` returning `Changed`, merged or not | `execute` returning `Unchanged` or `Err` |
| `undo` returning `Some` | `undo`/`redo` returning `None` or `Err` |
| `redo` returning `Some` | `save`, `autosave_*`, `begin_gesture`, `end_gesture` |
| a successful `new` or `open` | a failed `open` |

Undo bumps: a version names a state the observer saw, not a content, so returning to an earlier
content gets a new number. `new` and `open` bump instead of resetting, so an observer that caches
by version can never confuse two documents. Their `DocumentChanged` lists every page and every
node of the new document; an observer drops what it holds for pages not in the set.

**Session and history.** `Session { history: History, path: Option<PathBuf>, read_only: bool,
clean_version: DocVersion, project_key, autosave state }`. The session owns the `History`, which is
the only place a command runs (ARQ-30), with the default 512 MiB budget (`history.rs` `DEFAULT_BUDGET`,
§5.1). `EditService` passes `clock.monotonic()` as `at` (§5.1). `ProjectService` and `EditService`
share one `SessionCell = Arc<Mutex<Option<Session>>>`, so §5.4's `projects(&self)` and
`edit(&mut self)` both hold; lock poisoning is ignored as in `bus.rs` `with_window`.

`new` and `open` replace the session and drop the old history: there is no undo across them. The
engine does not prompt; the UI asks first, using `ProjectService::is_modified()`. `open` loads and
validates before it swaps, so a failed `open` leaves the old session as it was. Any edit or
undo with no session is `EngineError::NoSession`.

**Dirty, three notions.**

1. `DocumentChanged.dirty`: per event, never accumulated, nothing clears it.
2. "Modified" (unsaved changes): `version != clean_version`. `new` and `open` set `clean_version`
   to their own version; only a successful `save` (and `save_as` in `E4`) of a snapshot sets it to
   that snapshot's version. An autosave never clears it, because the user's file did not change.
   Undoing back to the saved content still reports modified (conservative).
3. Autosave pending: `pending_since` of P13.

Reversible: yes; nothing here is persisted.

### P16. Shapes for `E3`: the interval, the errors, the fakes

- `EngineConfig` gains `autosave_interval: Duration`, default 10 s.
  `with_autosave_interval(self, Duration) -> Result<Self, EngineError>` accepts 5 s to 15 s
  **inclusive** and returns `InvalidConfig` otherwise (§7.2, FIJADA). Both bounds and one step
  outside each are tests.
- `EngineError` gains, in `E3b`: `NoSession`, `Command(CommandError)`,
  `InvalidDocument(ValidationError)`; in `E3c`: `ReadOnly` (ARQ-55), `NoPath` (a session never
  saved; its exit is `save_as` in `E4`), `Io(std::io::Error)` (a `FileSystem` call of the engine
  failed). `From` for `CommandError`, `ValidationError` and `io::Error`. The enum becomes
  `#[non_exhaustive]`, because `E4` to `E10` add variants.
- Fakes live **inside the engine, private** (`#[cfg(test)]` modules or `tests/support/`), as
  `gafira-media` did (`tests/support/mod.rs:45`, `:172`). The `gafira-testkit` stubs
  (`src/ports/file_system.rs`, `src/ports/clock.rs`) are left alone: filling them changes a crate
  every layer uses and its `ports` feature step, for one consumer. They move to the testkit when a
  second crate needs the same doubles. `E3c` owns: an in-memory `FileSystem` that records each
  `create`/`write`/`sync_all`/`replace`/`remove` and fails on the n-th one (ARQ-50), and an
  in-memory `ProjectRepository` that writes its bytes **through that `FileSystem`**, so the fault
  trace covers the repository's writes too.

Reversible: yes.

### P17. `E5`: the governor's order, arithmetic, bound and pressure rule

Facts: `ports::TileCache` has `bytes`/`trim` in bytes (`render.rs:112-121`); `RenderBackend` has
`memory_usage() -> MemoryUsage { host, device }` and `trim` (`render.rs:168`, `:315`);
`gafira-media`'s frame cache is a concrete type with `bytes`/`trim` (`cache.rs:199`, `:321`) and
the engine has no `gafira-media` edge until `E8`.

- **Trait.** `Trimmable { fn bytes(&self) -> Bytes; fn trim(&mut self, budget: Bytes); }` in the
  engine, with wrappers over `&mut dyn TileCache` and `&mut dyn RenderBackend` (host bytes). The
  frame cache is wrapped by the composition root until `E8` brings the edge. The governor does not
  own the caches (the frame loop does): `enforce(&mut self, caches: &mut [(CacheKind, &mut dyn
  Trimmable)]) -> GovernorOutcome { before, after, level }`.
- **Order.** `CacheKind { Frames, Tiles, Backend }`, sorted stably in that order whatever the
  caller's order: frames first (ARQ-44, ARQ-145), then tiles (content-addressed, rebuilt on demand,
  ARQ-142), then backend textures (most expensive to rebuild). Equal kinds keep the caller's order.
- **Arithmetic.** `total` is the `saturating_add` of every `bytes()` (`Bytes` is `u64`,
  `common.rs:25`). `total <= limit` calls no `trim` at all: **exactly at the budget is within the
  budget**. Otherwise `excess = total - limit`, and for each cache in order while `excess > 0`:
  `b = bytes()`, `trim(b.saturating_sub(excess))`, `a = bytes()`, `freed = b.saturating_sub(a)`,
  `excess = excess.saturating_sub(freed)`. A cache that reports more after a trim frees 0, never a
  negative amount.
- **Bound.** At most `MAX_TRIM_PASSES = 2` passes, so at most `2 * caches.len()` `trim` calls per
  `enforce`. The second pass runs only when a fresh sum of every `bytes()` (not the running
  `excess`) is still above the limit. No loop waits for a cache that cannot shrink; the next try is
  the host's next `enforce`.
- **Zero budget.** `limit = Bytes(0)` is valid: every cache is trimmed to 0 in order. An empty
  slice has `total = 0 <= limit` and does nothing. `memory_limit: None` still refuses to build the
  governor (`InvalidConfig`, P5).
- **Pressure.** `MemoryPressure(PressureLevel)` with `PressureLevel { Normal, Critical }`:
  `Critical` when the total is still above the limit after the bound. The governor publishes only
  on a **change** of level (it starts at `Normal`, unpublished), so a host calling `enforce` every
  frame does not flood the bus. "Trimmed and now within budget" is in the outcome, not an event.
  On `Critical` the engine neither panics nor refuses work in `E5`; activating proxies (§7.3,
  "Memoria") is the reaction of `E8` and the UI, and `E7` decides whether a job waits.
- **Scope.** The governor counts only what is passed to it: not the history (its own 512 MiB,
  §5.1), not the document. The §6.7 test "1 GB limit, +10 %" is a process test, not `E5`'s.

Reversible: the order is one table in the governor (P7).

### P18. `E3c`: `open`, the atomic save and the read-only case

- `open(path)`: `repository.load` (a newer schema loads with `read_only` set, `repository.rs:75`),
  then `History::new` (a `ValidationError` is `InvalidDocument`), then the swap of P15.
- `save()`: no session is `NoSession`; `read_only` is `ReadOnly` **before any `FileSystem` or
  repository call** (ARQ-55; the test asserts an empty fake trace); no path is `NoPath`. The
  snapshot (`Arc<Document>`) and its version are taken under the lock, the lock is released, and:
  1. `repository.save(&tmp, &snapshot)` with `tmp = <file name>.tmp-<n>` in the target's folder,
     `n` an engine counter (no `rand`, P1; a stale name is truncated by `create`). Step 2 of §7.2,
     the `fsync` of the file, is the repository adapter's, because the engine never holds the
     writer (SP6).
  2. `filesystem.replace(&tmp, &path)`, which renames atomically and syncs the folder per OS
     (`system.rs:47`).
  On any failure: a best-effort `filesystem.remove(&tmp)` whose own error does not replace the
  first one, the target untouched, `clean_version` unchanged, the error returned. On success,
  `clean_version` becomes the snapshot's version if the session is still the same (`project_key`).
- ARQ-50 test: for one save, inject a failure at every operation of the fake trace in turn; after
  each, the target holds the old bytes or the new bytes in full.
- A read-only session accepts edits in memory, refuses `save`, and its autosave still writes
  recovery slots (they are not the user's file); `save_as` in `E4` is the way out. **Provisional**
  on SP7.

Reversible: yes.

## E2 fix round (instructions for `tdd-apply`)

Same branch `feat/gafira-engine-e2`, about 120 authored lines. Order: the card is not edited; the
implementer's notes go in a commit after the code (E2-W6).

| # | Finding | Change | Proof (RED first) |
|---|---|---|---|
| F1 | E2-W3 | P11: `EventChannel::{new, bus, receiver}` replaces `EventBus::channel`; `EventBus: Clone`; `Gafira::new(ports, registries, config, events: EventChannel)`; the doc comments of `Gafira::new` and the `bus.rs` header cite P11 instead of the deviation note | an event published through `channel.bus()` **before** `Gafira::new` is the first one drained from `gafira.events()`, in order |
| F2 | E2-C1 | The facade test builds the channel with a counting waker | publish through the facade, drain one event from `events()`, assert waker count 1; mutant X3 (the facade ignores the injected waker) must fail it |
| F3 | E2-W1 | Restore the counted every-variant test: all six variants (with `LocaleChanged` and `JobProgress`), each forwarded to the real `NoopSink` | one count per event, six in total |
| F4 | E2-S2 | The window decision and the `send` under one lock; the waker after the lock is released | two `EventBus` clones in one thread: a pending progress published through one leaves before a state event published through the other |
| F5 | E2-S1 | `flush` moves the window only when it delivers | flushing every 16 ms with nothing pending does not hold back the next `JobProgress` once 100 ms have passed since the last delivered one |
| F6 | E2-W4 | `EngineError`'s doc comment points to P12 (`E3b`, `E11`) instead of "Planned for E2"; no mapping code | none (doc only) |

Hand mutants for the round (rule level): the facade ignores the waker of the channel it is given
(F2 must fail); `flush` moves the window with nothing pending (F5 must fail); a variant is dropped
from the every-variant list (F3 must fail). E2-W5 (`.odd/state.json`) is the coordinator's, not
`tdd-apply`'s. E2-W2 is recorded by the size note in the delivery table.

## Critical code

Hand mutants apply to all of these.

| Delivery | Critical | Not critical |
|---|---|---|
| `E1` | The `data_dir` validation of `EngineConfig`, the bus delegation to its sink, the `Ports` wiring (a missing port must not compile, not default) | Doc comments, `Display` text |
| `E2` | The coalescing window, the `Waker` call (through `Gafira::new`, P11), the channel capacity choice, the ordering of the drain, the window decision and `send` under one lock, `flush` moving the window only on delivery | Doc comments |
| `E3a` | `newest_touches` reads the undo top and `undone_touches` the redo top (swapping them is the mutant), `None` on an empty stack | Doc comments |
| `E3b` | **Version bump**: exactly the bumps of the P15 table, one `DocumentChanged` per bump, strictly increasing, never reset by `new`; **dirty set**: the set from `newest_touches`/`undone_touches` per operation (P14); **history boundary**: the session owns the `History`, `new` drops it, no undo across it; the `NoSession` refusal; one `ToMessage` key per `EngineError` variant | `Display` text |
| `E3c` | **Save atomicity**: the order `repository.save(tmp)` then `filesystem.replace(tmp, path)`, the `remove(tmp)` on failure, the target untouched on every injected fault (ARQ-50); the read-only refusal before any file operation (ARQ-55); `clean_version` set only on success; a failed `open` leaving the old session | Doc comments |
| `E3d` | The interval bounds 5 s and 15 s inclusive; due at exactly `interval`; no job without changes; the snapshot is the document at job creation; a change during the job stays pending; slot rotation only on success; retry one interval after a failure; a stale outcome ignored | Doc comments |
| `E5` | **Trim order** (frames, tiles, backend, stable); **byte arithmetic** (saturating sum, `freed` never negative, target `b - excess`); **budget boundary**: `total == limit` calls no `trim`; **zero budget** trims every cache to 0; **MemoryPressure rule**: `Critical` only when above the limit after the bound, published only on a level change; the bound of two passes | Doc comments |
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
| O2 | **Answered: (a)** (decided-by: owner, 2026-11-06). `DocVersion` is an engine-owned counter bumped by `EditService` on every applied command; `gafira-doc`'s schema is not touched. Semantics in P15. | (a) A counter owned by the engine, bumped by `EditService` on every applied command: no crate outside the engine changes, and the version is a session fact. (b) `gafira-doc` gains `DocVersion` in the document: every consumer sees it, but it changes the persisted schema while D-36 keeps schema 1 open. (c) `gafira-edit`'s `History` exposes a revision counter: closest to the truth (the history is what advances), and a change in a layer-1 crate that is in flight nowhere. | (a), taken. If a later delivery needs the version outside the engine, (c) is the migration. |
| O3 | `SelectionService` and `SelectionSnapshot` are named in §5.3 (1342) and §5.2 (1318) and `gafira-edit` has no `Selection` at all. | (a) The engine defines `SelectionSnapshot` (ids plus flags) and `gafira-edit` gains `Selection` later in its own change. (b) `gafira-edit` gains `Selection` first, and the engine only carries it. (c) Drop both from the Facade until a UI needs them. | (a) now, (b) as the follow-up change, since hit testing and the tools of the UI are what need the model. |
| O4 | `crossbeam-channel` becomes a direct dependency of the engine in `E2`. Its licence is MIT OR Apache-2.0, both allowed by `deny.toml`, and §6 of the spec lists it as VERIFICADA. It is **not** in `Cargo.lock` today (only `crossbeam-deque`, `-epoch` and `-utils` are, through `rayon-core`), so `E2` resolves a new package and runs `cargo deny check`. | (a) Take it: it is what §5.2 (1335) names, and using another channel would contradict the spec. (b) Use `std::sync::mpsc`: no new dependency, but no `select`, no bounded capacity choice and no coalescing window with a timeout. | (a), confirmed by `cargo deny check` on the three targets in `E2`. |
| O5 | Until a journal port exists, `E3d` writes each recovery slot as a whole project through `ProjectRepository::save` (P13). That does not meet the "JSON only" rule nor the 200 ms per autosave figure of §7.2, both FIJADA. Accept the interim for `E3d`? | (a) Accept: autosave and `E4`'s `recover` work now with existing ports; the format and the figure arrive with a journal method in `gafira-ports` (its own change, with a format spec), before the release, and only `AutosaveJob::run` changes. (b) Block `E3d` until that ports change lands: the spec is met from the first slot, but the engine has no autosave meanwhile and `E4` waits too. (c) `E3d` ships the scheduler only and writes nothing: nothing deviates, but ARQ-51 has nothing to stand on and the scheduler's write path is untested. | (a), with the journal port recorded as a prerequisite of the release, not of `E4`. |

## Needs explorer

| # | Claim | Check | Blocks |
|---|---|---|---|
| XP1 | **Resolved in `E1`.** The V5 step and the V12 copy comparison accept a new row whose list is the whole transitive `gafira-*` tree (the step compares every transitive dependency, not only the direct ones); V5 and V12 exit 0 on the engine row. The ARQ-107 step does **not** leave layer 3 alone: it iterates **every** crate of the V5 table, applies its dependency check to all of them, and skips the `Locale` file search only for the names on its exemption list. That list holds two crates, `gafira-ports` and `gafira-engine`, because layer 3 resolves the locale by design (02 §14.1, ADR 0016). Membership is by name, so any future layer-3 crate that carries a `Locale` type must be added to that list deliberately rather than assumed exempt. | The V5, V12 and ARQ-107 shell snippets run locally against the new row: all three exit 0, and ARQ-107 prints `gafira-ports: layer 3, exempt from the Locale search` and `gafira-engine: layer 3, exempt from the Locale search`. | — |
| XP2 | ARQ-05 (76) requires `cargo public-api` in CI for `gafira-doc`, `gafira-ports` and `gafira-engine`, and the current `ci.yml` has no such step for any of the three. | `grep -n "public-api" .github/workflows/ci.yml` and `docs/adr/`. | `E1` records it; not fixed here |
| XP3 | A `Gafira` that owns `Arc<dyn Trait>` ports and an `Arc<dyn EventSink>` bus is `Send + Sync`, which the UI needs to hold it across frames. | A compile-time assertion in a test: `fn assert_send_sync<T: Send + Sync>() {}` applied to `Gafira`. | `E1` |
| XP4 | Every `Command` in `gafira-edit` returns a defined `touches()` after `merge` (the union of the merged steps) and after `revert`, so P14's accessors never report an "unspecified" set (`reparent.rs:63-69` documents the before-apply case only). | Read each `impl Command` in `crates/gafira-edit/src/` (`transaction.rs`, `reparent.rs`, `set_property.rs`, `moves.rs`, ...) for its `merge` and `touches`; list any that recompute `touches` from state `merge` or `revert` leave stale. | `E3a` (P14 stays provisional until then) |
| XP5 | No in-tree `ProjectRepository` adapter exists outside tests (`crates/gafira-ports/tests/repository.rs:13`, the engine's test double), so nothing yet shows whether a real adapter writes through `FileSystem` and syncs the file before returning (P18 step 1, SP6). | `grep -rn "impl ProjectRepository" crates/` and the `gafira-format` plan in `specs/`. | `E3c` (records the gap; does not block the engine's own order) |
| XP6 | `crossbeam_channel::Sender::send` on an unbounded channel never blocks and never calls back into user code, so sending while holding the window mutex cannot deadlock (P11, E2-S2). | `crossbeam-channel 0.5.17` docs and source of `unbounded`/`send`. | `E2` fix round |
| XP7 | `Arc<Mutex<Option<Session>>>` holding a `History` keeps `Gafira: Send + Sync` (`Command: Send + Sync`, `command.rs:46`). | The XP3 compile-time assertion, re-run after `E3b`. | `E3b` |

## Needs specifier

| # | ID | Request | Blocks |
|---|---|---|---|
| SP1 | 02 §5.4 (1356) | `new` returns `Result<Self, EngineError>` but §5.4 never says what can fail at construction. This design narrows it to `InvalidConfig` and the port errors; confirm or extend. | `E1` |
| SP2 | 02 §5.2 (1317-1328) | The payload types the spec names for the bus (`SaveState`, `JobResult`, `RecoveryEntry`, `Level`, `StartupStep`) belong to no crate. Confirm that the engine defines them, which P3 assumes. | `E2`, `E4`, `E5`, `E7` |
| SP3 | ARQ-102, 02 §7.3 (1591) | (a) `UserError { kind, message, detail, actions }`: the values of `kind`, the type of `detail` (ARQ-102 forbids visible `String` fields; §7.3 sends the detail to the local log), where `actions` come from, and whether the engine or the UI builds it. (b) `CommandError` (`gafira-edit`) and `ValidationError` (`gafira-doc`) do not implement `ToMessage`, and the ARQ-102 compliance list does not name them; say which change owns them. | `E11`; (b) does not block `E3b` (P12) |
| SP4 | 02 §5.4 (1356) vs §5.2 (1331) | The summary signature `new(ports, registries, cfg)` cannot receive the channel §5.2 has the root build first. Amend §5.4 to take the channel (P11). | Nothing (P11 proceeds) |
| SP5 | 02 §7.2 (1567) | `recovery/<project_id>/` names a project id that no type carries (`Document` has none; `Project` arrives with §3.1). Say what it is and whether it survives reopening the same file. P13 mints an interim key. | `E4` (`recover` must find a project's slots) |
| SP6 | 02 §7.2 step 2, ARQ-50; `ProjectRepository::save` (`repository.rs:82`) | The port's contract does not say that `save` syncs the file before it returns, and the engine never holds the writer, so step 2 can only be the adapter's. State it in the port contract. | `gafira-format`; recorded in `E3c` |
| SP7 | ARQ-55 | "Read-only and Save disabled" does not say whether edits are allowed in memory. P18 allows them and refuses `save`. Confirm, or say edits are refused. | `E3c` (provisional) |
| SP8 | 02 §5.2 `MemoryPressure(Level)`, §6.7 (1466) | (a) The levels of `Level`; P17 uses `Normal` and `Critical`. (b) The VRAM limit (70 % of reported VRAM): `E5` governs host bytes only; say which part enforces the device limit. | `E5` (provisional on (a)); (b) does not block |

## Next

`E2` fix round (F1 to F6, "E2 fix round" above) on `feat/gafira-engine-e2`, then its verify. After
`E2` passes, `feat/gafira-engine-e3` from `E2`'s HEAD with `E3a`, `E3b`, `E3c`, `E3d` and `E5`
in that order. O5 should be answered before `E3d` starts; XP4 before `E3a`.
