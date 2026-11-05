# Design — `gafira-render-cpu`

Status: design, 2026-11-05. Mode: full (a new crate needs a new V5 row, which is a hard floor).
Author: `architect`. Not implemented: this document is the contract `tdd-apply` follows.
Owner decision: **O3** of `specs/history/2026-11-04-layer-3/design.md` scheduled this change
"right after layer 3, on its own", and layer-3b has closed. Nothing here reopens layer 3.

## Quick path

1. The delivery table: nine deliveries, `SC1` first, `PC1` second, `CB1`–`CB7` after.
2. "The three acceptances": which delivery moves which, and which one this change cannot close.
3. `D1`: **the port does not say where a tile is** — the one change this design asks of `gafira-ports`.
4. `D2`: a coverage source (tiny-skia) plus the existing `gafira-raster` primitives; no new blends.
5. "What this change cannot prove": read before planning the hand mutants.
6. "The V5 row": the exact line, and why it permits exactly that.

## Dependency state

`develop` after layer-3b. `gafira-render` is a leaf crate (nothing depends on it) and carries the
graph, the plan, the cache, the frame loop and the `PageRenderer`. `gafira-ports` carries
`RenderBackend`, `PageRenderer` and the plan a backend reads. There is no active change folder and
no outcome in `.odd/state.json` for this work.

Two facts constrain the schedule and are not negotiable by this design:

- **Another worker is in `crates/gafira-render/` and `crates/gafira-media/` right now.** `PC1` is the
  only delivery that edits `gafira-render` (the planner, `page.rs` and their tests). It must be cut
  from a `develop` that already holds that branch, or that branch rebases. Everything from `CB1` on
  lives in the new crate and does not collide.
- `crates/gafira-render-cpu/` does not exist. `apps/gafira` does not exist, so the composition root
  that would construct a backend is out of this change (see "Out, and why").

Verified against the code, not from memory: `crates/gafira-ports/src/render.rs` (`RenderBackend`,
`RenderPlan`, `TileTask`, `RenderOp`, `TargetDesc`, `TileCache`, `RenderError`),
`crates/gafira-ports/src/export.rs` (`PageRenderer`), `crates/gafira-render/src/{planner,page,frame}.rs`,
`crates/gafira-raster/src/{lib,view,blend,effects/mod,transitions/mod}.rs`,
`.github/workflows/ci.yml` (V5, V12, ARQ-107), `crates/gafira-geom/clippy.toml`.

## What the change is

The real CPU rasterizer behind `ports::RenderBackend`: a layer-4 adapter, `gafira-render-cpu`, that
takes a `ports::RenderPlan` and a `ports::TileCache` and produces pixels in the target format
(`Rgba8` sRGB premultiplied for `Preview`, `RgbaF32` linear premultiplied for `Export`, 02 §6.2). It
is the reference backend and the golden oracle (02 §2.1, §4.5; ARQ-113).

It must be a compositor, not a drawing command: it reads the seven `RenderOp` variants — which are
closed on purpose so that a new variant stops a backend from compiling — and it composes them on a
surface in plan order.

## Scope of the change

### In

| Area | Requirement IDs | Where |
|---|---|---|
| The crate: manifest, `#![forbid(unsafe_code)]`, `clippy.toml` copy, `[workspace.dependencies]` entry, V5 row, `Cargo.lock` | ARQ-01, ARQ-02, ARQ-03, ARQ-05, ARQ-06, ARQ-70, ARQ-71, ARQ-107; 02 §2.1 | `SC1` |
| The plan carries the place of a tile: `TileTask.region` and `TargetDesc.origin`, filled by the planner and the page renderer | 02 §4.6, §6.1; ARQ-121, ARQ-142 | `PC1` |
| Surface, frame, tile loop, cache read/write, `read_back`, `memory_usage`, `trim`, cancellation, op culling, `FillPath` with 8-bit coverage in both formats | 02 §4.6, §6.2; ARQ-117, ARQ-119, ARQ-139, ARQ-143, ARQ-44 (partial), ARQ-111 | `CB1` |
| Layer stack: `PushLayer`/`PopLayer`, opacity, all 16 blend modes through `gafira-raster`, nested layers, tile-local buffers | 02 §6.1, §6.2; ARQ-119, ARQ-117 | `CB2` |
| The ARQ-42 golden: preview at 100 % against the export, ≤ 1/255 on 99.9 % of the pixels; the transparency case of FD-55 (b); the oracle mechanism of ARQ-113 | ARQ-42, ARQ-113; FD-55 (b) (c) | `CB3` |
| `DrawImage`: affine resample, bilinear filtering, premultiplied in both formats | 02 §6.1; ARQ-121, ARQ-117 | `CB4` |
| `ApplyMask`, `ApplyEffect` (ROI-grown), `ApplyTransition` (provisional reading) | ARQ-118, ARQ-119, ARQ-120, ARQ-112 | `CB5` |
| `bench_export_png_1080` and the cancellation measurement of FD-55 (d) | CAL-11, ARQ-40, ARQ-114, ARQ-43 | `CB6` |
| Optional: parallelism over the row bands of one tile, with an injected pool | ARQ-138; CAL-11 | `CB7` (conditional) |

### Out, and why

| Out | Reason | Requirement IDs |
|---|---|---|
| `gafira-render-gpu` | A separate adapter with its own device, WGSL and driver questions (02 §2.1). The CPU backend is the one that must exist first, because it is the oracle. | 02 §2.1 |
| The node kinds `ImageNode`, `TextNode`, `VideoNode`, … | They do not exist in `gafira-doc` (`NodeKind` has `Group`, `Shape`, `Extension`; ARQ-134). `DrawImage` is implemented at the **op** level in `CB4` and unit-tested on a synthetic plan; no document produces one yet, so no golden covers it. | ARQ-134, 02 §3.1 |
| `gafira-io-image`, the exporters, `ExportRequest`, the export queue | Layer 4 and a different concern. CAL-11's real end-to-end measurement (render + encode + write) belongs to FD-55's exporter; `CB6` measures render + encode in-crate with `png` as a dev-dependency and says so. | FD-55, CAL-11 |
| `perf-budgets.toml` and the reference-machine nightly job | They do not exist (ARQ-40 is unbuilt). `CB6` produces the number and prints it; the **gate** is a separate change. `CAL-11` is therefore measured here and not closed. | ARQ-40 |
| The parallel tile loop of 02 §6.7 ("render CPU en tiles paralelos") | `FrameLoop::frame` calls `execute` with a **one-task plan per tile** (`crates/gafira-render/src/frame.rs`), so the backend cannot see more than one tile per call. Parallel tiles need a change in `gafira-render`, not in the adapter. `CB7` is the conditional fallback that parallelises *rows inside one tile*. | CAL-11, 02 §6.7 |
| Effect, mask and transition **emission** from the graph | `RenderGraphBuilder` emits only `PushLayer`, `FillPath` and `PopLayer` today. `CB5` implements the three variants the trait requires; nothing exercises them from a document, so nothing can golden them. | 02 §6.1 |
| The composition root (`apps/gafira`) and the backend selector's real degradation path | The app crate does not exist. `FrameLoop`/`BackendSelector` already exist in `gafira-render` (layer-3b) and take a `RenderBackend`; nothing here needs to wire them. | ARQ-22 |
| `f16`, GPU-only formats, CMYK | `f16` is GPU-internal (ARQ-117, 02 §6.2); CMYK is a PDF concern (FD-56). | ARQ-117 |

### What is reversible

Reversible without touching a public API: the coverage source (tiny-skia is behind one private
module and one function that returns an `AlphaMask`), the resampler, the layer-stack strategy, and
the in-crate bench.

**Not cheaply reversible once other backends exist**: the two new port fields of `PC1`. They are
additive and no behaviour changes, but every constructor of `TileTask`/`TargetDesc` in the workspace
must be edited in the same commit, and a GPU backend written later reads them. That is why `PC1`
comes second, before any pixel code depends on it.

**Not reversible at all**: the 8-bit coverage decision (`D3`). If a later change wants analytic
f32 coverage for the export, the ARQ-42 golden changes its numbers, and the preview/export agreement
it measures is a different agreement.

## The three acceptances this change exists to unblock

O3 kept three pixel-level acceptances out of layer 3 because only a rasterizing backend can prove
them. The layer-3 design lists them at its "Out, and why" table: ARQ-42, CAL-11, and the golden pixel
oracle (ARQ-113). Per delivery:

| Acceptance | Moved by | How |
|---|---|---|
| **ARQ-42** — preview at 100 % on CPU against the export, ≤ 1/255 per channel on 99.9 % of the pixels | `CB1`, `CB2`, **`CB3`** | `CB1` and `CB2` are the two qualities actually rendering; `CB3` compares two `GraphPageRenderer`s (preview, export) over one document and one region at scale 1.0. |
| **CAL-11** — `bench_export_png_1080` < 2 s | `CB1`–`CB4`, **`CB6`** | `CB6` measures the whole export path in-crate: plan → tiles → f32 surface → sRGB PNG bytes, single-threaded, on the reference profile. |
| **The golden oracle (ARQ-113)** — the backend that layer-2 goldens compare against | `CB1`, `CB2`, **`CB3`** | `CB3` delivers the comparator path (export → sRGB8 with ARQ-117's rounding) and one stored golden of a fill-and-layers scene. |

**Verdict, stated before the estimates:**

- **ARQ-42 is reachable**, with a residual limit: the tolerance is proven for the scene set of
  `CB3` (opaque and semi-transparent fills, groups, one nested layer) and not for arbitrary
  documents. The mechanism is `D3`; the residual risk is `XP2`.
- **CAL-11 is measurable here and cannot be closed here.** The number is produced by `CB6`; the gate
  needs `perf-budgets.toml` and the reference runner (ARQ-40), and the *technique* 02 §6.7 names
  (parallel tiles) needs a `gafira-render` change the port contract does not allow today.
- **ARQ-113 cannot be closed by this change.** The oracle exists after `CB3`, and the comparator
  already exists in `gafira-testkit::golden`; the *suite* ARQ-113 describes (a golden per node type,
  effect, blend mode and text run) needs the document node kinds and the exporters that do not
  exist. This change makes the suite possible and proves its mechanism on one scene.

## Delivery table

Forecast is authored lines, with the x1.4 planning figure in brackets. Every delivery stays under
~700 lines at x1.4 and passes, on its own, the gates of ADR 0017:

```
cargo fmt --check
cargo clippy -p gafira-render-cpu --all-targets --locked -- -D warnings   # leaf crate scope
cargo test   -p gafira-render-cpu --locked
```

The scope column names the exception: `SC1` and `PC1` touch shared files and therefore run the same
three commands with `--workspace` (ADR 0017: anything that touches `gafira-ports`, `gafira-render`'s
dependents, `Cargo.toml`, `Cargo.lock` or `ci.yml`). The full workspace suite runs once at the
delivery HEAD of the change and once before the `develop` → `main` PR, not per commit.

| Delivery | Scope | Requirements | Depends on | Mode | Est. lines | Acceptance moved |
|---|---|---|---|---|---|---|
| `SC1` | `crates/gafira-render-cpu/`: `Cargo.toml`, `src/lib.rs` (doc comment, `#![forbid(unsafe_code)]`), `clippy.toml` copy; root `Cargo.toml` (`gafira-render-cpu`, `tiny-skia`); `ci.yml` V5 row; `Cargo.lock` | ARQ-01, ARQ-02, ARQ-03, ARQ-05, ARQ-06, ARQ-70, ARQ-71, ARQ-107; 02 §2.1 | — | serial, `--workspace` | 120 (170) | none |
| `PC1` | `gafira-ports`: `TileTask.region`, `TargetDesc.origin`; `gafira-render`: the internal plan carries the viewport origin, `into_ports_plan` passes both, `page.rs` sets the target origin; the test files that construct them | 02 §4.6, §6.1; ARQ-121, ARQ-142 | `SC1` (nothing of the crate is used, but it is the same change) | serial, `--workspace`, single writer | 220 (310) | enables ARQ-42 and CAL-11 |
| `CB1` | `gafira-render-cpu`: `CpuBackend` (`caps`, `begin_frame`, `execute`, `read_back`, `memory_usage`, `trim`), the surface, the tile loop with cache and cancellation, op culling, the coverage module (kurbo → tiny-skia mask), `FillPath` in both formats | 02 §4.6, §6.2; ARQ-117, ARQ-119, ARQ-139, ARQ-143, ARQ-44 (partial), ARQ-111 | `PC1` | serial in `CB` | 460 (644) | ARQ-42 (the two formats exist) |
| `CB2` | `gafira-render-cpu`: layer stack, `PushLayer`/`PopLayer`, opacity, the 16 blend modes through `gafira-raster`, nesting, tile-local buffers, buffer reuse | ARQ-119, ARQ-117 | `CB1` | serial in `CB` | 340 (475) | ARQ-42 (the golden's scene needs groups) |
| `CB3` | `gafira-render-cpu` tests: the ARQ-42 comparison (preview vs export, both qualities, scale 1.0), the sRGB comparator with ARQ-117 rounding, the FD-55 (b) transparency case, one stored golden of ARQ-113's mechanism | ARQ-42, ARQ-113; FD-55 (b) (c) | `CB2` | serial in `CB` | 300 (420) | **ARQ-42**, ARQ-113 (mechanism) |
| `CB4` | `gafira-render-cpu`: `DrawImage`, affine resampling, bilinear filter, both formats | 02 §6.1; ARQ-121, ARQ-117 | `CB2` | serial in `CB` | 360 (505) | ARQ-113 (the image case of the oracle) |
| `CB5` | `gafira-render-cpu`: `ApplyMask`, `ApplyEffect` with the ROI-grown tile, `ApplyTransition` | ARQ-118, ARQ-119, ARQ-120, ARQ-112 | `CB2`, `CB4` | serial in `CB` | 420 (590) | ARQ-113 (the effect case of the oracle) |
| `CB6` | `gafira-render-cpu` benches: `bench_export_png_1080` (criterion, `harness = false`), the cancellation gap measurement, a plain `#[test]` guard that asserts 2 s only when the build is optimised | CAL-11, ARQ-40, ARQ-114, ARQ-43 | `CB4` | serial in `CB` | 260 (365) | **CAL-11** (measured) |
| `CB7` | `gafira-render-cpu`: parallel row bands inside one task, with an injected `WorkerPool` (no thread is created here) | ARQ-138; CAL-11 | `CB6` | serial in `CB`, **conditional** | 300 (420) | CAL-11 (the margin, only if `CB6` is over) |

Total: 2 780 authored lines (x1.4: 3 892). The largest delivery is `CB1` at 460 (x1.4: 644).

`CB7` is cut **only** if `CB6` measures above 2 s on the reference profile. It is listed so the
coordinator does not have to invent it under time pressure, and so that the owner sees the cost of
the sequential port contract (`D12`). If `CB6` is inside the budget, the change closes at `CB6`.

### Why each mark holds

| Mark | Ground |
|---|---|
| `SC1` serial, first | It edits every shared file at once: the root `Cargo.toml`, `Cargo.lock`, the V5 table. The V5 step fails for a crate without a row, the V12 step fails for a crate in the table without a byte-identical `clippy.toml`, and `cargo build` fails for a crate without a `[workspace.dependencies]` entry. |
| `PC1` serial, second | It edits `crates/gafira-ports/src/render.rs` (two fields), the only shared source file in the change, plus five files that construct them across `gafira-ports` and `gafira-render`, tests included. One writer, and the gate is `--workspace` (ADR 0017). |
| `PC1` before `CB1` | `CB1` cannot place a tile without the origin, and this design will not have the adapter guess the grid geometry from the tile key (`D1`, alternative b). |
| All `CB*` serial | They share `crates/gafira-render-cpu/src/lib.rs` (the module list) and the manifest. Layer-3's `RE2`/`RE3` were marked serial for the same reason. |
| `CB2` after `CB1` | The layer stack composes the fill `CB1` produces; there is nothing to composite before `CB1`. |
| `CB3` after `CB2`, before `CB4` | The ARQ-42 golden needs fills and layers only. Landing it before images and effects means the acceptance is proven while the diff is small and the failure is diagnosable. It also becomes the regression net for `CB4`–`CB7`. |
| `CB4` and `CB5` after `CB2`, `CB5` after `CB4` | Both extend the pixel paths `CB2` fixed; `CB5`'s effect ROI grows a tile that already contains images in real documents. Neither blocks `CB6`. |
| `CB6` last but for `CB7` | A bench measures what exists; it must be cut on a HEAD where nothing under it changes, or its baseline is meaningless. |
| `CB7` conditional | It exists only to buy the CAL-11 margin, and it changes the determinism argument (`ARQ-111`: identical bytes with 1, 2 and 4 threads). Cutting it without a measurement is risk for no gain. |

## What this change cannot prove

Stated up front, per delivery, because these are the lines the hand-mutant lists must not blur.

| Delivery | Cannot prove | Why |
|---|---|---|
| `SC1` | That the crate is pure in the ARQ-02 sense | The lint is there (V12 requires the copy); it proves syntax, not absence of side effects. Purity here is trivial: the code opens no file and reads no clock because it has nothing to open. |
| `PC1` | That a backend *uses* the origin | It proves the data travels (planner → port → recording backend). `CB1` proves the pixel consequence. |
| `CB1` | That the user sees the page in the right place, end to end | There is no UI and no composition root. The test uses a `PageRenderer` and a region at the origin; a region with a non-zero
origin is exercised only at the plan level, and whether a caller ever passes one is the
composition root's question. |
| `CB1` | Deadline behaviour under real load | The deadline is checked per task (the loop) and per row band (here). No test can prove a wall-clock deadline in CI; FD-55 (d) is what measures the cancel gap, in `CB6`. |
| `CB2` | Blend modes beyond the ones the graph can emit | `NodeKind` has three kinds and only `Shape`/`Group` carry a `blend`. All 16 are implemented through `gafira-raster` (ARQ-119 is already proven there); the adapter proves it *calls* them with the right operands, not how they look. |
| `CB3` | ARQ-42 beyond its scene set | The tolerance is measured on the scenes `CB3` builds. Arbitrary documents (deep nesting, many semi-transparent overlaps) accumulate 8-bit rounding; `XP2` measures how far. |
| `CB3` | That the preview and the export share a graph | That is layer-3b's proof (`apply-progress.md` of `2026-11-05-layer-3b-render`, RE5). Here the two renderers are constructed by the test and could differ; what is proven is that the pixels agree. |
| `CB4` | That an image node renders correctly | No document node produces `DrawImage` (ARQ-134). The resampler is proven against analytic cases (a 1×1 image, an axis-aligned integer scale, a rotation of a checkerboard), not against a user's document. |
| `CB5` | Mask placement, transition semantics, and cancellation inside an effect | The port carries neither a mask rect nor the two inputs of a transition (`D7`). The readings are provisional; nothing emits them, so nothing contradicts them either — which is the risk. And `Effect::run_cpu` receives no `CancelToken` (ARQ-112), so a long effect on a grown tile cannot answer ARQ-43's 100 ms; the checkpoints are around it, not inside. |
| `CB6` | CAL-11 as a product number | It measures render + encode with `png` as a stand-in for `gafira-io-image`, single-threaded. The criterion bench runs under `cargo bench` (bench profile); the 2 s guard runs under `cargo test`, in debug, so it is skipped unless the build is optimised. The printed criterion number is the number; the gate does not exist (`D10`). |
| `CB7` | That parallelism is worth it | It proves identical bytes at 1, 2 and 4 threads (ARQ-111) and reports a number. Whether the wall clock improves enough to keep the code is a coordinator decision on `CB6`'s and `CB7`'s numbers. |

## Decisions

Each decision follows the shape of the layer-3 design: what it costs, and whether it is reversible.

### D1. The port carries the place of a tile: `TileTask.region` and `TargetDesc.origin`

**What the code says.** `ports::TileTask` is `{ key, priority, ops }` and `ports::TargetDesc` is
`{ width, height, format }`. The ops are in **page** coordinates (a `FillPath` carries the page-space
`Placement` transform; the page background of `page.rs` is the page rect under `Affine::IDENTITY`),
while the surface is `request.region` translated to its own origin (`target_region` in `page.rs`
returns `(0, 0, w, h)`). A backend therefore cannot know *which* page rectangle a tile covers, nor
where the surface sits in the page. The layer-3b implementation record already names this: "A region
with an offset is planned correctly (the tiles clip to it) but the offset itself is not carried to
the backend".

Options: **(a)** add `region: IRect` (page space, clipped to the viewport) to `TileTask` and
`origin: (i32, i32)` to `TargetDesc`; **(b)** let the backend derive the tile rect from
`TileKey { column, row, level }` plus `TILE_SIZE`, so only `origin` is needed; **(c)** pre-translate
every op to surface coordinates in the planner, so `region` alone suffices.

Decision: **(a)**. Reason: `PlannedTile.region` already holds exactly the value, clipped, and it is
dropped only in `into_ports_plan`; passing it costs one field. (b) re-derives the grid law of ARQ-121
inside an adapter — a second source of truth for `TILE_SIZE` and the level scale, on the one crate
that must not know `gafira-render` (ARQ-06). (c) makes each op viewport-dependent, so the same tile
key would cache a raster that depends on the scroll position: it breaks ARQ-142's content-addressing.

Cost: two public fields on structs of a layer-3 shared crate; five files construct them
(`gafira-ports/tests/render.rs`, `gafira-render/src/planner.rs`, `gafira-render/src/page.rs`,
`gafira-render/src/frame.rs`, `gafira-render/tests/ports_doubles.rs`); `--workspace` on that
delivery. The doc comments must state the semantics: `region` is page space clipped to the viewport,
`origin` is the page-space position of the surface's top-left pixel.

Reversible: additive, but not free — see "What is reversible".

### D2. tiny-skia for coverage; `gafira-raster` for the pixels. The backend composes, it does not rasterize paths itself

**What already exists.** `gafira-raster` has every pixel primitive the backend needs and no path
rasterizer: `ImageBuf`/`PixelBuf`/`ImageView` in `Rgba8` (sRGB, premultiplied), `RgbaF32` (linear,
premultiplied) and `Alpha8`; `premultiply_u8`/`unpremultiply`; `blend_u8` and `blend_f32` for all 16
modes (linear light, D-53); `AlphaMask`; mips; effects and transitions with their `run_cpu`
contracts. `gafira-pathops` has booleans, stroke and hit testing — no filling. Nothing in the
workspace turns a `BezPath` into coverage, so the backend needs one primitive from outside.

Options: **(a)** tiny-skia (`Mask::fill_path` → 8-bit coverage; `PathBuilder` from kurbo) plus
`gafira-raster`'s blends; **(b)** vello_cpu (02 §9.2 lists it as the CPU alternative to tiny-skia;
`cargo-deny`-verified); **(c)** a scanline coverage pass of our own; **(d)** tiny-skia for the whole
pixel path, including blending.

Decision: **(a)**, which is what 02 §2.1 names ("tiny-skia y compositor f32 propio"). Reason: ARQ-119
forbids reimplementing the blend formulas ("`gafira-render-cpu` las usa, no las redefine"), so (d) is
out by spec; (c) is a scanline rasterizer project of its own, with an AA quality nobody has measured;
(b) is a newer and less stable API for the same job, and it would leave the spec's named library
unbuilt. tiny-skia 0.12.0 is BSD-3-Clause, already on `deny.toml`'s allow list, and present in the
local registry (`~/.cargo/registry/src/*/tiny-skia-0.12.0`) — no network needed.

Dependency shape: `tiny-skia = { version = "0.12", default-features = false, features = ["std", "simd"] }`
— dropping `png-format` keeps the `png` codec out of the product graph (it stays a dev-dependency).

Cost: one new external dependency, its transitive tree (`tiny-skia-path`, `arrayref`, `arrayvec`,
`bytemuck`, `log`), and `THIRD_PARTY_NOTICES.md` when that generator lands. The kurbo → tiny-skia
path conversion is ours (~40 lines) and is where arcs do not exist (kurbo's `BezPath` has none).

Reversible: yes, behind `src/coverage.rs`.

### D3. Coverage is 8-bit in both qualities

Options: **(a)** tiny-skia's 8-bit AA coverage shared by `Preview` and `Export`; **(b)** 8-bit for
`Preview` and an analytic f32 coverage for `Export`.

Decision: **(a)**. Reason: ARQ-42 compares the preview with the export. A finer coverage in the
export would make every anti-aliased edge differ by more than 1/255 by construction — the export
would be *better*, and would fail the acceptance. Sharing the 8-bit mask also means the two paths
differ only by the colour quantisation, which is exactly the quantity the tolerance measures.

Cost: the export's AA is 8-bit, so a future 16-bit or analytic export needs a new acceptance. That is
the "not reversible" item above.

Reversible: no, by design.

### D4. One surface, page-sized, one surface pixel per page pixel; tile-local layer stack

The surface is `TargetDesc`-sized (33 MB at 1080p in `RgbaF32`, 133 MB at 4K). It is the same size the
port already implies (`read_back` returns `region`-sized pixels), so no choice exists there.

The **layer stack** is the choice. Options: **(a)** one buffer per open layer, allocated at the tile's
size and reused across tiles from a per-depth pool; **(b)** one buffer per open layer at surface
size; **(c)** composite directly into the surface with a saved backdrop.

Decision: **(a)**. Reason: (b) costs `depth × 33 MB` for a page with a hundred groups, and D-03's
1.5 GB cap (which ARQ-44 makes the caches respect) does not leave room for that. (c) requires a copy
of the backdrop per layer, which is the same memory at surface size. With (a) the ceiling is
`depth × 256 KiB` (`Rgba8`) or `depth × 1 MiB` (`RgbaF32`), and `trim` drops the pool.

Consequence to state: `PushLayer` carries no bounds, so the adapter defines the layer region as the
tile's region (grown by the effect ROI in `CB5`). A layer is therefore never larger than a tile.

At a level above zero, a level-`L` tile covers `256 << L` page pixels in a 256×256 raster, so the
adapter draws it scaled by `2^L` into the surface, whose pixels stay page pixels. At 100 % (level 0)
the mapping is 1:1, which is what ARQ-42 compares.

Cost: none in exactness for the blends themselves — every mode in `gafira-raster` is a per-pixel
function of source and backdrop, so a group composited tile by tile equals the group composited whole,
as long as each tile's layer holds the page content clipped to that tile (which it does: the ops are
page space and the mask is the clip). A seam can only appear when a pixel depends on pixels outside
its tile: that is a filter, whose `roi` grows the tile in `CB5`, and a whole-frame transition, whose
`run_cpu` sees one tile at a time (`D7`). The second is a real limitation and is stated in "What this
change cannot prove".

Reversible: yes (the pool is internal), at a cost in memory or in the hidden seam.

### D5. The backend validates the format of a cached tile

`TileKey` is `(ContentKey, level, column, row)` — no quality, no format. `ContentKey` is derived from
the node and its sampled parameters, and layer-3b's record says the two qualities have *different*
keys; but the cache trait is shared, and a caller can hand one `TileCache` to a preview renderer and
an export renderer. A cached `Rgba8` tile read into an `RgbaF32` frame would be silently wrong.

Options: **(a)** on `get`, compare the returned `ImageBuf`'s `PixelFormat` with the frame's and treat
a mismatch as a miss (overwriting on `put`); **(b)** add the format to `TileKey` (another port
change); **(c)** trust the caller.

Decision: **(a)**. Reason: it is three lines, it is testable, and it keeps `PC1` to the two fields
that a backend cannot work around. (b) is a second shared-crate change for a hazard the adapter can
absorb; (c) is how a golden turns into a heisenbug.

Cost: one comparison per tile, and a re-render when the cache is shared across qualities.

Reversible: yes.

### D6. The tile mask is the clip; ops are culled by bounds

Options: **(a)** the coverage mask is the tile's size, so every op is drawn into it and pixels outside
are simply not touched; **(b)** an explicit clip rect per op.

Decision: **(a)**, plus a bounds cull: an op whose page-space bounds do not meet the tile's region is
skipped before conversion, and a `FillPath` is converted with the tile-origin translation
pre-multiplied into the path transform.

Reason: `paint_background` inserts the **page rect** into *every* task. Without a cull, a 1080p
export converts and scans the same page-sized path 40 times; with it, each tile converts a path and
scans only its own 256×256 window. This is also the difference between CAL-11 and CAL-11 × the tile
count.

Cost: a bounds test whose `BezPath::bounding_box` is exact enough (kurbo's control-point box is a
superset — correct for culling, conservative). The cull must use the **transformed** box.

Reversible: yes.

### D7. `ApplyMask`, `ApplyEffect`, `ApplyTransition`: the port under-specifies two of them, and the design says which reading it takes

`ApplyEffect { effect, params, scale }` is fully specified by `Effect::roi`, `cost_hint` and
`run_cpu`. Decision: the adapter computes `roi(cx, p, tile_rect)`, renders the task's ops into a
scratch buffer of that size (offset by the ROI origin, with the same coverage and layer machinery),
runs `run_cpu` with `EffectCx { scale, tile: None }`, and blits the crop. Cost: a second pass over the
grown area for the tiles of an effected node; exact at seams, which is the point of `roi`.

`ApplyMask { mask }` carries an `AlphaMask` (width and height) and **no rect**, and `PushLayer`
carries **no bounds**. Decision: the mask is anchored at the layer's top-left, one mask pixel per
layer pixel, clipped to the overlap; a mask smaller than the layer masks only its own rectangle.
`ApplyTransition { transition, params, progress }` carries **no boundary between its two inputs**.
Decision: `from` is the backdrop under the current layer and `to` is the current layer's pixels; both
are the layer's region, and `run_cpu` replaces the layer with the result.

Why not change the port now: nothing emits either op (`RenderGraphBuilder` emits three variants), so a
richer op would be a guess with no test — and a wrong guess in a shared crate is worse than a
provisional reading in one adapter. The change that makes the graph emit masks or transitions is
where the field belongs (the video work behind the layer-3 O1, not this change).

Cost: two readings that a later change will have to confirm or replace; both are recorded as `SP2` and
`O5`. The transition reading is the weaker one: with a `from` that is the composited backdrop, a
correct two-clip transition is not expressible, and no test here can show the difference.

Reversible: yes for the adapter; the ops' shapes are the port's problem.

### D8. The backend is pure, single-threaded, clockless and budget-aware

`gafira-render-cpu` takes the canonical `clippy.toml` copy because V12 requires it for every crate in
the V5 table: no `std::fs`, no `std::thread::spawn`, no `Instant::now`, no `std::env`, no `HashMap`.
The deadline arrives as `ports::Deadline` + the caller's clock (`FrameLoop` already enforces the
per-tile check), cancellation as `CancelToken`.

Decision: check the token at the start of `execute` and every 64 rows of any full-tile pass
(coverage conversion, compositing, resampling). 64 rows of a 256×256 tile is a few microseconds; the
100 ms of ARQ-43 is met with three orders of magnitude of slack, and the check is cheap enough not to
move CAL-11. The one pass that cannot be interrupted is `Effect::run_cpu`/`Transition::run_cpu`:
ARQ-112 keeps both outside the cancellation row (no token in their signatures), so the adapter checks
before and after and not inside. A single effect on a grown 4K tile can therefore exceed 100 ms and
nothing in this change fixes that.

Decision: no thread is created, and no global pool is used. `CB7` takes a pool as a parameter, as
ARQ-138 requires of the layer-3 crate.

Cost: the whole change is single-threaded until `CB7`, so CAL-11's margin comes from culling, buffer
reuse and avoiding copies — not from cores.

Reversible: yes.

### D9. The ARQ-42 comparator: the export is encoded to sRGB8 with the same rules the preview is stored with

The preview surface is `Rgba8` sRGB premultiplied; the export is `RgbaF32` linear premultiplied.
`gafira-testkit::golden::Image` is 8-bit only and `quantize` is `round(c × 255)` — it does not encode
sRGB, so it cannot convert the export by itself.

Decision: `CB3` converts the export tile by tile with the rules ARQ-117 fixes: unpremultiply
(`a = 0` → `(0, 0, 0, 0)`), encode with the sRGB curve (`LinearRgba::to_srgb`, the same function
`gafira-color` gives the preview path), premultiply with `(c × a + 127) / 255`, and compare with
`gafira_testkit::golden::compare(Mode::CrossPlatform, preview, encoded_export)` — which allows
`pixels / 1000` differing pixels at more than 1 level, i.e. 99.9 % at ≤ 1/255.

Reason: ARQ-42's criterion is a statement about the two qualities, not about the transfer function;
sharing the transfer isolates the raster. Using the golden comparator rather than a bespoke loop
means the tolerance rule of §10 has one implementation.

Cost: the comparator is test code in `CB3`; if the transfer were wrong, this test would not see it
(the preview uses the same function).

Reversible: yes.

### D10. CAL-11 is measured in-crate, named exactly `bench_export_png_1080`, and not gated

Options: **(a)** a criterion bench in the adapter's `benches/`, rendering 1080p through
`GraphPageRenderer` in `Export` and encoding with `png` (`harness = false`); **(b)** wait for
`gafira-io-image` and measure the real exporter; **(c)** a `#[test]` with a 2 s assert only.

Decision: **(a)** plus a plain guard test that asserts the 2 s **only when the build is optimised**
(`!cfg!(debug_assertions)`) — otherwise a debug CI run fails for the wrong reason. Reason: the number
must exist before the exporter does; the bench name is the one CAL-11 and ARQ-114 require. (b) leaves
CAL-11 unmeasured for another change and couples this crate to layer 4. (c) makes a machine-dependent
assertion a merge gate, which ARQ-114 explicitly forbids ("No son una puerta de `cargo test`").

Cost: `criterion` and `png` as dev-dependencies (both already in `[workspace.dependencies]`); the
number is render + encode without the file write and the validation of FD-58.

Reversible: yes.

### D11. The golden and the bench take `gafira-render` as a **dev**-dependency

`ARQ-06` says an adapter depends on `gafira-ports` and layers 0–2, never on `gafira-render`; the V5
step enforces it over `cargo tree -e normal,build`, which excludes dev-dependencies. The ARQ-42
comparison needs two `GraphPageRenderer`s, and CAL-11 names the real export path.

Options: **(a)** dev-dependency from `gafira-render-cpu` to `gafira-render`, tests in the adapter;
**(b)** the ARQ-42 test lives in `crates/gafira-render/tests/` with `gafira-render-cpu` as *its*
dev-dependency; **(c)** compare the two qualities on the same `RenderPlan` without `PageRenderer`.

Decision: **(a)**, and the library code never names the crate. Reason: the delivery stays inside its
own folder (one writer, no collision with the concurrent `gafira-render` branch); (b) puts the
adapter's acceptance in a crate this change is told not to touch; (c) is weaker than the sentence of
ARQ-42, which says "the preview", i.e. the page renderer at scale 1.0.

Cost: a dev-only edge in the dependency graph that the V5 step cannot see — recorded as `O2` because
it is a rule question, not a technical one.

Reversible: yes.

### D12. The port's `execute` is one tile per call, so parallelism cannot be bought here

`FrameLoop::frame` builds `RenderPlan { tasks: vec![task] }` per tile and calls `execute` per tile. A
backend that wanted four tiles at once cannot ask for them. `CB7` therefore parallelises **row bands
inside one tile**, which scales with the tile's height and not with the page.

Decision: accept the sequential contract in this change; if `CB6` is over 2 s, cut `CB7` and report
the number to the owner with the note that the *correct* fix is a `gafira-render` change (a batched
`execute` or a pool handed to `FrameLoop`). Reason: changing the frame loop is a layer-3 change with
its own budget semantics (02 §6.2's cumulative deadlines, `left > 0`), and it does not belong in the
adapter's change.

Cost: CAL-11's technique ("tiles paralelos", 02 §6.7) is not the one measured. Recorded as `O4`.

Reversible: yes.

## Critical code

Hand mutants apply to all of these; `tdd-apply` receives the rule and the wrong behaviour, not the
code. T4 at close covers `gafira-render-cpu`, `gafira-ports` and `gafira-render` (the three crates the
change edits; `PC1` is the only one that touches source outside the new crate).

| Delivery | Critical | Not critical |
|---|---|---|
| `SC1` | None: configuration, checked by V5, V12, the ARQ-107 step (`cargo tree` and the `Locale` grep) and `cargo deny` | All |
| `PC1` | The viewport origin stored in the plan, the region carried unchanged (clipped, not re-derived), the target origin from the request region, the `is_empty` cases | Doc comments |
| `CB1` | The page → tile translation, the coefficient order of kurbo → `tiny_skia::Transform`, the coverage α used as `source` alpha, the two format paths (`blend_u8`/`blend_f32`) using the same operands, the premultiply/unpremultiply rules of ARQ-117, the bounds cull, the cache hit/miss and format check, the placement rect and its clipping to the target, the cancellation checkpoints, `left` on a mid-tile deadline, `memory_usage` accounting, `trim` dropping the pool | Benches, `Display` of errors |
| `CB2` | The push/pop depth and the buffer index, the pop blending with `(blend, opacity)` instead of `Normal`, the opacity applied once (not to each child), the layer cleared to transparent at push, the nested pop order, the reuse of a buffer after a pop (no residue) | The buffer pool's growth policy |
| `CB3` | The comparator's unpremultiply/encode/premultiply order, the premultiply rounding of ARQ-117, the same region on both renderers, the 99.9 % budget of the mode, the transparency case (α = 0 where nothing is drawn) | The golden file's name |
| `CB4` | The inverse transform of the sample point (page → image), the bilinear weights and the edge clamp (outside → transparent black, ARQ-120), the premultiplication of the source sample, the destination clip | The filter's code shape |
| `CB5` | The ROI computation and the origin offset back to the tile, the effect's destination region, the mask anchor and its overlap clip, the transition's `progress` clip to `[0, 1]` (ARQ-118) and its `from`/`to` choice, the cancellation checkpoint inside the grown pass | The effect's parameters |
| `CB6` | The bench's scene (fixed document, fixed region, fixed output size), the encode path (same rules as `CB3`), the measured unit (one page, no warm-up guesswork) | The criterion configuration |
| `CB7` | Identical bytes at 1, 2 and 4 threads, the band boundary (no row lost, no row doubled), the cancel check per band | The pool's size default |

## Shared files

| File | Touched by | Resolution |
|---|---|---|
| `crates/gafira-ports/src/render.rs` | `PC1` | Single writer. Two fields, no trait change. No other file of `gafira-ports` changes. |
| `crates/gafira-render/src/{planner,page}.rs` and the test files that construct the two structs | `PC1` | Single writer, and only after the in-flight branch in that crate is merged (see "Dependency state"). Nothing else in `gafira-render` changes in this change. |
| `Cargo.toml` (root) | `SC1` | Adds `gafira-render-cpu` and `tiny-skia` to `[workspace.dependencies]`. Nothing else. |
| `Cargo.lock` | `SC1` | Never hand-merged: take `develop`'s file, `cargo check --workspace --all-targets`, then the gates with `--locked`. |
| `.github/workflows/ci.yml` | `SC1` | One V5 row, nothing else. The ARQ-107 step and V12 read the crate list from that table, so no other edit is needed. |
| `crates/gafira-render-cpu/clippy.toml` | `SC1` | New file, byte-identical to `crates/gafira-geom/clippy.toml`; V12 compares it. |
| `deny.toml` | Nobody | BSD-3-Clause is already allowed; `XP3` confirms the transitive tree. |
| `crates/gafira-render-cpu/src/lib.rs` | `CB1`–`CB7` | Serial by order: each delivery adds its module to the list. This is the reason all `CB*` are serial. |
| `.cargo/mutants.toml` | The T4 fix rounds | Exclusions only with a written reason, one regex per mutant. |

## The V5 row

The table lives in the `policy` job, step "Domain crates respect the layer table (V5)", and its shape
is `crate  allowed-internal-deps`, whitespace-separated, `-` for none. The row to add, after the
`gafira-media` row and before the closing quote:

```
gafira-render-cpu gafira-ports gafira-doc gafira-geom gafira-color gafira-raster
```

Why exactly those five, and nothing else:

| Dependency | Why it must be there | Why nothing more |
|---|---|---|
| `gafira-ports` | It *implements* `RenderBackend`; the plan, the ops, the cache, `TargetDesc`, `RenderError` all live there. | — |
| `gafira-raster` | ARQ-119: the blend formulas are used, not redefined; the buffers (`ImageBuf`, `AlphaMask`), the effects and the transitions come from there. | — |
| `gafira-doc` | `RenderOp` carries `BlendMode` and `ParamValues`, and `CancelToken`/`Progress` live in `gafira-doc::contract` (ADR 0005). | — |
| `gafira-geom` | `IRect` for regions, `Affine` for the op transforms. | — |
| `gafira-color` | `Color` from `FillPath`, `Rgba`/`LinearRgba` for the two format paths. | — |
| not `gafira-render` | ARQ-06: an adapter never depends on the layer-3 render crate. The place of a tile arrives through the port (`PC1`), not through the planner. | The V5 step greps `cargo tree -e normal,build`, so the `D11` dev-dependency does not appear here — and must not leak into `[dependencies]`. |
| not `gafira-time`, `gafira-anim` | The deadline is a `Duration` and the instant belongs to the caller (ARQ-02, 02 §4.6); the backend samples nothing at `t`. | Two fewer crates to recompile per mutant. |
| not `gafira-pathops`, `gafira-text`, `gafira-layout` | Coverage comes from tiny-skia; no text is shaped here (no `TextNode`); the placement is already page space (ADR 0009). | — |

The same step's `forbidden` regex is `egui|eframe|winit|wgpu|tokio|mio|socket2|reqwest|ureq`;
`tiny-skia` matches none of them. Two further requirements come from the V5 table being the one list
of domain crates: the crate must carry a `clippy.toml` identical to the canonical copy (V12), and the
ARQ-107 step bans `gafira-i18n`/`icu_*` in its tree **and the substring `Locale` in its files** — so
no identifier, comment or doc line in `crates/gafira-render-cpu/` may contain that word. Neither
constraint costs anything real here; both fail the build if forgotten.

## Open questions (owner)

| # | Question | Options and tradeoffs | Recommendation |
|---|---|---|---|
| `O1` | `PC1` is a two-field change to `gafira-ports` *and* an edit inside `gafira-render` (planner, `page.rs` and their tests) — the crate another worker holds right now. Is it part of this change, and in which order? | (a) In this change, cut and merged **before** the other worker's branch (they rebase). (b) In this change, merged **after** theirs, so `CB1` starts only once it lands (the same wait, recorded in the schedule instead of in the history). (c) Its own small change now, merged first, and this change starts from it. | (c) is the cleanest history and the shortest blocker, but costs a change folder and a verify round for ten lines of source. (a) is the pragmatic one if the other branch is not about to merge. Either way the adapter cannot start `CB1` before it lands: without the origin, only the export at region (0,0) is correct. |
| `O2` | `D11`: the adapter's tests need `gafira-render` as a **dev**-dependency. ARQ-06's letter is "adapters depend on `gafira-ports` and layers 0–2"; the V5 step cannot see dev edges. Is that acceptable, or does the golden move into `gafira-render`'s tests? | (a) Accept the dev edge (tests only, library never names it). (b) Move the ARQ-42 comparison to `crates/gafira-render/tests/`, which needs that crate's folder and the other worker's branch. (c) Amend ARQ-06 to say "normal and build dependencies". | (c), with (a) until it is amended: the rule should state what it means, and the V5 step already implements it that way. |
| `O3` | CAL-11 is measured here and not gated (`D10`): no `perf-budgets.toml`, no reference-machine job, and the PNG encoder is a stand-in for `gafira-io-image`. Does the change close with a number, or must the gate come first? | (a) Close with the measured number and the bench in place; the gate arrives with ARQ-40's change. (b) Build `perf-budgets.toml` and the nightly job in this change (a second deliverable family, no product code). (c) Leave CAL-11 open until FD-55's exporter exists and measure it there. | (a): the number is what CAL-11 needs to stop being invisible, and the gate is a CI-infrastructure change that would otherwise delay the oracle by weeks. |
| `O4` | The "tiles paralelos" technique of 02 §6.7 cannot be expressed through `RenderBackend` (`D12`): `FrameLoop` hands one task per `execute`. Does a `gafira-render` change for a batched/pooled frame loop go into the queue next, or does the CPU backend stay sequential for the MVP? | (a) Sequential for the MVP; `CB7` only if the number is over. (b) A `gafira-render` change now, before the GPU backend, so both backends get the batch. (c) Change the trait: `execute` takes the whole plan and the deadline, and the loop stops iterating. | (b) after this change closes: it is the right shape for both backends, and it is a layer-3 change with its own budget argument. (a) until then. |
| `O5` | `D7`: `ApplyMask` has no rect, `PushLayer` has no bounds and `ApplyTransition` has no boundary between its inputs. Are the provisional readings acceptable, or should the port be extended now (before any emitter exists)? | (a) Provisional readings in the adapter, recorded, and the port extended by the change that emits them. (b) Extend the port now: a mask rect, layer bounds, and the two input spans of a transition — untested shapes with no emitter. (c) Leave the three variants unimplemented (not possible: `RenderError` has no `Unsupported`, and the closed enum forces a reading). | (a). The risk of a wrong shared type is higher than the risk of a documented reading in one adapter. |
| `O6` | 02 §2.1 names tiny-skia for this crate, and 02 §9.2 lists vello_cpu as the CPU alternative. tiny-skia 0.12 is chosen (`D2`). Confirm, or does the newer f32-native API of vello_cpu change the choice? | (a) tiny-skia, as §2.1 says; coverage only, the pixels stay `gafira-raster`'s. (b) vello_cpu: f32 targets would make the export's AA independent of the preview's, which breaks `D3` and ARQ-42's tolerance. (c) Both, behind a feature: a second AA path to keep in sync. | (a). And `D3` is the argument that matters: an f32-native rasterizer in the export is what ARQ-42 must *not* have. |

## Needs explorer

Provisional claims. Nothing is built on any of them until it is measured. `XP3` runs before `SC1`
verifies, `XP1` and `XP4` before `CB1`, and `XP2` before `CB3`.

| # | Claim | Check | Blocks |
|---|---|---|---|
| `XP1` | A single-threaded CPU render of a 1080p page, with a few groups and a page background, stays under CAL-11's 2 s with margin. Unmeasured: the backend does not exist, and `blend_f32` is a per-pixel `f64` call with a finite check. | A throwaway bench over the existing crates: `tiny_skia::Mask::fill_path` for the 40 tiles of a 1080p page, with and without the `D6` cull, and `gafira_raster::blend_{u8,f32}` over 2.07 M pixels × 3 layers. Report the three costs apart. | `CB1`, `CB6`, `O4` |
| `XP2` | The 8-bit preview and the f32 export agree within 1/255 on 99.9 % of the pixels for scenes with up to three nested semi-transparent layers. Unmeasured: each `blend_u8` rounds to nearest, and the roundings accumulate. | A loop over the scene family `CB3` will use: 1–3 nested groups with alpha 0.5–0.9 and a full-opacity shape on top, both paths, counting pixels that differ by 2 or more after the `D9` conversion. | `CB3`, `D3` |
| `XP3` | `tiny-skia` 0.12 builds and passes `cargo deny` on the three targets, and its output is byte-identical across those targets for the same input at the same level of `simd`. | `cargo deny check` on the three targets, a build on each, and a hash of one rendered tile per target. | `SC1` |
| `XP4` | The surface plus the layer pool fits ARQ-44's budget: 33 MB at 1080p, 133 MB at 4K, plus `depth × 1 MiB`; and FD-55's 16 384 px side (4.3 GB in `RgbaF32`) is either feasible or a stated limit. | Arithmetic against 02 §6.6's machine (8 GB) and the 1.5 GB cap of D-03; a single allocation test at the 16 384 px side. | `CB1`, and the "cannot prove" table of `CB1` |

## Needs specifier

02 does not say enough for some of this. Each row is a request, not a rewrite.

| # | ID | Request | Blocks |
|---|---|---|---|
| `SP1` | 02 §4.6, §4.17 | State that a tile task carries its page-space region and that a target carries its page-space origin, with the units and whether the region is clipped to the viewport. Today §4.6 describes the plan without the place of a tile. | `PC1`, `CB1` |
| `SP2` | 02 §4.6 | Say what `ApplyMask` masks, what `PushLayer` bounds are, and where a transition's two inputs begin and end in the op stream. Three ops are unimplementable as written; today only the adapter's reading (`D7`) exists. | `CB5`, and the change that first emits them |
| `SP3` | 02 §2.1, §6.2 | Add a §4.17 row set for `gafira-render-cpu` (the "first cut / outside the cut" table the six layer-2 engines have): which ops, which qualities, which formats. | Records this design; nothing blocks |
| `SP4` | 02 §6.7, CAL-11 | Reconcile "render CPU en tiles paralelos" with `RenderBackend::execute` being handed one tile per call (`D12`). Either the technique changes or the contract does. | `CB6`, `CB7`, `O4` |
| `SP5` | 02 §6.7, ARQ-40 | Name the owner of `perf-budgets.toml` and the reference-machine job, and whether a bench whose number exists but is not gated is acceptable while that job is unbuilt. | `CB6`, `O3` |
| `SP6` | §10 | Say in which space the ARQ-42 tolerance is measured (8-bit sRGB premultiplied, as `D9` reads it) and that the export is encoded with ARQ-117's rules before comparing. | `CB3` |

## Next

The coordinator runs `explorer` for `XP1`–`XP4` before the deliveries they block (`XP3` before `SC1`
verifies, `XP1` and `XP4` before `CB1`, `XP2` before `CB3`), and `XP2` again before `CB3` verifies if the
first measurement is a scene-dependent surprise. The owner settles `O1`–`O6`; `O1`
decides whether the change proceeds past `SC1` or waits for the in-flight `gafira-render` branch, and `O6` is
the only one that changes a named library. `specifier` takes `SP1` and `SP2` before `PC1` and `CB5`,
and `SP4`–`SP6` before `CB3` and `CB6` verify.

`SC1` can start without waiting for the owner: it only adds a crate, a row and a workspace entry, and
none of the six questions changes those lines.

## Owner decisions taken (2026-11-05)

The six open questions above are settled. `O4` and `O5` were decided by the coordinator as reversible readings and are recorded with the rest; `O1`, `O2`, `O3` and `O6` are the owner's.

| # | Decision | Reason |
|---|---|---|
| `O1` | `PC1` stays **inside this change**, and is cut once the in-flight `gafira-render` branch of the other worker is merged, rebasing that branch if the timing needs it. Option (a). | A change folder and a verify round for ten lines of source costs more than the wait it saves. |
| `O2` | The adapter's tests may take `gafira-render` as a **dev-dependency**, and **ARQ-06 is amended** to say "normal and build dependencies". Option (c). | The V5 step already implements exactly that reading, so the spec should state what the gate enforces rather than contradict it. |
| `O3` | `CAL-11` closes with **the measured number** and the bench in place; the reference-machine gate arrives with ARQ-40's change. Option (a). | The number is what CAL-11 needs in order to stop being invisible; building the gate first delays the oracle by weeks and adds no product code. |
| `O4` | The CPU backend stays **sequential** for the MVP; the batched frame loop is a `gafira-render` change that goes in the queue after this one. Option (a) until (b). | `CB7` exists only to buy margin, and it changes the determinism argument of `ARQ-111`. Cutting it without a measurement is risk for no gain. |
| `O5` | The provisional readings of `ApplyMask`, `PushLayer` and `ApplyTransition` stay in the adapter, documented; the port is extended by the change that first emits them. Option (a). | A wrong shared type is worse than a documented reading in one adapter. |
| `O6` | **`tiny-skia` 0.12** for coverage only; the pixels stay `gafira-raster`'s. Option (a). | It is what 02 §2.1 names, it is BSD-3-Clause and in `deny.toml`'s allow-list, and an f32-native rasterizer in the export is exactly what ARQ-42's tolerance must not have. |

Consequence for `O2`: `specifier` amends ARQ-06 before `CB3` writes the comparison, so the criterion is not citing a row that contradicts the gate.

## Corrections from the XP round (2026-11-05)

`specs/changes/gafira-render-cpu/spike.md` measured XP1, XP3 and XP4 before `SC1` verified. Four statements above are superseded; where they disagree, the spike wins and this section says what changed.

| # | Superseded | Measured | Consequence |
|---|---|---|---|
| 1 | `D6` justified the tile cull as the difference between CAL-11 and CAL-11 × the tile count, because "the same page-sized path is scanned 40 times". The scan is already the tile's window. | The cull saves ≤ 4 ms on a 200-shape page, nothing on the page background, and only 4.19 → 2.12 ms on a dense one. | The cull stays, for a different reason: it skips operations that miss the tile (−14 % sparse, −49 % dense) and bounds a pathological page. It is not what buys CAL-11's margin. |
| 2 | `XP1` placed the cost risk on `blend_f32`, whose per-pixel `f64` call with a finite check looked like the expensive primitive. `D8` said the margin came from culling, buffer reuse and avoiding copies. | `blend_f32` is **7.1× faster** than `blend_u8` for the same work: 90.7 ms against **640.7 ms** (Normal), and 213.1 ms against the same `blend_u8` figure (Multiply). `blend_u8` pays three `f64` `powf` per pixel. | **The expensive path is the `Rgba8` preview, not the `RgbaF32` export.** CAL-11 is comfortable (≈ 115 ms of its 2 000 ms, 17× margin), so `CB7`'s justification, if it ever has one, is the preview and ARQ-41's 16 ms, not the export. |
| 3 | `D2` listed five transitive crates for `tiny-skia`. | The tree has seven: `cfg-if` and `strict-num` were missing. `bytemuck`, `arrayvec` and `cfg-if` already sit in `Cargo.lock`. | `SC1` adds five new lock entries, not three. Nothing else changes. |
| 4 | `D4`'s one page-sized surface was assumed to cover FD-55's widest case. | The 16 384 px side is 4 GiB in `RgbaF32` — 2.9× the 1.5 GB cap of `D-03` (measured: the 1080p and 4K cases are 33.2 MB and 132.7 MB, ≤ 15.8 % of the cap). | The surface holds 1080p and 4K. FD-55's 16 384 px side becomes a **stated limit** in the crate's documentation, with the tiled or streaming path named as what would lift it. `CB1` stays at 1080p and 4K. |

What this round does **not** settle, and which delivery still carries the risk: `XP2` (preview and export agreeing at 1/255) blocks `CB3` and was not measured here; `XP3`'s byte-identity across targets needs the CI matrix, because linking fails locally on macOS and Windows; and `CB1`'s real pixel cost awaits the backend, measured here only as parts.
