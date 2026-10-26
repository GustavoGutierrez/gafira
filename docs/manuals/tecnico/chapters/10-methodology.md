# Development methodology

## Status

| Area | Status | Source |
|---|---|---|
| Flow modes, packs, gates G0 to G5, verify depths V0 to V3, mutation tiers T0 to T4 | **Implemented** (accepted decision; the agent definitions follow it) | `docs/adr/0001-flow-modes-and-packs.md`, `AGENTS.md`, `.agents/agents/coordinator.md` |
| `verification.json` | **Implemented** as a pilot; the review point is the archive of the next change that uses it | ADR 0001 decision record 5, `.agents/agents/verify.md` |
| Archive of a light run without delta merge | **Implemented** | ADR 0001 decision record 6, `.agents/agents/archive.md` |

## Lite SDD

Specs already say what to build, so there are no separate proposal, design or tasks phases by default. Two upstream stages are optional: `specifier` when the spec is missing or ambiguous, and `architect` when a real design decision or a delivery breakdown is needed. The full flow is:

```
(specify) -> spec -> (architect) -> (explorer, only when a claim is risky) -> tdd-apply -> mutants + verify -> archive
```

The mode of a request (next section) decides which stages run. The agents live in `.agents/agents/`.

| Step | Agent | Output |
|---|---|---|
| Turn a request into specs: requirements, scenarios, acceptance criteria, non-goals, open questions | `specifier` | Delta specs in `openspec/changes/<change>/specs/` |
| Decide how, when a spec leaves a design decision open; split the work into deliveries with a dependency table | `architect` | `openspec/changes/<change>/design.md`, cross-cutting ADRs in `docs/adr/` |
| Measure risky claims in a scratch directory and cross-check numbers across docs | `explorer` | `openspec/changes/<change>/spike.md` with command, output and verdict |
| Build one delivery with strict TDD | `tdd-apply` | Commits and `openspec/changes/<change>/apply-progress.md` |
| Mutation-test the code under the tier of the run; report surviving mutants (read-only) | `mutants` | A report in the session scratch directory, outside the repo |
| Check gates and every acceptance criterion, requirement and scenario | `verify` | `verify-report.md` and `verification.json` in the change folder, PASS or FAIL |
| Close the change after PASS and all deliveries done | `archive` | Delta specs merged into `openspec/specs/`; change moved to `openspec/changes/archive/` |
| Edit the technical manual, English first with a Spanish mirror | `manual-writer` | Chapters in both languages, `manual.py check` and `build` evidence |

`mutants` and `verify` run in parallel on the same delivery. `verify` runs the local gates, maps every criterion to a test or a command, reads the mutant report and writes a verdict with numbered findings (critical, warning, suggestion). A FAIL opens a fix round: `tdd-apply` writes the tests first and records the round in `apply-progress.md`, then `verify` runs again.

## Flow modes and packs

The `coordinator` classifies each request before it routes. It applies the hard floors, then the pack default, then the owner's explicit instruction. It runs only the agents and checks of the resulting mode. Decision: `docs/adr/0001-flow-modes-and-packs.md` (accepted).

### Modes

| | Light | Standard | Full |
|---|---|---|---|
| Agents | `coordinator`, `tdd-apply`, `verify` | adds `specifier`, optional `explorer`, `mutants`, `archive` | adds `architect`; `mutants` and `verify` per delivery |
| Acceptance source | Light spec block written by the coordinator | Delta specs from `specifier` | Delta specs and `design.md` with the delivery table |
| Gates | G0, G3, G4 | G0, G1, G3, G4, G5 | G0 to G5 |
| Default mutation tier | T1 or T2 per pack | T2 or T3 per pack | T2 per delivery, T4 at archive |
| Default verify depth | V1 or V2 per pack | V2 | V3 for the first delivery of a new crate or capability, V2 later |

### Hard floors

The first matching row sets the minimum mode and the higher mode always wins. The coordinator never lowers a mode below a floor. Only the owner can lower a pack default.

| Signal | Minimum mode |
|---|---|
| The criteria need an owner decision that `docs/specs/00-decisiones.md` does not give | Stop and ask the owner |
| New crate, dependency edge, V5 layer-table row, serialized format version, cross-cutting design choice, or several ordered deliveries | Full |
| Public API or format change in an existing crate; a new rule on critical code; a request so ambiguous that criteria would invent behavior | Standard |
| Forecast times 1.4 above about 300 changed lines | Standard |
| Forecast times 1.4 above about 700 lines, or parallel deliveries wanted | Full |

A request that matches no pack row, or two rows, is treated as the heavier one.

### Packs

A pack fixes the mode, agents, mutation tier and verify depth for a request type.

| Request type | Mode | Tier | Verify |
|---|---|---|---|
| Docs only | Light | T0 | V0 |
| Manual change | Light, `manual-writer` | T0 | V0 (`manual.py check` and `build`) |
| Skill or agent-definition change | Light, a general-purpose agent in a worktree, then `verify` | T0 | V1 |
| Dependency or toolchain bump | Light | T0 (T2 on the fallout diff if code changed) | V1 (V2 if code changed) |
| Bug fix | Light if the specs state the behavior and the cause is local; otherwise standard | T2 (T3 on a critical file) | V2, reproducing test RED first |
| Small refactor, no behavior change | Light | T2 (T3 on a critical file) | V2 and an empty public item diff |
| New function in an existing domain crate | Standard | T3 | V2 |
| New capability or crate | Full | T2 per delivery, T4 at archive | V3, then V2 |
| Spec-only or product-spec work | Standard without code, `specifier` | T0 | V0 |
| Release or versioning | Light | T0 | V1 |
| Investigation or spike | Light, `explorer` only | T0 | None; the output is `spike.md` |
| Hotfix on CI or tooling | Light | T0 | V1 |
| Test-only hardening or fix round after a FAIL | Inherits the mode of the change | Same tier, `--iterate` allowed | V2 on the findings only |

A change to a V5 row or an action pin in a CI hotfix is a full floor. The owner reviews every pull request that changes agent or skill definitions, because it changes the process itself.

### Recording and override

The coordinator writes one line in the header of `apply-progress.md` (inside the Light spec block in light mode):

```
Flow: <mode> | pack: <pack> | tier: T<n> | verify: V<n> | decided-by: coordinator|owner | reason: <one line>
```

Each change of mode is appended as `Flow-change: light -> standard (trigger: ambiguity, after verify round 1)`. The coordinator reports the chosen mode to the owner in its first report.

The owner's explicit instruction beats every other rule ("run it full", "run it light", "skip mutation", "T4 now"). The coordinator records `decided-by: owner`. If the instruction goes below a floor, the coordinator states the risk once, then obeys, and G0 stays in its minimal form. An override applies to the current change only. The coordinator can raise the mode at once. It can lower the mode only at a phase boundary, before work of the heavier mode starts, and never below a floor.

## Gates

The coordinator checks each gate before the next phase. A failed gate returns the work to the stage that owns it.

| Gate | Passes when | Modes |
|---|---|---|
| G0 | A committed artifact (Light spec block or delta spec) exists before the first implementation commit, with scope, non-goals, a testable typed criterion per behavior change and the `Flow:` line | All |
| G1 | After `specifier`: no open question, every requirement has a typed scenario, owner decisions respected | Standard, full |
| G2 | After `architect`: delivery table with dependencies, each delivery near or below about 700 lines | Full |
| G3 | After `tdd-apply`: `status: done` with commits, gate output, measured lines, RED evidence per criterion, deviations | All code modes |
| G4 | After `verify` and `mutants`: zero CRITICAL, tier thresholds met, `verification.json` written | All modes with V1 or above |
| G5 | Before push, pull request and archive: PASS for all deliveries, T4 run for every crate the change touched | Standard, full |

The coordinator cannot waive G0. If the owner asks for no spec, the coordinator writes a three-line Light spec.

## Light mode

In light mode the coordinator writes one artifact: a Light spec block of at most about 40 lines at the top of `openspec/changes/<change>/apply-progress.md`. It commits the block as `docs(<change>): light spec` before any `tdd-apply` run. The block holds the `Flow:` line, the reason, scope, non-goals, typed acceptance criteria, a plan of at most 7 steps, risk and reversibility, and the owner decisions touched.

The coordinator still does not write production code or tests, edit delta, main or product specs or ADRs, decide owner-only items, invent behavior, or approve its own work. It amends the block after implementation starts only in a separate `docs(<change>): light spec amend` commit that states why. The restriction to one file is a rule, not a tooling limit. `verify` enforces it: it checks that the Light spec commit touches only that file and that no implementer commit changed the block.

A light run has no delta spec. `archive` moves the change folder to `openspec/changes/archive/` without a delta merge.

### Escalation

The coordinator checks these triggers after every agent return. Raising the mode is immediate and recorded with `Flow-change:`.

| Trigger | Action |
|---|---|
| An acceptance criterion needs behavior no spec states (`blocked` or finding class `ambiguity`) | Standard: `specifier` writes the delta spec |
| Public API or serialized-format change in the diff | Standard |
| A new rule on critical code | Standard, tier raised to T3 |
| Measured size above about 300 lines, or forecast times 1.4 above about 300 | Standard; above about 700 or several deliveries: full |
| Architecture question (dependency edge, V5 row, new crate, layering) | Full: `architect` |
| An owner decision is touched | Stop and ask the owner |
| Second FAIL of any class on the same delivery | Upgrade one mode and tell the owner; no third light round |

## Verify depths

The depth sets how much independent work `verify` does. `verify` never edits source and never trusts the implementer's summary.

| Depth | What runs |
|---|---|
| V0 | No agent. The coordinator runs the deterministic checks of the pack (links, `manual.py check`, `cargo fmt --check`, `git diff --stat`) and reports them verbatim |
| V1 | Gates at HEAD; each acceptance criterion mapped to a test or a command; PASS or FAIL |
| V2 | V1, plus probe checks derived from the criteria before reading the implementer's tests, hand mutants (at most 5, at most 3 in light mode) on a scratch export, findings classified by `class:` |
| V3 | V2, plus per-commit gates on clean exports, negative runs for CI policy and the full requirement and scenario matrix |

### verification.json

`verify` writes `openspec/changes/<change>/verification.json` beside `verify-report.md` for every depth above V0. It holds one object per verify round in a `rounds` array: round, verified commit, mode, tier, depth, verdict, `next`, counts, findings (id, severity, `class`, reference, summary), gate exit codes, mutation counts and wall time, hand mutants, criteria covered, and measured size. The coordinator reads this file, not the long report, to route. If the two disagree, the report wins and the round runs again.

The file is a pilot. If routing does not read it after the next change that uses it is archived, it is removed.

## Mutation tiers

Tiers set how much of the code `mutants` and `verify` mutate. The [tooling chapter](09-tooling-and-quality.md) lists the commands.

| Tier | Scope | Applies to |
|---|---|---|
| T0 | None | No Rust source touched |
| T1 | Hand mutants by `verify` only, at most 3 to 5 | Tiny changes on non-critical code |
| T2 | `cargo mutants --in-diff`: changed lines | Any delivery that changes Rust |
| T3 | `--file` per changed non-test source file | Standard default; deliveries that edit an existing critical file |
| T4 | Whole crate | Archive, for every crate the change touched, in every mode |

The coordinator raises a tier by one when a delivery edits an existing critical file or adds a first-of-kind rule. Only the owner lowers a tier. `--iterate` is allowed only on test-only fix rounds with the same file set, never on the round that closes a delivery or on T4. In light mode `mutants` runs only from T2 up.

## Control of the cycle

The coordinator owns the cycle from the request to archive. Every agent returns its result to the coordinator and none calls another; the `next` field of a result is a recommendation. Subagents cannot launch subagents, so the coordinator must be the main-thread agent.

`verify` gives each CRITICAL and WARNING finding a class. The coordinator routes by class and never fixes a finding itself.

| Class | Meaning | Route |
|---|---|---|
| `implementation` | Code or test gap, gate failure, surviving critical mutant, size | `tdd-apply`, then `verify` and `mutants` |
| `architecture` | Wrong decomposition, decision or ADR | `architect`, then `specifier` if requirements change, then `tdd-apply` |
| `ambiguity` | Missing or contradictory requirement | `specifier`; open questions go to the owner |
| `claim` | Risky unproven claim | `explorer` |

A mixed report goes upstream first: ambiguity, then architecture, then implementation.

### Loop limits

- After two consecutive FAILs of the same class on one delivery, the coordinator stops and asks the owner, with one question and a recommendation.
- A delivery has at most three verify rounds across classes. A fourth round is an owner decision.
- In light mode the second FAIL of any class upgrades the mode instead.
- The coordinator never self-approves a FAIL, waives a finding or raises a cap silently. Owner-only items (decisions in `docs/specs/00-decisiones.md`, merges, pricing, license, scope) are never routed to an agent.

These limits are signals to stop and ask, not hard blocks. The delivery ceiling is about 700 lines; above it needs the owner's explicit approval.

## Strict TDD

RED (write the test, run it, see it fail), GREEN (minimal code), REFACTOR (tests stay green). One commit per reviewable work unit. Every commit passes the three local gates alone (see the tooling chapter). Where a test passes at once because the code already exists, RED is shown by a mutant instead (see the mutation testing section of the tooling chapter).

## Specs

| Place | Content |
|---|---|
| `docs/specs/00-decisiones.md` | Owner decisions (D-nn). They override other specs and are not reopened without the owner |
| `docs/specs/NN-*.md` | The document in progress; completed ones move to `docs/specs/archive/` with `git mv`, waiting ones sit in `docs/specs/backlog/` |
| `openspec/specs/<capability>/spec.md` | Main specs per capability, the source of truth, filled by archive |
| `openspec/changes/<change>/` | `proposal.md`, `design.md`, `tasks.md` (optional), `specs/<capability>/spec.md` (delta specs; none in a light run), `apply-progress.md`, `verify-report.md`, `verification.json` |

Spec format: RFC 2119 keywords; scenarios as Given/When/Then; each requirement tagged with its delivery (`D1`, `D2a`...) and each scenario with its type (unit, property, integration, command, manual). Delta specs use `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements` (with a Reason and a Migration note) and `## RENAMED Requirements`. Archive is the only step that edits main specs, and it warns before any destructive delta.

Change chain so far: six changes are closed and archived under `openspec/changes/archive/`, and no change is open. In order: `workspace-scaffold`, `domain-color` and `domain-doc` (2026-10-24), `domain-time` and `edit-history` (2026-10-25), and `engine-pathops` (2026-10-26). The `edit-history` change includes all its deliveries, D1 to D10, with the CI step V14.

## Commits, branches and pull requests

- Conventional commits; no AI attribution lines.
- Trunk-based development with short-lived branches. After a delivery passes verify, the coordinator pushes the branch and opens a pull request against `main` (or the previous unmerged delivery branch). The owner merges, in order, bottom-up, with merge commits. A pull request whose base was merged is retargeted to `main` first.
- State lives in the change folder and in git, not in an external memory tool.
- CI is disabled manually on GitHub for now (billing), so each delivery's gates run locally before the pull request; see the tooling chapter.

## Code and language rules

- Rust edition 2024, toolchain 1.93.1; `unsafe` is forbidden; `#[allow]` is not used to silence lints.
- Domain crates use `BTreeMap` and `BTreeSet`, never `HashMap` or `HashSet`.
- Artifacts (code, comments, docs, specs, commits) are in English. User-facing UI text is Spanish and English (D-08). Product-facing documents under `docs/specs/` are in Spanish.
- No absolute or user paths in tracked files. Skills are referenced by name only.

## Documentation

This manual lives in `docs/manuals/tecnico/` and is maintained with `manual.py` (see the folder README). A change that alters a public API, a layering rule, a gate or the process updates the matching chapter in the same pull request, and marks anything not yet built as **Planned**.
