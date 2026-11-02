# Verify report: color-lab, delivery CL1, round 1

Commit verified: `1d6eab3` (`feat/color-lab`, base `fdccea2`). Depth V2, mode full, tier T4, `mutation: archive` (no `mutants` report before close, not a finding).

**Verdict: FAIL.** The code is correct on every probe. The tests do not pin it: 10 non-equivalent hand mutants survive all nine tests, in critical code.

CRITICAL 4, WARNING 1, SUGGESTION 3. Classes: implementation 5.

## Gates (each exit code read alone, on a clean export of HEAD)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `cargo deny check` | 0 |
| CI policy steps, one by one (ARQ-07, forbid unsafe, V5, V12, V13, V14) | 0 each |

## Independent probes (step 1c, before reading the tests)

- Published source: `ciede2000testdata.txt` and `ciede2000noteCRNA.pdf` (the paper with table I) and `CIEDE2000.xls`, fetched from the authors' site. The 34 rows of the text file equal table I of the PDF, parsed separately: 0 mismatches.
- The 34 pairs in `tests/lab.rs` equal table I of the PDF: 238 numbers, 0 mismatches. The implementer's script was not used.
- `delta_e2000` on the 34 pairs, both directions: all round to the published 4 decimals, worst error 4.95e-5.
- Own CIEDE2000, written from the paper's equations, reproduces the published dE and the table's intermediates. Crate against it on 320,034 pairs (random, near-opposite hues, zero axes): worst 5.7e-14. Crate against scikit-image: worst 2.3e-13. Asymmetry: 0.
- Lab: 50,012 inputs against the CSS Color 4 §19 sample code taken from the spec: worst 2.8e-13. Blue and yellow match §9.1 within 0.011, inside ±0.02. White and black match AC2.
- Bradford matrix re-derived from the cone matrix: 2.2e-16 from the CSS matrix, which the code matches to rounding.
- `delta_e76` against the Euclidean distance: error 0.
- The expected values in the dark and mid-range tests match my reference.
- Table I prints h'2 of pairs 21 and 23 about 1e-3 from recomputation. dE is unaffected, and no test uses these values.

## Criteria (step 3, 3c)

| Criterion | Status | Proof |
|---|---|---|
| AC1 34 Sharma pairs ±1e-4 | covered | `arq_122_delta_e2000_matches_the_34_sharma_pairs` |
| AC2 white, black | covered | `arq_122_white_is_l100_and_black_is_zero`, `arq_122_black_is_zero` |
| AC3 blue, yellow ±0.02 | covered | `arq_122_blue_and_yellow_match_the_published_values` |
| AC4 ΔE76 is 5 ±1e-12 | covered | `arq_122_delta_e76_of_the_published_pair_is_five` |
| P6 signatures and exports | covered | API compiled from the tests; `lib.rs` export |
| P6 alpha is ignored | missing | C2 |
| ARQ-122 exact ε, P6 `f64` | partial | C4 |

Slice card: commit `6202244` touches only `apply-progress.md`, and it precedes the first code commit. No implementer commit changed the card: `1d6eab3` only appends below it, with 0 deleted lines. `git grep ARQ-122` finds the ID in the tests. Covered: ARQ-122 Lab, ΔE2000 and ΔE76 parts. Left out as the card says: palette and dHash (`RS6`).

Size: 335 lines before the notes commit, forecast 280 (390). The change folder has no delta spec: the coordinator named the Slice card, ARQ-122 and P6 as the acceptance source.

## Hand mutants (step 4b, scratch export)

54 applied, 38 killed, 6 equivalent, 10 survivors. A no-op control also passed, as it should, and is not counted.

Spot-check: "`R_T` sign flipped" is killed by Sharma pair 1, as recorded. "`dh < -180` wrap removed" is killed only by the symmetry test, as recorded.

Killed (38): every constant of `T`, `dtheta`, `S_L`, `S_C`, `S_H`, `R_C`, `G`, `25^7`; `R_C` from C-bar instead of C-bar-prime; `dtheta` in radians; hue from raw `a`; `<=` to `<` at mean-hue |h1-h2|; swapped +360 and -360 arms; `&&` to `||` in `hue_degrees`; KAPPA 903.3; D50 table white; Lindbloom Bradford row; `L` offset; sign of `a`; matrix order; both wraps removed; thresholds 190 and 270.

Survivors, all killed by a direct case with values from two implementations (own reference and scikit-image):

| Id | Mutant | Worst error | Killed by direct case |
|---|---|---|---|
| W5 | `dh < -180` to `< -170` | 36.6 dE | `(50, 7, -0.2)`, `(50, -58.6, -6.7)` = 50.2262991 |
| W7 | `dh > 180` to `> 170` | 36.6 dE | `(50, -57.4, -8.4)`, `(50, 7, -0.1)` = 50.3876197 |
| W12 | both thresholds 170 | same | both cases above |
| W3 | `<` to `<=` at -180 | 28.6 dE | `(50, 38.5, -0.5)`, `(50, -115.5, 1.5)` = 86.6728119, exact dh -180 |
| W2 | `>` to `>=` at 180 | 28.6 dE | `(50, -38.5, 0.5)`, `(50, 115.5, -1.5)` = 58.0350607, exact dh 180 |
| H7 | `h < 0` to `<= 0` in `hue_degrees` | 15.7 dE | `(50, 10, 0)`, `(50, -20, 0)` = 34.2689360 |
| H2 | `h1 + h2 < 360` to `<=` | 2.0e-4 dE | `(50, 26, 39.5)`, `(50, 104, -158)` = 52.3456095 |
| A1 | `r`, `g`, `b` multiplied by alpha | L 100 | same color at alpha 0, 0.5 and 1 |
| A9 | `f` narrowed to `f32` | 2.9e-5 Lab | mid-range at tolerance 1e-9 |
| A2 | `EPSILON` = 0.008856 | 1.7e-9 L | grey 0.00885622576, L 7.99979592708 ±1e-10 |

Equivalents, each checked independently:

- E1 `t >= EPSILON` (recorded): equivalent. At `t = EPSILON` both branches give the same f64, difference 0, stronger than the recorded "1 ulp".
- E2 `chromatic = true` (recorded): equivalent. With `C' = 0`, `dH' = 0`, so `th = 0`, and both `S_H` and `R_T` reach the result only through `th`. The recorded reason names `dH'` only, but holds. 0 difference on 320,034 pairs, 20,000 of them with zero axes.
- W9, W10 `+=` and `-=` swapped in the wraps: equivalent, `dh` moves 720 degrees, so `sin(dh/2)` keeps its value. Worst 8.5e-14, rounding.
- H4 achromatic arm of `h_bar` halved: equivalent, same `th = 0` argument.
- H5 guard of `hue_degrees` removed: equivalent, it differs only for a signed zero at `C' = 0`.

### The `dh < -180` wrap: direct cases are needed

The recorded survivor line is right, but its reason is not. Pairs 11 and 12 do reach `dh < -180` forward (-180.0015, -180.0031). The sign of `dH'` reaches the result only through `R_T * tc * th`, and those pairs have dC about 0, so the effect is 0. Only pairs 16, 17 and 19 have a visible effect, and all three go the other way (`dh > 180`).

- With the symmetry test skipped, removing the wrap (S2) passes every other test. Reversed pairs 16, 17 and 19 against their published values kill it directly, with no reference to the forward result.
- Symmetry cannot see the wrap limit. W5, W7 and W12 keep or do not reach the asymmetry on any of the 34 pairs (worst 4.7e-11, under its 1e-10 tolerance), yet the error reaches 36.6 dE.
- W2 and W3 need an exact ±180 pair with dC not zero. Sharma pairs 10 and 14 have dC about 0.

## CRITICAL

- **C1** class: implementation. `crates/gafira-color/src/lab.rs:115-117`, `tests/lab.rs:160`. The limit of the hue wrap is unpinned: W2, W3, W5, W7 and W12 survive, error up to 36.6 dE. Add forward cases on both sides of ±180 and the exact ±180 cases, plus reversed pairs 16, 17 and 19 against the published values.
- **C2** class: implementation. P6 "Alpha is ignored", `lab.rs:69`, `tests/lab.rs:6`. Every test uses alpha 1, so A1 survives. Add the same color at other alphas.
- **C3** class: implementation. `lab.rs:95`, `lab.rs:129`. Hue boundaries unpinned: H7 (15.7 dE on opposite colors on the `a` axis) and H2 survive. cargo-mutants generates both, so T4 would report them.
- **C4** class: implementation. `tests/lab.rs:46-61`, `lab.rs:35`. Tolerances of 1e-5 do not pin the `f64` rule of P6 or the exact ε of ARQ-122: A9 and A2 survive. Tighten the mid-range test to 1e-9, with more digits, and add a grey inside (0.008856, 216/24389).

## WARNING

- **W1** class: implementation, `follow-up`. `.cargo/mutants.toml`. T4 at close will report the equivalents above on critical code (at least E1, W9, W10; H4 and H5 if their forms appear). Each needs an exclusion with its reason before archive, and those lines must be the ones after the C1 and C3 fixes.

## SUGGESTION

- S1. In `apply-progress.md`, replace "the published pairs are one-directional" with the reason above.
- S2. In the `hue_degrees` doc, `[0, 360)` is not exact: for a tiny negative hue `h + 360` rounds to 360.0. No effect on results was found.
- S3. Run the direct cases of the table through a single named test per rule, so a future failure names the rule.

Checks that could not run: none. Per-commit gates are V3 and were not run.

next: tdd-apply
