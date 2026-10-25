# Gafira: development methodology

Gafira is a local-first design, image and video editor written in Rust (Cargo workspace of `gafira-*` crates). Product and architecture live in `docs/specs/` (see its `README.md` for the status table): only `00-decisiones.md` (owner decisions) and the document in progress sit there; a completed document moves with `git mv` to `docs/specs/archive/` (never `00-decisiones.md`) before the next one is promoted; the rest (01 product and quality, 03 visual design, 04 image/export, 05 video/distribution, 06 license) wait in `docs/specs/backlog/`. When the document in progress is completed, the coordinator promotes the next one from the backlog (`git mv` into `docs/specs/`, in its own commit) and keeps the status table current. Owner decisions in `00-decisiones.md` are not reopened without the owner.

## Flow (lite SDD)

Specs already say what to build, so there is no separate proposal, design or tasks phase.

```
spec  ->  (explorer, only when a claim is risky)  ->  tdd-apply  ->  mutants + verify  ->  archive
```

| Step | Agent (`.agents/agents/`) | Output |
|---|---|---|
| Measure risky claims in `/tmp` (serde limits, clippy per commit, CI regexes, delivery prefixes) and cross-check numbers across docs | `explorer` (Sonnet) | `openspec/changes/<change>/spike.md` with command, output, verdict |
| Build one delivery with strict TDD | `tdd-apply` (Sonnet) | commits + `openspec/changes/<change>/apply-progress.md` |
| Mutation-test the changed lines; report surviving mutants (read-only) | `mutants` (Sonnet) | report in the session scratchpad, outside the repo |
| Check gates and every requirement/scenario | `verify` (Opus) | `openspec/changes/<change>/verify-report.md`, PASS or FAIL |
| Close the change (PASS and all deliveries done) | `archive` (Haiku) | delta specs merged into `openspec/specs/`, change moved to `openspec/changes/archive/` |

On FAIL, or when deliveries remain, go back to `tdd-apply`.

`coordinator` (Opus) runs the whole flow: it reads the repo state, picks the next step, delegates to the agents above (one writer at a time), rejects `done` without evidence and stops for owner decisions and merges. Subagents cannot launch other subagents, so the coordinator must be the main-thread agent: start the session with `claude --agent coordinator` (supported by the installed Claude Code CLI). If an older CLI lacks the flag, ask the main session to follow `.agents/agents/coordinator.md`. `mutants` (Sonnet) runs `cargo-mutants` on changed lines in parallel with `verify`. `.claude/agents` is a symlink to `.agents/agents`.

A short design note is written only when a spec leaves a real decision open, and only for that decision. If a test or CI claim is risky, `explorer` settles it with a real experiment before specs or code rely on it; refuted claims are fixed in the specs first.

## Strict TDD

RED (write the test, run it, see it fail) -> GREEN (minimal code) -> REFACTOR (tests green). One commit per reviewable work unit. Every commit must pass alone:

```
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Testing and mutation policy

- Tests cover behavior of the public contract, not implementation (see `tdd-apply`).
- Critical code: validation, JSON load/save, ids and arena, transforms, size/DPI limits, any spec rule with an invariant. Non-critical: `Display`/`Error` text unless the spec fixes it, trivial accessors, test helpers.
- `mutants` runs `cargo-mutants` on the delivery diff; `verify` fails on a missed or timeout mutant in critical code and documents the rest. No score target. Archive first requests a whole-crate run.
- Flags: `cargo mutants -p <crates> --in-diff <diff> --jobs 4 -o <scratchpad dir>`. Default `cargo test` runner (nextest skips doctests), no `RUSTFLAGS=-D warnings`. `#[mutants::skip]` is forbidden (it needs a dependency); exclude through `.cargo/mutants.toml` with a reason. `--locked` comes from `.cargo/mutants.toml` (`additional_cargo_test_args`); do not pass it again (cargo rejects a duplicate).
- `cargo-mutants` is an optional dev tool, like `cargo-deny`; it is not a CI gate.

## Delivery rules

- Measure each delivery with `git diff --shortstat`; split when it exceeds ~700 lines. Estimates undershoot (measured x1.4 to x1.9 over forecast, mostly tests).
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
  specs/<capability>/spec.md               delta specs for this change
  apply-progress.md                        written by tdd-apply (existing name, keep it)
  verify-report.md                         written by verify
openspec/changes/archive/<date>-<change>/  closed changes
```

Spec format: RFC 2119 keywords, scenarios as Given/When/Then, each requirement tagged with its delivery (`D1`, `D2a`...) and each scenario with its type (unit, property, integration, command, manual). Product-facing docs are Spanish; spec files in changes follow the language already used in that change.

Delta specs use `## ADDED Requirements`, `## MODIFIED Requirements` (replaces the requirement of the same name), `## REMOVED Requirements` (needs a Reason and a Migration note) and `## RENAMED Requirements`.

Archive merge (the only step that edits main specs):
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

No Rust-specific skill is installed; the Rust rules are the ones above and in `openspec/config.yaml`. The user-level `CLAUDE.md` still applies to the main conversation (language, no AI attribution in commits).
