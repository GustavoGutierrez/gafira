# Glossary

| Term | Meaning |
|---|---|
| ADR | Architecture decision record in spec 02 (for example ADR-01 crate granularity, ADR-03 history) |
| ARQ-nn | Architecture requirement with an acceptance criterion, in spec 02 |
| Arena | The per-page `BTreeMap<NodeId, Arc<Node>>` that owns the nodes of a page |
| Canonical JSON | The deterministic compact form written by `Document::to_json` |
| Command | An undoable edit of a document; the `Command` trait in `gafira-edit` |
| Copy on write | Mutation through `Arc::make_mut`, which copies only the shared items along the edited path |
| D-nn | An owner decision in `docs/specs/00-decisiones.md`; it overrides other specs |
| Delivery | One reviewable slice of a change (`D1`, `D2a`...), built with strict TDD |
| DirtySet | The pages and nodes a command changes, used for cache invalidation |
| `DocMut` | The exclusive editing view of a `Document`, obtained with an `EditToken` |
| Domain crate | A crate in layers 0 and 1 with a `clippy.toml` and a V5 row |
| `EditToken` | The non-copyable proof that the holder may edit documents |
| Gesture | One user drag or similar action, named by a `GestureId`, whose commands may merge |
| Golden file | A stored expected output compared byte for byte in tests |
| Hexagonal architecture | Ports and adapters: the domain is pure and adapters implement ports |
| Lite SDD | The spec, apply, verify, archive flow of this repository |
| MergeKey | The key under which two commands may be merged into one history entry |
| Mutant | A small deliberate code change made by `cargo-mutants`; a surviving mutant means a test gap |
| OpenSpec | The layout of main specs and change folders under `openspec/` |
| P1 to P6 | The ordered passes of `Document::validate` |
| Planned | Specified but not implemented in the repository yet |
| Port | A trait defined by the application layer that adapters implement (**Planned**, `gafira-ports`) |
| Transaction | A composite command: many commands, one history entry, all or nothing |
| V5, V12, V13 | CI policy steps: layer table, `clippy.toml` sync, `EditToken` confinement |
| Wire form | The JSON shape of a type on disk |
