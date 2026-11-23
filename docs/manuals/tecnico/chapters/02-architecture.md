# Architecture and layering

## Hexagonal architecture

Gafira follows ports and adapters (spec 02, section 1.2). Dependencies point inward:

| Layer | Role | Rule |
|---|---|---|
| Domain (layers 0 and 1) | Document model, commands, geometry, color, time | Pure functions, no I/O |
| Compute engines (layer 2) | Path operations, text layout, raster, audio, segmentation | Pure: receive bytes or buffers, never open files or threads |
| Application (layer 3) | Render graph, ports (traits), media pipeline; `Session` and the Facade services are **Planned** (`gafira-engine`) | Talks to the outside only through ports |
| Adapters (layer 4) | File formats, codecs, render backends, UI, licensing, platform | Implement ports |
| Binaries | `gafira-cli`; the `gafira` desktop app is **Planned** | Composition root: wire implementations together |

ARQ-01 states the dependency rule: a crate depends only on crates of its own layer or lower, and the domain never depends on engines, application or adapters.

## Crate inventory

The repository holds seventeen library crates under `crates/` and one binary under `apps/`.
The table also lists the twenty crates of section 2.1 that only the spec has, so a reader
sees each layer whole. A crate is **Implemented** when it exists in the repository and
**Planned** when it does not; a row marked Implemented adds a "covers" note only when part
of its spec responsibility is still missing.

| Crate | Layer | Status | Covers |
|---|---|---|---|
| `gafira-geom` | 0 | Implemented | Lengths with units and DPI conversion, 2D transforms, `IRect`, sizes |
| `gafira-color` | 0 | Implemented | Color spaces and analytic conversions; gradients, ICC profiles and palette samples **Planned** |
| `gafira-time` | 0 | Implemented | Flicks, frame and sample rates, time ranges, timecode |
| `gafira-anim` | 1 | Implemented | `Animatable<T>`, keyframes, hold, linear, ease and cubic Bézier curves |
| `gafira-doc` | 1 | Implemented | Page, node, timeline and asset model, canonical JSON, validation, edit capability; `Project`, styles and visual identity **Planned** |
| `gafira-edit` | 1 | Implemented | `Command`, `Transaction`, `DirtySet`, `History`, the seven core commands; `Selection` **Planned** |
| `gafira-pathops` | 2 | Implemented | Path booleans, stroke outline, simplification, hit testing |
| `gafira-text` | 2 | Implemented | Shaping, bidi, line breaking, box and column layout, curved text, outlines |
| `gafira-layout` | 2 | Implemented | Guides, column grids, snap, alignment, distribution, bleed and safe areas; reflow on resize **Planned** |
| `gafira-raster` | 2 | Implemented | `ImageBuf`, `ImageView`, alpha masks, tiles, mipmaps, blending, effects and transitions |
| `gafira-audio` | 2 | Implemented | `AudioProcessor` chain, gain, fades, 3-band EQ, pan, mix, resampling, EBU R128 loudness |
| `gafira-segment` | 2 | Implemented | `Segmenter` and the classic path: GrabCut with GMM, max-flow, SLIC, guided refinement |
| `gafira-ports` | 3 | Implemented | Port traits, `Registry<T>` and one error enum per family (chapter 12) |
| `gafira-render` | 3 | Implemented | Render graph, content and tile caches, planner, frame loop; it emits fills and layers, images, masks and effects are **Planned** |
| `gafira-media` | 3 | Implemented | Keyframe index, frame cache, proxies, playback clock and A/V sync |
| `gafira-engine` | 3 | Planned | `Session`, Facade services, event bus, export queue, autosave, memory governor |
| `gafira-render-cpu` | 4 | Implemented | `CpuBackend`: layers and fills execute, images, masks, effects and transitions return an error |
| `gafira-format` | 4 | Planned | `.gaf` read and write, `versions/`, linked media |
| `gafira-assets` | 4 | Planned | Content-addressed store and local library index |
| `gafira-io-image` | 4 | Planned | PNG, JPEG, WebP and GIF import and export |
| `gafira-io-svg` | 4 | Planned | SVG import and export |
| `gafira-io-pdf` | 4 | Planned | PDF export with the Digital and Print presets |
| `gafira-io-pptx` | 4 | Planned | PPTX export, one slide per page |
| `gafira-mux` | 4 | Planned | MP4, WebM/Matroska, Ogg and WAV in Rust |
| `gafira-codec-free` | 4 | Planned | AV1 and Opus |
| `gafira-codec-os` | 4 | Planned | System codecs through FFI |
| `gafira-render-gpu` | 4 | Planned | GPU `RenderBackend` on wgpu |
| `gafira-platform` | 4 | Planned | Data paths, file locks, atomic rename, system fonts |
| `gafira-i18n` | 4 | Planned | Spanish and English message catalogs |
| `gafira-theme` | 4 | Planned | Design tokens loaded from data |
| `gafira-license` | 4 | Planned | Licence and activation, offline token verification |
| `gafira-ui` | 4 | Planned | egui/eframe shell, panels, canvas, timeline, tools |
| `gafira-captions` | 4 | Planned | SRT and WebVTT reading and writing |
| `gafira-plugin` | 4 | Planned | `PluginHost` with wasmi, extension manifest |
| `gafira-testkit` | support | Implemented | Fixtures, `proptest` generators, golden comparison, port doubles; dev-dependency only |
| `xtask` | support | Planned | Layer check and packaging, today a CI stand-in |
| `gafira-cli` (`apps/`) | binary | Implemented | `--help` and `--version`; batch `export` **Planned** |
| `gafira` (`apps/`) | binary | Planned | Desktop app: composition, registries and injection |

External dependencies declared in `[workspace.dependencies]` (`Cargo.toml`): `blake3`,
`kurbo` (geometry, with `serde`), `serde`, `serde_json` (`float_roundtrip`), `png`,
`tiny-skia` (in `gafira-render-cpu`), `parley` and `skrifa` (in `gafira-text`), `rayon`, and
for tests only `proptest`, `criterion`, `trybuild` and `assert_no_alloc`. `linesweeper` is
not in the workspace table: `gafira-pathops` declares it directly.

`gafira-testkit` is a dev-dependency of eight crates — `gafira-audio`, `gafira-doc`,
`gafira-edit`, `gafira-media`, `gafira-raster`, `gafira-render`, `gafira-segment` and
`gafira-text` — while it depends on `gafira-geom`, `gafira-color`, `gafira-doc` and
`gafira-edit`; its optional `ports` feature adds fakes for the port traits, which is the one
edge CI checks separately. Cargo accepts the cycle because a dev-dependency edge is not a
build edge, but the testkit must be used only from integration tests (`tests/`), where each
crate has a single copy of every type; inside a crate's own unit tests the types would be
two different copies.

## Crates gafira-time and gafira-pathops

Both are pure crates with a `clippy.toml` and a V5 row. Sources: `specs/done/legacy/capabilities/time-model/`, `specs/done/legacy/capabilities/timecode/`, `specs/done/legacy/capabilities/path-operations/` and the crate code.

### gafira-time

Layer 0, no internal dependencies (`serde` only). Time is a signed integer count of flicks, 1/705 600 000 s. The crate reads no clock.

| Item | Contract |
|---|---|
| `Flicks`, `FLICKS_PER_SECOND`, `FLICKS_PER_MILLISECOND` | Exact time; construction from seconds and milliseconds and the arithmetic are checked and return `TimeError` on overflow |
| `FrameRate` | Exact rational (`num`, `den`) of at least one frame per second; converts frames to flicks and floors a time to its containing frame |
| `SampleRate` | Audio rate in hertz; sample alignment to flicks is exact |
| `TimeRange` | Half-open range with a non-negative duration; `contains`, `intersects`, `intersection` |
| `Timecode`, `TimecodeFormat`, `TimecodeMode` | Validated frame label over flicks; non-drop, and drop-frame for 29.97 and 59.94 only; `parse` and `format` are canonical |

### gafira-pathops

Layer 2, depends on `gafira-geom`, `gafira-doc`, `kurbo` and `linesweeper`. Coordinates are `f64`; every operation takes a tolerance, with `DEFAULT_TOLERANCE` of `1e-3` as the default.

| Item | Contract |
|---|---|
| `Path`, `FillRule` | A Bézier path with an even-odd or non-zero fill rule |
| `union`, `intersection`, `difference`, `xor`, `boolean_op` (`BooleanOp`) | Boolean operations between two paths |
| `stroke_outline` (`StrokeStyle`) | The outline of a stroked path |
| `simplify` | Path simplification within a tolerance |
| `contains` (`Containment`), `distance_to_perimeter` | Geometric hit testing |
| `PathOpError` | Non-finite input, tolerance too fine for the input and other invalid inputs |

## The spec's crate table

Section 2.1 of `specs/02-arquitectura-software.md` is the authoritative table: it states each
crate's responsibility and prohibitions. The inventory above repeats the layer and a short
note only.

A new crate must satisfy one ADR-01 criterion: a layer boundary, an optional feature or legal exposure, incremental compile time, or reuse by several consumers. Internal organization alone is a module, not a crate.

## Allowed-dependency rules (V5)

CI enforces the layer table in the `policy` job, step "Domain crates respect the layer table (V5)". The step holds a table of `crate  allowed-internal-deps` and, for each row, reads `cargo tree -p <crate> -e normal,build --target all`:

| Crate | Allowed internal dependencies |
|---|---|
| `gafira-geom` | none |
| `gafira-color` | none |
| `gafira-time` | none |
| `gafira-doc` | `gafira-geom`, `gafira-color`, `gafira-time` |
| `gafira-edit` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-testkit` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit`, `gafira-time` |
| `gafira-pathops` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-layout` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-anim` | `gafira-time`, `gafira-geom`, `gafira-color` |
| `gafira-text` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-pathops`, `gafira-time` |
| `gafira-raster` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-audio` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-segment` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-raster`, `gafira-time` |
| `gafira-ports` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time`, `gafira-raster` |
| `gafira-render` | `gafira-ports`, `gafira-doc`, `gafira-geom`, `gafira-color`, `gafira-raster`, `gafira-time`, `gafira-anim` |
| `gafira-media` | `gafira-ports`, `gafira-doc`, `gafira-raster`, `gafira-time`, `gafira-geom`, `gafira-color` |
| `gafira-render-cpu` | `gafira-ports`, `gafira-doc`, `gafira-geom`, `gafira-color`, `gafira-raster`, `gafira-time` |

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
| ARQ-107, layers 0 to 2 know no language | A `policy` step greps those crates for language types and exempts `gafira-ports`, which defines `Locale` on purpose (`.github/workflows/ci.yml:156`) |
| ARQ-30 interim, edit capability | V13: `EditToken` may be named only in `gafira-doc`, `gafira-edit` and `gafira-testkit`, and `gafira-edit` must not re-export it. `trybuild` compile-fail fixtures in `gafira-edit` show that a crate outside it cannot mutate a `Document` without `History` |
| ARQ-81, every core command has a property test | CI step V14 (stand-in for `xtask`) checks that every type that implements `Command` has a test in `crates/gafira-edit/tests/prop_commands.rs` |

## The edit capability in the layering

Reading a `Document` is free; mutating it requires an `EditToken`. This funnels every change through commands, which live in `gafira-edit`. The document and edit chapters describe the mechanism.
