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
