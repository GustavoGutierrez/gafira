## Light spec
Flow: light | pack: agent/skill definitions | tier: T0 | verify: V1 | decided-by: owner | reason: owner asked to speed up the flow (mutation toggle, spec-direct execution, no forced full mode)
Reason: methodology-only change; no Rust source touched; floors checked: no crate, dependency edge, V5 row or format change.
Scope: replace the ADR 0001 default route with a fast lane: run product specs directly, toggle mutation, lighter verify and archive, faster cargo-mutants.
Non-goals: changing strict TDD, the per-commit gates, the PR/merge rules or owner decisions; rewriting archived changes or openspec/specs history.
Acceptance criteria (type in brackets):
  AC1 Given the Flow line, When the coordinator records it, Then it carries `mutation: off|archive|delivery`, default `archive` (one T4 run per touched crate at change close), and `mutants` runs nothing when `off`. [manual]
  AC2 Given `cargo mutants`, When it runs in this repo, Then it uses a `mutants` profile with `debug = "none"` set in `.cargo/mutants.toml` and `--jobs 8`; `cargo mutants --list -p gafira-geom` succeeds. [command]
  AC3 Given a clear product spec section, When the owner asks to execute it, Then `tdd-apply` works from the product requirement IDs (ARQ-, RF-...) in `docs/specs/` via a Slice card (IDs, scope, non-goals, plan) in `apply-progress.md` that satisfies G0; no proposal or delta spec is written; `specifier` runs only on a real ambiguity. [manual]
  AC4 Given a new crate, When the coordinator classifies, Then it no longer forces full mode; `architect` runs only when the spec leaves a design decision open or the slice needs ordered deliveries. [manual]
  AC5 Given verify, When no depth is given, Then the default is V1 for docs/tooling and V2 for code; V3 runs only on owner request. [manual]
  AC6 Given a fast-lane change at close, When it is archived, Then archive moves the change folder and updates the `docs/specs/README.md` status table with the covered IDs, with no delta merge; tests carry the requirement IDs so `git grep ARQ-nn` traces coverage. [manual]
  AC7 Given the decision, When it is recorded, Then a new ADR 0002 supersedes the affected parts of ADR 0001 (marked accordingly) and the technical manual chapters on methodology and tooling match. [manual]
Plan: 1 ADR 0002 + ADR index; 2 AGENTS.md; 3 coordinator, mutants, tdd-apply, verify, specifier, archive agents; 4 product-specs skill; 5 Cargo.toml profile + .cargo/mutants.toml; 6 manual chapters EN/ES.
Risk and reversibility: docs/config only, reverted by one revert of the PR. Owner decisions touched: none (00-decisiones.md not affected).
Escalation triggers watched: ambiguity, size.
