## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the CPU backend opens with the surface, the tile loop and the fill path, the pieces every later delivery draws through; O10 carries the rigor of O8 to this change
Product IDs: ARQ-111, ARQ-117, ARQ-119, ARQ-139, ARQ-143, ARQ-44 (partial) (source: specs/02-arquitectura-software.md §4.6, §6.2)
Scope: `CB1` of `specs/changes/gafira-render-cpu/design.md`: in `gafira-render-cpu`, `CpuBackend` with `caps`, `begin_frame`, `execute`, `read_back`, `memory_usage` and `trim`; the page-sized surface in both formats; the tile loop that reads from and writes to the `TileCache`; the cancellation and progress of ARQ-139; the operation cull; the coverage module that turns a kurbo path into a tiny-skia mask; and `FillPath` in `Rgba8` and `RgbaF32`.      Non-goals: layers and the sixteen blend modes (`CB2`), the ARQ-42 comparison (`CB3`), images (`CB4`), masks, effects and transitions (`CB5`), the benches (`CB6`), the parallel bands (`CB7`), and any change to `gafira-ports` or `gafira-render` (`PC1` did those).
Acceptance:
  AC1 Given a plan with a single `FillPath` and a surface, When `execute` runs, Then the tile it is handed carries the coverage of that path, in the format the target asks for, at the region `PC1` put on the task (ARQ-119, ARQ-117) [unit]
  AC2 Given the two formats, When the same fill is drawn to an `Rgba8` target and to an `RgbaF32` target, Then each obeys the premultiplication rule of ARQ-117: 8 bits with `(c × a + 127) / 255` on integer division, `f32` in linear light, and both premultiplied [unit]
  AC3 Given a tile the cache already holds, When the loop reaches it, Then the cached tile is reused and the path is not covered again; given one it does not hold, Then the result is written back at the key of the plan (ARQ-44 partial, 02 §6.1) [unit]
  AC4 Given an operation whose bounds miss the tile, When the loop runs, Then it is skipped and its absence changes no pixel (02 §6.2, the cull as corrected by the XP round) [unit]
  AC5 Given a cancelled token, When the loop reaches the next tile, Then it stops with the cancellation the port fixes and no partial surface is returned (ARQ-139) [unit]
  AC6 Given a long run, When it executes, Then the progress fraction is non-decreasing in `[0, 1]`, follows tiles done over tiles planned, and only reaches `1.0` on a normal end (ARQ-139) [unit]
  AC7 Given the same plan and the same surface, When the backend runs twice, Then the bytes are identical, and they do not change with the number of workers the pool reports (ARQ-111) [property]
  AC8 Given the memory the backend holds, When `memory_usage` is asked and `trim` is called, Then the surface plus the cache answer the size and drop to the budget given, without leaking a buffer (ARQ-44 partial, ARQ-143) [unit]
  AC9 Given the crate, When the CI policy steps run, Then `gafira-render-cpu` still has `#![forbid(unsafe_code)]`, its V5 row passes and the crate reads no clock and opens no file [command]
Plan: the delivery fills `crates/gafira-render-cpu/src/`, created by `SC1`. It implements `ports::RenderBackend` and consumes the place of a tile that `PC1` added to the plan; without it the backend cannot know where a tile goes and the design refused to decode it from the key. Coverage comes from `tiny-skia` behind one private module and the pixels from `gafira-raster`, used and never redefined (ARQ-119). The critical code is the tile loop, the cache key and its write-back, the premultiplication in both formats and the cull (design, "Critical code"), so those carry the hand mutants. The layer stack arrives with `CB2`, so a `PushLayer` in a plan of this delivery is a `blocked`, not a silent pass.
Owner decisions touched: O10 of 2026-11-05 (the rigor of O8 carries over: hand mutants, gates per commit, one V1 round per group, no T4 at close); O6 (tiny-skia for coverage).

# Apply progress: gafira-render-cpu

## CB1 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy -p gafira-render-cpu --all-targets --locked -- -D warnings`, `cargo test -p gafira-render-cpu --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`. The full workspace suite runs once at the delivery HEAD (`AGENTS.md`, ADR 0017).

### RED evidence

### Negative controls

### Choices and deviations
