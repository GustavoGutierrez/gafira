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

---

# Verify report: color-lab, delivery CL1, round 2 (closure)

Commit verified: `b43f439` (`feat/color-lab`, base `fdccea2`). Depth V2, mode full, tier T4, `mutation: archive` (no `mutants` report before close, not a finding).

**Verdict: PASS.** C1 to C4 are closed: every surviving mutant of round 1 now fails a test. T4 at close is still owed.

CRITICAL 0, WARNING 2 (both follow-up), SUGGESTION 3. Classes: implementation 2.

## Gates (each exit code read alone, on a clean export of `b43f439`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1180 passed, 0 failed) |
| `cargo deny check` | 0 |
| CI policy steps, one by one (ARQ-07, forbid unsafe, V5, V12, V13, V14) | 0 each |

These ran on Linux only. The Windows and macOS jobs of the CI matrix did not run (see WF2).

## Artifact integrity (steps 3, 3c)

- Slice card: `6202244` touches only `apply-progress.md`. From there to HEAD the file has 0 deleted lines, so no implementer commit changed the card.
- `ea62347` touches only `crates/gafira-color/tests/lab.rs`. Production code is byte-identical to round 1.
- `.cargo/mutants.toml` is unchanged, so there are no new exclusions to re-check.
- `ea62347` replaced one existing expectation: the mid-range test went from tolerance 1e-5 to 1e-9 with more digits, which tightens it.
- Criteria: 7 of 7 covered. P6 alpha and the ARQ-122 exact ε and `f64` rule, missing or partial in round 1, are now covered. Left out as the card says: palette and dHash (`RS6`).
- Size: 436 lines under `crates/` (641 with docs), under the ~700 ceiling.

## Closure of findings (mutants reapplied, scratch export, full `tests/lab.rs` run for each)

No-op control: 17 passed, exit 0.

| Finding | Mutant | Result | Killed by |
|---|---|---|---|
| C1 | W5 `< -170` | killed | the three hue-difference tests |
| C1 | W7 `> 170` | killed | the three hue-difference tests |
| C1 | W12 both 170 | killed | the three hue-difference tests |
| C1 | W3 `<=` at -180 | killed | `arq_122_hue_difference_of_exactly_180_is_not_wrapped` |
| C1 | W2 `>=` at 180 | killed | `arq_122_hue_difference_of_exactly_180_is_not_wrapped` |
| C1 | S2 `dh < -180` arm removed, symmetry test skipped | killed | `arq_122_delta_e2000_matches_the_reversed_sharma_pairs` |
| C1 | S2 `dh > 180` arm removed, symmetry test skipped | killed | `arq_122_delta_e2000_matches_the_34_sharma_pairs` |
| C2 | A1 rgb multiplied by alpha | killed | `arq_122_alpha_is_ignored` |
| C3 | H7 `h <= 0` | killed | `arq_122_hue_of_the_a_axis_is_0_or_180` |
| C3 | H2 `h1 + h2 <= 360` | killed | `arq_122_mean_hue_adjusts_for_a_sum_below_360` |
| C4 | A9 `f` input narrowed to `f32` | killed | `arq_122_mid_range_color_matches_the_reference_pipeline` |
| C4 | A9 variant, only `cbrt` in `f32` | killed | `arq_122_mid_range_color_matches_the_reference_pipeline` |
| C4 | A2 `EPSILON` = 0.008856 | killed | `arq_122_epsilon_is_exactly_216_over_24389` |

13 of 13 killed. The killers match the table in `apply-progress.md`, except that W5 and W7 also fail one more hue-difference test than it records. These are reapplied round 1 survivors, so no independent-mutant budget was spent.

Closed: C1, C2, C3, C4.

## Limited probe: are the exact-boundary cases really exact?

On this Linux build, the exact ±180 cases give `dh` of exactly -180.0 and 180.0, and the sum-360 case gives `h1 + h2 - 360` of exactly 0. So W2, W3 and H2 are killed for the stated reason.

Each hue in the ±180 cases and in pair 14 comes from a non-trivial `atan2`, which Rust takes from the system libm. A 1 ulp change to `atan2` (scratch export only) has these effects:

| Change | Result |
|---|---|
| all results down 1 ulp | exact-180 test fails, got 79.98, expected 86.67 |
| positive results below 3.0 down 1 ulp | published pair 14 fails, got 4.7461, published 4.8045 |
| all results up 1 ulp | a-axis test fails, but it perturbs `atan2(0, -x)`, whose value is fixed at pi, so it is artificial |
| positive up, negative up, negative down | all pass |

This is the known discontinuity of CIEDE2000 at 180 degrees, not a code defect. No real libm difference was observed: the only installed target is `x86_64-unknown-linux-gnu`, so no other libm was tried.

## CRITICAL

None open.

## WARNING

- **W1** class: implementation, `follow-up`, open, unchanged from round 1. `.cargo/mutants.toml`. T4 at close will report the verified equivalents on critical code, and each needs an exclusion with its reason.
- **WF2** class: implementation, `follow-up`, new. `crates/gafira-color/tests/lab.rs`, exact-180 test and Sharma pair 14. These cases depend on the exact bits of the system `atan2`, and 1 ulp flips the result by up to 6.7 dE. Pair 14 was already exposed before the fix. Whether they pass on Windows and macOS is unproven. Read those two CI jobs on the first run. If one fails, the expected value needs a platform-safe case, not a change to the code.

## SUGGESTION

- S2 of round 1, still open. The `hue_degrees` doc says `[0, 360)`, which is not exact for a tiny negative hue.
- S4. `tests/lab.rs`, comment on the reversed pairs says they reach "the +180 wrap". They reach the `dh < -180` arm, as `apply-progress.md` says and the S2 mutant confirms.
- S5. `tests/lab.rs`, comment on `arq_122_mean_hue_adjusts_for_a_sum_below_360` says the sum is "just under 360", and the test name says below. The sum is exactly 360, which is why it kills H2.

S1 is done (correction recorded in `apply-progress.md`). S3 is done (one named test per rule).

Checks that could not run: the Windows and macOS CI jobs (not a gate of this depth, see WF2). Per-commit gates are V3 and were not run.

Owed at close: T4 `mutants` on `gafira-color` before archive, with `W1` settled first, then a closure verify only if T4 leaves a critical survivor.

next: archive

---

# Verify report: color-lab, delivery CL1, round 3 (closure, T4 fix round)

Commit verified: `852b92f` (`feat/color-lab-t4`, base `978bbcb`). Depth V2, mode full, tier T4, `mutation: archive`.

**Verdict: PASS.** T4 at the fix commit leaves 0 critical missed. The four timeouts are not open: on re-run each is caught by a test assertion. All 7 new exclusions are equivalent and match exactly the 7 intended mutants.

CRITICAL 0, WARNING 0, SUGGESTION 5. Classes: none open.

## Gates (each exit code read alone, on a clean export of `852b92f`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1188 passed, 0 failed) |
| `cargo deny check` | 0 |
| CI policy steps, one by one (ARQ-07, forbid unsafe, V5, V12, V13, V14) | 0 each |

V13 and V14 call `git`, so they exited 1 on the plain export ("not a git repository"). That was the harness, not the code. All six were run again on a git clone at `852b92f`, and those exits are the ones in the table.

These ran on Linux only. For the other platforms see WF2 below.

## Artifact integrity (steps 3, 3c)

- Slice card: from `6202244` to HEAD, `apply-progress.md` has 77 added and 0 deleted lines, so the card is unchanged.
- `bbd911d` touches only `crates/gafira-color/tests/lab.rs`. `852b92f` touches `.cargo/mutants.toml` and `apply-progress.md`. `crates/gafira-color/src` is unchanged against `978bbcb`.
- Criteria: 7 of 7 covered, as in round 2. `ARQ-122` is in the tests. Left out as the card says: palette and dHash (`RS6`).
- Size: 446 lines under `crates/` (29 in this round), under the ~700 ceiling.

## T4 (step 4)

The report is valid for this commit. The run started at 22:03:57, after the commit at 22:03:35, from this worktree. It found 415 - 7 = 408 mutants, and its log has the new test failing, so it contains the fix.

| Run | Found | Caught | Missed | Timeout | Unviable | Wall |
|---|---|---|---|---|---|---|
| First T4, `978bbcb` | 415 | 387 | 8 | 0 | 20 | 3 min |
| T4, `852b92f`, 8 jobs, limit 20 s | 408 | 384 | 0 | 4 | 20 | 384 s |
| Re-run, lines 128 and 129, 4 jobs, limit 300 s | 9 | 9 | 0 | 0 | 0 | 19 s |

Critical missed: 0.

## The four timeouts: not open

| Mutant | T4 | Re-run | Test binary in the re-run |
|---|---|---|---|
| `lab.rs:128:13` `+` to `*` | timeout 20 s | caught, exit 101 | 5 tests fail |
| `lab.rs:129:23` `<` to `==` | timeout 20 s | caught, exit 101 | 4 tests fail |
| `lab.rs:129:23` `<` to `>` | timeout 20 s | caught, exit 101 | 3 tests fail |
| `lab.rs:129:23` `<` to `<=` | timeout 20 s | caught, exit 101 | 1 test fails, `arq_122_mean_hue_adjusts_for_a_sum_below_360` |

**Applying "Hanging mutants": the exception does not apply, and it is not needed.**

- They are not hanging. The rule needs a re-run that confirms the mutant does not terminate. Here each terminates, in 0.05 to 0.15 s of test phase. So they do not count as "detected" through that exception.
- The timeouts measured no test at all. All four T4 logs end in `Blocking waiting for file lock on package cache`, with 0 `running` lines and 0 `test result` lines. No test binary started in those 20 s. So the cause is contention, not the mutant.
- The stall was machine-wide. A fifth mutant, `128:13` `+` to `-`, spent 19.43 s in the same window before failing on assertions, and two neighbours took 43.85 s to build (usual build 3 to 15 s). 350 logs of the run show the same lock wait.
- Detection is proved directly. The re-run has the same 4 mutants, on the same tree, each killed by assertions. The limit was longer and the load lower. No source changed between the two runs.
- Independent check. I applied the 4 mutants by hand on a scratch export, with no contention: same failing tests, test binary 0.00 s, exit 101.

**Ruling.** `AGENTS.md` says a timeout that load explains stays a finding, and step 4 says it stays CRITICAL. I read that as guarding against counting an unproven timeout as detected. These four are not unproven: for them, the newest `mutants` report shows caught, not timeout. So they are recorded as finding TO1, closed, and not CRITICAL. The rule does not name this case in so many words, which is why S7 asks for it to be stated.

## Closure of findings

| Finding | Result | Evidence |
|---|---|---|
| M1 `lab.rs:129:18` `+` to `*`, real test gap | closed | Caught in T4 and in the re-run. Only `arq_122_mean_hue_adjusts_for_a_small_hue_and_a_large_hue` fails, confirmed by hand as well |
| W1 exclusions | closed | See below |
| WF2 atan2 bits on other platforms | closed | CI run 41380387263 at `978bbcb`, which holds the exact-180 test and pair 14: `policy`, Linux, `aarch64-apple-darwin` and `x86_64-pc-windows-msvc` all success |
| TO1 four timeouts | closed | See above |

Scope of WF2: the CI run shows these cases pass on those targets. The sensitivity is still there and could return with a new libm. CI has not run the new test, but its hues differ by 240 degrees, far from the discontinuity.

The new test is sound. Hues are 9.95 and 249.92, sum 259.87, difference 239.97. The right mean is 309.93, and the mutant gives -50.07. `T` has period 360, so the mutant only shows through `dtheta`, which is not periodic: 44.298 against 43.709. Expected 43.70902302239777 equals scikit-image 0.25.2 and my own implementation from the paper, in both orders. Tolerance is 1e-9.

## Exclusions (6 new entries, 7 mutants)

Each regex was run against the mutant list of the first T4: it removes exactly the 7 intended mutants, adds none, and nothing else in `lab.rs` matches. No other file is mutated under the name `lab.rs` (`tests/lab.rs` is not mutated). Neighbours stay tested, for example `118:16` `+=` to `*=`.

Each mutant was run against the original on 6,000,000 inputs: random, exact and tiny zero chroma, tiny against huge chroma (down to `5e-324` and up to `1e300`, where a quotient underflows), near-opposite hues, and near-equal colors across hue 0.

| Mutant | Reason in the file | Result | Worst difference | Bit differences |
|---|---|---|---|---|
| `41:10` `>` to `>=` | same f64 at `EPSILON` | equivalent | n/a | `cbrt` and the linear arm have the same bits at `EPSILON` |
| `111:24` `*` to `+` | `th` is 0 | equivalent | 0 | 0 |
| `111:24` `*` to `/` | `th` is 0 | equivalent | 0 | 0 |
| `116:16` `-=` to `+=` | full sine period | equivalent, rounding only | 8.5e-14 | yes |
| `118:16` `+=` to `-=` | full sine period | equivalent, rounding only | 9.2e-14 | yes |
| `126:12` `+` to `-` | `th` is 0 | equivalent | 0 | 0 |
| `126:12` `+` to `*` | `th` is 0 | equivalent | 0 | 0 |

- `dh` is used only in the sine, so 116 and 118 cannot reach anything else. The worst relative difference is 3.1% on a result of about 3e-12, where the value is itself rounding noise. No test could assert on that without aiming at the mutant.
- `th = 0` also needs `S_H > 0`, or it becomes `0/0`. Checked: `T` is periodic with a minimum of 0.362, so `S_H >= 1`. The file does not state this (S6).
- `/` is the only subtle case: `0/0` gives NaN, which is `!= 0`, and `c1/c2` can underflow to 0 while `c1*c2` is not 0. Neither changes the result: 0 differences, 0 NaN differences.
- All reasons are present. Three have wording issues, listed in S6. None changes the outcome.

## CRITICAL

None open.

## WARNING

None open.

## SUGGESTION

- S2 of round 1, still open. The `hue_degrees` doc says `[0, 360)`.
- S4 of round 2, still open. Comment on the reversed pairs says "the +180 wrap".
- S5 of round 2, still open. Comment says "just under 360", and the sum is exactly 360.
- S6. `.cargo/mutants.toml`. The header says each regex matches exactly one mutant, but three new entries match two, as `align.rs` already did. The `116|118` reason says the wraps differ by 720 degrees "in the sine argument": `dh` differs by 720, its half by 360. The `111:24` reason ends in "the same cases that matter", which is loose, and `S_H > 0` is not stated.
- S7. `AGENTS.md` "Hanging mutants" and step 4 of `verify` do not name a timeout that a re-run catches on assertions; state it. Also, a baseline of 0.15 s gives an auto limit of 20 s, which a package-cache lock can use up at 8 jobs before any test runs; a higher floor (`--minimum-test-timeout`) would avoid these re-runs. Both are owner or coordinator decisions.

Checks that could not run: none. Per-commit gates are V3 and were not run. No independent-mutant budget was spent: 5 mutants were applied by hand (M1 and the 4 timeouts), and the 7 exclusions were probed by differential.

next: archive
