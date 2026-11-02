# Verify report: engine-contracts D1, round 1

Commit `4e1f742`, branch `feat/engine-contracts-d1`, base `develop` 6bede8b. Depth V2, mode full, `mutation: archive`, no `mutants` report (not a finding before close). Tier T4 at close.

**Verdict: FAIL.** 2 CRITICAL, 2 WARNING, 2 SUGGESTION. The code met every probe; the failures are a surviving hand mutant in critical code and a missing G0 record.

## Gates

Each run alone, exit code read directly, on a clean export of HEAD (policy steps from the worktree, because V13 and V14 need git).

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1161 passed, 86 result lines, 0 failed) | 0 |
| `cargo deny check` | 0 |
| Policy: ARQ-07, forbid unsafe, V5, V12, V13, V14 (six steps of `ci.yml`, each run alone) | 0 each |

Disk: 38 GB free at start.

## Findings

### CRITICAL

**C1. No G0 record.** `class: ambiguity`. Ref: `.odd/state.json`, `design.md:10`, `design.md:453`.
`.odd/state.json` has `outcomes: []` on `develop` and on the branch, and the folder has no Slice card, Light spec or delta spec. `design.md` tells the coordinator to record the G0 entry from "Acceptance", and it was not recorded. `coordinator.md` G0 lists those four artifacts only and calls G0 not waivable; step 3 makes a missing one CRITICAL.
Mitigating fact: `design.md` holds scope, an out-of-scope list and the acceptance list, it precedes the first implementation commit (`7da03f3` is an ancestor of `03e19cc`), and no branch commit changed it. So the acceptance did not move under the implementer. The defect is the missing record, and its criteria are not typed. Whether `design.md` is enough for a full-mode change, or a late entry can stand in, is for the coordinator and owner.

**C2. Surviving hand mutants in `is_valid_key`.** `class: implementation`. Ref: `crates/gafira-doc/src/msg.rs:72`, rule C2 (`[a-z0-9_]`).
`is_valid_key` is critical code (`design.md`, Critical code). Two mutants widen the character class of a segment to neighbouring ASCII bytes, and all tests stay green:
- IM2a: `is_ascii_digit()` becomes `b >= b'/' && b <= b':'`, so `a.b/` and `a.b:` are accepted. `cargo test -p gafira-doc` exit 0.
- IM2b: `is_ascii_lowercase()` becomes `` b >= b'`' && b <= b'{' ``, so ``a.b` `` and `a.b{` are accepted. Exit 0.

The code is correct: the independent probe tests every byte in four positions and passes on it, and fails on IM2a, so the mutant is not equivalent. The gap is in `arq_107_is_valid_key_follows_the_grammar`, whose table has no byte adjacent to the `0-9` and `a-z` ranges (only `-`, space, `A`, `B`, `é`, 0x7f). The design's table is met; the rule is wider than the table. cargo-mutants would not produce these mutants, so T4 will not catch them. Rows for the neighbouring bytes of each range, in continuation position, close it.

### WARNING

**W1. Flow line disagrees with the design.** `class: ambiguity`. Ref: `apply-progress.md:1`, `design.md:13`.
`design.md` says `level: L2 | new capability`; `apply-progress.md` says `level: L3 | pack: new function`. No `Flow-change:` line records the move. The mode (full), `mutation`, tier and depth agree, so this round's obligations do not change.

**W2. One criterion has no RED, and the stated reason does not hold.** `class: implementation`. Ref: `apply-progress.md` Deviations, `msg.rs` `arq_102_every_arg_variant_and_unit_builds_and_compares`.
The deviation says declarations cannot fail before they exist. Replacing the `PartialEq` derive of `Message` with an impl that returns `false` fails this test on an assertion, so a RED was available; `specs/history/2026-10-25-edit-history/apply-progress.md:42` used that same stub. The test itself is sound. G3 asks for RED per criterion.

### SUGGESTION

- S1. `msg.rs` non-test code has 0 string literals, which meets ARQ-107. Its `#[cfg(test)]` module has invalid keys and `"should accept {k:?}"`. Moving the grammar test to `crates/gafira-doc/tests/` would keep the file literally free of them for a future scanner.
- S2. The `apply-progress.md` RED table merges two tests into one row with one test's output. The commit body has each correctly.

## RED evidence, per commit

Each commit was exported, its touched crate tested green, then the stub its body describes was rebuilt and the tests run. The stubs are not in git, so they are reconstructions.

| Commit | Stub | Result | Matches the record |
|---|---|---|---|
| `03e19cc` | `is_valid_key` returns `false` | 1 failed: `should accept "edit.set_property"` at `msg.rs:75:13` | yes, same line and column |
| `6b7224d` | `msg!` returns an empty `Message` | 3 integration tests failed, `compile_fail` doctest "compiled successfully" | yes, all quoted values; the example doctest fails too, unrecorded |
| `baae7bc` | constant returns | 7 of 7 failed | yes, all quoted values |

All are assertion failures, not compile errors.

Limit: this proves the tests fail against those stubs. It does not prove the order. Test, stub and implementation land in one commit, so "test written first" rests on the commit body alone, and git cannot show whether a stub preceded the assertions. The matching `75:13` shows the recorded run happened against a stub of that size and that test text, not which was typed first.

## Criteria

| Criterion | Proof | State |
|---|---|---|
| Every `MsgArg`, `Unit` builds; `Message` `Clone`, `PartialEq` | `arq_102_every_arg_variant_and_unit_builds_and_compares`; probe with exhaustive matches | covered |
| `msg!` 0, 1, 2 args, trailing comma, order | `tests/msg_macro.rs` (3 tests); probe | covered |
| Invalid key does not compile | `compile_fail` doctest, shown non-vacuous by IM1; probe of 9 bad forms, E0080, including generic and uncalled functions; non-literal key rejected | covered |
| `is_valid_key` acceptance table | `arq_107_is_valid_key_follows_the_grammar`: all 13 listed rows present | covered |
| C2 grammar beyond the table | same test | weak, C2 |
| ARQ-102 types | as above; `ToMessage` object-safe in probe | covered |
| ARQ-107 for D1 | no `Locale` in crates of layers 0 to 2; no `icu_*` or `gafira-i18n` in the workspace tree; 0 literals in `msg` non-test code; V5 exit 0 | covered |
| C1 `msg!` from a second crate | `tests/msg_macro.rs`; probe in `gafira-edit`; `use gafira_doc::msg` works as module and macro | covered |
| C10 `IRect` rules, `i32::MIN..i32::MAX`, saturation, inverted | 7 tests in `irect.rs`; probe against an `i64` model over boundary grids, const use | covered |
| V5 rows pass (D1 part) | policy exit 0 | covered |

10 criteria, 9 covered, 1 weak. D2a, D2b and D3 items are out of this delivery.

Probes (step 1c, written before reading the tests, on a scratch export): 15 tests and 12 compile checks, all as the spec expects. No code defect found.

Derives match `design.md` exactly. No serde, `union` or `Rect` conversion. `.cargo/mutants.toml`, `design.md`, `specs/02`, and ADRs unchanged by the branch.

## Hand mutants

Spot-check of the recorded list, item 4: uppercase in a continuation byte. `arq_107_...` fails on `a.bB`. Killed.

| # | Rule | Mutant | Result |
|---|---|---|---|
| IM1 | C1, invalid key does not compile | `const` assertion removed from `msg!` | killed, `compile_fail` doctest |
| IM2a | C2, `[a-z0-9_]` | digit range widened to `/` and `:` | **survived**, C2 |
| IM2b | C2, `[a-z0-9_]` | lowercase range widened to `` ` `` and `{` | **survived**, C2 |
| IM3 | C10, a negative value deflates and inverted is legal | `inflate` clamps so the result never inverts | killed, `inflate_moves_each_side_outward_and_saturates` |

5 run, 3 killed. A fifth independent attempt did not compile through my own quoting error; it was dropped and is not counted. The `PartialEq` stub in W2 is not a mutant of this list.

## Size

`git diff --shortstat`: 427 lines of crates, 487 with `apply-progress.md`. Forecast 370 (x1.4: 520).

## Not run

Per-commit full gates are V3 and were not run. `cargo test -p` of the touched crate passed at each of the three commits.

next: specifier, then tdd-apply (classes: ambiguity, implementation)
