## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: runs in parallel beside engine-contracts under the owner's parallel full route for layer 2
Product IDs: ARQ-122 (Lab, ΔE2000 and ΔE76 parts only; palette and dHash arrive with `RS6`) (source: specs/02-arquitectura-software.md, row ARQ-122)
Scope: a new `gafira-color` module with the value type `Lab`, conversion from linear sRGB through XYZ D50 with Bradford, `delta_e76` and `delta_e2000` in `f64`, as stated in ARQ-122 and design P6.      Non-goals: a `Color` or `ColorSpaceId` variant, any format change, validation of non-finite input, palette, dHash, reliability (ARQ-131).
Acceptance:
  AC1 Given the 34 pairs of Sharma, Wu and Dalal 2005 table I, When `delta_e2000` runs, Then each matches within ±1e-4 [unit]
  AC2 Given linear sRGB white and black, When converted to Lab, Then they give L 100, a 0, b 0 and 0, 0, 0 within ±1e-3 [unit]
  AC3 Given linear sRGB blue and yellow, When converted to Lab, Then they match the ARQ-122 values within its stated tolerance [unit]
  AC4 Given (50, 0, 0) and (53, 4, 0), When `delta_e76` runs, Then it gives 5 within ±1e-12 [unit]
Plan: design `specs/changes/layer-2/design.md` P6, `crates/gafira-color/src/` only; test first for each AC.
Owner decisions touched: none (checked D-44, which reads the averaging order only, for ARQ-131).

# Apply progress: color-lab

## CL1: Lab, delta_e76, delta_e2000 (tdd-apply)

Files: `crates/gafira-color/src/lab.rs`, `src/lib.rs` (exports), `tests/lab.rs`.

Source of the 34 pairs: the authors' published data file `ciede2000testdata.txt` (Sharma, Wu and Dalal 2005, hajim.rochester.edu/ece/sites/gsharma/ciede2000/dataNprograms/), fetched, 34 rows of `L1 a1 b1 L2 a2 b2 dE`, converted to the Rust array by script; nothing typed by hand.

Bradford D65 to D50 matrix: derived (Bradford cone matrix, chromaticities (0.3127, 0.3290) and (0.3457, 0.3585)) by an independent numpy script; equals the CSS Color 4 matrix. The expected Lab values of the dark and mid-range tests come from the same script (full CSS pipeline in f64), not from the code under test.

Commits: Lab and conversion; `delta_e76`; `delta_e2000` (with the symmetry test).

RED per criterion (stub returning zeros, assertion failures):
- AC2: `arq_122_white_is_l100_and_black_is_zero`: axis 0 `[0,0,0]` vs `[100,0,0]` (tol 1e-3). `arq_122_black_is_zero` passes on the stub (not RED).
- AC3: `arq_122_blue_and_yellow_match_the_published_values`: axis 0 `[0,0,0]` vs `[29.567, 68.298, -112.0294]`.
- AC4: `arq_122_delta_e76_of_the_published_pair_is_five`: got 0. Also `..._uses_all_three_axes_and_the_square_root` failed.
- AC1: `arq_122_delta_e2000_matches_the_34_sharma_pairs`: pair 1: got 0, published 2.0425.
- Not RED (gap-closing, proved by mutants): `arq_122_dark_colors_use_the_linear_branch` and `arq_122_mid_range_color_matches_the_reference_pipeline` (written after the first stub run, would fail on the stub), `arq_122_delta_e2000_is_symmetric`.

Hand mutants (applied, test failed, restored):

| # | Mutant | Killed by |
|---|---|---|
| 1 | D65 white in place of D50 | AC2 white, AC3, dark, mid-range |
| 2 | Bradford step omitted | AC2 white, AC3, dark, mid-range |
| 3 | `t > EPSILON` inverted to `<=` | AC2, AC3, dark, mid-range |
| 3 | threshold 10 x epsilon (wrong side) | AC3 |
| 3 | boundary `>` to `>=` | SURVIVES: equivalent, the piecewise function is continuous at epsilon (both branches give 6/29), so no input separates them beyond 1 ulp; not testable without aiming at the mutant |
| 4 | `dh > 180` wrap removed | Sharma pair 16, symmetry |
| 4 | `dh < -180` wrap removed | symmetry only (the published pairs are one-directional) |
| 4 | mean hue not adjusted (either branch) | Sharma pair 16 / pair 11 |
| 5 | `G` = 0 | Sharma pair 1 |
| 5 | `T` coefficient 0.17 to 0.18 | Sharma pair 1 |
| 5 | `R_T` sign flipped | Sharma pair 1 |
| 6 | `C' = 0` gating of `dh` removed (`chromatic = true`) | SURVIVES: equivalent, `dh` and the mean hue only reach the result through `dH' = 2 sqrt(C1' C2') sin(dh/2)`, which is 0 when `C' = 0`; pairs 7 and 8 pass either way |
| 7 | asymmetric mean lightness (`a.l`) | Sharma pair 17, symmetry |
| 8 | drop b, a or L axis; drop sqrt | `..._uses_all_three_axes_and_the_square_root`; AC4 for L, a, sqrt |

Gates (exit codes, each alone): `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0, on each of the three code commits.

Measured: `git diff --shortstat develop...HEAD` before this notes commit: 4 files changed, 335 insertions.

Deviations: none from the Slice card. Mutants 3 (boundary) and 6 are equivalent, reported above.

## Fix round 1 (verify round 1: C1, C2, C3, C4, W5 reason)

Tests only; production is unchanged. These tests close gaps and are not RED: each passes on the existing code, and each was proved by applying the surviving mutants of the verify report, seeing a test fail, and restoring.

Correction of the mutant 4 line above ("the published pairs are one-directional"): pairs 11 and 12 do reach `dh < -180`, but they have `dC` about 0 and the sign of `dH'` reaches the result only through `R_T * tc * th`, so there is no effect. Only pairs 16, 17 and 19, reversed, have a visible effect, and they are now checked against the published values.

| Id | Mutant | Killed by |
|---|---|---|
| W5 | `dh < -180` to `< -170` | `arq_122_hue_difference_wraps_above_minus_180`, `..._of_exactly_180_is_not_wrapped` |
| W7 | `dh > 180` to `> 170` | `arq_122_hue_difference_wraps_below_180`, `..._of_exactly_180_is_not_wrapped` |
| W12 | both thresholds 170 | the three hue-difference tests |
| W3 | `<` to `<=` at -180 | `arq_122_hue_difference_of_exactly_180_is_not_wrapped` |
| W2 | `>` to `>=` at 180 | `arq_122_hue_difference_of_exactly_180_is_not_wrapped` |
| H7 | `h < 0` to `<= 0` | `arq_122_hue_of_the_a_axis_is_0_or_180` |
| H2 | `h1 + h2 < 360` to `<=` | `arq_122_mean_hue_adjusts_for_a_sum_below_360` |
| A1 | rgb multiplied by alpha | `arq_122_alpha_is_ignored` (alpha 0, 0.25, 0.5, 0.999) |
| A9 | `f` narrowed to `f32` | `arq_122_mid_range_color_matches_the_reference_pipeline` (now 1e-9) |
| A2 | `EPSILON` = 0.008856 | `arq_122_epsilon_is_exactly_216_over_24389` (grey 0.008856226 (f32), L 7.99979592708, 1e-10) |
| S2 | each wrap removed, symmetry aside | `arq_122_delta_e2000_matches_the_reversed_sharma_pairs` (dh < -180 arm), the 34-pair test (dh > 180 arm) |

Expected values: from the verify report (two independent implementations, 7 decimals; tolerance 1e-6); the mid-range and grey Lab were also recomputed in f64 from the crate's matrices (12 digits).

Not done: W1 (`.cargo/mutants.toml`) is follow-up for T4 at close. S2 of the report (`hue_degrees` doc) touches production and is left out.
