Change-level status: every delivery PASS (D1 r1, D2 r1, D3 r2, D4 r2, D5a r1, D6 r1, D5b r1); T4 critical 0 on gafira-ports and gafira-doc at f392064.

# Verify report: gafira-ports D5b, round 1

Commit verified: 951a5a6 (branch feat/gafira-ports-d5b; commits 28ffca8, 0eb654e, 25be3a8, 951a5a6 on develop ae9b544).
Flow: full | level: L3 | mutation: archive | tier: T4 at close | depth: V2 | round 1 (first verify of this code).
Build: scratch export of 951a5a6 (git archive, touched), own CARGO_TARGET_DIR, CARGO_INCREMENTAL=0; cleaned after.

## Gates (exit codes)

| Gate | Exit |
|---|---|
| cargo fmt --check | 0 |
| cargo clippy --workspace --all-targets --locked -- -D warnings | 0 |
| cargo test --workspace --locked | 0 (2267 passed, 0 failed) |
| CI forbid unsafe | 0 |
| CI V5 layer table | 0 |
| CI V12 clippy.toml copies | 0 |
| CI ARQ-107 | 0 |
| CI ARQ-07, V13, V14 (also run) | 0 (V13, V14 need git: run read-only in the worktree) |

Mutation: `mutation: archive`, no per-delivery run, no report; not a finding. T4 on gafira-ports and gafira-doc is owed before archive.

## Probes (step 1c, written before reading the implementer's tests, scratch only)

7 probe tests, all pass: `contains` each flag alone, all-but-one, cross flags, empty required; `exporters_with` filtering and registration order, empty required gives all; duplicate exporter id rejected without replacing; same id as importer and exporter valid; `blocks`; `ExportError` keys (render, store delegate, `error.export.io` with no argument, `error.job.cancelled`), `source()` set for every variant; `From<Cancelled>` for `ImportError` and `AssistError`; an exporter double gets pages through `ExportJob::pages`, writes through `ExportSink`, stops with `ExportError::Cancelled`; `import` leaves `cx.warnings` at its length at the call when all fail, on `Cancelled`, on a failed rewind after a try and with no candidates, and keeps only the successful try's warnings on `Ok`. Implementer tests cover every probed rule with equal or stronger assertions (6x6 flag matrix, both registration orders for `.gafira`). No weak-test finding.

## Criteria map

| Criterion | Proof | Status |
|---|---|---|
| FD-57 (types) `ExportScope`, `ExportRequest` | `fd_57_scope_pages_and_nodes_are_distinct_values` | covered |
| FD-58 `ExportIssue::blocks` | `fd_58_error_blocks_and_warning_does_not` | covered |
| ARQ-102 `ExportError`, one fixture per variant, key area `export`, delegation | `arq_102_export_error_messages` | covered |
| P8 `ExportCaps::contains`, each of 6 flags alone | `p8_contains_each_flag_alone`, `p8_contains_needs_every_required_flag`, `p8_caps_default_is_all_false` | covered |
| P8 `register_exporter` duplicates; `.gafira` both lists | `p8_duplicate_exporter_id_is_rejected_and_changes_nothing`, `p8_same_id_as_importer_and_exporter_is_valid` | covered |
| P8 `exporters_with` / 02 §4.5 UI list in registration order | `p8_exporters_with_filters_by_capabilities_in_registration_order`, `p8_exporters_keep_registration_order` | covered |
| 02 §4.5 S5 / ARQ-06 exporter gets pages via `PageRenderer` (ports + layers 0-2 only) | `arq_06_exporter_gets_pages_through_the_page_renderer`, `arq_06_renderer_and_sink_errors_reach_the_caller_as_export_errors` | covered |
| ARQ-20 object safety (`Exporter`, `ExportSink`, `PageRenderer`) | `arq_20_exporter_sink_and_renderer_are_object_safe` | covered |
| ADR 0014 / ADR 0012 rule 4 `From` for wrapped errors and `Cancelled` | `adr_0014_export_error_from_each_wrapped_error` | covered |
| 02 §6.4 temp file and rename in the sink, `ExportJob` not persisted, `ExportRequest` equality | doc comments on `ExportSink`, `ExportJob`; `PartialEq` on `ExportRequest` (round-trip pending `ParamValues` serde, per 02 §6.4) | covered (types) |
| D-55 editable project export is a save | doc comment on `ExportRequest`; no code gate | covered (documented; nothing to test in ports) |
| Fix 1: `From<Cancelled>` for `ImportError`, `AssistError` | `adr_0012_import_error_from_cancelled`, `adr_0012_assist_error_from_cancelled` | covered |
| Fix 2: 02 §4.5 rule 5, warnings as at the call on every `Err` | `fd_31_warnings_are_restored_when_every_importer_fails`, `..._an_importer_cancels`, `..._the_rewind_fails_after_a_try`, existing `fd_31_warnings_of_a_failed_try_are_discarded` | covered |

Criteria 13/13 covered. Product IDs in the D5b row: FD-57, FD-58, ARQ-102 all named by tests (`git grep`). None left out.

## Artifact integrity

Slice card in `apply-progress.md`: untouched by every D5b commit; design and 02 untouched. Commits conventional, no attribution lines, one reviewable unit each.

## Hand mutants (scratch export, each reverted and byte-checked)

| # | Mutant | Result |
|---|---|---|
| Spot-check D5b-2 | `contains` reversed (`required ⊇ self`) | killed: `p8_contains_needs_every_required_flag`, `p8_exporters_with_filters_by_capabilities_in_registration_order` |
| H1 | `ExportError::Io` puts the io text in a `detail` argument (ADR 0012 rule 3) | killed: `arq_102_export_error_messages` |
| H2 | `ExportError::Cancelled` gets its own key `error.export.cancelled` instead of delegating (ADR 0012 rule 4) | killed: `arq_102_export_error_messages` |
| H3 | `import` truncates warnings on `Ok` too, dropping the successful try's warnings (02 §4.5 rule 5) | killed: `fd_31_warnings_of_a_failed_try_are_discarded` |

4 run, 4 killed.

## Findings

### CRITICAL

None.

### WARNING

- W1, class: ambiguity, follow-up. `ExportIssue { severity, message }`, `IssueSeverity { Error, Warning }`, `ExportReport { files, warnings }`, `ExportedFile { name, bytes }` are implementer shapes: 02 §4.5 and design P4 name them but do not define them. Nothing in spec, design or ADR 0012 contradicts them (FD-58 errors block, warnings continue; the message names the resource). For the owner/specifier to confirm. Ref: crates/gafira-ports/src/export.rs.
- W2, class: implementation, follow-up. `AssistError` implements `std::error::Error` with the default `source()`, so `Cancelled` is not exposed, unlike `ImportError`, `ExportError`, `RenderError`, `MediaError`. ADR 0012 rule 5 asks only for `std::error::Error`, so it is not against spec. Ref: crates/gafira-ports/src/assist.rs:83.

### SUGGESTION

- S1. Design P8 `import` rule 2 ("truncate before each try") lags 02 §4.5 rule 5 as implemented (truncate on every `Err` too). Known doc follow-up for the specifier; not a code finding.

## Size

`git diff --shortstat ae9b544..HEAD`: 7 files, 958 insertions, 5 deletions; 885 without `d5b-progress.md` (forecast 365 at x1.4). Commit 25be3a8 alone is 800 lines. Recorded only, as instructed.

## Verdict

PASS. Critical 0, warning 2 (ambiguity 1, implementation 1, both follow-up), suggestion 1. D5b is the last delivery of gafira-ports; T4 (gafira-ports, then gafira-doc) is owed before archive under `mutation: archive`.

next: archive
