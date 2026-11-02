## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Product IDs: TX1 FD-41 (a, d), ARQ-115, ARQ-116, ARQ-75 (partial file), ARQ-110, ARQ-111; TX2 FD-41 (c, e to j); TX3 FD-45 (a, c, d, e), FD-46 (a, b, c, e, f), ARQ-14 (FitBox); TX4 FD-44 (a, c, d, e), FD-43 (a), ARQ-114 (text) (source: specs/02-arquitectura-software.md, specs/01-producto-y-calidad.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/05-funcionalidad-video-y-distribucion.md)
Scope: TX1 to TX4 of specs/changes/layer-2/design.md, with P7 and the XP3 and XP10 records; TX3 needs one workflow_dispatch during its verify (ARQ-14).      Non-goals: other streams; FD-41 (b), FD-47, FD-49, FD-42, FD-46 (d), FD-43 (b, c), FD-44 (b), FD-48; `StyleProperty::Locale` (ARQ-107).
Deliveries: TX1, TX2, TX3, TX4, serial; each verified and merged before the next.
Plan: specs/changes/layer-2/design.md, table and decisions; `crates/gafira-text/` plus the files P2 assigns.
Owner decisions touched: D-40 to D-44 as recorded; none reopened.

# Apply progress: gafira-text
