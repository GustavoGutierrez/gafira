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

## Round 1 — CB3

```json
{ "schema": 1, "change": "gafira-render-cpu", "delivery": "CB3", "rounds": [ {
  "round": 1, "commit": "24d4045dc8dfcb5628b30aeee7f01a9c8da521f2", "mode": "standard", "tier": "T2", "depth": "V2",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 4, "suggestion": 3},
  "findings": [
    {"id": "W4", "severity": "WARNING", "class": "architecture",
     "ref": "crates/gafira-render/src/planner.rs:295-315, crates/gafira-render-cpu/src/surface.rs:103",
     "summary": "A page with more than one root loses every root but the last (probe: red 0, blue 256 of 256). Pre-existing CB1/PC1; O12 on develop allocates it to CB3b. Not CB3.",
     "follow_up": true, "status": "open"},
    {"id": "W5", "severity": "WARNING", "class": "architecture",
     "ref": "crates/gafira-render/src/page.rs:211-239, crates/gafira-render-cpu/src/surface.rs:144",
     "summary": "render() hands (0,0,w,h) to read_back while the surface origin is the page offset: an opaque page at region (4,6,68,70) returns 616/4096 transparent px. Pre-existing; O13 allocates CB3b.",
     "follow_up": true, "status": "open"},
    {"id": "W6", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/tests/arq_42_comparison.rs:249",
     "summary": "M4 (preview at scale 2.0) is unobservable here: equivalent while both requests share SCALE and the surface is page-sized; CB4's image sampling makes it observable.",
     "follow_up": true, "status": "open"},
    {"id": "W7", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/tests/arq_42_comparison.rs (744) + manifest and lock (6)",
     "summary": "750 work lines (805 with the record) against the 300 forecast, 420 at x1.4: 1.79x and 1.92x, above the ~700 signal. Needs the owner's explicit OK before merge.",
     "follow_up": false, "status": "open"},
    {"id": "S3", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:157",
     "summary": "The card cites FD-55 in specs/05; the row lives in specs/04:648. The criterion is covered, the pointer is wrong.",
     "follow_up": false, "status": "open"},
    {"id": "S4", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-render-cpu/tests/arq_42_comparison.rs:732-743",
     "summary": "arq_113_one_altered_byte_fails_on_this_platform asserts differing==1 in Exact mode whatever current_mode() says, so a matrix target whose golden is not byte-identical would fail. Passes here.",
     "follow_up": false, "status": "open"},
    {"id": "S5", "severity": "SUGGESTION", "class": "architecture",
     "ref": "design.md:384-400 (D9), apply-progress.md:213",
     "summary": "D9 named gafira-testkit::golden::compare; the delivery wrote its own (recorded) because Mismatch::Pixels carries no worst channel. Amend D9 or extend the testkit before CB6 reuses it.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 5, "killed": 5},
  "criteria": {"total": 7, "covered": 7},
  "size": {"lines": 805, "forecast": 420}, "report": "verify-report.md" } ] }
```

- Delivery: `CB3` of `gafira-render-cpu`, branch `feat/gafira-render-cpu-cb3`, HEAD
  `24d4045dc8dfcb5628b30aeee7f01a9c8da521f2`, cut from `develop` `76b546f` (the slice-card commit,
  which is also the merge base). Not pushed, not merged.
- Mode `standard`, depth **V2** (the card's Flow line says `verify: V2`). `mutation: off`: no
  `cargo-mutants` run happened and per `O10` that is not a finding; the rigor is the hand mutants
  and the per-commit gates, so this round owes V2's own independent probes and mutant budget.
- Tier as recorded: `T2`, inert under `mutation: off`.
- Verdict: **PASS** — 0 CRITICAL, 4 WARNING, 3 SUGGESTION. Every gate exits 0, all seven acceptance
  criteria are covered, and the spot-check plus four independent hand mutants were all killed. The
  single question of the round — does the comparison prove `ARQ-42` or only that the test agrees
  with itself — is answered **no self-agreement**: both sides are distinct frames on distinct
  backends, the preview's bytes come from the backend's own `Rgba8` read-back, and an independently
  written conversion (including a hand-written sRGB curve) reproduces them.
- Toolchain: cargo/rustc 1.93.1, clippy 0.1.93, cargo-deny 0.20.2. Disk: 155 GB free at start,
  148 GB after the build folders were freed.

### Gates (delivery HEAD, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` in 21.57 s |
| `cargo test --workspace --locked` | 0 | **196 test binaries, 2479 passed, 0 failed, 0 ignored**; the delivery's own binaries are `arq_42_comparison` 11 passed and the crate's unit tests 28 passed |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0: Format; CLI has no UI deps (ARQ-07); `forbid(unsafe_code)`; V5 layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports` |

The extractor `/tmp/gafira-policy-run.py` was re-checked against the current `ci.yml`: job `policy`
has 12 steps — checkout, `Install toolchain (rust-toolchain.toml)` (skipped on purpose by the
extractor), the nine `run:` checks above, and `cargo-deny-action` (covered by the `cargo deny check`
row). Nothing applicable was skipped.

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-cb3-v2-target` for the worktree and
`/tmp/gafira-cb3-v2-probe-target` for the probe export, both `CARGO_INCREMENTAL=0`, both released
with `cargo clean --target-dir` at the end (7.0 GiB and 220 MiB). The worktree is clean; no source
file was edited by this round.

### The single question: a comparison, or the test agreeing with itself?

Derived from `ARQ-42`, `ARQ-113`, 02 §10 and FD-55 (b)/(c) **before** reading the delivery's tests,
and probed on a `git archive` export (`/tmp/gafira-cb3-probe`) in a probe file of my own.

| Way it could be hollow | Verdict | What settles it |
|---|---|---|
| The preview's bytes quantised by the test's own function instead of coming out of the backend's `Rgba8` path | **refuted** | `preview()` renders a `Preview` `GraphPageRenderer` and takes `PixelBuf::Rgba8` from its `read_back` (`arq_42_comparison.rs:467-477`); only the export is converted, by `encode_export` (`:481-491`). Probe 1 reproduces those bytes with a §10 conversion written from the spec, not imported: worst delta **0/255** on 4 096 pixels, i.e. the backend's quantisation and an outside implementation of the rule agree byte for byte. |
| Both sides deriving from one renderer or one read-back | **refuted** | `renderer()` builds a fresh `GraphPageRenderer` on a fresh `CpuBackend::new()` and a fresh `ContentTileCache` per call (`:443-451`); `preview()` and `export_srgb8()` each call it, so no backend, cache, frame or surface is shared. `GraphPageRenderer::render` opens and abandons one frame per call (`page.rs:203-230`). |
| A scene flat enough that every pixel matches trivially | **refuted, and guarded** | The scene is 3 shapes (two ellipses, anti-aliased), a `Screen` fill, a `Multiply` fill, a group at 0.75 and a group at 0.6 nested inside it (`:342-399`). The guard `distinct(&preview) >= 8` (`:639-643`) bites — mutant HM3 flattens the scene and the guard fails with `1 distinct pixels`. Probe 1's own scene measures 87 distinct colours and 4 096/4 096 opaque. |
| A golden that regenerates itself, or that is compared with a tolerance it defines itself | **refuted** | The golden is `include_bytes!("golden/arq_113_scene.rgba8")` (`:709`) and the test tree has no write path (AC5 below). The tolerance is not the file's: `current_mode()` picks `Mode::Exact` (`allowed = 0`) on `x86_64-unknown-linux-gnu` and `Mode::Tolerant` (`pixels / 1000`) elsewhere (`:82-108`, `:190-194`), which is ARQ-113's and §10's wording, not metadata stored with the image. Probe 5 shows the golden is a real image (106 distinct pixels, 572 opaque, 3 524 transparent), so "one altered byte fails" is not vacuous. |

Two limits the comparison does **not** prove, both stated by the design and neither a defect:
the sRGB **curve** is shared on purpose (`LinearRgba::to_srgb` on both sides, `D9`), and the
scene set of `CB3` is not arbitrary documents (`design.md`, "ARQ-42 is reachable"). Probe 6 pushes
back on the first: re-encoding the export with a hand-written IEC 61966-2-1 curve (no
`gafira_color`) still gives `differing(≥2/255) = 0` and `worst = 1/255`, so the agreement is a
raster agreement and not an artefact of one shared function.

### Acceptance criteria

| AC | Proves it | Verdict | Does it prove the criterion as worded? |
|---|---|---|---|
| AC1 the two qualities agree within ARQ-42's budget at scale 1.0 | `arq_42_the_preview_and_the_export_agree_on_the_scene` (11-test binary, this one green), plus probes 1 and 6 | covered | Yes. 4 096 pixels, `report.differing == 0`, `allowed == 4` (0.0977 %, inside 0,1 %), and the metadata is asserted: `distinct >= 8` and every pixel `a == 255`. Probes 1 and 6 reproduce the agreement independently with a different scene. |
| AC2 the export pixel is encoded in §10's order, hand-computed byte for byte | `arq_117_the_export_pixel_is_encoded_in_the_order_of_the_comparison_space` (3 rows) | covered | Yes. I recomputed all three rows by hand (`[0.5,0.25,0.1,1.0] → [188,137,89,255]`, `[0.25,…,0.5] → [94,62,45,128]`, `[0.05,0.125,0.0125,0.25] → [31,47,16,64]`) and they are correct; the third row is a rounding boundary that a truncating quantise misses (HM1), and the second row is where the premultiply rounding shows (M2). |
| AC3 alpha 0 answers four zeroes, never divides | `arq_117_a_zero_alpha_pixel_answers_four_zeroes` (3 rows, incl. `a = 0.001`), `fd_55_a_transparent_render_is_zero_where_nothing_is_drawn` | covered | Yes. Probe 4 confirms on a real render that both sides give the identical 3 840 zero pixels of 4 096 and that the region draws content. The division itself is not byte-observable (the record says so); what is pinned is the output, which is the rule. |
| AC4 one altered byte fails on this platform | `arq_113_one_altered_byte_fails_on_this_platform`, `arq_113_the_stored_golden_is_the_scene_on_its_platform`, `arq_113_the_golden_is_exact_only_on_the_platform_that_generates_it` | covered | Yes, and the stored golden passes **exactly** on this platform (the suite is green and `current_mode()` is `Exact` here), which is also the evidence that it was generated on `x86_64-unknown-linux-gnu`. See `S4` for the cross-platform reading of the altered-byte test. |
| AC5 no test writes the golden | command | covered | Yes. `git grep -nE 'fs::(write\|File::create\|OpenOptions\|rename\|copy)\|std::fs' -- crates/gafira-render-cpu/tests/` → exit 1; `GOLDEN` enters the suite only as `include_bytes!`; size 16 384 = 64 × 64 × 4 as the constants fix. |
| AC6 three nested semi-transparent layers keep the budget and the report names the worst channel and the count | `arq_42_three_nested_semi_transparent_layers_agree`, `arq_42_the_report_names_the_worst_channel_and_the_count`, `arq_42_the_failure_report_quotes_the_worst_channel` | covered | Yes. `differing == 0` at 0.5/0.5/0.5 (XP2's worst), and the fixture is shown to nest by `assert_ne!(nested, opaque)` at 1.0/1.0/1.0 (M8 kills it). The report is pinned to `worst == Blue`, `differing == 2` of 4 pixels, `max_delta == 3`, and the panic text. |
| AC7 policy: `forbid(unsafe_code)`, V5, no clock, no file | command | covered | Yes. `lib.rs:25` has `#![forbid(unsafe_code)]`; the `forbid(unsafe_code)` and V5 steps exit 0; `git grep -nE 'std::fs\|std::net\|std::thread::spawn\|std::time::(Instant\|SystemTime)::now\|std::env' -- 'crates/gafira-render-cpu/src/*'` → exit 1; the new test reads no environment (`current_mode` is built from `cfg!`). The V5 row is unchanged and `gafira-render` is a **dev** edge only (`cargo tree -e normal,build` names it zero times, and the V5 step passes). |

Product IDs of the card: `ARQ-42` (nine references in the test names and comments), `ARQ-113` (six),
`FD-55` (seven), plus `ARQ-117` (the rule AC2 and AC3 name). All are named by a test; none is
deliberately left out. The card's source line for `FD-55` points at `specs/05` while the row is in
`specs/04:648` — the criterion is covered, the pointer is wrong (`S3`).

### Artifact integrity (fast lane, step 3c)

- `76b546f docs(gafira-render-cpu): slice card for CB3` touches only
  `specs/changes/gafira-render-cpu/apply-progress.md` (+15, one file).
- The card block (lines 155–169) is byte-identical between `76b546f` and HEAD: the only implementer
  commit that touches the file, `24d4045`, has its hunk at `@@ -168,0 +169,55 @@`, entirely inside
  the `## CB3 (tdd-apply)` record that follows the card.
- Delivery diff against its base `76b546f`: **5 files, 805 insertions, 0 deletions** — exactly the
  figure the record states. Nothing outside `crates/gafira-render-cpu/` and that record changed;
  `src/` is untouched.
- `develop` moved during the round (to `5789465`), which is why `git diff develop` reports 12 files
  and 386 deletions: those are inbound timeline commits plus `1c9c3f8`, not this delivery. See
  "Observations".

### Independent probes (step 1c, before reading the delivery's tests)

Six probes in `/tmp/gafira-cb3-probe/crates/gafira-render-cpu/tests/verify_probe_v2.rs`, on a
`git archive` export of HEAD (files `touch`ed), each with its own conversion written from §10.

| Probe | Criterion | Measurement |
|---|---|---|
| 1 — two qualities, two renderers, one region | AC1, AC2 | Non-flat scene of my own (two anti-aliased ellipses, one rect, a semi-transparent overlap): `differing(≥2/255) = 0/4096`, **worst = 0/255**, 87 distinct pixels, 4 096/4 096 opaque. The backend's `Rgba8` bytes and my independent §10 conversion agree byte for byte. |
| 2 — two roots at the top level | claim 1 | Two disjoint opaque rects, preview read-back: **red = 0 pixels, blue = 256**. Only the last root survives. |
| 3 — a region with an offset origin | claim 2 | Opaque page, region `(4,6,68,70)`: **616 of 4 096** pixels come back `(0,0,0,0)` — the 4-column × 6-row band, exactly `4 096 − 60 × 58`. |
| 4 — FD-55 (b) on a real render | AC3 | Transparent page with one shape: 3 840 transparent pixels on the preview and the same 3 840 after converting the export; the region draws content. |
| 5 — the stored golden is an image | AC4, ARQ-113 | 106 distinct premultiplied sRGB8 pixels, 572 opaque, 3 524 transparent, 16 384 bytes. Not flat, not blank. |
| 6 — the export under a hand-written sRGB curve | AC1, AC2, `D9` | `differing(≥2/255) = 0`, `worst = 1/255` against the preview's own bytes. The curve is not hiding a self-agreement. |

Comparison with the delivery's tests: no criterion is proved only by a weak test. The delivery's
comparison is stronger than my probe on the fixed scene (it pins `differing == 0` on a scene with
two nested groups and two blend modes) and weaker nowhere; my probes are stronger on independence
(probe 6's curve) and on the two guard predicates (probes 2, 3, 5).

### Hand mutants (step 4b)

Applied to the export's copy of `arq_42_comparison.rs`, the named test run, the file restored from
a pristine copy each time. The list `tdd-apply` recorded is **not** repeated: one item is
spot-checked and four are mine.

| Mutant | Wrong behaviour | Result | Killed by (and the observed failure) |
|---|---|---|---|
| Spot-check **M2** (recorded) | `(c × a) / 255` instead of `(c × a + 127) / 255` | killed | `arq_117_the_export_pixel_is_encoded_in_the_order_of_the_comparison_space`: `left: [94, 62, 44, 128]`, `right: [94, 62, 45, 128]` — exactly the recorded assertion |
| **HM1** (mine) | the quantise step truncates instead of rounding to nearest (`floor(v × 255)` rather than `floor(v × 255 + 0,5)`) | killed | the AC2 opaque row: `left: [187, 136, 89, 255]`, `right: [188, 137, 89, 255]`. A different rule from M2 (quantisation, not premultiplication), so it is not a repeat. |
| **HM2** (mine) | the tolerance boundary excludes a delta of exactly 2/255 (`delta > 2`) | killed | `arq_113_the_budget_is_one_pixel_in_a_thousand`: `one pixel at 2/255: left: 0`, `right: 1` — ARQ-113's "1 píxel a 2/255 pasa" is exactly the boundary |
| **HM3** (mine) | the fixture is flattened to one opaque full-page rect | killed | the distinctness guard: `the scene is not one flat fill: 1 distinct pixels`. The "at least 8 distinct pixels" guard really bites. |
| **HM4** (mine, extra, for the other guard) | the request always asks for transparency, so the background is not painted | killed | the opacity guard: `without transparency the page background makes every pixel opaque`. The "every pixel opaque" guard really bites — and probe 3 shows its predicate is exactly what fails on the coordinate-space defect. |

5 run, 5 killed, no survivor. `git status` in the worktree stayed clean throughout (the mutants live
in the scratch export only).

### The two defect claims, verified independently

Both are **real**, with my own numbers rather than the record's.

1. **A page with more than one root loses every root but the last — confirmed.** Mechanism, read and
   not assumed: `crates/gafira-render/src/planner.rs:295-315` emits one task per `(tile, root)` and
   every one of them carries the same `region` with only that root's ops; `CpuBackend` writes a
   task's tile over the surface as the whole content of its region
   (`src/backend.rs:104` → `surface.rs:103-138`, a `copy_from_slice`). The second root's tile
   erases the first. Probe 2: red **0**, blue **256** of 256.
   Owner: **`CB3b`** (decision `O12` on `develop`), which puts the composition in the planner;
   the contract half is `PC1`'s (`TileTask` is "the tile's own content") and the erase half is
   `CB1`'s write. Not `CB3`, whose card forbids any change to `gafira-render`/`gafira-ports`.
2. **`GraphPageRenderer::render` and `CpuBackend::read_back` disagree on the region's coordinate
   space — confirmed.** `render` computes `target_region(request.region)` = `(0,0,w,h)`
   (`page.rs:253`) while the surface sits at `origin = (request.region.x0, y0)` (`page.rs:211-215`)
   and `Surface::read` treats its argument as page space (`surface.rs:144-150`, via
   `page_rect`/`source_index`). Reading `(0,0,64,64)` from a surface at `(4,6)` loses 4 columns and
   6 rows: probe 3 gives **616 of 4 096** transparent, the same number the record reports.
   Consequence, and it is the important part: both qualities lose the **same** band, so AC1 would
   have passed on a wrong page; only the opacity guard catches it (HM4 proves that guard bites).
   Owner: **`CB3b`** (decision `O13`), with the fix in `Surface::read`/`write`; the ambiguous
   contract is `gafira-ports`' `read_back`, which does not say which space it answers in.

Neither claim is absorbed into this round, and neither is a `claim`-class finding: both reproduced.

### `M4` (the preview at scale 2.0 survives)

**A limit that `CB4` makes observable, not a gap in `CB3`'s acceptance.** AC1 fixes both sides at
`scale 1.0`, and both requests come from one `request()` with one `SCALE` (`:249`, `:455-463`), so
the two qualities cannot disagree on the zoom by construction. Within this cut the level of detail
changes content keys and the tile grid but not a pixel, because the surface and the coverage are
page-sized and nothing samples an image yet; the mutant is **equivalent here**, which is why it
survives. It becomes observable with `CB4`'s `DrawImage` resampling, where the zoom changes the
sampled footprint. Classified `W6` as a WARNING with `follow_up: true` (CB4), not as a critical
survivor: the card's own hand-mutant rule ("the zoom being the same on both renderers") has no
observable subject until then, and the region half of that rule is covered (the recorded M3).

### Size

`git diff --shortstat $(git merge-base develop HEAD)..HEAD` → **5 files changed, 805 insertions(+)**:
`tests/arq_42_comparison.rs` 744, `Cargo.toml` 5, `Cargo.lock` 1, the record 55, and the golden is
a 16 384-byte binary that git does not count as lines. Work excluding the record: **750**. Against
the card's 300 (420 at ×1.4) that is 2.5× the forecast and **1.92×** of the ×1.4 figure, above the
~700-line signal. The 840 (or 846) insertion figure does **not** reproduce as this delivery's diff;
846 come from `git diff develop..HEAD`, where `develop` has advanced to `5789465` and the diff
carries 386 inbound deletions — see below. The coordinator decides; `W7` records it.

### Findings

| Id | Severity | Class | Ref | Summary | Status |
|---|---|---|---|---|---|
| W4 | WARNING | architecture | `planner.rs:295-315`, `surface.rs:103` | A page with more than one root loses every root but the last (probe: red 0, blue 256 of 256). Real, pre-existing in `CB1`/`PC1`, and already allocated to **`CB3b`** by `O12` on `develop`. Not `CB3`. | open |
| W5 | WARNING | architecture | `page.rs:211-239`, `surface.rs:144` | `render()` hands `(0,0,w,h)` to `read_back` while the surface origin is the page offset: an opaque page at region `(4,6,68,70)` returns 616/4 096 transparent pixels. Real, pre-existing; **`CB3b`** owns it per `O13`. Both sides lose the same band, so the comparison alone would not have seen it — the opacity guard did. | open |
| W6 | WARNING | implementation | `arq_42_comparison.rs:249` | The recorded `M4` (preview at 2.0) is equivalent in this cut: both requests share `SCALE` and the surface is page-sized. `CB4`'s image sampling makes it observable. Follow-up, not a gap in AC1. | open |
| W7 | WARNING | implementation | `arq_42_comparison.rs` 744, manifest+lock 6 | 750 work lines (805 with the record) against 300 forecast, 420 at ×1.4 — 1.79× and 1.92×, above the ~700 signal. Needs the owner's explicit OK before merge. | open |
| S3 | SUGGESTION | ambiguity | `apply-progress.md:157` | The card cites FD-55 in `specs/05`; the row is `specs/04:648`. Criterion covered, pointer wrong. | open |
| S4 | SUGGESTION | implementation | `arq_42_comparison.rs:732-743` | `arq_113_one_altered_byte_fails_on_this_platform` asserts `differing == 1` in `Mode::Exact` regardless of `current_mode()`, so on a matrix target whose golden is not byte-identical the count would be larger and the test red. It passes on this platform; the CI matrix is not runnable here. | open |
| S5 | SUGGESTION | architecture | `design.md:384-400` (`D9`), `apply-progress.md:213` | `D9` named `gafira_testkit::golden::compare`; the delivery wrote its own and recorded why. The justification is factual — `Mismatch` is `Size \| Channels \| Pixels { differing, allowed }` (`gafira-testkit/src/golden.rs:180-187`), so AC6's worst-channel report cannot come from it — but `D9` now describes code that does not exist, and `CB6`'s critical list reuses "the encode path (same rules as CB3)". Amend `D9` or extend the testkit before then. | open |

### Observations (not findings)

- **No self-agreement, and that is the round's answer.** The preview is the backend's own `Rgba8`
  read-back of its own `f32` accumulation; the export is a second, independent frame's `RgbaF32`
  surface converted once by the test. The comparator carries its own `premultiply8` and `quantise8`
  on purpose (`:35-45`) rather than calling `gafira_raster::premultiply_u8`, and my probe 1/6 land
  on the backend's bytes from outside. What the test proves is the raster's agreement, exactly as
  `ARQ-42` words it; what it cannot prove is the sRGB curve (shared by `D9`) — probe 6 shows the
  curve is nonetheless standard to ≤ 1/255.
- **`ARQ-113` is the mechanism only, as designed.** One stored golden of one scene (106 distinct
  pixels), exact on its platform, no write path. The suite ARQ-113 describes needs the node kinds
  and the exporters that do not exist (`design.md`, "ARQ-113 cannot be closed by this change"), and
  the card asks only for the mechanism.
- **`develop` moved during the round**, from the branch point `76b546f` to `5789465`: the timeline
  retime fixes plus `1c9c3f8 docs(gafira-render-cpu): decide the multi-root task, the read_back
  space and the single quantisation`, which adds `O12`, `O13` and `O14` and the deliveries `CB3b`
  and `CB8`. That commit assigns `W4` and `W5` to `CB3b` (before `CB4`) and supersedes nothing in
  this delivery's acceptance. The branch's card commit is unaffected, and nothing of this round's
  verdict changes: `CB3` is verified against `76b546f` and its own card.
- **The record's numbers reproduce.** 196 binaries / 2 479 passed / 0 failed / 0 ignored; 805
  insertions against the base; 616 of 4 096; 64 of 4 096 for the region shift; the golden's
  16 384 bytes; and the hand-mutant table's assertions, one of which I re-ran (M2) and got byte for
  byte. The one figure that does not reproduce is the 840-insertion count, which is the
  `develop`-relative diff (846/386) rather than the delivery's.
- **The `DriverBlocked` conflation from `W2` of `CB2` is untouched** by this delivery:
  `src/` is not modified, and the new code cannot reach a `RenderError` because it only renders.

next: tdd-apply
