# Verify report: anim-core D3c

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

## D3c round 2 (closure only)

Flow: standard | mutation: archive | tier: T4 | verify: V2. Owner-limited scope: closure of the round-1 findings W1, W2, W3, S1-S4 only; no hunt for new gaps. No cargo-mutants report (expected with `mutation: archive`; not a finding).
Scope: `afb7751..02cf69e`: `39c2253` (`tests/golden.rs` +242/-105, `tests/golden/sample.json` +4/-1), `02cf69e` (`apply-progress.md` +18/-0, pure append). Judged on a clean `git archive` export of HEAD (files touched after extraction, own `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`).

## Gates (clean export of `02cf69e`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `cargo deny check bans licenses sources` | 0 |
| CI policy (forbid(unsafe_code), V5 layer table, V12 clippy.toml sync) | 0: no manifest, `clippy.toml`, `deny.toml` or `.github` change since `afb7751`; no `src/` change |

Hygiene: no `src/` file changed (`git diff afb7751 HEAD -- crates/*/src` is empty); no Slice card or spec change (the `apply-progress.md` diff has 0 deleted lines); no AI attribution in commit messages or added lines; no new `*.proptest-regressions`.
Data file: the diff has 1 deleted line, `out_of_gamut_no_clamp`, and the new version equals the old one plus a trailing comma; the 3 new color cases are appended at the end of the color group. Every other existing line keeps its bytes.

## Closure

| Finding | Evidence | Status |
|---|---|---|
| W1 bit for bit at no-interpolation instants | `Golden::matches(got, want, exact)`; Color and Transform2D accept the tolerance only when `exact` is false. `is_exact_instant` (golden.rs:485-498) is true for a static track, `t <= first.at`, `t >= last.at`, `t` equal to a key `at`, or `t` inside a segment whose `out` is `Hold`, which matches the §3.4 bullet and the `sample_keys` branches in `src/animatable.rs:55-74`. Unit tests `arq_12_color_and_transform_are_bit_exact_where_no_interpolation_runs` and `arq_12_exact_instants_are_static_outside_range_keys_and_hold`. Re-run of the round-1 H1 (an interior key returns `from.interpolate(to, 1e-30)`) through the golden alone, one type at a time: Transform2D is now killed (`easing_0` at t=1e9: `sx` 1.22e-16 against a stored 0.0, a difference the tolerance would accept); Color killed. H1b (the Hold interior returns `interpolate(.., 1e-30)`): Color killed (`easing_2`) and Transform2D killed (`easing_4`). f32 and Vec2 pass H1 because `a + (b-a)*1e-30` rounds back to `a`; they were already compared bit for bit. | closed |
| W2 coverage test | `covers` runs per type: required names (`static`, `easing_0..5`, plus the Q-28/Q-30/Q-31/Q-33, `premultiplied_alpha`, `rotation_350_to_10`, `skew`, the S1/S2 cases, and `extreme_ats` for f32), a Static track per type, `i64::MIN`/`i64::MAX` sampled per type, samples at `at-1`, `at` and `at+1` for every key, a sample strictly inside every segment, and all 6 easings as a segment `out`. `extreme_ats` is asserted to have keys at `i64::MIN` and `i64::MAX`. H5 re-run: deleting the 5 cases together is killed ("f32: missing case extreme_ats"), and so is deleting each one alone (4 named "missing case" failures; dropping `extreme_ats` by line also breaks the JSON). The round-1 data probes are now killed: D3 (drop an `at+1` sample) gives "color easing_2: no sample at 1"; D4 (Vec2 loses `static`) gives "vec2: missing case static"; D5 (Transform2D loses its `i64::MAX` samples) gives "transform: extreme instants not sampled". Judgment on `extreme_ats` being f32-only: it meets W2. Keys at the ends of the i64 range only exercise the generic i128 arithmetic in `sample_keys`, which is the same for every `T`. The per-type value paths at extreme instants are covered by the per-type `i64::MIN`/`i64::MAX` requirement. ARQ-12 and §3.4 do not ask for extreme keys per type. | closed |
| W3 size | See the size note. | note |
| S1 same-space P3 and linear sRGB | `same_space_display_p3` and `same_space_linear_srgb` are required in the coverage test and present in the data. | closed |
| S2 f32 against f64 alpha rounding | `alpha_f32_rounding` (alpha 0.9 -> 0.1, Linear). H2 re-run (alpha computed in f64, then narrowed): the golden alone kills it (`alpha_f32_rounding` at t=996093750, alpha bits 1037251379 against 1037251384); the ARQ-19 color tests also kill it. | closed |
| S3 comment | The `covers` doc and comments now describe every key, per type; no "first key" wording is left about coverage. | closed |
| S4 last key's `out` | Easings are collected from `keys.windows(2)` as `w[0].out` only (golden.rs:577-585). | closed |

Spot-check of the implementer's table in "## D3c round 2": H2 confirmed (killed by the golden alone and by the ARQ-19 color tests); H5 confirmed (killed only by the coverage test, with the same message). The rows W1 and W1b match my H1 and H1b re-runs above.

## Hand mutants (4, scratch export, restored after each; the export was confirmed identical to the repo afterwards)

| # | Mutant | Result |
|---|---|---|
| H1 | interior key `t == from.at` returns `from.interpolate(to, 1e-30)` | killed by the golden for Color and Transform2D |
| H1b | Hold interior returns `from.interpolate(to, 1e-30)` | killed by the golden for Color and Transform2D |
| H2 | Color alpha in f64, narrowed to f32 | killed by the golden alone (`alpha_f32_rounding`) |
| H5 | delete the 5 claimed cases (together, then each alone) | killed by `arq_12_golden_file_covers_the_required_cases` |

## CRITICAL

None.

## WARNING

None. W1 and W2 are closed, and W3 is now a size note.

## SUGGESTION

None. S1-S4 are closed.

## Notes

- Size: D3c is now 881 lines, all of them test and data (`tests/golden.rs` 825 + `tests/golden/sample.json` 56, from `36282c5`), plus 33 lines of progress notes. That is about 900 against the card forecast of ~200 (x4.4) and above the ~700 delivery mark. It is test-only and readable case by case; this is a forecasting note, not a split request.
- ARQ-12 on the 3 CI platforms is still deferred, as the card accepts (README follow-up 2).

Verdict: PASS (0 CRITICAL, 0 WARNING, 0 SUGGESTION). D3c is the last delivery of the change. Per the plan, the T4 mutants run on gafira-anim (`mutation: archive`) comes before close.
Per class: implementation 0, architecture 0, ambiguity 0, claim 0.

next: archive
