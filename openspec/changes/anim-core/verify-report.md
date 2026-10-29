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

## T4 fix round (final verify, change level)

Flow: standard | mutation: archive | tier: T4 | verify: V2. Branch `feat/anim-core-t4` from `develop` at `9572a06`; commits `568e79d`, `5eb8cb2`, `f0f851a`, `adef88e`, `22fd047`. Judged on a clean `git archive` export of `22fd047` (files touched after extraction, own `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`). This is the G5 check before archive.

## Gates (clean export of `22fd047`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `cargo deny check bans licenses sources` | 0 |
| CI policy: forbid(unsafe_code), V5 layer table, V12 clippy.toml sync (steps extracted from `.github/workflows/ci.yml`, run on the export) | 0, 0, 0 |

Hygiene: no `src/` change in the round (`9572a06..HEAD` touches `tests/transform.rs` +76, `.cargo/mutants.toml` +27, `apply-progress.md` +33, all pure additions); the Slice card is unchanged (0 deleted lines in `apply-progress.md`); no AI attribution in commit messages or added lines; no new `*.proptest-regressions`. Size of the round: 136 lines.

## T4 report

Source: the implementer's re-run without `CARGO_TARGET_DIR` (`scratchpad/mutants-t4c/mutants.out`, cargo-mutants 27.1.0, `--jobs 8`): 293 tested, 279 caught, 0 missed, 0 timeout, 14 unviable, 19 excluded (312 listed), wall 194 s. The first run (`scratchpad/mutants-t4`) shared one target dir across jobs and is not used.
Validity of the re-run for HEAD: it started at 20:43:11Z, the commit time of `f0f851a`/`adef88e`; `src/` is identical in every commit since `9572a06` and `22fd047` changes only docs. The tested+unviable set equals my `cargo mutants -p gafira-anim --list` on the export of HEAD (293 lines, identical), and the mutants killed only by `f0f851a` (107:33, 108:33, 157:28) are in `caught.txt`, so the run included that test. I did not re-run T4.
Unviable (14): all `Default::default()` / `Vec::leak` return replacements for types without `Default`; not a finding.
critical_missed: 0.

## 1. Group A (`Parts::borrowing`, interpolate.rs 186-187)

Expected values re-derived in python3 f64 as `S = U^-1 R^T L` with `L = [[2, 4], [1, 2]]`, theta 0.7, k 0.8: `S11 = 2.5927766115593833`, `S22 = -1.047186374381787`, blend at t = 0.5 gives `[1.7563746546470602, 1.4793739633490726, -0.03158084084029318, 1.2500119533891993, 2, 1]`; the test's literals agree within 1.3e-14 (tolerance 1e-9). Independent of the code.
Each of the 8 mutants applied by hand on a scratch export, then the new test alone (`--exact`) and the rest of the suite (`--skip` the new test):

| Mutant | New test | Other failing tests |
|---|---|---|
| 186:43 `*` -> `/` | killed | 0 |
| 186:48 `-` -> `/` | killed | 12 |
| 187:29 `*` -> `+` | killed | 12 |
| 187:19 `*` -> `+` | killed | 12 |
| 186:43 `*` -> `+` | killed | 6 |
| 187:24 `-` -> `+` | killed | 0 |
| 187:19 `*` -> `/` | killed | 0 |
| 187:24 `-` -> `/` | killed | 11 |

All 8 are killed by the new test. Five of them were already killed by older tests (for example `d + co` is non-zero even with `d = 0`), so they were false "missed" from the shared-target run, and the apply-progress explanation that every one "changes nothing" holds only for three (S1).

## 2. The 19 exclusions

Regex check: each of the 22 `exclude_re` entries was matched against `cargo mutants --list --no-config` for gafira-anim (312) and for the whole workspace (1488). Every one of the 19 new entries matches exactly one mutant, workspace-wide; `--list` with the config drops exactly those 19 (293 left). The 3 older entries match 0 in gafira-anim and 1 each workspace-wide.

Independent probe: a standalone Rust copy of `bezier` with each of the 16 easing mutants selectable, against a 200-step f64 bisection reference with the ARQ-17 bound `1e-5 * max(1, |y1|, |y2|)`, over f32 inputs only: (a) a grid of 50 x1 by 50 x2 values (0, 1, 2^-k down to the smallest subnormal, 1 - 2^-k for k = 1..24, the CSS values) by ~3 900 u values (2^-k and 1.5 * 2^-k for k = 1..149, the 199 smallest subnormals, the 2 999 floats below 1, j/1000) by 8 y pairs; (b) 3M random cases; (c) 20M adversarial cases (x at 0, 1, subnormal, last 1 000 floats below 1, log-uniform; u in the last 5 000 floats below 1 or log-uniform over all positive floats), counting cases where a mutant fails while the original passes.

| Exclusion | Argument | Verdict |
|---|---|---|
| 82:16 `<` -> `<=` | `err == 0` breaks first (`1e-12 * u > 0` for every f32 `u > 0`) | proved, equivalent |
| 88:22 `-` -> `+`, `slope -> 0.0`, `slope -> -1.0` | every Newton candidate is rejected, so pure bisection with the same stop rule | proved (sign argument holds: `d >= 0` inside `[0, 1]`) |
| 88:22 `-` -> `/` | negative candidates rejected; accepted ones lie in the bracket | bracket kept, shrink rate not bounded; measured: 0 extra failures in 23M cases |
| 89:24/39/52 `>=`, `<=` | differ only at an exact tie | sound; the probe found identical `s` on every case |
| 89:24/39/52 `==` | comment says "changed only at an exact tie" | conclusion holds, reason is wrong: `==` turns Newton off everywhere (pure bisection), see S2 |
| 89:24 `<`, 89:39 `<`, 89:52 `>`, 89:31 `\|\|`, 89:44 `\|\|` | Newton converges and the bracket is kept | not proved; measured, 0 extra failures in 23M f32 cases, worst mutant-only error 7.0e-8 (W1) |
| interpolate 157:48 `\|\|` -> `&&` | finite coefficients: an overflowing `det` implies an overflowing `area` (Hadamard `\|det\| <= \|c1\|\|c2\|` and `\|c1\| >= \|a\|`, `\|c2\| >= \|d\|`) | proved, equivalent |
| interpolate 195:19 `>=`, 197:26 `<=` | tie at `\|dtheta\| = pi`, both short paths | not equivalent: the tie is reachable with common input (identity to `scale_xy(-1, 1)` gives `dtheta = pi` exactly; t = 0.5 is `[0, 1, 0, 0]` vs `[0, -1, 0, 0]`); allowed only because §3.4 leaves the tie open (W2) |

Input domain: `Easing::apply(u: f32)` and `bezier(.., u: f32)`; the only caller (`animatable.rs:72-73`) narrows `u` to f32 before `apply`. The 89:44 excursion at `u = 1 - 1e-9` is not reachable; the probe found no f32 input past the bound for any of the five weakest mutants.

The probe found something else: the **original** solver misses the ARQ-17 bound at a valid input (C1).

## 3. `f0f851a` (non-finite coefficient pinned)

107:33, 108:33 and 157:28 are observable only with a non-finite coefficient: an input outside the contract (ARQ-18 rejects such a track; Q-33 defines degeneracy only "for finite coefficients"; Q-32 calls non-finite results out of contract). They are not equivalent, so an exclusion would break the AGENTS.md rule "exclude equivalent mutants only". Keeping the test is the right tool: it is labeled as out of contract, cheap, and kills the 3 mutants. The root cause is that the spec does not say what `Interpolate` does with a non-finite end, so the test now defines it. Class `ambiguity` (W3), not `implementation`.

## 4. Line:column anchors (maintenance note, not a finding)

cargo-mutants does not warn about an `exclude_re` that matches nothing. When lines move, the excluded mutants come back and show as missed (loud, not silent). A silent wrong exclusion would need the same operator, replacement and function at the same line and column. That is unlikely, but it can happen if line 89 is rewritten in place. C1 changes `bezier`, so all 16 easing entries must be re-derived after the fix, and T4 must run again.

## Criteria (change level)

| ID | Proof | Status |
|---|---|---|
| ARQ-12 | 12 `arq_12_*` tests (`sampling.rs`, `golden.rs`, structural plus golden) | covered (cross-platform part deferred per card) |
| ARQ-16 | 15 `arq_16_*` tests (`sampling.rs`) | covered |
| ARQ-17 | 13 `arq_17_*` tests (`easing.rs`) | failing: the precision rule does not hold at a valid input (C1) |
| ARQ-18 | 13 `arq_18_*` tests (track, serde, interpolate, transform) | covered |
| ARQ-19 | 51 `arq_19_*` tests (color, interpolate, transform) | covered (tie at pi open, W2) |

## Hand mutants (3, scratch export, restored and compared afterwards)

| # | Mutant | Result |
|---|---|---|
| HM1 | `X_EPSILON` 1e-12 -> 1e-9 | killed (`arq_17_bezier_matches_f64_reference_within_tolerance`, `..._precision_grid_of_extreme_control_points`) |
| HM2 | absolute stop `\|err\| <= X_EPSILON` | killed (3 ARQ-17 tests plus the golden) |
| HM3 | Group A spot checks by hand (8, table above) | killed |

## CRITICAL

- **C1** class: implementation. `crates/gafira-anim/src/easing.rs:79` (stop rule) against ARQ-17 / §3.4 "Precisión" (`|e(u) − y(s*)| ≤ 1e-5 · max(1, |y1|, |y2|)`, `s*` the exact root). The only stop test is `|x(s) − u| <= 1e-12 · u`, a bound on `x`, not on `s`. Near a flat inflection `x(s) − u ≈ 4h³`, so an `x` error of 5e-13 allows `h ≈ 4.4e-5` in `s`. Valid failing input, through the public API: `Easing::CubicBezier { x1: 1.0, y1: 0.0, x2: 2^-40 (9.094947e-13), y2: 1.0 }.apply(0.5)` returns `0.5`; the exact root (python3 `Fraction` bisection) gives `y(s*) = 0.49993398099166364`, error 6.60e-5 against a bound of 1e-5 (6.6 times over). Also failing: `x2 = 2^-39` (1.10e-5) and `2^-44` (2.62e-5); about 10 values of `x2` between 2^-49 and 2^-39 fail at `u = 0.5`. The solver breaks on the first iterate (`err = 0.375·x2 <= 5e-13`). Every one of the 16 excluded mutants fails on the same inputs, so this is a defect of the code, not of a mutant. The grid test uses `x` in `{0, 1e-6, .., 1 − 1e-6, 1}` and never reaches `x2 ~ 1e-12`. Fix direction (tdd-apply): a stop rule that bounds the error in `s` (for example, keep going until the bracket is narrow enough, or stop on `|err| <= ε` only when the slope is not tiny), with a unit test on this input. Then re-derive the 16 easing exclusions and re-run T4.

## WARNING

- **W1** class: implementation. `.cargo/mutants.toml` easing entries 89:24 `<`, 89:39 `<`, 89:52 `>`, 89:31 `||`, 89:44 `||` (and 88:22 `/`). They are excluded as equivalent on measured evidence, not on a proof. My independent search (23M f32 cases plus a grid; details in section 2) found no killing input, and the worst error that only a mutant produced was 7.0e-8. I accept them as equivalent mutants in critical code, documented here as AGENTS.md requires. They must be re-checked after the C1 fix.
- **W2** class: ambiguity. §3.4 Transform2D says "la rotación por el camino corto (`|Δθ| ≤ π`)" but does not settle `|Δθ| = π` exactly. The tie is reachable with ordinary input: identity to `scale_xy(-1, 1)`, a horizontal flip, gives `dtheta = π`, and the result at t = 0.5 depends on the direction (`[0, 1, 0, 0]` or `[0, −1, 0, 0]`). That result is user-visible. `arq_19_transform_horizontal_flip_from_identity_goes_through_a_half_turn_frame` accepts both, and the exclusions 195:19 and 197:26 rest only on this gap. Specifier: fix the tie (for example, keep `Δθ = +π` as computed); tdd-apply then pins it and removes the two exclusions.
- **W3** class: ambiguity. `f0f851a` pins behavior for an end with a non-finite coefficient (a plain six-coefficient blend), and the spec does not state it (Q-33 covers finite coefficients only; ARQ-18 and Q-32 put such values out of contract). The test is acceptable and better than excluding (section 3). Specifier: add one line to §3.4 saying either that a non-finite end gives the six-coefficient blend, which the test already pins, or that the result is unspecified apart from never panicking. If it is the second, the test should be relaxed and the 3 mutants handled again.

## SUGGESTION

- **S1** `apply-progress.md` "Group A" says all 8 mutants changed nothing because `b = c = d = 0`; that holds for 186:43 `* -> /`, 187:24 `- -> +` and 187:19 `* -> /` only. The other 5 were artifacts of the shared target dir.
- **S2** `.cargo/mutants.toml` comment for the `==` variants: "changed only at an exact tie" is wrong for `==`. It makes the guard true only at the tie, so Newton is never taken (pure bisection, the same argument as `slope -> 0.0`). Reword when the entries are re-derived after C1.

## Notes

- Tooling (not a finding, already resolved by the re-run): the first T4 run shared one `CARGO_TARGET_DIR` across jobs on the coordinator's instruction, which AGENTS.md forbids. It gave false "missed" results (5 of the 8 Group A mutants are killed by older tests) and, per the implementer, false "caught" results. Do not cite it. The re-run after C1 must use the default per-job target.
- With `mutation: archive`, T4 at close is the only cargo-mutants run; after the C1 fix it must run again on gafira-anim (default per-job target, no `CARGO_TARGET_DIR`).
- The ARQ-12 golden on 3 CI platforms is still deferred, as the card accepts (README follow-up 2).
- Probe sources (outside the repo): `scratchpad/verify-final/probe/src/main.rs`, `scratchpad/verify-final/apiprobe/src/main.rs`.

Verdict: FAIL (1 CRITICAL, 3 WARNING, 2 SUGGESTION). T4: 0 missed, 0 timeout, critical_missed 0; the 19 exclusions match one mutant each and are accepted (W1 measured, W2 spec tie), but ARQ-17 fails at a valid input (C1), so the change cannot be archived yet.
Per class: implementation 2 (C1, W1), ambiguity 2 (W2, W3), architecture 0, claim 0.

next: specifier, tdd-apply (specifier settles W2 and W3 first so that one tdd-apply round fixes C1, pins the tie and re-derives the exclusions; then T4 again and verify)
