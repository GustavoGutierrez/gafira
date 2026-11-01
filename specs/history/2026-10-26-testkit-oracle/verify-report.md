# testkit-oracle: verify report

- Change: `testkit-oracle` (whole change, one delivery, no delivery ids)
- Branch: `spec/testkit-oracle`, verified commit `97c85e1`, base `main` at `aea45a7`
- Mode: standard. Pack: test-only hardening. Tier: T4 (crate run by a separate `mutants` agent).
- Depth: V3. The `Flow:` line carries no tier or verify field; the coordinator prompt said "V2/V3", so V3 was run.
- Round: 1
- Verdict: **PASS** (0 CRITICAL). Condition W1 is met: the independent T4 run confirms the implementer's counts (see "Mutation (T4)")

Spec edits after the first test commit (`db8d06f`, `4e14161`, `97c85e1`) are coordinator-authored and ratified by the process (prompt); they are not an implementer finding. No implementer commit touched the delta spec.

## Scope (test-only)

- `git diff --name-only main...HEAD`: `crates/gafira-testkit/tests/corruption.rs`, `crates/gafira-testkit/tests/generator.rs`, `openspec/changes/testkit-oracle/apply-progress.md`, `openspec/changes/testkit-oracle/specs/testkit/spec.md`. Both test files are new.
- `git diff main..HEAD -- crates ':!crates/gafira-testkit/tests'`: empty (all `src/`, `Cargo.toml`, `Cargo.lock` byte-identical; the `#[cfg(test)] mod guards` block unchanged).
- `.cargo/mutants.toml`: no diff. `mutants::skip`: no hit in the tree. No `exclude_re` added. CI config untouched.
- Per commit (`git show --stat`): the three `docs(spec)` commits touch only the delta spec, `484cc36` only `tests/generator.rs`, `119305b` only `tests/corruption.rs`, `355bd01` only `apply-progress.md`. No AI attribution in any message.
- New tests use only the public API of `gafira-testkit`, `gafira-doc`, `gafira-geom`, `gafira-color`, `serde_json` and `proptest`.

## Gates

Clean `git archive` exports in the session scratchpad, files touched, one `CARGO_TARGET_DIR` per export, `CARGO_INCREMENTAL=0`, `PROPTEST_CASES` unset. Disk before: 84 GB free.

| Tree | fmt | clippy | test | deny |
|---|---|---|---|---|
| `484cc36` (first test commit) | 0 | 0 | 0 | 0 |
| `97c85e1` (HEAD; crates identical to `119305b`) | 0 | 0 | 0 (779 passed, 0 failed) | 0 |

The docs-only commits (`cacdbce`, `db8d06f`, `4e14161` on top of `main`; `355bd01`, `97c85e1` on top of `119305b`) carry the same `crates/` tree as `main` or `119305b`, so the two exports cover every distinct code tree. `git diff --stat 119305b HEAD -- crates Cargo.toml Cargo.lock` is empty.

CI `policy` job steps, extracted from `.github/workflows/ci.yml` and run on the worktree at HEAD: Format, ARQ-07, forbid unsafe, V5, V12, V13, V14: all exit 0. Negative run: one misformatted line appended to `tests/generator.rs` in a scratch export makes `cargo fmt --check` exit 1 naming that file; restored, exit 0. Other policy negatives were not run: no CI, policy or layer surface changed.

## PROPTEST_CASES

`cargo test -p gafira-testkit --locked --test generator --test corruption` on the HEAD export:

- unset: 7 + 5 passed, exit 0
- `PROPTEST_CASES=1`: 7 + 5 passed, exit 0

Neither file uses `proptest!`; every oracle draws through `TestRunner::deterministic()` and `new_tree`, with constant draw counts (`COLOR_DRAWS = 2 000`, `SIDE_DRAWS = 20 000`, picks `0..1000`, empty document plus 2 or 4 generated). `new_tree` does not read `cases`, so the draw count is the same in both settings. `tests/harness.rs` is out of the scenario (C6).

## Independent probes (V2, written before reading the implementer's tests)

Scratch-only file in a HEAD export, public API only, `TestRunner::deterministic()`:

- Pick table over picks `0..1000` on the empty document and 3 generated ones, locating the injected node by id set difference: next-id offset `pick % 3`; page-size value `[..][pick % 3]` with both units `Px` and side by `(pick / 3)` parity; `NonFinite` field `pick % 7`, value class `(pick / 7) % 3`, exactly one non-finite field; skew `[..][pick % 5]`; extension payload depth `65 + pick % 3` and `ExtensionTooDeep` reported. Pass: the table in the spec states current behaviour.
- Guards: every `Corruption::ALL` pair whose observed errors differ in variant is rejected by both `explained_by` and `matches`. Pass.
- 2 000 documents: all three colour spaces, a negative value in each of r, g, b, and px band counts `[286, 271, 296, 298, 323]` (lower, upper, near lower, near upper, random), all at least 10. Pass.

Comparison with the implementer's tests: each probe has a matching or stronger implementer test (the guard test also checks the accepting side and duplicate ids; the exact-limit test requires 10 per limit instead of 1). One gap is S2.

## Hand mutants (5, on a scratch export of HEAD, restored after each)

| # | Baseline mutant | Edit in `crates/gafira-testkit/src/lib.rs` | Failing test | Result |
|---|---|---|---|---|
| H1 | 5 (colour) | delete arm `1 => Color::LinearSrgb(..)` (line 64) | `generator::every_colour_space_is_generated` | killed |
| H2 | 16 (page size) | `unit == Unit::Px` to `!=` (line 238) | `generator::px_pages_reach_both_exact_limits` | killed |
| H3 | 18 (guard) | `Corrupted::explained_by` body to `true` (line 458) | `corruption::a_guard_rejects_the_error_of_every_other_class`, `corruption::page_size_kinds_share_an_error_and_duplicate_id_kinds_share_it_by_id` | killed |
| H4 | 23 (sub-case, C3 exact mapping) | `(pick / 3)` to `(pick * 3)` (line 585) | `corruption::page_size_value_and_side_follow_the_pick_table` | killed |
| H5 | 38 (sub-case, C5) | `65 + pick % 3` to `65 + pick + 3` (line 652) | `corruption::extension_payload_depth_follows_pick_modulo_3` | killed |

5 run, 5 killed. These also give per-test RED attribution for five of the 36 targets.

## Mutation (T4)

Independent T4 run by a separate `mutants` agent, on fresh builds (confirmed): 143 mutants, 118 caught, 23 unviable, 2 missed, 0 timeouts, about 885 s wall. The counts match the implementer's final T4 in `apply-progress.md` (6 m 12 s there; wall time is not a gate). Missed: `lib.rs:235:18` (mutant 13, C2) and `harness.rs:84:9` (mutant 36), the two accepted equivalents; the same two lines as the implementer's `missed.txt`. Condition W1 is resolved by this run. `gafira-testkit` is test support code (non-critical under AGENTS.md), so no missed mutant is critical. No exclusion was added.

## Requirement and scenario matrix (22 scenarios)

| # | Requirement / scenario | Evidence | Status |
|---|---|---|---|
| 1 | Test-only scope / Source untouched (command) | `git diff --name-only main...HEAD` (Scope) | covered |
| 2 | Test-only scope / Gates per commit (command) | per-tree gates above | covered |
| 3 | Test-only scope / Oracles ignore PROPTEST_CASES (command) | two runs above, constant draws | covered |
| 4 | Test-only scope / No exclusion added (command) | `.cargo/mutants.toml` no diff, no `mutants::skip` | covered |
| 5 | Baseline / Baseline reproduced (command) | `apply-progress.md` Step 0: 143 / 82 / 23 / 38 / 0, reported to the coordinator, spec extended (C5) | covered (record) |
| 6 | Colour / Every colour space is generated (unit) | `generator.rs::every_colour_space_is_generated`; H1 | covered |
| 7 | Colour / Negative channels are generated (unit) | `generator.rs::negative_channels_are_generated`; T4 (mutants 1 to 3) | covered |
| 8 | Px bands / Exact limits on px pages (unit) | `generator.rs::px_pages_reach_both_exact_limits` (filters on `Unit::Px`); H2 | covered |
| 9 | Px bands / Bands near the limits (unit) | `generator.rs::px_pages_reach_the_bands_next_to_the_limits`, 20 000 draws, min 10 | covered |
| 10 | Px bands / Random band (unit) | `generator.rs::px_pages_reach_the_random_band` (excludes 100.0) | covered |
| 11 | Px bands / Shifted random band is an accepted equivalent (manual) | no value-pinning test; mutant 13 in the final missed list (C2) | covered |
| 12 | Guards / A foreign error is rejected (property) | `corruption.rs::a_guard_rejects_the_error_of_every_other_class` (deterministic docs x picks, both functions); H3 | covered |
| 13 | Guards / Shared variants (unit) | `corruption.rs::page_size_kinds_share_an_error_and_duplicate_id_kinds_share_it_by_id` | covered |
| 14 | Pick table / Next-id offset (unit) | `corruption.rs::next_id_offset_follows_pick_modulo_3` (reference: corrupted `next_id`, empty doc included) | covered |
| 15 | Pick table / Page-size value and side (unit) | `corruption.rs::page_size_value_and_side_follow_the_pick_table`; H4 | covered (S2) |
| 16 | Pick table / Non-finite field and value (unit) | `corruption.rs::non_finite_field_and_value_follow_the_pick_table` (21 pairs, other fields finite) | covered |
| 17 | Pick table / Skew values (unit) | `corruption.rs::skew_values_follow_the_pick_table` | covered |
| 18 | Pick table / Extension payload depth (unit) | `corruption.rs::extension_payload_depth_follows_pick_modulo_3`; H5 | covered |
| 19 | Pick table / Periodicity across the drawn range (property) | every pick-table test iterates `0..1000` on the empty doc plus generated docs | covered |
| 20 | Mutation run constraints / Pre-run disk check (manual) | process rule of the `mutants` agent; no `df` output recorded in `apply-progress.md` | covered (process) |
| 21 | Acceptance T4 / Only the two equivalents survive (command) | independent T4 run by the `mutants` agent (143 / 118 / 23 / 2 / 0, same two missed lines); implementer final T4 in `apply-progress.md` | covered (W1 confirmed) |
| 22 | Acceptance T4 / Each oracle has RED evidence (manual) | `apply-progress.md` "Done" table names the mutants per test; evidence is Step 0 missed vs final T4 caught (the only change is the new tests), plus H1 to H5 here | covered |

Edge cases: empty document is in every pick-table run; mutant 16 filter on unit is present; `NonFinite` on opacity is read through `f64::from(f32)`; run time recorded (6 m 12 s against 5 m 10 s baseline and 10 m 17 s archived).

## Findings

### CRITICAL

None.

### WARNING

- **W1** (resolved) class: claim. ref: spec "Acceptance: T4 on gafira-testkit" / scenario 21. Resolved before archive: the independent T4 run (separate `mutants` agent) shows 143 / 118 / 23 / 2 / 0 and the same two missed lines as the implementer's run.
- **W2** class: implementation. ref: AGENTS.md "Delivery rules". `git diff --shortstat main...HEAD`: 970 insertions, over the ~700 ceiling. 446 are the two test files; 524 are the delta spec and `apply-progress.md`, written mostly by the coordinator and specifier. No forecast is recorded. Signal for the coordinator to note or get the owner's OK; no code action.
- **W3** class: ambiguity. ref: `apply-progress.md` line 1. The `Flow:` line lacks the tier and verify-depth fields and says "coordinator to confirm". Depth was taken from the prompt (V3 run).
- **W4** class: implementation. ref: `lib.rs:235:18`, `harness.rs:84:9`. Two missed mutants remain, both accepted equivalents (C2 and the archived `edit-history` decision); non-critical, documented, no action under this change.

### SUGGESTION

- **S1** `apply-progress.md` "Deviations and findings", first bullet, still says the scenario requires the whole package to pass with `PROPTEST_CASES=1`; `97c85e1` scoped it (C6). Update the note at archive.
- **S2** `page_size_value_and_side_follow_the_pick_table` compares `value()` only, not that both sides are `Unit::Px` ("100 px" in the table). The probe confirms they are; asserting the unit would close it.
- **S3** RED attribution per test relies on the T4 delta; a per-test attribution (T3 with `--test`, or hand mutants as here) would make scenario 22 direct for all 36.

## Counts

- CRITICAL 0, WARNING 4, SUGGESTION 3
- By class: claim 1, implementation 2, ambiguity 1, architecture 0
- Criteria: 22 scenarios, 22 covered (W1 resolved by the independent T4 run)
- Hand mutants: 5 run, 5 killed

Scratch: exports, probe and hand-mutant logs in the session scratchpad `vto` directory; build folders freed with `cargo clean --target-dir`.

next: archive (W1 confirmed by the independent T4 run)
