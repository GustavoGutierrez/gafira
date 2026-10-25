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
- W4 (reusability after a failed apply) is covered by the harness: laws 2 and 5 run on every command.

### Deviations and notes

- Law 4 and pages: the spec says "every page or node whose `Arc` is not pointer-equal". A page `Arc` also changes whenever one of its nodes changes, but `DirtySet` lists a page only for its own fields or `roots`. The harness therefore requires a page only when its fields or `roots` differ in value (arena ignored), and requires a node whenever its `Arc` differs or it exists on one side only. D3 and later commands must list nodes, not their page, for node edits.
- Law 5 compares the whole document (value) and each page `Arc` (pointer).
- Law 6 runs only when `merge` returns `Ok`, and only when both commands apply in sequence (the second command is generated for the original document, so it may fail on the result; then the case is skipped).
- Laws 1 and 2 also run when `apply` returns `Unchanged`.
- API: `prop_command(commands: Fn(&Document) -> Strategy<C>)` (needs `C: Command + Clone`) and `try_prop_command` returning `LawViolation { law, detail, counterexample }`. Cases: 1 000 by default, `PROPTEST_CASES` overrides; failure persistence is off (no regression files).
- `gafira-edit` does not yet dev-depend on `gafira-testkit`; D3 adds it with `prop_commands.rs`. `gafira-testkit` has normal dependencies on `gafira-doc` and `gafira-edit` (V5 row `gafira-geom gafira-color gafira-doc gafira-edit`).
- V12 count left at `>= 3` (verify S1 is a spec edit, not part of D2).
- Untracked `.claude/worktrees/` in the checkout is not mine and was not committed.
