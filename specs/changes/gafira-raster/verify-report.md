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
