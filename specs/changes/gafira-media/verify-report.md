# Verify report: gafira-media, ME1, round 1

Commit `c299ae7` on `feat/layer-3-media`, base `feat/layer-3-scaffold` at `267b372`. Depth V2, mode full, level L2, tier T4, `mutation: archive` (no `mutants` report, not a finding). Date 2026-11-04.

**Verdict: FAIL.** CRITICAL 1, WARNING 6, SUGGESTION 3. Classes: implementation 6, claim 1, ambiguity 0, architecture 0.

The gates are all green. The delivery fails on one code defect that an independent probe found and that no test of the delivery reaches: `Medium::seek` ignores the cancel token when it seeks by keyframe, so a cancelled seek returns success.

Gates, probes and hand mutants ran on a scratch export outside the repo (`git archive` of `c299ae7` into `/tmp/gafira-verify-export`, own `CARGO_TARGET_DIR=/tmp/gafira-verify-export-target`, `CARGO_INCREMENTAL=0`). The export tree was checksum-identical to `c299ae7` before the run. The shared worktree `/tmp/gafira-wt/me` was **not** used for the gates: while this round ran, uncommitted ME2 files appeared in it (`crates/gafira-media/src/cache.rs`, `crates/gafira-media/tests/cache.rs` and a modified `crates/gafira-media/src/lib.rs`, written between 11:01 and 12:19), so a gate run there would have measured a tree that is not `c299ae7`.

## Gates

Each command ran alone and its own exit code was read, in the clean export of `c299ae7`.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `cargo deny check` (0.20.2, the version CI builds) | 0 (`advisories ok, bans ok, licenses ok, sources ok`) |
| CLI has no UI dependencies (ARQ-07) | 0 |
| Domain crates forbid unsafe code | 0 |
| Domain crates respect the layer table (V5) | 0 |
| Domain clippy.toml copies are in sync (V12) | 0 |
| Layers 0-2 know no language (ARQ-107) | 0 |
| EditToken only in the edit crates (V13) | 0 |
| Every core command has a property test (V14) | 0 |
| Testkit ports feature compiles (ADR 0013) | 0 |

The CI policy steps were extracted from the `run:` blocks of `.github/workflows/ci.yml` with a YAML parse and run with `bash -euo pipefail`, in the export. The `cargo-deny` step is a `uses:` action in CI, so `cargo deny check` ran locally instead.

Corroboration in the shared worktree (same commit, but the tree was being edited concurrently, so it is not the source of record): `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0, `cargo deny check` 0, and the eight policy steps 0.

## Acceptance criteria

| AC | Type | Status | Proof |
|---|---|---|---|
| AC1 | integration | covered | `arq_144_the_seek_lands_on_the_previous_keyframe_and_the_exact_seek_on_the_frame`; probes P1, P2, P18 (P18 adds the frame *at or after* the requested time, which no test of the delivery covers) |
| AC2 | unit | covered | `arq_144_a_time_before_the_first_keyframe_returns_the_first_keyframe`; probe P3 |
| AC3 | unit | covered | `arq_144_an_empty_index_reports_the_absence_of_a_keyframe`; probe P4 |
| AC4 | integration | covered | `arq_144_reopening_the_medium_rebuilds_the_index`; probe P5 (the probe also checks that the container is read twice, so the index is really rebuilt) |
| AC5 | command | covered | `arq_144_opening_a_medium_writes_no_file`; probe P6 (the file system of the probe **accepts** a write, so a write shows in the log instead of failing the call) |
| AC6 | command | covered | Probe P15 walks the whole crate tree (`src/` and `tests/`, not one file) and finds none of the six forbidden paths; the clippy gate with the canonical `clippy.toml` copy bans the same paths; probe P7 shows the elapsed time comes from the injected `Clock` and from nowhere else |
| AC7 | unit | **failing** | The scan half holds (`arq_139_the_scan_follows_the_progress_contract`, probes P8, P9, P16). The seek half does not: the keyframe path never checks the token (**C1**) and the step of the fraction breaks the 0,02 bound when the medium does not match the plan (**W1**) |
| AC8 | property | covered | `arq_140_the_index_does_not_depend_on_the_pool` (pools of 1, 2 and 4); probes P11 (pools of 0, 1, 2 and 4) and P14 |
| AC9 | command | covered | The forbid step, the V5 step and the testkit `ports` step all exit 0; `gafira-media/src/lib.rs:10` carries `#![forbid(unsafe_code)]` |

Eight of the nine criteria are covered; AC7 is covered for the scan and refuted for the seek.

## Product IDs (step 3c)

`git grep <ID>` over `crates/*/tests/` and `crates/*/src/` in the export of `c299ae7`.

| ID | Proof | Named by |
|---|---|---|
| ARQ-138 | The lint of CI with the canonical `clippy.toml` copy gives 0 warnings in the crate (clippy gate, exit 0) and the time comes from the injected clock (probe P7: a clock that advances 250 ms per reading gives exactly 250 ms of elapsed time, a wall clock could not). | Nothing that proves the criterion: `git grep ARQ-138` over the tests reaches only the module header of `crates/gafira-media/tests/support/mod.rs`. The behaviour is proved by `arq_144_the_scan_reports_its_cost_against_the_budget` (two clocks, two results) under another ID. **W4** |
| ARQ-139 | `arq_139_the_scan_follows_the_progress_contract` (the checker of the testkit: non-decreasing, in `[0, 1]`, 1,0 only at the end, cancellation at report `k`); probes P8 (the fraction follows the units done over the units planned), P9 (a cancelled scan returns the cancelled error), P16 (the seek reports a non-decreasing fraction), P17 (**W1**) | The test itself |
| ARQ-140 | `arq_140_the_index_does_not_depend_on_the_pool`; probes P11 (pools of 0, 1, 2 and 4 give the same bytes) and P14 (two runs give the same bytes) | The test itself |
| ARQ-144 | Seven tests of `crates/gafira-media/tests/keyframe.rs`; probes P1 to P6, P12 (the budget of 6 s, 30 s, 60 s and 120 s of material), P13 (**W2**) | The tests themselves |
| FV-11 | **Left out of ME1 on purpose.** FV-11 (05 §A6) is the real-time preview and the scrubbing; its acceptance (a) `bench_playback_3tracks` and `bench_scrub_1080`, (b) ten minutes of playback without a loss of fluidity or of memory, (c) the audio is never cut, (d) the 39 ms / 40 ms drop tolerance, all need the frame cache (ME2), the playback clock and the A/V sync (ME4) or the exporter (ME5). What ME1 does deliver of FV-11 is its "indexado de keyframes al importar", and it delivers it through ARQ-144. | Nothing, on purpose |
| CAL-14 (partial) | **Left out of ME1 on purpose.** CAL-14 (01 §CAL-14) is "scrubbing 1080p: ≥ 24 updates/s, cache search < 50 ms, without cache < 250 ms (FIJADA)", and its mechanism in the row of 01 is `bench_scrub_1080`, which needs the frame cache of ME2. ME1 contributes the "without cache" half only as the keyframe seek it implements. | Nothing, on purpose |

ARQ-02 (named by AC6, not by the card) is covered by the clippy gate, the canonical `clippy.toml` copy and probes P7 and P15.

## Independent probes (step 1c)

Derived from the acceptance criteria, written before reading the tests of the delivery and run on the export of `c299ae7`. They do not reuse the doubles of the delivery: the container, the demuxer, the decoder, the file system, the pool and the clock of the probes are local to `crates/gafira-media/tests/verify_probes.rs` of the export, so what they observe is the crate and not a double the same delivery wrote.

| Probe | Criterion | Result |
|---|---|---|
| P1 | AC1: the keyframe seek lands on the 2 s keyframe and the exact seek on the 3,5 s frame | pass |
| P2 | AC1: the search is inclusive at both ends and saturates after the last keyframe | pass |
| P3 | AC2: a time before the first keyframe lands on that keyframe | pass |
| P4 | AC3: an empty index reports the absence of a keyframe, no panic | pass |
| P5 | AC4: reopening rebuilds the index and the container is read twice | pass |
| P6 | AC5: opening a medium writes no file (the file system of the probe accepts the write) | pass |
| P7 | ARQ-138: the elapsed time comes from the injected clock (250 ms of step, 250 ms of elapsed) | pass |
| P8 | AC7: the fraction follows the units done over the units planned and never falls | pass |
| P9 | AC7: a cancelled scan returns the cancelled error and no index | pass |
| P10 | AC7: a cancelled seek returns the cancelled error, in both modes | **FAIL: C1** |
| P11 | AC8: the index does not depend on the pool (0, 1, 2 and 4 workers) | pass |
| P12 | ARQ-144: the budget is one second per minute (6 s, 30 s, 60 s and 120 s) | pass |
| P13 | ARQ-144: the keyframe times of the index are in ascending order | **FAIL: W2** |
| P14 | ARQ-140: two runs give the same bytes | pass |
| P15 | AC6: the crate tree holds none of the six forbidden paths | pass |
| P16 | AC7: the seek reports a non-decreasing fraction that ends at 1,0 | pass |
| P17 | AC7: the step of the fraction stays under 0,02 between two reports | **FAIL: W1** |
| P18 | AC1: the exact seek lands on the first frame at or after the requested time | pass |

Fifteen probes pass, three fail. The failures are the findings C1, W1 and W2 below.

## Hand mutants (step 4b)

Applied one at a time to the export, the tests run, the file restored. The implementer recorded four mutants in `apply-progress.md`; the first one was spot-checked and four independent ones were added.

| Id | Mutant | Killed by | Result |
|---|---|---|---|
| SC-1 | Spot-check of the recorded "boundaries of the lookup": `*t <= time` becomes `*t < time` | `arq_144_a_seek_to_the_time_of_a_keyframe_lands_on_it` | killed, and the killing test is the one the delivery recorded |
| IM-1 | The index keeps every packet, not only the keyframes | `arq_140_the_index_does_not_depend_on_the_pool` | killed |
| IM-2 | The keyframe times come out in reverse order (a rule cargo-mutants cannot express) | `arq_144_the_seek_lands_on_the_previous_keyframe_and_the_exact_seek_on_the_frame`, `arq_144_a_seek_to_the_time_of_a_keyframe_lands_on_it`, `arq_144_a_time_before_the_first_keyframe_returns_the_first_keyframe` | killed |
| IM-3 | The exact seek does not decode forward and returns the keyframe | `arq_144_the_seek_lands_on_the_previous_keyframe_and_the_exact_seek_on_the_frame` | killed |
| IM-4 | The budget is one second per second of material, not per minute | `arq_144_the_scan_reports_its_cost_against_the_budget` | killed |

Five mutants run, five killed. No survivor.

## Artifact integrity (step 3c)

| Check | Result |
|---|---|
| `fb1a621` `docs(gafira-media): slice card` touches only `apply-progress.md` | pass (`1 file changed, 30 insertions`) |
| It precedes the first implementation commit (`ff58ca0`) | pass |
| No implementer commit changed the Slice card | pass: `ff58ca0` and `07ae973` touch no file of `specs/`; `c299ae7` (`record the ME1 progress`) appends the section `## ME1 (tdd-apply)` below the card and changes no line of the card |
| The amend `a57f8ea` (`slice card amend`) touches only that file and states why | pass: `1 file changed, 3 insertions(+), 1 deletion(-)`, the body says the card now covers the whole media stream and that the criteria, the Flow line and the owner decisions are unchanged |
| The amend `98e6d57` (`slice card amend`), committed while this round ran, touches only that file and states why | pass: `1 file changed, 18 insertions(+)`, the body says it adds the ME2 section and that the Flow line, the ME1 criteria and the owner decisions are unchanged |
| The code tree is the same at `c299ae7` and at the head of the branch | pass: `git diff --stat c299ae7 HEAD -- crates/ Cargo.lock` is empty |

## The two deviations of the delivery, judged

### 1. The testkit manifest, a single-writer file of `SC1`

`ff58ca0` edited `crates/gafira-testkit/Cargo.toml`, which `specs/changes/layer-3/design.md:139` reserves to `SC1` ("Single writer. The `ports` feature of ADR 0013, with `gafira-ports` and `gafira-raster` as optional dependencies"). The change adds `gafira-time = { path = "../gafira-time", optional = true }` and `dep:gafira-time` to the `ports` feature. The reason is sound: `ports::media::Demuxer::seek` takes `gafira_time::Flicks`, so a double of the testkit cannot implement `Demuxer` without the crate of time.

Checked by hand, on the export of `c299ae7`:

| Check | Result |
|---|---|
| The feature is off by default | pass: `ports = ["dep:gafira-ports", "dep:gafira-raster", "dep:gafira-time"]`, none of them a default feature |
| The default tree of the testkit holds neither `gafira-ports` nor `gafira-raster` nor `gafira-time` | pass: `cargo tree -p gafira-testkit -e normal,build --target all --prefix none -f '{p}'` lists only `gafira-testkit`, `gafira-doc`, `gafira-edit`, `gafira-geom` and `gafira-color` |
| The V5 row of the testkit still passes | pass: the V5 step exits 0 (the row allows `gafira-geom gafira-color gafira-doc gafira-edit`, and the optional dependencies are not in the default tree) |
| The ARQ-107 step is still green | pass: exit 0 |
| `cargo deny check` | pass: exit 0 |

The default graph did **not** change, so this is not the CRITICAL class `implementation` the coordinator asked about. It stays a **WARNING** (W3): the rule of the design was broken, in a file that two parallel streams share, and the delivery disclosed it in `apply-progress.md`. The graph of the crate is unchanged, so the fix is not urgent; the clean repair is to own that one line in a single-writer commit.

### 2. The budget of the index is reported, not enforced

**Judgment: ARQ-144 does not require the crate to enforce the budget, so this is not a CRITICAL class `ambiguity`.** The reading that supports it:

- ARQ-144 states a bound on the import ("con una cota de ≤ 1 s por minuto de material 1080p, FIJADA, §6.5"), not an error condition, and its acceptance column lists four proofs, none of which is the budget.
- §6.7, the map of "figures → technique → test" of 02, has no row for this figure: it names `bench_scrub_1080` and `bench_playback_3tracks` for FV-11 and CAL-14, and nothing for the keyframe scan. The spec states no mechanism that could enforce it.
- `MediaError` has no variant for "too slow", and adding one is a change to `gafira-ports`, which the card lists as a non-goal of the delivery.
- ARQ-139 says the estimate of time is computed by `gafira-engine` with its `Clock`, not by this crate; the crate measures with the injected clock and hands the number to the caller, which is the layering the row asks for.

What the delivery does is enough for that reading: `IndexReport` carries `elapsed` (from `ports::Clock`), `budget` and `within_budget()`, and the test `arq_144_the_scan_reports_its_cost_against_the_budget` proves that two clocks give two verdicts (probe P7 proves the same at the level of the elapsed time).

What is missing is the **measurement** of the bound, and that is a finding of its own: **W6** (class `claim`).

## Size

Measured with the measure of `AGENTS.md` (`git diff --shortstat`), from the commit of the Slice card `fb1a621` to the verified commit `c299ae7`:

| Scope | Lines |
|---|---|
| The whole delivery | 16 files changed, 1394 insertions(+), 43 deletions(-) = **1437** changed lines |
| The code only (`crates/` and `Cargo.lock`) | 15 files, 1359 insertions(+) + 42 deletions(-) = **1401** |
| `crates/gafira-media/src/` | 426 |
| `crates/gafira-media/tests/` | 533 |
| `crates/gafira-testkit/src/ports/` (the five doubles) | 393 insertions + 41 deletions = 434 |
| The two manifests | 5 insertions + 1 deletion = 6 |
| `Cargo.lock` | 2 |
| The notes of the delivery (`apply-progress.md`) | 35 + 1 = 36 (not code) |

Of the 1352 Rust lines added, 970 are authored (neither blank nor a comment line). The number of the delivery notes (1401 code lines) is exact.

The forecast of the design (`specs/changes/layer-3/design.md:101`) is 430 lines, which `AGENTS.md` turns into 600 with the x1,4 factor. The measured 1401 is **2,0× the ~700 ceiling** of the delivery rules and **2,3× the adjusted forecast**, outside the x1,4–x1,9 band `AGENTS.md` describes. Above the ceiling the delivery needs the explicit OK of the owner. The decision is the coordinator's; it is recorded as **W5**.

## Findings

### CRITICAL

**C1. `Medium::seek` ignores the cancel token when it seeks by keyframe, so a cancelled seek returns success.** class: implementation. Ref: `crates/gafira-media/src/medium.rs:104-111`; ARQ-139 (`specs/02-arquitectura-software.md:1215`); AC7 of the Slice card.

`Medium::seek` receives a `CancelToken` and a `Progress` in both modes, so ARQ-139 makes it a long operation: "Al ver la cancelación devuelve un error de cancelado, no entrega salida parcial y no emite más progreso", and the acceptance of the row demands it. The exact path checks the token once per packet (`medium.rs:113`). The keyframe path never checks it: the index lookup, the seek of the container, the report of 1,0 and the return of `Ok(SeekOutcome::Keyframe(..))` happen with no `cancel.check()` anywhere. The early return of `SeekOutcome::NoKeyframe` has the same hole.

Probe P10, on the export of `c299ae7`: a token cancelled before the call returns `Ok(SeekOutcome::Keyframe(Flicks(2822400000)))` instead of `Err(MediaError::Cancelled(_))`. The exact mode of the same probe returns the cancelled error, so the defect is specific to the keyframe path.

No test of the delivery reaches it: `crates/gafira-media/tests/contract.rs` claims to check "the cancellation and the progress of ARQ-139" but drives only `Medium::open` (the scan), and the cancellation part of the checker of the testkit finds no cancel point at all against the seek, because the seek emits a single report (see S1). The repo's own checker has the name for this shape, `ProgressViolation::OutputAfterCancel`.

Fix: one `cancel.check()?` at the top of `seek`, before the index lookup, and the row of the contract test extended to the seek.

Bounded impact, stated so the coordinator can reclassify: the keyframe seek is one seek of the container, so the user-visible cost of a cancelled keyframe seek is a position applied that the user asked not to apply, not a long operation. The criterion of ARQ-139 is nonetheless explicit, and the fix is one line.

### WARNING

**W1. The step of the progress fraction breaks the 0,02 bound of ARQ-139 when the medium does not hold exactly `duration x fps` packets.** class: implementation. Ref: `crates/gafira-media/src/index.rs:177-182` (`Work::step`), `crates/gafira-media/src/index.rs:224-237` (`planned_frames`).

The crate plans the units of the work as `duration x frame_rate` and counts the units done as the packets read. When the two disagree, the reports jump: `step` reports `done / planned` only while `done < planned`, so a stream with more packets than planned freezes its fraction below 1,0 and then jumps to 1,0 in the last report.

Probe P17, on the export of `c299ae7`: a medium of 30 packets that announces 1 s at 2 fps (2 units planned) makes the checker of the testkit that the delivery itself uses (`gafira_testkit::progress::check_progress_contract(0.02, ..)`) return `Err(StepTooLarge { at: 1 })`. The delivery's own test passes only because its reference medium holds exactly 120 packets for 6 s at 20 fps, so the plan and the count agree to the packet. The ARQ-139 acceptance ("la fracción sube ≤ 0,02 entre dos informes seguidos") is proved by a test that holds only for that one material; on a stream with a variable frame rate, or with a container whose duration is rounded, the same contract fails.

Fix: reconcile the two numbers (plan from the packets actually read, or reserve a tail of the fraction for the work that is left), and extend the contract test to a medium whose count does not match its plan.

**W2. The index does not sort its keyframe times, although its own documentation says they are kept in ascending order and that the search is a binary one.** class: implementation. Ref: `crates/gafira-media/src/index.rs:44-47` (the struct), `crates/gafira-media/src/index.rs:195-220` (`keyframe_times`); ARQ-144.

`KeyframeIndex` documents "The keyframe times are kept in ascending order, so a search is a binary one", and `keyframe_at_or_before` relies on that order with `partition_point`. The builder never sorts: it keeps the times in the order the container gave the packets. The port carries `pts` and `dts` separately, so nothing in `ports::media::Demuxer` promises the presentation order.

Probe P13, on the export of `c299ae7`: the same three keyframes (0 s, 2 s, 4 s) handed out in another order give the index `[2 s, 0 s, 4 s]` and the search for 3,5 s lands on the wrong keyframe. The spec does not say the packets arrive in presentation order, so the defect is in the code relative to its own documented invariant.

Fix: sort the times in `KeyframeIndex::build` (and drop the duplicates), or weaken the documentation and the search to tolerate any order.

**W3. The manifest of the testkit, a single-writer file of `SC1`, was edited by the implementer.** class: implementation. Ref: `crates/gafira-testkit/Cargo.toml` (commit `ff58ca0`); `specs/changes/layer-3/design.md:139`.

The line added is `gafira-time = { path = "../gafira-time", optional = true }` plus `dep:gafira-time` in the `ports` feature. The default graph does not change (the feature is off, and the default tree of the testkit carries neither `gafira-ports` nor `gafira-raster` nor `gafira-time`), so the V5 row, the ARQ-107 step and `cargo deny` all stay green; the rule exists because the media and the render streams run in parallel and the file is shared. The delivery disclosed the deviation in `apply-progress.md`.

Fix: own that one line in a single-writer commit, and let the next delivery of the stream start from a manifest that already carries it.

**W4. ARQ-138, a Product ID of the card, has no test that names it.** class: implementation. Ref: `crates/gafira-media/tests/` (`git grep ARQ-138` reaches only the module header of `tests/support/mod.rs`); `specs/02-arquitectura-software.md:1214`.

Read literally, step 3c makes a missing name CRITICAL. It is read as a WARNING here, the same judgement the layer-3-scaffold round recorded for ARQ-01, ARQ-03 and ARQ-06 (its W1): the row is proved in substance (the lint of CI with the canonical `clippy.toml` copy gives 0 warnings in the crate, and the time comes from the injected clock, proved by `arq_144_the_scan_reports_its_cost_against_the_budget` and by probe P7), but the ID does not trace to the proof.

Fix: name ARQ-138 in the test that proves the clock half (a comment line is enough), or add a test `arq_138_the_time_comes_from_the_injected_clock`.

**W5. The delivery is twice the ceiling of the delivery rules.** class: implementation. Ref: the size table above; `specs/changes/layer-3/design.md:101` (forecast 430, x1,4 = 600); `AGENTS.md`, "Delivery rules".

1401 code lines against a ceiling of ~700 and an adjusted forecast of 600. The split: 426 of crate, 533 of tests, 434 of the doubles of the testkit, 8 of manifests and lockfile. Above the ceiling the delivery needs the explicit OK of the owner; the decision is the coordinator's.

**W6. XP6, the claim that the design marks as blocking ME1, was never measured, so the figure FIJADA of ARQ-144 is unproven.** class: claim. Ref: `specs/changes/layer-3/design.md:320` (XP6, "Blocks: ME1"); `specs/02-arquitectura-software.md:1248` (ARQ-144) and `:1418` (§6.5).

The design opens that table with "Provisional claims. Nothing in the table is built on until it is measured", and XP6 is exactly the claim of the ≤ 1 s per minute of 1080p material, with the check "a timing of a full packet scan of a 1080p file, with the allocation counted". No spike for XP6 exists: the spike of the scaffold (`specs/changes/layer-3-scaffold/spike.md`) covers XP8, XP9 and XP10, and the media folder has none. The crate ships no bench for the scan either, although the design counts the benches of `gafira-media` among the things the crate contributes (P20).

The claim is measurable in this cut with the ports and a double (XP10 could not be, because it needs a real codec; XP6 needs only the cost of `read_packet`, which the double reproduces), so the delivery was built on a claim that could have been settled.

Route: `explorer`, for XP6. It does not block the fix of C1.

### SUGGESTION

**S1.** The exact seek emits no progress but the report of the end: the implementer recorded the choice ("El seek reporta el end de la decodificación solamente"). With one single report, the cancellation part of the checker of the testkit finds no cancel point at all (`cancel_points(1)` is empty), so the ARQ-139 acceptance ("a double of `Progress` that cancels at the report `k`") passes vacuously against the seek. A work model for the decode (the frames done over the frames planned) would make the seek cancellable through the progress, which is how a scrub is cancelled in practice.

**S2.** The budget does not scale with the resolution that the figure names. ARQ-144 says "≤ 1 s por minuto de material 1080p", and `budget_for` uses the duration alone, so a minute of 4K material is compared against the rate of 1080p and `within_budget()` can be false while the rate of the figure is met. Either scale the budget by the pixel count, or state in the spec that the figure is the reference of 1080p and that the report is informational.

**S3.** The card names FV-11 and CAL-14 (partial) as Product IDs of ME1, but no criterion of the card covers them: ME1 proves them only through ARQ-144, and the mechanism of CAL-14 (`bench_scrub_1080`) needs the frame cache of ME2. Saying in the card which part of each ID ME1 covers would keep the traceability of the later deliveries unambiguous.

## Notes

- The tree of the shared worktree was not touched: only this report and `verification.json` are written, and they live in `specs/changes/gafira-media/`.
- The worktree carried the ME2 files while this round ran, and ME2 landed on the branch as `6d6eab9` (`feat(gafira-media): add the frame cache`) at 12:32, in the middle of the round. That commit adds `crates/gafira-media/src/cache.rs` and `crates/gafira-media/tests/cache.rs` and changes `crates/gafira-media/src/lib.rs`; it touches **no file of ME1** (`index.rs`, `medium.rs`, `ports.rs`, `tests/keyframe.rs`, `tests/contract.rs` are byte-identical between `c299ae7` and `6d6eab9`), so C1 is still open at the head of the branch. The gates of record are the ones of the export of `c299ae7`, not the ones of the worktree.
- The scratch export and its target dir are freed at the end of the round (`cargo clean --target-dir`).

next: explorer (W6, XP6), then tdd-apply (C1, W1, W2, W3, W4, W5)
