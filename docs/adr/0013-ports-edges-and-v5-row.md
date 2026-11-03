# 0013. `gafira-ports` edges, V5 row and test doubles

- Status: proposed
- Date: 2026-11-02
- Related: ARQ-01, ARQ-02, ARQ-06, ARQ-20, ARQ-107, ADR-01 (02 §2.3), ADR 0007, ADR 0008, change `gafira-ports`

## Context

`gafira-ports` is the first layer-3 crate. Three forces meet:

- Port signatures name `ImageBuf`, `Effect` and `Transition` (02 §4.6, §4.11), all in `gafira-raster`. Adapters must reach them and the render contract without `gafira-render` or `gafira-engine` (ARQ-06).
- The V12 and ARQ-107 steps of `.github/workflows/ci.yml` read their crates from the V5 table. The ARQ-107 step greps `Locale` in each, and `gafira-ports` defines `Locale` (02 §14.1). A plain row fails CI.
- 02 §4.14, §4.16 and §5.3 put port fakes in `gafira-testkit`, while ADR 0007 rejects `testkit -> raster`, and the engines have `gafira-testkit` as a dev-dependency. `testkit -> ports -> raster` would break the testkit row and rebuild two extra crates on every raster mutant.

## Decision

1. The row is `gafira-ports  gafira-geom gafira-color gafira-doc gafira-time gafira-raster`. No other internal edge. External dependencies are `serde` and `kurbo`. `gafira-audio` and `gafira-segment` stay out, and their registries are `Registry<dyn _>` at the composition root.
2. `gafira-ports` has a byte-identical `clippy.toml` and holds no I/O, though ARQ-02 does not bind layer 3.
3. The ARQ-107 step skips only its `Locale` grep for `gafira-ports`. Its dependency check still applies.
4. `gafira-testkit` gains an optional feature `ports`, off by default, holding the port fakes. Consumers enable it in `dev-dependencies`. V5 keeps checking the default graph, so its testkit row does not change, and "no `testkit -> raster`" of ADR 0007 holds there. This narrows ADR 0007 and does not replace it.

## Consequences

- Every adapter links `gafira-raster` through ports. It would anyway: most use `ImageBuf`.
- A change in `gafira-raster` rebuilds ports and all adapters.
- ICU, parley and `rayon` stay out of adapters that do not ask for them.
- A later layer-3 crate needs the same `Locale` skip if it mentions `Locale`.
- Decision 4 is unproven until `explorer` runs XP1 of the `gafira-ports` design. No fake lands before that.

## Alternatives considered

- **Move `ImageBuf` below raster, or define a second one in ports.** A move breaks ADR 0005, and a copy needs conversions at every adapter.
- **`ports -> text`, `audio` or `segment`.** No signature needs them, and each brings a heavy tree to every adapter.
- **Plain `testkit -> ports`.** Breaks ADR 0007 and slows T4 of raster.
- **New support crate for fakes.** Clean, but it needs a 02 §2.1 row and a V5 row, and 02's text "en `gafira-testkit`" changes. It is the fallback if XP1 fails.
- **Feature on `gafira-ports` with its fakes.** Puts test code in a production crate's public surface.
- **No `clippy.toml`, with a V12 exemption.** A second exemption for no gain.
- **Add `gafira-ports` to a separate table.** Two lists of crates to keep in step.
