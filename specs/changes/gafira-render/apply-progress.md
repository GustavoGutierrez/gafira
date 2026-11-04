## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: the render stream opens with the graph and its content key, the piece every later render delivery builds on
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
