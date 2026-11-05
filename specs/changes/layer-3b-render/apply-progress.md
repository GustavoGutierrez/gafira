## Slice card
Flow: standard | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the render stream of layer-3b opens with the frame budget that the planner and the tile cache feed; O8 carries O4 and O5 of layer 3 to this cut
Product IDs: ARQ-22, ARQ-41, ARQ-43 (source: specs/02-arquitectura-software.md §6.2, §6.3)
Scope: `RE4` of `specs/history/2026-11-04-layer-3/design.md`: in `gafira-render`, the frame budget of 02 §6.2 (the cumulative deadlines and the margin no stage may spend), the deadline as a monotonic instant from the injected clock, the backend selection with its degradation to CPU, and the progressive refinement that presents the best mip available when the deadline passes.      Non-goals: the `PageRenderer` and the export scope (`RE5`), the tile cache itself (`RE3`, done), any change to `gafira-ports`, and the real CPU or GPU backend (`gafira-render-cpu` is O3, its own change).
Acceptance:
  AC1 Given a frame, When the budget is computed, Then the stages are cumulative deadlines from the start of the frame and not independent budgets: the graph is ready before `start + 6 ms`, the visible render ends before `start + 14 ms`, and the 2 ms of margin from `start + 14 ms` to `start + 16 ms` belongs to no stage (02 §6.2) [unit]
  AC2 Given a frame that finishes exactly at its deadline, When it is reported, Then it counts as incomplete with `left > 0`; one that finishes a nanosecond before counts as complete with `left == 0`; `Deadline::Unbounded` never passes (02 §6.2) [unit]
  AC3 Given the deadline, When it is computed, Then the instant comes from the injected `ports::Clock` and the crate reads no clock of its own (02 §6.2, ARQ-02) [unit]
  AC4 Given a render that does not finish inside its deadline, When the frame ends, Then the best mip available is presented and the refinement continues in the next frame, and the caller is never blocked (02 §6.2) [unit]
  AC5 Given a backend that fails or is unavailable, When the selector runs, Then the render degrades to CPU and the degradation is reported, the document survives it, and the degradation lasts the rest of the session (ARQ-22) [unit]
  AC6 Given a long render, When it runs, Then the cancel token is checked per tile and the progress fraction follows units done over units planned (02 §6.3, ARQ-43) [unit]
  AC7 Given the same input, When the budget runs twice, Then it takes the same decisions (determinism, ARQ-02) [property]
  AC8 Given the crate, When the CI policy steps run, Then `gafira-render` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the crate still reads no clock [command]
Plan: the delivery adds the budget module to `crates/gafira-render/src/`, on top of the plan of `RE2` and the cache of `RE3`. It adds no dependency and touches no shared file. The critical code is the split of the budget, the deadline computation, the degradation path and the `left > 0` report (design of layer 3, "Critical code"), so those carry the hand mutants. `ARQ-41` (the UI thread never blocks more than 16 ms) and the `<= 100 ms` of `ARQ-43` are §6.7 measurements: they are proved as a shape here, and their benches stay with the debt already recorded.
Owner decisions touched: O8 of 2026-11-05 (the rigor of layer 3 carries over); no D-nn is contradicted.

# Apply progress: layer-3b-render

## RE4 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
