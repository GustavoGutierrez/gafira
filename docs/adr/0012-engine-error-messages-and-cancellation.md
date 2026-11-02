# 0012. Engine errors share one key rule and one cancelled value

- Status: proposed
- Date: 2026-11-01
- Related: ARQ-102, ARQ-110, ARQ-112, 02 §7.3, 02 §14.2, ADR 0005, changes `engine-contracts`, `layer-2`

## Context

Every layer-2 crate needs an error enum that implements `ToMessage` (ARQ-102). ARQ-112 requires an `Err` of "cancelled", but each crate has its own enum and no common variant. Without a rule, each crate invents its key names, puts library text in arguments and spells cancellation differently. Keys are a stable contract with the catalogs, so a late rename touches both languages.

Existing errors are hand-written (`crates/gafira-layout/src/error.rs`, `crates/gafira-pathops/src/error.rs`). 02:1298 says `thiserror`, which `Cargo.lock` does not list.

## Decision

1. **Keys.** `error.<area>.<variant>`, two or three segments, `snake_case`. `<area>` is the crate name without `gafira-`, or a shared area: `param` for `ParamError`, `job` for `Cancelled`. A nested cause folds into the third segment (`error.pathops.tolerance_non_finite`).
2. **Mapping.** `to_message` is an exhaustive `match` with no `_` arm. Argument names match the field names and are stable.
3. **Arguments.** Numbers go in `Int`, `Count`, `Decimal` or `Quantity`. Stable technical identifiers (`field`, `param`, a node id) go in `Text`, untranslated. Free text from a third-party library never goes in an argument; it stays in `Display` for the local log.
4. **Cancellation.** `gafira_doc::contract::Cancelled` is the one cancelled value, with key `error.job.cancelled`. An engine error whose operations take a `CancelToken` has a variant `Cancelled(Cancelled)` and `From<Cancelled>`, and its `to_message` delegates to it. Engines with no long operation have no such variant.
5. **Form.** Errors stay hand-written: `Display` in English, `std::error::Error`, no `thiserror`.

## Consequences

- A new variant does not compile until it has a key. New crates copy one pattern, and `?` works from `CancelToken::check`.
- `gafira-engine` can treat every cancelled job the same way, by key or by variant.
- Non-finite classes and library strings do not reach the person; they stay in the log.
- `Text` now carries more than user content, so 02 §14.2 needs a wording change (`specifier`).
- Hand-written `Display` costs a few lines per variant, and `Display` text stays non-critical for mutation.

## Alternatives considered

- **Keys per crate with no rule.** Cheaper today; it pushes the rename cost to the catalogs.
- **Mapping `field` to a translated key.** `field` is a runtime `&'static str`, so this needs a closed table of dozens of literals in `gafira-layout` alone, for text a person should rarely see.
- **A new `MsgArg::Field` variant.** Clearer, but it adds a variant that 02 §14.2 does not list and that `Translator` must handle, with the same output as `Text`.
- **An unrelated `Cancelled` variant per enum.** `CancelToken::check` would have no single return type, and each crate would add its own key.
- **One shared engine error enum.** Contradicts one enum per crate (02:1298) and couples unrelated engines.
- **`thiserror`.** Saves lines, but adds a dependency to crates that have none, for no behaviour.
