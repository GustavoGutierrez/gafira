## Slice card
Flow: standard | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator | reason: new crate gafira-anim already described in 02 (layer 1, deps time/geom/color, so the V5 row follows 02); spec gaps fixed by specifier first (724bdcb)
Product IDs: ARQ-12 (partial: structural proof; the 1 µs benchmark stays with ARQ-40), ARQ-16, ARQ-17, ARQ-18, ARQ-19 (source: docs/specs/02-arquitectura-software.md §3.4, §2.1, §2.2)
Scope: new crate `crates/gafira-anim`: `Interpolate`, `Easing` with its curves, `Keyframe`, `KeyTrack` with its invariant, `Animatable<T>` with serde and `sample`, and `Interpolate` for `f32`, `Vec2`, `Color` and `Transform2D`.
Non-goals: `GradientStops` (arrives with the gradient type); the ARQ-12 benchmark and `perf-budgets.toml` (ARQ-40); `ToMessage` for anim errors (ARQ-102, belongs above gafira-doc); wiring `Animatable` into gafira-doc nodes; animation presets; `SetKeyframe`.
Deliveries (serial):
  D1 crate scaffold (workspace entry, V5 row `gafira-anim gafira-time gafira-geom gafira-color`, clippy.toml copy, forbid(unsafe_code)), `Easing` and its curves (ARQ-17), ~300 lines
  D2a `Interpolate` with `f32` and `Vec2` (ARQ-19 part), `Keyframe`, `KeyTrack` and its invariant (ARQ-18 construction), commits c2b26f9..d9d2e50, 454 lines measured
  D2b `Animatable` with `sample` (ARQ-16, ARQ-12 structural) and serde through an unvalidated wire shape (ARQ-18 deserialization), commits 71681b3..d8ab4cf, 640 lines measured
  D3a `Interpolate` for `Color` (ARQ-19: premultiplied alpha Q-27, alpha <= 0 rule Q-31, tolerance per §3.4, `is_finite`), plus widening the `f32`/`Vec2` property domains to [-MAX/2, MAX/2] (Q-32 A, test-only), ~350 lines
  D3b `Interpolate` for `Transform2D` (ARQ-19: modified decomposition, shortest-path rotation, degenerate endpoints Q-28 B and Q-30 A, exact endpoints, `is_finite`), ~450 lines
  D3c ARQ-12 cross-platform golden: a file of tracks of the four types with sampled `t` and stored expected results, checked bit for bit or within the §3.4 tolerances; locally only Linux runs it while CI is disabled (README follow-up 2), ~200 lines
Plan: D1 -> verify -> merge to develop; D2a -> verify -> merge; D2b -> verify -> merge; D3a, D3b, D3c each -> verify -> merge; T4 mutants on gafira-anim; archive (also fixes the README "Detalle" entry that lists ARQ-12 under domain-doc).
Owner decisions touched: none (checked D-03: the 1 µs figure is unchanged); Q-27..Q-31 answered by the owner and recorded in 02 §12.2 (9907bf9, c10e38b).
Flow-change: D3 split into D3a, D3b, D3c and the ARQ-12 golden assigned to D3c (trigger: size forecast x1.4 above ~700, and verify D2b W1: the golden had no delivery)
Flow-change: D2 split into D2a and D2b at commit boundaries (trigger: size, D2 measured 1107 lines; owner OK to split)

## D1 progress (tdd-apply)
Done: crate scaffold (workspace entry, V5 row, clippy.toml copy, forbid(unsafe_code)); `Easing` with `is_valid` and `apply` (ARQ-17). Only dependency: serde (+ proptest, serde_json dev); gafira-time/geom/color are in the V5 row but not yet used by code.
RED evidence (stub `apply` = identity, `is_valid` = true):
- ARQ-17 validity: `arq_17_cubic_bezier_validity_rule` failed at `(-0.01, 0.0, 0.5, 1.0)`.
- ARQ-17 precision/CSS curves: `arq_17_bezier_matches_f64_reference_within_tolerance` and `arq_17_named_curves_match_css_reference_values` failed on the tolerance assertion.
- ARQ-17 Linear, bit-for-bit named==bezier, exact ends, bounds, serde names: pass against the stub by construction (identity is Linear; ends and bounds hold for identity), so no RED for them; the named==bezier test is guaranteed by delegation and Hold step test was added after implementation.
Deviations: the spec does not define `Easing::apply` for `Hold`; chosen as a step (0 for u<1, 1 at u=1) to meet e(0)=0, e(1)=1. `u` outside [0,1] or NaN clamps to the nearest end (NaN -> 0). Added `Debug`/`PartialEq` derives beyond the spec snippet.

## D1 round 2
Fixes for verify round 1 (`openspec/changes/anim-core/verify-report.md`).
- C1: solver stop is now relative, `|x(s) - u| <= 1e-12 * u`, instead of an absolute `< 1e-12`, so tiny `u` no longer returns the first guess `s = u`. RED: `arq_17_bezier_is_precise_for_tiny_u_with_flat_start` failed with `u = 1e-12: got 2.9999999880125916e-12, want 2.9997000060049960e-4`; also `arq_17_bezier_precision_grid_of_extreme_control_points` failed at `(0,-2,0,-2) u = 1e-12`.
- C3: precision property now draws `u` log-biased near 0 and 1 and control-point x values at/near 0 and 1 (4096 cases), plus the fixed grid test. RED: `arq_17_bezier_matches_f64_reference_within_tolerance` failed (got 1.908e-4, want 1.689e-4, tol 1e-5) before the solver fix.
- C2: validity test now has x1 = 1.0, x2 = 1.0, x1 = 0.0 as valid and the adjacent floats outside each end (`-MIN_POSITIVE`, next float above 1.0) as invalid. No RED against the code (the code was already right; the gap was the test). Hand mutant `(0.0..1.0)` on x1 is now killed.
- W1: `arq_17_every_curve_clamps_its_input_and_maps_nan_to_zero` covers u < 0, -0.0, -inf, NaN -> 0 and u > 1, +inf -> 1 for every variant. No RED (behavior already implemented, matches the updated spec: Hold is a step, output not clamped). Hand mutant "NaN not handled" is now killed.
- M2 (epsilon 1e-12 -> 1e-6) is now killed by both the grid and the tiny-u tests.
- W3, surviving mutants kept as equivalent:
  - Dropping the final `y.clamp(min, max)`: `y` is a convex combination of values that are exact f32 hull bounds, so a rounding overshoot in f64 is about 1e-16 and the cast to f32 rounds it back to the bound. Unobservable on the f32 output, so equivalent.
  - Dropping the Newton bracket guard (`next > lo && next < hi`): `x` is monotone and the bracket is updated every iteration with the bisection fallback, so convergence within 64 iterations holds on all tested inputs (grid plus 4096 random curves); no input found where the result differs beyond tolerance. Treated as equivalent for the output contract.
- Test note: the grid test skips `u >= 1.0` because e(1) = 1 is pinned by the exact-ends tests and the f64 reference is ill-conditioned at u = 1 with x2 = 1.

## D2 progress (tdd-apply)
Done: `Interpolate` (+ `f32`, `Vec2`), `Keyframe`, `KeyTrack` with `KeyTrackError`/`KeyTrackRule`, `Animatable<T>` with `sample` and serde (untagged write; read through the private `RawAnimatable` wire shape, then validate). Deps added: gafira-time, gafira-geom (both already in the V5 row).
RED evidence (stubs first, then implementation):
- ARQ-19 f32/Vec2: `arq_19_f32_endpoints_are_exact` (minimal input a = 0.0, b = -0.0), `arq_19_vec2_endpoints_are_exact`, `arq_19_f32_follows_the_formula_and_extrapolates`, `arq_19_vec2_follows_the_formula_per_component` failed against an identity stub. The between-ends properties pass against the stub (identity lies between the ends), so they have no RED; the formula tests kill that gap.
- ARQ-18 construction: `arq_18_construction_names_the_rule_and_index`, `arq_18_non_finite_vec2_component_is_rejected`, `arq_18_error_message_names_rule_and_index` failed against a `new` that accepts everything.
- ARQ-16 / ARQ-12: against a `sample` that returns the first key: 12 of 17 tests failed (table rows, `arq_12_sample_calls_interpolate_at_most_once`, the sampling properties).
- ARQ-18 serde: `arq_18_invalid_tracks_fail_naming_rule_and_index` failed twice: with a plain derive the invalid track was accepted ("empty: Keyed(KeyTrack { keys: [] })"); with validation inside `KeyTrack` but a plain untagged enum the error was "data did not match any variant of untagged enum Animatable" (spike claim 3). Fixed by the `RawAnimatable` + `try_from` pattern.
- Round-trip, golden and residual-ambiguity serde tests pass against the naive derive by construction (no RED).
Deviations:
- `Interpolate` has a second required method `is_finite(&self) -> bool`, which the spec snippet does not list; ARQ-18 needs a finiteness check over generic `T` and there is no other way to get it.
- `KeyTrack.keys` is private with `keys()` and `KeyTrack::new`, so no public operation can leave an invalid track (spec: "Ninguna operación pública deja una pista inválida"); the snippet shows it `pub`.
- Error type: `KeyTrackError { rule: KeyTrackRule, index: Option<usize> }`; `index` is `None` only for the empty-track rule. Added `Debug`/`PartialEq` derives.
- Non-finite values through JSON are covered only by construction tests (JSON cannot carry them).
- `f32::interpolate` uses the spec formula `a + (b - a) * t`; with finite ends whose difference overflows f32 (|a|, |b| near 3e38 with opposite signs) the result is infinite. The property tests stay within 1e30. Not specified; flag to the specifier if it matters.
- Test sizes: source ~256 lines, tests ~815, so the delivery is past the ~700 line mark in total (git diff --shortstat 261dd77..HEAD: see the coordinator result).

## D3a progress (tdd-apply)
Done: `Interpolate for Color` (linear-light blend, premultiplied alpha Q-27, straight blend when the resulting alpha <= 0 Q-31, shared space kept else linear sRGB, no clamping, exact endpoints, `is_finite`); `gafira-color` added to `gafira-anim` dependencies (the V5 row and 02 §2.2 already listed it, so no doc change); Q-32 test-only widening of the `f32`/`Vec2` between-ends domains to [-MAX/2, MAX/2] (`HALF32`/`HALF64` plus the extremes as fixed cases).
RED evidence (stub `interpolate` = `*self`, `is_finite` = true; all 11 tests in `tests/color.rs` failed on assertions):
- Endpoints: `arq_19_color_endpoints_are_exact` (t = 1 returned `a`).
- Reference match / premultiplied: `arq_19_color_matches_premultiplied_reference`, `arq_19_color_premultiplies_alpha` (r 0.8 expected, got the start color).
- Result space: `arq_19_color_result_space_rule`; linear light: `arq_19_color_blends_in_linear_light`; no clamp: `arq_19_color_does_not_clamp_channels`.
- Alpha bit-exact: `arq_19_color_alpha_is_linear_and_bit_exact`.
- Red to transparent black: `arq_19_opaque_red_to_transparent_black_keeps_hue`; Q-31: `arq_19_color_transparent_to_transparent_blends_straight`, `arq_19_color_negative_alpha_extrapolates_straight` (a = -0.25).
- ARQ-18/19 finiteness: `arq_19_color_is_finite_checks_every_channel`.
- Q-32 widening: no RED (test-only, the code already satisfies the wider domain; passes first run).
Deviations:
- Red-to-transparent-black test: for Display-P3 the expected hue is the converted red in linear sRGB (g, b nonzero there); g = b = 0 is asserted only for sRGB and linear sRGB. Spec wording ("g = b = 0") holds in the sRGB primaries.
- Alpha uses the `f32` formula (`a + (b - a) * t` in f32, same as `f32::interpolate`); channels are computed in f64 and narrowed once.
- Tolerance 1e-5 (ARQ-19) is tested against an f64 reference; the cross-platform 4*eps bound of §3.4 belongs to the D3c golden.
