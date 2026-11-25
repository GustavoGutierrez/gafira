---
name: gafira-specs
description: "Trigger: specs, spec workflow, backlog, done, numbering, spec status, status banner, specs/README.md index, Definition of Ready, TDD, test-first, moving or closing a spec, requirement IDs (ARQ-, RF-, RNF-, CAL-, FD-, FV-, D-). Organize Gafira specs with three-digit IDs, status folders and TDD for critical logic."
license: GPL-3.0-or-later
metadata:
  author: Gafira
  version: "1.0"
---

# Gafira spec workflow

Rules for creating, moving and closing specs in this repository. The index is
`specs/README.md`; this skill is the contract. Gates and code rules live in
`AGENTS.md`.

## Folder contract

- `specs/` — **active specs**: in progress or in stable execution. Also holds the product reference documents `00-..07-*.md`.
- `specs/backlog/` — **not started**: pending review, pending a decision, blocked before start, or just an idea. No implementation work here.
- `specs/done/` — **completed and verified specs only**. Closed specs are immutable except for status or link corrections.
- `specs/done/legacy/` — record of the earlier multi-agent flow (changes, history, capabilities, old index and follow-up list). Read-only.
- `specs/README.md` — index, not a spec: active, done and backlog specs, and the next free ID.
- `docs/` — manuals, ADRs (`docs/adr/`), research and reference material. Documentation does not consume a spec ID.

Product reference documents (`00-decisiones.md`, `01`, `02`, `04`, `05`, `07`
in `specs/`; `03`, `06` in `specs/backlog/`) keep their two-digit names and
their own `Estado` header row. They are not lifecycle specs: a new spec cites
their IDs (`ARQ-12`, `FD-03`, `D-08`) and sections (`02 §5.4`). `00-decisiones.md`
holds owner decisions and wins over the others; it is never reopened without the
owner. An empty active set is valid: it means nothing is being executed now.

## Numbering and names

- Three digits, zero-padded: `008`, `009`, ... `999`. IDs below `008` belong to the product reference documents.
- File name: `NNN-kebab-case.md`. One spec per file.
- Assign the next free number when the spec is created. Never reuse, renumber or reorder.
- The ID is immutable across `backlog -> active -> done` moves (`git mv`, never copy).
- The title includes the ID: `# NNN — Título`.

## Status banner

Every spec starts with an explicit status banner right under its title:

```markdown
> **Estado:** backlog | in-progress | done — YYYY-MM-DD — evidence or blocker.
```

Transitions:

1. Idea, open question or pending decision -> `specs/backlog/`, `backlog`.
2. Ready to execute and dependencies satisfied -> move to `specs/`, set `in-progress`.
3. All acceptance criteria and tests/evidence pass -> move to `specs/done/`, set `done`.
4. Blocked mid-execution -> keep it in `specs/` with the blocker, or return it to `backlog/` if work stops. Never mark it done with open blockers.
5. A completed spec is not reopened in place. A new scope gets a new ID; the old spec stays in `done/`.

## Definition of Ready

A spec can leave `backlog/` only when it has:

- Problem and expected outcome.
- Scope and non-goals.
- Dependencies and assumptions (crates, layers, ADRs, owner decisions).
- Functional requirements with IDs (its own, or the product IDs it implements).
- Acceptance criteria that can be checked.
- TDD cases for its critical logic: the tests that must fail first.
- Evidence expected for done.
- Risks and open decisions.

If one of these is missing, it stays in `backlog/`.

## TDD for critical logic

TDD only for critical logic; the rest is validated with integration tests per
feature; full suite and gates at spec close. Critical logic in Gafira: document
model, JSON load/save and validation, ids and arena, edit commands and history,
time model and timecode, color conversions, geometry, transforms and path
operations, animation interpolation, size/DPI limits, file format parsing, and
any spec rule with an invariant.

1. **Red:** write the failing test first. Name the requirement ID it proves in the test name or a one-line comment.
2. **Verify red:** run it and confirm it fails for the expected reason, not a typo or missing setup.
3. **Green:** implement the minimum to pass.
4. **Refactor:** clean up with the tests green.
5. Repeat per work unit; do not batch implementation and backfill tests later.

Commands while working (scope to the touched crate; `--workspace` when it has dependents):

- `cargo test -p <crate> --locked`
- `cargo clippy -p <crate> --all-targets --locked -- -D warnings`
- Optional, critical logic only: `cargo mutants -p <crate> --jobs 8 -o <scratch dir>`.

Hard rules:

- Do not move a spec to `done/` if its new tests are missing or failing.
- For critical logic, do not write the implementation first and add tests afterwards.
- A bug fix in critical logic starts with a regression test.
- A docs-only spec cannot claim code TDD; it needs structural readback and link checks (`manual.py check` for the manual).
- Record the exact commands and results in the spec's evidence section.
- If a test cannot be automated, record the manual procedure and why; never invent a passing result.

## Definition of Done

A spec is done only when:

- Every acceptance criterion is checked.
- New/updated tests exist and pass; integration tests cover what unit tests cannot.
- The gates in `AGENTS.md` "Before calling something done" pass (fmt, clippy and tests over the workspace).
- `README.md`, `AGENTS.md` and `specs/README.md` are updated when behavior, commands or status changed.
- The status banner is updated and the file is moved to `specs/done/`.
- Limitations and follow-up work are recorded; no known blocker is hidden.
- Cross-links still resolve after the move.

## Workflow

1. **Capture:** create the spec in `specs/backlog/` with the next ID and the status banner; add it to the index.
2. **Refine:** complete the Definition of Ready; keep it in backlog until the owner decides to start.
3. **Start:** branch from `develop`, `git mv` it to `specs/`, set `in-progress`, write the TDD cases first.
4. **Execute:** implement in work units, one conventional commit each.
5. **Verify:** run the tests and gates the spec requires.
6. **Close:** update the banner, `git mv` it to `specs/done/`, update the index and the evidence, merge into `develop`.
7. **Reference check:** after any move, `git grep` the old path and fix the links.

## New spec template

```markdown
# 009 — Título

> **Estado:** backlog — 2026-11-16 — pendiente de decisión.

## Objetivo

## Alcance / no alcance

## Dependencias

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |

## TDD

Casos que deben fallar primero:

1. ...

## Evidencia esperada

## Riesgos y decisiones abiertas
```

## Boundaries

- Brand and UI rules come from `gafira-brand`; the manual from `technical-manual`. This skill governs only the spec lifecycle and TDD.
- Do not invent a done state, a test result or a passing run.
- Do not move a spec just to keep the folder tidy; move it only when its state is true.
- Spec bodies are Spanish (existing convention); code, comments and commits are English.
