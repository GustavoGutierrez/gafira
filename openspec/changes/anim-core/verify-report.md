# Verify report: anim-core D3a fix round

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.
D2a: PASS in round 1 at `d9d2e50` (0 CRITICAL, 2 WARNING ambiguity: `is_finite` missing from the §3.4 snippet and the f32/Vec2 overflow domain, both routed to specifier, 4 SUGGESTION); merged into `develop` at `37769a0`.
D2b: PASS in round 1 at `f30293b` (0 CRITICAL, 1 WARNING ambiguity: ARQ-12 golden assigned to no delivery, since assigned to D3c, 4 SUGGESTION); merged into `develop` at `c048b97`.
D3a: PASS in round 1 at `1dfa4a2` (0 CRITICAL, 1 WARNING W1 Display-P3 hue test, 3 SUGGESTION S1-S3); merged into `develop` at `4e1089e`, which then went RED on `arq_19_color_matches_premultiplied_reference`.

Flow: standard | mutation: archive | tier: T4 | verify: V2 on the findings only. No cargo-mutants report (expected with `mutation: archive`; not a finding). Hand mutants only.
Acceptance source: `docs/specs/02-arquitectura-software.md` §3.4 (Color row, determinism table) and ARQ-19 (Color part); D3a findings W1, S1, S2; the `develop` failure.
Scope: `4e1089e..8a9d36e` on `feat/anim-core-d3b`: `1c56418` (test-only: `tests/color.rs`, `tests/color.proptest-regressions`) and `8a9d36e` (appends `## D3a fix round` to `apply-progress.md`). No `src/` change (0 diff lines). Out of scope: D3b, D3c.

## Round 2 of D3a (fix round, commit `8a9d36e`)

### Verdict: PASS (delivery D3a; D3b and D3c remain)

Counts: 0 CRITICAL, 1 WARNING (ambiguity), 2 SUGGESTION.

### Method

- Disk free before building: 68 GB. Seven `git archive` exports of `8a9d36e` in the scratchpad (gates, 5 hand mutants, old-reference RED check), files touched after export, one `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`; all build folders freed with `cargo clean --target-dir`.

### (1) Diagnosis against §3.4 / ARQ-19

- Code (`src/interpolate.rs`, unchanged since D3a): `alpha = a.a.interpolate(&b.a, t)` in f32; branch `wr <= 0.0` and divisor `wr = f64::from(alpha)`. §3.4: the premultiplied channels "se dividen por su `a`" (the result's own alpha), and the determinism table pins `a` bit for bit (IEEE f32 only); ARQ-19: "`a` es la interpolación lineal de los dos alfas". The result's `a` is therefore the f32 value, and the code divides by exactly that. Diagnosis confirmed: the old reference divided by an f64 alpha, which is not the spec's divisor.
- RED reproduced on a scratch export with the old reference line restored: the saved case fails with `ch 2: [0.0, 0.0, 199.28341674804688, 0.00010126829147338867] vs [0.0, 0.0, 199.3598177686992]`. With the regression file absent (run outside the source tree) and fresh seeds, the old reference fails 50 of 100 runs at 2000 cases: the D3a property was flaky, not a one-off draw, and the cause was cancellation in the f32 alpha amplified by the division (not the branch split S1 anticipated).
- Tolerances diffed (`git diff 4e1089e..HEAD`, every `1e-` / `tol` line): `assert_close` and the property keep `1e-5 * max(1, |want|)` unchanged. The red-to-transparent-black test keeps `1e-5` but now applies it to the result space's own channels instead of linear sRGB, which is tighter (removes the ~1.7e-4 encoded-P3 slack of D3a W1; for sRGB, encoded `g`, `b` near 0 are tighter than linear). No tolerance loosened.

### (2) Hand mutants (5 run, 5 killed; `src/interpolate.rs`, Color impl, against the fixed tests)

| # | Mutant | Killed by |
|---|---|---|
| M1 | Straight instead of premultiplied (`if wr <= 0.0` -> `if true`) | `matches_premultiplied_reference` (property), `premultiplies_alpha`, `opaque_red_to_transparent_black_keeps_hue` |
| M2 | Wrong result space (never convert back: `if self.space() == to.space()` -> `if false`) | `result_space_rule`, `blends_in_linear_light`, `opaque_red_to_transparent_black_keeps_hue` (new space assert, W1) |
| M3 | Clamping (`(value as f32).clamp(0.0, 1.0)`) | property, `does_not_clamp_channels`, `negative_alpha_extrapolates_straight`, `transparent_to_transparent_blends_straight`, `opaque_red...` |
| M4 | Wrong `t` direction for channels (`let t = 1.0 - f64::from(t)`, Color only) | property, `blends_in_linear_light`, `does_not_clamp_channels`, `negative_alpha_extrapolates_straight`, `opaque_red...` |
| M5 | Alpha <= 0 branch (`wr <= 0.0` -> `wr < 0.0`) | `transparent_to_transparent_blends_straight` |

The property test itself kills M1, M3 and M4. M2 is invisible to it (it compares in linear sRGB) and M5 needs an alpha of exactly 0 (measure zero for continuous draws); both are killed by unit tests, as in D3a. Not a regression (see S1).

### (3) Flakiness

- proptest 1.12.0 (Cargo.lock) reads `PROPTEST_RNG_SEED` (`test_runner/config.rs`); the default is `RngSeed::Random`, and `tests/color.rs` uses the default config. Seed variation was proved: the old-reference binary fails on half the seeds and passes on the other half.
- Fixed tests, built once on the clean export:
  - `arq_19_color_matches_premultiplied_reference`, 200 distinct seeds x 2000 cases, run in the source tree (regression file replayed): 0 failures.
  - Whole `tests/color.rs` (11 tests), 200 more seeds (1001..1200), `PROPTEST_CASES` cycling 500..2300, run outside the tree (no regression replay): 0 failures.
  - The 10 ARQ-19 Color tests at 1,000,000 cases (seed 987654321) and the reference property at 4,000,000 cases (seed 42; run time scales linearly, so the case count is honoured): pass.
- About 5.4 million reference-property cases in total, 0 failures. Why it is stable now: the code and reference share the divisor and branch, so what remains is f32 narrowing and the space round trip, both relative errors near 1e-7, well inside `1e-5 * max(1, |want|)`.

### (4) Gates (clean export of `8a9d36e`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (no failures; `tests/color.rs` 11 passed) |
| `cargo deny check bans licenses sources` | 0 |
| CI: Domain crates forbid unsafe code | 0 |
| CI: V5, V12, V13, V14 | not re-run: the range touches no manifest, `Cargo.lock`, `clippy.toml`, `src/` or `gafira-edit` file; all 0 at D3a |

### (5) Commit and card checks

- `crates/gafira-anim/tests/color.proptest-regressions` is tracked and gained the failing case (`cc 76c539d9...`) in `1c56418`.
- `8a9d36e` adds one hunk at line 68 of `apply-progress.md` (the `## D3a fix round` section) and removes nothing; the `## Slice card` block is unchanged. No implementer commit changed the card.
- No AI attribution or Co-Authored-By in `4e1089e..HEAD`; conventional subjects.
- Size: 30 insertions, 17 deletions (tests 39 changed lines, regression file 1, notes 7).
- The note corrects its own D3a deviation about Display-P3 (W1 rationale), as asked.

### Findings map

| Item | Proof | Status |
|---|---|---|
| `develop` RED (ARQ-19 reference property) | reference alpha = f32 `a + (b - a) * t` widened; RED reproduced on old reference; 0 failures in about 5.4 M cases | fixed |
| D3a W1 (P3 hue test in the result space) | `opaque_red_to_transparent_black_keeps_hue` asserts `r.space() == space` and own channels `(1, 0, 0)` within 1e-5 for all three spaces; M2 now also killed there | fixed |
| D3a S1 (reference branch/divisor alpha) | same change as the RED fix | fixed |
| D3a S2 (Q-31 in every space) | `transparent_to_transparent_blends_straight` loops over `ColorSpaceId::ALL` (finite, `a = 0`, reference) and keeps the sRGB 0.5 check | fixed |
| D3a S3 (log-uniform arm) | not addressed; it was optional | open, optional |

### Findings

#### CRITICAL

None.

#### WARNING

- W1 (class: ambiguity) ARQ-19 Color criterion "difiere de la interpolación premultiplicada de referencia en `f64` en ≤ 1e-5": read on its own, "reference in f64" suggests an f64 alpha divisor, and under that reading the code fails (50 % of runs at 2000 cases; 0.076 on 199.36 for the saved case), while an implementation dividing by an f64 alpha but storing the f32 alpha would pass. The code and the fixed test follow the stronger reading (§3.4 "se divide por su `a`" plus the bit-exact `a`), which I accept. Suggested one-line spec change: "the reference divides by the result's `a` (the f32 interpolation of the alphas)". Non-blocking; specifier, optional.

#### SUGGESTION

- S1 ARQ-19 lists "el espacio del resultado sigue la regla de §3.4" among the property tests; it is proved by the exhaustive 9-pair unit test (the space domain is finite), which is enough. If wanted, the reference property could also assert `r.space()`, so it would kill M2 too.
- S2 D3a S3 (log-uniform arm for the f32/Vec2 between-ends generators) is still open and optional.

next: tdd-apply (D3b)
