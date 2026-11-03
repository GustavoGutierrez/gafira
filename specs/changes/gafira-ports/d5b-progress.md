# D5b progress (tdd-apply)

Flow: full | level: L3 | mutation: archive | verify: V2
Branch `feat/gafira-ports-d5b`, cut from `develop` ae9b544 (D1 to D6 merged). Design: `specs/changes/gafira-ports/design.md` (D5b row, P2, P4, P6, P8). Writes `export.rs`, the exporter side of `format.rs`, `assist.rs` (one impl), `tests/export.rs`, `tests/format.rs`, `tests/assist.rs` and this file.

## Commits

1. `fix(ports): add From<Cancelled> for ImportError and AssistError`
2. `fix(ports): leave import warnings as at the call on every Err`
3. `feat(ports): add the export types, PageRenderer and the exporter side of FormatRegistry`

## Done

- Fix 1 and 2: `From<Cancelled>` for `ImportError` and `AssistError` (ADR 0012 rule 4 via ADR 0014).
- Fix 3: `FormatRegistry::import` leaves `cx.warnings` at its length at the call on every `Err` (all fail, `Cancelled`, failed rewind after an earlier try, no candidates). The loop moved to a private `try_each`; `import` truncates on `Err`. The design (P8 `import` rule 2, "truncate before each try") follows 02 §4.5 rule 5 as the spec says: the design text is not edited here, see deviations.
- `export.rs`: `ExportCaps` (+ `contains(self, required)`), `ExportRequest`, `ExportScope`, `IssueSeverity`, `ExportIssue` (+ `blocks`), `ExportedFile`, `ExportReport`, `ExportJob`, `ExportSink`, `PageRenderRequest`, `PageRenderer`, `ExportError` (+ `From` for each wrapped error, `ToMessage`), `Exporter`. D-55 is stated in the doc comment of `ExportRequest`: «Exportar proyecto editable con recursos» is a save, not gated by `can_export`.
- `format.rs`: `FormatRegistry` gains `exporters`, `register_exporter`, `exporters`, `exporters_with(required)` (registration order). Each list checks its own ids, so `.gafira` can be importer and exporter.

## RED evidence

| Criterion | Test | RED (wrong skeleton body, failing assertion) |
|---|---|---|
| ADR 0012 rule 4 | `adr_0012_import_error_from_cancelled`, `adr_0012_assist_error_from_cancelled` | skeleton `From` gave `Unsupported` / `ModelMissing`: `assert_eq!` left ModelMissing right Cancelled(Cancelled); import `matches!` failed with `Unsupported` |
| 02 §4.5 rule 5 | `fd_31_warnings_are_restored_when_every_importer_fails`, `..._an_importer_cancels`, `..._the_rewind_fails_after_a_try` | "only the caller's warning is left" left 2 right 1; left 2 right 1; left 3 right 1 |
| P8 `contains` | `p8_contains_each_flag_alone`, `p8_contains_needs_every_required_flag` | "has transparency, requires multipage" left true right false; panic `transparency` (negative case) |
| FD-58 `blocks` | `fd_58_error_blocks_and_warning_does_not` | `assert!(!issue(Warning).blocks())` |
| P8 `register_exporter` | `p8_duplicate_exporter_id_is_rejected_and_changes_nothing`, `p8_same_id_as_importer_and_exporter_is_valid` | `unwrap_err()` on an `Ok`; `assert!(register_exporter(repeat).is_err())` |
| P8 `exporters_with` | `p8_exporters_with_filters_by_capabilities_in_registration_order` | left ["zeta","plain","alpha","mid"] right ["zeta","alpha","mid"] |
| ARQ-102 `ExportError`, ARQ-20, ARQ-06 `PageRenderer`, FD-57 `ExportScope`, From conversions | `arq_102_export_error_messages`, `arq_20_..._object_safe`, `arq_06_*`, `fd_57_*`, `adr_0014_export_error_from_each_wrapped_error` | Compile-level only: plain types, `to_message` delegating, `From` impls and a double over `PageRenderer`; there is no wrong body to fail an assertion on (same deviation as D1) |

## Hand mutants

| # | Mutant | Killed by |
|---|---|---|
| D5b-1 | `contains` tests only `transparency` | `p8_contains_each_flag_alone`, `p8_contains_needs_every_required_flag`, `p8_exporters_with_filters_by_capabilities_in_registration_order` |
| D5b-1b | `contains` ignores `notes` (last flag); ignores `bleed` | same three |
| D5b-2 | `contains` reversed (`required` contains `self`) | `p8_contains_needs_every_required_flag`, `p8_exporters_with_filters_by_capabilities_in_registration_order` |
| D5b-3 | `Warning` blocks; `Error` does not block | `fd_58_error_blocks_and_warning_does_not` |
| D5b-4 | `exporters_with` sorts by id | `p8_exporters_with_filters_by_capabilities_in_registration_order` |
| D5b-4b | `exporters_with` ignores `required`; filter reversed | same |
| D5b-5 | An importer with the same id blocks the exporter | `p8_same_id_as_importer_and_exporter_is_valid` |
| D5b-6 | A repeated exporter id replaces the old one | `p8_duplicate_exporter_id_is_rejected_and_changes_nothing`, `p8_same_id_as_importer_and_exporter_is_valid` |
| D5b-6b | Duplicate rejected but the item is pushed | same two |
| Fix-3a | Warnings kept after every importer fails | `fd_31_warnings_are_restored_when_every_importer_fails` |
| Fix-3b | Warnings kept after `Cancelled` | `fd_31_warnings_are_restored_when_an_importer_cancels` |
| Fix-3c | Warnings kept after a failed rewind | `fd_31_warnings_are_restored_when_the_rewind_fails_after_a_try` |

All applied by hand, a test failed, reverted; none committed.

## Gates (exit codes, run before each commit)

`cargo fmt --check` 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` 0; `cargo test --workspace --locked` 0; V5 step (extracted from `ci.yml`) 0.

## Measured lines

Against ae9b544: 6 files changed, 885 insertions, 5 deletions. Source only (`src/`): 314 insertions, 5 deletions; the rest is tests and this file. Total passes ~700.

## Deviations

- `ExportIssue`, `IssueSeverity`, `ExportReport`, `ExportedFile` are open shapes (the spec names them, never defines them). Chosen: `ExportIssue { severity, message: Message }`, `IssueSeverity { Error, Warning }`, `ExportReport { files: Vec<ExportedFile { name, bytes }>, warnings: Vec<Message> }`. For the owner to confirm.
- `ExportCaps::contains` takes `self` and `required` by value (both `Copy`) to satisfy clippy; the design only names `contains(required)`.
- `register_exporter` mirrors `register_importer` (hand-checked list, `RegistryError::DuplicateId`).
- Design P8 `import` rule 2 still says "truncate before each try"; the code now also truncates on `Err` per 02 §4.5 rule 5. The design text is for the specifier to align.
- Compile-level RED for plain types (see the RED table).
- Neighbouring gap, not fixed: `AssistError` implements `Error` with the default `source()`, so `Cancelled` is not exposed as the source, unlike `ImportError`, `RenderError` and `MediaError`.

## Follow-ups from verify of D2-D6

For the owner; no fix made here.

- D2 open shapes: `ReasonCode`, `NumberStyle` and `DateStyle` are shaped by the implementer, not fixed by the spec. `LicenseError::Rejected` carries no reason argument in its message, though the variant holds a `ReasonCode`.
- D4 open media plain data: `CodecId`, `CodecParams`, `StreamInfo` and the other media plain data are the implementer's shapes.
- D6: `RewriteRequest.language` is `Option<String>` instead of `Locale`. `AssetQuery` with `limit: 0` means no limit. The other asset and assist shapes (`AssetItem`, `AssetPage`, `Materialized`, `AssistReq`, `Tone`, `Length`, `Suggestion`) are also open.
