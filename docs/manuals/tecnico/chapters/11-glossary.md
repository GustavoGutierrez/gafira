# Glossary

| Term | Meaning |
|---|---|
| ADR | Architecture decision record in spec 02 (for example ADR-01 crate granularity, ADR-03 history) |
| ARQ-nn | Architecture requirement with an acceptance criterion, in spec 02 |
| Budget | The most estimated bytes of snapshots a `History` keeps before it compacts the oldest entries; `DEFAULT_BUDGET` is 512 MiB |
| Arena | The per-page `BTreeMap<NodeId, Arc<Node>>` that owns the nodes of a page |
| Canonical JSON | The deterministic compact form written by `Document::to_json` |
| Compaction | Dropping the snapshots of the oldest history entries over the budget; the entry keeps its command and is undone with `revert` |
| Command | An undoable edit of a document; the `Command` trait in `gafira-edit` |
| Copy on write | Mutation through `Arc::make_mut`, which copies only the shared items along the edited path |
| D-nn | An owner decision in `specs/00-decisiones.md`; it overrides other specs |
| Delivery | One reviewable slice of a change (`D1`, `D2a`...), built with strict TDD |
| DirtySet | The pages and nodes a command changes, used for cache invalidation |
| `DocMut` | The exclusive editing view of a `Document`, obtained with an `EditToken` |
| Domain crate | A crate in layers 0 and 1 with a `clippy.toml` and a V5 row |
| `EditToken` | The non-copyable proof that the holder may edit documents |
| Flick | The exact time unit of `gafira-time`: 1/705 600 000 of a second |
| Gesture | One user drag or similar action, named by a `GestureId`, whose commands may merge |
| Fix round | A return to `tdd-apply` after a verify FAIL, with tests first and a new verify |
| Golden file | A stored expected output compared byte for byte in tests |
| Hand mutation | A deliberate one-line change to finished code, applied by hand to show that a test fails; recorded as a selection, not a full run |
| Hexagonal architecture | Ports and adapters: the domain is pure and adapters implement ports |
| History | The owner of the session `Arc<Document>` and its undo and redo stacks (`gafira-edit`) |
| Law | One of the six properties `prop_command` checks for a command |
| Lite SDD | The spec, apply, verify, archive flow of this repository |
| Fast lane | The route of ADR 0002 (on covered work the outcome entry now holds the card fields): a product spec runs directly from its requirement IDs through a Slice card, with no proposal, design or delta specs |
| Finding class | The `class:` of a verify finding (`implementation`, `architecture`, `ambiguity`, `claim`); the coordinator routes by it |
| Flow mode | Light, standard or full: the set of agents, gates and checks a request runs; chosen by the coordinator |
| Gate (G0 to G5) | A check the coordinator makes between phases of a flow; not the same as the local gates (`fmt`, `clippy`, `test`) |
| Light spec | The block of acceptance criteria the coordinator writes at the top of `apply-progress.md` in light mode |
| Slice card | The block the coordinator writes at the top of `apply-progress.md` in the fast lane: `Flow:` line, product IDs, scope, non-goals, deliveries and plan; it satisfies G0 |
| Mutation tier (T0 to T4) | How much code the mutation run covers: none, hand mutants, diff, changed files, whole crate |
| Mutation toggle | The `mutation: off\|archive\|delivery` field of the `Flow:` line; it sets when `cargo-mutants` runs (default `archive`: T4 per touched crate at change close) |
| Pack | The predefined mode, agents, `mutation` value and verify depth for a request type |
| `verification.json` | The machine-readable record of each verify round, written by `verify` (pilot) |
| Verify depth (V0 to V3) | How much independent work `verify` does, from deterministic checks only to the full matrix |
| MergeKey | The key under which two commands may be merged into one history entry |
| Mutant | A small deliberate code change made by `cargo-mutants`; a surviving mutant means a test gap |
| NodeSpec | A node described without ids, the input of `InsertNodes` |
| BLOCKED | A `verify` verdict: a critical check could not run; it goes to the owner, never counts as PASS and costs no round |
| Change folder | The folder `specs/changes/<change>/` that holds the artifacts of one change; it closes into `specs/history/` |
| ODD lite | Outcome-driven routing on top of the flow modes (ADR 0011, proposed): risk levels, outcome entries and BLOCKED |
| Outcome entry | The object in `.odd/state.json` that records the criteria, status and next action of one active outcome |
| Risk level (L0 to L3) | The class the coordinator gives a request by impact, uncertainty and reversibility; it can raise the flow mode and never lowers a floor |
| P1 to P6 | The ordered passes of `Document::validate` |
| Planned | Specified but not implemented in the repository yet |
| Port | A trait defined by the application layer that adapters implement (**Planned**, `gafira-ports`) |
| Snapshot | The `Arc<Document>` before or after a history entry; it shares unchanged pages and nodes with its neighbors |
| `testkit` | `gafira-testkit`, the test-support crate with the document generator and `prop_command` |
| `trybuild` | The dev-dependency that checks that fixtures fail (or pass) to compile; evidence for ARQ-30 |
| Transaction | A composite command: many commands, one history entry, all or nothing |
| V5, V12, V13 | CI policy steps: layer table, `clippy.toml` sync, `EditToken` confinement |
| V14 | CI step that checks that every core command has a property test (ARQ-81) |
| Wire form | The JSON shape of a type on disk |
