# Verify report: anim-core D1

Verified commit: `f7e64ba` (delivery `9219560..f7e64ba`: `3f13d9a` scaffold, `f7e64ba` Easing), on a clean `git archive` export outside the repo.
Flow: standard | mutation: archive | tier: T4 | verify: V2. No cargo-mutants report for this delivery (expected with `mutation: archive`; not a finding).
Acceptance source: Slice card (`apply-progress.md`), `docs/specs/02-arquitectura-software.md` §2.1, §2.2, §3.4, ARQ-17.
Note: at verify time the working tree had an uncommitted change to `docs/specs/02-arquitectura-software.md`, made by another agent. It is not part of the verified commit and was not judged.

## Verdict: FAIL

Counts: 3 CRITICAL, 3 WARNING, 3 SUGGESTION. Per class: implementation 3 CRITICAL + 2 WARNING; ambiguity 1 WARNING.

## Gates (HEAD export, toolchain 1.93.1)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (gafira-anim: 10 tests pass) |
| CI "Domain crates forbid unsafe code" (replayed) | 0 |
| CI "Domain crates respect the layer table (V5)" (replayed) | 0 |
| CI "Domain clippy.toml copies are in sync (V12)" (replayed) | 0 |
| `cargo deny --locked check` | 0 |

## Slice card checks (fast-lane rule 3c, applied to the standard-mode card)

- `9219560 docs(anim-core): slice card` touches only `openspec/changes/anim-core/apply-progress.md`. OK.
- `3f13d9a` does not touch `apply-progress.md`. `f7e64ba` only appends the `## D1 progress (tdd-apply)` section. The Slice card block is unchanged. OK.
- Product ID in D1 scope: ARQ-17. All tests are named `arq_17_*` (`git grep ARQ-17`/`arq_17` in `crates/gafira-anim/tests/easing.rs`). ARQ-12, ARQ-16, ARQ-18 and ARQ-19 are deliberately left to D2/D3 by the card.

## Criteria map

| # | Criterion | Proof | Status |
|---|---|---|---|
| 1 | Workspace entry for `gafira-anim` (§2.1, layer 1) | `Cargo.toml` (`crates/*` member + workspace dep); builds | covered |
| 2 | V5 row `gafira-anim gafira-time gafira-geom gafira-color` (§2.2) | `ci.yml` row; V5 step replayed, exit 0 | covered |
| 3 | `clippy.toml` copy | V12 step replayed, exit 0 (byte-identical to the canonical copy) | covered |
| 4 | `#![forbid(unsafe_code)]` | `src/lib.rs:6`; forbid step replayed, exit 0 | covered |
| 5 | ARQ-17 `Linear` is the identity | `arq_17_linear_is_the_identity` | covered |
| 6 | ARQ-17 named curves == their `CubicBezier`, bit for bit, 1 001 `u` | `arq_17_named_curves_equal_their_cubic_bezier_bit_for_bit`; probe agrees | covered |
| 7 | ARQ-17 `e(0)=0`, `e(1)=1` exact | `arq_17_bezier_ends_are_exact`, `arq_17_every_curve_is_exact_at_the_ends`; probe agrees | covered |
| 8 | ARQ-17 precision `<= 1e-5 * max(1,|y1|,|y2|)` against an f64 bisection reference | `arq_17_bezier_matches_f64_reference_within_tolerance` passes, but the probe finds violations (C1); the test is weak (C3) | failing |
| 9 | ARQ-17 bounds `[min(0,y1,y2), max(1,y1,y2)]` | `arq_17_bezier_stays_within_bounds`; probe (200 000 cases) agrees | covered |
| 10 | Validity rule for `CubicBezier` (x1, x2 in [0,1], all four finite; §3.4/ARQ-18 rule, needed by D2) | `arq_17_cubic_bezier_validity_rule` — weak at the upper bound of x1 (C2) | covered (weak) |
| - | serde names of `Easing` (§3.4 format) | `arq_17_serde_names_follow_the_spec_enum`; probe agrees | covered (extra) |

Total 10, covered 9 (one of them weak), failing 1.

## Independent probes (V2, written before reading the implementer's tests)

The probes ran on the scratch export: validity bounds (including x1 = 1, -1e-7, 1.0000001, NaN, ±inf), exact ends for every variant, named == bezier on 1 001 points, CSS values (`ease-in(0.5) ~ 0.3153`, `ease-out(0.5) ~ 0.6847`, `ease-in-out(0.5) = 0.5`), serde names, 200 000 random curves checked for precision and bounds, a deterministic grid (x1, x2 in {0, 1e-6, 0.1, 0.42, 0.58, 0.9, 1}, y1, y2 in {-2, -1, 0, 0.5, 1, 3}, u = 2 001 uniform points plus 10^-k and 1-10^-k for k = 1..45, plus subnormals), clamping and NaN, and no panic on invalid curves.
Results: all pass except the precision grid. That grid has 286 violations (C1).

## Hand mutants (5, scratch export, `src/easing.rs`)

| ID | Mutation | Implementer tests | Probes | Reading |
|---|---|---|---|---|
| M1 | validity: `(0.0..=1.0).contains(&x1)` -> `(0.0..1.0)` (rejects x1 = 1) | survived | killed | real gap (C2) |
| M2 | solver: `X_EPSILON` 1e-12 -> 1e-6 | survived | killed (200 000 random cases) | weak precision test (C3) |
| M3 | bounds: drop the final `y.clamp(min, max)` | survived | survived | likely equivalent (W3) |
| M4 | solver: drop the bisection bracket guard on the Newton step | survived | survived | likely equivalent on tested inputs (W3) |
| M5 | NaN `u` returns 1.0 instead of 0.0 | survived | killed | deviation (2) untested (W1) |

Run 5, killed by the implementer's suite 0.

## CRITICAL

- **C1** class: implementation. ref: `crates/gafira-anim/src/easing.rs` (`X_EPSILON`, `bezier`), ARQ-17 precision. The solver stops when the absolute error `|x(s) - u| < 1e-12`. When `u <= 1e-12` it stops at once on the initial guess `s = u`. When x1 and x2 are near 0, `x(s) ~ s^3` is flat near 0, so the real root `s* = u^(1/3)` is far from that guess. Counterexample: `CubicBezier{x1:0, y1:1, x2:0, y2:1}`, `u = 1e-12` gives `e = 3.0e-12`, while the reference is `3.0e-4`. That is 30 times the 1e-5 tolerance. Violations appear for `u` from about 1e-16 to 1e-12 (ratios 1.4 to 30) and also with `y1=3, y2=-2` and `x2 = 1e-6`. This can happen in real use: a key 30 minutes away and `t` one flick after the key give `u ~ 8e-13`. The named curves are not affected (worst ratio 0.003). Fix: use a relative or root-based stopping criterion (for example stop on the bracket width in `s`, or on `|x(s)-u| <= eps * u` together with a bracket width bound), then add a regression test that names this case.
- **C2** class: implementation. ref: `crates/gafira-anim/tests/easing.rs` `arq_17_cubic_bezier_validity_rule`; validity rule (critical code: validation). No case has `x1 = 1.0` valid, so M1 (half-open range on x1) survives. The out-of-range cases (±0.01) are also coarse. Add the boundary cases `x1 = 1.0` and `x2 = 0.0` as valid, and the next float outside each end as invalid.
- **C3** class: implementation. ref: `arq_17_bezier_matches_f64_reference_within_tolerance`. With uniform `u` and the default 256 cases, it neither finds C1 nor kills M2 (a solver threshold one million times looser). Strengthen it: draw `u` on a log scale near 0 and 1 (for example from `f32` bit patterns), raise the case count or add a fixed grid like the probe grid, and pin the C1 counterexample.

## WARNING

- **W1** class: implementation. ref: `easing.rs` `apply`, deviation (2). The behavior for `u` outside [0,1] and for NaN (`NaN -> 0`, `u<0 -> 0`, `u>1 -> 1`) is documented in the doc comment but no implementer test covers it (M5 survives). Add a test if the specifier keeps this behavior (see W2).
- **W2** class: ambiguity. ref: §3.4 Curvas / ARQ-17. The spec defines `e(u)` only for `u in [0,1]` (the sampling table guarantees that range). `Easing::apply` is public, so what it does outside [0,1] or with NaN, and what it does on an invalid `CubicBezier` (the probe shows no panic), is public behavior the spec does not state. Judgment on deviation (2): a reasonable and total choice, not an implementation defect. The specifier should record it, or state that it is unspecified, before archive.
- **W3** class: implementation. ref: hand mutants M3, M4. Neither the implementer's tests nor the probes kill them. Both are likely equivalent: the hull clamp is defensive, and the bracket guard is not needed on the tested inputs. This is documented, not blocking. If the clamp is meant as a guarantee, a test that produces a rounding overshoot would pin it.

## SUGGESTION

- **S1** ref: deviation (1), `Hold`. The step behavior (0 for u<1, 1 at u=1) follows from §3.4: the `Hold` row ("escalón") together with "all curves satisfy e(0)=0 and e(1)=1". Judgment: consistent with the spec, not a defect. Recording `e(u)=0 for u<1` in the `Hold` row would remove any doubt. D2 must still not call `interpolate` for `Hold` (ARQ-12).
- **S2** ref: deviation (3), extra `Debug`/`PartialEq` derives. Judgment: harmless and useful. `PartialEq` is what the ARQ-18 round-trip property (`deserialize(serialize(a)) == a`) needs in D2. Not a finding.
- **S3** ref: `arq_17_named_curves_match_css_reference_values`. Despite its name, it compares against the same bisection reference, not against published CSS values. Adding two or three literal CSS values (for example `ease-in(0.5) ~ 0.3153`) would make it independent.

## Size

Delivery diff: 371 inserted lines (106 non-test source, 175 test, plus manifests, CI row, clippy.toml, progress). Forecast ~300. Within budget.

next: specifier (W2, ambiguity: record apply() outside [0,1]/NaN/invalid curve), then tdd-apply (implementation: C1, C2, C3, W1, W3)
