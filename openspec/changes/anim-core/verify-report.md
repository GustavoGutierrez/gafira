# Verify report: anim-core D3b

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.
D2a: PASS in round 1 at `d9d2e50` (0 CRITICAL, 2 WARNING ambiguity: `is_finite` missing from the §3.4 snippet and the f32/Vec2 overflow domain, both routed to specifier, 4 SUGGESTION); merged into `develop` at `37769a0`.
D2b: PASS in round 1 at `f30293b` (0 CRITICAL, 1 WARNING ambiguity: ARQ-12 golden assigned to no delivery, since assigned to D3c, 4 SUGGESTION); merged into `develop` at `c048b97`.
D3a: PASS in round 1 at `1dfa4a2` (0 CRITICAL, 1 WARNING W1 Display-P3 hue test, 3 SUGGESTION); merged at `4e1089e`, then RED on `develop` (flaky Color reference property).
D3a fix round: PASS in round 2 at `8a9d36e` (0 CRITICAL, 1 WARNING ambiguity: ARQ-19 Color reference divisor not stated, 2 SUGGESTION; 5.4M property cases, 0 failures); merged into `develop` at `6710050`.
D3b round 1: FAIL at `33417d3` (1 CRITICAL C1 implementation: no opposite-sign determinant test, sign-on-`sx` mutant survived; 1 WARNING W1 ambiguity: "determinante 0" undefined in floats, settled as Q-33 B; 3 SUGGESTION S1-S3).

Flow: standard | mutation: archive | tier: T4 | verify: V2 (on the round-1 findings and the Q-33 change). No cargo-mutants report (expected with `mutation: archive`; not a finding). Hand mutants only.
Acceptance source: Slice card D3b row (`apply-progress.md`), 02 §3.4 Transform2D rows (now with "Prueba de degeneración", Q-33), the ARQ-19 Transform2D criteria (new Q-33 unit tests with stated values), the determinism table (1e-9 Transform2D cross-platform tolerance), and the round-1 findings.
Scope: `6710050..33f1a4c` on `feat/anim-core-d3b`; new in this round: `33960ab`, `662f195` (spec, `docs/specs/02-arquitectura-software.md` only), `bde57a1` (`src/interpolate.rs` +5/-2, `tests/transform.rs` +220), `33f1a4c` (`apply-progress.md`).

## Round 2 of D3b (commit `33f1a4c`)

### Verdict: FAIL

Counts: 1 CRITICAL, 2 WARNING, 3 SUGGESTION. Per class: implementation 2 (C2, W2), ambiguity 1 (W3), architecture 0, claim 0.

### Method

- Disk free before building: 67 GB. Two `git archive` exports of HEAD in the scratchpad (`head`: gates, CI steps, implementer seeds; `mut`: probes, hand mutants, a no-slack copy of the implementer's opposite-sign property), files touched after export, one `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`; both freed with `cargo clean --target-dir`.
- Probes (`probe_d3b2.rs`, 11 tests) written from §3.4 / ARQ-19 / Q-33 before reading the new implementer tests: the four Q-33 values, a column-sine threshold edge (1e-13 / 1e-11), scale invariance of the threshold (same near-singular shape at scale 1e-6, 1, 1e6), a tiny rotated scale towards a different rotation (distinguishes relative from absolute threshold), fixed flip cases, strict 1e-9 reference properties for opposite-sign and same-sign pairs (`t` in [-0.5, 1.5]) and a 2M-pair error census. All pass on the clean code.

### Gates (clean export of `33f1a4c`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (`tests/transform.rs` 28 passed) |
| `cargo deny check bans licenses sources` | 0 |
| CI: Domain crates forbid unsafe code | 0 |
| CI: V5 layer table | 0 |
| CI: V12 clippy.toml copies | 0 |

`.github/workflows/ci.yml` is unchanged in the range; the step scripts are the round-1 extracts.

### Card, commit and size checks

- `apply-progress.md`: `git diff 6710050..HEAD` removes no line; `33f1a4c` edits only the implementer's own `## D3b progress` notes (S2 correction) and appends `## D3b round 2`. The Slice card is unchanged.
- Spec commits `33960ab`, `662f195` touch only `docs/specs/02-arquitectura-software.md`; no implementer commit touches `docs/specs`.
- Size: crates `6710050..HEAD` = 131 + 518 insertions, 1 deletion (649 lines, ceiling ~700). OK.
- Hygiene: all four commits conventional, author Gustavo Gutierrez, no Co-Authored-By or AI attribution.

### Round-1 findings

| Finding | Status | Evidence |
|---|---|---|
| C1 opposite-sign coverage | closed | 3 unit tests (`..._flip_from_identity_collapses_the_y_axis_at_the_middle`, `..._horizontal_flip_..._half_turn_frame`, `..._rotated_flip_keeps_the_rotation_...`) and property `..._opposite_sign_pairs_match_the_decomposition_reference`; sign-on-`sx` mutant M1 killed by all 4. Expected values match my probes (`[1, 0, 0, 0]`; `[0, ±1, 0, 0]`). |
| W1 degeneracy test (Q-33 B) | code matches spec; one rule untested (C2) | `Parts::decompose`: `!is_finite6 || !det.is_finite() || !area.is_finite() || |det| <= 1e-12·area`, `area = hypot(a, b)·hypot(c, d)`, all `f64`, exactly §3.4. |
| S1 non-finite end never panics | closed | `arq_19_transform_non_finite_endpoint_never_panics` (NaN, ±inf per coefficient, either or both ends, 5 `t`). |
| S2 "mixed-sign unspecified" note | closed | note now cites "un volteo es siempre `sy < 0`". |
| S3 borrowed frame on one test | closed | `..._degenerate_end_uses_the_diagonal_in_a_rotated_sheared_frame` (expectation `diag(U^-1 R^T L)`, checked by hand: `s11 = cos θ + k sin θ`, `s22 = 0`), asserts it differs from the 6-coefficient blend, both directions; M5 killed by 5 tests. |

### Q-33 criteria (ARQ-19 numbers, zero translation, towards the identity, t = 0.5)

| # | Spec value | Implementer test | Probe | Status |
|---|---|---|---|---|
| Q1 | `[0.1, 0.3, 0.7, 2.1]` (det ≈ 2.8e-17) degenerate -> `[0.55, 0, 0, 1.55]` | `arq_19_q33_rank_one_end_with_rounding_determinant_is_degenerate` (asserts det != 0 premise) | `q33_rank_one_by_rounding_is_degenerate` (both directions) | covered |
| Q2 | `[1.1, 2.2, 3.3, 6.6]` (det 0 exact) -> `[1.05, 0, 0, 3.8]` | same test | `q33_exact_zero_det_is_degenerate` | covered |
| Q3 | `[1e200, 1e200, 1e200, 2e200]` (det NaN) degenerate -> `[5e199, 0, 0, 1e200]`, all finite | `arq_19_q33_finite_coefficients_with_non_finite_determinant_are_degenerate` (`is_finite` asserted; CI runs the 3 OS) | `q33_non_finite_det_is_degenerate` (both directions) | covered |
| Q4 | `scale(1e-9)` not degenerate -> `scale(0.5 + 5e-10)` | `arq_19_q33_tiny_uniform_scale_is_not_degenerate` | `q33_tiny_scale_spec_example` | covered as literally specified, but not observable (W3) |
| Q5 | Threshold `|det| <= 1e-12·|c1|·|c2|`, scale-independent ("no depende de la escala") | `arq_19_q33_threshold_is_relative_to_the_column_lengths` (sine 1e-13 / 1e-11, unit-length columns only) | `q33_threshold_is_the_column_sine`, `q33_threshold_scale_invariant`, `q33_tiny_rotated_scale_is_decomposed` | **weak: C2** |

The Q-33 unit tests use 1e-9 without slack (`assert_rel`, `assert_abs_rel`). Color divisor wording (`33960ab`): code divides by `f64::from(alpha)` with `alpha = a.a.interpolate(&b.a, t)` (the f32 rule, bit exact) and the `tests/color.rs` reference divides by the same f32 alpha; matches the new text, no Color code change in the range.

Earlier criteria 1-18 (round 1) unchanged and still green; total for D3b now 18 + Q1-Q5 = 23 criteria, 22 covered, 1 weak (Q5).

### Hand mutants (5 run on `src/interpolate.rs`, scratch export `mut`; implementer suite: 4 killed, 1 survived)

| # | Mutant | Implementer tests | Probes |
|---|---|---|---|
| M1 | Sign on `sx` (`sx = hypot.copysign(det)`, `sy = |p|`) | killed: 4 (3 flip unit tests, opposite-sign property) | killed: `c1_flip_cases`, `pp_opposite_sign_strict` |
| M2 | Exact `det == 0.0` instead of the relative test | killed: 3 (rounding rank-1, non-finite det, column sine) | killed: 4 |
| M3 | Absolute threshold `|det| <= 1e-12` | **survives (28/28 pass)** | killed: `q33_threshold_scale_invariant`, `q33_tiny_rotated_scale_is_decomposed` |
| M4 | Non-finite checks removed (`!det.is_finite() || !area.is_finite()`) | killed: non-finite det test | killed: `q33_non_finite_det_is_degenerate` |
| M5 | Borrowing disabled (one degenerate end -> 6-coefficient lerp) | killed: 5 | killed: 5 |

M3 is not equivalent: `1e-7·rotation(0.3)` to `rotation(2.0)` at t = 0.5 must keep its own rotation (θ = 1.15), but under M3 (det 1e-14) it borrows θ = 2.0; and `[1e6, 0, 2e6·cos 1e-13, 2e6·sin 1e-13]` (sine 1e-13, det 0.2) must be degenerate but is decomposed with a ~1e13 shear. Note: removing only `!area.is_finite()` is equivalent (a non-finite area makes `1e-12·area = ∞`, so any finite `|det|` already passes the threshold, and `|det| ≤ |c1|·|c2|` keeps a NaN det from coexisting with a finite area); not counted.

### Implementer deviation judged: 1e3 slack on the opposite-sign property

- Not a loosening of a spec tolerance. ARQ-19 asks only for a same-sign property; the opposite-sign property is extra coverage (round-1 C1 suggested it as optional). The 1e-9 in the determinism table is the cross-platform drift tolerance between platforms, not an accuracy bound against a reference, and the ARQ-19 1e-9 for Q-33 applies to the four Q-33 unit tests, which keep it. No spec text is weakened, so not class `ambiguity`.
- The stated reason ("near-singular ends give large shears") does not hold in the test's own domain (`|det| > 1e-3`, coefficients in [-3, 3]): census of 2,000,000 opposite-sign pairs, `t` in [-0.5, 1.5], against an independent reference: worst error 9.3e-4 of the 1e-9·max(1, |w|) bound, 0 over. A copy of the implementer property with the slack removed passes 150 distinct seeds × 2048 cases. The slack makes the property accept 1e-6 relative error, 1000× more than needed. Class `implementation` (test quality), non-blocking: M1 is killed by large margins and the 3 unit tests use the plain 1e-9 bound (W2).

### Flake evidence (proptest, `PROPTEST_RNG_SEED`, outside the tree; no `transform.proptest-regressions` file created)

- HEAD `tests/transform.rs` binary (clean export), 150 distinct seeds (`2654435761·i + 17`), all 28 tests: 0 failures (about 1.1M property cases).
- No-slack copy of the opposite-sign property, same 150 seeds × 2048: 0 failures.
- Verifier properties `pp_opposite_sign_strict`, `pp_same_sign_strict`, 150 seeds × 4096: 0 failures. Plus the 2M-pair census.

### Findings

**C2 (CRITICAL, class: implementation)** `crates/gafira-anim/tests/transform.rs` `arq_19_q33_threshold_is_relative_to_the_column_lengths` / `arq_19_q33_tiny_uniform_scale_is_not_degenerate`; 02 §3.4 "Prueba de degeneración" ("no depende de la escala"), Q-33 B. Transforms are critical code (`AGENTS.md`). The absolute-threshold mutant M3 (`|det| <= 1e-12`) passes all 28 implementer tests: the column-sine test uses unit-length columns, where relative and absolute thresholds agree, and `scale(1e-9)` to the identity gives the same result whether the end is decomposed or borrows the identity frame. The progress note's claim that the `scale(1e-9)` test "guards against an absolute threshold" is wrong. Fix (test-only): add a case where scale changes the verdict, e.g. the sine-1e-13 shape scaled by 1e6 (`[1e6, 0, 2e6·cos 1e-13, 2e6·sin 1e-13]`, det 0.2, must be degenerate: towards the identity at t = 0.5 it borrows the identity frame and gives `[500000.5, 0, 0, (1 + 2e-7)/2]`, no large shear), and a tiny rotated scale towards a different rotation (`1e-7·rotation(0.3)` to `rotation(2.0)`, t = 0.5 gives `0.50000005·rotation(1.15)`); show M3 killed.

**W2 (WARNING, class: implementation)** `crates/gafira-anim/tests/transform.rs:460` opposite-sign property. The `* 1e3` slack is not needed (census: worst 9.3e-4 of the 1e-9 bound over 2M pairs; slack-free copy green on 150 seeds) and its rationale is inaccurate. Drop it, or justify it with a measured error; not blocking, as the property still kills M1.

**W3 (WARNING, class: ambiguity)** 02 ARQ-19 Q-33 criterion "`scale(1e-9)` no es degenerado y da `scale(0,5 + 5e-10)`". The example cannot observe the rule it illustrates: towards the identity, a degenerate `scale(1e-9)` borrows the identity frame and its diagonal is `1e-9`, giving the same `scale(0.5 + 5e-10)`. The specifier should replace or add an observable example (a tiny scale with a rotation different from the other end, or a near-singular shape at large scale) so the mandated test pins scale independence. Non-blocking; the implementation part of the gap is C2.

**S4 (SUGGESTION)** ARQ-19 says "diferencia relativa ≤ 1e-9 por coeficiente", which is undefined for expected zeros; `assert_rel` (relative to `max(|got|, |want|)`, zeros must be exact) and `assert_abs_rel` (`1e-9·max(1, |w|)`) read it two ways. The specifier could state `1e-9·max(1, |c|)` as in the determinism table.

**S5 (SUGGESTION)** The `!area.is_finite()` clause in `Parts::decompose` is redundant (see the M4 note); it matches the spec text literally, so keep it, but a comment saying it is implied would stop a future mutant run from flagging it as a survivor.

**S6 (SUGGESTION)** The progress note for round 2 should drop the claim that `arq_19_q33_tiny_uniform_scale_is_not_degenerate` guards against an absolute threshold, once C2 adds a test that does.

### Covered IDs

ARQ-19 (Transform2D part, including Q-33): all listed tests present; Q5 weak (C2). ARQ-18 (`is_finite` for Transform2D): covered. ARQ-19 Color divisor wording: matches code and test reference. ARQ-12 golden: out of scope (D3c).

next: tdd-apply (D3b round 3: C2 test-only, W2 optional); then specifier for W3 and S4 (non-blocking)
