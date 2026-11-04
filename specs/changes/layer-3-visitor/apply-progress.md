## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: the layer 3 visitor contract is the seam the render stream builds on, and the spec had thirteen gaps that `specifier` closed before any code was written
Product IDs: ARQ-13, ARQ-20, ARQ-134, ARQ-135, ARQ-136, ARQ-137 (source: specs/02-arquitectura-software.md §4.1)
Scope: `DC1` of `specs/changes/layer-3/design.md`: the visitor contract over the node kinds that exist, in `crates/gafira-doc/src/`: `NodeVisitor`, `walk`, `WalkOrder`, `PageView` and `MeasureCx`.      Non-goals: the `visit_*` methods of the node kinds that do not exist yet (`visit_frame`, `visit_text`, `visit_image`, `visit_video`, `visit_adjustment`, `visit_sticky`, `visit_table`), the render graph, the extension render handler (that is `RE1`), and any change to `gafira-doc`'s dependencies.
Acceptance:
  AC1 Given a page with a group, a shape and an extension, When `walk` runs, Then the visitor receives exactly one call per node and no node twice [unit]
  AC2 Given a visitor that implements only `visit_node` and none of the `visit_*` methods, When it is compiled and `walk` runs, Then it compiles and receives one call per node [unit]
  AC3 Given the trait `NodeVisitor`, When it is read, Then it declares three `visit_*` methods, one per `NodeKind` variant, and the dispatch over `NodeKind` has no `_` arm [unit]
  AC4 Given a `PageView` built over a page, When the walk runs, Then the page and its arena are borrowed, not cloned or copied, and the arena walked is the same one [unit]
  AC5 Given a page with no roots, When `walk` runs, Then it returns an empty vector [unit]
  AC6 Given the output type of the visitor, When `walk` is called, Then the output is an owned value that cannot borrow from a node that no longer exists [unit]
  AC7 Given a page with nested groups, When `walk` runs with `WalkOrder::PreOrder` and again with `WalkOrder::PostOrder`, Then the parent comes before its children in the first and after them in the second, the siblings are visited in saved order from back to front, and each node is visited once in both [unit]
  AC8 Given a node graph with a cycle, When `walk` runs, Then it terminates and each node is visited once [unit]
  AC9 Given a node whose child does not exist, When `walk` runs, Then it does not panic and the missing child is skipped [unit]
  AC10 Given a `MeasureCx` with a non-finite or non-positive DPI, When it is used, Then the result is the degenerate one that 02 §4.1 states [unit]
  AC11 Given the public API of `gafira-doc`, When it is searched for `RenderGraphBuilder` and for `gafira_render`, Then neither appears [command]
  AC12 Given the crate, When the CI policy steps run, Then `gafira-doc` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the document invariants of ARQ-13 still hold [command]
Plan: the delivery adds one module, `crates/gafira-doc/src/visit.rs`, and the crate's `lib.rs` declares it. The trait, the two walk orders, the borrowed page view and the measurement context are the whole delivery. The critical code is the walk order, the cycle guard, the borrow rules of the page view and the empty-page case (design "Critical code"), so those carry the hand mutants.
Owner decisions touched: none (checked D-01 to D-57: the visitor contract is a domain capability, ARQ-20 keeps `gafira-doc` untouched by the render crate, and the extension seam of ARQ-137 puts the handler outside the domain).

# Apply progress: layer-3-visitor

## DC1 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
