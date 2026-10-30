## Slice card
Flow: full | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator | reason: new V5 row and cross-cutting choices (ADRs 0005 to 0010); owner decisions D-14 to D-39 settled
Flow-change: standard -> full (trigger: spike found a new V5 row and the guide-type location conflict, before implementation)
Product IDs: FD-03, FD-04, FD-05, FD-06, FD-07, FD-09, FD-11 first-slice criteria, ARQ-110, ARQ-111, ARQ-114 (source: docs/specs/04-funcionalidad-diseno-imagen-exportacion.md §2 and §3; docs/specs/02-arquitectura-software.md §4.17)
Scope: `gafira-doc` page layout data (bleed, margins, guides, grid), `gafira-edit` properties, and the new crate `gafira-layout`: bounds, rects, guides, grid, snap, align, distribute.
Non-goals: FD-53, FD-54, reflow, `notes`, `duration`, `background`, text and image nodes, UI, PDF export, `rstar`.
Deliveries (serial, design.md delivery table): D1 doc types and wire form ~460; D2 validation and edit properties ~420; D3 crate, V5 row, bounds, rects ~420; D4 guides and grid ~475; D5 snap ~475; D6 align and distribute ~335; D7 benches ~125.
Plan: one `tdd-apply` per delivery on `feat/layout-core-<delivery>` cut from `develop`, in the table order; `verify` V2 per delivery; T4 `mutants` on `gafira-doc`, `gafira-edit` and `gafira-layout` at close; `archive`, copying tasks T1 and T2 of design.md to the follow-up list.
Owner decisions touched: none reopened (checked D-14 to D-39; Q-34 does not apply to this change).
