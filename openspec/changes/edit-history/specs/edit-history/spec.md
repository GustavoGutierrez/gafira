# Edit History Specification

## Purpose

`History` in `gafira-edit`: the owner of the session document, the only place where commands run against it, and the undo/redo stack built on immutable `Arc<Document>` snapshots (Command + Memento, ADR-03), with gesture merging and a memory budget. The `Command` contract and the core commands live in `edit-commands`. `EditService`, the event bus and `Selection` are out of scope (application layer; Q5).

Decisions are closed in `design.md`; a requirement that depends on one names it ("Qn"). Scenario types: unit, property, integration, command, manual.

## ADDED Requirements

### Requirement: History owns the session document (ARQ-30) (D4, D7, Q2)

`History` MUST own the current document as an `Arc<Document>`, built from a validated document: `History::new(doc: Document) -> Result<Self, ValidationError>` and `History::with_budget(doc: Document, budget: usize) -> Result<Self, ValidationError>` MUST run `validate()` and return its error for an invalid document (D4, D7). It MUST expose it only as `document(&self) -> &Arc<Document>` (and read-only views). No public API of `gafira-edit` MAY return `&mut Document`, a `DocMut`, or an `EditToken` (by value or by reference). `History` MUST be `Send + Sync` when every stored command is (always, by the `Command` bounds).

#### Scenario: Readers share the snapshot (D4, unit)
- GIVEN a history and `let a = history.document().clone()`
- WHEN a command is executed and `history.document()` is read again
- THEN `a` still equals the old state, and the two `Arc`s are not pointer-equal

#### Scenario: Invalid document refused (D4, unit)
- GIVEN a document whose DPI is 0
- WHEN `History::new` is called with it
- THEN it returns `Err(ValidationError)`

#### Scenario: Send and Sync (D4, unit)
- GIVEN `fn assert_send_sync<T: Send + Sync>()`
- WHEN it is instantiated with `History`
- THEN the test compiles (no thread is spawned)

### Requirement: Atomic execute with validation in debug builds (ARQ-13, ARQ-30) (D4)

`History::execute(cmd: Box<dyn Command>, at: Duration) -> Result<Applied, CommandError>` MUST apply the command to a copy of the current document obtained through `Arc::make_mut` on a clone of the `Arc` (so only the edited path is copied) and through the private token. On `Ok(Changed)` it MUST install the new document, push an entry and clear the redo stack. On `Ok(Unchanged)` it MUST change nothing, push nothing and keep the redo stack (Q14). On `Err` it MUST drop the copy and leave the current `Arc` pointer-equal to before. After every successful apply, undo and redo it MUST run `debug_assert!(doc.validate().is_ok())` (ARQ-13). `at` is a monotonic time chosen by the caller (Q3); `History` MUST NOT read a clock.

#### Scenario: Failure leaves the Arc untouched (D4, unit)
- GIVEN a history and `let before = history.document().clone()`
- WHEN a command that fails is executed
- THEN `Arc::ptr_eq(&before, history.document())` holds and `len()` is unchanged

#### Scenario: New change clears redo (D4, unit)
- GIVEN two executed commands and one undo
- WHEN a third command is executed
- THEN `can_redo()` is false

#### Scenario: Unchanged is not recorded (D4, unit, Q14)
- GIVEN a history after one undo
- WHEN a command that returns `Unchanged` is executed
- THEN `len()` is unchanged and `can_redo()` is still true

#### Scenario: A buggy command trips the assert in debug (D4, unit)
- GIVEN a test-only command that leaves an orphan node and returns `Changed`
- WHEN it is executed in a debug test build
- THEN the test panics with the `debug_assert` message (`#[should_panic]`, gated on `debug_assertions`)

### Requirement: Undo and redo by snapshot (ADR-03, ARQ-32) (D4, D7)

Each entry MUST hold the command, the snapshot before it (`Arc<Document>`) and, while the entry is the newest or the next one still holds its snapshot, the state after it. `undo()` MUST restore the before-snapshot of the newest undone-able entry and `redo()` MUST restore its after-state, both by installing a stored `Arc`, without calling any `Command` method, for every entry that still holds its snapshots. Both MUST have the signature `fn(&mut self) -> Result<Option<CommandLabel>, CommandError>`: `Ok(Some(label))` with the entry's label, `Ok(None)` when there is nothing to undo or redo (with no change), and `Err` only on the compacted path of "Memory budget and compaction" (an entry that holds its snapshots never fails). `len()` MUST count the entries that can be undone; `can_undo()` and `can_redo()` MUST report the stacks. The structural proof stands in for ARQ-32's timed criterion, which is deferred to the ARQ-40 benchmarks (Q8).

#### Scenario: Undo restores the exact Arc (D4, unit)
- GIVEN `let s0 = history.document().clone()` and one executed command
- WHEN `undo()` runs
- THEN `Arc::ptr_eq(&s0, history.document())` holds and it returned `Ok(Some(label))` with the command's label

#### Scenario: Undo calls no command method (D4, unit)
- GIVEN a test command that counts calls to `apply` and `revert`, executed once
- WHEN `undo()` and `redo()` run 1 000 times alternately
- THEN the counters still read one `apply` and zero `revert`

#### Scenario: Cost does not grow with the document (D4, property)
- GIVEN generated documents from 1 to the generator's maximum number of nodes, one command each
- WHEN `undo()` and `redo()` run
- THEN each installs a stored `Arc` (pointer-equal to the recorded snapshot) and calls no command method, whatever the size

#### Scenario: Nothing to undo (D4, unit)
- GIVEN a fresh history
- WHEN `undo()` and `redo()` run
- THEN both return `Ok(None)` and the document is pointer-equal to before

### Requirement: Preview without history (D4)

`History::preview(&self, cmd: Box<dyn Command>) -> Result<Arc<Document>, CommandError>` MUST apply the command to a copy of the current document and return the result without installing it or touching the stacks (02 §4.2 `ToolOutcome::Preview`). The current document MUST stay pointer-equal to before.

#### Scenario: Preview leaves the session alone (D4, unit)
- GIVEN a history
- WHEN a placement change is previewed
- THEN the returned document shows the change, `history.document()` is pointer-equal to before, and `len()` is unchanged

### Requirement: Gesture merge (ARQ-33, ADR-03) (D6, Q3)

`History::begin_gesture()` MUST return a fresh `GestureId` and mark it open; `end_gesture()` MUST close it. At most one gesture is open: `begin_gesture()` while a gesture is open MUST close that gesture and open the new one. When the newest entry and the new command have equal `merge_key`s, the gesture in the key decides:
- a key that carries a `GestureId` MUST merge only while that gesture is the open one, with no time limit; once its gesture is closed it MUST NOT merge, even within 500 ms;
- a key without a `GestureId` MUST merge only when the new `at` is less than 500 ms after the newest entry's last `at`, whether or not a gesture is open. The gap is computed saturating at zero: an `at` earlier than the entry's last `at` counts as 0 ms apart (callers pass a monotonic time, Q3).

When the rule allows it, `execute` MUST call `merge` on the newest entry's command. On `Ok` the entry MUST keep its before-snapshot, take the new after-state and the new `at`, and `len()` MUST NOT grow. On `Err(next)` `next` MUST become a new entry. A merge MUST NOT happen when either key is `None`. An `undo()` or a `redo()` MUST end the merge run: the next command MUST NOT merge into any entry, neither the undone one nor an older entry still on the undo stack, until it has become a new entry itself (stricter than forbidding only the undone entry). The merged entry MUST undo to the state before the first merged command, and it MUST be kept even when its before and after states are equal.

#### Scenario: A 300-event drag is one entry (D6, unit)
- GIVEN a history with `len() = k` and an open gesture
- WHEN 300 `SetProperty` placement commands for the same node and gesture are executed, then the gesture ends
- THEN `len()` is `k + 1`, and one `undo()` restores the placement before the drag

#### Scenario: Arrow keys within 500 ms merge (D6, unit)
- GIVEN no open gesture
- WHEN two equal-key nudges are executed at 1 000 ms and 1 499 ms, and a third at 2 000 ms
- THEN the first two form one entry and the third is a new entry (501 ms later)

#### Scenario: Different keys never merge (D6, unit)
- GIVEN an open gesture
- WHEN a placement change of node a and one of node b are executed
- THEN `len()` grows by 2

#### Scenario: No merge after undo (D6, unit)
- GIVEN one nudge, an undo, and another equal-key nudge 100 ms later
- WHEN `len()` is read
- THEN the new nudge is its own entry and redo is cleared

#### Scenario: A gesture has no time limit (D6, unit)
- GIVEN an open gesture
- WHEN 300 equal-key commands of that gesture are executed 10 s apart from each other, then the gesture ends
- THEN they form one entry, and one `undo()` restores the state before the first

#### Scenario: A closed gesture never merges while another is open (D6, unit)
- GIVEN `g1 = begin_gesture()`, one command with key gesture `g1`, then `g2 = begin_gesture()` with no `end_gesture()` between
- WHEN a command with the same key and gesture `g1` is executed
- THEN `len()` grows by 1 (no merge)

#### Scenario: Undo ends the merge run (D6, unit)
- GIVEN one open gesture, a command on lane 1 and a command on lane 2 (different keys) 10 ms apart, then an undo
- WHEN another lane 1 command with the same key and gesture is executed 10 ms later
- THEN it is a new entry and does not merge into the lane 1 entry still on the undo stack

#### Scenario: Merging equals the sequence (D6, property)
- GIVEN generated sequences of equal-key `SetProperty` commands inside one gesture
- WHEN they are executed and compared with applying them one by one to a copy
- THEN the documents are equal, and one undo returns the original

### Requirement: Memory budget and compaction (ADR-03) (D7, Q7)

`History` MUST take a budget in bytes at construction, default 512 MiB (536 870 912 bytes, Q7 unit; FIJADA figure in ADR-03): `History::with_budget(doc, budget)` takes it, and `History::new(doc)` uses the public constant `DEFAULT_BUDGET` (536 870 912), exported by `gafira-edit`. A budget of `usize::MAX` never compacts. It MUST keep a deterministic estimate per entry: the bytes of the pages and nodes its snapshot holds that are not pointer-equal to the same items in the next newer state, using a fixed size model (record size plus string and extension-payload heap lengths), plus a fixed document record per entry (the document record and its list of page handles), so every entry that holds snapshots counts more than zero and a budget of 0 compacts every entry but the newest. An entry's estimate depends only on its own before and after states; an undone entry (on the redo stack) keeps the estimate it had, and its after-state is not counted again. `estimated_bytes()` MUST be the sum over the undo and redo stacks. When the sum exceeds the budget after an `execute` that recorded or merged a `Changed` command (undo, redo, and `Unchanged` or failed commands never compact), it MUST compact the oldest entries first, dropping their snapshots and keeping their commands, until the sum fits or only the newest entry keeps its snapshots. Undo of a compacted entry MUST call `revert` on a copy of the current document and redo MUST call `apply`, with the same atomic rule as `execute`: on `Err` the document and the stacks MUST stay unchanged and the error MUST be returned. A compacted redo whose `apply` returns `Unchanged` MUST count as success (the document then already holds the result), and the entry MUST move back to the undo stack, still compacted. The estimate MUST be exposed read-only for tests (`estimated_bytes()`). The estimate is a lower bound: measured on documents of 100 nodes and 20 pages it under-counts real heap by 1.2x to 2.5x and never over-counts (spike R9). If the spec measures with a counting allocator, `dhat` and `stats_alloc` have OK licenses under the current `deny.toml`.

#### Scenario: Small budget compacts the oldest (D7, unit)
- GIVEN a budget that fits about two snapshots and ten executed commands
- WHEN `estimated_bytes()` and each entry's compaction state are read
- THEN the sum is within the budget and only the oldest entries are compacted

#### Scenario: Undo through revert reaches the same states (D7, property)
- GIVEN a generated document, a generated sequence of core commands, and the snapshots recorded with an unlimited budget
- WHEN the same sequence runs with a budget of 0 and is undone to the start and redone to the end
- THEN every intermediate document equals the recorded snapshot of the same step

#### Scenario: Failed revert changes nothing (D7, unit)
- GIVEN a compacted entry whose test command's `revert` fails
- WHEN `undo()` runs
- THEN it returns the error, the document is pointer-equal to before, and `can_undo()` is still true

#### Scenario: Compacted redo reported as Unchanged succeeds (D7, unit)
- GIVEN a compacted entry whose command is a drag that ends where it started, undone through `revert`
- WHEN `redo()` runs and its `apply` returns `Unchanged`
- THEN it returns `Ok(Some(label))`, `can_undo()` is true, and the entry is still compacted

#### Scenario: Undone entries keep their estimate (D7, unit)
- GIVEN an unlimited budget and one executed command
- WHEN `undo()` and then `redo()` run
- THEN `estimated_bytes()` is the same positive value after each

#### Scenario: Shared data is not double counted (D7, unit)
- GIVEN a document with 100 nodes and ten commands that each change one node
- WHEN `estimated_bytes()` is read
- THEN it is less than two full-document estimates (each entry counts only what it does not share)

### Requirement: Ids after undo (D8, Q6)

After an undo, a new command MAY be issued ids that an abandoned redo branch had used: undo restores the snapshot's `next_id`. Within every document state ids MUST stay unique (`validate()`).

#### Scenario: Reissued id after undo (D8, unit)
- GIVEN `next_id = 10`, an `InsertNodes` that used id 10, and an undo
- WHEN another `InsertNodes` is executed
- THEN it receives id 10 and `validate()` passes
