# Edit, command and history model

## Status

| Part | State |
|---|---|
| `Command` trait, `Applied`, `CommandError` (with `RollbackFailed`), `DirtySet`, `MergeKey`, `GestureId` | Implemented in `gafira-edit` |
| `Transaction` (Composite) | Implemented |
| `History`: owned `Arc<Document>`, atomic `execute`, undo and redo by snapshot, `preview`, gesture merge, memory budget with compaction | Implemented |
| `SetProperty` (eight node properties), `InsertNodes`, `RemoveNodes` | Implemented |
| `debug_assert` after each command, undo and redo (ARQ-13) | Implemented in `History` |
| `gafira-testkit` (document generator, `prop_command` with six laws) | Implemented |
| `trybuild` compile-fail fixtures for ARQ-30 | Implemented |
| `Reorder`, `Reparent` (delivery D9) | **Planned**: in progress, not merged |
| `InsertPage`, `RemovePage`, page and DPI targets of `SetProperty` (delivery D10) and CI step V14 | **Planned** |
| `EditService`, `Session`, `SelectionService`, event bus | **Planned**: layer 3, `gafira-engine` |
| `Selection`, `HitTester`, `Tool` | **Planned** |

The crate is layer-1 domain code: no I/O, no clocks, no `unsafe`. It depends on `gafira-geom`, `gafira-color`, `gafira-doc` and, to size extension payloads, `serde_json`. Its public items are `Command`, `Applied`, `GestureId`, `MergeKey`, `DirtySet`, `CommandError`, `Transaction`, `History`, `DEFAULT_BUDGET`, `SetProperty`, `Property`, `InsertNodes`, `NodeSpec` and `RemoveNodes`. It must not publicly re-export `EditToken` (CI V13). The sources of truth for this chapter are the code under `crates/gafira-edit/`, `openspec/specs/` and the open change `openspec/changes/edit-history`.

## The Command contract

Every document change is a command (Command pattern, spec 02 section 4.3). `Command: Any + Send + Sync + Debug`:

| Method | Contract |
|---|---|
| `label() -> CommandLabel` | Message key shown as "Undo {label}"; no text, only a key |
| `apply(&mut self, &mut DocMut) -> Result<Applied, CommandError>` | Applies the change. Returns `Changed` when the command is an undo step and `Unchanged` when the document already held the result. A failed `apply` MUST leave the document equal to its prior state (ARQ-13) |
| `revert(&mut self, &mut DocMut) -> Result<(), CommandError>` | Restores the state before the last successful `apply` |
| `merge_key() -> Option<MergeKey>` | Default `None`; the key under which this command may merge with a later one |
| `merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>>` | Default hands `next` back untouched. An override recovers the concrete type by probing by reference first (`(&*next as &dyn Any).is::<Own>()`), because a failed by-value downcast cannot return the box |
| `touches() -> DirtySet` | Pages and nodes this command changes |

Commands receive `&mut DocMut<'_>`, the capability view of `gafira-doc`. They do not hold an `EditToken` themselves; the owner of the document does.

### Supporting types

- `Applied`: `Changed` or `Unchanged`.
- `GestureId(pub u64)`: names one drag or similar gesture so its commands can merge.
- `MergeKey { kind: &'static str, page: Option<PageId>, nodes: Vec<NodeId>, gesture: Option<GestureId> }`. Two commands merge only when their keys are equal; `History` decides when (same gesture, or inside the merge window).
- `DirtySet { pages: BTreeSet<PageId>, nodes: BTreeSet<NodeId> }` with `union`. A page is listed when its own fields or its `roots` change; a node when its record or its `children` change. Its purpose is cache invalidation in the render layer.
- `CommandError` (non-exhaustive): `MissingPage`, `MissingNode`, `NotAContainer`, `IndexOutOfRange { index, len }`, `WouldCreateCycle`, `InvalidValue(&'static str)`, `InvalidDocument(ValidationError)`, `IdSpaceExhausted`, `NotApplied`, `RollbackFailed { apply, revert }`. It is `Clone`, `PartialEq` and implements `std::error::Error`.

## Transaction

`Transaction::new(label, commands)` is a Composite command: many commands, one history entry, all or nothing (ADR-03).

- `apply` runs the children in order. If a child fails, the children already applied are reverted in reverse order (every revert is tried, even after one fails) and the failing child's error is returned, so the document ends as it started. If one of those reverts fails too, the document may not equal its prior state and the error is `CommandError::RollbackFailed { apply, revert }`, carrying the original error and the first revert error; `History` applies on a copy, so it simply drops the copy. The result is `Changed` if any child changed the document.
- `revert` reverts the children in reverse order and stops at its first error.
- `touches` is the union of the children's sets.

## History

`History` is Command plus Memento (ADR-03) and the only place where commands run against the session document. It owns an `Arc<Document>`; there is no `&mut Document` accessor (a compile-fail fixture proves it).

| Operation | Behavior |
|---|---|
| `History::new(doc)`, `History::with_budget(doc, budget)` | Validate `doc` and return `ValidationError` when it is invalid. `new` uses `DEFAULT_BUDGET` |
| `execute(cmd, at: Duration)` | Atomic: runs `cmd` on a copy that shares every page and node it does not touch. `Changed` installs the copy and records the command as an undo step; `Unchanged` records nothing; `Err` leaves the document as it was. A recorded change clears the redo stack |
| `undo()`, `redo()` | Return `Result<Option<CommandLabel>, CommandError>`; `None` when there is nothing to do. An entry that holds its snapshots swaps the stored `Arc<Document>` in and calls no command method |
| `preview(cmd)` | The document `cmd` would produce, without installing it or touching either stack |
| `begin_gesture()`, `end_gesture()` | Open and close a gesture; `begin_gesture` returns its `GestureId` |
| `document()`, `len()`, `is_empty()`, `can_undo()`, `can_redo()`, `estimated_bytes()` | Read-only state |

Because `Document` shares pages and nodes by `Arc`, one snapshot costs only the changed path. Each entry holds a `before` and an `after` snapshot.

### Gesture merge

A changed command merges into the newest undo entry when both have a `merge_key`, the keys are equal and `Command::merge` accepts it. The `gesture` field of the key then decides:

- With `Some(gesture)`, they merge only while that gesture is the open one.
- With `None`, they merge only when `at` is less than 500 ms after the newest entry's last command.

`at` is a monotonic `Duration` supplied by the caller: the crate reads no clock (ARQ-02). Undo and redo end a merge run. A merged entry keeps its first `before` snapshot and takes the newest `after`, so one undo reverts the whole drag.

### Memory budget and compaction

`DEFAULT_BUDGET` is 512 MiB (536 870 912 bytes); `with_budget` sets another value, and `usize::MAX` never compacts.

- The estimate is per entry: the bytes of its `before` snapshot that its `after` snapshot does not share by `Arc` identity, under a fixed size model (record sizes plus the heap length of strings and extension payloads), not real heap. It is cached per entry, so `estimated_bytes()` is O(entries).
- After an `execute` that records or merges a change, `History` drops the snapshots of the oldest entries, oldest first, until the sum fits the budget. A compacted entry keeps its command and counts zero. The newest undo entry is never compacted.
- Undo of a compacted entry runs `revert` on a copy of the current document; redo runs `apply` again, and `Unchanged` counts as success. On `Err` nothing changes and the entry returns to its stack. This is why `revert` must restore the exact prior state, including the id counter.

### Validation

`History` runs `debug_assert!(doc.validate().is_ok())` after every applied command and after every undo and redo (ARQ-13). Release builds do not run `validate()` (it is O(n)); each command checks its own inputs in all builds. The history is never saved with the document.

## Core commands

Commands act on one page and do not check `locked`: tools enforce locks, not commands (owner decision, Q12 of the change). Every successful `apply` records what `revert` needs, so a command stays reusable after a failed one.

### SetProperty

`SetProperty::new(page, nodes, value: Property, gesture: Option<GestureId>)` sets one property on one or more nodes. `Property` has eight variants:

| Variant | Node field | Check |
|---|---|---|
| `Name(Option<String>)` | name | none |
| `Placement(Placement)` | placement | values finite, `w` and `h` not negative, skew strictly between -90 and 90 |
| `Opacity(f32)` | opacity | in `0.0..=1.0` |
| `Blend(BlendMode)` | blend mode | none |
| `Visible(bool)` | visibility | none |
| `Locked(bool)` | lock flag | none |
| `Fill(Option<Color>)` | shape fill | finite color, alpha in `0.0..=1.0`; shapes only |
| `Geometry(ShapeGeometry)` | shape geometry | shapes only |

A bad value is `InvalidValue`; a missing page or node is `MissingPage` or `MissingNode`; a shape property on another node kind and an empty target list are `InvalidValue`. It returns `Unchanged` when every target already holds the value. Its merge key is the property kind (for example `set_property.opacity`), the page, the targets and the gesture; merging keeps the first command's previous values and takes the newer value.

### InsertNodes and NodeSpec

`InsertNodes::new(page, parent, index, specs)` inserts subtrees as page roots (`parent` is `None`) or as children of a `Group`. A `NodeSpec` describes a node without ids (name, placement, opacity, blend, visible, locked, kind and `children`); only a `Group` may have children. Ids are allocated from the document counter on `apply`, in preorder. `revert` removes the nodes and lowers the counter with `DocMut::restore_next_id`, so a redo reuses the same ids and rebuilds an identical document. Errors: `MissingPage`, `MissingNode`, `NotAContainer`, `IndexOutOfRange`, `IdSpaceExhausted`, and `InvalidValue` for an empty list or children on a non-group.

### RemoveNodes

`RemoveNodes::new(page, nodes)` removes nodes with all their descendants. A node listed together with one of its descendants counts once, as the ancestor. It keeps the very `Arc<Node>` values it removed, and `revert` puts every subtree back at its old parent and index. An empty list is `InvalidValue`.

### Planned commands

- `Reorder` and `Reparent` inside one page (delivery D9, in progress): **Planned**.
- `InsertPage`, `RemovePage`, and the page fields (`name`, `size`, `locked`, `hidden`) and document DPI targets of `SetProperty` (delivery D10): **Planned**.
- Commands for kinds or data that do not exist yet (clips, keyframes, masks, templates, versions), moving nodes between pages and lock enforcement in commands are out of scope of the change.

## Evidence for the contract

### Property harness (ARQ-81)

`gafira-testkit::prop_command` takes a function that builds a command strategy for a generated document and checks six laws:

1. `apply; revert` gives a document equal to the original.
2. `apply; revert; apply` gives a document equal to the first `apply`.
3. After a successful `apply`, `validate()` passes.
4. `touches` is exact: every page or node that changed is listed, and every listed one exists.
5. On `Err`, the document is pointer-equal to the original.
6. When two commands merge, applying the merged command equals applying both in sequence.

It runs 1 000 cases unless the `PROPTEST_CASES` environment variable says otherwise. `try_prop_command` returns a `LawViolation` (law number, detail, minimized input) instead of panicking. `crates/gafira-edit/tests/prop_commands.rs` runs it for every core command. The CI step V14, which would check that each core command has such a test, is **Planned** (it stands in for `xtask`, which does not exist yet).

### Compile-fail fixtures (ARQ-30)

`crates/gafira-edit/tests/ui.rs` uses `trybuild` with fixtures in `tests/ui/` that act as a crate outside `gafira-edit`. One fixture must compile: an outside `Command` run through `History`. Eight must not: taking `&mut Document` from `History`, assigning through `&Document` or through `&Arc<Document>`, calling `Document::edit` without a token or on a shared reference, writing a private field, and reaching `page_mut` or `node_mut` from a shared document or page. The constructor of `EditToken` is public but hidden, so no fixture can show that building a token fails; V13 covers that. A change to the `Document::edit` signature needs the `.stderr` snapshots regenerated with `TRYBUILD=overwrite`. Whether the snapshots are identical on Windows and macOS is not verified yet.

### Structural tests

- Undo and redo of an entry that holds its snapshots swap one `Arc` and call no command method, for any document size (ARQ-32, structural proof; the timed criterion waits for the ARQ-40 benchmark).
- A 300-move drag copies only the dragged node (`Arc::ptr_eq` for every other node).
- A property test shows that undo and redo through `revert` and `apply` (compacted entries) reach the same states as through snapshots; today it covers `SetProperty` only.
