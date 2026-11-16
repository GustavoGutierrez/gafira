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
| Critical logic | Code where a defect corrupts data or breaks an invariant (document model, JSON, ids, edit history, time, color, geometry, limits, format parsing); it is built test-first (TDD) |
| D-nn | An owner decision in `specs/00-decisiones.md`; it overrides other specs |
| Definition of Ready | The list of sections a spec needs before it leaves `specs/backlog/`: problem, scope, dependencies, requirements, acceptance criteria, TDD cases, evidence, risks |
| DirtySet | The pages and nodes a command changes, used for cache invalidation |
| `DocMut` | The exclusive editing view of a `Document`, obtained with an `EditToken` |
| Domain crate | A crate in layers 0 and 1 with a `clippy.toml` and a V5 row |
| `EditToken` | The non-copyable proof that the holder may edit documents |
| Flick | The exact time unit of `gafira-time`: 1/705 600 000 of a second |
| Gate | A check in `AGENTS.md` "Before calling something done" (`fmt`, `clippy`, tests, policy steps, `manual.py check`, `sync_agents.py --check`) |
| Gesture | One user drag or similar action, named by a `GestureId`, whose commands may merge |
| Golden file | A stored expected output compared byte for byte in tests |
| Hand mutation | A deliberate one-line change to finished code, applied by hand and then reverted, to show that a test can fail |
| Hexagonal architecture | Ports and adapters: the domain is pure and adapters implement ports |
| History | The owner of the session `Arc<Document>` and its undo and redo stacks (`gafira-edit`) |
| Law | One of the six properties `prop_command` checks for a command |
| MergeKey | The key under which two commands may be merged into one history entry |
| Mutant | A small deliberate code change made by `cargo-mutants`; a surviving mutant means a test gap |
| NodeSpec | A node described without ids, the input of `InsertNodes` |
| P1 to P6 | The ordered passes of `Document::validate` |
| Product reference document | One of `specs/00-..07-*.md`: owner decisions, product, architecture, features or open debt; cited by specs, not a lifecycle spec |
| Planned | Specified but not implemented in the repository yet |
| Port | A trait of layer 3 that adapters implement; the port traits and their registries live in `gafira-ports` |
| Snapshot | The `Arc<Document>` before or after a history entry; it shares unchanged pages and nodes with its neighbors |
| Spec | One file `specs/NNN-name.md` that describes a unit of work, with a status banner, requirements and acceptance criteria |
| Spec ID | The three-digit, zero-padded number of a spec (`009`); assigned at creation and never reused or changed |
| Status banner | The `> **Estado:**` line under a spec's title: `backlog`, `in-progress` or `done`, a date and the evidence or blocker |
| Sync script | `tools/sync_agents.py`: links `.claude/`, `.opencode/` and `.pi/` to `.agents/`; `--check` reports drift |
| `testkit` | `gafira-testkit`, the test-support crate with the document generator and `prop_command` |
| `trybuild` | The dev-dependency that checks that fixtures fail (or pass) to compile; evidence for ARQ-30 |
| Transaction | A composite command: many commands, one history entry, all or nothing |
| V5, V12, V13 | CI policy steps: layer table, `clippy.toml` sync, `EditToken` confinement |
| V14 | CI step that checks that every core command has a property test (ARQ-81) |
| Wire form | The JSON shape of a type on disk |

## Historical terms

These terms belong to the earlier multi-agent flow that ADR 0018 retired. They appear in `specs/done/legacy/` and in the superseded ADRs.

| Term | Meaning |
|---|---|
| Lite SDD, ODD lite | The earlier flow: a coordinator routed each request through repo agents (spec, apply, verify, archive), with risk levels and flow modes (ADR 0001, 0002, 0004, 0011) |
| Change, change folder | One unit of work of the earlier flow and its folder of artifacts (proposal, design, delta specs, verify report); closed folders are in `specs/done/legacy/history/` |
| Delta spec | The requirements a change added, modified or removed in a capability spec, merged at archive |
| Delivery | One reviewable slice of a change (`D1`, `D2a`...) |
| Slice card | The block that defined the scope and deliveries of a fast-lane change |
| Outcome entry | The record of one active outcome in `.odd/state.json`, a file that no longer exists |
| Mutation tier (T0 to T4) | How much code a mutation run covered, from none to the whole crate |
