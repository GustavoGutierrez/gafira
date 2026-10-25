# Verify report: edit-history (C3)

## D1: crate skeleton, `msg`, `Command` contract, `Transaction`

Branch `feat/edit-history-1`, base `origin/main` c67d068, range `origin/main..bcee81d`. Verified on 2026-10-24 from separate clones under the session scratchpad, one per commit, each with its own `CARGO_TARGET_DIR`. The main checkout was not touched. CI steps were pulled from that commit's own `ci.yml` (PyYAML) and run under `bash --noprofile --norc -eo pipefail`.

**Delivery verdict: PASS** (0 CRITICAL, 7 WARNING, 4 SUGGESTION). **Change verdict: not final.** D2 to D10 remain, so `next` is `tdd-apply` for D2.

### Gates per commit

| Commit | fmt | clippy `-D warnings --locked` | tests passed (all targets plus doctests) | `cargo deny check` | ARQ-07 | V11 | V5 | V12 | V13 |
|---|---|---|---|---|---|---|---|---|---|
| ab06f6e skeleton + V5 row | 0 | 0 | 313 (0 failed) | ok | 0 | 0 | 0 | 0 | 0 |
| f5785a1 `msg` | 0 | 0 | 314 | ok | 0 | 0 | 0 | 0 | 0 |
| 563ba35 `Command` contract | 0 | 0 | 319 | ok | 0 | 0 | 0 | 0 | 0 |
| d83e0f5 `Transaction` | 0 | 0 | 324 | ok | 0 | 0 | 0 | 0 | 0 |
| bcee81d apply-progress (docs only) | 0 | 0 | 324 | ok | 0 | 0 | 0 | 0 | 0 |

`deny` printed `advisories ok, bans ok, licenses ok, sources ok` on every commit. Doc-test sections ran for `gafira_color`, `gafira_doc` (10), `gafira_edit` (0) and `gafira_geom`. The test totals match `apply-progress.md`. Size: `git diff --shortstat origin/main..HEAD -- crates .github` gives 637 insertions, or 708 with the docs. That is under the ~700-line split limit for code, and ×1.12 over the ~570 forecast, so below the ×1.3 re-cut trigger.

### Negative and command runs (scratch clone at bcee81d, all reverted, `git status` clean afterwards)

| Run | Result |
|---|---|
| Dependency set: `cargo tree -p gafira-edit -e normal,build --target all --prefix none -f '{p}'`, filtered to `gafira-*` | {`gafira-edit`, `gafira-doc`, `gafira-geom`, `gafira-color`} |
| Dev-dependencies are not layer edges: `proptest` in that tree | 0 lines |
| V5 clean with xtrace | rc 0; trace shows `cargo tree -p gafira-edit` |
| E-N1: row cut to `gafira-edit gafira-geom gafira-color` | rc 1, `gafira-edit must not depend on gafira-doc` |
| E-N2: row deleted | rc 1, `gafira-edit has clippy.toml but no V5 row` |
| Forced edge: `gafira-cli` path dep added to `gafira-edit` | rc 1, `gafira-edit must not depend on gafira-cli` |
| Forbidden third-party crate: `serde` added to the pattern | rc 1, `forbidden serde v1.0.229` (first hit is `gafira-geom`) |
| Clock: scratch `std::time::Instant::now()` in `crates/gafira-edit/tests/` | clippy rc 101, `disallowed_methods`, `note: ARQ-02` |
| `HashMap` and `HashSet` in a `gafira-edit` test | clippy rc 101, `disallowed_types`, both ARQ-10 notes, no "does not refer to an existing" warning |
| `std::thread::spawn` in a `gafira-edit` test | clippy rc 101, `disallowed_methods`, ARQ-02 |
| Copies identical: `diff -q` of the geom copy against color, doc and edit | all identical; each of the 4 copies has both `Hash*` entries |
| V11: forbid removed from `crates/gafira-edit/src/lib.rs` | rc 1, `missing forbid(unsafe_code): crates/gafira-edit/src/lib.rs` |
| Workspace membership | `gafira-edit 2024 1.93 []`; root `Cargo.toml:16` declares it by path |
| No serde on `MessageKey` | `grep -c 'Serialize\|Deserialize' crates/gafira-doc/src/msg.rs` = 0 |

### Requirement and scenario coverage (D1)

| Requirement / scenario | Status | Evidence |
|---|---|---|
| Crate purity: forbid, canonical `clippy.toml`, dependencies, no clock, threads, env, `Hash*` or mutable statics | covered | `crates/gafira-edit/src/lib.rs:7`; runs above; grep finds no `static`, `#[allow]`, `thread::`, `Instant` or `Hash*` in the crate sources |
| Scenario: Dependency set (command) | covered | run above, V5 rc 0 |
| Scenario: Clock call rejected (integration) | covered | run above |
| Command contract: trait shape and defaults | covered | `crates/gafira-edit/src/command.rs:46-72`, matches 02 §4.3 and Q1 |
| `Applied` has exactly `Changed` and `Unchanged` | covered | `command.rs:15-20` |
| `DirtySet`: two `BTreeSet`s plus union | covered | `crates/gafira-edit/src/dirty.rs:12-26`. The page/node semantics belong to concrete commands and law 4 (D2+) |
| `MergeKey` is `Clone + Eq + Debug` with kind, targets and optional `GestureId` | covered | `command.rs:30-40` (see item W2) |
| `CommandError`: `non_exhaustive`, Clone, PartialEq, Display, Error, all 9 cases | covered | `crates/gafira-edit/src/error.rs:10-31, 33-53`; bound check `error.rs:62` |
| Scenario: Object safety and bounds | covered | `command.rs:132` (no thread) |
| Scenario: Default merge returns next unchanged | covered | `command.rs:140` (pointer-equal via `ptr::addr_eq`) |
| Scenario: Merge can recover the concrete type (probe first) | covered | `command.rs:115-126` (test `Sum`), test `command.rs:149` |
| Scenario: DirtySet union is ordered and deduplicated | covered | `dirty.rs:40` (shares page 3 and node 5) |
| Failed commands change nothing (D1 part, through `Transaction`) | covered for atomicity | `transaction.rs:166`; reusability after failure not tested (W4) |
| Transaction: ordered children, label set at construction, rollback i−1..0, error of i | covered | `transaction.rs:21-49`; test `transaction.rs:166` (doc equals before, log `revert b, revert a`) |
| Scenario: All or nothing | covered | `transaction.rs:166` |
| Scenario: Reverse-order revert | covered | `transaction.rs:189` |
| Scenario: Touches is the union | covered | `transaction.rs:209` |
| Changed iff a child changed; empty is Unchanged | covered | `transaction.rs:225` (5-row table, row 1 empty) |
| `merge_key` is None | covered | `transaction.rs:247` |
| Message key for command labels (`msg`) | covered | `crates/gafira-doc/src/msg.rs:7-11`, identical to 02 §14.2 lines 1585-1588; no text, no `Locale` |
| Scenario: Label is a key | covered | `msg.rs:18` |
| Scenario: No serde on MessageKey | covered | grep above |
| Workspace tooling: `clippy.toml` copy plus V5 row in the skeleton commit | covered | `git show --stat ab06f6e` has `clippy.toml`, the `ci.yml` row and the manifest together |
| V5 table: `gafira-edit` row; E-N1, E-N2; dev-dependencies outside | covered | runs above; `ci.yml:83` |
| V11/V12 coverage of `gafira-edit`; copies identical; HashMap/HashSet scenario for `gafira-edit` | covered | runs above (the `gafira-testkit` parts are D2) |
| V13 (D1 part): stays green with `crates/gafira-edit/` present | covered | V13 rc 0 on every commit; the D5 parts are out of scope |
| Each commit stays green | covered | table above |

### Mutation

`mutants` report (`scratchpad/mutants-C3-D1/report.md`): 8 mutants, 4 caught, 1 missed, 0 timeout, 3 unviable.
- Missed: `error.rs:35`, `Display for CommandError::fmt -> Ok(Default::default())`. Non-critical: the spec does not fix the message text. Documented as W1.
- Unviable: `merge_key -> Some(Default::default())`, `Transaction::label -> Default::default()` and `Transaction::apply -> Ok(Default::default())` (no `Default` impl).

Hand mutations, for rules that cargo-mutants cannot express (5, all KILLED):

| # | Mutation | Killed by |
|---|---|---|
| H1 | Rollback in forward order (`.rev()` dropped, `transaction.rs:38`) | `a_failing_child_rolls_back_the_earlier_ones` |
| H2 | Rollback also reverts the failing child (`[..=i]`) | same |
| H3 | `revert` in forward order (`transaction.rs:52`) | `revert_undoes_the_children_in_reverse_order` |
| H4 | `touches` keeps only the last child (`transaction.rs:61`) | `touches_is_the_union_of_the_children` |
| H5 | `DirtySet::union` drops the other set's nodes (`dirty.rs:23`) | `union_is_ordered_and_deduplicated`, `touches_is_the_union_of_the_children` |

The `Changed` arm of `Transaction::apply` and the probe-first merge are covered by the non-vacuity runs in `apply-progress.md`, so they were not repeated.

### CRITICAL

None.

### WARNING

1. **Missed mutant, Display text** (`crates/gafira-edit/src/error.rs:33-53`). This is non-critical by policy because the spec fixes no text. No test reads the rendered message. Accept as is. Do not add a text-pinning test unless a spec fixes the wording.
2. **Open shapes in the spec** (`command.rs:24`, `command.rs:30-40`, `error.rs:24`, `error.rs:20`). The spec fixes no field layout, so `MergeKey { kind: &'static str, page, nodes, gesture }`, `GestureId(pub u64)`, `InvalidValue(&'static str)` and `IndexOutOfRange { index, len }` all meet the D1 text. This is acceptable for D1 and is not a spec defect. Later deliveries must still honour the following:
   - D3: `SetProperty` must fold the property into `kind` (for example `"set_property.placement"`). Otherwise two properties on the same nodes would merge.
   - D6: `GestureId` has a public field, so any caller can forge an id equal to the open gesture's. `begin_gesture` returning a "fresh" id stays true for `History`'s counter, but nothing stops forgery. D6 should decide whether the field stays public.
   - D10: the document-DPI target can use `page: None, nodes: []`. This needs no change now.
3. **`Transaction::apply` discards rollback errors** (`transaction.rs:38-43`). The spec requires returning child *i*'s error and the prior state. If a child's revert fails, the prior state is not restored, and the caller only sees *i*'s error. That breaks a child contract (a cleanly applied command must revert). Silently swallowing it is defensible against the spec text. `Transaction::revert` (`transaction.rs:52-54`) stops at the first error and leaves a partially reverted document. History (D4) keeps snapshots, which limits the damage. The rollback should be considered for D4, which could surface the first revert error or record it.
4. **Reusability after a failed apply is not tested for `Transaction`** ("Failed commands change nothing": "the command MUST stay reusable"). `a_failing_child_rolls_back_the_earlier_ones` (`transaction.rs:166`) checks atomicity only. Re-applying works if the children are reusable, but no test shows it (for example, a child that fails once and then succeeds). The D2 harness (law 2 / law 5) will cover it generically. Note it there.
5. **Low-value tests (test-scope rule)**:
   - `it_has_its_own_label_and_never_merges` (`transaction.rs:247`) is half an accessor test (`label()` returns what was passed). The `merge_key() == None` half is spec-required.
   - In `an_error_is_cloneable_comparable_and_an_error` (`error.rs:62`), the runtime asserts `clone == self` and `!= IdSpaceExhausted` re-test derived impls. The compile-time `assert_contract` bound is the spec evidence.
   - `a_boxed_command_is_send_and_sync` (`command.rs:132`) also asserts the default `merge_key()`, which is unrelated to its scenario.
   None of these is harmful.
6. **V12 count left at `>= 3`** (`ci.yml:119`). The binding spec ("Policy checks cover every domain crate", item 3) still says "fewer than three". No requirement or scenario in this change raises it, so leaving the body unchanged is correct. However, `proposal.md:82` lists "V12 count" as a D1 item, which is misleading. With four copies, deleting one copy (for example `crates/gafira-edit/clippy.toml`) passes every check. V12 still counts 3, and V5 still runs the `gafira-edit` row, because its coverage loop iterates the copies, not the rows. The crate would then lose its clippy bans (ARQ-02, ARQ-10) with CI green. This was measured on a scratch clone: with the copy removed, V5 rc 0, V12 rc 0, and clippy passes on an `Instant::now()` test. Spec gap, not a D1 defect. See S1.
7. **`proptest` dev-dependency unused until D3** (`crates/gafira-edit/Cargo.toml:15`). Neither V5 (dev-dependencies are outside `-e normal,build`, as confirmed by the run above) nor `cargo deny` (bans and licenses ok) objects. `Cargo.lock` gained only the `gafira-edit` package. Acceptable, but it is a declared-but-unused dependency for 1 or 2 deliveries.

### SUGGESTION

1. Make the V12 count exact or derive it from the V5 table, through a spec edit (`workspace-tooling`, V12 item 3) before or with D2. Also fix the "V12 count" wording in `proposal.md:82`. One option is to make V5 also fail when a row's crate has no `clippy.toml`, except for the `gafira-testkit` exemption.
2. `msg` is not re-exported at the crate root (`crates/gafira-doc/src/lib.rs:17`). That matches the spec, which names the path `gafira_doc::msg::CommandLabel`, and 02 §4.3. No change needed.
3. In `crates/gafira-edit/src/lib.rs:16`, `mod transaction;` comes after the `pub use` block. Group the `mod` declarations together (cosmetic).
4. The test-only `Step` command (`transaction.rs:86-155`) is a good candidate for `gafira-testkit` in D2, as `apply-progress.md` already notes.

### Other checks

- Code rules: no `#[allow]`, no threads, no clock and no `unsafe` in the diff. Comments, identifiers and errors are in English. The tests use `Arc<Mutex<_>>` only to satisfy the `Send + Sync` supertraits, and spawn no threads.
- Commits: conventional subjects, author Gustavo Gutierrez, no `Co-Authored-By` or AI attribution lines.
- No `proptest-regressions` files are tracked or present.
- The `mutants` report was read. Its numbers match its `mutants.out` lists.
