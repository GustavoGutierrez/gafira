<p align="center">
  <!--<img src="docs/manuals/marca/assets/gafira-logo-primary.png#gh-light-mode-only" alt="Gafira" width="240"> #gh-dark-mode-only -->
  <img src="docs/manuals/marca/assets/gafira-logo-dark-background.png" alt="Gafira" width="240">
</p>

<p align="center">
  <a href="https://github.com/GustavoGutierrez/gafira/releases"><img src="https://img.shields.io/github/v/release/GustavoGutierrez/gafira?include_prereleases&sort=semver&label=release&color=3FBFA9" alt="Latest release"></a>
  <a href="https://github.com/GustavoGutierrez/gafira/releases"><img src="https://img.shields.io/github/downloads/GustavoGutierrez/gafira/total?color=3FBFA9" alt="Downloads"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-18243B" alt="License: GPL-3.0-or-later"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/Rust-1.93.1-18243B?logo=rust" alt="Rust 1.93.1"></a>
  <br>
  <img src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-18243B" alt="Platforms: Windows, macOS, Linux">
  <img src="https://img.shields.io/badge/languages-Espa%C3%B1ol%20%7C%20English-18243B" alt="Languages: Spanish, English">
  <a href="https://gafira.com/en/"><img src="https://img.shields.io/badge/website-gafira.com-FF6B57" alt="Website: gafira.com"></a>
  <a href="https://gafira.com/en/#support"><img src="https://img.shields.io/badge/support-full%20illustration%20packs-FF6B57?logo=gumroad&logoColor=white" alt="Support Gafira: full illustration packs"></a>
</p>

<p align="center"><b>English</b> · <a href="README.es.md">Español</a></p>

# Gafira

**Haz visibles tus ideas. Bring your ideas to life.**

Gafira is a local-first graphic design app for everyone and for content creators, written in Rust (egui). Pick a template, make it yours and share it. It runs on Windows, macOS and Linux, in Spanish and English. It is free and open source software under the GPL, version 3 or later.

Current version: `0.2.0` (see `CHANGELOG.md`).

Website: <https://gafira.com> (Spanish) · <https://gafira.com/en/> (English). Its source is in `website/`.

## Preview

<p align="center">
  <img src="assets/splash_screen.jpg" alt="Gafira splash screen" width="640">
</p>

## What it does

- **Start:** Home with search, recent designs and 75 templates per language (social media, presentations, prints, invitations, documents, web and email), or a design at your own size.
- **Design:** 215 shapes in 17 sections (lines, polygons, stars, arrows, diagrams, speech bubbles, clouds, quotes), lines that connect to shapes, text with 33 bundled font families, curved text, text inside shapes and text effects, freehand drawing with 16 brushes, frames and photo collage, tables (Excel and CSV import), charts and QR codes.
- **Colour:** one colour field everywhere, with an eyedropper and loupe, solid colours and gradients, design colours and preset palettes; page backgrounds with colour, gradient, pattern or image.
- **Images:** non-destructive adjustments, filters, shadows and crops with shapes; background removal, upscaling and the magic eraser with local models (CPU or GPU); free photos and illustrations from the Elements panel.
- **3D objects:** import `.glb` and `.obj` models and place them like any element; double-click to rotate, move, scale and orbit them with on-canvas gizmos, quick views and lighting presets, then apply. They are drawn on the CPU, saved inside the `.gaf` and exported as the view their camera sees.
- **Organise:** Layers with drag and drop, lock and hide; guides, snapping, rulers, align and distribute; versions inside the `.gaf` file and crash recovery.
- **Brand and packages:** brand kits (logos, colours, text styles, fonts) shared as `.gafkit`; asset packages (`.gafpack`) installed from Home and uninstalled from Settings or from their panel.
- **Export:** PNG, JPEG, WebP, AVIF, SVG, vector PDF (CMYK, password) and editable PowerPoint, in the background.

## Download

Installers of each version are on the [GitHub releases](https://github.com/GustavoGutierrez/gafira/releases) and at <https://gafira.com/en/#download>: Windows installer and portable `.zip`, macOS `.dmg` (Apple Silicon, not notarised), Linux `.deb` and AppImage.

## Support the project

Gafira is free. If you would like to support its development, buy one of the [full illustration packs](https://gafira.com/en/#support): you get every illustration in it, and you help us keep building Gafira. You can also contribute code, translations or bug reports (see `CONTRIBUTING.md`).

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

A tag `v<version>` on `main` builds Linux and Windows in `.github/workflows/release.yml` and publishes them, with the notes of `CHANGELOG.md` (spec 090). Run that workflow by hand on `develop` first: it checks the packages and leaves the cache the tag uses. macOS (`macos-15` runner) is built only when the workflow is run by hand on the tag with `macos: true`; it adds the `.dmg` to the same release. After a release, rebuild and deploy the website so its download links point to the new version.

## Repository layout

| Path | Content |
|---|---|
| `crates/` | `gafira-*` library crates (hexagonal layering enforced in CI); the desktop UI is `gafira-ui` |
| `apps/` | Binaries: `gafira` (the desktop app) and `gafira-cli` |
| `website/` | The landing page at gafira.com (Astro, static, Cloudflare Workers); see `website/README.md` |
| `specs/` | Numbered specs (in Spanish): active in `specs/`, not started in `specs/backlog/`, closed in `specs/done/` (index and next free ID in `specs/README.md`). Product reference documents `00-..07-*.md` (`00-decisiones.md` holds the owner decisions); the earlier flow's changes, history and capabilities are in `specs/done/legacy/` |
| `assets/` | Splash screen, design tokens (`theme/gafira.tokens.json`, loaded by `gafira-theme`), UI catalogs (`i18n/es.toml`, `i18n/en.toml`, compiled into `gafira-i18n`), fonts and other design assets (logos live in `docs/manuals/marca/assets/`) |
| `xtask/` | Repository tasks (`cargo xtask gen-icons`) |
| `packaging/` | Packages and desktop integration: `linux/` (`.deb`, AppImage, desktop entry, `.gaf` MIME type, `install.sh`), `windows/` (NSIS installer, `.zip`), `macos/` (`.dmg`) |
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

Gafira is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version (`GPL-3.0-or-later`; see `LICENSE`). It comes with no warranty.

The Gafira name, logo and app icon are not covered by that license; see `TRADEMARKS.md`. Illustration packs (`.gafpack`) carry their own license inside each pack. Third-party crates must use permissive licenses (`deny.toml`).

Want to help? Read `CONTRIBUTING.md`.
