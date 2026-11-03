## D2 progress (tdd-apply)

Branch `feat/gafira-ports-d2`. Files: `src/i18n.rs`, `src/license.rs`, `tests/i18n.rs`, `tests/license.rs`.

### Done

- `i18n`: `Locale` (serde lowercase), `LanguagePreference` (`parse`, `as_str`, hand-written serde), `Region` (own, `Copy`, private repr), `FormatLocale`, `SystemLocale`, `SystemLocaleSource`, `ResolvedLocale`, `LocaleSource`, `LocaleResolver`, `Translator`, `NumberStyle`, `DateStyle`.
- `license`: `LicenseStatus` (no `Trial`, only `can_export`, O1 a), `LicenseKey` (redacted `Debug`, no `Display`, `expose`), `ReasonCode`, `LicenseError` with `ToMessage`, `LicenseVerifier`.

### RED evidence (skeleton with wrong bodies, then fixed)

| ID / rule | Test | Failing assertion |
|---|---|---|
| P8 parse | `p8_preference_parses_exact_values_and_defaults_to_system` | `"system"`: left Es, right System |
| P8 as_str | `p8_preference_as_str_round_trips_through_parse` | left "", right "system" |
| P8 serde | `p8_preference_serde_is_the_lowercase_string` | left `"System"`, right `"system"` |
| P8 Region | `p8_region_accepts_two_letters_or_three_digits`, `p8_region_equality_ignores_input_case` | `"mx" should be valid`; `left != right` None vs None |
| ARQ-100 types | `arq_100_resolver_double_maps_tags_through_dyn_ports` | left None, right Some("419") |
| P8 can_export | `p8_can_export_only_for_active_and_offline_grace` | `active().can_export()` false |
| P8 expose | `p8_license_key_expose_gives_the_key` | left "", right "k-1" |
| ARQ-94 | `arq_94_license_key_debug_leaks_nothing` | skeleton derived `Debug` printed `LicenseKey("ABCD-...")` |
| ARQ-102 | `arq_102_license_error_messages` | left `error.license`, right `error.license.network` |
| ARQ-20 | `arq_20_license_verifier_is_object_safe`, `arq_20_translator_is_object_safe` | verifier: `expose()` empty gave Rejected; translator passed on skeleton (object safety is compile-level) |
| P8 Locale serde, no Display | `p8_locale_serde_is_es_and_en`, `p8_license_key_has_no_display` | green on the skeleton (derive and absence of impl); covered by mutants only where a wrong body exists |

### Hand mutants

All run with `RUSTFLAGS="--cfg wt_d2"` (after the coordinator's warning about the shared target) and re-run; baseline green before each.

| # | Mutant (rule level) | Killed by |
|---|---|---|
| 1 | An unknown preference reads as `Es` | `p8_preference_parses_exact_values_and_defaults_to_system` (+2) |
| 2 | Parsing is case-insensitive (`ES` is `Es`) | `p8_preference_parses_exact_values_and_defaults_to_system`, `p8_preference_serde_is_the_lowercase_string` |
| 3 | Serialized form is capitalised (`System`) | `p8_preference_serde_is_the_lowercase_string` |
| 4 | `Region` keeps lower case | `p8_region_accepts_two_letters_or_three_digits`, `p8_region_equality_ignores_input_case` |
| 5 | `Region` accepts 3 letters | `p8_region_accepts_two_letters_or_three_digits` |
| 6 | `can_export` true for `Expired` | `p8_can_export_only_for_active_and_offline_grace` |
| 7 | `can_export` false for `OfflineGrace` (ARQ-92) | `p8_can_export_only_for_active_and_offline_grace` |
| 8 | `Debug` of `LicenseKey` prints the key | `arq_94_license_key_debug_leaks_nothing` |
| 9 | `Debug` prints the first key byte | `arq_94_license_key_debug_leaks_nothing`. SURVIVED the first test (it checked only digits); the test now uses a key whose characters the redaction text does not spell, and kills it |
| 10 | `Rejected` shares the `Network` key | `arq_102_license_error_messages` |
| 11 | `Region` accepts 2 digits | `p8_region_accepts_two_letters_or_three_digits` |
| 12 | A non-string preference value reads as `System` | `p8_preference_serde_is_the_lowercase_string` |
| 13 | `LicenseKey` gains `Display` | `compile_fail` doctest of `LicenseKey` (line 52) |

### Gates (exit codes read separately, final state)

`cargo fmt --check` 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` 0; `cargo test --workspace --locked` 0; V5 step 0; ARQ-107 step 0. Negative control: the same ARQ-107 block with the `gafira-ports` skip removed exits 1 (`gafira-ports mentions Locale`); with the skip it exits 0.

### Deviations and open points

- Shapes the spec leaves open, chosen minimal: `ReasonCode` = `Revoked`, `UnknownKey`, `DeviceLimit`, `TokenTampered`; `NumberStyle` = `Decimal`, `Percent`, `Tabular`; `DateStyle` = `Short`, `Long`. `LicenseError::Rejected` carries no message argument (P6 has no field name for a tuple payload). Needs a specifier/owner look before adapters rely on them.
- "No `Display`" is proved by a `compile_fail` doctest (a runtime probe tripped clippy and `#[allow]` is forbidden).
- `ARQ-20` object-safety tests and `Locale` serde are green on first run (derive or compile-level), as in D1.
- Shared-target hazard: sibling worktrees overwrote the `gafira-ports` artifact (stale E0432 and wrong results); resolved with the `wt_d2` cfg and retries.
- Sweep: `Region` copy semantics and `FormatLocale` equality are derived; nothing else found.
