# Verify report: edit-history (C3)

## D1: crate skeleton, `msg`, `Command` contract, `Transaction`

Branch `feat/edit-history-1`, base `origin/main` c67d068, range `origin/main..bcee81d`. Verified on 2026-10-24 from separate clones under the session scratchpad, one per commit, each with its own `CARGO_TARGET_DIR`. The main checkout was not touched. CI steps were pulled from that commit's own `ci.yml` (PyYAML) and run under `bash --noprofile --norc -eo pipefail`.

**Delivery verdict: PASS** (0 CRITICAL, 7 WARNING, 4 SUGGESTION). **Change verdict: not final.** D2 to D10 remain, so `next` is `tdd-apply` for D2.

### Gates per commit

| Commit | fmt | clippy `-D warnings --locked` | tests passed (all targets plus doctests) | `cargo deny check` | ARQ-07 | V11 | V5 | V12 | V13 |
|---|---|---|---|---|---|---|---|---|---|
| ab06f6e skeleton + V5 row | 0 | 0 | 313 (0 failed) | ok | 0 | 0 | 0 | 0 | 0 |
| f5785a1 `msg` | 0 | 0 | 314 | ok | 0 | 0 | 0 | 0 | 0 |
| 563ba35 `Command` contract | 0 | 0 | 319 | ok | 0 | 0 | 0 | 0 | 0 |
| d83e0f5 `Transaction` | 0 | 0 | 324 | ok | 0 | 0 | 0 | 0 | 0 |
| bcee81d apply-progress (docs only) | 0 | 0 | 324 | ok | 0 | 0 | 0 | 0 | 0 |

`deny` printed `advisories ok, bans ok, licenses ok, sources ok` on every commit. Doc-test sections ran for `gafira_color`, `gafira_doc` (10), `gafira_edit` (0) and `gafira_geom`. The test totals match `apply-progress.md`. Size: `git diff --shortstat origin/main..HEAD -- crates .github` gives 637 insertions, or 708 with the docs. That is under the ~700-line split limit for code, and ×1.12 over the ~570 forecast, so below the ×1.3 re-cut trigger.

### Negative and command runs (scratch clone at bcee81d, all reverted, `git status` clean afterwards)

| Run | Result |
|---|---|
| Dependency set: `cargo tree -p gafira-edit -e normal,build --target all --prefix none -f '{p}'`, filtered to `gafira-*` | {`gafira-edit`, `gafira-doc`, `gafira-geom`, `gafira-color`} |
| Dev-dependencies are not layer edges: `proptest` in that tree | 0 lines |
| V5 clean with xtrace | rc 0; trace shows `cargo tree -p gafira-edit` |
| E-N1: row cut to `gafira-edit gafira-geom gafira-color` | rc 1, `gafira-edit must not depend on gafira-doc` |
| E-N2: row deleted | rc 1, `gafira-edit has clippy.toml but no V5 row` |
| Forced edge: `gafira-cli` path dep added to `gafira-edit` | rc 1, `gafira-edit must not depend on gafira-cli` |
| Forbidden third-party crate: `serde` added to the pattern | rc 1, `forbidden serde v1.0.229` (first hit is `gafira-geom`) |
| Clock: scratch `std::time::Instant::now()` in `crates/gafira-edit/tests/` | clippy rc 101, `disallowed_methods`, `note: ARQ-02` |
| `HashMap` and `HashSet` in a `gafira-edit` test | clippy rc 101, `disallowed_types`, both ARQ-10 notes, no "does not refer to an existing" warning |
| `std::thread::spawn` in a `gafira-edit` test | clippy rc 101, `disallowed_methods`, ARQ-02 |
| Copies identical: `diff -q` of the geom copy against color, doc and edit | all identical; each of the 4 copies has both `Hash*` entries |
| V11: forbid removed from `crates/gafira-edit/src/lib.rs` | rc 1, `missing forbid(unsafe_code): crates/gafira-edit/src/lib.rs` |
| Workspace membership | `gafira-edit 2024 1.93 []`; root `Cargo.toml:16` declares it by path |
| No serde on `MessageKey` | `grep -c 'Serialize\|Deserialize' crates/gafira-doc/src/msg.rs` = 0 |

### Requirement and scenario coverage (D1)

| Requirement / scenario | Status | Evidence |
|---|---|---|
| Crate purity: forbid, canonical `clippy.toml`, dependencies, no clock, threads, env, `Hash*` or mutable statics | covered | `crates/gafira-edit/src/lib.rs:7`; runs above; grep finds no `static`, `#[allow]`, `thread::`, `Instant` or `Hash*` in the crate sources |
| Scenario: Dependency set (command) | covered | run above, V5 rc 0 |
| Scenario: Clock call rejected (integration) | covered | run above |
| Command contract: trait shape and defaults | covered | `crates/gafira-edit/src/command.rs:46-72`, matches 02 §4.3 and Q1 |
| `Applied` has exactly `Changed` and `Unchanged` | covered | `command.rs:15-20` |
| `DirtySet`: two `BTreeSet`s plus union | covered | `crates/gafira-edit/src/dirty.rs:12-26`. The page/node semantics belong to concrete commands and law 4 (D2+) |
| `MergeKey` is `Clone + Eq + Debug` with kind, targets and optional `GestureId` | covered | `command.rs:30-40` (see item W2) |
| `CommandError`: `non_exhaustive`, Clone, PartialEq, Display, Error, all 9 cases | covered | `crates/gafira-edit/src/error.rs:10-31, 33-53`; bound check `error.rs:62` |
| Scenario: Object safety and bounds | covered | `command.rs:132` (no thread) |
| Scenario: Default merge returns next unchanged | covered | `command.rs:140` (pointer-equal via `ptr::addr_eq`) |
| Scenario: Merge can recover the concrete type (probe first) | covered | `command.rs:115-126` (test `Sum`), test `command.rs:149` |
| Scenario: DirtySet union is ordered and deduplicated | covered | `dirty.rs:40` (shares page 3 and node 5) |
| Failed commands change nothing (D1 part, through `Transaction`) | covered for atomicity | `transaction.rs:166`; reusability after failure not tested (W4) |
| Transaction: ordered children, label set at construction, rollback i−1..0, error of i | covered | `transaction.rs:21-49`; test `transaction.rs:166` (doc equals before, log `revert b, revert a`) |
| Scenario: All or nothing | covered | `transaction.rs:166` |
| Scenario: Reverse-order revert | covered | `transaction.rs:189` |
| Scenario: Touches is the union | covered | `transaction.rs:209` |
| Changed iff a child changed; empty is Unchanged | covered | `transaction.rs:225` (5-row table, row 1 empty) |
| `merge_key` is None | covered | `transaction.rs:247` |
| Message key for command labels (`msg`) | covered | `crates/gafira-doc/src/msg.rs:7-11`, identical to 02 §14.2 lines 1585-1588; no text, no `Locale` |
| Scenario: Label is a key | covered | `msg.rs:18` |
| Scenario: No serde on MessageKey | covered | grep above |
| Workspace tooling: `clippy.toml` copy plus V5 row in the skeleton commit | covered | `git show --stat ab06f6e` has `clippy.toml`, the `ci.yml` row and the manifest together |
| V5 table: `gafira-edit` row; E-N1, E-N2; dev-dependencies outside | covered | runs above; `ci.yml:83` |
| V11/V12 coverage of `gafira-edit`; copies identical; HashMap/HashSet scenario for `gafira-edit` | covered | runs above (the `gafira-testkit` parts are D2) |
| V13 (D1 part): stays green with `crates/gafira-edit/` present | covered | V13 rc 0 on every commit; the D5 parts are out of scope |
| Each commit stays green | covered | table above |

### Mutation

`mutants` report (`scratchpad/mutants-C3-D1/report.md`): 8 mutants, 4 caught, 1 missed, 0 timeout, 3 unviable.
- Missed: `error.rs:35`, `Display for CommandError::fmt -> Ok(Default::default())`. Non-critical: the spec does not fix the message text. Documented as W1.
- Unviable: `merge_key -> Some(Default::default())`, `Transaction::label -> Default::default()` and `Transaction::apply -> Ok(Default::default())` (no `Default` impl).

Hand mutations, for rules that cargo-mutants cannot express (5, all KILLED):

| # | Mutation | Killed by |
|---|---|---|
| H1 | Rollback in forward order (`.rev()` dropped, `transaction.rs:38`) | `a_failing_child_rolls_back_the_earlier_ones` |
| H2 | Rollback also reverts the failing child (`[..=i]`) | same |
| H3 | `revert` in forward order (`transaction.rs:52`) | `revert_undoes_the_children_in_reverse_order` |
| H4 | `touches` keeps only the last child (`transaction.rs:61`) | `touches_is_the_union_of_the_children` |
| H5 | `DirtySet::union` drops the other set's nodes (`dirty.rs:23`) | `union_is_ordered_and_deduplicated`, `touches_is_the_union_of_the_children` |

The `Changed` arm of `Transaction::apply` and the probe-first merge are covered by the non-vacuity runs in `apply-progress.md`, so they were not repeated.

### CRITICAL

None.

### WARNING

1. **Missed mutant, Display text** (`crates/gafira-edit/src/error.rs:33-53`). This is non-critical by policy because the spec fixes no text. No test reads the rendered message. Accept as is. Do not add a text-pinning test unless a spec fixes the wording.
2. **Open shapes in the spec** (`command.rs:24`, `command.rs:30-40`, `error.rs:24`, `error.rs:20`). The spec fixes no field layout, so `MergeKey { kind: &'static str, page, nodes, gesture }`, `GestureId(pub u64)`, `InvalidValue(&'static str)` and `IndexOutOfRange { index, len }` all meet the D1 text. This is acceptable for D1 and is not a spec defect. Later deliveries must still honour the following:
   - D3: `SetProperty` must fold the property into `kind` (for example `"set_property.placement"`). Otherwise two properties on the same nodes would merge.
   - D6: `GestureId` has a public field, so any caller can forge an id equal to the open gesture's. `begin_gesture` returning a "fresh" id stays true for `History`'s counter, but nothing stops forgery. D6 should decide whether the field stays public.
   - D10: the document-DPI target can use `page: None, nodes: []`. This needs no change now.
3. **`Transaction::apply` discards rollback errors** (`transaction.rs:38-43`). The spec requires returning child *i*'s error and the prior state. If a child's revert fails, the prior state is not restored, and the caller only sees *i*'s error. That breaks a child contract (a cleanly applied command must revert). Silently swallowing it is defensible against the spec text. `Transaction::revert` (`transaction.rs:52-54`) stops at the first error and leaves a partially reverted document. History (D4) keeps snapshots, which limits the damage. The rollback should be considered for D4, which could surface the first revert error or record it.
4. **Reusability after a failed apply is not tested for `Transaction`** ("Failed commands change nothing": "the command MUST stay reusable"). `a_failing_child_rolls_back_the_earlier_ones` (`transaction.rs:166`) checks atomicity only. Re-applying works if the children are reusable, but no test shows it (for example, a child that fails once and then succeeds). The D2 harness (law 2 / law 5) will cover it generically. Note it there.
5. **Low-value tests (test-scope rule)**:
   - `it_has_its_own_label_and_never_merges` (`transaction.rs:247`) is half an accessor test (`label()` returns what was passed). The `merge_key() == None` half is spec-required.
   - In `an_error_is_cloneable_comparable_and_an_error` (`error.rs:62`), the runtime asserts `clone == self` and `!= IdSpaceExhausted` re-test derived impls. The compile-time `assert_contract` bound is the spec evidence.
   - `a_boxed_command_is_send_and_sync` (`command.rs:132`) also asserts the default `merge_key()`, which is unrelated to its scenario.
   None of these is harmful.
6. **V12 count left at `>= 3`** (`ci.yml:119`). The binding spec ("Policy checks cover every domain crate", item 3) still says "fewer than three". No requirement or scenario in this change raises it, so leaving the body unchanged is correct. However, `proposal.md:82` lists "V12 count" as a D1 item, which is misleading. With four copies, deleting one copy (for example `crates/gafira-edit/clippy.toml`) passes every check. V12 still counts 3, and V5 still runs the `gafira-edit` row, because its coverage loop iterates the copies, not the rows. The crate would then lose its clippy bans (ARQ-02, ARQ-10) with CI green. This was measured on a scratch clone: with the copy removed, V5 rc 0, V12 rc 0, and clippy passes on an `Instant::now()` test. Spec gap, not a D1 defect. See S1.
7. **`proptest` dev-dependency unused until D3** (`crates/gafira-edit/Cargo.toml:15`). Neither V5 (dev-dependencies are outside `-e normal,build`, as confirmed by the run above) nor `cargo deny` (bans and licenses ok) objects. `Cargo.lock` gained only the `gafira-edit` package. Acceptable, but it is a declared-but-unused dependency for 1 or 2 deliveries.

### SUGGESTION

1. Make the V12 count exact or derive it from the V5 table, through a spec edit (`workspace-tooling`, V12 item 3) before or with D2. Also fix the "V12 count" wording in `proposal.md:82`. One option is to make V5 also fail when a row's crate has no `clippy.toml`, except for the `gafira-testkit` exemption.
2. `msg` is not re-exported at the crate root (`crates/gafira-doc/src/lib.rs:17`). That matches the spec, which names the path `gafira_doc::msg::CommandLabel`, and 02 §4.3. No change needed.
3. In `crates/gafira-edit/src/lib.rs:16`, `mod transaction;` comes after the `pub use` block. Group the `mod` declarations together (cosmetic).
4. The test-only `Step` command (`transaction.rs:86-155`) is a good candidate for `gafira-testkit` in D2, as `apply-progress.md` already notes.

### Other checks

- Code rules: no `#[allow]`, no threads, no clock and no `unsafe` in the diff. Comments, identifiers and errors are in English. The tests use `Arc<Mutex<_>>` only to satisfy the `Send + Sync` supertraits, and spawn no threads.
- Commits: conventional subjects, author Gustavo Gutierrez, no `Co-Authored-By` or AI attribution lines.
- No `proptest-regressions` files are tracked or present.
- The `mutants` report was read. Its numbers match its `mutants.out` lists.

## D2: `gafira-testkit`, `prop_command`, V5 row, D1 test cleanup

Branch `feat/edit-history-2`, base `main` e1e4963, range `e1e4963..c91cee1`. Verified on 2026-10-25. HEAD was checked in the main checkout; intermediate commits were checked from `git archive` exports under the session scratchpad, with a shared scratch `CARGO_TARGET_DIR`. CI steps were pulled from each commit's own `ci.yml` (PyYAML) and run under `bash --noprofile --norc -eo pipefail`. V13 needs a git index, so in the exports it ran inside a throwaway `git init`. Hand mutations ran on a scratch export only. The source was not edited.

**Delivery verdict: FAIL** (1 CRITICAL, 6 WARNING, 3 SUGGESTION). **Change verdict: not final.** `next`: `tdd-apply` to fix C1 on D2 (a small test-only fix), then D3.

### Gates per commit

| Commit | fmt | clippy `-D warnings --locked` | tests passed | `cargo deny check` | ARQ-07 | V11 | V5 | V12 | V13 |
|---|---|---|---|---|---|---|---|---|---|
| e4c5d20 test cleanup | 0 | 0 | 324 (0 failed) | ok | 0 | 0 | 0 | 0 | 0 |
| 224b6b6 generator move + V5 row | 0 | 0 | 324 | ok | 0 | 0 | 0 | 0 | 0 |
| 32c1108 `prop_command` | same tree as HEAD for code (`git diff 32c1108 HEAD` touches only `apply-progress.md`) | | | | | | | | |
| c91cee1 HEAD (docs) | 0 | 0 | 327 (21 result lines, 0 failed) | ok | 0 | 0 | 0 | 0 | 0 |

The test totals match `apply-progress.md`. Size: `git diff --shortstat e1e4963..HEAD -- crates .github` gives 526 insertions and 14 deletions (589 with docs). That is under the ~700-line limit and x1.17 over the ~450 forecast. Rename detection holds: `crates/gafira-doc/tests/common/mod.rs => crates/gafira-testkit/src/lib.rs`, similarity 98%. Commits use conventional subjects with no `Co-Authored-By`, Claude or AI lines (grep on the bodies finds 0). No `proptest-regressions` files exist.

### Negative and command runs

| Run | Result |
|---|---|
| V5 clean with xtrace | rc 0; the trace shows `cargo tree -p gafira-testkit` |
| V5 with `gafira-edit` cut from the `gafira-testkit` row | rc 1, `gafira-testkit must not depend on gafira-edit` |
| V11 with forbid removed from `crates/gafira-testkit/src/lib.rs` (scratch) | rc 1, `missing forbid(unsafe_code): crates/gafira-testkit/src/lib.rs` |
| Workspace membership | `gafira-testkit 2024 1.93 []`; root `Cargo.toml:17` declares it by path |
| Dev-dependency is not a layer edge (`gafira-doc` dev-depends on `gafira-testkit`) | V5 `gafira-doc` row passes, so `gafira-testkit` is absent from its normal/build tree |
| Generator guards still run | `cargo test -p gafira-testkit` runs `guards::generated_documents_reach_every_feature_within_the_bounds` and `guards::every_corruption_is_produced_and_breaks_its_own_rule` (6.09 s) |

### Requirement and scenario coverage (D2)

| Requirement / scenario | Status | Evidence |
|---|---|---|
| `gafira-testkit` provides `prop_command` | covered | `crates/gafira-testkit/src/harness.rs:62`, re-exported at `lib.rs:11` |
| Law 1: `apply; revert` equals the original | covered | `harness.rs:138-144` |
| Law 2: `apply; revert; apply` equals the first apply | covered | `harness.rs:145-153` |
| Law 3: `validate()` passes after a successful apply | covered | `harness.rs:133-135` |
| Law 4: exact `touches` | covered, with a deviation on pages (W1) | `harness.rs:136, 209-252` |
| Law 5: on `Err`, pointer-equal | covered for detection; the "no false positive" side is untested (C1) | `harness.rs:124-131, 184-192` |
| Law 6: merged equals the sequence | covered for detection; the skip rule is untested (W3) | `harness.rs:157-182` |
| Scenario: Harness catches a wrong revert (D2, unit) | covered | `tests/harness.rs:222-240` (`ForgetsRevert` gives law 1, counterexample names `Probe`); `tests/harness.rs:242-246` covers the panic path |
| Scenario: Harness catches an inexact touches (D2, unit) | covered | same table: `SilentNode` (node changed, empty set) gives law 4; `GhostTouch` (listed node does not exist) gives law 4 |
| 1 000 cases by default, `PROPTEST_CASES` overrides | implemented, not tested (W2) | `harness.rs:32, 77-85` |
| V5 row for `gafira-testkit` lands with the crate (C3 D2) | covered | `git show --stat 224b6b6` adds the row, the crate and the manifest together; `ci.yml:84` |
| `gafira-testkit` has no `clippy.toml`; V11 covers it | covered | `ls crates/gafira-testkit` shows only `Cargo.toml src tests`; V11 negative run above |
| Root `[workspace.dependencies]` declares `gafira-testkit` | covered | `Cargo.toml:17` |
| Q10 / spike R8: generator moved with `git mv`, `mod guards` gets `#[cfg(test)]` | covered | rename detected; `lib.rs:732` |
| `gafira-doc` property tests switched to the testkit | covered | `crates/gafira-doc/tests/properties.rs:4`; `Cargo.toml` dev-dependency |
| Spike R7: the harness is used only from `tests/` (one copy of each type) | covered | the harness tests live in `crates/gafira-testkit/tests/harness.rs` |
| D1 W5 and S3 handled | covered | diffs in `transaction.rs:247`, `error.rs:62`, `command.rs:132`, `lib.rs:9-12` |
| D1 W4 (reusability after a failed apply) | not covered, although `apply-progress.md` says it is (W4) | see W4 |

### Recorded deviations, judged

1. **Law 4 and pages: acceptable, but the spec text must be amended.** The requirement text says "every page or node whose `Arc` is not pointer-equal". The same spec also says that `DirtySet` reports a page when its own fields or `roots` change, and a node when its record or `children` change (`edit-commands`, line 34). The D9 Move scenario says `touches` names the two nodes "and not the page" (line 101). A literal page `Arc` check contradicts both, because `node_mut` copies the page on write. Hand mutation M1, a literal page `Arc::ptr_eq`, makes `sound_commands_pass_every_law` fail with `law 4 broken: page 1 changed but is not in touches`. The harness follows the binding `DirtySet` rule. See W1 for the spec edit.
2. **Law 5 as value equality plus page `Arc` identity: acceptable.** `Document` is not an `Arc`, and its pages are. M2 drops the pointer half, and `FailsAfterCopying` kills it.
3. **Law 6 only when `merge` returns `Ok` and both commands of the sequence apply: acceptable.** If the sequence cannot be applied, there is nothing to compare. This rule is not tested (W3).
4. **Laws 1 and 2 also run on `Unchanged`: acceptable and required.** The spec gives no exemption. `Transaction::revert` reverts every child, including children that returned `Unchanged`, so a revert after `Unchanged` must be correct. This rule is not tested (W3).

### Mutation

`mutants` report: `scratchpad/mutants-D2/mutants.out` (cargo-mutants 27.1.0, finished 13:05:55Z). The coordinator gave no path; the directory's timestamp and its mutant list, all in `harness.rs`, match this delivery. 30 mutants: 23 caught, 4 missed, 0 timeout, 3 unviable.
- Missed `harness.rs:186:5`, `same_document -> false`, and `harness.rs:186:14`, `== -> !=`. Law 5 is a spec rule with an invariant, so this is critical code. See C1.
- Missed `harness.rs:83:9`, `cases` field deleted (falls back to proptest's 256), and `harness.rs:84:9`, `failure_persistence` field deleted (would write regression files). See W2.
- Unviable (no `Default`): `applied`, `skeleton`, `nodes_of`.

Hand mutations (scratch export, `cargo test -p gafira-testkit --test harness`):

| # | Mutation | Result |
|---|---|---|
| M1 | Law 4 pages use a literal `Arc::ptr_eq` (the spec wording) | KILLED by `sound_commands_pass_every_law` (law 4 on page 1), which supports deviation 1 |
| M2 | Law 5 without page pointer equality (`&&` to `\|\|`) | KILLED by `each_broken_law_is_named_with_a_counterexample` (`FailsAfterCopying` not caught) |
| M3 | Laws 1 and 2 skipped when `apply` returns `Unchanged` | SURVIVED (W3) |
| M4 | Law 6 compares even when the sequence fails to apply (skip rule removed) | SURVIVED (W3) |
| M5 | Law 4 existence check on listed nodes dropped | KILLED by `GhostTouch` |

Every surviving mutant makes the harness stricter than the spec, so it would reject sound commands. No surviving mutant lets a broken command pass.

### CRITICAL

1. **Law 5 has no test for its accepting side** (`crates/gafira-testkit/src/harness.rs:128-135, 184-192`). The two missed mutants turn `same_document` into "always different", so every command that returns `Err` would be reported as breaking law 5. The suite still passes because every `Err`-returning probe (`FailsAfterMutating`, `FailsAfterCopying`) is supposed to fail law 5, and no sound probe returns `Err`. D3 and later commands return `Err` on purpose ("Failed commands change nothing"; `SetProperty` NaN and 1.5), so the harness rule they depend on is not proven. Fix (test-only): add a sound probe that rejects its input before mutating and returns `Err`, for example on a missing target or an invalid value, to `sound_commands_pass_every_law`. Check that both 186 mutants then fail.

### WARNING

1. **Law 4 wording contradicts the `DirtySet` rule.** Amend `edit-commands` "Command property laws" item 4 before archive. A page is required when its own fields or `roots` differ, or when it exists on one side only. A node is required when its `Arc` is not pointer-equal or it exists on one side only. The implementation already does this (deviation 1). This is a spec edit, not a code defect.
2. **The 1 000-case default is not tested** (`harness.rs:83-84`). Deleting `cases` silently drops to 256 cases. A probe that counts `apply` calls in a static `AtomicU32` (with `PROPTEST_CASES` unset) would pin it. `failure_persistence: None` has no spec-visible effect beyond regression files, so it is near-equivalent. These lines are configuration, and the 1 000-case figure is bound by the D3+ scenario "Every core command passes the laws". So this is a warning, but cheap to close together with C1.
3. **The rules from deviations 3 and 4 are untested** (hand mutants M3 and M4 survived). Add a sound probe that returns `Unchanged` and then reverts, on a document with pages, and a merging probe whose second command fails on the result of the first. Both fit the C1 fix.
4. **`apply-progress.md` overstates D1 W4.** It says "W4 ... is covered by the harness: laws 2 and 5 run on every command". Law 5 checks state after `Err`. Law 2 re-applies only after a successful apply and revert. Neither re-applies a command after a failed apply, which is what "the command MUST stay reusable" in "Failed commands change nothing" requires. The six laws do not include this, so it is not a D2 requirement. It is still open for D3, D8, D9 and D10 (per-command tests, or an optional extra harness check) and for the `Transaction` test from D1 W4. Correct the claim in `apply-progress.md`.
5. **Law 4 checks that listed items exist, not that they changed.** This matches the spec text ("every listed item exists in the original or in the result"). A command that lists a page it never touches passes, so "exact" is one-sided. This is accepted by the spec. Record it as a known limit when the spec is amended (W1).
6. **The V12 count is still `>= 3`.** D1 W6 and S1 are unchanged. Still a spec gap, now with four copies. Not a D2 defect.

### SUGGESTION

1. `ForgetsRevert` skips the whole revert, while the scenario says "forgets one field". Law 1 compares the whole document, so detection is the same. A probe that restores `locked` but not `name` would match the scenario wording more closely.
2. The test checks that the counterexample names `Probe`, not that it is minimized. Minimization comes from proptest's `TestError::Fail` (shrunk input). That is acceptable; an assertion on a small document (for example `pages.len() == 1`) would make it visible.
3. A node `Arc` that changes without a value change (copy on write with no edit) is required in `touches`. That is stricter than "the node record changes", but harmless while commands return `Unchanged` without touching `node_mut`. Mention it in the W1 spec edit.

### Other checks

- Code rules: no `#[allow]`, no `unsafe`, no clock and no `Hash*` in `gafira-testkit`. It uses `EditToken::__new_for_gafira_edit_only()`, which is allowed in `gafira-testkit` by V13 (rc 0). English throughout.
- `gafira-testkit` normal dependencies are `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit`, `proptest` and `serde_json`. They match the V5 row and Q10, and `Cargo.lock` changes only by the new crate and the `gafira-doc` dev edge.
- The main checkout's untracked `.claude/worktrees/` is unrelated and was not touched.

## D2 re-verify (after the fix round)

Branch `feat/edit-history-2`, base `main` e1e4963, HEAD 6ff02a6. The fixes are in b37b7d1 (`crates/gafira-testkit/tests/harness.rs` only, +61/-3) and 6ff02a6 (docs). `git diff c91cee1 HEAD` touches no file under `crates/*/src`, so the harness code is unchanged. The re-verify ran on 2026-10-25. Gates ran in the main checkout. Hand mutations ran on a `git archive` export in the session scratchpad. The source was not edited, and `git status` is clean afterwards.

**Delivery verdict: PASS** (0 CRITICAL, 5 WARNING, 4 SUGGESTION). **Change verdict: not final, because D3 and later deliveries remain.** `next`: `tdd-apply` for D3.

### Gates at HEAD 6ff02a6

| Gate | Result |
|---|---|
| `cargo fmt --check` | rc 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | rc 0 |
| `cargo test --workspace --locked` | rc 0, 329 passed, 0 failed (327 before, plus the 2 new tests) |
| `cargo deny check` | advisories, bans, licenses and sources ok |
| `ci.yml` policy steps ARQ-07, V11, V5, V12 and V13 (taken from HEAD's `ci.yml` and run with `bash --noprofile --norc -eo pipefail`) | all rc 0 |
| `PROPTEST_CASES=10 cargo test -p gafira-testkit --test harness` | 5 passed. The override path works, and `it_runs_a_thousand_cases_by_default` returns early |
| `it_runs_a_thousand_cases_by_default`, 5 runs in a row with `PROPTEST_CASES` unset | 5 out of 5 passed (no flakiness seen) |

Size: `git diff --shortstat e1e4963..HEAD -- crates .github` gives 584 insertions and 14 deletions, which is under the ~700 limit. The commit bodies contain no `Co-Authored-By`, Claude or AI lines (0 matches). There are no `proptest-regressions` files.

### Mutation (re-run)

`cargo mutants -p gafira-testkit --file crates/gafira-testkit/src/harness.rs --jobs 4`, with `PROPTEST_CASES` unset. The output is in `scratchpad/rv2/mutants/mutants.out`. All 30 mutants were tested: 26 caught, 1 missed, 0 timeout, 3 unviable. The 3 unviable mutants are the same as before.

- `harness.rs:186:5` `same_document -> false`: **CAUGHT** (missed before).
- `harness.rs:186:14` `== -> !=` in `same_document`: **CAUGHT** (missed before).
- `harness.rs:83:9` delete the `cases` field: **CAUGHT** (missed before), by `it_runs_a_thousand_cases_by_default`.
- `harness.rs:84:9` delete the `failure_persistence` field: still MISSED. This is configuration, not critical code. Its only effect is that proptest regression files would be written. See W2'.

Hand mutations, using `cargo test -p gafira-testkit --test harness` on the scratch export. Every mutation was reverted, and the export was byte-checked against the original:

| # | Mutation | Result |
|---|---|---|
| H1 | `same_document` always `false` (C1) | KILLED by `sound_commands_pass_every_law`: `law 5 broken: a failed apply changed the document` (`SoundRejects`) |
| H2 | `original == other` becomes `!=` in `same_document` (C1) | KILLED by `sound_commands_pass_every_law`, with the same law 5 message |
| H3 | Laws 1 and 2 skipped when `apply` returns `Unchanged` (old M3) | KILLED by `each_broken_law_is_named_with_a_counterexample` (`UnchangedBadRevert` is no longer caught) |
| H4 | Law 6 skip rule removed, so the merge is compared even when the sequence fails (old M4) | KILLED by `merge_is_not_compared_when_the_sequence_cannot_apply` |

### Disposition of the D2 findings

| Item | Status | Evidence |
|---|---|---|
| C1 Law 5 accepting side untested | **CLOSED** | `SoundRejects` was added to `sound_commands_pass_every_law` (`tests/harness.rs:252`). Both mutants at line 186 are caught (cargo-mutants, H1 and H2) |
| W1 Law 4 wording vs the `DirtySet` rule | open (spec edit before archive) | recorded under "Pending spec edits" in `apply-progress.md` |
| W2 1 000-case default untested | partly closed | `cases` is now pinned (mutant `83:9` caught). `failure_persistence` (`84:9`) is still missed but near-equivalent. See W2' |
| W3 Skip rules from deviations 3 and 4 untested | **CLOSED** | H3 and H4 are killed. `SoundUnchanged` and `UnchangedBadRevert` were added, and so was `merge_is_not_compared_when_the_sequence_cannot_apply` |
| W4 `apply-progress.md` overstated D1 W4 | **CLOSED** (the claim was corrected). The reusability check itself is still open for D3, D8, D9, D10 and the `Transaction` test | `apply-progress.md` line 98 |
| W5 Law 4 checks only that listed items exist | open (known limit, to record in the W1 spec edit) | unchanged |
| W6 V12 count `>= 3` | open (spec gap, not a D2 defect) | unchanged |

### CRITICAL

None.

### WARNING

1. W1 (unchanged): amend `edit-commands` "Command property laws" item 4 before archive.
2. W2': the cargo-mutants miss at `harness.rs:84:9` (`failure_persistence: None` deleted). Deleting it makes proptest write `proptest-regressions/` files on failure. It has no spec-visible effect on the laws. Non-critical configuration, so this is a warning, not a FAIL. An optional `.cargo/mutants.toml` exclusion with a reason would document it.
3. W4 remainder: a test that re-applies a command after a failed apply ("the command MUST stay reusable") is still owed in D3, D8, D9 and D10, and for `Transaction`.
4. W5 (unchanged): law 4 is one-sided for listed items. Record this when the spec is amended.
5. W6 (unchanged): the V12 copy count is `>= 3` with four copies.

### SUGGESTION

1. S1 to S3 from the D2 section are unchanged.
2. The "D2 fix round" in `apply-progress.md` does not record the new test total (329). Add it with the D3 progress update.
3. `it_runs_a_thousand_cases_by_default` relies on proptest calling the `prop_flat_map` closure exactly once per accepted case (2 strategy builds). It passed 5 runs in a row. If a later `document()` change adds `prop_filter` after the flat map, or the proptest version changes, update the assertion. A `>= 2_000` bound with a matching comment would be less brittle without losing the kill on mutant `83:9` (256 cases gives 512).

## D3: `SetProperty` (node and shape fields)

Branch `feat/edit-history-3`, base `main` a34cdb7, HEAD 64c5965 (commits 05aad32, 0a5c444, 64c5965). Verified on 2026-10-25. Gates ran in the main checkout. Per-commit runs, the outcome tally and the hand mutations ran on `git archive` exports in the session scratchpad (`scratchpad/vd3`). The source was not edited, and `git status` is clean apart from this report.

**Delivery verdict: FAIL** (1 CRITICAL, 5 WARNING, 4 SUGGESTION). **Change verdict: not final.** `next`: `tdd-apply` for a test-only D3 fix round (C1, and W2 if cheap), then re-verify. D4 follows after that.

### Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | rc 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | rc 0 |
| `cargo test --workspace --locked` | rc 0, 346 passed, 0 failed |
| Each commit alone (fmt, clippy, test) | 05aad32: rc 0, 0, 344 passed. 0a5c444: rc 0, 0, 346 passed. 64c5965 is docs only |
| `cargo deny check` | advisories, bans, licenses and sources ok |
| `ci.yml` policy steps ARQ-07, forbid unsafe, V5, V12, V13 (from HEAD's `ci.yml`, `bash --noprofile --norc -eo pipefail`) | all rc 0. V5 does not see the new `gafira-testkit` dev-dependency (`-e normal,build`) |
| `PROPTEST_CASES` = 10 / 1 000 / 20 000 on `prop_commands` | 0.06 s / 0.34 s / 5.77 s. The time scales with the count, so the 1 000 default cases really run. R14 on Linux: 2 x 1 000 cases in about 0.3 s (debug) |

Size: `git diff --shortstat a34cdb7..HEAD` gives 727 insertions (683 in `crates/`, of which 577 is `set_property.rs` with its unit tests, and 103 is `prop_commands.rs`). The code is under the ~700 limit; the total is just over it because of `apply-progress.md`. Acceptable. Commits use conventional subjects, author Gustavo Gutierrez, and contain no `Co-Authored-By`, Claude or AI lines (0 matches). No `proptest-regressions` files.

### Requirement and scenario coverage (D3)

| Requirement / scenario | Status | Evidence |
|---|---|---|
| SetProperty sets `name`, `placement`, `opacity`, `blend`, `visible`, `locked`, and `fill`, `geometry` on shapes | covered | `every_property_is_set_and_restored` (8 properties, set, `validate()`, revert equals the original) |
| Stores the previous value of every target and restores it on `revert` | covered | same test; `a_node_listed_twice_reverts_to_its_first_value`; deviation 1 below |
| `Unchanged` when **every** target already holds the value | **partly covered** | `the_same_value_is_unchanged_and_shares_the_page` covers "all hold it". The mixed case (some targets hold it, some do not) is untested. See C1 |
| Shape property on a non-shape fails with `InvalidValue` | covered | `a_shape_property_on_another_node_fails_without_a_partial_change` (`[A, GROUP]`, no partial write); hand mutant H1 killed |
| `merge_key` always `Some`, from kind + property, page, targets, gesture | covered | `the_merge_key_names_the_property_page_nodes_and_gesture` (8 kinds). D1 W2 carry-over (property folded into `kind`) is honoured: `set_property.<property>` |
| `merge`: equal keys keep the first command's previous values and the second's new value; `Err(next)` unchanged otherwise | covered | `a_merged_drag_applies_the_last_value_and_reverts_to_the_origin`, `merge_needs_an_equal_key_and_a_set_property` (gesture, no gesture, nodes, property, page, `Transaction`); H4 killed; cargo-mutants caught both guard mutants at line 209 |
| Scenario: Move several nodes | covered | `it_moves_several_nodes_and_reverts`: both moved, `touches` is exactly `{A, B}` with no page, revert equals the original |
| Scenario: Same value is Unchanged | covered | page `Arc` pointer-equal after `apply` and after `revert` |
| Scenario: Merged drag keeps the origin | covered | apply leads to the last value, revert restores the origin |
| Failed commands change nothing: Non-finite value rejected (NaN, 1.5) | covered | `invalid_values_fail_and_change_nothing` (13 rows) and `range_edges_are_accepted`. The checks match `validate.rs` (`numbers`, `below_quarter_turn`, `unit_interval`, fill channels finite, alpha in 0..=1). The checks are plain `if`s, so they also run in release builds (Q15) |
| Failed commands change nothing: Locked node is still editable (Q12) | covered | `a_locked_node_is_still_editable`; the code reads no `locked` flag |
| Missing page or node fails with no change | covered | `a_missing_page_or_node_fails_and_changes_nothing` |
| D1 W4: the command stays reusable after a failed apply | covered for `SetProperty` | `a_command_is_reusable_after_a_failed_apply`, `a_failed_apply_keeps_the_record_of_the_last_good_one`. Still owed for `Transaction`, D8, D9 and D10 |
| `touches` lists nodes, not the page | covered | unit assertion `touched.pages.is_empty()`. Law 4 cannot catch over-listing (D2 W5) |
| Every core command passes the laws (property, 1 000 cases) | covered | `set_property_obeys_the_command_laws`, `merged_set_property_drags_obey_the_command_laws`. See W3 for how much of each run reaches the `Changed` path |

### Recorded deviations, judged

1. **Command written before its tests (strict TDD broken).** `AGENTS.md` requires RED first. `apply-progress.md` records it honestly. The RED evidence is post hoc: hand mutants on the finished code, and the `mutants` run has 0 missed. This is accepted **once**, as a process WARNING (W1), and not as a FAIL, because the observable result is what counts at verify time. It is not a neutral trade, though: C1 is exactly the kind of gap test-first prevents. Nobody wrote a failing "mixed targets" test before the `all(...)`, so nothing pins it. cargo-mutants cannot express `all` to `any`, so "0 missed" does not prove the rule. The C1 fix should be written test-first, with the H3 mutant as its RED. D4 and later deliveries must follow RED-first. A second occurrence should be a FAIL.
2. **`previous` captured on every successful `apply`, not only the first.** Acceptable. The observable contract ("restores the state before the last successful `apply`", the `Command::revert` doc) is the same. It is also what keeps the command correct after a failed or repeated apply. `merge` still keeps the first command's record.
3. **`Unchanged` records an empty `previous`, so `revert` after it is a no-op without copy-on-write.** Acceptable, and covered (page `Arc` shared after revert).
4. **Law 4 cannot catch over-listing.** Correct. The unit test pins it. Same as D2 W5.
5. **`Property::Name(Option<String>)`.** Matches the optional node field. Acceptable.

### Mutation

`mutants` report: `scratchpad/mutants-D3/mutants.out` (cargo-mutants 27.1.0, `set_property.rs`). 36 mutants: 32 caught, **0 missed, 0 timeout**, 4 unviable. The 4 unviable mutants have no `Default` impl: `read -> Some(Default)`, `label`, `apply -> Ok(Default)` and `merge_key -> Some(Default)`. The `==` to `!=` mutant on the `Unchanged` comparison (line 167) and the `merge` guard mutants (line 209) are caught.

Hand mutations, for rules cargo-mutants cannot express. Each ran `cargo test -p gafira-edit --locked` on the export, which includes `prop_commands`. Each was reverted, and the file was byte-checked against the original:

| # | Mutation (`set_property.rs`) | Result |
|---|---|---|
| H1 | Shape property on a non-shape skips the node instead of failing (line 162) | KILLED by `a_shape_property_on_another_node_fails_without_a_partial_change` |
| H2 | `revert` drops its existence pre-check (lines 186-188), so `write_all` can fail half-way | **SURVIVED**. See W2 |
| H3 | `Unchanged` when **any** target holds the value (`all` to `any`, line 167) | **SURVIVED**, also with `PROPTEST_CASES=20000`. See C1 |
| H4 | `merge` takes the second command's `previous` | KILLED by `a_merged_drag_applies_the_last_value_and_reverts_to_the_origin` |
| H5 | `Unchanged` decided by the first target only (`old.first()`) | **SURVIVED**. See C1 |

Outcome tally of the generated commands (1 000 cases each, scratch-only test, not committed): the wide strategy gives 151 `Changed`, 191 `Unchanged` and 658 `Err`, with `merge` accepted 5 times and law 6 compared once. The drag strategy gives 887 `Changed`, 18 `Unchanged` and 95 `Err`, with law 6 compared 457 times.

### CRITICAL

1. **The "every target" half of the `Unchanged` rule is untested** (`crates/gafira-edit/src/set_property.rs:167`). The spec says "It MUST return `Unchanged` when every target already holds the value". It is a spec rule with an invariant, so it is critical code under `AGENTS.md`. Mutants H3 (`any`) and H5 (first target only) both survive the unit tests and 20 000 property cases. With either mutant, a multi-node edit where one target already holds the value returns `Unchanged` and writes nothing. In the product, that silently drops an edit on a multi-selection, and History (Q14) would not record it. The property laws cannot catch it: a command that changes nothing obeys laws 1 to 6, and no law checks that the value was set. The code itself is correct today; this is a test gap, like D2 C1. Fix (test-only, RED first against H3): two targets, one already holding the value (for example `A` visible, `B` hidden, set `visible = true`). Then `apply` returns `Changed`, both nodes hold the value, and `revert` restores the original. That test kills H3 and H5.

### WARNING

1. **Strict TDD deviation** (see deviation 1). Accepted once for D3. It must not repeat in D4 and later. C1 shows the cost.
2. **`revert` atomicity is not pinned** (`set_property.rs:186-188`, H2). `reverting_on_a_document_without_the_node_fails_cleanly` removes `B`, which is last in `[A, B]`. Because `revert` writes back to front, `B` fails before anything is written, so the pre-check is redundant for that test. Removing `A` instead would make H2 write `B` and then fail, which leaves a partial revert. The spec states no atomicity rule for `revert`, so this is not critical. The pre-check is still deliberate code that no test proves. A one-row change to the existing test (remove `A`, the first target) closes it.
3. **The wide property strategy mostly exercises the error path.** Only 15 % of cases reach `Changed`, and law 6 ran once in 1 000 cases. Laws 1 to 4 therefore get about 150 effective cases for the eight properties. Shape properties (`fill`, `geometry`) on shapes are rarer still. `drag` covers law 6 well, but only for `placement` and `opacity`. The unit tests cover every property, so this is a warning. Cheap improvements: generate the missing page and the missing node less often (for example 1 in 10), and draw the property in the drag strategy from the full `property()` set, minus the invalid values.
4. **D1 W4 remainder.** The reusability test is now done for `SetProperty`. It is still owed for `Transaction`, D8, D9 and D10.
5. **Open items carried from D2** (unchanged): W1 (law 4 wording), W2' (`failure_persistence` mutant), W5 (law 4 one-sided), W6 (V12 count `>= 3`).

### SUGGESTION

1. An empty target list (`nodes: vec![]`) is accepted and returns `Unchanged` (`all` over an empty list). The spec says "one or more nodes". Either reject it with `InvalidValue` or record it as accepted. The `drag` fallback relies on it today.
2. `apply-progress.md` says "14 unit tests". There are 15 (329 + 15 = 344 at 05aad32). It also still does not record the test totals (D2 S2). Record 344 and 346.
3. For D4 and D6: a merged drag that returns to its start (a to b to a) has `value == previous`. Redo then returns `Unchanged`. `History` should decide whether such an entry is dropped or kept.
4. The `merge` test checks that the returned command keeps its type, not that it is the same box. The spec says "unchanged". A pointer check like the D1 default-merge test would pin that as well (optional).

### Other checks

- Code rules: `#![forbid(unsafe_code)]` is inherited from the crate. No `#[allow]`, no `HashMap` or `HashSet`, no clock, no threads, and no global state in the diff. Comments, identifiers and errors are in English. `unreachable!("probed above")` follows the by-reference probe (spike R11), so it is unreachable.
- Hexagonal layering: `gafira-testkit` is a dev-dependency only. The normal dependencies of `gafira-edit` are unchanged (V5 rc 0). `Cargo.lock` gains only that edge.
- The `mutants` report was read. Its counts match `outcomes.json` (36 / 32 / 0 / 0 / 4).

## D3 re-verify (after the fix round)

Branch `feat/edit-history-3`, base `main` a34cdb7, HEAD d6c364b (fix commits 6312c2f, test and code, and d6c364b, docs only). Verified on 2026-10-25. Gates ran in the main checkout. Hand mutations and cargo-mutants ran on a `git archive HEAD` export in the session scratchpad (`scratchpad/vd3r`). The source was not edited. After the hand mutations the export file was byte-identical to the tracked file (`cmp`). `git status` is clean apart from this report.

**Delivery verdict: PASS** (0 CRITICAL, 4 WARNING, 4 SUGGESTION). **Change verdict: not final** (D4 onwards remain). `next`: the coordinator pushes `feat/edit-history-3` and opens its PR, then `tdd-apply` for D4.

### Gates at HEAD d6c364b

| Gate | Result |
|---|---|
| `cargo fmt --check` | rc 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | rc 0 |
| `cargo test --workspace --locked` | rc 0, 348 passed, 0 failed (346 + 2 new unit tests; the W2 test was renamed, not added). Matches `apply-progress.md` |
| 6312c2f alone | same tree under `crates/` as HEAD (d6c364b only touches `openspec/`), so the HEAD run covers it |
| `cargo deny check` | advisories, bans, licenses and sources ok |
| `ci.yml` policy steps ARQ-07, forbid unsafe (V11), V5, V12, V13 (extracted from HEAD's `ci.yml`, run with `bash --noprofile --norc -eo pipefail`) | all rc 0, empty stderr |

### Disposition of the D3 findings

| Finding | Status | Evidence |
|---|---|---|
| C1 every-target `Unchanged` rule untested | **closed** | `it_is_unchanged_only_when_every_target_holds_the_value`: `A` already hidden, `visible = false` on `[A, B]` and on `[B, A]`. It expects `Changed`, both nodes hidden, and `revert` back to the seed. H3 and H5 are now killed (below). H3 fails in either order. H5 fails on `[A, B]`, where the first target is the one that already holds the value |
| W1 strict TDD deviation | closed for the fix round | `apply-progress.md` records RED for each fix test. The re-run mutants confirm that each test fails against the defect it targets. The rule still stands for D4 and later |
| W2 `revert` atomicity not pinned | **closed** | `reverting_with_any_target_missing_fails_and_changes_nothing` removes `A`, then `B`. H2 is now killed |
| W3 wide strategy mostly takes the error path | open | Unchanged by the fix round. Carried to D4 as a WARNING |
| W4 D1 W4 remainder | open | `Transaction`, D8, D9 and D10 still owe the reusability test |
| W5 D2 carry-overs | open | Unchanged |
| S1 empty target list | **closed** | It is now refused. Judged below |
| S2 test counts in `apply-progress.md` | closed | 15 unit tests, 344, 346 and 348 recorded |
| S3 a to b to a drag, then redo | recorded | Recorded as "Note for D4" in `apply-progress.md`. D6 must decide it (Q14) |
| S4 `merge` returns the same box | closed | `std::ptr::addr_eq(before, &*back)` for each refused key |

### Empty target list, judged against the spec

The spec says "`SetProperty` MUST set one property on **one or more** nodes of one page". It defines no meaning for zero targets. Before the fix, `all` over an empty list returned `Unchanged`, so the command accepted an input that falls outside its contract. Refusing it is consistent with the spec. `InvalidValue` is the right existing variant ("not valid for the target"). The other variants name a specific id or index. Specifically:

- The check comes first in `apply`, so `revert`, `merge_key` and `touches` cannot meet an applied empty command. An empty command that fails keeps `previous = None`, so `revert` returns `NotApplied`.
- The check runs before the page lookup, so `nodes: []` on a missing page reports `InvalidValue`, not `MissingPage`. The spec sets no order for errors, so this is acceptable.
- The `drag` fallback (a document without nodes) now yields `Err` on every case. Law 5 (an `Err` changes nothing) still holds. The wide strategy now draws 1 to 2 targets, so it never generates the refused input.
- `an_empty_target_list_is_refused` checks the variant and that the document is unchanged. Hand mutant H6 kills it.
- `Transaction` keeps "an empty transaction MUST apply as `Unchanged`". That rule is different and stays untouched.

This is not a spec deviation. At archive time, a one-line spec note ("an empty target list fails with `InvalidValue`") would make the rule explicit (S2 below).

### Mutation (re-run)

cargo-mutants 27.1.0, `-p gafira-edit --in-diff` on `git diff a34cdb7..HEAD -- crates/gafira-edit/src/set_property.rs` (the whole new file, including the empty-targets check), `--jobs 4`, output in `scratchpad/vd3r/mut/mutants.out`. Result: 36 mutants, 32 caught, **0 missed, 0 timeout**, 4 unviable. The 4 unviable are the same `Default` replacements as before (`read`, `label`, `apply -> Ok(Default)`, `merge_key -> Some(Default)`). cargo-mutants generates no mutant for `if self.nodes.is_empty()`: it has no operator to swap, and replacing the whole `apply` body is unviable. H6 covers that check.

Hand mutations. Each ran `cargo test -p gafira-edit --locked`, which includes `prop_commands`:

| # | Mutation (`set_property.rs`) | Result |
|---|---|---|
| H1 | Shape property on a non-shape skips the node (`let Some(held) = ... else { continue }`) | KILLED by `a_shape_property_on_another_node_fails_without_a_partial_change` |
| H2 | `revert` drops its existence pre-check | KILLED by `reverting_with_any_target_missing_fails_and_changes_nothing` (was SURVIVED) |
| H3 | `Unchanged` when **any** target holds the value | KILLED by `it_is_unchanged_only_when_every_target_holds_the_value` (was SURVIVED) |
| H4 | `merge` also takes the second command's `previous` | KILLED by `a_merged_drag_applies_the_last_value_and_reverts_to_the_origin` |
| H5 | `Unchanged` decided by the first target only (`old.first().is_some_and(..)`) | KILLED by `it_is_unchanged_only_when_every_target_holds_the_value` (was SURVIVED) |
| H6 | The empty-targets guard is deleted (the pre-fix behaviour) | KILLED by `an_empty_target_list_is_refused` |

H6 goes past the ~5 hand-mutant budget. It was added because cargo-mutants cannot express the new guard, and the brief asked for that guard to be checked.

### Size and commits

`git diff --shortstat a34cdb7..HEAD`: 862 insertions in total. 715 of them are in `crates/` and `Cargo.lock`, and 147 in `openspec/`. Of the 715, `set_property.rs` is 608 (229 production lines, then the `#[cfg(test)]` module from line 230), `prop_commands.rs` is 103, and the manifests and `lib.rs` are 4. The 15 lines over the ~700 guide are all tests that the D3 verify asked for (C1, W2, S1, S4). The fix round added 51 lines and removed 20. Splitting these tests away from the code they pin would make no sense as a review slice. **Accepted, justified.**

Commits: 5 conventional subjects, author Gustavo Gutierrez. 0 matches for `Co-Authored-By`, Claude, Anthropic, "generated" or AI in any message. No `proptest-regressions` directory.

### CRITICAL

None.

### WARNING

1. **D3 W3 still open:** the wide `set_property` strategy mostly takes the error path, and law 6 rarely runs there. The fix to 1 to 2 targets does not change that. Address it in D4 or D6, where the merge property tests (Scenario "Merging equals the sequence") will lean on this generator.
2. **D1 W4 remainder** (reusability after a failed apply): still owed for `Transaction`, D8, D9 and D10.
3. **D2 carry-overs** W1, W2', W5 and W6: unchanged.
4. **Strict TDD:** RED-first is now mandatory for D4 and later deliveries (D3 W1). A repeat is a FAIL.

### SUGGESTION

1. The 6312c2f subject is `test(edit): ...`, but the commit also changes behaviour (`apply` now refuses an empty list). `fix(edit)` would be more accurate. Optional: reword it before the PR. If not, the PR description should name the behaviour change.
2. At archive, add one sentence to the `SetProperty` requirement: an empty target list fails with `InvalidValue` and changes nothing.
3. The `drag` doc comment ("law 6 runs on almost every case") does not mention that a document without nodes now gives an all-`Err` run. A one-line note would help.
4. D2 S3 (the brittle `it_runs_a_thousand_cases_by_default` bound) is unchanged.

## D4: `History` (execute, preview, undo, redo)

Branch `feat/edit-history-4`, base `main` b70a4a5, HEAD e2c06f1 (commits 9e39e5d, 7aec6b6, a76a699, e2c06f1). Verified on 2026-10-25. Gates ran in the main checkout. Per-commit runs, the release run and the hand mutations ran on `git archive` exports in the session scratchpad (`scratchpad/vd4`, one target dir per export). The source was not edited, and `git status` is clean apart from this report.

**Delivery verdict: PASS** (0 CRITICAL, 4 WARNING, 4 SUGGESTION). **Change verdict: not final** (D5 to D10 remain). `next`: `tdd-apply` for the next delivery (D5, or the D6/D8 chain heads, all based on D4).

### Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | rc 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | rc 0 |
| `cargo test --workspace --locked` | rc 0, 367 passed, 0 failed |
| Each commit alone (fmt, clippy, test) | 9e39e5d: rc 0, 0, 350 passed. 7aec6b6: rc 0, 0, 366 passed. a76a699: rc 0, 0, 367 passed. e2c06f1 is docs only. The counts match `apply-progress.md` |
| `cargo test -p gafira-edit --release --locked` | rc 0; 44 unit tests (the `debug_assertions`-gated should_panic test is compiled out, as intended) |
| `cargo deny check` | advisories, bans, licenses and sources ok |
| `ci.yml` policy steps ARQ-07, forbid unsafe, V5, V12, V13 (HEAD's `ci.yml`, `bash --noprofile --norc -eo pipefail`) | all rc 0. `ci.yml` is unchanged by D4 |

Size: `git diff --shortstat main..HEAD -- crates` gives 674 insertions and 12 deletions (`history.rs` 486, of which about 150 production lines and the rest the `#[cfg(test)]` module; `transaction.rs` +97/-12; `tests/history.rs` 78; `error.rs` 11; `lib.rs` 2). With `apply-progress.md` the total is 734. Under the ~700 guide for code. Accepted.

Commits: conventional subjects whose types match the content (`fix` for the `Transaction` rollback behaviour change, `feat` for `History`, `test` for the property, `docs` for progress), author Gustavo Gutierrez, 0 matches for `Co-Authored-By`, Claude, Anthropic, "generated" or AI. No `proptest-regressions` files.

### Requirement and scenario coverage (D4)

| Requirement / scenario | Status | Evidence |
|---|---|---|
| History owns the session document (ARQ-30, Q2) | covered | `History { doc: Arc<Document>, .. }`, `document() -> &Arc<Document>`. The public API of `gafira-edit` (all `pub fn`/`pub use` grepped) returns no `&mut Document`, `DocMut` or `EditToken` |
| Readers share the snapshot | covered | `readers_keep_their_snapshot_when_a_command_runs` (old `Arc` keeps dpi 96, not pointer-equal). See S2 |
| Send and Sync | covered | `a_history_is_send_and_sync` |
| Atomic execute, copy via `Arc::make_mut` on a clone, private token | covered | `apply_to_copy` (`history.rs:127-140`) |
| Failure leaves the Arc untouched | covered | `a_failed_command_leaves_the_session_untouched_and_usable` (the probe touches the copy before failing, so a leak would show; also proves the history stays usable) |
| New change clears redo | covered | `a_new_change_clears_redo`; hand mutant M1 |
| Unchanged is not recorded, redo kept (Q14) | covered | `an_unchanged_command_is_not_recorded` (len, pointer-equal) + `unchanged_and_failed_commands_keep_redo` (after an undo); hand mutant M2 |
| Buggy command trips the assert in debug | covered | `a_command_that_breaks_the_document_trips_the_debug_assert` (`#[cfg(debug_assertions)]`, orphan node, `should_panic(expected = "left the document invalid")`) |
| `debug_assert` after undo and redo | covered by code, untestable | `debug_check` (`history.rs:143-149`). It cannot fire: every installed snapshot was validated by `new` or `apply_to_copy`. Equivalent mutant, see W3 |
| `History` reads no clock (Q3) | covered | `at` is a parameter; no `Instant`/`SystemTime` in the diff; the domain clippy ban holds |
| Undo restores the exact Arc | covered | `undo_restores_the_exact_snapshot`, `redo_reinstalls_the_exact_state_the_command_produced`, `entries_undo_newest_first_and_redo_oldest_first`; hand mutant M3 |
| Undo calls no command method | covered | `undo_and_redo_call_no_command_method` (1 000 rounds, counts `(1, 0)`) |
| Cost does not grow with the document (property) | covered | `tests/history.rs::undo_and_redo_install_the_recorded_snapshots_whatever_the_size` (testkit `document()`, from the default document up to the generator's maximum, 1 to 3 commands, pointer-equal at every step, no extra command call) |
| Nothing to undo | covered | `with_nothing_to_undo_or_redo_both_return_none` |
| `len` / `can_undo` / `can_redo` | covered | tuples asserted in the undo/redo tests; all accessor mutants caught |
| Preview leaves the session alone | covered | `preview_returns_the_result_without_installing_it`, `a_failed_preview_returns_the_error` |

Carry-overs:

| Item | Status | Evidence |
|---|---|---|
| D1 W3: `Transaction::apply` swallowed rollback errors | closed for `apply` | `roll_back` tries every earlier revert, newest first, and returns `RollbackFailed { apply, revert }` with the first revert error; with clean reverts it still returns child *i*'s error. `a_failed_rollback_is_reported_after_trying_every_earlier_child`; hand mutants M4, M5. `Transaction::revert` still stops at the first error (see W2) |
| D1 W4: reusability after a failed apply | closed for `Transaction` and `History` | `a_transaction_is_reusable_after_a_failed_apply` (fail-once child, then a clean apply and revert); `History` drops a failed command and keeps working. Still owed by D8, D9, D10 |
| D3 note: a merged drag that ends where it started | decided: kept | Redo installs the stored `after` and never re-applies, so an `Unchanged` re-apply cannot lose the entry: `redo_does_not_depend_on_what_a_reapply_would_report`. The D7 consequence (treat `Unchanged` on a compacted redo as success) is recorded in `apply-progress.md`. Sound |

### Recorded deviations, judged

1. **`undo`/`redo` return `Result<Option<CommandLabel>, CommandError>`.** Accepted. The D7 requirement already says a compacted undo/redo "MUST ... return the error", so the D4 text ("return the entry's `CommandLabel` (or `None`)") cannot stand once D7 lands; fixing the signature now avoids churn in D5 to D7. Amend the D4 requirement text at archive (W1).
2. **`History::new(doc) -> Result<Self, ValidationError>`.** Accepted. It is the most direct reading of "built from a validated document", and `it_refuses_an_invalid_document` pins it.
3. **`_at` ignored.** Accepted. D4 has no merge; Q3 only forbids reading a clock. D6 must use it.
4. **`is_empty` untested.** Accepted as a trivial accessor demanded by `clippy::len_without_is_empty`; two missed mutants (W3). See S3 for a one-line fix.

### Strict TDD

Judged as met. `apply-progress.md` gives concrete RED evidence per unit: the `Transaction` test failed against a variant-only stub with a quoted assertion (`left: InvalidValue("c")`, `right: RollbackFailed { .. }`); `History` execute tests failed against a stub (7 failures, quoted assertions, including the should_panic that did not panic and `new` accepting dpi 0); undo/redo failed against an `Ok(None)` stub (6 failures), and the one test that passed on the stub was strengthened until it failed. Two tests passed at once and say so honestly: the `Transaction` reuse test (a characterization of existing behaviour) and the ARQ-32 property (its RED is a mutant, `undo` installing a deep clone). Both are acceptable: neither drove new code. RED cannot be replayed from git because each test lands with its code in one commit, which is the accepted commit-per-work-unit rule.

### Mutation

`cargo-mutants` 27.1.0 on the D4 diff (`scratchpad/mutants-D4/mutants.out`, read only): 23 mutants, 13 caught, 4 missed, 0 timeout, 6 unviable. No coordinator-written summary existed; the numbers above are from the raw lists.

| Missed mutant | Class | Disposition |
|---|---|---|
| `error.rs:43` `Display::fmt` -> `Ok(Default::default())` | non-critical (text not fixed by spec; D1 W1 precedent) | WARNING |
| `history.rs:83` `is_empty` -> `true` / `false` | non-critical (trivial accessor) | WARNING |
| `history.rs:144` `debug_check` -> `()` | equivalent (installed snapshots are always valid) | WARNING |

`roll_back` produced only unviable mutants, so its rule order was hand-mutated. Hand mutations (5, on an export, all restored and diffed clean):

| # | Mutation | Result |
|---|---|---|
| M1 | `redo.clear()` moved before the apply (clears on `Err` and `Unchanged`) | KILLED by `unchanged_and_failed_commands_keep_redo` |
| M2 | copy installed before the `Changed` check | KILLED by `an_unchanged_command_is_not_recorded` |
| M3 | `redo` takes the oldest undone entry (FIFO) | KILLED by `entries_undo_newest_first_and_redo_oldest_first` |
| M4 | rollback reverts oldest first (no `.rev()`) | KILLED by `a_failed_rollback_is_reported_after_trying_every_earlier_child` and `a_failing_child_rolls_back_the_earlier_ones` |
| M5 | rollback reports the last revert error instead of the first | KILLED by `a_failed_rollback_is_reported_after_trying_every_earlier_child` |

### CRITICAL

None.

### WARNING

1. **Spec text lags two accepted deviations.** `edit-history` "Undo and redo by snapshot" still says undo/redo return `CommandLabel` or `None` (code: `Result<Option<_>, CommandError>`), and `edit-commands` "Transaction" says apply returns "the error of command *i*" with no case for a failing revert (code: `RollbackFailed`). Neither is under "Pending spec edits" in `apply-progress.md`. Add both before archive; the `CommandError` list in `edit-commands` line 34 should name "rollback failed" too.
2. **`Transaction::revert` still stops at the first error** (the second half of D1 W3). The spec says revert all in reverse order. `History` never calls `revert` in D4, and D7 discards a failed copy, so there is no live damage, but D7 must decide whether a compacted `Transaction` undo should try every child, as `roll_back` now does.
3. **Missed mutants** (non-critical or equivalent): `Display` text, `is_empty` x2, `debug_check`. Documented above; not a FAIL.
4. **Open carry-overs:** D3 W3 (wide `set_property` strategy mostly errors) goes to D6; D1 W4 remainder to D8, D9, D10; D2 W1, W2', W5, W6 unchanged. D7 must implement the "after-state only while newest or next holds its snapshot" rule (D4 keeps `after` on every entry, which the D4 text allows).

### SUGGESTION

1. Every `Probe` returns the label `test.probe`, so the label assertions cannot tell which entry's label came back (a mutant returning the next entry's label would survive). Give the probe a label per instance in one test.
2. "Readers share the snapshot" asks that `a` still equals the old state; the test checks only the DPI. Compare `*a` with a clone taken before `execute`.
3. One `assert!(history.is_empty())` in `with_nothing_to_undo_or_redo_both_return_none` would close two missed mutants for free.
4. Running `ci.yml`'s "Install toolchain" step locally triggers a `rustup` self-update. Harmless for the repo; future verify runs can skip that step and run only the policy bodies.

### Other checks

- Code rules: no `#[allow]`, `unsafe`, clock, `HashMap` or threads in the diff. Comments and identifiers in English. `CommandError` stays `#[non_exhaustive]`, so the new variant is not a breaking change.
- The test-only token use (`EditToken::__new_for_gafira_edit_only`) stays inside `gafira-edit`; V13 passes.
