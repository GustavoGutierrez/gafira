## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator | reason: new crate with a new V5 row and port types the spec left without a home (architect, ADRs 0013 and 0014 accepted by the owner)
Product IDs: ARQ-01, ARQ-31, ARQ-50, ARQ-93, ARQ-107, V-5 (type) for D1; D2 to D6 per the delivery table (source: specs/02-arquitectura-software.md §2.1, §4.5-4.16; design: specs/changes/gafira-ports/design.md)
Scope: D1 of the design: the crate, all shared files, `ContentHash` in gafira-doc, `registry`, `common`, `system`, `store`, module stubs.      Non-goals: D2 to D6 modules; spec gaps S1 to S16 (specifier, in parallel on feat/gafira-ports); adapters.
Deliveries: D1 serial first (~420 at x1.4); then D2, D3, D4, D5a, D6 parallel, each in its own worktree; D5b after D3, D4, D5a. T4 at close: gafira-ports, then gafira-doc.
Plan: per design §Acceptance and §P8 for D1, test first; negative controls of §Acceptance run by verify.
Owner decisions touched: D-12 through O1 (a), D-02 and D-08 unchanged (checked D-48 to D-50).

## D1 progress (tdd-apply)

Branch `feat/gafira-ports-d1`. Commits: `feat(doc): add ContentHash`; `feat(ports): add the gafira-ports crate with its V5 row`; `feat(ports): add registry, common, system and store`.

### Done

- `ContentHash` in `gafira-doc` (P5), re-exported at the crate root. No serde, no methods.
- Crate `gafira-ports`: `Cargo.toml` with every dependency of P1 (`gafira-geom/color/doc/time/raster`, `serde`, `kurbo`; dev `proptest`, `serde_json`), byte-identical `clippy.toml`, `lib.rs` with all 13 `pub mod` and `#![forbid(unsafe_code)]`, 9 modules as `//!` lines for D2 to D6.
- Shared files touched, once: root `Cargo.toml` (one path entry), `Cargo.lock`, `ci.yml` (V5 row; ARQ-107 skips only the `Locale` grep for `gafira-ports`).
- `registry` (`Registry<T>` over `Vec<Arc<T>>` + `BTreeMap`, `RegistryError`), `common` (`Confidence`, `LegalClass`, `Bytes`, `UnixTime`), `system` (`ReadSeek`, `WriteSeek`, `FileWrite`, `FileSystem`, `Clock`, `Notifier`, blanket impls), `store` (`ContentStore`, `StoreError`).
- The testkit `ports` feature is not added: P7 says it lands with the first fake.
- D2 to D6 touch no shared file: each module they own exists as a stub, `lib.rs` declares it, the dependencies are all declared, `gafira-time` and `kurbo` are present, and `LegalClass`, `Confidence`, `Bytes`, `UnixTime`, `ReadSeek`, `WriteSeek`, `ContentStore`, `StoreError`, `Registry` are ready. `ContentStore`/`StoreError` ship with `to_message`, so `ExportError`, `ImportError` and `AssetError` can wrap them.

### RED evidence

| ID | Test | RED (failing assertion on a skeleton with wrong bodies) |
|---|---|---|
| ARQ-31 | `arq_31_registries_do_not_share_state`, `p8_*` in `tests/registry.rs` | `len` left 0 right 1; `get` left None right Some("b"); `iter` left [] right ["zeta","alpha","mid"]; `register` returned Ok for a duplicate (unwrap_err panic) |
| ARQ-102 | `arq_102_registry_error_message` | key left `error.registry` right `error.registry.duplicate_id` |
| ARQ-102 | `arq_102_store_error_messages` | skeleton key `error.store` against `error.store.not_found` |
| ARQ-50, ARQ-93 | `tests/system.rs` | Compile-level RED only (see deviations) |
| ARQ-01 (ContentHash), V-5 (type), FD-31 `Confidence` order | `tests/content_hash.rs`, `tests/common.rs` | Compile-level RED only (see deviations) |
| ARQ-01 (row), ARQ-107 | CI steps, scratch copy | See controls |

### Hand mutants (P8 list for D1)

| # | Mutant | Killed by |
|---|---|---|
| 1 | A duplicate id replaces the old item | `p8_duplicate_id_is_rejected_and_changes_nothing` |
| 2 | A duplicate is rejected but the item stays in `items` | `p8_duplicate_id_is_rejected_and_changes_nothing` |
| 3 | `iter` goes in id order | `p8_iter_is_in_registration_order` |
| 4 | `get` of an unknown id returns the first item | `p8_get_by_id_and_unknown_id` |
| 5 | `DuplicateId` key loses its id argument | `arq_102_registry_error_message` |
| 6 (extra, `system` blanket impls) | `?Sized` removed from `ReadSeek` or `WriteSeek` blanket impl | `p4_blanket_impls_cover_std_streams` (compile error E0277 on `dyn Wider`). The first version of the test let this survive; the test was strengthened |

### Local CI steps

V5 layer table, V12 clippy copies, ARQ-107, "forbid unsafe": exit 0. ARQ-31: no `static` item in the crate. Negative controls (scratch copy): plain row without the skip and a `Locale` mention in ports exits 1; with the skip exits 0; `Locale` in `gafira-raster` exits 1; a path crate named `icu_plurals` added to ports exits 1 (`gafira-ports depends on icu_plurals`).

### Deviations

- Items that are plain derives or bare traits (`ContentHash`, `Confidence`, `LegalClass`, `Bytes`, `UnixTime`, the `system` traits) were red as compile errors only: there is no behaviour to fail an assertion on without writing a deliberately wrong type. Behaviour items (`Registry`, `to_message`) were red on assertions.
- `StoreError::NotFound` gives its hash as lower-case hex text (`hash` argument), since `ContentHash` has no `Display`; the design fixes the argument name only.
- `Registry::len`, `is_empty` and `iter` sit in the impl without the `Describe` bound (design: `new` needs none).
