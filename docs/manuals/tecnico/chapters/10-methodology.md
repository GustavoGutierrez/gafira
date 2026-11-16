# Development methodology

## Status

| Area | Status | Source |
|---|---|---|
| Simple spec flow: numbered specs, status banner, main agent implements | **Implemented** | `docs/adr/0018-simple-spec-flow.md`, `AGENTS.md`, skill `gafira-specs` |
| TDD for critical logic, integration tests for the rest | **Implemented** | `AGENTS.md` "Rules" (Testing), skill `gafira-specs` |
| Single source for agents and skills, kept by `tools/sync_agents.py` | **Implemented** | `AGENTS.md` "Agent and skill sync", `.agents/README.md` |
| Earlier multi-agent flow (lite SDD with ODD lite) | **Retired** by ADR 0018; record kept read-only | `specs/done/legacy/` |

## Why the flow is simple

The earlier flow ran every request through a coordinator and up to eight repo agents, with risk levels, flow modes, Slice cards, an outcome state file and bounded verify rounds. It found real defects, but its ceremony often cost more than the work. Keeping the agents, their three runtime mirrors and the state file consistent was work of its own.

ADR 0018 replaces it. The main agent implements specs directly, with tests and the gates in `AGENTS.md`. The controls that remain are the gates, the CI policy steps and the test-first rule for critical logic. Independent verification rounds and mandatory mutation testing go away. Two alternatives were rejected: trimming the agents but keeping the routing rules, because most of the cost came from those rules, and deleting the old records, because code comments and ADRs still cite them.

## Specs and folders

The spec's status banner and `specs/README.md` are the state; git is the history.

| Place | Content |
|---|---|
| `specs/backlog/` | Specs not started: an idea, an open question, a pending decision |
| `specs/` | Active specs, in progress or in stable execution; also the product reference documents |
| `specs/done/` | Completed and verified specs; not reopened in place |
| `specs/README.md` | The index: active, done and backlog specs, and the next free spec ID |
| `specs/00-..07-*.md` | Product reference documents (owner decisions, product, architecture, features, open debt), not lifecycle specs |
| `specs/done/legacy/` | Record of the earlier flow; read-only |

Owner decisions in `specs/00-decisiones.md` (`D-nn`) win over the other documents and are not reopened without the owner. A new spec cites the product IDs it implements (`ARQ-12`, `FD-03`, `D-08`) and their sections.

## Spec ID and status banner

A spec ID has three digits, zero-padded, assigned when the spec is created: `008`, `009` and so on. IDs below `008` belong to the product reference documents. The file name is `NNN-kebab-case.md`, the title starts with the ID, and the ID never changes when the spec moves between folders (`git mv`, never a copy). For example, spec `009` (the engine `SettingsService`) lives in `specs/done/009-engine-settings-service.md`.

Every spec starts with a status banner right under its title:

```markdown
> **Estado:** backlog | in-progress | done — YYYY-MM-DD — evidence or blocker.
```

The banner moves with the folder: `backlog` in `specs/backlog/`, `in-progress` in `specs/`, `done` in `specs/done/`. A blocked spec stays in `specs/` with its blocker, or returns to `specs/backlog/` if work stops. A new scope for a closed spec gets a new spec ID. Spec bodies are written in Spanish.

## Definition of Ready

A spec leaves `specs/backlog/` only when it has all of these:

- Problem and expected outcome.
- Scope and non-goals.
- Dependencies and assumptions (crates, layers, ADRs, owner decisions).
- Functional requirements with IDs, its own or the product IDs it implements.
- Acceptance criteria that can be checked, and the evidence expected for done.
- TDD cases for its critical logic: the tests that must fail first.
- Risks and open decisions.

## Lifecycle

1. Capture: create the spec in `specs/backlog/` with the next free spec ID and the status banner, and add it to `specs/README.md`.
2. Refine: complete the Definition of Ready. The spec stays in backlog until the owner decides to start.
3. Start: branch from `develop`, `git mv` the spec to `specs/`, set `in-progress` and write the TDD cases first.
4. Execute: implement in work units, one conventional commit each.
5. Verify: run the tests and the gates the spec requires (see [Gates before done](#gates-before-done)).
6. Close: update the banner and the evidence, `git mv` the spec to `specs/done/`, update the index and merge into `develop`.
7. Check references: after any move, `git grep` the old path and fix the links.

A spec is done only when every acceptance criterion is checked, its new tests exist and pass, and no known blocker is hidden. Limitations and follow-up work are recorded in the spec.

## TDD for critical logic

Critical logic is the code where a defect corrupts data or breaks an invariant. In Gafira it covers the document model, JSON load and save and validation, ids and the arena, edit commands and history, the time model and timecode, color conversions, geometry, transforms and path operations, animation interpolation, size and DPI limits, file format parsing, and any spec rule with an invariant.

Critical logic follows red, green, refactor. Write the failing test first, run it and confirm that it fails for the expected reason, implement the minimum to pass, then clean up with the tests green. A bug fix in critical logic starts with a regression test. The rest of the code is validated with integration tests per feature in each crate's `tests/`; there is no end-to-end layer and no coverage threshold.

Tests cover the public contract and name the requirement ID they prove in their name or a one-line comment, so `git grep ARQ-nn` traces coverage. `cargo-mutants` is optional, only for critical logic, and not a CI gate (see the [tooling chapter](09-tooling-and-quality.md)). A docs-only spec cannot claim code TDD; it needs structural readback and link checks.

## Gates before done

While working, the gates are scoped to the touched crate (`-p <crate>`), and to `--workspace` when the change touches a crate others depend on, a `Cargo.toml`, a `Cargo.lock` or `ci.yml` (ADR 0017). The full suite runs once at spec close. `AGENTS.md` "Before calling something done" lists the gates:

| Gate | When |
|---|---|
| `cargo fmt --check`, workspace `clippy` with `-D warnings`, `cargo test --workspace --locked` | Always |
| `clippy` and tests of `gafira-testkit` with `--features ports` | `gafira-testkit` or `gafira-ports` changed |
| `cargo deny check` | Dependencies changed (when installed; CI runs it) |
| Policy steps of `ci.yml` (V5, V12, ARQ-107, V13, V14) | A crate, its layering or `ci.yml` changed |
| `python3 docs/manuals/tecnico/manual.py check` (and `--lang es`) | The technical manual changed |
| `python3 tools/sync_agents.py --check` | Always |

The exact commands, their results and the spec's evidence go in the spec. The tooling chapter describes each gate.

## Branches, commits and merges

- `main` holds released, stable code; `develop` is the integration branch (ADR 0003). Work happens on a short-lived branch cut from `develop` (`feat/<spec>-...`, `fix/...`, `docs/...`, `chore/...`).
- If the work passes its gates, the agent doing it merges the branch into `develop` with `git merge --no-ff`, pushes `develop` and deletes the branch.
- The only pull request is `develop` -> `main`. It opens only when the owner asks, or with the owner's OK at a release-worthy point, and the owner merges it. Releases and tags are cut from `main`, then `develop` is synced with `main`.
- Commits are conventional, one reviewable work unit each, with no AI attribution lines.
- The agent stops and asks the owner on a real block: an open product decision, a destructive or format-breaking change, or a change well beyond the spec's scope.

## Agents, skills and the sync script

`.agents/` is the single source of truth for agents (`.agents/agents/`) and skills (`.agents/skills/`). Claude Code, OpenCode and Pi read the same files through relative symlinks: `.claude/`, `.opencode/` and `.pi/` each hold an `agents` and a `skills` link to `.agents/`. Nobody edits agents or skills through those links.

The sync script `tools/sync_agents.py` creates missing links and repairs wrong or absolute ones. After any change in `.agents/`, run it, then confirm that its check mode passes:

```bash
python3 tools/sync_agents.py
python3 tools/sync_agents.py --check
```

`--check` changes nothing; it exits 1 on drift and 2 on an invalid source or a conflict. Shared agent frontmatter is limited to `name`, `description`, `mode: subagent` and `subagent_mode`; models and other per-runtime settings live in each runtime's config (`.agents/README.md`).

`manual-writer` is the only repo agent; it updates this manual when the owner asks. Skills are referenced by name and resolved through `.atl/skill-registry.md` (gitignored) with the `skill-registry` skill.

## Code and language rules

- Rust edition 2024, toolchain 1.93.1; `unsafe` is forbidden; `#[allow]` is not used to silence lints.
- Domain crates use `BTreeMap` and `BTreeSet`, never `HashMap` or `HashSet`.
- Code, comments, commits and generated artifacts are in English. Product specs under `specs/` are in Spanish. User-facing UI text is Spanish and English (D-08).
- No absolute or user paths in tracked files. Skills are referenced by name only.

## Documentation

This manual lives in `docs/manuals/tecnico/` and is maintained with `manual.py` (see the folder README). A change that alters a public API, a layering rule, a gate or the process updates the matching chapter in both languages, in the same branch, and marks anything not yet built as **Planned**.

## History

Before 2026-11-16 the repository followed a multi-agent flow: lite SDD with ODD lite (ADR 0001, 0002, 0004 and 0011). ADR 0018 supersedes those ADRs, amends ADR 0003 on who merges, and keeps the per-crate gate scope of ADR 0017. The record of that flow moved unchanged to `specs/done/legacy/`: closed changes in `history/`, capability specs in `capabilities/`, the open `gafira-engine` change folder in `changes/`, and the old index with its numbered follow-up list in `README.md`. That record is read, never rewritten. `specs/README.md` lists `gafira-engine` as in progress; resuming it means opening a spec with a new spec ID that cites the folder. The [glossary](11-glossary.md) keeps the terms of that flow in its "Historical terms" section.
