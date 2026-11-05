# Verify report: gafira-render-cpu

```json
{ "schema": 1, "change": "gafira-render-cpu", "delivery": "CB2", "rounds": [ {
  "round": 1, "commit": "08649a41804e1319a1f38bf8f2e9697b140ebaa6", "mode": "standard", "tier": "T0", "depth": "V2",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 3, "suggestion": 2},
  "findings": [
    {"id": "W1", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:88",
     "summary": "CB2 AC1 gives PushLayer a blend and an opacity; the port and 02 §6.1 put both on PopLayer, so the Given names an API shape that does not exist. The behaviour and its test are right.",
     "follow_up": false, "status": "open"},
    {"id": "W2", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:91",
     "summary": "CB2 AC4 says 'the error the port fixes'; RenderError has no plan-validation variant, so DriverBlocked is reused and also reports ops of later deliveries: a malformed plan looks like a missing feature.",
     "follow_up": false, "status": "open"},
    {"id": "W3", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src (742 insertions)",
     "summary": "742 insertions against the ~700 ceiling and the 340 forecast (476 at x1.4): 501 are tests, 196 src, 45 the record. Needs the owner's explicit OK before merge.",
     "follow_up": false, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/backend.rs:216-226",
     "summary": "No delivery test runs execute after a mid-frame trim; CB1 pins only the read-back. Probe 6 shows DeviceLost and no stale surface.",
     "follow_up": false, "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:84",
     "summary": "The CB2 Flow line records tier: T0 (no Rust touched) for a delivery that edits critical Rust; inert under mutation: off, but it misdescribes the rigor.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 4, "killed": 4},
  "criteria": {"total": 8, "covered": 8},
  "size": {"lines": 742, "forecast": 476}, "report": "verify-report.md" } ] }
```

## Round 1 — CB2

- Delivery: `CB2` of `gafira-render-cpu`, branch `feat/gafira-render-cpu-cb2`, HEAD
  `08649a41804e1319a1f38bf8f2e9697b140ebaa6`, cut from `develop` `1d68c30`.
- Mode `standard`, depth **V2** (the card's Flow line says `verify: V1`; the owner asked for the
  acceptance criteria and independent hand mutants before the merge, so this round runs what V2
  owes). `mutation: off`: no `cargo-mutants` run happened, and per design `O10` that is not a
  finding — hand mutants and the per-commit gates replace it.
- Tier as recorded in the Flow line: `T0` (see `S2`; inert under `mutation: off`).
- Verdict: **PASS** — 0 CRITICAL, 3 WARNING, 2 SUGGESTION. Every gate exits 0, all eight acceptance
  criteria are covered, and the spot-check plus three independent hand mutants were all killed.
- Toolchain: cargo/rustc 1.93.1, clippy 0.1.93, cargo-deny 0.20.2. Disk at start: 164 GB free.

### Gates (delivery HEAD, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` |
| `cargo test --workspace --locked` | 0 | 194 targets, **2457 passed, 0 failed, 0 ignored** |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0: Format; CLI has no UI deps (ARQ-07); `forbid(unsafe_code)`; V5 layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports` |

The extractor `/tmp/gafira-policy-run.py` was re-checked against the current `ci.yml` before use:
the `policy` job has 10 `run:` steps, of which one is the toolchain install that the extractor
skips on purpose, leaving the nine policy checks it ran. The job's last step is
`cargo-deny-action`, covered by the `cargo deny check` row above.

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-v2-cb2-target` for the worktree,
`/tmp/gafira-v2-cb2-probe-target` for the probe export, `CARGO_INCREMENTAL=0`, both released with
`cargo clean --target-dir` when the round ended.

### Acceptance criteria

| AC | Proves it | Verdict | Does it prove the criterion as worded? |
|---|---|---|---|
| AC1 layer composites with its own blend and opacity | `tests::arq_119_a_layer_composites_with_its_own_blend_and_opacity`, plus `arq_119_a_normal_layer_at_full_opacity_changes_no_pixel` | covered | Yes. The expectation is `blend_f32(Multiply, backdrop, layer × 0.5)` taken from a **separate** run of the same fill without a layer, and the test asserts the fixture tells the result apart from `Normal` and from opacity 1. The card's "`PushLayer` with a blend and an opacity" is the port's `PopLayer` — see `W1`; the behaviour is the one 02 §6.1 states. |
| AC2 the opacity applies once | `tests::arq_119_the_opacity_applies_once_to_the_composite` | covered | Yes. Two opaque fills overlap; the test states the composite-once number `[0,0,0.5,0.5]` and the per-child number `[0.25,0,0.5,0.75]` and asserts the result is the first and not the second. Both numbers are the ones hand arithmetic gives. |
| AC3 the buffer is cleared at push | `tests::arq_117_a_pushed_layer_is_cleared_to_transparent` | covered | Yes. The second push takes the buffer the first pop returned inside the same tile; the left pixel keeps the first layer's half-white instead of the second layer's full white. |
| AC4 nested pops in reverse order; unbalanced refused | `tests::arq_119_nested_layers_pop_in_the_reverse_order_of_their_pushes`, `tests::arq_119_an_unbalanced_plan_is_refused` (both directions) | covered | The order half is proved: the inner layer composites into its parent, so what the parent held outside the inner fill survives (a wrong stack loses it), and the inner pop's own opacity shows. The refusal half is proved for both a pop with no push and a push with no pop, but pinned to `RenderError::DriverBlocked`, which the port does not fix — `W2`. |
| AC5 the sixteen modes are called as `gafira-raster`'s | `tests::arq_119_all_sixteen_modes_composite_the_layer_as_source` | covered | Yes, with the limit the design states: the expectation is the crate's own `blend_f32(mode, backdrop, layer)` and the swapped call is an explicit `assert_ne!` where the two differ, so a reimplemented formula that agrees would pass. What it pins is the mode that arrives and the operand order. |
| AC6 a reused buffer leaves no residue | `tests::arq_117_a_reused_layer_buffer_leaves_no_residue` | covered | Yes. The second tile's layer draws nothing; its pop must leave the page transparent, so any residue of the first tile's white fails it. |
| AC7 determinism and pool independence | `tests::arq_111_a_layered_plan_is_deterministic_and_the_pool_does_not_reach_the_pixels` | covered | Yes. Two fresh backends give identical bytes; a third run after a three-deep stack left three buffers in the pool equals the cold one. (The worker-count clause of ARQ-111 has no subject until `CB7`; the card does not claim it here.) |
| AC8 policy: `forbid(unsafe_code)`, V5, no clock, no file | command | covered | Yes. `crates/gafira-render-cpu/src/lib.rs:25` has `#![forbid(unsafe_code)]`; the `forbid(unsafe_code)` and V5 steps exit 0; `git grep -nE 'std::fs\|std::net\|std::thread::spawn\|std::time::(Instant\|SystemTime)::now\|std::env' -- 'crates/gafira-render-cpu/src/*'` and `git grep -n Locale -- crates/gafira-render-cpu` both exit 1; no `#[allow(` in `src/`. |

Product IDs of the card and of the criteria: `ARQ-119` and `ARQ-117` (named by the tests above),
`ARQ-111` (AC7), `ARQ-143` and `ARQ-44` (partial: `arq_143_the_layer_pool_is_accounted_and_trim_drops_it`,
and the CB1 cache/surface tests), `ARQ-02` (AC8). All have a test or command that names them.

### Artifact integrity (fast lane, step 3c)

- `1d68c30 docs(gafira-render-cpu): slice card for CB2` touches only
  `specs/changes/gafira-render-cpu/apply-progress.md` (28 insertions, 1 file).
- The card block is byte-identical between `1d68c30` and HEAD: the only implementer commit that
  touches the file, `08649a4`, starts its hunk at line 98, in the `## CB2 (tdd-apply)` record; the
  card (lines 83–97) is untouched.
- Delivery diff against its base `1d68c30`: 5 files, 742 insertions, 52 deletions — exactly the
  figure the record states.

### Independent probes (step 1c, before reading the implementer's tests)

Derived from the card and 02 §6.1 and run on a scratch export (`git archive` of HEAD into
`/tmp/gafira-cb2-probe`, files `touch`ed), in a probe module of its own, not the delivery's tests.
All nine pass.

| Probe | Criterion | What it proved |
|---|---|---|
| 1 | AC1, AC5, AC2 | For all sixteen modes: a layer over an opaque backdrop gives `blend_f32(mode, backdrop, layer × 0.5)` on the centre pixel, and the result is not the `Normal` answer where the two differ. |
| 2 | AC2 | Two opaque shapes in one layer at 0.5 give `[0,0,0.5,0.5]`, not the per-child `[0.25,0,0.5,0.75]`. |
| 3 | AC3, AC6 | A second tile's empty layer is transparent after a first tile returned a white buffer. |
| 4 | AC4 | Nested layers keep the parent outside the inner fill; both unbalanced directions are errors (`DriverBlocked`, printed by the probe). |
| 5 | AC7 | Two runs identical; a plan run after a three-deep stack (pool holding three buffers) identical to the cold run. |
| 6 | ARQ-143, ARQ-44 | `memory_usage` includes the pooled buffer (exactly one 16×16 `f32` buffer more than a layerless frame); `trim` at the current usage is a no-op; `trim(0)` leaves 0 bytes; **a trim during an open frame makes the next `execute` answer `DeviceLost` and `read_back` fail — no stale buffer, no later tile into the old surface**. |
| 7 | AC3, AC6 | A pooled buffer taken for a smaller tile is resized and cleared; no row of the earlier tile survives. |
| 8 | risk in the record | The stated opacity clamp holds: 2.0 equals 1.0, 0.0 paints nothing, and a NaN opacity is `RenderError::Buffer(NonFinite)`. |
| 9 | AC7, cache path | A layered tile answered from the cache gives the same bytes as the cold render. |

Comparison with the implementer's tests: no criterion is proved only by a weak test. The delivery's
tests are at least as strict as the probes; AC1 and AC5 are stronger in the delivery (they also
assert the discriminating neighbours), and AC4 is stricter (it pins the error variant).

### Hand mutants (step 4b)

Applied on the scratch export against the green HEAD, the crate suite run with
`cargo test -p gafira-render-cpu --locked`, then reverted; `layer.rs` and `backend.rs` compare
identical to the delivery afterwards.

| Mutant | Wrong behaviour | Result | Killed by |
|---|---|---|---|
| Spot-check H4 (recorded) | a `PopLayer` with no `PushLayer` is accepted (`return Ok(())`) | killed | `arq_119_an_unbalanced_plan_is_refused` |
| M1 (mine) | the end-of-tile balance check is deleted, so a `PushLayer` left open is silently dropped | killed | `arq_119_an_unbalanced_plan_is_refused` (the push-with-no-pop case, which H4 did not cover) |
| M2 (mine) | the pop's opacity scales the layer's colour but not its alpha | killed | `arq_119_a_layer_composites_with_its_own_blend_and_opacity`, `arq_119_the_opacity_applies_once_to_the_composite`, `arq_119_nested_layers_pop_in_the_reverse_order_of_their_pushes`, `arq_117_a_pushed_layer_is_cleared_to_transparent` |
| M3 (mine) | `Layers::bytes` counts the open layers but not the pool | killed | `arq_143_the_layer_pool_is_accounted_and_trim_drops_it` |

4 run, 4 killed. No survivor. The recorded list (`apply-progress.md:129-140`) has ten rows for the
eight rules — two applied in both directions — and the one I spot-checked is the rule it names.

### Findings

| Id | Severity | Class | Ref | Summary | Status |
|---|---|---|---|---|---|
| W1 | WARNING | ambiguity | `apply-progress.md:88` (card AC1) | CB2 AC1 gives `PushLayer` a blend and an opacity; the port (`gafira-ports/src/render.rs:64-69`) and 02 §6.1 put both on `PopLayer`, so the Given names an API shape that does not exist. The delivery composites with the pop's pair, which is the only reading the frozen port allows; the product spec agrees. Only the card needs the fix. | open |
| W2 | WARNING | ambiguity | `apply-progress.md:91` (card AC4) | CB2 AC4 says "the error the port fixes". `RenderError` has no plan-validation variant (`DeviceLost`, `OutOfMemory`, `AdapterMissing`, `DriverBlocked`, `Effect`, `Buffer`, `Cancelled`), so the criterion has no defined answer. The delivery reuses `DriverBlocked` (`layer.rs:107`, `backend.rs:296-298`), which is also its answer for the ops of `CB4`/`CB5` (`backend.rs:338-340`), so **a caller that gets `DriverBlocked` cannot tell a malformed plan from a feature this backend does not have**. The refusal itself is needed and is correctly reachable only because the port has no variant; the choice is defensible but conflates two conditions, and the test now pins the reuse rather than a spec rule. The port needs a plan/unsupported variant in a later change, or AC4 should name `DriverBlocked`. | open |
| W3 | WARNING | implementation | `crates/gafira-render-cpu/src` | 742 insertions (52 deletions) against the ~700-line ceiling and the design's 340-line forecast (476 at x1.4, so 1.6×). The split explains it: `layer.rs` 148 + `backend.rs` 47 + `lib.rs` 1 = 196 src lines, `tests.rs` 501, the record 45. The code undershoots its 340 forecast; the tests overshoot, as the x1.4 rule predicts. Above the ceiling it needs the owner's explicit OK before merge — a coordinator/owner decision, not a defect of the code. | open |
| S1 | SUGGESTION | implementation | `backend.rs:216-226` | No delivery test drives `execute` after a mid-frame `trim`; CB1's `arq_143_memory_usage_is_the_surface_plus_the_cache_and_trim_drops_to_the_budget` pins only that the read-back fails. Probe 6 shows the code is right (`DeviceLost`, no stale surface, no later tile into the old buffer), so this is a missing assertion, not a defect. | open |
| S2 | SUGGESTION | ambiguity | `apply-progress.md:84` | The CB2 Flow line records `tier: T0`, which the tier table defines as "no Rust touched", for a delivery that edits critical Rust code. With `mutation: off` the field changes nothing, but a reader of the Flow line is told the wrong rigor. | open |

### Observations (not findings)

- **The layer pool.** It keeps every buffer a pop returns, so its size converges to the deepest
  stack the frame drew (`layer.rs:18-22`, `85-93`); `Layers::bytes` counts the open layers and the
  pool (`layer.rs:57-63`); `memory_usage` adds it to the surface and the observed cache bytes
  (`backend.rs:208-214`); `trim` drops it with the surface and the cache accounting
  (`backend.rs:216-226`). Probe 6 and mutant M3 confirm both accounting and dropping, and that a
  trim cannot leave a later tile with a stale buffer. The all-or-nothing shape of `trim` (a
  governor trim of one byte over budget destroys the open frame) is CB1's verified decision, not
  this delivery's.
- **`RenderError::DriverBlocked`**: the refusal is reachable in exactly the two places the plan can
  be malformed, and nowhere else — but it is the same variant as "this backend cannot render that
  op yet". See `W2`; this is the one place where the delivery's error choice hides a distinction
  the caller may need.
- **Develop moved after the branch was cut.** `develop` is now `c243fb1 docs(gafira-render-cpu):
  spike the cost of one frame`, which adds `specs/changes/gafira-render-cpu/spike-frame-cost.md`
  (148 lines). `git diff develop` therefore reports 6 files and 200 deletions; the delivery's own
  diff, against its base `1d68c30`, is the 5 files / 742 / 52 the record states. Nothing to fix —
  a `--no-ff` merge keeps both.
- **Size arithmetic.** The design forecast for `CB2` is 340 (475 at x1.4, and the record says 475;
  `size.forecast` above uses 476 as the rounded x1.4 figure). Code alone is below forecast; tests
  and the record carry the overrun.
- **`O10` says `verify` runs at V1.** This round ran V2 at the owner's request, before the merge;
  that is a deliberate override, not a breach.

next: tdd-apply
