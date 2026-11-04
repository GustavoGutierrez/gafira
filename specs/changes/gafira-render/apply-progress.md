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

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations

## RE3

Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the render stream closes with the tile cache that the plan writes into; the mutation run and the deep verify were waived by the owner (O4, O5 of the layer-3 design)
Product IDs: ARQ-10, ARQ-44, ARQ-142 (source: specs/02-arquitectura-software.md §6.1, §4.6)
Scope: `RE3` of `specs/changes/layer-3/design.md`: in `gafira-render`, the content-addressed tile cache: the LRU, the byte budget, the `trim` call and the eviction order, addressed by `ports::TileCache`.      Non-goals: the planner (`RE2`), the frame budget (`RE4`), the `PageRenderer` (`RE5`), and any change to `gafira-ports` (the trait already carries `bytes()` and `trim(budget)` from `SC1`).
Acceptance:
  AC9 Given a budget of 0, When a tile is inserted, Then no entry is kept and `bytes()` gives 0 [unit]
  AC10 Given a `trim` with a budget larger than the current size, When it runs, Then nothing changes and it does not fail [unit]
  AC11 Given a `trim`, When it runs, Then `bytes()` is at most the budget given [unit]
  AC12 Given two entries of the same size, When the cache overflows, Then the one unused for the longest time is discarded [unit]
  AC13 Given the key of §4.6, When it is built, Then it is the content key, the level, the column and the row [unit]
  AC14 Given the crate, When the CI policy steps run, Then `gafira-render` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the cache travels through `ports::TileCache` [command]
Plan: the delivery adds the tile cache module to `crates/gafira-render/src/`, on top of the graph of `RE1` and the plan of `RE2`. It implements the trait that `SC1` left in `ports::TileCache`; the trait itself is not touched. The critical code is the eviction order, the byte accounting, the `trim` boundary and the zero-size budget (design "Critical code"), so those carry the hand mutants.
Owner decisions touched: none (checked D-01 to D-57: the tile cache is a domain capability and ARQ-44 keeps it inside the memory governor).

## RE3 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
