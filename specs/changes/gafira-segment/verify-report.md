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
