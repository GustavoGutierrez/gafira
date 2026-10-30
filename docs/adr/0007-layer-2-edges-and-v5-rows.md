# 0007. Layer-2 dependency edges and V5 rows

- Status: accepted
- Date: 2026-10-30
- Related: ARQ-01, ARQ-02, ARQ-102, ADR-01 (02 §2.3), 02 §2.2, ADR 0005, changes `layout-core`, `engine-contracts`

## Context

Five layer-2 crates need V5 rows and `clippy.toml` copies. Three points are open:

- 02:170 says layout uses `gafira-text`; 02:136 says `layout -> geom, doc`.
- Labels and errors need `MessageKey` and `ToMessage` from `gafira-doc`, but 02:136 lists no doc edge for raster, and the generic line above it says only "según necesidad".
- The golden harness might need `gafira-testkit -> gafira-raster` (`spike-layer2.md` X6).

The V5 step reads the whole `cargo tree`, not the direct dependencies (`.github/workflows/ci.yml`). So a row must list transitive `gafira-*` crates too. The `gafira-layout gafira-geom gafira-doc` row proposed in the spike would fail, because `gafira-doc` pulls `gafira-color`.

## Decision

Rows, added in the delivery that creates each crate:

```text
gafira-layout  gafira-geom gafira-color gafira-doc
gafira-text    gafira-geom gafira-color gafira-doc gafira-pathops
gafira-raster  gafira-geom gafira-color gafira-doc
gafira-audio   gafira-geom gafira-color gafira-doc gafira-time
gafira-segment gafira-geom gafira-color gafira-doc gafira-raster
gafira-pathops gafira-geom gafira-color gafira-doc   (changed when PathOpError gains ToMessage)
```

- Every engine may depend on `gafira-doc`. The generic line of 02 §2.2 already allows it; `specifier` writes the edges out there.
- **No `layout -> text`.** 02:136 stands and ADR-01's example drops "y layout". The reflow of FD-54 receives measured boxes through an injected measure function defined in `gafira-layout`.
- **No `testkit -> raster`.** The harness and the ARQ-132 scenes use raw bytes (ADR 0010).
- **No `doc -> time` now.** It arrives with `Page.duration` (decision L5 of the `layout-core` design), and then `gafira-time` joins every row that contains `gafira-doc`.
- **No `audio -> anim`.** `gafira-media` samples volume tracks and hands plain values to audio.
- **No `raster -> anim`** until Fase 2 tracking.
- Each new domain crate gets a byte-identical `clippy.toml` (V12) and `#![forbid(unsafe_code)]`. The row, the `clippy.toml` and the `Cargo.toml` entries land in one commit: V5 fails on a `clippy.toml` without a row and on a row without a crate.

## Consequences

- The only engine-to-engine edges are `text -> pathops` and `segment -> raster`, as 02:136 says. The order inside layer 2 is free apart from those.
- `gafira-layout` stays free of the text stack and its font crates.
- Reflow gets one extra input, the measure function. `gafira-engine` supplies it.
- `ResizeDesign` in `gafira-edit` (layer 1) cannot call `gafira-layout` (layer 2): it must receive the computed result. This holds with or without this ADR, and is recorded in ADR 0008.
- Adding `doc -> time` later touches the rows of doc, edit, testkit and all five engines, as one commit.

## Alternatives considered

- **`layout -> text`, as ADR-01 words it.** Brings parley and font data into a crate of pure rectangle functions for one feature that is not in the first slice.
- **Direct-dependency rows.** Would need a change to the V5 step, and would stop catching a transitive path to a forbidden crate.
- **`testkit -> raster` for an `ImageView` harness.** Saves one line in each test, but `gafira-testkit` then sits above an engine, and a unit test in raster would see a second copy of its own types (`crates/gafira-testkit/src/lib.rs` limits testkit to integration tests for this reason).
