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
