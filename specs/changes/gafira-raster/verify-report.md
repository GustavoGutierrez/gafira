# Verify report: gafira-raster, RS3b, round 2

- Commit: `808840f` (branch `feat/gafira-raster`). Fix commits: `30769fb` (Wipe mask), `3b9e626` (tests), `808840f` (notes). Spec amendment: `d67674b` (P15, by `architect`, allowed upstream).
- Closure round, depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS3b round 1, which stays in git at `4f8969f`. RS3a round 2 stays at `9739549`, RS3a round 1 at `107f9d9`, RS2 round 2 at `1909734`, RS2 round 1 at `0b563fe`, RS1 round 2 at `e5e1d0e`. All rounds are also in `verification.json`.

**Verdict: FAIL.** 2 CRITICAL, 1 WARNING (follow-up), 2 SUGGESTION.
Per class: implementation 2 (2 CRITICAL), ambiguity 1 (1 WARNING, follow-up), architecture 0, claim 0.

C2, W1, W2, W3 and W4 are closed. C1 is closed on the four axis directions and stays open on `circular`. The new C3 is the same root cause at `s = 0`: on a `circular` Wipe, a pixel with exact `u == p` is computed with `u` one rounding away from `p`. It affects the default softness.

**Stop signal for the coordinator.** This is the second consecutive FAIL with class `implementation` in RS3b, and round 3 is the last allowed (`AGENTS.md`, delivery rules). The route below stands, but those rules ask the coordinator to stop and ask the owner.

## Gates

Each exit code was read alone from its own file, on a fresh clone of `808840f` with no `proptest-regressions` folder.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1 454 passed, 0 failed, 0 ignored; 136 in the `gafira-raster` library) | 0 |
| `cargo deny check` | 0 |
| policy: ARQ-07 CLI | 0 |
| policy: forbid unsafe | 0 |
| policy: V5 layers | 0 |
| policy: V12 clippy copies | 0 |
| policy: ARQ-107 language | 0 |
| policy: V13 EditToken | 0 |
| policy: V14 property tests | 0 |

The 15 seeds in the untracked `proptest-regressions/` also pass on the unmutated code, on a scratch export (exit 0, 136 passed). The CI test job's Windows and macOS targets were not run here. Disk: 35 GB free at the start, 30 GB at the lowest reading, 35 GB after `cargo clean` of all five build folders.

## Exact reference

A new reference, written for this round, shares no code with the crate. `p`, `s` and `u` are exact rationals. Circular `u` is held as its exact square, so `u == p` is decided in integers. The one irrational step, a square root, runs at 2 600 bits, which covers `2^-1074`. The crate ran through its public API, black to white, opaque, in both formats. Bounds are those of 02 §4.17.4: 1e-5 for `f32`, 1/255 for bytes.

**Round 1 grid, replayed.** The same 60 052 cases, with precision raised from 300 to 2 600 bits: **0 mismatches**, against 10 064 wrong pixels at `608b196`.

**Softness grid.** 130 575 cases and 59 601 750 pixels: 21 sizes from 1x1 to 1024x1, squares 2x2 to 64x64 included; 46 softness decades, from 1 down to 5e-324, with `s = 0`, the smallest normal, subnormals and powers of two; every direction; values of `p` that land on, or as close as an `f32` can to, pixels of each image.

| Direction | Pixels with exact `u == p` | Over the bound |
|---|---|---|
| `right`, `left`, `down`, `up`, `s > 0` | 682 428 | **0** |
| `right`, `left`, `down`, `up`, `s = 0` | 9 222 | **0** |
| `circular`, `s > 0` | 6 512 | **1 704** |
| `circular`, `s = 0` | 88 | **36** |

All 1 740 `f32` values over the bound are those exact-edge pixels; 1 524 bytes are over 1/255. No axis direction has an error over the bound at any softness.

| `s`, `circular`, exact-edge pixels | Worst `f32` error | Worst byte error |
|---|---|---|
| 0 | 1 | 255 |
| 5e-324 to 1e-16 (decades) | 0.81 | 135 |
| 1e-15 | 0.11 | 14.4 |
| 1e-14 | 1.1e-2 | 1.62 |
| 1e-13 | 1.1e-3 | 0.62 |
| 1e-12 | 1.1e-4 | 0.48 |
| 1e-11 | 1.1e-5 | 0.48 |
| 1e-10 and above | under the bound | under the bound |

**Root-cause control (scratch export only, no source changed).** `u` taken from one exact quotient of squares and one `sqrt`, in place of two `hypot` calls: 136 of 136 tests pass, and the same 130 575 cases have **0 over the bound**, all 6 600 circular edge pixels right. So the cause is the rounding of `u`, not the `p + gap / s` form. It also shows a fix exists at this level. Exact squares fit an `f64` for sides up to about 2^26, and a `u` that an `f32` holds has a square an `f64` holds, so the quotient and the root are then exact.

**Near hits, the residual.** A search for pixel and `p` pairs with a tiny non-zero gap, with `s` set just above that gap: see W5.

## Closure

| Finding | Status | Evidence |
|---|---|---|
| C1 | **Open**, axis closed, `circular` not | Table above. I2 and the reverted expression are killed. |
| C2 | Closed | I1 reapplied: killed by `arq_118_slide_offset_is_computed_in_f64` (`crates/gafira-raster/src/transitions/slide.rs:367`). The cases hold in the reference. |
| W1 | Closed | Confirmed by instrumentation, below. |
| W2 | Closed | `arq_111_every_configuration_is_deterministic` and two proptests (`crates/gafira-raster/src/transitions/mod.rs:1378`, `:1400`, `:1426`). W2a, W2b and W2c killed. |
| W3 | Closed | `design.md:380` (`d67674b`) now binds near-end bounds to Dissolve, Fade and Wipe with `s = 1` only, as ARQ-118 does. |
| W4 | Closed | `apply-progress.md:285` corrected. I8 reapplied at `808840f`: 136 pass, still equivalent, as the note now says. |
| S2 | Closed | `apply-progress.md:285` says "below tolerance". |
| S3 | Closed | The ends test names FV-06 (a) (`mod.rs:158`). |
| S1, S4 | Open | Not touched; see below. |

**W1, confirmed.** The recorded evidence was code analysis plus one table row, so a scratch copy was instrumented. It counts, in the ramp branch, any quotient that is non-finite or above 1 and any non-finite `p + quotient`. In the saturated branch it also performs the skipped division, as a positive control.

| Measure (119 355 runs, 73 softness values, every subnormal binade, 13 sizes, 5 directions) | Count |
|---|---|
| `Err`, or runs with non-finite output | 0 |
| Ramp quotients taken | 2 547 088 |
| Of them, with subnormal softness | 997 716 |
| Of them, non-finite or above 1 | **0** |
| Saturated pixels | 97 376 772 |
| Skipped divisions that would have been non-finite (control) | 74 805 832 |

No infinity passes through the mask. Dropping the saturation test survives all 136 tests, as the note says it would, since the infinity is then clamped. The proof is by this instrumentation, which is not in the repo: no tracked test can see an intermediate.

## Hand mutants

Closure probes only, no independent budget spent. Applied one at a time to a scratch export; `cargo test -p gafira-raster --lib`. 8 run, 6 killed, 2 equivalent.

| # | Mutant | Result |
|---|---|---|
| I1 | Slide `d` in `f32` | Killed: `arq_118_slide_offset_is_computed_in_f64` |
| I2 | Wipe: softness below 1e-9 treated as hard | Killed: `arq_118_wipe_tiny_softness_keeps_the_edge_pixel_at_p`, `arq_118_wipe_mask_equals_exact_integer_arithmetic_for_dyadic_softness` |
| C1r | The old expression restored | Killed: the same two tests |
| W2a | Entropy in the soft ramp of `circular` only | Killed: `arq_111_every_configuration_is_deterministic` and 2 more |
| W2b | Soft Wipe, `up`, softness 0.4, breaks `c <= a` | Killed: `every_configuration_keeps_premultiplied_pixels_valid` |
| W2c | `circular` at 1e-16 writes `+∞` | Killed: `every_configuration_gives_finite_output_for_finite_input` and 1 more |
| I8 | Guard: end copies removed for all six | Survived, **equivalent** (W4) |
| W1d | Saturation test dropped | Survived, **equivalent** at the output (W1) |

## Criteria

19 criteria: 17 covered, 2 failing. Round 1 marked the strict edge rule as covered; its grid had no power-of-two square above 2x2, which is why this was not seen then.

| Criterion | Status | Proof |
|---|---|---|
| Wipe mask with softness | **Failing** (C1) | Axis directions match the reference at every softness. `circular` fails it on exact-edge pixels below `s` about 1e-10 |
| Wipe `s = 0`, strict `u < p` | **Failing** (C3) | Axis directions hold. `circular` fails it on 36 of 88 exact-edge pixels |
| Slide `d` half away from 0, in `f64` | Covered | `arq_118_slide_offset_is_computed_in_f64` |
| ARQ-110 finite property, per direction | Covered | `every_configuration_gives_finite_output_for_finite_input` |
| ARQ-111 per direction | Covered | `arq_111_every_configuration_is_deterministic` |
| The other 14 | Covered | As in round 1 (`4f8969f`); the replayed grid has 0 mismatches |

**Artifacts (step 3c).** `b64090c` (slice card) touches only `apply-progress.md`, and `75e360d` is the amend. Against `fadac62`, `apply-progress.md` has 58 additions and 0 removed lines, so no implementer commit changed the card. The five implementer commits of RS3b touch only `crates/gafira-raster`. `d67674b` touches only `specs/changes/layer-2/design.md`, one line. No commit carries AI attribution.

**Product IDs (`git grep` over the tests).** ARQ-118, ARQ-25, ARQ-110, ARQ-111 and FV-06 are named. Left out on purpose: FV-06 (b) (GPU, no `GpuTransition` in this cut), FV-06 (c) (not in this crate), Q-47 (owner).

## Findings

### CRITICAL

- **C1. The Wipe mask is still wrong at very small softness on a `circular` edge pixel.** class: implementation. Open from round 1, narrowed. `crates/gafira-raster/src/transitions/wipe.rs:127`, `:119`. `(cx - w / 2).hypot(cy - h / 2) / half_diagonal` divides two `hypot` results, each rounded, so a `u` that is exactly `p` comes out one rounding away. `gap` is then about 1e-16 where it should be 0, and `gap / s` amplifies it. This is the amplification of round 1, now through `u`.
  - Observed through `run_cpu`, `RgbaF32`, 8x8, `circular`, `from` 0, `to` 1, `p` 0.875, pixel (0,0). Here `u` is `3.5√2 / 4√2 = 7/8`, so the exact mask is 0.875 for every `s > 0`:

    | `s` | Exact mask | Output | Error |
    |---|---|---|---|
    | 1e-16 | 0.875 | 1 | 0.125 |
    | 1e-14 | 0.875 | 0.8861023 | 1.1e-2 |
    | 1e-12 | 0.875 | 0.875111 | 1.1e-4 |
    | 1e-10 | 0.875 | 0.8750011 | 1.1e-6 |

  - Worst over the grid: 16x16, `p` 0.1875, `s` 1e-17, pixel (6,6): 1 against 0.1875, 135/255 in bytes.
  - Real frame sizes reach it: 1080x1080, `p` 0.375, `s` 1e-16, the four pixels on the diagonals give 0.9301 against 0.375. `p` values 1/8, 5/8 and 7/8 happen to be right on that size. Any square with a dyadic `odd / 2^k` on its diagonal is exposed.
  - The note of `30769fb` says the edge pixel keeps exactly `p`, and `apply-progress.md:300` says the hand table covers circular. That holds for the four axis directions only. The test of it for `circular` is 2x2 at 0.5 (`wipe.rs:600`), where the rounding happens to cancel, and the exact test is `right` only.
  - A fix needs a test on squares 4x4 to 64x64, with `p` on a diagonal pixel, against a hand-worked or exact value.

- **C3. New. A hard `circular` Wipe breaks the strict `u < p` rule on an exact-edge pixel.** class: implementation. `crates/gafira-raster/src/transitions/wipe.rs:127`, `:131`. ARQ-118 with `s = 0`: "1 si `u < p` y 0 si no", so `u == p` gives 0. The computed `u` falls just below `p`, and the pixel takes `to` in full. Softness 0 is the default.
  - 8x8, `p` 0.875, pixel (0,0): 1 against 0, 255 of 255 in bytes. The same on 4x4 at 0.75, 8x8 at 0.375, 16x16 at 0.1875 and 0.4375, 32x32 at 0.34375 and 0.96875, 64x64 at 0.328125 and 0.984375. 36 of 88 exact-edge pixels are wrong.
  - 1080x1080, `p` 0.375: the four pixels are 255 where the rule gives 0.
  - Same root cause as C1, present since `3b316f6`. `arq_118_wipe_edge_is_strict_and_at_the_pixel_centre` (`wipe.rs:347`) has no `circular` case.
  - The size of the visible effect is small: a few pixels on one frame. It is CRITICAL because it breaks a stated rule of critical code, by the full range of the output.

### WARNING

- **W5. New, follow-up. No rule says what bound holds for `circular` at softness below about 1e-11.** class: ambiguity. ARQ-118, 02 §4.17.4. `softness` is valid across `[0, 1]` and the bound is 1e-5, but a non-dyadic `u` carries a rounding of about 1e-16 in `f64`, and `1/s` amplifies it. No fix of `u` in `f64` removes this: the control gives the same figures, or about half on one size.

  | Image, crafted `p`, `s` just above the gap | Code | Control |
  |---|---|---|
  | `right` 1920x1 | 3.2e-8 | 3.2e-8 |
  | `left` 1920x1 | 2.6e-6 | 2.6e-6 |
  | `right` 6001x1 | 3.2e-6 | 3.2e-6 |
  | `circular` 1000x1000, `s` 6.7e-13 | 1.1e-4 | 5.1e-5 |
  | `circular` 1081x1081, `s` 1.8e-13 | 1.3e-4 | 1.3e-4 |
  | `circular` 1920x1080, `s` 1.9e-13 | 3.6e-4 | 3.6e-4 |

  Bytes stay within 1/255 in all of them, at 0.47 at worst, and the softness grid does not show it. It takes a pixel and a `p` that nearly coincide, and a softness tuned to their gap. Marked follow-up: it does not block the fix of C1 and C3, and the choice (a floor for the bound, or more precision) can wait for the first use of such a softness. A sentence in the numeric table of 02 §4.17.4 would settle it.

### SUGGESTION

- S1. Still open: `samples` in `crates/gafira-raster/src/transitions/zoom.rs:47` allocates a `Vec` per pixel, for a fixed 8. O5 and XP13 own the cost.
- S4. Still open: `proptest-regressions/` is untracked, still with 15 seeds. The Slice card still says `RS3`. Both are the coordinator's.

## Size

1 810 lines added in `crates/gafira-raster` against `fadac62`, 271 of them in this fix round, against a forecast of 450 (630). Noted, no finding, per the owner.

next: tdd-apply (C1, C3, implementation); W5 (ambiguity) is a follow-up. Second consecutive FAIL of class `implementation`: the coordinator should stop and ask the owner before round 3, the last.

# Verify report: gafira-raster, RS3b, round 3

- Commit: `02f7b2e` (branch `feat/gafira-raster`). Fix commit: `2cff356` (Wipe). Note: `02f7b2e`.
- Closure round, depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS3b round 2, which stays in git at `4a2ae70`. All rounds are also in `verification.json`.

**Verdict: PASS.** 0 CRITICAL, 1 WARNING (follow-up), 2 SUGGESTION.
Per class: ambiguity 1 (1 WARNING, follow-up), implementation 0, architecture 0, claim 0.

C1 and C3 are closed. The implementer's claim on the ramp mutant holds at the output. W5 stays open as a follow-up, and this round found a plainer case of it (below). RS4 to RS6 remain.

## Gates

Each exit code was read alone from its own file, on a fresh clone of `02f7b2e` with no `proptest-regressions` folder. `CARGO_TARGET_DIR` was not passed, as the coordinator asked: each export used its own default `target/`, with `CARGO_INCREMENTAL=0`.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1 456 passed, 0 failed, 0 ignored; 138 in the `gafira-raster` library) | 0 |
| `cargo deny check` | 0 |
| policy: ARQ-07 CLI | 0 |
| policy: forbid unsafe | 0 |
| policy: V5 layers | 0 |
| policy: V12 clippy copies | 0 |
| policy: ARQ-107 language | 0 |
| policy: V13 EditToken | 0 |
| policy: V14 property tests | 0 |

The 15 seeds in the untracked `proptest-regressions/` also pass on the unmutated code, on a scratch export (138 passed). The CI test job's Windows and macOS targets were not run here. Disk: 34 GB free at the start, 24 GB at the lowest reading, 29 GB after cleaning.

## Independent edge sweep

Written for this round; it reuses neither the crate's code nor the earlier harness. `p` and `s` are exact rationals. Circular `u` is exact when `u^2` is a rational square, and otherwise a root at 2 700 bits, so `u == p` is decided exactly. The crate ran through its public API, black to white, opaque, in `RgbaF32` and `Rgba8`. Bounds are those of 02 §4.17.4: 1e-5 for `f32`, 1/255 for bytes.

Grid: 402 944 cases, 215 042 496 pixels; 36 sizes from 1x1 to 1024x1, including squares of 4 to 128, odd squares and non-square shapes; 48 softness values from 1 down to 5e-324, with `s = 0`, subnormals and powers of two (every third value on images of 400 pixels or more); all 5 directions; `p` on dyadic values and on the nearest `f32` to pixel values of `u`. No non-finite output.

| Group | Pixels with exact `u == p` | `f32` over the bound at HEAD | Bytes over at HEAD | Same grid with two `hypot` calls restored |
|---|---|---|---|---|
| axis, `s = 0` | 27 858 | 0 | 0 | 0 |
| axis, `s > 0` | 422 742 | 0 | 0 | 0 |
| `circular`, `s = 0` | 988 | 0 | 0 | **372** |
| `circular`, `s > 0` | 14 896 | **8, none of them exact-edge (W5)** | 0 | **4 504** (4 032 in bytes) |

The last column is the positive control: this sweep sees the defect of round 2, at up to 255/255. At HEAD all 15 884 circular exact-edge pixels are right.

## Closure

| Finding | Status | Evidence |
|---|---|---|
| C1 | **Closed** | `crates/gafira-raster/src/transitions/wipe.rs:130`: no circular exact-edge pixel over the bound at any softness. `arq_118_wipe_circular_exact_edge_pixels_keep_the_rule` (`wipe.rs:662`) and `arq_118_wipe_circular_exact_edge_on_1080_square` (`wipe.rs:690`) fail with the old `u` restored. |
| C3 | **Closed** | 988 of 988 hard circular exact-edge pixels give 0, against 372 wrong in the control. Mutant `u <= p` is killed by both new tests and two older ones. |
| C2, W1, W2, W3, W4 | Closed in round 2 | Not reopened. The saturation test is unchanged, and on the new path `|gap| <= s` keeps the quotient at most 1. |
| W5 | Open, follow-up | See below. Not routed, as the coordinator said. |
| S1, S4 | Open | Not touched. |

The comment's claim holds too. For an exact edge `u == p`, `p` has at most 24 bits, so `p^2` fits an `f64`; the quotient is then exact and the root returns `p`. `a^2 + b^2` and `w^2 + h^2` are exact up to sides of about 2^26.

## Claim: ramp quotient regardless of `|p - u| <= s`

**Equivalent at the output: confirmed.** Applied on a scratch export.

- 138 of 138 tests pass, so the mutant survives.
- The full sweep is byte-identical to HEAD (`cmp` exit 0 over 1 075 212 480 bytes, `f32` and `Rgba8`).
- A bit comparison of both masks, on 400 000 000 draws at the extremes (`u` within 4 ulp of `p`, `s` subnormal, `s` equal to `|gap|`, `u` 0 and 1; 225 699 735 saturated): 0 differences, 0 NaN.
- By argument: with `|gap| > s` the rounded quotient has magnitude at least 1, so `p + q` clamps to 1, or to 0 since `p <= 1`. `gap` and `p` are finite and `s > 0`, so no NaN arises.

One limit on the word "equivalent": the mutant is not equivalent inside. It went through an infinity on 36 667 405 of those draws, which ARQ-110 forbids for an intermediate, while the code had 0 inside its ramp. So the saturation test is what holds ARQ-110 there, and no tracked test can see it, as round 2 recorded under W1.

## Hand mutants

Closure probes only, no independent budget spent. Applied one at a time to a scratch export; `cargo test -p gafira-raster --lib`. 5 run, 4 killed, 1 equivalent.

| Mutant | Result |
|---|---|
| Two-`hypot` quotient restored | Killed: both new tests |
| `u <= p` in the hard Wipe | Killed: both new tests, `arq_118_wipe_defaults_are_right_and_hard`, `arq_118_wipe_edge_is_strict_and_at_the_pixel_centre` |
| `diagonal_sq` as `2 w^2` (square only) | Killed: `arq_118_wipe_circular_centre_and_half_diagonal`, `arq_118_wipe_matches_the_criterion_for_every_direction` |
| Clamp removed | Killed: the four recorded tests and `arq_118_wipe_soft_stays_within_1_255_of_the_ends_near_them` |
| Ramp quotient regardless of `|p - u| <= s` | Survived, **equivalent** at the output (above) |

## Criteria

19 criteria: 19 covered. The two failing in round 2 now hold: the Wipe mask with softness (C1) and `s = 0` strict `u < p` (C3). The other 17 stand as in round 2; the code outside `u` and the test module of `wipe.rs` did not change.

**Artifacts (step 3c).** Since `4a2ae70`: `2cff356` touches only `wipe.rs` (69 insertions, 2 deletions), `02f7b2e` only `apply-progress.md` (9 insertions, 0 removed). The Slice card (`b64090c`, amended by `75e360d`) is unchanged. `02` and `design.md` are unchanged. Neither commit carries AI attribution.

**Product IDs (`git grep` over the tests).** ARQ-118, ARQ-25, ARQ-110, ARQ-111 and FV-06 are named. Left out on purpose: FV-06 (b) (GPU, no `GpuTransition` in this cut), FV-06 (c) (not in this crate), Q-47 (owner).

## Findings

### CRITICAL

None.

### WARNING

- **W5. Open, follow-up. No rule says what bound holds for `circular` at softness below about 1e-11.** class: ambiguity. ARQ-118, 02 §4.17.4. Round 2 stands, with one new data point: the softness need not be tuned to the gap. On 128x128, `p` 0.7984436 (nearest `f32` to `u` of pixel (29,0), not an exact edge), `s` 1e-12, 8 symmetric pixels read 0.8897039 against 0.8896594: 4.5e-5, over the 1e-5 bound. Bytes stay within 1/255, at 0.50 at worst. `u` there is already the nearest `f64` (0.40 ulp off), so no `f64` `u` does better, and this is not a defect of the fix. `p` must still land within about 1e-13 of a non-dyadic `u`.

### SUGGESTION

- S1. Still open: `samples` in `crates/gafira-raster/src/transitions/zoom.rs:47` allocates a `Vec` per pixel, for a fixed 8. O5 and XP13 own the cost.
- S4. Still open: the Slice card still says `RS3`. The coordinator's. `proptest-regressions/` is left out here: a known pending owner decision.

## Size

1 877 lines added in `crates/gafira-raster` against `fadac62`, 69 of them in this fix round, against a forecast of 450 (630). Noted, no finding, per the owner.

next: tdd-apply (next delivery, RS4). W5 (ambiguity) is a follow-up, not routed. T4 is owed at close.

# Verify report: gafira-raster, RS4, round 1

- Commit: `c063a3b` (branch `feat/gafira-raster`), base `07ba7d3`. Implementation commits: `3d400cd`, `e2358e1`, `5243b16`; notes `c063a3b`.
- First verify of RS4, depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS3b round 3, which stays in git at `144e888`.
- Acceptance source: Slice card in `apply-progress.md`; ARQ-120, ARQ-114 (`bench_shadow_1080`), ARQ-110, ARQ-111 in `specs/02-arquitectura-software.md` with `07ba7d3`; FD-35 (a, d; Paralela only, D-42) in `specs/04`; P11 in `specs/changes/layer-2/design.md`.
- Probes ran on a scratch export before the implementer's tests were read. The worktree was not changed.

**Verdict: FAIL.** 3 CRITICAL, 5 WARNING, 3 SUGGESTION.
Per class: implementation 6 (3 CRITICAL, 3 WARNING), ambiguity 2 (2 WARNING), architecture 0, claim 0.

The shadow is sound in the main domain: tiles match bit for bit, FD-35 (d) holds, every ARQ-110 table entry holds, and the budget is met. All three CRITICALs have small fixes. One (C1) is a real accuracy gap at blur radii a user can set.

## Gates

Each run alone, exit code read directly, `CARGO_TARGET_DIR` not passed.

| Gate | Exit | Note |
|---|---|---|
| `cargo fmt --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 1 484 passed, 0 failed, 0 ignored |
| `cargo deny check` | 0 | advisories, bans, licenses and sources ok |
| CI policy (7 steps of the `policy` job besides Format, read from `ci.yml` with a YAML parser) | 0 | ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13, V14 |

`git status` at the end: only `crates/gafira-raster/proptest-regressions/` untracked (known pending). The worktree holds the seed `effects/shadow.txt` and the gates replay it; a clean export has no seeds and the `effects::` tests pass there too.

## Artifact integrity

- The Slice card was not changed in RS4: `07ba7d3..c063a3b` removes no line of `apply-progress.md`, it only appends `## RS4`.
- `07ba7d3` touches only `specs/02-arquitectura-software.md` (the coordinator's amend giving `roi` and `cost_hint` a `cx`) and precedes the first implementation commit. No implementer commit touches `specs/` other than `apply-progress.md`.
- The code matches the amended signatures in 02 §4.4.

## Independent probes (step 1c)

Public API only, f32 impulse with the shadow offset clear of the element.

**Impulse sigma (ARQ-120, ±2 %).**

| Radius × scale | Measured σ | Wanted | Error |
|---|---|---|---|
| 0.1, 0.25 | 0 | 0.05, 0.125 | −100 % |
| 0.5 | 0.0259 | 0.25 | −89.6 % |
| 0.75 | 0.2325 | 0.375 | −38.0 % |
| **1.0** | 0.4637 | 0.5 | **−7.26 %** |
| 1.1 | 0.5331 | 0.55 | −3.08 % |
| 1.2 | 0.5929 | 0.6 | −1.18 % |
| 1.5, 2, 3, 5 | | | −0.03 %, −0.20 %, −0.14 %, −0.40 % |
| 10, 40, 100 | | | −1.01 %, −1.25 %, −1.31 % |
| 2000, 2700 | | | −1.32 %, −1.33 % |
| 2800, 2900, 3000 | | | −1.62 %, −2.10 %, −2.66 % (cap 4096) |
| 3200, 4000 | | | −3.96 %, −11.06 % (cap 4096) |

Scale goes through the same path: radius 2 at scale 0.5 gives −7.26 %, and radius 1 at scale 0.5 gives −89.6 %.

**Other probes.**

| Probe | Result |
|---|---|
| Tiny blur | Blur `1e-162`, `2e-162`, `1e-170` and down to `5e-324`, and `1e-160` at scale `1e-30`: `RgbaF32` gives `Err(NonFinite { field: "backdrop" })`; `Rgba8` gives `Ok` with the shadow gone (differs from blur 0). Blur `3e-162` and above are right. |
| Tiles with `roi` | 3 scales × 6 parameter sets × 5 tiles, including offsets 101, −20, 30, 0.5, 1.5: max diff 0 in `Rgba8` and in `RgbaF32`. |
| Half image, offset 0, noisy 2×2 blocks, margin clear of the edges | r 40: 0.50/255; r 16: 1.00; r 8: 1.50; r 6: 1.75; r 5: 2.25; r 4: 3.00; r 3: 5.75; r 2: 16.25; r 1: 17.50. |
| Half image, even offset | (4, 6) blur 16: 1.00/255; blur 0: 0. |
| Half image, odd offset | (3, 3) blur 16: 5.75/255; blur 8: 17.00; blur 0: 134.25; (1, 1) blur 8: 17.00. |
| Constant opaque, over 3σ inside | r 2 to 40: at most 4.6e-5/255 off. |
| FD-35 (a), `Rgba8`, blur 0, `floor(a·op/100 + 0.5)` in integers | 20 of 51 456 mismatches on opacities in steps of 0.5, all one byte low at an exact tie: a 50 op 29 gives 14, not 15; a 90 op 35 gives 31, not 32; a 100 op 14.5 gives 14, not 15. |
| ARQ-110 | NaN, ±∞ in each parameter, the colour and `cx.scale` are `Err` naming it; NaN colour under alpha 0 is `Err`; an unknown id with NaN is ignored; ±0.01 outside a range is `Err`. `offset_x = -0.01` is `Ok`, as its range allows. Overflow in the composite is `Err` and `dst` stays intact. Scale −1, 0, `MIN_POSITIVE`, `±MAX`: no panic in `run_cpu`, `roi` or `cost_hint`. |
| FD-35 (d) | Extremes pass, outside is `Err`; opacity 0 is byte-exact. |
| `roi` at scale 2, offset 0 | Contains `out` grown by `⌈2r⌉` for r 1, 10, 100. With offset (100, −100) and r 1 the far side is not grown (S2). |
| Premultiplied `Rgba8` | 0 channels above alpha on valid input. |
| Offset rounding | 0.5 → 1, −0.5 → −1, 0.49 → 0; 3 at scale 0.5 → 2; −3 at scale 0.5 → −2. |

**Budget (ARQ-114).** `bench_shadow_1080` exists under that name. Timings below are on this machine with concurrent `rustc` jobs, so they are not reference figures.

| Case, 1080p, single thread | `Rgba8` | `RgbaF32` |
|---|---|---|
| r 40, opacity 60, median of 9, lower load | 81.8 ms | 83.9 ms |
| r 40, opacity 100, median of 9, lower load | 80.9 ms | 82.2 ms |
| r 40, opacity 60, load average about 15 | 122.2 ms | 148.2 ms |
| `cargo bench`, load average 15 to 21 | 107.9 to 147.9 ms | |

The < 100 ms figure and the implementer's 78.8 ms are credible at low load. It is not a `cargo test` gate. `RgbaF32`, which P11 left unmeasured, is within 3 % of `Rgba8`. Non-x86 stays unmeasured.

## Comparison with the implementer's tests

- `arq_120_impulse_response_has_sigma_of_half_the_radius` tests only radii 7, 10, 16 and 40, so it never reaches the band where the rule fails (C1). The reference-kernel test compares against the same sampled Gaussian, so it cannot catch it.
- `arq_110_finite_input_gives_finite_output` draws `blur` uniformly in `0..=100`, which never gets near `1e-162` (C2).
- No test reaches the composite overflow in `RgbaF32` (C3).
- `arq_120_a_half_size_image_at_half_scale_matches_after_rescaling` uses one block, r 24, offset 0, alpha only. It passes, but it does not show the limits in W1 and W2.
- `fd_35_a_*` uses opacities 100, 50, 30 and 1, with no tie that the `f64` factor gets wrong (W4).

## Hand mutants (step 4b)

| Mutant | Rule | Result |
|---|---|---|
| Spot-check: colour alpha ignored in `Rgba8` (recorded) | deviation 5 | Killed: `fd_35_the_colour_tints_the_shadow_and_its_alpha_scales_it` |
| M1: `RgbaF32` composite written straight into `dst` | ARQ-110, `Err` before anything is written | **SURVIVES**. Non-equivalent: on `s = [0.5,0,0,0.5, 3.3e38,0,0,0.5]` with colour r `1e16`, HEAD leaves `dst` at its sentinel and the mutant leaves pixel 0 written, both with `Err` |
| M2: offset rounded half up, `floor(v + 0.5)` instead of half away from zero | deviation 1, FD-35 sign symmetry | **SURVIVES**. −0.5 moves 0 px instead of −1 |
| M3: `Rgba8` tint from linear light instead of sRGB bytes | deviation 5, same rule as Fade | Killed: `fd_35_the_colour_tints_the_shadow_and_its_alpha_scales_it` |

## Rulings

**The nine recorded deviations.**

| # | Deviation | Ruling |
|---|---|---|
| 1 | Offsets × `cx.scale`, rounded half away from zero | Multiplying: accepted, the tile and half-size criteria force it, and `roi` and pixels share it. Rounding to whole device pixels: accepted for now, its limit is W1, and the rule is untested (W3). |
| 2 | Defaults, animatable, ids and label keys | Accepted. The keys follow the `glossary.transition_*` and `transition.<name>.<param>` convention. |
| 3 | Cap of 4096 taps | Accepted, with S1. `roi`, cost and pixels agree. σ stays within ±2 % up to radius × scale of about 2 880 (blur 100 at scale about 28.8). Above that, accuracy falls: −3.96 % at 3 200, −11.06 % at 4 000. Only blur near 100 at scale near 29 or more reaches it. No test proves the accuracy near the cap, only the `f64::MAX` clamp. |
| 4 | Zero shadow keeps the element; transparent element gives the shadow | Accepted. FD-35 (d) needs the first, and ARQ-119 (alpha 0 source leaves the backdrop) gives the second. |
| 5 | Colour alpha scales the shadow; sRGB bytes in `Rgba8`, linear in `RgbaF32` | Accepted. Same conversions as Fade (`fade.rs:128`, `fade.rs:146`). An out-of-gamut colour stays unclipped in `RgbaF32`, also as in Fade (colour r 1.5 gave channel 1.27 at alpha 0.5). |
| 6 | `Rgba8` alpha blurred in 0..255 | Accepted; blur 0 is exact in the plane. The tie error of W4 comes later, from the opacity factor. |
| 7 | `roi` and `cost_hint` read bad numbers as 0 | Accepted: ARQ-110 leaves that value unspecified. |
| 8 | Scale check after computing is an equivalent mutant | Accepted: `run_cpu` computes nothing observable before writing, and the implementer's test pins the scale check before the parameters. |
| 9 | `proptest-regressions/` untracked | Known pending, no finding. |

**RED record.** Tests did not precede all code, so this is W5. The record says a stub was written first (`schema` empty, `run_cpu` `Ok(())`, `roi` returning `out`, cost 0), and also a stub `over_u8`, and that the tests ran RED against it. The stub is signatures with empty bodies, so the logic came after those tests. But `e2358e1` holds tests and code together, so git cannot show the order. The record itself lists five groups not seen RED, one of them `fd_35_the_colour_tints_*`, added after the code for a surviving mutant. No stub remains at HEAD.

**Size.** 1 660 lines against forecast 420 (590). Not a finding, on the owner's explicit decision.

## Criteria

| Criterion | Evidence | State |
|---|---|---|
| ARQ-120 impulse σ ±2 % | Probe, table above | **Failing** for radius × scale below about 1.16 (C1), and at about 2 880 or more (S1) |
| ARQ-120 constant opaque beyond 3σ | `arq_120_constant_image_keeps_*`, probe | Covered |
| ARQ-120 half image at scale 0.5 | `arq_120_a_half_size_*` | Covered for the case tested; limits in W1, W2 |
| ARQ-120 `roi` at scale 2 | `arq_120_roi_grows_*`, probe | Covered |
| ARQ-120 tiles at scale 1, 0.5, 2 | `arq_120_a_tile_*`, probe | Covered, bit-exact |
| ARQ-120 radius 0 identity | `arq_120_radius_zero_is_the_identity` | Covered |
| ARQ-120 transparent outside | `arq_120_matches_the_reference_gaussian_*`, `arq_120_shifted_origin_*` | Covered |
| ARQ-120 radius × scale | `arq_120_radius_and_offset_*` | Covered |
| FD-35 (a) | `fd_35_a_*`, probe | Covered; tie rounding in W4 |
| FD-35 (d) opacity 0 | `fd_35_d_opacity_zero_*` | Covered |
| FD-35 (d) ranges | `fd_35_d_extremes_*` | Covered |
| ARQ-110 table, write nothing | `arq_110_*`, probe | Covered; composite overflow unguarded (C3) |
| ARQ-110 intermediate non-finite | Probe | **Failing** (C2) |
| ARQ-110 `roi`, `cost_hint` no panic | `arq_110_roi_and_cost_hint_*`, probe | Covered |
| §4.17 `Cost` monotonic in pixels | `arq_110_cost_hint_never_falls_*` | Covered |
| ARQ-111 | `arq_111_two_runs_*`; no threads, no entropy | Covered |
| ARQ-114 `bench_shadow_1080` | `benches/shadow.rs`, run | Covered |
| P11 kernel, 3σ, `roi` | `blur.rs`, `arq_120_half_width_*` | Covered |

Every ID has tests naming it: ARQ-120, FD-35, ARQ-110, ARQ-111 in `src/effects/`; ARQ-114 in `benches/shadow.rs`. Left out by the Slice card: FD-35 (b), (c), Brillo and Contorno (D-42).

## Findings

### CRITICAL

- **C1**, class: `implementation`. `crates/gafira-raster/src/effects/blur.rs:23` (`kernel`), ARQ-120. σ misses ±2 % below radius × scale of about 1.16: −7.26 % at 1 px, −38 % at 0.75, −89.6 % at 0.5, and 0 below that. It is reachable: blur 1 in the Inspector, or blur 2 at scale 0.5. *Root cause:* sampling a Gaussian at integer taps gives less variance than σ² once σ falls under about 0.6, because tail weights fall much faster than σ². *Fix:* for `sigma < 0.6` use the exact-variance kernel `[σ²/2, 1 − σ², σ²/2]` on taps −1, 0, +1, with any outer taps 0 so `half_width`, `roi` and cost do not change; at σ 0.6 the two forms differ by 1.18 % in σ. Extend the sigma test to radii 0.25, 0.5, 0.75, 1, 1.1, 1.2, 1.5, 2, 3, 5, and to scale 0.5 and 2 through `run_cpu`. If the owner prefers to keep the plain sampled Gaussian, the alternative is a spec bound on this criterion (radius × scale ≥ 1.2).
- **C2**, class: `implementation`. `blur.rs:9` (`half_width`) and `blur.rs:23`, ARQ-110, "un resultado intermedio no finito da `Err` que nombra el campo". For a blur in range below about `3e-162`, `sigma * sigma` underflows to 0, so the centre weight is `0/0`, NaN, and the whole kernel is NaN. `Rgba8` returns `Ok` and silently loses the shadow, because `NaN as u8` is 0. `RgbaF32` returns `Err` naming `backdrop`, not `blur`. Input is extreme, but it sits inside FD-35's range and breaks an explicit sentence, the same class as the tiny-softness Wipe fix in RS3b. *Fix:* one line in `half_width`: return 0 unless `radius_px * radius_px > 0.0`, which also covers NaN and gives the identity. Test blur `1e-200` and `5e-324`, and `1e-160` at scale `1e-30`, in both formats, equal to blur 0.
- **C3**, class: `implementation`. `crates/gafira-raster/src/effects/shadow.rs:241`, ARQ-110. Survivor M1: the code is correct today, but no test proves that a composite overflow leaves `dst` intact. *Fix:* a test with the input of M1, asserting `Err` and `dst` equal to its sentinel bit for bit.

### WARNING

- **W1**, class: `ambiguity`. FD-35, ARQ-120, deviation 1. The spec does not say whether offsets are sub-pixel. Rounding to whole device pixels makes an odd offset miss the half-size criterion: 17/255 at (1, 1) blur 8 and 134/255 at (3, 3) blur 0. Animated offsets also step by whole pixels. *Fix:* the specifier decides either "offsets round to whole device pixels and the half-size criterion applies to offsets that are whole at that scale" (no code), or sub-pixel offsets (a fractional kernel phase, linear at blur 0).
- **W2**, class: `ambiguity`. ARQ-120, "una imagen y su mitad". No image or radius is named. On noisy 2×2 blocks at offset 0 it holds for r ≥ 16 and misses at r 8 (1.50/255). r 2 to 5 give 2.25 to 16.25, and r 1 gives 17.50. By analysis, not measured, no Gaussian can meet it on hard edges at small radii: the 2×2 mean adds variance that the half-size run does not have. C1's fix may help at r 1 to 2; re-measure after it. *Fix:* the specifier names the test image (smooth, or blocks) and a lower radius, for example `r × scale ≥ 8` at the lower scale.
- **W3**, class: `implementation`. `shadow.rs:107`, deviation 1. Survivor M2: half away from zero is untested. Not critical, because this is the implementer's rule and the spec is silent, and it depends on W1. *Fix:* assert that offsets −0.5 and −1.5 at scale 1 move 1 and 2 px left.
- **W4**, class: `implementation`. `shadow.rs:212` and `:220`, FD-35 (a), deviation 6. `factor = opacity / 100` is inexact, so 20 of 51 456 pairs land one byte low at an exact tie (a 50, opacity 29 gives 14, not 15). Each is a nearest value, so "exacto" is not clearly broken, but the implementer's own rule is: the same tie rounds both ways. *Fix:* compute `cover * opacity * tint.alpha / 100.0`, which is exact at those ties. Add a table of the 13 integer pairs (alpha, opacity): (50, 29), (90, 35), (170, 35), (150, 41), (50, 57), (150, 57), (25, 58), (150, 69), (45, 70), (85, 70), (165, 70), (175, 70), (75, 82).
- **W5**, class: `implementation`. `apply-progress.md` RS4 RED record. Not test-first throughout, against the owner's rule: see "RED record" above. Tests that are already green cannot be re-ordered, so *fix:* in the fix rounds, write each new test, run it RED against the code as it stands, record that, and add no stub.

### SUGGESTION

- **S1**, class: `ambiguity`. `blur.rs:5`, deviation 3. Set `MAX_HALF_WIDTH` to 4800 (1.5 × blur 100 × FD-07 maximum zoom 32) so the cap has a reason and 3σ holds if `scale` follows the zoom. Or give `cx.scale` an upper bound in the spec. Add a test of σ at the cap.
- **S2**, class: `ambiguity`. ARQ-120, `roi` criterion. With an offset larger than the kernel, `roi` does not grow the far side, and it does not need to, so tiles stay exact. Read literally, the criterion covers offset 0. A note in the row would close it.
- **S3**, class: `claim`. `benches/shadow.rs:4` says a nightly job reads the figure, and only `ci.yml` exists; `benches/transitions.rs` says the same. Either add the job or reword both. Non-x86 stays unmeasured, as P11 expected.

## Next

Fix order: W1 and W2 in the spec first, since they settle what C1's test must cover at half scale. Then C1, C2, C3, W3, W4 in code, with new tests RED first (W5). Round 2 re-checks these only; T4 stays owed at close.

next: specifier (ambiguity: W1, W2), then tdd-apply (implementation: C1, C2, C3, W3, W4, W5)
