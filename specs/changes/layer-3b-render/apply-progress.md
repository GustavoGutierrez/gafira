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

- `a1bb4b7 feat(gafira-render): compute the cumulative deadlines of the frame budget` — `FrameBudget`, `Stage` and the constants of 02 §6.2, with the three behaviour tests and the module export of `lib.rs`.
- `2e9e6da feat(gafira-render): degrade a lost GPU to the CPU and report it` — `BackendSelector`, `Degradation` and the ARQ-22 test, which injects the loss on the second task of the plan.
- `436299d feat(gafira-render): run the frame inside its deadline with the cancel per tile` — `FrameLoop`, `FrameRequest`, `FrameOutcome` and the deadline seam, with the seven tests of the refinement, the deadline boundary, the cancel and the progress.

Gates per commit, each run alone, exit 0: `cargo fmt --check` (0), `cargo clippy --workspace --all-targets --locked -- -D warnings` (0), `cargo test --workspace --locked` (0: 186 `test result: ok` blocks and 2365 tests after the first commit, 2366 after the second and 2373 after the third, none failed; `cargo test -p gafira-render --lib` gives 43, 44 and 51 tests), `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`), and the CI policy steps — Format, CLI/UI (ARQ-07), `forbid(unsafe_code)`, V5, V12, ARQ-107, V13, V14 and `Testkit ports feature compiles (ADR 0013)` — each with its `run:` extracted literally from `.github/workflows/ci.yml` and run under `bash -euo pipefail`, all 0.

### RED evidence

Every criterion has a test that names the requirement it proves. The RED of each one was taken on the green code by breaking the behaviour that the criterion names and running that test: the assertion that fails is the one below. The mutations H1 to H6 are the hand-mutant table that follows; the rest (the per-tile cancel, the progress, the tile report and the determinism) are the extra mutations those criteria needed. None of them is committed.

- **AC1** — `budget::tests::arq_41_the_stages_are_cumulative_deadlines` (H1: the visible stage's deadline moved to the end of the frame): `left: At(116ms)`, `right: At(114ms)`.
- **AC2** — `frame::tests::arq_41_the_left_tiles_are_reported_at_the_exact_deadline`. H2 (exclusive comparison): `left: RenderReport { done: 2, left: 0 }`, `right: RenderReport { done: 1, left: 1 }`; H3 (`Unbounded` passes): `left: RenderReport { done: 0, left: 2 }`, `right: RenderReport { done: 2, left: 0 }`. And `frame::tests::arq_41_a_tile_the_backend_leaves_is_not_counted`, with the report of `execute` ignored: `left: RenderReport { done: 3, left: 0 }`, `right: RenderReport { done: 0, left: 3 }`.
- **AC3** — `budget::tests::arq_02_the_deadline_comes_from_the_injected_clock` (H6: `begin` starts at `Duration::ZERO`): `assertion left == right failed: start 7ms`, `left: At(6ms)`, `right: At(13ms)`.
- **AC4** — `frame::tests::arq_41_a_late_frame_presents_the_best_mip_and_refines` (H5: `left` is always 0): `left: RenderReport { done: 1, left: 0 }`, `right: RenderReport { done: 1, left: 1 }`.
- **AC5** — `frame::tests::arq_22_a_lost_device_degrades_the_session_to_the_cpu` (H4: `degrade` does nothing): `left: None`, `right: Some(DeviceLost)`.
- **AC6** — `frame::tests::arq_43_the_cancel_stops_the_frame_between_tiles`, with the loop's per-tile check removed: `left: 2`, `right: 1` (the double ignores the token on purpose, so only the loop can stop it). And `frame::tests::arq_43_the_progress_is_the_units_done_over_the_units_planned`, with the fraction fixed at `1.0`: `left: [1.0, 1.0, 1.0, 1.0]`, `right: [0.25, 0.5, 0.75, 1.0]`.
- **AC7** — `frame::tests::arq_02_the_same_frame_takes_the_same_decisions`, with the outcome taking a value from a process counter: `left: FrameOutcome { report: RenderReport { done: 1, left: 2 }, mip: 0, refine: true }`, `right: FrameOutcome { report: RenderReport { done: 1, left: 2 }, mip: 1, refine: true }`.
- **AC8** — a command, not a test: `crates/gafira-render/src/lib.rs` opens with `#![forbid(unsafe_code)]`, the `forbid(unsafe_code)` and V5 steps pass, and `git grep -nE 'std::fs|std::net|std::thread::spawn|std::time::(Instant|SystemTime)::now|std::env' -- 'crates/gafira-render/src/*' 'crates/gafira-render/tests/*'` finds nothing (exit 1).

### Negative controls

The four pieces of critical code of `RE4` (design, "Critical code": the split of the budget, the computation of the deadline, the degradation path and the `left > 0` report) carry the six hand mutants of the Slice card. Each one was applied on the green code, the test that names it was run, and the change was reverted. None of them is committed.

| # | Mutation | Test that kills it | Failing assertion |
|---|---|---|---|
| H1 | the visible stage reaches the end of the frame (`VISIBLE_DONE = 16 ms`), so it spends the margin | `budget::tests::arq_41_the_stages_are_cumulative_deadlines` | `left: At(116ms)`, `right: At(114ms)` |
| H2 | the comparison is exclusive (`now > at` in `deadline_passed`) | `frame::tests::arq_41_the_left_tiles_are_reported_at_the_exact_deadline` | `left: RenderReport { done: 2, left: 0 }`, `right: RenderReport { done: 1, left: 1 }` |
| H3 | `Deadline::Unbounded` always passes | `frame::tests::arq_41_the_left_tiles_are_reported_at_the_exact_deadline` | `left: RenderReport { done: 0, left: 2 }`, `right: RenderReport { done: 2, left: 0 }` |
| H4 | `degrade` keeps the failing GPU and reports nothing | `frame::tests::arq_22_a_lost_device_degrades_the_session_to_the_cpu` | `left: None`, `right: Some(DeviceLost)` |
| H5 | the frame always reports `left == 0` | `frame::tests::arq_41_a_late_frame_presents_the_best_mip_and_refines` | `left: RenderReport { done: 1, left: 0 }`, `right: RenderReport { done: 1, left: 1 }` |
| H6 | the deadline starts at `Duration::ZERO`, from a clock the crate owns | `budget::tests::arq_02_the_deadline_comes_from_the_injected_clock` | `assertion left == right failed: start 7ms`, `left: At(6ms)`, `right: At(13ms)` |

H2 and H3 are the equality of the deadline and the `Unbounded` case of 02 §6.2; H5 is the report of the frame that does not finish. The three directions of the criterion now have a test that fails when the behaviour moves.

### Choices and deviations

- **The stages are cumulative by construction**: the budget stores the instant its frame started and derives every deadline as `start + offset`; no method reads the clock again, so a stage that runs into the margin cannot move the deadlines of the frame. H1 is the "independent budgets" mutation, and the test of AC1 also advances the scripted clock between the budget and the deadline to pin that the instant is the start.
- **The equality rule has one seam, in the loop**: `deadline_passed` delegates to `ports::Deadline::has_passed` (the port owns the arithmetic), and the loop calls it before every tile. H2 and H3 are applied on that seam, so the boundary the crate enforces is the code the mutants move.
- **One tile per `execute` call**: the loop of 02 §6.3 drives one task of the `RenderPlan` per call, which is what lets it check the cancel token and report the progress at every tile boundary and stop exactly at the deadline; the backend also receives the deadline and the token for its own check. A backend that batches several tiles per call is a later optimisation, and it would have to give the loop the same boundaries through the port.
- **`FrameRequest` instead of eight arguments**: the request carries the clock, the target, the deadline, the cache, the cancel token and the progress; clippy's `too_many_arguments` is why, and the frame call stays at the request plus the plan.
- **The degradation triggers on `RenderError::DeviceLost` only** (ARQ-22, 02 §6.4), from `begin_frame` or from `execute`, and the GPU is dropped. No GPU at construction is reported as `Degradation::AdapterMissing`; a GPU `OutOfMemory` is not stated to degrade the session, so the selector keeps the GPU for it. The selector never creates a backend (P10).
- **"The document survives" is structural in this crate**: the loop receives a plan and a cache, never the document, so the failure cannot touch it. The ARQ-22 test builds a page and its graph, loses the device on the second task of the plan, and asserts the graph's key and the node count are unchanged; the edit-session half of ARQ-22 (the next edit applies and the UI stays editable) is the engine's, and the real-driver case waits for `gafira-render-gpu` (design, "Out, and why").
- **The best mip available is the level of the last plan that finished**: `FrameLoop` keeps `presented`, a late frame reports that level and `refine`, and the next frame continues the plan the caller submits; the cache of `RE3` is what makes the finished tiles free. The loop does not own the plan across frames: the caller resubmits it (cheap to rebuild, ARQ-140) and the outcome says what to present meanwhile.
- **The progress is per frame**: the fraction is the tiles of this plan done in this frame over the tiles of the frame, so the caller hands a frame-scoped `Progress`; the contract of ARQ-112 checks the token before every report, and the fraction reaches `1.0` only when the frame finished the plan.
- **ARQ-41 and the `<= 100 ms` of ARQ-43 are proved as a shape**: the 16 ms arithmetic, the deadline contract and the per-tile cancel are unit behaviour; their §6.7 measurements and benches stay with the recorded debt, and this delivery adds no bench ("Critical code" does not mark the benches of `RE4`).
- **ARQ-143 is the product row this budget implements**, although the Slice card names ARQ-22, ARQ-41 and ARQ-43: the tests name the card's IDs, so `git grep -n 'ARQ-22\|ARQ-41\|ARQ-43'` finds them.
- **No shared file and no dependency**: the change is three files of `gafira-render`; `Cargo.toml`, `Cargo.lock`, `gafira-ports`, `gafira-doc`, the testkit and the `#![forbid(unsafe_code)]` of `lib.rs` are untouched.

### The size

`git diff --shortstat 414bef4`: **1232 insertions, 5 deletions, 3 files** (`budget.rs` and `frame.rs` new, `lib.rs` the module list and its doc). Split: `a1bb4b7` adds 241 and deletes 5, `2e9e6da` adds 380, `436299d` adds 629 and deletes 18. Of those 1232, about 435 are the module code (115 in `budget.rs`, 313 in `frame.rs`, and the exports of `lib.rs`) and the rest are the tests, the doubles and the page and graph helpers they need: the code alone stays under the 460 of the design's forecast (645 at x1.4), and the total with the tests is over it, as `RE1` and `RE3` also were, because the two module suites carry the doubles, the scripted clocks and the page and graph helpers. The crate ends with 51 unit tests (43 after the budget, 44 after the selector, 51 with the loop) and the workspace suite at 2373. With these notes the branch measures 1277 insertions and 6 deletions over four files.

## RE5
Flow: standard | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the render stream of layer-3b closes with the PageRenderer that the exporters reach; O8 carries O4 and O5 of layer 3 to this cut
Product IDs: ARQ-42 (partial: the shared graph and the quality switch here; the 1/255 golden needs the CPU backend of O3), FD-57 (partial: the page scope here; the naming, the multi-file output and the PDF are the exporters of layer 4) (source: specs/02-arquitectura-software.md §4.5, §6.2, specs/04-funcionalidad-diseno-imagen-exportacion.md FD-57)
Scope: `RE5` of `specs/history/2026-11-04-layer-3/design.md`: in `gafira-render`, the `PageRenderer` that `ports::export` describes: it renders one page through the graph, the plan and the tile cache, with the preview and export qualities and the page scope.      Non-goals: the real CPU and GPU backends (`gafira-render-cpu` is O3), the golden of 1/255 (it needs that backend), the importers and exporters of layer 4, the naming and the multi-file output of FD-57, and any change to `gafira-ports`.
Acceptance:
  AC1 Given a `PageRenderRequest`, When the renderer runs, Then it answers an `ImageBuf` built from the graph of `RE1`, the plan of `RE2` and the tiles of `RE3`, and it implements `ports::PageRenderer` (ARQ-42, 02 §4.5) [unit]
  AC2 Given the same page, When it is rendered for the preview and for the export, Then both use the same graph and the same operations, and only the quality parameters of the content keys differ (ARQ-42, 02 §6.2) [integration]
  AC3 Given each quality, When it renders, Then `Preview` answers 8-bit sRGB premultiplied and `Export` answers `f32` in linear light at full resolution, as 02 §6.2 fixes [unit]
  AC4 Given a page scope, When it is mapped to render requests, Then a single page and a page range are handled, and a page outside the range is rejected with `ScopeError::OutOfRange { page, pages }` before any render starts, while `PageRenderer::render` stays total over `PageId` and never panics (FD-57 (a), FD-58) [unit]
  AC5 Given a cancelled render, When the token fires, Then the renderer answers `RenderError::Cancelled` and returns no partial buffer (02 §6.3, ARQ-43) [unit]
  AC6 Given the same page and the same quality, When it renders twice, Then the two buffers are identical byte by byte (determinism, ARQ-02) [property]
  AC7 Given the request, When the transparent background is asked for, Then the buffer has an alpha channel and the page background is not painted (`transparent`, 02 §4.5) [unit]
  AC8 Given the crate, When the CI policy steps run, Then `gafira-render` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the crate reads no clock [command]
Plan: the delivery adds the page renderer module to `crates/gafira-render/src/`, on top of the graph of `RE1`, the plan of `RE2`, the tile cache of `RE3` and the frame loop of `RE4`. It implements the trait that `SC1` left in `ports::export`; the trait itself is not touched. `PageRenderRequest` carries no quality field, so the renderer takes the quality when it is constructed and builds the graph for it. `PageRenderRequest` carries `scale`, which the plan cannot represent, so `scale` is read as the preview's zoom (`lod = level_for(scale)`) and `Export` renders at `lod 0`; the output size of FD-55 belongs to the exporter and to the O3 backend. An out-of-range page is rejected by the scope mapping and not by `render`: `RenderError` has no variant for an unknown page, the port documents `render` as answering only `Cancelled`, and the ARQ-112 double reads it as total over `PageId`, so inventing a variant there would make a caller degrade the session or tell the user they cancelled. Both readings are recorded as deviations. The critical code is the quality switch, the mapping of the export scope and the single-page and range cases (design of layer 3, "Critical code"), so those carry the hand mutants.
Owner decisions touched: O8 of 2026-11-05 (the rigor of layer 3 carries over); O3 keeps the CPU backend, and with it the 1/255 golden of ARQ-42, out of this change.

## RE5 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
