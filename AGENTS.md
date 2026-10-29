# Gafira: development methodology

Gafira is a local-first design, image and video editor written in Rust (Cargo workspace of `gafira-*` crates). Product and architecture live in `docs/specs/` (see its `README.md` for the status table): only `00-decisiones.md` (owner decisions) and the document in progress sit there; a completed document moves with `git mv` to `docs/specs/archive/` (never `00-decisiones.md`) before the next one is promoted; the rest (01 product and quality, 03 visual design, 04 image/export, 05 video/distribution, 06 license) wait in `docs/specs/backlog/`. When the document in progress is completed, the coordinator promotes the next one from the backlog (`git mv` into `docs/specs/`, in its own commit) and keeps the status table current. Owner decisions in `00-decisiones.md` are not reopened without the owner.

## Flow (lite SDD)

Specs already say what to build, so there is no separate proposal, design or tasks phase by default. Two optional upstream stages exist: `specify` when the spec is missing or ambiguous, and `architect` when a real design decision or a delivery breakdown is needed.

Fast lane (default, `docs/adr/0002-fast-lane.md`): when the owner asks to execute a product spec in `docs/specs/` (or a section or ID range of it), the coordinator writes a Slice card and `tdd-apply` works straight from the product requirement IDs; no proposal, design or delta specs.

```
(specify)  ->  spec  ->  (architect)  ->  (explorer, only when a claim is risky)  ->  tdd-apply  ->  verify  ->  (mutants T4 per touched crate)  ->  archive
```

Stages in parentheses are skipped when the spec is already clear and complete. `spec` is the product spec plus the Slice card (fast lane), the Light spec block (light) or delta specs. `mutants` runs per the `mutation` value of the Flow line: once per touched crate at change close by default, per delivery only with `mutation: delivery`, never with `off`.

### Flow modes and packs

Decisions: `docs/adr/0001-flow-modes-and-packs.md` (accepted, partly superseded) and `docs/adr/0002-fast-lane.md` (accepted). The coordinator classifies each request: hard floors first (owner decision needed: stop; new V5 row, dependency edge not already stated in `02`, format version or a cross-cutting design choice the spec leaves open, or parallel deliveries wanted: full; public API or format change, new rule on critical code, ambiguity, or a new crate that `02` already describes: standard; forecast x1.4 above ~300 lines: standard), then the pack default, then the owner's explicit override (`decided-by: owner`). It records `Flow: <mode> | pack | mutation: off|archive|delivery | tier | verify | decided-by | reason` in `apply-progress.md` and every change as `Flow-change:`.

| Mode | Runs | Spec | Gates |
|---|---|---|---|
| light | coordinator, `tdd-apply`, `verify` (docs and tooling packs: one writer, `verify` V0/V1) | Light spec block (acceptance criteria) written by the coordinator at the top of `apply-progress.md` | G0, G3, G4 |
| standard | adds `mutants` (per `mutation`), `archive`; optional `explorer`; `specifier` only on ambiguity or when there is no product spec | product spec IDs + Slice card (fast lane), or delta specs | G0, G1 (only if `specifier` ran), G3, G4, G5 |
| full | adds `architect` (design, delivery table) | as standard, plus `design.md` | G0 to G5 |

G0 (acceptance criteria committed before the first implementation commit) applies to every mode. In light mode the coordinator writes only the Light spec block, and in the fast lane only the Slice card; never code, tests, other specs or ADRs; `verify` checks that commit touches only that file and the implementer did not change it. Verify depth defaults: V1 for docs and tooling packs, V2 for code; the docs-only and manual packs may use V0 (coordinator runs deterministic checks, no agent); V3 (per-commit gates and the full matrix) only on owner request.

Mutation (`mutation:` in the Flow line): `archive` (default) runs no per-delivery mutants and one T4 per touched crate when the change closes, before archive; `delivery` runs the per-delivery tiers (T2/T3, `--iterate` on test-only fix rounds) plus T4 at close, only on owner request or when the coordinator flags a new rule on critical code; `off` runs no `cargo-mutants` (verify's hand mutants at V2 still apply). "Skip mutation" from the owner means `off`.

Delivery size: ~700 lines per delivery stays a signal to stop and ask. One change may run several deliveries serially without `architect` when the Slice card lists them; `architect` runs only for an open design decision or a dependency table with parallel deliveries.

Packs: docs, manual, agent/skill definitions, bumps, bug fix, refactor, new function, new capability, spec-only, release, spike, CI hotfix, test-only round (table in ADR 0001, defaults amended by ADR 0002, and in `coordinator.md`). Agent and skill definitions are written by a general-purpose agent in a worktree, delegated by the coordinator; the owner reviews the PR. Light and fast-lane runs are archived without delta merge (archive moves the folder and, for a fast-lane change, updates the `docs/specs/README.md` status table with the covered IDs).

Feedback loop: the coordinator keeps control of the whole cycle; every agent returns to it and none calls another. `verify` classifies each finding, and the coordinator routes it back to the matching stage:

```
specify -> architect -> explorer -> tdd-apply -> mutants + verify -> archive
   ^          ^            ^           ^               |
   |          |            |           +-- implementation (code/test gap, gate, mutant, size)
   |          |            +-------------- claim (risky unproven claim)
   |          +--------------------------- architecture (design/ADR error, undeliverable as designed)
   +-------------------------------------- ambiguity (missing/contradictory requirement, spec gap)
```

After two consecutive FAILs of the same class on one delivery the coordinator stops and asks the owner.

| Step | Agent (`.agents/agents/`) | Output |
|---|---|---|
| Turn an owner request into specs (scope, requirements, Given/When/Then scenarios, acceptance criteria, edge cases, non-goals, open questions); defines WHAT, never HOW | `specifier` (Opus) | fast lane: only on a reported ambiguity, and it fixes the product spec in `docs/specs/`; otherwise delta specs in `openspec/changes/<change>/specs/` (no product spec, or the owner asks) |
| Decide HOW when a spec leaves a design decision open; analyse the code; break the work into deliveries (~700 lines) with a dependency table (parallel or serial) | `architect` (Opus) | `openspec/changes/<change>/design.md` (proposal/tasks only if needed), cross-cutting ADRs in `docs/adr/` |
| Measure risky claims in `/tmp` (serde limits, clippy per commit, CI regexes, delivery prefixes) and cross-check numbers across docs | `explorer` (Sonnet) | `openspec/changes/<change>/spike.md` with command, output, verdict |
| Build one delivery with strict TDD | `tdd-apply` (Sonnet) | commits + `openspec/changes/<change>/apply-progress.md` |
| Mutation-test per the Flow line's `mutation` (default: whole crate at change close; per delivery only with `delivery`); report surviving mutants (read-only) | `mutants` (Sonnet) | report in the session scratchpad, outside the repo |
| Check gates and every requirement/scenario | `verify` (Opus) | `openspec/changes/<change>/verify-report.md`, PASS or FAIL |
| Close the change (PASS and all deliveries done) | `archive` (Haiku) | change moved to `openspec/changes/archive/`; fast lane: status table in `docs/specs/README.md` updated with the covered IDs; delta specs (when present) merged into `openspec/specs/` |
| Update, extend or regenerate the technical manual (`docs/manuals/tecnico/`), English first with a Spanish mirror | `manual-writer` (Sonnet) | edited chapters in both languages, `manual.py check` and `build` evidence, `docs(manual)` commits |

When deliveries remain, go back to `tdd-apply`. On FAIL, the coordinator routes each finding by its `class:` (see the feedback loop above and `.agents/agents/coordinator.md`).

`coordinator` (Opus) runs the whole flow: it reads the repo state, picks the next step, delegates to the agents above (one writer at a time), rejects `done` without evidence and stops for owner decisions and merges. Subagents cannot launch other subagents, so the coordinator must be the main-thread agent: start the session with `claude --agent coordinator` (supported by the installed Claude Code CLI). If an older CLI lacks the flag, ask the main session to follow `.agents/agents/coordinator.md`. `mutants` (Sonnet) runs `cargo-mutants`: by default once per touched crate at change close (T4), or per delivery in parallel with `verify` with `mutation: delivery`. `manual-writer` (Sonnet) sits outside the spec flow: the coordinator delegates to it whenever the owner asks to change the technical manual; it needs the `technical-manual` and `skill-registry` skills and stops if `technical-manual` is unavailable. `.claude/agents` is a symlink to `.agents/agents`.

A design note (`architect`) is written only when a spec leaves a real decision open, a change needs a delivery breakdown or parallelisation, or code must be analysed before choosing an approach, and only for that decision. `specifier` and `architect` cannot launch agents: they list open questions and risky claims, and the coordinator asks the owner (one question at a time, with a recommendation) or runs `explorer`. ADR convention: cross-cutting decisions go in `docs/adr/NNNN-short-title.md` (index and template in `docs/adr/README.md`), change-scoped decisions in the change's `design.md`, owner decisions stay in `docs/specs/00-decisiones.md` and are never reopened by the architect. If a test or CI claim is risky, `explorer` settles it with a real experiment before specs or code rely on it; refuted claims are fixed in the specs first.

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
- Mutation tiers (`mutants`; `verify` fails on a missed or timeout mutant in critical code and documents the rest; no score target): T0 none (no Rust touched); T1 hand mutants by `verify` only (tiny non-critical code); T2 `--in-diff` (diff-scoped); T3 `--file` per changed non-test source file (per-delivery tier for standard mode with `mutation: delivery`, raised to it when a delivery edits an existing critical file; `--iterate` only on test-only fix rounds with the same file set, never on the closing PASS round); T4 whole crate at change close, before archive, for every crate the change touched. T2 and T3 run only with `mutation: delivery`; `mutation: archive` (default) runs T4 alone; `mutation: off` runs nothing. Never share one `CARGO_TARGET_DIR` across mutated trees.
- Flags: T2 `cargo mutants -p <crates> --in-diff <diff> --jobs 8 -o <scratchpad dir>`; T3 `cargo mutants -p <crate> --file <file> --jobs 8 -o <scratchpad dir>`; T4 `cargo mutants -p <crate> --jobs 8 -o <scratchpad dir>`. Default `cargo test` runner (nextest skips doctests), no `RUSTFLAGS=-D warnings`. `#[mutants::skip]` is forbidden (it needs a dependency); exclude through `.cargo/mutants.toml` with a reason. `--locked` comes from `.cargo/mutants.toml` (`additional_cargo_test_args`); do not pass it again (cargo rejects a duplicate). The `mutants` cargo profile (`Cargo.toml`, `inherits = "test"`, `debug = "none"`) is set by `profile = "mutants"` in `.cargo/mutants.toml`; do not pass `--profile`. A fast linker (mold, wild) is optional and not installed.
- `cargo-mutants` is an optional dev tool, like `cargo-deny`; it is not a CI gate.

## Delivery rules

- Measure each delivery with `git diff --shortstat`; the ceiling is ~700 lines, and above it needs the owner's explicit OK. Estimates undershoot (measured x1.4 to x1.9 over forecast, mostly tests): apply x1.4 to forecasts; a forecast x1.4 above ~300 lines leaves light mode. At most three verify rounds per delivery, and a stop after two consecutive FAILs of the same class. These are signals for the coordinator to stop and ask, not hard blocks.
- Stacked PRs cut at commit boundaries; each prefix passes the gates alone.
- Conventional commits, no AI attribution lines.
- The coordinator may push branches and open PRs after verify PASS; merges stay with the owner. Each new delivery targets `main` (or the previous unmerged delivery branch), never a long-lived integration branch.
- Integration: trunk-based, short-lived branches. After each delivery's verify PASS the coordinator pushes that branch and opens its PR against `main` (or the previous unmerged delivery branch) right away, instead of accumulating many stacked PRs. The owner merges promptly, in order, bottom-up, with merge commits. Never merge a PR whose base is another delivery branch after that base has been merged: retarget it to `main` first.
- State lives in the change folder and git, not in an external memory tool.

## Code rules

- Rust edition 2024, toolchain 1.93.1, `unsafe_code` denied (`#![forbid(unsafe_code)]` per crate), no `#[allow]` to silence lints.
- Hexagonal layering enforced by the CI layer table (V5); domain crates use `BTreeMap`/`BTreeSet`, never `HashMap`/`HashSet` (clippy ban).
- Permissive third-party licenses only (`deny.toml`); the product itself is proprietary.
- Action pins in CI are 40-hex SHAs.
- Artifacts (code, comments, docs, specs, commits) are in English; user-facing UI text is Spanish and English (D-08).

## OpenSpec in this repo

Layout (`openspec/config.yaml` has the test, lint and format commands and per-phase rules; read it first):

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
| Delivery over ~400 lines, stacked PRs, review slices | `chained-pr` |
| Preparing a PR (coordinator opens it, owner merges) | `branch-pr` |
| Creating, promoting or archiving product specs; deriving OpenSpec changes | `product-specs` |
| Writing specs, guides, README, review-facing docs | `cognitive-doc-design` |
| PR and issue comments | `comment-writer` |
| Creating, refreshing or reading the skill registry | `skill-registry` |
| Updating, finding a chapter in, or building the PDFs of the technical manual | `technical-manual` |

No Rust-specific skill is installed; the Rust rules are the ones above and in `openspec/config.yaml`. The user-level `CLAUDE.md` still applies to the main conversation (language, no AI attribution in commits).
