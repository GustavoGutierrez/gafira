# Verify report: engine-contracts D2a, round 1

- Commit: `9df391b3192884151850143921363deaebc01621` (`feat/engine-contracts-d2a`), base `9faa98e`
- Mode full, level L3, depth V2, tier T4, `mutation: archive` (no `mutants` report before close: not a finding)
- Critical code in scope: `ParamSchema` checks (design "Critical code")

**Verdict: FAIL.** The code matched every rule on all probes; no defect found. The FAIL is test strength: hand mutants survive in critical code.

## Gates (each exit code read alone, on a clean export of HEAD)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1192 passed, 0 failed, 0 ignored) |
| `cargo deny check` | 0 |
| CI policy: ARQ-07, forbid unsafe, V5, V12, V13, V14 | 0 each |

The Format step of the policy job is the fmt gate above. V13 and V14 need git, so the policy steps ran in the worktree (read-only); it stayed clean.

## CRITICAL

### C1. `check_values` numeric bounds are not pinned at the edges that matter
- class: implementation
- ref: `crates/gafira-doc/src/contract/params.rs:72`, `:79`; `crates/gafira-doc/tests/contract_param_schema.rs:375-433`
- The acceptance asks for "one fixture per fault and per bound, boundary values included". Three mutants of a spec rule pass the whole suite:
  - IM3: Float upper bound as `v <= max + f64::EPSILON`. It accepts `1.0000000000000002` on `[0, 1]`. The only `next_up` row uses `max = 10`, where one ulp exceeds `EPSILON`.
  - IM4: Float upper bound on `|v|`. It rejects valid negatives on `[-10, 5]`. No `check_values` fixture has a Float range with a negative bound.
  - IM2: Int range compared through `f64`. It accepts `2^53 + 1` when `max = 2^53`. Extreme Int bounds appear in `validate_definition` fixtures only.
- The code is correct: probes P2 and P3 pass on it and fail on each mutant.

### C2. A default in an all-negative Float range is not pinned
- class: implementation
- ref: `crates/gafira-doc/src/contract/params.rs:52`; `crates/gafira-doc/tests/contract_param_schema.rs:234-242`
- IM5: `default.abs() <= max` in `is_consistent` survives. It rejects the valid `Float { min: -2, max: -1, default: -1.5 }`. The accepted rows hold no valid range below zero.
- The code is correct: probe P1 passes on it and fails on the mutant.

### C3. A repeated option id is proved only for adjacent options with the default among them
- class: implementation
- ref: `crates/gafira-doc/src/contract/params.rs:58-61`; `crates/gafira-doc/tests/contract_param_schema.rs:107-116`
- `SAME_ID` is the only fixture: `[a, a]`, default `a`. Two mutants survive:
  - IM6: compare adjacent options only; `[x, y, x]` is accepted.
  - IM7: require only the default id to be unique; `[a, b, b]` with default `a` is accepted.
- The code is correct: probe P6 passes on it and fails on IM6 and IM7. The parameter-level check already has a non-adjacent row; the option-level one does not.

## WARNING

### W1. Non-finite value of another kind (implementer's note 1)
- class: ambiguity
- ref: `specs/changes/engine-contracts/design.md:213`
- Classified as a real, minor ambiguity. "A kind mismatch is `WrongType`" and "Non-finite `Float` or `Color` is `NonFinite`" are both unconditional and overlap on these inputs; the row orders `NonFinite` only against the range.
- ARQ-110 is met either way, since both are a typed `Err` naming the field. Only the user-visible key differs.
- The case is wider than the note: it also covers infinities, a NaN `Color`, and every non-matching kind. Observed: `WrongType` in all of them.
- IM8 and IM9 (non-finite before the kind match) survive. They are not CRITICAL, because no rule says they are wrong. Recommendation: the kind decides first, which keeps the code; then pin it with a test.

### W2. Repeated id in `get`, `defaults` (implementer's note 2)
- class: ambiguity
- ref: `specs/changes/engine-contracts/design.md:212`; `crates/gafira-doc/src/contract/params.rs:148,198-217`
- Classified as ambiguity, and wider than the note. The design has no row for `get` or `defaults`, and the precondition that a schema passed `validate_definition` appears only in `apply-progress.md`.
- Observed on `[d: Float default 0.25, d: Int default 7]`:
  - `get` returns the first definition.
  - `defaults` gives the last: `Int(7)`.
  - `resolve` on empty values gives the first: `Float(0.25)`. So `resolve(empty) != defaults()`.
  - `check_values` tests both definitions, so every value is `WrongType`.
- The schema is rejected by `validate_definition`, so this is outside the valid domain. No rule is broken. Recommendation: state the precondition in the design rather than add code.

### W3. `resolve` can return `Ok` with a non-finite value
- class: ambiguity
- ref: `specs/02-arquitectura-software.md:866`; `specs/changes/engine-contracts/design.md:214-215`; `crates/gafira-doc/tests/contract_param_schema.rs:607`
- Observed, not noted by the implementer:
  - `Float { min: NaN, max: 1, default: NaN }` resolves to `Ok(NaN)`; a default of `+inf` resolves to `Ok(inf)`.
  - An unknown id holding NaN is kept in the `Ok` value, as the design asks.
- ARQ-110 says no non-finite in the output. It also says constructors and plain data do not validate, and it speaks of layer-2 engines, while `resolve` lives in `gafira-doc`.
- The design's `resolve` row matches the code, and a test pins that a default is not looked at. So the code follows the design; the gap is between design and ARQ-110. Same root as W2: nobody states who must call `validate_definition`, or when.

### W4. The ADR 0005 import check misses two forms
- class: implementation
- ref: `crates/gafira-doc/tests/contract_imports.rs:82-119`
- The rule holds at HEAD: a direct grep of `src/contract/` finds only `crate::msg`, `gafira_color`, `std` and local modules, and no `Locale`. So the criterion is met, but the test is weaker than it reads.
- IM11b, an inline `serde_json::Value` path, survives: inline paths are scanned only for `crate::` and `gafira_`.
- IM12b, a `use serde_json::Value` right after an item ending in `}`, survives: statements split on `;`, so that `use` is not at statement start. It is the same fault when the item before is any block.
- WARNING, not CRITICAL: the checker is a test helper and is not in the design's critical table. IM13, a `use` following another inside a function, is killed.

## SUGGESTION

- S1. A small oracle sweep (values such as NaN, both infinities, `MAX`, signed zero, subnormals, `next_up` around 1, `2^53 + 1`, against the written rule) would close C1 and C2 in one test instead of many rows. `proptest` is already a dev-dependency.
- S2. `SOURCES` in `contract_imports.rs` is a manual list, so `cancel.rs` in D2b is unchecked until someone adds it. The implementer flagged this for D2b; a test that the list is complete would remove the dependence on memory.
- S3. The implementer's note on `ParamChoice.label` is for the specifier. ARQ-98 names `ParamSchema`, which does contain it, so nothing is contradicted; naming it would help `check-i18n` later.

## Implementer's claim: items 2 and 3 are equivalent

**The claim holds.** With finite bounds, `(min..=max).contains(&default)` already rejects `min > max` and a non-finite default, so separate checks would change nothing.

- Reasoning: `contains` needs `min <= d` and `d <= max`. Both true imply `min <= max`, and with finite bounds they imply a finite `d`. NaN fails every comparison.
- Oracle P1: all 3,375 combinations of 15 special values (NaN, both infinities, `MAX`, `MIN`, signed zero, subnormals, `next_up` and `next_down` around 1) match a written-out version of every rule.
- EQ-A: adding the explicit `min <= max` and `default.is_finite()` passes the whole suite and all probes.
- The spot-check shows the replacement mutants are real: `!(d < min || d > max)` (row 2) is killed by `arq_110_a_non_finite_bound_or_default_is_a_bad_definition`.
- It depends on one error kind per parameter: a fault-specific error would make these branches observable.

One refinement. The note says "Only the Float bounds need `is_finite`", which is true but more than is needed. EQ-B: `min != -inf && max != +inf` is also equivalent (suite and probes pass), because a NaN bound also fails `contains`. So the NaN halves of mutants 2 can only be killed through the `contains` rewrite, not by dropping `is_finite`. No finding.

## Probes (step 1c, derived before reading the implementer's tests)

13 probes on a scratch export, all pass on HEAD:

| Probe | Covers |
|---|---|
| P1, P1b | Float and Int `validate_definition` against a written oracle (3,375 and 729 cases) |
| P2, P3 | `check_values` Float and Int oracle: NaN, both infinities, subnormals, extremes, `2^53 + 1` |
| P4 | NaN, `+inf`, `-inf` in each channel of each `Color` variant, default and value; wide finite colours pass |
| P5 | 5 x 5 kind matrix, finite and non-finite |
| P6, P7 | non-adjacent repeats, empty options, first-fault order, exact Choice match |
| P8 | order across fault kinds, `resolve` idempotent, input unchanged, id order |
| P9 | signed zero in ranges and defaults |
| P10, P11 | accessors; error table, key grammar, `dyn Describe`, `const` `Descriptor`, `Send + Sync` |
| C1 probe | characterisation of inputs the rules leave open (W1 to W3) |

## Hand mutants (step 4b, implementer's suite only)

| # | Mutant | Result |
|---|---|---|
| Spot | Row 2: default NaN accepted through `!(d < min \|\| d > max)` | killed, recorded test |
| IM1 | Sign: `-inf` value gives `OutOfRange` | killed, `arq_110_check_values_reports_non_finite_before_the_range` |
| IM2 | Boundary: Int check through `f64` | **survived** (C1) |
| IM3 | Boundary: Float upper bound with `EPSILON` | **survived** (C1) |
| IM4 | Sign: Float upper bound on `\|v\|` | **survived** (C1) |
| IM5 | Sign: Float default upper bound on `\|d\|` | **survived** (C2) |
| IM6 | Option ids compared when adjacent only | **survived** (C3) |
| IM7 | Only the default option id must be unique | **survived** (C3) |
| IM8 | Non-finite Float before the kind match | survived, rule open (W1) |
| IM9 | Non-finite Color before the kind match | survived, rule open (W1) |
| IM10 | `check_values` stops after the first present value | killed, `arq_110_check_values_reports_the_first_fault_in_definition_order` |
| IM11b | Inline `serde_json` path | survived (W4) |
| IM12b | `use serde_json` after an item | survived (W4) |
| IM13 | `use` inside a function | killed, `adr_0005_contract_imports_only_msg_color_serde_and_std` |

14 run, 4 killed. EQ-A and EQ-B are equivalence experiments and are not counted. IM11 and IM12 used `kurbo`, which `gafira-doc` does not link, so they did not compile and were replaced by IM11b and IM12b. Each survivor IM2 to IM9 fails a probe, so none is equivalent.

## Criteria

| # | Criterion | Proof | State |
|---|---|---|---|
| 1 | `Describe`, `Descriptor`, `AudioCrossfade` | `contract_describe`; design form read; P11 | covered |
| 2 | ARQ-98 labels are `MessageKey` | `arq_98_*`; typed fields | covered |
| 3 | `new`, `set` plain data | `arq_110_new_*`, `arq_110_set_*` | covered |
| 4 | `validate_definition`, each fault and bound | table tests | **partial**: C2, C3 |
| 5 | `check_values`, each rule and bound | table tests | **partial**: C1 |
| 6 | Unknown ids ignored and kept | `arq_110_a_missing_id_*`, `arq_110_resolve_fills_*` | covered |
| 7 | `resolve` | three `arq_110_resolve_*` | covered |
| 8 | Accessors, no conversion | 5 x 5 table | covered |
| 9 | Shared error table | `contract_param_error`, never calls `Display` | covered |
| 10 | ADR 0012 keys, exhaustive `match`, `Text` args | same; source read: no `_` arm | covered |
| 11 | ARQ-110 NaN, `+inf`, `-inf` | `arq_110_check_values_reports_non_finite_before_the_range` | covered |
| 12 | ARQ-111 iteration order | `arq_111_iter_*` | covered |
| 13 | ADR 0005 import rule | direct grep; test is weak (W4) | covered |
| 14 | V5 rows, no `Locale` in `gafira-doc` | CI step exit 0; `git grep` empty | covered |

12 of 14 covered, 2 partial.

**Product IDs** (`git grep` over tests): ARQ-98, ARQ-110, ARQ-102 (`ParamError`) and ARQ-111 (order) are named and proved, except the partial items above. Left out of D2a by the delivery table: ARQ-107 (D1), ARQ-112 (D2b), ARQ-102 for engine errors (D3).

**Artifact integrity (step 3c):** passes.
- `docs(engine-contracts): slice card` (`07947c5`) and `slice card amend` (`7b8c2cd`) each touch only `apply-progress.md`, and both precede D2a.
- D2a changes the spec artifacts only by appending 103 lines to `apply-progress.md`; its text up to the base is byte-identical. `design.md` is untouched.
- The commits carry RED evidence and no attribution lines.

## Size

1,352 lines (394 source, 958 tests), against a forecast of 360. No finding, per the owner's statement relayed by the coordinator that the size of serial deliveries does not concern him.

## Result

- Verdict: FAIL
- Counts: 3 CRITICAL, 4 WARNING, 3 SUGGESTION
- Per class: implementation 4 (3 CRITICAL, 1 WARNING), ambiguity 3 (3 WARNING)

next: specifier (W1, W2, W3), then tdd-apply (C1, C2, C3, W4, and pins for the W1 and W2 decisions)

---

# Verify report: engine-contracts D2a, round 2 (closure)

- Commit: `a57966e4842ea5a3da96bc62c212b8bfc2f7bd34` (`feat/engine-contracts-d2a`), base `9faa98e`
- Mode full, level L2 (the design's Flow line; round 1 wrote L3 by mistake), depth V2, tier T4, `mutation: archive` (no `mutants` report before close: not a finding)

**Verdict: PASS for delivery D2a.** All seven findings are closed. D2b remains, so the change is not done.

## Gates (each exit code read alone, on a clean export of HEAD)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1200 passed, 0 failed, 0 ignored) |
| `cargo deny check` | 0 |
| CI policy: ARQ-07, forbid unsafe, V5, V12, V13, V14 | 0 each |

The count rose by 8 from round 1, matching the 7 new tests in `contract_param_schema.rs` and 1 in `contract_imports.rs`. As in round 1, the policy steps ran in the worktree (read-only); it stayed clean.

## Artifact integrity

Passes.
- `9a22c04` touches only `design.md`. Git cannot show who wrote it: every commit has the same author. Its attribution to `architect` is the coordinator's statement.
- `03c5e3b`, `06f8865` and `a57966e` do not touch `design.md`. `a57966e` touches only `apply-progress.md`.
- `apply-progress.md` was only appended to in the fix round.
- `params.rs` changed by 8 doc-comment lines only, as `apply-progress.md` claims. The rest of the fix round is tests.
- No commit in the delivery carries attribution lines. The slice card checks of round 1 are unchanged: their commits precede D2a.

## Closure

Every mutant was reapplied on a fresh export of HEAD against the implementer's suite alone. All compiled, and each failed in the test named.

| Finding | Mutant | Result | Status |
|---|---|---|---|
| C1 | IM2 Int through `f64` | killed, `arq_110_int_values_match_the_rule_over_special_values` | closed |
| C1 | IM3 Float upper + `EPSILON` | killed, `arq_110_float_values_match_the_rule_over_special_values` | closed |
| C1 | IM4 Float upper on `\|v\|` | killed, same | closed |
| C2 | IM5 default on `\|d\|` | killed, `arq_110_float_definitions_match_the_rule_over_special_values` | closed |
| C3 | IM6 adjacent only | killed, `arq_110_a_choice_default_must_be_an_option_and_option_ids_must_differ` | closed |
| C3 | IM7 default id only | killed, same | closed |
| W1 | IM8 non-finite Float first | killed, `arq_110_a_non_finite_value_of_another_kind_is_a_wrong_type` | closed |
| W1 | IM9 non-finite Color first | killed, same | closed |
| W4 | IM11b inline `serde_json` path | killed, `adr_0005_contract_imports_only_msg_color_serde_and_std` | closed |
| W4 | IM12b `use` after an item | killed, same | closed |

- **W1:** the design now says the kind decides first, which is what the code does, so IM8 and IM9 are now wrong against a written rule.
- **W2:** closed by the Domain rule and C12. The `get` and `defaults` rows exist, and the `ParamSchema` doc states the precondition. `resolve` on empty values equals `defaults`, pinned by `arq_110_resolve_on_no_values_is_defaults`.
- **W3:** closed by the same Domain rule and the unknown-ids row. `arq_110_resolve_keeps_an_unknown_id_whatever_it_holds` pins NaN and both infinities, and the test that pinned an out-of-range default was rewritten inside the domain.
- **"No test pins it" (W2, W3):** closure probe DOM asserted `validate_definition` at the start of `get`, `check_values` and `defaults`, which also covers `resolve`. All `gafira-doc` tests still pass, so no test calls these on an invalid schema.
- **Probes:** the 13 probes of round 1 pass at HEAD. No independent mutants were spent.

## Criteria

14 of 14 covered. Criteria 4 and 5, partial in round 1, are now covered by the closures of C1 to C3. Criterion 13 now also has a stronger test (W4).

## Noted, not findings

- SP16, SP17 and SP18 are spec edits still to be applied. SP18 (`roi`, `cost_hint` and `bind` cannot return `Err`) is outside D2a and points to `RS4`, so it needs routing by the coordinator.
- ARQ-110 and T4: the critical `ParamSchema` code still owes its T4 run at close (`mutation: archive`).

## SUGGESTION

- S3 (carried). `ParamChoice.label` should be named in ARQ-98 for `check-i18n`.
- S4 (new). The doc of `get` (`params.rs:154`) still says "The first parameter with this id"; the design now says "The definition with this id", and on a valid schema there is only one.

S1 and S2 of round 1 were taken up by the fix round.

## Size

1,623 lines under `crates/`, against a forecast of 360. No finding, for the same reason as round 1.

## Result

- Verdict: PASS (delivery D2a; D2b remains)
- Counts: 0 CRITICAL, 0 WARNING, 2 SUGGESTION
- Per class: none open

next: tdd-apply (D2b)
