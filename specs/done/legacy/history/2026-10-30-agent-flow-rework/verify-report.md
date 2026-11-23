# Verify report: agent-flow-rework

Round 2 (closure) | depth V1 | mode light | pack agent-definition | mutation off | tier T0
Verified commit: `df1a58ae083452e6abaa13a794540d1d4cf13189` on `chore/agent-flow-rework`, base `develop`.

Verdict: PASS (0 CRITICAL, 1 WARNING `follow-up`, 5 SUGGESTION). All round 1 findings are closed.

Skipped by instruction: cargo gates (no Rust changed), mutation (`off`), hand mutants (V1). CI policy checks: none apply; no Rust, workspace, CI or action-pin file changed. `manual.py check` exits 0 (11 chapters, English and Spanish mirror each other, links resolve). `git diff --check` is clean. No `.rs`, `Cargo*`, `.github` or `.cargo/mutants.toml` file is in the diff.

## Round 1 findings

| Finding | Status | Evidence |
|---|---|---|
| C1 | closed | AC1a. `352f325` removes both exclusions: `coordinator.md:124` "in any mode, including the light Bug fix and Small refactor packs; no critical code: no list", `AGENTS.md:80` "in any mode". `coordinator.md:80` lets the light implementer receive the list. `tdd-apply.md` has no exclusion. G3 holds in all code modes. |
| W1 | closed | AC8. `coordinator.md:125`: survivor to `tdd-apply`, T4 re-run on that crate, closure-round `verify` with the new report. `verify.md` Rounds: only the new report proves closure, its `commit` must contain the fix, a stale report leaves it CRITICAL. `mutants.md` allows the re-run and reports `commit`. G5 reads each crate's latest report. `--iterate` stays forbidden for T4. |
| W2 | closed | AC9. "T4 covers it at close" is gone from `.agents`, `AGENTS.md`, the manuals and the ADRs. `coordinator.md:141` passes every follow-up to `archive`; `archive.md` step 2c appends it to "Pending follow-ups" in `docs/specs/README.md`, a list that exists. Light runs reach `archive` (`coordinator.md:81`), so "any mode" holds. |
| W3 | closed | AC11. `docs/adr/0004-hand-mutant-split-and-verify-rounds.md` names ADR 0001 lines 82 and 153 and the ADR 0002 `archive` row; those line numbers match. No accepted ADR is in the diff; `docs/adr/README.md` gains only its index row. Both manuals state the new budget (V2 row, T1 row, "Hand mutants") and no old budget text remains outside ADR 0001 and the ADR 0004 quotes. ADR 0004 is `proposed` by design: ADR 0001 keeps its old lines until the owner accepts it, and ADR 0004 Consequences lists the status and index updates due then. |
| W4 | closed | AC10. `verify.md` Rounds keeps the gates and the integrity checks of steps 3, 3b and 3c in closure rounds; `AGENTS.md:87` and both manuals agree. |

## Acceptance criteria

| AC | Status | Evidence |
|---|---|---|
| AC1, AC1a | covered | see C1 |
| AC2 | covered | unchanged since round 1 |
| AC3 | covered | unchanged in substance. "T4 stays once at close" and the non-goal "no change to T4" are narrowed by AC8, a later amendment; the wording was updated to match and is recorded under Deviations |
| AC4, AC5, AC6 | covered | unchanged since round 1, except AC5's "covered by T4" condition, which AC9 replaces |
| AC7 | covered | no absolute or user path in added lines; agent text is ASCII English, and Spanish appears only in the manual mirror that AC11 requires; `.claude/agents` links to `../.agents/agents` (9 definitions) |
| AC8 | covered | see W1 |
| AC9 | covered | see W2 |
| AC10 | covered | see W4 |
| AC11 | covered | see W3 |

## Integrity (steps 3, 3b)

- `b91a7aa` (`light spec`) touches only `apply-progress.md`, 16 insertions.
- `db18553` (`light spec amend`) touches only `apply-progress.md`, 7 insertions, no deletions, and its message states why.
- Lines 1 to 23 at HEAD are identical to `db18553`. The only later commit on that file, `bd331b3`, appends from line 45.
- 14 conventional commits, no AI attribution. Size: 12 files, +235 -41.

## Findings

### CRITICAL

None.

### WARNING

- **W5** class: ambiguity, `follow-up`. ref: `.agents/agents/coordinator.md:79`, `.agents/agents/verify.md:30`. New, pre-existing on `develop`. `coordinator.md:79` tells the coordinator not to "amend the Light spec after implementation starts", while `verify.md` step 3b allows a `light spec amend` commit that states why, and `coordinator.md:97` allows the Slice card equivalent. This change used the amend path on the owner's decision. The rule stays usable, since `verify` accepts the commit. Start: align line 79 with line 97.

### SUGGESTION

- **S1** (open from round 1, more relevant now) `verification.json` findings have no `follow-up` field, yet the coordinator routes from that file and must pass follow-ups to `archive`. This report uses a `follow-up:` summary prefix.
- **S2** (open from round 1) The schema has one top-level `delivery`, while round lookup is per delivery. The T4 cycle is now counted, but the plain change-level final verify is still not stated against the cap.
- **S3** (open from round 1) `CARGO_INCREMENTAL=0` removal in `mutants.md` and the leftover `touch` line. Compare disk and wall time at the next T4 report.
- **S4** `apply-progress.md` says AC11 is "not in this work" and does not list `1095563` or `df1a58a`. Add a line so the record is complete.
- **S5** G5 applies to standard and full only, so its "latest report" clause does not gate the light Bug fix and Small refactor packs, which run T4. `coordinator.md:125` still covers them in any mode, so AC8 holds.

---

## Round 1 (historical)

Round 1 | depth V1 | mode light | pack agent-definition | mutation off | tier T0
Verified commit: `fbfcca110228360cd984dfb70b4e69e58c1405d3` on `chore/agent-flow-rework`, base `develop`.

Verdict: FAIL (1 CRITICAL, 4 WARNING, 3 SUGGESTION).

Skipped by instruction: cargo gates (no Rust changed), mutation (`off`), hand mutants (V1). CI policy checks were not run: they cover Rust, workspace and action pins, and no such file changed.

## Acceptance criteria

| AC | Status | Evidence |
|---|---|---|
| AC1 | failing (partial) | `coordinator.md:124` requires the rule-level list and `tdd-apply.md` step 2b runs it, records kills and blocks `done`, but the list is excluded in light mode; see C1 |
| AC2 | covered | `verify.md` step 4b: no repeat, one spot-check, 2 to 3 independent |
| AC3 | covered | T4 text unchanged; `AGENTS.md:80` agrees with `verify.md`; remaining `verify only` text (`AGENTS.md:81`, `mutants.md:14`) is about T1, which is non-critical and has no list |
| AC4 | covered | `coordinator.md` route step 0 (Order), `AGENTS.md:63`, `specifier.md` step 4b lists NaN, infinities, overflow, thresholds, ties, boundary operators |
| AC5 | covered | `verify.md` "Rounds" paragraph, steps 1c and 4b; `coordinator.md:125` and the `follow-up` bullet; loop limits unchanged |
| AC6 | covered | `grep -n CARGO_TARGET_DIR` hits `mutants.md:18,33` and `coordinator.md:125`, all unset or never passed; no text tells the agent to set one |
| AC7 | covered | no absolute or user path in added lines or changed files; added lines are ASCII English; `.claude/agents` is a mode 120000 link to `../.agents/agents` and lists the 9 definitions |

## Light spec and commits (step 3b)

- `b91a7aa` touches only `openspec/changes/agent-flow-rework/apply-progress.md` (16 insertions).
- The first 16 lines at HEAD are identical to `b91a7aa`; only `fbfcca1` touched the file later, and it only appends.
- Six conventional commits, no AI attribution in any message.
- Size: 7 files, +65 -19, as recorded.

## Internal consistency

| Topic | Result |
|---|---|
| Hand-mutant budget | agent definitions and `AGENTS.md` agree on 2 to 3 independent plus one spot-check; the light-mode exclusion is consistent across them but breaks AC1 (C1) |
| Round rule | `verify.md`, `coordinator.md`, `AGENTS.md:87` agree; caps unchanged |
| T4 | agree; gaps in W1 and W2 |
| `CARGO_TARGET_DIR` | agree; `verify.md:20` still sets one, but for its own scratch exports, not for `mutants`, so no conflict |

## Findings

### CRITICAL

- **C1** class: implementation. ref: `coordinator.md:124`, `AGENTS.md:80`, AC1. AC1 says "Given critical code in a delivery" with no mode limit. The implementation adds "Light mode or no critical code: no list" and "outside light mode". Light mode can hold critical code: the Bug fix and Small refactor packs are light, V2, with `tdd-apply`. The narrowing is mentioned under AC1 in `apply-progress.md` but is not listed under Deviations. Fix: remove the exclusion, or, if it is wanted, the coordinator records it in a `light spec amend` commit that states why.

### WARNING

- **W1** class: ambiguity. ref: `verify.md` Rounds and steps 1b, 4, 4b; `coordinator.md:125`. Reported gap (a), confirmed. After a T4 fix, the coordinator passes "the T4 report", which still lists the survivor. Read literally that is CRITICAL at step 4; verify may not run mutation and step 4b is round 1 only. Nothing says who re-runs T4 or how closure is proved. In anim-core the implementer re-ran T4 outside any written rule. The route text predates this change, but this change removed the hand-mutant way out. Fix: state a T4 re-run, or a targeted `mutants` re-run, before the final verify.
- **W2** class: ambiguity. ref: `verify.md` Rounds, AC5. Reported gap (b), confirmed. "T4 covers it at close" is false with `mutation: off`, and also at the change-level final verify, when T4 has already run. The only sink is one line in the coordinator's report to the owner, with no tracked item. The wording comes from AC5, so the root is the spec, not the implementer. Fix: limit the claim to changes with a T4 still to run, and otherwise require a tracked follow-up.
- **W3** class: architecture. ref: `docs/adr/0001-flow-modes-and-packs.md:82,153`, `docs/manuals/tecnico/chapters/10-methodology.md:169,185`, `docs/manuals/tecnico/chapters-es/10-*.md:169,185`. Reported gap (c), confirmed with changes. ADR lines 82 and 153 and both manual lines still give the old budget (at most 5, 3 in light; T1 3 to 5), and the Spanish mirror has the same drift, which the gap report did not name. ADR line 258 is not a contradiction: it rejects a `cargo mutants --in-diff` self-run, which stays not adopted; it is only context, since it says to revisit. `docs/adr/README.md:17` forbids editing an accepted ADR, so the fix is a new ADR that partly supersedes 0001 (owner confirms it), plus a `manual-writer` pass in both languages.
- **W4** class: ambiguity. ref: `verify.md` Rounds and steps 3, 3b, 3c; `AGENTS.md:27`. Found by this verify. Closure rounds run only the gates and the closure of earlier findings, so steps 3, 3b and 3c do not run on fix commits. An implementer commit that changes the Light spec or Slice card in a fix round goes unchecked, although `AGENTS.md:27` says verify checks this. This follows AC5's "closure of prior findings only". Fix: keep the spec-integrity check in every round.

### SUGGESTION

- **S1** `verify.md` `verification.json` schema: findings have no `follow-up` field, yet the coordinator routes from that file and must skip `follow-up` WARNINGs. Add the field, or fix a summary prefix.
- **S2** `verify.md` Inputs: a missing round is taken from `verification.json` "for this delivery", but the schema has a single top-level `delivery`. anim-core added a per-round `delivery` by hand. Add it to the schema. Also state whether the change-level final verify counts toward the cap of three rounds.
- **S3** `mutants.md`: `CARGO_INCREMENTAL=0` was dropped on the reasoning that it only made sense with a set target dir, but the variable is independent of the target dir. This is not a contradiction, but `strings` on the installed `cargo-mutants` finds no such name, so it likely does not set it and per-job builds may now be incremental (not measured). Compare disk and wall time at the next T4 report. The `touch` line in `mutants.md:19` also looks left over, since the agent describes no export.

## Reported gaps

| Gap | Judgment | Reason |
|---|---|---|
| a | WARNING (W1) | No stated way to prove closure, and a literal reading fails on a stale report |
| b | WARNING (W2) | The claim is false with `off` and after T4, and the only backstop is an untracked line to the owner |
| c | WARNING (W3) | Lines 82, 153 and the manual lines disagree, and the ADR rule needs a superseding ADR; line 258 alone is not a contradiction |

Round 1 routing: specifier (ambiguity: W1, W2, W4), then architect (architecture: W3), then tdd-apply (implementation: C1)

next: archive (pass follow-up W5 to the `archive` step)
