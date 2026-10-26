# Design: Edit History (C3, open decisions only)

This note covers only the decisions that `docs/specs/02-arquitectura-software.md` and `docs/specs/00-decisiones.md` leave open for `gafira-edit`. Everything else follows 02 §4.3, §5.1 (ADR-03) and the C2b constraints ("Decisions that constrain C3" in the archived `domain-doc` design).

Status: every question is CLOSED. The owner closed Q4, Q5, Q7 (unit) and Q12 on 2026-10-24; the coordinator closed the technical ones (Q1 to Q3, Q6, Q7 method, Q8 to Q11, Q13 to Q15) on 2026-10-24 with the recommendation recorded in each question, adjusted by the explorer spike (`spike.md`). The option in bold in each question is the decision; the delta specs implement it. A decision changes only through the owner (owner-level questions) or a new design note.

## Summary

| # | Question | Decision | Closed by |
|---|---|---|---|
| Q1 | What `Command::apply` receives | `&mut DocMut<'_>` | CLOSED: coordinator, 2026-10-24 (amends 02 §4.3) |
| Q2 | Who owns the session document | `History` owns an `Arc<Document>` | CLOSED: coordinator, 2026-10-24 |
| Q3 | Time source for the 500 ms merge window | caller passes a monotonic `Duration` | CLOSED: coordinator, 2026-10-24 |
| Q4 | Page and document commands (not in the 02 list) | add `InsertPage`, `RemovePage`, page and DPI targets of `SetProperty` | CLOSED: owner, 2026-10-24 (option A) |
| Q5 | `Selection` in this change | defer | CLOSED: owner, 2026-10-24 (option A) |
| Q6 | Ids reissued after undo | allowed | CLOSED: coordinator, 2026-10-24 |
| Q7 | How the 512 MiB budget is counted, and the unit | estimate by `Arc` identity; 512 MiB | CLOSED: owner, 2026-10-24 (unit and option A); coordinator for the method |
| Q8 | Evidence for ARQ-32 | structural proof now, timed benchmark with ARQ-40 | CLOSED: coordinator, 2026-10-24 |
| Q9 | ARQ-81 check before `xtask` exists | CI step V14 | CLOSED: coordinator, 2026-10-24 |
| Q10 | `gafira-testkit` now | create it, generator moved from `gafira-doc` | CLOSED: coordinator, 2026-10-24 |
| Q11 | `CommandLabel` before the `msg` module exists | add the minimal `gafira-doc::msg::MessageKey` | CLOSED: coordinator, 2026-10-24 |
| Q12 | Do commands enforce `locked` | no, tools do | CLOSED: owner, 2026-10-24 (option A) |
| Q13 | What `trybuild` can prove for ARQ-30 | no-token paths fail, external `Command` passes; constructor stays V13 | CLOSED: coordinator, 2026-10-24 (amends ARQ-30 wording) |
| Q14 | No-op commands in the history | not recorded | CLOSED: coordinator, 2026-10-24 |
| Q15 | Validation in release builds | `debug_assert` only, commands check their own inputs | CLOSED: coordinator, 2026-10-24 |

## Open Questions

### Q1. What `Command::apply` receives

**CLOSED** by coordinator, 2026-10-24 (amends 02 §4.3). Spike R11: `merge` probes the incoming command by reference before upcasting (see `spike.md`).

02 §4.3 writes `apply(&mut self, doc: &mut Document)`. Since C2b, a `&mut Document` grants nothing below the root: fields are private and fine-grained mutation needs `Document::edit(&EditToken) -> DocMut`.

| Option | Effect |
|---|---|
| A. `&mut Document` (02 literally) | Core commands call `doc.edit(&TOKEN)` inside `gafira-edit`. A command from another crate gets `&mut Document` but can only replace the whole value (`*doc = other`). Extension commands (02 §4.3 "extensions bring their own") are crippled. |
| **B. `&mut DocMut<'_>`** | Every command, core or extension, mutates through the same capability, and only while `History` runs it. No new public type. 02 §4.3 changes one parameter type. |
| C. A `gafira-edit` wrapper (`EditCx`) around `DocMut` | Same power as B, plus room for context (selection, clock). One more public type to keep stable. |

Decision: **B**. `revert` takes the same type. Amendment to 02 §4.3, applied in its own commit.

### Q2. Who owns the session document

**CLOSED** by coordinator, 2026-10-24.

| Option | Effect |
|---|---|
| **A. `History` owns `Arc<Document>`** and exposes `document() -> &Arc<Document>` | Only `gafira-edit` ever holds the token. `EditService` (engine, later) wraps `History`. Snapshots, undo and preview stay in one place. |
| B. `EditService` owns the document and lends `&mut` to `History` | The engine would need the token or a `DocMut`, which V13 forbids outside the edit crates. |

Decision: **A**.

### Q3. Time source for the 500 ms merge window

**CLOSED** by coordinator, 2026-10-24.

ADR-03 merges equal `merge_key`s inside one gesture, or less than 500 ms apart (arrow keys, sliders). The domain `clippy.toml` bans `Instant::now` and `SystemTime::now` in every target, tests included.

| Option | Effect |
|---|---|
| **A. The caller passes a monotonic `std::time::Duration` (since any epoch it chooses) on each `execute`** | Deterministic, testable with plain values. The engine reads its injected `Clock` port. |
| B. The caller passes `std::time::Instant` | Tests cannot build an `Instant` without `Instant::now`, which clippy rejects in tests. |
| C. Merge only inside explicit gestures | Drops the 500 ms rule of ADR-03; a deviation the holder would have to approve. |

Decision: **A**. `History` never reads a clock. Gestures are explicit: `begin_gesture() -> GestureId` and `end_gesture()`.

### Q4. Page and document commands 

**CLOSED** by owner, 2026-10-24 (option A).

02 §4.3 lists `SetProperty<T>`, `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent`, `SetFrameContent`, `PaintMask`, `ApplyTemplate`, `ResizeDesign`, `SplitClip`, `TrimClip`, `MoveClip`, `SetKeyframe` and `SetSwatch`. The current model (`Document`, `Page`, `Group`, `Shape`, `Extension`) supports only the first five. No listed command adds or removes a page, and C2b constraint 5 says a DPI change must validate or be rejected.

| Option | Effect |
|---|---|
| **A. Add `InsertPage` and `RemovePage`, and let `SetProperty` target page fields (`name`, `size`, `locked`, `hidden`) and the document DPI** | Every field of today's model is editable through a command. Amends the 02 §4.3 list. Costs one delivery (D10). |
| B. `SetProperty` page and DPI targets only, no page insert or remove | Smaller. Pages can still not be created through the history. |
| C. Node commands only | Smallest. Page fields stay editable only through raw `DocMut`, which nothing outside `gafira-edit` may use. |

Decision: **A**, built as the last delivery (D10).

### Q5. `Selection` in this change 

**CLOSED** by owner, 2026-10-24 (option A).

02 §2.1 places `Selection` in `gafira-edit`, with no ARQ of its own. Its only figure (selection < 16 ms, §6.7) needs bounds and an R-tree (`rstar`, VERIFICAR), which need `NodeBehavior`, which does not exist yet. `SelectionService` lives in the engine.

| Option | Effect |
|---|---|
| **A. Defer** to the change that adds hit testing or `EditService` | C3 stays on commands and history. Undo does not restore selection yet (no consumer exists). |
| B. A minimal `Selection` value now (ordered node ids per page, pruned when nodes are removed) | ~250 more lines with no consumer and no measurable ARQ. |

Decision: **A**.

### Q6. Ids reissued after undo

**CLOSED** by coordinator, 2026-10-24.

C2b constraint 3: a snapshot restore also restores `next_id`, so after undo and a new edit, an id from the abandoned redo branch can be issued again.

| Option | Effect |
|---|---|
| **A. Allowed** | Ids stay unique inside every document state, which is what `validate` checks. No change to `gafira-doc`. Caches must key by content (02 §6.1 already does). |
| B. Monotonic across undo | Needs a new `DocMut` operation in `gafira-doc` (for example `reserve_ids(min)`) and a `next_id` that differs from the snapshot, so undo is no longer an exact restore. |

Decision: **A**. Revisit when the render cache or the selection keys by `NodeId`.

### Q7. How the history budget is counted, and its unit

**CLOSED** by owner, 2026-10-24 (unit and option A); coordinator for the method. Spike R9: the estimate is a lower bound (it under-counts real heap by 1.2x to 2.5x and never over-counts). Unit: 512 MiB = 536 870 912 bytes.

ADR-03 fixes the budget at "512 MB, FIJADA" and says old snapshots are compacted (only commands remain) when it is exceeded. It does not say how memory is counted. Shared `Arc`s make a naive sum wrong, and measuring real heap needs a counting global allocator, which needs `unsafe` (forbidden) or a new dependency.

Method options:

| Option | Effect |
|---|---|
| **A. A deterministic estimate per entry**: the bytes of the pages and nodes its snapshot holds that are not pointer-equal to the same items in the next newer state, using a fixed size model (`size_of` of the record plus the heap length of strings and extension payloads) | Deterministic, testable, cheap (O(nodes touched) with `Arc::ptr_eq` per page and per node). An estimate, not real heap. |
| B. Real allocation through a counting allocator in a test harness | Accurate, only in tests; the runtime still needs a model. Needs a dependency (spike R9). |
| C. Count entries instead of bytes | Ignores the FIJADA figure. |

Unit options: 512 MiB (536 870 912 bytes) or 512 MB (512 000 000 bytes).

Decision: method **A** (compared once against B in the spike, R9); unit **MiB** (536 870 912 bytes), closed by the owner. The budget is a constructor parameter with that default, so tests use a tiny one.

### Q8. Evidence for ARQ-32 (undo and redo O(1))

**CLOSED** by coordinator, 2026-10-24.

ARQ-32's criterion is timed (100 layers, 1 000 undos, p99 < 16 ms). A timed test cannot live in `gafira-edit`: `Instant::now` is banned in all its targets. There is no benchmark harness yet (ARQ-40, `perf-budgets.toml`).

| Option | Effect |
|---|---|
| **A. Structural proof now, timing later** | Tests show that undo and redo of an entry that still holds its snapshot call no command method and swap one `Arc` (pointer-equal to the stored snapshot), for any document size. The timed criterion is recorded as deferred to the ARQ-40 benchmark change. |
| B. A timed test in `apps/gafira-cli` (no domain `clippy.toml`) | Real timing now, on CI runners that are not the reference hardware, so the 16 ms figure is noise. |

Decision: **A**.

### Q9. ARQ-81 check before `xtask` exists

**CLOSED** by coordinator, 2026-10-24. Spike R13: V5 does not enforce that `trybuild` and `proptest` are dev-only; the D5 `cargo tree -i` scenario does.

ARQ-81: every core `Command` has a property test, checked by `xtask` against `prop_commands.rs`. `xtask` does not exist.

| Option | Effect |
|---|---|
| **A. CI step V14** in the `policy` job: list `impl Command for <Type>` under `crates/gafira-edit/src`, fail when a type is missing from `crates/gafira-edit/tests/prop_commands.rs`; positive control and explicit exit codes, as in V13 | Enforced now; replaced by `xtask` later. Textual, with known limits (macros, generic impls). |
| B. Review only until `xtask` | No enforcement. |

Decision: **A**.

### Q10. `gafira-testkit` now

**CLOSED** by coordinator, 2026-10-24. Spike R8: `mod guards` of the moved generator gets `#[cfg(test)]`.

C2b constraint 7: the generator lives in `crates/gafira-doc/tests/common` and C3 moves it to `gafira-testkit` (an allowed V13 path). 02 §4.3 names `gafira-testkit::prop_command`.

| Option | Effect |
|---|---|
| **A. Create `gafira-testkit`**: the generator moved with `git mv`, plus the `prop_command` harness. After the move, `mod guards` MUST get `#[cfg(test)]` (else clippy reports dead code in the library); the two guard tests then run in a separate binary (~6 s on Linux). Normal dependencies `gafira-doc`, `gafira-edit`, `proptest`. Used only as a dev-dependency by the integration tests of `gafira-doc` and `gafira-edit`. A V5 row, `#![forbid(unsafe_code)]` (V11), and **no** domain `clippy.toml` (it will read golden files later) | One home for generators. The dev-dependency cycle works for integration tests only (spike R7). |
| B. Copy the generator into `gafira-edit/tests/common` | Two generators drift apart. |

Decision: **A**. Its V5 row exists although V5's coverage check only requires rows for crates with a `clippy.toml`.

### Q11. `CommandLabel` before the `msg` module exists

**CLOSED** by coordinator, 2026-10-24.

02 §14.2 defines `MessageKey(pub &'static str)` and `type CommandLabel = MessageKey` in `gafira-doc::msg`. C2b left `msg` out.

| Option | Effect |
|---|---|
| **A. Add the minimal `gafira-doc::msg`**: `MessageKey` and the `CommandLabel` alias exactly as 02 §14.2, nothing else | The final type from day one. Touches `gafira-doc` (one small ADDED requirement in `document-model`). |
| B. A temporary `CommandLabel(&'static str)` in `gafira-edit` | A breaking change later. |

Decision: **A**. `Message`, `MsgArg`, `ToMessage` and `msg!` stay for the i18n change.

### Q12. Do commands enforce `locked` 

**CLOSED** by owner, 2026-10-24 (option A).

`ElementBase::locked` and `Page::locked` exist; locks do not cascade in the data (C2b). 02 does not say whether the domain rejects edits of locked items or whether tools refuse them.

| Option | Effect |
|---|---|
| **A. Commands ignore locks**; tools and services decide | Unlocking is an ordinary `SetProperty`. Scripts, templates and tests can still edit. |
| B. Commands reject edits of locked nodes and pages, except a change of `locked` itself | Domain-level guarantee; every command needs the check and a new error. |

Decision: **A**.

### Q13. What `trybuild` can prove for ARQ-30

**CLOSED** by coordinator, 2026-10-24 (amends ARQ-30 wording). Spike R3: identical `.stderr` snapshots on Windows and macOS stay unverified until the D5 CI run; the spec carries a Linux-only fallback.

ARQ-30: "`Document` exposes `&mut` only inside `gafira-edit`. A `trybuild` test shows that a mutation from `gafira-ui` does not compile." Since C2b, the token constructor `__new_for_gafira_edit_only` is `pub` and `#[doc(hidden)]`: any crate can call it, so no fixture can show that building a token fails. `gafira-ui` does not exist yet.

| Option | Effect |
|---|---|
| **A. Fixtures act as an outside crate that depends on `gafira-doc` and `gafira-edit`.** Compile-fail: no `&mut Document` from `History`; assigning through `&Document` or `&Arc<Document>`; `Document::edit` without a token; no field access. Pass: an outside `Command` implementation executes through `History` (the legitimate path, ARQ-20). The hidden constructor stays covered by V13 and review | Honest evidence; ARQ-30 wording amended from "`EditService::execute`" to "`History::execute` in `gafira-edit` (wrapped by `EditService` later)", and from "`pub(crate)`" to "the `EditToken` capability". |
| B. Make token construction impossible outside `gafira-edit` | Not possible with a public constructor in `gafira-doc`; features unify across the workspace (C2b A11). |

Decision: **A**.

### Q14. No-op commands in the history

**CLOSED** by coordinator, 2026-10-24.

`Applied` is unspecified in 02. Setting a property to its current value changes nothing.

| Option | Effect |
|---|---|
| **A. `Applied::{Changed, Unchanged}`; `History` records only `Changed`** | "Undo" never undoes nothing. Redo is not cleared by a no-op. |
| B. Always record | Simpler; empty undo steps. |

Decision: **A**.

### Q15. Validation in release builds

**CLOSED** by coordinator, 2026-10-24.

ARQ-13 asks for `debug_assert` after each command. Full `validate()` is O(n) per command.

| Option | Effect |
|---|---|
| **A. `debug_assert!` only, and each command checks its own inputs** (target exists, container kind, index range, finite and in-range values, no cycle, page size at the DPI) and returns `CommandError` with the document unchanged | O(touched) in release; the assert catches command bugs in tests. |
| B. Run `validate()` in release too and reject on failure | Safety net in production at O(n) per edit. |

Decision: **A**.
