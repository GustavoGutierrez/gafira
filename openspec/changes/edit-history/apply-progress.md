# Apply progress: edit-history (C3)

## D1: crate skeleton, `msg`, `Command` contract, `Transaction`

Status: done. Base: `origin/main` c67d068. Branch `feat/edit-history-1`, range `origin/main..HEAD`.

### Commits and gates

Every commit passed alone: `cargo fmt --check`, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check` (advisories, bans, licenses, sources ok), and the `ci.yml` steps V11, V5, V12, V13 under `bash --noprofile --norc -eo pipefail` (all rc 0).

| Commit subject | Tests passed (all targets plus doctests) |
|---|---|
| feat(edit): add gafira-edit crate skeleton with its V5 row | 313 |
| feat(doc): add the msg module with MessageKey and CommandLabel | 314 |
| feat(edit): add the Command contract and its supporting types | 319 |
| feat(edit): add Transaction, an all-or-nothing composite command | 324 |

Size (`git diff --shortstat origin/main..HEAD -- crates .github`): 637 insertions after the last code commit (forecast ~570).

### Scenario to test

| Scenario | Evidence |
|---|---|
| Dependency set (command) | V5 rc 0 with the `gafira-edit` row; the `cargo tree` filter is what V5 runs |
| Clock call rejected (integration) | scratch `Instant::now()` in `crates/gafira-edit/tests/`: clippy fails with `disallowed_methods`, note ARQ-02; scratch file removed |
| Object safety and bounds | `command::tests::a_boxed_command_is_send_and_sync` |
| Default merge returns next unchanged | `command::tests::default_merge_hands_next_back_unchanged` |
| Merge can recover the concrete type | `command::tests::merge_recovers_its_own_type_and_returns_any_other_intact` |
| DirtySet union | `dirty::tests::union_is_ordered_and_deduplicated` |
| `CommandError` is Clone, PartialEq, Display, Error | `error::tests::an_error_is_cloneable_comparable_and_an_error` |
| All or nothing | `transaction::tests::a_failing_child_rolls_back_the_earlier_ones` |
| Reverse-order revert | `transaction::tests::revert_undoes_the_children_in_reverse_order` |
| Touches is the union | `transaction::tests::touches_is_the_union_of_the_children` |
| Changed iff a child changed; empty is Unchanged | `transaction::tests::it_reports_changed_only_when_a_child_changed` (table) |
| Own label, `merge_key` None | `transaction::tests::it_has_its_own_label_and_never_merges` |
| Label is a key (Copy, equality) | `msg::tests::a_label_is_a_copyable_key` |
| No serde on MessageKey (command) | `grep -c 'Serialize\|Deserialize' crates/gafira-doc/src/msg.rs` prints 0 |

### RED evidence

- Skeleton: crate and `clippy.toml` added without the row: V5 exits 1 with `::error::gafira-edit has clippy.toml but no V5 row`; with the row, rc 0.
- `msg`: stub `PartialEq` returning false: `assertion left == right failed` on `MessageKey("edit.set_property")`.
- Default merge (stub returned `Ok(())`): the test panicked on `expect_err`. `DirtySet::union` (stub returned `self.clone()`): `left [1, 3], right [1, 2, 3]`.
- Transaction (stub without rollback, forward revert, first child's touches): three assertion failures (document kept dpi 600; log `revert 1, revert 2`; touches `([], [3])`).

### Tests that passed at once, proven non-vacuous

- Send/Sync test: removing `Sync` from the `Command` supertraits fails the build (`dyn Command` cannot be shared between threads).
- Merge recovery: replacing the by-reference probe with `if false` makes the test panic.
- `it_reports_changed_only_when_a_child_changed`: ignoring a child's `Changed` fails it.
- The `error` trait-bound test and `it_has_its_own_label_and_never_merges` are compile or default checks required by the spec; not mutated.

### Deviations and notes

- V12 body unchanged: the spec says "fewer than three"; four copies pass it. No threshold change was made.
- The spec fixes no field layout for `MergeKey`, `GestureId` or the `CommandError` payloads. Chosen (internal, D3 may extend): `MergeKey { kind: &'static str, page: Option<PageId>, nodes: Vec<NodeId>, gesture: Option<GestureId> }`; `GestureId(pub u64)`; `CommandError` variants `MissingPage`, `MissingNode`, `NotAContainer`, `IndexOutOfRange { index, len }`, `WouldCreateCycle`, `InvalidValue(&'static str)`, `InvalidDocument(ValidationError)`, `IdSpaceExhausted`, `NotApplied`.
- `msg` is a public module (`gafira_doc::msg::{MessageKey, CommandLabel}`), not re-exported at the crate root, as the spec names the path.
- `Transaction::apply` ignores errors from the rollback reverts (it reports the failing child's error), as the spec states only that error.
- `gafira-edit` has `proptest` as a dev-dependency (unused until D2/D3); `gafira-edit` was also added to the root `[workspace.dependencies]`.
- Test-only command lives inside `transaction.rs` tests; it moves to a shared module when a second consumer needs it.

## D2: `gafira-testkit`, `prop_command`, V5 row, D1 test cleanup

Status: done. Base: `main` e1e4963. Branch `feat/edit-history-2`, range `main..HEAD`.

### Commits and gates

Every commit passed alone: `cargo fmt --check`, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check` (advisories, bans, licenses, sources ok), and the `ci.yml` steps ARQ-07, V11, V5, V12, V13 under `bash --noprofile --norc -eo pipefail` (all rc 0).

| Commit subject | Tests passed (all targets plus doctests) |
|---|---|
| test(edit): drop assertions that re-test derives and accessors | 324 |
| refactor(testkit): move the document generator into gafira-testkit | 324 |
| feat(testkit): add prop_command with the six 02 §10 laws | 327 |

Size (`git diff --shortstat e1e4963..HEAD -- crates .github`): 526 insertions, 14 deletions in code and CI (forecast ~450). The move is a rename (`crates/gafira-doc/tests/common/mod.rs => crates/gafira-testkit/src/lib.rs`, 9 changed lines); the move commit alone is 40 insertions, 5 deletions.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Harness catches a wrong revert (law 1) | `harness::each_broken_law_is_named_with_a_counterexample` (`ForgetsRevert` gives law 1 with a counterexample naming the command); `prop_command_panics_on_a_broken_law` for the panic path |
| Harness catches an inexact touches (law 4) | same table: `SilentNode` (changed node not listed) and `GhostTouch` (listed node does not exist) |
| Laws 2, 3, 5, 6 | same table: `DriftsOnReapply`, `InvalidResult`, `FailsAfterMutating` and `FailsAfterCopying`, and a `Dial` command with an adding merge |
| Sound commands pass (no false positives) | `harness::sound_commands_pass_every_law` (page flag, node with listed touches, last-wins merge) |
| V5 row for `gafira-testkit` | V5 rc 0; with `gafira-edit` cut from the row, rc 1 and `gafira-testkit must not depend on gafira-edit` |
| Generator guards still run | `cargo test -p gafira-testkit` runs the 2 guards (`mod guards` has `#[cfg(test)]`); `gafira-doc` properties unchanged |

### RED evidence

- Harness: each law was disabled in turn on the finished harness (all laws, node touches, reapply, merge comparison, page-skeleton rule, pointer equality on `Err`). Each time `harness.rs` failed on an assertion, except the pointer-equality mutant, which survived until `FailsAfterCopying` (copy on write, no value change, then `Err`) was added; it now fails with that mutant.
- The first run of `sound_commands_pass_every_law` failed with law 3 (a test command that bumped the DPI left a page over its pixel limit), which also showed the harness reports minimized inputs.

### D1 warnings handled

- W5: `it_has_its_own_label_and_never_merges` is now `a_transaction_never_merges` (the `label()` accessor assertion is gone); `an_error_is_cloneable_comparable_and_an_error` keeps only the compile-time bound; `a_boxed_command_is_send_and_sync` no longer asserts `merge_key()`.
- S3: `mod transaction;` grouped with the other `mod` declarations.
- W4 (reusability after a failed apply) is NOT covered. Law 5 checks the state after `Err`; law 2 re-applies only after a successful apply and revert. No law re-applies a command after a failed apply, and the six laws do not include it. Still open for D3, D8, D9, D10 (per-command tests or an extra harness check) and for a `Transaction` test.

### Deviations and notes

- Law 4 and pages: the spec says "every page or node whose `Arc` is not pointer-equal". A page `Arc` also changes whenever one of its nodes changes, but `DirtySet` lists a page only for its own fields or `roots`. The harness therefore requires a page only when its fields or `roots` differ in value (arena ignored), and requires a node whenever its `Arc` differs or it exists on one side only. D3 and later commands must list nodes, not their page, for node edits.
- Law 5 compares the whole document (value) and each page `Arc` (pointer).
- Law 6 runs only when `merge` returns `Ok`, and only when both commands apply in sequence (the second command is generated for the original document, so it may fail on the result; then the case is skipped).
- Laws 1 and 2 also run when `apply` returns `Unchanged`.
- API: `prop_command(commands: Fn(&Document) -> Strategy<C>)` (needs `C: Command + Clone`) and `try_prop_command` returning `LawViolation { law, detail, counterexample }`. Cases: 1 000 by default, `PROPTEST_CASES` overrides; failure persistence is off (no regression files).
- `gafira-edit` does not yet dev-depend on `gafira-testkit`; D3 adds it with `prop_commands.rs`. `gafira-testkit` has normal dependencies on `gafira-doc` and `gafira-edit` (V5 row `gafira-geom gafira-color gafira-doc gafira-edit`).
- V12 count left at `>= 3` (verify S1 is a spec edit, not part of D2).
- Untracked `.claude/worktrees/` in the checkout is not mine and was not committed.

### D2 fix round (verify FAIL: C1 and cheap warnings)

- C1: `SoundRejects` (returns `Err` before touching the document) is in `sound_commands_pass_every_law`. With `same_document` forced to `false`, or `==` turned into `!=`, that test now fails.
- W2: `it_runs_a_thousand_cases_by_default` counts strategy builds (2 per case = 2 000); with `cases` deleted it fails. Skipped when `PROPTEST_CASES` is set.
- W3: `SoundUnchanged` passes, `UnchangedBadRevert` (revert changes the page after `Unchanged`) gives law 1, so laws 1 and 2 run on `Unchanged`; `merge_is_not_compared_when_the_sequence_cannot_apply` fails when the skip rule is removed.
- W4: the claim above is corrected.

### Pending spec edits (not code)

- W1: `edit-commands`, "Command property laws" item 4: a page is required when its own fields or `roots` differ or it exists on one side only; a node when its `Arc` is not pointer-equal or it exists on one side only. Record the known limits: listed items are only checked for existence (W5), and a copy-on-write `Arc` change without a value change counts as a change (S3).
- W6 / S1: `workspace-tooling`, V12 item 3: the copy count is still `>= 3` with four copies; make it exact or derive it from the V5 table, and fix the "V12 count" wording in `proposal.md` D1 row.

## D3: `SetProperty` (node and shape fields)

### Commits and gates

| Commit | Content |
|---|---|
| feat(edit): add SetProperty for node and shape fields | `Property`, `SetProperty`, 15 unit tests (344 tests in the workspace) |
| test(edit): run the command laws over SetProperty | `gafira-testkit` dev-dependency, `tests/prop_commands.rs` (2 properties, 1 000 cases each); 346 tests in the workspace |
| docs(edit-history): record D3 progress | this section |

Each commit: fmt, clippy `-D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (ARQ-07, forbid unsafe, V5, V12, V13) all rc 0. V5 needed no change: it reads `-e normal,build`, so the dev-dependency is not seen.

Size (`git diff --shortstat a34cdb7..HEAD`): 684 insertions in code and tests before this file (forecast ~600); about 300 of them are the command and 380 tests.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Move several nodes | `it_moves_several_nodes_and_reverts` (touches = the two nodes, no page) |
| Same value is Unchanged | `the_same_value_is_unchanged_and_shares_the_page` (page `Arc` shared, also after revert) |
| Merged drag keeps the origin | `a_merged_drag_applies_the_last_value_and_reverts_to_the_origin` |
| Non-finite value rejected | `invalid_values_fail_and_change_nothing` (NaN, 1.5, negative sizes, skew bounds, fill channels and alpha) and `range_edges_are_accepted` |
| Locked node is still editable | `a_locked_node_is_still_editable` |
| Shape property on a non-shape | `a_shape_property_on_another_node_fails_without_a_partial_change` |
| MergeKey.kind carries the property (carry-over) | `the_merge_key_names_the_property_page_nodes_and_gesture` (8 kinds), `merge_needs_an_equal_key_and_a_set_property` |
| D1 W4: re-apply after a failed apply | `a_command_is_reusable_after_a_failed_apply`, `a_failed_apply_keeps_the_record_of_the_last_good_one` |
| Laws 1 to 6 | `set_property_obeys_the_command_laws` (wide space, invalid values included) and `merged_set_property_drags_obey_the_command_laws` (same page, node and gesture so law 6 runs on almost every case) |

### RED evidence

The command was drafted before the tests, so the first test run was green. RED was shown afterwards by mutating the finished code; each mutant makes an assertion fail:

- Unit: revert without `.rev()` (duplicate targets), dropping `w >= 0`, skew bound `>` to `>=`, `Unchanged` without recording, `merge` that does not take the new value, `merge` that ignores the key, never `Unchanged`, empty-revert shortcut, not storing `previous`.
- Property: `touches` empty (law 4), revert as no-op (law 1), `merge` that drops the new value (law 6; this survived the first property until `drag` was added).

### Deviations and notes

- `previous` is captured by every successful `apply`, not only the first (spec: "on first apply"). Observable behavior is the same (the recorded value is the one before that apply) and a failed or repeated apply stays correct; `merge` keeps the first command's record as the spec asks.
- `Unchanged` records an empty `previous`, so `revert` after it is a no-op that does not copy on write.
- Law 4 only checks that listed items exist, so over-listing (the page in `touches`) is not caught by the harness; the unit test asserts `touches.pages` is empty.
- `Property::Name` carries `Option<String>` (the node field is optional). No `Display` or accessor tests.
- `Cargo.lock` gains the `gafira-testkit` edge under `gafira-edit`.

### D3 fix round (verify FAIL: C1, W2, suggestions), test first

RED evidence (tests written first, run against the defect):

- C1: `it_is_unchanged_only_when_every_target_holds_the_value` (A already hidden, set `visible = false` on `[A, B]` and `[B, A]`: Changed, both hidden, revert restores). Fails against H3 (`all` to `any`) and H5 (first target only); both demonstrated.
- W2: `reverting_with_any_target_missing_fails_and_changes_nothing` (removes A, then B). Fails with the `revert` existence pre-check deleted (A missing leaves B written).
- S1: `an_empty_target_list_is_refused` was RED on the old code (empty list returned `Unchanged`); now `InvalidValue`, matching "one or more nodes". The wide property strategy draws 1 to 2 targets; the `drag` fallback for a document without nodes now returns `Err` (law 5), which is still sound.
- S4: the merge-refusal test now checks the returned box is the same box (`addr_eq`).

Totals after the fix: 348 tests in the workspace (D2 S2 closed for D3).

### Note for D4

A merged drag that ends where it started (a to b to a) has `value == previous`; applying it again (redo) returns `Unchanged`. `History` must decide whether such an entry is dropped or kept (Q14).

## D4: `History` (execute, preview, undo, redo)

Base: `main` b70a4a5. Branch `feat/edit-history-4`.

### Commits and gates

| Commit | Content | Tests in workspace |
|---|---|---|
| fix(edit): report a failed rollback instead of ignoring it in Transaction | `CommandError::RollbackFailed`, W6 and D1 W4 for `Transaction` | 350 |
| feat(edit): add History with atomic execute, preview, undo and redo | `history.rs`, 16 unit tests | 366 |
| test(edit): prove undo and redo install stored snapshots at any document size | `tests/history.rs`, one property (256 cases) | 367 |

Each commit: fmt, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps ARQ-07, forbid unsafe, V5, V12, V13 (all rc 0).

Size (`git diff --shortstat main..HEAD -- crates`): 674 insertions, 12 deletions (forecast ~630, ceiling ~700). About 330 lines are `history.rs` tests.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Readers share the snapshot | `readers_keep_their_snapshot_when_a_command_runs` |
| Send and Sync | `a_history_is_send_and_sync` (an `Rc` field breaks the build) |
| Failure leaves the Arc untouched | `a_failed_command_leaves_the_session_untouched_and_usable` (also: the history works afterwards) |
| New change clears redo | `a_new_change_clears_redo` |
| Unchanged is not recorded, redo kept (Q14) | `an_unchanged_command_is_not_recorded`, `unchanged_and_failed_commands_keep_redo` |
| Buggy command trips the assert | `a_command_that_breaks_the_document_trips_the_debug_assert` (`#[cfg(debug_assertions)]`, orphan node) |
| Undo restores the exact Arc | `undo_restores_the_exact_snapshot`, `redo_reinstalls_the_exact_state_the_command_produced` |
| Undo calls no command method | `undo_and_redo_call_no_command_method` (1 000 rounds: 1 apply, 0 revert) |
| Cost does not grow (property) | `tests/history.rs::undo_and_redo_install_the_recorded_snapshots_whatever_the_size` |
| Nothing to undo | `with_nothing_to_undo_or_redo_both_return_none` |
| Preview | `preview_returns_the_result_without_installing_it`, `a_failed_preview_returns_the_error` |
| Invalid start document | `it_refuses_an_invalid_document` |

### RED evidence

Tests were written before the code each time.

- `Transaction` rollback: `RollbackFailed` was added as a variant only; `a_failed_rollback_is_reported_after_trying_every_earlier_child` failed with `left: InvalidValue("c")`, `right: RollbackFailed { .. }`. Mutant `break` after the first failed revert: the log assertion fails (`revert a` missing).
- `History` execute: against a stub (`execute` returned `Unchanged`, `len` 0, `preview` returned the current document) 7 tests failed on assertions (`left: Ok(Unchanged) right: Ok(Changed)`, `left: 0 right: 1`, `left: Dpi(96) right: Dpi(600)`, a should_panic that did not panic, `History::new` accepting dpi 0).
- Undo and redo: against a stub returning `Ok(None)`, 6 tests failed on assertions, and `undo_and_redo_call_no_command_method` was strengthened (it passed on the stub) until it failed.
- Mutants on the finished code, each caught: no `redo.clear()`, recording `Unchanged`, redo installing `before`, `apply_to_copy` returning the old document. The property was caught by `undo` installing a deep clone (`Arc::ptr_eq` fails). The property test itself passed at once on the finished code (the unit tests had driven the implementation), so its RED is the mutant only.
- `a_transaction_is_reusable_after_a_failed_apply` (D1 W4) passed at once: it characterizes existing behavior (the test command's fail-once switch is what makes it meaningful).

### Carry-overs

- D3 note (a merged drag that ends where it started): decision is to keep the entry. `History` never asks `apply` again on redo, it installs the stored `after` snapshot, so an `Unchanged` re-apply cannot lose the entry (`redo_does_not_depend_on_what_a_reapply_would_report`). D7 must treat `Unchanged` from `apply` on a compacted redo as success, because the document then already equals the after-state. D6 keeps a merged entry even if its before and after are equal.
- D1 W6: `Transaction::apply` now tries every earlier revert and returns `RollbackFailed { apply, revert }` (first revert error) when one fails; with all reverts fine it still returns the failing child's error, as the spec says. `History` drops the copy either way. `Transaction::revert` still stops at the first error (spec: revert all in reverse order); a failed revert there leaves a partial copy that `History` (D7 compacted undo) discards.
- D1 W4: covered for `Transaction` and for `History` (it keeps working after a failed command). `History` takes the command by value, so a failed command is dropped, not re-applied.
- Law 4 limits (D2 W5/S3), recorded for the pending W1 spec edit: the harness only checks that listed items exist (over-listing passes) and counts a copy-on-write `Arc` change without a value change as a change. `History` does not use `touches` yet.
- D3 W3 (wide `set_property` strategy mostly takes the error path, law 6 rarely runs): not touched; it belongs to D6, where the merge property leans on the `drag` strategy.

### Deviations and notes

- `undo` and `redo` return `Result<Option<CommandLabel>, CommandError>`, not `Option`. D7 needs the error (revert/apply on a compacted entry), so the signature is fixed now to avoid changing D5/D6 code. No error is possible in D4.
- `History::new(doc: Document) -> Result<Self, ValidationError>` validates the document ("built from a validated document").
- `execute` takes `_at` and ignores it; D6 uses it. Entries hold `command`, `before` and `after`; the `at` field and the gesture state are D6's.
- `is_empty` exists only because clippy (`len_without_is_empty`) requires it; no test.
- The debug assertion also runs after undo and redo (`debug_check`); it cannot fail there because every installed snapshot was validated when installed, so it has no test.
- To keep D6/D7 and D8 apart: D6 and D7 edit `history.rs` (same chain); D8 needs only new files and `tests/` (its Q6 scenario can live in `tests/`).

## D6: gesture merge

Base: `main` 714a29a. Branch `feat/edit-history-6`.

### Commits and gates

| Commit | Content |
|---|---|
| feat(edit): merge commands into one history entry within a gesture or 500 ms | `begin_gesture`, `end_gesture`, `Entry::at`, merge in `execute` (`at` is now used), 12 unit tests |
| test(edit): prove a 300-move drag is one entry and merging equals the sequence | `tests/gesture.rs`, 2 properties with the real `SetProperty` |
| test(edit): let the SetProperty laws reach the success path more often | D3 W3 |

Each commit: fmt, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps ARQ-07, forbid unsafe, V5, V12, V13 (all rc 0).

Size (`git diff --shortstat main..HEAD -- crates`, before this file): 422 insertions, 8 deletions (forecast ~525).

### Scenario to test

| Scenario | Evidence |
|---|---|
| 300-event drag is one entry | `a_300_move_drag_is_one_entry_that_undoes_to_the_origin` (real `SetProperty`, 1 s apart so only the gesture merges) and the unit `a_gesture_of_many_commands_is_one_entry_that_undoes_to_the_start` |
| Arrow keys within 500 ms | `equal_keys_less_than_500_ms_apart_merge_and_500_ms_do_not` (499 ms merges, exactly 500 ms does not, window from the last command) |
| Different keys never merge / None keys | `different_keys_never_merge`, `commands_without_a_key_never_merge` |
| No merge after undo | `no_merge_into_an_undone_entry_or_after_an_undo`, `an_undo_between_commands_stops_the_merge_into_older_entries` |
| Closed gesture | `a_key_with_a_gesture_never_merges_once_the_gesture_closed`, `a_gesture_from_a_previous_begin_does_not_merge_into_a_new_one` |
| Merge refused becomes a new entry | `a_command_that_does_not_merge_becomes_a_new_entry` |
| Merged entry keeps before, takes after | `a_merged_entry_keeps_its_before_and_takes_the_new_after` |
| Merging equals the sequence (property) | `merging_equals_applying_the_sequence_one_by_one` |

### RED evidence

- Unit tests were written against a stub (`begin_gesture` returned `GestureId(0)`, no merge): 6 failed on assertions (`len` 301 vs 2, `GestureId(0) != GestureId(0)`, `Dpi(100)` vs `Dpi(96)` after one undo, `2 vs 1`). The window test then failed once on my own arithmetic (1499 to 1999 is 500 ms), fixed in the test.
- The integration properties passed at once on the finished code; RED is by mutant: merge disabled fails both (`300` vs `1`); `entry.after` not updated survived until the redo `Arc::ptr_eq` check was added to the 300-move test, then fails; "any gesture merges while closed" fails the closed-gesture unit test.

### Carry-overs

- D4 note: a merged entry whose before and after are equal is kept (`a_merged_command_that_ends_where_it_started_is_kept`). `execute` uses `at` now.
- D3 W3: the page index and node pick of the wide `set_property` strategy are weighted 9:1 toward existing targets. Measured over 300 deterministic samples, `apply` succeeded 83 times before and 133 after (28% to 44%); the rest are invalid values, shape properties on non-shapes and documents without nodes. Not asserted by a test (test-infrastructure tuning).

### Deviations and notes

- Merge rule: the key's `gesture` decides. A key with `Some(g)` merges only while `g` is the open gesture (no time limit); a key with `None` merges inside 500 ms. A gesture id that is closed never merges, even inside 500 ms. This reads (a) and (b) of the spec with "across a closed gesture" taking precedence.
- `undo` and `redo` end the run (`mergeable = false`), so a command after either never merges, even into an older entry still on the undo stack.
- The window uses `saturating_sub`, so an `at` earlier than the entry's counts as zero apart (merges). Callers pass a monotonic time.
- No `touches` use; the compaction state is D7's.

### D6 fix round (verify FAIL: C1, W1, W2), test first

- C1 (M5): `a_key_with_a_closed_gesture_never_merges_while_another_gesture_is_open` (begin g1, execute g1, begin g2, execute g1 again: `len` 2). With `Some(_) => open.is_some()` it fails (`len` 1 vs 2).
- W2 (M1): `an_undo_between_commands_stops_the_merge_into_older_entries` now uses one open gesture and commands 10 ms apart, so the window cannot hide the rule. With `undo` no longer clearing `mergeable` it fails (`len` 1 vs 2). The rule is kept; it is stricter than the spec and recorded as such.
- W1 (pending spec edit, not code): `edit-history` "Gesture merge" should state that a key with a gesture merges only while that gesture is open (no time limit), that the 500 ms window applies only to keys without a gesture, and that an undo or redo ends the merge run.

## D8: `InsertNodes` and `RemoveNodes`

Base: `main` 1518560. Branch `feat/edit-history-8`.

### Decision A (owner): `DocMut::restore_next_id(u64)`

Law 1 (`apply; revert` equals the original) compares `next_id`, and `DocMut` could only raise the counter, so `InsertNodes::revert` could not satisfy it. The owner chose option A: an additive `DocMut::restore_next_id(value)` that only lowers the counter (`min`); a value at or above the counter is ignored. `revert` calls it with the counter recorded before the allocation. The clarification of "ids never reused" lives in `specs/edit-commands/spec.md` (requirement "Restoring the id counter"); the main `edit-capability` spec is not edited here. V13 does not enumerate `DocMut` methods, so `ci.yml` is untouched.
## D5: trybuild, ARQ-30 fixtures

Base: `main` 1518560. Branch `feat/edit-history-5`.

### Commits and gates

| Commit | Content |
|---|---|
| feat(doc): add DocMut::restore_next_id so a revert can return the id counter | method, 2 tests, delta spec requirement |
| feat(edit): add InsertNodes with ids allocated once and restored on revert | `insert_nodes.rs`, `NodeSpec`, 12 tests in `tests/node_commands.rs` |
| feat(edit): add RemoveNodes that restores subtrees to their exact positions | `remove_nodes.rs`, 9 more tests |
| test(edit): run the command laws over InsertNodes and RemoveNodes | two properties (1 000 cases) in `tests/prop_commands.rs` |

Each commit: fmt, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, CI policy steps ARQ-07, forbid unsafe, V5, V12, V13 (all rc 0). V11 has no step of that name in `ci.yml`.

Size (`git diff --shortstat main..HEAD -- crates`): 1037 insertions, 2 deletions (forecast ~675, ceiling ~700): about 33 + 526 + 382 + 97 per commit. Over the ceiling; about 550 lines are tests. Cut points for stacked PRs: after commit 2 (~560), then 3 and 4.
| test(edit): run an outside Command through History with trybuild (ARQ-30) | `trybuild = "1"` in `[workspace.dependencies]` (locked 1.0.122, 18 days old), dev-dependency of `gafira-edit`, `tests/ui.rs`, pass fixture `pass_outside_command.rs` (outside `Command`: execute, undo, redo) |
| test(edit): prove with compile-fail fixtures that a Document cannot be mutated from outside (ARQ-30) | 8 compile-fail fixtures with `.stderr` |

Each commit: fmt, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check` (advisories, bans, licenses, sources ok), and the CI policy steps ARQ-07, forbid unsafe, V5, V12, V13 (all rc 0).

Size (`git diff --shortstat main..HEAD -- . ':!Cargo.lock' ':!openspec'`): 20 files, 204 insertions (forecast ~380). `Cargo.lock` adds 137 lines.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Insert then redo reuses ids | `insert_then_redo_reuses_ids` (10, 11, 12; revert equals the original; redo equals the first result) |
| Remove a subtree and restore order | `remove_a_subtree_and_restore_the_order` |
| Index out of range | `an_index_above_the_length_is_refused`, `the_end_of_the_list_is_a_valid_index` |
| Insert under a shape fails cleanly | `a_shape_cannot_be_a_parent` |
| Node plus descendant equals the ancestor | `listing_a_node_and_its_descendant_is_listing_the_ancestor` |
| Exhausted id space | `an_exhausted_id_space_changes_nothing` |
| D1 W4 (reuse after a failed apply) | `a_command_is_reusable_after_a_failed_apply`, `a_remove_command_is_reusable_after_a_failed_apply` |
| Reissued id after undo (Q6, through a real `History`) | `an_id_is_reissued_after_an_undo` |
| Counter restored in a rollback | `a_failed_transaction_gives_the_ids_back` |
| Laws 1 to 5 | `insert_nodes_obeys_the_command_laws`, `remove_nodes_obeys_the_command_laws` |

### RED evidence

- `restore_next_id`: against a no-op stub `restore_next_id_lowers_the_counter_to_a_recorded_value` failed (`left: 3, right: 1`); the "never raises" test caught the mutant `next_id = value`.
- `InsertNodes` and `RemoveNodes`: tests were written against stubs (`apply` returned `Unchanged`, `revert` `Ok`): 12 of 12 insert tests and 6 of 7 remove tests failed on assertions before the code (the remaining one passed vacuously and is guarded by mutants).
- Mutants caught by unit tests: no `restore_next_id` in revert, no `validate_new`, no descendant dedupe, no sort of places, no index check in revert (the last two needed two extra tests: `positions_follow_the_list_not_the_id_order`, `a_revert_that_cannot_find_its_place_fails_and_changes_nothing`).
- The properties passed at once on the finished code (unit tests drove the implementation); RED by mutant: no counter restore gives law 1, touches without the parent node or the page gives law 4, no dedupe or no sort gives law 1.
- One slip: a commit was first made with a clippy `type_complexity` error because the gate pipeline hid the exit code; it was amended before anything else was built on it, and the gates were rerun green.

### Deviations and notes

- `apply` always allocates its ids from the counter and records them (spec: reuse on a later apply). After a `revert` the counter is back at the recorded value, so a redo gets the same ids; if a caller applies it on a document whose counter moved, fresh ids are used, which keeps ids unique. `History` never re-applies on redo.
- The new nodes are checked with `validate` on a scratch one-page document before the real one is touched; a bad value gives `InvalidDocument(ValidationError)`. Children below a non-group spec give `InvalidValue`.
- Empty node lists give `InvalidValue("an empty list of nodes")` (as `SetProperty`).
- `RemoveNodes` keeps the removed `Arc<Node>`s, so revert restores the identical nodes; it scans the page once for parents and indices (O(page nodes), not O(touched)).
- Neither command has a `merge_key` (spec: only `SetProperty` merges).
- Law 4: root inserts and removals list the page; inserts and removals inside a group list the parent node and not the page.

### D8 fix round (verify FAIL: C1, W2, W3, W4), test first

- C1: `an_id_is_reissued_after_an_undo` (real `History`: InsertNodes takes id 10, undo, a second InsertNodes takes 10 again, `validate()` passes). The behavior already held, so RED is by mutant: with `undo` not installing the `before` snapshot (temporary, reverted) it fails with `left: 11, right: 10`. The Q6 evidence row above was wrong (a `Transaction` rollback test) and is corrected.
- W2/H3: `a_revert_whose_parent_is_gone_fails_and_changes_nothing` fails with the missing-parent check removed.
- W3/H6: `a_revert_of_nodes_that_are_already_back_fails_and_changes_nothing` fails with the "already back" check removed.
- W4 (pending spec edits, for the coordinator before archive):
  1. `edit-commands` "InsertNodes and RemoveNodes": reword "on a later apply (redo after revert) it MUST reuse the same ids" as "a redo after `revert` produces the same ids".
  2. Archive carries the "Restoring the id counter" clarification of "ids never reused" into the main `edit-capability` "Id allocation" requirement.
- Not done: SUGGESTION 1 (guard `restore_next_id` in `InsertNodes::revert` against non-LIFO reverts) and the size decision (WARNING 1).
| Fixtures 1 to 5 fail to compile with the recorded code | E0599 `fail_history_document_mut`; E0594 `fail_assign_through_history` (`**history.document() =`), `fail_assign_through_ref`; E0061 `fail_edit_without_token`; E0596 `fail_edit_on_shared_ref`, `fail_mut_page_from_shared_doc`, `fail_mut_node_from_shared_page`; E0616 `fail_write_private_field` |
| Outside command takes the legitimate path | `pass_outside_command.rs` asserts the change, the undo and the redo |
| Wrong error code fails | `fail_write_private_field.stderr` changed E0616 to E0599: `mismatch`, test FAILED; reverted, ok |
| Deny passes unchanged | `cargo deny check` all ok; `git diff main -- deny.toml` empty |
| Dev-only | `cargo tree -e normal,build -i trybuild --workspace` prints `warning: nothing to print.` |
| Lock delta bounded | new names: equivalent glob hashbrown indexmap serde_spanned target-tuple termcolor toml toml_datetime toml_parser toml_writer trybuild winapi-util windows-link windows-sys winnow; all in the `cargo metadata` closure of `trybuild`; no `git+` source in the lock |
| V13 E16, E17 | clean tree rc 0; untracked `tests/ui/scratch.rs` with `pub use gafira_doc::EditToken;`, a glob, and a multi-line group: rc 1 each with the re-export message; removed |

### RED evidence

- Pass fixture: first version asserted one page after undo; trybuild reported `Test case failed at runtime` (`left: 0, right: 1`); fixed to the real value.
- Compile-fail: `fail_history_document_mut.rs` first read `history.document()` as `&Document`, which compiles: `Expected test case to fail to compile, but it succeeded.` Changed to `document_mut()`. `fail_assign_through_history.rs` first used `*history.document() =`, which gave E0308 (assigns the `Arc`), not the intended E0594; changed to `**`. With the other fixtures and no `.stderr` the test failed with `wip` for each (snapshots written, then accepted with `TRYBUILD=overwrite` and read by hand).

### Deviations and notes

- V13 needed no change: the D1 body already scans every `.rs` under `crates/gafira-edit`, fixtures included (E16, E17 verified above). No CI step for "trybuild is dev-only" (owner decision pending, spike R13); the check is the `cargo tree` command above.
- R3 stays UNVERIFIED: `.stderr` equality on Windows and macOS needs a CI run, and nothing was pushed. The Linux-only fallback (`#[cfg(target_os = "linux")]`) is not applied; if CI shows a mismatch, add it and say so in the verify report.
- `fail_edit_without_token.stderr` quotes the `Document::edit` signature of `gafira-doc` (`$WORKSPACE/crates/gafira-doc/src/edit.rs`); a change of that line needs `TRYBUILD=overwrite`.
- Fixture 1 shows "no such method" (`document_mut`); the `&Arc<Document>` side is `fail_assign_through_history`.
- The shared scratchpad was overwritten by a sibling delivery's `gates.sh` once; D5 scripts live in a `d5/` subdirectory.

## D7: memory budget and compaction

Base: `main` 1518560. Branch `feat/edit-history-7`.

### Commits and gates

| Commit | Content |
|---|---|
| feat(edit): estimate the bytes each history entry holds alone | `budget.rs` (size model), `Entry::bytes`, `History::estimated_bytes` |
| feat(edit): compact the oldest snapshots over a memory budget | `DEFAULT_BUDGET`, `History::with_budget`, compaction, revert/apply undo and redo |
| test(edit): prove undo and redo through revert reach the recorded states | `tests/budget.rs`, one property (256 cases) |
| test(edit): pin that a 300-move drag copies only the dragged node | `tests/gesture.rs`, one property (`Arc::ptr_eq` per untouched node) |

Each commit: `cargo fmt --check`, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the `ci.yml` policy steps ARQ-07, forbid unsafe (V11), V12, V13, V5 under `bash --noprofile --norc -eo pipefail` (all rc 0). 402 tests in the workspace after the last code commit.

Size (`git diff --shortstat main..HEAD -- crates`, before this file): 891 insertions, 34 deletions (forecast ~630, ceiling ~700). About 560 of them are tests (`history_budget_tests.rs` 457, `budget.rs` unit tests about 110, `tests/budget.rs` 72, `tests/gesture.rs` 20); production code is about 230. Over the ceiling. The cut points are commit boundaries: commits 1 and 2 are 799 insertions and stand alone; commits 3 and 4 (92 lines of property tests) can go to a follow-up PR if the owner wants the delivery split.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Small budget compacts the oldest | `a_small_budget_compacts_the_oldest_entries_first` (ten entries, budget 2.5 entries: first eight compacted, last two held; after three entries only the oldest is gone), `a_sum_equal_to_the_budget_fits`, `the_newest_entry_keeps_its_snapshots_even_over_the_budget` |
| Undo through revert reaches the same states (property) | `tests/budget.rs::undo_and_redo_through_revert_reach_the_same_states` (real `SetProperty`, budget 0 against `usize::MAX`) and the unit `undo_of_a_compacted_entry_reverts_and_redo_applies` (counts: 2 reverts, 3 + 2 applies) |
| Failed revert changes nothing | `a_failed_revert_changes_nothing`, `a_transaction_that_fails_midway_through_revert_leaves_no_partial_change` |
| Failed apply on a compacted redo changes nothing | `a_failed_apply_on_a_compacted_redo_changes_nothing` |
| Shared data is not double counted | `shared_data_is_not_double_counted` (100 nodes, ten single-node commands, under twice a whole-document entry) and the `budget.rs` unit tests (one changed node does not count its siblings, a page missing from the newer state counts in full, strings and payloads count their length) |
| Default budget 512 MiB | `the_default_budget_is_512_mib` (536 870 912) |
| Compaction after a merge | `an_entry_that_grows_by_merging_can_push_older_ones_out` |
| Estimate follows the entry through merge, undo and redo | `a_merged_entry_is_estimated_from_its_first_before_to_its_last_after`, `undone_entries_keep_their_estimate`, `an_entry_counts_the_snapshot_it_replaces_not_what_the_command_added` |
| 300-move drag copies no untouched node (carry-over) | `tests/gesture.rs::a_drag_copies_only_the_dragged_node` |

### RED evidence

- Estimate: stub `unshared_bytes` returning 0: `strings_and_payloads_count_their_heap_length` (`left: 0, right: 1000`), `a_state_that_shares_every_page_counts_only_its_own_record` (`record > 0`) and `one_changed_node_does_not_count_its_siblings` (`one > shared`) failed on assertions. At the History level, stub `estimated_bytes` returning 0: `shared_data_is_not_double_counted` (`ten.estimated_bytes() > 0`) and `undone_entries_keep_their_estimate` (`held > 0`) failed.
- Compaction and revert path: with `compact()` a no-op and undo/redo handling only entries with snapshots, 8 tests failed on assertions (`left: 0, right: 536870912`; `[false, false, false]` vs `[true, true, false]`; `Ok(Some(..))` vs `Err(InvalidValue("revert"))`; `estimated_bytes() <= budget`). The merge test was written after finding that the merge path returned before `compact()`: `[false, false]` vs `[true, false]`, then fixed.
- Hand mutants on the finished code (a selection, not the full set; the cargo-mutants run below is the full one): no early `break` in `compact`; compacting the newest entry; not zeroing `bytes`; `<` for `<=` (survived until `a_sum_equal_to_the_budget_fits`); no push-back after a failed undo or redo; swapped `unshared_bytes` arguments. These were caught by the tests at that point.
- The property in `tests/budget.rs` and the drag test passed at once (the unit tests drove the code); RED is by mutant: undo using `apply` instead of `revert`, no compaction (the `estimated_bytes` comparison fails), redo using `revert`, and a `SetProperty` that copies every node of the page (drag test fails).

### Carry-overs

- D4/D6: `Unchanged` from `apply` on a compacted redo is success (`redo_of_a_compacted_entry_succeeds_when_apply_reports_unchanged`: a drag that ends where it started, compacted, undone through revert and redone through an `Unchanged` apply). The entry moves back to the undo stack and stays compacted.
- D4 W2, `Transaction::revert` stops at its first error: kept. `History` runs a compacted undo on a copy and drops it on `Err`, so the partial reverts never reach the session (test above, second child reverted on the copy, first fails, document pointer-equal). The spec line "revert all in reverse order" is read as "all, unless one fails".
- `undo`/`redo` already returned `Result<Option<_>>`; no signature change.
- Merge into a compacted entry is refused (`merge_into_newest` returns the command when the entry has no snapshots). It cannot happen today because the newest entry is never compacted and `mergeable` is cleared by undo and redo; the guard replaces an `unreachable!`.

### Note for D8 (`next_id`)

Undo of an entry that holds its snapshots installs the snapshot, so `next_id` goes back (Q6). A compacted undo goes through `Command::revert` and D7 adds no `DocMut` operation, so for `SetProperty` (no allocation) both paths give equal documents, which is what the property checks. The owner approved D8 option A (`DocMut::restore_next_id`, called by `InsertNodes::revert`), so a compacted undo of an insert returns the counter and the compacted redo through `apply` reuses the same ids. The revert property in `tests/budget.rs` covers `SetProperty` only; extend it to `InsertNodes` and `RemoveNodes` after D8 merges (D9/D10 or the integration delivery).

### Deviations and notes

- The estimate adds a fixed document record per entry (`size_of::<Document>()` plus the page handle list), so every snapshot costs something even when a command changes only the DPI, and budget 0 compacts every entry but the newest. The spec lists pages and nodes only; this is a lower bound as before.
- The estimate is cached per entry (`bytes`), computed when the entry is pushed and again when a merge replaces its after-state, so `estimated_bytes()` is O(entries), not O(nodes). It depends only on the entry's own before and after, so compacting a neighbor never changes it.
- Compaction runs after an `execute` that records or merges a `Changed` command; undo, redo and `Unchanged` or failed commands never grow the estimate, so they do not compact.
- Compaction walks entries oldest first and never touches the newest of the undo stack. It runs only when the redo stack is empty (every `Changed` execute clears it).
- API: `History::with_budget(doc, budget)`, `History::new` uses `DEFAULT_BUDGET` (`pub const`, exported). The spec says "take a budget at construction"; the two constructors are the choice. A budget of `usize::MAX` never compacts.
- `gafira-edit` gains `serde_json` as a normal dependency (to size extension payloads without allocating). V5 only constrains `gafira-*` edges; `cargo deny` passes. `Cargo.lock` gains the edge.
- The compaction state is read in tests through the private `Entry::snaps` (the test module lives in `src/history_budget_tests.rs`, declared with `#[path]` under `history.rs`); no public accessor was added. The spec's `estimated_bytes()` is public.
- Not asserted: the 1.2x to 2.5x factor against real heap (spike R9); it is a property of the size model, not of the code.
- Untracked `.claude/worktrees/` is not part of this delivery and was not staged.

### D7 fix round (verify FAIL: C1, C2, W2, W3, W4), test first

Each test was written against the finished code and passed at once; RED is by cargo-mutants mutant, applied one at a time with the test run restored after each (script outside the repo).

- C1: `one_execute_drops_as_many_snapshots_as_it_takes_to_fit` (budget 3 small entries; then a medium move that must drop two, and a 57-node move that must drop three). `history.rs:128:23` `-=` to `/=` fails it (`[true, false, false, false]`); `-=` to `+=` fails it and `a_small_budget_compacts_the_oldest_entries_first`.
- C2: exact-estimate tests in `budget.rs`: `the_document_record_counts_one_handle_per_page` (7 pages), `a_page_counts_its_record_name_roots_and_arena_slots`, `a_node_counts_its_record_name_and_children` (30 children), `an_extension_counts_its_type_id_and_payload_exactly` (type id +9, string payload 500, object keys, nested array and object). All 13 surviving mutants (`16:40` x2, `35:28` x2, `36:28` x2, `36:51`, `52:55`, `57:9`, `57:31` x2, `64:9`, `67:67`) now fail at least one of them. The two overflow cases verify found (7 pages, 30 children) are in these tests; the generator caps pages at 3, so they are hand-built, not generated.
- W2: `main` (3d92768) merged into the branch. `Cargo.lock` keeps `serde_json` and `trybuild`; this file and `verify-report.md` keep both sides (D5 first, then D7).
- W3: the revert-versus-snapshot property covers `SetProperty` only (see the D8 note above): extend it to `InsertNodes` and `RemoveNodes` after D8 merges. Note for D9/D10.
- W4: the "each caught" claim above is corrected to "a selection". The cargo-mutants run had 64 mutants, 44 caught, 14 missed (all in C1 and C2), 6 unviable.
- S2: the `next_id` note is updated (option A decided).

Size of the fix round: 140 lines of tests (`git diff --shortstat c3259a8..HEAD -- crates` before the merge: `budget.rs` 104, `history_budget_tests.rs` 36). Total for the delivery, `git diff --shortstat main..HEAD -- crates`: 1030 insertions, 34 deletions.

## D9: `Reorder` and `Reparent`

Base: `main` dd35a14. Branch `feat/edit-history-9`.

### Commits and gates

| Commit | Content |
|---|---|
| test(edit): cover InsertNodes and RemoveNodes in the undo-through-revert property | D7 W3 carry-over: `tests/budget.rs` steps are now `SetProperty`, root `InsertNodes` and `RemoveNodes` |
| feat(edit): add Reorder and Reparent with cycle and container checks | `moves.rs` (shared plan, write, checked revert), `reorder.rs`, `reparent.rs`, 27 tests in `tests/move_commands.rs` |
| test(edit): run the command laws over Reorder and Reparent | two properties (1 000 cases) in `tests/prop_commands.rs` (held for the owner's go-ahead on size, see below) |

Each commit: fmt, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, CI policy steps ARQ-07, forbid unsafe, V5, V12, V13 (all rc 0). V11 has no step of that name in `ci.yml`.

Size (`git diff --shortstat main..HEAD -- crates`): 888 insertions, 24 deletions after commit 2; 940 insertions with the properties (forecast ~630, ceiling ~700). About 560 lines are tests.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Bring to front | `bring_to_front_and_back` |
| Group two nodes | `group_two_nodes` (roots `[g]`, children `[a, b]`, `validate()` passes) |
| Cycle rejected | `a_group_cannot_move_under_itself_or_a_descendant` (descendant, itself, one culprit among several nodes) |
| Non-`Group` target | `a_shape_is_not_a_parent` |
| Index range | `a_reorder_index_must_be_below_the_sibling_count`, `the_index_counts_the_target_list_without_the_moved_nodes` |
| Failed commands change nothing, D1 W4 (reuse after a failed apply) | `refused` (pointer-equal page) in every refusal test; `a_reorder_is_reusable_after_a_failed_apply`, `a_reparent_is_reusable_after_a_failed_apply` |
| Law 4 touches | `reordering_inside_a_group_touches_the_group_and_not_the_page`, `reordering_a_root_touches_the_page`, `a_reparent_touches_both_parents_and_the_moved_nodes` |
| Validate-before-mutate in revert | `a_reorder_revert_over_a_changed_list_fails_and_changes_nothing`, `a_reorder_revert_whose_group_is_gone_...`, `a_revert_whose_page_is_gone_...`, `a_reparent_revert_over_a_changed_list_...`, `a_reparent_revert_whose_old_parent_is_gone_...`, `reverting_a_move_that_changed_nothing_is_a_no_op_...` (each asserts the document equals the pre-revert clone) |
| Rollback in a transaction | `a_failed_transaction_undoes_a_reparent` |
| Laws 1 to 5 | `reorder_obeys_the_command_laws`, `reparent_obeys_the_command_laws` |

### RED evidence

- Commit 1 passed at once on the finished code (D8 already restores the counter); RED by mutant: `InsertNodes::revert` without `restore_next_id` fails the property (`next_id` 26 vs 25); `RemoveNodes::revert` pushing roots at the end instead of the recorded index fails it (roots `[8, 2, 5]` vs `[2, 5, 8]`).
- Commands: tests were written against a stub (`apply` returned `Unchanged`, `revert` `Ok`): 24 of 26 failed on assertions (for example `left: Ok(Unchanged), right: Err(IndexOutOfRange { index: 4, len: 4 })`, `left: Ok(Unchanged), right: Err(WouldCreateCycle(NodeId(3)))`, roots still `[2, 3, 7, 8]` vs `[2, 7, 8]`); the 2 that passed are the `Unchanged` cases, guarded by the others.
- Hand mutants on the finished code, applied one at a time: revert state check removed, `MissingNode`/`MissingPage` in revert replaced, index bound off by one, container check removed, ancestor dedupe removed, cycle check removed, document order reversed, `touches` without moved nodes or without the page, revert writing the after-lists, empty-list check, missing-node check: all caught by `tests/move_commands.rs`. One survivor, "an unchanged move reverts as a no-op even if the page is gone", led to `reverting_a_move_that_changed_nothing_is_a_no_op_...`, now caught.
- The properties passed at once (unit tests drove the code); RED by mutant: no cycle check and no container check give law 3, `touches` without the page gives law 4, revert writing the after-lists gives law 1. Not caught by laws, caught by unit tests: moved nodes missing from `touches` (law 4 only requires a superset), moved order reversed (apply and revert agree).

### Decisions and notes

- Index of `Reparent`: counts the target list once the moved nodes are out of it (`0..=len`); a larger value gives `IndexOutOfRange { index, len }` with that len. `Reorder`: final index, `0..siblings`; `index == siblings` gives `IndexOutOfRange { index, len: siblings }`. Same position gives `Unchanged` and no write.
- Moved nodes keep their document order (parents before children, siblings in list order), whatever the order of the list given. A node listed with one of its ancestors moves once, with the ancestor (as in `RemoveNodes`).
- Check order in `Reparent::apply`: empty list, page, listed nodes, parent exists, parent is a `Group` (`NotAContainer`), cycle (`WouldCreateCycle(top)` names the moved node that holds the target), index. A shape moved under itself therefore reports `NotAContainer`.
- Revert (validate-before-mutate): `revert` compares every list it will rewrite with the list `apply` left (page present, owner nodes present, equal contents) and fails with `MissingPage`, `MissingNode` or `InvalidValue("the lists changed since apply")` before writing; it restores whole old lists. A stricter rule than `RemoveNodes::revert`: a revert over any other state of those lists is refused, not guessed. `History` and `Transaction` only revert in LIFO order, so they always meet the left state. An `Unchanged` apply stores an empty record; its revert is `Ok` and writes nothing.
- Law 4: a root list change lists the page; a children list change lists the owner node and not the page; the moved nodes are listed too (the spec says "the node"), a superset the law allows. Before `apply`, `touches` lists the target nodes only (as `RemoveNodes`).
- These commands allocate no ids, so `restore_next_id` is not used.
- `places` moved from `remove_nodes.rs` to `moves.rs` and is shared; no behavior change.
- Two error branches in `moves::write` (page or owner missing) are unreachable: `plan` and the revert check prove both exist before the first write.
- Neither command has a `merge_key` (spec: only `SetProperty` merges).
- Not extended: the budget property (`tests/budget.rs`) does not run `Reorder` or `Reparent`; their revert path is covered by the unit tests and laws 1 and 2. Adding them is about 15 lines if the owner wants it.
- Untracked `.claude/worktrees/` is not part of this delivery and was not staged.

### D9 fix round (verify: W1, W2, W3, S1), test first

The tests passed at once on the finished code; RED is by hand mutant, applied one at a time with the file restored after each (script outside the repo), then GREEN on the real code.

- W1 / M1: `a_shape_moved_under_itself_is_not_a_container` (`refused(...)`, `Reparent` of shape `a` under `a`). With the cycle check moved before the container check it fails (`left: Err(WouldCreateCycle(NodeId(2)))`, `right: Err(NotAContainer(NodeId(2)))`). The order is now pinned.
- W2 / M5: `a_reparent_revert_checks_every_list_before_it_writes_one` (a reparent into `e`, then a `Reorder` inside `e`, so the roots still match and only `e`'s children differ; `revert` must fail with `InvalidValue` and leave the document equal and page pointer-equal). With a revert that checks and writes one list at a time (roots first) it fails: the document differs from the clone taken before the revert.
- W3: not a test. The spec defines `touches` only after a successful `apply`; its value before `apply` (the listed target nodes, as `RemoveNodes`) is a known non-requirement, not pinned. The two pre-apply mutants (`reorder.rs`, `reparent.rs`) are accepted as surviving.
- S1 (pending spec edit, for the coordinator before archive): `edit-commands` "Reorder and Reparent" must state the index semantics. `Reorder`: the index is the final position, `0..siblings` (`index == siblings` gives `IndexOutOfRange`). `Reparent`: the index counts the target list once the moved nodes are out of it, `0..=len`. It should also state that moved nodes keep document order and that a node listed with its ancestor moves with the ancestor.

Size of the round: 28 lines of tests. Gates on the test commit: fmt, clippy, `cargo test --workspace --locked` (467 passed), `cargo deny check`, ARQ-07, forbid unsafe, V5, V12, V13, all rc 0.

### D9 mutants round (valid isolated run: 2 misses at `moves.rs:98:28`)

The misses were `+` to `-` and `+` to `*` in `.take(at.len() + 1)`, the ancestor walk of the cycle check.

- Reachability: the walk visits distinct nodes of one page's `at` map, so a chain has at most `at.len()` elements and `take(at.len())` is already enough. The spare step was unreachable slack. `+` to `*` gave `take(at.len())`, which is equivalent; `+` to `-` gave `take(at.len() - 1)`, which is observable only in a chain as deep as the page.
- Test first: `the_cycle_check_walks_a_chain_to_its_farthest_ancestor` (page with `g1 > g2 > g3 > g4 > g5`; moving `g1`, then `g2`, under `g5` must give `WouldCreateCycle`). RED against `-` (`left: Ok(Changed)`, `right: Err(WouldCreateCycle(NodeId(2)))`); GREEN on the real code. Against `*` it passes, as the equivalence says.
- Simplification (behavior-preserving, tests green): the bound is now `.take(at.len())`, so both mutants no longer exist. The hand mutant `take(at.len() - 1)` still fails the new test.
- Mutants, own export (`git archive` of 5825717, default per-job target dirs), `cargo mutants -p gafira-edit -f crates/gafira-edit/src/moves.rs --jobs 4`: 25 mutants, 18 caught, 0 missed, 0 timeout, 7 unviable. All 25 logs show `Compiling gafira-edit` on the mutated tree first (the `Fresh` lines in 19 logs are the later `cargo test` run on that same tree).
- The two pre-apply `touches` survivors (W3) stay accepted, as recorded above.
- Gates on each commit: fmt, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked` (468 passed), `cargo deny check`, ARQ-07, forbid unsafe, V5, V12, V13, all rc 0.

### D9 mutants round: correction (re-verify)

- The `take(at.len())` refactor is behavior-preserving only on documents that pass `validate()`. The line "the spare step was unreachable slack" above holds for valid pages only. Verify found a counterexample on an invalid page: a top group unreachable from the roots (it breaks P5). Before the refactor `Reparent` returned `Err(WouldCreateCycle)`; after it, `Ok(Applied::Unchanged)`. Neither writes anything, so ARQ-13 holds either way. The command laws (`prop_command`) generate valid documents only, so they do not cover this case.
- The 2 pre-apply `touches` survivors stay accepted (W1 of the re-verify; W3 of the first verify): the spec defines `touches` only after a successful `apply`.
