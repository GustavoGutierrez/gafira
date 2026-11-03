## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator | reason: new crate with a new V5 row and port types the spec left without a home (architect, ADRs 0013 and 0014 accepted by the owner)
Product IDs: ARQ-01, ARQ-31, ARQ-50, ARQ-93, ARQ-107, V-5 (type) for D1; D2 to D6 per the delivery table (source: specs/02-arquitectura-software.md §2.1, §4.5-4.16; design: specs/changes/gafira-ports/design.md)
Scope: D1 of the design: the crate, all shared files, `ContentHash` in gafira-doc, `registry`, `common`, `system`, `store`, module stubs.      Non-goals: D2 to D6 modules; spec gaps S1 to S16 (specifier, in parallel on feat/gafira-ports); adapters.
Deliveries: D1 serial first (~420 at x1.4); then D2, D3, D4, D5a, D6 parallel, each in its own worktree; D5b after D3, D4, D5a. T4 at close: gafira-ports, then gafira-doc.
Plan: per design §Acceptance and §P8 for D1, test first; negative controls of §Acceptance run by verify.
Owner decisions touched: D-12 through O1 (a), D-02 and D-08 unchanged (checked D-48 to D-50).
