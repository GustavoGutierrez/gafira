<p align="center">
  <img src="assets/gafira-logo-symbol.svg" alt="Gafira" width="96">
</p>

# Gafira

Gafira is a local-first design, image and video editor written in Rust. It is proprietary software.

Status: `gafira-geom`, `gafira-color` and the document model (`gafira-doc`) are in place; edit history is next. There is no GUI yet.

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
| `assets/` | Logo, splash screen and other design assets |
| `.agents/` | Agents and skills for Claude Code; `.claude/agents` and `.claude/skills` are symlinks to it |

## Development workflow

Development uses Claude Code with the repo's `coordinator` agent. From the repository root:

```
claude --agent coordinator
```

Then ask it to execute, continue, verify or archive a spec, change or delivery.

The coordinator runs the lite SDD flow and delegates each step to a repo agent:

```
spec -> explorer (only for risky claims) -> tdd-apply -> mutants + verify -> archive
```

- One writer at a time; each delivery is built with strict TDD and one commit per work unit.
- `mutants` (cargo-mutants, optional dev tool) lists surviving mutants on changed lines.
- `verify` checks the gates and every scenario; on FAIL the work goes back to `tdd-apply`.
- The coordinator rejects `done` without evidence.
- The coordinator may push branches and open PRs after verify PASS; the owner does every merge. Each delivery targets `main` (or the previous unmerged delivery branch), never a long-lived integration branch.

First-time setup: generate the gitignored skill registry (`.atl/skill-registry.md`):

```
.agents/skills/skill-registry/refresh.sh
```

The full methodology (flow, TDD, delivery rules, OpenSpec layout) is in `AGENTS.md`.

## Assets

Audio under `assets/musics/` is kept out of git (owner decision D-10 in `docs/specs/00-decisiones.md`). Keep local copies there; do not commit them.

## License

Proprietary; see `LICENSE`. The full license agreement and privacy policy are written in the final licensing phase (`docs/specs/backlog/06-licencia-y-activacion.md`). Third-party crates must use permissive licenses only (`deny.toml`).
