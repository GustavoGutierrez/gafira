## Light spec
Flow: light | pack: agent/skill definitions | mutation: off | tier: T0 | verify: V1 | decided-by: owner
Reason: owner asked to adopt `docs/methodology/odd-ligero-migracion.md` through safe, reversible adjustments; the brief settles the cross-cutting choices, the ADR records them. No Rust, no public API or format change.
Scope: phases A and B of the brief, on instructions and agent definitions only. Phase C (pilot) is delivery D3 of `layout-core`, run afterwards under the coordinator.
Non-goals: Rust code; product spec rewrites or repartition into capability specs; deleting or renaming agents, skills, ADRs, or OpenSpec history; new wrapper skills or commands; a database; deprecating the current flow (phase D, after the pilot).
Acceptance criteria:
  AC1 Given the repo, When the audit is read, Then `docs/adr/0011-odd-lite.md` holds the inventory of brief §3 and a table "component -> keep/adapt/merge/deprecate -> reason -> impact -> check". [manual]
  AC2 Given `AGENTS.md`, When diffed against `develop`, Then it gains an "ODD lite" section adapted from brief §6 and no existing safety, architecture, TDD, testing, delivery or Git rule is lost. [command]
  AC3 Given `coordinator.md`, When a request arrives, Then it classifies L0-L3 by impact, uncertainty and reversibility, each level maps to the existing modes and agents, and every hard floor, gate and owner stop still applies. [manual]
  AC4 Given new work on a capability that an existing spec covers, When it runs, Then no `openspec/changes/<change>/` folder, proposal, design or tasks file is required by default; L3 keeps design and ADR. [manual]
  AC5 Given resumable state, When work resumes, Then one small tracked `.odd/state.json` holds outcome, spec, criteria status and next action, the coordinator checks branch, diff and tests first, and `layout-core` keeps its folder until archive. [command]
  AC6 Given `verify.md`, When a critical check cannot run, Then the verdict can be BLOCKED with escalation to the owner; PASS and FAIL rules, gates and the `verification.json` schema stay compatible. [manual]
  AC7 Given the brief's `odd-spec`, `odd-run` and `odd-verify`, When the ADR is read, Then each maps to existing roles (`specifier` with `product-specs`, `coordinator` with `tdd-apply`, `verify`), every agent file still exists, and `mutants` stays mandatory on critical code by risk. [command]
  AC8 Given the ADR, When read, Then it states status `proposed` until the pilot ends, the retained flows, the rollback steps, and "not measured" where no baseline exists. [manual]
  AC9 Given the diff, Then it touches no `crates/`, `apps/`, `docs/specs/` or `openspec/changes/archive/` path, and no absolute or user path enters a tracked file. [command]
Plan: 1 audit and ADR 0011; 2 `AGENTS.md` and `CLAUDE.md`; 3 `coordinator.md` router and state; 4 `verify.md` BLOCKED; 5 `specifier.md` and `tdd-apply.md` only if the state or spec rule needs it; 6 `.odd/state.json`; 7 ADR index.
Risk and reversibility: low; instructions only, one revert restores the flow.        Owner decisions touched: none (D-nn are product decisions; ADR 0001 to 0004 amended, not removed).
Escalation triggers watched: ambiguity, public API/format, domain invariant, size, architecture
