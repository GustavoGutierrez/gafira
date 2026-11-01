## Slice card
Flow: full | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator | reason: new V5 row and cross-cutting choices (ADRs 0005 to 0010); owner decisions D-14 to D-39 settled
Flow-change: standard -> full (trigger: spike found a new V5 row and the guide-type location conflict, before implementation)
Product IDs: FD-03, FD-04, FD-05, FD-06, FD-07, FD-09, FD-11 first-slice criteria, ARQ-110, ARQ-111, ARQ-114 (source: docs/specs/04-funcionalidad-diseno-imagen-exportacion.md §2 and §3; docs/specs/02-arquitectura-software.md §4.17)
Scope: `gafira-doc` page layout data (bleed, margins, guides, grid), `gafira-edit` properties, and the new crate `gafira-layout`: bounds, rects, guides, grid, snap, align, distribute.
Non-goals: FD-53, FD-54, reflow, `notes`, `duration`, `background`, text and image nodes, UI, PDF export, `rstar`.
Deliveries (serial, design.md delivery table): D1 doc types and wire form ~460; D2 validation and edit properties ~420; D3 crate, V5 row, bounds, rects ~420; D4 guides and grid ~475; D5 snap ~475; D6 align and distribute ~335; D7 benches ~125.
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

- Size: round 3 adds 235 insertions and 84 deletions in `crates` (src 34 inserted, tests 201). Against `develop` the branch is now 1 370 insertions with these notes; well over the ~700 ceiling, as already flagged in W5.
