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
