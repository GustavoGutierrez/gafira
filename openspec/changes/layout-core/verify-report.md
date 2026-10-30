# Verify report: layout-core D1

Change `layout-core`, delivery D1, round 1, depth V2, mode full, tier T4 (`mutation: archive`, so no `mutants` report and no mutation finding). Commit verified: `9bff7a0`. Verdict: **FAIL**.

Counts: 2 CRITICAL, 3 WARNING, 1 SUGGESTION. Per class: implementation 5 (2 CRITICAL, 3 WARNING); architecture 0; ambiguity 0; claim 0.

## CRITICAL

**C1. A `grid` without `style` fails to load.** class: `implementation`. Ref: `crates/gafira-doc/src/layout.rs:90`, ADR 0006 rule 4.
ADR 0006 rule 4 says `GridSettings.style` is always written and its `#[serde(default)]` is `Lines`. The field has no attribute, and only a whole missing `grid` gets defaults. Probe `adr_0006_rule_4_grid_without_style_loads_as_lines` gives `Json(Error("missing field `style`"))`. JSON load is critical code. `tests/layout.rs` has no case for it, and the implementer's "Deviations: none" is therefore wrong. No saved file is affected today, but the accepted rule is not implemented.

**C2. The closed-enum rule for `GridStyle` has no test; hand mutant H3 survives.** class: `implementation`. Ref: `crates/gafira-doc/src/layout.rs:72`, ADR 0006 rule 4.
The code is correct today: an unknown style is a load error. But adding a `#[serde(other)]` variant, so that `"style":"cross"` loads silently, leaves `cargo test -p gafira-doc` green. My probe `closed_enums_reject_unknown` fails on the same mutant, so the mutant is not equivalent. A survivor in critical code is CRITICAL.

## WARNING

**W1. Other new nested fields have no read default.** class: `implementation`. Ref: `crates/gafira-doc/src/layout.rs`, 02 §3.5, design.md L1.
A `grid` without `visible`, `step`, `subdivisions` or `snap`, `margins` without `linked`, and a guide without `locked` all fail with "missing field". 02 §3.5 asks for `#[serde(default)]` fields, the L1 table gives these defaults, and the repo puts field-level defaults on flags (`Flip`, `ElementBase.locked`, `Page.locked`). Only `style` is named by an ADR, so this is a WARNING. Required `Length` sides of `Margins`, and guide `axis` and `position`, have no stated default of their own and match `Placement`.

**W2. The old-bytes test does not prove canonical re-save.** class: `implementation`. Ref: `crates/gafira-doc/tests/layout.rs:68`, ADR 0006 Consequences.
ADR 0006 says the fixture must load and re-save in the new canonical form, "a stronger test than the old byte equality". The test only checks that the output contains `"bleed"`. Behaviour is correct: my probe shows `pre_layout/minimal` and `extension` re-save byte-equal to the current goldens, and `full` differs only in the page 1 layout data. Weak test only.

**W3. The history memory estimate ignores guides.** class: `implementation`, follow-up to D2. Ref: `crates/gafira-edit/src/budget.rs:32`.
`page_bytes` counts the heap of `name`, `roots` and `nodes` but not `guides`. Measured with a scratch test: `unshared_bytes` gives 304 for 0 guides and 304 for 10 000. D1 did not touch this file, but `Page` now owns a heap list, and D2's `Property::Guides` makes it reachable by every edit.

## SUGGESTION

**S1. Pin "nothing is clamped" in D2.** The generators stay inside the spec ranges, so a clamp on read would pass them. D2 load tests should include out-of-range values. Today, bleed 99 mm, step 0 and subdivisions 0 load unchanged, as D1 intends (probe); subdivisions 256 is a JSON error.

## Gates

Run on a clean clone at `9bff7a0`, own target dir, `CARGO_INCREMENTAL=0`.

| Gate | Exit | Note |
|---|---|---|
| `cargo fmt --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 892 passed, 0 failed |
| CI policy (ARQ-07, unsafe, V5, V12, V13, V14) | 0 | Steps taken from `ci.yml` |
| `cargo deny --offline check` | 0 | Offline, so the advisory DB was not refreshed |

## Artifact integrity (step 3c)

- `f08d005` touches only `openspec/changes/layout-core/apply-progress.md` (9 insertions).
- `9bff7a0` adds 31 lines to that file and removes none, so the card block is unchanged. No `slice card amend` commit exists.
- There is no `.cargo/mutants.toml` change.
- `pre_layout/{minimal,full,extension}.json` are byte-identical to the `develop` goldens.
- Size: 505 insertions over `develop`, confirmed, against the 460 forecast and the ~700 ceiling. That figure includes the 9 card lines; `9bff7a0` alone is 496, of which 459 are `.rs`. No finding.

## Criteria

| # | Criterion | Status | Proof |
|---|---|---|---|
| 1 | FD-03(b) guides survive save and open | covered | `fd_03_every_guide_field_survives_save_and_open`, `fd_03_guide_order_and_duplicates_survive`, `layout_data_round_trips`; probe: 99 951 values, 0 bit mismatches |
| 2 | FD-03 guide shape: axis, position, lock; unrounded, own unit, off-page allowed | covered | Same tests; `-3.0 in`, `1e9 pt` |
| 3 | FD-05 data: `bleed`, `Margins`, `linked`, defaults | covered | `fd_05_fd_07_defaults_match_the_spec`, `fd_05_documents_without_layout_fields_load_with_defaults`, `fd_05_fd_07_layout_fields_survive_save_and_open` |
| 4 | FD-07 data side: `GridSettings` fields and defaults | covered | Same tests; `grid_style_and_guide_axis_use_snake_case_on_the_wire` |
| 5 | 02 §3.1 fields and order after `size` | covered | `adr_0006_default_valued_layout_fields_are_written`; H1 killed |
| 6 | ADR 0006 rule 1, D-36: schema stays 1, old documents load | covered | `fd_05_documents_without_layout_fields_load_with_defaults`; each field absent alone (probe) |
| 7 | ADR 0006 rule 3: every field written, no `skip_serializing_if` | covered | Same test, `minimal_document_has_exact_bytes`, `page_wire_form_writes_every_field`; `grep` finds none in `crates/` |
| 8 | ADR 0006 rule 4: `style` reads as `Lines` when absent | **failing** | C1 |
| 9 | ADR 0006 rule 4: `GridStyle` closed, unknown variant fails | **missing test** | C2 |
| 10 | ADR 0008: types in `gafira-doc`, exported at the root, `bleed` is `Length` | covered | `lib.rs`; `tests/layout.rs` imports from the root |
| 11 | Design D1: 7 tests and 3 goldens updated, old bytes kept as fixture | covered, weak | Fixture used by `tests/layout.rs`; see W2 |
| 12 | Design D1: testkit generators fill every page | covered | `document()` assigns the layout; `generated_documents_validate`, `round_trip_is_idempotent_and_bit_identical` |
| 13 | L5, Slice card non-goals: no `notes`, `duration`, `background`, no validation | covered | `page.rs` has none; out-of-range probe |

Total 13, covered 11. D-35, D-37 and D-38 have no D1 surface. ADR 0006 rule 2 applies only after the first saving build. FD-03(a, c, d) and the FD-05 and FD-07 ranges and rectangles belong to D2 to D5. Every test in `tests/layout.rs` that proves an ID names it, except `grid_style_and_guide_axis_use_snake_case_on_the_wire`, which names no ID; it is a wire-form test and FD-07 is proved by the named tests.

## Hand mutants

Run on a scratch export, never in the repo.

| Mutant | Rule | Result | Killed by |
|---|---|---|---|
| Spot-check, item 3: `skip_serializing_if` empty on `guides` | ADR 0006 rule 3 | Killed; recorded tests confirmed | `adr_0006_default_valued_layout_fields_are_written`, `minimal_document_has_exact_bytes` and 5 more |
| H1: `guides` before `margins` on the wire | 02 §3.1 order | Killed | `adr_0006_default_valued_layout_fields_are_written`, goldens |
| H2: `left` and `right` margins swapped on read only | FD-05 four sides | Killed | `fd_05_fd_07_layout_fields_survive_save_and_open`, `layout_data_round_trips`, goldens |
| H3: unknown `GridStyle` accepted through `#[serde(other)]` | ADR 0006 rule 4 | **Survived** | None (C2) |

Run 4, killed 3.

## Probes (step 1c)

Written from the specs before reading the implementer's tests. 11 probes: 10 pass, 1 fails (C1). They cover old documents, each field absent alone, nested partial objects, closed enums and malformed values, unknown keys dropped inside the new types, key order and wire shape, exact bits in every unit, order and duplicates, fixture re-save, and out-of-range values kept.

next: tdd-apply
