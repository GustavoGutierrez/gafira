# Verify report: anim-core (final, change level)

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.
D2a: PASS in round 1 at `d9d2e50` (0 CRITICAL, 2 WARNING ambiguity: `is_finite` missing from the §3.4 snippet and the f32/Vec2 overflow domain, both routed to specifier, 4 SUGGESTION); merged into `develop` at `37769a0`.
D2b: PASS in round 1 at `f30293b` (0 CRITICAL, 1 WARNING ambiguity: ARQ-12 golden assigned to no delivery, since assigned to D3c, 4 SUGGESTION); merged into `develop` at `c048b97`.
D3a: PASS in round 1 at `1dfa4a2` (0 CRITICAL, 1 WARNING W1 Display-P3 hue test, 3 SUGGESTION); merged at `4e1089e`, then RED on `develop` (flaky Color reference property).
D3a fix round: PASS in round 2 at `8a9d36e` (0 CRITICAL, 1 WARNING ambiguity: ARQ-19 Color reference divisor not stated, 2 SUGGESTION; 5.4M property cases, 0 failures); merged into `develop` at `6710050`.
D3b round 1: FAIL at `33417d3` (1 CRITICAL C1 implementation: no opposite-sign determinant test, sign-on-`sx` mutant survived; 1 WARNING W1 ambiguity: "determinante 0" undefined in floats, settled as Q-33 B; 3 SUGGESTION S1-S3).
D3b round 2: FAIL at `33f1a4c` (1 CRITICAL C2 implementation: absolute-threshold mutant survived; 2 WARNING: W2 implementation 1e3 slack, W3 ambiguity unobservable `scale(1e-9)` example, settled in `6b37f9e`; 3 SUGGESTION S4-S6).
D3b round 3: FAIL at `e314c4f` (2 CRITICAL implementation: C3 `|c1|·|c1|` mutant H2 and C4 `<` mutant H1 survived; 2 SUGGESTION S7, S8).
D3b round 4: PASS at `a2684e5` (0 CRITICAL, 0 WARNING, 0 SUGGESTION; limited closure round); merged into `develop` at `36282c5`.
D3c round 1: PASS at `84d5f17` (0 CRITICAL, 3 WARNING implementation: W1 key-instant exactness compared with tolerance for Color/Transform2D, W2 coverage test weaker than claimed, W3 size; 4 SUGGESTION).
D3c round 2: PASS at `02cf69e` (0 CRITICAL, 0 WARNING, 0 SUGGESTION; closure-only round; W1, W2, S1-S4 closed; size note).
T4 fix round (final, change level): FAIL at `22fd047` (1 CRITICAL C1 implementation: bezier solver misses the ARQ-17 bound at a valid f32 input near the triple root; 3 WARNING; 2 SUGGESTION; T4 0 missed, critical_missed 0, 19 exclusions checked).
T4 fix round 2 (final, change level): PASS at `c49f2a7` (0 CRITICAL, 1 WARNING W1 claim: the bisection solver costs ~20x the old one, ARQ-12 1 µs at risk on the reference hardware; 3 SUGGESTION; T4 0 missed, critical_missed 0; C1, W1-W3, S1, S2 of the previous round closed).

## T4 fix round 2 (final verify, G5 before archive)

Flow: standard | mutation: archive | tier: T4 | verify: V2. Branch `feat/anim-core-t4`; commits since the last report (`b0857f5`): `f22f0c9` spec (ARQ-19 tie and non-finite end), `337864b` stop rule, `3a509ee` tie tests, `0566a3a` solver rewritten as plain bisection plus golden regeneration, `04ef3d1` exclusions dropped, `c49f2a7` progress notes. Judged on a clean `git archive` export of `c49f2a7` (files touched after extraction, own `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`; 63 GB free before building).

## Gates (clean export of `c49f2a7`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (62 test binaries, 0 failures) |
| `cargo deny check bans licenses sources` | 0 |
| CI policy: forbid(unsafe_code), V5 layer table, V12 clippy.toml sync (steps extracted from `.github/workflows/ci.yml`, run on the export) | 0, 0, 0 |

Hygiene: no AI attribution in commit messages or added lines; no new `*.proptest-regressions`; the Slice card is unchanged (`9572a06..HEAD` deletes 0 lines of `apply-progress.md`; the one edited line since `b0857f5` is the Group A note of the T4 round, S1). `crates/` is identical between `0566a3a` and HEAD. Size of the round: 94 insertions, 79 deletions (easing.rs 46, tests 43, golden 26, mutants.toml 25, spec 6, progress 27).

## 1. Closure of the previous findings

| Finding | Evidence at `c49f2a7` | Status |
|---|---|---|
| C1 ARQ-17 precision near the flat inflection | Former failing inputs through the public API, checked against an exact `Fraction` bisection (140 halvings): `x1 = 1`, `x2 = 2^-39, 2^-40, 2^-44, 2^-49`, `u = 0.5`, `y = (0, 1)` and `(-1, 1)`: worst error 0.022 of the bound (was 6.6 times the bound). New unit test `arq_17_bezier_is_precise_near_a_flat_inflection` (k = 30..60, both mirrors, 3 y pairs, 5 u). Full probe in section 2. | closed |
| W1 easing exclusions on measured equivalence | Newton, `slope` and every guard removed; all 16 easing entries dropped from `.cargo/mutants.toml`. The only comparison of the loop, 77:31 (`<=`, `==`, `>`), and every arithmetic mutant of lines 73-83 are in `caught.txt`. | closed |
| W2 tie at `\|Δθ\| = π` | §3.4 now fixes it (`Δθ` keeps the sign of `θ_b − θ_a`). `arq_19_transform_horizontal_flip_tie_at_pi_keeps_dtheta_as_computed` pins the four values of ARQ-19; I re-derived them (θ = π/2, `sy = 0` at t = 0.5; θ = π/4, `sy = 0.5` at 0.25 and 0.75). `interpolate.rs:195:19 >=` and `197:26 <=` are no longer excluded and are in `caught.txt`. Code (`blend`, lines 194-199) matches the rule. | closed |
| W3 non-finite end | ARQ-19 and §3.4 now state it. `arq_19_transform_end_with_a_non_finite_coefficient_blends_the_six_coefficients_linearly` matches the text: identity linear part, `tx` NaN and `+inf`, `ty = 4`, towards and from θ 2, k 0.8, sx 2, sy 3, translation (1, −2), t = 0.5, coefficients 0, 1, 2, 3, 5 within `1e-9 · max(1, \|want\|)`. | closed |
| S1 Group A note | Corrected in `apply-progress.md` (3 of 8 needed the new test). | closed |
| S2 `==` comment | Gone with the entries. | closed |

## 2. The rewritten solver (`crates/gafira-anim/src/easing.rs`)

Design read against §3.4 and ARQ-17: 53 halvings of `[0, 1]` on `coord(x1, x2, mid) < u`, then `y` at the bracket midpoint, clamped to `[min(0, y1, y2), max(1, y1, y2)]`, cast to f32. For `s` in `[0.5, 1)` the f64 spacing is 2^-53, so 53 halvings keep every midpoint exact and end at one ulp of width. The invariant `coord(lo) < u <= coord(hi)` holds for the computed `coord` (`coord(0) = 0 < u`, `coord(1) = 1 >= u`), so the root is bracketed up to the rounding of `coord`, about 4 ulps (all terms are non-negative for valid `x1`, `x2`, so no cancellation). Near the flat inflection (`x1 = 1`, `x2 = 0`, `x(s) − u = 4h^3`) that floor gives `h ~ 2e-6` and the worst `y` error, `|y'(0.5)| / sc ≤ 2.25` at `y = (−1, 1)`.

Independent probe (outside the repo, `scratchpad/verify-final2/probe/src/main.rs`): calls the real `Easing::apply` of the export and compares with a double-double reference (bisection over f64 `s` until the bracket is one ulp, `x(s) − u` and `y(s)` in double-double, sign wrong only when `|x(s) − u| < 1e-31`, so the reference error is below 1e-10 of the bound). All inputs are f32; y pairs include `(−1, 1)` (the worst slope ratio), `(−2, 3)`, `(3, −2)`, mixed, and in the random run log-uniform magnitudes from 1e-30 to 1e30.

| Run | Cases | Over the bound | Out of hull bounds | Worst error / bound |
|---|---|---|---|---|
| Flat inflection: `x1` in {1, 1 − 2^-k}, `x2` in {0, 2^-k down to the smallest subnormal}, both mirrors, 801 f32 `u` around the inflection value and 5 fixed `u`, 10 y pairs | 60 450 000 | 0 | 0 | 0.472 |
| Adversarial random (x at 0, 1, subnormal, last floats below 1, log-uniform; u in the last 5 000 floats below 1, log-uniform over all positive floats, around 0.5, uniform) | 24 000 000 | 0 | 0 | 0.472 |
| Grid (50 x1 by 50 x2 by ~4 100 u by 10 y pairs, as in the previous round) | 102 551 040 | 0 | 0 | 0.472 |

The worst case in all three runs is `CubicBezier { x1: 1, y1: −1, x2: 0, y2: 1 }.apply(0.5)` = `0.12499528...`; there `s* = 0.5` and `y(s*) = 0.125` exactly, error 4.72e-6, bound 1e-5. That is the f64 noise floor of the triple root, about half the bound.

Other rules, checked on the export through the public API:

- `e(0) = 0` and `e(1) = 1` exactly, and `u = −1, −inf, NaN, −NaN, −0` give `+0`, `u = 2, +inf` give 1, bit for bit, for all 7 curve kinds (`ends` mode, 0 mismatches). The early returns in `apply` come before `bezier`.
- Output bounds: 0 of 187M cases outside `[min(0, y1, y2), max(1, y1, y2)]`.
- Termination: a fixed 53-step loop, no data-dependent iteration count, no other loop.
- Determinism: plain IEEE `+ − *` and comparisons; `git grep mul_add crates/gafira-anim` finds nothing. Bit-for-bit golden passes on this platform; cross-platform is still the deferred ARQ-12 CI part.
- Cost (ARQ-12, rough, local i7-12700H laptop, release build, `black_box` inputs): one `CubicBezier` sample (EaseInOut constants, random `u`) 645-657 ns; the previous Newton solver on the same loop 32-34 ns; a branchless variant of the same bisection 465-479 ns (latency bound, about 9 ns per `coord`); one `sample` of a 10 000-key f32 track, all keys `CubicBezier`, 710-728 ns. See W1.

## 3. Golden regeneration (`crates/gafira-anim/tests/golden/sample.json`)

- Structure: a JSON walk of `b0857f5` against HEAD finds the same 46 cases (8 f32, 7 vec2, 14 color, 17 transform), the same names, tracks, sample lists and `t` values; no case and no sample removed or added. `tests/golden.rs` (with the coverage test) is untouched since `39c2253`.
- Changed values: 33 values in 14 samples. 7 f32 values (2 f32, 5 Color), each 1 ulp. 26 f64 values (Vec2, Transform2D): 16 of magnitude 3e-18 to 2e-17 (absolute change ≤ 2.1e-24) and 10 of magnitude 9e-9 to 10, changed by 1 to 5 ulps (absolute ≤ 2.2e-15). The implementer's note says all f64 values have magnitude ~1e-17; that holds for 16 of 26 (S1).
- Cause: every changed sample sits at `u ≤ 1e-9` (t = 1 or one flick after a key), and they reduce to 4 distinct (curve, u) pairs: EaseIn and EaseInOut at `u = 1e-9`, the custom bezier `(0.17, −0.6, 0.83, 1.9)` at `u = 1e-9` and `u = 3.33e-10`. New `e(u)` against the exact `Fraction` reference: errors of 2.8e-20, 2.8e-20, 8.5e-12 and 4.4e-12 of the bound. At these tiny `u` the new solver is less precise in relative terms (2^-53 absolute in `s`), but ARQ-17 is an absolute bound.
- Legitimacy: the file is a snapshot of this implementation used to compare platforms; §3.4 fixes precision, not the bits of a given solver. The regeneration used the documented `GAFIRA_BLESS_GOLDEN=1` procedure (`tests/golden.rs` header), sits in the same commit as the solver change and is explained there. Accepted.

## 4. Mutation (T4)

Source: the implementer's run, `scratchpad/mutants-t4d/mutants.out` (cargo-mutants 27.1.0, `--jobs 8`, no `CARGO_TARGET_DIR`, per-job build dirs `/tmp/cargo-mutants-gafira-*.tmp` in `debug.log`): 264 tested, 250 caught, 0 missed, 0 timeout, 14 unviable, 1 excluded, wall 131 s (21:16:55Z to 21:19:06Z).
Validity for HEAD: the run started at the commit time of `04ef3d1`; `crates/` is identical from `0566a3a` to HEAD. `sort(caught + unviable)` equals my `cargo mutants -p gafira-anim --list` on the export of HEAD (264 lines, `diff` empty). I did not re-run T4.
Exclusions: `exclude_re` has 4 entries; the only gafira-anim one is `interpolate.rs:157:48 || -> &&`, which matches exactly 1 mutant in `--list --no-config` for gafira-anim (265) and for the workspace (1 441), and is the only mutant dropped by the config. Its argument (an overflowing `det` implies an overflowing `area` by Hadamard for finite coefficients; a finite `det` with an infinite `area` is caught by the last term) is unchanged and was proved in the previous round. The other 3 entries are outside gafira-anim and unchanged.
Unviable (14): `Default::default()` / `Vec::leak` replacements for types without `Default`; not a finding.
critical_missed: 0.

## Hand mutants (5, scratch export `scratchpad/verify-final2/hm`, own target, restored and compared with `cmp` afterwards)

| # | Mutant (rule cargo-mutants cannot express) | Result |
|---|---|---|
| HM1 | `BISECTIONS` 53 -> 20 | killed (`arq_12_golden_file_matches_sample_on_this_platform`); the precision tests accept it, which is right: 20 halvings still meet the ARQ-17 bound |
| HM2 | `BISECTIONS` 53 -> 30 | killed (golden) |
| HM3 | `y` evaluated at `lo` instead of the bracket midpoint | killed (golden) |
| HM4 | `coord` written with `mul_add` (FMA), against the §3.4 "no `mul_add`" rule | survived; with it the probe still gives 0 of 84M cases over the bound or the hull, and the golden does not change. Not a spec violation in itself (IEEE fma is correctly rounded); the rule is a code rule no test can see (S2) |
| HM5 | clamp removed | survived; equivalent: the computed `y(s)` stays within f64 rounding of the convex hull (Bernstein weights) and the f32 cast absorbs it; probe 0 of 84M cases out of bounds. The clamp is defensive |

## Criteria (change level)

| ID | Proof | Status |
|---|---|---|
| ARQ-12 | 12 `arq_12_*` tests (`sampling.rs`, `golden.rs`, structural plus golden, regenerated golden checked in section 3) | covered (cross-platform part deferred per card; 1 µs bench is ARQ-40, see W1) |
| ARQ-16 | 15 `arq_16_*` tests (`sampling.rs`) | covered |
| ARQ-17 | 14 `arq_17_*` tests (`easing.rs`), including the new flat-inflection test; independent probe of 187M f32 cases, worst 0.47 of the bound | covered |
| ARQ-18 | 13 `arq_18_*` tests (track, serde, interpolate, transform) | covered |
| ARQ-19 | 51 `arq_19_*` tests (color, interpolate, transform), tie at π and non-finite end pinned as the new text says | covered |

## CRITICAL

None.

## WARNING

- **W1** class: claim. `crates/gafira-anim/src/easing.rs` (`BISECTIONS = 53`) against ARQ-12 "< 1 µs por muestra" with 10 000 keys on the reference hardware (§6.6: Core i5 10th gen or Ryzen 5 3600; FIJADA). The new solver costs about 650 ns per `CubicBezier` evaluation on a faster laptop CPU (i7-12700H), about 20 times the previous solver (33 ns), and one 10 000-key `sample` with `CubicBezier` keys takes 710-728 ns here, so the margin is about 30 % on a faster machine than the reference one. The apply-progress note says "the cost is 53 `coord` calls" but gives no measurement, and the real benchmark is ARQ-40 (`bench_anim_sample_10k`), not yet built. The cost is latency bound (a branchless version is still 470 ns), so it scales with the iteration count. Not blocking: ARQ-17 and ARQ-12 as testable now both hold. Before ARQ-40 (or now, if the owner prefers), either measure on the reference profile or cut the cost while keeping a bound on the error in `s` (for example a safeguarded Newton that stops on the bracket width, or about 24-30 halvings, which still meet the bound by the slope argument above; any of these re-blesses the golden and needs T4 again).

## SUGGESTION

- **S1** `apply-progress.md` "T4 fix round 2", Step 2: the golden note says the f64 changes are values of magnitude ~1e-17; 10 of the 26 have magnitude 9e-9 to 10 and moved by 1 to 5 ulps (absolute ≤ 2.2e-15). Harmless, but the note should say so.
- **S2** The §3.4 rule "no `mul_add`" is not observable by any test (HM4). If it matters for cross-platform bits, a one-line CI grep for `mul_add` in `crates/gafira-anim/src` would hold it.
- **S3** The ARQ-17 test reference (`tests/easing.rs:13`) is the same f64 bisection as the solver, so it shares the noise floor (4.7e-6 at the triple root) and cannot see it. One unit point with an exact expected value, `CubicBezier { 1, −1, 0, 1 }.apply(0.5)` against `0.125` within `1e-5`, would make a regression of that floor visible.

## Notes

- Probe and bench sources (outside the repo): `scratchpad/verify-final2/probe/src/main.rs`, `scratchpad/verify-final2/bench/src/main.rs`; hand-mutant script `scratchpad/verify-final2/hm.sh`.
- The ARQ-12 golden on 3 CI platforms is still deferred, as the card accepts (README follow-up 2).

Verdict: PASS (0 CRITICAL, 1 WARNING, 3 SUGGESTION). T4: 0 missed, 0 timeout, critical_missed 0; one exclusion left, checked. All deliveries of `anim-core` are done.
Per class: claim 1 (W1), implementation 0, architecture 0, ambiguity 0.

next: archive
