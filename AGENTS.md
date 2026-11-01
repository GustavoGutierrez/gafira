# Gafira: development methodology

Gafira is a local-first design, image and video editor written in Rust (Cargo workspace of `gafira-*` crates). Product and architecture live in `docs/specs/` (see its `README.md` for the status table): only `00-decisiones.md` (owner decisions) and the document in progress sit there; a completed document moves with `git mv` to `docs/specs/archive/` (never `00-decisiones.md`) before the next one is promoted; the rest (01 product and quality, 03 visual design, 04 image/export, 05 video/distribution, 06 license) wait in `docs/specs/backlog/`. When the document in progress is completed, the coordinator promotes the next one from the backlog (`git mv` into `docs/specs/`, in its own commit) and keeps the status table current. Owner decisions in `00-decisiones.md` are not reopened without the owner.

## ODD lite

Outcome-driven routing on top of the flow below (`docs/adr/0011-odd-lite.md`, proposed until the `layout-core` D3 pilot ends). It selects mode and agents; it removes no gate, floor or owner stop. An outcome is an observable result, a spec describes a stable capability, a task is executable work: three different things. Natural-language requests stay the entry point; there are no ODD commands (`odd-spec` is `specifier` with skill `product-specs`, `odd-run` is `coordinator` with `tdd-apply`, `odd-verify` is `verify`).

- Put verifiable results and code changes before ceremonial artifacts.
- Read the code and the relevant spec first; do not load every spec.
- Tie a request to an existing spec unless it brings a genuinely new capability.
- Do not create a change, proposal, design, plan or spec per file, endpoint or subtask.
- Before implementing, hold enough acceptance criteria and constraints; ask only about real blocks.
- Run small batches: test (when it applies), implementation, tests, verification.
- Use `verify` for the independent check of the result, and fix its findings.
- Update specs only when requirements, interfaces or lasting decisions change.
- Record progress so it can resume, in `.odd/state.json`, without duplicating git or the Slice card.
- Raise design and verification rigour on risky changes (permissions, migrations, data loss, security, licensing, the project format).
- Never call something complete without evidence for its acceptance criteria and the tests run.
- Keep higher-precedence instructions and the runtime's own configuration.

### Risk levels

The level comes from impact, uncertainty and reversibility, never from the file count. The mode is the heaviest of floor, level and pack default; that rule fixes the mode only. Verify depth and agents come from the pack row of the chosen mode, and a level may raise them, never lower them. A level can raise rigour, never goes below a floor and never lowers a pack default, with one exception: `mutation: off` (see "Mutation"). New findings raise it at any time.

| Level | Examples | Mode | Path |
|---|---|---|---|
| L0 trivial | text, comments, docs, style with no behaviour change | light | docs-style path: one writer (or the pack's agents), V0, or the pack's depth when higher, `mutation: off` where allowed (no critical code), spec only if a contract changes |
| L1 routine | bounded bug, interaction tweak | light | criteria, `tdd-apply`, `verify` |
| L2 functional | new tool, new function, a persistence feature the specs already cover | standard | update the existing spec when requirements change, serial batches, `verify`, T4 at close |
| L3 critical | data migration, security, licensing, project format | full | folder with design and ADR, migration plan if any, broad tests, `verify`, owner approval where it applies |

### Change folder rule

ODD changes the default: new work on a capability an existing spec covers needs no `openspec/changes/<change>/`, proposal, design or tasks file. Its acceptance criteria are the outcome's entry in `.odd/state.json`, or the spec row when requirements change, and a committed entry satisfies G0. The outcome id takes the place of `<change>` in branch names, commit scopes and agent inputs.

| Work | Acceptance record | Folder |
|---|---|---|
| Capability covered by an existing spec (L0 to L2) | outcome entry (product IDs for a product-spec slice, typed criteria otherwise) | none |
| No spec covers it (tooling, agent definitions, a new capability) | Light spec or delta spec | yes |
| Full mode or L3 | Slice card or delta spec | yes, with `design.md` and an ADR when cross-cutting |
| A folder already in progress | what it has | keeps folder, card and design until archive |

`layout-core` is such a folder: its remaining deliveries D3 to D7 run with minimal notes (the evidence G3 and `verify` need, no narrative).

With no folder, `tdd-apply` records its progress notes in the body of the last implementer commit of the delivery (skipping coordinator commits that touch only `.odd/state.json`); `verify` writes `verify-report.md` and `verification.json` to the scratchpad path the coordinator gives it, and the coordinator keeps what must survive a session in the entry's `verify` field (`round`, `commit`, `verdict`, `next`, `findings`, `rounds`); `explorer` returns its result without a `spike.md`.

The old path stays available until phase D of ADR 0011: when the owner says `flow: legacy`, the change runs with its folder and no level, and the Flow line ends in `| flow: legacy`.

### Outcome state

`.odd/state.json` is tracked and small: `{ "schema": 1, "outcomes": [ ... ] }`, one object per active outcome. The coordinator updates it at boundaries (criteria recorded, delivery verified, next action changed) in a commit that touches only that file; `tdd-apply` never does. An entry is deleted when its outcome closes; git keeps the history.

| Field | Content |
|---|---|
| `outcome_id` | stable kebab-case id; for a folder in progress, the change name |
| `spec_id` | file name of the spec without extension, or `none` |
| `status` | `specifying`, `implementing`, `verifying` or `blocked` |
| `criteria` | list of `{id, status, evidence}`: `id` is a product ID, delivery or AC; `status` is `pending`, `passed`, `failed` or `blocked`; `evidence` is a test, command or commit, omitted while pending |
| `next_action` | one line |

Only when the entry is the G0 artifact (no Slice card or Light spec holds these): `flow` (the Flow line), `scope` (with the serial delivery list when there is more than one), `non_goals`, `text` and `type` on a criterion the spec does not state, and `verify` (below). Do not copy what git or the Slice card already hold.

`verify` is `{round, commit, verdict, next, findings, rounds}`: the first four describe the last run; `findings` and `rounds` are optional, so older entries stay valid and `schema` stays 1.

| Field | Content |
|---|---|
| `findings` | every CRITICAL and WARNING of any run, as `{id, severity, class, ref, summary, follow_up, status}` copied from `verification.json` (same ids, each unique across the entry's rounds; `summary` <=200 chars). `follow_up` is true for a WARNING marked follow-up, and for every WARNING still open when a delivery ends in PASS. `status` is `open` or `closed`, as the latest closure round's `verification.json` lists it; a re-listed id updates its finding. SUGGESTIONs are not stored |
| `rounds` | one `{round, verdict, classes}` row per verify run of the current delivery; `classes` holds the distinct classes of its CRITICAL and WARNING findings that are not follow-ups, including earlier findings still open in that round. A BLOCKED row is kept and does not count toward the stops; its re-run has the same round number and replaces the row |

The coordinator commits this update on the delivery branch before routing, on PASS, FAIL and BLOCKED alike, because the scratchpad does not survive a session. A PASS does not route WARNINGs, so on a delivery PASS every still-open WARNING that is not a follow-up becomes `follow_up: true` and reaches `archive`. When the next delivery of the same outcome starts, the coordinator resets `rounds` and drops closed findings; open follow-ups stay until `archive` consumes them.

Resume: check the branch, `git status`, the diff against `develop` and the tests before continuing. An outcome mid-delivery may be ahead of `develop`: list the unmerged local branches, read `.odd/state.json` on each and on `develop`, and collect the active entries. A branch matches an outcome by the exact `outcome_id` in its file, never by a substring of its name. With no outcome named and more than one active, ask the owner which. Trust the entry on the delivery branch; with several, the one with the newest commit touching that file. The entry is a pointer, not the truth: when it disagrees with git, git wins and the entry is corrected. Be idempotent: repeating a step must not duplicate a spec row or a commit, or overwrite correct code, and a resumed run does not redraft the plan.

## Flow (lite SDD)

Specs already say what to build, so there is no separate proposal, design or tasks phase by default. Two optional upstream stages exist: `specify` when the spec is missing or ambiguous, and `architect` when a real design decision is open or deliveries must run in parallel (serial deliveries listed in a Slice card need no `architect`, ADR 0002).

Fast lane (default, `docs/adr/0002-fast-lane.md`): when the owner asks to execute a product spec in `docs/specs/` (or a section or ID range of it), the coordinator writes a Slice card (ODD lite: on covered work, the outcome entry with the same fields) and `tdd-apply` works straight from the product requirement IDs; no proposal, design or delta specs.

```
(specify)  ->  spec  ->  (architect)  ->  (explorer, only when a claim is risky)  ->  tdd-apply  ->  verify  ->  (mutants T4 per touched crate)  ->  archive
```

Stages in parentheses are skipped when the spec is already clear and complete. `spec` is the product spec plus the outcome entry or Slice card (fast lane), the outcome entry or Light spec block (light) or delta specs. `mutants` runs per the `mutation` value of the Flow line: per touched crate at change close by default, per delivery only with `mutation: delivery`, never with `off`.

### Flow modes and packs

Decisions: `docs/adr/0001-flow-modes-and-packs.md` (accepted, partly superseded) and `docs/adr/0002-fast-lane.md` (accepted). The coordinator classifies each request: hard floors first (owner decision needed: stop; new V5 row, dependency edge not already stated in `02`, format version or a cross-cutting design choice the spec leaves open, or parallel deliveries wanted: full; public API or format change, new rule on critical code, ambiguity, or a new crate that `02` already describes: standard; forecast x1.4 above ~300 lines: standard), then the pack default, then the owner's explicit override (`decided-by: owner`). Before the floors it sets the risk level (see "ODD lite"). It records `Flow: <mode> | level: L<n> | pack | mutation: off|archive|delivery | tier | verify | decided-by | reason` in `apply-progress.md`, or in the outcome entry when there is no folder, and every change as `Flow-change:`. Flow lines without `level` stay valid.

| Mode | Runs | Spec | Gates |
|---|---|---|---|
| light | coordinator, `tdd-apply`, `verify` (docs and tooling packs: one writer, `verify` V0/V1) | outcome entry on covered work; otherwise Light spec block (acceptance criteria) written by the coordinator at the top of `apply-progress.md` | G0, G3, G4 |
| standard | adds `mutants` (per `mutation`), `archive`; optional `explorer`; `specifier` only on ambiguity, when requirements change or when there is no product spec | product spec IDs + outcome entry or Slice card (fast lane), or delta specs | G0, G1 (only if `specifier` ran), G3, G4, G5 |
| full | adds `architect` (design, delivery table) | as standard, plus `design.md` | G0 to G5 |

G0 (acceptance criteria committed before the first implementation commit) applies to every mode. In light mode the coordinator writes only the Light spec block, in the fast lane only the Slice card, and in either the outcome entry; never code, tests, other specs or ADRs; `verify` checks that commit touches only that file and the implementer did not change it. Verify depth defaults: V1 for docs and tooling packs, V2 for code; the docs-only and manual packs may use V0 (coordinator runs deterministic checks, no agent); V3 (per-commit gates and the full matrix) only on owner request.

Mutation (`mutation:` in the Flow line): `archive` (default) runs no per-delivery mutants and one T4 per touched crate when the change closes, before archive, re-run on a crate after a critical survivor is fixed; `delivery` runs the per-delivery tiers (T2/T3, `--iterate` on test-only fix rounds) plus T4 at close, only on owner request or when the coordinator flags a new rule on critical code; `off` runs no `cargo-mutants` (hand mutants still apply, see the testing policy). "Skip mutation" from the owner means `off`. ODD lite: the coordinator may choose `off` only for L0 and L1 work that touches no critical code, the only case where a level lowers a pack default; on critical code T4 at close stays mandatory unless the owner says otherwise.

Delivery size: ~700 lines per delivery stays a signal to stop and ask. One change may run several deliveries serially without `architect` when the Slice card lists them; `architect` runs only for an open design decision or a dependency table with parallel deliveries.

Packs: docs, manual, agent/skill definitions, bumps, bug fix, refactor, new function, new capability, spec-only, release, spike, CI hotfix, test-only round (table in ADR 0001, defaults amended by ADR 0002, and in `coordinator.md`). Agent and skill definitions are written by a general-purpose agent in a worktree, delegated by the coordinator; the owner reviews them, at the latest in the `develop` -> `main` PR. Light and fast-lane runs are archived without delta merge (archive moves the folder and, for a fast-lane change, updates the `docs/specs/README.md` status table with the covered IDs). An outcome with no folder has nothing to move: `archive` runs only for covered IDs or follow-ups, and the entry is deleted. With `mutation: archive` or `delivery` the entry stays until T4 closes the change: if `archive` does not run, the coordinator deletes it after T4 on a short branch merged like any other; only with `off` does it go on the last branch before its merge.

Feedback loop: the coordinator keeps control of the whole cycle; every agent returns to it and none calls another. `verify` classifies each finding, and the coordinator routes it back to the matching stage:

```
specify -> architect -> explorer -> tdd-apply -> mutants + verify -> archive
   ^          ^            ^           ^               |
   |          |            |           +-- implementation (code/test gap, gate, mutant, size)
   |          |            +-------------- claim (risky unproven claim)
   |          +--------------------------- architecture (design/ADR error, undeliverable as designed)
   +-------------------------------------- ambiguity (missing/contradictory requirement, spec gap)
```

After two consecutive FAILs of the same class on one delivery the coordinator stops and asks the owner. BLOCKED (a critical check could not run) goes straight to the owner, is never PASS and costs no round.

| Step | Agent (`.agents/agents/`) | Output |
|---|---|---|
| Turn an owner request into specs (scope, requirements, Given/When/Then scenarios, acceptance criteria, edge cases, non-goals, open questions); defines WHAT, never HOW | `specifier` (Opus) | fast lane: only on a reported ambiguity, and it fixes the product spec in `docs/specs/`; otherwise delta specs in `openspec/changes/<change>/specs/` (no product spec, or the owner asks) |
| Decide HOW when a spec leaves a design decision open; analyse the code; break the work into deliveries (~700 lines) with a dependency table (parallel or serial) | `architect` (Opus) | `openspec/changes/<change>/design.md` (proposal/tasks only if needed), cross-cutting ADRs in `docs/adr/` |
| Measure risky claims in `/tmp` (serde limits, clippy per commit, CI regexes, delivery prefixes) and cross-check numbers across docs | `explorer` (Sonnet) | `openspec/changes/<change>/spike.md` with command, output, verdict (no folder: in its result) |
| Build one delivery with strict TDD | `tdd-apply` (Sonnet) | commits + `openspec/changes/<change>/apply-progress.md` (no folder: notes in the body of the last implementer commit) |
| Mutation-test per the Flow line's `mutation` (default: whole crate at change close; per delivery only with `delivery`); report surviving mutants (read-only) | `mutants` (Sonnet) | report in the session scratchpad, outside the repo |
| Check gates and every requirement/scenario | `verify` (Opus) | `openspec/changes/<change>/verify-report.md` (no folder: under the scratchpad), PASS, FAIL or BLOCKED |
| Close the change (PASS and all deliveries done) | `archive` (Haiku) | change moved to `openspec/changes/archive/` (no folder: nothing to move, entry deleted); fast lane: status table in `docs/specs/README.md` updated with the covered IDs; delta specs (when present) merged into `openspec/specs/`; any mode: follow-up WARNINGs appended to the follow-up list of `docs/specs/README.md` |
| Update, extend or regenerate the technical manual (`docs/manuals/tecnico/`), English first with a Spanish mirror | `manual-writer` (Sonnet) | edited chapters in both languages, `manual.py check` and `build` evidence, `docs(manual)` commits |

When deliveries remain, go back to `tdd-apply`. On FAIL, the coordinator routes each finding by its `class:` (see the feedback loop above and `.agents/agents/coordinator.md`).

`coordinator` (Opus) runs the whole flow: it reads the repo state, picks the next step, delegates to the agents above (one writer at a time), rejects `done` without evidence and stops for owner decisions and merges. Subagents cannot launch other subagents, so the coordinator must be the main-thread agent: start the session with `claude --agent coordinator` (supported by the installed Claude Code CLI). If an older CLI lacks the flag, ask the main session to follow `.agents/agents/coordinator.md`. `mutants` (Sonnet) runs `cargo-mutants`: by default per touched crate at change close (T4), or per delivery in parallel with `verify` with `mutation: delivery`. `manual-writer` (Sonnet) sits outside the spec flow: the coordinator delegates to it whenever the owner asks to change the technical manual; it needs the `technical-manual` and `skill-registry` skills and stops if `technical-manual` is unavailable. `.claude/agents` is a symlink to `.agents/agents`.

A design note (`architect`) is written only when a spec leaves a real decision open, a change needs parallel deliveries or a split that depends on a design decision, or code must be analysed before choosing an approach, and only for that decision. `specifier` and `architect` cannot launch agents: they list open questions and risky claims, and the coordinator asks the owner (one question at a time, with a recommendation) or runs `explorer`. ADR convention: cross-cutting decisions go in `docs/adr/NNNN-short-title.md` (index and template in `docs/adr/README.md`), change-scoped decisions in the change's `design.md`, owner decisions stay in `docs/specs/00-decisiones.md` and are never reopened by the architect. If a test or CI claim is risky, `explorer` settles it with a real experiment before specs or code rely on it; refuted claims are fixed in the specs first. For a numeric crate (floats, tolerances, integer overflow), when upstream stages are needed, `explorer` runs before `specifier`.

## Strict TDD

RED (write the test, run it, see it fail) -> GREEN (minimal code) -> REFACTOR (tests green). One commit per reviewable work unit. Every commit must pass alone:

```
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Testing and mutation policy

- Tests cover behavior of the public contract, not implementation (see `tdd-apply`).
- Fast lane: each test names the product requirement ID it proves (`ARQ-nn`, `RF-nn`...) in its name or a one-line comment, so `git grep ARQ-nn` traces coverage.
- Critical code: validation, JSON load/save, ids and arena, transforms, size/DPI limits, any spec rule with an invariant. Non-critical: `Display`/`Error` text unless the spec fixes it, trivial accessors, test helpers.
- Hand mutants: on critical code, in any mode, the coordinator gives `tdd-apply` a rule-level list (spec rule and wrong behaviour, no code); `tdd-apply` applies each, confirms a test fails, reverts and records item and killing test in `apply-progress.md` (no folder: the body of the last implementer commit), and `done` needs all killed. `verify` at V2 and V3 does not repeat the list: it spot-checks one item and adds 2 to 3 independent hand mutants, whatever the `mutation` value. Neither replaces T4.
- Mutation tiers (`mutants`; `verify` fails on a missed or timeout mutant in critical code, except a confirmed hanging mutant (below), and documents the rest; no score target): T0 none (no Rust touched); T1 hand mutants by `verify` only (tiny non-critical code); T2 `--in-diff` (diff-scoped); T3 `--file` per changed non-test source file (per-delivery tier for standard mode with `mutation: delivery`, raised to it when a delivery edits an existing critical file; `--iterate` only on test-only fix rounds with the same file set, never on the closing PASS round); T4 whole crate at change close, before archive, for every crate the change touched, unless the Flow line says `mutation: off`. T2 and T3 run only with `mutation: delivery`; `mutation: archive` (default) runs T4 alone; `mutation: off` runs nothing. Leave `CARGO_TARGET_DIR` unset for `cargo-mutants` (per-job dirs); the coordinator never passes it.
- Hanging mutants (ADR 0004): a timeout mutant in critical code counts as detected, not CRITICAL, only when both hold: a re-run with a longer limit confirms it does not terminate (or iterates a range no test run could finish), and the report lists it by file and line with that evidence. A timeout that does not meet this, or that load explains, stays a finding. Where a test can fail on an assertion first, the test must be reordered or split so it does (for example a separate binary that sorts later), so a hang is a last resort. A missed mutant has no such exception, and critical code as defined above has none for geometry loops.
- Flags: T2 `cargo mutants -p <crates> --in-diff <diff> --jobs 8 -o <scratchpad dir>`; T3 `cargo mutants -p <crate> --file <file> --jobs 8 -o <scratchpad dir>`; T4 `cargo mutants -p <crate> --jobs 8 -o <scratchpad dir>`. Default `cargo test` runner (nextest skips doctests), no `RUSTFLAGS=-D warnings`. `#[mutants::skip]` is forbidden (it needs a dependency); exclude through `.cargo/mutants.toml` with a reason. `--locked` comes from `.cargo/mutants.toml` (`additional_cargo_test_args`); do not pass it again (cargo rejects a duplicate). The `mutants` cargo profile (`Cargo.toml`, `inherits = "test"`, `debug = "none"`) is set by `profile = "mutants"` in `.cargo/mutants.toml`; do not pass `--profile`. A fast linker (mold, wild) is optional and not installed.
- `cargo-mutants` is an optional dev tool, like `cargo-deny`; it is not a CI gate.

## Delivery rules

- Measure each delivery with `git diff --shortstat`; the ceiling is ~700 lines, and above it needs the owner's explicit OK. Estimates undershoot (measured x1.4 to x1.9 over forecast, mostly tests): apply x1.4 to forecasts; a forecast x1.4 above ~300 lines leaves light mode. At most three verify rounds per delivery, and a stop after two consecutive FAILs of the same class; round 1 is the only one with independent probes and mutants, later rounds run the gates and artifact-integrity checks and check closure of earlier findings (`verify`). These are signals for the coordinator to stop and ask, not hard blocks.
- Deliveries cut at commit boundaries; each prefix passes the gates alone.
- Conventional commits, no AI attribution lines.
- Branches (ADR 0003): `main` is released and stable; `develop`, created from `main`, is the integration branch. They are the only long-lived branches. Every change and delivery is a short-lived branch cut from `develop` (`feat/<change>-<delivery>`, `docs/...`, `chore/...`).
- Integration: after a delivery's verify PASS the coordinator merges its branch into `develop` locally with `git merge --no-ff`, runs the gates once on `develop` (skip cargo when no Rust or Cargo files changed), pushes `develop` and deletes the branch (locally, and remotely if pushed). No PR per delivery or change; deliveries of one change merge in order. A FAIL never reaches `develop`.
- The only PR is `develop` -> `main`, opened only when the owner asks or at a release-worthy point (a release or tag, a completed product document, a milestone the owner names); the coordinator may suggest one in one line at those points and opens it only with the owner's OK. The owner reviews and merges it with a merge commit. Releases and tags are cut from `main` after that merge; then the coordinator syncs `develop` (`git pull`, `git merge --ff-only origin/main`, else a plain merge) and pushes it.
- State lives in the change folder when there is one, in `.odd/state.json` and in git, not in an external memory tool.

## Code rules

- Rust edition 2024, toolchain 1.93.1, `unsafe_code` denied (`#![forbid(unsafe_code)]` per crate), no `#[allow]` to silence lints.
- Hexagonal layering enforced by the CI layer table (V5); domain crates use `BTreeMap`/`BTreeSet`, never `HashMap`/`HashSet` (clippy ban).
- Permissive third-party licenses only (`deny.toml`); the product itself is proprietary.
- Action pins in CI are 40-hex SHAs.
- Artifacts (code, comments, docs, specs, commits) are in English; user-facing UI text is Spanish and English (D-08).

## OpenSpec in this repo

Layout (`openspec/config.yaml` has the test, lint and format commands and per-phase rules; read it first). A change folder exists only when "ODD lite" calls for one:

```
openspec/config.yaml                       project context, strict_tdd, commands, rules
openspec/specs/<capability>/spec.md        main specs (source of truth; filled by archive)
openspec/changes/<change>/
  proposal.md  design.md  tasks.md         optional; existing changes already have them
  specs/<capability>/spec.md               delta specs for this change (none in the fast lane or light mode)
  apply-progress.md                        written by tdd-apply (existing name, keep it); starts with the `Flow:` line, or with the block written by the coordinator: Light spec (light mode) or Slice card (fast lane: product IDs, scope, non-goals, deliveries, plan, Flow line)
  verify-report.md                         written by verify
  verification.json                        written by verify beside the report (pilot, every depth above V0; see ADR 0001)
openspec/changes/archive/<date>-<change>/  closed changes
```

Spec format: RFC 2119 keywords, scenarios as Given/When/Then, each requirement tagged with its delivery (`D1`, `D2a`...) and each scenario with its type (unit, property, integration, command, manual). Product-facing docs are Spanish; spec files in changes follow the language already used in that change.

Delta specs use `## ADDED Requirements`, `## MODIFIED Requirements` (replaces the requirement of the same name), `## REMOVED Requirements` (needs a Reason and a Migration note) and `## RENAMED Requirements`.

Archive merge (the only step that edits main specs; only for changes with delta specs; fast-lane changes have none, and `openspec/specs/` stays as the record of past changes):
- No main spec for the capability yet: copy the delta with `cp`, then `diff` it.
- Main spec exists: apply the delta block by block (append ADDED, replace MODIFIED, delete REMOVED), keep every other requirement untouched, then show a `diff` of the result.
- A delta that targets a base spec still inside an unarchived change (see the note at the top of `domain-color/specs/workspace-tooling` and `domain-doc/specs/workspace-tooling`) means archive the older change first: `workspace-scaffold`, then `domain-color`, then `domain-doc`.
- Warn before any destructive delta (REMOVED, or MODIFIED that drops scenarios).

`workspace-scaffold` and `domain-color` are merged to `main` but not archived yet; archive them with the flow above before `domain-doc`.

## Skills

Rule: no absolute or user paths in tracked files. Skills are referenced by NAME only; global skills are registered and resolved through `.atl/skill-registry.md` (gitignored), created and read with the local `skill-registry` skill (`.agents/skills/skill-registry/`; `.claude/skills` is a symlink to `.agents/skills`).

Registry structure: header, "Sources scanned" (project `.claude/skills`, `.agents/skills`, then user skill dirs under `$HOME`), "Contract", "Skills" table (`Skill | Trigger / description | Scope | Path`; project wins on duplicate names) and "Loading protocol". Create or refresh it with `.agents/skills/skill-registry/refresh.sh`; a missing name is reported as unavailable, never guessed.

Skills are read, not invoked: subagents cannot call the Skill tool. The coordinator resolves the names below to paths with `refresh.sh resolve NAME...` before delegating and passes those paths in the prompt; each agent reads the `SKILL.md` before working.

| When | Skill |
|---|---|
| UI code, theme tokens, colors, fonts, logo/icon, microcopy, splash, any visual rule | `gafira-brand` |
| Splitting work into commits, tests and docs with code | `work-unit-commits` |
| Committing, branching, pushing, PRs, releases | `git-workflow-and-versioning` |
| Delivery over ~400 lines, review slices | `chained-pr` |
| Preparing the `develop` -> `main` PR (coordinator opens it, owner merges) | `branch-pr` |
| Creating, promoting or archiving product specs; deriving OpenSpec changes | `product-specs` |
| Writing specs, guides, README, review-facing docs | `cognitive-doc-design` |
| PR and issue comments | `comment-writer` |
| Creating, refreshing or reading the skill registry | `skill-registry` |
| Updating, finding a chapter in, or building the PDFs of the technical manual | `technical-manual` |

No Rust-specific skill is installed; the Rust rules are the ones above and in `openspec/config.yaml`. The user-level `CLAUDE.md` still applies to the main conversation (language, no AI attribution in commits).
