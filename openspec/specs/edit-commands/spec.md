# Edit Commands Specification

## Purpose

The `Command` contract of `gafira-edit` (02 §4.3), its supporting types, `Transaction` (Composite) and the core commands that today's document model supports (`Document`, `Page`, `Group`, `Shape`, `Extension`). History, undo, merging and the budget live in `edit-history`; the edit capability (`EditToken`, `DocMut`, ARQ-30 evidence) lives in `edit-capability`. Commands for frames, masks, templates, clips, keyframes and swatches are out of scope until their data exists.

Decisions are closed in `design.md`; a requirement that depends on one names it ("Qn"). Scenario types: unit, property, integration, command, manual.

## Requirements

### Requirement: Crate purity and dependencies (ARQ-01, ARQ-02) (D1, D7)

`gafira-edit` MUST be a layer 1 domain crate: `#![forbid(unsafe_code)]`, a byte-identical copy of the canonical domain `clippy.toml`, and direct normal dependencies limited to `gafira-geom`, `gafira-color`, `gafira-doc`, `serde_json` and the standard library. `serde_json` is allowed for the memory estimate of `edit-history` (D7), which sizes extension payloads (`serde_json::Value`) without allocating; it is already in the normal tree of `gafira-doc`, so the edge adds no package. How the checks treat it: V5 constrains only `gafira-*` edges and the forbidden UI, async and network prefixes (`workspace-tooling`, "Per-crate allowed-dependency table (V5)"), so the `serde_json` edge passes V5 by design and is not a V5 row entry; third-party crates are governed by this requirement, `cargo deny check` and review. Adding any other direct normal dependency MUST be an explicit edit of this requirement. It MUST NOT read clocks, the environment, files, the network or spawn threads (ARQ-02), and MUST NOT use `HashMap` or `HashSet` (ARQ-10 ban). It MUST NOT contain mutable global state (ARQ-31); the one allowed `static` is the private, immutable `EditToken` (see `edit-capability`).

#### Scenario: Dependency set (D1, D7, command)
- GIVEN the final tree
- WHEN `cargo tree -p gafira-edit -e normal,build --target all --prefix none -f '{p}'` is filtered for `gafira-*`, and `cargo tree -p gafira-edit -e normal --depth 1 --prefix none -f '{p}'` lists the direct dependencies
- THEN the `gafira-*` set is exactly `gafira-edit`, `gafira-doc`, `gafira-geom`, `gafira-color`; the direct dependencies are exactly `gafira-color`, `gafira-doc`, `gafira-geom` and `serde_json`; and V5 and `cargo deny check` exit 0

#### Scenario: Clock call rejected (D1, integration)
- GIVEN a scratch `std::time::Instant::now()` call in a `gafira-edit` test
- WHEN `cargo clippy -p gafira-edit --all-targets -- -D warnings` runs
- THEN it fails citing `disallowed_methods` and ARQ-02

### Requirement: Command contract (ARQ-30, ADR-03) (D1, D4)

`gafira-edit` MUST define `pub trait Command: Any + Send + Sync + Debug` with:
- `label(&self) -> CommandLabel` (`gafira_doc::msg::CommandLabel`, a `MessageKey`);
- `apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError>` and `revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError>` (Q1; 02 §4.3 writes `&mut Document`, amendment 1);
- `merge_key(&self) -> Option<MergeKey>`, default `None`;
- `merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>>`, default `Err(next)` unchanged;
- `touches(&self) -> DirtySet`.

`Applied` MUST have exactly the variants `Changed` and `Unchanged` (Q14). `DirtySet` MUST hold a `BTreeSet<PageId>` and a `BTreeSet<NodeId>`, MUST support union, and MUST report a page when its own fields or its `roots` change and a node when the node record or its `children` change. A change confined to nodes MUST be reported on those nodes and MUST NOT list their page, although copy on write gives the page a new `Arc`. `MergeKey` MUST be `Clone + Eq + Debug` and carry a command kind, the targets and an optional `GestureId`. `CommandError` MUST be `#[non_exhaustive]`, `Clone`, `PartialEq`, `Display` and `Error`, and MUST distinguish at least: missing page, missing node, not a container, index out of range, would create a cycle, invalid value, invalid document after the change (carrying the `ValidationError`), id space exhausted, revert without a prior apply (`NotApplied`), and rollback failed (`RollbackFailed`, carrying the error of the failed apply and the first error met while rolling back; see "Transaction").

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

Every core command MUST check its inputs before it mutates: the target page and nodes exist, a parent is a `Group`, indices are in range, values are finite and inside the ranges `validate()` enforces, no cycle is created, and a page size stays inside its pixel limits at the document DPI. On failure, `apply` MUST return `Err(CommandError)` and leave the document equal to its prior state; the command MUST stay reusable (a later `apply` on a valid document behaves as if the failed call never happened). These checks MUST run in release builds too (Q15). Commands MUST NOT check `locked` flags (Q12). A command that takes a list of target nodes (`SetProperty` on nodes, `InsertNodes`, `RemoveNodes`, `Reparent`) MUST refuse an empty list with `InvalidValue`.

`revert` follows the same rule (D3, D8, D9, D10). Every core command other than `Transaction` MUST check, before it writes anything, that the document is in a state it can revert (the preconditions each command lists below); when a check fails, `revert` MUST return `Err(CommandError)` and leave the document equal to its state before the call. `revert` on a command that never applied successfully MUST return `Err(NotApplied)`. After an apply that returned `Unchanged`, `revert` MUST return `Ok(())` and write nothing. `History` and `Transaction` revert only in last-in, first-out order, so they always meet the state a command's `apply` left; the preconditions protect a revert on any other state.

#### Scenario: Insert under a shape fails cleanly (D8, unit)
- GIVEN a page with a `Shape` root and a clone of the document
- WHEN `InsertNodes` targets the shape as parent
- THEN it returns `Err(NotAContainer)` and the document equals the clone

#### Scenario: Non-finite value rejected (D3, unit)
- GIVEN a node
- WHEN `SetProperty` sets its opacity to NaN, and separately to 1.5
- THEN both return `Err(InvalidValue)` and the node is unchanged

#### Scenario: Revert before apply (D3, D8, D9, D10, unit)
- GIVEN a core command other than `Transaction` that was never applied
- WHEN `revert` runs
- THEN it returns `Err(NotApplied)` and the document is unchanged

#### Scenario: Empty target list (D3, D8, D9, unit)
- GIVEN a page with nodes
- WHEN `SetProperty` on nodes, `InsertNodes`, `RemoveNodes` or `Reparent` is built with an empty list and applied
- THEN each returns `Err(InvalidValue)` and the document equals the original

#### Scenario: Locked node is still editable (D3, unit, Q12)
- GIVEN a node with `locked = true`
- WHEN `SetProperty` changes its name
- THEN it returns `Ok(Applied::Changed)`

### Requirement: Transaction (ADR-03) (D1, D4, D7)

`Transaction` MUST implement `Command`, hold an ordered list of boxed commands and a label given at construction, and apply them in order. If command *i* fails, `Transaction::apply` MUST revert commands *i−1* down to 0, in reverse order, and return the error of command *i*; the document MUST then equal its prior state (value equality: the rollback may leave pages that are equal copies, not the original `Arc`s). If one of those reverts fails, `apply` MUST still try every remaining earlier revert and MUST return `RollbackFailed` with the error of command *i* and the first revert error; the document MAY then differ from its prior state, and a caller that applies on a copy (`History`) drops it (D4). `revert` MUST revert all commands in reverse order; it MUST stop at the first child whose `revert` fails and return that error, and the document MAY then be partly reverted (`History` runs such a revert on a copy and drops it, see `edit-history` "Memory budget and compaction") (D4, D7). `touches` MUST be the union of the children's sets. A transaction MUST report `Changed` when any child did, and `Unchanged` otherwise. `merge_key` MUST be `None`. An empty transaction MUST apply as `Unchanged`.

#### Scenario: All or nothing (D1, unit)
- GIVEN a transaction of three test commands where the third fails
- WHEN it is applied
- THEN it returns the third command's error, and the document equals the state before the transaction

#### Scenario: Failed rollback is reported (D4, unit)
- GIVEN a transaction of three test commands where the third fails on apply and the second fails on revert
- WHEN it is applied
- THEN it returns `RollbackFailed` holding the third command's apply error and the second command's revert error, and the call log is apply 1, apply 2, apply 3, revert 2, revert 1 (every earlier revert was tried)

#### Scenario: Reverse-order revert (D1, unit)
- GIVEN a transaction of two test commands that record the order of their calls
- WHEN it is applied and then reverted
- THEN the call log is apply 1, apply 2, revert 2, revert 1

#### Scenario: Touches is the union (D1, unit)
- GIVEN one child touching node 3 and another touching page 1 and node 5
- WHEN the transaction's `touches` is read
- THEN it holds page 1 and nodes 3 and 5, and nothing else

### Requirement: SetProperty on nodes and shapes (ARQ-81) (D3)

`SetProperty` MUST set one property on one or more nodes of one page: `name`, `placement`, `opacity`, `blend`, `visible`, `locked`, and for `Shape` nodes `fill` and `geometry`. It MUST store the previous value of every target when it applies and restore them on `revert`, so that `revert` restores the state before the last successful `apply` (a failed apply keeps the record of the last successful one). A node listed twice MUST end on its first previous value after `revert`. Before it writes, `revert` MUST check that the page and every target node exist (`MissingPage`, `MissingNode`). It MUST return `Unchanged` when every target already holds the value. Setting a shape property on a non-shape node MUST fail with `InvalidValue`. Its `merge_key` MUST always be `Some`, built from the command kind and the property together (so two properties on the same targets never merge), the page, the target ids and the optional `GestureId` given at construction; `edit-history` decides when equal keys merge (gesture or 500 ms window). `merge` of two `SetProperty` with equal keys MUST keep the first command's previous values and the second command's new value, and MUST return `Err(next)` unchanged for any other command.

#### Scenario: Move several nodes (D3, unit)
- GIVEN two nodes on one page
- WHEN `SetProperty` sets both placements and is then reverted
- THEN both moved, `touches` names exactly those two nodes (and not the page), and after revert the document equals the original

#### Scenario: Same value is Unchanged (D3, unit)
- GIVEN a node whose `visible` is `true`
- WHEN `SetProperty` sets `visible = true`
- THEN `apply` returns `Ok(Applied::Unchanged)` and the document is pointer-equal to before

#### Scenario: Unchanged only when every target holds the value (D3, unit)
- GIVEN nodes A and B where only A is hidden
- WHEN `SetProperty` sets `visible = false` on `[A, B]` (and separately on `[B, A]`) and is reverted
- THEN `apply` returns `Ok(Applied::Changed)`, both nodes are hidden, and after revert the document equals the original

#### Scenario: Revert checks every target first (D3, unit)
- GIVEN `SetProperty` applied to `[A, B]`, then A (and separately B) removed from the document by another edit
- WHEN `revert` runs
- THEN it returns `Err(MissingNode)` naming the removed node, and the document equals its state before the call (the other target keeps the value the apply set)

#### Scenario: Merged drag keeps the origin (D3, unit)
- GIVEN three placement changes with the same merge key: a to b, b to c, c to d
- WHEN the second and third are merged into the first and the result is reverted
- THEN apply leads to d, and revert restores a

### Requirement: InsertNodes and RemoveNodes (ARQ-81) (D8, D10)

`InsertNodes` MUST insert one or more node subtrees, described without ids, into one page, either as roots or as children of a `Group`, at a given index (`0..=len`). It MUST allocate every id through `DocMut::alloc_node_id`, parents before children; a redo after `revert` MUST produce the same ids, so it rebuilds an identical document (`revert` restores the counter, see "Restoring the id counter"). An exhausted id space MUST fail with `IdSpaceExhausted` and no change. A node description with children whose kind is not `Group` MUST fail with `InvalidValue`, and a description that `validate()` would reject (a non-finite or out-of-range value) MUST fail with `InvalidDocument(ValidationError)`, both before the document is touched. Before it writes, `InsertNodes::revert` MUST check that the page and every node it inserted exist (`MissingPage`, `MissingNode`) and that the id counter stands exactly where its `apply` left it (the recorded counter plus the number of ids it allocated); otherwise it MUST fail with `InvalidValue`, so that restoring the counter never hands out an id that a later command took. After a successful apply, `touches` MUST list every inserted node plus the page (root insert) or the parent node (insert into a `Group`).

`RemoveNodes` MUST remove the listed nodes with all their descendants from one page; listing a node and one of its descendants MUST behave like listing the ancestor alone. It MUST remember each removed root's parent and index so that `revert` restores the exact roots and children order. A missing id MUST fail with no change. Before it writes, `RemoveNodes::revert` MUST check that the page exists (`MissingPage`), that no removed node is present again (`InvalidValue`), that every old parent exists (`MissingNode`), and that every old index is within its list (`IndexOutOfRange`). After a successful apply, `touches` MUST list every removed node (descendants included), plus the page when a root was removed and each old parent node of a removed child.

#### Scenario: Insert revert refuses a moved counter (D10, unit)
- GIVEN `InsertNodes` applied, then an `InsertPage` that took the next id
- WHEN the `InsertNodes` is reverted
- THEN it returns `Err(InvalidValue)`, the document is unchanged and `validate()` passes

#### Scenario: Remove revert checks before it writes (D8, unit)
- GIVEN `RemoveNodes` that removed child `c` of group `g`, then `g` removed by another command
- WHEN the first command is reverted
- THEN it returns `Err(MissingNode(g))` and the document is unchanged; and reverting a removal whose nodes are already back returns `Err(InvalidValue)` with no change

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

### Requirement: Restoring the id counter (ARQ-81) (D8, D10, Decision A)

`DocMut` MUST offer `restore_next_id(value: u64)`, which lowers the shared id counter to `value` and MUST ignore a `value` at or above the counter, so the counter never moves up. A command that allocated ids MUST record the counter before its first allocation and `revert` MUST restore it, so that `apply; revert` gives a document equal to the original (law 1). The clarification of "ids MUST never be reused within a document" lives in the `edit-capability` delta, which modifies the main requirement "Id allocation": a revert or an undo returns the counter to a value it held before, which is the same exact-restore rule as snapshot undo (Q6). A command MUST NOT lower the counter under an id that a later command allocated: the insert reverts check the exact counter first ("InsertNodes and RemoveNodes", "Page and document commands").

#### Scenario: Counter restored, never raised (D8, unit)
- GIVEN a counter at 1 that allocated two ids
- WHEN `restore_next_id(1)` runs, then `restore_next_id` with a higher value
- THEN the counter is 1 after the first call, the next allocation returns the first id again, and the higher value changes nothing

### Requirement: Reorder and Reparent (ARQ-81) (D9)

`Reorder` MUST move one node to a new index inside its current sibling list (the page roots or its parent's children) and restore the old index on `revert`. Its index is the node's final position, `0..siblings`; an index at or above the number of siblings MUST fail with `IndexOutOfRange { index, len: siblings }`. A node that is not in any sibling list of the page MUST fail with `MissingNode`.

`Reparent` MUST move one or more nodes of one page under a new `Group` parent or to the roots, at a given index, and restore every old parent and index on `revert`. Its index counts the target list once the moved nodes are taken out of it, `0..=len`; a larger index MUST fail with `IndexOutOfRange { index, len }` with that `len`. The moved nodes MUST keep document order (parents before children, siblings in list order), whatever the order of the given list. A node listed together with one of its ancestors MUST move once, with the ancestor. Moving a node under itself or under one of its descendants MUST fail with `WouldCreateCycle`, naming the listed node that holds the target. A non-`Group` target MUST fail with `NotAContainer`. The container check MUST come before the cycle check, so a shape moved under itself fails with `NotAContainer`. Moves across pages are out of scope.

A move that leaves every list as it was MUST return `Unchanged` and write nothing. Before it writes, `revert` of either command MUST check that the page exists (`MissingPage`), that the owner of every list it will rewrite exists (`MissingNode`), and that each of those lists still equals the list its `apply` left (`InvalidValue` otherwise); it then restores the old lists whole. After a successful apply, `touches` MUST list the page when the roots changed, each group whose `children` changed, and the moved nodes.

#### Scenario: Bring to front (D9, unit)
- GIVEN roots `[a, b, c]`
- WHEN `Reorder` moves `a` to index 2 and then reverts
- THEN the roots are `[b, c, a]` and then `[a, b, c]` again

#### Scenario: Group two nodes (D9, unit)
- GIVEN roots `[a, b, g]` with an empty group `g`
- WHEN `Reparent` moves `[a, b]` into `g` at index 0
- THEN the roots are `[g]`, `g`'s children are `[a, b]`, and `validate()` passes

#### Scenario: Reorder index is the final position (D9, unit)
- GIVEN roots `[a, g, b, e]`
- WHEN `Reorder` moves `a` to index 4
- THEN it returns `Err(IndexOutOfRange { index: 4, len: 4 })` and nothing changes

#### Scenario: Reparent index counts the list without the moved nodes (D9, unit)
- GIVEN roots `[a, g, b, e]`
- WHEN `Reparent` moves `[a, b]` to the roots at index 2, and separately at index 3
- THEN the first gives roots `[g, e, a, b]`, and the second returns `Err(IndexOutOfRange { index: 3, len: 2 })` with nothing changed

#### Scenario: Ancestor and descendant move once (D9, unit)
- GIVEN roots `[a, g, b, e]`, `g` holding `[c, h]`, `h` holding `[d]`, `e` empty
- WHEN `Reparent` moves `[d, g, h]` into `e` at index 0
- THEN the roots are `[a, b, e]`, `e`'s children are `[g]`, `h` still holds `[d]`, and `validate()` passes

#### Scenario: Shape under itself is not a container (D9, unit)
- GIVEN a shape `a`
- WHEN `Reparent` moves `[a]` under `a`
- THEN it returns `Err(NotAContainer(a))` and nothing changes

#### Scenario: Revert checks every list before it writes (D9, unit)
- GIVEN a `Reparent` from the roots into group `e`, then a `Reorder` inside `e`
- WHEN the `Reparent` is reverted
- THEN it returns `Err(InvalidValue)`, the document equals the state before the call, and its page is pointer-equal to before

#### Scenario: Cycle rejected (D9, unit)
- GIVEN group `g` with child group `h`
- WHEN `Reparent` moves `g` under `h`
- THEN it returns `Err(WouldCreateCycle)` and nothing changes

### Requirement: Page and document commands (ARQ-81) (D10, Q4)

`SetProperty` MUST also target page fields (`name`, `size`, `locked`, `hidden`) and the document DPI. A size or DPI change that would leave any page outside its pixel limits MUST fail with `InvalidDocument(ValidationError::PageSize { .. })` and no change (C2b constraint 5); a DPI change MUST check every page. A DPI outside the range `validate()` accepts MUST fail with `InvalidValue` before any page is checked. A page or DPI property given node targets MUST fail with `InvalidValue`. The merge key of a page field names the page and no nodes; the merge key of the DPI names no page and no nodes. After a successful apply, `touches` of a page field MUST list that page. `touches` of a DPI change MUST be empty: the DPI is a document field, and no page or node `Arc` changes (owner decision, 2026-10-25). `DirtySet` therefore carries no DPI signal, and a consumer that depends on the document DPI (for example a renderer or a thumbnail cache of pages not sized in `px`) MUST compare `Document::dpi` before and after a command itself. Before it writes, `revert` MUST check that the page still exists (`MissingPage`), and that the old size still fits its pixel limits at the current DPI, or the old DPI still fits every current page (`InvalidDocument(PageSize)` otherwise).

`InsertPage` MUST insert one empty page at an index of the page list (`0..=len`; a larger index MUST fail with `IndexOutOfRange`). Its size MUST fit the pixel limits at the document DPI, or it MUST fail with `InvalidDocument(PageSize)` naming the id the page would get. It MUST allocate its id from the counter, and a redo after `revert` MUST produce the same id; an exhausted id space MUST fail with `IdSpaceExhausted` and no change. Before it writes, `InsertPage::revert` MUST check that the inserted page is still in the list (`MissingPage`), that it still equals the page it inserted (`InvalidValue`: the page was edited since, and removing it would lose that edit), and that the id counter stands exactly one above the counter it recorded (`InvalidValue` otherwise); it then removes the page and restores the counter. After a successful apply, `touches` MUST list the new page.

`RemovePage` MUST remove one page with its whole arena and restore the same page at the same index on `revert`. A missing page MUST fail with `MissingPage`. Before it writes, `RemovePage::revert` MUST check that the page is not in the list already (`InvalidValue`) and that its old index is within the list (`IndexOutOfRange`). After a successful apply, `touches` MUST list the page and every node of its arena. Removing the last page MUST be allowed (an empty document is valid).

#### Scenario: DPI pushes a page over the limit (D10, unit)
- GIVEN an A0 page at 300 DPI
- WHEN `SetProperty` sets the DPI to 351
- THEN it returns the page-size error and the DPI is still 300

#### Scenario: A DPI change touches nothing (D10, unit)
- GIVEN a document at 96 DPI with two pages
- WHEN `SetProperty` sets the DPI to 100, and separately a page name of the second page
- THEN both return `Ok(Applied::Changed)`, the DPI command's `touches` has no page and no node, and the rename's `touches` lists exactly the second page

#### Scenario: Page revert checks the size at the current DPI (D10, unit)
- GIVEN a page resized by `SetProperty`, then the DPI raised so that the old size no longer fits
- WHEN the resize is reverted
- THEN it returns `Err(InvalidDocument(PageSize { .. }))` and the document is unchanged

#### Scenario: Insert page then redo reuses its id (D10, unit)
- GIVEN a document whose counter is `k`
- WHEN `InsertPage` is applied, reverted and applied again
- THEN both applies insert a page with id `k`, the documents after them are equal, and `touches` lists that page

#### Scenario: Insert page revert refuses an edited page or a moved counter (D10, unit)
- GIVEN an applied `InsertPage`, then, in turn, the new page renamed, the new page removed, and a later `InsertNodes` that took the next id
- WHEN the `InsertPage` is reverted in each case
- THEN it returns `Err(InvalidValue)`, `Err(MissingPage)` and `Err(InvalidValue)` respectively, and the document is unchanged each time

#### Scenario: Remove page revert checks before it inserts (D10, unit)
- GIVEN an applied `RemovePage`, then, in turn, the page already back, and the page list emptied
- WHEN the `RemovePage` is reverted in each case
- THEN it returns `Err(InvalidValue)` and `Err(IndexOutOfRange)` respectively, and the document is unchanged each time

#### Scenario: Remove and restore a page (D10, unit)
- GIVEN pages `[p1, p2, p3]`, `p2` with nodes
- WHEN `RemovePage` removes `p2` and reverts
- THEN the pages are `[p1, p3]` and then `[p1, p2, p3]` with `p2` equal to the original, and after the apply `touches` lists `p2` and every node of `p2`

### Requirement: Command property laws (ARQ-81, ARQ-13) (D2 harness; D3, D8, D9, D10 per command)

`gafira-testkit` MUST provide `prop_command`, a harness that, given a generated valid document and a generated command for it, checks:
1. `apply; revert` gives a document equal to the original (02 §10);
2. `apply; revert; apply` gives a document equal to the first `apply` (02 §10);
3. after a successful `apply`, `validate()` passes;
4. `touches` covers every change, following the `DirtySet` rule: a page MUST be listed when its own fields or its `roots` differ in value, or when it exists on one side only (a change confined to its nodes does not require the page); a node MUST be listed when its `Arc` is not pointer-equal to the original, or when it exists on one side only; and every listed page or node exists in the original or in the result;
5. on `Err`, the document is equal to the original and every page `Arc` is pointer-equal to the original's;
6. for commands that merge, applying the merged command equals applying the sequence (02 §10). The law runs only when `merge` returns `Ok` and both commands apply in sequence.

Laws 1 and 2 MUST also run when `apply` returns `Unchanged`. The laws cover generated valid documents only.

Known limits of law 4, accepted: a listed item is only checked to exist, so a command that lists something it did not change passes (the law is one-sided); and a node whose `Arc` changes by copy on write without a change in value counts as changed. A document field such as the DPI is outside the law: a DPI change replaces no page or node `Arc`, so its empty `touches` passes law 4, and consumers that depend on the DPI compare `Document::dpi` themselves ("Page and document commands").

Law 5 and `Transaction`: a transaction whose child fails restores an equal document, not the original page `Arc`s ("Transaction"), so law 5 cannot hold for a failing transaction. The `Transaction` property MUST therefore generate only children that cannot fail on any document the transaction can start from; the rollback is covered by the `Transaction` unit tests, which check value equality.

Every core command MUST have a test in `crates/gafira-edit/tests/prop_commands.rs` that runs the harness with 1 000 cases by default (`PROPTEST_CASES` may raise it). `Transaction` is a core command for this rule (it implements `Command` under `crates/gafira-edit/src`, so V14 lists it). V14 checks the list (`workspace-tooling`).

#### Scenario: Harness catches a wrong revert (D2, unit)
- GIVEN a deliberately broken test command whose `revert` forgets one field
- WHEN `prop_command` runs on it
- THEN law 1 fails with a minimized counterexample

#### Scenario: Harness catches an inexact touches (D2, unit)
- GIVEN a test command that changes a node but reports an empty `DirtySet`
- WHEN `prop_command` runs
- THEN law 4 fails

#### Scenario: A change confined to a node does not require its page (D2, unit)
- GIVEN a sound test command that changes one node's record and lists only that node
- WHEN `prop_command` runs
- THEN every law passes, although the page `Arc` changed by copy on write

#### Scenario: A clean refusal passes law 5 (D2, unit)
- GIVEN a sound test command that returns `Err` before it touches the document
- WHEN `prop_command` runs
- THEN every law passes

#### Scenario: Every core command passes the laws (D3, D8, D9, D10, property)
- GIVEN `prop_commands.rs`
- WHEN `cargo test -p gafira-edit --test prop_commands --locked` runs
- THEN each core command's property passes 1 000 cases, `Transaction`'s included
