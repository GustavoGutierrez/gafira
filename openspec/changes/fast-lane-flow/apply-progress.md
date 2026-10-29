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

## Apply progress

Done (commits `ec7df07` docs(adr), `06ea83c` build, `df38377` docs(agents)):
- AC1: Flow line `Flow: <mode> | pack | mutation: off|archive|delivery | tier | verify | decided-by | reason`, default `archive` (one T4 per touched crate at change close); `mutants` runs nothing with `off`; "skip mutation" = `off`. Files: `AGENTS.md`, `coordinator.md`, `mutants.md`, `verify.md`, ADR 0002 section 1.
- AC2: `Cargo.toml` `[profile.mutants]` (`inherits = "test"`, `debug = "none"`); `.cargo/mutants.toml` `profile = "mutants"`; `--jobs 8` in `AGENTS.md` and `mutants.md`; fast linker noted as optional, not installed.
- AC3: Slice card (product IDs, scope, non-goals, deliveries, plan, Flow line) in `coordinator.md`, committed `docs(<change>): slice card`, satisfies G0; `tdd-apply` and `verify` read the card plus product spec rows by ID; tests name the ID; `specifier` only on `blocked`/`ambiguity`, fixing the product spec. `product-specs` section 4 makes this the default derivation.
- AC4: new crate described in `02` is standard; full only for a new V5 row, dependency edge not in `02`, format version, open cross-cutting choice or parallel deliveries; serial deliveries in the Slice card need no `architect` (`AGENTS.md`, `coordinator.md`, `architect.md`).
- AC5: verify defaults V1 docs/tooling, V2 code, V3 on owner request (`AGENTS.md`, pack table in `coordinator.md`, `verify.md` inputs).
- AC6: `archive.md` step 2b updates the `docs/specs/README.md` "Detalle" cell with covered IDs, no delta merge, `openspec/specs/` untouched; test-ID tagging in `tdd-apply.md`, `AGENTS.md`, verify step 3c.
- AC7 (ADR part): ADR 0002 accepted and indexed; ADR 0001 status "accepted, partly superseded by 0002" with a note naming the superseded parts.

Command evidence:
- `cargo mutants --list -p gafira-geom | head`: exit 0, lists mutants (config with `profile` parses).
- `cargo build --profile mutants -p gafira-geom`: exit 0, "Finished `mutants` profile [unoptimized] target(s) in 6.21s".
- `cargo fmt --check`: exit 0.
- `git grep "jobs 4"`: only ADR 0001 (historical measurements, accepted ADR not edited) and ADR 0002 context.

Measured lines: `git diff --shortstat main...HEAD` = 17 files changed, 266 insertions(+), 99 deletions(-) (before this section; includes the Light spec commit).

Deviations:
- AC7 manual chapters (`docs/manuals/tecnico/`) not touched here: out of this writer's scope, done by another agent.
- ADR 0001 got a status suffix plus a short note at the top (owner asked for the note); its body is unchanged.
- `README.md` workflow line and `mutants` bullet updated (they stated per-delivery mutants); `CLAUDE.md` unchanged (states no old default).
- Light mode code packs (bug fix, refactor) now default to `mutation: archive` (T4 at close), replacing per-delivery T2/T3; this matches the owner decision in ADR 0001 that T4 runs at close in every mode that changed Rust.
