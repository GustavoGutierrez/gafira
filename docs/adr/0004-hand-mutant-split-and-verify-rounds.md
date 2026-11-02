# 0004. Hand-mutant split and bounded verify rounds

- Status: accepted, amended by 0011
- Date: 2026-10-30 (proposed and accepted the same day)
- Related: ADR 0001 (supersedes the lines named below), ADR 0002 (extends section 1), `.agents/agents/`, `AGENTS.md`, changes `anim-core`, `agent-flow-rework`

## Context

In `anim-core`, D3b took four verify rounds: each of the three FAILs was a hand-mutant survivor or missing test on a numeric rule (C1 to C4 in the archived `verify-report.md`), found by `verify` after the implementer had returned. Its first FAIL also carried a spec gap ("determinant 0" undefined in floats). Later, the final verify after the T4 fix found a real defect (C1, ARQ-17 precision in `easing.rs`) that T4 had not shown (`critical_missed: 0`). Two needs follow: find test gaps earlier without losing independence, and bound verify rounds without letting a real defect through.

## Decision

Split hand mutants between `tdd-apply` and `verify`, spend the independent budget once per delivery, prove T4 survivor closure with a new report, and settle numeric edges before the spec.

### 1. Hand-mutant split

- On critical code, in any mode, the coordinator gives `tdd-apply` a rule-level list (spec rule and wrong behaviour, no code). `tdd-apply` applies each item, confirms a test fails, reverts and records item and killing test in `apply-progress.md`. `done` needs every item killed.
- `verify` at V2 and V3 does not repeat the list. It spot-checks one item and adds 2 to 3 independent hand mutants per delivery, in any mode. With no recorded list, the 2 to 3 still apply; tier T1 relies on them alone.
- Neither replaces T4.

### 2. Verify rounds

- Round 1 is the only round with independent probes and hand mutants.
- Rounds 2+ are closure rounds: gates, artifact-integrity checks and closure of earlier findings only. A verify after a T4 fix round, or the change-level final verify, is a closure round unless the coordinator says it is the first verify of that code.
- A new test gap after round 1 is a non-blocking `follow-up` WARNING, in any mode and whatever the `mutation` value. `archive` records it in "Pending follow-ups" of `docs/specs/README.md`.
- A real code defect, a gate failure or a critical survivor in the newest report still blocks, in any round.

### 3. T4 survivor closure

A critical T4 survivor goes to `tdd-apply`, then T4 re-runs on that crate, then a closure-round `verify` reads the new report. Only that report proves closure; a stale one leaves the survivor CRITICAL. Each cycle counts as one verify round of the change's last delivery, so the existing loop limits apply.

### 4. Numeric crates

For a numeric crate (floats, tolerances, integer overflow), when upstream stages are needed, `explorer` runs before `specifier` and hands it `spike.md`. `specifier` checks each requirement against NaN, infinities, overflow, thresholds, ties and boundary operators.

### Hanging mutants (owner decision, 2026-11-01)

A timeout mutant in critical code was always CRITICAL, yet some mutants cannot end a run any other way. It now counts as detected only when a re-run with a longer limit confirms it does not terminate (or iterates a range no test run could finish) and the report lists it by file and line with that evidence. Any other timeout, or one load explains, stays a finding. Where a test can fail on an assertion first, it must be reordered or split so it does, so a hang is a last resort. `mutants` reports the evidence and never reclassifies such a timeout as non-critical on its own. Missed mutants and the `verification.json` schema do not change.

### What this supersedes

- ADR 0001 line 82 (depth V2): the cap "at most 5; at most 3 in light mode". Replaced by section 1. The rest of the V2 row stays.
- ADR 0001 line 153 (tier T1): the budget "at most 3 to 5". Replaced by 2 to 3 independent hand mutants.
- ADR 0002 section 1, row `archive`, gains the T4 re-run of section 3 before the final `verify`; the row is otherwise unchanged.

### What stands

ADR 0001 line 258, which did not adopt a `tdd-apply` self-run of `cargo mutants --in-diff`, stands. Rule-level hand mutants are a different thing: the items come from the coordinator, are rule-level rather than a tool run, and `cargo-mutants` stays outside the author's hands. `verify`'s independent mutants keep independence.

## Consequences

Easier:
- Test gaps on critical rules surface before the first verify, so fewer rounds go to them.
- A delivery's rounds converge: later rounds cannot open new test-gap FAILs.
- Follow-ups outlive the change in one list.

Harder and risks:
- `tdd-apply` does more work per delivery on critical code, and the coordinator must write the list.
- Closure rounds do not look for new defects. Independent checks on fix code depend on the coordinator marking it as the first verify of that code, as the `anim-core` defect shows.
- A T4 fix cycle costs a whole-crate run per cycle.
- On acceptance, ADR 0001's status and index entry gain the supersession by 0004, and ADR 0002's entry the extension.

## Alternatives considered

- Keep `verify` as sole owner of hand mutants. Rejected: it is what produced the D3b rounds.
- Exclude light mode from the list. Rejected: critical code carries the same risk in any mode.
- Keep independent mutants in every round. Rejected: each round can open new FAILs, so rounds do not converge.
- Make a follow-up non-blocking only when T4 covers it. Rejected: with `mutation: off` no T4 runs, and the gap would have no record.
