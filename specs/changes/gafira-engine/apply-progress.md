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

Filled after the delivery, in the last implementer commit body (no folder rule does not apply:
this change has a folder).
