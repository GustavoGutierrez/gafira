# Verify report: anim-core D3b

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.
D2a: PASS in round 1 at `d9d2e50` (0 CRITICAL, 2 WARNING ambiguity: `is_finite` missing from the §3.4 snippet and the f32/Vec2 overflow domain, both routed to specifier, 4 SUGGESTION); merged into `develop` at `37769a0`.
D2b: PASS in round 1 at `f30293b` (0 CRITICAL, 1 WARNING ambiguity: ARQ-12 golden assigned to no delivery, since assigned to D3c, 4 SUGGESTION); merged into `develop` at `c048b97`.
D3a: PASS in round 1 at `1dfa4a2` (0 CRITICAL, 1 WARNING W1 Display-P3 hue test, 3 SUGGESTION); merged at `4e1089e`, then RED on `develop` (flaky Color reference property).
D3a fix round: PASS in round 2 at `8a9d36e` (0 CRITICAL, 1 WARNING ambiguity: ARQ-19 Color reference divisor not stated, 2 SUGGESTION; 5.4M property cases, 0 failures); merged into `develop` at `6710050`.

Flow: standard | mutation: archive | tier: T4 | verify: V2. No cargo-mutants report (expected with `mutation: archive`; not a finding). Hand mutants only.
Acceptance source: Slice card D3b row (`apply-progress.md`), `docs/specs/02-arquitectura-software.md` §3.4 Transform2D rows (decomposition, degenerate end Q-28 B / Q-30 A), common rules (exact endpoints, `is_finite`), determinism table (1e-9 tolerance), ARQ-19 (Transform2D part) and ARQ-18 (`is_finite`). Spike claim 1 (modified decomposition, sign kept on 0/200000 same-sign pairs).
Scope: `6710050..33417d3` on `feat/anim-core-d3b`: `d6c05c2` (`src/interpolate.rs` +128/-1, `tests/transform.rs` +298) and `33417d3` (appends `## D3b progress` to `apply-progress.md`).

## Round 1 of D3b (commit `33417d3`)

### Verdict: FAIL

Counts: 1 CRITICAL (implementation), 1 WARNING (ambiguity), 3 SUGGESTION. Per class: implementation 1, ambiguity 1, architecture 0, claim 0.

### Method

- Disk free before building: 67 GB. Two `git archive` exports of `33417d3` in the scratchpad (`head`: gates, CI steps, implementer seeds; `mut`: probes and hand mutants), files touched after export, one `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`; both freed with `cargo clean --target-dir`.
- Probes written from §3.4 / ARQ-19 before reading `tests/transform.rs` (19 tests, 5 properties at 2048 cases), including an independent reference decomposition (`sx = |col1|`, `sy = det / sx`, `k = col1·col2 / (sx·sy)`, `θ = atan2`), compared within 1e-9 on random non-degenerate pairs with `t` in [-0.5, 1.5]. All 19 pass on the clean code.

### Gates (clean export of `33417d3`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (`tests/transform.rs` 18 passed) |
| `cargo deny check bans licenses sources` | 0 |
| CI: Domain crates forbid unsafe code | 0 |
| CI: V5 layer table | 0 |
| CI: V12 clippy.toml copies | 0 |

### Card, commit and size checks

- Only `33417d3` touches `apply-progress.md` in the range, and it only appends the `## D3b progress` section: the Slice card is unchanged by implementer commits.
- Size: `git diff --shortstat 6710050..HEAD` = 3 files, 432 insertions, 1 deletion (forecast ~450). OK.
- Hygiene: both commits conventional, author Gustavo Gutierrez, no Co-Authored-By or AI attribution.

### Criteria map

| # | Criterion (§3.4 / ARQ-19 / ARQ-18) | Implementer test | Verifier probe | Status |
|---|---|---|---|---|
| 1 | Two translations interpolate only the translation | `arq_19_transform_two_translations_interpolate_only_the_translation` | `p_translations_and_scales` | covered |
| 2 | 350° to 10°, t = 0.5 gives 0° | `arq_19_transform_rotation_takes_the_short_way` | `p_rotation_350_to_10` | covered |
| 3 | Shortest path `|Δθ| ≤ π` | `arq_19_transform_rotations_follow_the_short_arc` (property) | `pp_short_arc` | covered |
| 4 | Two scales per axis | `arq_19_transform_two_scales_interpolate_per_axis_and_extrapolate` | `p_translations_and_scales` | covered |
| 5 | Identity to shear `k`: continuous, Δt = 1e-3, ≤ 1e-2·max(1,|k|) | `arq_19_transform_shear_from_identity_is_linear_and_continuous` | `p_shear_from_identity` | covered |
| 6 | Scale 0 to B: linear part `t·B`, translation lerped | `arq_19_transform_scale_zero_to_b_scales_the_linear_part_by_t` | `p_scale0_to_b_and_back` | covered |
| 7 | `scaleX(0)` to identity, t = 0.5 gives `scale(0.5, 1)` | `arq_19_transform_scale_x_zero_to_identity_gives_half_scale_at_the_middle` | `p_scale_x0_to_identity` | covered |
| 8 | Rank-1 jump Q-30 (`[1,2,2,4]`: t = 0 bitwise, 0.5, 1e-6) | `arq_19_transform_rank_one_endpoint_jumps_as_documented` | `p_rank_one_jump` | covered |
| 9 | Both degenerate: 6-coefficient lerp | `arq_19_transform_both_degenerate_...`, `..._degenerate_pairs_are_the_linear_blend` (property) | `p_both_degenerate`, `pp_degenerate_pairs_lerp` | covered |
| 10 | Q-30 diagonal in the borrowed (θ, k) frame | rank-1 test only | `p_degenerate_in_rotated_sheared_frame` | covered (see S3) |
| 11 | Same-sign determinant kept, `|det| > 1e-3`, coeffs [-3,3], t in 0.01..0.99 | `arq_19_transform_same_sign_determinants_keep_their_sign` | `pp_same_sign_keeps_sign` | covered |
| 12 | Exact endpoints at t = 0 / 1, also degenerate | `arq_19_transform_endpoints_are_exact` (property), `..._for_fixed_cases` | `pp_endpoints_exact` | covered |
| 13 | Extrapolation (no wrap, no clamp) | `..._rotation_extrapolates_without_wrapping`, scales at 1.5 / -0.5 | `p_extrapolation` | covered |
| 14 | Decomposition form `[sx c, sx s, sy(kc - s), sy(ks + c)]`, each part lerped, 1e-9 | indirect (fixed cases) | `pp_matches_reference` | covered |
| 15 | "Un volteo (determinante negativo) es siempre `sy < 0`" | `..._flip_is_carried_by_the_y_scale`, `..._flip_blends_the_rotation_...` (same-sign pairs only) | `p_flip_is_sy_crossing_zero` (mixed-sign) | **weak: C1** |
| 16 | `is_finite` false on NaN / ±∞ per coefficient, true when finite (ARQ-18, ARQ-19) | `arq_18_transform_is_finite_checks_every_coefficient` | `p_is_finite` | covered |
| 17 | Non-finite coefficient: track invalid (out of contract for `interpolate`) | none (by ARQ-18 at track level) | `p_non_finite_no_panic` | covered (out of contract, S1) |
| 18 | Tolerance 1e-9·max(1,|c|) per coefficient | `assert_close` helper | `close(…, 1e-9)` | covered |

Total 18, covered 17, weak 1.

### Hand mutants (5 run, 5 killed; `src/interpolate.rs` Transform2D impl, scratch export `mut`)

| # | Mutant | Killed by implementer tests | Killed by probes |
|---|---|---|---|
| M1 | Longest-path rotation (`if |dθ| < π { dθ -= copysign(τ, dθ) }`) | 11 tests incl. `rotation_takes_the_short_way`, `rotations_follow_the_short_arc` | 11 |
| M2 | Missing shear (`k = 0`) | `shear_from_identity_...`, `scale_zero_to_b_...`, `equal_endpoints_stay_put_...` | 4 incl. `pp_matches_reference` |
| M3 | No t = 0 / 1 special case (Transform2D only) | `endpoints_are_exact`, `..._for_fixed_cases`, `rank_one_...` (t = 0 bitwise) | `p_rank_one_jump`, `pp_endpoints_exact` |
| M4 | Degenerate rule swapped (one degenerate end -> 6-coefficient lerp, Q-28 A instead of B) | `rank_one_endpoint_jumps_as_documented` only | `p_rank_one_jump` |
| M5 | Wrong per-axis scale for a degenerate end (column norms instead of the diagonal in the borrowed frame, Q-30) | `rank_one_endpoint_jumps_as_documented` only | `p_rank_one_jump`, `p_degenerate_in_rotated_sheared_frame` |

### Flake evidence (proptest 1.12, `PROPTEST_RNG_SEED`, outside the tree, no `transform.proptest-regressions` file exists)

- `tests/transform.rs` binary from the clean export, 100 distinct seeds (`104729·i + 13`, i = 1..100), 1024 cases per property: 0 failures.
- Verifier probes, 60 distinct seeds (`15485863·i + 7`), 2048 cases per property: 0 failures.
- About 0.6M implementer property cases and 0.6M probe cases, no flake. (A first probe batch failed on every seed because the binary was still the M3 mutant build; rebuilt on the original source and rerun: 0 failures. Recorded so the count is not misread.)

### Implementer deviations, judged

1. Non-finite coefficient on either end yields the 6-coefficient lerp, no panic. §3.4: "Un coeficiente no finito hace inválida la pista (ARQ-18)", so `interpolate` on such a value is out of contract; any total behavior is acceptable. Probe confirms no panic. Accepted (S1).
2. "Degenerate" = `det == 0.0` exactly or non-finite decomposed parts. Matches §3.4 literally ("Un extremo con determinante 0") and the Q-28 question wording ("determinante 0 o coeficientes no finitos"); the non-finite-parts extension only triggers for finite coefficients near `f64::MAX`. Accepted, but the literal test exposes a spec gap (W1).
3. Mixed-sign determinants "unspecified": not so. §3.4 states "Cada componente se interpola linealmente" and "Un volteo (determinante negativo) es siempre `sy < 0`", which fully fix the mixed-sign result: `sx` stays > 0 and `sy` lerps through 0. The code does exactly that (probe: identity to `scale(1, -1)` at t = 0.5 gives `[1, 0, 0, 0]`). Not an ambiguity; the note is inaccurate (S2) and the rule is untested (C1).
4. Sign-on-`sx` mutant claimed equivalent: refuted. For a det < 0 end it maps `(sx, sy, k, θ)` to `(-sx, -sy, k, θ + π)`; between two det < 0 ends (or two det > 0 ends, or with a degenerate end, whose borrowed diagonal also flips) the results coincide, which is why it survives. Between ends of opposite sign it differs: identity to `scale(1, -1)` at t = 0.5 gives `[0, 0, 1, 0]` (a quarter-turn path) instead of the spec's `[1, 0, 0, 0]`. The implementer suite has no mixed-sign pair. C1.
5. Det-sign property swaps `b`'s two linear columns (`[b2, b3, b0, b1]`) when signs differ. The swap negates the determinant and keeps `|det|` and the coefficient range, so it is a bijection between the two sign classes: `b` is still uniform over same-sign endpoints with `|det| > 1e-3`, coefficients in [-3, 3], and `a` covers both signs. It tests what ARQ-19 asks. Accepted.

### Findings

**C1 (CRITICAL, class: implementation)** `crates/gafira-anim/tests/transform.rs`; §3.4 Transform2D row "Un volteo (determinante negativo) es siempre `sy < 0`". Transforms are critical code (`AGENTS.md`), and this spec rule is only observable between ends of opposite determinant sign, which no implementer test exercises; the sign-on-`sx` hand mutant (the implementer's own) survives and is not equivalent (deviation 4 above: `[0, 0, 1, 0]` vs `[1, 0, 0, 0]` for identity to `scale(1, -1)`, t = 0.5). This is the common flip preset path. Fix (test-only): unit tests with opposite-sign ends, e.g. identity to `scale_xy(1, -1)` at t = 0.5 gives `[1, 0, 0, 0]`, and identity to `scale_xy(-1, 1)` (det < 0: `sx = 1`, `θ = π`, `sy = -1`) at t = 0.5 gives `[0, ±1, 0, 0]`; optionally an opposite-sign property against an independent reference decomposition. Rerun the sign-on-`sx` mutant and show it killed; correct the "mixed-sign unspecified" note.

**W1 (WARNING, class: ambiguity)** 02 §3.4 Transform2D degenerate row (Q-28) / ARQ-19. "Determinante 0" is not defined for floating point. The code tests `det == 0.0`, so a rank-1 end whose determinant rounds away from 0 is decomposed with a near-zero `sy` and a huge shear: `[0.1, 0.3, 0.7, 2.1]` (det 2.8e-17) to the identity at t = 0.5 gives `c = 4.0e15, d = 2.9e15`, while `[1.1, 2.2, 3.3, 6.6]` (det exactly 0) gives the Q-28 B result `[1.05, 0, 0, 3.8]`. Also, finite coefficients near 1e200 give `det = NaN` (∞ - ∞), treated as non-degenerate with an arbitrary sign. Presets produce exact zeros, so not blocking for D3b; the specifier should state whether the test is exact (and that near-singular ends are out of contract) or a relative threshold such as `|det| ≤ ε·|a|·|d|`-style, and what a non-finite determinant of finite coefficients means.

**S1 (SUGGESTION)** No implementer test pins that `interpolate` never panics with a non-finite end (out of contract, total by construction). Optional one-line test.

**S2 (SUGGESTION)** `apply-progress.md` D3b note "Mixed-sign determinants are unspecified" is inaccurate (deviation 3); fix it with C1.

**S3 (SUGGESTION)** The Q-28 B borrowing rule differs from a plain 6-coefficient lerp only when the degenerate end does not fit the borrowed frame, so M4 and M5 are each killed by the single rank-1 test. Add a case such as `scale_xy(0, 1)` to a rotated and sheared `B`, with the expected diagonal in B's frame, so the rule does not hang on one test.

### Covered IDs

ARQ-19 (Transform2D part): all listed tests present; criterion 15 weak (C1). ARQ-18 (`is_finite` for Transform2D): covered. ARQ-12 golden: out of scope (D3c).

next: tdd-apply (D3b round 2: C1 test-only); then specifier for W1 (non-blocking)
