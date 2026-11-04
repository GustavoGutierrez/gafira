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

Commits: `df8ab59` the visitor contract over the node kinds (`crates/gafira-doc/src/visit.rs`, its `lib.rs` line and `crates/gafira-doc/tests/visit_contract.rs`).

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step), each by extracting its `run:` from `ci.yml`. The gates were run on the delivered tree after the hand mutants were reverted; the mutants themselves were run against the test suite alone and were never committed.

### RED evidence

The RED state was the module with its whole public API and the behaviour missing: the walk reduced to its first root, the dispatch with a catch-all arm and the measurement without the DPI gate. Every test below failed with an assertion or a panic, never with a compile error.

| AC | RED (test and failing assertion) | GREEN |
|---|---|---|
| AC1 | `visit::tests::arq_134_every_node_is_visited_once`: `assertion left == right failed: left: [1], right: [1, 2, 3]` | the same test, `ok` |
| AC2 | `visit::tests::arq_134_a_visitor_with_only_visit_node_receives_every_node`: `left: [1], right: [1, 2, 3]` | the same test, `ok` |
| AC3 | `visit_contract::arq_134_the_dispatch_has_no_catch_all_arm`: the dispatch block printed with the arm `_ => v.visit_node(node),` in it | the same test, `ok` |
| AC4 | `visit::tests::arq_135_the_view_borrows_the_page_and_its_arena`: `left: [1], right: [1, 2]` | the same test, `ok` |
| AC5 | `visit::tests::arq_135_a_page_without_roots_walks_to_an_empty_vector`: panicked `the page has a root` | the same test, `ok` |
| AC6 | `visit::tests::arq_135_the_walk_answers_with_owned_values`: `left: [1], right: [1, 2]`. The type half of the criterion is a compile-time check: the page is dropped before the assertion, so the test only compiles because `Out` is owned (`walk` returns `Vec<V::Out>`, the visitor's `Out` is `NodeId`) | the same test, `ok` |
| AC7 | `visit::tests::arq_136_pre_order_visits_the_parent_before_its_children`: `left: [1], right: [1, 2, 4, 3]`; `visit::tests::arq_136_post_order_visits_the_parent_after_its_children`: `left: [1], right: [4, 2, 3, 1]` | both tests, `ok` |
| AC8 | `visit::tests::arq_136_a_cycle_ends_the_walk_and_visits_each_node_once`: `left: [1], right: [1, 2]` (the mutual cycle; the own cycle already reached `[1]`) | the same test, `ok` |
| AC9 | `visit::tests::arq_136_a_missing_child_is_skipped_and_the_rest_is_walked`: `left: [1], right: [1, 2, 3]` | the same test, `ok` |
| AC10 | `visit::tests::arq_136_an_unusable_dpi_degenerates_the_measurement`: `assertion left == right failed: dpi NaN, left: (10.0, 10.0), right: (0.0, 0.0)` | the same test, `ok` |
| AC11 | command `grep -rn "RenderGraphBuilder\|gafira_render" crates/gafira-doc/src/`: no match (grep exit 1, which is the pass). The companion `visit_contract::arq_137_the_domain_depends_on_no_crate_above_it` reads the manifest and finds no crate above the domain | the same command, no match, and the test, `ok` |
| AC12 | the CI policy steps and `cargo test --workspace --locked`: the ARQ-13 invariants are checked by `Document::validate`, whose tests are part of that run. `crates/gafira-doc/src/lib.rs` keeps `#![forbid(unsafe_code)]` | every step exit 0, `cargo test --workspace --locked` exit 0 |

The RED run of `visit_contract::arq_134_the_trait_declares_one_method_per_node_kind` failed on its own name extraction (the names came out without the `visit_` prefix, `the trait declares ["node", "group", "shape", "extension"]`), so the dispatch test above carries the RED evidence of AC3; the trait test was fixed and passes.

### Negative controls (run, then reverted)

The critical code of DC1 is the walk order, the cycle guard, the borrow rules of `PageView` and the empty-page case (design, "Critical code"). Each mutant was applied to the delivered code, the test that kills it was run, and the mutant was reverted. No mutant was committed.

| Mutant | Wrong behaviour | Killing test | Failure |
|---|---|---|---|
| 1 | the two orders are inverted: `PreOrder` visits the parent after its children | `visit::tests::arq_136_pre_order_visits_the_parent_before_its_children` | `left: [4, 2, 3, 1], right: [1, 2, 4, 3]` (5 tests failed) |
| 2 | the cycle guard becomes a depth cap: the walk still ends, but a node is visited more than once | `visit::tests::arq_136_a_cycle_ends_the_walk_and_visits_each_node_once` | `left: [1, 1, 1, 1, 1, 1, 1, 1], right: [1]` |
| 3 | `PageView` owns a clone of the page instead of borrowing it | `visit::tests::arq_135_the_view_borrows_the_page_and_its_arena` | `assertion left == right failed: the walk cloned or copied the arena, left: [2, 2], right: [1, 1]` |
| 4 | the walk panics when the page has no roots | `visit::tests::arq_135_a_page_without_roots_walks_to_an_empty_vector` | panicked `the page has a root` |

Mutant 2 was written as a depth cap and not as the removal of the guard: the removal does not terminate, and the criterion is that the walk ends *and* visits each node once, so a mutant that hangs would not fail an assertion (ADR 0004).

The assertions that must not move, and that the mutants above show bite:

- a child that does not exist in the arena is skipped, the other children are walked and nothing panics (`arq_136_a_missing_child_is_skipped_and_the_rest_is_walked`);
- the arena is not cloned: the `Arc::strong_count` of every node is the same before and after the walk, and the node the visitor receives is the one in the arena (`arq_135_the_view_borrows_the_page_and_its_arena`);
- the dispatch over `NodeKind` has no catch-all arm (`arq_134_the_dispatch_has_no_catch_all_arm`);
- the domain's public API names no render graph (the `grep` of AC11, and `arq_137_the_domain_depends_on_no_crate_above_it`);
- the walk does not filter by `visible` or `locked` (`arq_136_the_walk_visits_hidden_and_locked_nodes`).

### Choices and deviations

- The instant is a type parameter, not `Flicks`: the layer table of CI (V5) gives `gafira-doc` only `gafira-geom` and `gafira-color`, and this delivery changes no manifest and no `Cargo.lock`, so it can run beside the `RE` and `ME` streams. `NodeBehavior<T = ()>` and `NodeExtension<T = ()>` therefore take the instant as the caller's type and the domain names no time type. ARQ-136's rule holds: the instant is a separate argument, never in the context.
- `local_bounds` answers with the node's own placed box; the union of a group's children is `gafira-layout`'s answer (D-28, ADR 0009). The domain does not depend on the layout engine.
- The DPI decides whether the measurement is defined and nothing else, because no node kind sizes itself in physical units yet: with a usable DPI the box is the node's page-space box, and with an unusable one it is the degenerate box of 02 §4.1 (width 0, height 0) and the hit test is `false`.
- `NodeExtension` is declared here with `bounds` and `inspector_schema`, and with no render reference (ARQ-137). The handler that draws an extension belongs to the render engine (design P5, `RE1`): the domain registers nothing and knows no handler type.
- The dispatch over `NodeKind` has no catch-all arm, and the shape is checked by a test that reads the source (`arq_134_the_dispatch_has_no_catch_all_arm`), because ARQ-134 asks for the grep and for the compile failure that adding a variant produces; that compile failure cannot be a unit test.
- The walk answers with owned values: the visitor's `Out` cannot borrow a node, because the trait's methods take `&Node` with a short lifetime and return `Self::Out` (ARQ-135).
- `PageView` and `MeasureCx` are neither `Clone` nor `Copy`: they hold a `&Page` and nothing else, so handing one over copies nothing.
- The hit test follows kurbo's convention: the top-left corner is inside the box and the bottom-right edge is not.
- The walk uses an explicit stack, so a deep tree cannot overflow the call stack, as `Document::validate` does for its own walk.

Measured: `git diff --shortstat develop...HEAD` before this note: 4 files, 753 insertions, of which 33 are the Slice card: about 720 authored lines for DC1 (forecast 430, x1.4 = 600). Over the ~700 signal of `AGENTS.md`: the coordinator decides.

### Coordinator decisions on verify round 1

- **W3 (720 lines of code, over the ~700 ceiling): accepted.** The delivery is the visitor contract with its tests and the shape checks, and the size is 3 % over the ceiling of a `full` run. The owner gave the coordinator the decision in YOLO mode on 2026-11-03, and the coordinator accepts it here. The measurement is recorded in the verification report.
- **W1 (ARQ-136 says the instant travels apart as `t: Flicks`): routed to `specifier`.** The rule is satisfied: the instant is a separate argument and never a field of `MeasureCx`. The literal type is unreachable from `gafira-doc` by the layering of 02 §2.1 and §2.2, so the row has to say it in a way that the domain can honour. The row stays as it is until `specifier` reconciles it.
- **W2 (nothing ties "the instant is not in the context"): routed to a fix round of `tdd-apply`.** A source-shape test has to fail if `MeasureCx` ever grows a time field.
