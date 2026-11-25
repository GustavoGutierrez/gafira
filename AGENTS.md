# Gafira: development methodology

Gafira is a local-first design, image and video editor written in Rust (Cargo workspace of `gafira-*` crates under `crates/`, binaries under `apps/`). The agent implements specs directly, with tests and the gates below; there are no subagents in the spec flow (ADR 0018).

## Specs

- Lifecycle, numbering, status banner, Definition of Ready and TDD rules: skill `gafira-specs` (`.agents/skills/gafira-specs/SKILL.md`). Index and next free ID: `specs/README.md`.
- `specs/` holds the active specs, `specs/backlog/` the ones not started, `specs/done/` the closed ones. The spec's status banner and `specs/README.md` are the state; git is the history.
- `specs/00-..07-*.md` are product reference documents (owner decisions, product, architecture, features, open debt), not lifecycle specs. Owner decisions in `specs/00-decisiones.md` are not reopened without the owner.
- `specs/done/legacy/` keeps the record of the earlier multi-agent flow (changes, history, capabilities, old index). Read it, do not rewrite it.

## Rules

Code:

- Rust edition 2024, toolchain 1.93.1 (`rust-toolchain.toml`), `#![forbid(unsafe_code)]` in every crate, no `#[allow]` to silence lints.
- Hexagonal layering enforced by the CI layer table (V5 in `.github/workflows/ci.yml`); domain crates use `BTreeMap`/`BTreeSet`, never `HashMap`/`HashSet` (clippy ban, V12). A new domain crate needs a V5 row and a `clippy.toml` copy.
- Permissive third-party licenses only (`deny.toml`); the product is open source under GPL-3.0-or-later (D-129, `LICENSE`; brand in `TRADEMARKS.md`), runs fully local and needs no external binaries.
- Action pins in CI are 40-hex SHAs.
- Targets: Windows (MSI), Linux x86_64, macOS 12+ aarch64.
- Language: code, comments, commits and generated artifacts in English; product specs in `specs/` are Spanish; user-facing UI text is Spanish and English (D-08); conversation with the owner in neutral Spanish.

Testing:

- TDD (red, green, refactor) only for critical logic: document model, JSON load/save and validation, ids and arena, edit commands and history (undo/redo, merge), time model and timecode, color conversions, geometry, transforms and path operations, animation interpolation, size/DPI and other limits, file format parsing, and any spec rule with an invariant. A bug fix in critical logic starts with a regression test.
- The rest is validated with integration tests per feature (each crate's `tests/`); there is no e2e layer and no coverage threshold.
- Tests cover the public contract and name the requirement ID they prove (`ARQ-nn`, `RF-nn`, `FD-nn`...) in their name or a one-line comment, so `git grep ARQ-nn` traces coverage.
- While working, scope the gates to the touched crate (`-p <crate>`); use `--workspace` when the change touches a crate others depend on, `Cargo.toml`, `Cargo.lock` or `ci.yml` (ADR 0017). The full suite runs once at spec close.
- `cargo-mutants` is optional, only for critical logic, and not a CI gate: `cargo mutants -p <crate> --jobs 8 -o <scratch dir>`. Settings live in `.cargo/mutants.toml` (it already passes `--locked` and the `mutants` profile; exclude equivalent mutants there with a reason, never `#[mutants::skip]`).

Delivery (ADR 0003):

- `main` is released and stable; `develop` is the integration branch. Work on a short-lived branch cut from `develop` (`feat/<spec>-...`, `fix/...`, `docs/...`, `chore/...`).
- When the work passes its gates, merge it into `develop` with `git merge --no-ff`, push `develop` and delete the branch.
- The only PR is `develop` -> `main`, opened only when the owner asks or with the owner's OK at a release-worthy point; the owner merges it. Releases and tags are cut from `main`, then `develop` is synced with `main`.
- Conventional commits, one reviewable work unit each, no AI attribution lines.
- Stop and ask the owner on a real block: an open product decision, a destructive or format-breaking change, or a change well beyond the spec's scope.

## Agent and skill sync

`.agents/` (agents and skills) is the single source of truth. `.claude/`, `.opencode/` and `.pi/` hold relative symlinks to `.agents/agents` and `.agents/skills`; never edit agents or skills there. After changing anything in `.agents/`, run `python3 tools/sync_agents.py`; its `--check` mode must pass before calling something done. Shared agent frontmatter is limited to `name`, `description`, `mode: subagent` and `subagent_mode`; models and other per-runtime settings go in each runtime's config (`.agents/README.md`).

## Before calling something done

1. `cargo fmt --check`.
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`.
3. `cargo test --workspace --locked`.
4. If `gafira-testkit` or `gafira-ports` changed: `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` and `cargo test -p gafira-testkit --features ports --locked`.
5. If dependencies changed: `cargo deny check` (when installed; CI runs it).
6. If a crate, its layering or `ci.yml` changed: the policy steps of `.github/workflows/ci.yml` (V5, V12, ARQ-107, V13, V14) pass locally.
7. If the technical manual changed: `python3 docs/manuals/tecnico/manual.py check` (and `--lang es`).
8. `python3 tools/sync_agents.py --check` exits 0.
9. Update `README.md` if what is implemented changes. Update the spec's status banner and `specs/README.md`; move it to `specs/done/` only with tests and evidence.

## Skills

No absolute or user paths in tracked files. Skills are referenced by name and resolved through `.atl/skill-registry.md` (gitignored), created and read with the local `skill-registry` skill (`.agents/skills/skill-registry/refresh.sh`); skills live in `.agents/skills`, linked from `.claude/`, `.opencode/` and `.pi/` (see "Agent and skill sync"). A missing name is reported as unavailable, never guessed.

| When | Skill |
|---|---|
| Creating, starting, closing or moving a spec; the specs index; TDD scope | `gafira-specs` |
| UI code, theme tokens, colors, fonts, logo/icon, microcopy, any visual rule | `gafira-brand` |
| Updating or building the technical manual (`docs/manuals/tecnico/`) | `technical-manual` (agent `manual-writer`) |
| Committing, branching, pushing, PRs, releases | `git-workflow-and-versioning` |
| Splitting work into commits, tests and docs with code | `work-unit-commits` |
| Preparing the `develop` -> `main` PR | `branch-pr` |
| Writing specs, guides, README, review-facing docs | `cognitive-doc-design` |
| Creating, refreshing or reading the skill registry | `skill-registry` |

`manual-writer` (`.agents/agents/`, mirrored for OpenCode and Pi) is the only repo agent; use it when the owner asks to change the technical manual.
