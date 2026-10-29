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

## D3a fix round (tdd-apply)
Trigger: `develop` RED on `arq_19_color_matches_premultiplied_reference` (proptest seed saved in `crates/gafira-anim/tests/color.proptest-regressions`) and D3a verify W1/S1/S2.
Diagnosis: test bug, code matches §3.4. Input a = Srgb alpha 0.528554, b = Srgb alpha 0.0253676, t = 1.0502127 gives a result alpha of about 1.0e-4. The code divides by its own f32 alpha (§3.4: "se divide por su a"), while the test reference divided by an f64 alpha. The f32 alpha carries about 6e-8 absolute cancellation error, so about 6e-4 relative at that magnitude, and un-premultiplying amplified it to 0.076 on a value of 199.36 (got 199.28341674804688, reference 199.3598177686992). The reference now takes its alpha (divisor and branch test) from the f32 formula `a + (b - a) * t`, widened, the same alpha the bit-exact alpha test pins (S1). No spec ambiguity: the 1e-5 tolerance is against a reference that applies the spec's divisor.
Fixes (test-only, `tests/color.rs`): reference alpha as above (S1); `arq_19_opaque_red_to_transparent_black_keeps_hue` asserts, for each space, the result space and that the result's own channels are (1, 0, 0) within 1e-5 (W1); the Q-31 transparent-to-transparent test loops over the three spaces (S2).
Correction to the D3a note above: the deviation "g = b = 0 is asserted only for sRGB and linear sRGB; spec holds in the sRGB primaries" was wrong. Both ends share the space, so the result is in that space (Display-P3 included) and the criterion applies to its own channels.
RED: the saved regression case fails `arq_19_color_matches_premultiplied_reference` before the fix (`ch 2: [0.0, 0.0, 199.28341674804688, 0.00010126829147338867] vs [0.0, 0.0, 199.3598177686992]`).

## D3b progress (tdd-apply)
Done: `Interpolate for Transform2D` in `crates/gafira-anim/src/interpolate.rs` (decomposition into translation, `sx > 0`, `sy` signed by the determinant, shear `k`, rotation by the shortest path; all lerped, recomposed to `[sx c, sx s, sy(k c - s), sy(k s + c)]`; Q-28 B degenerate end borrows rotation and shear of the other end with per-axis scale as the diagonal in that frame (Q-30 A); both degenerate -> 6-coefficient lerp; exact endpoints at t = 0 and 1; extrapolation); `is_finite` over the 6 coefficients. Tests in `crates/gafira-anim/tests/transform.rs`. No change to gafira-geom (`coeffs()` and `From<[f64; 6]>` already exist).
RED evidence (stub `interpolate` = `*self`, `is_finite` = true): 15 of 17 tests failed on assertions: `arq_19_transform_rotation_takes_the_short_way`, `..._rotation_extrapolates_without_wrapping`, `..._two_translations_...`, `..._two_scales_...`, `..._flip_is_carried_by_the_y_scale`, `..._shear_from_identity_is_linear_and_continuous`, `..._scale_zero_to_b_...`, `..._scale_x_zero_to_identity_...`, `..._rank_one_endpoint_jumps_as_documented`, `..._both_degenerate_...`, `..._endpoints_are_exact_for_fixed_cases` and the properties (`endpoints_are_exact`, `rotations_follow_the_short_arc`, `degenerate_pairs_are_the_linear_blend`); `arq_18_transform_is_finite_checks_every_coefficient` (NaN accepted). Pass against the stub by construction (no RED): the det-sign property (identity keeps the sign) and equal-endpoints round trip; the real checks for those are the decomposition tests and the mutants below.
Hand mutants killed: longest-path rotation (3 tests), no shear (4), no t=0/1 special case (4), degenerate frame swapped (2). The sign-on-`sx` mutant was wrongly called equivalent here; it differs between ends of opposite determinant sign (round 2, C1).
Multi-seed: 40 distinct `PROPTEST_RNG_SEED` values (7919 * 1..40), 17/17 each, no flake. The det-sign property avoids global rejects by swapping the rows of `b` when its determinant sign differs.
Deviations: a non-finite coefficient on either end (out of contract, ARQ-18 rejects it in tracks) yields the 6-coefficient lerp, never a panic. Det is tested with `== 0.0` exactly (superseded by Q-33 in round 2); a finite end whose parts overflow or divide to non-finite is treated as degenerate. Mixed-sign determinants are specified by §3.4 ("un volteo es siempre `sy < 0`", each component lerped): `sx` stays positive and `sy` crosses 0 as a lerp (corrected in round 2).

## D3b round 2 (tdd-apply)
Findings fixed (verify round 1 FAIL): C1, Q-33 B (W1), S1, S2, S3. Tests in `crates/gafira-anim/tests/transform.rs`, code in `crates/gafira-anim/src/interpolate.rs` (`Parts::decompose`).
- Q-33: an end with finite coefficients is degenerate if `det` or `|c1|·|c2|` is not finite, or `|det| <= 1e-12·|c1|·|c2|` (f64, `c1 = (a, b)`, `c2 = (c, d)`). Replaces `det == 0.0`. Numbers checked against §3.4 / ARQ-19 text: `[0.1, 0.3, 0.7, 2.1]` -> `[0.55, 0, 0, 1.55]`, `[1e200, 1e200, 1e200, 2e200]` -> `[5e199, 0, 0, 1e200]`, `scale(1e-9)` not degenerate -> `scale(0.5 + 5e-10)`; zero translation, towards identity, t = 0.5, 1e-9 tolerance; plus the exact-zero `[1.1, 2.2, 3.3, 6.6]` -> `[1.05, 0, 0, 3.8]`.
- RED (assertion failures before the code change): `arq_19_q33_rank_one_end_with_rounding_determinant_is_degenerate` (huge shear coefficients instead of `[0.55, 0, 0, 1.55]`), `arq_19_q33_finite_coefficients_with_non_finite_determinant_are_degenerate` (non-finite result), `arq_19_q33_threshold_is_relative_to_the_column_lengths` (sine 1e-13 end was decomposed). `arq_19_q33_tiny_uniform_scale_is_not_degenerate` passed before (guards against an absolute threshold).
- C1 (test-only, passes on the code): `..._flip_from_identity_collapses_the_y_axis_at_the_middle` (identity to `scale(1,-1)`, t = 0.5 -> `[1,0,0,0]`, both directions, plus t = 0.25 and 1.5), `..._horizontal_flip_from_identity_goes_through_a_half_turn_frame`, `..._rotated_flip_keeps_the_rotation_and_collapses_the_y_axis`, and property `arq_19_transform_opposite_sign_pairs_match_the_decomposition_reference` (2048 cases, independent reference decomposition, ties at half a turn skipped; tolerance 1e-9·max(1,|w|)·1e3 because near-singular ends give large shears). RED evidence is the mutant: sign moved onto `sx` (`sx = hypot.copysign(det)`, `sy` positive) fails 4 tests: the flip-from-identity test (`[0,0,1,0]`-style quarter-turn path), the horizontal flip, the rotated flip and the property; the clean code passes all.
- S1: `arq_19_transform_non_finite_endpoint_never_panics` (NaN, +-inf in each coefficient, either end or both, several t).
- S3: `arq_19_transform_degenerate_end_uses_the_diagonal_in_a_rotated_sheared_frame` (`scaleY(0)` with translation vs a B with theta 0.7, k 0.8, scales 2 and 3; expected computed independently as the diagonal of `U^-1 R^T L`; asserted different from the plain 6-coefficient blend; mirrored direction too). Mutants killed: borrowing sx replaced by the column norm (this test and the rank-one test), borrowing disabled (5 tests).
- Multi-seed: 60 distinct `PROPTEST_RNG_SEED` values (`15485863*i + 101`, i = 1..60), `PROPTEST_CASES=4096`, 28/28 tests each, 0 failures; no `transform.proptest-regressions` file created. A first run failed on every seed from "too many global rejects" in the new property (angle wrap applied after the assume); fixed in the test before commit.
- Deviation: Q-33 test and fix are in one commit (single file, same behavior unit).
