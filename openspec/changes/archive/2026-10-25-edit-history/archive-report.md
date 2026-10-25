# Archive report: edit-history (C3)

Archived 2026-10-25 on branch `chore/archive-edit-history` (from `main` c4a4771). Archive basis: `verify-report.md` "Whole-change re-verify (V1, before archive)", verdict PASS, 0 CRITICAL; `verification.json` kept beside it. Deliveries D1 to D11 are done.

## Merged into `openspec/specs/`

| Capability | Action |
|---|---|
| `document-model` | ADDED "Message key for command labels (ARQ-107) (D1, Q11)" appended |
| `edit-capability` | MODIFIED "Id allocation (D5b)" and "V13 CI step (D5b)"; REMOVED "trybuild is deferred (D5b)"; ADDED "trybuild evidence for ARQ-30 (ARQ-30, ARQ-20) (D5, Q13)" |
| `workspace-tooling` | MODIFIED "Formatting and per-crate Clippy policy", "Per-crate allowed-dependency table (V5)" (six rows, `gafira-time` kept), "Policy checks cover every domain crate (V11, V12)" (see destructive changes), "V13 EditToken step (D5b)"; ADDED "Compile-test dependency (trybuild) (ARQ-30, ARQ-70, ARQ-71) (D5)" and "V14 command property coverage (ARQ-81) (D10, Q9)" |
| `edit-commands` | New capability, copied from the delta; `## ADDED Requirements` renamed to `## Requirements` |
| `edit-history` | New capability, copied from the delta; `## ADDED Requirements` renamed to `## Requirements` |

The "Purpose" sections of the deltas are not merged (the main specs keep their own). No "Owner decisions" or "Closed decisions" record was found as a separate section in any delta; owner decisions appear inline in requirement bodies and are kept there.

## Destructive changes

1. `workspace-tooling`, "Policy checks cover every domain crate (V11, V12)": scenario **"Too few copies fails V12 (H-N4, manual negative test)" dropped**. Owner decision 2026-10-25 (`apply-progress.md`, "Owner decision: V12 copies derived from the V5 table"): the `>= 3` copy floor is removed and V12 derives its copies from the V5 table.
   Rename, not a loss: "Hardened V12 timing (command)" is now "Hardened V12 timing (C2b history, command)" (body identical).
2. `edit-capability`, REMOVED "trybuild is deferred (D5b)": requirement removed together with its scenarios **"Lock unchanged except gafira-doc (command)"** and **"Deferral recorded (manual)"**. The delta carries Reason and Migration notes; the replacements are "trybuild evidence for ARQ-30" and "Compile-test dependency (trybuild)". This removal was not in the archive brief and is recorded here.

## Documentation commits (not part of the spec merge)

- `docs/specs/02-arquitectura-software.md`: ARQ-32 wording ("ni a `apply` ni a `revert`, solo a `label()`"), §4.3 `SetProperty` as one command with a `Property` enum in the root of `gafira-edit`, and the `touches` step. Commit `857b4a6`.
- `docs/specs/README.md`: status row updated (C3 done, no active 02 change). Commit `1a05ad1`. Document 02 is not archived.

## Mutation results (T4)

| Crate | Total | Caught | Missed | Status |
|---|---|---|---|---|
| `gafira-edit` | 281 | 227 | 6 at T4 | All six closed in the follow-up: 3 by tests (`CommandError` Display, `History::is_empty`), 1 equivalent and excluded (`History::debug_check`, `.cargo/mutants.toml`, commit 8a59fc8), 2 gone after the `touches` refactor. T3 re-run: 54 found, 43 caught, 0 missed. |
| `gafira-doc` | 149 | 120 | 0 | Figures from the archive brief; the change records do not contain this run. |
| `gafira-testkit` | 143 | 84 | 36 | Non-critical test helper. Not excluded. 35 of the misses may be killed by a future testkit-oracle test change; deferred and accepted. |

`harness.rs:84` (`failure_persistence` field, equivalent): **the requested exclusion was not added.** In cargo-mutants 27.1.0 an `exclude_re` does not remove a `delete field` (StructField) mutant. `harness.rs:84:9: delete field failure_persistence from struct Config expression in try_prop_command` stayed listed with the exact regex and with `^`, while ordinary mutants (`History::debug_check`) are removed. A config entry would be a silent no-op, so `.cargo/mutants.toml` is unchanged. Options for the owner: accept it as a documented equivalent miss, or refactor the helper to an assignment in a separate delivery.

## Accepted risks

- Cross-platform CI (spec R3, whole-change verify W2): no GitHub Actions run has started since D5 (account payments failed). Windows and macOS `trybuild` `.stderr` identity and the D5 to D11 gates on those platforms are unverified. Owner acceptance 2026-10-25 (option B, `apply-progress.md`, "Owner acceptance: cross-platform CI"): archived on Linux evidence. R3 is not marked verified.
- Follow-up condition: once CI works, run the full Windows, Linux and macOS matrix on `main` first. If a `.stderr` differs, restrict `tests/ui.rs` to Linux and record it in a follow-up.

## Deferred and next archive (`domain-time`)

- Archive order: `edit-history` first, then `domain-time`. The `domain-time` `workspace-tooling` delta is ADD-only ("gafira-time domain crate wiring (V5 row, clippy.toml copy) (ARQ-01, ARQ-02, ARQ-10) (D0)"). No name collides with the merged main spec; the `gafira-time` V5 row and copy are already in the merged table, so do not add them again.
- `domain-time` must fix its own spec first: "`lib.rs` MUST start with `#![forbid(unsafe_code)]`" is false (the attribute is on line 6, after the `//!` docs). Reword it, then run its own verify before its archive.
- Carried suggestions, not blocking: `design.md` line 134 still says "call no command method" (history only); the V5 requirement cites `ci.yml:98` (now line 102).
