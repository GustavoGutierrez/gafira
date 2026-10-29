# Overview

## Purpose of this manual

This manual describes how Gafira is built: its crates, the rules between them, the public API of the domain crates, the document format, the quality gates and the development method. It documents what exists in the repository today. Parts that are specified but not yet implemented are marked **Planned**.

Product intent (what and why) lives in `docs/specs/`; this manual is the technical counterpart (how it is built). When this manual and a spec disagree, the spec and the owner decisions in `docs/specs/00-decisiones.md` win, and this manual is corrected.

## What Gafira is

Gafira is a local-first desktop editor for graphic design, images and video, written in Rust. The workspace is a Cargo workspace of `gafira-*` crates plus binaries under `apps/`. It targets Windows x86_64, Linux x86_64 and macOS aarch64 (ARQ-03, D-05). The product is proprietary; every third-party dependency must have a permissive license (`deny.toml`).

## Current state

| Area | State |
|---|---|
| Workspace, CI, license gate | Implemented |
| `gafira-geom` (lengths, transforms) | Implemented |
| `gafira-color` (color spaces, conversions) | Implemented |
| `gafira-doc` (document model, JSON, validation, edit capability) | Implemented |
| `gafira-time` (flicks, frame and sample rates, time ranges, timecode) | Implemented |
| `gafira-edit` (`Command`, `Transaction`, `DirtySet`, `History`, `SetProperty`, `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent`, `InsertPage`, `RemovePage`) | Implemented |
| `gafira-pathops` (path booleans, stroke outline, simplification, hit testing) | Implemented |
| `gafira-testkit` (document generator, command property harness) | Implemented (test support only) |
| `gafira-cli` | Only `--help` and `--version` |
| Format, render, engine, UI, codecs, licensing | **Planned** (see the architecture chapter) |

Workspace version: `0.1.0-beta.1`. Edition 2024, toolchain `1.93.1` pinned in `rust-toolchain.toml`.

## Repository map

| Path | Content |
|---|---|
| `crates/` | Library crates (`gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit`, `gafira-pathops`, `gafira-testkit`) |
| `apps/gafira-cli/` | Command-line binary |
| `docs/specs/` | Product and architecture specs; `00-decisiones.md` holds owner decisions |
| `docs/manuals/` | Brand manual (`marca/`) and this technical manual (`tecnico/`) |
| `openspec/specs/` | Main specs per capability, merged at archive time; fast-lane changes do not edit them |
| `openspec/changes/` | Open changes; closed ones move to `openspec/changes/archive/` |
| `.agents/agents/` | Agents of the development flow (`.claude/agents` is a symlink) |
| `.github/workflows/ci.yml` | CI: tests, policy checks, `cargo-deny` |
| `deny.toml`, `.cargo/mutants.toml` | License and source policy; mutation-testing settings (the `mutants` profile is in `Cargo.toml`) |
| `docs/adr/` | Architecture decision records, including the flow decisions (ADR 0001 modes and packs, ADR 0002 fast lane) |

## Reading order

1. Architecture and layering, to know where code may live.
2. The crate chapters, bottom-up: geometry, color, document, edit.
3. Tooling and methodology, before sending a change. Product specs run through the fast lane described in the methodology chapter.
