## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the timeline opens with the edge that lets the document name time, which every later delivery of this change needs; the rigor is the one the owner fixed for the previous two cuts (O4, O8, O10): hand mutants, gates per commit, one V1 round per group, no T4 at close
Product IDs: ARQ-01, ARQ-02, ARQ-10, ARQ-13, ARQ-30, ARQ-136 (source: specs/02-arquitectura-software.md §2.1, §2.2, §3.1, §3.2, §4.14; ADR 0007; V-2 of specs/05-funcionalidad-video-y-distribucion.md)
Scope: `TL1` of `specs/changes/gafira-timeline/design.md`: in `gafira-doc`, the dependency on `gafira-time` and `Page.duration: Option<Flicks>` with its wire form; in `ci.yml`, the V5 rows that must gain `gafira-time` because the V5 step reads the whole `cargo tree`; and the goldens the new field moves.      Non-goals: `Sequence`, `Track` and `Clip` (`TL2`), the persistence of the model (`TL3`), the commands and the resolver (later changes), and any change to `gafira-ports` or to an engine crate beyond the `Cargo.toml` edge and its V5 row.
Acceptance:
  AC1 Given `gafira-doc`, When its manifest is inspected, Then it declares `gafira-time` and the crate builds, and the edge is the one ADR 0007 named as arriving with `Page.duration` (ARQ-01, V-5) [command]
  AC2 Given a `Page`, When it carries a duration, Then it is an `Option<Flicks>`, absent when the page has none, and the type comes from `gafira-time` and not from a unit of the crate's own (V-2, ARQ-136) [unit]
  AC3 Given a page without a duration, When it is written and read back, Then the field is absent in the wire form and the round trip gives `None`, so a document written before this change still loads (ARQ-10, ARQ-13, ADR 0006 rule 3) [unit]
  AC4 Given the V5 step, When the workspace is checked, Then every crate whose `cargo tree` now reaches `gafira-time` has it in its row, and the step passes (V-5) [command]
  AC5 Given the byte-pinned golden, When the tests run, Then it moves with the new field and the artifact-integrity review confirms it changed only for that reason (ARQ-10, ADR 0006 rule 3) [unit]
  AC6 Given the crate, When the CI policy steps run, Then `gafira-doc` still has `#![forbid(unsafe_code)]`, its V5 row passes, V12 keeps its `clippy.toml` identical and ARQ-107 still holds (ARQ-02, ARQ-107) [command]
  AC7 Given a `Document` with pages and their durations, When the document is cloned or compared, Then the new field takes part in equality and in the deterministic ordering the crate already fixes (ARQ-10, ARQ-13) [unit]
Plan: the delivery is an edge and a field, not a model. It touches the crates' manifests and the V5 table as one work unit, because the V5 step reads the whole `cargo tree`: a row without the manifest entry, or the reverse, fails the build. `Page.duration` is the trigger ADR 0007 named, and it is additive in the wire form, so no migration is needed while `Document::SCHEMA_VERSION` stays at 1 (owner decision Q1). The critical code is the wire form of the new field and the V5 rows (design, "Critical code"), so those carry the hand mutants. The layer rule that matters: `gafira-doc` may depend on `gafira-time` after this edge and still on nothing else.
Owner decisions touched: the rigor the owner fixed for O4, O8 and O10, applied here as the same pattern; Q1 of the timeline design (schema 1, additive) if accepted; ADR 0007 (the edge and its trigger); no D-nn is contradicted.

# Apply progress: gafira-timeline

## TL1 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` (the edge touches many manifests, so the workspace scope applies), `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
