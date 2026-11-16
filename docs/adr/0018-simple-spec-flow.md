# 0018. Simple spec flow: one agent, numbered specs, TDD for critical logic

- Status: accepted
- Date: 2026-11-16 (decided by the owner)
- Related: supersedes ADR 0001, 0002, 0004 and 0011; amends ADR 0003 (who merges) and ADR 0017 (the gate scope stays); `AGENTS.md`, skill `gafira-specs`, `specs/README.md`, `specs/done/legacy/`

## Context

The lite SDD flow with ODD lite (ADR 0001, 0002, 0004, 0011) ran every request through a coordinator and up to eight repo agents (`specifier`, `architect`, `explorer`, `tdd-apply`, `mutants`, `verify`, `archive`), with risk levels, flow modes, packs, Slice cards, outcome entries in `.odd/state.json`, bounded verify rounds and mutation tiers. It found real defects, but its ceremony cost more than the work on many deliveries, and keeping the agents, their three runtime mirrors and the state file consistent was work of its own. A sibling project runs a much simpler flow with good results: the main agent implements numbered specs directly, with TDD for critical logic and integration tests for the rest.

## Decision

The main agent implements specs directly, with tests and the gates in `AGENTS.md`. Specs use three-digit IDs (next free: `008`) and a status banner, and live in `specs/` (active), `specs/backlog/` and `specs/done/`; the banner and `specs/README.md` are the state. TDD applies only to critical logic; the rest is validated with integration tests per feature, and the full suite and gates run at spec close. `cargo-mutants` is optional, for critical logic only. The repo agents other than `manual-writer` and the file `.odd/state.json` are removed; the earlier change folders, history, capabilities and index move unchanged to `specs/done/legacy/`. The product documents `specs/00-..07-*.md` stay as reference documents.

## Consequences

- Less ceremony per request: no flow classification, no Slice card, no outcome state, no verify agent rounds.
- Independent verification and mandatory mutation testing go away; the gates, the CI policy steps and the test-first rule for critical logic remain the controls.
- Branches stay as in ADR 0003 (`develop` integration, PR to `main` only with the owner's OK); the agent doing the work merges into `develop` after the gates pass.
- The per-crate gate scope of ADR 0017 stays; its verification-artefact part no longer applies.
- The technical manual chapters on methodology and its glossary still describe the earlier flow and need an update through `manual-writer`.

## Alternatives considered

- **Keep ODD lite and trim agents.** Rejected: most of the cost came from the routing and state rules, not from the agent count.
- **Delete the legacy specs.** Rejected: code comments, ADRs and `07-deuda-abierta.md` cite them; git history alone is harder to read.
