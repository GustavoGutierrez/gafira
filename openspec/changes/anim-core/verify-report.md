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

## D3c round 1

Flow: standard | mutation: archive | tier: T4 | verify: V2. No cargo-mutants report (expected with `mutation: archive`; not a finding). Hand mutants and probes only.
Acceptance source: Slice card D3c row (amended by the coordinator in `e0e2b49`, reason stated) and 02 ARQ-12 golden criterion plus the §3.4 determinism table (including the bullet "en `t` igual al `at` de un key, fuera del rango de keys y con `Hold`, ... bit a bit ... para todos los tipos").
Scope: `36282c5..84d5f17`: `b861cb1` (`tests/golden.rs` +688, `tests/golden/sample.json` +53 lines / 92 KB), `84d5f17` (`apply-progress.md` +15, pure append). Judged on a clean `git archive` export of HEAD.

## Gates (clean export of `84d5f17`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `cargo deny check bans licenses sources` | 0 |
| CI: forbid(unsafe_code), V5 layer table, V12 clippy.toml sync | 0, 0, 0 |

Hygiene: no AI attribution in commit messages or files; no new `*.proptest-regressions` (the four in gafira-anim predate D3c). Slice card: the `apply-progress.md` diff is a pure append (`## D3c progress`); no implementer commit touched the card.

## Criteria

| # | Criterion | Proof | Status |
|---|---|---|---|
| 1 | Golden file of tracks of the four §3.4 types | `tests/golden/sample.json` (f32 8, Vec2 7, Color 11, Transform2D 17 cases) | covered |
| 2 | Sampled `t` values with stored expected results | `samples[{t, bits}]`, IEEE bit patterns; `arq_12_golden_file_matches_sample_on_this_platform` | covered |
| 3 | `f32` bit for bit | `bits_equal`; implementer M1 and M5 killed (M5 spot-checked, D1 below) | covered |
| 4 | `Vec2` bit for bit (both components) | `bits_equal` over `[x, y]`; probe P7 (y-only drift 1e-12) killed by the golden | covered |
| 5 | Easing curves bit for bit | indirect through f32/Vec2 tracks; probe P3 (EaseIn constant) and hand mutant H3 (curve output +1 ULP, killed only by the golden) | covered |
| 6 | `Color` r,g,b within `4·2^-23·max(1,|c|)`, alpha bit for bit, same space | `color_within` (tag, alpha bits, per-channel tolerance); probe P1 (1e-5 drift) killed | covered |
| 7 | `Transform2D` within `1e-9·max(1,|c|)` | `transform_within`; probe P2 (1e-8 relative on one coefficient) killed | covered |
| 8 | §3.4: key instants, out of range and `Hold` bit for bit for every type | f32/Vec2 bit for bit; Color/Transform2D only within tolerance (W1) | covered, weak |
| 9 | Run on the 3 CI platforms | Linux x86_64 only, CI disabled (README follow-up 2) | deferred, accepted in the card |

Total 9: 8 covered (one weak), 1 deferred by the card.

## Independence of the expected data

The expected data is a snapshot of the current implementation: `authored()` builds the tracks and `case()` stores `track.sample(t)` for each instant. I regenerated with `GAFIRA_BLESS_GOLDEN=1` and the output is byte-identical to the committed file. That is acceptable for a cross-platform golden (its job is to catch divergence between platforms and future regressions, not to prove correctness), and the test is not circular at check time: the check reads tracks and expected bits from the committed file only, and `authored()` runs only under the switch. Correctness comes from the ARQ-16/17/19 tests. As an independent cross-check I recomputed from the JSON with a Python script (`scripts/indep.py` in the scratchpad): Q-33 rounding det `[0.55,0,0,1.55]`, exact det 0 `[1.05,0,0,3.8]`, `1e200` `[5e199,0,0,1e200]`, both relative-threshold examples, rotation 350->10 = identity, Q-30 `[1,0,0,2.5]`, Q-28 degenerate end and both degenerate, all within 1e-9; Q-31 zero alpha straight blend, premultiplied red->transparent `(1,0,0,0.5)`, negative-alpha samples (alpha -0.125 at the overshoot peak) match a straight linear-light extrapolation to f32 precision; EaseIn f32 segment within 6.2e-7 of an independent bisection reference. The snapshot is therefore also right where the spec states numbers.

## Implementer deviations

1. Compile-time `option_env!("GAFIRA_BLESS_GOLDEN") == "1"` that prints between markers: acceptable. It respects the ARQ-02 purity lint without `#[allow]`, is documented in the module docs and the progress notes, only the exact value `1` counts (`GAFIRA_BLESS_GOLDEN=0` printed nothing; `arq_12_regenerate_switch_is_opt_in`), it never writes, and the check always compares against the committed file, so a bless run cannot silently pass a mismatch. The extraction one-liner works.
2. Q-31 negative alpha through a bezier overshoot: consistent with ARQ-16 and §3.4. `sample` gives `u` in (0, 1) and passes `e(u)` unclamped to `interpolate`; §3.4 states "con una curva con rebote, `a` puede salir negativo". The data holds three negative-alpha samples.
3. Size: see W3.

## Probes (step 1c, derived from the criteria before reading the tests; scratch export)

| Probe | Change | Golden result |
|---|---|---|
| P1 | Color channels x(1+1e-5) | killed |
| P2 | Transform2D `sx·cos` x(1+1e-8) | killed |
| P3 | EaseIn constant 0.42 -> 0.42000002 | killed |
| P7 | Vec2 y uses `t + 1e-12` | killed |
| D3/D4 | data: drop the at±1 and at±1e6 samples; keep `static` only in f32 | coverage test passes (part of W2) |

## Hand mutants (5, scratch export, full gafira-anim suite)

| # | Mutant | Full suite | Golden alone |
|---|---|---|---|
| H1 | interior key instant returns `from.interpolate(to, 1e-30)` instead of the exact value | killed (`arq_12_sample_calls_interpolate_at_most_once`) | Transform2D cases pass (W1); Color killed only by the space tag |
| H2 | Color alpha computed in f64 then narrowed | killed (ARQ-19 alpha tests) | survives (S2) |
| H3 | easing output +1 ULP | killed only by the golden | killed |
| H4 | i64 differences in `sample` with `wrapping_sub` | killed (golden `extreme_ats`, `arq_16_adjacent_and_extreme_keys_are_total`) | killed |
| H5 | data: delete `q28_both_degenerate`, `q33_det_zero_exact`, `premultiplied_alpha`, `rotation_350_to_10`, `extreme_ats` | survives | coverage test passes (W2) |

Spot-check of the implementer's table: M5 (stored f32 bits 841731191 -> 841731192 in `easing_0`, `t = 1`) is killed by `arq_12_golden_file_matches_sample_on_this_platform`. Confirmed.

## CRITICAL

None.

## WARNING

- W1 (class: implementation, ref: `crates/gafira-anim/tests/golden.rs:111-124`, §3.4 bullet under the determinism table). The golden compares Color and Transform2D at key instants, outside the key range and under `Hold` with the tolerance, but §3.4 requires those results bit for bit for every type. H1 (key value replaced by a near value) passes every Transform2D case of the golden. The full suite catches H1 through the call-count test, so it is not a correctness hole today, but the golden does not apply the rule it claims to cover. Fix: compare bit for bit when `t` is a key `at`, `t <= first.at`, `t >= last.at`, or the segment's `out` is `Hold` (or store a per-sample "exact" flag).
- W2 (class: implementation, ref: `crates/gafira-anim/tests/golden.rs:491-578`, `apply-progress.md` "## D3c progress"). `arq_12_golden_file_covers_the_required_cases` is weaker than the coverage the notes say it asserts. It does not require `q28_both_degenerate`, `q33_det_zero_exact`, `extreme_ats` (the only keys at `i64::MIN`/`i64::MAX`), `premultiplied_alpha`, `rotation_350_to_10`; it checks names in a pooled list, so `static` in one type satisfies all four; it does not check instants around keys. H5 and D3/D4 pass it. Fix: per-type required names including the ones above, and assert at±1 samples exist for every key.
- W3 (class: implementation, ref: card D3c "~200 lines"). Measured 741 lines (688 test + 53 data lines, 92 KB) against a ~200 forecast (x3.7), and slightly above the ~700 delivery mark when the data is counted. Test-only and reviewable per case; noted for forecasting, no split asked.

## SUGGESTION

- S1: no same-space interpolation in Display-P3 or linear sRGB is in the data (interpolated results are tag 0 or 1; the 26 tag-2 results are key clones). Add a P3->P3 pair so the linear->P3 output path is in the golden.
- S2: no Color alpha case where the f32 formula and an f64 formula round differently (H2 passes the golden). Add an alpha pair such as 0.3 -> 0.7 at a non-dyadic `t`.
- S3: the comment at `golden.rs:564` says "the first key of every keyed track" but the code checks every key.
- S4: the easing coverage counts the last key's `out`, which `sample` never uses; restrict to `keys[..len-1]`.

Verdict: PASS (0 CRITICAL, 3 WARNING implementation, 4 SUGGESTION). D3c is the last delivery of the change; per the plan the T4 mutants run on gafira-anim (`mutation: archive`) comes before close.
Per class: implementation 3 (WARNING), architecture 0, ambiguity 0, claim 0.

next: archive
