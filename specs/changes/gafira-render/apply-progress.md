## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: the render stream continues with the planner that turns the graph into a tiled plan
Product IDs: ARQ-40, ARQ-41, ARQ-121, CAL-13, CAL-14 (partial) (source: specs/02-arquitectura-software.md §6.1, §6.3, §9.3)
Scope: `RE2` of `specs/changes/layer-3/design.md`: in `gafira-render`, the planner that turns the graph into a plan: the tile grid, the level of detail, the priority order of the operations and the `RenderPlan` that `ports::RenderPlan` describes.      Non-goals: the tile cache (`RE3`), the frame budget and the backend selection (`RE4`), the `PageRenderer` (`RE5`), any change to `gafira-ports`, and the tile grid arithmetic of the raster engine.
Acceptance:
  AC1 Given a page and a viewport, When the planner runs, Then the plan holds the tiles that cover the viewport, in the priority order that 02 §6.3 states [unit]
  AC2 Given a viewport and a zoom, When the level of detail is chosen, Then it is the one that 02 §6.1 fixes for the tile size of 256×256 px and the mipmap chain of ARQ-121 [unit]
  AC3 Given an empty viewport, When the planner runs, Then it holds no tile and does not panic [unit]
  AC4 Given a plan, When its tiles are clipped to the viewport, Then every tile lies inside the viewport bounds [unit]
  AC5 Given the same graph, When the planner runs twice, Then both plans are equal [property]
  AC6 Given a long planning, When it runs, Then the cancel token is checked at least once per tile and the progress fraction is non-decreasing in `[0, 1]` and reaches `1,0` only on success [unit]
  AC7 Given the plan benches of ARQ-40, When they run, Then they measure the plan and not the wall clock, and they use the `Clock` double [command]
  AC8 Given the crate, When the CI policy steps run, Then `gafira-render` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the crate has no `std::time::Instant::now` [command]
Plan: the delivery adds the planner module to `crates/gafira-render/src/` and reuses the graph that `RE1` built; it adds no new dependency and touches no shared file. The testkit doubles of the render stream are already filled by `RE1`, so this delivery does not touch `crates/gafira-testkit/src/ports/**` unless it needs a new double, in which case it adds a file inside the module `SC1` declared. The critical code is the tile grid arithmetic, the LOD choice, the priority order, the clipping bounds and the empty-viewport case (design "Critical code"), so those carry the hand mutants.
Owner decisions touched: none (checked D-01 to D-57: the planner is a domain capability of the render engine and ARQ-40 fixes the size of the plan benches).

# Apply progress: gafira-render

## RE2 (tdd-apply)

Commits:

- `fd1e71a feat(gafira-render): add the planner and the render plan` — the planner (the tile grid, the level of detail, the clipped tiles and the priority order), the `RenderPlan` that `ports::RenderPlan` describes, the `root_ops` addition of `graph.rs` and the plan bench.
- `docs(gafira-render): record the RE2 progress` — these notes.

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (2319 tests, 0 failed), `cargo deny check` (advisories, bans, licenses and sources ok), and the CI policy steps (Format, CLI/UI ARQ-07, forbid(unsafe_code), V5, V12, ARQ-107, V13, V14, and the testkit `ports` step), each by extracting its `run:` from `ci.yml` and running it with `bash -euo pipefail`.

### RED evidence

Every criterion has the test that names it. The RED of each one was taken on the green code by breaking the behaviour that the criterion names and running the test: the assertion that fails is the one below. None of the mutations is committed.

- **AC1** — `planner::tests::arq_41_the_plan_holds_the_tiles_that_cover_the_viewport`. With the last column and the last row of the viewport dropped by the planner, `assert_eq!(plan.tiles().len(), 9)` fails: `left: 4`, `right: 9`.
- **AC2** — `planner::tests::arq_121_the_level_of_detail_is_the_nearest_mip`. With the mip chosen by rounding down instead of by nearest, `assert_eq!(grid.level_for(zoom), want, "zoom 3")` fails: `left: 1`, `right: 2`.
- **AC3** — `planner::tests::arq_121_the_plan_holds_no_tile_for_an_empty_viewport`. With the empty-viewport guard and the empty-region filter removed, `assert!(plan.tiles().is_empty())` fails: `assertion failed: plan.tiles().is_empty()`.
- **AC4** — `planner::tests::arq_121_the_tiles_of_the_plan_lie_inside_the_viewport`. With the clip inclusive on the right edge, `assert!(tile.region.x1 <= viewport.rect.x1)` fails: `assertion failed: tile.region.x1 <= viewport.rect.x1`.
- **AC5** — `planner::tests::arq_140_the_same_graph_gives_the_same_plan`. With the plan reversed on alternate calls, `assert_eq!(tiles_of(&first), tiles_of(&second))` fails: the left plan starts with the `Interactive` tile of `(column 1, row 1)` and ends with `(column 2, row 2, Visible)`; the right one starts with `(column 2, row 2, Visible)` and ends with the `Interactive` tile.
- **AC6** — `planner::tests::arq_139_the_planning_reports_the_progress_and_honours_the_cancel`. With the per-tile `checkpoint` replaced by a bare `progress.report`, so the cancel is not checked per tile, `assert_eq!(result, Ok(Ok(())))` fails: `left: Ok(Err(ReportsAfterCancel { k: 1, extra: 63 }))`, `right: Ok(Ok(()))`.
- **AC7** — a command, not a test: `cargo bench -p gafira-render --bench plan` exits 0 and prints the measure of every zoom of the range of 02 §6.1: `bench_plan_zoom_0.05: level 0, tiles 40, ops 120, at 0ns`, `… zoom 1: level 0, tiles 40, ops 120, at 300ms`, `… zoom 2: level 1, tiles 12, ops 36, at 450ms`, `… zoom 8: level 3, tiles 1, ops 3, at 600ms`, `… zoom 32: level 5, tiles 1, ops 3, at 750ms`, `… zoom 64: level 6, tiles 1, ops 3, at 900ms`. The bench reads no wall clock: the only time source is the injected `Clock` double `SweepClock`, whose `monotonic` starts at zero and advances 150 ms per step.
- **AC8** — a command, not a test: `crates/gafira-render/src/lib.rs:10` is `#![forbid(unsafe_code)]`, the CI step `Domain crates forbid unsafe code` exits 0, the V5 step exits 0 with the row `gafira-render  gafira-ports gafira-doc gafira-geom gafira-color gafira-raster gafira-time gafira-anim`, and `git grep -nE 'std::time::Instant::now' -- crates/gafira-render/src crates/gafira-render/tests crates/gafira-render/benches` finds nothing (the only occurrence in the crate is the ban row of `crates/gafira-render/clippy.toml:25`).

### Negative controls

The critical code of `RE2` (Slice card, Plan: the tile grid arithmetic, the LOD choice, the priority order, the clipping bounds and the empty-viewport case) carries the hand mutants. Each one was applied on the green code, the crate suite was run, and the change was reverted. None of them is committed and none survived, so no behaviour test was added in this round.

| Critical code | Mutation | Test that kills it | Failing assertion |
|---|---|---|---|
| The tile grid (02 §6.1, ARQ-121) | `TILE_SIZE` set to 128 | `planner::tests::arq_121_a_page_of_257_pixels_is_split_in_two_tiles` | `left: (3, 1)`, `right: (2, 1)` (also `arq_121_the_tiles_of_the_plan_lie_inside_the_viewport`, `arq_41_the_plan_holds_the_tiles_that_cover_the_viewport` and `arq_41_the_tasks_of_the_plan_come_in_the_priority_order`) |
| The LOD choice (02 §6.1, ARQ-121) | the mip by rounding down instead of by nearest | `planner::tests::arq_121_the_level_of_detail_is_the_nearest_mip` | `assertion left == right failed: zoom 3`, `left: 1`, `right: 2` |
| The priority order (02 §6.3) | the sort reversed, `Reverse(tile.priority)` first | `planner::tests::arq_41_the_tasks_of_the_plan_come_in_the_priority_order` | `left: Visible`, `right: Interactive` |
| The clipping bounds (AC4) | the clip inclusive on the right edge, `viewport.rect.x1 + 1` | `planner::tests::arq_121_the_tiles_of_the_plan_lie_inside_the_viewport` | `assertion failed: tile.region.x1 <= viewport.rect.x1` (also `arq_41_the_plan_holds_the_tiles_that_cover_the_viewport`: `left: (0, 0, 601, 600)`, `right: (0, 0, 600, 600)`) |
| The empty viewport (AC3) | an empty viewport yields one tile (guard and filter removed) | `planner::tests::arq_121_the_plan_holds_no_tile_for_an_empty_viewport` | `assertion failed: plan.tiles().is_empty()` |

### Choices and deviations

- **The level comes from the zoom, and the planner reads no clock** (02 §6.1, ARQ-02, ARQ-121): `level_for_zoom` is `round(log2(zoom))` clamped to the chain of `⌊log2(max(width, height))⌋ + 1` levels, so a zoom below 100 % stays at the level zero; each level halves a side with the floor, with a minimum of one. The deadline of the frame is `RE4`'s and comes with the caller.
- **The grid covers the page at every level** (ARQ-121): the tile count rounds up, and the last row and column of a level absorb the odd pixel — the last tile of an odd side covers three source pixels instead of two — so no pixel is lost. A page of 257×1 pixels is two tiles.
- **The priority order of the plan** (02 §6.3): the tile that contains the focus is `Interactive` and the rest are `Visible`; the focus is the centre of the viewport when the caller gives none. This cut emits neither `Prefetch` nor `Background`: there is no prefetch ring and no export queue yet. The sort is stable by (priority, row, column), so the roots of a tile keep the paint order of the graph.
- **The tile is clipped to the viewport, the operations are not**: the region is the intersection of the tile region and the viewport, half-open, and an empty intersection is dropped, so an empty viewport gives no tile and no panic (AC3, AC4). The operations of a tile are the ones of the root it draws and they are not clipped: `ports::TileTask` carries the key, the priority and the operations, not the region, so the backend derives the region from the key (`TileGrid::tile_region`) and clips there; the `region` of `PlannedTile` is the clipped rectangle the planner uses and the cache of `RE3` will key on.
- **A tile carries the operations of the root it draws** — deviation, an addition to the public surface of `RE1`: `RenderGraph::root_ops` and its `root_op_ranges` were added to `graph.rs` by this delivery, so the planner pairs a root with its operations without re-walking the graph. `RenderOp` is not `Clone` (the port is closed on purpose), so `copy_op` copies it field by field and a new variant of the port breaks the copy on purpose.
- **The bench is a `[[bench]]` with `harness = false`** — deviation, the shared file this delivery touches: `crates/gafira-render/Cargo.toml` gains `[[bench]] name = "plan", harness = false`, and `benches/plan.rs` has `fn main()` that prints the measure of each zoom. The measure is the shape of the plan (tiles and operations), not a duration: the crate has no clock and the only time source in the bench is the injected `Clock` double (ARQ-02, ARQ-40, AC7).
- **The graph must be built at the level the planner resolves** (02 §6.1: the level is part of the content key of a node): `Planner::plan` documents it. A caller that builds the graph at another level gets tiles whose keys the cache of `RE3` will not match; nothing in this cut enforces it.
- **ARQ-40 and ARQ-41 are proven as a shape, not as a threshold**: the ≤ 2 ms of graph and planning (02 §6.2), the ≤ 16 ms of the UI thread (ARQ-41) and the count of tiles at 3200 % (XP3) are §6.7 measurements that `ARQ-114` keeps out of `cargo test`; the bench of AC7 and the tests above prove the plan is bounded by the tiles of the viewport and carries no wall clock.
- **The progress counts tiles** (ARQ-139): `planned = tiles × roots + 1`, one `checkpoint` per task and a final `1.0`; `checkpoint` checks the token before reporting, so the cancel is checked at least once per tile.
- **The size**: `git diff --shortstat 7791695 fd1e71a` measures 5 files changed, 879 insertions — over the forecast of the design (450, at most 630) and over the ~700 the guide marks as a signal to stop and ask; these notes add their own lines on top. The coordinator decides whether this delivery needs a chained PR.
- **Out of the delivery** (Slice card, non-goals): the tile cache (`RE3`), the frame budget and the backend selection (`RE4`), the `PageRenderer` (`RE5`), any change to `gafira-ports` and the tile grid arithmetic of the raster engine. `CAL-13` and `CAL-14` are partial, as the card says: the plan is served, the cache that feeds it is not.

