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
