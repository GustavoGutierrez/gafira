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

## Icons and Linux install

The application and `.gaf` icons are generated from the brand SVGs (`specs/done/046-application-icon.md`):

```
cargo xtask gen-icons            # writes assets/app-icon/hicolor/
cargo xtask gen-icons --check    # fails if a tracked icon is stale
```

Per-user install on Linux (builds nothing; `--uninstall` removes it):

```
cargo build --release --locked -p gafira
packaging/linux/install.sh [path/to/gafira]
```

## Packages and releases

| System | Command | Result in `dist/` |
|---|---|---|
| Linux | `packaging/linux/build-packages.sh` | `.deb` and AppImage |
| Windows (or Linux with `mingw-w64` and `nsis`) | `packaging/windows/build.sh` | installer and portable `.zip` |
| macOS | `packaging/macos/build.sh` | `.dmg`, not notarized |

A tag `v<version>` on `main` builds Linux and Windows in `.github/workflows/release.yml` and publishes them, with the notes of `CHANGELOG.md` (spec 090). Run that workflow by hand on `develop` first: it checks the packages and leaves the cache the tag uses. macOS is built only when asked for.

## Repository layout

| Path | Content |
|---|---|
| `crates/` | `gafira-*` library crates (hexagonal layering enforced in CI) |
| `apps/` | Binaries (`gafira-cli`; the GUI app comes later) |
| `specs/` | Numbered specs (in Spanish): active in `specs/`, not started in `specs/backlog/`, closed in `specs/done/` (index and next free ID in `specs/README.md`). Product reference documents `00-..07-*.md` (`00-decisiones.md` holds the owner decisions); the earlier flow's changes, history and capabilities are in `specs/done/legacy/` |
| `assets/` | Splash screen, design tokens (`theme/gafira.tokens.json`, loaded by `gafira-theme`), UI catalogs (`i18n/es.toml`, `i18n/en.toml`, compiled into `gafira-i18n`), fonts and other design assets (logos live in `docs/manuals/marca/assets/`) |
| `xtask/` | Repository tasks (`cargo xtask gen-icons`) |
| `packaging/` | Desktop integration (`linux/`: desktop entry, `.gaf` MIME type, `install.sh`) |
| `.agents/` | Skills and the `manual-writer` agent, the single source for every runtime; `.claude/`, `.opencode/` and `.pi/` link to it (`python3 tools/sync_agents.py`) |

## Development workflow

The agent (Claude Code or another) implements specs directly, following `AGENTS.md` and the `gafira-specs` skill:

- Specs have three-digit IDs and a status banner (`backlog`, `in-progress`, `done`); `specs/README.md` is the index.
- TDD for critical logic (document model, serde and validation, edit commands and history, time, color, geometry and path ops, limits, format parsing); integration tests per feature for the rest; full suite and gates at spec close.
- Work happens on a short-lived branch cut from `develop` and merges into `develop` once its gates pass; the only PR is `develop` -> `main`, opened only when the owner asks or at a release-worthy point, and the owner reviews and merges it (`docs/adr/0003-develop-integration-branch.md`, `docs/adr/0018-simple-spec-flow.md`).
- `cargo-mutants` is an optional dev tool for critical logic.

First-time setup: generate the gitignored skill registry (`.atl/skill-registry.md`):

```
.agents/skills/skill-registry/refresh.sh
```

The full methodology (rules, TDD, gates, branches) is in `AGENTS.md`.

## Assets

Audio under `assets/musics/` is kept out of git (owner decision D-10 in `specs/00-decisiones.md`). Keep local copies there; do not commit them.

## License

Proprietary; see `LICENSE`. The full license agreement and privacy policy are written in the final licensing phase (`specs/backlog/06-licencia-y-activacion.md`). Third-party crates must use permissive licenses only (`deny.toml`).
