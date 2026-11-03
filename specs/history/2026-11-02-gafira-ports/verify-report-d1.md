# Verify report: gafira-ports D1, round 1

- Change `gafira-ports`, delivery `D1`, round 1 (first verify of this code).
- Commit verified: `12a2825` (branch `feat/gafira-ports-d1`; implementation `22003e7`, `523d955`, `0f63145`; progress `1dc075a`; spec gaps merged from `d7a3d2e`). Base `develop` `f4936b9`.
- Flow: full | level L3 | mutation: archive | tier T4 at close | depth V2 (from the Flow line).
- Acceptance source: Slice card (`758f112`), design `specs/changes/gafira-ports/design.md` (D1 row, P1, P4-P8, §Acceptance, §Shared files, D1 negative controls), ADR 0013 and 0014 (accepted), `specs/02-arquitectura-software.md` as amended by `d7a3d2e`.
- All runs on scratch exports outside the repo (`git archive` + `touch`, one `CARGO_TARGET_DIR` each, `CARGO_INCREMENTAL=0`); disk 29 GB free before the build; target dirs freed with `cargo clean` afterwards.

## Verdict

**PASS for D1** (0 CRITICAL). Deliveries D2, D3, D4, D5a, D5b and D6 remain, so the change is not done.

next: tdd-apply

## Gates (exit codes, at `12a2825`)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (2173 passed, 0 failed) |
| `cargo deny check` | 0 (advisories, bans, licenses, sources ok) |
| CI "Domain crates forbid unsafe code" | 0 |
| CI V5 layer table | 0 |
| CI V12 clippy copies | 0 (`crates/gafira-ports/clippy.toml` byte-identical to geom's) |
| CI ARQ-107 | 0 (`gafira-ports: layer 3, exempt from the Locale search`) |
| CI V13, V14, ARQ-07 (unaffected, run anyway) | 0, 0, 0 |

Per-commit gates on clean exports are V3 only and were not run.

## Negative controls (design §Acceptance, D1)

| Control | Expected | Result |
|---|---|---|
| Plain row, skip removed, `Locale` defined in `gafira-ports` | step fails | exit 1, `gafira-ports mentions Locale (ARQ-107)` |
| Plain row, skip removed, D1 as is (no `Locale` in ports yet) | (informative) | exit 0, see SUGGESTION S1 |
| Skip kept, `Locale` defined in ports | step passes | exit 0 |
| `icu_plurals` (path crate of that name) added to `gafira-ports` | step fails | exit 1, `gafira-ports depends on icu_plurals` |
| `Locale` added to `gafira-raster` | step fails | exit 1, `gafira-raster mentions Locale` |
| Extra (V5): `gafira-text` added to `gafira-ports` | V5 fails | exit 1, `must not depend on gafira-text` (and `gafira-pathops`) |

## Independent probes (step 1c, before reading the implementer's tests)

One probe file (`zz_verify_probe.rs`, scratch only), 7 tests, all pass on HEAD code:
registry order with unsorted ids and a duplicate that leaves items, order and `get` intact, then a new id lands last; `Registry::new`/`default` on a trait with no `Describe`; `Registry<dyn Describe>`; `RegistryError` and both `StoreError` keys, args and `is_valid_key`; `Confidence` order, three `LegalClass`, `Bytes`, `UnixTime`, `ContentHash` copy/order; ARQ-50 create/write/sync/replace trace through `dyn FileSystem`; ARQ-93 `dyn Clock` injected, `dyn Notifier`; `dyn ContentStore` round trip and `NotFound`; blanket impls on `Box<dyn ReadSeek + Send>` and `?Sized`; `Send`/`Sync` of the port objects.
The same file names every lower-crate type P1/P4 assigns to D2-D6 (`ParamValues`, `ParamSchema`, `CancelToken`, `Cancelled`, `Progress`, `msg::Unit`, `Document`, `Page`, `Node`, `NodeId`, `PageId`, `BlendMode`, `ImageBuf`, `PixelFormat`, `AlphaMask`, `dyn Effect`, `dyn Transition`, `BufferError`, `EffectError`, `Flicks`, `FrameRate`, `SampleRate`, `IRect`, `Affine`, `Color`, `kurbo::BezPath`) and a `serde` derive with `rename_all`, from inside `crates/gafira-ports/tests`: it compiles with the D1 `Cargo.toml` and lock unchanged.

Comparison with the implementer's tests: every probe has a matching implementer assertion of equal or greater strength (the implementer's duplicate test also checks that `get` stays aligned after a later register; the store test pins the exact hex text). No criterion rests on a weak test.

## Coverage (step 3, 3c)

| Criterion | Proof | Status |
|---|---|---|
| ARQ-01 (row of P1) | CI V5 step, exit 0; negative V5 control fails | covered |
| ARQ-31 (by value, no `static`) | `arq_31_new_registry_is_empty_without_describe`, `arq_31_registries_do_not_share_state`; `git grep` of `static`, `lazy_static`, `inventory::`, `OnceLock`, `thread_local` in the crate: only `'static` lifetimes | covered |
| ARQ-50 (write, sync, replace trace) | `arq_50_file_system_expresses_the_atomic_save_trace`, `arq_50_file_system_other_operations_run_through_dyn` | covered |
| ARQ-93 (`Clock` injectable) | `arq_93_clock_double_is_injectable` | covered |
| ARQ-107 (ports exempt only from the `Locale` grep) | CI step exit 0; three design negative controls behave | covered |
| V-5 (type) | `v_5_legal_class_has_three_classes` (exhaustive match) | covered |
| ARQ-102 for `RegistryError`, `StoreError` | `arq_102_registry_error_message`, `arq_102_store_error_messages` (one fixture per variant, area, arg names, no detail) | covered |
| ARQ-20 (object-safe, D1 traits) | `dyn ContentStore`, `dyn FileSystem`, `dyn FileWrite`, `dyn Clock`, `dyn Notifier`, `dyn ReadSeek/WriteSeek` in tests | covered |
| P8 `Registry::register` | `p8_duplicate_id_is_rejected_and_changes_nothing` | covered |
| P8 `get`, `iter`, `len`, `is_empty`, `new` without `Describe` | `p8_iter_is_in_registration_order`, `p8_get_by_id_and_unknown_id`, `arq_31_new_registry_is_empty_without_describe` | covered |
| P5 `ContentHash` (derives, no serde, no methods) | source review; `arq_01_content_hash_is_a_copy_ordered_value` | covered |
| P4 blanket impls (`?Sized`) | `p4_blanket_impls_cover_std_streams` | covered |
| §Shared files: 13 `pub mod`, stubs, all deps, row, skip, clippy copy | source review, probe compile, CI steps | covered |

15 of 15 covered. Product IDs of the Slice card for D1: ARQ-01, ARQ-31, ARQ-50, ARQ-93, ARQ-107, V-5, all covered; `git grep` finds each in a test name or comment (ARQ-107 and the V5 row are CI commands, as the design's Acceptance table says). Left out on purpose: the D2-D6 IDs of the card (ARQ-06, ARQ-22, ARQ-26, ARQ-77, ARQ-94, ARQ-100, FD-31 rules, FD-57, FD-58, FD-64), which belong to later deliveries.

Compile-only RED for plain derives and bare traits (`ContentHash`, `Confidence`, `LegalClass`, `Bytes`, `UnixTime`, `system` traits): judged acceptable. There is no behaviour to fail an assertion on; the design classes plain data and dyn tests as not critical (P8), and every behaviour item (`Registry`, `to_message`) went red on assertions. Not refiled.

## Artifact integrity (step 3, 3c)

- `758f112 docs(gafira-ports): slice card` touches only `specs/changes/gafira-ports/apply-progress.md` (7 insertions).
- No implementer commit changed the card: `1dc075a` only appends below it (`@@ -5,3 +5,48 @@`, additions only); `22003e7`, `523d955`, `0f63145` touch no file under `specs/`.
- No implementer commit changed `design.md`, the ADRs or `specs/02`; `d7a3d2e` is the specifier's gap commit, merged in `12a2825`.

## Shared files and D2-D6 parallelism

Checked that D2, D3, D4, D5a and D6 need no shared file:
- Root `Cargo.toml`: one path entry for `gafira-ports`, added in `523d955`. No later delivery adds a crate.
- `Cargo.lock`: the `gafira-ports` entry already lists `gafira-color`, `gafira-doc`, `gafira-geom`, `gafira-raster`, `gafira-time`, `kurbo`, `serde`, `proptest`, `serde_json`; no new third-party crate is needed (P1).
- `crates/gafira-ports/Cargo.toml`: every P1 dependency and dev-dependency declared; the probe compiled all D2-D6 lower-crate types and a serde derive against it.
- `src/lib.rs`: all 13 `pub mod`; the 9 D2-D6 modules exist as one `//!` line each; no re-exports to extend.
- `.github/workflows/ci.yml`: the row and the `Locale` skip are in; a `Locale` type in `i18n.rs` (D2) passes ARQ-107 (control above). V12 and V5 bodies unchanged.
- `clippy.toml` present and identical; `deny.toml` and `.cargo/mutants.toml` unchanged.
- New integration test files `tests/<module>.rs` need no manifest entry (autotests).
- `common` already holds `LegalClass`, `Confidence`, `Bytes`, `UnixTime`; `store` holds `ContentStore`/`StoreError` with `to_message`, so `ImportError`, `ExportError`, `AssetError` can wrap it.
Residual, as the design says: D5b edits `format.rs` from D5a and stays serial; change folders and `.odd/state.json` follow §Shared files (one folder per parallel delivery).

## Mutation

`mutation: archive`: no `mutants` run for this delivery, no report; not a finding. T4 of `gafira-ports` and `gafira-doc` is owed at change close. No `.cargo/mutants.toml` exclusion added.

## Hand mutants (step 4b, scratch export)

| # | Kind | Mutant | Result |
|---|---|---|---|
| H1 | spot-check of recorded #3 | `iter` walks `by_id` (id order) | killed by `p8_iter_is_in_registration_order` |
| H2 | independent, rule "duplicate leaves the registry unchanged" (rule order: check before write) | `register` inserts into `by_id` first and errs if an entry existed, so the map now points past `items` | killed by `p8_duplicate_id_is_rejected_and_changes_nothing` (index panic) |
| H3 | independent, ARQ-102 "variants told apart" | `StoreError::Io` gives `error.store.not_found` | killed by `arq_102_store_error_messages` |
| H4 | independent, `get` by id | `get` reads `items[at - 1]` (saturating) | killed by `p8_get_by_id_and_unknown_id`, `p8_duplicate_id_is_rejected_and_changes_nothing` |
| H5 | independent, non-critical text form | `hash` argument in upper-case hex | killed by `arq_102_store_error_messages` |

5 run, 5 killed. All reverted (byte-compared).

## Findings

### CRITICAL

None.

### WARNING

- **W1** class: implementation. follow-up: no. Size: `git diff --shortstat 758f112 1dc075a` = 912 insertions (867 in code, manifests and CI; about 449 in tests; 45 in `apply-progress.md`) against a forecast of 300 (420 at x1.4) and the ~700 ceiling of `AGENTS.md`. Recorded only: the coordinator reports the owner said size does not matter to him; it does not block, but the ceiling rule asks for his explicit OK, so the coordinator may want it on record for D1.

### SUGGESTION

- **S1** The design's control "a plain row without the ARQ-107 change fails the step" is vacuous at D1: `gafira-ports` has no `Locale` yet, so the plain row passes (exit 0). It fails only once a `Locale` mention exists (exit 1 here, and in the implementer's run). Re-run the plain-row control in D2's verify, when `Locale` lands, or word it with the mention.
- **S2** `arq_01_content_hash_is_a_copy_ordered_value` (`crates/gafira-doc/tests/content_hash.rs`) cites ARQ-01, the layer rule, but proves P5 (plain ordered value). A `p5_` name would trace better; ARQ-01 itself is proved by the V5 step.
- **S3** The design's Flow line says `level: L2`; the Slice card says `L3`. The coordinator owns both; align them when the design is next touched.

## Counts

critical 0, warning 1 (implementation 1), suggestion 3.

next: tdd-apply
