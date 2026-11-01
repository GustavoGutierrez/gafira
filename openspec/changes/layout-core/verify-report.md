# Verify report: layout-core D2

Change `layout-core`, delivery D2, round 1, depth V2, mode full, tier T4 (`mutation: archive`, so no `mutants` report and no mutation finding).

Current state: round 2 at `5e4e3f2`, **FAIL**. 1 CRITICAL (C4), 1 WARNING (W1, held), 1 SUGGESTION (S2, with the owner). C1, C2, C3 and S1 are closed. See "Round 2" at the end.

## Round 1

Commit verified: `ad865cc`. Verdict: **FAIL**. Counts: 3 CRITICAL, 1 WARNING, 2 SUGGESTION. Per class: implementation 3 (3 CRITICAL); ambiguity 1 (WARNING); architecture 0; claim 0. The sections below keep the round 1 record.

The D1 report is in git history at `b8b600a`; this file now covers D2.

## CRITICAL

**C1. A DPI change can leave the document invalid and impossible to reopen.** class: `implementation`. Ref: `crates/gafira-edit/src/set_property.rs:205` (`check_pages`), ARQ-13, FD-05, 02 §3.1.
`validate()` now depends on the DPI through the bleed, but `apply_dpi` and `revert_dpi` run only the page-size check. Probe: 100 px bleed at 300 DPI (8.47 mm, accepted), then `Property::Dpi(96)`.

- Debug build: `history.rs:310` panics with `left the document invalid ... OutOfRange { field: Bleed, value: 26.458333333333332 }`.
- Release build: the command returns `Ok(Changed)`, `to_json` writes the file, and `from_json` fails with `LoadError::Invalid(PageLayout ...)`. The user's saved document does not open.

`CommandError::InvalidDocument` exists for this case, and `a_dpi_that_pushes_a_page_over_the_limit_is_refused` sets the precedent for page sizes. No test combines the two: `bleed()` in `gafira-testkit` generates only `Unit::Mm`, and `layout_properties_obey_the_command_laws` never issues a DPI command. The fix needs a check of every page's bleed in both DPI paths, a test, and a generator that also yields px bleed.

**C2. Hand mutant H2 survives: validation of only the first page.** class: `implementation`. Ref: `crates/gafira-doc/src/validate.rs:78`, design L1.
With `self.pages.iter().take(1)`, `cargo test -p gafira-doc -p gafira-edit -p gafira-testkit` stays green (398 passed). Every fixture in `layout_validation.rs` has one page, and generated documents are in range. A bad page 2 would load. My probe `second_page_named_and_order_in_p3` fails on the mutant, so it is not equivalent. The code is correct today; the test is missing.

**C3. Hand mutant H1 survives: rule order in P3.** class: `implementation`. Ref: `crates/gafira-doc/src/validate.rs:70`, design L1 ("after page sizes").
Running the layout step before the page sizes leaves all tests green (398 passed). No test has a bad size and a bad layout together, nor a bad layout and a dangling root. The same probe fails on the mutant. The code is correct today.

## WARNING

**W1. No tolerance is stated for a bleed at exactly 10 mm in pixels.** class: `ambiguity`. Ref: `crates/gafira-doc/src/layout.rs:220`, FD-05(d), 02 §4.17.2 row "Sangrado".
Measured over every valid DPI, 1 to 10 000:

| Value | Result |
|---|---|
| `10 mm`, `1 cm`, `10/25.4 in`, `720/25.4 pt`, and `10 mm` converted to in, cm, pt | Accepted; independent of the DPI |
| `10 mm` converted to px by `Length::convert` | Rejected at 550 of 10 000 DPIs; first is 13 DPI, 10.000000000000002 mm |
| Literal `10 * dpi / 25.4` px | Rejected at 528 of 10 000 DPIs |
| 30 common DPIs (72, 96, 150, 300, 600, 1200, 2400 and others) | Literal form accepted at all of them |

Each of the 528 rejected values is above 10 mm in exact arithmetic, by rounding (exact rational check). So is each of 8 167 accepted values, because the conversion rounds both ways. The behaviour is deterministic, and 10.01 mm and -0.01 mm stay rejected in every unit. FD-05(d) and the 02 row say the end is included and name no tolerance for a converted value. Owner or `specifier` decides: keep the exact comparison, or state a tolerance well below 0.01 mm. The same question reaches `bleed_rect` in D3 only if a tolerance is chosen.

## SUGGESTION

**S1. Stale module comment.** `crates/gafira-doc/src/layout.rs:3` still says "nothing here validates", while the file now holds the checks.

**S2. Test redundancy** (see "Size"). About 130 to 150 of the 874 test lines could go without losing a criterion.

## Probe answers

**Do the `Property` checks and `validate()` share one source of truth?** For a value being set, yes. Both call `check_bleed`, `Margins::check`, `GuideSet::check` and `GridSettings::check`; the constants exist once. My probe `bad_values_give_the_same_error_as_validate` compares the two errors for five bad values: equal. The rule does not hold for the DPI, a second input of the bleed check: `check_pages` in `set_property.rs` is a partial copy of the P3 loop. That is C1.

**FD-03(d) covered only by finiteness?** That is correct: the criterion names only NaN and ±∞, and FD-03 lets a guide sit off the page. Both sides are tested: on load, every sign and any position in the set; on command, a bad second guide leaves the page pointer-equal.

## Gates

Run on a clean export of `ad865cc`, own target dir, `CARGO_INCREMENTAL=0`.

| Gate | Exit | Note |
|---|---|---|
| `cargo fmt --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 924 passed, 0 failed |
| CI policy (ARQ-07, unsafe, V5, V12, V13, V14) | 0 | Steps run from `ci.yml`, at the repo HEAD |
| `cargo deny --offline check` | 0 | Offline, so the advisory DB was not refreshed |

## Artifact integrity (step 3c)

- `f08d005` touches only `apply-progress.md`, and the card block at HEAD is identical to that commit.
- `ad865cc` touches only `apply-progress.md`: 34 insertions, none removed.
- No `slice card amend` commit exists. `.cargo/`, `deny.toml`, `Cargo.toml`, `Cargo.lock` and `ci.yml` are unchanged.

## Criteria

| # | Criterion | Status | Proof |
|---|---|---|---|
| 1 | FD-03(d): NaN or ±∞ refused, `Page.guides` unchanged | covered | `arq_110_invalid_layout_values_fail_and_change_nothing`, `arq_110_non_finite_values_are_rejected_in_every_new_field`, `fd_03_a_bad_guide_in_any_position_fails` |
| 2 | FD-03: a finite guide may lie off the page | covered | `fd_03_guide_positions_are_finite_only` |
| 3 | FD-04(c): apply and undo restore the exact set | covered | `fd_04_c_applying_guides_and_undoing_restores_the_exact_previous_set` (snapshot and compacted), `fd_04_c_any_guide_set_reverts_to_the_previous_one` |
| 4 | FD-05(d) load side: bleed ends included, 10.01 mm an error | covered | `fd_05_bleed_bounds_are_inclusive_on_both_ends`; see W1 for px |
| 5 | FD-05: range in mm for any unit at the document DPI | covered | `fd_05_bleed_range_is_in_millimetres_for_any_unit`, `fd_05_bleed_is_checked_in_millimetres_at_the_document_dpi` |
| 6 | FD-05, L1: margins finite and not negative; no area is not a load error | covered | `fd_05_margin_bounds_are_inclusive_on_the_zero_end`, `fd_05_any_negative_margin_is_rejected` |
| 7 | FD-07(e): step 1 to 500 in its own unit | covered | `fd_07_grid_step_bounds_are_inclusive_on_both_ends` |
| 8 | FD-07(e): subdivisions 1 to 10 | covered | `fd_07_subdivision_bounds_are_inclusive_on_both_ends` |
| 9 | L1, D1 S1: nothing is clamped on load | covered | `fd_05_fd_07_out_of_range_values_on_load_are_rejected_not_clamped`, `fd_05_fd_07_range_ends_on_load_are_kept_as_written` |
| 10 | ARQ-110: typed error naming the field | covered | The `arq_110_*` tests; my probe adds each sign in each field |
| 11 | Design D2, S6: four `Property` variants with ranges and property tests | covered | `layout_properties_obey_the_command_laws`, `fd_05_every_layout_property_is_set_and_restored` |
| 12 | D1 W3: history estimate counts guides | covered | `fd_04_the_history_estimate_grows_with_the_guide_count`; probe: 240 610 bytes for 10 000 guides, 634 for 1 |
| 13 | L1: layout step after page sizes | **missing test** | C3 |
| 14 | L1: every page is validated | **missing test** | C2 |
| 15 | ARQ-13: a command leaves a valid document, DPI included | **failing** | C1 |

Total 15, covered 12. D1's W3 and S1 are closed.

Product IDs of the card left to later deliveries: FD-03(a, c), FD-04(a, d, e, f), FD-05(b, e, f and the rectangle part of d), FD-06, FD-07(a, c, d and the zoom part of e), FD-09(d), FD-11, ARQ-111, ARQ-114. FD-03(b) was D1. Out of this change per 02 §4.17.2: FD-04(b), FD-05(a, c), FD-07(b), FD-09(a, b, c). Each proving test names its ID; `layout_properties_have_distinct_merge_keys` and `layout_checks_run_in_a_fixed_order` name none, and no criterion rests on them alone.

## Hand mutants

Run on a scratch export, never in the repo. `apply-progress.md` records all six items killed.

| Mutant | Rule | Result | Killed by |
|---|---|---|---|
| Spot-check, item 5: previous guides captured reversed | FD-04(c) | Killed; recorded tests confirmed | 7 tests, including the `fd_04_c_*` tests and `layout_properties_obey_the_command_laws` |
| H1: layout step before page sizes | L1 order | **Survived** | None (C3) |
| H2: layout checked on the first page only | L1, all pages | **Survived** | None (C2) |
| H3: check after the write in `apply_page` | FD-03(d), `Page.guides` unchanged | Killed | `arq_110_invalid_layout_values_fail_and_change_nothing`, `layout_properties_obey_the_command_laws` |

Run 4, killed 2.

## Probes (step 1c)

Written from the specs before reading the implementer's tests. 15 probes: 14 pass, 1 fails (C1). `second_page_named_and_order_in_p3` passes on the real code and is the probe that fails on H1 and H2. Covered: the 10 mm boundary in each unit at every DPI, each non-finite value in each field, page order in P3, own-unit ranges, margins with no area, load errors, undo and redo in both history modes, gestures with a refused step, transactions, scope misuse, error equality with `validate()`, the estimate, and the DPI change.

Edges seen, no finding: `-0.0` bleed passes; `-1e-320 px` at 10 000 DPI underflows to `-0.0` mm and passes; `1e308 in` gives `NonFinite` (ARQ-110, intermediate result), `1e308 mm` gives `OutOfRange`; subdivisions 256 is a JSON error.

## Size

`git diff --shortstat develop`: 1 157 insertions, 19 deletions, confirmed. Measured with `--numstat`: 249 in `src/` (about 10 of them test lines inside `error.rs`), 874 in the two test files, 34 notes. Forecast 420. Reported for the owner's decision; no finding.

Redundancy, an estimate from reading the tests (no reduced suite was run): about 130 to 150 test lines could go.

| File | Candidate | Lines |
|---|---|---|
| `layout_commands.rs` | `arq_110_invalid_layout_values_fail_and_change_nothing`: one good and one bad row per property, as the leaf ranges are proved in `gafira-doc` | ~30 |
| `layout_commands.rs` | `fd_05_fd_07_range_ends_are_accepted`: same reason | ~12 |
| `layout_commands.rs` | `fd_05_every_layout_property_is_set_and_restored`: the four `expect` blocks become one page comparison | ~12 |
| `layout_commands.rs` | `fd_04_c_revert_gives_back_the_set_the_last_apply_replaced` and `fd_04_a_guide_drag_merges_and_reverts_to_the_origin`: overlap the law test and the guide-set property | ~35 |
| `layout_validation.rs` | Merge the two bleed tables and the inches test; fold the first-bad-side test into `layout_checks_run_in_a_fixed_order`; a helper for the load rows | ~40 |

That does not reach ~700: about 1 000 would remain, before the lines that C1 to C3 add. The main weight is source plus table-style tests across two layers, not duplication alone.

## Round 2 (closure)

Commit verified: `5e4e3f2`. Verdict: **FAIL**. Counts: 1 CRITICAL, 1 WARNING, 1 SUGGESTION. Per class: implementation 1 (CRITICAL); ambiguity 1 (WARNING); architecture 0; claim 0. No new probes or independent mutants; only H1, H2 and the C1 probe were re-run.

### CRITICAL

**C4. The fix commit deleted 9 tracked regression-seed files from other crates.** class: `implementation`. Ref: `c1551a2`.
`c1551a2` removes every `*.proptest-regressions` file in the repo: `gafira-anim` (`color`, `easing`, `interpolate`, `sampling`), `gafira-pathops` (`prop_boolean`), `gafira-time` (`flicks`, `ranges`, `rates`, `timecode`). That is 71 lines and 17 pinned cases. Neither the commit message nor `apply-progress.md` mentions it, and those crates are outside D2. Several seeds arrived with real fixes (`0a2f336` bezier solver, `1c56418` Color alpha, `9c0d95d` boolean fill rule), and proptest replays them first on every run. After the merge, a return of those bugs would only be caught by chance.

Nothing is broken today: with the 9 files restored on a scratch export, `gafira-anim`, `gafira-time` and `gafira-pathops` pass (362 passed, 0 failed). The cause is not proved; earlier changes record deleting `proptest-regressions` files generated by mutation runs, so a cleanup that also caught tracked files is likely.

Why CRITICAL and not the follow-up WARNING for a new test gap: that rule covers a gap found in the code under review. This is an undeclared removal of existing protection, outside the delivery, that merging would make permanent. The fix is one commit: `git checkout develop -- <the 9 files>`, with no change to the rest.

### Earlier findings

| Finding | Status | Evidence |
|---|---|---|
| C1 | Closed | Same probe, debug and release: `Err(InvalidDocument(PageLayout ... 26.458333333333332))`, DPI stays 300, `validate` is `Ok`, reload is `Ok`. `check_pages` now runs all sizes, then `Page::check_layout` on all pages, and serves both `apply_dpi` and `revert_dpi`. `arq_13_*` tests pin it |
| C2 | Closed | H2 re-applied: `fd_05_fd_07_a_bad_value_on_a_later_page_is_found` and `fd_05_a_later_page_with_a_bad_value_is_rejected_on_load` fail |
| C3 | Closed | H1 re-applied: `design_l1_page_sizes_are_reported_before_layout_data` fails |
| S1 | Closed | Module comment of `layout.rs` corrected |
| W1 | Open, held | class: `ambiguity`. Exact comparison unchanged; held for the owner or `specifier` by the coordinator. It does not need to block: no common DPI is affected and the behaviour is deterministic |
| S2 | Open | Owner's size question |

Note on C1: `bleed()` in `gafira-testkit` still generates only mm, so the DPI property test never meets a px bleed. The explicit `arq_13_*` tests and the implementer's mutants prove the fix, so no finding.

Criteria: 15 of 15 covered (13, 14 and 15 closed by C3, C2 and C1).

Gates, on a clean export of `5e4e3f2`: fmt 0, clippy 0, test 0 (931 passed, 0 failed), `cargo deny --offline check` 0. CI policy 0 (ARQ-07, unsafe, V5, V12, V13, V14), run at the repo HEAD.

### Artifact integrity

- The card block at HEAD is identical to `f08d005`.
- `5e4e3f2` touches only `apply-progress.md`: 9 insertions, none removed.
- `fb58135` touches only `verify-report.md` and `verification.json`.
- No change to `.cargo/`, `deny.toml`, `Cargo.toml`, `Cargo.lock`, `ci.yml` or `docs/`.

### The 16 files of `git diff --shortstat ad865cc`

| Files | Count | Expected |
|---|---|---|
| `layout.rs`, `set_property.rs`, `layout_validation.rs`, `layout_commands.rs` | 4 | Yes |
| `apply-progress.md` | 1 | Yes |
| `verify-report.md`, `verification.json` | 2 | Yes |
| `*.proptest-regressions` deletions | 9 | **No** (C4) |

### Size

`git diff --numstat develop 5e4e3f2`, insertions:

| Part | Lines |
|---|---|
| Code (`src/`, about 10 of them test lines in `error.rs`) | 267 |
| Tests | 1 106 |
| Notes (`apply-progress.md`) | 43 |
| Verify artifacts | 89 |
| Total | 1 505 |

Without the verify artifacts: 1 416 insertions. There are 199 deletions, 71 of them the seed files. Forecast 420. The fix round added 232 test lines and 18 code lines. Reported for the owner's decision; no finding.

### Incident during this round

A guard-less script of mine failed to `cd` into its scratch export and ran in the repo. It wrote hand mutants H1 and H2 to the tracked file `crates/gafira-doc/src/validate.rs` and ran tests there. I saw it in `git status`, ran `git restore` on that one file, and confirmed its hash equals `HEAD` (`09893d3`). No other file changed, no seed file or probe stayed behind, and nothing was committed. All results of that run were discarded; every figure above comes from a re-run on clean exports.

next: tdd-apply (implementation); W1 (ambiguity) held for the owner or `specifier`
