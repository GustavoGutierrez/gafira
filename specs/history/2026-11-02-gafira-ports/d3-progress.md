## D3 progress (tdd-apply)

Design: `specs/changes/gafira-ports/design.md` (D3 row, P2, P6, P8). Branch `feat/gafira-ports-d3`. Files: `crates/gafira-ports/src/render.rs`, `crates/gafira-ports/tests/render.rs`, this file.

### Done

- `render`: `RenderBackend`, `TileCache` (traits), `RenderPlan`, `TileTask`, `TileKey`, `ContentKey`, `Priority`, `RenderQuality`, closed `RenderOp` (no `#[non_exhaustive]`), `FrameCx`, `FrameId`, `BackendCaps`, `BackendKind`, `TargetDesc`, `MemoryUsage`, `Deadline`, `RenderReport`, `RenderError` (+ `From` for `Cancelled`, `EffectError`, `BufferError`; `ToMessage`).
- `PageRenderer` belongs to D5b (`export` module in P4): left out.
- Shapes the design leaves open: `BackendCaps { kind, max_texture, formats, gpu_effects }`, `TargetDesc { width, height, format }`, `MemoryUsage { host, device }`, `RenderReport { done, left }`, `FrameCx { id }`, `Deadline::At(Duration)`.

### RED evidence (skeleton with wrong bodies: `has_passed` false, `is_complete` false, one shared render key)

| ID | Test | Failing assertion |
|---|---|---|
| ARQ-06, P8 | `arq_06_backend_double_runs_through_dyn` | `assert!(report.is_complete())` |
| ARQ-06 (02 §6.2) | `p2_late_render_is_a_report_not_an_error` | left `{done:5,left:0}` right `{done:2,left:3}` |
| P8 Deadline | `p8_deadline_has_passed_table` | `At(1µs) at 1µs` left false right true |
| P8 RenderReport | `p8_report_is_complete_only_when_nothing_is_left` | left 0: left false right true |
| ARQ-22 | `arq_22_fallback_failures_are_told_apart` | distinct keys left 1 right 4 |
| ARQ-102 | `arq_102_render_error_messages` | key left `error.render` right `error.render.device_lost` |
| ARQ-06 closed op, S13, cancel, priority, delegation | `arq_06_render_op_is_closed`, `s13_*`, `p2_cancel_*`, `p2_priority_*`, `arq_102_wrapped_*` | Pass on the skeleton: they check structure that the skeleton already had (compile-level RED, as D1) |

### Hand mutants

| # | Mutant | Killed by |
|---|---|---|
| 1 | `At(t)` strict: passed only when `now > t` | `p8_deadline_has_passed_table`, `p2_late_render_is_a_report_not_an_error` |
| 2 | `Unbounded` reports passed | `p8_deadline_has_passed_table`, `arq_06_backend_double_runs_through_dyn` |
| 3 | `is_complete` true when `left <= 1` | `p8_report_is_complete_only_when_nothing_is_left` |
| 3b | `is_complete` also true when `done > 0` (ignores `left`) | `p8_report_is_complete_only_when_nothing_is_left`, `p2_late_render_is_a_report_not_an_error` |
| 4 | `OutOfMemory` shares the `DeviceLost` key | `arq_102_render_error_messages`, `arq_22_fallback_failures_are_told_apart` |
| 5 | `Cancelled` gets its own key `error.render.cancelled` | `arq_102_render_error_messages`, `arq_102_wrapped_errors_delegate_and_convert` |

### Deviations

- Acceptance rule is `now >= t` (design P8, 02 §4.6); the task's example "passed only when `now > t`" is mutant 1.
- The shared target dir makes sibling worktrees overwrite each other's `gafira-ports` rlib (same package, same workspace-relative metadata hash): a first test build linked another worktree's stub. Fixed by `RUSTFLAGS="--cfg wt_d3"` (changes the metadata hash, touches no file). Nothing in the repo changed.
- `Display` text and plain data are untested, per design.

## Verify round 1 fix (C1, S2)

- C1: added `EffectRegistry = Registry<dyn Effect>` and `TransitionRegistry = Registry<dyn Transition>` (design P4). Test `p4_effect_and_transition_registries_register_by_id` registers a real raster effect (`ColorAdjust`) and transition (`Cut`) through each, rejects a repeat, and builds an `ApplyEffect` op from the stored `Arc`. RED: compile-level (the aliases did not exist), as aliases hold no behaviour to fail an assertion.
- Alias mutant: `TransitionRegistry` over `dyn Effect` is killed by the same test (type error on `Cut`). No other logic.
- S2: doc notes that `Alpha8` is not a render target on `TargetDesc` and `BackendCaps`.
