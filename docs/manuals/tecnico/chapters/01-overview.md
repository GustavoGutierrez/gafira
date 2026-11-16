# Overview

## Purpose of this manual

This manual describes how Gafira is built: its crates, the rules between them, the public API of the domain crates, the document format, the quality gates and the development method. It documents what exists in the repository today. Parts that are specified but not yet implemented are marked **Planned**.

Product intent (what and why) lives in `specs/`; this manual is the technical counterpart (how it is built). When this manual and a spec disagree, the spec and the owner decisions in `specs/00-decisiones.md` win, and this manual is corrected.

## What Gafira is

Gafira is a local-first desktop editor for graphic design, images and video, written in Rust. The workspace is a Cargo workspace of `gafira-*` crates plus binaries under `apps/`. It targets Windows x86_64, Linux x86_64 and macOS aarch64 (ARQ-03, D-05). The product is proprietary; every third-party dependency must have a permissive license (`deny.toml`).

## Current state

| Area | State |
|---|---|
| Workspace, CI, license gate | Implemented |
| Layers 0 and 1: `gafira-geom`, `gafira-color`, `gafira-time`, `gafira-anim`, `gafira-doc`, `gafira-edit` | Implemented |
| Layer 2: `gafira-pathops`, `gafira-text`, `gafira-layout`, `gafira-raster`, `gafira-audio`, `gafira-segment` | Implemented |
| Layers 3 and 4: `gafira-ports`, `gafira-render`, `gafira-media`, `gafira-render-cpu` | Implemented; the render graph emits fills and layers only, and the CPU backend returns an error for images, masks, effects and transitions (`crates/gafira-render-cpu/src/backend.rs`) |
| `gafira-testkit` (document generator, command property harness) | Implemented (test support only) |
| `gafira-cli` | Only `--help` and `--version` |
| `gafira-engine`, `gafira-format`, `gafira-ui`, codecs, licensing, packaging | **Planned** (the architecture chapter holds the full inventory) |

The table groups the crates by layer; the architecture chapter lists the full crate inventory.

Workspace version: `0.1.0-beta.2`. Edition 2024, toolchain `1.93.1` pinned in `rust-toolchain.toml`.

## Repository map

| Path | Content |
|---|---|
| `crates/` | Seventeen library crates in layers 0 to 4, including the test-support crate `gafira-testkit`; the architecture chapter holds the full inventory |
| `apps/gafira-cli/` | Command-line binary |
| `specs/` | Numbered specs (active, `backlog/`, `done/`; index in `specs/README.md`) and the product and architecture documents; `00-decisiones.md` holds owner decisions |
| `docs/manuals/` | Brand manual (`marca/`) and this technical manual (`tecnico/`) |
| `specs/done/legacy/` | Record of the earlier flow: capability specs, change folders and closed changes |
| `.agents/agents/` | The `manual-writer` agent (`.claude/agents` is a symlink) |
| `.github/workflows/ci.yml` | CI: tests, policy checks, `cargo-deny` |
| `deny.toml`, `.cargo/mutants.toml` | License and source policy; mutation-testing settings (the `mutants` profile is in `Cargo.toml`) |
| `docs/adr/` | Architecture decision records, including the flow decisions (ADR 0018 simple spec flow, which supersedes ADR 0001, 0002, 0004 and 0011) |

## Reading order

1. Architecture and layering, to know where code may live.
2. The crate chapters, bottom-up: geometry, color, document, edit.
3. Tooling and methodology, before sending a change. Specs run through the flow in `AGENTS.md` and the `gafira-specs` skill (ADR 0018); the methodology chapter still describes the earlier flow.
