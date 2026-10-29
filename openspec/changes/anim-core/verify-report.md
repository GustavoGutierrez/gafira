# Verify report: anim-core D3b

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.
D2a: PASS in round 1 at `d9d2e50` (0 CRITICAL, 2 WARNING ambiguity: `is_finite` missing from the §3.4 snippet and the f32/Vec2 overflow domain, both routed to specifier, 4 SUGGESTION); merged into `develop` at `37769a0`.
D2b: PASS in round 1 at `f30293b` (0 CRITICAL, 1 WARNING ambiguity: ARQ-12 golden assigned to no delivery, since assigned to D3c, 4 SUGGESTION); merged into `develop` at `c048b97`.
D3a: PASS in round 1 at `1dfa4a2` (0 CRITICAL, 1 WARNING W1 Display-P3 hue test, 3 SUGGESTION); merged at `4e1089e`, then RED on `develop` (flaky Color reference property).
D3a fix round: PASS in round 2 at `8a9d36e` (0 CRITICAL, 1 WARNING ambiguity: ARQ-19 Color reference divisor not stated, 2 SUGGESTION; 5.4M property cases, 0 failures); merged into `develop` at `6710050`.
D3b round 1: FAIL at `33417d3` (1 CRITICAL C1 implementation: no opposite-sign determinant test, sign-on-`sx` mutant survived; 1 WARNING W1 ambiguity: "determinante 0" undefined in floats, settled as Q-33 B; 3 SUGGESTION S1-S3).
D3b round 2: FAIL at `33f1a4c` (1 CRITICAL C2 implementation: absolute-threshold mutant survived; 2 WARNING: W2 implementation 1e3 slack, W3 ambiguity unobservable `scale(1e-9)` example, settled in `6b37f9e`; 3 SUGGESTION S4-S6).

Flow: standard | mutation: archive | tier: T4 | verify: V2 (on the round-2 findings and the `6b37f9e` spec change). No cargo-mutants report (expected with `mutation: archive`; not a finding). Hand mutants only.
Acceptance source: Slice card D3b row (`apply-progress.md`), 02 §3.4 Transform2D rows ("Prueba de degeneración", Q-33), ARQ-19 Transform2D criteria as amended by `6b37f9e` (two observable threshold examples, tolerance `|got − want| ≤ 1e-9·max(1, |want|)`), and the round-2 findings.
Scope: `6710050..e314c4f` on `feat/anim-core-d3b`; new in this round: `6b37f9e` (spec, `docs/specs/02-arquitectura-software.md` only, 1 line), `e314c4f` (`src/interpolate.rs` +2 comment lines, `tests/transform.rs` +24/-22, `apply-progress.md`).

## Round 3 of D3b (commit `e314c4f`)

### Verdict: FAIL

Counts: 2 CRITICAL, 0 WARNING, 2 SUGGESTION. Per class: implementation 2 (C3, C4), ambiguity 0, architecture 0, claim 0.

### Method

- Disk free before building: 66 GB. Two `git archive` exports of HEAD in the scratchpad (`head`: gates, CI steps, seed runs; `mut`: probes and hand mutants), files touched after export, one `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`; both freed with `cargo clean --target-dir` (1.8 GiB, 358 MiB).
- Probes written from the amended ARQ-19 / §3.4 text before reading the new implementer tests (`probe_d3b3.rs`, 6 tests, spec tolerance): the two new Q-33 examples in both directions where meaningful, the exact threshold boundary (`[1, 0, 1, 1e-12]`: `|det| = 1e-12 = 1e-12·|c1|·|c2|` exactly in f64, so degenerate by `≤`), unequal column lengths (`|c1| = 1e6`, `|c2| = 1e-6`, sine 1e-3: not degenerate), and extrapolation (`t` = -0.5, 1.5, 2) with one and with two degenerate ends (§3.4 "se extrapola"). Plus the 11 round-2 probes. All 17 pass on the clean code.

### Gates (clean export of `e314c4f`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (`tests/transform.rs` 29 passed) |
| `cargo deny check bans licenses sources` | 0 |
| CI: Domain crates forbid unsafe code | 0 |
| CI: V5 layer table | 0 |
| CI: V12 clippy.toml copies | 0 |

`.github/workflows/ci.yml`, `.cargo/`, `deny.toml` and `Cargo.lock` are unchanged in `6710050..HEAD`; the step scripts are the round-1 extracts.

### Card, commit and size checks

- Slice card unchanged. `e314c4f` edits in `apply-progress.md` only the implementer's own round-2 note (the S6 correction, one line) and appends `## D3b round 3`.
- `6b37f9e` touches only `docs/specs/02-arquitectura-software.md`; no implementer commit touches `docs/specs`.
- Size: crates `6710050..HEAD` = 132 + 520 insertions, 1 deletion (652 lines, ceiling ~700). OK.
- Hygiene: commits conventional, author Gustavo Gutierrez, no Co-Authored-By or AI attribution. No `transform.proptest-regressions` in the tree (the tracked `color`, `easing`, `interpolate`, `sampling` files are unchanged in the range); the one my H3 mutant run wrote stayed in the scratch export.

### Round-2 findings

| Finding | Status | Evidence |
|---|---|---|
| C2 absolute threshold | closed | `arq_19_q33_large_scale_with_tiny_column_sine_is_degenerate` (`[1e6, 0, 2e6·cos 1e-13, 2e6·sin 1e-13]` -> `[500000.5, 0, 0, (1 + 2e-7)/2]`) and `arq_19_q33_tiny_rotated_scale_keeps_its_own_rotation` (`1e-7·rotation(0.3)` to `rotation(2.0)` -> `0.50000005·rotation(1.15)`, `rotation(θ) = [cos, sin, −sin, cos]`): numbers, zero translation and `t = 0.5` match the amended ARQ-19 exactly. M3 re-run: killed by exactly these two implementer tests (and 4 probes). |
| W2 1e3 slack | closed | `transform.rs` opposite-sign property now uses `1e-9·max(1, |w|)` with no factor. |
| W3 unobservable example | closed (spec `6b37f9e`) | `scale(1e-9)` example replaced by the two observable ones; the old test was removed. |
| S4 tolerance wording | closed (spec `6b37f9e`) | ARQ-19 states `|obtenido − esperado| ≤ 1e-9·max(1, |esperado|)`; `assert_rel` removed, every Q-33 test (rank-1, exact zero, non-finite det, both new examples) uses `assert_abs_rel`, which is that formula. |
| S5 redundant clause | closed | comment above the guard in `Parts::decompose` (comment only, no logic change). |
| S6 progress note | closed | round-2 RED line now says the `scale(1e-9)` test did not guard against an absolute threshold. |

### Q-33 criteria (amended ARQ-19)

| # | Criterion | Implementer test | Status |
|---|---|---|---|
| Q1 | `[0.1, 0.3, 0.7, 2.1]` -> `[0.55, 0, 0, 1.55]` | `arq_19_q33_rank_one_end_with_rounding_determinant_is_degenerate` | covered |
| Q2 | `[1.1, 2.2, 3.3, 6.6]` -> `[1.05, 0, 0, 3.8]` | same test | covered |
| Q3 | `[1e200, 1e200, 1e200, 2e200]` -> `[5e199, 0, 0, 1e200]`, finite | `arq_19_q33_finite_coefficients_with_non_finite_determinant_are_degenerate` | covered |
| Q4a | sine 1e-13 at scale 1e6 degenerate | `arq_19_q33_large_scale_with_tiny_column_sine_is_degenerate` | covered |
| Q4b | `1e-7·rotation(0.3)` not degenerate | `arq_19_q33_tiny_rotated_scale_keeps_its_own_rotation` | covered |
| Q5 | Rule `|det| ≤ 1e-12·|c1|·|c2|` (§3.4) | the above plus `arq_19_q33_threshold_is_relative_to_the_column_lengths` | **weak: C3, C4** |

Total for D3b: 18 (round 1) + Q1-Q3, Q4a, Q4b, Q5 = 24 criteria, 23 covered, 1 weak (Q5).

### Hand mutants (budget: 1 spot-check + 3 own, scratch export `mut`, `src/interpolate.rs`, source restored and compared after the runs)

| # | Mutant | Implementer suite (all gafira-anim targets) | Probes |
|---|---|---|---|
| M3 (spot-check of the implementer list) | absolute threshold `|det| <= 1e-12` | killed: `arq_19_q33_large_scale_with_tiny_column_sine_is_degenerate`, `arq_19_q33_tiny_rotated_scale_keeps_its_own_rotation` (exactly the two listed) | killed: 4 |
| H1 | threshold `<` instead of `<=` | **survives (all pass)** | killed: `r3_threshold_boundary_is_degenerate` |
| H2 | wrong column: `area = |c1|·|c1|` instead of `|c1|·|c2|` | **survives (all pass)** | killed: `r3_unequal_columns_not_degenerate` |
| H3 | no extrapolation on the two-degenerate-ends blend (`t` clamped to [0, 1]) | killed: `arq_19_transform_both_degenerate_interpolate_the_six_coefficients`, `arq_19_transform_degenerate_pairs_are_the_linear_blend` | killed: `r3_two_degenerate_ends_extrapolate` |

Implementer's list: accurate as far as checked (M3 killed by exactly the two tests it names; the other four were run and killed in round 2 by a superset of the tests now listed). It covers the guard's terms and borrowing, but not the comparison operator or which column enters `|c1|·|c2|`; both survive. Shortest arc on the degenerate path was considered and not run: `borrowing` copies the other end's `theta`, so `dtheta = 0` there and any wrap mutant is equivalent on that path (the general wrap is covered by `arq_19_transform_rotation_takes_the_short_way` and the short-arc property).

H1 is not equivalent: `[1, 0, 1, 1e-12]` (zero translation) sits exactly on the threshold in f64 (`det = 1e-12`, `|c1|·|c2| = 1·hypot(1, 1e-12) = 1`) and must be degenerate (towards the identity at t = 0.5: `[1, 0, 0, (1 + 1e-12)/2]`); under `<` it is decomposed with shear `k = 1e12`. H2 is not equivalent: `[1e6, 0, 1e-6·√(1 − 1e-6), 1e-6·1e-3]` (sine 1e-3, `|c1|·|c2| = 1`) must be decomposed (to the identity at t = 0.5: `sx = 500000.5`, `k ≈ 499.99975`); under H2 the threshold is `1e-12·1e12 = 1 > 1e-3` and the end borrows the identity frame. The two spec examples have `|c2| = 2|c1|` and `|c1| = |c2|`, and the column-sine test uses unit columns, so no implementer test separates `|c1|·|c2|` from `|c1|²` (or `|c2|²`, by symmetry).

### Flake evidence (proptest, `PROPTEST_RNG_SEED`, outside the tree)

- HEAD `tests/transform.rs` binary (clean export, run from a scratch cwd), 200 distinct seeds (`2246822519·i + 31`) with `PROPTEST_CASES=4096`, all 29 tests: 0 failures (6 properties, about 4.9M cases).
- 20 more seeds (`97531·i + 7`) with `PROPTEST_CASES=200000`: 0 failures (about 24M cases). Timing confirms the env override is honored (4096 cases in 0.01 s, 200000 in 0.47 s for the opposite-sign property).
- No `transform.proptest-regressions` file created in the `head` export or the repo.

### Findings

**C3 (CRITICAL, class: implementation)** `crates/gafira-anim/tests/transform.rs` Q-33 tests; 02 §3.4 "Prueba de degeneración" (`|det| ≤ 1e-12·|c1|·|c2|`, `c1 = (a, b)`, `c2 = (c, d)`). Transforms are critical code (`AGENTS.md`). Hand mutant H2 (`area = a.hypot(b) * a.hypot(b)`) passes the whole implementer suite: every threshold test has columns of equal or 2:1 length, so the product of both column norms is never pinned. Fix (test-only): add an end with very unequal column lengths near the threshold, e.g. `[1e6, 0, 1e-6·√(1 − 1e-6), 1e-9]` (sine 1e-3, `|c1|·|c2| = 1`) to the identity at t = 0.5, which must be decomposed (`[500000.5, 0, 0.5·(1 + 1e-9)·499.99975…, 0.5·(1 + 1e-9), 0, 0]`; compute the expectation from the decomposition), and, for the mirror `|c2|²` mutant, the same shape with the columns' roles swapped; show H2 killed.

**C4 (CRITICAL, class: implementation)** same tests; same §3.4 rule (`≤`). Hand mutant H1 (`<` for `<=`) passes the whole implementer suite: no test sits on the threshold. The boundary is reachable with plain literals and cargo-mutants generates this mutant, so the archive-time T4 run would report it missed in critical code. Fix (test-only): `[1, 0, 1, 1e-12, 0, 0]` to the identity at t = 0.5 is degenerate and gives `[1, 0, 0, (1 + 1e-12)/2, 0, 0]` (assert the premise `|det| == 1e-12·|c1|·|c2|` in the test, as `r3_threshold_boundary_is_degenerate` does); show H1 killed.

**S7 (SUGGESTION)** `arq_19_q33_tiny_rotated_scale_keeps_its_own_rotation` tests only one direction; adding `rotation(2.0)` to `1e-7·rotation(0.3)` (same expected value) would pin the `(Some, None)` and `(None, Some)` arms symmetrically, as the large-scale test does via the identity.

**S8 (SUGGESTION)** No implementer test extrapolates (`t` outside [0, 1]) with exactly one degenerate end (§3.4 "se extrapola"); my probe `r3_one_degenerate_end_extrapolates` (`scaleX(0)` to the identity at t = 1.5 and -0.5 gives `scale(1.5, 1)` and `scale(-0.5, 1)`) passes on the code. Cheap to add next to the `scaleX(0)` test.

### Covered IDs

ARQ-19 (Transform2D part, including amended Q-33): all listed tests present and matching the spec numbers and tolerance; the general rule Q5 weak (C3, C4). ARQ-18 (`is_finite` for Transform2D): covered. ARQ-12 golden: out of scope (D3c).

### Note for the coordinator

This was round 3, the last allowed for D3b. Both open items are test-only, with the exact inputs and expectations given above, and the code is correct on them (probes pass on the clean code).

next: tdd-apply (C3, C4 test-only; S7, S8 optional); round limit reached, so the owner decides whether a fourth round is allowed
