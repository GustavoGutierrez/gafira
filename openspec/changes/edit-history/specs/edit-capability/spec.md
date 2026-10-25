# Edit Capability Specification (delta for C3 `edit-history`)

## Purpose

C3 adds `gafira-edit`, the only crate that holds an `EditToken`, and the `trybuild` compile test for ARQ-30 that C2b deferred (ED3, ED5). This delta replaces the deferral with the real test, and updates V13 now that `crates/gafira-edit/` exists. The other `edit-capability` requirements are unchanged. Open decisions: `design.md` Q13. The requirement name "V13 CI step (D5b)" keeps its C2b delivery tag so that the merge matches it; C3 changes it in its own delivery D5.

## ADDED Requirements

### Requirement: trybuild evidence for ARQ-30 (ARQ-30, ARQ-20) (D5, pending Q13)

`gafira-edit` MUST have a test that runs `trybuild` on fixtures under `crates/gafira-edit/tests/ui/`, each a `.rs` file with a committed `.stderr` for compile-fail cases. The fixtures act as an outside crate that depends on `gafira-doc` and `gafira-edit` (02 ARQ-30 names `gafira-ui`, which does not exist yet). Compile-fail fixtures MUST cover at least:
1. getting `&mut Document` from `History` (no such method, and `document()` is `&Arc<Document>`);
2. assigning to `*history.document()` or through `&Document`;
3. calling `Document::edit` without a token, and on a `&Document`;
4. writing a private `Document` field;
5. getting `&mut Page` or `&mut Node` from `&Document` (`page()`, `Page::node()`).

Each compile-fail fixture MUST fail with exactly the error code recorded in its `.stderr` (`trybuild` compares the full output). A pass fixture MUST show an outside type implementing `Command` and executing through `History::execute`, undo and redo (the legitimate path; ARQ-20). The hidden constructor `__new_for_gafira_edit_only` is callable from any crate, so no fixture MAY claim otherwise; V13 and review keep covering it. Fixtures MUST NOT contain `pub use` or `pub type` naming `EditToken` nor public globs (V13). If the explorer shows that `.stderr` output differs per platform (R3), the test MUST run on Linux only (`#[cfg(target_os = "linux")]`) and the verify report MUST say so.

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

## MODIFIED Requirements

### Requirement: V13 CI step (D5b)

The `policy` job MUST contain a step V13, run under `set -euo pipefail`, that searches the repository with `git grep --untracked -nE 'EditToken|__new_for_gafira_edit_only' -- '*.rs'` and fails when a hit lies outside `^crates/(gafira-doc|gafira-edit|gafira-testkit)/`. The step MUST: capture every `grep` exit code explicitly and treat only exit 1 (no match) as clean, so that any exit above 1 fails the step (`git grep` exits 128 for a broken pattern or no repository; the filter `grep` exits 2 for a broken pattern); run a positive control that fails the step unless the search finds `pub struct EditToken {` in a file under `crates/gafira-doc/src/` (so a wrong working directory, a renamed definition or an empty search cannot pass); print the offending hits; and also fail when any Rust file under `crates/gafira-edit/` (listed with `git ls-files --cached --others --exclude-standard -- crates/gafira-edit | grep '\.rs$' || true`: a plain directory pathspec, so untracked files count and a missing directory prints nothing; `|| true` is required because `grep` exits 1 on no match under `pipefail`) publicly re-exports the token. The re-export check MUST split each file into records at `;` so that a multi-line brace group is one record, and MUST fail on a record that has a `pub use` or `pub type` item naming `EditToken`, and on every public glob re-export (`pub use ...::*`), even if it does not name the token (stricter than needed, accepted). `pub(crate)` re-exports are deliberately allowed. Only `*.rs` files are searched, so `openspec/` and `ci.yml` never match. V13 does NOT prove ARQ-30: the limits are in the table above.

From C3 on, `crates/gafira-edit/` exists. The re-export check MUST scan every Rust file under it, the `trybuild` fixtures under `crates/gafira-edit/tests/ui/` included; fixtures MUST therefore never contain a `pub use` or `pub type` naming the token nor a public glob re-export. The check MUST still pass silently on a tree without the directory (for example a scratch copy used in negative runs): listing files with a plain directory pathspec prints nothing for a missing directory (verified on git 2.34.1). A glob pathspec such as `'crates/gafira-edit/*.rs'` is NOT silent (it prints `warning: could not open directory` to stderr, exit 0), so the step MUST NOT use one. Even if git printed a warning, it MUST NOT break the positive control or change the exit-code mapping.

Known limits of V13 (recorded in the verify report as not met; review covers them, and C3's `trybuild` fixtures cover only the no-token mutation paths, see "trybuild evidence for ARQ-30"): (1) a two-step alias, `use gafira_doc::EditToken as T;` followed by `pub use self::T;`, whose public line does not name the token; (2) a public function, field or wrapper type in `gafira-edit` that hands out `EditToken` or `&EditToken` (an allowed mention, not a re-export); (3) a re-export generated by a macro; (4) a `;` inside a comment or string literal splits a record and can hide a match; (5) type inference in a consumer that never names the token; (6) a re-export of the whole crate or of the module holding the token that never names it (`pub use gafira_doc as d;`, `pub use gafira_doc::edit;`). Matching `EditTokenX` as well is stricter than necessary and accepted. The step is textual; `trybuild` complements it and does not replace it, because the hidden constructor is callable from any crate.

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

## REMOVED Requirements

### Requirement: trybuild is deferred (D5b)

**Reason**: C3 adds `trybuild` as a dev-dependency of `gafira-edit` and the ARQ-30 compile test it deferred (ED3, ED5).

**Migration**: replaced by "trybuild evidence for ARQ-30" in this capability and by "Compile-test dependency (trybuild)" in `workspace-tooling`. The C2b verify report's deferral note stays as history.
