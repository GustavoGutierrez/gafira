# D5a progress (tdd-apply)

Flow: full | level: L3 | mutation: archive | verify: V2
Branch `feat/gafira-ports-d5a`, cut from `develop` 7a613e6. Design: `specs/changes/gafira-ports/design.md` (D5a row, P4, P6, P8). Writes only `format.rs`, `tests/format.rs` and this file.

## Done

- `format.rs`: `FormatKind`, `ByteSignature::matches`, `FormatDescriptor::sniff_signatures`, `ImportOptions`, `ImportCx`, `ImportedAsset`, `Imported`, `ImportError` with `ToMessage`, `Importer`, `FormatRegistry` (`new`, `register_importer`, `importers`, `resolve_import`, `import`).
- Shape for D5b: `FormatRegistry` is a struct deriving `Default` with one private field (`importers`); D5b adds an `exporters` field and `register_exporter` without touching this code. `register_importer` checks duplicates in its own list only (the same id as importer and exporter stays valid).
- `Importer` is not `Describe` (its `descriptor()` returns `FormatDescriptor`), so the list is a plain `Vec<Arc<dyn Importer>>` checked by hand; the error is `RegistryError::DuplicateId`.
- `resolve_import` sniffs each importer once; candidates are `(confidence, listed for the extension, importer)`.

## RED evidence (skeleton with wrong bodies, then code)

| Criterion | Test | RED assertion |
|---|---|---|
| FD-31 `matches` | `p8_signature_matches_table` | "last byte differs" left true right false |
| FD-31 `sniff_signatures` | `p8_sniff_signatures_high_on_any_match_else_none` | "first signature" left None right High |
| ARQ-102 | `arq_102_import_error_messages` | key left `error.import.unsupported` right `error.import.corrupt` |
| FD-31 rule 1 | `fd_31_extension_is_compared_in_lower_case`, `fd_31_last_extension_is_used` | `a.png` left [png, other] right [png]; left [tar, gz] right [gz] |
| FD-31 rules 2, 3 | `fd_31_none_sniff_is_dropped`, `fd_31_fallback_tries_every_importer_when_the_extension_has_none`, `fd_31_extension_filter_takes_priority_over_confidence` | left [a, b] right [b]; left [png, jpg] right [png]; left [a, b] right [a] |
| FD-31 rule 4 | `fd_31_sorted_by_confidence_descending`, `fd_31_tie_order_holds_on_a_long_list` | left [low, high, mid] right [high, mid, low] |
| P8 `register_importer` | `p8_duplicate_importer_id_is_rejected_and_changes_nothing` | unwrap_err on Ok |
| FD-31 import 1 to 5 | `fd_31_first_ok_is_the_result`, `..._failure_moves_to_the_next_importer`, `..._each_try_reads_from_byte_zero`, `..._failed_seek_is_an_io_error`, `..._cancelled_stops_the_tries`, `..._when_all_fail_the_first_error_is_returned`, `..._warnings_of_a_failed_try_are_discarded` | the skeleton `import` returned `Unsupported`: `unwrap` of the Err in the helpers, `matches!(.., Io/Cancelled)` false, log of tries empty |

## Hand mutants (D5a list, plus extras), each applied, test failed, reverted

| # | Mutant | Killed by |
|---|---|---|
| 1 | Short `head` panics (`&head[a..b]`) | `p8_signature_matches_table`, `p8_sniff_signatures_..` |
| 2 | Offset overflow panics (`offset + len`) | `p8_signature_matches_table` |
| 3 | Empty `bytes` matches | `p8_signature_matches_table` |
| 3b/3c | `sniff_signatures` uses `all`; returns `Medium` | `p8_sniff_signatures_high_on_any_match_else_none` |
| 4 | Extension compared case-sensitively | `fd_31_extension_is_compared_in_lower_case` |
| 4b | First extension, not the last | `fd_31_last_extension_is_used`, `fd_31_missing_extension_matches_no_importer` |
| 5 | A `None` sniff stays in the list | `fd_31_all_none_gives_an_empty_list`, `fd_31_none_sniff_is_dropped`, `fd_31_fallback_..`, `fd_31_import_without_candidates_is_unsupported` |
| 6 | No fallback to all importers | `fd_31_fallback_tries_every_importer_when_the_extension_has_none`, `fd_31_missing_extension_..`, `fd_31_non_utf8_extension_..` |
| 6b | Extension filter never applied | `fd_31_extension_filter_takes_priority_over_confidence`, `fd_31_extension_is_compared_in_lower_case` |
| 7 | Sort ascending | `fd_31_sorted_by_confidence_descending` and others |
| 8 | Unstable sort (`sort_unstable_by_key`) | `fd_31_tie_order_holds_on_a_long_list` only (64 importers: small lists sort by insertion and hide it) |
| 8b | Sort ascending then `reverse()` (ties reversed) | `fd_31_tie_keeps_registration_order`, `fd_31_tie_order_holds_on_a_long_list` |
| 9 | No seek between tries | `fd_31_each_try_reads_from_byte_zero`, `fd_31_failed_seek_is_an_io_error` |
| 9b | A failed seek is ignored | `fd_31_failed_seek_is_an_io_error` |
| 10 | `Cancelled` moves to the next importer | `fd_31_cancelled_stops_the_tries` |
| 11 | The last error is returned | `fd_31_when_all_fail_the_first_error_is_returned` |
| 12 | Warnings of a failed try kept | `fd_31_warnings_of_a_failed_try_are_discarded` |
| 12b | Warnings cleared to 0 (the caller's lost) | `fd_31_warnings_of_a_failed_try_are_discarded` |
| 13 | Duplicate importer id accepted | `p8_duplicate_importer_id_is_rejected_and_changes_nothing` |
| 14 | Empty candidate list gives another error | `fd_31_import_without_candidates_is_unsupported` |

No survivor.

## Deviations and notes

- Shared `CARGO_TARGET_DIR`: same-named crates in sibling worktrees share an artifact hash, so one build linked another worktree's `gafira-ports` (first test run failed to find `FormatKind`). All commands use `RUSTFLAGS="--cfg wt_d5a"` to split the hash. The coordinator was told.
- `Importer` is outside `Describe`, so `Registry<T>` is not reused (it needs `Describe`).
- `importers()` is public, as the registry-order view D5b and the UI list mirror for exporters; not named by the design.
- Warnings are truncated before each try, as the design says; the last failed try's warnings are not discarded when every importer fails.
- The non-UTF-8 extension test is `#[cfg(unix)]`.
