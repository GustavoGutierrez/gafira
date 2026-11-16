# 0002. Fast lane: spec-direct execution, mutation toggle, lighter defaults

- Status: superseded by 0018
- Date: 2026-10-29 (decided by the owner)
- Related: ADR 0001 (supersedes the parts named below), `AGENTS.md`, `.agents/agents/coordinator.md`, change `fast-lane-flow`; product spec `docs/specs/02-arquitectura-software.md`

## Context

The owner finds the flow too slow. Measured causes in the ADR 0001 flow:
- Every slice of `02-arquitectura-software.md` creates a new crate, and "new crate" is a full floor: `specifier` and `architect` on Opus, ~700-line deliveries, per-delivery `mutants` and V3 `verify`, T4 at archive.
- The owner's product spec is already refined (Spanish, stable IDs, acceptance criteria per row). It is rewritten as `proposal.md` plus English delta specs and merged again into `openspec/specs/` at archive.
- V3 (per-commit clean exports, probes, hand mutants) produces repeated rounds: `engine-pathops` took six verify rounds.
- `mutants` runs every delivery and every fix round with `--jobs 4` on a 20-core machine, with debug symbols in the test profile.

## Decision

Make a fast lane the default route. The coordinator runs a product spec (or a section or ID range of it) directly from its requirement IDs, mutation testing runs once per touched crate at change close unless the Flow line says otherwise, verify defaults drop to V1/V2, and a new crate that `02` already describes no longer forces full mode.

### 1. Flow line and mutation toggle

`Flow: <mode> | pack: <pack> | mutation: off|archive|delivery | tier: T<n> | verify: V<n> | decided-by: coordinator|owner | reason: <one line>`

| `mutation` | Effect |
|---|---|
| `archive` (default) | No per-delivery `cargo-mutants` run. When the change closes (all deliveries PASS, before archive), one T4 run per touched crate. A critical survivor goes back to `tdd-apply`, then the final `verify` |
| `delivery` | ADR 0001 behaviour: T2/T3 per delivery (with `--iterate` on test-only fix rounds), T4 at close. Only on owner request, or when the coordinator flags a new rule on critical code |
| `off` | No `cargo-mutants` at all. `verify` hand mutants at V2 still apply |

Owner overrides such as "skip mutation" map to `off`. `tier` keeps naming the per-delivery tier for `delivery`, and is `T4` for `archive`, `T0` for `off`.

### 2. cargo-mutants speed

- `Cargo.toml` gets `[profile.mutants]` (`inherits = "test"`, `debug = "none"`); `.cargo/mutants.toml` sets `profile = "mutants"`.
- `--jobs 8` replaces `--jobs 4` everywhere. Disk and target-dir hygiene rules stay.
- A fast linker (mold, wild) would help further; it is optional and not installed.

### 3. Spec-direct execution (Slice card)

When the owner asks to execute a product spec in `docs/specs/` (or a section or ID range), the coordinator writes a "Slice card" at the top of `openspec/changes/<change>/apply-progress.md`, committed as `docs(<change>): slice card` before any implementation. It satisfies G0.

```
## Slice card
Flow: <mode> | pack: <pack> | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator|owner | reason: <one line>
Product IDs: <ARQ-nn, RF-nn ...> (source: docs/specs/<NN-doc>.md §x)
Scope: <1 to 3 lines>      Non-goals: <list>
Deliveries: <only if more than one: D1 <scope, ~lines>, D2 ...; serial>
Plan: <ordered steps, expected crates and files>
Owner decisions touched: none (checked D-nn: ...)
```

- No `proposal.md`, no `design.md`, no delta specs. The acceptance source for `tdd-apply` and `verify` is the product spec rows of those IDs plus the Slice card.
- Each test names the requirement ID it proves (in the test name or a one-line comment), so `git grep ARQ-nn` traces coverage.
- `specifier` runs only when `tdd-apply` reports `blocked` or `verify` reports a finding of class `ambiguity`. It then fixes the product spec in `docs/specs/` (Spanish, skill `product-specs`), not a delta spec, unless the owner asks for one.
- The coordinator's write authority (ADR 0001 section 4) extends to the Slice card with the same limits; it never edits the product spec itself.
- Requests that are not product-spec slices keep the Light spec block unchanged.

### 4. Floors

| Signal | Minimum |
|---|---|
| Owner decision needed | stop and ask |
| New V5 layer-table row, new dependency edge not already stated in `02`, new serialized format version, or a cross-cutting design choice the spec leaves open | full (`architect`) |
| New crate that `02` already describes (crate list, layer, dependencies) | standard (no `architect`) |

Delivery size: ~700 lines stays a signal to stop and ask. One change may run several deliveries serially without `architect` when the Slice card lists them; `architect` runs only for an open design decision or when deliveries need a dependency table with parallel work.

### 5. Verify defaults

V1 for docs and tooling packs, V2 for code. V3 runs only on owner request. `verify` reads mutation results only when a run happened; with `mutation: archive` it receives the T4 report at the final verify of the change (or the coordinator checks for critical survivors before archive and routes them back).

### 6. Archive

A fast-lane change has no delta merge: `archive` moves the change folder to `openspec/changes/archive/<date>-<change>/` and updates the "Detalle" column of the `docs/specs/README.md` status table with the covered IDs. Changes with delta specs keep the ADR 0001 merge. `openspec/specs/` stays as the record of past changes; new fast-lane work does not edit it.

### What ADR 0001 keeps

Modes (light, standard, full), packs, tiers T0 to T4, depths V0 to V3, gates G0 to G5, `verification.json`, the feedback loop and retry caps, the Light spec block and the coordinator's limits. Superseded parts of 0001: the new-crate full floor, the default tiers per delivery, the default verify depths, and delta specs as mandatory for product-spec slices.

## Consequences

Easier:
- A slice of `02` runs as `tdd-apply` (one or more deliveries) plus `verify` V2, then one T4 per crate and archive: two to four agent runs, no Opus spec or design run unless something is open.
- The product spec stays the single source of truth; no English restatement drifts from it.
- Mutation cost drops to one whole-crate run per touched crate per change, at about twice the parallelism and without debug info.

Harder and risks:
- Test adequacy on changed lines is checked later: a weak test shows up at the close, not per delivery. Mitigation: V2 probes and hand mutants per delivery, `delivery` mode for new critical rules, and the T4 critical-survivor route back to `tdd-apply`.
- Coverage traceability depends on tests naming IDs. Mitigation: `verify` checks every Slice card ID has a test that names it.
- Product spec rows must be precise enough to test; gaps now surface as `blocked` and go to `specifier` against the product spec.
- `openspec/specs/` stops growing for new work; readers look at `docs/specs/` and the tests.

## Alternatives considered

- Keep full mode for new crates and only lower verify. Rejected: the spec rewrite and `architect` runs are most of the cost and add nothing when `02` already fixes the crate, layer and dependencies.
- Drop mutation testing. Rejected as default: survivors found most CRITICAL items (ADR 0001); `off` exists for the owner.
- Keep delta specs but generate them mechanically from the product spec. Rejected: still two documents to keep in step, and an archive merge with no reader.
- Raise `--jobs` to 16 or more. Not adopted: each job copies the tree and builds; 8 leaves room for the rest of the machine and the disk rule.
