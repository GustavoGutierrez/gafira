## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Product IDs: RS1 ARQ-117, ARQ-121, ARQ-110, ARQ-111; RS2 ARQ-119, FD-12 (c, CPU); RS3 ARQ-118, ARQ-25 (ends); RS4 ARQ-120, FD-35 (a, d; Paralela only, D-42), ARQ-114 (bench_shadow_1080); RS5 FD-33 (a, d to h) except colours detected by ΔE; RS6 ARQ-122, FD-26, FD-28 (b), FD-33 (detected colours), FD-34 (a, b; D-40, D-41), ARQ-114 (bench_dhash_12mp) (source: specs/02-arquitectura-software.md, specs/01-producto-y-calidad.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/05-funcionalidad-video-y-distribucion.md)
Scope: RS1 to RS6 of specs/changes/layer-2/design.md, with P11 for the blur.      Non-goals: other streams; the filter catalogue (Q-34); Brillo and Contorno (D-42); `f16` buffers.
Deliveries: RS1 to RS6, serial; each verified and merged before the next.
Plan: specs/changes/layer-2/design.md, table and decisions; `crates/gafira-raster/` plus the files P2 assigns.
Owner decisions touched: D-40 to D-44 as recorded; none reopened.

# Apply progress: gafira-raster
