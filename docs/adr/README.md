# Architecture decision records

Cross-cutting technical decisions that outlive one change. Written by the `architect` agent; the owner confirms them through the coordinator.

Where a decision lives:

| Kind | Location |
|---|---|
| Owner decisions | `specs/00-decisiones.md` (never reopened by the architect) |
| Cross-cutting technical decisions | `docs/adr/NNNN-short-title.md` (this folder) |
| Decisions scoped to one change | `specs/changes/<change>/design.md`, section "Decisions" |

## Rules

- Numbers are sequential and never reused: `0001-short-title.md`.
- A new ADR starts as `proposed` and becomes `accepted` when the owner confirms it.
- An accepted ADR is not edited. A new ADR supersedes it; the old one changes only its status to `superseded by NNNN`.
- Keep it short: one decision per file.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-flow-modes-and-packs.md) | Flow modes, request packs and graded mutation testing | accepted, partly superseded by 0002 and 0004, amended by 0011 |
| [0002](0002-fast-lane.md) | Fast lane: spec-direct execution, mutation toggle, lighter defaults | accepted, extended by 0004, amended by 0011 |
| [0003](0003-develop-integration-branch.md) | Develop integration branch, PR to main on request | accepted |
| [0004](0004-hand-mutant-split-and-verify-rounds.md) | Hand-mutant split and bounded verify rounds | accepted, amended by 0011 |
| [0005](0005-layer-2-shared-contracts.md) | Layer-2 shared contracts live in a `gafira-doc` module | accepted |
| [0006](0006-format-evolution-before-first-release.md) | Schema 1 stays open until a build can save, and every field is written | accepted |
| [0007](0007-layer-2-edges-and-v5-rows.md) | Layer-2 dependency edges and V5 rows | accepted |
| [0008](0008-persisted-data-in-doc-engine-inputs-in-engines.md) | Persisted types live in `gafira-doc`; engines own their inputs | accepted |
| [0009](0009-node-placement-is-page-space.md) | Every node's `Placement` is in page space, group children included | accepted |
| [0010](0010-pixel-golden-format.md) | Pixel goldens are PNG files compared on raw bytes | accepted |
| [0011](0011-odd-lite.md) | ODD lite: outcome-first routing on top of the existing flow | accepted |

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
