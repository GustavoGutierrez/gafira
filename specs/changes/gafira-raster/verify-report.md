# Verify report: gafira-raster, RS3b, round 1

- Commit: `608b196` (branch `feat/gafira-raster`, base `fadac62`). Delivery commits: `bbd49c5` (Slide), `3b316f6` (Wipe), `f78c829` (Zoom), `608b196` (notes).
- First verify of this code, depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS3a round 2, which stays in git at `9739549`. RS3a round 1 stays at `107f9d9`, RS2 round 2 at `1909734`, RS2 round 1 at `0b563fe`, RS1 round 2 at `e5e1d0e`. All rounds are also in `verification.json`.

**Verdict: FAIL.** 2 CRITICAL, 4 WARNING, 4 SUGGESTION.
Per class: implementation 5 (2 CRITICAL, 3 WARNING), ambiguity 1 (1 WARNING), architecture 0, claim 0.

Slide and Zoom match an exact independent reference on every case. The two CRITICAL are the Wipe mask at very small softness, which is a code defect, and an untested `f64` rule in Slide.

## Gates

Each exit code was read alone, on an export of `608b196` with no `proptest-regressions` folder.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1 448 passed, 0 failed, 0 ignored; 130 in the `gafira-raster` library) | 0 |
| `cargo deny check` | 0 |
| policy: ARQ-07 CLI | 0 |
| policy: forbid unsafe | 0 |
| policy: V5 layers | 0 |
| policy: V12 clippy copies | 0 |
| policy: ARQ-107 language | 0 |
| policy: V13 EditToken | 0, in a git clone |
| policy: V14 property tests | 0, in a git clone |

V13 and V14 exited 1 on the first run, on the `git archive` export: both logs read `fatal: not a git repository`, from their own `git grep` and `git ls-files`. That is a fault of my export, not of the code. Both, and ARQ-107, were re-run on a real clone at `608b196`, and those are the codes above.

The 15 seeds in the untracked `proptest-regressions/` also pass on the unmutated code, on the scratch export. The CI test job's Windows and macOS targets were not run here. Disk: 36 GB free at the start, 31 GB at the lowest reading, 35 GB after `cargo clean` of all three build folders.

## Independent probes (step 1c)

Written from ARQ-118 before the implementer's tests were read, on a scratch export.

**Exact reference.** A Python reference shares no code with the crate. It uses exact rationals for `p`, `s`, `u`, `d` and the taps, and 300-bit arithmetic for the circular distance. The crate ran through its public API, with results compared by bits.

- 60 052 cases: 15 sizes from 1x1 to 64x1, 18 values of `p` from 1e-45 to 0.99999994, 21 softness values from 5e-324 to 1, every direction and both formats. `Rgba8` was compared with a tolerance of 1/255 from the unrounded target.
- Slide, 2 272 cases, ties included: **0 mismatches**.
- Zoom, 1 080 cases: **0 mismatches**.
- Wipe, 56 700 cases: 5 901 `f32` and 4 163 `Rgba8` pixels wrong, all at softness <= 1e-12 (C1). None at softness >= 1e-10 in this grid; a finer sweep puts the limit of the 1e-5 bound near 3e-12.

**Public API probes, all passing:**

| Probe | Result |
|---|---|
| 02 §4.13 table: ids, names, version, `Both`, schema, labels, `animatable` | As the table |
| `softness` NaN, `±∞`, 1.0000000000000002, -5e-324, -0.1, 1.5, `Int` | `Err` naming it; also at `p` 0 and 1 and on size 0 |
| `softness` 0, -0.0, 5e-324, 1 | Accepted |
| `direction` `diagonal` or a `Float`, on all three | `Err` |
| Unknown id with NaN or `±∞` | Same bytes as without it |
| Ends at `p` 0, -0.0, -0.5, -1e-45, `-MAX`, 1, 1.5, 1.0000001, `MAX`, pixels with `±MAX`, 1e30, 1e-45, -0.0, all softness values | Identical bit for bit |
| Pixels with `±MAX`, 1e30, -1e38, subnormals, mid `p` | Finite; `c <= a` kept in bytes |
| Zoom constants, 19 540 224 bytes, `c <= a` | 0 differences |
| Zoom constants, `f32`, `±MAX` to 1e-45 | Within 1e-6 relative |
| Zoom opaque inputs, 2 001 values of `p`, 6 sizes, both directions | Alpha exactly 255 and 1.0 |
| Zoom mirror on the same grid | 1/255 worst; 2.1e-7 in `f32` |
| Wipe `s = 1` at 0.0001 and 0.9999, 640x3, black to white both ways | <= 1/255 |
| 3 000 000x1 and 1x3 000 000 strips | No panic |

Measured, `--release`, 1080p opaque `Rgba8`, one run at `p` 0.37: Slide 8.5 ms, hard Wipe 9.7 ms, soft Wipe 87 ms, Zoom 303 ms. `MIX_SPACE` is provisional pending O5: no finding.

**Against the implementer's tests.** Geometry tests are strong: every mutant on them below dies. Three are weak. `arq_118_wipe_matches_the_criterion_for_every_direction` repeats the code's own expression and tries softness 0, 0.2, 0.5 and 1 only (C1). `arq_118_slide_offset_rounds_half_away_from_zero` has no `p` that separates `f64` from `f32` (C2). The generic tests run on default parameters only (W2).

## Hand mutants (step 4b)

Applied one at a time to a scratch export; `cargo test -p gafira-raster --lib`. 10 run, 7 killed, 2 real survivors, 1 equivalent.

| # | Mutant, rule | Result |
|---|---|---|
| S1 | Spot-check of the recorded list: half diagonal as the longer side | Killed: `arq_118_wipe_circular_centre_and_half_diagonal`, `arq_118_wipe_matches_the_criterion_for_every_direction` |
| I1 | Slide `d` computed in `f32` (rule: "en `f64`") | **Survived** (C2) |
| I2 | Wipe: softness below 1e-9 treated as hard (rule: only `s = 0`) | **Survived** (C1) |
| I3 | Wipe: no upper clamp of the mask | Killed: 4 tests, among them `arq_118_wipe_soft_edge_values` |
| I4 | Wipe circular: distance from the pixel corner | Killed: `arq_118_wipe_circular_centre_and_half_diagonal` |
| I5 | Zoom: pivot `y` from the width | Killed: `arq_118_zoom_pivot_is_the_continuous_centre` |
| I6 | Zoom: `y` tap clamped by the width | Killed: `arq_118_zoom_pivot_is_the_continuous_centre` |
| I7 | Zoom: `trunc` for `floor`, negative positions extrapolate | Killed: `arq_118_zoom_ramp_values_for_in_and_out` |
| I8 | Guard: end copies removed for all six | Survived, **equivalent** (W4) |
| I9 | Wipe: `(1 + s)` dropped in `circular` only | Killed: `arq_118_wipe_circular_centre_and_half_diagonal` |

## Criteria

19 criteria: 15 covered, 3 weak, 1 failing.

| Criterion | Status | Proof |
|---|---|---|
| 02 §4.13 identity and schema | Covered | `slide_`, `wipe_`, `zoom_has_the_identity_and_schema_of_the_table`, probe |
| `direction` `diagonal` is `Err` | Covered | `arq_110_slide_rejects_an_unknown_direction`, Wipe and Zoom equivalents |
| Ends exact: ARQ-118, ARQ-25, FV-06 (a) | Covered | `arq_118_ends_are_exact_copies_for_every_transition`, `arq_118_wipe_ends_are_exact_with_any_softness`, probe |
| `progress` clipped | Covered | Same tests, probe |
| Slide 10x4 rows | Covered | `arq_118_slide_acceptance_rows_on_10_by_4` |
| Slide `d` half away from 0, in `f64` | **Weak** (C2) | `arq_118_slide_offset_rounds_half_away_from_zero` |
| Slide replaces; alpha 0 clears `from` | Covered | `arq_118_slide_replaces_and_does_not_composite`, `arq_118_slide_copies_f32_pixels_bit_for_bit` |
| Wipe 4x2 rows | Covered | `arq_118_wipe_acceptance_rows_on_4_by_2` |
| Wipe `s = 0`, strict `u < p`, pixel centre | Covered | `arq_118_wipe_edge_is_strict_and_at_the_pixel_centre` |
| Wipe mask with softness | **Failing** (C1) | Fails the reference below `s` about 3e-12 |
| Wipe circular; `p` 1 leaves no `from` | Covered | `arq_118_wipe_circular_centre_and_half_diagonal` |
| Wipe `s = 1` near the ends | Covered | `arq_118_wipe_soft_stays_within_1_255_of_the_ends_near_them` |
| Zoom scales, pivot, sampling, bilinear premultiplied | Covered | `arq_118_zoom_ramp_values_for_in_and_out`, `arq_118_zoom_filters_premultiplied_values`, `arq_118_zoom_matches_the_criterion_tap_by_tap`, reference |
| Zoom edge clamp, opaque, constant | Covered | `arq_118_zoom_constant_images_stay_constant_and_opaque_images_opaque` |
| Zoom `out` mirrors `in` | Covered | `arq_118_zoom_out_mirrors_zoom_in` |
| Formats, sizes, `Alpha8`, size 0, `Err` before writing | Covered | Generic tests of `mod.rs` over all six |
| ARQ-110 parameters | Covered | `arq_110_wipe_rejects_bad_parameters_before_writing` |
| ARQ-110 finite property, per direction | **Weak** (W2) | Probes pass; no such test through `run_cpu` |
| ARQ-111 per direction | **Weak** (W2) | `arq_111_transitions_are_deterministic`, default parameters only |

**Artifacts (step 3c).** `b64090c` (slice card) touches only `apply-progress.md`, and `75e360d` is the amend. Against `fadac62`, `apply-progress.md` has 43 additions and 0 removed lines, so no implementer commit changed the card. The P15 and spec amendments are not in this delivery. No commit carries AI attribution.

**Product IDs (`git grep` over the tests).** ARQ-118, ARQ-25, ARQ-110 and ARQ-111 are named. FV-06 is not named by any test; see S3. Left out on purpose: FV-06 (b) (GPU, no `GpuTransition` in this cut), FV-06 (c) (not in this crate), Q-47 (owner).

W6 of RS3a: `the_f64_curves_agree_with_the_analytic_transform` exists and passes. Not re-mutated here.

## Findings

### CRITICAL

- **C1. The Wipe mask is wrong at very small softness, on a pixel at the edge.** class: implementation. `crates/gafira-raster/src/transitions/wipe.rs:133`. `(p (1 + s) - u) / s` subtracts two nearly equal numbers and divides by a tiny `s`, so the rounding error of `1 + s` and `p (1 + s)` is amplified. Where `u == p` the formula gives `p`, exactly, for any `s > 0`.
  - Observed through `run_cpu`, `RgbaF32`, 8x1, `right`, `from` 0, `to` 1, `p` 0.4375, pixel 3:

    | `s` | Exact mask | Output | Error |
    |---|---|---|---|
    | 1e-16 | 0.4375 | 0 | 0.44 |
    | 3e-16 | 0.4375 | 0.3701 | 0.067 |
    | 1e-14 | 0.4375 | 0.4385 | 1.0e-3 |
    | 1e-13 | 0.4375 | 0.4374 | 7.2e-5 |
    | 1e-12 | 0.4375 | 0.4375 | 3.9e-5 |

  - Worst error over 8x1 and 1024x1: 0.98 at 1e-16, 8.3e-5 at 1e-12, 1.9e-5 at 3e-12. The bound of 02 §4.17.4 is 1e-5 for `f32` formulas and 1/255 for bytes. In bytes the error reaches 191 of 255.
  - `softness` is valid across `[0, 1]`, so this is inside the domain. `p` is dyadic and a power-of-two side puts `u` on it, so the case is not contrived in the geometry, only in the softness.
  - Test gap: no test tries softness between 0 and 0.2, and the reference test repeats the same expression, so it would agree with the error. Mutant I2 (softness below 1e-9 is hard) survives for the same reason.
  - The same expression in `p + (p - u) / s` form has no such error. A fix needs a test with `u == p` and tiny softness, against a value worked by hand.

- **C2. The `f64` rule of Slide has no test.** class: implementation. `crates/gafira-raster/src/transitions/slide.rs:101`. ARQ-118 says `d` is computed "en `f64`". Mutant I1, the same line in `f32`, survives all 130 tests. It is a real difference: side 5 at `p` 0.1 gives `d` 4 in `f64` and 5 in `f32`; side 10 at 0.05 gives 9 and 10; side 10 at 0.55 gives 4 and 5. 52 witnesses among 9 sides and 1 999 values of `p`. The code is right, and the reference confirms it; the gap is the test. Critical code, so CRITICAL by step 4b.

### WARNING

- **W1. A subnormal softness goes through `±∞` and returns `Ok`.** class: implementation. `crates/gafira-raster/src/transitions/wipe.rs:133`. With `s` 5e-324, `(p - u) / s` overflows to `±∞`, and the clamp hides it. ARQ-110 says a non-finite intermediate result gives `Err`. The output is finite and matches the formula away from `u == p`, so no value is lost beyond C1. An `Err` on a valid softness would be a poor answer; testing the two saturated cases first, then dividing only inside the ramp, removes the `∞` and needs no spec change.

- **W2. The generic tests see one configuration of each new transition.** class: implementation. `crates/gafira-raster/src/transitions/mod.rs:89`, `:114`, `:570`, `:773`. `run_u8` and `run_f32` pass an empty `ParamValues`, so `all()` covers Slide `left`, hard Wipe `right` and Zoom `in` only. ARQ-111 does not reach `circular`, soft Wipe or Zoom `out`. ARQ-110's property (finite input in the domain gives finite output) has no test through `run_cpu` for the three, and `dissolve_and_fade_keep_premultiplied_pixels_valid` was not extended to Wipe and Zoom, which also mix. My probes pass on all of these, so this is a test gap, not a defect.

- **W3. P15 asks for near-end behaviour that Zoom does not have.** class: ambiguity. `specs/changes/layer-2/design.md:380`, ARQ-118. P15 says the maths of each transition reaches the limits, "so a test at `p` near 0 and near 1 stays within 1/255 of the end". ARQ-118 lists only Dissolve, Fade and Wipe with `s = 1`. Measured on a 16x9 black and white `Rgba8` checker: Zoom is 4/255 from its end at `in` `p` 0.0001 and `out` 0.9999, and 2/255 at the other two. Slide at a long side moves whole columns. The implementation follows ARQ-118, the later and narrower text, and its deviation 1 records no Zoom test, though it cites "P15 item 3" instead of step 8. P15 needs one sentence aligned with ARQ-118, or ARQ-118 needs a bound for Zoom.

- **W4. The recorded claim about the end copies is false.** class: implementation. `specs/changes/gafira-raster/apply-progress.md:285`. It says Slide and Zoom depend on the guard's copies, "no mix reaches the exact end", and "the tests fail if they are removed". Mutant I8 removes them for all six, and all 130 tests pass. My ends probe with awkward pixels and tiny softness passes on it too. The mutant is equivalent: at `p` 0 Slide has `d = side`, at 1 `d = 0`; Zoom has scale 1, a tap of weight exactly 1 and the weight-1 shortcut; Wipe saturates. Keep the branches, since P15 step 8 asks that no conversion runs at the ends, but correct the note: RS3a's equivalence extends to all six.

### SUGGESTION

- S1. `samples` in `crates/gafira-raster/src/transitions/zoom.rs:47` allocates a `Vec` per pixel, for a fixed 8. An array removes it. Zoom is 303 ms per 1080p frame, and soft Wipe 87 ms; O5 and XP13 own the cost, so no finding. The Zoom and Wipe figures may be useful to the P15 table.
- S2. `apply-progress.md:285` calls the decode join 0.04045 moved to 0.04 "equivalent". It is not equivalent: the curves differ by up to 4.8e-7 between the two. That is under the test's tolerance, and in the `Encoded` arm, which is not in use. Call it "below tolerance".
- S3. No test names FV-06. Its criterion (a) is ARQ-25's, which is named, and FV-06 is not among the card's product IDs. A comment on the ends test would make `git grep FV-06` trace.
- S4. Still open: `proptest-regressions/` is untracked, still with 15 seeds. The Slice card still says `RS3`. Both are the coordinator's.

## Size

1 540 lines added in `crates/gafira-raster` against `fadac62`, against a forecast of 450 (630). About 1 050 of them are tests. Noted, no finding, per the owner.

next: specifier (W3, ambiguity), then tdd-apply (C1, C2, W1, W2, W4, implementation)
