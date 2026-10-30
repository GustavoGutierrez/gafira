# 0008. Persisted types live in `gafira-doc`; engines own their inputs

- Status: proposed
- Date: 2026-10-30
- Related: FD-03, FD-05, FD-07, FD-39, FD-41, FD-54, 02 §3.1, 02 §3.2, D-19, change `layout-core`

## Context

FD-03 stores guides in `Page.guides`, and `gafira-doc` cannot depend on `gafira-layout`. The same question repeats for text input, painted masks and tiles: the specs name a type without saying which side of the doc/engine line owns it. `spike-layer2.md` also raises a stricter case: `PaintMask` and `ResizeDesign` are commands in `gafira-edit` (layer 1), which cannot see layer 2.

## Decision

One rule for all of them:

1. **A type that is saved in a document lives in `gafira-doc`.** Engines read it; they never define it.
2. **An engine defines its own input and output types.** They are plain structs, not serialized. Mapping document data to engine input happens in layer 3 (`gafira-render`, `gafira-engine`).
3. **A layer-1 command never calls an engine.** It receives a computed result or swaps a reference.

Applied:

| Item | Home |
|---|---|
| `Guide`, `GuideAxis`, `GuideSet`, `Margins`, `GridSettings`, `GridStyle` | `gafira-doc`. `gafira-layout` operates on them and re-exports nothing |
| `Page.bleed` | `gafira_geom::Length` in `Page`; no new type |
| Text input (`gafira-text`) | Plain structs in `gafira-text`, with fonts as bytes. `TextNode` is not a prerequisite |
| `RichText`, `TextPath`, `TextAlign`, `VAnchor`, `TextSizing`, `StyleRef` | `gafira-doc`, in a later change that adds `TextNode` |
| `TileSetRef`, `Mask` | `gafira-doc` (references, no pixels) |
| `Tile`, `TileSet`, `AlphaMask` | `gafira-raster` |
| `PaintMask` | Swaps one `TileSetRef` for another; copy-on-write of pixels stays in raster and layer 3 |
| `ResizeDesign` | Receives the output of `gafira-layout::reflow` |

## Consequences

- `gafira-text` can be built and tested before `TextNode` exists, and a change in `TextNode` does not reach it.
- FD-03's sentence about the crate of `Guide` is settled; `gafira-layout` keeps only the operations.
- `RichText` and `TextPath` stay undefined until the change that needs them. Their shape is sketched in the `layout-core` design for that change.
- Layer 3 owns one mapping per node kind. It is more code than sharing one type, and it is the price of keeping fonts and pixels out of the document crate.

## Alternatives considered

- **`Guide` and friends in `gafira-geom`.** They are page data with lock flags and grid styles, not geometry. `gafira-geom` must not know nodes or pages (02 §2.1).
- **`gafira-text` consumes `TextNode`.** Forces a document change first, with six undefined types, and makes the text engine depend on serialized layout.
- **`Tile` in `gafira-doc`.** Puts pixel buffers in the document crate, against its 02 §2.1 row, only so `PaintMask` can hold them.
- **`gafira-edit -> gafira-raster` or `-> gafira-layout`.** Forbidden by ARQ-01.
