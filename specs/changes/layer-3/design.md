# Design: layer 3 — `gafira-render` and `gafira-media`

This plan builds the two layer-3 crates of `specs/02-arquitectura-software.md`: `gafira-render` (render graph, content and tile caches, planner with priorities, frame budget, backend selection and degradation) and `gafira-media` (keyframe index, frame cache, proxies, A/V sync, playback clock, video export orchestration). The backends `gafira-render-cpu` and `gafira-render-gpu`, the muxers and the codecs stay out: they are layer 4 (02 §2.1).

The key move is a scaffold delivery, `SC1`, which lands every shared-file change once — the two new crates, their V5 rows, their `clippy.toml` copies, the root `Cargo.toml` entries, the `gafira-ports` trait change and the testkit double stubs. After `SC1`, the render stream (`RE`) and the media stream (`ME`) touch disjoint crates and run in parallel worktrees.

The second key move is a negative one: `gafira-render` and `gafira-media` do **not** depend on each other. They meet only through `gafira-ports` (ADR 0015). 02 §2.2 draws them as siblings and ARQ-01 requires an acyclic graph, so the seam that both directions need — decoded video frames one way, rendered pages the other — becomes port traits instead of crate edges.

Evidence: `AGENTS.md`, `specs/00-decisiones.md` (D-01 to D-57), `specs/01-producto-y-calidad.md` (CAL-11, CAL-13, CAL-14, CAL-16, CAL-17, CAL-27, CAL-35), `specs/02-arquitectura-software.md` (§2.1 rows `gafira-render`/`gafira-media`, §2.2, §2.3, §4.1, §4.4, §4.5, §4.6, §4.8, §4.13, §4.14, §4.17, §5.4, §6.1 to §6.7, §7.1, §9.2, §10, §11.2), `specs/04-funcionalidad-diseno-imagen-exportacion.md` (FD-55 to FD-58), `specs/05-funcionalidad-video-y-distribucion.md` (FV-11, FV-12, FV-22, FV-23, FV-24), ADR 0005, 0007, 0008, 0009, 0010, 0012, 0013, 0014, `.github/workflows/ci.yml` (V5, V12, V13, V14, the `forbid(unsafe_code)` step, the ARQ-107 step), `Cargo.toml` (root), `deny.toml`, `.cargo/mutants.toml`, `crates/gafira-geom/clippy.toml`, and the code of `crates/gafira-ports`, `crates/gafira-doc`, `crates/gafira-layout`, `crates/gafira-raster`, `crates/gafira-audio`, `crates/gafira-anim`, `crates/gafira-segment`, `crates/gafira-time`, `crates/gafira-testkit`.

Precedent followed: `specs/history/2026-11-02-layer-2/design.md` (its delivery table, its "Why each mark holds", its P2 shared-file table and its P5 "types 02 names but no crate owns"), and `specs/history/2026-11-02-layer-2-scaffold/apply-progress.md` (the Slice card format).

## Quick path

1. Run `SC1` alone. It proves the two V5 rows, the `clippy.toml` copies and the new workspace dependencies with one `workflow_dispatch` on its branch, as layer 2 did.
2. Open one worktree per stream: `RE` and `ME`. `DC1` may run beside them; it touches only `crates/gafira-doc/src/`.
3. Run `DC1`, then `RE1` and `ME1` in parallel.
4. Run each stream in order: `RE1` → `RE2` → `RE3` → `RE4` → `RE5`, `ME1` → `ME2` → `ME3` → `ME4` → `ME5`.
5. Run `explorer` for XP1 to XP5 before `RE2` and `ME2` verify, and for XP8 to XP10 before `SC1` verify.
6. Merge each delivery after its verify PASS, one at a time, in dependency order.
7. Record O1 to O3 and their reversibility in "Owner decisions taken (YOLO, 2026-11-03)". The owner resolved all three in YOLO mode on 2026-11-03, on the recommended option of each; no question is left to ask (P1, P20).

## Dependency state

State at `develop` 6b9b225. `gafira-ports` is done and archived (`2026-11-02-gafira-ports`, C14). Layers 0, 1 and 2 are closed and archived. There are no active outcomes in `.odd/state.json` and no other active change folder. Nothing in this cut depends on another change.

Two known pending items from the C14 note are inside this cut: **ARQ-22** ("fallback pendiente en `gafira-render`") and, from layer 2, the `ports` feature of `gafira-testkit` that ADR 0013 describes but that no crate has yet (there is no `[features]` section in `crates/gafira-testkit/Cargo.toml`). `SC1` creates both.

| Delivery | State | Blocked by |
|---|---|---|
| `SC1` | Ready | Nothing |
| `DC1` | Ready | Nothing |
| `RE1` | Blocked | `SC1`, `DC1` |
| `RE2` | Blocked | `RE1` |
| `RE3` | Blocked | `RE1` |
| `RE4` | Blocked | `RE2`, `RE3` |
| `RE5` | Blocked | `RE2`, `RE4` |
| `ME1` | Blocked | `SC1` |
| `ME2` | Blocked | `ME1` |
| `ME3` | Blocked | `ME1`, `ME2` |
| `ME4` | Blocked | `SC1` |
| `ME5` | Blocked | `ME2`, `ME4` |

`RE2` and `RE3` are siblings that can run in parallel inside the render stream: `RE2` owns the planner module and `RE3` owns the cache module, and they meet at `ports::TileCache`, which `SC1` has already fixed. They share `crates/gafira-render/src/lib.rs`, so they are marked `serial` anyway (P2 of layer 2 gave the same rule).

## Scope of the cut

### In

| Area | Requirement IDs | Where |
|---|---|---|
| Render graph: `RenderGraph`, `RenderGraphBuilder`, node kinds, op emission, content keys | 02 §6.1, §4.4, §4.13; ARQ-110, ARQ-111, ARQ-117 | `RE1` |
| Visitor contract over the node kinds that exist | 02 §4.1; ARQ-13, ARQ-20 | `DC1`, `RE1` |
| Planner: viewport clipping, tile grid, LOD, priority order, `RenderPlan` | 02 §6.1, §6.3; ARQ-121, ARQ-41; CAL-13, CAL-14 (partial) | `RE2` |
| Content-addressed tile cache implementing `ports::TileCache` | 02 §6.1; ARQ-10, ARQ-44 | `RE3` |
| Frame budget, deadline, backend selection and CPU degradation, progressive refinement | 02 §6.2, §6.3; ARQ-22, ARQ-41, ARQ-43 | `RE4` |
| `PageRenderer` implementation, preview and export quality on one graph | 02 §4.5, §6.2; ARQ-42; FD-57 (partial) | `RE5` |
| Keyframe index and seek | 02 §6.5; FV-11; CAL-14 (partial) | `ME1` |
| Frame cache: LRU by (asset, frame index, proxy level), budget, prefetch, nearest hit | 02 §6.5; FV-11 (b); ARQ-44; CAL-14 | `ME2` |
| Proxy store: MJPEG intra at 540p/720p in the user cache, byte budget, cancellation | FV-22; 02 §6.5, §7.1; ARQ-43 | `ME3` |
| Playback clock and A/V sync, frame dropping | FV-11; 05 FV-10 (c); 02 §6.5 | `ME4` |
| Video export orchestration: decode → compose → encode → mux | FV-12, FV-24 (partial); ARQ-43; CAL-16, CAL-17 (partial); D-02 | `ME5` |
| New crates, V5 rows, `clippy.toml` copies, workspace dependencies | ARQ-01, ARQ-02, ARQ-03, ARQ-05, ARQ-70, ARQ-71, ARQ-107 | `SC1` |
| `ports::TileCache` gains `bytes()` and `trim()` | ARQ-44 | `SC1` |
| Testkit doubles behind a new `ports` feature | ADR 0013; ARQ-112 (partial) | `SC1`, `RE*`, `ME*` |

### Out, and why

| Out | Reason | Requirement IDs |
|---|---|---|
| `gafira-render-cpu`, `gafira-render-gpu` | Layer 4 adapters (02 §2.1). The CPU backend is the golden oracle and unblocks `bench_export_png_1080` and the ARQ-42 golden, but it is real compositor work and belongs to its own change (O3). | ARQ-42, CAL-11 |
| `gafira-mux`, `gafira-codec-free`, `gafira-codec-os` | Layer 4 adapters (02 §2.1). `gafira-media` drives them through the `Container`, `Codec`, `Decoder`, `Encoder` and `Muxer` ports. | ARQ-74, ARQ-77; D-02 |
| `gafira-engine`: `Session`, `ExportService`, `MediaService`, the persisted export queue, `MemoryGovernor` | Layer 3 but a different concern. 02 §6.4 puts the persisted `ExportRequest` and the job state machine in the engine; FV-23 names `ExportService::{enqueue, cancel, list}`. | ARQ-40, ARQ-43; FV-23 |
| The timeline model: `Sequence`, `Track`, `Clip` | 02 §2.1 gives it to `gafira-doc` (layer 1), and it does not exist yet. `gafira-doc` today has `Document`, `Page` and three node kinds. Building it here would change the persisted schema (D-36 keeps schema 1 open) and is far outside a layer-3 cut. `gafira-media` works per asset and per time range instead (P11). | 02 §3.1, §7.1; D-36 |
| The node kinds `FrameNode`, `TextNode`, `ImageNode`, `VideoNode`, `AdjustmentNode`, `StickyNode`, `TableNode` | Layer 1 work in `gafira-doc`. 02 §4.1 names ten visit methods; `NodeKind` has three. The visitor covers the three that exist (P5). | 02 §4.1 |
| `Project`, the `.gafira` container, `versions/` | `gafira-format` (layer 4). | ARQ-50 to ARQ-59 |
| The UI: `MediaService::{play, pause, seek, scrub}`, the export dialog, the proxy badge | `gafira-ui` (layer 4). | FV-11, FV-12, FV-22 |
| The golden pixel oracle and the pixel-level benches | They need a rasterizing backend. `gafira-render` contributes the plan-level benches only (P20). | ARQ-113; CAL-11 |
| ARQ-22's real-driver case | The `DeviceLost` injection uses a testkit double; the real driver case arrives with `gafira-render-gpu`. | ARQ-22 |
| `gafira-i18n`, `gafira-theme`, `gafira-license`, `gafira-platform` | Layer 4 adapters. `gafira-media` takes the cache root and the RAM budget as parameters instead of reading them. | ARQ-100, ARQ-75 |

### What is reversible

Reversible without touching a public API: the cut boundary itself (adding `gafira-render-cpu` later does not change `gafira-render`), the absence of a compile-time edge between the two crates (a future seam is a new port trait, not a new edge — ADR 0015), and the per-asset shape of `gafira-media` (when `gafira-doc` gains `Sequence`/`Track`/`Clip`, `gafira-media` gains a resolver that maps a timeline onto the per-asset requests it already serves).

**Not cheaply reversible**: the visitor covers only the node kinds that exist. Adding a kind to `NodeKind` is a breaking change for every `NodeVisitor` implementor — `gafira-render` now, and later `gafira-io-svg`, `gafira-io-pdf`, `gafira-edit` and `gafira-engine`. SP1 asks `specifier` for a default method so that the next kind is not breaking; if `specifier` cannot, this is the decision the owner should look at first.

## Delivery table

Forecast is authored lines, with the x1.4 planning figure in brackets. Every delivery stays under ~700 lines at x1.4 and passes `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` on its own.

| Delivery | Scope | Specs covered | Depends on | Mode | Est. lines |
|---|---|---|---|---|---|
| `SC1` | `gafira-render`, `gafira-media` (crates, `clippy.toml` copies), root `Cargo.toml`, `ci.yml` V5 rows, `Cargo.lock`, `gafira-ports` `TileCache`, testkit `ports` feature and double stubs | ARQ-01, ARQ-02, ARQ-03, ARQ-05, ARQ-06, ARQ-44 (partial), ARQ-70, ARQ-71, ARQ-107 | None | serial | 200 (280) |
| `DC1` | `gafira-doc::visit`: `NodeVisitor`, `walk`, `WalkOrder`, `PageView` | 02 §4.1; ARQ-13, ARQ-20 | None | parallel | 430 (600) |
| `RE1` | `gafira-render`: `RenderGraph`, `RenderGraphBuilder`, content keys, op emission | 02 §6.1, §4.4, §4.13; ARQ-110, ARQ-111, ARQ-117 | `SC1`, `DC1` | serial in `RE` | 480 (672) |
| `RE2` | `gafira-render`: planner, tile grid, LOD, priority order, `RenderPlan` | 02 §6.1, §6.3; ARQ-121, ARQ-41; CAL-13, CAL-14 (partial); ARQ-40 (plan benches) | `RE1` | serial in `RE` | 450 (630) |
| `RE3` | `gafira-render`: content-addressed tile cache, LRU, byte budget, `trim` | 02 §6.1; ARQ-10, ARQ-44 | `SC1`, `RE1` | serial in `RE` | 420 (590) |
| `RE4` | `gafira-render`: frame budget, deadline, backend selection, CPU degradation, progressive refinement | 02 §6.2, §6.3; ARQ-22, ARQ-41, ARQ-43 | `RE2`, `RE3` | serial in `RE` | 460 (645) |
| `RE5` | `gafira-render`: `PageRenderer`, preview and export quality, export scope | 02 §4.5, §6.2; ARQ-42; FD-57 (partial) | `RE2`, `RE4` | serial in `RE` | 440 (615) |
| `ME1` | `gafira-media`: keyframe index, seek, forward decode | 02 §6.5; FV-11; CAL-14 (partial) | `SC1` | parallel stream `ME` | 430 (600) |
| `ME2` | `gafira-media`: frame cache, LRU, budget, prefetch policy, nearest hit | 02 §6.5; FV-11 (b); ARQ-44; CAL-14 | `ME1` | serial in `ME` | 470 (660) |
| `ME3` | `gafira-media`: proxy store, byte budget, background priority, cancellation | FV-22; 02 §6.5, §7.1; ARQ-43 | `ME1`, `ME2` | serial in `ME` | 460 (645) |
| `ME4` | `gafira-media`: playback clock, A/V sync, frame dropping | FV-11; 05 FV-10 (c); 02 §6.5 | `SC1` | serial in `ME` | 420 (590) |
| `ME5` | `gafira-media`: video exporter (`ports::Exporter`), decode → compose → encode → mux, progress, cancellation, audio track | FV-12, FV-24 (partial); ARQ-43; CAL-16, CAL-17 (partial); D-02 | `ME2`, `ME4` | serial in `ME` | 500 (700) |

Total: 5 160 authored lines (x1.4: 7 224). The largest delivery is 500 lines (x1.4: 700).

### Why each mark holds

| Mark | Ground |
|---|---|
| `SC1` serial | It edits every shared file: the root `Cargo.toml`, `Cargo.lock`, the V5 table in `ci.yml`, two `clippy.toml` copies, `crates/gafira-ports/src/render.rs`, and the testkit manifest and module list. Layer 2's P2 gave the same rule, and it is the rule that avoided the merge conflicts of that cut. |
| `SC1` first | Every other delivery adds a crate that must exist in the workspace, must appear in the V5 table or the V5 step fails, and must carry a `clippy.toml` copy or the V12 step fails. The V5 step also greps `crates/*/src/lib.rs` for `#![forbid(unsafe_code)]`. |
| `DC1` parallel | It touches only `crates/gafira-doc/src/`. It adds no dependency, so `Cargo.lock` does not change and the merge cannot conflict with `SC1`. `gafira-doc` is a dependency of every crate, but the change is additive: a new module and new public names, no change to an existing signature. |
| `RE` and `ME` parallel streams | ADR 0015: no compile-time edge between the two crates. They share `gafira-ports` only, and neither changes `gafira-ports` after `SC1`. Their testkit work lands in separate files under the modules `SC1` declares. ADR 0007's rule — no edge between two engines — is the layer-2 precedent for running two engine streams beside each other. |
| `RE1` depends on `DC1` | `RE1` implements the `NodeVisitor` trait that `DC1` defines. It cannot compile before it. |
| `RE2` and `RE3` serial in `RE` | They meet at `ports::TileCache` and both edit `crates/gafira-render/src/lib.rs`. They could run beside each other inside the stream, but the module list would be edited twice; the conservative mark is `serial`. |
| `RE3` depends on `RE1` | The cache stores tiles under `ContentKey`, and `RE1` is what computes the key. |
| `RE4` depends on `RE2` and `RE3` | The budget splits the 16 ms across the stages the planner schedules and the cache serves; neither exists before them. |
| `RE5` depends on `RE4` | The export path needs the quality switch and the deadline the budget sets. |
| `ME1` and `ME4` independent of `ME2` | The keyframe index is a scan of the container; the playback clock is a pure time source. They share only the crate, so they are marked `serial` inside `ME` but carry no dependency edge. |
| `ME2` depends on `ME1` | The frame cache stores decoded frames and needs the seek the index provides. |
| `ME3` depends on `ME2` | A proxy is produced by decoding the original and re-encoding it; the frame cache holds the decoded frames. |
| `ME5` depends on `ME2` and `ME4`, not on `RE5` | The exporter composes through `ports::PageRenderer`. `ME5` verifies against the testkit double, so it does not wait for `gafira-render`; the composition root wires the real `PageRenderer` in later. This is what keeps the two streams parallel. |
| `ME5` serial, last in `ME` | It is the largest delivery and touches the exporter, the muxing order and the audio track at once. |

## Shared files

After `SC1` no stream should touch these. If one must, this table applies.

| File | Touched by | Resolution at merge |
|---|---|---|
| `Cargo.toml` (root) | `SC1` | Single writer. `SC1` adds `gafira-render`, `gafira-media` and `blake3` to `[workspace.dependencies]` and nothing else. A later dependency is a stop: mark that delivery serial and merge it alone. |
| `Cargo.lock` | `SC1` | Never hand-merge. Take `develop`'s file, run `cargo check --workspace --all-targets` to regenerate, then the gates with `--locked`. |
| `.github/workflows/ci.yml` | `SC1` | Serial by order. `SC1` adds the two V5 rows and changes nothing else. The ARQ-107 step needs no edit: it reads the crate list from the V5 table. The `Locale` grep of that step applies to the two new crates, so they must not contain the word. |
| `crates/gafira-render/clippy.toml`, `crates/gafira-media/clippy.toml` | `SC1` | New files, byte-identical to `crates/gafira-geom/clippy.toml`. V12 compares them, so a later change to the canonical copy needs every copy in one commit. |
| `crates/gafira-ports/src/render.rs` | `SC1` | Single writer. Two trait methods on `TileCache`. No other file of `gafira-ports` changes in this cut. |
| `crates/gafira-testkit/Cargo.toml` | `SC1` | Single writer. The `ports` feature of ADR 0013, with `gafira-ports` and `gafira-raster` as optional dependencies. |
| `crates/gafira-testkit/src/lib.rs` | `SC1` | Single writer. `SC1` declares every module the streams will fill, so no later delivery edits `lib.rs`. This is the layer-2 P2 trick, applied to the module list. |
| `crates/gafira-testkit/src/ports/**` | `SC1` (stubs), `RE*` and `ME*` (their own files) | Separate files. A stream that needs a new double adds a file inside the module `SC1` declared; if it needs a new module, it asks the coordinator to fold the `lib.rs` edit into the next `SC1`-style serial delivery. |
| `crates/gafira-render/**`, `crates/gafira-media/**` | `RE*`, `ME*` | One crate per stream. A stream never touches the other crate's files. |
| `deny.toml` | `SC1` only if a new licence appears | Single writer. `blake3` is CC0-1.0 OR Apache-2.0 and both are allowed; `cc`, `arrayref`, `arrayvec` and `constant_time_eq` are MIT, Apache-2.0 or Unlicense, and `cc` and `arrayvec` are already in `Cargo.lock`. XP8 confirms with `cargo deny` on the three targets. |
| `.cargo/mutants.toml` | The T4 fix rounds | Keep both entries. Its regexes hold line numbers, so re-check them after a merge that moves that file. T4 runs one crate at a time, which keeps these serial. |
| `specs/README.md` | The coordinator, at archive | One archive at a time. |
| `.odd/state.json` | The coordinator, each branch | Union by `outcome_id`: keep every entry, and for the same id take the one with the newest commit. One outcome per stream. |

Change folders: the render stream gets `specs/changes/gafira-render/` and the media stream `specs/changes/gafira-media/`, each with its `apply-progress.md`, `verify-report.md` and `verification.json`, citing this design. Separate folders avoid conflicts on those fixed file names. `specs/changes/layer-3/` holds this design only.

## Decisions

### P1. The cut is two crates, no adapters, no engine, no timeline

Options: (a) two crates and the twelve deliveries above; (b) pull `gafira-render-cpu` in, so the golden oracle and `bench_export_png_1080` land with layer 3; (c) add the timeline model to `gafira-doc` first and build `gafira-media` on it.

Decision: (a). Reason: `gafira-render-cpu` is a real compositor and a layer-4 adapter with its own licence and platform questions; the CPU backend does not exist, so every pixel-level acceptance in 02 would stay unproven either way. The timeline model is layer 1 and touches the persisted schema while D-36 keeps schema 1 open.

Reversible: yes. Adding `gafira-render-cpu` later does not change `gafira-render`'s API, because the backend is already behind `ports::RenderBackend`.

### P2. `gafira-render` and `gafira-media` do not depend on each other

Options: (a) siblings that meet only through `gafira-ports`; (b) `gafira-media` → `gafira-render`, so the media crate can call the renderer directly; (c) `gafira-render` → `gafira-media`, so the graph can pull decoded frames.

Decision: (a), recorded as ADR 0015. Reason: 02 §2.2 draws them as siblings and ARQ-01 requires an acyclic graph, so (b) and (c) both create an edge that the spec's diagram does not have. The two seams exist and are named in ADR 0015: `ports::PageRenderer` for the renderer, and a frame-source trait in `gafira-ports` for when `VideoNode` lands.

Reversible: yes, and only in one direction — the frame-source seam is a new port trait, never a direct crate edge.

### P3. Layer-3 crates are pure in the ARQ-02 sense

Options: (a) the two crates take the `clippy.toml` copy and are pure: no `std::fs`, no `std::thread::spawn`, no `Instant::now`, no `std::env`; (b) no `clippy.toml` copy for them; (c) relax the canonical copy for layer 3.

Decision: (a), recorded as ADR 0016. Reason: option (b) is impossible — the V5 step requires a `clippy.toml` copy for every crate in its table, and the V12 step compares them. Option (c) forks the lint set and adds CI surface. With (a) the crates read through `ports::FileSystem`, take time from `ports::Clock`, receive the render pool as a parameter and never create a thread. `gafira-media` is pull-based: the caller owns the decode threads (02 §6.3 gives one thread per active stream) and calls into the crate.

Reversible: yes, per crate, but a relaxation would need a new canonical `clippy.toml` and every copy in one commit.

### P4. `gafira-ports` changes in this cut are two trait methods

`TileCache` gains `fn bytes(&self) -> Bytes` and `fn trim(&mut self, budget: Bytes)`. Reason: ARQ-44 makes the caches respect the `MemoryGovernor`, and the governor has to ask a cache how much it holds and tell it to shrink. The trait is in `gafira-ports` and the implementation is in `gafira-render` (§6.1), so the budget call has to travel through the trait. No other port changes: `RenderPlan`, `RenderOp`, `TileCache`, `FrameCx`, `Deadline`, `RenderReport`, `RenderBackend`, `ContentKey`, `TileKey`, `Priority`, `RenderQuality`, `BackendCaps`, `BackendKind`, `TargetDesc`, `MemoryUsage`, `RenderError`, `PageRenderer`, `PageRenderRequest`, `Exporter`, `ExportJob`, `ExportSink`, `Container`, `Demuxer`, `Muxer`, `Codec`, `Decoder`, `Encoder`, `Packet`, `SeekMode`, `FileSystem` and `Clock` all exist today and are enough. `CancelToken` and `Progress` are not ports: they live in `gafira-doc::contract` (ADR 0005) and are enough too.

Verified against the code: `crates/gafira-ports/src/render.rs` (traits `RenderBackend`, `TileCache`; types `RenderPlan`, `TileTask`, `RenderOp`, `ContentKey`, `TileKey`, `Priority`, `RenderQuality`, `Deadline`, `RenderReport`, `RenderError`, `FrameCx`, `FrameId`, `BackendKind`, `BackendCaps`, `TargetDesc`, `MemoryUsage`), `crates/gafira-ports/src/export.rs` (`PageRenderer`, `PageRenderRequest`, `Exporter`, `ExportJob`, `ExportRequest`, `ExportSink`), `crates/gafira-ports/src/media.rs` (`Container`, `Demuxer`, `Muxer`, `Codec`, `Decoder`, `Encoder`, `Packet`, `SeekMode`, `CodecRegistry`) and `crates/gafira-ports/src/system.rs` (`FileSystem`, `Clock`).

### P5. The visitor lives in `gafira-doc::visit` and covers the kinds that exist

`NodeVisitor`, `walk`, `WalkOrder` and `PageView` are named in 02 §4.1 and owned by no crate — the layer-2 P5 case. They live in `gafira-doc` because a visitor over `gafira-doc`'s node types cannot live below it, and because §4.1's promise that a new operation is added "without touching `gafira-doc`" (ARQ-20) needs one dispatch point that every operation shares.

The trait covers `Group`, `Shape` and `Extension`, the three kinds `NodeKind` has today. `Extension` nodes carry an opaque payload and produce no render op in this cut; 02 §4.1's `NodeExtension::render_into(&mut RenderGraphBuilder)` would make `gafira-doc` depend on `gafira-render`, which ARQ-01 forbids, so SP4 asks `specifier` where that seam goes. `gafira-render` defines an `ExtensionHandler` trait of its own (ADR 0008 rule 2: an engine owns its inputs) and the composition root binds it.

### P6. `PageView` is a borrowed view, not a snapshot

`PageView` borrows a `Page` and its arena; `walk` takes `&PageView`. Reason: 02 §4.1 writes `walk(doc: &PageView, ...)`, and a copy would cost a page clone on every frame, against the ≤ 2 ms of 02 §6.2. The consequence is that a visitor cannot hold a node reference past the walk; `RE1` therefore copies the values it needs (placement, blend, opacity) into the graph.

### P7. The content key is BLAKE3 truncated to 128 bits

`ContentKey(pub u128)` already exists in `gafira-ports`. The hash covers the node type, the parameters sampled at `t`, the input keys and the LOD, as 02 §6.1 says. The function is BLAKE3 truncated to 16 bytes, the same function 02 §7.1 already fixes for `assets/<blake3-hex>`. Reason: one hash function in the project; a second one would let an asset hash and a tile hash disagree about what "the same content" means. SP6 asks `specifier` for the exact canonical byte description, which this design does not fix.

The feature set of `blake3` is left open for XP8: the default features assemble the SIMD kernels with `cc` on x86_64, and the `pure` feature drops that at a speed cost that XP2 has to weigh against the 2 ms of 02 §6.2. The crate is a pure-Rust dependency either way, so `forbid(unsafe_code)` in this crate is not affected.

### P8. The tile cache is an LRU over `TileKey` with a byte budget

`ContentTileCache` implements `ports::TileCache`. `get` and `put` are O(1) amortised; `put` evicts from the least recently used end until the total is at or below the budget. `bytes()` reports the current total; `trim(budget)` shrinks to a new budget and reports how much it released. Eviction order and accounting are `BTreeMap`/`BTreeSet` based, never `HashMap`, because the canonical `clippy.toml` bans the hash types and ARQ-10 wants a deterministic iteration order.

### P9. The frame budget is a fixed split and the deadline comes from the injected clock

02 §6.2 fixes 16 ms per UI frame with the split "orientativo": ≤ 4 ms UI, ≤ 2 ms graph and planning, ≤ 8 ms visible render, 2 ms of margin. `gafira-render` owns the graph-and-planning share and the visible-render share. `Deadline::At` is an absolute monotonic instant, so the budget reads `ports::Clock::monotonic()` and computes it; no crate calls `Instant::now` (P3). The margin is not spent: when the deadline passes, `execute` returns a `RenderReport` with `left > 0` and the caller presents the best mip available and refines on the next frame (02 §6.2, progressive refinement).

### P10. The backend selector degrades to CPU and keeps the session editable

`BackendSelector` picks a backend from the `BackendCaps` it is given and never creates one. On `RenderError::DeviceLost` it switches to the CPU backend for the rest of the session and reports the switch, so ARQ-22's "the GPU degrades to CPU without losing the document" holds. The acceptance test injects a `DeviceLost` in the middle of a frame through the testkit double; the real driver case arrives with `gafira-render-gpu`.

### P11. `gafira-media` works per asset and per time range

The timeline model does not exist (P1). `gafira-media` therefore exposes operations over a single asset and a time range: the keyframe index of an asset, the frame of an asset at a source time, the proxy of an asset, the clock of a range, and the export of a range. Which clip covers which time is the caller's decision — the engine's, once the timeline model lands. `gafira-media` gains a resolver then, without changing its own API.

### P12. The keyframe index is built by scanning packets

No new port method. `ports::media::Demuxer::read_packet` yields `Packet { keyframe: bool, pts, dts }`, so the index is a scan that keeps the timestamps of the key packets, and `Demuxer::seek(t, SeekMode::Keyframe)` lands on the previous keyframe for the decode-forward path of 02 §6.5. `SeekMode::Exact` is the same seek plus a forward decode. The scan happens in the background at import, ≤ 1 s per minute of 1080p material (02 §6.5, FIJADA).

### P13. The frame cache is keyed by (asset, frame index, proxy level)

02 §6.5 gives the key; SP7 asks `specifier` whether the "asset" is the `ContentHash` or the `ProviderAssetId`, and what the proxy levels are. The budget is a parameter, injected by the caller, because reading the RAM size is `gafira-platform`'s job. The cache holds the nearest hit to a requested time so that scrubbing can show a frame while the exact one decodes (FV-11).

### P14. The proxy store writes through the `FileSystem` port

Proxies live in the user cache, outside the `.gafira` (02 §7.1, FV-22). The root path is a parameter; the crate does not read it from the environment and does not touch `std::fs` (P3). The store enforces a byte budget (10 GB by default, PROPUESTA in FV-22) and evicts the least recently used proxies. Generation runs at `Priority::Background` with a `CancelToken` and a `Progress`. The export path never reads a proxy (FV-22 (b), V-3).

### P15. The audio clock is the master and the video drops frames

02 §6.5 and FV-11: the master clock is the audio one; the video drops frames when it falls behind and never blocks the audio. `PlaybackClock` is driven by `ports::Clock`; the drift tolerance is the one FV-12 (b) gives, under 40 ms. The drop decision and the drift computation are the critical code of `ME4`.

### P16. The video exporter is a `ports::Exporter` and takes the renderer through the job

FV-12 names `ExportJob` and `Exporter` in `gafira-media`. `ports::Exporter::export` receives an `ExportJob` that borrows the document and the ports, so the exporter reaches the renderer, the codecs and the muxer through the job's ports and not through a crate edge. The pipeline is decode → compose → encode → mux, with the audio track mixed by `gafira-audio` (layer 2) before it is encoded. The composition and the encoding run in different threads per 02 §6.7; the crate drives the order and the cancellation, and the threads belong to the caller (P3).

### P17. The testkit doubles live behind a new `ports` feature

ADR 0013 describes a `ports` feature of `gafira-testkit`, off by default, holding the port doubles. No crate has it yet. `SC1` creates it with the module list fixed in advance: a fake `RenderBackend`, a fake `Codec`, `Decoder`, `Encoder`, `Container` and `Muxer`, a fake `FileSystem`, a fake `Clock` and a fake `PageRenderer`. `gafira-raster` keeps its rule: it does not activate the feature and does not depend on `gafira-ports` in any section of its manifest.

### P18. The ARQ-107 step already covers the two new crates

The step reads its crate list from the V5 table, so adding the two rows extends it with no edit. Its `Locale` grep applies to `gafira-render` and `gafira-media`: neither may contain the word "Locale". Its dependency check applies too: neither may pull `gafira-i18n` or an ICU crate.

### P19. `gafira-render` does not depend on `gafira-layout` or `gafira-text` in this cut

The graph reads the placement from `ElementBase.placement`, which is in page space (ADR 0009), and layout results reach the document through a `Command` (ARQ-30) before anything renders. Text shaping needs a `TextNode`, which does not exist. Both edges can be added later without changing the public API. Reason: two fewer crates in the V5 row and two fewer merge edges.

### P20. Only the benches that do not need a rasterizing backend are in this cut

`gafira-render` contributes the plan-level benches (`bench_plan_*`, the invalidation bench behind `bench_recolor`). The pixel benches (`bench_export_png_1080`, `bench_shadow_1080`) and the ARQ-42 golden need `gafira-render-cpu`. `gafira-media` contributes the index and cache benches. Reason: a bench that measures a fake backend measures the fake.

## Critical code

Hand mutants apply to all of these. T4 at close covers every crate in the table.

| Delivery | Critical | Not critical |
|---|---|---|
| `SC1` | None; configuration only, checked by V5, V12, the ARQ-107 step and `cargo deny` | All |
| `DC1` | The walk order, the cycle guard, the `PageView` borrow rules, the empty-page case | The trait's doc text |
| `RE1` | The content-key derivation, the transform composition, the blend and opacity emission, the layer push/pop balance | Benches |
| `RE2` | The tile grid arithmetic, the LOD choice, the priority order, the clipping bounds, the empty-viewport case | Benches |
| `RE3` | The eviction order, the byte accounting, the `trim` boundary, the zero-size budget | Benches |
| `RE4` | The budget split, the deadline computation, the degradation path, the `left > 0` report | Benches |
| `RE5` | The quality switch, the export-scope mapping, the single-page and range cases | Benches |
| `ME1` | The keyframe lookup boundaries, the seek direction, the empty index, the time before the first keyframe | Benches |
| `ME2` | The cache key, the eviction order, the nearest-frame search, the budget boundary | Benches |
| `ME3` | The proxy naming, the byte budget, the cancellation checkpoints, the "export never uses a proxy" rule | Benches |
| `ME4` | The clock monotonicity, the drop decision, the drift computation, the zero-length range | Benches |
| `ME5` | The frame loop, the A/V alignment, the cancellation checkpoints, the mux ordering, the frame count | Benches |

T4 at close: `gafira-render`, `gafira-media`, `gafira-doc`, `gafira-ports`, `gafira-testkit`.

## Open questions (owner)

The owner resolved O1 to O3 in YOLO mode on 2026-11-03, on the recommended option of each. The table below is kept as the record of what was considered and what was recommended; "Owner decisions taken (YOLO, 2026-11-03)" carries the answers.

| # | Question | Options and tradeoffs | Recommendation |
|---|---|---|---|
| O1 | The timeline model (`Sequence`, `Track`, `Clip`) does not exist, and 02 §2.1 gives it to `gafira-doc` (layer 1). Without it, `gafira-media` can only deliver the machinery: the export pipeline runs over a time range, not over a sequence. Is a follow-up `gafira-doc` change for the timeline approved, and does it come before or after layer 3? | (a) Layer 3 delivers the machinery now; the timeline lands in a later `gafira-doc` change and `gafira-media` gains a resolver. The cut stays bounded and the persisted schema is untouched. (b) Pull the timeline into this change: layer 3 grows a layer-1 crate, the schema changes while D-36 keeps schema 1 open, and the estimate grows well past the table above. (c) Stop layer 3 until the timeline exists: the two layer-3 crates would wait for a layer-1 crate, and the layer order in 02 §2.1 would be inverted. | (a). The owner confirms whether the timeline change is approved and where it sits in the queue. |
| O2 | 02 §6.5 fixes the frame cache budget at "1 GB or 15 % of RAM, the lesser", and D-03 fixes "≤ 1,5 GB with 20 pages" and "≤ 300 MB at rest". On the reference machine (8 GB) the frame cache is 1 GB, which leaves 0,5 GB for everything else under the 1,5 GB cap. Is the frame cache inside the 1,5 GB or on top of it? | (a) Inside: the governor trims the frame cache first to fit the 1,5 GB. One number to respect, and the idle budget is untouched. (b) On top: the 1,5 GB covers the document only and the cache is extra, which is easier to reason about but lets the process pass the number D-03 fixed. (c) Lower the frame cache default: contradicts the FIJADA figure in 02 §6.5 without a new owner decision. | (a). It changes `gafira-media`'s default budget and the governor's trim order, so the owner confirms it. |
| O3 | `gafira-render-cpu` is the golden oracle (02 §4.5, ARQ-113) and unblocks `bench_export_png_1080` (CAL-11) and the ARQ-42 preview/export golden. It is layer 4 and out of this cut. Should it be planned as its own change right after layer 3, or pulled into this one? | (a) Its own change right after layer 3: layer 3 stays at the size above, and the oracle lands when the plan and the graph are stable. (b) Pulled in: the pixel-level acceptances land with layer 3, but the delivery grows by roughly a compositor and its own licence and platform questions, and the two streams stop being parallel. | (a), planned as the next change. The owner confirms the priority. |

## Owner decisions taken (YOLO, 2026-11-03, second session)

The owner asked for the change to be finished faster and named what to skip. Two decisions were taken on 2026-11-03, after `SC1` and `DC1` had already been verified at depth V2 and merged.

### O4. The T4 mutation run at close is skipped — `mutation: off`

The five crates the change touches (`gafira-render`, `gafira-media`, `gafira-doc`, `gafira-ports`, `gafira-testkit`) do not get the whole-crate `cargo mutants` run that `mutation: archive` would require at close.

Reason: the owner decides that the code is already validated by the per-delivery gates. Every delivery runs `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check` and the nine CI policy steps **on each of its commits**, plus the hand mutants of the design's critical code, which the implementer applies and kills before committing.

Cost of the decision: the mutation report is the one piece of evidence that says no wrong behaviour survives the whole test suite of a crate. It is not replaced. What replaces it is the V1 verification of every delivery (gates at HEAD plus a criterion map), the hand mutants on the critical code of each delivery, and the remote CI on the three product targets for every branch.

Reversible: yes. The T4 run is a command over the same tree; a later change can run it on the two new crates without touching the code.

### O5. The remaining deliveries are verified at depth V1

From `ME2` and `RE2` onwards, the verification depth is **V1**: the gates at HEAD plus a map of every acceptance criterion to the test or command that proves it. The independent probes, the independent hand mutants and the scratch-export runs of V2 are not run for those deliveries.

Reason: the owner asks for speed and states that the created code is already validated. V1 keeps the gates and the criterion map, which is what says whether a criterion has a proof; what it drops is the second opinion that V2 adds on top.

What this means per delivery: the `tdd-apply` still applies the hand mutants of the design's critical code and kills each one before committing, and still runs the full gate suite on every commit. The `verify` still checks every acceptance criterion against a test or command, still checks the integrity of the Slice card and the notes, and still runs the gates. The rounds that already ran at V2 (`SC1` rounds 1 and 2, `DC1` rounds 1 and 2, `ME1` round 1 and `RE1` round 1) keep their depth and their reports.

Cost of the decision: a defect that only an independent probe would find, as the one the V2 round of `ME1` found in the keyframe seek, may reach `develop`. The gates and the hand mutants do not cover that class of defect.

Reversible: yes. Any delivery can be re-verified at V2 later, and the V2 report of `RE1` is the reference for what that costs.

## Owner decisions taken (YOLO, 2026-11-03)

The owner worked in YOLO mode and was not available for questions, so O1 to O3 were resolved on 2026-11-03 on the recommended option of each. The table of options above stays as the record of what was weighed. Each decision below stands on the recommendation it adopted, and each names what the owner can revert.

### O1. The timeline model stays out of this cut — option (a)

Layer 3 delivers the machinery now. The timeline model (`Sequence`, `Track`, `Clip`) arrives later, in its own `gafira-doc` change at layer 1, and `gafira-media` gains a resolver then. The timeline change is not opened now; it is noted as next in the queue, after layer 3.

Reason: the cut stays bounded to the two layer-3 crates and the persisted schema stays untouched while D-36 keeps schema 1 open.

Reversible: yes, and cheap. The timeline change is a separate change in a separate crate, and `gafira-media`'s public API does not have to change when it lands (P11). What is not reversible here is the content of that later change: the schema it writes, the node kinds it adds and the order it comes after layer 3. Those are the owner's to fix in the timeline change itself.

### O2. The frame cache lives inside the 1,5 GB of D-03 — option (a)

The frame cache counts against the "≤ 1,5 GB with 20 pages" figure of D-03, and the `MemoryGovernor` trims the frame cache first to fit. The default of 02 §6.5 is not lowered: it stays FIJADA at "1 GB or 15 % of RAM, the lesser". D-03 is not contradicted; this is the reading that makes both hold at once.

Reason: one number to respect, and the idle budget of D-03 is untouched.

Reversible: yes. The trim order is a governor decision, so a later change can move the frame cache outside the 1,5 GB without touching `gafira-media`'s cache API. What is not reversible is the arithmetic itself: while the frame cache holds 1 GB and the cap is 1,5 GB, the document has 0,5 GB to work in, and the idle figure of 300 MB means the cache must be released when it is not playing. Changing either number means changing a FIJADA figure of 02 §6.5 or of D-03, which is the owner's call.

### O3. `gafira-render-cpu` is planned as its own change right after layer 3 — option (a)

The CPU backend stays out of this cut and is planned as a separate change immediately after layer 3.

Reason: layer 3 stays at the size of the delivery table, the golden oracle and the pixel-level acceptances land when the plan and the graph are stable, and the two streams keep running in parallel.

Reversible: yes. The backend sits behind `ports::RenderBackend` (P4), so adding `gafira-render-cpu` later does not change `gafira-render`'s API. What is not reversible is the queue order: the oracle, `bench_export_png_1080` (CAL-11) and the ARQ-42 golden stay unproven until that change lands, and the owner may reorder that queue.

## Needs explorer

Provisional claims. Nothing in the table is built on until it is measured.

| # | Claim | Check | Blocks |
|---|---|---|---|
| XP1 | The 02 §6.2 split (≤ 2 ms for graph and planning, ≤ 8 ms for the visible render, 2 ms of margin) holds for the graph this cut produces. Unmeasured: the graph will grow with the node kinds that do not exist yet, so the number may be far off for real MVP documents. | A prototype that builds and plans a 100-layer page and a 20-page document, on the D-03 reference profile, reporting the two stages separately. | `RE2`, `RE4` |
| XP2 | The content key can be computed for every tile of a 100-layer page inside the 2 ms of XP1. Unmeasured: BLAKE3 over a canonical description of every node, every frame. | The same prototype, with the key derivation timed apart from the plan. | `RE1`, `RE2` |
| XP3 | The 256×256 tile grid and the mip levels keep the visible tile count inside the budget for a 3200 % zoom. Unmeasured: ARQ-121 fixes the tile size but not the count at the extremes of the zoom range. | A count over the zoom range 5 % to 3200 % on a 1080p page, with and without the LOD rule of 02 §6.1. | `RE2` |
| XP4 | The tile cache byte accounting matches reality. A 256×256 tile is 256 KiB in `Rgba8` and 1 MiB in `RgbaF32`, and a cached tile may hold mips too. | Measure the real size of a populated cache against `bytes()` on both formats, and the cost of `trim` on a full cache. | `RE3` |
| XP5 | The prefetch policy of 02 §6.5 ("in the direction of the playhead") keeps the frame cache hit rate high enough for `bench_scrub_1080` (≥ 24 updates/s, cache search under 50 ms). Unmeasured: the policy is a heuristic and the hit rate depends on the material. | A simulation over a real 1080p clip with the cache in `ME2`, reporting the hit rate and the nearest-frame distance. | `ME2` |
| XP6 | The keyframe scan of 02 §6.5 (≤ 1 s per minute of 1080p material) holds with the ports' `Demuxer`, whose `read_packet` allocates a `Vec<u8>` per packet. | A timing of a full packet scan of a 1080p file, with the allocation counted. | `ME1` |
| XP7 | `gafira-media` being pull-based and single-threaded (P3) is enough for the eight decode streams of 02 §6.3. The thread count belongs to the caller, but the policy of which stream to decode next lives in this crate. | A prototype that drives the frame cache from eight concurrent decoders and reports the contention on the cache. | `ME2`, `ME5` |
| XP8 | The two new crates pass `cargo deny` on the three targets and compile on all three, with `blake3` as a new direct dependency and `kurbo` as another. The feature set of `blake3` is open: the default assembles the SIMD kernels with `cc` on x86_64, and `pure` drops that at a speed cost that XP2 has to weigh. | `cargo deny check` on the three targets, a `workflow_dispatch` run of the CI workflow on the `SC1` branch as layer 2 did in XP2, and a build of `gafira-render` with and without the `pure` feature. | `SC1` |
| XP9 | Enabling the `ports` feature of `gafira-testkit` in the dev-dependencies of `gafira-render` and `gafira-media` does not make the mutants of those crates recompile `gafira-ports` on every mutant. Layer 2 already saw load timeouts with `--jobs 8` (its P3 item 1). | A `cargo mutants` run on `gafira-render` that measures the load time of a sample of mutants. | `SC1`, the T4 of both crates |
| XP10 | The proxy store reaches "≥ 1× real time at 1080p on mid-range hardware" (FV-22, PROPUESTA). It depends on the MJPEG codec, which is layer 4 and out of this cut, so the number cannot be measured here. | Measure it with the codec-free crate in the layer-4 change. | Nothing in this cut; it is recorded so the number is not claimed |
| XP11 | `gafira-render` can build a `RenderPlan` for a page with an `Extension` node without touching `gafira-doc`, as ARQ-20 requires. | A test that registers an `ExtensionHandler` in `gafira-render` and checks that a document with an extension node renders without a change in `gafira-doc`. | `RE1` |

## Needs specifier

02 does not say enough for some of this cut. Each item is a request, not a rewrite.

| # | ID | Request | Blocks |
|---|---|---|---|
| SP1 | 02 §4.1 | State which node kinds the `NodeVisitor` covers in the first cut, and whether adding a kind to `NodeKind` is allowed to break the trait's implementors. A default method per kind would make it not breaking; the trait as written in 02 §4.1 has none. | `DC1`, and every later kind |
| SP2 | 02 §4.1 | Define `PageView`: its fields, its lifetime, and whether it owns the page or borrows it. | `DC1` |
| SP3 | 02 §4.1 | Define `MeasureCx` and `WalkOrder`. `MeasureCx` appears in `NodeBehavior::local_bounds` and must not depend on `gafira-layout`, which would be a cycle. | `DC1` |
| SP4 | 02 §4.1 | `NodeExtension::render_into(&mut RenderGraphBuilder)` makes `gafira-doc` depend on `gafira-render`, which ARQ-01 forbids. Say where that seam lives: a trait in `gafira-ports`, a trait owned by `gafira-render`, or the engine. | `RE1` |
| SP5 | 02 §4.17 | Add a §4.17 subsection for `gafira-render` and one for `gafira-media`, with the "first cut / outside the cut" rows the six layer-2 engines have. §4.17 stops at `gafira-segment`. | Nothing; it records this design |
| SP6 | 02 §6.1 | The exact fields of the content-key hash and the truncation. §6.1 says only "hash(type, parameters sampled at t, input keys, LOD)". | `RE1` |
| SP7 | 02 §6.5 | The frame cache key in full: whether the "asset" is the `ContentHash` or the `ProviderAssetId`, and what the "proxy level" values are. | `ME2` |
| SP8 | 02 §6.2 | Fix the per-stage numbers of the frame budget and the rule for the margin. The split is marked "orientativo" while the total is FIJADA. | `RE4` |
| SP9 | 02 §6.5, FV-11 | The frame-drop policy and the A/V tolerance of the playback clock. FV-12 (b) gives under 40 ms for the exported file; the preview tolerance is not stated. | `ME4` |
| SP10 | 02 §6.4 | `ParamValues` has no serialized form, and §6.4 says it arrives with the export queue. Confirm that `gafira-media`'s exporter reads its options from `ParamValues` in memory and that persistence arrives with the engine's queue, so this cut does not have to serialize them. | `ME5` |
| SP11 | ARQ-22 | The point at which a `DeviceLost` is injected in the acceptance test, so the fake backend can fail in the middle of a frame rather than before it. | `RE4` |
| SP12 | ARQ-44 | The name and unit of the budget call on a cache: `bytes()` and `trim(budget)` in bytes, added to `ports::TileCache`. | `SC1`, `RE3` |
| SP13 | 02 §6.5 | The keyframe index: is it kept in memory only, or written to the user cache next to the proxies? 02 §7.1 says proxies and caches are not stored in the `.gafira`, but not whether the index is persisted. | `ME1` |

## Next

The coordinator runs `explorer` for XP8 and XP9 before `SC1` verifies, and for XP1 to XP5 before `RE2` and `ME2` verify. `specifier` takes SP1 to SP4 before `DC1` starts, and SP6 to SP13 before the delivery each one names. The owner resolved O1 to O3 in YOLO mode on 2026-11-03, on the recommended option of each, and they are recorded in "Owner decisions taken (YOLO, 2026-11-03)". No owner decision is outstanding, so `SC1` can start without waiting for the owner.

`SC1` is the first delivery and the first `tdd-apply`. Then `DC1`, then the two streams `RE` and `ME` in parallel worktrees.

The two new ADRs, `docs/adr/0015-layer-3-sibling-seams.md` and `docs/adr/0016-layer-3-purity-through-ports.md`, stay `proposed` until the owner confirms them.
