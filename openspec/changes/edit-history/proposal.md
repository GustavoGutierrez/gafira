# Proposal: Edit History (C3, commands and undo)

## Intent

`gafira-doc` (C2b) gives a validated document and one way to mutate it (`EditToken` -> `DocMut`), but nothing that turns an edit into an undoable step. This change adds `gafira-edit` (layer 1 domain, 02 §2.1): the `Command` trait (02 §4.3), the core commands that today's model can support, `Transaction` (Composite) and `History` (Command + Memento, ADR-03) with a memory budget and gesture merging. It also turns ARQ-30 from a textual CI check into a compile test (`trybuild`), and runs `debug_assert!(doc.validate().is_ok())` after every command (ARQ-13).

Product IDs: ARQ-13 (after each command), ARQ-30 (compile test part), ARQ-32 (structural part), ARQ-33, ARQ-81, ADR-03, and ARQ-01/ARQ-02 for the new crate (V5, `clippy.toml`). Open decisions are in `design.md` (Q1 to Q15); the specs follow its recommendations and name the question where one applies.

## Change Chain

| Order | Change | Content |
|---|---|---|
| C1 | `workspace-scaffold` (archived) | Workspace, CI, `gafira-geom`, `gafira-cli --version` |
| C2a | `domain-color` (archived) | `gafira-color`, table-driven V5 |
| C2b | `domain-doc` (archived) | `gafira-doc` model, serde, `validate()`, `EditToken`, V13 |
| C3 | `edit-history` (this) | `gafira-edit`: `Command`, core commands, `Transaction`, `History` (snapshots, budget, gesture merge), `debug_assert` after each command, `trybuild` for ARQ-30, `gafira-testkit` |
| C4 | `gafira-format` change (name to be chosen) | `.gafira` reader and writer, `gafira-cli validate` (deferred from C2b, A15) |
| C5+ | render, engine (`EditService`, `SelectionService`), UI | Per 02 §11.2 |

## Scope

### In Scope

- New crate `crates/gafira-edit` (layer 1): `#![forbid(unsafe_code)]`, the canonical domain `clippy.toml` copy (V12), a V5 row `gafira-edit gafira-geom gafira-color gafira-doc`.
- Minimal `gafira-doc::msg`: `MessageKey(pub &'static str)` and `type CommandLabel = MessageKey`, exactly as 02 §14.2 (Q11).
- `Command: Any + Send + Sync + Debug` with `label`, `apply`, `revert`, `merge_key`, `merge`, `touches` (02 §4.3). `apply` and `revert` receive `&mut DocMut<'_>` (Q1, amendment 1). Supporting types: `Applied { Changed, Unchanged }` (Q14), `CommandError`, `DirtySet` (`BTreeSet`s of pages and nodes), `MergeKey`, `GestureId`.
- Core commands the current model supports (only `Group`, `Shape`, `Extension` exist):
  - `SetProperty`: node fields (`name`, `placement`, `opacity`, `blend`, `visible`, `locked`), shape `fill` and `geometry`, and (Q4) page fields (`name`, `size`, `locked`, `hidden`) and the document DPI.
  - `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent` (inside one page).
  - (Q4) `InsertPage`, `RemovePage`.
- `Transaction` (Composite): many commands, one history entry, all or nothing.
- `History`: owns the session `Arc<Document>` (Q2); `execute`, `undo`, `redo`, `preview`; snapshot per entry (Memento); redo cleared by a new change; gesture merge with explicit gestures and a 500 ms window on caller-supplied time (Q3); memory budget, default 512 MiB (Q7), with compaction to commands only and `revert` for compacted entries.
- `debug_assert!(doc.validate().is_ok())` after every applied command and every undo or redo (ARQ-13); commands check their own inputs in all builds (Q15).
- `gafira-testkit` (support crate, dev-dependency only): the document generator moved from `crates/gafira-doc/tests/common` with `git mv`, and the `prop_command` harness with the 02 §10 laws (Q10).
- A property test for every core command (ARQ-81) and CI step V14 that checks it (Q9).
- `trybuild` (dev-dependency of `gafira-edit`) with compile-fail and pass fixtures under `crates/gafira-edit/tests/ui/` (an allowed V13 path, ED5) for ARQ-30 (Q13).
- Tooling deltas: V5 rows for `gafira-edit` and `gafira-testkit`, the fourth `clippy.toml` copy, V13 updated for an existing `gafira-edit`, the `trybuild` dependency gate, V14.

### Out of Scope

- `EditService`, `Session`, `SelectionService`, the event bus and `DocumentChanged` (application layer, `gafira-engine`).
- `Selection` and `HitTester` (Q5; `HitTester` also needs `NodeVisitor`, not built yet).
- Commands for kinds or data that do not exist yet: `SetFrameContent`, `PaintMask`, `ApplyTemplate`, `ResizeDesign`, `SplitClip`, `TrimClip`, `MoveClip`, `SetKeyframe`, `SetSwatch`, `RestoreVersion`.
- Moving nodes between pages; lock enforcement in commands (Q12).
- Tools (`Tool`, `ToolOutcome`), UI, persistence of the history (02 §7.1: the history is never saved).
- The timed ARQ-32 benchmark (deferred to the ARQ-40 benchmark change, Q8); `xtask` (V14 stands in for ARQ-81 until then).
- `Message`, `MsgArg`, `ToMessage`, `msg!` (i18n change).

## Capabilities

### New Capabilities
- `edit-commands`: the `Command` contract, its supporting types, atomicity, `Transaction`, the core commands and their property laws (ARQ-81).
- `edit-history`: `History`, snapshots, undo and redo, preview, gesture merge (ARQ-33), budget and compaction (ADR-03), the `debug_assert` (ARQ-13), O(1) undo (ARQ-32, structural).

### Modified Capabilities
- `document-model`: the minimal `msg` module (ADDED).
- `edit-capability`: `trybuild` evidence for ARQ-30 (ADDED), "trybuild is deferred" (REMOVED), V13 with an existing `gafira-edit` (MODIFIED).
- `workspace-tooling`: V5 table, `clippy.toml` copies, V11/V12 coverage and the V13 cross-reference (MODIFIED); `trybuild` gate and V14 (ADDED).

## Approach

Strict TDD, one reviewable work unit per commit, every commit passes `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` and the policy steps.

- **New third-party dependency**: `trybuild` (dev-only, `gafira-edit`). Its tree must pass `deny.toml` unchanged (R1, R2). `proptest` is already in the workspace.
- **Token**: `gafira-edit` keeps one private `static TOKEN: EditToken` and never exposes it (V13).
- **Atomic execute**: `History` applies a command to its own copy of the current document (`Arc::make_mut` copies only the touched path). On `Err` the copy is dropped and the session document is untouched.
- **Amendments to `docs/specs/02` (PROPOSAL, applied by the holder in their own commit, not here)**:
  1. §4.3: `apply`/`revert` take `&mut DocMut<'_>` instead of `&mut Document`; `Command` gains the `Any` supertrait so `merge` can downcast `next` (R11).
  2. §4.3: core command list for C3 = `SetProperty`, `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent`, plus `InsertPage` and `RemovePage` (Q4); the rest wait for their data.
  3. ARQ-30: "pasa por `History::execute` de `gafira-edit` (envuelto después por `EditService::execute`)"; enforcement is the `EditToken` capability plus V13 and `trybuild` (Q13). Restates C2b amendment 3, still pending.
  4. ARQ-32: the timed criterion is measured with the ARQ-40 benchmarks; C3 gives the structural proof (Q8).
  5. ARQ-81: V14 enforces it until `xtask` exists (Q9).
  6. §9.2: add `trybuild` (dev, MIT/Apache, VERIFICAR with `cargo deny`).
  7. §5.1: name the budget unit (Q7).

## Delivery plan

Measured size has run ×1.4 to ×1.9 over raw forecasts (mostly tests); C2b's deliveries averaged about ×1.5. The "Forecast" column below is already raw × 1.5 (exact files such as manifests, `clippy.toml` and CI lines at ×1.0). Each delivery is at most ~700 changed lines, measured with `git diff --shortstat <prev>...HEAD -- . ':!Cargo.lock' ':!openspec'`. Re-cut the remaining deliveries if one measures above ×1.3 of its forecast.

| # | Delivery | Forecast | Depends on |
|---|---|---|---|
| D1 | Crate skeleton (`Cargo.toml`, `clippy.toml` copy, `lib.rs`), V5 row, V12 count; `gafira-doc::msg` (`MessageKey`, `CommandLabel`); `Command` trait, `Applied`, `CommandError`, `DirtySet`, `MergeKey`, `GestureId`; `Transaction` with a test-only command | ~570 | `main` (f198790) |
| D2 | `gafira-testkit`: generator moved by `git mv` (rename-detected), `prop_command` harness (the 02 §10 laws), V5 row; `mod guards` of the moved generator gets `#[cfg(test)]` (else clippy dead-code errors in the library; its two guard tests then run in a separate binary, ~6 s on Linux); `gafira-doc` property tests switched to it | ~450 | D1 |
| D3 | `SetProperty` (node and shape fields) with its unit and property tests | ~600 | D2 |
| D4 | `History`: owned `Arc<Document>`, atomic `execute`, `debug_assert`, undo and redo by snapshot, redo cleared, `len`/`can_undo`/`can_redo`, `preview`, `Send + Sync` static checks, ARQ-32 structural tests | ~630 | D3 |
| D5 | `trybuild` dev-dependency, ARQ-30 fixtures (`tests/ui/`), V13 update, `edit-capability` REMOVED/ADDED, deny check | ~380 | D4 |
| D6 | Gesture merge: `begin_gesture`/`end_gesture`, 500 ms window on caller time, `merge` with downcast, ARQ-33 (300 moves -> one entry), merge law | ~525 | D4 |
| D7 | Memory budget: per-entry estimate (Q7), compaction of the oldest snapshots, undo/redo through `revert`/`apply` for compacted entries | ~630 | D6 |
| D8 | `InsertNodes` (ids allocated on first apply, reused on redo) and `RemoveNodes` (subtrees, positions restored), with properties | ~675 | D4 |
| D9 | `Reorder` and `Reparent` (cycle and container checks), with properties | ~630 | D8 |
| D10 | (Q4) Page and DPI targets of `SetProperty`, `InsertPage`, `RemovePage`, with properties; CI step V14 (ARQ-81) | ~600 | D9 |

Total forecast ~5 690. D5 to D7 and D8 to D10 are two chains after D4; they are listed in merge order, and each PR targets `main` or the previous unmerged delivery branch.

## Risky claims for explorer

Each claim needs a real experiment in a scratch project under `/tmp` before the delivery that relies on it. Results go to `spike.md`; a refuted claim is fixed in the specs first.

| # | Claim | Experiment | Needed by |
|---|---|---|---|
| R1 | A `trybuild` release at least two weeks old builds and runs on stable 1.93.1 inside `cargo test --workspace --locked`, and works offline once the registry cache is warm (its nested `cargo` call respects the copied `Cargo.lock`) | Scratch workspace with a `gafira-doc`-like crate; run with `--locked`, then with the network off (`CARGO_NET_OFFLINE=true`) | D5 |
| R2 | `trybuild`'s full dependency tree passes the current `deny.toml` with no allowlist change (expected: `serde`, `serde_json`, `toml` with `winnow`, `glob`, `target-tuple`, `termcolor` (Unlicense OR MIT), `windows-sys`); record any `multiple-versions` warning (spike R2: `cargo deny check` is fully ok, no `multiple-versions` warnings; the tree also has `itoa`, `memchr`, `zmij`, `serde_spanned`, `toml_datetime`, `toml_parser`, `toml_writer`, `syn`, `winapi-util`, `windows-link`) | `cargo deny check licenses bans sources` on the scratch lock; `cargo tree -p trybuild` | D5 |
| R3 | `.stderr` snapshots are identical on Windows, Linux and macOS (path normalization, `$DIR`, CRLF) with the pinned toolchain; otherwise the fixtures run on Linux only, which the spec must then say | Generate on Linux, compare on the CI matrix (a throwaway branch) | D5 |
| R4 | Fixture paths `crates/gafira-edit/tests/ui/*.rs` pass V13: they match the allowed regex, and the re-export `awk` (which scans every `.rs` under `crates/gafira-edit`, fixtures included) finds no `pub use`/`pub type` naming `EditToken` and no `pub use …::*` | Run the V13 body on a scratch tree with sample fixtures, including one that should trip it | D5 |
| R5 | The hidden constructor `EditToken::__new_for_gafira_edit_only()` compiles from a `trybuild` fixture crate, so no fixture may claim that building a token fails (Q13) | One pass fixture in the scratch project | D5 |
| R6 | `cargo mutants -p gafira-edit` works with `trybuild` tests in the crate (copied tree, nested build time); if not, how to exclude them (`.cargo/mutants.toml`, test name filter) without `#[mutants::skip]` | Run `cargo mutants --list` and one mutant on the scratch crate | D5 |
| R7 | The dev-dependency cycle `gafira-testkit -> gafira-edit/gafira-doc` and back compiles; integration tests (`tests/`) see one copy of each type, while unit tests in `src/` would see two (so the harness is used only from `tests/`); V5 (`-e normal,build`) ignores the cycle | Scratch workspace with the three crates | D2 |
| R8 | Moving `crates/gafira-doc/tests/common/mod.rs` (921 lines) with `git mv` keeps `git diff --shortstat` small (default rename detection), and `gafira-doc`'s property tests keep their runtime | `git mv` on a scratch branch, measure | D2 |
| R9 | The Q7 estimate tracks real heap growth closely enough (within a factor recorded in `spike.md`) for documents of 100 nodes and 20 pages, measured with a counting allocator in a scratch crate. A counting allocator in the repo needs `unsafe impl GlobalAlloc` (forbidden) or a dependency (`dhat`, `stats_alloc`): check its license, its `deny.toml` result, and that it needs no `unsafe` in our code | Scratch crate, both measurements | D7 |
| R10 | The domain `clippy.toml` (applied to all targets) does not block the plan: no `Instant::now`/`SystemTime::now` (time injected, Q3); no `thread::spawn` in tests (`Send + Sync` proven with static assertions, not threads); a private `static TOKEN: EditToken` compiles (`const fn` constructor, `EditToken: Sync`) and does not trip ARQ-31's search (`static mut`, `lazy_static!`, `inventory::`) | Scratch crate with the copied `clippy.toml`, `cargo clippy --all-targets -D warnings` | D1 |
| R11 | `trait Command: Any + Send + Sync + Debug` supports upcasting `Box<dyn Command>` to `Box<dyn Any + Send + Sync>` and `downcast::<T>()` on 1.93.1 (trait upcasting is stable since 1.86), so `merge(&mut self, next: Box<dyn Command>)` can recover a same-type `next`. Returning `next` unchanged on a mismatch does NOT work by value (a failed `Box<dyn Any>::downcast` cannot give back the `Box<dyn Command>`): `merge` probes by reference first (`(&*next as &dyn Any).is::<Own>()`), returns `Err(next)` on a mismatch, then upcasts and downcasts the box (spike R11) | Scratch crate | D1, D6 |
| R12 | A 300-event merged drag keeps one entry and does not copy the whole document per event (COW only along the edited path), and memory stays flat | Scratch test counting `Arc::ptr_eq` of untouched nodes after the drag | D6 |
| R13 | V5 rows for `gafira-edit` and `gafira-testkit` pass and constrain: V5 constrains only `gafira-*` edges and forbidden prefixes and does NOT enforce that `trybuild`/`proptest` are dev-only (that is checked by the D5 `cargo tree -i` scenario); a forced `gafira-*` edge fails (spike R13) | Run the V5 body on a scratch tree with both rows and one negative | D1, D2 |
| R14 | The `debug_assert!` (O(n) `validate` per command) keeps the 1 000-case property runs of every command within acceptable CI time on the three systems | Time `cargo test -p gafira-edit` in the scratch crate with the generator | D3 |
| R15 | Cross-document numbers agree: 512 MB (ADR-03, FIJADA via D-03) and 500 ms (§5.1) appear nowhere else with another value; ARQ-33's 300 events and ARQ-32's 100 layers / 1 000 undos match 01 if 01 cites them | `git grep` over `docs/specs` and `docs/specs/backlog` | D6, D7 |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Size overrun (C2b ran ×1.5, up to ×1.9 per delivery) | High | Ten deliveries ≤ ~700; re-cut after D1 if above ×1.3 |
| `trybuild` snapshots differ per platform | Med | R3 first (unverified locally: Linux stable only, CI throwaway run in D5); a `.stderr` quoting another crate's source line goes stale when that line changes; Linux-only fallback written into the spec |
| Backlog label mismatch: `docs/specs/backlog/04-funcionalidad-diseno-imagen-exportacion.md:614` labels 512 MB "(PROPUESTA)" while 02 and CAL-29 say FIJADA (spike R15) | Low | Out of this change's scope; backlog docs are not edited here, fix in a separate docs change |
| `trybuild` tree fails `deny.toml` | Low | R2 first; any exception is justified in writing (workspace-tooling) |
| Merge downcast impossible as specified in 02 | Low | R11 (confirmed in part: probe by reference before the by-value downcast); `Any` supertrait is amendment 1 |
| Budget estimate far from real memory | Med | R9; the budget is configurable; FIJADA figure kept as the default |
| Owner picks other options for Q4, Q5, Q7, Q12 | Med | Q4 work is isolated in D10; the others change single requirements |
| V13 trips on fixtures | Med | R4; fixtures never use `pub use`/`pub type` of the token or public globs |

## Rollback Plan

Revert per delivery, newest first. In full: delete `crates/gafira-edit/` and `crates/gafira-testkit/`; restore `crates/gafira-doc/tests/common/mod.rs` with `git mv` back and the `gafira-doc` dev-dependency; remove `gafira-doc::msg`; remove the root dependency lines, the `trybuild` lines of `Cargo.lock`, the V5 rows and V14; restore the V13 body. No persisted user data exists.

## Success Criteria

- [ ] Gates pass after every delivery (fmt, clippy, test, deny, V5, V11, V12, V13, V14 from D10), on Windows, Linux and macOS.
- [ ] `cargo tree -p gafira-edit -e normal,build` has only `gafira-geom`, `gafira-color`, `gafira-doc` among `gafira-*` crates, and no forbidden prefix (ARQ-01).
- [ ] Every core command satisfies `apply; revert` = previous state, `apply; revert; apply` = `apply`, and exact `touches`, in 1 000 generated cases (ARQ-81).
- [ ] `execute` that fails leaves the document pointer-equal to before; every successful command leaves `validate()` passing (ARQ-13).
- [ ] 300 move events inside one gesture raise `History::len()` by exactly 1 (ARQ-33).
- [ ] Undo and redo of a snapshot entry call no command method and restore the stored `Arc` (ARQ-32, structural).
- [ ] With a small budget, the oldest snapshots are compacted and undo through `revert` reaches the same states (ADR-03).
- [ ] `trybuild` fixtures: no-token mutations fail to compile with the recorded errors; an outside `Command` passes (ARQ-30).

## Proposal question round

The executor cannot ask interactively. The holder answers Q1 to Q15 in `design.md` (owner-level: Q4, Q5, Q7 unit, Q12). Until then the specs follow the recommendations.
