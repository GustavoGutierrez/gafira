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
Flow-change: light stays light (trigger: verify round 1 ambiguity findings W1, W2, W4 would raise to standard; decided-by: owner on 2026-10-30, resolved below; risk accepted: no `specifier` pass, the owner reviews at merge)
Amendment (verify round 1 FAIL; why: AC1 was narrowed by the coordinator's brief and the round rules left gaps the owner has now settled):
  AC1a The hand-mutant list applies to all critical code in any mode; no light-mode exclusion in `coordinator.md`, `tdd-apply.md` or `AGENTS.md`. [manual]
  AC8 Given a critical T4 survivor fixed after T4, Then the coordinator re-runs T4 on that crate and gives `verify` the new report; closure of a survivor is proved only by that report. [manual]
  AC9 Given a follow-up WARNING in any mode or mutation value, Then it is recorded in the follow-up list of `docs/specs/README.md` when the change closes; "covered by T4" is no longer a condition for it being non-blocking. [manual]
  AC10 Given a closure round, Then `verify` still runs the gates and the artifact-integrity checks (Light spec or Slice card commit touches only that file; no implementer commit changed it). [manual]
  AC11 Given ADR 0001's superseded V2 and T1 hand-mutant budget, Then a new ADR 0004 (status `proposed` until the owner confirms) supersedes those lines, without editing accepted ADRs, and `docs/manuals/tecnico/` (English and Spanish) states the new budget. [manual]

## Progress
Commits:
- 5986c92 docs(agents): split hand mutants between tdd-apply and verify
- e137f8f docs(agents): limit verify rounds after the first to closure
- ffc2e08 docs(agents): route numeric crates through explorer before specifier
- 7588202 docs(agents): keep CARGO_TARGET_DIR unset for cargo-mutants
- docs(agent-flow-rework): record progress (this section)

Measured lines (`git diff --shortstat develop...HEAD`): 7 files changed, 65 insertions(+), 19 deletions(-)

How each AC is met:
- AC1: `coordinator.md` route step 2 requires the rule-level list (no code) and G3 requires it killed and recorded; `tdd-apply.md` step 2b runs it, records item and killing test, and a survivor blocks `done`. Light mode or no critical code: no list.
- AC2: `verify.md` step 4b does not repeat the list, spot-checks one item and adds 2 to 3 independent hand mutants; with no record the 2 to 3 still apply. The `verification.json` shape is unchanged; only the example values moved to fit the new budget.
- AC3: T4 text is untouched. The `AGENTS.md` hand-mutant bullet agrees with AC1 and AC2. `T1 hand mutants by verify only` stays because T1 is non-critical code, which has no list.
- AC4: `coordinator.md` route step 0 (Order) runs `explorer` before `specifier` for numeric crates, also stated in `AGENTS.md`; `specifier.md` step 4b is the numeric-edge checklist.
- AC5: the `verify.md` "Rounds" paragraph holds the rule; `coordinator.md` passes the round number and does not route `follow-up` WARNINGs. Loop limits are unchanged.
- AC6: `grep -n CARGO_TARGET_DIR` hits `mutants.md` lines 18 and 33 and `coordinator.md` line 125; all three say unset or never passed.
- AC7: no absolute or user path in the added lines (grep); added text in `.agents` and `AGENTS.md` is English and ASCII; `.claude/agents` still links to `../.agents/agents`.

Deviations:
- `mutants.md` lost its `CARGO_INCREMENTAL=0` guidance and its `cargo clean --target-dir` line, which only made sense with a set target dir. cargo-mutants 27.1.0 deletes its scratch dirs unless `--leak-dirs` is passed (`--help`).
- `verify.md` step 4 split into 4 and 4b, because hand mutants do not depend on a `mutants` run.
