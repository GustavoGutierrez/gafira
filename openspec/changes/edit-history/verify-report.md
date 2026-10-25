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
