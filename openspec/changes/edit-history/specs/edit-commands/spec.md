# Edit Commands Specification

## Purpose

The `Command` contract of `gafira-edit` (02 §4.3), its supporting types, `Transaction` (Composite) and the core commands that today's document model supports (`Document`, `Page`, `Group`, `Shape`, `Extension`). History, undo, merging and the budget live in `edit-history`; the edit capability (`EditToken`, `DocMut`, ARQ-30 evidence) lives in `edit-capability`. Commands for frames, masks, templates, clips, keyframes and swatches are out of scope until their data exists.

Decisions are closed in `design.md`; a requirement that depends on one names it ("Qn"). Scenario types: unit, property, integration, command, manual.

## ADDED Requirements

### Requirement: Crate purity and dependencies (ARQ-01, ARQ-02) (D1)

`gafira-edit` MUST be a layer 1 domain crate: `#![forbid(unsafe_code)]`, a byte-identical copy of the canonical domain `clippy.toml`, and normal dependencies limited to `gafira-geom`, `gafira-color`, `gafira-doc` and the standard library. It MUST NOT read clocks, the environment, files, the network or spawn threads (ARQ-02), and MUST NOT use `HashMap` or `HashSet` (ARQ-10 ban). It MUST NOT contain mutable global state (ARQ-31); the one allowed `static` is the private, immutable `EditToken` (see `edit-capability`).

#### Scenario: Dependency set (D1, command)
- GIVEN the tree after D1
- WHEN `cargo tree -p gafira-edit -e normal,build --target all --prefix none -f '{p}'` is filtered for `gafira-*`
- THEN the set is exactly `gafira-edit`, `gafira-doc`, `gafira-geom`, `gafira-color`, and V5 exits 0

#### Scenario: Clock call rejected (D1, integration)
- GIVEN a scratch `std::time::Instant::now()` call in a `gafira-edit` test
- WHEN `cargo clippy -p gafira-edit --all-targets -- -D warnings` runs
- THEN it fails citing `disallowed_methods` and ARQ-02

### Requirement: Command contract (ARQ-30, ADR-03) (D1)

`gafira-edit` MUST define `pub trait Command: Any + Send + Sync + Debug` with:
- `label(&self) -> CommandLabel` (`gafira_doc::msg::CommandLabel`, a `MessageKey`);
- `apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError>` and `revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError>` (Q1; 02 §4.3 writes `&mut Document`, amendment 1);
- `merge_key(&self) -> Option<MergeKey>`, default `None`;
- `merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>>`, default `Err(next)` unchanged;
- `touches(&self) -> DirtySet`.

`Applied` MUST have exactly the variants `Changed` and `Unchanged` (Q14). `DirtySet` MUST hold a `BTreeSet<PageId>` and a `BTreeSet<NodeId>`, MUST support union, and MUST report a page when its own fields or its `roots` change and a node when the node record or its `children` change. `MergeKey` MUST be `Clone + Eq + Debug` and carry a command kind, the targets and an optional `GestureId`. `CommandError` MUST be `#[non_exhaustive]`, `Clone`, `PartialEq`, `Display` and `Error`, and MUST distinguish at least: missing page, missing node, not a container, index out of range, would create a cycle, invalid value, invalid document after the change (carrying the `ValidationError`), id space exhausted, and revert without a prior apply.

#### Scenario: Object safety and bounds (D1, unit)
- GIVEN a test-only command type
- WHEN it is boxed as `Box<dyn Command>` and passed to `fn assert_send_sync<T: Send + Sync + ?Sized>()`
- THEN the test compiles and runs; no thread is spawned (threads are banned in the crate's tests)

#### Scenario: Default merge returns next unchanged (D1, unit)
- GIVEN a command without a `merge` override and a second boxed command
- WHEN `merge` is called
- THEN it returns `Err` holding the same second command (pointer-equal box contents)

#### Scenario: Merge can recover the concrete type (D1, unit)
- GIVEN a test command that overrides `merge` by first probing `next` by reference (`(&*next as &dyn Any).is::<Own>()`), returning `Err(next)` on a mismatch, and only then upcasting the box to `Box<dyn Any + Send + Sync>` and downcasting it to its own type (a failed by-value `Box<dyn Any>::downcast` cannot give back the `Box<dyn Command>`, so the probe MUST come first; spike R11)
- WHEN it merges a command of its own type and then one of another type
- THEN the first returns `Ok(())`, and the second returns `Err` with the other command intact

#### Scenario: DirtySet union is ordered and deduplicated (D1, unit)
- GIVEN two dirty sets that share one page and one node
- WHEN they are joined
- THEN the result iterates pages and nodes in ascending id order with no duplicates

### Requirement: Failed commands change nothing (ARQ-13) (D1, D3, D8, D9, D10)

Every core command MUST check its inputs before it mutates: the target page and nodes exist, a parent is a `Group`, indices are in range, values are finite and inside the ranges `validate()` enforces, no cycle is created, and a page size stays inside its pixel limits at the document DPI. On failure, `apply` MUST return `Err(CommandError)` and leave the document equal to its prior state; the command MUST stay reusable (a later `apply` on a valid document behaves as if the failed call never happened). These checks MUST run in release builds too (Q15). Commands MUST NOT check `locked` flags (Q12).

#### Scenario: Insert under a shape fails cleanly (D8, unit)
- GIVEN a page with a `Shape` root and a clone of the document
- WHEN `InsertNodes` targets the shape as parent
- THEN it returns `Err(NotAContainer)` and the document equals the clone

#### Scenario: Non-finite value rejected (D3, unit)
- GIVEN a node
- WHEN `SetProperty` sets its opacity to NaN, and separately to 1.5
- THEN both return `Err(InvalidValue)` and the node is unchanged

#### Scenario: Locked node is still editable (D3, unit, Q12)
- GIVEN a node with `locked = true`
- WHEN `SetProperty` changes its name
- THEN it returns `Ok(Applied::Changed)`

### Requirement: Transaction (ADR-03) (D1)

`Transaction` MUST implement `Command`, hold an ordered list of boxed commands and a label given at construction, and apply them in order. If command *i* fails, `Transaction::apply` MUST revert commands *i−1* down to 0, in reverse order, and return the error of command *i*; the document MUST then equal its prior state. `revert` MUST revert all commands in reverse order. `touches` MUST be the union of the children's sets. A transaction MUST report `Changed` when any child did, and `Unchanged` otherwise. `merge_key` MUST be `None`. An empty transaction MUST apply as `Unchanged`.

#### Scenario: All or nothing (D1, unit)
- GIVEN a transaction of three test commands where the third fails
- WHEN it is applied
- THEN it returns the third command's error, and the document equals the state before the transaction

#### Scenario: Reverse-order revert (D1, unit)
- GIVEN a transaction of two test commands that record the order of their calls
- WHEN it is applied and then reverted
- THEN the call log is apply 1, apply 2, revert 2, revert 1

#### Scenario: Touches is the union (D1, unit)
- GIVEN one child touching node 3 and another touching page 1 and node 5
- WHEN the transaction's `touches` is read
- THEN it holds page 1 and nodes 3 and 5, and nothing else

### Requirement: SetProperty on nodes and shapes (ARQ-81) (D3)

`SetProperty` MUST set one property on one or more nodes of one page: `name`, `placement`, `opacity`, `blend`, `visible`, `locked`, and for `Shape` nodes `fill` and `geometry`. It MUST store the previous value of every target on first apply and restore them on `revert`. It MUST return `Unchanged` when every target already holds the value. Setting a shape property on a non-shape node MUST fail with `InvalidValue`. Its `merge_key` MUST always be `Some`, built from the command kind, the property, the page, the target ids and the optional `GestureId` given at construction; `edit-history` decides when equal keys merge (gesture or 500 ms window). `merge` of two `SetProperty` with equal keys MUST keep the first command's previous values and the second command's new value, and MUST return `Err(next)` unchanged for any other command.

#### Scenario: Move several nodes (D3, unit)
- GIVEN two nodes on one page
- WHEN `SetProperty` sets both placements and is then reverted
- THEN both moved, `touches` names exactly those two nodes (and not the page), and after revert the document equals the original

#### Scenario: Same value is Unchanged (D3, unit)
- GIVEN a node whose `visible` is `true`
- WHEN `SetProperty` sets `visible = true`
- THEN `apply` returns `Ok(Applied::Unchanged)` and the document is pointer-equal to before

#### Scenario: Merged drag keeps the origin (D3, unit)
- GIVEN three placement changes with the same merge key: a to b, b to c, c to d
- WHEN the second and third are merged into the first and the result is reverted
- THEN apply leads to d, and revert restores a

### Requirement: InsertNodes and RemoveNodes (ARQ-81) (D8)

`InsertNodes` MUST insert one or more node subtrees, described without ids, into one page, either as roots or as children of a `Group`, at a given index (`0..=len`). On first apply it MUST allocate every id through `DocMut::alloc_node_id`; on a later apply (redo after revert) it MUST reuse the same ids, so redo rebuilds an identical document. An exhausted id space MUST fail with `IdSpaceExhausted` and no change. `RemoveNodes` MUST remove the listed nodes with all their descendants from one page; listing a node and one of its descendants MUST behave like listing the ancestor alone. It MUST remember each removed root's parent and index so that `revert` restores the exact roots and children order. A missing id MUST fail with no change.

#### Scenario: Insert then redo reuses ids (D8, unit)
- GIVEN an empty page with `next_id = 10`
- WHEN `InsertNodes` adds a group with two shape children, is reverted, and is applied again
- THEN the first apply used ids 10, 11, 12; the second apply produced a document equal to the first result

#### Scenario: Remove a subtree and restore order (D8, unit)
- GIVEN roots `[a, g, b]` where group `g` has children `[c, d]`
- WHEN `RemoveNodes` removes `g` and `d`, then reverts
- THEN after apply the roots are `[a, b]` and `g`, `c`, `d` are gone; after revert the roots are `[a, g, b]` and `g`'s children are `[c, d]`

#### Scenario: Index out of range (D8, unit)
- GIVEN roots of length 2
- WHEN `InsertNodes` targets index 3
- THEN it returns `Err(IndexOutOfRange)` and nothing changes

### Requirement: Restoring the id counter (ARQ-81) (D8, Decision A)

`DocMut` MUST offer `restore_next_id(value: u64)`, which lowers the shared id counter to `value` and MUST ignore a `value` at or above the counter, so the counter never moves up. A command that allocated ids MUST record the counter before its first allocation and `revert` MUST restore it, so that `apply; revert` gives a document equal to the original (law 1). This clarifies the main `edit-capability` requirement "Id allocation": "ids MUST never be reused within a document" holds for allocation (`alloc_*` never returns an id twice in a row of states); a revert or an undo returns the counter to a value it held before, which is the same exact-restore rule as snapshot undo (Q6).

#### Scenario: Counter restored, never raised (D8, unit)
- GIVEN a counter at 1 that allocated two ids
- WHEN `restore_next_id(1)` runs, then `restore_next_id` with a higher value
- THEN the counter is 1 after the first call, the next allocation returns the first id again, and the higher value changes nothing

### Requirement: Reorder and Reparent (ARQ-81) (D9)

`Reorder` MUST move one node to a new index inside its current sibling list (the page roots or its parent's children) and restore the old index on `revert`. `Reparent` MUST move one or more nodes of one page, keeping their relative order, under a new `Group` parent or to the roots, at a given index, and restore every old parent and index on `revert`. Moving a node under itself or under one of its descendants MUST fail with `WouldCreateCycle`. A non-`Group` target MUST fail with `NotAContainer`. Moves across pages are out of scope.

#### Scenario: Bring to front (D9, unit)
- GIVEN roots `[a, b, c]`
- WHEN `Reorder` moves `a` to index 2 and then reverts
- THEN the roots are `[b, c, a]` and then `[a, b, c]` again

#### Scenario: Group two nodes (D9, unit)
- GIVEN roots `[a, b, g]` with an empty group `g`
- WHEN `Reparent` moves `[a, b]` into `g` at index 0
- THEN the roots are `[g]`, `g`'s children are `[a, b]`, and `validate()` passes

#### Scenario: Cycle rejected (D9, unit)
- GIVEN group `g` with child group `h`
- WHEN `Reparent` moves `g` under `h`
- THEN it returns `Err(WouldCreateCycle)` and nothing changes

### Requirement: Page and document commands (ARQ-81) (D10, Q4)

`SetProperty` MUST also target page fields (`name`, `size`, `locked`, `hidden`) and the document DPI. A size or DPI change that would leave any page outside its pixel limits MUST fail with `InvalidDocument(ValidationError::PageSize { .. })` and no change (C2b constraint 5). `InsertPage` MUST insert one empty page at an index of the page list, allocating its id on first apply and reusing it on redo. `RemovePage` MUST remove one page with its whole arena and restore it at the same index on `revert`. Removing the last page MUST be allowed (an empty document is valid).

#### Scenario: DPI pushes a page over the limit (D10, unit)
- GIVEN an A0 page at 300 DPI
- WHEN `SetProperty` sets the DPI to 351
- THEN it returns the page-size error and the DPI is still 300

#### Scenario: Remove and restore a page (D10, unit)
- GIVEN pages `[p1, p2, p3]`, `p2` with nodes
- WHEN `RemovePage` removes `p2` and reverts
- THEN the pages are `[p1, p3]` and then `[p1, p2, p3]` with `p2` equal to the original

### Requirement: Command property laws (ARQ-81, ARQ-13) (D2 harness; D3, D8, D9, D10 per command)

`gafira-testkit` MUST provide `prop_command`, a harness that, given a generated valid document and a generated command for it, checks:
1. `apply; revert` gives a document equal to the original (02 §10);
2. `apply; revert; apply` gives a document equal to the first `apply` (02 §10);
3. after a successful `apply`, `validate()` passes;
4. `touches` is exact: every page or node whose `Arc` is not pointer-equal to the original is in the set, and every listed item exists in the original or in the result;
5. on `Err`, the document is pointer-equal to the original;
6. for commands that merge, applying the merged command equals applying the sequence (02 §10).

Every core command MUST have a test in `crates/gafira-edit/tests/prop_commands.rs` that runs the harness with 1 000 cases by default (`PROPTEST_CASES` may raise it). V14 checks the list (`workspace-tooling`).

#### Scenario: Harness catches a wrong revert (D2, unit)
- GIVEN a deliberately broken test command whose `revert` forgets one field
- WHEN `prop_command` runs on it
- THEN law 1 fails with a minimized counterexample

#### Scenario: Harness catches an inexact touches (D2, unit)
- GIVEN a test command that changes a node but reports an empty `DirtySet`
- WHEN `prop_command` runs
- THEN law 4 fails

#### Scenario: Every core command passes the laws (D3, D8, D9, D10, property)
- GIVEN `prop_commands.rs`
- WHEN `cargo test -p gafira-edit --test prop_commands --locked` runs
- THEN each core command's property passes 1 000 cases
