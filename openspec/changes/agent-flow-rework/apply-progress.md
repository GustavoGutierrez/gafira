## Light spec
Flow: light | pack: agent-definition | mutation: off | tier: T0 | verify: V1 | decided-by: coordinator
Reason: agent-definition pack, owner-approved process change from anim-core; no code, no V5 row, no public API or format; floors checked and not hit.
Scope: reduce rework and bound verify loops in `.agents/agents/` and `AGENTS.md`, per the owner's anim-core retrospective.
Non-goals: no Rust, CI or `.cargo/mutants.toml` change; no new agent; no change to T4, gates, delivery size or the G0-G5 table; no ADR edits; `docs/specs/00-decisiones.md` untouched.
Acceptance criteria (type = manual: read the named file; command: grep):
  AC1 Given critical code in a delivery, When the coordinator delegates to `tdd-apply`, Then `coordinator.md` requires it to pass a rule-level hand-mutant list (spec rules, no code), and `tdd-apply.md` requires running it before `done` and recording each kill in `apply-progress.md`, with a survivor blocking `done`. [manual]
  AC2 Given that record, When `verify` runs, Then `verify.md` says it does not repeat the list, spot-checks one item and adds 2 to 3 independent hand mutants. [manual]
  AC3 Given any mode, Then cargo-mutants T4 stays once at close as today; `AGENTS.md` and `verify.md` agree with AC1 and AC2 and no text still says verify alone owns all hand mutants. [manual]
  AC4 Given a numeric crate (floats, tolerances, integer overflow), When upstream stages are needed, Then `coordinator.md` routes `explorer` before `specifier`, and `specifier.md` has a numeric-edge checklist (NaN, inf, overflow, thresholds, ties, boundary operators). [manual]
  AC5 Given a verify round on one delivery, Then `verify.md` says the independent-mutant budget is spent in round 1 only; rounds 2+ check closure of prior findings only; a NEW test gap after round 1 is a non-blocking WARNING (follow-up, covered by T4); a probe that shows a real code defect still blocks. `coordinator.md` routes accordingly and its loop limits stay. [manual]
  AC6 Given the `mutants` agent, Then `mutants.md` and `coordinator.md` state the coordinator never passes `CARGO_TARGET_DIR` to it, and the agent leaves it unset (cargo-mutants' per-job dirs), replacing any text that tells it to set one. [command: grep -n CARGO_TARGET_DIR .agents/agents/mutants.md .agents/agents/coordinator.md]
  AC7 Then no absolute or user paths appear in the diff, artifacts stay in English, and `.claude/agents/` still resolves to the same definitions. [command]
Plan: 1 `tdd-apply.md`; 2 `verify.md`; 3 `specifier.md`; 4 `mutants.md`; 5 `coordinator.md`; 6 `AGENTS.md`; 7 record measured lines under this block. Small, one commit per concern, conventional commits, no AI attribution.
Risk and reversibility: low, text only, revert by commit.        Owner decisions touched: none (process change approved by the owner on 2026-10-29).
Escalation triggers watched: ambiguity, public API/format, domain invariant, size, architecture
