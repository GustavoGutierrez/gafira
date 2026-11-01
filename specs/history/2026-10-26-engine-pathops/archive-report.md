# Archive report: engine-pathops (C4)

- Archived: 2026-10-26, from `openspec/changes/engine-pathops/` to `openspec/changes/archive/2026-10-26-engine-pathops/`.
- Flow: full | tier T3 | depth V3 (from `apply-progress.md`). Deliveries D1-D4 all done; crate `gafira-pathops`.
- Final verdict: round 6 PASS at `3afbf01` (0 CRITICAL, 0 WARNING, 0 new SUGGESTION). Gates at that commit: fmt, clippy, test (767 passed, 0 failed), cargo deny, ARQ-07, forbid-unsafe, V5, V12, V13, V14, all rc 0.
- Merged capabilities: `path-operations` (new, copied with `cp`); `workspace-tooling` (two MODIFIED requirements replaced by name).

## Verify rounds

| Round | Commit | Verdict | Notes |
|---|---|---|---|
| 1 | `2a9f1a7` | FAIL | 9 CRITICAL, 9 WARNING, 6 SUGGESTION. C8 raised (flaky property in D2-D4). |
| 2 | fix round 1 | FAIL | 2 CRITICAL, 4 WARNING, 5 SUGGESTION. C8 carried to the owner. |
| 3 | fix rounds 2-3 | FAIL | 2 CRITICAL, 5 WARNING, 4 SUGGESTION. Third round, owner asked. |
| 4 | fix round 4 | FAIL | Owner-authorized final fix round: 2 CRITICAL, 2 WARNING, 5 SUGGESTION. |
| 5 | closing pass | FAIL | 1 CRITICAL (process only): implementer-authored delta-spec commits needed ratification. Code clean. |
| 6 | `3afbf01` | PASS | Ratification recorded in `a67a03e`. No code, CI or dependency change since round 5. |

## Mutation (T3)

- Round 4, full T3 over `simplify.rs`, `path.rs`, `boolean.rs`, `hittest.rs`, `validate.rs`, `stroke.rs`, `error.rs` and `lib.rs`: 126 mutants, 105 caught, 0 missed, 0 timeouts, 21 unviable. No critical mutant survives. The `.cargo/mutants.toml` exclusion (`replace < with <= in Path::distance_to_perimeter`) has a reason and is sound.
- Round 5 closing pass, files changed in that pass (`simplify.rs`, `path.rs`): 51 found, 46 caught, 5 unviable, 0 missed, 0 timeouts. Round 6 reused this evidence because it changed no code.
- Hand mutants (round 5): 5 run, 4 killed, 1 equivalent.

## Owner decisions and waivers

- C8 (flaky property in `9a760e0`, `754099b`, `f198f6a`, `15f4b30`): waived on 2026-10-25. History is not rewritten. The fix is in `a8d2c0a`. The property `the_difference_is_contained_in_its_first_operand` was removed; its replacement is `the_boxes_of_the_results_are_contained`. HEAD passed 0 failures in 80 runs (round 2), and round 6 passed 767 tests.
- Flaky-property history: the D2 note "fixed before the D2 commit" in `apply-progress.md` is false and is kept as an append-only correction.
- Owner decisions OQ1, OQ1 follow-up and OQ2 recorded in `apply-progress.md` ("Owner decisions after verify round 1").
- Ratification of `61fb23a`, `f89f883`, `951e777` and `5f9ead0` by the owner's instruction to close the change (2026-10-26), recorded in `a67a03e`.

## Size

- Whole change against `main`: `git diff --shortstat c7187ca..a67a03e`: 36 files, +8648/-1.
- This is a single PR beyond the ~700-line ceiling. The owner approved a single PR.
- Deliveries measured at D1-D4 time: D1 745 (670 in code), D2 552, D3 589, D4 379. Forecast for D1 was 420.

## Known gaps (accepted, not counted as blockers)

- Windows and macOS: not observed. The spike ran on Linux x86_64 only (`design.md` R8 "not verified"). Platform support for these targets is unproven.
- No initial `MoveTo` (round 4 S1, "Not settled"): a path with no initial move is accepted. The spec is silent, the gap only shows in release (kurbo debug-asserts at construction), and the union reads it from the origin. `close_subpaths` returns it unchanged while `validate_closed` reports `OpenSubpath`.
- `simplify` `Z Z` collapse choice (round 5 S1, informational): a zero-length subpath keeps its doubled close (`M(1,1) L(1,1) Z Z` comes back unchanged), while every other subpath collapses `Z Z`. Harmless (P3: every operation accepts it, and the result is idempotent). Kept as is; one canonical form would need a code change.

## Follow-ups

- `openspec/specs/path-operations/spec.md` was copied verbatim from the delta, as the archive procedure requires. It still carries the delta title "(delta for C4 `engine-pathops`)" and a `## ADDED Requirements` heading. The `workspace-tooling` main spec uses `## Requirements`. Normalising the heading is a separate edit.
- `docs/specs/README.md` 02 row updated to record C4 as archived.
