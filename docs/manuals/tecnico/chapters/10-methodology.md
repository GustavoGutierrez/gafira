# Development methodology

## Lite SDD

Specs already say what to build, so there are no separate proposal, design or tasks phases. The flow is:

```
spec -> (explorer, only when a claim is risky) -> tdd-apply -> mutants + verify -> archive
```

| Step | Agent (`.agents/agents/`) | Output |
|---|---|---|
| Measure risky claims in a scratch directory and cross-check numbers across docs | `explorer` | `openspec/changes/<change>/spike.md` with command, output and verdict |
| Build one delivery with strict TDD | `tdd-apply` | Commits and `openspec/changes/<change>/apply-progress.md` |
| Mutation-test the changed lines; report surviving mutants (read-only) | `mutants` | A report in the session scratch directory, outside the repo |
| Check gates and every requirement and scenario | `verify` | `openspec/changes/<change>/verify-report.md`, PASS or FAIL |
| Close the change after PASS and all deliveries done | `archive` | Delta specs merged into `openspec/specs/`; change moved to `openspec/changes/archive/` |

On FAIL, or when deliveries remain, the flow returns to `tdd-apply`.

In practice `mutants` and `verify` run in parallel on the same delivery. `verify` runs the local gates, checks every requirement and scenario against code and tests, reads the mutant report and writes `verify-report.md` with a PASS or FAIL verdict and numbered findings (critical, warning, suggestion). A FAIL opens a fix round: `tdd-apply` writes the tests first and records the round in `apply-progress.md`, then `verify` runs again on the result and the re-verify verdict goes into the same report.

The `coordinator` agent runs the whole flow: it reads the repository state, picks the next step, delegates to the agents above with one writer at a time, rejects "done" without evidence and stops for owner decisions and merges. Subagents cannot launch subagents, so the coordinator must be the main-thread agent. A short design note is written only when a spec leaves a real decision open.

## Strict TDD

RED (write the test, run it, see it fail), GREEN (minimal code), REFACTOR (tests stay green). One commit per reviewable work unit. Every commit passes the three local gates alone (see the tooling chapter). Where a test passes at once because the code already exists, RED is shown by a mutant instead (see the mutation testing section of the tooling chapter).

## Specs

| Place | Content |
|---|---|
| `docs/specs/00-decisiones.md` | Owner decisions (D-nn). They override other specs and are not reopened without the owner |
| `docs/specs/NN-*.md` | The document in progress; completed ones move to `docs/specs/archive/` with `git mv`, waiting ones sit in `docs/specs/backlog/` |
| `openspec/specs/<capability>/spec.md` | Main specs per capability, the source of truth, filled by archive |
| `openspec/changes/<change>/` | `proposal.md`, `design.md`, `tasks.md` (optional), `specs/<capability>/spec.md` (delta specs), `apply-progress.md`, `verify-report.md` |

Spec format: RFC 2119 keywords; scenarios as Given/When/Then; each requirement tagged with its delivery (`D1`, `D2a`...) and each scenario with its type (unit, property, integration, command, manual). Delta specs use `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements` (with a Reason and a Migration note) and `## RENAMED Requirements`. Archive is the only step that edits main specs, and it warns before any destructive delta.

Change chain so far: `workspace-scaffold`, `domain-color` and `domain-doc` are closed (the last two under `openspec/changes/archive/`); `edit-history` is open: its deliveries up to D8 (`Command` and `Transaction`, `History`, gesture merge, memory budget, `SetProperty`, `InsertNodes` and `RemoveNodes`, `gafira-testkit`, `trybuild`) are merged to `main`; D9 (`Reorder`, `Reparent`) is in progress and D10 (page and DPI commands, CI step V14) is **Planned**.

## Commits, branches and pull requests

- Conventional commits; no AI attribution lines.
- Trunk-based development with short-lived branches. After a delivery passes verify, the coordinator pushes the branch and opens a pull request against `main` (or the previous unmerged delivery branch). The owner merges, in order, bottom-up, with merge commits. A pull request whose base was merged is retargeted to `main` first.
- State lives in the change folder and in git, not in an external memory tool.
- CI is disabled manually on GitHub for now (billing), so each delivery's gates run locally before the pull request; see the tooling chapter.

## Code and language rules

- Rust edition 2024, toolchain 1.93.1; `unsafe` is forbidden; `#[allow]` is not used to silence lints.
- Domain crates use `BTreeMap` and `BTreeSet`, never `HashMap` or `HashSet`.
- Artifacts (code, comments, docs, specs, commits) are in English. User-facing UI text is Spanish and English (D-08). Product-facing documents under `docs/specs/` are in Spanish.
- No absolute or user paths in tracked files. Skills are referenced by name only.

## Documentation

This manual lives in `docs/manuals/tecnico/` and is maintained with `manual.py` (see the folder README). A change that alters a public API, a layering rule, a gate or the process updates the matching chapter in the same pull request, and marks anything not yet built as **Planned**.
