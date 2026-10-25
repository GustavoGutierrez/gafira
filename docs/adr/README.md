# Architecture decision records

Cross-cutting technical decisions that outlive one change. Written by the `architect` agent; the owner confirms them through the coordinator.

Where a decision lives:

| Kind | Location |
|---|---|
| Owner decisions | `docs/specs/00-decisiones.md` (never reopened by the architect) |
| Cross-cutting technical decisions | `docs/adr/NNNN-short-title.md` (this folder) |
| Decisions scoped to one change | `openspec/changes/<change>/design.md`, section "Decisions" |

## Rules

- Numbers are sequential and never reused: `0001-short-title.md`.
- A new ADR starts as `proposed` and becomes `accepted` when the owner confirms it.
- An accepted ADR is not edited. A new ADR supersedes it; the old one changes only its status to `superseded by NNNN`.
- Keep it short: one decision per file.

## Index

| ADR | Title | Status |
|---|---|---|
| (none yet) | | |

## Template

```markdown
# NNNN. Short title

- Status: proposed | accepted | superseded by NNNN
- Date: YYYY-MM-DD
- Related: requirement IDs, owner decisions (`D-nn`), change names

## Context

Forces and constraints that make a decision necessary.

## Decision

What we will do, in one or two sentences, in the active voice.

## Consequences

What becomes easier, what becomes harder, follow-up work and risks.

## Alternatives considered

Each option with the reason it was rejected.
```
