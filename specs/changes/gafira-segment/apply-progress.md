## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Product IDs: SG1 ARQ-130 (shape, sizes, hints), ARQ-110; SG2 ARQ-130 (seeds, zones), FD-38 (reduction); SG3 ARQ-131, ARQ-132, FD-32 (e), FD-38 (a, b, c), ARQ-43, CAL-06, ARQ-111, ARQ-112 (source: specs/02-arquitectura-software.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/01-producto-y-calidad.md)
Scope: SG1 to SG3 of specs/changes/layer-2/design.md, under D-45: colour only, working copy of long side 2048 px, 16 fixed blocking scenes, 12 fresh scenes as a warning. XP11 is resolved: a preallocated open-addressing table of u64 pair keys, chunks of 256 rows for adjacency and 250 for the guided filter, about 11 ms or less per whole image on the prototype. O4 is closed by D-45; where design.md still treats it as open, D-45 wins.      Non-goals: other streams; models or ONNX; a real-photo set; changing the budgets of CAL-06 and CAL-26 (Q-48 goes to the owner only if `bench_segment_4000` fails).
Deliveries: SG1, SG2, SG3, serial; each verified and merged before the next.
Plan: specs/changes/layer-2/design.md, table and decisions; `crates/gafira-segment/`, and `crates/gafira-testkit/src/scenes.rs` in SG3.
Owner decisions touched: D-45 as recorded; D-20, D-23, D-44 respected; none reopened.

# Apply progress: gafira-segment
