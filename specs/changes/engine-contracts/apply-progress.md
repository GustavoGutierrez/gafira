## Slice card
Flow: full | level: L3 | pack: new function | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: public contracts shared by all layer-2 engines with design decisions the spec left open; owner chose the parallel full route
Flow-change: design.md line 13 said level L2 -> L3 (trigger: serialized contracts every engine copies; level raised by the coordinator, mode unchanged)
Late record: the coordinator skipped this card before D1. It was written after D1's code, at verify round 1, finding C1. The scope and acceptance it points to were committed in `design.md` at 6bede8b, before D1, and did not change. It precedes D2a, D2b and D3.
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
| `is_valid_key` table (ARQ-107) | `msg::tests::arq_107_is_valid_key_follows_the_grammar` | stub `false`: `should accept "edit.set_property"` |
| Every `MsgArg` and `Unit` builds, `Message` is `Clone` + `PartialEq` | `msg::tests::arq_102_every_arg_variant_and_unit_builds_and_compares` | none: declaration-only types, the test passes as soon as they exist (deviation below) |
| `msg!` with 0, 1, 2 args, order | `tests/msg_macro.rs::arq_102_msg_with_no_arguments_has_the_key_and_no_args`, `..._keeps_argument_names_and_values_in_order` | stub returns key `""` and no args: `left: MessageKey("")`, `right: MessageKey("export.pages_selected")` |
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

- The type-only test for `Unit`, `MsgArg` and `Message` has no RED: declarations cannot fail before they exist. It lives in the commit of `is_valid_key`, whose test has a real RED.
- `msg!` accepts a trailing comma inside a variant's parentheses too (`Quantity(1.5, Unit::Px,)`), as the design's pattern `$(,)?` allows.
- One expectation in the first `inflate` test was wrong (full rectangle inflated by `i32::MIN` is `(0, 0, -1, -1)`); fixed in the test during GREEN, no code change.

### Findings for other stages

None. `git grep Locale -- crates/gafira-doc` finds nothing. Out of scope and untouched: `contract`, `ToMessage` impls, pathops, layout.
