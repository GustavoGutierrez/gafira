# Edit History Specification

## Purpose

`History` in `gafira-edit`: the owner of the session document, the only place where commands run against it, and the undo/redo stack built on immutable `Arc<Document>` snapshots (Command + Memento, ADR-03), with gesture merging and a memory budget. The `Command` contract and the core commands live in `edit-commands`. `EditService`, the event bus and `Selection` are out of scope (application layer; Q5).

Open decisions are in `design.md`; a requirement that depends on one names it ("pending Qn"). Scenario types: unit, property, integration, command, manual.

## ADDED Requirements

### Requirement: History owns the session document (ARQ-30) (D4, pending Q2)

`History` MUST own the current document as an `Arc<Document>`, built from a validated document. It MUST expose it only as `document(&self) -> &Arc<Document>` (and read-only views). No public API of `gafira-edit` MAY return `&mut Document`, a `DocMut`, or an `EditToken` (by value or by reference). `History` MUST be `Send + Sync` when every stored command is (always, by the `Command` bounds).

#### Scenario: Readers share the snapshot (D4, unit)
- GIVEN a history and `let a = history.document().clone()`
- WHEN a command is executed and `history.document()` is read again
- THEN `a` still equals the old state, and the two `Arc`s are not pointer-equal

#### Scenario: Send and Sync (D4, unit)
- GIVEN `fn assert_send_sync<T: Send + Sync>()`
- WHEN it is instantiated with `History`
- THEN the test compiles (no thread is spawned)

### Requirement: Atomic execute with validation in debug builds (ARQ-13, ARQ-30) (D4)

`History::execute(cmd: Box<dyn Command>, at: Duration) -> Result<Applied, CommandError>` MUST apply the command to a copy of the current document obtained through `Arc::make_mut` on a clone of the `Arc` (so only the edited path is copied) and through the private token. On `Ok(Changed)` it MUST install the new document, push an entry and clear the redo stack. On `Ok(Unchanged)` it MUST change nothing, push nothing and keep the redo stack (pending Q14). On `Err` it MUST drop the copy and leave the current `Arc` pointer-equal to before. After every successful apply, undo and redo it MUST run `debug_assert!(doc.validate().is_ok())` (ARQ-13). `at` is a monotonic time chosen by the caller (pending Q3); `History` MUST NOT read a clock.

#### Scenario: Failure leaves the Arc untouched (D4, unit)
- GIVEN a history and `let before = history.document().clone()`
- WHEN a command that fails is executed
- THEN `Arc::ptr_eq(&before, history.document())` holds and `len()` is unchanged

#### Scenario: New change clears redo (D4, unit)
- GIVEN two executed commands and one undo
- WHEN a third command is executed
- THEN `can_redo()` is false

#### Scenario: Unchanged is not recorded (D4, unit, pending Q14)
- GIVEN a history after one undo
- WHEN a command that returns `Unchanged` is executed
- THEN `len()` is unchanged and `can_redo()` is still true

#### Scenario: A buggy command trips the assert in debug (D4, unit)
- GIVEN a test-only command that leaves an orphan node and returns `Changed`
- WHEN it is executed in a debug test build
- THEN the test panics with the `debug_assert` message (`#[should_panic]`, gated on `debug_assertions`)

### Requirement: Undo and redo by snapshot (ADR-03, ARQ-32) (D4)

Each entry MUST hold the command, the snapshot before it (`Arc<Document>`) and, while the entry is the newest or the next one still holds its snapshot, the state after it. `undo()` MUST restore the before-snapshot of the newest undone-able entry and `redo()` MUST restore its after-state, both by installing a stored `Arc`, without calling any `Command` method, for every entry that still holds its snapshots. Both MUST return the entry's `CommandLabel` (or `None` when there is nothing to undo or redo, with no change). `len()` MUST count the entries that can be undone; `can_undo()` and `can_redo()` MUST report the stacks. The structural proof stands in for ARQ-32's timed criterion, which is deferred to the ARQ-40 benchmarks (pending Q8).

#### Scenario: Undo restores the exact Arc (D4, unit)
- GIVEN `let s0 = history.document().clone()` and one executed command
- WHEN `undo()` runs
- THEN `Arc::ptr_eq(&s0, history.document())` holds and it returned the command's label

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
- THEN both return `None` and the document is pointer-equal to before

### Requirement: Preview without history (D4)

`History::preview(&self, cmd: Box<dyn Command>) -> Result<Arc<Document>, CommandError>` MUST apply the command to a copy of the current document and return the result without installing it or touching the stacks (02 §4.2 `ToolOutcome::Preview`). The current document MUST stay pointer-equal to before.

#### Scenario: Preview leaves the session alone (D4, unit)
- GIVEN a history
- WHEN a placement change is previewed
- THEN the returned document shows the change, `history.document()` is pointer-equal to before, and `len()` is unchanged

### Requirement: Gesture merge (ARQ-33, ADR-03) (D6, pending Q3)

`History::begin_gesture()` MUST return a fresh `GestureId` and mark it open; `end_gesture()` MUST close it. When the newest entry and the new command have equal `merge_key`s, and either (a) both carry the open gesture's id, or (b) the new `at` is less than 500 ms after the newest entry's last `at`, `execute` MUST call `merge` on the newest entry's command. On `Ok` the entry MUST keep its before-snapshot, take the new after-state and the new `at`, and `len()` MUST NOT grow. On `Err(next)` `next` MUST become a new entry. A merge MUST NOT happen into an entry that was undone, across a closed gesture, or when either key is `None`. The merged entry MUST undo to the state before the first merged command.

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

#### Scenario: Merging equals the sequence (D6, property)
- GIVEN generated sequences of equal-key `SetProperty` commands inside one gesture
- WHEN they are executed and compared with applying them one by one to a copy
- THEN the documents are equal, and one undo returns the original

### Requirement: Memory budget and compaction (ADR-03) (D7, pending Q7)

`History` MUST take a budget in bytes at construction, default 512 MiB (536 870 912 bytes, pending Q7 unit; FIJADA figure in ADR-03). It MUST keep a deterministic estimate per entry: the bytes of the pages and nodes its snapshot holds that are not pointer-equal to the same items in the next newer state, using a fixed size model (record size plus string and extension-payload heap lengths). When the sum exceeds the budget after an `execute`, it MUST compact the oldest entries first, dropping their snapshots and keeping their commands, until the sum fits or only the newest entry keeps its snapshots. Undo of a compacted entry MUST call `revert` on a copy of the current document and redo MUST call `apply`, with the same atomic rule as `execute`: on `Err` the document and the stacks MUST stay unchanged and the error MUST be returned. The estimate MUST be exposed read-only for tests (`estimated_bytes()`).

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

#### Scenario: Shared data is not double counted (D7, unit)
- GIVEN a document with 100 nodes and ten commands that each change one node
- WHEN `estimated_bytes()` is read
- THEN it is less than two full-document estimates (each entry counts only what it does not share)

### Requirement: Ids after undo (D8, pending Q6)

After an undo, a new command MAY be issued ids that an abandoned redo branch had used: undo restores the snapshot's `next_id`. Within every document state ids MUST stay unique (`validate()`).

#### Scenario: Reissued id after undo (D8, unit)
- GIVEN `next_id = 10`, an `InsertNodes` that used id 10, and an undo
- WHEN another `InsertNodes` is executed
- THEN it receives id 10 and `validate()` passes
