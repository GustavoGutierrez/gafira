# Verify report: gafira-segment, SG3, round 1

Commit 4653a6f, branch feat/gafira-segment. Mode full, tier T4, depth V2, `mutation: archive` (no `cargo-mutants` report before close, not a finding).

**Verdict: FAIL.** Critical 6, warning 6, suggestion 5. Classes: implementation 8, ambiguity 3, claim 1.

The blocking gate passes and the gates are clean. The delivered segmenter is worse than D-45 records: **5 of 12** on the scenes that gave 9 of 12. The new warning says 12 of 12 and nobody sees it.

## Gates (each run alone, worktree, no `CARGO_TARGET_DIR`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1822 passed, 0 failed) |
| `cargo deny --offline check` | 0 |
| CI policy steps (ARQ-07, unsafe, V5, V12, ARQ-107, V13, V14) | 0 each |

## Quality gate: the owner's questions

### Measured counts, IoU >= 0.90, 4000x3000, zone Foreground

| Set | Delivered (reg 400) |
|---|---|
| Implementer's 16 gate scenes | 16/16 (min 0.9896) |
| Implementer's 12 fresh scenes | 12/12 (min 0.9806) |
| XP5 prototype gate, 16 scenes (source of the D-45 work) | 16/16 (min 0.9737) |
| **XP5 prototype fresh, 12 scenes (the set behind "9 of 12")** | **5/12** |
| Verifier's own set, 12 recipes hard edge plus 12 soft edge with noise | 18/24 |

XP5 scenes were ported unchanged from the prototype source in the scratchpad. On those 12 failures, 5 gradient scenes (ΔE 7.6 to 15.3) collapse to a 100 % foreground and 2 scenes with strong pixel texture (F7, F12) collapse to 0 %. All 7 report `reliable` false, so the product at least says so. On my set, the 6 failures are the 3 gradient recipes at ΔE 10 to 14, each with and without soft edges, also at 100 % foreground and unreliable. Two of my low-contrast recipes measured ΔE 5.90 and 5.92, just under the floor, by a calibration miss on my side. Both passed, so no count depends on them.

### Are the 12 fresh scenes comparable? No, they are easier.

They meet ARQ-132 to the letter: same generator, other seed, flat, gradient and textured, centred and offset, concave and holed, low contrast, area and ΔE in range. They are as hard as the gate, because they are the gate's recipes with other colours. That is the problem. The composition is met, the range is not exercised:

| | Implementer gate | Implementer fresh | XP5 gate | XP5 fresh | ARQ-132 allows |
|---|---|---|---|---|---|
| Area | 6.5 to 19.1 % | 6.6 to 18.7 % | 2.4 to 45 % | 2.9 to 26.8 % | 2 to 50 % |
| Gradient ΔE76 | 48 to 80 | 52 to 71 | 25 | 7.6 to 15.3 | >= 6 |
| Textured ΔE76 | 39 to 50 | 30 to 53 | 25 | 7.3 to 12.3 | > 6 |
| Lowest low contrast | 7.85 | 6.35 | 6.27 | 6.03 | 6 |

`pick_subject` forces gradient scenes to ΔE 35 to 80, and 25 from both gradient ends; textured to 22 to 55 (`scenes.rs:490-495`). Texture is a 12 px blocky cell, not per-pixel. A second proof that fresh is not independent of the gate: both move together with the same knob (reg 25: gate 13/16, fresh 9/12; reg 400: 16/16, 12/12), while XP5 fresh goes the other way (7/12 at reg 25, 5/12 at 400, 3/12 at 1000). The 12 of 12 is a meaningless number. The implementer said so in its notes, which is to its credit, but the warning itself carries no such caution. See SG3-C2.

### Regularisation 400: permitted, and the wrong fix.

Permitted by the text: "se pueden cambiar sin tocar esta spec mientras se cumplan ARQ-132 y CAL-06", and both hold. It is still wrong, because it compensates for two defects:

1. `cut.rs:181`: the edge capacity is `gamma * exp(...)`. The prototype has `gamma * shared_border_length * exp(...)`. The size-weighted unary terms then swamp the smoothness, which is why the implementer found gamma "almost inert". The note left it as is; that was the root cause.
2. Band nodes stay free in the graph (`cut.rs`, `Pin::Free`). The prototype pins them as background. With free band nodes a gradient can lose its whole background side, `unary_costs` returns `None`, and the result is the 100 % foreground above.

Candidate fix, tried on the scratch copy with the documented reg 25 (gamma x30 as a stand-in for the border length, and all band nodes pinned):

| Set | Delivered, reg 400 | Reg 25, gamma 15000, band pinned |
|---|---|---|
| Gate, fresh, XP5 gate | 16, 12, 16 | 16, 12, 16 |
| XP5 fresh | 5/12 | **9/12**, the D-45 figure exactly |
| Verifier's set | 18/24 | 22/24 |

Each change alone is not enough: gamma 15000 with free band and reg 25 gives 14/16 on the gate; band pinned with reg 400 gives 7/12 on XP5 fresh. Limits of this candidate: the x30 is a uniform stand-in, not the real border length; pinning every band node breaks one test (`arq_130_a_seed_steers_the_cut_on_elongated_images_in_both_orientations`, on thin images most nodes touch the border; the prototype pins only nodes with at least half their pixels in the band); and I did not measure a subject touching the border, which a hard band can harm. It has also not been run with the other parameter sets it would need. It shows the cause, it is not a finished patch.

**D-45's note.** As history it stays true. As a statement about this code it no longer does: the "known result 9 of 12" and "these values were tuned" in 02 (§4.17) describe gamma 500 and reg 25, which the code no longer has as an effective pair. With the fix in SG3-C1 the sentence becomes true again without touching D-45, which is one more reason to prefer it. If the owner keeps reg 400, the 02 text needs a correction (SG3-W1).

### Are the 16 gate scenes weakened?

They satisfy ARQ-132 and the commit history shows no weakening after tuning: `scenes.rs` was committed once (5cbd7a5), before the tuning. But they are softer than the gate D-45 was reached on (table above), and they ask less than ARQ-132's range allows. The delivered code passes the harder XP5 gate 16/16, so the softness hides no gate failure today. It does hide the regression, because no scene in the set reaches the region where the code breaks. Nothing protects the composition either: SG3-C3.

### Did tuning use only gate scenes?

Consistent with that, not proven by commit order. `REGULARISATION` changed once, in 1642222; no parameter of `gmm.rs`, `cut.rs`, `slic.rs` or `refine.rs` changes after it (the chunk sizes came earlier, a7f4d77). `sweep.sh` filters `--test gate arq_132_gate` and greps "gate scene", which excludes the fresh test. However 1642222 adds the fresh test in the same commit as the parameter, and the code commits were all made in nine minutes (10:19 to 10:28) after the tuning (`*.good` files 09:58 to 10:01), so commit order cannot show when the fresh set was first read. The claim "run once" rests on the implementer's word. I found nothing against it.

## Rulings

- **`reliable` on the returned mask.** Conforms: ARQ-131 says "los píxeles de `mask`". Not a defect. The effect is real: on my 2.19 % subject, Foreground and Element are reliable and Background is not (mask 97.81 %), and FD-32 then dims Fondo for an image where Primer plano works. That is a spec question: SG3-W2. The rule has no test pinning it: SG3-C5.
- **Dev-profile `opt-level` in root `Cargo.toml`.** Accepted. It is outside the Slice card's files, but additive, in its own commit (1444da9), recorded, and follows the existing `[profile.dev.package."*"]`. Overflow checks and debug assertions stay. The mutants profile inherits it, so T4 mutant rebuilds get a little slower. Watch for a merge conflict with parallel streams. SUGGESTION S1.
- **Guided filter overhead, 13 % to 40 %.** Accepted. XP11's 250 was a measured choice, not a rule, and smaller bands only add margin rows. Paired runs on the loaded machine: the wall time difference is lost in noise (1.2 to 2.0 s either way), and the longest gap falls from 46 to 95 ms at 250 rows to 16 to 31 ms at 80. It helps ARQ-43 more than it costs CAL-06.
- **CAL-06, CAL-26, ARQ-43 not on D-03 hardware.** The figures are valid as evidence and do not discharge the criteria. See below and SG3-W4.

## CAL-06, CAL-26, ARQ-43: my own measurement

Release, i7-12700H, machine under load (average 20 to 40 from parallel streams, so worse than idle). Worst inputs of mine: per-pixel noise, XP5 F7, soft textured V08, textured scene 12.

| Check | Result | Budget | Margin |
|---|---|---|---|
| CAL-06, whole run, 1 thread, worst of 6 | 1055 ms | < 5 s | 4.7x |
| CAL-06, 4 threads | 463 ms | < 5 s | 10.8x |
| ARQ-43, longest gap between reports, 1 thread | 43.2 ms | <= 100 ms | 2.3x |
| ARQ-43, 2 to 6 threads | 10 to 23 ms | <= 100 ms | > 4x |
| Real cancel latency, 24 cancels on noise | 10.8 ms (1 thread), 25.5 ms (4) | <= 100 ms | 4x |
| Heap peak, 4000x3000 (counting allocator) | 34.7, 72.9, 130.3, 302.5 MB for 1, 4, 8, 20 threads | CAL-26 1.5 GB | |

These agree with the implementer's figures in kind. Its 293 MB at 20 threads is 302.5 MB of heap in mine, still under its 316.7 MB estimate. Mask and `reliable` match across 1, 2 and 4 threads on 23 of my scenes (ARQ-111). The Foreground and Background complement is exact.

Ruling. CAL-06 and ARQ-43 name mid-range hardware (D-03), and FD-38 (b) says so too. This machine is not that, so the note "CAL-06, CAL-26 and ARQ-43 hold... nothing goes to the owner" says more than was measured. A 4.7x margin makes a CAL-06 failure on D-03 unlikely. ARQ-43 has 2.3x on one thread under load, likely enough but with the least room. CAL-26 is a whole-process budget with 20 pages, so this delivery proves only the segmenter's share, which is small. No D-03 machine was available: listed as a check that could not run. The owner should decide whether this margin is accepted until a D-03 run exists.

## Hand mutants (scratch export)

| Id | Mutant | Result |
|---|---|---|
| S1 (spot-check of `tdd-apply`'s list) | filter bands back to 250 rows | killed, `arq_112_progress_and_cancellation_on_the_reference_input` |
| H1 | no gradient background in the set | **survived** |
| H2 | no concave shape, no hole, no offset | **survived** |
| H3 | low-contrast recipe on a gradient, not flat | killed, `arq_132_gate_scene_04` (only because the scene fails the cut; no test checks flatness) |
| H4 | `reliable` of Background judged on the subject mask | **survived** |
| H5 | area of `reliable` counted by alpha weight, not pixels | **survived** |
| H6 | no final reserve, 1.0 before the end | killed, ARQ-112 reference and stroke tests |

7 run, 3 killed, 4 survived. An earlier run gave false kills: a restored file kept its old timestamp, so cargo reused a stale build. I discarded it. This table is from the clean rerun with a passing baseline.

## Coverage

| ID | Proof | State |
|---|---|---|
| ARQ-131 rule, means, D-44, unreliable inputs, FD-32 (e) | `reliable.rs`, `tests/reliable.rs` | covered; area rule and Background weak (SG3-C4, C5) |
| ARQ-132 area, ΔE, low contrast, textured | `tests/scenes.rs` | covered |
| ARQ-132 composition (backgrounds, position, shapes) | none | **missing** (SG3-C3) |
| ARQ-132 gate, 16 scenes at IoU 0.90, also reliable | `tests/gate.rs` | covered |
| ARQ-132 IoU 0.90 and 0.8999, empty masks, different seed, `0/12` | `tests/scenes.rs`, `tests/gate.rs` | covered |
| ARQ-132 fresh set informs | `tests/gate.rs` | **failing**, nothing reaches the output (SG3-C6) |
| FD-38 (a), (b), (c) | gate, benches, ARQ-112 tests | covered, not named (SG3-W6); (b) not on D-03 |
| ARQ-112 | `tests/progress.rs` | covered |
| ARQ-111 | `tests/progress.rs`, `table.rs` | covered, pool test not named (SG3-W6) |
| ARQ-43, CAL-06 | `bench_cancel_segment`, `bench_segment_4000` | exist; not on D-03 |
| CAL-26, Q-48 | RAM test | segmenter's share only |
| SG2 W5, W6 | `hints::tests::arq_110_a_stroke_from_beyond_1e16_ends_where_it_ends`, `contract.rs:146` | closed |

IDs left out on purpose: FD-32 (a to d, f) and FD-38 (d) are UI, per 02.

Integrity (3c): `docs(gafira-segment): slice card` (feb4895) touches only `apply-progress.md`. In this delivery the only change to that file is one appended hunk, so the card is unchanged.

## CRITICAL

### SG3-C1. The cut has two defects, and regularisation 400 hides them. class: implementation
- Ref: `crates/gafira-segment/src/cut.rs:181` (edge capacity), `cut.rs` `Pin::Free` arm and band initialisation, `gmm.rs:10`.
- Evidence: 5/12 on the D-45 set against 9/12 recorded; gradient scenes at ΔE 7.6 to 15 give a 100 % foreground. Raising reg makes it worse (3/12 at 1000).
- Root cause: edge capacity without the shared border length, and free band nodes. See above.
- Fix: weigh each edge by its shared border length; pin nodes with at least half their pixels in the band, keeping seeds first; go back to reg 25 and gamma 500; retune only on the gate. Then re-measure the XP5 fresh set, the elongated-image seed test, and a subject touching the border. Target: gate 16/16 and about 9/12 on XP5 fresh.

### SG3-C2. The fresh set cannot detect what it exists to detect. class: implementation
- Ref: `crates/gafira-testkit/src/scenes.rs:490-495`, `:291-296`.
- Fix: widen the generator's ranges to ARQ-132's: gradient and textured ΔE from about 7, per-pixel texture, area 2 to 50 %. Keep gate recipes as the XP5 definitions, which the code passes, and draw the fresh scenes over the whole range. The wider fresh ranges need the specifier's confirmation that they are still "the same generator" (SG3-W1).

### SG3-C3. The composition of the gate has no test. class: implementation
- Ref: `crates/gafira-testkit/tests/scenes.rs:61`. Mutants H1 and H2 survive: all gradients, every concave shape, every hole and every offset can vanish with all tests green.
- Fix: expose background, shape class, offset and flatness on `Recipe`, and assert at least one of each kind, and that low-contrast recipes are flat on both sides.

### SG3-C4. The area of `reliable` is not pinned to pixels. class: implementation
- Ref: `crates/gafira-segment/src/reliable.rs`, `assess`. Mutant H5 survives.
- Fix: a test where half-transparent pixels keep the area inside the range by pixel count, and out of it by weight.

### SG3-C5. `reliable` on the returned mask is not pinned. class: implementation
- Ref: `crates/gafira-segment/src/lib.rs`, `segment`. Mutant H4 survives.
- Fix: a test with a subject under 5 %: Foreground reliable, Background not. If the owner changes the rule (SG3-W2), the test flips with it.

### SG3-C6. The fresh-set warning never appears. class: implementation
- Ref: `crates/gafira-segment/tests/gate.rs:84`. It uses `eprintln!`, which `cargo test` captures for a passing test. The gate log has 0 lines with "WARNING" or "fresh scene". ARQ-132 says the set informs of its count; only the implementer's `--nocapture` run ever saw it.
- Fix: write with `std::io::stderr().write_all`, which libtest does not capture, or add a CI step that runs this test with `--nocapture`. Confirm the line appears in a plain `cargo test`.

## WARNING

### SG3-W1. D-45's known result and the parameter note describe other code. class: ambiguity
- Ref: 02 ARQ-132, "Parámetros de partida", 01 P-04. After SG3-C1 the note holds. Otherwise the specifier records the measured result. ARQ-132 also fixes no range for the fresh set beyond "same generator".

### SG3-W2. Fondo goes unreliable under a 5 % subject while Primer plano is reliable. class: ambiguity
- Ref: ARQ-131, FD-32. Options: keep the literal rule, or judge Fondo on the subject mask. Recommendation: judge the subject mask, so the same image gives one answer. Owner or specifier decides.

### SG3-W3. Q-49 is still open in 02. class: ambiguity
- Ref: 02 Q-49, owner row. The code took the recommendation (4000x3000). Close it, or the test comment cites an unanswered question.

### SG3-W4. The figures are off D-03 and the note says they hold. class: claim
- Ref: `apply-progress.md`, "Q-48: ... hold on this machine; nothing goes to the owner"; `benches/segment.rs` comment "the mid-range machine of CAL-06". Fix: record them as indicative, with margins, and keep Q-48 open until a D-03 run or the owner's acceptance.

### SG3-W5. `ram_bytes` is only right for 4000x3000. class: implementation
- Ref: `crates/gafira-segment/src/lib.rs`, `ram_estimate`. Measured on one thread: 46.2 MB at 4000x4000, 96.0 MB at 8000x6000, estimate 44.3 MB; at 20 threads 625.8 against 316.7 MB. `requirements` takes no image. Its test pins the formula, not a measurement. Fix: state it as a figure for the reference size, or make the fixed part cover the square working copy, and have the caller scale.

### SG3-W6. Two criteria are proved by tests that do not name them. class: implementation
- Ref: `arq_130_the_mask_does_not_depend_on_the_thread_count` carries ARQ-130 and ARQ-10, not ARQ-111; the FD-38 (a, b, c) proofs carry ARQ IDs only. Fix: add the IDs to the comments.

## SUGGESTION

- S1. Keep the dev-profile edit; the coordinator watches for conflicts on root `Cargo.toml`.
- S2. `refine.rs:2` still says "row bands of 250", and the note that band nodes stay free "as P12 says" overstates: P12 says only "border band".
- S3. `bench_segment_4000` uses one easy scene. Add a noise input; here it costs about the same.
- S4. `reliable` cannot rescue a bad mask: reg 25 gives `reliable` true at IoU 0.2. Worth a look when retuning.
- S5. A long zigzag stroke (200 points, radius 400) takes 3.2 s on one thread under load, with gaps still at 11 ms. No budget names it.

## Checks that could not run

- CAL-06, ARQ-43 and FD-38 (b) on D-03 mid-range hardware: no such machine here.
- CAL-26 as a process with 20 pages: needs the application.

With CRITICALs present the verdict is FAIL, so these are listed and cost nothing here.

## Notes on method

- Probes ran on a scratch export with a probe example and environment overrides for reg, gamma, band and band rows; the defaults equal HEAD. The worktree stayed clean; the export's build was freed with `cargo clean` (1.6 GB).
- Three process slips, all mine or environmental, none in the results above. Free disk fell to 19.6 GB during the run, so I stopped building until it recovered. The export's `target` was removed from outside during a sweep, so four sweep rows (reg 100 and 400 at gamma 15000, reg 25 and 100 at gamma 50000, free band) never ran and were not retried; the hard-band sweep replaced them. The stale mutant run is described above.
- Timings are under heavy load from other streams, and only margins are claimed from them.

next: specifier (SG3-W1, W2, W3), then explorer (SG3-W4), then tdd-apply (SG3-C1 to C6, W5, W6)

---

# Earlier rounds (SG1 and SG2)

# Verify report: gafira-segment, SG1 + SG2, round 2 (closure)

Commit a3f97f6 (branch `feat/gafira-segment`), base develop. Depth V2, mode full, tier T4, `mutation: archive` (no mutants report, not a finding).
Scope as set by the coordinator: C1 to C7, W2, W3, W4, S1, S2 of round 1 and the criteria of a2a2924 and 314f3ba. W1 closed: the owner accepted the recorded TDD breach. Size not assessed. S3 not in scope.

**Verdict: PASS for SG1 + SG2.** 0 CRITICAL, 2 WARNING (both `implementation`, follow-up), 0 SUGGESTION. All round 1 findings are closed. SG3 remains, so the change is not done.

One call to review: W5 is a real defect that I rated WARNING, not CRITICAL, for the reason given there.

## Gates (each run alone, exit code read alone)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 (cached from the same tree) |
| `cargo test --workspace --locked` | 0 (1588 passed, 0 failed) |
| `cargo deny --locked check` | 0 |
| CI policy: ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13, V14 | 0 each |

## Artifact integrity (3c)

- The Slice card (feb4895) is unchanged at HEAD: the diff from feb4895 only adds lines after it.
- No implementer commit touches `specs/` outside `apply-progress.md`. a2a2924 and 314f3ba are spec-only commits; a3f97f6 touches only `apply-progress.md`.
- IDs named by tests: ARQ-130, ARQ-110, FD-38.
- Left out for SG3 by the card: ARQ-131, ARQ-132, FD-32 (e), FD-38 (a, b, c), ARQ-43, CAL-06, ARQ-111 and ARQ-112 figures.

## Round 1 findings

| Id | State | Evidence |
|---|---|---|
| C1 | closed | `Segmenter: Describe + Send + Sync`, `segment.classic` v1; `arq_130_segmenter_describes_itself` goes through `&dyn Segmenter` |
| C2 | closed | `SegReq.ram_bytes`, 85,590,016; `arq_130_requirements_estimate_ram_from_the_working_size`. First estimate, SG3 calibrates it (Q-48) |
| C3 | closed | Round 1 table re-run, below |
| C4 | closed | SG1-A and SG1-B killed; tables cover four zones, every element of the pixel, x and y of both lists |
| C5 | closed | SG1-C2 killed; size test loops over four zones |
| C6 | closed | Per-axis scale; round 1 differential re-run, below; SG2-D killed |
| C7 | closed | SG2-E killed, only by the new test with a larger side object |
| W1 | closed | Owner accepted the recorded breach |
| W2 | closed | SG2-F killed; test asserts mean 64 |
| W3 | closed | a2a2924 names the zones; code and tests match, criteria 7 to 9, 20 to 23 |
| W4 | closed | a2a2924 and FD-38: the guided filter is the refinement, no separate pass |
| S1 | closed | Progress note corrected |
| S2 | closed | Token and progress per band of at most 256 rows, proved by `fd_38_a_stroke_reports_progress_and_checks_the_token_per_chunk`. Each stroke is still worked per segment, not rasterised once: 4000x3000, 1000 points, radius 500 took 4.9 s against 0.6 s with no stroke (release, noisy run). For SG3's bench to weigh |

C3 probe, 64x48, zone Foreground:

| Stroke | Result |
|---|---|
| `[-1e19, 10] -> [1e19, 10]`, radius 1 | `Ok`, row 10 covered 64/64 |
| `[-1e20, 10] -> [1e20, 10]`, radius 1 | `Ok`, 64/64 |
| `[MAX, 10] -> [-MAX, 10]`, radius 1 | `Ok`, 64/64 |
| `[1e30, 0]`, radius 0 | `Ok` |
| `[3, 3]`, radius `1e30` or `MAX` | `Ok`, all >= 243 |
| Diagonals at +-MAX and +-1e19 | `Ok` (finite and `Ok` only, as instructed) |

C6 probe, background seed on a 28-pixel stripe, stripe pixels still foreground:

| Size | Wide | Tall |
|---|---|---|
| 33 x 4000 | 0 | 0 |
| 50 x 5000 | 0 | 0 |
| 201 x 6000 | 0 | 0 |

## The six original survivors

Re-applied on a scratch export. Each killed by one test. Clean run after them: exit 0, and the export source matches the repo.

| Id | Rule, wrong behaviour | Killed by |
|---|---|---|
| SG1-A | ARQ-110: hints unchecked in zone All | `arq_110_non_finite_hint_is_err_before_any_work` |
| SG1-B | ARQ-110: alpha of `f32` pixels unchecked | `arq_110_non_finite_pixel_is_err` |
| SG1-C2 | ARQ-130: pipeline panics on 1x1 and width 2048 | `arq_130_mask_has_the_input_dimensions` |
| SG2-D | Seeds not scaled to the working copy | `arq_130_a_seed_steers_the_cut_on_elongated_images_in_both_orientations` |
| SG2-E | Elemento keeps the largest region | `arq_130_element_keeps_the_central_object_even_when_the_other_is_larger` |
| SG2-F | Reduction samples the first pixel | `work::tests::fd_38_reduction_averages_source_blocks` |

## Criteria

1 to 18 are round 1's; 19 to 27 come from a2a2924.

| # | Criterion | Proof | State |
|---|---|---|---|
| 1, 2 | Reduce above 2048, never enlarge | `fd_38_working_size_reduces_only_above_2048` | covered |
| 3, 4 | GMM colour only; same colour, same cost bit for bit | same-cost test | covered |
| 5 | Mask has input size, four sizes | `arq_130_mask_has_the_input_dimensions` | covered |
| 6 | Width or height 0 is `Err` | `arq_130_zero_size_is_err` | covered |
| 7, 8 | Primer plano and Elemento: seed bounds >= 243, <= 12 | `arq_130_seed_strokes_set_the_alpha_under_them`, single-pixel test | covered (see W5) |
| 9 | Fondo exact complement, bounds under seeds | `arq_130_background_is_the_exact_complement_and_all_is_solid` | covered |
| 10 | Elemento keeps the central object, even when the other is larger | `arq_130_element_keeps_the_central_object_even_when_the_other_is_larger` | covered |
| 11 | `SegHints` zone and strokes, no default needed | `arq_130_hints_are_zone_and_strokes_only` | covered |
| 12 | Trait shape of §4.7 | `arq_130_segmenter_describes_itself` | covered |
| 13 | `SegReq` contents | RAM and working-size tests | covered |
| 14 | `ClassicSegmenter` receives the pool | constructor | covered |
| 15 | ARQ-110: non-finite input, typed `Err` naming the field | `arq_110_non_finite_*` | covered |
| 16 | ARQ-110: non-finite intermediate | `arq_110_extreme_finite_strokes_are_applied`; finite strokes stay finite in `f64` | covered |
| 17 | FD-38 reduced working copy | `fd_38_*` | covered |
| 18 | Seeds reach the working copy at the right place | elongated-image test | covered |
| 19 | `Alpha8` in any zone: `UnsupportedFormat` | `arq_130_alpha8_is_unsupported` | covered |
| 20 | Both seed kinds on a pixel: >= 243 | `arq_130_foreground_wins_an_overlap_and_neighbours_keep_their_bounds` | covered |
| 21 | Adjacent pixels keep their bounds | same | covered |
| 22 | Todo solid 255, with or without seeds | `arq_130_all_ignores_the_seeds` | covered |
| 23 | Todo with a non-finite seed: `Err` | `arq_110_non_finite_hint_is_err_before_any_work` | covered |
| 24 | Primer plano keeps both objects | Element tests | covered |
| 25 | Ramp edge: a ramp pixel with 0 < alpha < 255 | `fd_38_a_ramp_edge_gives_graded_alpha` | covered (W6) |
| 26 | FD-38 (e) | same | covered |
| 27 | Same RGB, other alpha: same mask, `Rgba8` and `RgbaF32` | `arq_130_input_alpha_does_not_enter_the_mask` | covered |

27 of 27 covered.

314f3ba (FD-32): left out with reason. Attenuation is UI, and `reliable` comes with ARQ-131 in SG3. `reliable` is `false` in every zone today, so nothing here contradicts it. SG3 should compute it for Todo too, as the new text says.

## WARNING

### W5. A stroke from beyond about 1e16 that ends inside the image lands wrong. class: implementation, follow-up
- Ref: `crates/gafira-segment/src/hints.rs:82-89`, `clip`. When the far end is not cut, it is `a + 1 * d`, not `b`.
- With `a` at 1e19, `b - a` loses `b`. Probe on `for_covered`, row stroke, radius 0, width 3000:

| Stroke | Should cover | Covers |
|---|---|---|
| `[500.5, 4.5] -> [1e19, 4.5]` | 500..2999 | 500..2999 |
| `[1e19, 4.5] -> [500.5, 4.5]` | 500..2999 | 0..2999: 500 pixels too many |
| `[1e19, 4.5] -> [1500.5, 4.5]` | 1500..2999 | 1500, then 2047..2999: a gap of 546 pixels |
| `[1e19, 4.5] -> [2500.5, 4.5]` | 2500..2999 | 2047..2999: 453 pixels too many |
| `[1e15, 4.5] -> [500.5, 4.5]` | 500..2999 | exact |
| `[1e16, 4.5] -> [500.5, 4.5]` | 500..2999 | 499..2999 |

- Why it is not CRITICAL: every intermediate is finite and the result is `Ok`, so ARQ-110 holds. Coordinates beyond about 1e16 pixels are outside any real input, and `for_covered` documents that limit. This is my judgement: the owner's exception names diagonals, and this stroke is axis-parallel. Read strictly, a pixel in the gap is under a seed and is not stamped.
- The progress note is wrong on one point: "Axis-parallel ones are exact" holds only when the far end is cut too. No test has an axis-parallel stroke ending inside the image from that far.
- Fix: return `b` itself when `leave` is `None`, as a sibling of the exact-side rule, with this table as the test. Correct the note.

### W6. The ramp test does not look at the ramp. class: implementation, follow-up
- Ref: `crates/gafira-segment/tests/contract.rs:146`. The criterion says a pixel of the ramp; the test counts graded pixels anywhere in the image.
- The code is right: my probe finds 11,358 graded pixels of 18,900 inside the ramp. A further 11,474 lie outside it, so the test could pass with none in the ramp.
- Fix: restrict the count to pixels at distance 1 to 15 from the subject.

## Notes on method

- Probes and mutants ran on a scratch export in the report folder, default target, `CARGO_INCREMENTAL=0`, release, freed with `cargo clean`. Gates ran in the worktree with no `CARGO_TARGET_DIR`. The worktree stayed clean.
- My first version of the far-end probe went through the whole pipeline and could not tell stamped pixels from the cut. I discarded it and read `for_covered` directly with a temporary unit test, removed before the mutants.
- Disk: 29 GB free at start, 26 GB before the clean. Margin is shrinking toward the 20 GB limit.

next: tdd-apply (SG3)
