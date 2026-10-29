# Verify report: anim-core D3b

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.
D2a: PASS in round 1 at `d9d2e50` (0 CRITICAL, 2 WARNING ambiguity: `is_finite` missing from the §3.4 snippet and the f32/Vec2 overflow domain, both routed to specifier, 4 SUGGESTION); merged into `develop` at `37769a0`.
D2b: PASS in round 1 at `f30293b` (0 CRITICAL, 1 WARNING ambiguity: ARQ-12 golden assigned to no delivery, since assigned to D3c, 4 SUGGESTION); merged into `develop` at `c048b97`.
D3a: PASS in round 1 at `1dfa4a2` (0 CRITICAL, 1 WARNING W1 Display-P3 hue test, 3 SUGGESTION); merged at `4e1089e`, then RED on `develop` (flaky Color reference property).
D3a fix round: PASS in round 2 at `8a9d36e` (0 CRITICAL, 1 WARNING ambiguity: ARQ-19 Color reference divisor not stated, 2 SUGGESTION; 5.4M property cases, 0 failures); merged into `develop` at `6710050`.
D3b round 1: FAIL at `33417d3` (1 CRITICAL C1 implementation: no opposite-sign determinant test, sign-on-`sx` mutant survived; 1 WARNING W1 ambiguity: "determinante 0" undefined in floats, settled as Q-33 B; 3 SUGGESTION S1-S3).
D3b round 2: FAIL at `33f1a4c` (1 CRITICAL C2 implementation: absolute-threshold mutant survived; 2 WARNING: W2 implementation 1e3 slack, W3 ambiguity unobservable `scale(1e-9)` example, settled in `6b37f9e`; 3 SUGGESTION S4-S6).
D3b round 3: FAIL at `e314c4f` (2 CRITICAL implementation: C3 `|c1|·|c1|` mutant H2 and C4 `<` mutant H1 survived; 2 SUGGESTION S7, S8).

Flow: standard | mutation: archive | tier: T4 | verify: V2, LIMITED round 4 (owner-authorized): scope is only the closure of the round-3 findings C3, C4, S7, S8; no hunt for new mutants or gaps (owner decision). No cargo-mutants report (expected with `mutation: archive`; not a finding). Hand mutants only.
Acceptance source: the round-3 findings, checked against 02 §3.4 "Prueba de degeneración" (`|det| ≤ 1e-12·|c1|·|c2|`), ARQ-19 Q-33 and its tolerance `|got − want| ≤ 1e-9·max(1, |want|)`, and my round-3 probes (`probe_d3b3.rs`).
Scope: `bf6b643..a2684e5`, one commit `a2684e5` (`crates/gafira-anim/tests/transform.rs` +85, `apply-progress.md` +13).

## Round 4 of D3b (commit `a2684e5`)

### Verdict: PASS

Counts: 0 CRITICAL, 0 WARNING, 0 SUGGESTION. Per class: implementation 0, ambiguity 0, architecture 0, claim 0.

### Method

- Disk free before building: 66 GB. Two `git archive` exports of HEAD in the scratchpad `verify-d3b4` (`head`: gates; `mut`: round-3 probes and hand mutants), files touched after export, one `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`; both freed with `cargo clean --target-dir` (1.8 GiB, 339 MiB).
- The 6 round-3 probes (`probe_d3b3.rs`) re-run on the clean `mut` export: 6 passed.
- Hand mutants H1, H2 re-run with the same `mutate3.py` edits as round 3, plus H2m (the mirror `area = |c2|·|c2|`, asked for in C3), each against all gafira-anim targets and the probes; source restored and compared byte-for-byte with the `head` export afterwards.

### Gates (clean export of `a2684e5`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (`tests/transform.rs` 33 passed) |
| `cargo deny check bans licenses sources` | 0 |
| CI policy steps (forbid unsafe, V5 layer table, V12 clippy copies) | not re-run: `.github/`, `.cargo/`, `deny.toml`, `Cargo.lock`, manifests and every `src/` file are unchanged in `bf6b643..HEAD`; round-3 results (all 0) stand |

### Card, commit and size checks

- No source change: `a2684e5` touches only `tests/transform.rs` and `apply-progress.md`; `git diff bf6b643..HEAD -- crates/*/src` is empty.
- Slice card unchanged: the `apply-progress.md` diff is a pure append (`## D3b round 4`), no line removed or edited.
- Hygiene: conventional commit, author Gustavo Gutierrez, no Co-Authored-By or AI attribution in `6710050..HEAD`. No `transform.proptest-regressions` in the tree or the exports; the tracked regression files are unchanged. Working tree clean.
- Size note for the owner (not a finding): crates `6710050..HEAD` = 132 (`src/interpolate.rs`) + 605 (`tests/transform.rs`) = 737 lines, above the ~700 ceiling; the excess is tests added by verify rounds 2-4.

### Round-3 findings

| Finding | Status | Evidence |
|---|---|---|
| C3 `|c1|·|c2|` not pinned | closed | `arq_19_q33_unequal_columns_with_sine_1e_3_are_decomposed_in_both_orders`: `[1e6, 0, 1e-6·√(1−1e-6), 1e-9]` with the identity at t = 0.5, both argument orders, expects `[500000.5, 0, sy·k, sy, 0, 0]`, `sy = 0.5·(1 + 1e-9)`, `k = 0.5·x/1e-9` (my probe's values, coefficient 2 ≈ 249.99988); and the swapped columns `[x, 1e-9, 1e6, 0]`, expectation built from the §3.4 decomposition (`θ = atan2(1e-9, x)`, `sx = |c1|`, `sy = det/sx < 0`, `k = (c1·c2)/det`) and recomposed after the lerp with the identity; I re-derived it and it is consistent. Premise asserts guard that the shear is present. Spec tolerance via `assert_abs_rel` (`1e-9·max(1, |want|)`). H2 killed by the first half, H2m by the swapped half (failure message shows the swapped expectation). |
| C4 `<=` not pinned | closed | `arq_19_q33_determinant_exactly_at_the_threshold_is_degenerate`: `[1, 0, 1, 1e-12]`, premise `|det| == 1e-12·|c1|·|c2|` asserted exactly, expects `[1, 0, 0, (1 + 1e-12)/2, 0, 0]` at t = 0.5 in both orders (my probe's value), plus coefficient 2 exactly 0. H1 killed (mutant gives coefficient 2 = 2.5e11). |
| S7 tiny rotation one direction | closed | `arq_19_q33_tiny_rotated_scale_keeps_its_own_rotation_reversed`: `rotation(2.0)` to `1e-7·rotation(0.3)` at 0.5 gives `0.50000005·rotation(1.15)`, as in my probe. |
| S8 no one-degenerate extrapolation | closed | `arq_19_s8_extrapolation_with_one_degenerate_end`: degenerate end `[1e6, 0, 2e6·cos 1e-13, 2e6·sin 1e-13]` with the identity at t = -0.5, 1.5, 2 in both orders, expects the lerp of the diagonal parts in the identity frame (§3.4 borrowing, "se extrapola"). Different input from my probe (`scaleX(0)`), same rule; my probe also passes. |

### Hand mutants (scratch export `mut`, `src/interpolate.rs`)

| # | Mutant | Implementer suite | Probes |
|---|---|---|---|
| H1 | threshold `<` instead of `<=` | killed: `arq_19_q33_determinant_exactly_at_the_threshold_is_degenerate` (only) | killed: `r3_threshold_boundary_is_degenerate` |
| H2 | `area = |c1|·|c1|` | killed: `arq_19_q33_unequal_columns_with_sine_1e_3_are_decomposed_in_both_orders` (only) | killed: `r3_unequal_columns_not_degenerate` |
| H2m | `area = |c2|·|c2|` (mirror) | killed: same test, swapped-column half | survives (my probe had only one column order; the implementer's test covers it) |

Implementer's round-4 table (`apply-progress.md`): accurate; H1 and H2 are killed by exactly the tests it names.

### Q-33 criteria

Q5 (rule `|det| ≤ 1e-12·|c1|·|c2|`, §3.4) is now covered: the comparison operator (C4) and both column norms (C3) are pinned, in addition to the relative threshold (round 3, M3). Total for D3b: 24 criteria, 24 covered.

### Findings

None. No probe showed a code defect.

### Covered IDs

ARQ-19 (Transform2D part, including amended Q-33): covered. ARQ-18 (`is_finite` for Transform2D): covered. ARQ-12 golden: out of scope (D3c).

### Note for the coordinator

D3b passes; D3c (ARQ-12 golden) remains, so the change is not done. Size note above (737 lines, test-driven) is for the owner.

next: tdd-apply (D3c, after merging D3b into `develop`)
