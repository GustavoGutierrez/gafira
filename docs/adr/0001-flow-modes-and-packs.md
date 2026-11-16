# 0001. Flow modes, request packs and graded mutation testing

> Partly superseded by [ADR 0002](0002-fast-lane.md) (2026-10-29) on: the new-crate full floor, the default tiers per delivery (now `mutation: archive`), the default verify depths (now V1/V2, V3 on owner request), and delta specs as mandatory for product-spec slices (now a Slice card). Modes, packs, tiers, depths and gates below stay valid.

- Status: superseded by 0018
- Date: 2026-10-25 (proposed and accepted the same day)
- Related: `AGENTS.md` (lite flow, testing and mutation policy), `.agents/agents/coordinator.md` (route, feedback loop), owner decisions in `docs/specs/00-decisiones.md` (none reopened), changes `workspace-scaffold`, `domain-color`, `domain-doc`, `edit-history`

## Context

Today the coordinator runs one flow for every request: spec, optional `specifier` and `architect`, `tdd-apply`, `mutants` + `verify` per delivery, whole-crate mutation and `archive`. The owner asks that the coordinator choose the flow per request so it does not launch every agent and every check each time, that mutation testing be graded (a step between the diff-scoped run and the whole-crate run), and that typical request types have a predefined "pack" of agents and checks.

Owner rules this ADR must keep (all are already true or become true here):
- `specifier` does not implement and produces testable requirements; `architect` delivers design and tasks with explicit dependencies; `tdd-apply` works RED, GREEN, REFACTOR; `verify` produces independent checks from the acceptance criteria and does not trust only the implementer's tests; the coordinator applies gates between phases, limits retries and asks for a human when needed.
- Agents exchange structured contracts and persistent artifacts so a run can resume, decisions can be audited and no agent receives another's full history.
- Practical minimum for small tasks: three roles (Orchestrator, Implementer, Verifier), provided the specification and acceptance criteria are documented BEFORE implementing.

What the repository history shows (reports read: `edit-history` D1 to D8, `domain-doc` D1 to D6b, `domain-color`, `workspace-scaffold`; counts come from verdict headings and CRITICAL lists, not an exhaustive audit):

| Observation | Evidence | Implication |
|---|---|---|
| About half of first verdicts on code with invariants were FAIL: `edit-history` D2, D3, D6, D7, D8 (5 of 8); `domain-doc` D2a, D2b, D3a, D4a, D4b (5 of 12) | verify reports | The check that earns its cost on domain code is test adequacy, not more gates |
| Every FAIL I found was class `implementation` (test gap): untested accepting side of law 5 (D2), untested "every target" rule (D3), unpinned closed-gesture rule (D6), 14 missed mutants in the size model (D7), scenario without a test (D8), swapped composition order (domain-doc D2a), `dpi()` never read back (D2b), an exclusion that hid a live mutant (`domain-color` PR2) | same | No architecture or ambiguity FAIL occurred in four changes. Upstream stages did not cause rework; they prevented it, or were not needed |
| Surviving cargo-mutants and hand mutants were the origin of most CRITICAL items | same | Keep mutation for code; drop it only where there is no code |
| Gates (fmt, clippy, tests, deny, CI policy) passed on every commit in the reports read; `tdd-apply` already runs them per commit | same | Re-running every gate on every commit in clean exports is only worth it for the heaviest class |
| Warnings about spec lag ("pending spec edits") appear in almost every `edit-history` delivery (D1 W6, D2 W1, D4, D6, D8 W4) | same | Specs drift when the implementer deviates; a delta spec is needed whenever a contract outlives the change |
| Size: forecasts undershot x1.12 to x1.9; D8 landed at 1037 lines against the ~700 limit | `apply-progress.md`, verify D8 | Size triggers must use the measured factor |
| Strict TDD was broken and declared twice (D3, `domain-doc` D2a); caught only post hoc | verify W1 | Require RED evidence per acceptance criterion in the hand-off |
| Reports are 111 to 1910 lines each; the coordinator and the next agent read them | `wc -l` | A small machine-readable result saves context |
| Measured mutation cost (cargo-mutants 27.1.0, `--jobs 4`, this machine): diff runs 13 to 47 mutants in 36 s to about 2 min (about 2.5 s per mutant in `gafira-doc`); whole `gafira-geom` 53 mutants in 29 s (baseline 8 s build); whole-crate mutant counts from `--list`: geom 53, color 175, doc 149, edit 182, testkit 143 | verify reports, local runs | Cost scales with mutant count; whole `gafira-edit` (182 mutants, the largest) took 4 min 54 s, so a whole-crate run is minutes, not hours; no earlier report recorded one |

## Decision drivers

- Spend agent runs and tokens in proportion to risk and reversibility, not size alone.
- Never weaken the checks that found real defects (mutation, requirement-to-test mapping) on code with invariants.
- Documented acceptance criteria before implementation in every mode; this is what independent verification is measured against.
- Keep the coordinator in control without making it a writer of code (`coordinator.md`: "you coordinate; you do not write code or specs").
- Simple to apply: a table lookup plus a short list of escalation triggers, not a scoring model.
- Resume and audit from files and git only (`AGENTS.md`: state lives in the change folder and git).

## Decision

The coordinator classifies each request into a pack (table below), applies the hard floors and escalation triggers, records the result in a `Flow:` line, and runs only the agents and checks of that mode. There are three modes (light, standard, full), five mutation tiers (T0 to T4) and four verify depths (V0 to V3). The owner can override any choice. The gate "acceptance criteria documented before implementing" (G0) applies to every mode.

### 1. Selection criteria

Step 1, floors (the first matching row sets the minimum mode; higher always wins):

| Signal the coordinator evaluates | Minimum mode |
|---|---|
| The criteria cannot be written without an owner decision (`docs/specs/00-decisiones.md` silent, license, pricing, scope) | stop and ask the owner; no mode runs |
| New crate, new dependency edge, V5 layer-table row, new serialized format version, cross-cutting design choice, or more than one delivery with ordering | full |
| Public API or serialized-format change in an existing crate; a new rule on critical code (validation, JSON load/save, ids and arena, transforms, size/DPI limits, any spec rule with an invariant); request ambiguous enough that criteria would invent behavior | standard |
| Forecast above ~300 changed lines (apply the x1.4 measured undershoot: forecast x1.4 above ~400 counts) | standard |
| Forecast above ~700 lines (x1.4 applied) or parallel deliveries wanted | full |
| None of the above | the pack default |

Step 2, pack default (table in section 3). Step 3, adjust: reversibility (anything the owner can drop by reverting one commit stays at its default; irreversible effects such as publishing, tags or licensing raise approval points, not mode), dependencies and parallelism (only `architect`'s table may mark deliveries parallel), and the owner's explicit instruction, which beats everything (section 7).

The coordinator never lowers a mode below a floor on its own. The pack default can be lowered only by the owner.

### 2. Modes

Roles: Orchestrator is the coordinator. Implementer is `tdd-apply` (or `manual-writer` for the manual). Verifier is `verify` with `mutants` running in parallel when the tier needs it.

| | Light | Standard | Full |
|---|---|---|---|
| Agent order | Orchestrator, then `tdd-apply`, then `verify` | Orchestrator, `specifier`, (`explorer` per risky claim), `tdd-apply`, `mutants` + `verify`, `archive` | Orchestrator, `specifier`, (`explorer`), `architect`, (`explorer`), per delivery `tdd-apply`, `mutants` + `verify`, whole-crate `mutants`, `archive` |
| Who writes the spec | the Orchestrator: a "Light spec" block (section 4) | `specifier`: delta spec under `openspec/changes/<change>/specs/` | `specifier`, plus `design.md` with delivery table by `architect` |
| Artifacts | `apply-progress.md` (Light spec block, progress), `verify-report.md`, `verification.json` | delta specs, `apply-progress.md`, `verify-report.md`, `verification.json` | delta specs, `design.md` (decisions, delivery table; `tasks.md` only if the design lacks the table), ADRs if cross-cutting, `apply-progress.md`, `verify-report.md`, `verification.json` per delivery |
| Gates | G0, G3, G4 | G0, G1, G3, G4, G5 | G0, G1, G2, G3, G4, G5 |
| Default mutation tier | T1 or T2 per pack | T2 or T3 per pack | T2 per delivery, T4 at archive (T3 for deliveries that edit existing critical files) |
| Default verify depth | V1 or V2 per pack | V2 | V3 for the first delivery of a new crate or capability, V2 for later deliveries |
| Typical human points | open decisions only; PR merge | open questions, PR merge | open questions, ADR acceptance, delivery cut, PR merges |

Verify depth:

| Depth | What `verify` does |
|---|---|
| V0 | No agent. The coordinator runs the deterministic checks of the pack (links, `manual.py check`, `cargo fmt --check`, `git diff --stat`) and reports them verbatim. It edits nothing |
| V1 | Gates at HEAD (fmt, clippy, test, deny and CI policy steps that apply); map each acceptance criterion to a test or a command; PASS or FAIL |
| V2 | V1, plus independent checks: derive probe checks from the acceptance criteria BEFORE reading the implementer's tests; hand-mutate (at most 5; at most 3 in light mode) the rules cargo-mutants cannot express; run probes and hand mutants only on a scratch export; classify findings by `class:` |
| V3 | V2, plus per-commit gates on clean exports, negative runs for CI policy, and the full requirement and scenario matrix (today's depth) |

Verify never edits source, never trusts the implementer's summary, and checks the Light spec or delta spec was not changed by the implementer after the fact (section 4).

### 3. Packs

Mutation tiers T0 to T4 are defined in section 5. "Approval" lists points where the coordinator stops; PR merges always stay with the owner.

| Request type | Mode | Agents | Tier | Verify | Human approval points |
|---|---|---|---|---|---|
| Docs only (README, comments, non-manual docs) | light | Orchestrator, one writer | T0 | V0 | none beyond PR merge |
| Manual change (`docs/manuals/tecnico/`) | light | Orchestrator, `manual-writer` | T0 | V0 (`manual.py check` and `build` evidence) | PR merge |
| Skill or agent-definition change (`.agents/`, `AGENTS.md`, skills) | light | Orchestrator, one writer, then `verify` | T0 | V1 (structure, front matter, symlinks, no absolute paths, consistency with `AGENTS.md`) | owner reviews the PR because it changes the process itself |
| Dependency or toolchain bump | light | Orchestrator, `tdd-apply`, `verify` | T0 (T2 on the fallout diff if code changed) | V1 (deny, licenses, CI policy; V2 if code changed) | new license, major toolchain change |
| Bug fix | light if the specs already state the behavior and the cause is local; standard if the spec is silent or the fix changes a contract | Orchestrator, `tdd-apply`, `verify` (+ `specifier` in standard) | T2 (T3 when the fixed file holds critical code) | V2 (the reproducing test is RED first) | spec gap, behavior change |
| Small refactor, no behavior change | light | Orchestrator, `tdd-apply`, `verify` | T2 (T3 on a critical file, because moved lines make in-diff cover most of it anyway) | V2 plus "public item diff is empty" | none |
| New function in an existing domain crate | standard | Orchestrator, `specifier`, `tdd-apply`, `mutants` + `verify` | T3 | V2 | open questions |
| New capability or crate | full | all stages | T2 per delivery, T4 at archive | V3 then V2 | questions, ADRs, delivery cut, each PR |
| Spec-only or product-spec work (`docs/specs/`) | standard without code | Orchestrator, `specifier` (skill `product-specs`), `explorer` for risky claims | T0 | V0 (structure, requirement IDs, status table) | owner approves the spec |
| Release or versioning | light | Orchestrator, one writer (skill `git-workflow-and-versioning`), `verify` | T0 | V1 (changelog, version, gates, deny) | tag and publish are the owner's |
| Investigation or spike | light, no implementer | Orchestrator, `explorer` | T0 | none; the output is `spike.md` | the owner decides what follows |
| Hotfix on CI or tooling | light | Orchestrator, `tdd-apply`, `verify` | T0 | V1 (replay the failing CI step locally, RED first) | none; escalate if a V5 row or action pin changes (floor: full) |
| Test-only hardening or fix round after a FAIL | inherits the mode of the change | `tdd-apply`, `verify` | same tier, `--iterate` allowed (section 5) | V2 on the findings only | per the two-FAIL rule |

A request that matches no row, or two rows, is treated as the heavier one and recorded as such.

### 4. Light mode: the Orchestrator writes a minimal spec

Conflict. `coordinator.md` says "you coordinate; you do not write code or specs". The owner's practical minimum is three roles with specification and planning assumed by the Orchestrator. This ADR resolves it by narrowing, not removing, the rule:

- In light mode, and only there, the coordinator writes exactly one artifact: a "Light spec" block at the top of `openspec/changes/<change>/apply-progress.md`, committed as `docs(<change>): light spec` BEFORE any `tdd-apply` run. The follow-up edit gives the coordinator `Edit`/`Write` for that file only, by rule (tools cannot be path-scoped; `verify` enforces it, see below).
- The block is at most ~40 lines and has these required fields:

```
## Light spec
Flow: light | pack: <pack> | tier: T<n> | verify: V<n> | decided-by: coordinator|owner
Reason: <one line: why light, which floors were checked and not hit>
Scope: <1 to 3 lines>      Non-goals: <list>
Acceptance criteria (each testable; type = unit|property|integration|command|manual):
  AC1 Given ... When ... Then ...   [type]   [ref: requirement or D-nn if one exists]
Plan: <ordered steps, at most 7, expected files, test first for each AC>
Risk and reversibility: <one line>        Owner decisions touched: none (checked D-nn: ...)
Escalation triggers watched: ambiguity, public API/format, domain invariant, size, architecture
```

- The coordinator still must NOT: write production code, tests or any Rust; edit delta specs, main specs (`openspec/specs/`), product specs (`docs/specs/`) or ADRs; decide anything in `docs/specs/00-decisiones.md` or any owner-only item; invent behavior the specs and owner decisions do not state (if an AC needs invention it is ambiguity, which is a floor for standard); verify or approve its own work; amend the Light spec after implementation starts except through a separate `docs(<change>): light spec amend` commit that states why.
- The implementer receives only the Light spec block, the paths, and the skills; it must report `blocked` if a criterion is not testable or needs a decision (already in `tdd-apply.md`).
- `verify` checks that the Light spec commit touches only that file, that the block was not changed by an implementer commit, and that every AC has a test or command, with probes written from the ACs first. The author of the criteria (coordinator) and the author of the tests (implementer) differ, and the checker (verify) differs from both.

Why not delegate the spec to `specifier` in light mode: it adds a model run and a hand-off for criteria that are a few lines; the history shows the value of `specifier` is on contracts that outlive the change, which is what the standard floor captures.

Escalation triggers light to standard or full (the coordinator checks them after every agent return, and `tdd-apply` or `verify` report them as `blocked` or as a `class:`):

| Trigger | Action |
|---|---|
| Ambiguity: an AC needs behavior no spec states; `tdd-apply` returns `blocked` class `ambiguity`; `verify` finding class `ambiguity` | standard: `specifier` writes the delta spec; the Light spec is retired and linked |
| Public API or serialized-format change appears in the diff | standard |
| A new rule on critical code (domain-crate invariant) appears | standard, tier raised to T3 |
| Measured size above ~300 lines, or forecast x1.4 above ~400 | standard; above ~700 or several deliveries needed: full (`architect` splits) |
| Architecture question: new dependency edge, V5 row, new crate, layering, a design decision that is not local | full: `architect`, then `specifier` if requirements change |
| Owner decision touched | stop, ask the owner |
| Second FAIL of any class on the same delivery | upgrade one mode and tell the owner; do not run a third light round |

### 5. Mutation tiers

Measured basis (cargo-mutants 27.1.0): `--in-diff` mutates only code under the diff and ignores test-only diffs; "edits in one region can leave code elsewhere less well tested"; `--iterate` skips mutants caught in earlier runs but matches by file, line and column, so moved code is retested, and it is a heuristic to be confirmed by a full run; `--shard k/n` runs a slice (`round-robin` evens the cost); `--file`/`--re`/`--exclude` narrow by path or name; `--baseline=skip` avoids repeating a green baseline.

| Tier | Name | Command shape (scratch output outside the repo) | Applies to | Cost basis | Catches |
|---|---|---|---|---|---|
| T0 | none | none | no Rust source touched (docs, config, release) | 0 | n/a |
| T1 | hand mutants | `verify` only, at most 3 to 5, on a scratch export | tiny code diffs on non-critical or non-domain code (tooling, test-support) where a cargo-mutants run costs more than it finds | minutes of one agent | order and rule mutations cargo-mutants cannot express |
| T2 | diff-scoped (today's default) | `cargo mutants -p <crates> --in-diff <diff> --jobs 4` | any delivery that changes Rust code | measured: 13 to 47 mutants, 36 s to about 2 min | survivors on changed lines |
| T3 | touched files (new, intermediate) | `cargo mutants -p <crate> --file <each changed non-test src file> --jobs 4`; on a fix round add `--iterate` with the same output dir | deliveries that edit existing files holding critical code; refactors of such files; new function in a domain crate | all mutants of the changed files: in `gafira-edit` 1 to 45 per file (history 45, budget 44, set_property 36), so a typical delivery touches 25 to 80 mutants (roughly 1 to 3 min at the measured 1.6 s per mutant of wall time), between the diff run and the 182 of the whole crate | tests that stopped covering unchanged lines in the same file, which `--in-diff` skips by design |
| T4 | whole crate | `cargo mutants -p <crate> --jobs 4` | at archive of a change (every crate it touched), before a release that changes code, and after a toolchain bump that changed code | measured: `gafira-geom` 29 s (53 mutants); `gafira-edit` 4 min 54 s (182 mutants: 148 caught, 30 unviable, 4 missed) | regressions anywhere in the crate; equivalence-exclusion hygiene |

Rules:
- Pick the tier by the pack, then raise it by one when the delivery edits an existing critical file (T2 to T3) or adds a first-of-kind rule. Lower only by the owner.
- `--iterate` is allowed only on fix rounds that are test-only and keep the same file set; the round that closes the delivery as PASS and T4 never use it (the docs say to confirm with a full run).
- Optional sampling for long changes: `--shard 1/4 --sharding round-robin` on the whole crate as a mid-change checkpoint (about 25% of T4). Not a default; offered when a change has more than four deliveries before its archive.
- A critical survivor is CRITICAL class `implementation`; a non-critical survivor stays WARNING (unchanged policy). Exclusions in `.cargo/mutants.toml` need a reason and are re-checked by `verify` (`domain-color` PR2 shows why).
- `mutants` reports the wall time and mutant counts in its result, and `verify` copies them to `verification.json`, so tier costs are measured per run from now on.
- In light mode, `mutants` runs only when the tier is T2 or above; T1 uses `verify` alone.

### 6. Artifact contract, gates, retries

Mapping to the repository (no new folders):

| Contract artifact | In this repo | Written by |
|---|---|---|
| `spec.md` | `openspec/changes/<change>/specs/<capability>/spec.md` (delta); light mode: Light spec block in `apply-progress.md` | `specifier`; coordinator (light) |
| `design.md` | `openspec/changes/<change>/design.md` (decisions, delivery table with dependencies and `parallel`/`serial`) | `architect` |
| `tasks.md` | the delivery table in `design.md`; a `tasks.md` only when the design does not hold the table | `architect` |
| progress and evidence | `openspec/changes/<change>/apply-progress.md` (Light spec or `Flow:` line, done items, RED evidence, deviations, measured lines) | `tdd-apply` appends; coordinator writes the header |
| `verification.json` | `openspec/changes/<change>/verification.json` (new, below) beside `verify-report.md` | `verify` |

`verification.json` earns its keep because the coordinator today reads a report of 111 to 1910 lines to find `class:` and `next:`, and a resumed run has no compact record of rounds and tiers. Proposed schema v1, at most ~40 lines, one object per verify round appended to a `rounds` array:

```json
{ "schema": 1, "change": "...", "delivery": "D3", "rounds": [ {
  "round": 1, "commit": "<sha verified>", "mode": "standard", "tier": "T3", "depth": "V2",
  "verdict": "FAIL", "next": "tdd-apply",
  "counts": {"critical": 1, "warning": 4, "suggestion": 2},
  "findings": [ {"id": "C1", "severity": "CRITICAL", "class": "implementation",
                 "ref": "path:line or requirement id", "summary": "<=200 chars"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": {"tier": "T3", "found": 36, "caught": 32, "missed": 0, "timeout": 0,
               "unviable": 4, "critical_missed": 0, "wall_s": 120},
  "hand_mutants": {"run": 5, "killed": 5},
  "criteria": {"total": 9, "covered": 9},
  "size": {"lines": 684, "forecast": 600}, "report": "verify-report.md" } ] }
```

The prose report stays the human record; the JSON is the machine record and the only thing the coordinator reads to route. If the two disagree, the report wins and the round is re-run. It is not emitted for V0. It must stay small; if it grows beyond these fields it stops earning its keep and is dropped (see Risks).

Gates (the coordinator checks each before the next phase; failing a gate returns the work to the stage that owns it):

| Gate | Passes when | Applies |
|---|---|---|
| G0 Acceptance criteria documented before implementing (hard precondition) | A committed artifact (Light spec block, or delta spec) exists BEFORE the first implementation commit, with: scope, non-goals, at least one acceptance criterion per behavior change, each criterion testable and typed (Given/When/Then, or a command with expected output for tooling and docs), the `Flow:` line with tier and depth. The coordinator greps for the fields; `tdd-apply` returns `blocked` without it; `verify` fails the delivery (class `ambiguity`) if the artifact is missing or changed by the implementer. Docs-only packs satisfy it with a checklist of facts that must be present. Not waivable by the coordinator; if the owner says "no spec", the coordinator writes a three-line Light spec anyway | all modes |
| G1 After `specifier` | No unresolved open question; every requirement has a scenario with a type; owner decisions respected | standard, full |
| G2 After `architect` | Delivery table with dependencies; each delivery near or below ~700 lines; risky claims resolved by `explorer` | full |
| G3 After `tdd-apply` | `status: done` with commit list, gate output, measured lines, RED evidence per criterion, deviations | all code modes |
| G4 After `verify` (+ `mutants`) | Zero CRITICAL; tier thresholds met; `verification.json` written | all modes with V1 or above |
| G5 Before push/PR and before archive | PASS for all deliveries; T4 run for full; archive order respected | standard, full |

Retries and escalation (aligned with the feedback-loop rule in `coordinator.md`):
- Route each FAIL by `class:` as already defined (implementation, architecture, ambiguity, claim); mixed reports go upstream first.
- After two consecutive FAILs of the same class on the same delivery: stop and ask the owner (one question, with a recommendation). `domain-doc` D4a is the only delivery in the history that reached this (two FAILs, then PASS), so the rule fires rarely and cheaply.
- Hard cap across classes: three verify rounds per delivery; the fourth is an owner decision.
- Light mode is stricter: the second FAIL of any class upgrades the mode (section 4) instead of running a third light round.
- Never self-approve a FAIL, never waive a finding, never raise the cap silently. Owner-only items are never routed to an agent.

### 7. Recording and override

- The coordinator writes one line in `apply-progress.md` (header; for light mode inside the Light spec block): `Flow: <mode> | pack: <pack> | tier: T<n> | verify: V<n> | decided-by: coordinator|owner | reason: <one line>`.
- Every change of mode is appended: `Flow-change: light -> standard (trigger: ambiguity, after verify round 1)`. `verification.json` repeats `mode`, `tier` and `depth` per round, so the decision is auditable and a resumed session reads it instead of re-deciding.
- The coordinator tells the owner the chosen mode in its first report, in one line.
- Owner override: "run it full", "run it light", "skip mutation", "T4 now". The coordinator records `decided-by: owner`. If the override goes below a floor, the coordinator says so once, in one line with the risk, then obeys; G0 stays (minimal form). An override applies to the current change only; a lasting rule needs a new ADR.
- Mode may be lowered mid-run only at a phase boundary and only if no work of the heavier mode started (for example, `architect` returns a single delivery and no design decision, so a full run continues as standard). Raising is immediate (section 4).

## Consequences

Easier:
- Small requests (docs, bumps, hotfixes, local bug fixes) run with two or three agent runs instead of five to seven, and with no mutation run unless Rust changed.
- Mutation cost matches risk: T3 closes the gap that `--in-diff` leaves in edited critical files without paying a whole-crate run every delivery.
- The coordinator reads `verification.json` instead of long reports; a resumed run has a compact audit trail.
- Light-mode criteria exist before code, which makes `verify` independent of the implementer's tests.

Harder:
- The coordinator now classifies requests and writes a small spec in light mode; this widens its authority and needs the enforcement described in section 4.
- Two new artifacts to maintain (`Flow:` lines, `verification.json`) and one more mutation tier to run.
- Misclassification is possible; the floors and escalation triggers bound it but do not remove it.

Risks:
- Authority creep: the coordinator drifts from the Light spec into writing tests or code. Mitigation: explicit prohibition, `verify` checks commit scope, owner reviews PRs.
- Under-sized runs (a "light" change that is really a contract change). Mitigation: floors keyed to signals visible in the diff, plus the rule that a mismatch found by `verify` is an `ambiguity` or `architecture` finding that upgrades the mode.
- `verification.json` drifting from the report. Mitigation: the report wins; drop the JSON if it is not used for routing within the first changes.
- Light-mode independence is weaker than standard (the same orchestrator planned and routes). Mitigation: blind probes from the ACs, hand mutants, and the second-FAIL upgrade.
- Thresholds (300, 400, 700 lines; the x1.4 factor; three rounds) are judgment calls from one domain-heavy history and need a review after about ten runs, using the `mode`, `tier` and verdict fields as data.

Follow-up edits (applied after acceptance, branch `docs/apply-flow-modes`):
- [x] `.agents/agents/coordinator.md`, `verify.md`, `mutants.md`, `tdd-apply.md`, `archive.md`
- [x] `AGENTS.md` (flow modes and packs, mutation tiers, size thresholds, `verification.json` in the OpenSpec layout) and the index row in `docs/adr/README.md`
- [x] `docs/manuals/tecnico/chapters/09-tooling-and-quality.md`: mutation tiers and flow modes (delegate to `manual-writer`; separate change)

## Alternatives considered

- Keep one flow and only skip stages ad hoc. Rejected: it is today's behavior, and it leaves the choice implicit, unrecorded and inconsistent between sessions.
- Two modes only (light and full), as in the owner's example. Rejected as the default: the history shows a middle class (a new rule in an existing domain crate) where a delta spec is needed (spec lag warnings in almost every `edit-history` delivery) but `architect` adds nothing (no architecture FAIL ever). Standard exists to avoid paying for `architect` there. If the owner prefers two modes, standard can fold into full with `architect` marked optional.
- A numeric risk score. Rejected: false precision from a handful of data points; floors plus pack defaults are simpler to apply and audit.
- Let `specifier` write the Light spec in light mode. Rejected for cost and hand-off overhead; kept as the escalation target. If the owner wants the coordinator never to write, the same block can be produced by a single `specifier` run at the price of a fourth agent run.
- Whole-crate mutation on every delivery. Rejected: minutes of cost per delivery for little gain over T3 on files actually edited; T4 stays at archive.
- Sampling (`--shard`) as the intermediate tier. Rejected as the default: it is random with respect to what changed; offered only as an optional mid-change checkpoint.
- Function-level restriction with `--re` for T3. Rejected: names change during refactors and the list would be hand-maintained; file globs are derivable from `git diff --name-only`.
- `verification.json` replaced by stricter report headings. Not rejected outright: the feedback-loop change already made `class:` and `next:` mechanical; the JSON is justified by report size and resume needs, and can be dropped if the pilot shows it unused.
- Give `tdd-apply` a self-run of `cargo mutants --in-diff` before returning. Not adopted now: it would move the highest-yield check into the author's hands and remove independence; revisit if fix rounds keep costing a full verify cycle each.

## Decision record

The owner answered the seven open decisions on 2026-10-25. Where an answer differs from the proposal text above, the answer wins and the agent definitions follow it.

1. Modes: three (light, standard, full), kept.
2. Light spec author: the coordinator writes the Light spec block itself at the top of `apply-progress.md`, committed before the first `tdd-apply` run. Restriction to that single file is by rule, not tooling. `verify` checks that the Light-spec commit touches only that file and that the implementer did not change it afterwards. The coordinator still must not write production code or tests, edit delta, main or product specs or ADRs, decide owner-only items, invent behavior or approve its own work.
3. Mutation: T3 (mutation per changed non-test source file with `--file`; `--iterate` only on test-only fix rounds) is adopted as the intermediate tier and is the default for standard mode. T4 (whole crate) runs at archive for every crate the change touched, in every mode, including light runs that changed Rust (owner-confirmed, not an extrapolation).
4. Thresholds: escalate from light when the forecast x1.4 exceeds about 300 lines (this replaces the "x1.4 above ~400" wording in sections 1 and 4); delivery ceiling about 700 lines, and above it needs the owner's explicit OK (as accepted before for `edit-history` D4a, D7, D8 and D9); at most three verify rounds per delivery, plus the existing rule "two consecutive FAILs of the same class". These are signals for the coordinator to stop and ask, not hard blocks.
5. `verification.json`: adopted as a pilot for every mode except V0, emitted by `verify` beside `verify-report.md`. Review point: after the next change that uses it is archived. If routing did not read it, remove it.
6. Light-run change folders are archived with `archive` into `openspec/changes/archive/` without delta merge (there is no delta spec).
7. No new agents. Definitions of agents and skills are written by a general-purpose agent that the coordinator delegates to, in a worktree, and the owner reviews the PR before merge.

## Verification of this ADR

Local measurements used (cargo-mutants 27.1.0, `--jobs 4`): `--list` mutant counts per crate and per `gafira-edit` file; whole-crate runs on the current `main`: `gafira-geom` 53 mutants (39 caught, 14 unviable) in 29 s; `gafira-edit` 182 mutants (148 caught, 30 unviable, 4 missed) in 4 min 54 s (user time 22 min with 4 jobs). The 4 missed are `CommandError` `Display` (non-critical, already W1 in D1), `History::is_empty` returning `true` or `false` (accessor), and `History::debug_check` replaced by `()` (debug-only check); none is a critical-code survivor, which supports leaving T4 at archive only. Individual mutant runs in `gafira-edit` cost 11 to 34 s of test time each because of the property tests, so the diff cost there is dominated by test time, not mutant count.

## Sources

Consulted on 2026-10-25; summarized in my own words.
- Microsoft for Developers, spec-driven development (lean spec with scenarios, constraints and acceptance criteria; not every change needs the full lifecycle): https://developer.microsoft.com/blog/spec-driven-development-ai-native-engineering/
- Hidde de Smet, hidden costs of spec-driven development (three lanes: full, light, none; reversible-in-a-day goes straight through; light spec of problem, scope, success criteria, risks, validation): https://hiddedesmet.com/the-hidden-costs-of-spec-driven-development
- Augment Code, Coordinator-Implementor-Verifier (verifier checks against the original spec, deterministic checks before semantic review, three attempts per subtask, replanning cap, human checkpoints, when not to use multi-agent): https://www.augmentcode.com/guides/coordinator-implementor-verifier
- Spec Kit Agents, arXiv 2604.05278 (validate spec, plan and tasks before code; phase-scoped discovery and validation hooks; failure taxonomy; budgets): https://arxiv.org/html/2604.05278v1
- Cleveroad, AI spec-driven development (reversal cost rather than task size decides whether a durable spec is needed; mechanical changes do not need one): https://www.cleveroad.com/blog/ai-spec-driven-development/
- Augment Code, multi-agent orchestration architecture (schema validation gates between agents, independent verification separate from the reviewing agent): https://www.augmentcode.com/guides/multi-agent-orchestration-architecture-guide
- Testlio, risk-based testing (acceptance criteria that address identified risks, included in the definition of done): https://www.testlio.com/blog/risk-based-testing
- cargo-mutants manual: `--in-diff` https://mutants.rs/in-diff.html ; `--iterate` https://mutants.rs/iterate.html ; shards and sampling https://mutants.rs/shards.html ; local `cargo mutants --help` for `--file`, `--re`, `--baseline`.
