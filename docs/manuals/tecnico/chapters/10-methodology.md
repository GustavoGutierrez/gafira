# Development methodology

## Status

| Area | Status | Source |
|---|---|---|
| Flow modes, packs, gates G0 to G5, verify depths V0 to V3, mutation tiers T0 to T4 | **Implemented** (accepted decision; the agent definitions follow it) | `docs/adr/0001-flow-modes-and-packs.md`, `AGENTS.md`, `.agents/agents/coordinator.md` |
| Fast lane: spec-direct execution with a Slice card, the `mutation` toggle, V1/V2 verify defaults | **Implemented** (accepted decision; supersedes parts of ADR 0001) | `docs/adr/0002-fast-lane.md`, `AGENTS.md`, `.agents/agents/coordinator.md` |
| `verification.json` | **Implemented** as a pilot; the review point is the archive of the next change that uses it | ADR 0001 decision record 5, `.agents/agents/verify.md` |
| Hand-mutant split between `tdd-apply` and `verify`, closure rounds, T4 survivor closure | **Implemented** (accepted decision; the agent definitions follow it) | `docs/adr/0004-hand-mutant-split-and-verify-rounds.md`, `AGENTS.md`, `.agents/agents/verify.md`, `.agents/agents/tdd-apply.md`, `.agents/agents/coordinator.md` |
| Archive of a light or fast-lane run without delta merge | **Implemented** | ADR 0001 decision record 6, ADR 0002 section 6, `.agents/agents/archive.md` |
| ODD lite: risk levels L0 to L3, outcome entries in `.odd/state.json`, folders only when needed, verify findings in the entry, the BLOCKED verdict | **Implemented** in the process files; its ADR is **proposed** until the `layout-core` D3 pilot ends | `docs/adr/0011-odd-lite.md`, `AGENTS.md`, `.agents/agents/coordinator.md`, `.agents/agents/verify.md` |

## Lite SDD

Specs already say what to build, so there are no separate proposal, design or tasks phases by default. Two upstream stages are optional: `specifier` when the spec is missing or ambiguous, and `architect` when a real design decision is open or deliveries must run in parallel (serial deliveries listed in a Slice card need no `architect`). The full flow is:

```
(specify) -> spec -> (architect) -> (explorer, only when a claim is risky) -> tdd-apply -> verify -> (mutants T4 per touched crate) -> archive
```

On work that an existing spec covers, the default route records the acceptance criteria in an outcome entry in `.odd/state.json` (see [ODD lite](#odd-lite)). `tdd-apply` works straight from the product requirement IDs, with no proposal, no design and no delta spec. The Slice card of the fast lane (ADR 0002, [Fast lane](#fast-lane)) stays the form for a change folder in progress. In this flow `spec` means the product spec plus the outcome entry or the Slice card.

The mode of a request (next section) decides which stages run. `mutants` runs according to the `mutation` value of the `Flow:` line. The agents live in `.agents/agents/`.

| Step | Agent | Output |
|---|---|---|
| Turn a request into specs: requirements, scenarios, acceptance criteria, non-goals, open questions | `specifier` | Fast lane: only on a reported ambiguity, and it fixes the product spec in `specs/`. Otherwise delta specs in `specs/changes/<change>/specs/` |
| Decide how, when a spec leaves a design decision open; split the work into deliveries with a dependency table | `architect` | `specs/changes/<change>/design.md`, cross-cutting ADRs in `docs/adr/` |
| Measure risky claims in a scratch directory and cross-check numbers across docs | `explorer` | `specs/changes/<change>/spike.md` with command, output and verdict; with no folder, the result only |
| Build one delivery with strict TDD | `tdd-apply` | Commits and `specs/changes/<change>/apply-progress.md`; with no folder, notes in the body of the last implementer commit |
| Mutation-test the code according to the `mutation` value (default: whole crate at change close); report surviving mutants (read-only) | `mutants` | A report in the session scratch directory, outside the repo |
| Check gates and every acceptance criterion, requirement and scenario | `verify` | `verify-report.md` and `verification.json` in the change folder (with no folder, in the scratchpad), PASS, FAIL or BLOCKED |
| Close the change after PASS and all deliveries done | `archive` | Change moved to `specs/history/` (with no folder, nothing to move and the entry is deleted); fast lane: status table in `specs/README.md` updated with the covered IDs; delta specs, when present, merged into `specs/capabilities/` |
| Edit the technical manual, English first with a Spanish mirror | `manual-writer` | Chapters in both languages, `manual.py check` and `build` evidence |

`verify` runs the local gates, maps every criterion to a test or a command, reads the mutant report when a run happened and writes a verdict with numbered findings (critical, warning, suggestion). A FAIL opens a fix round: `tdd-apply` writes the tests first and records the round in `apply-progress.md`, then `verify` runs again. Round 1 is the only round with independent probes and hand mutants; later rounds are closure rounds (see [Verify rounds](#verify-rounds)).

## ODD lite

ODD lite routes a request by risk and records its progress outside a change folder. It sits on top of the flow modes and removes no gate, floor or owner stop. The decision is `docs/adr/0011-odd-lite.md`, status **proposed** until the `layout-core` D3 pilot ends. The process files (`AGENTS.md`, the agent definitions) already apply it.

Natural-language requests remain the entry point; there are no ODD commands. The brief's `odd-spec` is `specifier` with the `product-specs` skill, `odd-run` is `coordinator` with `tdd-apply`, and `odd-verify` is `verify`. An outcome is an observable result, a spec describes a stable capability and a task is executable work.

### Risk levels

The coordinator sets the level before the floors. The level comes from impact, uncertainty and reversibility, never from the file count. The mode is the heaviest of floor, level and pack default. That rule fixes the mode only: verify depth and agents come from the pack row, and a level can raise them, never lower them.

| Level | Examples | Mode | Path |
|---|---|---|---|
| L0 trivial | Text, comments, docs, style with no behavior change | Light | One writer (or the pack's agents), V0 or the pack's depth when higher, `mutation: off` where allowed |
| L1 routine | A bounded bug, an interaction tweak | Light | Criteria, `tdd-apply`, `verify` |
| L2 functional | A new tool or function, a feature the specs already cover | Standard | Update the spec when requirements change, serial batches, `verify`, T4 at close |
| L3 critical | Data migration, security, licensing, the project format | Full | Folder with design and ADR, broad tests, `verify`, owner approval where it applies |

A level never goes below a floor and never lowers a pack default. The one exception is `mutation: off` for L0 and L1 work that touches no critical code. New findings raise the level at any time.

### Outcome entries and change folders

New work on a capability that an existing spec covers needs no folder under `specs/changes/`. Its acceptance criteria are the outcome's entry in `.odd/state.json`, or the spec row when requirements change. A committed entry satisfies G0. The outcome id replaces `<change>` in branch names, commit scopes and agent inputs.

| Work | Acceptance record | Folder |
|---|---|---|
| Capability covered by a spec (L0 to L2) | Outcome entry | None |
| No spec covers it (tooling, agent definitions, a new capability) | Light spec or delta spec | `specs/changes/<change>/` |
| Full mode or L3 | Slice card or delta spec | `specs/changes/<change>/`, with `design.md` and an ADR when the decision is cross-cutting |
| A folder already in progress | What it has | Keeps folder, card and design until archive |

When the owner says `flow: legacy`, the change runs with its folder and no level, and the `Flow:` line ends in `| flow: legacy`. This path stays available until phase D of ADR 0011 closes.

With no folder, the other artifacts move:

- `tdd-apply` records its notes in the body of the last implementer commit of the delivery.
- `verify` writes `verify-report.md` and `verification.json` to the scratchpad path the coordinator gives it.
- `explorer` returns its result without a `spike.md`.
- `archive` has nothing to move and runs only for covered IDs or follow-ups; the entry is deleted.

### Outcome state

`.odd/state.json` is tracked and small: `{ "schema": 1, "outcomes": [ ... ] }`, with one object per active outcome. The coordinator updates it at boundaries in a commit that touches only that file, scoped to the outcome (`docs(<outcome>): ...`). `tdd-apply` never edits it. The entry is deleted when its outcome closes, and git keeps the history.

| Field | Content |
|---|---|
| `outcome_id` | Stable kebab-case id; for a folder in progress, the change name |
| `spec_id` | File name of the spec without extension, or `none` |
| `status` | `specifying`, `implementing`, `verifying` or `blocked` |
| `criteria` | List of `{id, status, evidence}`; `status` is `pending`, `passed`, `failed` or `blocked` |
| `next_action` | One line |

When the entry is the G0 artifact, it also holds `flow` (the `Flow:` line), `scope`, `non_goals`, `text` and `type` on a criterion the spec does not state, and `verify`.

The `verify` field is `{round, commit, verdict, next, findings, rounds}`. The first four describe the last run. The other two store what a later session needs, because the scratchpad does not survive a session.

| Field | Content |
|---|---|
| `findings` | Every CRITICAL and WARNING of any run, as `{id, severity, class, ref, summary, follow_up, status}`, copied from `verification.json`; no SUGGESTION |
| `rounds` | One `{round, verdict, classes}` row per verify run of the current delivery; a BLOCKED row stays and does not count toward the stops |

The coordinator commits this update on the delivery branch before it routes, on PASS, FAIL and BLOCKED alike. A WARNING still open when a delivery ends in PASS becomes a follow-up and reaches `archive`.

On resume, the coordinator checks the branch, `git status`, the diff against `develop` and the tests before it continues. The entry is a pointer: when it disagrees with git, git wins and the entry is corrected. A resumed run does not redraft the plan or duplicate a spec row or a commit.

### BLOCKED verdict

`verify` returns BLOCKED when a critical check could not run: a gate of the depth, the only proof of a criterion, an applicable CI policy check or a T4 report on critical code (ADR 0011). BLOCKED goes to the owner, never counts as PASS and costs no round. Gate G4 rejects it.

## Flow modes and packs

The `coordinator` classifies each request before it routes. It sets the risk level (see [ODD lite](#odd-lite)), applies the hard floors, then the pack default, then the owner's explicit instruction. It runs only the agents and checks of the resulting mode. Decisions: `docs/adr/0001-flow-modes-and-packs.md` (accepted, partly superseded) and `docs/adr/0002-fast-lane.md` (accepted).

### Modes

| | Light | Standard | Full |
|---|---|---|---|
| Agents | `coordinator`, `tdd-apply`, `verify` | adds optional `explorer`, `mutants` (per `mutation`), `archive`; `specifier` only on ambiguity or without a product spec | adds `architect` |
| Acceptance source | Outcome entry on covered work, otherwise a Light spec block written by the coordinator | Product spec IDs plus outcome entry or Slice card (fast lane), or delta specs | As standard, plus `design.md` with the delivery table |
| Gates | G0, G3, G4 | G0, G1 (only if `specifier` ran), G3, G4, G5 | G0 to G5 |
| Default `mutation` | `off` or `archive` per pack | `archive` (T4 at close) | `archive` (T4 at close) |
| Default verify depth | V0, V1 or V2 per pack | V2 | V2 (V3 only on owner request) |

### Hard floors

The first matching row sets the minimum mode and the higher mode always wins. The coordinator never lowers a mode below a floor. Only the owner can lower a pack default.

| Signal | Minimum mode |
|---|---|
| The criteria need an owner decision that `specs/00-decisiones.md` does not give | Stop and ask the owner |
| New V5 layer-table row, new dependency edge not already stated in `specs/02-arquitectura-software.md`, new serialized format version, a cross-cutting design choice the spec leaves open, or parallel deliveries wanted | Full |
| New crate that `02` already describes (crate list, layer, dependencies); public API or format change in an existing crate; a new rule on critical code; a request so ambiguous that criteria would invent behavior | Standard |
| Forecast times 1.4 above about 300 changed lines | Standard |

About 700 lines per delivery stays a signal to stop and ask, not a floor. One change may run several deliveries serially without `architect` when the Slice card lists them. A request that matches no pack row, or two rows, is treated as the heavier one.

### Packs

A pack fixes the mode, agents, `mutation` value and verify depth for a request type. The Mutation column gives the value and the tier.

| Request type | Mode | Mutation | Verify |
|---|---|---|---|
| Docs only | Light | `off` (T0) | V0 |
| Manual change | Light, `manual-writer` | `off` (T0) | V0 (`manual.py check` and `build`) |
| Skill or agent-definition change | Light, a general-purpose agent in a worktree, then `verify` | `off` (T0) | V1 |
| Dependency or toolchain bump | Light | `off` (T0); `archive` (T4) if code changed | V1 (V2 if code changed) |
| Bug fix | Light if the specs state the behavior and the cause is local; otherwise standard | `archive` (T4) | V2, reproducing test RED first |
| Small refactor, no behavior change | Light | `archive` (T4) | V2 and an empty public item diff |
| New function in an existing domain crate | Standard (fast lane when a product spec states it) | `archive` (T4) | V2 |
| Product-spec slice: new capability or crate that `02` describes (fast lane) | Standard; full only on a full floor | `archive` (T4) | V2 |
| Spec-only or product-spec work | Standard without code, `specifier` | `off` (T0) | V0 |
| Release or versioning | Light | `off` (T0) | V1 |
| Investigation or spike | Light, `explorer` only | `off` (T0) | None; the output is `spike.md` |
| Hotfix on CI or tooling | Light | `off` (T0) | V1 |
| Test-only hardening or fix round after a FAIL | Inherits the mode of the change | Same value; `--iterate` allowed with `delivery` | V2 on the findings only |

A change to a V5 row or an action pin in a CI hotfix is a full floor. The owner reviews every change to agent or skill definitions, at the latest in the `develop` -> `main` pull request, because it changes the process itself.

### Recording and override

The coordinator writes one line in the header of `apply-progress.md` (inside the Light spec block in light mode, inside the Slice card in the fast lane), or in the outcome entry when there is no folder:

```
Flow: <mode> | level: L<n> | pack: <pack> | mutation: off|archive|delivery | tier: T<n> | verify: V<n> | decided-by: coordinator|owner | reason: <one line>
```

Flow lines without `level` stay valid, and a legacy run ends the line in `| flow: legacy`. In the fast lane the line sits inside the Slice card. The `tier` field names the per-delivery tier with `mutation: delivery`, `T4` with `archive` and `T0` with `off`.

Each change of mode is appended as `Flow-change: light -> standard (trigger: ambiguity, after verify round 1)`. The coordinator reports the chosen mode to the owner in its first report.

The owner's explicit instruction beats every other rule ("run it full", "run it light", "skip mutation" meaning `mutation: off`, "mutate every delivery" meaning `delivery`, "T4 now", "V3"). The coordinator records `decided-by: owner`. If the instruction goes below a floor, the coordinator states the risk once, then obeys, and G0 stays in its minimal form. An override applies to the current change only. The coordinator can raise the mode at once. It can lower the mode only at a phase boundary, before work of the heavier mode starts, and never below a floor.

## Gates

The coordinator checks each gate before the next phase. A failed gate returns the work to the stage that owns it.

| Gate | Passes when | Modes |
|---|---|---|
| G0 | A committed artifact (outcome entry, Light spec block, Slice card with its product spec IDs, or delta spec) exists before the first implementation commit, with scope, non-goals, a testable typed criterion per behavior change and the `Flow:` line | All |
| G1 | After `specifier`: no open question, every requirement has a typed scenario, owner decisions respected | Standard, full |
| G2 | After `architect`: delivery table with dependencies, each delivery near or below about 700 lines | Full |
| G3 | After `tdd-apply`: `status: done` with commits, gate output, measured lines, RED evidence per criterion, deviations | All code modes |
| G4 | After `verify` (and `mutants` when a run happened): zero CRITICAL, verdict not BLOCKED, tier thresholds met, `verification.json` written | All modes with V1 or above |
| G5 | Before merge into `develop` and archive: PASS for all deliveries; with `mutation: archive` or `delivery`, T4 run for every crate the change touched and no critical survivor | Standard, full |

The coordinator cannot waive G0. If the owner asks for no spec, the coordinator writes a three-line Light spec. For a product-spec slice the Slice card satisfies G0.

## Light mode

In light mode the coordinator writes one artifact. On covered work it is the outcome entry, committed as `docs(<outcome>): outcome entry`. Otherwise it is a Light spec block of at most about 40 lines at the top of `specs/changes/<change>/apply-progress.md`, committed as `docs(<change>): light spec`. Either commit comes before any `tdd-apply` run. The block holds the `Flow:` line, the reason, scope, non-goals, typed acceptance criteria, a plan of at most 7 steps, risk and reversibility, and the owner decisions touched.

The coordinator still does not write production code or tests, edit delta, main or product specs or ADRs, decide owner-only items, invent behavior, or approve its own work. It amends the block after implementation starts only in a separate `docs(<change>): light spec amend` commit that states why. The restriction to one file is a rule, not a tooling limit. `verify` enforces it: it checks that the Light spec commit touches only that file and that no implementer commit changed the block.

A light run has no delta spec. `archive` moves a change folder to `specs/history/` without a delta merge; with no folder, nothing moves.

### Escalation

The coordinator checks these triggers after every agent return. Raising the mode is immediate and recorded with `Flow-change:`.

| Trigger | Action |
|---|---|
| An acceptance criterion needs behavior no spec states (`blocked` or finding class `ambiguity`) | Standard: `specifier` fixes the product spec when one exists (fast lane), otherwise writes the delta spec |
| Public API or serialized-format change in the diff | Standard |
| A new rule on critical code | Standard; the coordinator considers `mutation: delivery` (T3) and states why |
| Measured size above about 300 lines, or forecast times 1.4 above about 300 | Standard; above about 700: split into serial deliveries listed in the Slice card, or `architect` if the split needs a design decision or parallel work |
| Architecture question (new V5 row, dependency edge not stated in `02`, format version, open cross-cutting choice) | Full: `architect` |
| An owner decision is touched | Stop and ask the owner |
| Second FAIL of any class on the same delivery | Upgrade one mode and tell the owner; no third light round |

## Fast lane

The fast lane executes a product spec directly (ADR 0002, section 3). When the owner asks to execute a document in `specs/`, or a section or ID range of it, the coordinator writes a Slice card at the top of `specs/changes/<change>/apply-progress.md`. It commits the card as `docs(<change>): slice card` before any implementation, and the card satisfies G0. On covered work with no folder, the same fields go in the outcome entry instead; the card remains the form for L3, full mode and folders in progress.

```
## Slice card
Flow: <mode> | pack: <pack> | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator|owner | reason: <one line>
Product IDs: <ARQ-nn, RF-nn ...> (source: specs/<NN-doc>.md §x)
Scope: <1 to 3 lines>      Non-goals: <list>
Deliveries: <only if more than one: D1 <scope, ~lines>, D2 ...; serial>
Plan: <ordered steps, expected crates and files>
Owner decisions touched: none (checked D-nn: ...)
```

Rules of the fast lane:

- There is no `proposal.md`, no `design.md` and no delta spec. The acceptance source for `tdd-apply` and `verify` is the product spec rows of the listed IDs plus the Slice card.
- Each test names the requirement ID it proves, in the test name or a one-line comment, so `git grep ARQ-nn` traces coverage. `verify` checks that every Slice card ID has a test that names it.
- `specifier` runs only when `tdd-apply` reports `blocked` or `verify` reports a finding of class `ambiguity`. It fixes the product spec in `specs/` (Spanish), not a delta spec, unless the owner asks for one.
- The coordinator writes the Slice card with the same limits as the Light spec. It never edits the product spec, and it amends the card only in a separate `docs(<change>): slice card amend` commit that states why.
- A new crate that `02` already describes is a standard floor, not full. Delivery size and serial deliveries follow the hard floors above.
- Requests that are not product-spec slices keep the Light spec block.
- `archive` does no delta merge. It moves the folder and updates the "Detalle" column of the `specs/README.md` status table with the covered IDs. Changes with delta specs keep the ADR 0001 merge, and `specs/capabilities/` stays as the record of past changes.

## Verify depths

The depth sets how much independent work `verify` does. `verify` never edits source and never trusts the implementer's summary.

| Depth | What runs |
|---|---|
| V0 | No agent. The coordinator runs the deterministic checks of the pack (links, `manual.py check`, `cargo fmt --check`, `git diff --stat`) and reports them verbatim |
| V1 | Gates at HEAD; each acceptance criterion mapped to a test or a command; PASS or FAIL |
| V2 | V1, plus probe checks derived from the criteria before reading the implementer's tests, round 1 only: one spot-check of the `tdd-apply` hand-mutant list plus 2 to 3 independent hand mutants on a scratch export (see [Hand mutants](#hand-mutants)), findings classified by `class:` |
| V3 | Only on owner request: V2, plus per-commit gates on clean exports, negative runs for CI policy and the full requirement and scenario matrix |

### Verify rounds

Round 1 is the first `verify` of a delivery and the only round that runs the independent probes and the independent hand mutants. Rounds 2 and later are closure rounds.

| Round | What `verify` does |
|---|---|
| 1 | Gates, criteria map, independent probes and hand mutants (V2 and V3) |
| 2+ (closure) | Gates, artifact-integrity checks and closure of earlier findings only; no independent-mutant budget |

- A `verify` after a fix round, after a T4 fix round, or the change-level final `verify` is a closure round, unless the coordinator says it is the first verify of that code.
- A new test gap found after round 1 is a WARNING of class `implementation` marked `follow-up`. It does not block, in any mode and whatever the `mutation` value, and costs no round. The coordinator lists it for the owner and `archive` appends it to the follow-up list of `specs/README.md`.
- A real code defect shown by a probe, a gate failure and a critical survivor in the newest `mutants` report are CRITICAL in any round.
- A critical T4 survivor goes to `tdd-apply`, then T4 runs again on that crate, then a closure-round `verify` reads the new report. Only that new report proves closure; a stale report leaves the survivor CRITICAL.

### verification.json

`verify` writes `verification.json` beside `verify-report.md` for every depth above V0: in `specs/changes/<change>/`, or in the scratchpad path the coordinator gives it when there is no folder. It holds one object per verify round in a `rounds` array: round, verified commit, mode, tier, depth, verdict, `next`, counts, findings (id, severity, `class`, reference, summary), gate exit codes, mutation counts and wall time, hand mutants, criteria covered, and measured size. The coordinator reads this file, not the long report, to route, and copies the findings into the outcome entry when there is no folder. If the two disagree, the report wins and the round runs again.

The file is a pilot. If routing does not read it after the next change that uses it is archived, it is removed.

## Mutation tiers

Tiers set how much of the code `mutants` and `verify` mutate. The [tooling chapter](09-tooling-and-quality.md) lists the commands.

| Tier | Scope | Applies to |
|---|---|---|
| T0 | None | No Rust source touched |
| T1 | Hand mutants by `verify` only: 2 to 3 independent mutants | Tiny changes on non-critical code |
| T2 | `cargo mutants --in-diff`: changed lines | `mutation: delivery`, deliveries that change Rust |
| T3 | `--file` per changed non-test source file | `mutation: delivery`; deliveries that edit an existing critical file |
| T4 | Whole crate | Change close, before archive, for every crate the change touched; the default (`mutation: archive`) |

The coordinator raises a tier by one when a delivery edits an existing critical file or adds a first-of-kind rule. Only the owner lowers a tier. `--iterate` is allowed only on test-only fix rounds with the same file set, never on the round that closes a delivery or on T4. In light mode `mutants` runs only from T2 up.

### Mutation toggle

The `mutation` field of the `Flow:` line selects when `cargo-mutants` runs (ADR 0002, section 1).

| Value | Effect |
|---|---|
| `archive` (default) | No per-delivery run. One T4 run per touched crate when the change closes, before archive. A critical survivor goes back to `tdd-apply`, then T4 runs again on that crate, then a closure-round `verify` reads the new report |
| `delivery` | T2 or T3 per delivery (`--iterate` on test-only fix rounds), plus T4 at close. Only on owner request, or when the coordinator flags a new rule on critical code |
| `off` | No `cargo-mutants`. The hand mutants of `tdd-apply` and `verify` still apply |

### Hand mutants

Hand mutants show that a test can fail. Their budget is split between the two agents, in any mode and whatever the `mutation` value. Neither part replaces T4.

| Agent | Hand-mutant work |
|---|---|
| `coordinator` | On critical code, gives `tdd-apply` a rule-level list: each item names a spec rule and its wrong behavior, with no code |
| `tdd-apply` | Applies each item, confirms a test fails, reverts it and records the item and its killing test in `apply-progress.md`; a survivor needs a new test, an item that cannot be applied is `blocked`, and `done` needs every item killed |
| `verify` (V2 and V3, round 1) | Does not repeat the list. Spot-checks one item by applying it and confirming the recorded test fails, then adds 2 to 3 independent hand mutants chosen from the spec rules |

With no recorded list there is no spot-check, and the 2 to 3 independent mutants still apply. Tier T1 relies on them alone. A hand-mutant survivor is class `implementation`, and CRITICAL in critical code.

## Control of the cycle

The coordinator owns the cycle from the request to archive. Every agent returns its result to the coordinator and none calls another; the `next` field of a result is a recommendation. Subagents cannot launch subagents, so the coordinator must be the main-thread agent.

`verify` gives each CRITICAL and WARNING finding a class. The coordinator routes by class and never fixes a finding itself.

| Class | Meaning | Route |
|---|---|---|
| `implementation` | Code or test gap, gate failure, surviving critical mutant, size | `tdd-apply`, then `verify` and `mutants` |
| `architecture` | Wrong decomposition, decision or ADR | `architect`, then `specifier` if requirements change, then `tdd-apply` |
| `ambiguity` | Missing or contradictory requirement | `specifier`; open questions go to the owner |
| `claim` | Risky unproven claim | `explorer` |

A mixed report goes upstream first: ambiguity, then architecture, then implementation. A BLOCKED verdict is not a class: it goes straight to the owner.

### Loop limits

- After two consecutive FAILs of the same class on one delivery, the coordinator stops and asks the owner, with one question and a recommendation.
- A BLOCKED run costs no round; with no folder, the stops read the `rounds` of the entry and skip BLOCKED rows.
- A delivery has at most three verify rounds across classes. A fourth round is an owner decision. A T4 fix cycle counts as one verify round of the change's last delivery.
- In light mode the second FAIL of any class upgrades the mode instead.
- The coordinator never self-approves a FAIL, waives a finding or raises a cap silently. Owner-only items (decisions in `specs/00-decisiones.md`, merges, pricing, license, scope) are never routed to an agent.

These limits are signals to stop and ask, not hard blocks. The delivery ceiling is about 700 lines; above it needs the owner's explicit approval.

## Strict TDD

RED (write the test, run it, see it fail), GREEN (minimal code), REFACTOR (tests stay green). One commit per reviewable work unit. Every commit passes the three local gates alone (see the tooling chapter). Where a test passes at once because the code already exists, RED is shown by a mutant instead (see the mutation testing section of the tooling chapter).

## Specs

| Place | Content |
|---|---|
| `specs/00-decisiones.md` | Owner decisions (D-nn). They override other specs and are not reopened without the owner |
| `specs/NN-*.md` | The document in progress; completed ones move to `specs/archive/` with `git mv`, waiting ones sit in `specs/backlog/` |
| `specs/capabilities/<capability>/spec.md` | Main specs per capability, the source of truth, filled by archive |
| `specs/changes/<change>/` | Active change folder, only for L3, full mode, work no spec covers and `flow: legacy`: `proposal.md`, `design.md`, `tasks.md` (optional), `specs/<capability>/spec.md` (delta specs; none in a light or fast-lane run), `apply-progress.md` (starts with the Light spec or the Slice card), `verify-report.md`, `verification.json` |
| `specs/history/<date>-<change>/` | Closed change folders |
| `.odd/state.json` | Outcome entries of active outcomes (see [ODD lite](#odd-lite)) |

Spec format: RFC 2119 keywords; scenarios as Given/When/Then; each requirement tagged with its delivery (`D1`, `D2a`...) and each scenario with its type (unit, property, integration, command, manual). Delta specs use `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements` (with a Reason and a Migration note) and `## RENAMED Requirements`. Archive is the only step that edits main specs, only for changes with delta specs, and it warns before any destructive delta. Fast-lane changes have none, so new fast-lane work does not edit `specs/capabilities/`.

Closed changes sit in `specs/history/<date>-<change>/`, named by closing date. Their text is kept as it was when they closed.

## Commits, branches and pull requests

- Conventional commits; no AI attribution lines.
- `main` holds released, stable code and `develop`, created from `main`, is the integration branch; they are the only long-lived branches (ADR 0003). Every change and delivery is a short-lived branch cut from `develop` (`feat/<change>-<delivery>`, `docs/...`, `chore/...`).
- After a delivery passes verify, the coordinator merges its branch into `develop` locally with `git merge --no-ff`, runs the gates once on `develop`, pushes `develop` and deletes the branch. There is no pull request per delivery or change; deliveries of one change merge in order, and a FAIL never reaches `develop`.
- The only pull request is `develop` -> `main`, opened only when the owner asks or at a release-worthy point (a release or tag, a completed product document, a milestone the owner names); the coordinator may suggest one at those points and opens it only with the owner's OK. The owner reviews and merges it with a merge commit; releases and tags are cut from `main` after that merge, and the coordinator syncs `develop` with `main` (fast-forward when possible) and pushes it.
- State lives in the outcome entry, in the change folder when there is one, and in git, not in an external memory tool.
- CI is disabled manually on GitHub for now (billing), so each delivery's gates run locally before its merge into `develop`; see the tooling chapter.

## Code and language rules

- Rust edition 2024, toolchain 1.93.1; `unsafe` is forbidden; `#[allow]` is not used to silence lints.
- Domain crates use `BTreeMap` and `BTreeSet`, never `HashMap` or `HashSet`.
- Artifacts (code, comments, docs, specs, commits) are in English. User-facing UI text is Spanish and English (D-08). Product-facing documents under `specs/` are in Spanish.
- No absolute or user paths in tracked files. Skills are referenced by name only.

## Documentation

This manual lives in `docs/manuals/tecnico/` and is maintained with `manual.py` (see the folder README). A change that alters a public API, a layering rule, a gate or the process updates the matching chapter in the same pull request, and marks anything not yet built as **Planned**.
