# Edit, command and history model

## Status

| Part | State |
|---|---|
| `Command` trait, `Applied`, `CommandError`, `DirtySet`, `MergeKey`, `GestureId` | Implemented in `gafira-edit` |
| `Transaction` (Composite) | Implemented |
| `History` (undo, redo, snapshots, memory budget, gesture merge), core commands, `debug_assert` after each command, `gafira-testkit`, `trybuild` | **Planned**: open change `openspec/changes/edit-history` |
| `EditService`, `Session`, `SelectionService`, event bus | **Planned**: layer 3, `gafira-engine` |
| `Selection`, `HitTester`, `Tool` | **Planned** |

The crate is layer-1 domain code: no I/O, no clocks, no `unsafe`. It depends on `gafira-geom`, `gafira-color` and `gafira-doc`. Its public items are `Command`, `Applied`, `GestureId`, `MergeKey`, `DirtySet`, `CommandError` and `Transaction`. It must not publicly re-export `EditToken` (CI V13).

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
- `CommandError` (non-exhaustive): `MissingPage`, `MissingNode`, `NotAContainer`, `IndexOutOfRange { index, len }`, `WouldCreateCycle`, `InvalidValue(&'static str)`, `InvalidDocument(ValidationError)`, `IdSpaceExhausted`, `NotApplied`. It is `Clone`, `PartialEq` and implements `std::error::Error`.

## Transaction

`Transaction::new(label, commands)` is a Composite command: many commands, one history entry, all or nothing (ADR-03).

- `apply` runs the children in order. If a child fails, the children already applied are reverted in reverse order and the failing child's error is returned, so the document ends as it started. The result is `Changed` if any child changed the document.
- `revert` reverts the children in reverse order.
- `touches` is the union of the children's sets.

## Planned: core commands

The change `edit-history` plans, for the model that exists today (`Group`, `Shape`, `Extension`):

- `SetProperty`: node fields (`name`, `placement`, `opacity`, `blend`, `visible`, `locked`), shape `fill` and `geometry`, page fields (`name`, `size`, `locked`, `hidden`) and the document DPI.
- `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent` (inside one page), `InsertPage`, `RemovePage`.

Commands for kinds or data that do not exist yet (clips, keyframes, masks, templates, versions) are out of scope until their model exists. Moving nodes between pages and lock enforcement in commands are also out of scope of that change.

## Planned: History

`History` is specified as Command plus Memento (ADR-03). The design in `openspec/changes/edit-history` plans:

- It owns the session `Arc<Document>`; operations `execute`, `undo`, `redo` and `preview`.
- One snapshot per entry. Because `Document` shares pages and nodes by `Arc`, a snapshot costs only the changed path.
- A new change clears the redo stack.
- Gesture merge: explicit `GestureId` plus a 500 ms window on caller-supplied time. The crate reads no clock (ARQ-02), so time is passed in.
- A memory budget, default 512 MiB (536 870 912 bytes), with compaction to commands only and `revert` for compacted entries.
- `debug_assert!(doc.validate().is_ok())` after every applied command, undo and redo (ARQ-13); commands check their own inputs in all builds.
- The history is never saved with the document.

These are design decisions of an open change and may still move; treat the change folder as the source of truth until it is archived.
