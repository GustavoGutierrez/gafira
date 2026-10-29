# Verify report: anim-core D3a

D1: PASS in round 2 at `0a2f336` (0 CRITICAL, 1 WARNING W3 equivalent bezier mutants, 4 SUGGESTION); merged into `develop` at `261dd77`.
D2a: PASS in round 1 at `d9d2e50` (0 CRITICAL, 2 WARNING ambiguity: `is_finite` missing from the §3.4 snippet and the f32/Vec2 overflow domain, both routed to specifier, 4 SUGGESTION); merged into `develop` at `37769a0`.
D2b: PASS in round 1 at `f30293b` (0 CRITICAL, 1 WARNING ambiguity: ARQ-12 golden assigned to no delivery, since assigned to D3c, 4 SUGGESTION); merged into `develop` at `c048b97`.

Flow: standard | mutation: archive | tier: T4 | verify: V2. No cargo-mutants report for this delivery (expected with `mutation: archive`; not a finding). Hand mutants only.
Acceptance source: Slice card as amended by `e0e2b49` (D3a row); `docs/specs/02-arquitectura-software.md` §3.4 (common Interpolate rules, Color row, Q-32 contract range, determinism table) and ARQ-19 (Color part and the Q-32 test-domain rule), as settled by `cf62a10`.
Scope: delivery commits `e21e561` (test-only, Q-32 domain widening) and `1dfa4a2` (Interpolate for Color, plus the `## D3a progress` notes). Tree judged: `1dfa4a2` (branch HEAD), clean export. Out of scope: D3b (`Transform2D`), D3c (ARQ-12 cross-platform golden and the 4·ε bound), the ARQ-12 benchmark (ARQ-40).

## Round 1 (commit `1dfa4a2`)

### Verdict: PASS (delivery D3a; D3b and D3c remain)

Counts: 0 CRITICAL, 1 WARNING (implementation), 3 SUGGESTION.

### Method

- Disk free before building: 68 GB. Three `git archive` exports of `1dfa4a2` in the scratchpad (gates, probes, hand mutants), files touched after export, one `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`; build folders freed with `cargo clean --target-dir` at the end.
- V2 probes (8 checks, written from §3.4 and ARQ-19 before reading `tests/color.rs`, run as an extra test file on the probe export): one property with 4096 cases over random spaces, channels in [-0.5, 1.5], alphas in [0, 1] and `t` in [-0.5, 1.5] (exact ends, result space, alpha bit-equal to the f32 formula, channels within 1e-5 of an f64 premultiplied/straight reference in linear sRGB); exact ends for all 9 space pairs including `t = -0.0`; opaque red to transparent black in each shared space for 999 values of `t`, asserting the result space and `(1, 0, 0)` in that space's own channels; Q-31 transparent-to-transparent in every space; `a = 0.5 -> 0, t = 1.5` in every space (a = -0.25, straight reference); no clamping and `t` direction (0.25, 1.5, -0.5, alpha 1.5); `is_finite` per slot, value and space; a non-finite Color key rejected by `KeyTrack::new` and a Color track sampled through `Animatable`. All 8 pass. Display-P3 red to transparent black: worst deviation from `(1, 0, 0)` in P3 channels is 5.96e-8.
- Hand mutants: 5 on a third export, against the implementer's tests (cap of 5 per delivery; see below for the sixth one asked for).
- Per-commit gates were not run (V3 only); `e21e561` is test-only on an unchanged source.

### Gates (export of `1dfa4a2`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (`tests/color.rs` 11 passed; no failures in the workspace) |
| `cargo deny check bans licenses sources` | 0 |
| CI: Domain crates forbid unsafe code | 0 |
| CI: Domain crates respect the layer table (V5) | 0 (gafira-anim now depends on gafira-color, geom and time, all in its row) |
| CI: Domain clippy.toml copies are in sync (V12) | 0 |

### Slice card and commit checks (fast lane, 3c)

- `e0e2b49 docs(anim-core): slice card amend` touches only `openspec/changes/anim-core/apply-progress.md` and states why (size forecast, D2b W1 golden). Allowed.
- `cf62a10 docs(specs): settle animation open question Q-32` touches only `docs/specs/02-arquitectura-software.md` (specifier, upstream).
- `e21e561` touches only `crates/gafira-anim/tests/interpolate.rs`. `1dfa4a2` touches `Cargo.lock`, `crates/gafira-anim/{Cargo.toml, src/interpolate.rs, tests/color.rs, tests/color.proptest-regressions}` and appends one hunk (`## D3a progress`) at the end of `apply-progress.md`; the Slice card block is unchanged. No implementer commit changed the card.
- No AI attribution or Co-Authored-By lines in `c048b97..1dfa4a2`. Conventional commit subjects.
- Size: `git diff --shortstat e0e2b49..1dfa4a2 -- . ':!docs'`: 292 insertions (src 45, tests 223, regressions 7, manifests 2, notes 15); forecast ~350. Below ~700.
- No `#[allow]`, `unsafe`, `mul_add`, `HashMap`, `unwrap` or `expect` in `gafira-anim/src/interpolate.rs`.
- `git grep ARQ-19 -- crates/gafira-anim/tests`: `tests/color.rs` 11, `tests/interpolate.rs` 7, `tests/sampling.rs` 1. Q-32 named in `tests/interpolate.rs`.
- The new dependency `gafira-color` is internal and already in the V5 row and 02 §2.2; no new third-party crate.

### Criteria map

| # | Criterion | Proof | Status |
|---|---|---|---|
| 1 | ARQ-19 Color: `t = 0` returns `a`, `t = 1` returns `b` (PartialEq, no conversion) | property `arq_19_color_endpoints_are_exact` (bits, any pair of spaces); probes | covered |
| 2 | ARQ-19 Color: r, g, b after conversion to linear sRGB within 1e-5 of the f64 premultiplied reference | property `arq_19_color_matches_premultiplied_reference` (t in [-0.5, 1.5]); M1, M3, M4 killed | covered |
| 3 | ARQ-19 Color: `a` is the linear interpolation of the two alphas | property `arq_19_color_alpha_is_linear_and_bit_exact` | covered |
| 4 | §3.4 result space: shared space kept, else linear sRGB | `arq_19_color_result_space_rule` (3 shared, 6 mixed); M2 killed | covered |
| 5 | §3.4 linear light (not encoded space) | `arq_19_color_blends_in_linear_light` | covered |
| 6 | Q-27 premultiplied alpha differs from straight | `arq_19_color_premultiplies_alpha` (0.8/0.2, alpha 0.625); M1 killed | covered |
| 7 | ARQ-19 opaque red to transparent black keeps the red hue for every `a > 0` (`g = b = 0`, `r` of the red) | `arq_19_opaque_red_to_transparent_black_keeps_hue`; literal `g = b = 0` only for sRGB and linear sRGB (W1); probe confirms it in P3 too | covered (weak for P3, W1) |
| 8 | Q-31 both alphas 0, same space, `t = 0.5`: `a = 0`, straight reference, finite | `arq_19_color_transparent_to_transparent_blends_straight` (sRGB); M5 killed | covered |
| 9 | Q-31 `a = 0.5 -> 0`, `t = 1.5`: `a = -0.25` unclamped, straight extrapolation, no sign flip from the division | `arq_19_color_negative_alpha_extrapolates_straight` (checks `r < 0`, `b > 1`, which the premultiplied branch would not give) | covered |
| 10 | §3.4 no clamping, extrapolation | `arq_19_color_does_not_clamp_channels` (t = 2); M3 killed | covered |
| 11 | ARQ-19 / ARQ-18 `is_finite`: false with NaN, +inf, -inf in any channel; true when all finite | `arq_19_color_is_finite_checks_every_channel` (4 slots x 3 values x 3 spaces) | covered |
| 12 | Q-32: f32 and Vec2 between-ends generators produce components in [-MAX/2, MAX/2] | `f32_any`/`f64_any` add `-HALF..=HALF` and both extremes; every other arm lies inside the range; `arq_19_{f32,vec2}_stays_between_ends` and endpoint properties pass | covered |
| 13 | §3.4 determinism: in-platform bit-for-bit; cross-platform 4·ε per channel | in-platform: pure function, alpha bit-exact test; cross-platform bound: D3c golden per the Slice card | covered (cross-platform part deferred to D3c by the card) |

13 of 13 covered.

### Implementer deviations

1. Red to transparent black in Display-P3. Both ends are Display-P3, so by the result-space rule the result is Display-P3 and the "starting red" is P3 `(1, 0, 0)`. The criterion `g = b = 0` therefore applies in P3 channels and holds: the probe measures a worst deviation of 5.96e-8 over 999 values of `t`. The progress note ("g = b = 0 holds in the sRGB primaries") misreads the rule. The test's P3 branch checks that the linear-sRGB channels equal the converted red within 1e-5 absolute, which proves the hue is kept, but it does not assert the literal criterion in the result space, and a 1e-5 linear-sRGB error allows about 1.7e-4 in encoded P3 g, b. Classified W1, implementation (test gap; the spec is clear).
2. Alpha uses the f32 formula `a + (b - a) * t` bit for bit (as `f32::interpolate`), with r, g, b in f64 narrowed once. Consistent with "a is the linear interpolation of the two alphas", with `f32::interpolate` and with the bit-for-bit expectation for IEEE-only arithmetic. The branch test (`<= 0`) and the divisor use the result's f32 alpha, as §3.4 says ("divided by its a"). Accepted, no finding (see S1 for the test reference).
3. The 4·ε cross-platform tolerance is left to D3c. The amended Slice card assigns the golden and its §3.4 tolerances to D3c; D3a tests the in-platform 1e-5 bound against an f64 reference. Accepted, no finding.

### Hand mutants (5 run, 5 killed)

| # | Mutant (`src/interpolate.rs`, Color impl) | Result |
|---|---|---|
| M1 | Straight instead of premultiplied: `(pa + (pb - pa) * t) / wr` -> `ca + (cb - ca) * t` | killed (`premultiplies_alpha`, `matches_premultiplied_reference`, `red_to_transparent_black`) |
| M2 | Wrong result space: always `linear.convert(self.space())` (mixed pairs take the first end's space) | killed (`result_space_rule`) |
| M3 | Clamping: `value.clamp(0.0, 1.0) as f32` | killed (`does_not_clamp_channels`, `negative_alpha_extrapolates_straight`, `matches_premultiplied_reference`, `red_to_transparent_black`) |
| M4 | Wrong `t` direction for channels: `let t = 1.0 - f64::from(t)` | killed (5 tests) |
| M5 | Alpha <= 0 branch: `wr <= 0.0` -> `wr < 0.0` (alpha exactly 0 divides 0/0) | killed (`transparent_to_transparent_blends_straight`) |

Not run (cap of 5 per delivery): "`is_finite` skipping a channel". The impl delegates to `gafira_color::Color::is_finite`, which is outside this delivery, and `arq_19_color_is_finite_checks_every_channel` sets each of the 4 slots in each of the 3 spaces to NaN, +inf and -inf, so any skipped channel fails it; the T4 run at change close covers the delegate body.

### Findings

#### CRITICAL

None.

#### WARNING

- W1 (class: implementation) `crates/gafira-anim/tests/color.rs` `arq_19_opaque_red_to_transparent_black_keeps_hue`: for Display-P3 ends the test does not assert the ARQ-19 criterion `g = b = 0`, `r` of the red in the result space (P3), only linear-sRGB equality to the converted red within 1e-5 (about 1.7e-4 slack in encoded P3 g, b). The behavior is correct (probe: 5.96e-8). The progress note's rationale ("holds in the sRGB primaries") misreads the result-space rule. Fix: for each space, assert `r.space() == space` and that the result's own channels are `(1, 0, 0)` within 1e-5, and correct the note. Non-blocking.

#### SUGGESTION

- S1 `tests/color.rs` `reference`: the reference picks its branch and divisor from an f64 alpha, while the code (and §3.4) use the result's f32 alpha. When the exact alpha is about 0 the two can fall on different branches (premultiplied over a tiny f64 alpha against straight), so the property could flake on a rare draw. Use `aa + (ab - aa) * t` in f32, widened, as the reference alpha.
- S2 `arq_19_color_transparent_to_transparent_blends_straight` covers sRGB only; looping over the three spaces (as the probe does) costs two lines.
- S3 The between-ends `-HALF..=HALF` arms draw almost only huge magnitudes; the small-value arms already cover the rest, so this is fine, but a log-uniform arm would also reach the mid range (1e3..1e30) more often.

next: tdd-apply (D3b; W1 can ride along as a test-only fix)
