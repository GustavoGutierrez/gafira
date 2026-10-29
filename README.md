<p align="center">
  <!--<img src="docs/manuals/marca/assets/gafira-logo-primary.png#gh-light-mode-only" alt="Gafira" width="240"> #gh-dark-mode-only -->
  <img src="docs/manuals/marca/assets/gafira-logo-dark-background.png" alt="Gafira" width="240">
</p>

# Gafira

Gafira is a local-first design, image and video editor written in Rust. It is proprietary software.

Status: `gafira-geom`, `gafira-color` and the document model (`gafira-doc`) are in place; edit history is in progress (D1, the `Command` contract and `Transaction`, is done). There is no GUI yet.

Current version: `0.1.0-beta.1` (see `CHANGELOG.md`).

## Preview

<p align="center">
  <img src="assets/splash_screen.jpg" alt="Gafira splash screen" width="640">
</p>

## Requirements

- Rust toolchain pinned in `rust-toolchain.toml` (installed automatically by `rustup`).
- Optional: `cargo-deny`, to check third-party licenses against `deny.toml`.

## Build and gates

```
cargo build --workspace --locked
```

Every commit must pass these three gates:

```
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

## Repository layout

| Path | Content |
|---|---|
| `crates/` | `gafira-*` library crates (hexagonal layering enforced in CI) |
| `apps/` | Binaries (`gafira-cli`; the GUI app comes later) |
| `docs/specs/` | Product specs in progress (in Spanish) plus `00-decisiones.md` (owner decisions); completed ones go to `docs/specs/archive/`, the rest waits in `docs/specs/backlog/`, see `docs/specs/README.md` |
| `openspec/` | Specs and changes that drive development (`config.yaml`, `specs/`, `changes/`) |
| `assets/` | Splash screen and other design assets (logos live in `docs/manuals/marca/assets/`) |
| `.agents/` | Agents and skills for Claude Code; `.claude/agents` and `.claude/skills` are symlinks to it |

## Development workflow

Development uses Claude Code with the repo's `coordinator` agent. From the repository root:

```
claude --agent coordinator
```

Then ask it to execute, continue, verify or archive a spec, change or delivery.

The coordinator runs the lite SDD flow and delegates each step to a repo agent:

```
spec -> explorer (only for risky claims) -> tdd-apply -> verify -> mutants (whole crate, at change close) -> archive
```

- One writer at a time; each delivery is built with strict TDD and one commit per work unit.
- A product spec in `docs/specs/` runs directly from its requirement IDs (fast lane, `docs/adr/0002-fast-lane.md`); no proposal or delta specs.
- `mutants` (cargo-mutants, optional dev tool) lists surviving mutants; by default once per touched crate when the change closes (`mutation: archive` in the Flow line; `delivery` and `off` are the alternatives).
- `verify` checks the gates and every scenario; on FAIL the work goes back to `tdd-apply`.
- The coordinator rejects `done` without evidence.
- Each change and delivery is a short-lived branch cut from `develop`. After verify PASS the coordinator merges it into `develop`, runs the gates and pushes `develop`; the only PR is `develop` -> `main`, opened only when the owner asks or at a release-worthy point, and the owner reviews and merges it (`docs/adr/0003-develop-integration-branch.md`).

First-time setup: generate the gitignored skill registry (`.atl/skill-registry.md`):

```
.agents/skills/skill-registry/refresh.sh
```

The full methodology (flow, TDD, delivery rules, OpenSpec layout) is in `AGENTS.md`.

## Assets

Audio under `assets/musics/` is kept out of git (owner decision D-10 in `docs/specs/00-decisiones.md`). Keep local copies there; do not commit them.

## License

Proprietary; see `LICENSE`. The full license agreement and privacy policy are written in the final licensing phase (`docs/specs/backlog/06-licencia-y-activacion.md`). Third-party crates must use permissive licenses only (`deny.toml`).
