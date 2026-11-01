## Slice card
Flow: full | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator | reason: new V5 row and cross-cutting choices (ADRs 0005 to 0010); owner decisions D-14 to D-39 settled
Flow-change: standard -> full (trigger: spike found a new V5 row and the guide-type location conflict, before implementation)
Product IDs: FD-03, FD-04, FD-05, FD-06, FD-07, FD-09, FD-11 first-slice criteria, ARQ-110, ARQ-111, ARQ-114 (source: docs/specs/04-funcionalidad-diseno-imagen-exportacion.md §2 and §3; docs/specs/02-arquitectura-software.md §4.17)
Scope: `gafira-doc` page layout data (bleed, margins, guides, grid), `gafira-edit` properties, and the new crate `gafira-layout`: bounds, rects, guides, grid, snap, align, distribute.
Non-goals: FD-53, FD-54, reflow, `notes`, `duration`, `background`, text and image nodes, UI, PDF export, `rstar`.
Deliveries (serial, design.md delivery table): D1 doc types and wire form ~460; D2 validation and edit properties ~420; D3 crate, V5 row, bounds, rects ~420; D4 guides and grid ~475; D5 snap ~475; D6 align and distribute ~335; D7 benches ~125. Amended 2026-11-01, owner decision: D5, D6 and D7 run as one delivery on `feat/layout-core-d5`, with one verify and no per-delivery size ceiling, to finish the feature in one flow.
Plan: one `tdd-apply` per delivery on `feat/layout-core-<delivery>` cut from `develop`, in the table order; `verify` V2 per delivery; T4 `mutants` on `gafira-doc`, `gafira-edit` and `gafira-layout` at close; `archive`, copying tasks T1 and T2 of design.md to the follow-up list.
Owner decisions touched: none reopened (checked D-14 to D-39; Q-34 does not apply to this change).

## D1: doc types and wire form (`feat/layout-core-d1`)

Status: done. Files: `crates/gafira-doc/src/layout.rs` (new), `page.rs`, `lib.rs`, `json.rs`, `tests/layout.rs` (new), goldens, `crates/gafira-testkit/src/lib.rs`.

Done
- `Margins`, `GuideAxis`, `Guide`, `GuideSet` (transparent newtype), `GridStyle`, `GridSettings` in `gafira-doc`, exported from the crate root; `Page` gains `bleed, margins, guides, grid` after `size`, always written, `#[serde(default)]` on read. Schema stays 1. Nothing added for `notes`, `duration`, `background`. No validation (D2).
- Updated the 7 tests and 3 goldens named by the spike: `page::tests::page_wire_form_writes_every_field`, `json::tests::{minimal_document_has_exact_bytes, full_golden_equals_the_built_document}`, `golden::{canonical_goldens_round_trip_byte_for_byte, noncanonical_golden_normalizes_to_the_canonical_one}`, `serde::{payload_numbers_and_keys_get_one_canonical_spelling, unknown_fields_are_ignored_and_dropped}`; goldens `minimal.json`, `full.json`, `extension.json`. The `full` builder now sets non-default layout data on page 1, so the golden carries real guides, margins, bleed and grid.
- Old bytes kept as input fixtures in `tests/golden/pre_layout/{minimal,full,extension}.json`; `extension_noncanonical.json` is untouched and now also loads without the fields.
- Testkit generators: `bleed`, `margins`, `guide`, `guide_set`, `grid_settings`; the document generator fills them on every page (values inside the spec ranges, so D2 validation will accept them).

RED evidence (stub: new `Page` fields with `skip_serializing`)
- FD-03(b) `fd_03_every_guide_field_survives_save_and_open`, `fd_03_guide_order_and_duplicates_survive`: assertion `left == right` on the guides read back (default empty set).
- FD-05 `fd_05_documents_without_layout_fields_load_with_defaults`: assertion that the re-saved text contains `"bleed"`.
- FD-05/FD-07 `fd_05_fd_07_layout_fields_survive_save_and_open`, `layout_data_round_trips`: page differs after save and open.
- ADR 0006 `adr_0006_default_valued_layout_fields_are_written`: output lacks the default-valued fields.
- `grid_style_and_guide_axis_use_snake_case_on_the_wire`: output lacks `"style":"dots"`.
- `fd_05_fd_07_defaults_match_the_spec` passed at the stub (defaults come from `Page::new`); it is killed by the mutants of item 4.
- With the stub removed, exactly the 7 predicted existing tests failed (byte comparisons), then were updated.

Hand mutants (each applied, `cargo test -p gafira-doc` failed, reverted, not committed)
1. Old-document load: `#[serde(default)]` removed from `bleed`, `guides`, `grid`, `margins` (one at a time) -> `fd_05_documents_without_layout_fields_load_with_defaults`, `page::tests::page_defaults_required_fields_and_unknown_fields`, `noncanonical_golden_normalizes_to_the_canonical_one`.
2. Round trip: `locked` skipped on write, `position` skipped on write, `locked` / `axis` ignored on read, `position` unit forced to mm on read -> `fd_03_every_guide_field_survives_save_and_open`, `fd_03_guide_order_and_duplicates_survive`, `canonical_goldens_round_trip_byte_for_byte`.
3. Always written: `skip_serializing_if` default on `guides`, `bleed`, `grid`, `margins` -> `adr_0006_default_valued_layout_fields_are_written`, `json::tests::minimal_document_has_exact_bytes`.
4. Defaults: `linked` false, bleed 3 mm, step 20, subdivisions 2, style dots, snap false, visible true, margin unit mm -> `fd_05_fd_07_defaults_match_the_spec`, `adr_0006_default_valued_layout_fields_are_written`, `minimal_document_has_exact_bytes`.
5. Guide order: reversed on write, deduplicated on write, sorted by position on read -> `fd_03_guide_order_and_duplicates_survive` (all three), `layout_data_round_trips`.
All items killed; no survivors.

Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` all pass.

Deviations: none. Note for D2: the generators already stay inside the L1 ranges.

### D1 fix round 1 (verify FAIL: C1, C2, W1, W2)

Correction: the D1 statement "Deviations: none" was wrong. `GridSettings.style` had no read default (ADR 0006 rule 4), and the nested fields below had none either; fixed here.

- C1 / W1: `GridSettings` now has container-level `#[serde(default)]` (every field reads its `Default`: visible false, step 10 px, subdivisions 1, style lines, snap true); `Margins.linked` reads true; `Guide.locked` reads false. Margin sides, guide `axis` and `position` stay required (no stated default).
- C2: test that an unknown `GridStyle` and an unknown `GuideAxis` fail to load.
- W2: `adr_0006_old_bytes_resave_in_the_canonical_form` compares the re-saved old minimal and extension bytes with the current goldens, and old full with the current golden (page 1 layout data aside), and checks the result is stable.
- RED: `fd_07_each_absent_grid_field_reads_its_default` failed with `missing field visible` (unwrap on the load error); `fd_05_absent_linked_and_locked_read_their_defaults` failed with `missing field linked`. The W2 and C2 tests passed on the existing code (they strengthen coverage); C2 is proved by its mutant.
- Hand mutants (reverted, not committed): `#[serde(other)] Unknown` variant on `GridStyle` -> killed by `adr_0006_unknown_enum_values_fail_to_load`; grid container default removed -> `fd_07_each_absent_grid_field_reads_its_default`; `linked` default removed, `default_linked` returning false, `locked` default removed -> `fd_05_absent_linked_and_locked_read_their_defaults`.
- Not in this round: W3 (`budget.rs`) and S1 go to D2.

## D2: validation and edit properties (`feat/layout-core-d2`)

Status: done. Files: `crates/gafira-doc/src/{layout,page,error,validate,lib}.rs`, `crates/gafira-doc/tests/layout_validation.rs` (new), `crates/gafira-edit/src/{set_property,budget}.rs`, `crates/gafira-edit/tests/layout_commands.rs` (new).

Done
- `gafira-doc`: `PageField`, `PageLayoutError { NonFinite, OutOfRange }`, `check_bleed(Length, Dpi)`, `Margins::check`, `GuideSet::check`, `GridSettings::check`, `Page::check_layout(dpi)`; new `ValidationError::PageLayout { page, error }`; P3 runs it after page sizes, over all pages, before the root checks. Order inside: bleed, margins (top, right, bottom, left), guides (in order), grid step, subdivisions. Nothing is clamped.
- Ranges (L1): bleed 0 to 10 mm after conversion at the document DPI; margins finite and >= 0 in their own unit, no upper bound; guide position finite only (may lie off the page); grid step 1 to 500 on the stored number in its own unit; subdivisions 1 to 10. All ends inclusive.
- `gafira-edit`: `Property::{Bleed, Margins, Guides, Grid}` (page scope, `SetProperty::page`), merge kinds `set_property.{bleed,margins,guides,grid}`, same checks as `validate()` before any write, error `CommandError::InvalidDocument(ValidationError::PageLayout)`. Undo writes back the exact value captured by `apply`.
- W3: `budget::page_bytes` adds `guides * size_of::<Guide>()`.
- S1: load tests use out-of-range values (bleed 99 and -1 mm, margin -5, step 0 and 501, subdivisions 0 and 11) and assert the validation error, not a clamp.

RED evidence (stubs: checks returning `Ok(())`; edit variants without read/write)
- FD-05(d) `fd_05_bleed_bounds_are_inclusive_on_both_ends`: `Ok(())` where `OutOfRange { Bleed, 10.01 }` expected; `fd_05_bleed_range_is_in_millimetres_for_any_unit`: 0.4 in accepted.
- FD-05 margins `fd_05_margin_bounds_are_inclusive_on_the_zero_end`, `fd_05_margins_report_the_first_bad_side`: `Ok(())` for -0.01 / -1.
- FD-07(e) `fd_07_grid_step_bounds_are_inclusive_on_both_ends`, `fd_07_subdivision_bounds_are_inclusive_on_both_ends`: `Ok(())` for 0.99, 500.01, 0, 11.
- FD-03(d), ARQ-110 `arq_110_non_finite_values_are_rejected_in_every_new_field`, `fd_03_a_bad_guide_in_any_position_fails`: `Ok(())` for NaN.
- S1 `fd_05_fd_07_out_of_range_values_on_load_are_rejected_not_clamped`: document loaded with bleed 99 mm instead of `LoadError::Invalid`.
- Edit: `fd_05_every_layout_property_is_set_and_restored`, `fd_05_the_same_layout_value_is_unchanged_and_shares_the_page`, `fd_04_c_*`, `arq_110_invalid_layout_values_fail_and_change_nothing`: `Err(InvalidValue("a node property on a page"))` where `Changed`, `Unchanged` or `PageLayout` was expected.
- W3 `fd_04_the_history_estimate_grows_with_the_guide_count`: grown estimate 0, expected 24 for one guide.
- Passed at the stub (strengthen coverage, proved by mutants): `fd_03_guide_positions_are_finite_only`, `fd_05_ten_millimetres_in_inches_is_valid`, `fd_05_fd_07_range_ends_on_load_are_kept_as_written`, `layout_properties_have_distinct_merge_keys`.

Hand mutants (each applied, a test failed, reverted, not committed)
1. Bounds, one end at a time, every field: bleed low and high exclusive -> `fd_05_bleed_bounds_are_inclusive_on_both_ends`; margin low -> `fd_05_margin_bounds_are_inclusive_on_the_zero_end`; step low and high -> `fd_07_grid_step_bounds_are_inclusive_on_both_ends`; subdivisions low and high -> `fd_07_subdivision_bounds_are_inclusive_on_both_ends`. The shared range helper made exclusive on either end also fails `fd_05_fd_07_range_ends_are_accepted` and `layout_properties_obey_the_command_laws` in `gafira-edit`. Margins have no upper bound and guide positions have no bound at all, so those have no end to flip.
2. Non-finite accepted: infinity only (`!is_nan`) -> `arq_110_non_finite_values_are_rejected_in_every_new_field`; `finite()` always `Ok` -> same test, `fd_03_a_bad_guide_in_any_position_fails`, `layout_checks_run_in_a_fixed_order`; bleed conversion failure treated as 0 mm -> `arq_110_non_finite_values_are_rejected_in_every_new_field`.
3. Skipped step: bleed, margins, guides, grid, grid step alone, subdivisions alone, the whole page-layout step in `validate` -> the range, `arq_110_*` and `fd_05_fd_07_out_of_range_values_on_load_are_rejected_not_clamped` tests of `layout_validation.rs`; in `gafira-edit`, skipping all checks or each property's check -> `arq_110_invalid_layout_values_fail_and_change_nothing`, `layout_properties_obey_the_command_laws` (bleed also `fd_05_bleed_is_checked_in_millimetres_at_the_document_dpi`; ignoring the document DPI is killed by the same test).
4. Clamp on load (bleed, step, subdivisions, margins clamped in `DocumentWire` conversion) -> `fd_05_fd_07_out_of_range_values_on_load_are_rejected_not_clamped`.
5. Undo: previous value read as the default for guides, bleed, margins and grid -> `fd_04_c_applying_guides_and_undoing_restores_the_exact_previous_set`, `fd_04_c_revert_gives_back_the_set_the_last_apply_replaced`, `fd_05_every_layout_property_is_set_and_restored`, `layout_properties_obey_the_command_laws`; guide order sorted or reversed -> the same `fd_04_c_*` tests; duplicates removed -> `fd_04_c_any_guide_set_reverts_to_the_previous_one`, `layout_properties_obey_the_command_laws`. The `fd_04_c_applying_*` test runs with an unlimited budget (snapshot path) and budget 0 (compacted entry, so undo calls `revert`).
6. Budget ignores guides, or counts 1 byte per guide -> `fd_04_the_history_estimate_grows_with_the_guide_count`.
All items killed; no survivors.

Gates (before each commit): `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (924 passed, 0 failed).

Deviations: none. Notes: (a) the bleed is compared after conversion to mm with `Length::convert`, so a bleed written in another unit sits on floating-point rounding at exactly 10 mm; 10 mm written as `10/25.4 in` passes (tested), other units were not probed. (b) Out-of-range guide positions do not exist (finite only), so FD-03(d) is covered by the non-finite tests. (c) `gafira-layout` is untouched (D3).

### D2 fix round 1 (verify FAIL: C1, C2, C3, S1)

- C1: `check_pages` (DPI path of `SetProperty`) now checks every page size, then every page's layout data via `Page::check_layout(dpi)`, same order as `validate()`; a refused change returns `InvalidDocument(PageLayout)` and changes nothing. RED: `arq_13_a_dpi_change_that_breaks_a_bleed_is_refused` (300 to 96 dpi with a 100 px bleed on page 2): `unwrap_err` on `Ok(Changed)`. Companion tests `arq_13_a_dpi_change_that_keeps_bleeds_in_range_applies_and_reverts`, `arq_13_a_dpi_change_reports_page_sizes_before_layout`.
- C2: `fd_05_fd_07_a_bad_value_on_a_later_page_is_found` (one bad field per case on page 2), `fd_05_a_later_page_with_a_bad_value_is_rejected_on_load`, `fd_05_a_layout_value_is_checked_on_the_page_it_targets`. They passed on existing code (coverage gap); proved by mutants.
- C3: `design_l1_page_sizes_are_reported_before_layout_data` (same page, and layout error on page 1 with size error on page 2). Passed on existing code; proved by mutant.
- S1: module comment of `layout.rs` corrected.
- New mutants (reverted): validate layout on the first page only -> `fd_05_fd_07_a_bad_value_on_a_later_page_is_found`, `fd_05_a_later_page_with_a_bad_value_is_rejected_on_load`; layout step before page sizes in `validate` -> `design_l1_page_sizes_are_reported_before_layout_data`; DPI path layout on first page only, or no layout loop -> `arq_13_a_dpi_change_that_breaks_a_bleed_is_refused`; DPI path layout before sizes -> `arq_13_a_dpi_change_reports_page_sizes_before_layout` (the first version of this test survived because no bleed broke at the chosen DPI; rewritten so both a size and a bleed break). All killed.
- Not done: W1 (exact 10 mm comparison unchanged), S2 (no trimming).

### D2 fix round 2 (verify round 2, C4)

- C4: commit `c1551a2` deleted 9 tracked regression-seed files, out of scope. Restored byte for byte from `develop` with `git checkout develop -- <paths>`: `gafira-anim/tests/{color,easing,interpolate,sampling}.proptest-regressions`, `gafira-pathops/tests/prop_boolean.proptest-regressions`, `gafira-time/tests/{flicks,ranges,rates,timecode}.proptest-regressions`.
- Check: `git diff --name-status develop HEAD -- '*.proptest-regressions'` is empty after the commit.
- Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` all pass (931 passed, 0 failed). The test run created no new or changed `*.proptest-regressions` files.
- Correction: the previous fix round (round 1) did not declare this deletion in its notes; it was an unintended side effect of the round.
- No behaviour change: no RED step, no hand mutants. W1 and S2 remain follow-ups, untouched.

## D3: crate, V5 row, bounds, rects (`feat/layout-core-d3`)

Status: done. Files: `crates/gafira-layout/{Cargo.toml,clippy.toml,src/{lib,error,bounds,rects}.rs,tests/{bounds,rects}.rs}` (new), `Cargo.toml`, `Cargo.lock`, `.github/workflows/ci.yml` (V5 row `gafira-layout gafira-geom gafira-color gafira-doc`, as ADR 0007; checked with `cargo tree`).

Done
- `clippy.toml` is a byte copy of the canonical `gafira-geom` one (V12). No serde, nothing mutated, `#![forbid(unsafe_code)]`.
- `LayoutError { NonFinite, InvalidBox, OutOfRange { field }, NoArea, UnknownNode, Cycle }`.
- `bleed_rect(page, Length, dpi)`, `safe_rect(page, percent)` (5.0 is 5 %, range `[0, 50)`), `margin_rect(page, &Margins, dpi)`, `overflow(bounds, rect) -> Overflow { left, top, right, bottom, fully_outside }` with `exceeds()`, `node_bounds(page, id)`. Every function checks finiteness of all inputs first, then ranges.
- `node_bounds`: leaf by `to_transform().transform_rect_bbox`, group by union of children, empty group by its own box; a missing child gives `UnknownNode`, a cycle gives `Cycle`; negative `w`/`h` give `InvalidBox`.

RED evidence (stub bodies returning the input / zero; the implementation was written first and the stub applied afterwards, see deviations)
- FD-05(b) `fd_05_b_overflow_at_the_safe_edge_boundary`: safe rect equals the page instead of (54..1026); FD-05(d) `fd_05_d_bleed_expands_all_four_sides_from_the_dpi_input`, `fd_05_d_bleed_range_is_zero_to_ten_mm_inclusive`: `Ok(page)` where `Err(OutOfRange bleed)` expected; FD-05(e) `fd_05_e_safe_rect_insets_each_axis_by_its_own_size`; FD-05(f) `fd_05_f_margins_that_reach_the_size_leave_no_area`, `fd_05_f_fd_09_d_overflow_edges_and_outside_rule`: right 0 where 24 expected; D-28 `d_28_a_leaf_uses_the_box_aligned_to_the_axes`, ADR 0009 `adr_0009_a_group_is_the_union_...`: zero rect returned; ARQ-110 `arq_110_every_numeric_input_rejects_non_finite_values` (`Ok` with NaN x0 where `NonFinite page` expected), `arq_110_inverted_boxes_are_errors`, `arq_110_bad_placements_and_broken_trees_are_errors` (`Ok` where `NonFinite x`).
- ARQ-111 `arq_111_the_same_input_gives_the_same_bounds` and the property test pass at the stub (determinism holds for constant output); proved only as a regression guard.

Hand mutants (each applied, a test failed, reverted, not committed)
1. Overflow boundary: right excess shifted by 0.5, `exceeds` with `>=` -> `fd_05_b_overflow_at_the_safe_edge_boundary` (+ `fd_05_f_fd_09_d_...`). Survivor: `excess` using `v >= 0.0` is an equivalent mutant (returns 0 either way).
2. `safe_rect`: horizontal inset from the height, vertical from the width, one side per axis not inset (x1, y0) -> `fd_05_e_safe_rect_insets_each_axis_by_its_own_size`.
3. `bleed_rect`: one side not expanded (x0, then y1), DPI fixed at 96, range `0..10` exclusive, upper bound 10.02, lower bound 0.001 -> `fd_05_d_bleed_expands_...` or `fd_05_d_bleed_range_...`.
4. Margins: `>` instead of `>=` for width and for height, either check removed -> `fd_05_f_margins_that_reach_the_size_leave_no_area`.
5. Overflow: each of the four "fully outside" comparisons made strict, left/top excess as `abs` -> `fd_05_f_fd_09_d_overflow_edges_and_outside_rule`.
6. Finiteness: each check removed one at a time (percent, dpi, margins sides, `rect`, `bleed`, the placement `skew`, the result check of `node_bounds`) -> `arq_110_*` tests. Two (bleed value, margin sides) first survived because `Length::convert` also rejects them; killed by adding ordering assertions (a NaN after a bad range, a NaN bleed with dpi 0): finiteness first, whole call.
7. `node_bounds`: rotation ignored, last child dropped, union replaced by the last or first child, empty-group fallback removed -> `d_28_...`, `adr_0009_...`; cycle guard removed -> test process aborts with stack overflow in `arq_110_bad_placements_and_broken_trees_are_errors`.
8. FD-09(d): left and right excess swapped -> `fd_05_b_...`, `fd_05_f_fd_09_d_...`; `>` for the x = 1080 rule -> `fd_05_f_fd_09_d_...`.
All killed except the equivalent one noted in item 1.

Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (944 passed, 0 failed). A seed file written by a mutant run was removed before the commit; no `*.proptest-regressions` is committed. V5 row checked by hand with `cargo tree -p gafira-layout`: gafira-color gafira-doc gafira-geom.

Measured: ~277 source lines, ~490 test lines, 17 Cargo.toml, clippy.toml copy; over the ~700 ceiling counting the clippy.toml copy.

Deviations: (a) the implementation preceded the tests; RED was produced afterwards by stubbing the bodies. (b) `percent` is "5.0 = 5 %" (spec text says 5 %, `[0, 50)`); margins take `&Margins` plus a dpi (not four px) because FD-05 stores them as `Length` in any unit. (c) `Overflow::exceeds` is an addition for the warning rule.

### D3 fix round 1 (verify round 1)

Order: every item had its test written and run before the code (W1 answered; the first round's stub RED is replaced by real RED below). Commits: `2e2993a` (S1), `9ec851b` (C1, W5, range rule), then the rects commit (C2 to C4, W4, W7, DPI, margins). `f2dafd1` untouched (W2).

RED (failing run before the code change)
- C1 `arq_110_a_very_deep_chain_gives_a_result_not_a_stack_overflow` (100 000 levels, debug): "has overflowed its stack", SIGABRT. Green in debug and in release. Iterative walk with a frame stack, `BTreeMap` of finished boxes and `BTreeSet` of open groups (ARQ-10 bans hash types); no nesting limit; a closed loop at depth 100 000 gives `Err(Cycle)`.
- W5 `arq_110_a_group_checks_its_own_placement_but_not_nodes_outside_the_subtree`: `Ok` where `Err(NonFinite { field: "x" })` expected.
- C3 `arq_110_a_computed_non_finite_value_...`: `margin_rect(Rect(-MAX, 0, MAX, 100))` gave `Ok(Rect(-MAX, 10, MAX, 90))`; overflow `-inf` cases gave `Ok`.
- W4 `fd_05_f_rounding_that_erases_an_axis_is_no_area`: `Ok` with `x0 == x1 == 1.0000000000000004e16`. W7/margins per axis `fd_05_f_margins_are_decided_per_axis_...`: `Err(NoArea)` where the flat box with zero margins must stay `Ok`.
- Passed on existing code, proved by mutants only (no RED possible): ARQ-111 bytes (`arq_110_arq_111_finite_input_...`: M1, M2), `range_rule_a_skew_of_90_...` (mutant: range check on skew), `fd_05_a_dpi_that_is_not_positive_...` (dpi 0 and -96, both functions), `adr_0009_a_group_shared_by_two_groups_...`.

Hand mutants (applied, reverted, not committed)
- bounds: skip first child, last child wins (union replaced), group placement unchecked, open set not cleared after a group (false cycle on a shared group, survived until the shared-group test used a group), skew range check: killed by `adr_0009_*`, `arq_110_a_group_checks_*`, `adr_0009_a_group_shared_*`, `range_rule_*`.
- M1 `margin_rect` differs on every second call: killed by `arq_110_arq_111_...`. CORRECTION (round 2): the earlier note that M2 was killed was wrong; verify measured the round 1 M2 text (sign flips on every second zero excess, counted per excess) surviving 6 of 7 runs, because the operations had 0, 2 or 4 zero excesses and the period-2 flip repeated.
- M3a `bleed_rect` result check removed, M3b `safe_rect` result check removed, M3c check on one side only (four variants), intermediate infinity clamped again, margin width or sum check removed: killed by `arq_110_a_computed_non_finite_value_...`; M3b also by `fd_05_f_rounding_...`.
- Rounding check removed in `safe_rect`, in `margin_rect`, ignored on one axis: `fd_05_f_rounding_...`. Rounding applied on a zero-size axis, zero-size box with zero margins rejected, margins decided on the x axis only: `fd_05_f_margins_are_decided_per_axis_...` (the y-axis zero-size case was added after the per-axis mutant survived).
- Equivalent: none.

Gates: fmt, clippy `-D warnings`, `cargo test --workspace --locked` all pass at each commit; layout tests also pass with `--release`. Mutant runs wrote a `rects.proptest-regressions`; removed each time (all from mutants, none from real code).

Spec reading: field names of the new `NonFinite` errors are not specified; the code uses `overflow`, `page` (width, height) and `margins` (sums). `Rect(…)` with an inverted box after rounding cannot occur: `NoArea` comes first.

Measured: round delta over `f2dafd1` in `crates/`: 4 files, 331 insertions, 103 deletions (+228 net); the crate against `develop`: 1057 insertions. Not in scope: W3, W8, S2.

## D3 fix round 2 (tests only)
- C2: the bytes property in `tests/rects.rs` gains `overflow(Rect(x-1, y, x+w, y+h), page)`, one overflowing side and three zero excesses (odd). M2 as written in round 1 (static counter, sign of every second zero excess flipped): killed in 10 of 10 runs of `arq_110_arq_111_finite_input_gives_finite_repeatable_output`.
- C4: `tests/bounds.rs` gains `arq_110_arq_111_node_bounds_is_finite_and_repeatable` (finite output, `to_bits` equal over two runs, flat and nested groups, rotation and skew) and `arq_110_node_bounds_names_any_non_finite_field` (any of the six placement fields set to NaN or an infinity gives `NonFinite` naming it). The first passes on existing code and cannot see an input check removed, because its inputs are in the domain; the second is the one that kills: dropping each of `x`, `y`, `w`, `h`, `rotation_deg`, `skew` from `check_placement` fails it, 6 of 6 (reverted).
- Passed on existing code, no RED possible; proof is the mutants above. Mutants reverted, not committed; no `*.proptest-regressions` left.
- S3 left as is (not a one-word comment change).

## D4: guides and grid (`feat/layout-core-d4`)

Status: done after round 3 (the open question below was answered in round 2). Files: `crates/gafira-layout/src/{guides,grid,lib,rects}.rs`, `tests/{guides_position,guides_generate,grid}.rs`.

API (the spec names only `generate(spec, page)`): `place_guide(axis, written, ruler, dpi) -> (Guide, Length)`; `GuideSpec { columns, rows, gap, margin }` and `generate(&spec, page: Rect, dpi) -> Vec<Guide>` (px, vertical first); `grid_lines(&GridSettings, view: Rect /* page px */, zoom /* ratio, 0.05 to 32 */, dpi) -> GridLines { vertical, horizontal }` of `GridLine { position, major }`. `rects::convert` is now `pub(crate)`. Errors: `columns` / `rows` for `c < 1`, `gap`, `margin`, `step`, `subdivisions`, `zoom`, `dpi` out of range; `NonFinite` names `position`, `gap`, `margin`, `dpi`, `page`, `view`, `zoom`, `step`.

RED (stub that compiles; each failed on an assertion unless noted)
- FD-03(a) `fd_03_a_position_is_stored_unrounded_and_shown_within_a_hundredth`, `fd_03_a_typed_value_survives_the_extreme_dpis` (stub `Err(NoArea)`, failed at the `unwrap`). FD-03(d) `fd_03_d_non_finite_position_or_dpi_is_rejected_by_field`: `left Err(NoArea)`, `right Err(NonFinite{position})`; overflow case likewise.
- FD-04(a) `fd_04_a_twelve_columns_in_1080`: `left 0, right 24`; (d) `fd_04_d_column_width_boundary_is_one_px_inclusive`: `left [], right [0,1,9,10]`; (e) `fd_04_e_three_by_three_in_1080`: `left 0, right 12`; (f) `fd_04_f_zero_gap_...`, `fd_04_f_negative_gap_or_margin_...`: `Ok([])` vs `Err(OutOfRange gap)`; `fd_04_zero_divisions_...`, `fd_04_columns_use_the_width_and_rows_the_height`, `fd_04_gap_and_margin_follow_the_dpi_input`, property `fd_04_every_axis_has_2n_ordered_mirrored_guides`, `arq_110_every_float_input_is_checked_even_when_unused`, `arq_111_generating_twice_...`: all `Ok([])` against expected guides or errors. `arq_110_finite_input_gives_finite_output` passed vacuously at the stub.
- FD-07 (a) `fd_07_a_lines_fall_on_multiples_of_the_converted_step`: `left 0, right 2545`; (d) `fd_07_d_ten_px_step_with_five_subdivisions`: `left [], right [0,40,...,200]`; (e) `fd_07_e_ranges_...`: `Ok` vs `Err(OutOfRange step)`; fallback, 6 px / 25 % table, off-page view, ARQ-110, ARQ-111, small-step test: empty lines. `fd_07_c_at_most_2000_lines_...` passed vacuously at the stub (empty output); it is proved by the mutants and by `fd_07_c_a_small_step_at_a_high_zoom_stays_under_the_limit` (641 + 361 lines in the densest case).

Hand mutants (applied, tests failed, reverted, not committed)
- Guides: (1) `i * (cell + gap)` to `i * cell`, `(i - 1) * gap` to `i * gap` -> `fd_04_a_twelve_columns_in_1080`, `fd_04_e_...`; (2) margin guide dropped at margin 0 -> `fd_04_columns_use_the_width_and_rows_the_height`, `fd_04_a_...`; (3) dedup removed -> `fd_04_f_zero_gap_emits_each_position_once_per_axis`; dedup across axes (seen-set over both) -> `fd_04_e_three_by_three_in_1080`; (4) horizontal first -> `fd_04_f_zero_gap_...`; sort removed or reversed -> `fd_04_a_...`; (5) `c >= 1` to `c > 1` -> `fd_04_d_column_width_boundary_is_one_px_inclusive`; (6) columns use the height -> `fd_04_columns_use_the_width_and_rows_the_height`; (7) rounding to 0.01 -> `fd_04_a_...`, property; (8) `n = 0` emits -> `fd_04_a_...`, `fd_04_f_negative_...`; (14) `dpi` check dropped -> `arq_110_every_float_input_is_checked_even_when_unused`.
- Grid: (9) 25 % `>=` to `>` -> `fd_07_minor_grid_needs_25_percent_zoom_and_6_px_spacing`; (10) minor `>=` to `>` same test, and the doubling `<` to `<=` (survived once, killed by the added `px(3.0)` case: 3 px doubles to exactly 6); (11) fallback by `+= 1` instead of doubling -> `fd_07_fallback_is_the_smallest_power_of_two_multiple`; (12) minor shown without the 6 px check -> `fd_07_d_...`; (13) zoom, step and subdivision ends made exclusive, each side -> `fd_07_e_ranges_include_both_ends_and_reject_just_outside`; (14) finiteness drops for `zoom`, `step`, `dpi`, `view` -> `arq_110_every_float_input_is_checked`; major flag inverted, minor flag ignored, y range used for x -> `fd_07_d_...`, `arq_111_...`.
- Equivalent (no test can tell, noted rather than killed; the `ceil`/`floor` swap and the raw `finite(gap)` / `finite(margin)` checks were claimed here and are NOT equivalent, see round 3): `(count - 1) * gap` finite check (the next check on `cell` fails identically); `major < 6` to `<= 6` (the fallback with multiple 1 gives the same lines); fallback multiple starting at 2 (the major level never fits at 1); (15) accumulating positions by repeated addition (drift below 1e-9 px over the lines of a screen; the tolerance is 0.01 px).

Deviations / findings
- FD-03(a): `Length::convert` has no limit that bites in 1 to 10 000 DPI; it returns `NonFiniteResult` only on overflow, and `place_guide` maps that to `NonFinite { position }`.
- OPEN QUESTION (ARQ-110, spec silent): the number of guides or lines is not bounded for inputs outside the stated domain. `generate` with a very large page side and a large `columns` (a `u32`) allocates up to 2n guides; `grid_lines` over a view far larger than 3840 x 2160 screen px loops over every line. FD-07(c) bounds the count only for a viewport up to 3840 x 2160. No limit was invented. Question for the owner: should `generate` and `grid_lines` return `Err(OutOfRange)` above a stated maximum (for example the page side maximum 16 384 px for `generate`, a view of 3840 x 2160 screen px for `grid_lines`), or is the caller trusted? Every `Vec` is built before return, so no partial result exists either way.
- Size: `git diff --shortstat develop -- crates` gives 879 insertions: about 190 source lines and 690 test lines. Over the ~700 ceiling in total; the source part is under the 340 forecast.
- `.github/workflows/ci.yml` and the V5 row need no change.

Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` pass on each commit.

## D4 round 2: bounded guide and grid output (rows "Lado de página de `generate`" and "Vista de `grid_lines`")

Commits: `generate` page side limit (`src/guides.rs`, `tests/guides_generate.rs`); `grid_lines` view limit (`src/grid.rs`, `tests/grid.rs`). The earlier open question is answered by these rows.

RED (before the code change, assertion failures)
- Page side: `fd_04_page_side_limit_is_inclusive_and_checked_per_side` (`left Ok([.. 16384.01 ..])`, `right Err(OutOfRange page)`); `fd_04_page_side_is_checked_after_non_finite_and_before_the_column_width` (`left Err(OutOfRange columns)`, `right Err(OutOfRange page)`). The property `fd_04_every_ok_has_at_most_32768_guides_per_axis` passed on the old code (c >= 1 px already implies n <= side); no mutant that drops the side check is killed by it alone, the example tests carry the proof.
- View: `fd_07_view_screen_size_is_limited_per_axis_with_exact_comparison` (`Ok` where `Err(OutOfRange view)` was expected); `fd_07_view_is_checked_after_non_finite_and_the_zoom_range` already passed in its zoom and NaN parts and failed only on the overflow-product case once the check existed (mutant below), so its order proof is the mutants.

Existing tests adjusted (views or pages above the new limits): `guides_generate.rs` gap-overflow case uses `page(1000.0, 1.0)`; `grid.rs` `screen()` now trims the far edge so `width * zoom <= 3840` holds exactly, `fd_07_a` caps `hi` at `3800 / zoom`, `fd_07_e` uses a 50 x 50 view (100 x 100 at zoom 32 is 3200 px high).

Hand mutants (applied, failed, reverted, not committed)
- `>=` on the side: `fd_04_page_side_limit_is_inclusive_...`, `..._after_non_finite_and_before_...`. Skipped when n = 0: `fd_04_page_side_limit_is_inclusive_...`. Only width / only height: same test. Check after the column-width rule: `fd_04_page_side_is_checked_after_non_finite_and_before_the_column_width`.
- View `>=` on both axes, on width only, on height only: `fd_07_view_screen_size_is_limited_per_axis_with_exact_comparison`, `fd_07_c_*`. Ignores zoom: same test and `fd_07_c_at_most_2000_...`. Width and height swapped, either accepted (`&&`), only width, only height: `fd_07_view_screen_size_is_limited_per_axis_...`. Check before the zoom range: `fd_07_view_is_checked_after_non_finite_and_the_zoom_range`, `arq_110_every_float_input_is_checked`. Product overflow reported as the range rule: `fd_07_view_is_checked_after_non_finite_and_the_zoom_range`.
- No `*.proptest-regressions` kept in the layout crate.

## D4 round 3: verify round 1 fixes (C1, C2, C3, W1 to W4)

Commits: `bde75e9` (public `grid` and `guides` modules), `b46d0b8` (grid), `ab93c04` (generate). Gates pass on each: fmt, clippy, workspace tests.

RED (assertion failures before the code change)
- C1 `fd_07_a_every_line_between_two_edge_lines_is_kept` (exact integer oracle, no `prop_assume!`): `left 0, right 1` lines; `fd_07_a_mm_steps_keep_a_line_on_either_view_edge`: `left 13, right 14` (1 mm at 150 dpi, k = 13).
- Epsilon and W3 `fd_07_view_screen_size_is_limited_per_axis_with_a_tolerance_of_a_millionth` (3840.0000009 gave `Err(view)`); `fd_07_c_at_most_2000_...` fails on the plain `x + 3840 / zoom` view (`screen()` has no `next_down` trim now, corners to 16 384).
- W1 `fd_07_dpi_is_checked_after_non_finite_and_before_every_range_rule`: `left Err(zoom), right Err(dpi)`; `fd_04_dpi_is_checked_after_non_finite_and_before_the_other_range_rules`: `left Err(dpi), right Err(NonFinite gap)`.
- W4 modules: the tests import `gafira_layout::{grid, guides}`; RED was the compile error E0603 (private module), the only form a visibility fix can take.
- No RED possible, the code was right (proof is the mutants below): C2 values beside the limits, C3 `fd_03_guides_are_measured_from_the_top_left_corner_of_the_page`, W2 `fd_07_c_the_densest_cases_stay_under_the_limit`, W4 `fd_07_lines_at_negative_positions_are_major_...`.

Hand mutants (applied, failed, reverted, not committed)
- V1 `cell < 1 - 1e-9` -> `fd_04_d_column_width_boundary_is_one_px_inclusive`. V2 dedup within 1e-6 -> `fd_04_f_zero_gap_emits_each_position_once_per_axis`. V3 positions offset by the page corner, columns and rows each -> `fd_03_guides_are_measured_from_the_top_left_corner_of_the_page`. V5 page side `+ 1e-6`, width and height each -> `fd_04_page_side_limit_is_inclusive_and_checked_per_side`.
- V4 view epsilon 1e-7, 1e-5, 0, on one axis only (each) and `>=` at the limit (each axis) -> `fd_07_view_screen_size_is_limited_per_axis_with_a_tolerance_of_a_millionth` (epsilon 0 and one axis also `fd_07_c_at_most_2000_...`).
- V11 `ceil`/`floor` swapped (the original code, first and last bound each) -> `fd_07_a_every_line_between_two_edge_lines_is_kept`, `fd_07_a_mm_steps_keep_a_line_on_either_view_edge`. V9 no major flag on negative `k` -> `fd_07_lines_at_negative_positions_are_major_...`, `fd_07_a_every_line_...`.
- V6 minor without the 6 px check -> `fd_07_c_the_densest_cases_stay_under_the_limit` (1, 10 subdivisions, zoom 32: 121 lines, not more) and 3 more. V7 fallback removed -> `fd_07_c_the_densest_cases_...` (step 1 at zoom 0.05) and 5 more. Both now die from the direct test, not from chance.
- dpi check removed or after the zoom check in `grid_lines` -> `fd_07_dpi_is_checked_after_...`. Raw `finite(gap)` / `finite(margin)` removed in `generate` -> `fd_04_dpi_is_checked_after_...`.
- Inclusive filter made exclusive at either view edge -> 7 grid tests.

Changes by item
- C1: first and last `k` widened by one line, the `contains` filter decides. W3: epsilon `1e-6` px on each axis of the view limit; docs say so; `screen()` has no trim; the `3800 / zoom` cap went with `fd_07_a`, now an exact integer property. W1: dpi first in `grid_lines` after the non-finite checks; non-finite `gap` and `margin` before `convert` in `generate`. W4: `pub mod grid`, `pub mod guides`; root re-exports kept. S1: the 641 x 361 case uses step 6, 1 subdivision, and zoom 33 uses 1000 x 1000; S3: this section and the status line.
- W5: removed `fd_03_a_typed_value_survives_the_extreme_dpis` (the property covers it) and the zoom and view NaN cases of the order test (`arq_110_every_float_input_is_checked` has them).
- Not done: S2 (V10 is unreachable, no spec rule); W2 property for the `2^k` fallback stays as is.

- Size: round 3 adds 235 insertions and 84 deletions in `crates` (src 25 inserted, tests 210). Against `develop` the branch was 1 235 insertions in `crates` plus notes (the earlier 1 370 also counted 135 lines of verify artifacts); well over the ~700 ceiling, as already flagged in W5.

## D4 fix round 2: verify round 2 (C4, W6, S4)

Commits: `9ab2e9f` (majors), `8c261f3` (W6 tests), one follow-up clippy fix (see deviations), then these notes. Final tree passes fmt, clippy, workspace tests.

RED
- C4 `fd_07_a_major_line_on_a_view_edge_is_kept_for_a_non_integer_step_with_subdivisions`: `left 33, right 34`. `fd_07_a_majors_sit_exactly_on_multiples_of_the_step` (oracle: `m * n + 1` lines, major at `i % n == 0` equal to `(i / n) as f64 * step`, minor within 1e-9 of `i * step / n`): `left 20.499999999999996, right 20.5`.
- W6: no RED, the code was right (`fd_03_d_non_finite_position_is_reported_at_dpi_zero`, `arq_110_a_nan_margin_is_non_finite_at_dpi_zero`).

Fix: a major line is `(k / every) as f64 * unit`; minor lines stay `k * unit / div`. The fallback (every 1) is the same value as before.

Hand mutants (applied, failed, reverted, not committed)
- Majors as `k * unit / div` -> the two C4 tests. Major test `k % every == 1` -> 8 grid tests, among them both C4 tests. Duplicate emission at a major position -> 13 grid tests, among them both C4 tests.

Deviations
- The second commit passed my gate script although clippy failed on `grid.rs` (`manual_is_multiple_of`), and it also committed `grid.proptest-regressions` made by the RED run. No amend, so a third commit fixes both; the first two commits do not pass clippy alone.
- S4: corrected above (src 25, tests 210).

## D5 + D6 + D7: snap, align, distribute, benches (`feat/layout-core-d5`)

Status: done. One delivery by owner decision (Slice card amendment `a584943`). Files, all in `gafira-layout` unless noted: `src/{snap,attractors,align,distribute}.rs` (new), `src/{grid,bounds,lib}.rs`, `tests/{snap,snap_rotation,attractors,align,distribute,benches}.rs` (new), `benches/layout.rs` (new), `Cargo.toml`, `Cargo.lock`, `crates/gafira-layout/Cargo.toml`.

Commits (each passed `cargo fmt --check`, `clippy --workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked` alone; exit codes read one by one, no pipe on the gate)
1. `4a99a1a` refactor: `grid::level` and `Level` shared by `grid_lines` and the magnet (no new rule; the 17 grid tests are the guard).
2. `117c1dc` snap: kinds, threshold, zoom, DPI, ARQ-110/111, priority, ties, guides, margins, page, elements.
3. `866b8b6` snap to the grid lines of `grid_lines`.
4. `761a2ef` refactor: `bounds_with` keeps a cache so many `node_bounds` calls walk each node once.
5. `7a1b454` `attractors` (FD-06 c).
6. `65217bc` `snap_rotation` (FD-06 i).
7. `b947b02` `align` (FD-11 a, d, f; D-29).
8. `594f304` `distribute` (FD-11 b, d, e; D-17).
9. `0c686e2` `criterion` and the four benches (ARQ-114).

API: `snap(&SnapInput) -> Result<SnapResult>` with `SnapKind {Guide, Margin, Page, Element, Grid}` (declared in priority order), `SnapTarget {coordinate, kind}`, `AxisSnap {offset, target}`; `snap_rotation(angle)`; `attractors(&Page, &[NodeId]) -> Result<Vec<Rect>>`; `align(AlignAction, page, &[Item])`, `distribute(DistributeAxis, DistributeMode, &[Item])` with `Item {rect, locked}` and `Offset {dx, dy}`.

RED (each test written first, run against a compiling stub, failing on an assertion; the stubs returned "no snap", `Ok(vec![])`, the input angle, zero offsets)
- FD-06 a, d(property), e, f, g, h; D-16; ARQ-110/111 (commit 2): 25 of 28 tests failed (`left: None right: Some(SnapTarget {..})`, `Ok(..)` where `Err(NonFinite {field})` expected). Passed at the stub: `fd_06_h_*`, `arq_111_*`, both property tests on finiteness and on the threshold (constant output); they are proved by the mutants below.
- Grid target (commit 3): 8 of 8 new tests failed (`left: None right: Some(10.0)`; `Some(SnapTarget Guide)` where Grid expected for "grid at 2, guide at 3"; the grid_lines property `expect("a line within the threshold")`).
- `attractors` (commit 5): 15 of 17 failed (`left: [] right: [boxes]`, `Ok([])` where `UnknownNode(99)` or `NonFinite{x}` expected). `arq_111` passed at the stub; the NaN-outside test got an `assert_eq!(want.len(), 2)` so it fails at the stub too.
- Rotation (commit 6): 7 of 10 failed (`Ok(7.4)` where `Ok(0.0)`, `Ok(NaN)` where `Err(NonFinite{angle})`, `-0.0` bits where `0.0`).
- `align` (commit 7): 9 of 12 failed (`dx 0.0` where `-50.0`, `Ok([])` where `Err(NonFinite{page})`). The `-0.0` zero test and the empty/single-locked test passed at the stub.
- `distribute` (commit 8): 12 of 16 failed (`[0.0; 4]` where `[0.0, 30.0, 65.0, 0.0]`). My first implementation sorted with `total_cmp`; `d_28_equal_positions_keep_the_input_order` caught it (`-0.0` ties).
- ARQ-114 (commit 9): `arq_114_each_layout_figure_has_a_benchmark_named_after_its_row` failed with "`bench_guides_generate` must be registered once" against an empty `main`.

Hand mutants (each applied, a test failed, reverted, never committed; `cargo test --no-fail-fast` since cargo stops at the first failing binary)
1. `<` for `<=`; zoom division removed; threshold times zoom: `fd_06_the_threshold_is_compared_exactly_and_divided_by_the_zoom`, `fd_06_e_distance_6_snaps_and_6_01_does_not`, `threshold_must_be_finite_and_above_zero`, `d_16_the_winner_is_*`.
2. Each adjacent pair swapped (Guide/Margin, Margin/Page, Page/Element, Element/Grid): `fd_06_f_at_equal_distance_each_kind_beats_every_lower_one` (Element/Grid also `fd_06_f_the_grid_loses_a_tie_*`).
3. Priority before distance: `fd_06_f_a_nearer_target_of_a_lower_kind_beats_a_farther_higher_one`, `d_16_the_winner_is_*`.
4. Larger coordinate, coordinate ignored: `fd_06_after_the_kind_the_smaller_coordinate_wins`; later reference: `fd_06_after_the_coordinate_the_reference_order_is_left_centre_right`. Reference order before coordinate survives: equivalent. If two targets are tied at the minimal distance from two different references `a < b` and the later reference has the smaller coordinate, then `a + d > b - d`, so the centre reference lies strictly closer than `d` to one of them. A tie in which the order differs from the coordinate order cannot be the minimum.
5. Axes coupled (y uses x references, x uses y targets, guide axis swapped): `an_element_attracts_*`, `fd_06_g_each_axis_is_resolved_on_its_own`, `d_16_*`.
6. Left, centre and right reference each dropped (x only, y only; replaced by a copy of another reference, so the error path does not kill it): `fd_06_each_reference_reaches_targets_on_its_own`, `fd_06_the_grid_is_reached_from_each_reference_of_the_box`. A first run used `INFINITY` and was killed only by the distance error; redone.
7. Centre dropped for page and for element, x and y; margin centre added; margin edge dropped: `the_page_attracts_*`, `an_element_attracts_*`, `the_margin_rectangle_attracts_*`.
8. Grid attracts without `visible`, without `snap`, `||`; ranges skipped when it does not attract: `fd_06_the_grid_attracts_only_when_visible_and_set_to_snap`, `grid_ranges_are_checked_also_when_the_grid_does_not_attract`.
9. Inactive skips checks / moves: `fd_06_b_an_inactive_magnet_*`, `arq_110_every_float_input_*`.
10. Hidden guide attracts / skips finite / skips conversion: `guides_convert_to_px_*`, `the_order_of_checks_*`, `arq_110_a_result_that_is_not_finite_*`.
11. Margin as Page kind, margins None treated as Err: `d_16_zero_margins_give_the_margin_kind_*`.
12. Attractors: selection descendant kept, selected node kept (equivalent: the ancestor pass also clears it), ancestor kept, hidden/locked ancestor not inherited, group box its own placement, sibling dropped, bounds of every node read (NaN in an excluded node), hidden ignored, locked ignored, unknown selection ignored, output reversed, selection not bubbled, selection at a root ignored: `fd_06_c_*`, `arq_110_a_nan_outside_*`. The cache removed in `attractors` (quadratic) is killed by timeout: the 100 000 chain does not finish in 60 s against 0.2 s.
13. Rotation `floor(a/15 + 0.5)`, `round`, `round_ties_even`, tie to the lower value, normalized with `rem_euclid(360)` and `%`, `-0.0` kept, overflow check removed: `fd_06_i_*` and `arq_110_a_result_that_overflows_*`.
14. Major as `k * step / subdivisions` (in `grid::Level::line`): `fd_06_a_major_line_is_exactly_k_times_the_step` and the grid tests; candidates `[k]`, `[k-1,k]`, `[k+1,k+2]`, tie up, `ceil`, `round`: `fd_06_the_grid_*`. Survivor, equivalent: "first found wins a tie" (candidates are scanned in ascending order). CORRECTED in fix round 1: candidates `[k, k+1]` are not equivalent, they differ from about 6e14 px and are killed by `fd_06_the_grid_target_is_the_nearest_line_also_where_the_quotient_rounds`. The old claim: "the extra neighbours only matter when the quotient rounds across an integer" was false. And, old text: "first found wins a tie" (candidates are scanned in ascending order).
15. Align: other axis moved (Left, Top, Middle), locked excluded from the union, locked box moves, one free uses its own box, one free with one locked uses the page, single locked moves, `-0.0` kept, offset check removed, order before numbers, page/locked/centre unchecked, output shorter: `fd_11_*`, `d_29_*`, `arq_110_*`. Two shortcuts (single locked, all locked) were equivalent mutants and are deleted.
16. Distribute: first or last moves, locked counted, 4 free needed, unstable sort, `total_cmp`, ties reversed, negative gap rejected or clamped, centres mode using edges, gap mode sorting by centre, `n` for `n - 1`, cursor ignoring size, axis swapped, output reversed or shorter, gap/offset check removed: `fd_11_*`, `d_17_*`, `d_28_*` (the unstable sort first survived on already-sorted input; `d_28_a_long_shuffled_run_*` kills it). CORRECTED in fix round 1: `n < 2` for `n < 3` is NOT equivalent (the gap of two wide overlapping boxes overflows, `Err` instead of no movement); killed by `d_17_two_wide_overlapping_free_elements_give_no_error_and_no_movement`. The inner `finite` checks of distribute were unreachable and are removed (the `+ 0.0` was reachable, W3, and is back); an offset overflow is reachable and tested (`[-MAX, MAX]`, `[-MAX/2]`, `[MAX/8]`).
17. Benches: each name renamed, registered twice, `criterion_main!` removed: `arq_114_*`.

Checks and choices the specs leave open (no spec gap blocked the work)
- Order of checks in `snap`, `align`, `distribute`: all numbers of all inputs, then all `x1 < x0` boxes, then DPI (`snap`), then threshold, zoom, grid ranges, then computed values. Fields: `moving`, `page`, `margins`, `element`, `guide`, `step`, `zoom`, `threshold`, `dpi`, `distance`, `item`, `gap`, `offset`, `angle`.
- Every centre is checked (moving, page, element, item) with the magnet off and for an align action that ignores it. The spec's `x = -MAX` example with a zero-size box fails at the centre (`moving`); the `distance` error is tested with `Rect(-MAX, 0, 0, 0)`. An inactive magnet computes no distances.
- The caller passes `margins: None` when `margin_rect` gives `NoArea`; the crate has no helper for it.
- `attractors` returns boxes in document order (roots as listed, parent before children). A group that attracts reads every descendant through `node_bounds`, hidden ones included, so a NaN in a hidden child of an attracting group is an error. A cycle anywhere in the tree is `Cycle`, also under a selected node. Only nodes reachable from the roots are returned.
- Zero offsets are `+0.0` in all four functions (snap, align and distribute normalize; fix round 1 found that distribute can produce `-0.0`).

Gates and checks: the three gates pass at every commit (final: 1 087 tests); `cargo test -p gafira-layout --release` passes (100 000 level chain in debug and release); `cargo deny --offline check` passes with `criterion` (advisories, bans, licenses, sources ok), `deny.toml`, the V5 row (`cargo tree -e normal,build` still lists color, doc, geom) and `clippy.toml` need no change. `criterion` is a workspace dev-dependency with `default-features = false, features = ["cargo_bench_support"]`: no rayon, no plotters, no reports. `Cargo.lock` gains 33 packages (dev-only; `half` comes in through `ciborium`, so ARQ-117's "no `half`" still holds for normal builds). No CI step runs the benches. Seed files written by mutant runs were removed; `git diff --name-status develop` shows no deleted file.

Bench figures, this machine, worst-case inputs, not the D-03 reference: `bench_guides_generate` 353 µs (16 384 x 16 384 divisions), `bench_layout_rects` 25 ns, `bench_snap_1000` 31 µs, `bench_grid_lines` 7.9 µs.

Measured (`git diff --shortstat develop -- crates Cargo.toml Cargo.lock`): 17 files, 4 026 insertions, 43 deletions: source +804 -42, tests and bench +2 918, `Cargo.lock` +299. Over the ~700 reference by far; the owner waived the ceiling for this delivery. Deviations: none from the Slice card. Next: verify.

## Fix round 1 of D5-D7 (verify-report round 1)

Status: done, no spec gap. Spec rows read: "Entradas del Imán", "Elementos que atraen", "Destino en la Cuadrícula", "Umbral del Imán" (`ab3cd7a`, `971d7c4`).

Commits: `d19a59a` (C1, W1, W2 in `snap.rs`), `0554dfa` (C2, W3, FD-11 d in `distribute`), `3ee32de` (C3, C4, FD-03 c tests, attractors doc), `00b1bcd` (inactive magnet and guides in px). Gates (fmt, clippy, test, exit codes read one by one): `d19a59a` all 0; `3ee32de` and `00b1bcd` all 0. `0554dfa` was split from the tree that passed all three gates, so its own gates were not run (subset of a green tree; no standalone run).

RED per item
- C1: `fd_06_a_grid_target_is_never_negative_zero_on_any_level`, `left: 9223372036854775808 right: 0` (major level, reference `-0.0`; the test also runs `(-0.3).round()`, a minor level, a major-only level and the `step * 2^k` level, on both references). Fix: `floor() + 0.0` in `grid_target`, the only place where `k` can be `-0.0` (`k - 1`, `k + 1`, `k + 2` never are).
- C2: passes on existing code (the code was right): `d_17_two_wide_overlapping_free_elements_give_no_error_and_no_movement`. Proved by the `n < 2` mutant (killed).
- C3: passes on existing code: `the_box_of_every_input_is_checked_before_the_dpi_rule` (moving, page, margins, element, x and y inverted, dpi 0). Proved by the box-after-DPI mutants.
- C4: passes on existing code: `arq_110_a_hidden_or_locked_child_of_an_attracting_group_is_read_and_one_outside_is_not` (E hidden and locked with NaN `x`: moving C and empty selection give `Err x`; moving D, B or A do not). `tests/attractors.rs:357` ("hidden under an attractor", read) agrees with the new wording; the doc of `attractors` said "placements elsewhere are not checked" and now says descendants of attracting nodes are read.
- W1: `arq_110_an_inactive_magnet_computes_no_centre_quotient_guide_or_distance`, `Err(NonFinite { field: "moving" })` where `Ok` (five inputs, active and inactive side by side). `arq_110_an_inactive_magnet_still_computes_the_step_on_screen` and `..._still_checks_the_inputs_that_do_not_attract` passed on existing code (they pin behaviour the fix must keep). Fix: `snap` returns after `level(...)` when inactive; nothing is computed before it.
- W2: `fd_06_the_grid_target_is_the_nearest_line_also_where_the_quotient_rounds` passes on existing code and kills `[k, k+1]`. Decision: code NOT changed. An oracle of 129 lines around the reference agrees with the code at 0 of 20 000 cases per band up to about 1e15 px and disagrees from about 1.3e15 (about 2^51) because the rounded quotient is off by many lines and the lines collapse onto the same `f64`; a fix needs a window that grows with `|at|`, against the no-loop rule (`arq_110_the_grid_does_not_loop_over_a_huge_reference`), at 4.5e10 times the largest page. The claim is corrected above and the `grid_target` doc states the limit. If the owner wants "nearest" exact past 2^51 px, that is a spec decision (range) first.
- W3: `fd_11_a_zero_offset_is_positive_zero_also_when_the_gap_underflows`, `left: 9223372036854775808 right: 0` (`EqualGap`, boxes `(-1,-0.0)`, `(0,5e-324)`, `(0,1)`). Fix: `+ 0.0` in `moved`. Equal-centres mode cannot give `-0.0` (a target is never `-0.0`), tested on the same input. `align` and `snap` already normalize (mutants below).
- W4: `fd_03_c_the_magnet_snaps_to_a_guide_within_the_threshold_and_not_beyond`, `fd_11_d_four_elements_with_one_locked_distribute_the_three_free_ones`; both pass on existing code. S2 to S4 not done: S1 and S2 are spec or follow-up; S3 (bench comment) and S4 (wording) add no behaviour and are left to the owner.

Hand mutants (applied, failed, reverted, never committed)
- `n < 2`: `d_17_two_wide_overlapping_free_elements_give_no_error_and_no_movement`.
- Grid target without `+ 0.0`: `fd_06_a_grid_target_is_never_negative_zero_on_any_level`. Candidates `[k, k+1]`: `fd_06_the_grid_target_is_the_nearest_line_also_where_the_quotient_rounds`.
- Box check after DPI, one mutant each for moving, page, margins, element: `the_box_of_every_input_is_checked_before_the_dpi_rule` (moving also `the_order_of_checks_*`).
- Inactive magnet computing the moving centre, an element centre, the threshold quotient, a guide in px: `arq_110_an_inactive_magnet_computes_no_centre_quotient_guide_or_distance` (the guide in px survived until the MAX-inch guide case was added in `00b1bcd`).
- `+ 0.0` removed: `resolve` of `snap` (`arq_111_the_same_input_gives_the_same_bytes`), `snap_rotation` (`fd_06_i_a_zero_result_is_positive_zero`), `align` (`fd_11_zero_size_boxes_are_valid_and_a_zero_offset_is_positive_zero`), `distribute` (`fd_11_a_zero_offset_is_positive_zero_also_when_the_gap_underflows`).
- `attractors` reading every node and not only the attracting ones: `arq_110_a_nan_outside_the_attracting_nodes_changes_nothing` and the new C4 test.

Measured (`git diff --shortstat 971d7c4 -- crates`, before this note): 6 files, 312 insertions, 19 deletions. Under 700. Deviations: `0554dfa` has no standalone gate run (above); W2 is a corrected claim and a documented limit, not a code change; S3 and S4 not done.

## T4 fix round (mutants of `gafira-layout` and `gafira-doc`)

Status: done. Owner instruction: close the gaps; only a truly equivalent mutant stays. Rows read: `02-arquitectura-software.md` 4.17.2, FD-05, FD-06, FD-11. Final T4 of `gafira-layout`: 452 mutants, 434 caught, 11 unviable, 7 timeouts (all intrinsic hangs, below), 0 missed. `gafira-doc` `error.rs`: 5 caught, 1 unviable.

Commits: `9bc73d5`, `ba75462`, `1087905`, `80b77b9`, then the `grid_target` / `beats` simplifications and the test split, then the `mutants.toml` entries. Gates (fmt, clippy, test, exit codes read one by one) were run on the green working tree before each commit; the first four commits are subsets of it and have no standalone run. The later ones ran on their own tree.

Outcome per survivor
- `align.rs` `check_boxes` 62:37 `/`->`%`, 63:37 `/`->`%`: equivalent. The check only asks if the centre is finite; for finite `x`, `x/2` and `x%2` are finite, for an infinite sum both are not (NaN cannot arise after the number check). Excluded in `.cargo/mutants.toml`. 63:37 `/`->`*` was a real gap (y centre): `arq_110_a_centre_that_is_finite_is_not_an_error_however_large_the_box` (sum between MAX/2 and MAX, both axes).
- `align.rs:107` `[]` arm: removed (the general path gives the same result).
- `attractors.rs:83` `|=`->`&=`: `fd_06_c_a_node_reached_by_two_parents_takes_both_out_when_it_holds_the_selection`.
- `error.rs:36` `Display`: `arq_110_every_error_says_what_failed_and_names_its_input` (the message holds the field or the id; no text fixed).
- `grid.rs:113` `<`->`<=` in `level`: equivalent. At exactly 6 px the doubling loop runs zero times and yields `unit = step, div = 1, every = 1`, which the other branches give too (the minor branch needs subdivisions 1 at that spacing). Excluded.
- `guides.rs:110` `gap + 0.0`->`-`: the sign of the gap never reaches a position (it is never the first or only term and `cell >= 1`); the normalization is removed and `fd_04_a_gap_of_negative_zero_is_a_gap_of_zero` pins it bitwise. The margin keeps `+ 0.0`.
- `rects.rs:140` (four): `fd_05_the_warning_is_raised_by_any_one_side_alone` (each side alone, all zero, and as `overflow` reports it). Negative excess is unreachable (`overflow` clamps).
- `rects.rs:155` `>`->`>=`: not equivalent, `-0.0 - 0.0` is `-0.0`: `fd_05_an_edge_on_the_limit_has_an_excess_of_positive_zero`.
- `snap.rs` `Candidate::beats` 98, 101, 104, 106 (x2): equivalent as written, because each `<` sits after a `!=` guard (and references arrive ascending). Brute force over 3M random candidate sets (every field of the winner compared): 0 differences for all five. The function is now one tuple comparison `(distance, kind, coordinate, reference) <`, which leaves one equivalent mutant (`<`->`<=`: equal tuples are the same candidate), excluded; `==`, `>` are killed by the D-16 tests.
- `snap.rs` `grid_target` 172:63, 172:76 (x2): the tie clause `(distance == d && line < p)` never fired in the ascending scan. Brute force over 60M cases (units up to 1e9, references up to 1e25, half-spacing ties): 0 differences with and without it. Removed; ascending order and a strict `distance < d` keep the smaller of two equal lines (`fd_06_the_grid_attracts_to_the_nearest_line_on_either_side_and_a_tie_goes_down` kills `<=`).
- `snap.rs` `grid_target` 169:38 (`k + 2.0` `+`->`-`, `+`->`*`): claim "equivalent" is FALSE past 2^51 px. Brute force (24M cases, spacing >= 0.1875 px): 0 differences with a 3-neighbour window up to 2e15, 656 differences from 7e16 (e.g. step 58.227, 10 subdivisions, reference -72684900000000000: three neighbours give a line 20 px off, four give the reference itself). Test `fd_06_past_2_pow_51_the_target_is_a_line_that_grid_lines_draws_nearest` (3 cases, oracle: nearest line of `grid_lines`). Deviation: this pins behaviour in the range the doc declines to guarantee (the doc limit stays true in general); the owner may drop the test if "nearest" past 2^51 is out of scope.
- `gafira-doc` `error.rs:181`: `a_page_layout_error_exposes_its_cause`.

Timeouts (15 in the earlier run, re-run with `--timeout 150`)
- Hangs, detected by non-termination (still running at 150 s; the suite takes about 3 s): `attractors.rs:61:34`, `69:31` and `bounds.rs:59:23` (`+=`->`*=` on the child index: stays 0, the walk never ends), `grid.rs:82:35`, `83:34` (`/`->`*` in `axis_lines`: a loop over a range of astronomical size), `grid.rs:115:31`, `116:22` (the doubling loop of `level` never reaches 6 px). No assertion can fail first: the loop sits in the first call of nearly every test.
- Not hangs, only slow (the `u32::MAX` count walked 4 billion times): `guides.rs` 63:46 `%`, 63:46 `*`, 64:13 `>`, 64:13 `==`, 70:42 `/` and `+`, 70:38 `*`, 70:61 `+`. They were already caught by quick assertions, but the test binary waited for the slow tests. The `u32::MAX` assertions now live in `tests/guides_huge.rs` (sorted after `guides_generate`, so cargo stops at the first failing binary); `guides_generate` uses a count of 100 000. Re-run: all caught, none times out.

Measured (`git diff --shortstat 78906a8 HEAD`): see the result of the round; about 230 lines, mostly tests. Under 700.
