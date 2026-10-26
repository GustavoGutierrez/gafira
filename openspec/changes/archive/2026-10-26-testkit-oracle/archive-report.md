# Archive report: testkit-oracle

Archived 2026-10-26 on branch `spec/testkit-oracle` (verified commit `97c85e1`, base `main` at `aea45a7`). Archive basis: `verify-report.md`, verdict PASS, 0 CRITICAL, standard mode, depth V3, tier T4. `verification.json` kept beside it. Whole change, one delivery.

## Merged into `openspec/specs/`

| Capability | Action |
|---|---|
| `testkit` | New capability, copied from the delta (`cp`). Heading `# Testkit Specification (delta for ...)` renamed to `# Testkit Specification`; `## ADDED Requirements` renamed to `## Requirements` (same as `edit-commands`). No requirement text changed. |

Delta contents: ADDED only. No MODIFIED, REMOVED or RENAMED block.

Diff of the source against the merged spec (after the heading edits):

```
1c1
< # Testkit Specification (delta for `testkit-oracle`)
---
> # Testkit Specification
89c89
< ## ADDED Requirements
---
> ## Requirements
```

Before the heading edits, `diff` source vs destination was empty (`cp` only).

## Destructive changes

None. No requirement is removed or modified in any main spec.

## Mutation results (T4, `gafira-testkit`)

| Run | Found | Caught | Unviable | Missed | Timeouts |
|---|---|---|---|---|---|
| Baseline (Step 0, before the first test commit) | 143 | 82 | 23 | 38 | 0 |
| Final, implementer run (`apply-progress.md`) | 143 | 118 | 23 | 2 | 0 |
| Final, independent run (separate `mutants` agent) | 143 | 118 | 23 | 2 | 0 |

The archived `edit-history` figures (84 caught, 36 missed) did not reproduce on this tree; the baseline of record is 38 missed (C5, spec "Baseline"). Two survivors are accepted equivalents: `crates/gafira-testkit/src/lib.rs:235:18` (C2) and `crates/gafira-testkit/src/harness.rs:84:9` (`failure_persistence` field, documented as an equivalent in the `edit-history` archive report). No exclusion was added (`.cargo/mutants.toml` unchanged).

## Accepted decisions and follow-ups

- Two accepted equivalent survivors (above). Non-critical test helper.
- Pre-existing flake (C6, follow-up, not fixed here): `tests/harness.rs` `each_broken_law_is_named_with_a_counterexample` fails intermittently (about half of runs) with `PROPTEST_CASES=1`. The new oracles in `tests/generator.rs` and `tests/corruption.rs` pass with it set or unset. The scenario "Oracles ignore PROPTEST_CASES" is scoped to those two files.
- Separate docs follow-up (not part of this change): the document-model text about the generator location is stale. Not edited here.
- Platforms: Windows and macOS are unobserved. Evidence is Linux only.
- CI is disabled (no GitHub Actions runs); the CI `policy` steps were run locally.
- Size: 970 insertions at `97c85e1` (`git diff --shortstat main...HEAD`), over the ~700 ceiling. Accepted by the owner's request to do the whole change as one delivery.
- `docs/specs/README.md`: no status row exists for `testkit-oracle`; no docs status edit made.

## Archive

Folder moved to `openspec/changes/archive/2026-10-26-testkit-oracle/` with `git mv`. `next: none`.
