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

- `bc3310e feat(gafira-render): add the content-addressed tile cache` — `ContentTileCache`, its nine behaviour tests and the module export of `lib.rs`.

Gates per commit, each run alone, exit 0: `cargo fmt --check` (0), `cargo clippy --workspace --all-targets --locked -- -D warnings` (0), `cargo test --workspace --locked` (0: 181 `test result: ok` blocks, 2328 tests, none failed; `cargo test -p gafira-render --lib` alone: 38 tests, the 9 new ones among them), `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`), and the CI policy steps — Format, CLI/UI (ARQ-07), `forbid(unsafe_code)`, V5, V12, ARQ-107, V13, V14 and `Testkit ports feature compiles (ADR 0013)` — each with its `run:` extracted literally from `.github/workflows/ci.yml` and run under `bash -euo pipefail`, all 0.

### RED evidence

Every criterion has a test that names the requirement it proves. The first cut of the module answered nothing (`get` gave `None`, `bytes()` gave 0), and the RED of AC10 to AC14 is that cut's failing assertion; AC9 passed against it, so its RED was taken against a cache that stores every tile and ignores the budget, and the second run of each criterion is the hand mutant of the table below on the green code. None of the cuts or mutations is committed.

- **AC9** — `cache::tests::arq_142_a_budget_of_zero_keeps_nothing`. Against the cache that ignores the budget: `a zero budget keeps no entry`, and `bytes()` is not 0.
- **AC10** — `cache::tests::arq_44_a_trim_above_the_current_size_changes_nothing`. Against the first cut: `assertion failed: cache.get(&first).is_some()`; against the mutant that empties the cache on such a trim (H4): `left: Bytes(0)`, `right: Bytes(524288)`.
- **AC11** — `cache::tests::arq_44_a_trim_leaves_the_cache_at_or_below_the_budget`. Against the first cut: `assertion left == right failed: trim to 262144`, `left: Bytes(0)`, `right: Bytes(262144)`; against the mutant that stops the eviction one entry early (H3) the `<=` assertion fails at `trim to 131072`.
- **AC12** — `cache::tests::arq_142_the_least_recently_used_entry_is_discarded`. Against the first cut: `the first entry is read`; against the mutant that evicts the most recently used entry (H1): `the least recently used entry goes`.
- **AC13** — `cache::tests::arq_142_the_key_is_the_content_the_level_the_column_and_the_row`. Against the first cut: `assertion left == right failed: every variant is its own entry`, `left: Bytes(0)`, `right: Bytes(1310720)`; against the mutant that drops the level from the key (H5): `left: Bytes(1048576)`, `right: Bytes(1310720)`.
- **AC14** — a command and a test. The command: the `forbid(unsafe_code)` step and the V5 step pass on the commit. The test: `cache::tests::arq_44_the_cache_travels_through_the_port` puts, reads and trims a `ContentTileCache` behind `&mut dyn TileCache`; against the first cut `left: Bytes(0)`, `right: Bytes(262144)`.
- The byte accounting of XP4 has its own test, `cache::tests::arq_142_the_byte_accounting_matches_the_buffer_size` (RED on the first cut: `left: Bytes(0)`, `right: Bytes(262144)`), and the replacement of a key and the budget a `trim` leaves behind are proved by `cache::tests::arq_44_storing_the_same_key_again_replaces_the_entry` (`left: Bytes(0)`, `right: Bytes(1048576)`) and `cache::tests::arq_44_a_trim_lowers_the_budget_of_the_later_inserts` (`left: Bytes(0)`, `right: Bytes(262144)`).

### Negative controls

The four pieces of critical code of `RE3` (design, "Critical code": the eviction order, the byte accounting, the `trim` boundary and the zero-size budget) carry the six hand mutants of the Slice card. Each one was applied on the green code, the test that names it was run, and the change was reverted. None of them is committed.

| # | Mutation | Test that kills it | Failing assertion |
|---|---|---|---|
| H1 | the eviction takes the most recently used entry (`order.last()`) | `cache::tests::arq_142_the_least_recently_used_entry_is_discarded` | `the least recently used entry goes` — the entry that went was the one just inserted |
| H2 | a budget of 0 keeps the first entry anyway | `cache::tests::arq_142_a_budget_of_zero_keeps_nothing` | `a zero budget keeps no entry` |
| H3 | `trim` stops one entry too early | `cache::tests::arq_44_a_trim_leaves_the_cache_at_or_below_the_budget` | `cache.bytes() <= Bytes(budget)` fails, `trim to 131072` with `bytes()` at 262144 |
| H4 | a `trim` above the current size empties the cache | `cache::tests::arq_44_a_trim_above_the_current_size_changes_nothing` | `left: Bytes(0)`, `right: Bytes(524288)` |
| H5 | the key drops the level, so two LODs collide | `cache::tests::arq_142_the_key_is_the_content_the_level_the_column_and_the_row` | `left: Bytes(1048576)`, `right: Bytes(1310720)` |
| H6 | the accounting counts pixels instead of bytes | `cache::tests::arq_142_the_byte_accounting_matches_the_buffer_size` | `left: Bytes(65536)`, `right: Bytes(262144)` |

### Choices and deviations

- **A read is a use, and `get` takes `&self`**: `ports::TileCache::get` is `&self`, so the least-recent-use order lives behind a `RefCell`, which keeps the `Send` bound of the trait and the `forbid(unsafe_code)` of the crate. The instant of the last use is a `u64` counter the cache owns (ARQ-02: no clock), and the order is a `BTreeSet<(u64, TileKey)>`, never the hash types ARQ-10 bans. Without the refresh in `get`, an entry read on every frame could still be evicted; AC12 and P8 say the one "unused for the longest time" goes.
- **The byte accounting is measured, not computed** (XP4): `ImageBuf::data()` gives the `PixelBuf`, and its length is what the tile costs — 262 144 bytes for a 256×256 `Rgba8` tile and 1 048 576 for an `RgbaF32` one, 4 bytes per `f32`. The cache does not recompute `width × height × channels`: the buffer is the truth the `ImageBuf` constructor already checked (ARQ-117).
- **`trim` stores its budget**: the call sets the limit the later inserts respect, and only the eviction changes the contents. P8 says `trim(budget)` "shrinks to a new budget", and AC10 only fixes that nothing changes when the budget is above the current size; with a smaller budget the new limit has to outlive the call, or the next `put` would grow the cache past what the governor asked for.
- **A second `put` under the same key replaces the entry**: the total subtracts the old buffer before adding the new one, so the cache never counts two tiles that answer the same `get`.
- **The eviction runs on insert as well as on `trim`** (ARQ-142: "al insertar y superar el presupuesto, la caché descarta la entrada menos usada hasta volver al presupuesto"), so `bytes()` comes out of `put` at or below the budget.
- **No bench**: the "Critical code" table marks the benches of `RE3` as not critical, and P20 keeps the pixel-level work out of this cut.
- **The size**: 310 insertions across 2 files (`git diff --shortstat 9c80b39 HEAD` gives `2 files changed, 310 insertions(+)` from the Slice card commit to the feature commit), below the 420 of the design's forecast (590 at most).
