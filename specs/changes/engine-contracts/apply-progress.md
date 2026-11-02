## Slice card
Flow: full | level: L3 | pack: new function | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: public contracts shared by all layer-2 engines with design decisions the spec left open; owner chose the parallel full route
Flow-change: design.md line 13 said level L2 -> L3 (trigger: serialized contracts every engine copies; level raised by the coordinator, mode unchanged)
Late record: the coordinator skipped this card before D1. It was written after D1's code, at verify round 1, finding C1. The scope and acceptance it points to were committed in `design.md` at 6bede8b, before D1, and did not change. It precedes D2a, D2b and D3. The owner accepted this late record on 2026-11-01; history was not rewritten.
Product IDs: ARQ-102 (types, engine errors; catalog, UI, lint and `UserError` parts pending), ARQ-107, ARQ-98 (labels), ARQ-110 (params), ARQ-112 (on doubles) (source: specs/02-arquitectura-software.md)
Scope: `specs/changes/engine-contracts/design.md`, "Scope", in column.      Non-goals: same table, out column.
Deliveries: D1, D2a, D2b serial; D3 parallel after D1 (design.md "Delivery table").
Acceptance: design.md "Acceptance", copied by reference, unchanged.
Plan: design.md "Quick path" and "Public API".
Owner decisions touched: ADR 0012 accepted 2026-11-01; none in `specs/00-decisiones.md`.

Flow: full | level: L3 | pack: new function | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: public contracts shared by all layer-2 engines with design decisions the spec left open; owner chose the parallel full route

# Apply progress: engine-contracts

## D1 (branch `feat/engine-contracts-d1`)

Done: `msg` types, `is_valid_key`, `msg!` (ARQ-102 types, ARQ-107), `gafira_geom::IRect` (C10).

Commits:

- `feat(doc): add message types and the message key grammar`
- `feat(doc): add the msg! macro with a compile-time key check`
- `feat(geom): add IRect, a half-open signed integer rectangle`

Each commit body holds its RED output. Gates run per commit, exit codes read separately: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0.

### RED evidence

Each test was written first, and a compile stub gave a real assertion failure (no compile-error RED). The stub is replaced in the same commit.

| Criterion | Test | RED assertion (against the stub) |
|---|---|---|
| `is_valid_key` table (ARQ-107) | `msg::tests::arq_107_is_valid_key_follows_the_grammar` (moved to `tests/msg_key.rs` in fix round 1) | stub `false`: `should accept "edit.set_property"` |
| Every `MsgArg` and `Unit` builds, `Message` is `Clone` + `PartialEq` | `msg::tests::arq_102_every_arg_variant_and_unit_builds_and_compares` | reconstructed in fix round 1, see below: `Message` `PartialEq` stubbed to `false`: `assertion left == right failed` at `msg.rs:165:9` |
| `msg!` with 0 args | `tests/msg_macro.rs::arq_102_msg_with_no_arguments_has_the_key_and_no_args` | stub returns key `""` and no args: `left: MessageKey("")`, `right: MessageKey("export.pages_selected")` |
| `msg!` with 1, 2 args, order | `tests/msg_macro.rs::arq_102_msg_keeps_argument_names_and_values_in_order` | same stub: the expected args are not present (see commit body for the exact output) |
| `msg!` trailing comma | `tests/msg_macro.rs::arq_102_msg_accepts_trailing_commas` | `left: []`, `right: [("x", Text("v"))]` |
| Invalid key does not compile | `compile_fail` doctest on `msg!` | `Test compiled successfully, but it's marked compile_fail` |
| `IRect` rules | seven tests in `crates/gafira-geom/src/irect.rs` | 7 of 7 failed against constant-return stubs (see commit body) |

### Hand mutants

| # | Mutant | Killing test | Result |
|---|---|---|---|
| 1 | 4 segments accepted; 3 rejected | `arq_107_is_valid_key_follows_the_grammar` (`a.b.c.d`; `error.format.too_new`) | killed |
| 2 | segment may start with a digit | same (`a.2b`) | killed |
| 3 | empty segment accepted: trailing, middle, leading | same (`a.b.`, `a..b`, `.a`) | killed |
| 4 | uppercase or non-ASCII accepted, at segment start or later | same (`A.b`, `a.bB`, `a.bé`) | killed after adding rows `a.bB`, `aB.c`, `a.bé`, `é.b`: the first table let both survive |
| 5 | `contains` with `x <= x1`, `y <= y1`, `x < x0`, `y < y0` | `contains_is_half_open` | killed |
| 6 | `width`/`height` as `(x1 - x0) as u32` | `width_and_height_are_the_extent_and_never_overflow`, `area_is_computed_in_u64` (overflow panic) | killed |
| 7 | inverted: width/height not 0, `is_empty` false, `contains` true | `an_inverted_rect_is_empty_with_zero_extent_and_contains_nothing` | killed |
| 8 | `area` in `u32` (wrapping) | `area_is_computed_in_u64` | killed |
| 9 | `intersect` min/max swapped, lows and highs, x and y | `intersect_takes_the_max_of_lows_and_min_of_highs` | killed |
| 10 | `inflate` wrapping instead of saturating; wrong direction, all four sides | `inflate_moves_each_side_outward_and_saturates` | killed |
| 11 | `msg!` argument order reversed; trailing comma dropped (after the arguments, and inside a variant's parentheses) | `arq_102_msg_keeps_argument_names_and_values_in_order`, `arq_102_msg_accepts_trailing_commas` | reversed order and dropped comma killed; the in-variant comma survived until the row `Quantity(1.5, Unit::Px,)` was added, then killed (the consumer test fails to compile) |

Equivalent mutant, not in the list: `x1.wrapping_sub(x0) as u32` gives the same value as `abs_diff` when `x1 > x0`.

### Measured lines

`git diff --shortstat develop` (before this file): 4 files changed, 427 insertions(+). Forecast 370 (x1.4: 520). Within bounds.

### Deviations

- Corrected in fix round 1 (the earlier text, "declarations cannot fail before they exist", was wrong): the type-only test does get a real RED from a stubbed `PartialEq`. On an export of `03e19cc` (before this fix, history untouched) `impl PartialEq for Message { fn eq(..) -> bool { false } }` replaced the derive; `cargo test -p gafira-doc --lib arq_102` failed on `assertion left == right failed` at `msg.rs:165:9`. The stub is a reconstruction, not in git; the original commit recorded no RED for this test.
- `msg!` accepts a trailing comma inside a variant's parentheses too (`Quantity(1.5, Unit::Px,)`), as the design's pattern `$(,)?` allows.
- One expectation in the first `inflate` test was wrong (full rectangle inflated by `i32::MIN` is `(0, 0, -1, -1)`); fixed in the test during GREEN, no code change.

### Findings for other stages

None. `git grep Locale -- crates/gafira-doc` finds nothing. Out of scope and untouched: `contract`, `ToMessage` impls, pathops, layout.

## D1 fix round 1 (verify round 1: C2, W2, S1, S2)

- C2: new `crates/gafira-doc/tests/msg_key.rs::arq_107_only_the_class_bytes_are_accepted_in_every_position` tries all 256 bytes at the start of the first segment, the start of a later segment, mid-segment and at the end. Honestly not RED: it closes a test gap and the code was already correct. Proven by hand mutants, each reverted: IM2a (digit range widened to `/` and `:`) killed; IM2b (lowercase widened to `` ` `` and `{`) killed; IM2c (segment-start range widened to `` ` ``..`{`) killed; `-` accepted in continuation killed.
- S1: the grammar table test moved from `msg.rs` `#[cfg(test)]` to `tests/msg_key.rs`, unchanged. Not RED: a move.
- S2: the merged RED row is split (above).
- W2: RED recorded (Deviations).
- Not touched: C1 and W1 (coordinator and owner).

## D3 (branch `feat/engine-contracts-d3`)

Done: `ToMessage` for `LayoutError` and `PathOpError` (C11 tables, ADR 0012), both exhaustive with no `_` arm; `gafira-pathops -> gafira-doc`; V5 row `gafira-pathops gafira-geom gafira-color gafira-doc`. `Cargo.lock` gained only the `gafira-doc` edge.

Commits:

- `feat(layout): map LayoutError to message keys`
- `feat(pathops): map PathOpError to message keys` (with `Cargo.toml`, `Cargo.lock` and the V5 row, ADR 0007)

Gates per commit, exit codes read separately: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0. V5 step of `ci.yml` run locally: exit 0.

### RED evidence

A compile stub returned key `error.<area>.stub` with no arguments, so RED is an assertion failure.

| Criterion | Test | RED assertion |
|---|---|---|
| ARQ-102 layout, per-variant | `gafira-layout/tests/message.rs::arq_102_every_layout_variant_gives_its_table_key_and_arguments` | `left == right` failed: `NonFinite { field: "bleed" }`, left `error.layout.stub`, right `error.layout.non_finite` with `field` |
| ARQ-102 layout, valid and distinct | `arq_102_layout_keys_are_valid_and_distinct` | failed: stub key shared by all variants |
| ARQ-102 pathops, per-variant | `gafira-pathops/tests/message.rs::arq_102_every_pathops_variant_gives_its_table_key_and_arguments` | `left == right` failed: `NonFiniteCoordinate { value: Nan }`, left `error.pathops.stub` |
| ARQ-102 pathops, valid and distinct | `arq_102_pathops_keys_are_valid_and_distinct` | `error.pathops.stub is used by two rows` |
| `OpenSubpath` saturation | `arq_102_an_open_subpath_index_saturates_at_i64_max` | `left: []`, `right: [("subpath", Int(0))]` |
| Class and `Geometry` text out of the message | `arq_102_the_non_finite_class_and_the_geometry_text_stay_out_of_the_message` | not RED: passed against the stub (no arguments). Gap-closing, proved by mutants 6 and 7 |

No test calls `Display`; `to_message` is compared with literal messages.

### Hand mutants

| # | Mutant | Killing test | Result |
|---|---|---|---|
| 1 | two variants share a key (layout: `invalid_box` -> `non_finite`; pathops: `open_subpath` -> `geometry`) | `*_keys_are_valid_and_distinct` and per-variant | killed (both crates) |
| 2 | wrong key, each of 6 layout and 9 pathops rows in turn | per-variant fixture tests | killed, 15 of 15 |
| 3 | argument name wrong (`field`, `node`, `value`, `limit`, `subpath`, `minimum`), extra argument, missing argument, `value`/`limit` swapped, order reversed | per-variant fixture tests | killed |
| 4 | `NotPositive` and `NonFinite` keys swapped, stroke width and tolerance | `arq_102_every_pathops_variant_gives_its_table_key_and_arguments` | killed, both |
| 5 | `OpenSubpath` as `*subpath as i64` (wraps) | `arq_102_an_open_subpath_index_saturates_at_i64_max` | killed |
| 6 | `Geometry(s)` passes `s` as `Text` | `arq_102_the_non_finite_class_and_the_geometry_text_stay_out_of_the_message` and per-variant | killed |
| 7 | argument from `Display` (`Text(self.to_string())` for layout `field`, `node`; pathops `Geometry`, `NonFiniteCoordinate` class) | per-variant fixtures | killed, all four |
| 8 | V5 row without `gafira-color gafira-doc` (copy of the step script, scratchpad) | the step itself | exits 1 with `gafira-pathops must not depend on gafira-color` and `... gafira-doc`; the real row exits 0 |

### Measured lines

`git diff --shortstat 9faa98e` (before this section): 7 files changed, 279 insertions(+), 1 deletion(-). Forecast 200 (x1.4: 280). At the limit, not over.

### Deviations

- Mutant 4 was applied as a key swap between the two `NotPositive`/`NonFinite` rows of each area, which gives the same observable message as routing the cause to the other arm.
- Mutant 8 has no committed test: it is a run of the step script with the row edited, as the list asked ("a negative test for V5"). Not repeatable from git; the real row passes.

### Findings for other stages

None.

### D3 fix round 1 (verify round 1: C1, C2, C3, S1)

Tests only in `crates/gafira-pathops/tests/message.rs`; no production change. These close gaps in code that was already right, so there is no RED: the new tests pass on the unchanged code, and each hand mutant of the report was applied, seen to fail, and restored (source diff empty afterwards).

- C1: the non-finite test now walks all three classes (`Nan`, `Infinity`, `NegativeInfinity`) for stroke width and tolerance, and compares the whole message with the table key, so the test name's claim holds.
- C2 and S1: `arq_102_an_open_subpath_index_saturates_at_i64_max` uses literal expected values: `0`, `3`, `i32::MAX`, `usize::MAX`, and on 64-bit targets `i32::MAX + 1`, `i64::MAX - 1`, `i64::MAX`, `i64::MAX + 1`. On 32-bit `usize::MAX` keeps its value.
- C3: new `arq_102_a_not_positive_number_keeps_its_value_and_sign` runs `-3.0`, `-0.5`, `0.0`, `2.0e-9` through both stroke width and tolerance.

| Mutant | Result | Killing test |
|---|---|---|
| H1 NaN width and tolerance routed to `*_not_positive` with a NaN value | killed | `arq_102_the_non_finite_class_and_the_geometry_text_stay_out_of_the_message` |
| H2 saturation over the `i32` range | killed | `arq_102_an_open_subpath_index_saturates_at_i64_max` |
| H3a `tolerance_not_positive` uses `value.abs()` | killed | `arq_102_a_not_positive_number_keeps_its_value_and_sign` |
| H3b same on `stroke_width_not_positive` (extra) | killed | same, plus the table test |
| H3c tolerance value hard-coded to `0.0` (extra) | killed | `arq_102_a_not_positive_number_keeps_its_value_and_sign` |
## D2a (branch `feat/engine-contracts-d2a`, from `develop` 9faa98e)

Done: `gafira_doc::contract` with `Describe`, `Descriptor`, `AudioCrossfade`, `ParamChoice`, `ParamKind`, `ParamDef`, `ParamSchema` (`new`, `params`, `get`, `defaults`, `validate_definition`, `check_values`, `resolve`), `ParamValue`, `ParamValues`, `ParamError` with `Display`, `Error` and `ToMessage` (ARQ-98 labels, ARQ-110 params, ARQ-102 for `ParamError`, ARQ-111 order). Out of scope and untouched: `CancelToken`, `Progress`, `checkpoint`, testkit, `cancel.rs`.

Commits (each passes the three gates alone, exit codes read separately: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0):

1. `a5aa493` feat(doc): add Describe, Descriptor and AudioCrossfade
2. `ee01e9d` feat(doc): add ParamValue and ParamValues
3. `7672c92` feat(doc): add the ParamSchema types, new, get and defaults
4. `4fe1420` feat(doc): add ParamError and ParamSchema::validate_definition
5. `a7dea1e` feat(doc): add ParamSchema::check_values
6. `378dc7a` feat(doc): add ParamSchema::resolve
7. `75b150d` test(doc): cover every accessor against every other kind
8. `56f3c71` feat(doc): implement ToMessage for ParamError
9. `a2b4c13` test(doc): check the import rule of gafira_doc::contract

Each commit body holds its RED output. RED is a failing assertion against a compile stub in every case; no RED was a compile error.

### RED evidence

| Criterion | Test (`crates/gafira-doc/tests/`) | RED assertion (against the stub) |
|---|---|---|
| Descriptor keeps id, label key, version (ARQ-98) | `contract_describe::arq_98_a_descriptor_keeps_id_label_key_and_version_behind_describe` | `new` returned an empty descriptor: `left: ""`, `right: "blur.gaussian"` |
| `ParamValues` set, get, replaced value (ARQ-110) | `contract_param_values::arq_110_set_stores_without_validating_and_returns_the_replaced_value` | `left: None`, `right: Some(Float(2.0))` |
| `ParamValues` iteration order (ARQ-111) | `contract_param_values::arq_111_iter_follows_id_order_not_insertion_order` | `left: []`, `right: [("alpha", Int(1)), ...]` |
| Accessors, no conversion (ARQ-110) | `contract_param_values::arq_110_accessors_return_only_their_own_kind_and_never_convert` | `left: None`, `right: Some(1.5)`; rewritten as a 5 x 5 table in commit 7, see below |
| `new` plain data (ARQ-110) | `contract_param_schema::arq_110_new_keeps_the_definitions_in_order_and_validates_nothing` | `new` dropped its input: `left: []` |
| `get` first match (ARQ-98) | `contract_param_schema::arq_98_get_finds_the_first_definition_with_the_id` | `left: None`, `right: Some("a")` |
| `defaults` per kind (ARQ-110) | `contract_param_schema::arq_110_defaults_gives_each_parameter_its_default_in_its_own_kind` | `left: ParamValues { values: {} }` |
| `validate_definition`: repeated id | `arq_110_a_repeated_id_is_a_bad_definition` | `left: Ok(())`, `right: Err(BadDefinition { param: "a" })` |
| non-finite bound or default, 10 rows | `arq_110_a_non_finite_bound_or_default_is_a_bad_definition` | row "min NaN": `left: Ok(())` |
| `min > max`, `min == max` | `arq_110_min_above_max_is_bad_and_min_equal_to_max_is_valid` | `Float { min: 2.0, max: 1.0 }`: `left: Ok(())` |
| default outside, at each bound | `arq_110_a_default_outside_the_range_is_bad_and_a_bound_is_inside` | `default: -5e-324`: `left: Ok(())` |
| Choice default and option ids | `arq_110_a_choice_default_must_be_an_option_and_option_ids_must_differ` | default `"diagonal"`: `left: Ok(())` |
| non-finite Color default | `arq_110_a_non_finite_color_default_is_a_bad_definition` | `Srgb` NaN red: `left: Ok(())` |
| first fault in order | `arq_110_validate_definition_reports_the_first_fault_in_definition_order` | `left: Ok(())`, `right: Err(BadDefinition { param: "z" })` |
| `check_values`: range edges | `arq_110_check_values_rejects_a_number_just_outside_the_range` | `f = Float(-5e-324)`: `left: Ok(())` |
| NonFinite before range | `arq_110_check_values_reports_non_finite_before_the_range` | NaN: `left: Ok(())`, `right: Err(NonFinite { param: "f" })` |
| Choice not in options | `arq_110_check_values_rejects_a_choice_that_is_not_an_option` | `"diagonal"`: `left: Ok(())` |
| WrongType, no conversion | `arq_110_check_values_rejects_every_other_kind_without_converting` | `softness = Int(3)`: `left: Ok(())`, `right: Err(WrongType { param: "softness" })` |
| first fault in schema order | `arq_110_check_values_reports_the_first_fault_in_definition_order` | `left: Ok(())`, `right: Err(OutOfRange { param: "z" })` |
| `resolve` fills, keeps given and unknown | `arq_110_resolve_fills_missing_ids_and_keeps_given_and_unknown_values` | the three defaults missing from the result |
| `resolve` returns the check error | `arq_110_resolve_returns_the_error_of_check_values` | `left: Ok(.. NaN ..)`, `right: Err(NonFinite { param: "softness" })` |
| `resolve` checks before filling | `arq_110_resolve_checks_the_given_values_before_filling_defaults` | `left: None`, `right: Some(5.0)` |
| `ParamError` keys and argument | `contract_param_error::arq_102_each_param_error_gives_its_key_and_the_param_argument` | `key: "error.param.stub"`, `args: []` |
| keys valid and distinct | `contract_param_error::arq_102_param_error_keys_are_valid_and_distinct` | the stub key repeats |
| `param` argument follows the id | `contract_param_error::arq_102_the_param_argument_follows_the_parameter_id` | `left: []`, `right: [("param", Text("softness"))]` |

Not RED, they pass against the stub, and each is proved by a hand mutant: `arq_110_a_consistent_schema_is_a_valid_definition` (accepting side), `arq_110_check_values_accepts_values_inside_the_schema_and_bounds_are_inclusive` (killed by mutants 8), `arq_110_a_missing_id_passes_and_an_unknown_id_is_ignored` (mutants 12, 13), the accessor table of commit 7 (mutants 15), and `contract_imports` (mutant 17). `AudioCrossfade` has no test: a derive-only enum with one variant.

### Hand mutants

Each applied by hand to the code, tests run, then restored; none committed. "Killed" names a failing test.

| # | Mutant | Killing test(s) | Result |
|---|---|---|---|
| 1 | repeated id accepted | `arq_110_a_repeated_id_is_a_bad_definition`, `arq_110_validate_definition_reports_the_first_fault_in_definition_order` | killed |
| 2 | `min` non-finite accepted (NaN, -inf) | `arq_110_a_non_finite_bound_or_default_is_a_bad_definition` | killed |
| 2 | `max` non-finite accepted (NaN, +inf) | same | killed |
| 2 | `default` non-finite accepted (NaN; infinity), as `!(d < min \|\| d > max)` and as `\|\| default.is_infinite()` | same | killed |
| 3 | `min > max` accepted, Float and Int | `arq_110_min_above_max_is_bad_and_min_equal_to_max_is_valid` | killed |
| 3 | `min == max` rejected, Float and Int | same | killed |
| 4 | default below `min` accepted; above `max` accepted; equal to a bound rejected; Float and Int each | `arq_110_a_default_outside_the_range_is_bad_and_a_bound_is_inside`, `arq_110_min_above_max_is_bad_and_min_equal_to_max_is_valid` | killed (6 of 6) |
| 5 | `Choice` default not in options accepted | `arq_110_a_choice_default_must_be_an_option_and_option_ids_must_differ` | killed |
| 5 | repeated option id accepted | same | killed |
| 6 | non-finite `Color` default accepted | `arq_110_a_non_finite_color_default_is_a_bad_definition` | killed |
| 7 | `validate_definition` reports the last fault; or the duplicate pass runs before the consistency pass; or after it | `arq_110_validate_definition_reports_the_first_fault_in_definition_order` | killed (3 of 3) |
| 7 | `check_values` walks the values (id order) instead of the schema | `arq_110_check_values_reports_the_first_fault_in_definition_order` | killed |
| 8 | Float max exclusive; Float min exclusive; Int max exclusive; Int min exclusive | `arq_110_check_values_accepts_values_inside_the_schema_and_bounds_are_inclusive` | killed (4 of 4) |
| 9 | range before non-finite (NaN gives `OutOfRange`); range before non-finite with NaN passing the range; Float non-finite not checked; Color non-finite not checked | `arq_110_check_values_reports_non_finite_before_the_range` | killed (4 of 4) |
| 10 | kind mismatch accepted; `Int` converted to `Float`; `Float` converted to `Int` | `arq_110_check_values_rejects_every_other_kind_without_converting` | killed (3 of 3) |
| 11 | `Choice` not in options accepted | `arq_110_check_values_rejects_a_choice_that_is_not_an_option` | killed |
| 12 | missing id rejected by `check_values` | `arq_110_a_missing_id_passes_and_an_unknown_id_is_ignored` and five more | killed |
| 13 | unknown id rejected by `check_values` | `arq_110_a_missing_id_passes_and_an_unknown_id_is_ignored` | killed |
| 13 | unknown id dropped by `resolve`; unknown id rejected by `resolve` | `arq_110_resolve_fills_missing_ids_and_keeps_given_and_unknown_values` | killed (2 of 2) |
| 14 | `resolve` fills before checking | `arq_110_resolve_checks_the_given_values_before_filling_defaults` | killed |
| 14 | `resolve` overwrites a given value; drops the given values; skips the check | `arq_110_resolve_fills_missing_ids_and_keeps_given_and_unknown_values`, `arq_110_resolve_returns_the_error_of_check_values` | killed (3 of 3) |
| 15 | `float()` converts `Int`; `int()` converts `Float` | `arq_110_accessors_return_only_their_own_kind_and_never_convert` | killed |
| 15 | `float()` accepts `Bool`; `choice()` accepts `Color` | same, after commit 7 | survived the first test, killed after the 5 x 5 table |
| 15 | `bool()` or `int()` of an absent id is a value; `set` returns `None`; `iter` reversed | accessors test; `arq_110_set_stores_without_validating_and_returns_the_replaced_value`; `arq_111_iter_follows_id_order_not_insertion_order` | killed |
| 16 | `to_message` key swapped between two variants; key typo (two variants); wrong argument name; argument is `Count` not `Text`; fixed argument value; no argument | `arq_102_each_param_error_gives_its_key_and_the_param_argument`, `arq_102_the_param_argument_follows_the_parameter_id`, `arq_102_param_error_keys_are_valid_and_distinct` | killed (8 of 8) |
| 17 | `contract` imports `crate::Document`, `crate::document::Document`, `gafira_geom`, `serde_json`, a multi-line `crate::{.., Page}`, or a `crate::Node` path | `contract_imports::adr_0005_contract_imports_only_msg_color_serde_and_std` | killed (6 of 6); a test can state the rule, so the check was added |

Two surviving mutants led to a code change, not a test change. Equivalent mutants found while doing items 2 and 3: with `(min..=max).contains(&default)`, a separate `min <= max` check and a separate `default.is_finite()` check are redundant (an inverted range is empty; a finite range holds no NaN or infinity), so the code does not have them and the mutants "min > max accepted" and "non-finite default accepted" were applied to the range test itself (rows 2 and 3 above). Only the Float bounds need `is_finite`, because `-inf..=inf` would contain every finite default.

### Measured lines

`git diff --shortstat 9faa98e HEAD` (before this section): 9 files changed, 1352 insertions(+). Source 394, tests 958. Forecast 360 (x1.4: 505). **Over ~700: the coordinator decides.** The excess is tests: the table-driven fixtures per fault and per bound that "Acceptance" asks for (`contract_param_schema.rs` 612 lines) and the import checker (174).

### Deviations

- No `Locale`, serde or `Display` test; `Display` text is not tested (not specified).
- `validate_definition` has no explicit `min > max` or default-finite branch, see the equivalent mutants above. Behaviour is the table's.
- `ParamKind::check` and `default_value` are private helpers on `ParamKind` (free by "private helpers are free").
- `include_str!` is used in `contract_imports.rs` because `std::fs::read_to_string` is disallowed by the crate's `clippy.toml`. The list `SOURCES` is explicit.

### Findings for other stages

- Ambiguity, minor, not tested: the design table does not say what `check_values` returns for a `Float` value on an `Int` parameter that is also NaN. The code matches the kind first, so it is `WrongType`; no test fixes it.
- Ambiguity, minor, not tested: duplicate ids in a schema (a bad definition) make `get` return the first and `defaults` the last. `check_values` is defined only for schemas that pass `validate_definition`.
- D2b: add `cancel.rs` to `SOURCES` in `crates/gafira-doc/tests/contract_imports.rs` when it exists, and `pub mod`/`mod cancel;` to `contract/mod.rs`. D2b also edits `mod.rs` and `lib.rs` is already done.
- Spec edit for the specifier (not done here): ARQ-98 lists `ParamSchema` labels among the keys `check-i18n` extracts; `ParamChoice.label` is a second source of keys in the same struct and should be named there.

### D2a fix round 1 (verify round 1: C1, C2, C3, W1, W2, W4, S1, S2)

None of these is RED: the code already met every rule (the report's probes passed on it), so each change closes a test gap. Each was proved by applying the report's surviving mutant, seeing a test fail, and restoring the code. Production code changed by comments only (the `ParamSchema` doc states the precondition of `get`, `defaults`, `check_values` and `resolve`, per C12).

| Mutant | Closed by | Result |
|---|---|---|
| IM2 Int range compared through `f64` | `arq_110_int_values_match_the_rule_over_special_values` (`2^53 + 1`) | killed |
| IM3 Float upper bound `max + EPSILON` | `arq_110_float_values_match_the_rule_over_special_values` (`1.0.next_up()` on `[-1.5, 1]`) | killed |
| IM4 Float upper bound on `\|v\|` | same (negative bounds) | killed |
| IM5 default checked as `\|d\| <= max` | `arq_110_float_definitions_match_the_rule_over_special_values` (`[-2, -1]`, `-1.5`) | killed |
| IM6 option ids compared when adjacent | `arq_110_a_choice_default_must_be_an_option_and_option_ids_must_differ` (`[x, y, x]`) | killed |
| IM7 only the default option id unique | same (`[a, b, b]`, default `a`) | killed |
| IM8 non-finite `Float` before the kind match | `arq_110_a_non_finite_value_of_another_kind_is_a_wrong_type` | killed |
| IM9 non-finite `Color` before the kind match | same | killed |
| IM11b inline `serde_json::Value::Null` | `adr_0005_contract_imports_only_msg_color_serde_and_std` | killed |
| IM12b `use serde_json::Value;` right after an item | same | killed |
| IM13 `use` inside a function (already killed) | same | killed |
| `cancel` declared in `mod.rs`, not in `SOURCES` | `adr_0005_sources_lists_every_module_mod_rs_declares` | killed |

The oracle sweeps (S1) are four tests over 22 float and 10 int special values (NaN, both infinities, `MAX`, `MIN`, signed zero, subnormals, `next_up` and `next_down` around 1 and -1, `2^53 - 1`, `2^53`, `2^53 + 1`), each against the written rule: float and int definitions, float and int values.

Other changes:
- W1: the kind decides first; pinned for NaN, both infinities and a non-finite `Color` of each variant, on every other kind.
- W2: the tests outside the validated domain were rewritten inside it: `arq_98_get_finds_the_definition_with_the_id` (unique ids) and `arq_110_resolve_checks_the_given_values_before_filling_defaults` (default inside its range). `arq_110_new_keeps_the_definitions_in_order_and_validates_nothing` stays: `new` is defined for every schema.
- Pins: `arq_110_resolve_keeps_an_unknown_id_whatever_it_holds` (NaN and both infinities), `arq_110_resolve_on_no_values_is_defaults`.
- W4: the import checker finds every `use` by word, not by statement start, and checks the root of every path written out in code (lowercase roots other than `std`, `serde`, `gafira_color`, `self`, `super`, primitives, declared modules and names a `use` brought in).
- S2: the completeness check compares `SOURCES` with the `mod` lines of `mod.rs`, not with the directory, because `std::fs::read_dir` is disallowed by the crate's `clippy.toml` (ARQ-02). A file `mod.rs` does not declare is not compiled.

Gates on the last code commit, each alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0.

Measured: `git diff --shortstat 9df391b HEAD` before this section: 4 files changed, 337 insertions(+), 48 deletions(-); `params.rs` only comments (8 lines), the rest tests.

## D2b (branch `feat/engine-contracts-d2b`, from `develop` 3cebb86, rules of the checker from de57b41)

Commits: 75eaa41 `CancelToken`, `Cancelled`; 2a4cd74 `Progress`, `NoProgress`, `checkpoint`; dbd6997 `gafira_testkit::progress`. `cancel.rs` is in `SOURCES` of `contract_imports.rs` and `mod cancel;` is in `contract/mod.rs`. Gates on each commit, each alone: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0.

### RED per acceptance item (compile stub, then GREEN)

| Item | RED (failing test, assertion) |
|---|---|
| `CancelToken` clone sees `cancel`; `next_generation`; stale `cancel`; twice | stub `is_cancelled` false, `cancel` no-op, `next_generation` a clone: 9 of 14 failed, e.g. `arq_112_cancel_is_seen_by_the_token_and_by_its_clones`, `arq_112_next_generation_cancels_older_tokens_and_is_live`, `arq_112_cancel_twice_does_not_skip_a_generation` |
| `Send + Sync + Clone`; live token; independent tokens; `Cancelled` message | not RED: the stub already met them (`arq_112_token_is_send_sync_and_clone`, `arq_102_cancelled_is_the_job_key_with_no_arguments`) |
| `checkpoint` | stub always reports, `Ok`: `arq_112_checkpoint_on_a_cancelled_token_is_err_and_reports_nothing`, `arq_112_a_cancel_inside_report_k_leaves_no_report_after_the_next_checkpoint`, `arq_112_no_progress_discards_reports_and_is_usable_as_dyn`; live-token and `Send + Sync` tests not RED |
| `check_progress_contract` | stub `Ok(Ok(()))`: 18 of 20 failed, e.g. `arq_112_decreasing_is_strict`, `arq_112_step_too_large_is_strict_without_tolerance`, `arq_112_reports_after_cancel_allows_one_and_names_extra`; the conforming-fake and `ProgressLog` tests not RED |

Two test bugs were fixed in GREEN, not code bugs: a conforming fake at `n = 2` was held to `max_step = 0.02`, and a step fixture of 0.5 against a limit of 0.25.

### Hand mutants

First run of mutants 2 (`store`) survived: the stale-cancel test cancelled the stale handle before the current one, so the later `fetch_max` restored the value. Fixed by cancelling the stale handle last (`arq_112_a_stale_cancel_leaves_the_new_generation_live`), then killed.

| # | Mutant | Killed by |
|---|---|---|
| 1 | `is_cancelled` with `<` | `arq_112_cancel_is_seen_by_the_token_and_by_its_clones` |
| 1 | only the next generation counts as cancelled | `arq_112_next_generation_cancels_older_tokens_and_is_live` (two generations back) |
| 1 | `is_cancelled` with `>` | **equivalent**: the counter is never below `issued` through the API |
| 2 | `cancel` is `fetch_add(1)` / `store(issued + 1)` | `arq_112_a_stale_cancel_leaves_the_new_generation_live` |
| 3 | `cancel` twice skips a generation | same test |
| 4 | `next_generation` does not advance / new token starts cancelled | `arq_112_a_clone_of_a_new_generation_is_live`, `arq_112_next_generation_cancels_older_tokens_and_is_live` |
| 5 | `Clone` resets `issued` to 0 / copies the atomic | `arq_112_a_clone_of_a_new_generation_is_live`, `arq_112_a_clone_shares_the_flag_in_both_directions` |
| 6 | `checkpoint` reports before checking; reports on `Err`; ignores the token | `arq_112_checkpoint_on_a_cancelled_token_is_err_and_reports_nothing` |
| 7 | `checkpoint` skips the report; reports 0.0 | `arq_112_checkpoint_on_a_live_token_reports_once_and_is_ok` |
| 8 | `check` returns `Ok` when cancelled | `arq_112_cancel_is_seen_by_the_token_and_by_its_clones` |
| 9 | per violation: each check removed (range, decreasing, early, step, not completed, not cancelled/output swapped, reports-after, fewer-than-k, early and scan in a cancelled run, step in a cancelled run); cancel points `1`, `n / 2`, `n - 1` shifted by one, `0` and `n` admitted, no dedup, one point dropped; double cancels at `k + 1`, `k - 1`, `> k`; `Err(E)` and baseline `Ok(true)` ignored | `arq_112_*` in `crates/gafira-testkit/tests/progress.rs`; each negative control has its fake |
| 10 | `<=` for decreasing; early `1.0` allowed or `>= 0.99`; NaN passes; bounds `0`/`1` exclusive, loosened or tightened by one ulp; step `>=`, tolerance, first report against 0 | `arq_112_decreasing_is_strict`, `arq_112_out_of_range_includes_nan_and_is_inclusive_at_0_and_1`, `arq_112_one_before_the_last_report_is_early_completion`, `arq_112_step_too_large_is_strict_without_tolerance`, `arq_112_even_steps_are_measured_in_f32_with_no_tolerance` |
| 11 | `Cancelled` wrong key; an extra argument | `arq_102_cancelled_is_the_job_key_with_no_arguments` |

44 mutants of items 9 and 10, plus 2 of item 11; every one killed except the equivalent `>` of item 1.

### Deviations and judgments

- The `design.md` table has "Fewer than k reports" before the rule "the return value is judged before the log". Implemented: `Ok(false)` goes by the `Ok(false)` row (`NotCancelled` if the log ends at `1.0`, else `OutputAfterCancel`, even with fewer than `k` reports); `Ok(true)` checks fewer than `k` first. Tested for `Ok(true)` only; the `Ok(false)` plus fewer-than-`k` case is not pinned.
- Same index, scan order is range, decreasing, early completion, step, as the table says; pinned by `arq_112_a_scan_fault_comes_before_not_completed_and_in_index_order`.
- The test double `Probe` in `contract_checkpoint.rs` is local: `gafira-doc` tests cannot use `gafira_testkit::progress` for it without a second commit order; it is five lines.

### Measured lines

`git diff --shortstat 3cebb86 HEAD` over code and tests (without specs): source 289 (`cancel.rs` 109, `progress.rs` 177, 3 lines of wiring), tests 756 (`contract_cancel.rs` 175, `contract_checkpoint.rs` 104, `progress.rs` 477). Total about 1 045 against a forecast of 270 (x1.4: 380). **Over ~700: the coordinator decides.** The excess is tests: one fake per violation and per cancel point.
