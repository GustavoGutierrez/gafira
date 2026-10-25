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

## D5: trybuild, ARQ-30 fixtures

Base: `main` 1518560. Branch `feat/edit-history-5`.

### Commits and gates

| Commit | Content |
|---|---|
| test(edit): run an outside Command through History with trybuild (ARQ-30) | `trybuild = "1"` in `[workspace.dependencies]` (locked 1.0.122, 18 days old), dev-dependency of `gafira-edit`, `tests/ui.rs`, pass fixture `pass_outside_command.rs` (outside `Command`: execute, undo, redo) |
| test(edit): prove with compile-fail fixtures that a Document cannot be mutated from outside (ARQ-30) | 8 compile-fail fixtures with `.stderr` |

Each commit: fmt, clippy `--workspace --all-targets --locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check` (advisories, bans, licenses, sources ok), and the CI policy steps ARQ-07, forbid unsafe, V5, V12, V13 (all rc 0).

Size (`git diff --shortstat main..HEAD -- . ':!Cargo.lock' ':!openspec'`): 20 files, 204 insertions (forecast ~380). `Cargo.lock` adds 137 lines.

### Scenario to test

| Scenario | Evidence |
|---|---|
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
