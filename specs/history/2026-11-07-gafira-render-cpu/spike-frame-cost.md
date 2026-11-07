# Spike: the cost of one frame in `gafira-render-cpu`

Facts only. Measured 2026-11-05 against `develop` at `1d68c30`, which is `CB1` (superficie,
bucle de tiles, caché, cull, `coverage`) **without `CB2`**: the numbers below do not include
the layer stack, the images or the effects.

Scratch: one throwaway crate outside the repo, `<scratch>/cbprofile`, which depends by path on
`gafira-render-cpu`, `gafira-render`, `gafira-doc`, `gafira-ports`, `gafira-raster`,
`gafira-geom`, `gafira-color` and `gafira-time`, and mirrors the repo's `[profile.release]`
(`opt-level = 3`, `lto = "thin"`, `codegen-units = 4`). Release build, median of 11 samples.
No repo file was modified.

```sh
CARGO_TARGET_DIR=<scratch>/cbprofile-target cargo run --release --offline
```

The scene is one 1080p page (1920×1080) with fifty `FillPath` ops, a page-covering background
and forty-nine overlapping boxes of different sizes and alphas — the shape `CB1` renders, since
`CB2`'s layer ops are not in this HEAD. Forty tiles of 256 px, 158 op-tile pairs, 3 648 400
blended pixels. The cache budget is 1 GiB, so the frame's tiles fit in it.

## Summary

| Claim | Verdict | Number (run A / run B) |
|---|---|---|
| One full 1080p page, `Rgba8`, cache cold | measured | **163.47 / 195.32 ms** |
| The same plan again, cache warm | measured | 79.92 / 90.83 ms |
| `read_back` of the whole page (`Rgba8`) | measured | 111.21 / 130.93 ms |
| …of which the sRGB quantisation itself | measured | 104.60 / 121.39 ms |
| The tile work, replicated with the calls the backend makes | measured | 49.16 / 56.43 ms |
| Blending (`blend_f32`, Normal) | measured, not the first cost | 42.83 / 49.56 ms |
| Coverage (`tiny-skia::Mask::fill_path`, 158 masks) | measured, cheap | 1.32 / 1.54 ms |
| Quantisation of the tiles **into the cache** | measured, matches the gap | **135.31 ms** (run B) |
| The unexplained gap in `execute` | **explained**: the tile quantisation | 114.31 / 138.89 ms |
| `Rgba8` frame = two quantisation passes | **found** | ~240–266 ms of a ~327 ms cold frame |
| Tile buffers, forty of 1 MiB, first touch vs pool | **the premise is refuted** | 0.61 ms fresh, 0.03 ms pooled |
| CAL-11 (one 1080p PNG frame < 2 s) | **confirmed, ample margin** | ~85 ms of render, 23× margin |
| CAL-16 (60 s reel < 180 s) | **at risk, and not this change's number** | ~153 s of render alone, 85 % of the budget |
| ARQ-143 preview (≤ 8 ms of visible render) | **over by 20–24×** | 163–195 ms per cold full page |

## 1. Where the time of one `Rgba8` frame goes

The backend renders each tile into a tile-local premultiplied `f32` buffer, encodes it for the
cache (`surface::encode_tile`) and writes it into the frame's `f32` surface. With an `Rgba8`
target that encoding is the sRGB quantisation, and `read_back` quantises the page **again**.

| Stage | ms (run B) | Note |
|---|---:|---|
| tile work (coverage + blending + copies) | 56.43 | the replica of `render_tile` + `write` + cache |
| quantisation of the tiles into the cache | 135.31 | `chunks_exact(4)` over 40 × 65 536 px |
| **`execute`, cold** | **195.32** | 56.43 + 135.31 = 191.74, within 2 % |
| `read_back` of the whole page | 130.93 | 121.39 of it is the same quantisation |
| **one cold `Rgba8` frame, end to end** | **~326** | two full passes over the pixels |

The replica's own parts do not time the tile quantisation, which is why they sum to 56.43 ms
against a 195.32 ms `execute`. Run A shows the same shape one machine-state earlier: 163.47 ms
of `execute`, 49.16 ms of replicated parts, 114.31 ms of gap, 135.31 ms of measured tile
quantisation. The gap is not page faults, not the cull and not the cache.

**The finding is a defect of `CB1`, not a property of the hardware: an `Rgba8` frame pays for
the sRGB curve twice.** The cache is format-agnostic (`TileKey` carries no format, D5), so it
can hold the `f32` tile as it comes out of `render_tile`; and a surface that will only ever be
read back as `Rgba8` does not need to be `f32` for the whole frame. Either fix removes one of
the two passes.

## 2. The preview budget

A full-page cold frame is 163–195 ms against ARQ-143's ≤ 8 ms of visible render and the 16 ms
frame. ARQ-143 defers and refines (`execute` returns `left > 0`, the caller presents the best
mip and the work continues next frame), so a cold full-resolution frame is **not** a violation
by itself. What the measurement does say:

- A cold page cannot be presented at 100 % in one frame on this path, and the frame budget
  cannot absorb a full-page repaint at any zoom level. Only tile-incremental work can.
- The dominant term is the quantisation, which is the **last** thing a preview needs and the
  first thing it currently pays for on every fresh tile.
- `read_back` already takes a region (`RenderBackend::read_back(cx, region)`, and
  `FrameLoop::read_back(region)` passes it through). Quantising one 256 px tile instead of the
  page is 1/40th of the numbers above. Nothing in the contract has to change for that; the
  caller — the app, not this change — has to ask for the rectangle it invalidated.

## 3. The two export numbers

| Path | ms per frame | Against its budget |
|---|---:|---|
| `RgbaF32` target: `execute` + `read_back` (a copy, no curve) | 84.68 | — |
| CAL-11: one 1080p PNG frame < 2 000 ms | 84.68 + PNG encode | **inside**, 23× margin before the encode |
| CAL-16: a 60 s reel < 180 000 ms | 84.68 × 1 800 = 152 424 | **inside by 15 %**, before decode, codec and mux |

CAL-11 is *extracción de un fotograma PNG < 2 s* (05 §A, row CAL-11): one frame, not a reel.
The scratch harness closes by multiplying a per-frame cost by 30 and comparing that to CAL-11's
2 000 ms. **That line is void** — it measures thirty frames (one second of video at 30 fps)
against a one-frame budget, and it is where the report of "CAL-11 in risk at 2 677 ms" comes
from. The budget that a reel threatens is CAL-16, whose number is FD-55's:

```
1800 frames × 84.68 ms = 152.4 s of render alone, of the 180 s CAL-16 allows.
```

That is the margin CB7 buys, and it is thin: the decode, the codec and the mux of the export
pipeline are not in this measurement and cannot be, from this crate.

## 4. What the pool does and does not buy

Forty tile buffers of one mebibyte, written once per page, fresh against reused:

| | ms |
|---|---:|
| allocated fresh (first touch, one write per page) | 0.61 |
| reused from a pool | 0.03 |

A pool saves 0.58 ms on this scene. The hypothesis that ~113 ms of the frame was page faults
on freshly mapped tile buffers **does not survive measurement**: the kernel hands out zeroed
pages far faster than the pixel work. The pool still has a reason — it removes an allocation
from the hot path and bounds the memory — but it is not a performance fix for this frame.

## 5. What this changes in the change

1. **A delivery that quantises once is justified.** The `Rgba8` target currently pays the sRGB
   curve in `encode_tile` and again in `Surface::read`. It is the single largest cost measured
   here (~135 ms of a 195 ms `execute`) and it is a `CB1` decision (the cache's format, the
   surface's format for an 8-bit target), not a hardware limit. It is smaller than `CB2` and
   does not touch the layer stack.

   **It is a preview cost, and no bench of this change covers the preview.** The export target is
   `RgbaF32`, so `encode_tile` copies and `read_back` copies: `CB6`'s number does not see the
   doubled curve at all. Whoever takes this delivery must measure it against ARQ-143's
   visible-render budget, not against CAL-11, or it will be a fix nobody counts.
2. **`CB7` stops being conditional on CAL-11.** Its justification moves to CAL-16's 15 % margin
   and to the preview's throughput. `CB6` still produces the number; the decision to keep the
   parallelism is made on `CB6`'s and `CB7`'s numbers, as the design already says.
3. **The pool is not urgent.** It stays where `O11` needs it — the layer stack's `depth × 1 MiB`
   — and it is not sold as a frame-time win.
4. **Region-scoped `read_back` is the preview's fix, and it is the caller's.** Recorded here so
   that the app's frame loop is not built around a full-page read.

## What this cannot prove

- **It is `CB1`, not the change's HEAD.** Layers, opacity per layer, the sixteen modes, images,
  masks, effects and transitions all add cost that this scene does not contain. `CB2`'s nesting
  in particular multiplies the buffers `O11` already prices.
- **One machine, one scene, single-threaded, release.** The absolute numbers move ~20 % between
  runs of the same binary on the same machine (run A against run B above); the ratios and the
  attribution are stable, the milliseconds are not.
- **The harness is disposable and lives outside the repo.** The reproducible artefact is
  `CB6`'s criterion bench, which is where CAL-11's number is supposed to come from; this spike
  is the evidence that decided the delivery, not the gate.
- **It says nothing about the GPU backend** (`gafira-render-gpu`), which ARQ-143 also governs.
