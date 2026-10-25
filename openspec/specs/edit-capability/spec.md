# Edit Capability Specification

## Purpose

The compile-visible edit capability of `gafira-doc`: only `gafira-edit` (C3) mutates a `Document` in place, as the interim enforcement of ARQ-30 until `trybuild` arrives in C3. This change defines `EditToken`, `Document::edit`, `DocMut` and id allocation; it MUST NOT define `Command`, `History` or `EditService` (C3). New capability; no existing spec.

Scenario type tags: (unit), (integration), (command), (manual). All requirements here belong to D5b of the design, except where noted.

## Decisions followed from the design (still open for the holder)

| # | Decision | Design ref |
|---|---|---|
| ED1 | The root type is `Document`; its fields are private. The value types below it (`Page`, `Node`, ...) have public fields, which is safe because no `&mut` to them is reachable from `&Document` or `Arc<Document>`. | A7, A11 |
| ED2 | The constructor is `#[doc(hidden)] pub const fn __new_for_gafira_edit_only() -> EditToken`; `EditToken` is `pub struct EditToken { _private: () }` with no `Clone`, `Copy`, `Default` or `Debug`. | A11 |
| ED3 | `trybuild` is not part of this change (C3). The `compile_fail` doctests below are regression aids only: stable rustdoc does NOT verify the `EXXXX` code in `compile_fail,EXXXX` (a doctest failing for another reason still passed in a trial), so the codes are comments and the doctests are NOT evidence of ARQ-30. The evidence is the V13 step, the guarantees table and review; `trybuild` arrives in C3. | Proposal amendment 3 |
| ED5 | C3 note: `trybuild` fixtures that name `EditToken` will trip V13 unless they live under an allowed path (`crates/gafira-edit/`, `crates/gafira-testkit/`) or V13's allowlist is extended in the same change. | Corrected design |
| ED4 | CI step name V13; allowed crates `gafira-doc`, `gafira-edit`, `gafira-testkit`. | A11 |

## What Rust guarantees and what it does not

| Guaranteed | NOT guaranteed (and how it is covered) |
|---|---|
| Calling `Document::edit` requires a value of `EditToken`. | The `#[doc(hidden)]` constructor is callable by any crate. V13 rejects the name outside the allowed crates. |
| No crate outside `gafira-doc` can build a token with a struct literal, `Default`, `Clone` or `Copy`. | A holder can lend `&EditToken` anywhere, and inference lets a callee use it without naming the type. V13 also rejects a `pub use ... EditToken` re-export in `gafira-edit`; a public `gafira-edit` function returning `&EditToken` is caught only by review until `trybuild` in C3. |
| With `&Document` or `Arc<Document>` there is no field-level `&mut`. | Whole-value replacement: whoever owns a `Document`, or reaches one with `Arc::make_mut`, can write `*doc = other` (an `other` built by deserialization or `from_parts`). This does not touch the session document, which C3's `EditService` owns. It is documented, not prevented. |
| (nothing) | V13 is textual. It catches public `pub use` and `pub type` items naming the token, multi-line brace groups and public globs inside `gafira-edit`, but NOT a two-step alias (`use ... EditToken as T;` then `pub use self::T;`), a public function or wrapper handing out the token, a macro-generated re-export, a re-export of the whole crate or of the `edit` module (`pub use gafira_doc as d;`), or a `;` in a comment or string that splits a record; these are known limits covered only by review until `trybuild` in C3. |
| `unsafe` cannot bypass this in any workspace crate (`forbid`, V11). | Building a new valid document from JSON is allowed by design: it is construction, not mutation. |

## Requirements

### Requirement: EditToken is a capability (D5b)

`EditToken` MUST NOT implement `Clone`, `Copy`, `Default` or `Debug`, MUST have a private field, and MUST have no constructor other than the `#[doc(hidden)]` one named in ED2. No other public function MAY return an `EditToken` by value. Only `gafira-doc` (its own tests included), `gafira-edit` and `gafira-testkit` MAY name the token or the constructor.

#### Scenario: Not Clone (integration, compile-fail doctest)
- GIVEN a doctest calling `.clone()` on an `EditToken`
- WHEN `cargo test --doc -p gafira-doc` runs
- THEN the doctest annotated `compile_fail` passes (a regression aid; the code `E0599` is expected but not verified)

#### Scenario: Not Copy (integration, compile-fail doctest)
- GIVEN a doctest that uses a token after moving it into a function
- WHEN the doctests run
- THEN the doctest annotated `compile_fail` passes (expected `E0382`, not verified)

#### Scenario: No struct literal (integration, compile-fail doctest)
- GIVEN a doctest that writes `EditToken { _private: () }` from outside the crate
- WHEN the doctests run
- THEN the doctest annotated `compile_fail` passes (expected `E0451`, not verified). To confirm that each doctest fails for the intended reason, the verify step MUST also compile each snippet once by hand (scratch crate) and record the actual error code

#### Scenario: Hidden from the docs (command)
- GIVEN `cargo doc -p gafira-doc --no-deps`, and doctests that need a token with the line calling the constructor hidden by the `# ` prefix (a doctest renders its visible lines, so an unhidden line would put the name in `struct.EditToken.html`; rustdoc also copies the whole source into `src/<crate>/lib.rs.html`, which is why that tree is excluded)
- WHEN every file under `target/doc` outside the `src/` directories (the HTML pages and the search index) is searched for `__new_for_gafira_edit_only`
- THEN it is absent

### Requirement: Document::edit and DocMut (D5b)

`Document::edit(&mut self, token: &EditToken) -> DocMut<'_>` MUST be the only way to obtain `&mut` access to anything inside a `Document`. `DocMut` MUST borrow the document exclusively for its lifetime. `Document` fields MUST be private, and no public API reachable from `&Document` or `Arc<Document>` MAY return `&mut` to its contents. One token MUST serve any number of calls. `DocMut` MUST offer at least: `alloc_node_id`, `alloc_page_id`, `set_dpi(Dpi)`, `pages_mut() -> &mut Vec<Arc<Page>>`, `page_mut(PageId) -> Option<&mut Page>`, `node_mut(PageId, NodeId) -> Option<&mut Node>` and `as_doc() -> &Document`. `page_mut` and `node_mut` MUST use copy on write (`Arc::make_mut`) along the edited path only. `DocMut` MUST NOT validate implicitly: after any sequence of operations the document MAY be invalid and `validate()` is the check.

#### Scenario: Edit through the capability (unit)
- GIVEN a test-only token and `Document::default()`
- WHEN `edit` allocates a page id and a node id, pushes a page, and inserts a group as a root through `page_mut`
- THEN the document shows the page, the node and an advanced `next_id`, and `validate()` passes

#### Scenario: One token, many edits (unit)
- GIVEN one token
- WHEN `edit` is called three times in sequence
- THEN all three compile and apply

#### Scenario: No write without a token (integration, compile-fail doctest)
- GIVEN doctests that assign `doc.dpi = ...`, call `doc.pages_mut()` on a `Document`, and call `doc.edit()` without an argument
- WHEN the doctests run
- THEN each, annotated `compile_fail`, passes (expected privacy `E0616`/`E0599` or arity `E0061`, not verified by rustdoc; recorded by hand as above)

#### Scenario: Shared Arc cannot be mutated (integration, compile-fail doctest)
- GIVEN an `Arc<Document>` as the UI holds it
- WHEN a doctest pushes to `pages` through `Arc::get_mut`, and another gets a `&mut Page` from `&Document`
- THEN neither compiles

#### Scenario: Invalid state is allowed until validated (unit)
- GIVEN an edit that inserts a node into a page's arena without listing it as a root or child
- WHEN `edit` returns
- THEN it did not fail and `validate()` reports `Orphan`

#### Scenario: Copy on write keeps sharing (unit)
- GIVEN a document, its clone, and one node changed with `node_mut`
- WHEN compared with `Arc::ptr_eq`
- THEN the edited node and its page differ, and every untouched node and page is still pointer-equal

#### Scenario: Whole-value replacement is documented, not prevented (manual)
- GIVEN the rustdoc of `Document` and `Document::edit`
- WHEN reviewed
- THEN it states that `*doc = other` is possible and is owned by `EditService` in C3

### Requirement: Id allocation (D5b)

C3 delivery: D8, D10. Scenarios without a delivery tag below belong to C2b D5b.

`DocMut::alloc_node_id` and `alloc_page_id` MUST return the current `next_id` as a `NodeId` or `PageId` and then increment it; ids MUST never be reused within a document. When `next_id == u64::MAX` the call MUST return `Err(IdSpaceExhausted)` and leave `next_id` unchanged (no wrap). `IdSpaceExhausted` MUST be `Copy`, `Eq`, `Display` and `Error`. Allocation outside `DocMut` MUST NOT be public. The counter is shared by nodes and pages.

"Never reused" applies to allocation: inside every valid document state ids are unique and `next_id` lies above every id (`validate()`), so allocation never returns an id that a node or page of that state holds. It does not forbid returning to an earlier state. From C3 on, `DocMut` MUST also offer `restore_next_id(value: u64)`, which lowers `next_id` to `value` and MUST ignore a `value` at or above `next_id`, so it never raises the counter. A revert that gives back the ids its apply allocated, like a snapshot undo, returns the counter to a value it held before, so a redo after a revert, or a new command after an undo, MAY be issued the same ids again (`edit-history` "Ids after undo", Q6). A caller MUST NOT lower the counter under an id that a later allocation handed out; the `gafira-edit` insert commands check this before they restore it (`edit-commands` "Restoring the id counter").

#### Scenario: Shared counter (unit)
- GIVEN `next_id = 5`
- WHEN a page id, a node id and a node id are allocated
- THEN the values are 5, 6, 7 and `next_id()` is 8

#### Scenario: Last issuable id (unit)
- GIVEN `next_id = u64::MAX - 1`
- WHEN an id is allocated
- THEN it returns `u64::MAX - 1` and `next_id` becomes `u64::MAX`

#### Scenario: Exhaustion (unit)
- GIVEN `next_id = u64::MAX`
- WHEN an id is allocated
- THEN it returns `Err(IdSpaceExhausted)` and `next_id` is still `u64::MAX`

#### Scenario: Restore lowers, never raises (D8, unit)
- GIVEN `next_id = 1` and two allocated ids
- WHEN `restore_next_id(1)` runs, then `restore_next_id` with a value above `next_id`
- THEN `next_id` is 1 after the first call, the next allocation returns 1 again, and the second call changes nothing

### Requirement: Readers hold Arc<Document> (D5b)

Consumers outside `gafira-edit` MUST read a document through `&Document` or `Arc<Document>` and the accessors of `document-model`. Cloning an `Arc<Node>` or `Arc<Page>` out of a document MUST NOT make the document mutable.

#### Scenario: Copy on write handle (unit)
- GIVEN a node `Arc` cloned out of a document and changed with `Arc::make_mut`
- WHEN the document is read again
- THEN its node is unchanged and the two `Arc`s are no longer pointer-equal

### Requirement: V13 CI step (D5b)

C3 delivery: D1, D5. Scenarios without a delivery tag below belong to it.

The `policy` job MUST contain a step V13, run under `set -euo pipefail`, that searches the repository with `git grep --untracked -nE 'EditToken|__new_for_gafira_edit_only' -- '*.rs'` and fails when a hit lies outside `^crates/(gafira-doc|gafira-edit|gafira-testkit)/`. The step MUST: capture every `grep` exit code explicitly and treat only exit 1 (no match) as clean, so that any exit above 1 fails the step (`git grep` exits 128 for a broken pattern or no repository; the filter `grep` exits 2 for a broken pattern); run a positive control that fails the step unless the search finds `pub struct EditToken {` in a file under `crates/gafira-doc/src/` (so a wrong working directory, a renamed definition or an empty search cannot pass); print the offending hits; and also fail when any Rust file under `crates/gafira-edit/` (listed with `git ls-files --cached --others --exclude-standard -- crates/gafira-edit | grep '\.rs$' || true`: a plain directory pathspec, so untracked files count and a missing directory prints nothing; `|| true` is required because `grep` exits 1 on no match under `pipefail`) publicly re-exports the token. The re-export check MUST split each file into records at `;` so that a multi-line brace group is one record, and MUST fail on a record that has a `pub use` or `pub type` item naming `EditToken`, and on every public glob re-export (`pub use ...::*`), even if it does not name the token (stricter than needed, accepted). `pub(crate)` re-exports are deliberately allowed. Only `*.rs` files are searched, so `openspec/` and `ci.yml` never match. V13 does NOT prove ARQ-30: the limits are in the table above.

From C3 on, `crates/gafira-edit/` exists. The re-export check MUST scan every Rust file under it, the `trybuild` fixtures under `crates/gafira-edit/tests/ui/` included; fixtures MUST therefore never contain a `pub use` or `pub type` naming the token nor a public glob re-export. The check MUST still pass silently on a tree without the directory (for example a scratch copy used in negative runs): listing files with a plain directory pathspec prints nothing for a missing directory (verified on git 2.34.1). A glob pathspec such as `'crates/gafira-edit/*.rs'` is NOT silent (it prints `warning: could not open directory` to stderr, exit 0), so the step MUST NOT use one. Even if git printed a warning, it MUST NOT break the positive control or change the exit-code mapping.

Known limits of V13 (recorded in the verify report as not met; review covers them, and C3's `trybuild` fixtures cover only the no-token mutation paths, see "trybuild evidence for ARQ-30"): (1) a two-step alias, `use gafira_doc::EditToken as T;` followed by `pub use self::T;`, whose public line does not name the token; (2) a public function, field or wrapper type in `gafira-edit` that hands out `EditToken` or `&EditToken` (an allowed mention, not a re-export); (3) a re-export generated by a macro; (4) a `;` inside a comment or string literal splits a record and can hide a match, and the reverse is a false positive: a fixture comment such as `// pub use x::*;` trips the re-export check (spike R4); (5) type inference in a consumer that never names the token; (6) a re-export of the whole crate or of the module holding the token that never names it (`pub use gafira_doc as d;`, `pub use gafira_doc::edit;`). Matching `EditTokenX` as well is stricter than necessary and accepted. The step is textual; `trybuild` complements it and does not replace it, because the hidden constructor is callable from any crate.

#### Scenario: Clean tree passes (E0, command)
- GIVEN the final tree
- WHEN V13 runs from a scratch copy under `bash --noprofile --norc -eo pipefail`
- THEN it exits 0 and the positive control found the definition

#### Scenario: Forced use outside fails (E1, manual negative test)
- GIVEN `// EditToken` temporarily added to `apps/gafira-cli/src/main.rs`
- WHEN V13 runs
- THEN it exits 1 naming that file; with no step it would pass (the RED); reverted afterwards

#### Scenario: Constructor name fails (E2, manual negative test)
- GIVEN `__new_for_gafira_edit_only` in a test of `crates/gafira-color`
- WHEN V13 runs
- THEN it exits 1

#### Scenario: Positive control (E3, manual negative test)
- GIVEN the definition renamed to `pub struct EditTok {`
- WHEN V13 runs
- THEN it exits 1 with the positive-control message

#### Scenario: grep errors are not clean (E4, manual negative test)
- GIVEN the search pattern broken (`'EditToken('`), and separately the `allowed` pattern broken (`'^crates/(gafira-doc'`)
- WHEN V13 runs
- THEN it exits 1 with "git grep failed" and "filter grep failed" respectively

#### Scenario: Wrong place (E5, manual negative test)
- GIVEN V13 run from `crates/`, and from a copy that is not a git repository
- WHEN it runs
- THEN it exits 1 in both cases (positive control; git status 128)

#### Scenario: Re-export in gafira-edit (E6, manual negative test)
- GIVEN an untracked `crates/gafira-edit/src/lib.rs` containing `pub use gafira_doc::EditToken as Cap;`
- WHEN V13 runs
- THEN it exits 1 with the re-export message

#### Scenario: Missing gafira-edit directory (E10, command)
- GIVEN a scratch copy of the final tree with `crates/gafira-edit/` removed
- WHEN V13 runs with standard error captured
- THEN it exits 0, the positive control found the definition, and the re-export check printed nothing (a git warning, if any, does not change the result)

#### Scenario: Public re-export forms are caught (E7 multi-line group, E8 `pub type`, E9 glob; the first form is the one-line group, manual negative test)
- GIVEN an untracked `crates/gafira-edit/src/lib.rs` containing, in turn: `pub use gafira_doc::{EditToken, other};` on one line; a multi-line `pub use gafira_doc::{` newline `EditToken,` newline `};`; `pub type Cap = gafira_doc::EditToken;`; and `pub use gafira_doc::*;`
- WHEN V13 runs on each
- THEN it exits 1 on each with the re-export message

#### Scenario: pub(crate) re-export is allowed (E12, manual negative test)
- GIVEN an untracked `crates/gafira-edit/src/lib.rs` containing `pub(crate) use gafira_doc::EditToken;`
- WHEN V13 runs
- THEN it exits 0

#### Scenario: Known limits are not detected (E11 alias, E13 lend, E15 crate or module re-export; known limitation, manual)
- GIVEN an untracked `crates/gafira-edit/src/lib.rs` with `use gafira_doc::EditToken as T;` followed by `pub use self::T;` (E11); separately `pub fn lend(t: &gafira_doc::EditToken) -> &gafira_doc::EditToken { t }` (E13); and separately `pub use gafira_doc as d;` and `pub use gafira_doc::edit;` (E15)
- WHEN V13 runs on each
- THEN it exits 0 on each (limits 1, 2 and 6 above, recorded in the verify report as not met and left to review)

#### Scenario: Allowed crates pass (E14, manual negative test)
- GIVEN `EditToken` mentioned in `crates/gafira-doc/tests/edit.rs`, and in a scratch `crates/gafira-testkit/src/lib.rs` and `crates/gafira-edit/src/lib.rs` without re-export
- WHEN V13 runs
- THEN it exits 0

#### Scenario: trybuild fixtures pass V13 (E16, command)
- GIVEN the final tree with `crates/gafira-edit/src/` and the fixtures under `crates/gafira-edit/tests/ui/`
- WHEN V13 runs
- THEN it exits 0, the positive control found the definition, and the re-export check listed the fixture files

#### Scenario: A fixture that re-exports the token fails (E17, manual negative test)
- GIVEN an untracked `crates/gafira-edit/tests/ui/scratch.rs` containing `pub use gafira_doc::EditToken;`
- WHEN V13 runs
- THEN it exits 1 with the re-export message; removed afterwards

### Requirement: trybuild evidence for ARQ-30 (ARQ-30, ARQ-20) (D5, Q13)

`gafira-edit` MUST have a test that runs `trybuild` on fixtures under `crates/gafira-edit/tests/ui/`, each a `.rs` file with a committed `.stderr` for compile-fail cases. The fixtures act as an outside crate that depends on `gafira-doc` and `gafira-edit` (02 ARQ-30 names `gafira-ui`, which does not exist yet). Compile-fail fixtures MUST cover at least:
1. getting `&mut Document` from `History` (no such method, and `document()` is `&Arc<Document>`);
2. assigning to `*history.document()` or through `&Document`;
3. calling `Document::edit` without a token, and on a `&Document`;
4. writing a private `Document` field;
5. getting `&mut Page` or `&mut Node` from `&Document` (`page()`, `Page::node()`).

Each compile-fail fixture MUST fail with exactly the error code recorded in its `.stderr` (`trybuild` compares the full output). A pass fixture MUST show an outside type implementing `Command` and executing through `History::execute`, undo and redo (the legitimate path; ARQ-20). The hidden constructor `__new_for_gafira_edit_only` is callable from any crate, so no fixture MAY claim otherwise; V13 and review keep covering it. Fixtures MUST NOT contain `pub use` or `pub type` naming `EditToken` nor public globs (V13). Identical `.stderr` output on Windows, Linux and macOS is UNVERIFIED locally (only Linux stable was checked; a CI throwaway run in D5 is needed, spike R3). A `.stderr` that quotes another crate's source line goes stale when that line changes. If the D5 run shows that `.stderr` output differs per platform (R3), the test MUST run on Linux only (`#[cfg(target_os = "linux")]`) and the verify report MUST say so.

#### Scenario: No mutable access from outside (D5, integration)
- GIVEN the compile-fail fixtures 1 to 5
- WHEN `cargo test -p gafira-edit --test ui --locked` runs
- THEN every fixture fails to compile and its output matches the committed `.stderr` (expected codes recorded in each file, for example `E0599`, `E0594`, `E0061`, `E0596`, `E0616`)

#### Scenario: Outside command takes the legitimate path (D5, integration)
- GIVEN the pass fixture with an outside `Command` implementation
- WHEN the `trybuild` test runs
- THEN it compiles and its `main` asserts the change, the undo and the redo

#### Scenario: A wrong error code fails the test (D5, manual negative test)
- GIVEN one fixture's `.stderr` edited to another error code
- WHEN the `trybuild` test runs
- THEN it fails showing the mismatch (unlike `compile_fail` doctests, ED3); reverted afterwards

#### Scenario: Deny gate still passes (D5, command)
- GIVEN the lock after adding `trybuild`
- WHEN `cargo deny check licenses bans sources` runs
- THEN it exits 0 with no change to `deny.toml`'s allowlist (or with a written, per-crate exception, see `workspace-tooling`)
