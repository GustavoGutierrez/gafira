## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V0 | decided-by: owner | reason: the render stream opens with the graph and its content key; the owner waived the verification round and the mutation run (O4, O7 of the layer-3 design), so the delivery closes on the gates
Product IDs: ARQ-110, ARQ-111, ARQ-117, ARQ-138, ARQ-139, ARQ-140, ARQ-141 (source: specs/02-arquitectura-software.md §4.17.7, §6.1, §4.4, §4.13)
Scope: the render stream of `specs/changes/layer-3/design.md`: `RE1` (the `RenderGraph`, the `RenderGraphBuilder`, the content key and the emission of operations), `RE2` (the planner, the tile grid, the level of detail, the priority order and the `RenderPlan`), `RE3` (the content-addressed tile cache, its LRU, its byte budget and its `trim`), `RE4` (the frame budget, the deadline, the backend selection, the degradation to CPU and the progressive refinement) and `RE5` (the `PageRenderer`, the preview and export quality and the export scope), all in `gafira-render`, plus the crate-level contract the stream inherits (purity through ports, cancellation and progress, determinism).      Non-goals: the media crate, any change to `gafira-ports`, any change to `gafira-doc`, the timeline model (`O1`), and the layer-4 backends `gafira-render-cpu` and `gafira-render-gpu` (`O3`).
Deliveries: RE1, RE2, RE3, RE4, RE5, serial; each verified and merged before the next. `RE1` opens the crate and its acceptance criteria are the ones written first in this card; `RE2` to `RE5` add their own criteria in their own section, below.
Acceptance:
  AC1 Given two nodes with the same data, When their content keys are derived, Then the keys are equal [unit]
  AC2 Given a node with a different parameter, a different LOD, a different input key or a different node type, When its content key is derived, Then the key differs [unit]
  AC3 Given the same page, When it is walked in `WalkOrder::PreOrder` and again in `WalkOrder::PostOrder`, Then the content keys are the same [unit]
  AC4 Given the canonical description of §6.1, When the content key is derived, Then it is the BLAKE3 hash of that description truncated to 16 bytes and read as a big-endian `u128`, with the NaN pattern of §6.1 for a non-finite parameter [unit]
  AC5 Given a page with a group, a shape and an extension, When the graph is built, Then it holds the operations of the page and none of the extension [integration]
  AC6 Given the graph built for the preview and the graph built for the export, When both are compared, Then they hold the same operations and only the quality parameters differ [integration]
  AC7 Given a non-finite value in an input, When the graph is built, Then the operation is rejected with the error that §4.17.7 states and the build does not panic [unit]
  AC8 Given the crate, When its sources are read, Then there is no `std::fs`, no `std::net`, no `std::thread::spawn`, no `std::time::Instant::now`, no `std::time::SystemTime::now` and no `std::env`; the time comes from `ports::Clock` and the file system from `ports::FileSystem` [command]
  AC9 Given a long operation, When it runs, Then it checks the cancel token at least once per operation, emits a `Progress` with a non-decreasing fraction in `[0, 1]` that only reaches `1.0` on success, and the fraction follows units done over units planned [unit]
  AC10 Given the same input, When the crate runs the operation twice, Then the bytes are the same, and with a pool of 1, 2 and 4 threads the graph and the keys are identical [property]
  AC11 Given the crate, When the CI policy steps run, Then `gafira-render` still has `#![forbid(unsafe_code)]`, its V5 row still passes, and the API of `gafira-doc` still mentions neither `RenderGraphBuilder` nor `gafira_render` [command]
Plan: the delivery adds the graph module to `crates/gafira-render/src/` and fills the render doubles of the testkit. The testkit is the single writer of `crates/gafira-testkit/src/ports/**` for the render files: this delivery owns `render_backend.rs` and `page_renderer.rs`; the media stream owns `container.rs`, `codec.rs`, `decoder.rs`, `encoder.rs` and `muxer.rs`. `crates/gafira-testkit/src/lib.rs` is not touched, because `SC1` already declared the whole module list. The crate declares `gafira-testkit` with `features = ["ports"]` in its `dev-dependencies`, which adds two lines to `Cargo.lock`; that is why this delivery is serial in the render stream. The critical code is the content-key derivation, the transform composition, the blend and opacity emission and the layer push/pop balance (design "Critical code"), so those carry the hand mutants.
Owner decisions touched: none (checked D-01 to D-57: the graph is a domain capability of the render engine and ARQ-137 keeps the extension handler outside `gafira-doc`).

# Apply progress: gafira-render

## RE1 (tdd-apply)

Commits:

- `b8e9ce1 feat(gafira-testkit): fill the render doubles and test them` — the fake `RenderBackend` and the fake `PageRenderer` of the testkit, and the test of `gafira-render` that drives them.
- `52e196a feat(gafira-render): add the content key of a node` — the canonical description of §6.1 and the `ContentKey` it hashes into.
- `44b3269 feat(gafira-render): add the render graph and its builder` — the `RenderGraph`, the `RenderGraphBuilder` that walks the page with the visitor of `DC1`, the pool seam and the operation emission.

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step), each by extracting its `run:` from `ci.yml`.

### RED evidence

Every criterion has a test that names the requirement it proves. The RED of each one was taken on the green code by breaking the behaviour that the criterion names and running the test: the assertion that fails is the one below. None of the mutations is committed.

- **AC1** — `graph::tests::arq_141_two_nodes_with_the_same_data_give_the_same_key`. With the key mixing the document id, `assert_eq!(first, second)` fails: `left: ContentKey(126317715373268091742341736725497556161)`, `right: ContentKey(126317715373268091742341736725497556162)`.
- **AC2** — `key::tests::arq_141_a_different_input_gives_another_key`. With the level of detail out of the description, `assert_ne!(base, content_key(NodeType::Shape, 1, &[], &params()))` fails: both sides are `ContentKey(252690102959187073671782147330397278576)`.
- **AC3** — `graph::tests::arq_141_the_keys_do_not_depend_on_the_order_of_the_walk`. With the key mixing the order of the walk, `assert_eq!(pre.key_of(NodeId::new(1)), post.key_of(NodeId::new(1)))` fails: `left: Some(ContentKey(111779631591067130758887846790569649577))`, `right: Some(ContentKey(320526234264728048547059940913987045254))`.
- **AC4** — `key::tests::arq_141_the_key_is_the_hash_of_the_description` and `key::tests::arq_141_the_non_finite_pattern_of_the_section_is_used`. With the hash read little-endian, the first fails: `left: ContentKey(95064311730694717303499754240486351256)`, `right: ContentKey(202217487713631911907341341595570570311)`. With a NaN keeping its own bits, the second fails: `left: ContentKey(334589905096010773106109678791768976270)`, `right: ContentKey(43236348239098085800313808250283411287)`.
- **AC5** — `graph::tests::arq_141_the_graph_holds_the_operations_of_the_page_and_none_of_the_extension`. With the extension drawing a shape, `assert_eq!(kinds, vec!["push", "push", "fill", "pop", "pop"])` fails: `left: ["push", "push", "fill", "pop", "push", "fill", "pop", "pop"]`.
- **AC6** — `graph::tests::arq_141_the_preview_and_the_export_hold_the_same_operations`. With the quality changing the operations, `assert_eq!(op_kinds(&preview), op_kinds(&export))` fails: `left: ["push", "push", "fill", "pop", "pop"]`, `right: ["push", "push", "push", "fill", "pop", "pop"]`.
- **AC7** — `graph::tests::arq_110_a_non_finite_number_is_an_error`. With the non-finite check not firing, `assert_eq!(result.err(), Some(GraphError::NonFinite { field: "opacity" }))` fails: `left: None`, `right: Some(NonFinite { field: "opacity" })`.
- **AC8** — a command, not a test: the canonical `clippy.toml` (ADR 0016) gives 0 warnings for the crate, and `git grep -nE 'std::fs|std::net|std::thread::spawn|std::time::(Instant|SystemTime)::now|std::env' -- 'crates/gafira-render/src/*' 'crates/gafira-render/tests/*'` finds nothing.
- **AC9** — `graph::tests::arq_139_the_build_reports_the_progress_and_honours_the_cancel`. With the fraction going back, the contract of the testkit fails: `left: Ok(Err(Decreasing { at: 1 }))`, `right: Ok(Ok(()))`.
- **AC10** — `graph::tests::arq_140_the_graph_does_not_depend_on_the_number_of_threads` and `graph::tests::arq_140_the_same_input_gives_the_same_graph`. With a pool that hands the results back in another order when it has more than one thread, `assert_eq!(keys(1), keys(2))` fails: the same four keys in another order.
- **AC11** — a command, not a test: `crates/gafira-render/src/lib.rs` opens with `#![forbid(unsafe_code)]`, the V5 step passes, and `git grep -nE 'RenderGraphBuilder|gafira_render' -- crates/gafira-doc` finds nothing.

### Negative controls

The four pieces of critical code of `RE1` (design, "Critical code") carry the hand mutants. Each one was applied on the green code, the test that kills it was run, and the change was reverted. None of them is committed.

| Critical code | Mutation | Test that kills it | Failing assertion |
|---|---|---|---|
| The derivation of the content key | the hash is read little-endian | `key::tests::arq_141_the_key_is_the_hash_of_the_description` | `left: ContentKey(95064311730694717303499754240486351256)`, `right: ContentKey(202217487713631911907341341595570570311)` |
| The composition of the transforms | the placement of the group is composed into its children (the bug D-35 forbids) | `graph::tests::arq_141_the_transform_is_the_placement_of_the_node` | `left: [1.0, 0.0, 0.0, 1.0, 105.0, 205.0]`, `right: [1.0, 0.0, 0.0, 1.0, 5.0, 5.0]` |
| The emission of the blend and the opacity | the layer is popped with `Normal` and `1.0` | `graph::tests::arq_141_the_blend_and_the_opacity_are_emitted` | `left: (Normal, 1.0)`, `right: (Multiply, 0.25)` |
| The balance of push/pop of the layers | the layer of the group is not pushed | `graph::tests::arq_141_the_layers_are_balanced` | `a layer is popped before it is pushed` |

### Choices and deviations

- **The transform of a node is its own placement** (D-35, ADR 0009): the placement of every node is in page space, also the one of the children of a group, and the placement of a group does not move its children. The first cut of `RE1` composed the placement of the parent into the child; the spec forbids it, and the hand mutant of the transform composition is exactly that bug. The graph copies the placement, the blend and the opacity of the node (design P6).
- **The extension seam stays open** (Slice card, non-goals): `RE1` does not define the `ExtensionHandler` of ARQ-137. An extension node is reached by the walk and draws nothing: it is not in the graph and it emits no operation. The byte `3` of the node type is reserved by §6.1 for it.
- **A node of the graph is a node of the document**: §6.1 gives the byte of the node type (`1` = `Group`, `2` = `Shape`, `3` = `Extension`), so the graph holds the elements and the stages of §6.1 (`Source → Transform → Effect* → Mask → Blend → Composite`) are the operations the node emits. A shape is a `Source(Vector)` and a group is a `Composite` over its children. `RE1` emits no effect, mask or transition: the document model has none yet.
- **The quality is a parameter of the key**: the preview and the export differ in the quality, and the tile cache of ARQ-142 is addressed by (`ContentKey`, level, column, row) with no quality in it, so the quality has to be in the key for the cache not to serve a preview tile as an export one. The operations are the same in both (AC6) and only the quality parameters differ.
- **The pool comes by parameter** (ARQ-138): `WorkerPool` asks the caller to run one job per index and to hand the results back in the order of the indices; the builder builds the roots of the page with it and takes the result in the order of the roots, so the graph does not depend on the number of threads (ARQ-140). The crate never creates a thread: the pool of the test uses `std::thread::scope`, as `gafira-doc` and `gafira-testkit` already do in their tests.
- **The progress counts the nodes**: the fraction is the nodes that joined the graph over the nodes planned, plus the completion of the build; `checkpoint` checks the cancel token before every report (ARQ-139).
- **ARQ-111 and ARQ-117**: the determinism test proves both rows: ARQ-111 is the common rule and ARQ-140 the one of §4.17.7. ARQ-117 has nothing to check in this cut: the graph holds no pixel buffer — no effect, no mask and no image source reaches it yet — so the first buffer arrives with the planner and the tile cache.
- **`t` is carried, not consumed**: no parameter of the document model depends on time in this cut, so the key of a node is the same at every `t` (§6.1). `GraphParams::t` is the instant the graph is sampled at.
- **The `Cargo.lock` grows by one line**: the dev-dependency on `gafira-testkit` adds one line to the lock, not the two the spike XP9 predicted.
- **The size**: 1571 lines of code (`git diff --shortstat 7e3a23c 44b3269`), over the ~700 the guide marks; these notes add their own lines on top. `git diff --shortstat develop...HEAD` measures the whole branch against `develop`: it also carries the visitor contract of `DC1`, which is not on `develop` yet. The forecast of the design (480, at most 672) did not count the pool seam and its thread test, the cancellation and the progress of ARQ-139, nor the doubles of the testkit and their test. The coordinator decides whether this delivery needs a chained PR.
- **The branch carries the merge of `develop`**: `SC1` landed on `develop` after this branch was cut, so the branch had no `crates/gafira-render` at all. `develop` was merged in before the work; the merge is `7e3a23c` and it brings no change of this delivery.

## RE1 (fix round 1)

Closes C1 and C2 of `verify-report.md` (round 1). Test-only for C2, encoding change for C1. Nothing else in `RE1` was touched: the graph, the pool, the emission, the testkit doubles and the acceptance criteria stay as they were.

Commits:

- `5a4da07 fix(gafira-render): tag the kind of a parameter in the content key` — the value of a `KeyParam` opens with the tag byte of its kind.
- `4bcb404 test(gafira-render): pin the domain byte and the node types of §6.1` — the literals of §6.1 asserted as literals.

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the nine CI policy steps of `.github/workflows/ci.yml` (Format, CLI/UI ARQ-07, forbid(unsafe_code), V5, V12, ARQ-107, V13, V14 and the testkit ports feature of ADR 0013), each by extracting its `run:` block and running it with `bash -euo pipefail`. The workspace suite is green after both.

### C1 — two different parameters gave the same key

- **RED** — `key::tests::arq_141_a_different_parameter_kind_gives_another_key`, written before the fix, on the code of `7530b39`:
  `assertion 'left != right' failed: the parameters 0 and 1 give the same key`, with both sides `ContentKey(83267603764345867193730438760013285531)` — the key verify's probe reported. Parameters 0 and 1 are `Quality(Preview)` and `Geometry(Rect)`; the test derives the six kinds from one-value nodes, so `Fill(None)` and `Blend(Normal)` are covered too.
- **GREEN** — `KeyParam::tag` numbers the kinds `1` Quality, `2` Geometry, `3` Fill, `4` Transform, `5` Opacity, `6` Blend, and `write` leads the value with it. The tag stays inside the length-prefixed block §6.1 describes, so the framing of the description does not change: it is still `u32` length + bytes per parameter.
- **AC1 kept** — the same test ends with `assert_eq!(keys[2], content_key(NodeType::Shape, 0, &[], &[KeyParam::Fill(None)]))`, and `arq_141_the_same_data_gives_the_same_key` still passes unchanged.

### C2 — the literals §6.1 fixes were not anchored

- **RED** — this finding is a hole in the suite, so its RED is the survivor itself: the test `key::tests::arq_141_the_domain_and_the_node_types_are_the_ones_of_the_section` passes on the correct bytes by construction, and it fails the moment the bytes move. Both mutants of the list below (H1 and H2) were applied to gather that evidence, and both now fail the test.
- **GREEN** — the test states `b'R'` for the domain byte and `1`, `2`, `3` for `Group`, `Shape` and `Extension`, both on the constants and on the `[..2]` head of the description of each node type. `arq_141_the_description_holds_the_bytes_of_the_section` no longer compares against `DOMAIN` and `NodeType::Shape as u8` either: it asserts `[b'R', 2, 2]`.

### Hand mutants

Each was applied on the green code, its killing test was run, and the change was reverted. None is committed.

| # | Wrong behaviour | Test that kills it | Failing assertion |
|---|---|---|---|
| H1 | `DOMAIN = b'X'` | `key::tests::arq_141_the_domain_and_the_node_types_are_the_ones_of_the_section` | `left: 88`, `right: 82` |
| H2 | `NodeType::Shape` given the discriminant `5` | `key::tests::arq_141_the_domain_and_the_node_types_are_the_ones_of_the_section` | `left: 5`, `right: 2` |
| H3 | the tag byte removed from `KeyParam::write`, so the four kinds collide at `0x00` again | `key::tests::arq_141_a_different_parameter_kind_gives_another_key` | `assertion 'left != right' failed: the parameters 0 and 1 give the same key`, both sides `ContentKey(83267603764345867193730438760013285531)` |
| H4 | the tag depends on a call counter instead of the parameter, so two identical nodes differ | `key::tests::arq_141_the_same_data_gives_the_same_key` (and the same-kind assertion of `..._a_different_parameter_kind_gives_another_key`) | `left: ContentKey(92328368084581001185630601936379180973)`, `right: ContentKey(291030209336286807384734632626724749923)`; and `left: ContentKey(302593356595757854645899473531573265)`, `right: ContentKey(151018354611883218370633641501097623571)` |

H3 is exactly the defect of C1 and H4 its converse, so the two directions of ARQ-141 are now tied by a test.

### Choices and deviations

- **The numbering of the kinds** — `1` to `6` in the order the enum declares them, `0` left free, mirroring the rule §6.1 fixes for the node types ("cada tipo nuevo toma el siguiente número libre y no reutiliza ninguno"). §6.1 fixes the framing of a parameter (`u32` length + bytes) and not the content of those bytes, so the tag lives inside the value and the description keeps the exact shape the section describes. The numbers are fixed from here on: a kind added later takes the next free number and reuses none, the same way the node types do.
- **The NaN test moved by one byte** — `arq_141_the_non_finite_pattern_of_the_section_is_used` asserted the bytes of `write`, which now start with the tag, so it asserts `&bytes[1..]` against the canonical pattern. The assertion on the pattern is unchanged; only the offset of the payload moved. Its key-level half (`NaN` equals `-NaN`, `+∞` differs from `−∞`) is untouched.
- **One assertion replaced, none deleted** — `arq_141_the_description_holds_the_bytes_of_the_section` compared the head of the description against the crate's own constants and now compares it against `[b'R', 2, 2]`, which is stronger; the rest of that test (the order, the count, the keys, the parameter framing) is unchanged.
- **No other key changes** — the graph of `RE1` builds the parameter slots per node kind, so the tag does not change which slots exist; the keys of the graph change value (the description changed) and every test of the delivery that compares keys compares them with each other, not with a stored literal, so nothing else needed to move.

### The size

`git diff --shortstat 7530b39..4bcb404`: **87 insertions, 8 deletions, 1 file** (`crates/gafira-render/src/key.rs`). Split: `5a4da07` adds 66 and deletes 7, `4bcb404` adds 21 and deletes 1. The crate ends with 20 unit tests (was 18) and the suite of `gafira-render` is green.

## RE2
Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V0 | decided-by: owner | reason: the render stream continues with the planner that turns the graph into a tiled plan; the owner waived the verification round and the mutation run (O4, O7 of the layer-3 design), so the delivery closes on the gates
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

## RE3

Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V0 | decided-by: owner | reason: the render stream closes with the tile cache that the plan writes into; the owner waived the verification round and the mutation run (O4, O7 of the layer-3 design), so the delivery closes on the gates
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
