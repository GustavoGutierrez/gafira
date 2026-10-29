# Verify report: anim-core D1

Flow: standard | mutation: archive | tier: T4 | verify: V2 (round 2: on the round-1 findings only).
No cargo-mutants report for this delivery (expected with `mutation: archive`; not a finding).
Acceptance source: Slice card (`apply-progress.md`), `docs/specs/02-arquitectura-software.md` §2.1, §2.2, §3.4, ARQ-17 (as amended by `9907bf9`).

## Round 1 summary (commit `f7e64ba`): FAIL

3 CRITICAL, 3 WARNING, 3 SUGGESTION. C1: the absolute solver stop `|x(s)-u| < 1e-12` returned the first guess for tiny `u` with flat-start curves (30x the precision tolerance). C2: no valid `x1 = 1.0` case (hand mutant M1 survived). C3: the precision property used uniform `u` and missed C1 and M2. W1: input clamping and NaN untested (M5 survived). W2 (ambiguity): spec silent on `apply` outside [0,1] and NaN. W3: hand mutants M3 (hull clamp) and M4 (Newton bracket guard) survived, likely equivalent. S1 Hold step, S2 extra derives, S3 CSS literals.

## Round 2 (commit `0a2f336`)

Verified on a clean `git archive` export of `0a2f336` outside the repo (files touched after export, own `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`). Delivery range `9219560..0a2f336` without the docs commits `9907bf9` (spec) and `e501ee1` (round-1 report).

### Verdict: PASS (delivery D1; D2 and D3 remain)

Counts: 0 CRITICAL, 1 WARNING, 4 SUGGESTION. Per class: implementation 1 WARNING.

### Gates (export of `0a2f336`, toolchain 1.93.1)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (gafira-anim `easing`: 13 tests pass) |
| CI "Domain crates forbid unsafe code" (replayed from `ci.yml`) | 0 |
| CI "Domain crates respect the layer table (V5)" (replayed) | 0 |
| CI "Domain clippy.toml copies are in sync (V12)" (replayed) | 0 |
| `cargo deny --locked check` | 0 |

### Hygiene

- No AI attribution in any commit of the range (`9219560`, `3f13d9a`, `f7e64ba`, `9907bf9`, `e501ee1`, `0a2f336`). Conventional subjects.
- Slice card: `git diff 9219560 0a2f336 -- apply-progress.md` has no removed lines; `f7e64ba` and `0a2f336` only append `## D1 progress` and `## D1 round 2`. The card block is unchanged. OK.
- `0a2f336` adds 2 seeds to `tests/easing.proptest-regressions` (checked in as proptest recommends). OK.
- Size: `git diff --shortstat 9219560 0a2f336` (docs and report excluded) = 501 inserted lines, forecast ~300 (x1.67, inside the measured x1.4 to x1.9 band), below the ~700 ceiling.

### Round-1 findings re-checked

| ID | Status | Evidence |
|---|---|---|
| C1 | fixed | Stop is now `err.abs() <= X_EPSILON * u`. Round-1 probe file: `p_fail_map` shows no failures for any of the five curves that failed in round 1 (`(0,1,0,1)`, `(0,3,0,-2)`, `(0,3,1e-6,1)` ...); named-curve worst ratio 0.003. `zz_probe2::q_grid_fails_only_at_u_one`: 0 violations for `u < 1` over the round-1 grid plus `u` = smallest subnormal, largest subnormal, `MIN_POSITIVE`. Reverting the stop to the round-1 absolute form (R0) makes 3 implementer tests fail, confirming the RED claim. |
| C1 termination | bounded, precise | The loop is a `for` over `MAX_ITERATIONS = 64`, so it always ends. `u = 0` (and `-0.0`, NaN) never reaches the solver (`apply` returns early); for subnormal `u` the threshold `1e-12 * u` stays a normal f64 (~1.4e-57), so no underflow to 0. An instrumented replica of the solver (bit-identical to `apply` on all 6.9 M cases) over x1, x2 in {0, 1e-45, 1e-6, 0.1, 0.42, 0.58, 0.9, 1-eps, 1}, y in {-2,0,1,3} and ~5 200 `u` bit patterns from 1e-45 to 1-eps: 0 precision violations, all results finite. 300 000 random curves with log-uniform `u`: max 11 iterations, 0 cap hits, 0 violations. The 64-iteration cap is reached only with `x1 ~ 0` and `u < ~6e-15` (see S1); the result there is still within tolerance. |
| C2 | fixed | Validity test now has `x1 = 1.0`, `x2 = 1.0`, `x1 = x2 = 0.0` valid and `-MIN_POSITIVE` / `next_up(1.0)` invalid on both x. M1 killed by the implementer suite. |
| C3 | fixed | Precision property draws `u` log-biased near 0 and 1 and x near 0 and 1, 4 096 cases; plus a fixed grid test and a pinned C1 test (`arq_17_bezier_is_precise_for_tiny_u_with_flat_start`). M2 killed by the implementer suite. |
| W1 | fixed | `arq_17_every_curve_clamps_its_input_and_maps_nan_to_zero` covers u < 0, -0.0 (bit-exact +0), -inf, NaN -> 0 and u > 1, +inf -> 1 for every variant. M5 killed. |
| W2 | settled | `9907bf9` amends §3.4 and ARQ-17: `Hold` is `e(u)=0` for `u<1`, `e(1)=1`; input clamped to [0,1], NaN -> 0, output not clamped. The code matches: `apply` returns 0 for NaN or `u <= 0`, 1 for `u >= 1`, `Hold` -> 0 inside; `bezier` output may leave [0,1] up to the hull bounds. The ARQ-17 unit tests the spec now lists (Hold at 0, 0.5, 1; every curve at -1, -inf, NaN, 2, +inf) are all present. |
| W3 | open, documented equivalent | See W3 below. |
| S1, S2 (round 1) | closed | Hold step now in the spec; derives are not a finding. |
| S3 (round 1) | open | Carried as S3 below. |

Note on the round-1 probe `p_precision_grid`: it still reports 15 "violations", all at `u = 1.0` exactly with `x2 = 1` (for example `(1,-1,1,-1)`). There `apply` returns 1 exactly (spec), and the f64 bisection reference is wrong because `1 - m^3` rounds to 1 for `m < ~5e-6`. This is a reference artefact, not a defect; the implementer's grid skipping `u >= 1.0` is justified (`e(1)=1` is pinned by the exact-ends tests).

### Criteria map

| # | Criterion | Proof | Status |
|---|---|---|---|
| 1 | Workspace entry for `gafira-anim` (§2.1, layer 1) | `Cargo.toml`; builds | covered |
| 2 | V5 row (§2.2) | `ci.yml` row; V5 replay exit 0 | covered |
| 3 | `clippy.toml` copy | V12 replay exit 0 | covered |
| 4 | `#![forbid(unsafe_code)]` | `src/lib.rs`; forbid replay exit 0 | covered |
| 5 | ARQ-17 `Linear` identity | `arq_17_linear_is_the_identity` | covered |
| 6 | ARQ-17 named == `CubicBezier` bit for bit, 1 001 `u` | `arq_17_named_curves_equal_their_cubic_bezier_bit_for_bit` | covered |
| 7 | ARQ-17 `e(0)=0`, `e(1)=1` exact | `arq_17_bezier_ends_are_exact`, `arq_17_every_curve_is_exact_at_the_ends` | covered |
| 8 | ARQ-17 precision vs f64 bisection | `arq_17_bezier_matches_f64_reference_within_tolerance`, `..._precision_grid_of_extreme_control_points`, `..._precise_for_tiny_u_with_flat_start`; probes agree | covered |
| 9 | ARQ-17 hull bounds | `arq_17_bezier_stays_within_bounds`; probes agree | covered |
| 10 | `CubicBezier` validity rule | `arq_17_cubic_bezier_validity_rule` (boundaries both sides) | covered |
| 11 | ARQ-17 `Hold` step (amended) | `arq_17_hold_is_a_step_that_reaches_one_only_at_the_end` + exact-ends test | covered |
| 12 | ARQ-17 input clamp, NaN -> 0 (amended) | `arq_17_every_curve_clamps_its_input_and_maps_nan_to_zero` | covered |
| - | serde names (§3.4 format) | `arq_17_serde_names_follow_the_spec_enum` | covered (extra) |

Total 12, covered 12. Product ID in D1 scope: ARQ-17 (all tests named `arq_17_*`). ARQ-12, ARQ-16, ARQ-18, ARQ-19 are left to D2/D3 by the card.

### Hand mutants (round-1 set re-run, scratch export, `src/easing.rs`)

| ID | Mutation | Implementer tests | Probes | Reading |
|---|---|---|---|---|
| M1 | x1 range `0.0..=1.0` -> `0.0..1.0` | killed | killed | C2 fixed |
| M2 | `X_EPSILON` 1e-12 -> 1e-6 | killed | killed | C3 fixed |
| M3 | drop final `y.clamp(min, max)` | survived | survived | equivalent (W3) |
| M4 | drop `next > lo && next < hi` guard | survived | survived | equivalent on all searched inputs (W3) |
| M5 | NaN `u` -> 1.0 | killed | killed | W1 fixed |

Run 5, killed 3 by the implementer suite. Extra check R0 (revert the stop to round-1 absolute `< 1e-12`, not counted as a mutant): killed (3 implementer tests fail).

### WARNING

- **W3** class: implementation. ref: `crates/gafira-anim/src/easing.rs` `bezier` (hull clamp, Newton bracket guard); hand mutants M3, M4. Both still survive tests and probes. Extra search this round (3 M curves, x biased to 0/1/log-small, `u` uniform and log-uniform): M4 gives 0 precision violations; without the clamp the guarded solver's f32 result never left the hull (0 of 3 M). Accepted as equivalent and documented in `apply-progress.md`; not blocking. The implementer's M4 rationale is slightly off: without the guard the bisection fallback only triggers on a tiny slope, so equivalence rests on the empirical search, not on the bracket argument (M4 hits the 64-iteration cap 208 928 times in that search and still meets tolerance).

### SUGGESTION

- **S1** ref: `easing.rs` `bezier`, slope threshold `d.abs() > 1e-9`. With `x1 ~ 0` and `u < ~6e-15` (x1 = x2 = 0) or `u < ~1e-19` (x1 = 0, x2 >= 0.1), the slope at the root is below the absolute 1e-9, so Newton is never taken, pure bisection cannot reach the relative stop and the loop runs all 64 iterations. Results stay within tolerance (bisection width 2^-64), so this is cost only (64 cheap iterations, still far below the ARQ-12 1 µs budget). A relative slope test or a stop on bracket width would end earlier.
- **S2** ref: `apply-progress.md` `## D1 round 2`, W3/M4 bullet. Reword the equivalence argument to rest on the search evidence (see W3), so it does not read as a proof.
- **S3** ref: `arq_17_named_curves_match_css_reference_values` (carried from round 1). It still compares to the bisection reference, not to published CSS values; two literals (`ease-in(0.5) ~ 0.3153`, `ease-out(0.5) ~ 0.6847`, both confirmed by the probes) would make it independent.
- **S4** ref: size. 501 lines is over the ~400 lines at which `AGENTS.md` suggests `chained-pr` review slices; the coordinator may want to keep D2/D3 closer to their forecasts.

next: tdd-apply (D2)
