# Verify report: gafira-ports, parallel wave D2, D3, D4, D5a, D6

Flow: full | level: L3 | mutation: archive | tier: T4 at close | verify: V2.
Combined tree `feat/gafira-ports-wave` 87fd306 (develop 06dc4ea + clean merges of the five branches). Each delivery was judged on its own branch (`git log develop..feat/gafira-ports-dN`). The build ran on a clean `git archive` export under this folder with its own target dir. That dir was removed afterwards with `cargo clean` (5.9 GiB). Disk: 59 GB free at the start, 51 GB at the end.

| Delivery | Round | Commit | Verdict | C / W / S |
|---|---|---|---|---|
| D2 | 1 | 18d23ea | PASS | 0 / 1 / 1 |
| D3 | 2 (closure) | d331c95 | PASS | 0 / 0 / 1 |
| D4 | 2 (closure; first full gates) | a794da8 | PASS | 0 / 1 / 2 |
| D5a | 1 | 3b999ec | PASS | 0 / 2 / 2 |
| D6 | 1 | 5899a8d | PASS | 0 / 2 / 1 |

PASS here means the delivery is done. The change is not done: D5b and T4 (gafira-ports, gafira-doc) remain.

## Gates (combined tree, run once, exit codes taken separately)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (173 test binaries ok, 0 failed; ports doctests incl. `LicenseKey` compile_fail at line 52 ran and passed) |
| CI V5 (layer table) | 0 |
| CI V12 (clippy.toml copies) | 0 |
| CI ARQ-107 | 0 (`gafira-ports: layer 3, exempt from the Locale search`) |
| CI forbid unsafe | 0 |
| CI V13, V14 (git grep; run read-only in the worktree, clean before and after) | 0, 0 |
| CI ARQ-07 | 0 |
| cargo-deny | not run: no Cargo file changed in this wave |

ARQ-107 negative controls, each on its own scratch copy:
1. The step without the `gafira-ports` skip exits 1, because `gafira-ports` mentions `Locale` (`tests/i18n.rs:9`, which now exists in ports).
2. `// Locale` added to `gafira-raster/src/lib.rs` exits 1 (`gafira-raster mentions Locale`).
3. A path crate `icu_plurals` added to `gafira-ports` `[dependencies]` exits 1 (`gafira-ports depends on icu_plurals`).

The unmodified step exits 0.

ARQ-31: no `static` item, `static mut`, `lazy_static`, `OnceLock` or `inventory::` in `crates/gafira-ports/src`. There is no `#[allow]`, `HashMap`, `#[non_exhaustive]` or `pub use` either.

## Artifact integrity (step 3c)

- The Slice card commit 758f112 `docs(gafira-ports): slice card` touches only `apply-progress.md`. The later 1dc075a (D1 progress) only appends below the card.
- No commit on any of the five branches touches `apply-progress.md`, `design.md`, `specs/02-arquitectura-software.md` or `.odd/state.json`.
- Each delivery wrote only its own files:

| Delivery | Files |
|---|---|
| D2 | `src/{i18n,license}.rs`, `tests/{i18n,license}.rs`, `d2-progress.md` |
| D3 | `src/render.rs`, `tests/render.rs`, `d3-progress.md` |
| D4 | `src/media.rs`, `tests/media.rs`, `d4-progress.md` |
| D5a | `src/format.rs`, `tests/format.rs`, `d5a-progress.md` |
| D6 | `src/{assets,assist,repository}.rs`, `tests/{assets,assist,repository}.rs`, `d6-progress.md` |

- Sizes from `git diff --shortstat develop...branch`, total with code-only in brackets. The coordinator asked for these to be recorded, not failed: D2 712 (656), D3 725 (681), D4 880 (827), D5a 1010 (951), D6 704 (664). AGENTS.md wants the owner's OK above ~700.

## Independent probes (1c; D2, D5a, D6 round 1; scratch file `probes/zz_verify_probe.rs`, 9 tests, all pass)

Derived from 02 §4.5, §4.11, §4.14 to §4.16 and §14.4 and from design P6/P8, before reading the implementers' tests.

| Delivery | What the probes check |
|---|---|
| D2 | `LanguagePreference` `system/es/en` parse, `as_str` and serde round-trip; `"ES"`, `"fr"`, `""`, `"En"`, `" es"` read as System; `1`, `null`, `{}`, `[]`, `true` are errors. `Locale` serde `es`/`en`. `Region`: `mx`/`MX` give `MX`, `419` is valid; `m`, `mex`, `41`, `4190`, `m1`, `ñx`, `""`, `1a`, `4 9` give None. `can_export` over all states and every `ReasonCode`. `LicenseKey` `Debug`/`{:#?}`/inside `Option` leak no key character. `LicenseError` keys valid and distinct. `dyn` for the 4 traits |
| D5a | `matches` with offset `usize::MAX`, empty bytes, short head and last byte. `sniff_signatures`. Every acceptance case of 02 §4.5: `.PNG`, PNG named `.jpg`, None never tried, ties in registration order, second try from byte 0, `Cancelled` stops, first error of two, empty gives `Unsupported`. Also: hidden `.png`, double extension, a listed Low beating an unlisted High, a failed seek is `Io` with no try, duplicate id unchanged, `to_message` of all 5 variants with no library text in arguments |
| D6 | Default `TextAssistantRegistry` empty, a double registers. Cancel delegates to `error.job.cancelled`. `TooNew` arguments `found`/`supported` as Count. `Corrupt` has no argument. `AssetError` keys, `id` as Text, `Store` delegates. `dyn` `AssetProvider`/`ProjectRepository` |

The probes found one behaviour for the spec: with every importer failing, `cx.warnings` keeps the last failed try's warnings (3 left, the caller had 2). See D5a W2.

Comparison with the implementers' tests: every criterion the probes cover also has an implementer test with an asserting check, and none rests only on a weak test. The D2 tests are stricter than the probes in places (`٤١٩`, `mx `).

## Hand mutants (4b)

Each mutant was applied to the export and run with `cargo test -p gafira-ports --test <module>`, then the file was restored. The restored `src/` was diffed equal to the worktree. Every kill below is a test failure, not a compile error.

| Delivery | Mutant | Kind | Result |
|---|---|---|---|
| D2 | #2 parse case-insensitive | spot-check | killed: `p8_preference_parses_exact_values_and_defaults_to_system`, `p8_preference_serde_is_the_lowercase_string` |
| D2 | I1 `null`/non-string reads as System (`Option<String>`) | independent | killed: `p8_preference_serde_is_the_lowercase_string` |
| D2 | I2 `Region` accepts 2 alphanumerics | independent | killed: `p8_region_accepts_two_letters_or_three_digits` |
| D2 | I3 `Region::as_str` ignores its length | independent | killed: same |
| D3 | #1 `At(t)` strict | spot-check (W1 owed) | killed: `p8_deadline_has_passed_table`, `p2_late_render_is_a_report_not_an_error` |
| D3 | I1 `Effect` does not delegate its message | independent | killed: `arq_102_render_error_messages`, `arq_102_wrapped_errors_delegate_and_convert` |
| D3 | I2 `AdapterMissing` shares the `DriverBlocked` key | independent | killed: `arq_102_render_error_messages`, `arq_22_fallback_failures_are_told_apart` |
| D4 | H1 (round 1 C1) `OsLicensed` ranks above `RoyaltyFree` inside hardware | closure | killed: `arq_77_hardware_decoders_tie_whatever_their_class`, `arq_77_order_is_hardware_then_os_then_royalty_free` |
| D4 | #7 names the last bad channel (`rfind`) | spot-check | killed: `p8_audio_buf_names_the_first_bad_channel` |
| D4 | I1 `expected` taken from the longest channel | independent | killed: same |
| D4 | I2 `expected`/`actual` swapped in `to_message` | independent | killed: `arq_102_media_error_messages` |
| D5a | #12 warnings of a failed try kept | spot-check | killed: `fd_31_warnings_of_a_failed_try_are_discarded` |
| D5a | I1 no seek before the first try | independent | killed: `fd_31_each_try_reads_from_byte_zero`, `fd_31_failed_seek_is_an_io_error` |
| D5a | I2 `Cancelled` after a failure returns the earlier error | independent | killed: `fd_31_cancelled_stops_the_tries` |
| D5a | I3 extension filter applied before the None drop (rule order) | independent | killed: `fd_31_fallback_tries_every_importer_when_the_extension_has_none` |
| D6 | R2 `TooNew` swaps `found`/`supported` | spot-check | killed: `arq_102_repository_error_messages` |
| D6 | I1 `NotFound` argument renamed `asset` | independent | killed: `arq_102_asset_error_messages` |
| D6 | I2 `Io` shares the `Corrupt` key | independent | killed: `arq_102_repository_error_messages` |

18 run, 18 killed, no survivor. Mutation: `archive`, so there is no `mutants` report in this wave. That is not a finding. T4 at close covers gafira-ports and gafira-doc.

## D2 (round 1): i18n, license

Coverage:

| Criterion | Proof | Status |
|---|---|---|
| ARQ-94 (`LicenseKey` `Debug`) | `arq_94_license_key_debug_leaks_nothing`; no `Display` via the compile_fail doctest | covered |
| ARQ-100 (types) | `arq_100_resolver_double_maps_tags_through_dyn_ports` | covered |
| ARQ-102 | `arq_102_license_error_messages` | covered |
| ARQ-20 | `arq_20_license_verifier_is_object_safe`, `arq_20_translator_is_object_safe` | covered |
| P8 `LanguagePreference` parse, `as_str`, serde (S11) | 3 `p8_preference_*` tests | covered |
| P8 `Locale` serde | `p8_locale_serde_is_es_and_en` | covered |
| P8 `Region` | 2 `p8_region_*` tests | covered |
| P8 `can_export` | `p8_can_export_only_for_active_and_offline_grace` | covered |
| P8 `expose` | `p8_license_key_expose_gives_the_key` | covered |

Derives match 02 §4.16. `Region` is an own `Copy` type with no ICU. There is no `Trial` and `can_export` is the only predicate (O1 (a), D-12, D-55).

### WARNING
- **D2-W1** class: `ambiguity`, follow-up. ref: `crates/gafira-ports/src/license.rs` (`ReasonCode`, `LicenseError::Rejected`), `src/i18n.rs` (`NumberStyle`, `DateStyle`).
  - These are open shapes that fix a contract adapters will rely on: `gafira-license` maps server answers into `ReasonCode` (`Revoked`, `UnknownKey`, `DeviceLimit`, `TokenTampered`), and `gafira-i18n` implements `NumberStyle` (`Decimal`, `Percent`, `Tabular`) and `DateStyle` (`Short`, `Long`).
  - `LicenseError::Rejected(ReasonCode)` gives the key no argument, so the UI cannot say why a key was rejected. ADR 0012 rule 2 names arguments after fields, and a tuple variant has none.
  - None of this conflicts with 02 or the design. Owner or specifier to confirm the variant lists, and whether `Rejected` becomes `Rejected { reason }` with a `reason` argument.

### SUGGESTION
- **D2-S1** Size 712 (656 code), above ~700 counting the progress file. Recorded.

## D3 (round 2, closure)

| Round 1 finding | Status | Evidence |
|---|---|---|
| C1 `EffectRegistry`/`TransitionRegistry` missing | **closed** | d331c95 adds `pub type EffectRegistry = Registry<dyn Effect>` and `TransitionRegistry = Registry<dyn Transition>` (design P4). `p4_effect_and_transition_registries_register_by_id` registers `ColorAdjust` and `Cut`, refuses repeats and builds `RenderOp::ApplyEffect` from the stored `Arc`. The test passes on the clean build |
| W1 hand mutants owed | **closed** | Spot-check #1 and independent I1, I2 all killed (table above) |

Round 1 S2 was addressed by doc notes on `TargetDesc` and `BackendCaps`. S1 and S3 were suggestions, not tracked. The fix commit touches only D3's own files. No new finding.

### SUGGESTION
- **D3-S1** Size 725 (681 code) after the fix round. Recorded.

## D4 (round 2, closure; first full gates)

Round 1 could not build for lack of disk. All gates of this depth now ran on the clean combined build and pass.

| Round 1 finding | Status | Evidence |
|---|---|---|
| C1 hardware tie across legal classes | **closed** | a794da8 adds `arq_77_hardware_decoders_tie_whatever_their_class` (`[hw RF, hw OS]` in both orders, first registered wins). Mutant H1 now fails it and the order test. `best_decoder` uses `min_by_key`, which keeps the first on a tie (02 §4.8, S9) |
| W1 `From<Cancelled>` for `MediaError` | **closed** | `impl From<Cancelled> for MediaError` at `media.rs:67`; `arq_102_cancelled_converts_into_media_error` passes |

Public traits and `Packet`, `VideoFrame`, `CodecCaps` and `AudioBuf` (private fields, P8 rule) match 02 §4.8 and design P4.

### WARNING
- **D4-W2** class: `ambiguity`, follow-up (new in round 2). ref: `crates/gafira-ports/src/media.rs:102-200`.
  - The open plain data fixes contracts that `gafira-mux`, `gafira-codec-*` and `gafira-media` will build:
    - `CodecId` is a closed list of 11 without `#[non_exhaustive]`.
    - `CodecParams` enum, `StreamInfo { index, codec, params, duration }`, `EncoderConfig { codec, params, bitrate }`, `MuxConfig { streams }`, `MuxReport { size, packets }`, `SeekMode { Keyframe, Exact }`.
    - `ChannelMismatch` fields are `usize`, sent as Count.
  - None of it conflicts with 02 or the design. Owner to confirm, or leave to the `gafira-media` design.

### SUGGESTION
- **D4-S1** `media_traits_are_dyn_compatible` does not name `ARQ-20` in its name or comment. The design asks tests to name their ID. Every other delivery names it.
- **D4-S2** Size 880 (827 code), above ~700. Recorded.

## D5a (round 1): format, import side

Coverage, against the 02 §4.5 acceptance list and design P8:

| Criterion | Proof | Status |
|---|---|---|
| `.PNG` matches `png` | `fd_31_extension_is_compared_in_lower_case` | covered |
| PNG named `.jpg` | `fd_31_fallback_tries_every_importer_when_the_extension_has_none` | covered |
| None never tried | `fd_31_none_sniff_is_dropped`, `fd_31_all_none_gives_an_empty_list` | covered |
| Tie in registration order | `fd_31_tie_keeps_registration_order`, `fd_31_tie_order_holds_on_a_long_list` | covered |
| Second try from byte 0 | `fd_31_each_try_reads_from_byte_zero` | covered |
| `Cancelled` stops | `fd_31_cancelled_stops_the_tries` | covered |
| First of two errors | `fd_31_when_all_fail_the_first_error_is_returned` | covered |
| Empty list gives `Unsupported` | `fd_31_import_without_candidates_is_unsupported` | covered |
| Last extension, missing or non-UTF-8 extension | `fd_31_last_extension_is_used`, `fd_31_missing_extension_*`, `fd_31_non_utf8_*` | covered |
| Seek failure gives `Io` | `fd_31_failed_seek_is_an_io_error` | covered |
| Descending sort | `fd_31_sorted_by_confidence_descending` | covered |
| `ByteSignature::matches`, `sniff_signatures` | `p8_signature_matches_table`, `p8_sniff_signatures_*` | covered |
| Duplicate importer id | `p8_duplicate_importer_id_is_rejected_and_changes_nothing` | covered |
| ARQ-102 | `arq_102_import_error_messages` | covered |
| ARQ-20 | `arq_20_importer_is_object_safe` | covered |

`Importer` without `Describe` follows 02 §4.5 exactly (`descriptor() -> &FormatDescriptor`), so it is not a finding. The duplicate check uses `RegistryError::DuplicateId` and only the importer list (P8).

### WARNING
- **D5a-W1** class: `implementation`. ref: `crates/gafira-ports/src/format.rs:112-121`; ADR 0012 rule 4, applied to ports by ADR 0014 decision 2.
  - `ImportError` has `Cancelled(Cancelled)` but no `From<Cancelled>`, so an importer cannot write `cancel.check()?`.
  - This is the same gap as D4 round-1 W1, which was fixed there.
  - Fix: add the impl test-first, as D4 did.
- **D5a-W2** class: `ambiguity`, follow-up. ref: `format.rs:241-259`; design P8 `import` step 2 against 02 §4.5 rule 5.
  - The design truncates `cx.warnings` before each try. 02 says the failed try's warnings are discarded.
  - When every importer fails, or one returns `Cancelled`, the last try's warnings stay in `cx` next to the `Err`. The probe shows 3 warnings left where the caller had 2.
  - The code follows the design to the letter and `d5a-progress.md` records it.
  - Specifier to choose: truncate also before returning `Err`, or state in 02 that the warnings of the final try are kept.

### SUGGESTION
- **D5a-S1** `FormatRegistry::importers()` is public but not named by the design. It is the natural view for the UI and D5b, and no adapter relies on it. Mention it in the design or 02 when D5b adds `exporters()`.
- **D5a-S2** Size 1010 (951 code), the largest of the wave, above ~700. Recorded.

## D6 (round 1): assets, repository, assist

| Criterion | Proof | Status |
|---|---|---|
| ARQ-26 (default registry empty; a double registers) | `arq_26_default_registry_is_empty_and_a_double_registers` | covered |
| FD-64 (trait in ports; cancel) | `fd_64_rewrite_reports_cancellation` | covered |
| ARQ-102 (3 enums: Repository, Asset, Assist) | `arq_102_repository_error_messages`, `arq_102_asset_error_messages`, `arq_102_assist_error_messages` | covered |
| ARQ-20 | `arq_20_provider_registers_and_runs_through_dyn`, `p4_repository_is_object_safe_and_round_trips_a_document` | covered |
| S15 (`ProjectRepository` over `Document`, `LoadedProject.read_only`, ARQ-55) | `p4_repository_*` | covered |
| P8 D6 hand-mutant rules (`detail` never an argument, `TooNew` order, default empty) | as above | covered |

The traits match 02 §4.11 and §4.15 signature for signature. `AssetKind` has the 8 kinds of 02.

### WARNING
- **D6-W1** class: `implementation`. ref: `crates/gafira-ports/src/assist.rs:66-72`; ADR 0012 rule 4 via ADR 0014 decision 2.
  - `AssistError` has `Cancelled(Cancelled)` but no `From<Cancelled>`. `rewrite` takes a `CancelToken`, so the rule applies.
  - `d6-progress.md` says "nothing needs them yet", but the ADR requires the impl.
- **D6-W2** class: `ambiguity`, follow-up. ref: `assist.rs:47-56`, `assets.rs:32-80`.
  - These are open shapes adapters will rely on:
    - `RewriteRequest.language: Option<String>` (a tag) instead of `Locale`, chosen to avoid a cross-delivery import. The idioma of 02 §4.15 could be `Locale`, now that D2 has landed.
    - `AssetQuery` (text, kinds, tags, color, format, offset, limit; `limit: 0` means no limit).
    - `AssetItem`, `AssetPage { items, total }`, `Materialized { hash, mime }`, `AssistReq { ram_bytes, needs_gpu, model_bytes }`, `Tone`, `Length`, `Suggestion { text }`.
  - None conflicts with 02 or the design. Owner to confirm, in particular `language` as `Option<Locale>` against a free tag, and `limit: 0` as unbounded.

### SUGGESTION
- **D6-S1** Size 704 (664 code). Recorded.

## Checks that could not run

None of this depth. Mutation is `archive`, so T4 is owed at change close, not here.

## Per-class counts (all deliveries)

| Class | Critical | Warning |
|---|---|---|
| implementation | 0 | 2 (D5a-W1, D6-W1) |
| ambiguity | 0 | 4 (D2-W1, D4-W2, D5a-W2, D6-W2), all follow-up |
| architecture | 0 | 0 |
| claim | 0 | 0 |

next: tdd-apply (D5b; it depends on D3, D4 and D5a, all PASS. D5a-W1 and D6-W1 can ride in it or in a small fix commit. The ambiguity follow-ups go to the owner or specifier)
