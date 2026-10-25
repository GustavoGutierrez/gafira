# Architecture and layering

## Hexagonal architecture

Gafira follows ports and adapters (spec 02, section 1.2). Dependencies point inward:

| Layer | Role | Rule |
|---|---|---|
| Domain (layers 0 and 1) | Document model, commands, geometry, color, time | Pure functions, no I/O |
| Compute engines (layer 2) | Path operations, text layout, raster, audio, segmentation | Pure: receive bytes or buffers, never open files or threads |
| Application (layer 3) | Session, services, render graph, ports (traits) | Talks to the outside only through ports |
| Adapters (layer 4) | File formats, codecs, render backends, UI, licensing, platform | Implement ports |
| Binaries | `gafira`, `gafira-cli` | Composition root: wire implementations together |

ARQ-01 states the dependency rule: a crate depends only on crates of its own layer or lower, and the domain never depends on engines, application or adapters.

## Crates in the workspace today

| Crate | Layer | Internal dependencies | Purpose |
|---|---|---|---|
| `gafira-geom` | 0 | none | Lengths with units and DPI conversion, 2D transforms, sizes |
| `gafira-color` | 0 | none | Color spaces, `f32` RGBA types, analytic conversions |
| `gafira-doc` | 1 | `gafira-geom`, `gafira-color` | Document model, JSON, validation, edit capability |
| `gafira-edit` | 1 | `gafira-geom`, `gafira-color`, `gafira-doc` | `Command` contract, `Transaction`, `DirtySet`, `History`, core commands (`SetProperty`, `InsertNodes`, `RemoveNodes`) |
| `gafira-testkit` | support | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit` | Test support: bounded document generator, corruption generators, `prop_command` harness. Used only as a dev-dependency |
| `gafira-cli` (`apps/`) | binary | none | `--help`, `--version` |

External dependencies declared in the workspace: `kurbo` (geometry, with `serde`), `serde`, `serde_json` (`float_roundtrip`) and, for tests only, `proptest` and `trybuild`.

`gafira-testkit` is a dev-dependency of `gafira-doc` and `gafira-edit` while it depends on both. Cargo accepts this because a dev-dependency edge is not a build edge, but the testkit must be used only from integration tests (`tests/`), where each crate has a single copy of every type; inside a crate's own unit tests the types would be two different copies.

## Planned crates

The spec lists further crates that are not in the repository yet. They are **Planned** and named here only to place the existing ones:

- Layer 0 and 1: `gafira-time`, `gafira-anim`.
- Layer 2 (engines): `gafira-pathops`, `gafira-text`, `gafira-layout`, `gafira-raster`, `gafira-audio`, `gafira-segment`.
- Layer 3 (application): `gafira-ports`, `gafira-render`, `gafira-media`, `gafira-engine`.
- Layer 4 (adapters): `gafira-format`, `gafira-assets`, `gafira-io-*`, `gafira-mux`, `gafira-codec-*`, `gafira-render-cpu`, `gafira-render-gpu`, `gafira-platform`, `gafira-i18n`, `gafira-theme`, `gafira-license`, `gafira-ui`, `gafira-captions`, `gafira-plugin`.
- Support: `xtask`.

The authoritative table with each crate's responsibility and prohibitions is section 2.1 of `docs/specs/02-arquitectura-software.md`.

A new crate must satisfy one ADR-01 criterion: a layer boundary, an optional feature or legal exposure, incremental compile time, or reuse by several consumers. Internal organization alone is a module, not a crate.

## Allowed-dependency rules (V5)

CI enforces the layer table in the `policy` job, step "Domain crates respect the layer table (V5)". The step holds a table of `crate  allowed-internal-deps` and, for each row, reads `cargo tree -p <crate> -e normal,build --target all`:

| Crate | Allowed internal dependencies |
|---|---|
| `gafira-geom` | none |
| `gafira-color` | none |
| `gafira-doc` | `gafira-geom`, `gafira-color` |
| `gafira-edit` | `gafira-geom`, `gafira-color`, `gafira-doc` |
| `gafira-testkit` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit` |

The step fails when:

- a crate depends, directly or transitively through normal or build edges, on a `gafira-*` crate that is not in its row;
- the tree contains a forbidden external crate: `egui`, `eframe`, `winit`, `wgpu`, `tokio`, `mio`, `socket2`, `reqwest` or `ureq`;
- `cargo tree` returns nothing for a listed crate;
- a crate has its own `clippy.toml` (the mark of a domain crate) but no row in the table.

Adding a domain crate means adding its row and a `clippy.toml` copy; the step itself stays unchanged. `gafira-testkit` has a row but no `clippy.toml`, so it is not a domain crate for V12; its row keeps its internal edges bounded.

## Other enforced rules

| Rule | Mechanism |
|---|---|
| ARQ-02, no I/O in domain crates | A `clippy.toml` per domain crate with `disallowed-methods` and `disallowed-types` (`std::fs`, `std::net`, threads, clocks, `std::env`, processes) |
| ARQ-10, deterministic order | `HashMap` and `HashSet` are disallowed types in domain crates; use `BTreeMap` and `BTreeSet` |
| V12, policy copies in sync | The `clippy.toml` of `gafira-geom` is canonical; the other copies must be byte-identical (`diff`) |
| ARQ-07, CLI without UI | `cargo tree -p gafira-cli --no-default-features` must not contain `gafira-ui`, `eframe`, `winit` or `wgpu` |
| No `unsafe` | `unsafe_code = "deny"` in workspace lints, plus `#![forbid(unsafe_code)]` in every `lib.rs` and `main.rs`, checked by CI |
| ARQ-30 interim, edit capability | V13: `EditToken` may be named only in `gafira-doc`, `gafira-edit` and `gafira-testkit`, and `gafira-edit` must not re-export it. `trybuild` compile-fail fixtures in `gafira-edit` show that a crate outside it cannot mutate a `Document` without `History` |
| ARQ-81, every core command has a property test | **Planned**: CI step V14 (stand-in for `xtask`); today the tests exist in `crates/gafira-edit/tests/prop_commands.rs` but nothing enforces them |

## The edit capability in the layering

Reading a `Document` is free; mutating it requires an `EditToken`. This funnels every change through commands, which live in `gafira-edit`. The document and edit chapters describe the mechanism.
