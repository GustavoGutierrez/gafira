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

## Round 1 — CB3b

```json
{ "schema": 1, "change": "gafira-render-cpu", "delivery": "CB3b", "rounds": [ {
  "round": 1, "commit": "c07f6a3f7839915be9d961fb9071938f7b06b8bc", "mode": "standard", "tier": "T2", "depth": "V2",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 2, "suggestion": 1},
  "findings": [
    {"id": "W8", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render/src/{key,planner}.rs, crates/gafira-render-cpu/src/{surface,tests}.rs, crates/gafira-render/tests/page_renderer.rs, crates/gafira-render-cpu/tests/arq_42_comparison.rs",
     "summary": "576 insertions (522 of work at 5dd0691 + 54 of record) against the card's 180 forecast, 250 at x1.4: 2.9x, 2.1x with the factor. Below the ~700 signal; the card leaves the call to the coordinator.",
     "follow_up": false, "status": "open"},
    {"id": "W9", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render/src/planner.rs:299-320",
     "summary": "A page with no roots now plans one task per region (the background alone): right, but unpinned. Skipping the task leaves all 118 tests of the two crates green; probe P4 is the only killer, and the record states the gap.",
     "follow_up": true, "status": "open"},
    {"id": "S6", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-render-cpu/tests/golden/arq_113_scene.rgba8, arq_42_comparison.rs:399-403",
     "summary": "The stored golden's scene is one wrapper root and its region is the page origin, so the fix leaves it untouched and the composition now has no golden coverage across platforms. The multi-root re-generation is the card's coordinator follow-up.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 4, "killed": 3},
  "criteria": {"total": 6, "covered": 6},
  "size": {"lines": 576, "forecast": 250}, "report": "verify-report.md" } ] }
```

- Delivery: `CB3b` of `gafira-render-cpu`, branch `feat/gafira-render-cpu-cb3b`, HEAD
  `c07f6a3f7839915be9d961fb9071938f7b06b8bc`, cut from `develop` `8e1a468`. Three commits:
  `4590c2b` (the read-back space), `5dd0691` (the multi-root composition), `c07f6a3` (the record).
  Not pushed, not merged. The card commit `8e1a468` is the merge base.
- Mode `standard`, depth **V2** (the card says `verify: V2`). `mutation: off`: no `cargo-mutants`
  run happened and per `O10` that is not a finding. The rigor is the per-commit gates and the hand
  mutants, so this round owes V2's own independent probes and its own mutant budget; the card's
  `tier: T2` is inert under `off`.
- Verdict: **PASS** — 0 CRITICAL, 2 WARNING, 1 SUGGESTION. Every gate exits 0, all six acceptance
  criteria are covered by probes I wrote before reading the delivery's tests, and 3 of 4 hand
  mutants are killed. Both defects of `CB3` (`W4`, `W5`) are gone, end to end and at the unit level,
  with the defect's own signature reproduced as the negative control.
- Toolchain: cargo/rustc 1.93.1, clippy 0.1.93, cargo-deny 0.20.2. Disk: `df -h /` at start 182 GB
  free, 173 GB after the build folders were freed.

### Gates (delivery HEAD, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` in 29.04 s |
| `cargo test --workspace --locked` | 0 | **196 test binaries, 2489 passed, 0 failed, 0 ignored**; the delivery's own binaries are `gafira-render` lib 71, `page_renderer` 3, `gafira-render-cpu` lib 30, `arq_42_comparison` 14 |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0: Format; CLI has no UI deps (ARQ-07); `forbid(unsafe_code)`; V5 layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports` |

The extractor `/tmp/gafira-policy-run.py` was re-checked against this `ci.yml`: job `policy` has 12
steps — checkout, `Install toolchain` (the one step the extractor skips), the nine `run:` checks
above, and `cargo-deny-action` (covered by the `cargo deny check` row) — and it reads no other job,
so the `SKIP` name for the test matrix's target guard is harmless. The record's claim of "nine
`run:` steps after the toolchain install" reproduces (10 `run:`, 1 skipped for the install).

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-cb3b-v2-wt-target` for the worktree,
`/tmp/gafira-cb3b-v2-target` for the probe export at HEAD, `/tmp/gafira-cb3b-v2-base-target` for the
negative control at the base, all with `CARGO_INCREMENTAL=0` and none shared. The worktree is clean
(`git status` empty) and no source file was edited by this round; the only file written is this one.

### Artifact integrity (fast lane, step 3c)

- `8e1a468 docs(gafira-render-cpu): slice card for CB3b` touches only
  `specs/changes/gafira-render-cpu/apply-progress.md` (+14), as G0 requires.
- No implementer commit changed the card: the delivery's diff against `8e1a468` removes **zero**
  lines from that file and only adds the `## CB3b (tdd-apply)`, `### RED evidence`,
  `### Negative controls` and `### Choices and deviations` sections. The card's acceptance text is
  byte-identical to the commit that wrote it.
- Every product ID in the card has a test that names it and proves its criterion: **ARQ-42** (6
  files, incl. the planner, the surface, the offset and the comparison tests), **ARQ-149** (5 files;
  `page.rs`, `planner.rs`, `coverage.rs`, `layer.rs`, `tests.rs`), **ARQ-121** (`planner.rs`, the
  tile grid and its `// ARQ-121` tests), **ARQ-02** (both `lib.rs` forbids, the ARQ-02 purity
  step), plus **ARQ-111** and **ARQ-113** which the card names through AC5 and the golden. No ID is
  left out.
- The record commit `c07f6a3` touches only `apply-progress.md` and states the deviation, as the
  fast lane requires.

### Independent probes (step 1c, before reading the delivery's tests)

Six probes on a clean `git archive` export at HEAD, in my own file
(`crates/gafira-render-cpu/tests/v2_probes.rs`), built from the card's AC text with my own scenes
(96 px page, my own ids, my own colours) and no recording backend: a real `Document`, the real
`GraphPageRenderer`, the real `CpuBackend`. They are not the delivery's tests and share no helper
with them.

| Probe | Criterion | What it asserts | HEAD | Base `8e1a468` |
|---|---|---|---|---|
| P1 | AC2 | two disjoint opaque roots (red 8..28, blue 60..80) on an opaque page: >300 red px, >300 blue px, the gap at (44,44) is the white background, and (16,16)/(70,70) are the two fills | pass | **fail: `the first root is drawn: 0 red pixels`** |
| P2 | AC3 | region `(4,6,68,70)` over an opaque page: 0 transparent pixels, and the 64×64 bytes equal the crop of a full-page read | pass | **fail: `616` transparent pixels** |
| P3 | AC4 | preview vs export on that region within 1/255, **with both sides asserted opaque first** | pass | fail (`the preview is opaque here`) |
| P4 | — (the no-roots case) | `roots: []` → one task per region, and the page reads back as 4096 opaque white pixels | pass | **fail: 0 tasks** |
| P5 | AC1 | one task per region, both fills in paint order, key changes with either root and with the order | pass | **fail: 2 tasks for 2 roots** |
| P6 | AC5 (same bytes half) | the same renderer twice answers identical bytes | pass | pass |

Two things this establishes. **The probes are not vacuous**: five of six fail on the pre-fix tree
and print the defect's own signature — `0` red pixels (the card's "red `0`, blue `256`) and `616`
transparent pixels, which is exactly `4096 − 60×58`. And **AC4 cannot be proved by agreement
alone**: P3 *passes* on the pre-fix tree, because both qualities lose the same band; only the
opacity premise makes it evidence. The committed test asserts that premise first
(`arq_42_the_preview_and_the_export_agree_on_an_offset_region`, `:832-835`), so the claim is not
hollowed there. P6 passing on the base is correct: determinism is not what `O12` changes.

Comparison with the delivery's tests afterwards: they are equal or stronger on every criterion. The
two-roots test asserts both pixels and the transparent gap (`:716-754`); the offset test asserts the
whole window against a crop of the full read and the first/last pixel by hand (`:796-818`); the
surface unit test does the same one level down and adds the corner bytes (`src/tests.rs:897-951`).

### Hand mutants (step 4b)

Four, on the export at HEAD (warm `CARGO_TARGET_DIR`), each reverted before the next; `git status`
of the export diffed against the backup copies after the last one. I did not repeat the record's
eleven.

| # | Mutation | Killed by | Result |
|---|---|---|---|
| SM | **spot-check of the recorded M1**: the roots composed in reverse paint order (`planner.rs`, `.rev()`) | the recorded test `planner::tests::arq_42_a_region_is_one_task_with_the_ops_of_every_root_in_paint_order` | **killed**, with the recorded signature `left: [blue, red]` |
| HM1 | **mine**: `composite_key` XOR-folds the roots' hashes instead of hashing the sequence — order lost, duplicate roots cancel | **my P5** (`the order of the roots is in the key`) and the delivery's `planner::tests::arq_42_the_merged_key_changes_when_either_root_changes` | **killed** |
| HM2 | **mine**: `Surface::write` maps through `source_index(clip.x0 - region.x0, y - region.y0)` while `read` stays page space, so the pair stops agreeing | **my P2** (`no pixel of an opaque page is transparent: 4096`) plus six committed tests of `gafira-render-cpu` | **killed** |
| HM3 | **mine**: the planner skips a region when no root contributes an op (`if merged.is_empty() { continue; }`) — the old no-roots behaviour | **my P4 only**; all 118 committed tests of the two crates stay green | **survived** → `W9` |

HM1 is the mutant the card asks for by name ("does it change when **either** root changes, and does
it depend on paint order? Pin it with your own mutant"): the key is a blake3 over a domain byte, the
count and the roots' keys in order, so a sequence-losing combiner is the failure mode, and it dies on
both my probe and the delivered test. HM2 targets the claim the `surface.rs` diff makes — that
`read` and `write` now share one page-space mapping — and dies. HM3 is the round's one survivor.

### `W9`: the no-roots case is right and unpinned

The behaviour is right. With `merged` empty the region loop still emits its task, `page.rs`'s
`paint_background` puts the page rect into it, and the surface is the opaque page instead of the
transparent buffer zero roots used to leave: my P4 reads 4096 of 4096 opaque white pixels at HEAD
and 0 tasks at the base. For a new empty page — the most common state of a document — the old
behaviour was a transparent frame.

But nothing pins it: HM3 restores the old behaviour and the whole committed suite passes, including
the delivery's own planner tests, because they all build a page with at least one root. The record
says so plainly ("No test pinned the old behaviour […] Stated for `verify`"), which is exactly the
honest reporting this round wants. I keep it a **WARNING**, not a CRITICAL: the delivered code is
correct, no acceptance criterion of the card names the blank page, and the card's critical list for
`CB3b` is the merge order, the key combination and the `read` mapping — HM3 touches none of the
three. It is a test gap of class `implementation` and it is marked `follow_up`, so it reaches
`archive` rather than being silently dropped.

### The two changed assertions: premise corrections, not concessions

Read both diffs as a reviewer:

- **`page_renderer`'s task key** (`tests/page_renderer.rs:312-327`). It pinned
  `task.key.content == root_key(graph, 0)`, which was a true statement of the *old* plan ("a tile
  holds the operations of the root it draws") and is false by construction under `O12`: the tile now
  carries every root. The replacement asserts the task's key differs from each root's. That is a
  premise correction — the old premise is the behaviour the delivery exists to change — but it is
  *weaker* than the rule: "differs from both" would also pass a constant key or a `root₁` key with a
  fixed mask. The record says the composition is pinned by the planner tests instead, and that is
  true: HM1 (a combiner that loses order and cancels duplicates) is killed there. So the weak
  assertion is backed, and the comment in the test says where. The quality half of the test still
  holds: the ops are still asserted equal between the qualities and the key still asserted different,
  so "the plans differ only in the content keys" is unchanged.
- **`page::tests::arq_149_a_request_off_the_page_origin_carries_its_origin_and_clipped_tasks`**
  (`page.rs:749`). It pinned `reads == [(0,0,w,h)]`, which *is* the defect: the caller translated the
  request's region to the surface origin before reading. The test is `CB1`/`PC1`'s, not `CB3`'s, so
  no `CB3` test was rewritten; and the new value `reads == [region]` is the stronger, correct rule
  (the exact page-space rectangle, not merely "not the origin"). The negative control proves the old
  value was the bug: with it, P2 measures the `616` band.

Verdict: **both are premise corrections.** Neither lowers what the suite can catch; the arq_149 one
raises it.

### The `O13` deviation: the card's text was wrong, the code is right

The card and the worktree's `design.md` say the fix lands in the surface because "the caller already
passes the right rectangle". That premise is false: `render` passed `target_region(request.region)`
= `(0,0,w,h)` while `Surface::write` and `Surface::read` already subtracted `TargetDesc.origin`, so
both ends were wrong. `develop` `e10ebe9` corrects the decision. The implementer's deviation —
touching `page.rs`, deleting `target_region`/`whole` — is therefore **correct and the minimal
change**, and judging the code rather than the stale text, it is not a finding. It also explains why
`CB3`'s `W5` was read as a surface bug: the surface was the only end that looked wrong in isolation.
The `surface.rs` half of the delivery is behaviour-preserving (the same index arithmetic through one
function); the observable fix is the caller, which my P2 measures.

### The stored golden (`ARQ-113`): unaffected, and that is the right call

The file `tests/golden/arq_113_scene.rgba8` (16 384 bytes) is untouched by the delivery
(`git log` on that path stops at `d291434`), and no test regenerates it. Its scene is
`document(nodes, &[10])` — one wrapper group — and its region is `REGION = (0,0,64,64)`. So: one
root means one task per region before and after `O12`, with the identical op list (the group's
subtree ops); and a region at the page origin means the old `target_region` was the identity, so the
`O13` fix does not move a byte. The golden compares a scene the fix **did not change**, which is why
it still passes and why not regenerating it is correct rather than lazy. The consequence is `S6`:
the pixel-level proof of the composition lives in the new two-roots test, not in the cross-platform
oracle. The card already assigns a multi-root golden to the coordinator as a follow-up, so this is a
SUGGESTION, not a demand of this round.

### Size

`git diff --shortstat 8e1a468..5dd0691` → 7 files, **522 insertions(+), 73 deletions(-)**; at HEAD
with the record, 8 files, **576**. Against the card's forecast of 180 (250 at ×1.4) that is
**2.9×** and 2.1×, and below the ~700-line signal at which the owner must be asked. 444 of the 522
are tests and the rest is `src/` (`key.rs` 28, `planner.rs` 30, `page.rs` 4 net, `surface.rs` a
rewrite of the same 16 lines). The record's numbers reproduce exactly, including the base-relative
diff and the split at `5dd0691`. `W8` records it for the coordinator, which is what the card asks
for; it does not change the verdict.

### Closure of `CB3`'s `W4` and `W5`

Not a closure round, so they are not re-listed in the JSON, but the coordinator needs the state:
**both are closed.** `W4` (a multi-root page loses every root but the last) — P1 at HEAD has both
roots, the base has `0` red pixels; `W5` (an opaque page at `(4,6,68,70)` returns `616` transparent
pixels) — P2 at HEAD has `0`, the base `616`. The committed tests that own each are
`arq_42_two_roots_of_a_page_both_survive_the_read_back` and the two offset tests, and both survive
the mutants HM1/HM2/SM. Their `follow_up: true` can become `closed` in the entry.

### Findings

| ID | Severity | Class | Ref | Summary | Status |
|---|---|---|---|---|---|
| W8 | WARNING | implementation | the delivery's 8 files | 576 insertions (522 of work) against the 180 forecast, 250 at ×1.4: 2.9× / 2.1×, below the ~700 signal. The coordinator decides, as the card says. | open |
| W9 | WARNING | implementation | `planner.rs:299-320` | The no-roots task is right (P4: 4096 opaque px) but unpinned: HM3 restoring "no task" leaves all 118 tests of the two crates green. Test gap, follow-up. | open |
| S6 | SUGGESTION | implementation | `tests/golden/arq_113_scene.rgba8`, `arq_42_comparison.rs:399-403` | The golden's scene is single-root at the page origin, so it is unaffected by both fixes and no golden covers the composition across platforms. Re-generation is the card's coordinator follow-up. | open |

### Observations (not findings)

- **The AC1 key is not hollowed.** `composite_key` hashes a domain byte, the count and the 16-byte
  keys in order; HM1 shows an order-losing combiner dies on my probe and on the delivered test. The
  one rule no test can kill is the count in the description, which the record already states as
  framing (fixed-width keys make it unobservable) — I agree, and it is not a survivor worth chasing.
- **AC5's "reused rather than re-rendered" is sound, not a proxy.** The test counts `put`s, which
  would be a weak proxy if render and put could diverge; in `backend.rs:91-100` they are the two
  statements of the same `None` arm of a `match cached_tile(...)`, so a hit skips both. There is no
  one-line mutant that renders without putting. The same test also pins the merged tile's two
  pixels and byte-for-byte equality across frames, so AC5's two halves both have evidence.
- **`page_renderer`'s new `assert_ne` cannot pin the composition** (see above) and says so in its
  comment; HM1 proves the composition *is* pinned, in `planner.rs`. A future reader should get the
  key's proof from there.
- **The branch's `design.md` still carries the false `O13` premise.** The correction is on
  `develop` at `e10ebe9`, and this branch does not touch `design.md`, so the merge keeps develop's
  corrected text. Nothing to fix here; recorded so the merge is not read as a regression.
- **`O14`/`CB8` is untouched**, as the card's non-goals say: no quantisation change, no `Rgba8`
  surface, and the two sRGB passes are still there.
- **V2 does not re-run the per-commit gates.** The record states that `4590c2b` and `5dd0691` each
  pass their own scope alone, and its test counts reproduce arithmetically (71 − 2 planner tests =
  69 at `4590c2b`; 30 − 1 = 29 for the CPU crate), but the per-commit runs are V3's, and this round
  does not claim them.
- **No T4 and no `cargo-mutants` run**, per `mutation: off` and `O10`. This round does not report a
  mutation score and its absence is not a gap.

next: tdd-apply

## Round 1 — CB4

```json
{ "schema": 1, "change": "gafira-render-cpu", "delivery": "CB4", "rounds": [ {
  "round": 1, "commit": "e3630cc97bd7d7e5608940f9e3eb8ad20b6e2aa2", "mode": "standard", "tier": "T2", "depth": "V2",
  "verdict": "FAIL", "next": "tdd-apply",
  "counts": {"critical": 2, "warning": 1, "suggestion": 3},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/tests/image.rs, src/image.rs:146-205",
     "summary": "No test filters an Rgba8 source: H-A (mix premultiplied sRGB, convert after) and H-C (straight-alpha mix) leave all 38 tests green; probes P4/P5 kill both. The 8-bit filtering space is unpinned.",
     "follow_up": false, "status": "open"},
    {"id": "C2", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/image.rs:93-95, src/tests/image.rs:303-340",
     "summary": "The destination clip's shape is unpinned: H-B (clip to the page-space bounding box, clamp the sample) survives all 38 tests; only a rotated image tells it from the per-pixel quad. P6 kills it.",
     "follow_up": false, "status": "open"},
    {"id": "W10", "severity": "WARNING", "class": "implementation",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:302 (AC6), src/backend.rs:290-302",
     "summary": "AC6's first clause is not observed: removing the cull leaves 38/38 green. Its only trace today is an off-tile Alpha8 not being refused. The cull earns its place as work skipped, not as a pixel rule.",
     "follow_up": true, "status": "open"},
    {"id": "S7", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/tests/image.rs:302-340",
     "summary": "AC5's checkerboard is invariant under a quarter turn in either direction, so the sign of the rotation is unpinned; the record states it, and M4 (forward vs inverse) covers it indirectly through AC4.",
     "follow_up": false, "status": "open"},
    {"id": "S8", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "crates/gafira-render-cpu/src/backend.rs:290-302, src/image.rs:129-139",
     "summary": "An off-tile Alpha8 image is skipped rather than refused, so the refusal depends on which tile asks. Benign today (the visible page is right); an eager per-tile conversion would make it inconsistent.",
     "follow_up": false, "status": "open"},
    {"id": "S9", "severity": "SUGGESTION", "class": "implementation",
     "ref": "the delivery's 7 files",
     "summary": "693 insertions against the card's 360 forecast (504 at x1.4): 1.9x, 450 of them tests. Inside the ~700 signal; the coordinator decides, as the card says.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 4, "killed": 1},
  "criteria": {"total": 7, "covered": 7},
  "size": {"lines": 693, "forecast": 504}, "report": "verify-report.md" } ] }
```

- Delivery: `CB4` of `gafira-render-cpu`, branch `feat/gafira-render-cpu-cb4`, HEAD
  `e3630cc97bd7d7e5608940f9e3eb8ad20b6e2aa2`, cut from `develop` `2b949be`. Four commits: `2b949be`
  (the card), `f0d3136` (the sampler), `bea13dc` (the record), `e3630cc` (the card amend). Not
  pushed, not merged.
- Mode `standard`, depth **V2** (the card says `verify: V2`). `mutation: off`: no `cargo-mutants`
  run happened and that is not a finding; the rigor is the hand mutants and the gates, so V2's
  independent probes and its own mutant budget are this round's.
- Verdict: **FAIL** — 2 CRITICAL, 1 WARNING, 3 SUGGESTION. The *code* answers the question this
  round exists for: it converts in ARQ-117's order and composites in linear premultiplied light, and
  nine independent probes with hand-derived constants say so. The *tests* do not: the three
  properties they never exercise — filtering an 8-bit source, and the per-pixel quad clip — are
  exactly where a plausible wrong reading lives, and each survives the whole suite.
- Toolchain: cargo/rustc 1.93.1, clippy 0.1.93, cargo-deny 0.20.2. Disk: `df -h /` at start 174 GB
  free.

### Gates (delivery HEAD, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` in 22.91 s |
| `cargo test --workspace --locked` | 0 | **196 test binaries, 2497 passed, 0 failed, 0 ignored**; the delivery's own binaries are `gafira-render-cpu` lib **38** (30 at CB3b + the 8 image tests) and `arq_42_comparison` 14 |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0: Format; CLI has no UI deps (ARQ-07); `forbid(unsafe_code)`; V5 layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports` |

The extractor `/tmp/gafira-policy-run.py` was re-checked against this `ci.yml`: job `policy` has 12
steps — `actions/checkout` (`uses`), `Install toolchain` (the one `run:` step the extractor skips),
the nine checks above, and `cargo-deny-action` (`uses`) — so the record's "ten `run:`, one skipped,
nine" reproduces, and the extractor's `SKIP` entry for the test matrix's target guard names a step
this job does not have (harmless). AC7's other halves: `src/lib.rs:25` still has
`#![forbid(unsafe_code)]`; the purity `grep` for `std::fs|std::net|std::thread::spawn|
std::time::(Instant|SystemTime)::now|std::env` exits 1; and
`git diff 2b949be..HEAD -- Cargo.toml Cargo.lock crates/gafira-render-cpu/Cargo.toml` is empty, so
no dependency arrived and no V5 row moved (`O2`).

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-cb4-v2-target-head` for the worktree,
`/tmp/gafira-cb4-v2-target` for the probe and mutant export, both with `CARGO_INCREMENTAL=0` and not
shared. A first `clippy` ran with the export's target dir, which breaks the one-dir-per-tree rule;
it was discarded and the gate re-run in the head dir, and its 0 is the one reported. The worktree is
clean (`git status` empty) and no source file was edited by this round; the only file written is
this one.

### Artifact integrity (fast lane, step 3c)

- `2b949be docs(gafira-render-cpu): slice card for CB4` touches only
  `specs/changes/gafira-render-cpu/apply-progress.md` (+15), as G0 requires.
- No implementer commit changed the card. `git diff 2b949be..bea13dc` on that file adds **56** lines
  and removes **0**: the `## CB4 (tdd-apply)` record and its three subsections only. The amendment
  is `e3630cc docs(gafira-render-cpu): slice card amend for CB4` by the coordinator: it touches
  only that file (+10/−2) and states why (the `AC4` wording read bilinear as nearest-neighbour, and
  the non-goal misplaced `level`).
- Every product ID in the card has a test that names it and proves its criterion: **ARQ-117**
  (`src/tests/image.rs:128`, `:181`, plus the CB1/CB2 hashes), **ARQ-121** (`:215`, `:257`),
  **ARQ-149** (`:350`, `:372`), **ARQ-42** (`:303`). No ID is left out. **ARQ-02** is AC7's command.

### Independent probes (step 1c, before reading the delivery's tests)

Nine probes in my own file (`crates/gafira-render-cpu/tests/cb4_probe.rs`) on a clean `git archive`
export at HEAD, built from the card and the product rows only: my own scenes, my own `TileCache`,
the real `CpuBackend`. Every expected number was derived first, in Python from the spec text
(`/tmp/cb4derive.py`: ARQ-117's `(c×a+127)/255`, `round(c×255/a)` with halves up, the CSS Color 4
sRGB curve of §10, and bilinear at `i+0.5`), not read from the delivery.

| Probe | Criterion | What it asserts | HEAD |
|---|---|---|---|
| P1 | AC1 | `[128,64,32,128]` → `(0.50196078, 0.10835351, 0.02573526, 0.50196078)`; over an opaque `Srgb(.2,.4,.6,1)` → `(0.51844826, 0.17452714, 0.18438405, 1)`; the pixel outside the image stays clear | pass |
| P2 | AC2 | `[.25,.5,.75,.5]` is returned bit for bit, no curve | pass |
| P3 | AC4 | a 4-wide ramp `0,¼,½,1` at 2× gives `[0, 1/16, 3/16, 5/16, 7/16, 10/16, 14/16, 1]`, and no weight at a corner | pass |
| P4 | AC1 (composition) | a 2×2 `Rgba8` image at 2× equals the mix of the four *converted* texels (`(0.62647059, 0.13523025, 0.01930144, 0.62647059)` at (1,0)) | pass |
| P5 | AC1 (no bleed) | opaque red beside transparency keeps `r == a` at every column, so the alpha ramps and the hue does not move to black | pass |
| P6 | AC6 + the clip | a 4×4 white image turned 45° about (2,2): page (0,0) and (5,5) are inside the box and outside the quad and stay clear; the centre is painted | pass |
| P7 | AC6 | an image at page (1000,1000) changes no pixel of the tile | pass |
| P8 | AC6 + Alpha8 | on-tile `Alpha8` → `RenderError::Effect(UnsupportedFormat)`; off-tile it is skipped with no error | pass |
| P9 | ARQ-149 | a 1×1 image at page (104,104) on a tile at region (100,100)-(108,108) paints exactly page (104,104) | pass |

Two things this establishes. **The arithmetic is independently confirmed**: my Python and the
implementation agree to `1e-6` on the AC1 texel, the composite, the AC4 quarters, the 2×2 four-tap
mix and the no-bleed case — so the amended `AC4` (weights at texel centres, the quarter/
three-quarter pair, no constant block) is the correct reading and the delivery's choice to implement
the parenthetical was right, not a deviation to excuse. And **the probes are not vacuous**: every
mutant I applied dies on at least one of them (P4/P5 on H-A and H-C, P6 on H-B, P2 on M3, P8 on the
removal of the cull).

Comparison with the delivery's tests afterwards: they are stronger than mine on the AC1 texel set
(four hand-computed texels, one partially transparent, plus the backdrop) and equivalent on AC2 to
AC4. They are weaker in one dimension that matters: **every test that involves interpolation uses
`RgbaF32`, and every `Rgba8` test is a 1:1 translate** — so no delivery test ever filters a texel
that needs converting (`C1`), and no delivery test uses a non-axis-aligned transform where the box
and the quad differ (`C2`).

### Hand mutants (step 4b)

Four counted on the export (warm target dir), each reverted before the next; the export's `src/`
diffs clean against HEAD afterwards. I did not repeat the record's list.

| # | Mutation | Killed by the delivery's tests? | Result |
|---|---|---|---|
| SM | **spot-check of the recorded M2**: `decode_srgb8` drops the premultiply (`[linear.r, …]`) | `arq_117_an_rgba8_texel_is_converted_once_in_the_order_arq_117_fixes` | **killed**, with the recorded signature `channel 0 is [1.0, 0.21586053, 0.051269464, 0.5019608]` |
| HM1 | **mine**: filter the `Rgba8` source in its own space — mix the premultiplied sRGB bytes, convert the mixed value once | **no** — 38/38 green | **survived** the suite; **killed by my P4 and P5** → `C1` |
| HM2 | **mine**: filter with straight alpha — unpremultiply each tap, mix, premultiply once at the end | **no** — 38/38 green | **survived** the suite; **killed by my P4 and P5** (blue `0.02409` vs `0.01930`) → `C1` |
| HM3 | **mine**: clip to the image's page-space **bounding box** and clamp the sample, instead of the per-pixel quad test | **no** — 38/38 green | **survived** the suite; **killed by my P6** (the box corner reads `[1,1,1,1]`, expected `[0,0,0,0]`) → `C2` |

Re-checks of the record's own claims, uncounted: **M3** (an sRGB decode applied to `RgbaF32`) is
**killed** by three of their tests (AC2 and both f32 AC3/AC4) and by P2 — the claim holds. **M7**
(the cull removed) leaves 38/38 green, as the record says. **M8** (the cull's box from the first
corner only) is **killed** by `arq_149_an_image_that_partly_covers_the_tile_draws_only_its_own_window`,
so the cull's *semantics* are pinned even though its *existence* is not.

### `C1`: the 8-bit filtering space is the one place the suite cannot see

`AC1` says the conversion is "unpremultiply, sRGB to linear, premultiply" and the plan says the
sampling "happens in premultiplied linear light, so an edge against transparency does not bleed
towards black". The code does both (`src/image.rs:146-165`: each tap through `decode_srgb8` before
any weight, `mix` in the premultiplied linear result). But the tests pin the order only at a 1:1
translate, where no two texels are ever weighed: `arq_117_…_the_order_arq_117_fixes` uses
`Affine::translate((2.0, 1.0))`, and the only four-tap tests (`AC3`, `AC4`, `AC5`) all use
`RgbaF32`, the format that must not be converted. HM1 and HM2 are the two plausible wrong
readings — filter in premultiplied sRGB and convert afterwards, or filter straight alpha and
premultiply at the end — and both leave the 38 tests green while moving a channel by a quarter
(HM2 on P4's (1,0): blue `0.02409` against `0.01930`) or bending the edge towards black (HM2 on P5).
This is not a rounding question; it is the silent wrong page the card says it exists to prevent.
The code is right today, so the fix is two tests, not behaviour: a scaled `Rgba8` case asserted
against per-tap hand constants, and an opaque-next-to-transparent case asserting `r == a` at every
column.

### `C2`: the clip's shape is unpinned

The delivery replaced a box clip with the per-pixel quad test on purpose and argues the box would
smear a rotated image's edge into the corners — **that argument is right**, and P6 confirms the
implementation does the quad. What no test does is distinguish the two: `AC6`'s partly-covers test
uses a translate (box = quad) and `AC5`'s quarter turn maps the box onto the quad exactly. HM3
substitutes the box clip and clamps the sample, and all 38 tests stay green; it is only visible on a
non-axis-aligned transform whose box corners fall outside the quad, which is P6. One rotated
fixture with a corner asserted clear closes it.

### `W10`: the cull is real, and its survival is not a defect of the code

M7 was reproduced: with the whole `matches!(…)` cull removed from the `DrawImage` arm the suite is
38/38 green, because the per-pixel clip rejects every sample a missed tile could produce. So AC6's
first clause — "the op is culled before any sampling" — cannot be falsified by the criterion's own
kind of evidence (`[unit]`, on pixels). It is not literally unfalsifiable, though: the cull has
exactly one behavioural trace today, and P8 shows it — **an off-tile `Alpha8` image is skipped
instead of refused**, while the same image on-tile errors. That is `D6` applied consistently (a
culled op contributes nothing, so nothing is sampled), and no test asserts it.

The cull earns its place: it trades one four-corner transform and a rectangle test per tile for
`width × height` inverse transforms and range tests (`256 × 256` on a full tile) on an op that can
draw nothing. That is a per-tile, not per-pixel, saving, and with many off-tile images it is the
difference between skipping a few dozen ops and running the sampler over every tile. It stays
pixel-invisible only while conversion is per tap; with an eager per-tile conversion it would become
observable, because the cull would be the only thing preventing the work. Recommendation: keep the
cull, and either reword AC6's first clause to the cull's observable form (an off-tile `Alpha8` image
is not refused) or assert that in a test. Not CRITICAL: the criterion's visible half (a missed image
changes no pixel) is proven, and the cull's shape and box are pinned by M8/M9/M10.

### The seven criteria, mapped

| AC | Proven by | State |
|---|---|---|
| AC1 | `arq_117_an_rgba8_texel_is_converted_once_in_the_order_arq_117_fixes` (four hand texels + the composite) and P1, P4, P5 | covered; the filtered 8-bit path is unpinned → `C1` |
| AC2 | `arq_117_an_rgba_f32_texel_is_taken_as_linear_premultiplied` (bit for bit) and P2; M3 re-checked and killed | covered |
| AC3 | `arq_121_a_one_texel_image_scaled_over_the_tile_is_that_colour` (64 px) and P3's edge exactness | covered |
| AC4 | `arq_121_an_integer_scale_takes_its_weights_at_texel_centres` (the eight quarters, the corner reading asserted not to be it) and P3; my Python reproduces the same eight numbers from the geometry | covered, and the amended card matches the code |
| AC5 | `arq_42_a_checkerboard_rotated_by_ninety_degrees_is_the_rotated_checkerboard` (16 px against `source(y, 3−x)`); I derived the same permutation from `rotate_about(π/2, (2,2))` | covered; the fixture's symmetry does not pin the sign (`S7`) |
| AC6 | the two `arq_149_…` tests, P7, P9 (the window and the tile origin), P8 for the skip | covered except the literal "before any sampling" clause (`W10`) |
| AC7 | `src/lib.rs:25`, the nine policy steps, the purity `grep` (exit 1), the empty manifest diff | covered |

### Flagged items, judged

1. **`AC4` and the parenthetical.** The amend is correct. At an integer 2× scale the destination
   centres sit a quarter texel either side of each texel boundary, so bilinear *must* answer
   `¾A + ¼B` and `¼A + ¾B`; "each texel covers its block" is nearest-neighbour and contradicts the
   filter the same card, the scope and the plan fix. My derivation reaches the card's eight numbers
   independently, and the delivery's choice to implement the parenthetical was right.
2. **The destination clip is per pixel.** Right, and the argument for it is right: a page-space box
   clip paints the area between a rotated image's quad and its box with the clamped edge, and M10
   shows the tile alone would flood it. The gap is only that no test makes the distinction (`C2`).
3. **`Alpha8` refused, and culled before the refusal.** Worth one SUGGESTION, not a defect: the
   page is right either way (a plan with a visible `Alpha8` image still fails on the tile that
   covers it), and D6 justifies skipping a culled op. It becomes a real inconsistency only if
   conversion moves ahead of the cull (`S8`).
4. **No converted copy cached.** The arithmetic is right: `sample` calls `texel` four times per
   output pixel (`src/image.rs:150-152`), so an `Rgba8` source pays up to four curve decodes, and a
   clamped or constant region pays fewer because the taps repeat. The deferral is honest: nothing
   emits `DrawImage` yet — `git grep 'fn visit_image'` is empty, `ARQ-134` keeps the visitor at three
   kinds, and `gafira-render`'s planner only passes the op through — so no document can reach this
   code, and `CB6` is where the measurement belongs. No finding.
5. **Size.** 693 insertions (+13 deleted) is 1.9× the 360 forecast (1.4× of it is 504), with 450 of
   the insertions in `src/tests/image.rs` and 243 in `src/`. It is inside the ~700 signal and the
   tests are the forecast's own x1.4 rule at work, so this is a SUGGESTION for the coordinator to
   decide on, as the card says (`S9`); it does not gate the round.

### Observations (not findings)

- **The record's own survivor is honestly stated.** M7 is not concealed, and two mutants (M8, M9)
  pin what the cull *is*. That is the right way to report a survivor; the finding above is about the
  criterion's wording, not about the record's candour.
- **`decode_srgb8` is shared, not duplicated.** The cached-tile read-back and the image path call
  one function (`src/surface.rs:299`), so the conversion order has one implementation. That is the
  right shape, and it means a fix for `C1`'s tests also protects the tile path.
- **The guard no test can see.** `draw` returns early when the inverse is not finite, and the
  destination clip would reject every pixel of such a transform anyway. The record claims it as
  untested; I agree, and P6's geometry shows why it is unobservable rather than a gap.
- **The negative control is sound.** M4 (the transform instead of its inverse) is killed by the AC4
  test, which is what keeps AC5's symmetric fixture from being the only evidence of the mapping.
- **No T4 and no `cargo-mutants` run**, per `mutation: off` and `O10`. This round's rigor is its own
  probes and mutants, and the absence of a mutation score is not a gap.

next: tdd-apply

## Round 2 — CB4 (closure)

```json
{ "schema": 1, "change": "gafira-render-cpu", "delivery": "CB4", "rounds": [ {
  "round": 2, "commit": "d762b43fc700c8413e0671eb4cdd06838baf733e", "mode": "standard", "tier": "T2", "depth": "V1",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 1, "suggestion": 2},
  "classes": {"claim": 1, "implementation": 2},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/tests/image.rs:190-246",
     "summary": "Closed: arq_117_a_scaled_rgba8_image_is_filtered_after_each_texel_is_converted pins the filtering space with nine hand-derived constants; HM1 reproduced gives 0.49064726 against 0.52189588375.",
     "follow_up": false, "status": "closed"},
    {"id": "C2", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/tests/image.rs:488-550",
     "summary": "Closed: arq_149_a_rotated_image_paints_its_quad_not_the_corners_of_its_box; HM3 reproduced fails on page (0, 0), the other 40 tests green.",
     "follow_up": false, "status": "closed"},
    {"id": "W10", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/tests/image.rs:583-644",
     "summary": "Closed: the off-tile Alpha8 skip and the on-tile refusal are asserted in one test; removing the cull fails it with the recorded message.",
     "follow_up": false, "status": "closed"},
    {"id": "S7", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/tests/image.rs:303-340",
     "summary": "Still open: AC5's checkerboard and the new 45-degree fixture are both invariant under the sign of the rotation, so the direction is still unpinned. Not owed by this fix.",
     "follow_up": false, "status": "open"},
    {"id": "S8", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "crates/gafira-render-cpu/src/tests/image.rs:591",
     "summary": "Closed as pinned: the off-tile skip is now asserted, so an eager per-tile conversion cannot change it silently; AC6's wording is the coordinator's to settle.",
     "follow_up": false, "status": "closed"},
    {"id": "S9", "severity": "SUGGESTION", "class": "implementation",
     "ref": "the delivery's 9 files",
     "summary": "Re-measured: 1228 insertions against the 360 forecast (3.4x; 2.4x at x1.4), 200 of them this round and all tests. 243 of the 1228 are production src. The coordinator decides.",
     "follow_up": false, "status": "open"},
    {"id": "S10", "severity": "WARNING", "class": "claim",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:373-374",
     "summary": "The record's HM1/HM2 rows pair a value only the float convert-once form answers (0.49064726) with the green count of the byte-space form (38). The float form also fails two pre-existing Rgba8 tests.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 4, "killed": 4},
  "criteria": {"total": 7, "covered": 7},
  "size": {"lines": 1228, "forecast": 360}, "report": "verify-report.md" } ] }
```

- Delivery: `CB4` of `gafira-render-cpu`, branch `feat/gafira-render-cpu-cb4`, HEAD
  `d762b43fc700c8413e0671eb4cdd06838baf733e`. Round 2 is a **closure round** over the round-1 `FAIL`
  (`C1`, `C2`, `W10`): the fix is `cc53364` (+81), `7724ad5` (+56/−1), `07908ba` (+63), all in
  `src/tests/image.rs`, plus `d762b43` (+22/−1) in `apply-progress.md`. Base of this round is the
  round-1 report commit `13f5195`. Not pushed, not merged.
- Mode `standard`, depth **V1** as the coordinator set it: the five gates, the artifact-integrity
  checks, and one reproduced mutant per earlier finding. `mutation: off` — no `cargo-mutants` run, and
  that is not a finding. The mutants below are *reproductions of round 1's findings*, not a new
  independent-mutant budget: there are 4, and all 4 were killed.
- Verdict: **PASS** — 0 CRITICAL. `C1`, `C2` and `W10` are closed, each proved by applying the round-1
  mutant on a clean export and watching the new test fail with the recorded message. One new WARNING
  (`S10`, class `claim`): the record's HM1/HM2 green count does not reproduce for the mutant that
  produces the recorded value. `S7` and `S9` stay open as SUGGESTIONs; neither is owed by this fix.
- Toolchain: cargo/rustc 1.93.1, clippy 0.1.93, cargo-deny 0.20.2. Disk: `df -h /` at start **172 GB**
  free.

### Gates (delivery HEAD, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` in 28.73 s |
| `cargo test --workspace --locked` | 0 | **196 test binaries, 2500 passed, 0 failed, 0 ignored** (round 1: 2497 — the +3 are this round's tests); the delivery's own `gafira-render-cpu` lib binary is **41** (38 at round 1) |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0: Format; CLI has no UI deps (ARQ-07); `forbid(unsafe_code)`; V5 layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports` |

The nine steps are the `run:` steps of the `policy` job after the toolchain install, extracted by
`/tmp/gafira-policy-run.py` and re-checked against `.github/workflows/ci.yml` (the job's `runs-on` and
the two `uses:` steps are not `run:` steps; the toolchain install is skipped because this machine has
it). Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-cb4-v2b-target` for the worktree gates and
`/tmp/gafira-cb4-r2-export-target` for the clean export, both `CARGO_INCREMENTAL=0`, not shared; both
freed with `cargo clean --target-dir` at the end.

### Artifact integrity (steps 3, 3b/3c)

- The fix does not touch the spec artifact's acceptance text. `git diff 13f5195..d762b43 --
  specs/changes/gafira-render-cpu/apply-progress.md` adds the `### Mutants the verify round left alive`
  section, its table and its bullets, and **replaces one record paragraph** (the `M7` survivor note,
  extended with one sentence). No acceptance criterion, Given/When/Then line or product ID changed.
- `cc53364`, `7724ad5` and `07908ba` touch one file, `crates/gafira-render-cpu/src/tests/image.rs`;
  `d762b43` touches only `apply-progress.md`. No implementer commit touched the card commit `2b949be`,
  the coordinator's amend `e3630cc` or any other spec file.
- Every AC still has its test or command, and the new tests only add to the map: AC1 gains the filtered
  8-bit case (`:223`), AC6 gains the cull's trace (`:591`) and the quad clip (`:503`). AC7 is still the
  policy command set and the empty manifest diff. All seven IDs the card names (`ARQ-117`, `ARQ-121`,
  `ARQ-149`, `ARQ-42`, `ARQ-02`, `ARQ-111`, `ARQ-113`) keep a test that names them.
- The worktree is clean (`git status` empty); the only file this round wrote is this report.

### "The sampler was not changed" — confirmed

```
git diff 13f5195..d762b43 -- crates/gafira-render-cpu/src/image.rs \
                              crates/gafira-render-cpu/src/backend.rs
(empty)
```

`git diff --name-only 13f5195..d762b43` lists exactly two paths: `src/tests/image.rs` and
`apply-progress.md`. The one qualification, and it is not a contradiction: `src/tests/image.rs` *is*
under `src/`, but `src/lib.rs:32` gates `mod tests` with `#[cfg(test)]`, so it is test code and no
production line moved. The claim holds at the level that matters — `image.rs`'s sampler, `backend.rs`'s
cull and everything the page depends on are byte-identical to what round 1 verified. `git diff` of the
production files is empty, and the 41-test crate suite is green on the export at HEAD before any mutant
is applied.

### Closure of the earlier findings

| Id | Round-1 finding | What closes it | How it was checked this round | Status |
|---|---|---|---|---|
| `C1` | The 8-bit filtering space is unpinned: HM1 and HM2 leave 38/38 green | A scaled `Rgba8` case asserting per-tap hand constants | HM1 applied on the export: `arq_117_a_scaled_rgba8_image_is_filtered_after_each_texel_is_converted` fails at `page (1, 1)` with `0.49064726` against `0.52189588375` — the recorded message, character for character. The test also fails with the byte-space form (`0.48697385`) | **closed** |
| `C2` | The destination clip's shape is unpinned: HM3 (bbox clip + clamp) leaves 38/38 green | A 4×4 image rotated 45° with box-corner pixels asserted clear | HM3 applied: the same one test fails on `page (0, 0) is inside the image's box and outside its quad`; **40 others green** | **closed** |
| `W10` | AC6's first clause is unobserved; removing the cull leaves 38/38 green | A test asserting the cull's only trace, both halves | The cull removed from `backend.rs:290-302`: `arq_149_an_off_tile_alpha8_image_is_culled_before_the_format_is_refused` fails with `a culled image is not sampled, so its format is not refused: Effect(UnsupportedFormat { format: Alpha8 })`; **40 others green** | **closed** |
| `S7` | AC5's fixture is blind to the sign of the rotation | not owed | The new 45° fixture is a uniform white 4×4 square: `+45°` and `−45°` map the three gap pixels to mirrored points, both outside the quad, and every painted pixel is the same white — so the sign is still unpinned | open (SUGGESTION) |
| `S8` | An off-tile `Alpha8` is skipped rather than refused, so refusal depends on which tile asks | a test or a reworded AC6 | The new test asserts the skip and the on-tile refusal as one rule; a future eager conversion now has to change a test on purpose | **closed** (pinned) |
| `S9` | 693 insertions against the 360 forecast | the coordinator's call | Re-measured: **1228** — 3.4× the raw forecast, 2.4× the ×1.4 gate; see *Size* | open (SUGGESTION) |

### The mutants reproduced

All four on a clean `git archive` export of `d762b43` with its own target dir; each reverted before the
next, and the export's `src/` diffed clean against itself (`diff` of `image.rs` and `backend.rs` against
the backups) before the closing green run, which reports **41 passed**.

| # | Mutation | Applied to | Result |
|---|---|---|---|
| HM1 | filter the premultiplied sRGB value and convert the mix once (float unpremultiply, `Rgba::to_linear`) | `image.rs`, `Source::sample` + a `raw` texel accessor + `convert_mixed` | **killed by the new test**: `page (1, 1) … channel 0 is [0.49064726, 0.11899241, 0.036425173, 0.6647059], expected [0.52189588375, …]`. **38 passed, 3 failed** (see `S10`) |
| HM1b | the same reading with the mix rounded back to bytes and `decode_srgb8` on the mix | `image.rs`, same site | **killed**: `channel 0 is [0.48697385, 0.118592285, 0.036320187, 0.6666667]`; **40 passed, 1 failed** |
| HM3 | clip to `page_bounds(image, transform)` and clamp the sample into the grid | `image.rs:92-95` | **killed**: `page (0, 0) is inside the image's box and outside its quad`; **40 passed, 1 failed** |
| M7′ | remove the `coverage::meets(…)` guard around `sampling::draw` | `backend.rs:290-302` | **killed**: `a culled image is not sampled, so its format is not refused: Effect(UnsupportedFormat { format: Alpha8 })`; **40 passed, 1 failed** |

The arithmetic of HM1 and HM1b was predicted before the runs: my own derivation gives `0.490647233`
and `0.48697` respectively, so the mutant values are the derived ones, not artefacts of my edit.

### `C1` — closed, and the constants are hand-derived

I re-derived all nine expected pixels from the rules alone: ARQ-117's integer unpremultiply
(`round(c × 255 / a)`), the §10 sRGB curve, per-tap conversion, and bilinear taps at
`image_coordinate − 0.5` with the edge clamp. My f64 derivation reproduces the test's table with a
**maximum difference of 3.06e-8** (tolerance 1e-6) — e.g. the centre `0.5218958732` against the
table's `0.52189588375`, and `(2, 1)` = the 0.5/0.5 vertical pair of `TEXEL_10` and `TEXEL_11`
(`0.7887902202`). So the table is the geometry, not a recording of the sampler. The test states the
derivation in its comment (the coordinate grid, which pairs weigh which, and why), and the two wrong
readings it names are the two the mutants confirm: 0.49064726 (sRGB space) and 0.436853 (straight
alpha). It is also the only test in the crate that weighs an `Rgba8` texel: the AC1 translate never
weighs two of them and every f32 test must not be converted, so the filtering space had no test —
which is exactly the gap round 1 reported. (The AC1 translate does kill the *float* form of HM1, but
only through the single-texel conversion `S10` describes, not through the filter; the byte-space form
it cannot see at all.)

### `C2` — closed

The geometry checks out independently: the inverse of `rotate_about(π/4, (2,2))` maps page
`(0.5, 0.5)` to image `(−0.1213, 2.0)`, and the box of the turned square is
`(−0.828, −0.828)-(4.828, 4.828)`, so that pixel is inside the box and outside the `[0,4)²` quad.
Page `(3, 0)` and `(4, 4)` are the same by symmetry. The test asserts the three are clear, then redraws
over an opaque backdrop and asserts they keep it (`GREY`) while the centre is painted — so the clip is
distinguished from "the sampler never writes" as well as from the box. HM3 dies on the first of the
three, which is the strongest form: the box clip *paints* a pixel the quad must leave alone.

### `W10` — closed, both halves asserted

The test asserts the skip (`execute` returns `Ok`, all 16 pixels clear for an image at page
`(100, 100)`) **and** the control (`scale(4.0)` over the same tile → `Effect(UnsupportedFormat
{ format: Alpha8 })`). Removing the cull fails on the first half with the recorded error, so the skip
cannot be a swallowed error and the cull cannot be a no-op with a green suite. This is the observable
form of AC6's first clause, which was round 1's recommendation.

### `S10` — the record's HM1/HM2 evidence does not reproduce (WARNING, class `claim`)

`apply-progress.md:373-374` records, for HM1: the failing assertion `channel 0 is [0.49064726, …]` and
"the other **38 were green**". I reproduced both halves and they belong to **two different mutants**:

- the **float** convert-once form answers exactly `0.49064726` — and it also fails two pre-existing
  tests, `arq_117_an_rgba8_texel_is_converted_once_in_the_order_arq_117_fixes` and
  `arq_149_an_image_that_partly_covers_the_tile_draws_only_its_own_window`, because a 1:1 sample still
  goes through `convert_mixed`, whose float unpremultiply answers `0.5` where `decode_srgb8` rounds
  `127.5` up to `128` (`channel 1` `0.10744026` vs `0.10835353`). So it is **38 passed, 3 failed**, not
  38 green.
- the **byte-space** form does keep every other test green (40/40) — but it answers `0.48697385`, not
  the recorded number.

No single mutant satisfies both cells of that row, and the suite count isolates it: at `cc53364` the
crate has 39 tests, so "the other 38" can only be true if the new test is the sole failure. This is
round 1's own `HM1` description ("mix the premultiplied sRGB **bytes**, convert the mixed value once")
paired with a value the float form produces — most likely a number derived analytically rather than
read off the run. It does not weaken the closure: the new test kills both forms, and kills HM2 for the
same reason. It matters because a mutant table is evidence, and one of its cells is not what a run
prints. HM3 and M7′'s rows do reproduce exactly (40 green, as written).

### `S9` — size, re-measured

`git diff --shortstat 2b949be..d762b43` → **9 files, 1228 insertions(+), 15 deletions(-)**, against
the card's 360 forecast: **3.4×**, or 2.4× the ×1.4 gate of 504. The record's figures reproduce
(`1207` at `07908ba` plus its 22). Composition: 243 production `src/` lines (`image.rs` 206,
`backend.rs` 21, `coverage.rs` 14, `lib.rs` 1, `surface.rs` 1), 649 test lines (`tests/image.rs` 644,
`tests.rs` 5), 87 record lines and 249 lines of round 1's report. This round adds **200 insertions and 1
deletion, all tests**, plus its 22-line record: no production line moved. That is the basis the
coordinator said it would accept on; the figure and its split are reported for that decision, not as a
gate.

### Observations (not findings)

- **The three tests are the minimum the findings ask for, and no more.** Each closes exactly one
  property, each was green on the unmutated tree first (41/41 on the export), and each fails for the
  plausible wrong reading. Nothing else in the suite changed: the diff of `src/tests/image.rs` is
  additions plus one `use` line (`FRAC_PI_4`).
- **The record's per-commit gates are not re-run here.** V1 does not owe them, and the counts do
  reproduce arithmetically: 39 (`cc53364`), 40 (`7724ad5`), 41 (`07908ba`/HEAD). V3 would run them.
- **`S8`'s ambiguity is now behavioural, not textual.** AC6's first clause still says "culled before
  any sampling" while the assertion is "an off-tile `Alpha8` is skipped" — the same condition, one
  level down. If the coordinator rewords the criterion, the test is its sentence; if not, the test is
  the rule.
- **No `cargo-mutants` run and no T4 report**, per `mutation: off`. The absence of a mutation score is
  not a gap this round, as round 1 recorded.

next: tdd-apply

---

## Round 1 — CB5

```json
{ "schema": 1, "change": "gafira-render-cpu", "delivery": "CB5", "rounds": [ {
  "round": 1, "commit": "b6693e37d1bd89c83e56810bc6d21b349bee5b1c", "mode": "standard", "tier": "T2", "depth": "V2",
  "verdict": "FAIL", "next": "architect",
  "counts": {"critical": 1, "warning": 6, "suggestion": 2},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/effect.rs:29-44, src/tests/effects.rs",
     "summary": "A tile with two ApplyEffect ops is not covered: only the first effect's roi grows the buffer (53/53 lib tests green with the break); the second is handed a crop. Probe: 12x12 instead of 16x16.",
     "follow_up": false, "status": "open"},
    {"id": "W1", "severity": "WARNING", "class": "claim",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:474, src/backend.rs:299-316",
     "summary": "The stated survivor is not equivalent: the cull at task.region drops an op wholly inside the growth and outside the tile, so the tile loses a pixel (0.4,0.3,0.2,0.5 -> 0). The cull is a pixel rule on a grown tile, not only work skipped.",
     "follow_up": false, "status": "open"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/tests/effects.rs:159",
     "summary": "No test pins the grown-rect cull: AC4's fixture's fill only touches the tile's corner and coverage::meets is inclusive, so task.region survives all 53 lib tests. Add a fill strictly inside the margin.",
     "follow_up": true, "status": "open"},
    {"id": "W3", "severity": "WARNING", "class": "architecture",
     "ref": "specs/changes/gafira-render-cpu/design.md:282-283",
     "summary": "D4 calls the layer's region 'the tile's region (grown by the effect ROI)', which contradicts 02 §4.6 (layer bounds are the task's) and D4's own next sentence. The growth is the render buffer for the effect's input; the layer stays the task's.",
     "follow_up": false, "status": "open"},
    {"id": "W4", "severity": "WARNING", "class": "ambiguity",
     "ref": "02 §4.6:688, ARQ-120, src/layer.rs:124-153",
     "summary": "A mask covers the layer and the layer is the task's region, so the margin a later effect reads stays unmasked: mask-then-effect is not tile equivalent (probe: page (8,8) 0 in one task, (1,0,0,1) in two). §4.6 needs the sentence before ARQ-134 emits these.",
     "follow_up": true, "status": "open"},
    {"id": "W5", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/effect.rs:66, apply-progress.md:479",
     "summary": "The effect's temporary grown dst is in no accounting: memory_usage and trim cannot see it. Bounded to one page-sized buffer per op: +33 MB at 1080p, +133 MB at 4K, both inside the cap; at MAX_SIDE 8192 it doubles a 1.07 GB surface past the 1.5 GB cap with no lever.",
     "follow_up": true, "status": "open"},
    {"id": "W6", "severity": "WARNING", "class": "ambiguity",
     "ref": "crates/gafira-render-cpu/src/effect.rs:88-99, ARQ-120, ARQ-149",
     "summary": "image_width/height is the surface's page rectangle, not the page, so an effect clamps at the surface edge. Right for a full-page surface; wrong for a sub-region request ARQ-149 allows, and the port carries no page extent.",
     "follow_up": true, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:481",
     "summary": "The transition's RgbaF32 inputs in both qualities are not a deviation: ARQ-118's clause is about from/to/dst, which agree and take the destination's format; O11 and 02 §4.6 fix the destination as f32. The record should say so.",
     "follow_up": false, "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "implementation",
     "ref": "the delivery's 8 files",
     "summary": "1244 insertions against the card's 420 forecast (590 at x1.4): 2.8x, 815 of them tests, 357 production. Above the ~700 signal; the coordinator decides, as the record says.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 5, "killed": 3},
  "criteria": {"total": 8, "covered": 8},
  "size": {"lines": 1244, "forecast": 590}, "report": "verify-report.md" } ] }
```

- Delivery: `CB5` of `gafira-render-cpu`, branch `feat/gafira-render-cpu-cb5`, HEAD
  `b6693e37d1bd89c83e56810bc6d21b349bee5b1c`, merge base `develop` `90603a6` (the card commit).
  Five commits: `2a7f72f` (mask), `2279fe8` (ROI growth), `6dbaa02` (transition), `e71a8fe` (the mask
  anchor), `b6693e3` (the record). Not pushed, not merged; `develop` has moved to `0bc90a9`, which
  does not change this branch's base.
- Mode `standard`, depth **V2** (the card says `verify: V2`). `mutation: off`: no `cargo-mutants`
  run and no `mutants` report, and that is not a finding; this round's rigour is the gates, ten
  independent probe tests and five hand mutants.
- Verdict: **FAIL** — 1 CRITICAL, 6 WARNING, 2 SUGGESTION. The *code* answers every acceptance
  criterion, and the anchor this round exists for is the card's, not `D4`'s (below). What fails is
  the evidence at two edges of the ROI work: a tile with **two** effects, and the grown-rect cull
  whose equivalence the record got wrong.
- Toolchain: cargo/rustc 1.93.1, clippy 0.1.93, cargo-deny 0.20.2. Disk: `df -h /` at start 171 GB
  free (433 GB used of 636 GB).

### The anchor: which buffer is the layer

**Ruling: `02 §4.6` supports the card. The mask anchors at the task's (tile's) corner, and `D4` is
what needs correcting.**

Three sentences, in the order the spec settles them:

1. Layer bounds. §4.6:687 — "`PushLayer` no lleva rectángulo, así que **los límites de la capa son
   los de la tarea**: toda la superficie del destino, que es la región de página recortada al
   viewport (ARQ-149), sin cota más fina". The layer's bounds are `TileTask.region`, which §4.6:686
   defines as the task's page rectangle clipped to the request's viewport. Nothing in §4.6 grows it,
   and an effect's private `roi` is not a property of the task.
2. The mask's anchor. §4.6:688 — "La máscara se ancla en la **esquina superior izquierda de la
   capa**, sin reescalado: un píxel de máscara por píxel de la capa, y como el op no lleva escala,
   la máscara llega del tamaño de la capa". With (1), that corner is `task.region`'s top-left, so
   inside a grown buffer it sits `region.x0 − drawn.x0` pixels in — exactly `backend.rs:278-283`.
3. The mask's reach. §4.6:688 — "Donde la máscara no llega, la capa queda como estaba". The mask is
   the layer's size, and the layer is the task's.

`D4`'s consequence (`design.md:282-283`) says "the adapter defines the layer region as the tile's
region **(grown by the effect ROI in `CB5`)**", and then "A layer is therefore never larger than a
tile" — the parenthetical contradicts both §4.6 and the design's own next sentence. The growth is
the **render buffer** that lets an effect read outside the tile; it is not the layer. The reading the
card states is also the only one a document can author: `ApplyMask` carries no rect and no scale
(§4.6:688, design `D7`), so an emitter must hand a mask cropped to a region it knows — the tile — and
a mask sized to a rectangle that depends on an effect's private `roi` is unknowable at emit time.

The implementer's `M5` (anchor at the grown origin) is therefore a genuine mutant and the fix in
`e71a8fe` is right. What the same finding exposes is that `D4` is now wrong about the layer and W3
carries the correction; W4 carries the consequence that the growth is *not* masked.

### Gates (delivery HEAD, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` in 25.44 s |
| `cargo test --workspace --locked` | 0 | **198 binaries, 2518 passed, 0 failed, 0 ignored**; the delivery's own binaries are `gafira-render-cpu` lib **53** (12 of them new: 9 `src/tests/ops.rs`, 3 `src/tests/effects.rs`) and `arq_42_comparison` 14 |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0: Format; CLI has no UI deps (ARQ-07); `forbid(unsafe_code)`; V5 layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports` (ADR 0013) |

AC8's other halves, re-run: `src/lib.rs:25` still opens with `#![forbid(unsafe_code)]`; the purity
grep `std::fs|std::net|std::thread::spawn|std::time::(Instant|SystemTime)::now|std::env` over
`crates/gafira-render-cpu/src/*` exits 1; `git diff 90603a6..HEAD -- Cargo.toml Cargo.lock
crates/gafira-render-cpu/Cargo.toml` is empty (`O2`), so no dependency and no V5 row moved.

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-cb5-v2-target` for the probe/mutant export and
`/tmp/gafira-cb5-v2-target-head` for the worktree, both with `CARGO_INCREMENTAL=0`, not shared and
not shared with any other tree. The worktree is clean (`git status` empty); no source file was
edited by this round, and the only file written is this one. Both target dirs are cleaned before the
round closes.

### Artifact integrity (fast lane, step 3c)

- `90603a6 docs(gafira-render-cpu): slice card for CB5` touches only
  `specs/changes/gafira-render-cpu/apply-progress.md` (+16), as G0 requires, and precedes the first
  implementation commit.
- No implementer commit changed the card: `2a7f72f`, `2279fe8`, `6dbaa02` and `e71a8fe` touch only
  `crates/gafira-render-cpu/**`, and `b6693e3` adds the `## CB5 (tdd-apply)` record (+72) to that
  file and nothing else. No amend commit.
- Every product ID in the card has a test that names it and proves its criterion: **ARQ-119**
  (`src/tests/ops.rs:46`, `:49`), **ARQ-110** (`:231`, `:235`; the non-finite `progress` is ARQ-110's
  typed error), **ARQ-118** (`:359`, `:363`, `:414`, `:416`, `:427`, `:430`, `:453`, `:456`, `:508`),
  **ARQ-120** (`src/tests/effects.rs:2`, `:153`, `:159`, `:193`, `:195`), **ARQ-112** (`:216`, `:220`).
  No ID is left out. **ARQ-02** is AC8's command.

### Independent probes (step 1c, before reading the delivery's tests)

Ten probe tests in my own file (`crates/gafira-render-cpu/tests/cb5_probe.rs`) on a clean `git archive`
export at HEAD — eleven checks, because P4's window assertion rides P3's scene and P11 is a side probe
— written from §4.6, the card and the product rows only: my own scenes, my own
`TileCache`, my own `Effect` and `Transition` doubles, the real `CpuBackend`.

| Probe | Criterion | What it asserts | HEAD |
|---|---|---|---|
| P1 | AC1 | a `(0.4, 0.3, 0.2, 0.5)` premultiplied layer under coverage 0, 255 and 128: all four channels scale (`0.4·128/255` on channel 0), 255 is exact | pass |
| P2 | §4.6 anchor | a task grown by an effect: the tile's `(0,0)` is the mask's `(0,0)` and becomes transparent; `(1,1)` unchanged | pass |
| P3 | AC4 | a fill **wholly outside** the tile (page 12..15 under a tile at 16..24) reaches the crop through a 4 px shift; the effect ran once | pass |
| P4 | AC4, ARQ-120 | `cx.tile` at run time is `(image 32×32, x 12, y 12, src 16×16)` — the roi's origin, not `out`'s | pass |
| P5 | AC4 | two effects on one tile: the second sees the union of both rois (16×16 from a 2 px and a 4 px pad) | pass |
| P6 | AC6 | `progress` 0 equals a background-only render and 1 equals the same ops with no transition, whole surface, and the two references differ | pass |
| P7 | AC6 | nested: at 0 the inner layer holds the enclosing layer (`(0,0,1,1)`), not the page | pass |
| P8 | AC7 | −0.5 reaches the op as 0.0 and 1.5 as 1.0, whole surface equal to the ends; NaN and ±∞ are refused and never reach the op | pass |
| P9 | AC3 | an `ApplyMask` with no open layer answers `DriverBlocked` | pass |
| P10 | AC5 | an effect that cancels inside `run_cpu` answers `Cancelled` and writes no tile to the cache | pass |
| P11 | W4 (side probe) | the same mask-then-effect ops: page (8,8) is `(0,0,0,0)` in one whole-page task and `(1,0,0,1)` in two half tiles | differs |

P6 also records that `from`, `to` and `dst` are all `RgbaF32` and all 8×8 — the three op buffers of
ARQ-118 agree, which is what S1 is about. P3 and P4 are the probes that pin what the delivery's own
suite does not (W1, W2) and what it cannot (P5 → C1).

### Hand mutants (step 4b)

Five applied on the green export tree, the named test run alone, the tree reverted each time
(`git status`-equivalent: the export is not a repository, so each file was restored from a copy and
the suite re-run green).

| # | Mutation | Killed? | What shows it |
|---|---|---|---|
| S (spot-check of the record's `M5`) | `mask_origin` is `(0, 0)` instead of the task's offset in the buffer | yes | `spec_02_4_6_the_mask_anchors_at_the_tile_when_the_task_grows`, exactly as recorded: `(0, 0) channel 0: … got 0.07058824 expected 0` |
| H1 (the stated survivor) | both op culls meet `task.region` instead of `drawn` | **no** — 53/53 lib tests green | no test can see it (the cull only differs when an effect grew the buffer, and no emitted plan has one); my P3 dies (`(0.4,0.3,0.2,0.5)` → `0`). W1 |
| H2 | `grown_rect` stops after the first `ApplyEffect` | **no** — 53/53 lib tests green | my P5 dies: the second effect is handed 12×12 instead of 16×16, so it reads a crop. C1 |
| H3 | `paint_fill` builds its coverage at `(0,0)` instead of the drawn rectangle's origin | yes | several tests, among them `arq_120_the_effect_reads_past_the_tile_edge_and_runs_once` and `arq_149_the_surface_sits_at_the_origin_and_clips_to_the_target` |
| H4 | the transition's clamp keeps only the low side (`max(0.0)`) | yes | `arq_118_the_backend_clips_the_progress_it_hands_a_transition`; the two `Dissolve` tests stay green, which is `M11`'s mechanism reproduced |

3 killed, 2 missed. H1 is documented as a survivor by the record and is **not equivalent** (W1);
H2 is a survivor in the design's critical code (C1).

### The stated survivor, judged the way `CB4`'s `M7` was

`CB4`'s `M7` earned the cull a place as work skipped: removing it changed no pixel, and `W10`
recorded that. `CB5`'s survivor is **not the same case**. `coverage::meets` is inclusive, so an op
whose box merely *touches* the tile is drawn either way — that is why the record's fixture (a fill
ending exactly at the tile's top-left, `src/tests/effects.rs:162`) survives the mutation. An op whose
box lies **strictly inside the grown margin** is a different animal: with the cull at `task.region` it
is dropped, and the effect that was grown to read it pulls black into the crop. P3 is exactly that
scene, and it fails under H1.

So the cull does earn its place on a grown tile — as a **pixel rule**, not only as an optimisation —
and the record's sentence ("changes no pixel … The cull is an optimisation over the sampling, exactly
as `CB4` recorded") is wrong for the case the growth exists to serve (W1). The code is on the right
side of it; nothing pins it (W2).

### The four flagged risks, ruled

1. **The effect's temporary grown dst is outside `memory_usage`.** It is a real hole in the
   accounting: `effect::run` allocates `vec![0; target.len()]` (`src/effect.rs:66`), frees it when the
   op returns, and neither `bytes()` nor `trim()` can ever see it, so the governor cannot ask for it
   and cannot shrink it — §4.6:690 gives the governor exactly `bytes()`/`trim()` on a cache, and this
   is neither a cache nor a persistent buffer. Does it matter at the sizes `ARQ-120` allows? At the
   two sizes the crate documents and measures — 1080p (`+33.2 MB`, peak 66 MB with the surface) and
   4K (`+132.7 MB`, peak 265 MB) — no: it stays far inside `D-03`'s 1.5 GB. At the top of the range
   the guard accepts (`MAX_SIDE = 8192`, `src/surface.rs:23`) the surface alone is 1.07 GB and one
   full-page temp buffer doubles it to 2.14 GB, past the cap, while `memory_usage()` reports ~1.07 GB
   and there is no lever to pull. Bounded to one buffer per effect op, and one op at a time. W5
   (follow-up); it does not fail the delivery.
2. **`cx.tile` is the tile's window for `roi` and the grown window for `run_cpu`.** That is
   self-consistent with the contract: §4.4's `TileWindow` is "dónde están `src` y `dst` en la imagen
   entera, en píxeles de `src`", so the window must describe the buffer that call is about. At `roi`
   time the input under discussion is the tile (`out` is the tile); at run time `src` **is** the grown
   region, and ARQ-120's "`x` e `y` son el origen de la `roi` después de intersecarla con la imagen,
   no el de `out`" is precisely the run call — P4 pins `(32, 32, 12, 12, 16, 16)`. `EffectCx::whole`
   is not in play here. `image_width/height` as the surface's page rectangle is the right extent
   **exactly when the surface is the page**, which is every render this delivery tests and ARQ-42's
   comparison; for a request whose region is smaller than the page — which ARQ-149 allows and §6.2's
   viewport renders will use — the effect clamps at the surface edge instead of the page's, and the
   port carries no page extent to do better. W6 (follow-up), not a defect of this delivery.
3. **`RgbaF32` inputs in both qualities.** ARQ-118's clause — "«Desde», «hacia» y destino tienen las
   mismas dimensiones y el mismo formato, `Rgba8` o `RgbaF32`" — is about the **op's three buffers**,
   not about the backend's internal accumulation or about the format `read_back` answers. Here all
   three are `RgbaF32` and the same size (P6 records it), which is the destination's format: `O11`
   and §4.6:1283 fix the destination as `f32` ("la vista previa acumula en `f32` y cuantiza una sola
   vez, al entregar la superficie"). So this is **not a deviation**; the record states the choice but
   not the reason it satisfies ARQ-118 as written. S1.
4. **`M11`: is the op's own `progress` clip redundant?** No. AC7 is the card's criterion ("Given a
   finite `progress` outside `[0, 1]`, When it runs, Then it is clipped to the range, and a NaN or
   infinite one answers `Err` (ARQ-118)"), so the clip is the backend's guarantee **at its port
   boundary** — the plan and `RenderOp` are the port's, a transition is a plugin (§4.13), and ARQ-118
   puts the errors before writing. The core transitions also guard the value, which is why the two
   `Dissolve` tests cannot see the op's own clip; the probe test is the only observable and must
   exist, and it does (`src/tests/ops.rs:508`, asserting both the clipped value the op was handed and
   that a non-finite one never reaches it). H4 reproduces `M11`'s asymmetry exactly — only the probe
   test dies. Keep both the clip and the probe.

### Criteria

| Criterion | Proof | Status |
|---|---|---|
| AC1 four premultiplied channels | `arq_119_a_mask_scales_all_four_premultiplied_channels` (`ops.rs:49`), P1 | covered |
| AC2 a smaller mask masks its rectangle only | `spec_02_4_6_a_mask_smaller_than_the_layer_masks_only_its_rectangle` (`ops.rs:118`, all 65 536 pixels of the spec's own 128×128-over-256×256 case), P1 | covered |
| AC3 a mask with no layer is refused | `spec_02_4_6_a_mask_outside_a_layer_is_refused` (`ops.rs:235`), P9 | covered |
| AC4 the grown rectangle, one run, the crop | `arq_120_the_effect_reads_past_the_tile_edge_and_runs_once` (`effects.rs:159`), `arq_120_the_grown_rectangle_is_clipped_to_the_page` (`:195`), P3, P4 | covered for one effect; a tile with two is C1 |
| AC5 a cancelled effect aborts the tile | `arq_112_a_cancelled_effect_aborts_the_tile_before_it_is_committed` (`effects.rs:220`), P10 | covered |
| AC6 `from` at 0 and `to` at 1, exact | `spec_02_4_6_a_transition_takes_from_the_background_and_to_the_layer` (`ops.rs:363`), `spec_02_4_6_a_nested_transition_takes_from_the_layer_it_composites_onto` (`:456`), P6, P7 | covered |
| AC7 the progress clip and the non-finite refusal | `arq_118_a_finite_progress_outside_the_range_is_clipped` (`:416`), `arq_110_a_non_finite_progress_is_refused` (`:430`), `arq_118_the_backend_clips_the_progress_it_hands_a_transition` (`:508`), P8 | covered |
| AC8 the crate's policy | `cargo fmt`, `clippy`, the nine `policy` steps, the purity grep, the empty manifest diff | covered (command) |

AC6's specific question: the two whole-surface comparisons really are against **different renders** —
`background` is a render of the background fill alone (`ops.rs:365-372`), `drawn` is a render of the
same ops with no transition (`:374-384`), `values()` is the whole surface (`tests.rs:360-375`), and
the test asserts `background != drawn` (`:400-404`) before comparing. The nested case exists
(`:456`), and it compares the two readings through `blend_f32` and asserts they differ. P6/P7 confirm
both independently. AC6 is the criterion that pins which buffer is `from`, and it does.

### Findings

| id | severity | class | what to change |
|---|---|---|---|
| C1 | CRITICAL | implementation | cover a tile with two `ApplyEffect` ops (the union of their rois); the second effect is currently handed a crop and the whole suite is green |
| W1 | WARNING | claim | the record's survivor is not equivalent; say the grown-rect cull is a pixel rule, not only skipped work |
| W2 | WARNING | implementation (follow-up) | add the fill strictly inside the margin so the cull's rectangle is pinned |
| W3 | WARNING | architecture | correct `D4`: the growth is the render buffer, the layer stays the task's region |
| W4 | WARNING | ambiguity (follow-up) | say in §4.6 that a mask cannot reach the growth, so mask-then-effect is not tile equivalent; the emitter (ARQ-134) must know before it emits |
| W5 | WARNING | implementation (follow-up) | the effect's temp buffer is outside `memory_usage`; bound it or report it, and say what happens at `MAX_SIDE` |
| W6 | WARNING | ambiguity (follow-up) | `image_width/height` is the surface, not the page; ARQ-120's "tamaño de la imagen entera" cannot be honoured for a sub-region request until the port carries the page extent |
| S1 | SUGGESTION | ambiguity | state that ARQ-118's format clause is satisfied by the three op buffers being `f32`, so the record does not read as an unexcused deviation |
| S2 | SUGGESTION | implementation | 1244 insertions, 2.8× the card's forecast (815 test lines, 357 production): the coordinator's size decision |

C1's evidence, so the fix can be checked: with `break` after the first `grown_rect` union the whole
library suite is 53/53 green (the 14 `arq_42_comparison` tests cannot be affected here: no
emitted plan carries an effect), and a second effect asking 4 px on a tile asking 2 px is handed 12×12
where the correct union is 16×16 — a blur at a tile edge then has no input where it needs one. The
code at HEAD is right; the suite is what is missing.

### Observations (not findings)

- **`M5` and `M11` reproduce.** `M5` is the row I spot-checked and it fails on the same assertion
  with the same numbers; `M11`'s mechanism reproduces through H4 (the two `Dissolve` tests stay green
  when only the high side of the clamp goes). Those two rows are evidence, not a claim; the other
  eleven were not re-run.
- **The mask's overlap clip is a refinement §4.6 does not require and does not forbid.** `columns`
  and `rows` are clipped to the buffer as well as to the mask, so a mask larger than the layer is
  harmless (a panic would be ARQ-110's failure mode). Nothing tests a mask larger than the layer; no
  test needs to.
- **`ApplyTransition` with no open layer is refused (`DriverBlocked`), which the card states only for
  `ApplyMask`.** §4.6 defines `from` through the `PopLayer`, so a transition outside a pair is as
  undefined as a mask; the choice is recorded and the port has no plan-validation error. Accepted.
- **Nothing emits these three ops from a document** (ARQ-134), so all of this is proven on synthetic
  plans. The record says so, and P11 is the shape of what the emitter will have to decide.
- **`EffectCx::whole(1.0)` for the transition is right at any level in this adapter**, because a tile
  of this backend holds one buffer pixel per page pixel (ARQ-149) and §4.13 says a transition ignores
  `cx.tile`.
- **No `cargo-mutants` run and no T4 report**, per `mutation: off`. Absence of a mutation score is not
  a gap this round, as round 1 of `CB4` recorded.

next: architect (then tdd-apply for C1/W2 — implementation; specifier for W4/W6 — ambiguity; explorer for W1 — claim)

---

## Round 2 — CB5 (closure)

```json
{ "schema": 1, "change": "gafira-render-cpu", "delivery": "CB5", "rounds": [ {
  "round": 2, "commit": "47ec4f1a369bcf6269bbc908018595a9ceb5b19a", "mode": "standard", "tier": "T2", "depth": "V1",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 7, "suggestion": 1},
  "classes": {"architecture": 1, "implementation": 1},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/effect.rs:42-60, src/tests/effects.rs:199",
     "summary": "Closed: grown_rect asks each roi against the buffer the previous ops produced. M-C1 reproduced fails arq_120_two_effects_grow_by_the_buffer_the_first_produced with left [(14, 14)] right [(16, 16)].",
     "follow_up": false, "status": "closed"},
    {"id": "W1", "severity": "WARNING", "class": "claim",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:474-478",
     "summary": "Closed: the record now calls the task.region cull NOT equivalent and names it a pixel rule on a grown tile; M-C2 kills the new test, so the sentence matches the code.",
     "follow_up": false, "status": "closed"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/tests/effects.rs:260",
     "summary": "Closed: arq_120_a_fill_inside_the_margin_reaches_the_tile_through_the_effect puts the fill at (4,4)-(6,6), outside the tile and inside the growth; M-C2 fails it with 56/57 green.",
     "follow_up": true, "status": "closed"},
    {"id": "W3", "severity": "WARNING", "class": "architecture",
     "ref": "specs/changes/gafira-render-cpu/design.md:292-303",
     "summary": "Closed by O15: D4 now says the layer's region is the task's and the roi grows a render buffer that is never the layer; the rejected reading and the emitter cost are stated.",
     "follow_up": false, "status": "closed"},
    {"id": "W4", "severity": "WARNING", "class": "ambiguity",
     "ref": "design.md O16, SP7; 02 §4.6:688 unchanged",
     "summary": "Still open, as routed: O16 states that mask-then-effect is not tile equivalent and SP7 (open) is the specifier's; §4.6 itself still anchors the mask in task space, and SP7 blocks ARQ-134.",
     "follow_up": true, "status": "open"},
    {"id": "W5", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/backend.rs:65, :233-263; src/effect.rs:142-165",
     "summary": "Closed by O17 and this fix: one backend scratch, in memory_usage().host, dropped by trim, refused against SURFACE_BUDGET before it grows. M-C3 and M-C3b reproduce the recorded numbers.",
     "follow_up": true, "status": "closed"},
    {"id": "W6", "severity": "WARNING", "class": "ambiguity",
     "ref": "design.md O18, the CB5b row",
     "summary": "Still open, as routed: O18 rules the window is page space and TargetDesc carries no page extent; the port change is listed as CB5b before CB6, not written yet.",
     "follow_up": true, "status": "open"},
    {"id": "W7", "severity": "WARNING", "class": "architecture",
     "ref": "specs/changes/gafira-render-cpu/design.md:356-364",
     "summary": "D7 still says the adapter computes roi(cx, p, tile_rect) and runs with EffectCx { tile: None }; that is now literally M-C1, and the code asks the grown buffer with Some(window). O15 corrected D4, not this line.",
     "follow_up": false, "status": "open"},
    {"id": "W8", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/backend.rs:109, :362; src/tests/effects.rs:297",
     "summary": "The refusal's held value is unpinned end-to-end: held = 0 and dropping layers.bytes() both leave 57/57 green, so only reserve_scratch's own boundary is tested. A cheaper seam than a 1 GB surface exists.",
     "follow_up": true, "status": "open"},
    {"id": "W9", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/effect.rs:115; src/tests/effects.rs:297",
     "summary": "The reused scratch's clear is unpinned: removing dst.fill(0.0) leaves 57/57 green, because every effect double writes all of dst. ARQ-111's determinism here rests on an unasserted line.",
     "follow_up": true, "status": "open"},
    {"id": "W10", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-render-cpu/src/backend.rs:302-319, :233-250",
     "summary": "The bounded peak is not the frame's peak: the tile's drawn buffer (drawn pixels x 16 B, 1.07 GB at MAX_SIDE) is in no term and in no memory_usage, and layers.push is never refused. 1.07 + 1.07 + 0.43 GB is accepted.",
     "follow_up": true, "status": "open"},
    {"id": "W11", "severity": "WARNING", "class": "claim",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:507-509",
     "summary": "The stated residual is right for dilation and translation rois (the union telescopes) but wrong for a shrinking roi before a dilating one: the tile then reads an edge no op fetched. Unreachable today (ARQ-134), and the record does not name the shape.",
     "follow_up": true, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-render-cpu/apply-progress.md:489-492",
     "summary": "Closed: the transition bullet now states that RgbaF32 inputs are the destination's format under O11 and that ARQ-118's clause is about the op's three buffers, as recorded.",
     "follow_up": false, "status": "closed"},
    {"id": "S2", "severity": "SUGGESTION", "class": "implementation",
     "ref": "the delivery's code and tests",
     "summary": "Re-measured: 1586 insertions (1244 + this round's 342: 177 tests, 165 production) against the card's 420 forecast, 3.8x (2.7x at x1.4). The coordinator decides.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 9, "killed": 6},
  "criteria": {"total": 8, "covered": 8},
  "size": {"lines": 1586, "forecast": 420}, "report": "verify-report.md" } ] }
```

- Delivery: `CB5` of `gafira-render-cpu`, branch `feat/gafira-render-cpu-cb5`, HEAD
  `47ec4f1a369bcf6269bbc908018595a9ceb5b19a`. Round 2 is a **closure round** over the round-1
  `FAIL` (`C1`, `W1`, `W2`) and over `O17`, the ruling `W5` became. The commits under test are
  `2e59d68` (the architect's four rulings, `O15`-`O18`, and the `CB5b` row), `6370d3c` (the composed
  growth, +52/−3), `8f42904` (the cull's test, +33), `a517c3e` (the scratch, +257/−32) and `47ec4f1`
  (the record, +43/−4). Base of this round is the round-1 report, committed inside `2e59d68`.
- Mode `standard`, depth **V1** as the coordinator set it: the five gates, the artifact-integrity
  checks, and one reproduced mutant per earlier finding. `mutation: off` — no `cargo-mutants` run and
  no `mutants` report, which is not a finding. The nine mutants below are *reproductions of the
  earlier findings* plus three probes of the fix's own claims, not a new independent-mutant budget:
  six were killed, three survived, and the three survivors are `W8`–`W10`.
- Verdict: **PASS** — 0 CRITICAL. `C1`, `W1`, `W2`, `W3`, `W5` and `S1` are closed, each proved by
  applying the mutant on a clean export and watching the new test fail with the recorded message.
  `W4` and `W6` stay open as routed follow-ups (`SP7`, `CB5b`); `S2` stays open as a SUGGESTION.
  Four new warnings, none of them a code defect: `W7` (the design's `D7` still prescribes the mutation
  `C1` fixed), `W8`/`W9` (two O17 properties only inspection pins) and `W10` (the peak O17 bounds is
  not the frame's peak), plus `W11` on the precision of the stated residual.
- Toolchain: cargo/rustc 1.93.1, clippy 0.1.93, cargo-deny 0.20.2. Disk: `df -h /` at start **156 GB**
  free (448 GB used of 636 GB).

### Gates (delivery HEAD, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` in 27.60 s |
| `cargo test --workspace --locked` | 0 | **198 test binaries, 2522 passed, 0 failed, 0 ignored** (round 1: 2518 — the +4 are this round's tests); the delivery's own binaries are `gafira-render-cpu` lib **57** (53 at round 1) and `arq_42_comparison` 14 |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0: Format; CLI has no UI deps (ARQ-07); `forbid(unsafe_code)`; V5 layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports` (ADR 0013) |

The nine steps are the `run:` steps of the `policy` job after the toolchain install, extracted by
`/tmp/gafira-policy-run.py` and re-checked against `.github/workflows/ci.yml`. Build hygiene:
`CARGO_TARGET_DIR=/tmp/gafira-cb5-v2b-target` for the worktree gates and
`/tmp/gafira-cb5-r2-export-target` for the clean `git archive` export, both with `CARGO_INCREMENTAL=0`,
not shared and not shared with any other tree; both freed with `cargo clean --target-dir` at the end.

### Artifact integrity (steps 3, 3c)

- The card is untouched. `47ec4f1 docs(gafira-render-cpu): record the CB5 fix round` has one hunk,
  `@@ -471,17 +471,56 @@`, so `apply-progress.md`'s `## CB5` block at lines 401-416 is byte-identical to
  `90603a6`, and the fix does not change an AC, a Given/When/Then line or a product ID.
- No implementer commit touched a spec file: `6370d3c`, `8f42904` and `a517c3e` touch only
  `crates/gafira-render-cpu/**`; `47ec4f1` touches only `apply-progress.md`. No amend commit.
- `2e59d68` is the architect's: `design.md` +247/−14 (the `O15`-`O18` section, the corrected `D4`,
  `SP7`, `XP5`, the `CB5b` row, the "cannot prove" line) **and** the round-1 report at +295, which is
  the verifier's file committed by the architect's commit. That is not an acceptance-text change and
  not a finding — the report is the record and committing it makes it durable — but the coordinator
  should know the architect's commit carries it.
- Every product ID the card names still has a test that names it and proves its criterion: **ARQ-119**
  (2 sites), **ARQ-110** (2), **ARQ-118** (7), **ARQ-120** (5, now including the chain and the cull),
  **ARQ-112** (1), **ARQ-110** as the non-finite refusal, and **ARQ-02** is AC8's command. `ARQ-143`
  and **ARQ-44** gain their first tests (`O17`). No ID was left out, and the crate's non-goals hold:
  `git diff --numstat 2e59d68..HEAD -- crates/` lists three files, all in `gafira-render-cpu`, and
  `Cargo.toml`/`Cargo.lock` did not move.
- The worktree is clean; the only file this round wrote is this report.

### Closure of the earlier findings

| Id | Round-1 finding | What closes it | How it was checked this round | Status |
|---|---|---|---|---|
| `C1` | A tile with two `ApplyEffect` ops is uncovered; only the first `roi` grows the buffer (53/53 green) | `effect::grown_rect` asks each `roi` over the buffer the previous ops produced (`effect.rs:52-57`) and `arq_120_two_effects_grow_by_the_buffer_the_first_produced` pins 16×16 | M-C1 reproduced: the test fails at `effects.rs:219` with `the buffer the first effect produced` — `left: [(14, 14)]`, `right: [(16, 16)]`, exactly the recorded numbers | **closed** |
| `W1` | The record calls the `task.region` cull equivalent; it drops a pixel the growth exists to fetch | The paragraph now reads "**not** an equivalent mutant … the cull is a pixel rule, not only skipped work" (`apply-progress.md:474-478`) | M-C2 reproduced: `arq_120_a_fill_inside_the_margin_…` fails at `effects.rs:270`; the claim now matches the measured behaviour | **closed** |
| `W2` | No test pins the grown-rect cull: the fixture's fill only touches the tile's corner | `arq_120_a_fill_inside_the_margin_reaches_the_tile_through_the_effect` (`effects.rs:260`), fill at (4,4)-(6,6) with a control outside the growth | M-C2 reproduced: **56 passed, 1 failed**, and the failure is that test only | **closed** |
| `W3` | `D4` said the layer's region is the tile's "grown by the effect ROI" | `O15` corrected it: the layer's region is the task's, the growth is a render buffer (`design.md:292-303`) | Read the corrected text; it agrees with `backend.rs:313-318` (`mask_origin`) and with §4.6:687-688 | **closed** |
| `W4` | A mask's reach is the task's, so mask-then-effect is not tile equivalent; §4.6 needs a sentence | `O16` rules it and `SP7` (open) carries the spec change; the design's "cannot prove" table now says it | §4.6 is untouched, so the ambiguity is still in the spec; `SP7` blocks `ARQ-134` | open (follow-up, `SP7`) |
| `W5` | The effect's temp destination is outside `memory_usage` and `trim` | `O17` and `a517c3e`: one backend scratch, in `host`, dropped by `trim`, refused before it grows | M-C3 (`left: Bytes(17408)`, `right: Bytes(21504)`) and M-C3b (`left: Bytes(4096)`, `right: Bytes(0)`) reproduce | **closed** |
| `W6` | `image_width/height` is the surface, not the page; the port carries no page extent | `O18` rules it and the `CB5b` row puts the port change before `CB6` | The code still clamps at the surface edge; the port change is a delivery, not done | open (follow-up, `CB5b`) |
| `S1` | The record does not say why `RgbaF32` inputs satisfy ARQ-118 | The bullet now says the clause is about the op's three buffers and `O11`/§4.6 fix the destination as `f32` | Read it: `apply-progress.md:489-492` | **closed** |
| `S2` | 1244 insertions against the 420 forecast | the coordinator's call | Re-measured: **1586** (3.8×, 2.7× at ×1.4); see *Size* | open (SUGGESTION) |
| `O17` | The ruling `W5` became: one scratch, counted, trimmed, refused first | `effect::reserve_scratch` + `memory_usage` + `trim` + the two tests | M-C3, M-C3b, M-C4 and M-C5 all killed; the boundary arithmetic below | **closed** as ruled |

### The mutants reproduced

All nine on a clean `git archive` export of `47ec4f1` (`/tmp/gafira-cb5-r2-export`) with its own
target dir; each applied alone, the named test or the whole lib run, then the file restored from a
copy. The export's green baseline before the first mutant and its closing run after the last both
report **57 passed**; `diff -q` against the backups is empty for `backend.rs` and `effect.rs`.

| # | Mutation | Killed? | What shows it |
|---|---|---|---|
| M-C1 | `grown_rect` asks every `roi` against the tile instead of the buffer the previous ops produced (`effect.rs:55`) | **yes** | `arq_120_two_effects_grow_by_the_buffer_the_first_produced`: `the buffer the first effect produced` — `left: [(14, 14)]`, `right: [(16, 16)]` |
| M-C2 | both op culls meet `task.region` instead of `drawn` (`backend.rs:334`, `:349`) | **yes** | `arq_120_a_fill_inside_the_margin_reaches_the_tile_through_the_effect` fails at `effects.rs:270`; **56 passed, 1 failed** |
| M-C3 | `memory_usage` leaves the scratch out of `host` | **yes** | `arq_143_the_effect_scratch_is_accounted_and_trim_drops_it`: `left: Bytes(17408)`, `right: Bytes(21504)` |
| M-C3b | `trim` drops the surface and the pool but keeps the scratch | **yes** | the same test: `trim drops the scratch with the surface and the pool` — `left: Bytes(4096)`, `right: Bytes(0)` |
| M-C4 | `fits` compares with `<`, so the cap is exclusive | **yes** | `arq_44_the_effect_peak_is_refused_before_the_scratch_is_allocated`: `the cap is inclusive: OutOfMemory` |
| M-C5 | the scratch is resized before the peak is checked | **yes** | the same test: `the refusal precedes the allocation` |
| P-A | the reused slice is not cleared (`dst.fill(0.0)` removed) | **no** — 57/57 green | every effect double writes all of `dst`, so nothing observes the reuse. `W9` |
| P-B | `execute` hands `held = 0` to the effect (`backend.rs:109`) | **no** — 57/57 green | the refusal's surface term is unobserved end-to-end. `W8` |
| P-C | the open layers are left out of `held` (`backend.rs:362`) | **no** — 57/57 green | the same: `layers.bytes()` never reaches a test through `execute`. `W8` |

### `C1` — closed, and the forward pass judged

The fix does what it says: `grown_rect` starts at the tile, and for each `ApplyEffect` in plan order
it asks `roi` with the **current** buffer as `out` (and as the window it sits in), then unions the
answer in. On the test's scene — tile (8,8)-(16,16), `Pad(2)` then `Shift(4)` — the first asks
(6,6)-(18,18), the second asks (2,2)-(14,14) of that, and the union is 16×16. Asking the second
against the tile gives (4,4)-(12,12) and 14×14, which is M-C1 and the round's number. The defect was
real, the fix is the right one, and the test pins both the rectangle and the single call per effect.

**The residual, judged.** The record states that "an effect's own input is not re-expanded for a later
effect's growth … a fixed-point loop would need a bound the port does not carry". That is accurate,
and for the rois real filters answer it costs nothing:

- For a **dilation** roi (`roi(out) = out ⊕ r`, a blur, a dilate, a drop shadow) the pass telescopes:
  `G₁ = T ⊕ r₁`, `G₂ = G₁ ⊕ r₂ = T ⊕ (r₁+r₂)`, which is exactly the input effect 1 needs to produce
  the region effect 2 reads. Exact, no fixed point.
- For a **translation** roi (a displacement map, `roi(out) = out + t`) it is exact for the same
  reason: `G₂ = T ∪ (T+t) ∪ (T+2t)` contains the `T+2t` effect 1 needs to feed effect 2 at `T+t`.
- It is **not** exact for a roi that shrinks its argument, which a mip-based or downsample effect can
  honestly answer: with `roi₁(X) = 2X` and `roi₂(X) = X ⊕ 1`, the pass yields `2T ⊕ 1` while the tile
  needs `2(T ⊕ 1)`, one pixel wider on the low side. Effect 1's output there is computed from input no
  op fetched, and effect 2 pulls it into the tile — a wrong pixel, not a smaller buffer.

So: a **correct limit, exactly stated as a limit** — but its consequence is a tile that can differ from
the whole-image render, and the record frames it as re-expansion rather than as that consequence; `W11`
asks for the sentence, not for the loop. Unreachable today: nothing emits `ApplyEffect` (`ARQ-134`), and
the emitter is the stage that would need the bound. No change is owed by this delivery, and the test
it does carry (two dilations) is the shape a real chain has.

### `W1`/`W2` — closed, and the cull is now the rule with a kill

M-C2 is the exact mutant the record names, and it is killed by exactly one test of 57. The fixture
matters: `coverage::meets` is inclusive, so the round-1 fill that ended on the tile's corner was drawn
under either rectangle, which is why the mutant survived then. The new fill is at (4,4)-(6,6) — wholly
outside the tile (8,8)-(16,16) and wholly inside the 4 px growth the `Shift(4)` asks for — so the
`task.region` cull drops it and the effect pulls transparent black into the tile. The control scene (a
fill at (0,0)-(2,2), outside the growth) keeps the pixel clear, so the assertion is the fill's and not a
default the effect writes. The record's paragraph now says the same thing in the same words ("**not**
an equivalent mutant … a pixel rule"), with `M-C2` named as its kill. `W1` and `W2` are closed, and the
`CB4` `M7` comparison the round made is now correctly distinguished in the record.

### `O17` — closed as ruled, and the arithmetic at the boundary

The scratch is one `Vec<f32>` on `CpuBackend` (`backend.rs:65`), handed to `run_cpu` as `dst`,
**cleared** before every effect (`effect.rs:115`), counted as `len × 4` in `memory_usage().host`
beside the surface, the observed cache bytes and `layers.bytes()` (`backend.rs:233-250`), dropped by
`trim` with the surface and the pool (`:252-263`), and grown only through `reserve_scratch`, which
compares **before** the `resize` (`effect.rs:150-158`) with the same constant and the same error
`Surface::new` uses (`surface.rs:54, :68`: `Bytes(bytes) > SURFACE_BUDGET`).

Boundary arithmetic, checked rather than read off the code: `SURFACE_BUDGET = 1 500 000 000` B;
16 `f32` is 64 B; `held = 1 499 999 936` gives exactly `1 500 000 000` and **fits** (M-C4's `<` fails
here, `the cap is inclusive`); `held = 1 499 999 937` gives `1 500 000 001` and **refuses**, with the
scratch still empty (M-C5's resize-first fails `the refusal precedes the allocation`). The `O17`
numbers reproduce: at `MAX_SIDE`, the 8192² surface is `1 073 741 824` B and a page-sized destination is
the same, so `2 147 483 648` B is refused by `reserve_scratch` without being allocated. The accounting
test's `21 504` is the surface (`16 384`), the cached tile (`1 024`) and one `16²` scratch (`4 096`);
round 1's `17 408` is the same figure without it, and the two effects serving one buffer are why it is
one scratch and not two.

The two halves this round's probes could not kill are `W8` and `W9` below; both are test gaps, not code
defects, and both are cheap to close without a 1 GB allocation.

### `W7` — `D7` still prescribes the mutation `C1` fixed

`design.md:356-364` still reads: "the adapter computes `roi(cx, p, tile_rect)`, renders the task's ops
into a scratch buffer of that size …, runs `run_cpu` with `EffectCx { scale, tile: None }`, and blits
the crop." Against HEAD that is wrong twice, and one of the two is the defect this round fixed:

1. `roi(cx, p, tile_rect)` is **M-C1** word for word. The code asks `roi` with the buffer the previous
   ops produced (`effect.rs:55`), the card's own Plan line says `roi(cx, p, out)` and the record says
   the composition is the fix. A delivery planned from `D7` would reintroduce `C1`.
2. `EffectCx { scale, tile: None }` contradicts `effect.rs:173-184`, which builds
   `Some(TileWindow { image_width, image_height, x, y })` over the grown region — the window round 1's
   probe P4 and the record's `cx.tile` bullet both accept as the right one. That half predates this fix
   (it has been `Some` since the ROI work), so it is not a regression, but the architect's `O15` corrected
   `D4` and left `D7`, which is the decision the effect's reading actually lives in.

This is a live contradiction with the code, with the card and with the fix's own record, and it is the
only finding this round whose owner is the design. `W7`, class `architecture`, no follow-up: the words
are one sentence, and the next delivery looks at `D7` before it cuts work.

### The two O17 properties only inspection pins (`W8`, `W9`)

The fix's stated risk is real, and the probes measure it: the refusal is exercised **through
`reserve_scratch`**, and nothing drives `execute` to a refusal. That is enough for the **rule** — the
cap's boundary is asserted in both directions and the 8192² numbers are refused without allocating —
but not for the **wiring**, and the wiring is two thirds of the guarantee:

- `held` reaches the rule from `execute` as *surface bytes* (`backend.rs:109`) plus `layers.bytes()`
  (`:362`). Setting the first to `0` (P-B) and dropping the second (P-C) both leave **57/57 green**.
  So nothing in the suite can tell whether the surface and the open layers are in the comparison at
  all — which is the sentence `O17` is built on ("with the open layers in the comparison").
- The clear (`effect.rs:115`) is the ARQ-111 half of the reuse: the old code allocated a zeroed
  destination per op, and now one slice serves every op and tile. Removing the `fill` leaves **57/57
  green** (P-A), because both doubles write every pixel of `dst`; a sparse writer — which `Effect`'s
  contract does not forbid (`gafira-raster/src/effects/mod.rs:49-72` says only that an **error** writes
  nothing) — would then read the previous op's bytes. The code is right; the property is unasserted.

Both are cheap: a `#[cfg(test)]` seam that calls `render_tile`/`reserve_scratch` with a `held` near the
cap settles `W8` with a few bytes, and a partial-writing `Probe` shape settles `W9`. Neither is owed by
the fix as the ruling words it, so both are WARNINGs marked follow-up rather than a FAIL.

### `W10` — the peak O17 bounds is not the frame's peak

The refusal bounds `surface + layers.bytes() + the new scratch`. Two terms of the frame's real peak are
outside it:

- The tile's own drawn buffer. `render_tile` allocates `vec![0.0f32; count]` with `count = drawn
  pixels × 4` (`backend.rs:302-319`) and that buffer lives for the whole tile pass — it is what every op
  composites into before the crop. It is in no `held` term and in no `memory_usage`. At `MAX_SIDE` a
  page-sized tile makes it `1 073 741 824` B, the same size as the surface: the scratch can be accepted
  at `surface 1.07 GB + scratch ≤ 0.43 GB` and the frame then really holds `1.07 + 1.07 + 0.43 ≈
  2.57 GB`, 1.7× the cap the check calls a ceiling. At 1080p it is 33 MB, invisible; at `MAX_SIDE` it is
  the doubling `W5` measured, one buffer up.
- The layer buffers' own growth. `layers.push(count)` is given the **grown** count and never compares
  anything with `SURFACE_BUDGET`; `O17`'s sentence "the check above covers it because `layers.bytes()`
  is already a term" covers only the *scratch* — a plan with two open layers on a page-sized grown tile
  holds `2 × 1.07 GB` that no refusal ever sees.

Both predate this fix (the tile buffer since `CB1`, the push rule since `CB2`), which is why this is a
follow-up WARNING and not a regression of `CB5`. It is recorded because `O17`'s stated purpose — "the
largest frame the backend accepts is bounded by the budget, with a lever" — is only two thirds true,
and because the reported `host` figure is the honest one (`memory_usage` also leaves the tile buffer
out, so `trim` is asked about `surface + cache + layers + scratch` while the process holds one buffer
more). The scratch itself is inside the rule, exactly as ruled.

### Size

Re-measured from git, not from the record:

- `git diff --numstat 2e59d68..HEAD -- crates/` → **342 insertions, 35 deletions** in three files:
  `src/tests/effects.rs` +177 (tests), `src/effect.rs` +100/−15 and `src/backend.rs` +65/−20
  (production, 165 net). The record's split reproduces.
- The delivery at round 1 was **1244** insertions (8 files, `90603a6..b6693e3`). With this round's 342
  the delivery's code, tests and card are **1586** insertions against the card's 420 forecast: **3.8×**,
  or **2.7×** the ×1.4 gate of 590. 815 test lines become 992; 357 production lines become 522.
- Read at HEAD, the delivery's own two paths are **1594** insertions (`crates/` 1483 + the card 111,
  `90603a6..HEAD`). The two figures count different things and neither is wrong: 1586 is the round
  totals added together, 1594 is what the tree holds after the fix replaced some round-1 lines. The
  branch against the card base (`git diff --shortstat 90603a6..HEAD`) is **10 files, 2136 insertions,
  60 deletions** — those 1594, the architect's 247 of design (14 replaced), this round's 43 of record
  and round 1's 295 of report.
- No line was added for a forecast's sake: the four tests are the four properties, and `effect.rs`'s
  +100/−15 is `grown_rect`'s loop and `reserve_scratch` replacing `vec![0; len]`.

### Observations (not findings)

- **`O17`'s check runs only when the scratch grows.** `reserve_scratch` returns early when
  `scratch.len() >= pixels` (`effect.rs:147-149`), so a scratch kept from an earlier frame is not
  re-compared against a bigger surface's `held`. The arithmetic bounds it in practice — the scratch is
  at most that frame's grown buffer, and a frame whose buffer is no larger than the retained scratch
  needs a surface at least that large — but a preview frame that grows a large scratch followed by an
  export frame with a bigger surface can hold `surface₂ + scratch₁ > SURFACE_BUDGET` with no refusal.
  `memory_usage` reports it and `trim` is the lever, which is the same shape the layer pool has had
  since `CB2`. Worth knowing before the governor work; not a defect of this fix.
- **The cache is in `host` but not in the refusal.** `memory_usage` counts `cache_bytes` (the caller's,
  observed), while `held` is the surface plus the layers, as `O17` words it. The two figures answer
  different questions and the test asserts both, so this is consistent, not an omission.
- **`D4`'s corrected sentence and `O17` still differ in one word.** `D4` says "a layer is never larger
  than a task"; `O17` says `Layers::push` gets the *grown* buffer, so a layer is as large as the drawn
  task. Both are true of different things (the layer's region versus its buffer) and `O17` spells it
  out, so no delivery can read it two ways — but a reader of `D4` alone can.
- **The other five `a517c3e` hunks are unchanged behaviour.** `run` still commits only after
  `cancel.check()`, a cancelled effect still leaves the layer as it was (`arq_112_…` green), and the
  scratch's `&mut` is threaded through `render_tile` without a second borrow.
- **`mutation: off`, no T4 report.** Its absence is not a gap, as round 1 of `CB4` and round 1 of `CB5`
  recorded.

next: tdd-apply (CB5b, the next delivery) — `W7` is the architect's line and belongs before a delivery is planned against `D7`; `W8`–`W10` are follow-ups for a later `gafira-render-cpu` round, and `W4`/`SP7` stays the specifier's
