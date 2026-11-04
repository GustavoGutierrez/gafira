## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: the render stream opens with the graph and its content key, the piece every later render delivery builds on
Product IDs: ARQ-110, ARQ-111, ARQ-117, ARQ-138, ARQ-139, ARQ-140, ARQ-141 (source: specs/02-arquitectura-software.md §4.17.7, §6.1, §4.4, §4.13)
Scope: `RE1` of `specs/changes/layer-3/design.md`: in `gafira-render`, the `RenderGraph`, the `RenderGraphBuilder`, the derivation of the content key and the emission of the render operations, walking the document with the visitor that `DC1` delivered.      Non-goals: the planner, the tile grid, the LOD and the priority order (`RE2`), the tile cache (`RE3`), the frame budget and the backend selection (`RE4`), the `PageRenderer` (`RE5`), any change to `gafira-doc` or `gafira-ports`, and the extension render handler of ARQ-137, which `RE1` only leaves the seam open for.
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

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
