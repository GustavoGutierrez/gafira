## D6 progress (tdd-apply)

Branch `feat/gafira-ports-d6`, cut from develop 7a613e6. Commits: `feat(ports): add the project repository port`; `feat(ports): add the text assistant port and its empty registry`; `feat(ports): add the asset provider port`; this file with the last one. Touched only `src/{assets,repository,assist}.rs`, `tests/{assets,repository,assist}.rs` and this file.

### Done

- `repository`: `ProjectRepository` (`load`/`save` over `Document`, S15), `LoadedProject { document, read_only }`, `RepositoryError` with `to_message`.
- `assist`: `TextAssistant`, `AssistReq`, `RewriteRequest` (+ `Tone`, `Length`), `Suggestion`, `AssistError` (`Cancelled` delegates), `TextAssistantRegistry = Registry<dyn TextAssistant>` (default empty, ARQ-26).
- `assets`: `AssetProvider`, `AssetKind`, `AssetQuery`, `AssetItem`, `AssetPage`, `ProviderAssetId`, `Materialized`, `AssetError` (`Store` delegates to `StoreError`), `AssetRegistry = Registry<dyn AssetProvider>`.

### RED evidence

| ID | Test | RED (failing assertion on a skeleton with wrong keys) |
|---|---|---|
| ARQ-102 | `arq_102_repository_error_messages` | key left `error.repository` right `error.repository.corrupt` |
| ARQ-102 | `arq_102_assist_error_messages` | left `error.assist` right `error.assist.model_missing` |
| ARQ-102 | `arq_102_asset_error_messages` | key `error.asset` against `error.asset.not_found` |
| ARQ-26, FD-64, ARQ-20, S15 | `arq_26_default_registry_is_empty_...`, `fd_64_rewrite_reports_cancellation`, `arq_20_provider_registers_...`, `p4_repository_is_object_safe_...` | Compile-level only (see deviations) |

### Hand mutants (P8, D6)

| # | Mutant | Killed by |
|---|---|---|
| R1 | `Corrupt` puts `detail` in an argument | `arq_102_repository_error_messages` |
| R2 | `TooNew` swaps `found` and `supported` | `arq_102_repository_error_messages` |
| R3 | `Io` key leaves the `error.repository` area | `arq_102_repository_error_messages` |
| A1 | `AssetError::Io` carries the io text as an argument | `arq_102_asset_error_messages` |
| A2 | `NotFound` key leaves `error.asset` | `arq_102_asset_error_messages` |
| A3 | `Store` no longer delegates to `StoreError` | `arq_102_asset_error_messages` |
| S1 | `Cancelled` no longer delegates | `arq_102_assist_error_messages` |
| S2 | `ModelMissing` key leaves `error.assist` | `arq_102_assist_error_messages` |
| S3 | The default text assistant registry is not empty (newtype pre-registering one) | `arq_26_default_registry_is_empty_and_a_double_registers` |

### Deviations

- Traits, aliases and data types are red at compile level only; behaviour (`to_message`) is red on assertions. `TextAssistantRegistry::default()` is a type alias, so no wrong body exists to fail first; S3 proves the test.
- Types that 02 leaves open were given minimal shapes: `AssetQuery` (text, kinds, tags, color, format, offset, limit), `AssetItem`, `AssetPage { items, total }`, `Materialized { hash, mime }`, `AssistReq { ram_bytes, needs_gpu, model_bytes }`, `Tone`, `Length`, `Suggestion { text }`. `RewriteRequest.language` is `Option<String>` (a tag) because `Locale` lives in `i18n`, owned by another parallel delivery; D5b or a later change may retype it.
- `AssetError::NotFound { id }` carries a `ProviderAssetId`; its argument is `Text`. `TooNew` fields are `u32`, argument `Count`.
- No `From` impls for the wrapped errors: nothing needs them yet.
- Gates run with `RUSTFLAGS="--cfg wt_d6"` after the coordinator's warning; the first repository gate run was repeated under it.
