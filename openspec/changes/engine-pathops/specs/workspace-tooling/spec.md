# Workspace Tooling Specification (delta for C4 `engine-pathops`)

## Purpose

C4 adds the layer-2 engine crate `gafira-pathops`, which needs a V5 row and a
`clippy.toml` copy like every other crate in `crates/`. This delta extends the V5
allowed-dependency table and the clippy-copy policy with the new crate. The V5 and
V12 step bodies stay unchanged: only the table and the copy list grow. Requirements
not listed here are unchanged. Decisions `design.md` Q1 and Q4 are closed; none stays open for this capability.

The blocks below are rebased onto the current `openspec/specs/workspace-tooling/spec.md`: each is that
requirement as it stands on main, with only the `gafira-pathops` row, note, copy and scenarios added.

## MODIFIED Requirements

### Requirement: Per-crate allowed-dependency table (V5)

C3 delivery: D1, D2. Scenarios without a delivery tag below belong to it. C4 delivery: D1 (the `gafira-pathops` row, its note and the scenarios tagged `C4 D1`).

CI step V5 MUST be driven by one table of allowed internal dependencies per domain crate, not by a step hard-coded for `gafira-geom`. The table after this change, in the order `ci.yml` lists it:

| Crate | Allowed `gafira-*` dependencies |
|---|---|
| `gafira-geom` | none (`-`) |
| `gafira-color` | none (`-`) |
| `gafira-time` | none (`-`) (added by `domain-time`) |
| `gafira-doc` | `gafira-geom`, `gafira-color` (and no other `gafira-*` crate) |
| `gafira-edit` | `gafira-geom`, `gafira-color`, `gafira-doc` |
| `gafira-testkit` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit` (no `clippy.toml`; the row constrains it anyway, `design.md` Q10) |
| `gafira-pathops` | `gafira-geom` (and no other `gafira-*` crate) (added by `engine-pathops`) |

The `gafira-doc` row MUST land in the same commit as the crate's `clippy.toml`, because a `clippy.toml` without a row fails V5; the same holds for the `gafira-edit` row (C3 D1) and for the `gafira-pathops` row (C4 D1). The `gafira-testkit` row lands with that crate (C3 D2). The `gafira-time` row and its `clippy.toml` copy landed together in `domain-time` D0. The `gafira-pathops` row lands with that crate (C4 D1). Dev-dependencies (`trybuild`, `proptest`, `gafira-testkit`) are outside `-e normal,build` and MUST NOT appear in a row.

For each table row, V5 MUST run `cargo tree -p <crate> -e normal,build --target all --prefix none -f '{p}'` with standard input redirected from `/dev/null` (so `cargo` cannot consume the table), capture its output before any filtering, and fail when:
- `cargo tree` fails, or the captured tree has no line starting with `<crate> v` (empty tree);
- the tree contains a `gafira-*` crate other than the crate itself and its allowed list;
- the tree contains a line whose crate name STARTS WITH any of `egui`, `eframe`, `winit`, `wgpu`, `tokio`, `mio`, `socket2`, `reqwest`, `ureq`. Main's V5 already uses this prefix match (`^($forbidden)`, `.github/workflows/ci.yml:98`), with no trailing space, so derived names such as `wgpu-core`, `tokio-util` or `winit-x` are also detected. The prefix match MUST be kept and MUST NOT be weakened (for example by a trailing-space anchor or an exact-name match).

The table MUST be iterated in the main shell, fed by a here-string, so the failure status survives the loop; the step MUST run correctly under `bash -eo pipefail` and MUST NOT pass because a `grep` found no match. V5 MUST also fail when a crate that has a `clippy.toml` under `crates/*/` (the domain-crate marker, shared with V12) has no table row. Adding a crate or an allowed edge MUST be an explicit edit of the table; the step body stays unchanged. V5 constrains only `gafira-*` edges and the forbidden prefixes above, not other third-party crates: a direct third-party dependency of a domain crate (for example `serde_json` in `gafira-edit`, allowed by `edit-commands` "Crate purity and dependencies") is governed by that crate's purity requirement, `cargo deny check` and review, and never appears in a row.

`gafira-pathops` is a layer-2 engine crate, not a layer-0/1 domain crate. It is
listed in the table because it carries a `clippy.toml` copy (the marker V5 and V12
use), not because it is a domain crate. Its row constrains its internal edges to
`gafira-geom` alone, which is the ARQ-01 rule for a layer-2 crate over layer 0.

Known limitation: a crate under `crates/` without a `clippy.toml` is not recognized as a domain crate by V5's coverage check and is therefore not required to have a row. V11 still requires `#![forbid(unsafe_code)]` in its `src/lib.rs`, but neither the layer table nor V12 constrains it; closing this gap is deferred to `xtask check-layers` (ARQ-01).

#### Scenario: Clean tree passes (N0 / D-N0, command)
- GIVEN the final tree with `gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit`, `gafira-testkit` and `gafira-pathops`
- WHEN V5 runs from a scratch copy under `bash --noprofile --norc -eo pipefail` with xtrace
- THEN it exits 0 and the trace shows `cargo tree` ran for each of the seven rows, `gafira-edit`, `gafira-testkit`, `gafira-time` and `gafira-pathops` included

#### Scenario: Forbidden internal edge fails (N1, manual negative test)
- GIVEN `gafira-geom.workspace = true` temporarily added to `gafira-color`'s `[dependencies]`
- WHEN V5 runs
- THEN it exits 1 with "gafira-color must not depend on gafira-geom"; after `git checkout -- crates/gafira-color/Cargo.toml Cargo.lock` V5 exits 0

#### Scenario: Misspelled row fails closed (N2, manual negative test)
- GIVEN the table row `gafira-colour -` instead of `gafira-color -`
- WHEN V5 runs
- THEN `cargo tree` fails ("did not match any packages"), the step exits non-zero, and the coverage check reports that `gafira-color` has no row

#### Scenario: Missing row fails (N3 / D-N3, manual negative test)
- GIVEN the `gafira-color` row deleted, and separately the `gafira-doc` row deleted
- WHEN V5 runs
- THEN it exits 1 with "gafira-color has clippy.toml but no V5 row" and "gafira-doc has clippy.toml but no V5 row" respectively

#### Scenario: Forbidden third-party crate fails (N4, manual negative test)
- GIVEN `serde` temporarily added to the `forbidden` pattern (no network needed)
- WHEN V5 runs
- THEN it exits 1 reporting the forbidden `serde v...` line

#### Scenario: Prefixed forbidden crate names fail (N5, manual negative test)
- GIVEN `wgpu-core` (or `tokio-util`) temporarily added as a dependency of `gafira-color`, or the pattern exercised against a captured tree line `wgpu-core v0.1.0`
- WHEN V5 runs
- THEN it exits 1 reporting the forbidden line, although its name is not exactly `wgpu` or `tokio`

#### Scenario: Crate without clippy.toml is not checked (known limitation, manual)
- GIVEN a scratch crate `crates/scratch/` with a valid manifest, `#![forbid(unsafe_code)]`, no `clippy.toml` and no table row
- WHEN V5 runs
- THEN V5 does NOT fail on it; this documents a known limitation that is not met by this change and is recorded in the verify report

#### Scenario: Doc may depend on geom and color (command)
- GIVEN the final tree
- WHEN `cargo tree -p gafira-doc -e normal,build --target all --prefix none -f '{p}'` is filtered for `gafira-*`
- THEN the lines are exactly the set `gafira-doc`, `gafira-geom`, `gafira-color` (order not significant: `cargo tree` lists `gafira-color` before `gafira-geom`), direct dependencies are `gafira-geom`, `gafira-color`, `serde`, `serde_json`, and V5 exits 0

#### Scenario: Shrinking the doc row fails (D-N1, manual negative test)
- GIVEN the row reduced to `gafira-doc gafira-geom` (without `gafira-color`)
- WHEN V5 runs
- THEN it exits 1 with "gafira-doc must not depend on gafira-color"; this is the evidence that the row constrains the edge

#### Scenario: Forced edge to gafira-cli (D-N2, manual negative test, verified with a warning)
- GIVEN `gafira-cli = { path = "../../apps/gafira-cli" }` temporarily added to `gafira-doc`'s `[dependencies]`
- WHEN V5 runs
- THEN Cargo warns "ignoring invalid dependency ... which is missing a lib target" but `cargo tree` still lists the edge, so V5 exits 1 with "gafira-doc must not depend on gafira-cli"; reverted afterwards. D-N1 remains the primary negative for the row

#### Scenario: Edit row constrains the edge (E-N1, manual negative test)
- GIVEN the `gafira-edit` row reduced to `gafira-edit gafira-geom gafira-color`
- WHEN V5 runs
- THEN it exits 1 with "gafira-edit must not depend on gafira-doc"; reverted afterwards

#### Scenario: Missing edit row fails (E-N2, manual negative test)
- GIVEN the `gafira-edit` row deleted
- WHEN V5 runs
- THEN it exits 1 with "gafira-edit has clippy.toml but no V5 row"

#### Scenario: Dev-dependencies are not layer edges (command)
- GIVEN the final tree, where `gafira-edit` dev-depends on `trybuild`, `proptest` and `gafira-testkit`
- WHEN `cargo tree -p gafira-edit -e normal,build --target all --prefix none -f '{p}'` runs
- THEN none of them appears, and V5 exits 0

#### Scenario: Pathops row constrains the edge (C4 D1, P-N1, manual negative test)
- GIVEN the `gafira-pathops` row reduced to `gafira-pathops -`
- WHEN V5 runs
- THEN it exits 1 with "gafira-pathops must not depend on gafira-geom"; reverted afterwards

#### Scenario: Missing pathops row fails (C4 D1, P-N2, manual negative test)
- GIVEN the `gafira-pathops` row deleted
- WHEN V5 runs
- THEN it exits 1 with "gafira-pathops has clippy.toml but no V5 row"

#### Scenario: Pathops may depend on geom (C4 D1, command)
- GIVEN the final tree
- WHEN `cargo tree -p gafira-pathops -e normal,build --target all --prefix none -f '{p}'` is filtered for `gafira-*`
- THEN the lines are exactly the set `gafira-pathops`, `gafira-geom`, and V5 exits 0

#### Scenario: Pathops must not depend on doc or edit (C4 D1, P-N3, manual negative test)
- GIVEN `gafira-doc.workspace = true` temporarily added to `gafira-pathops`'s `[dependencies]`
- WHEN V5 runs
- THEN it exits 1 with "gafira-pathops must not depend on gafira-doc"; reverted afterwards

### Requirement: Formatting and per-crate Clippy policy

C3 delivery: D1. Scenarios without a delivery tag below belong to it. C4 delivery: D1 (the `gafira-pathops` copy and the scenarios tagged `C4 D1`).

`rustfmt.toml` MUST exist. Each domain crate MUST have a `clippy.toml` whose `disallowed-methods`/`disallowed-types` cover `std::fs`, `std::net`, `std::thread::spawn`, `std::time::SystemTime::now` and `std::env` (ARQ-02). Every `clippy.toml` copy (`gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit`, `gafira-pathops`; the `gafira-time` copy comes from `domain-time`) MUST additionally list `std::collections::HashMap` and `std::collections::HashSet` in `disallowed-types`, with the reasons "ARQ-10: unstable iteration order; use BTreeMap" and "ARQ-10: unstable iteration order; use BTreeSet". The copies MUST stay byte-identical (V12). The ban applies to all targets (`--all-targets`), including tests. `gafira-geom` and `gafira-color` MUST still pass clippy after the change with no code edit (neither uses the types today). The only ordering constraint is that the commit adding the `HashMap`/`HashSet` ban to the `gafira-geom` and `gafira-color` copies precedes the `gafira-doc` skeleton commit; the hardened V12 is activated later, together with the `gafira-doc` copy (see V12 above). The `gafira-edit` copy (C3 D1) is produced with `cp` from the canonical copy in the crate's skeleton commit, together with its V5 row. The `gafira-pathops` copy (C4 D1) is produced with `cp` from the canonical copy in the crate's skeleton commit, together with its V5 row. `gafira-testkit` has no `clippy.toml`: it is a support crate that will read golden files (`design.md` Q10).

#### Scenario: Disallowed I/O flagged (integration)
- GIVEN a scratch call to `std::fs::read` in `gafira-geom`
- WHEN `cargo clippy -p gafira-geom -- -D warnings` runs
- THEN it fails citing `disallowed_methods`

#### Scenario: HashMap and HashSet flagged in every domain crate (H-N1, integration)
- GIVEN a scratch `let _m: std::collections::HashMap<u8, u8> = Default::default();` and a scratch `std::collections::HashSet<u8>` binding, in turn in `gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit` and `gafira-pathops`
- WHEN `cargo clippy -p <crate> --all-targets -- -D warnings` runs
- THEN each fails citing `disallowed_types` and ARQ-10, and the run shows no "does not refer to an existing" configuration warning

#### Scenario: BTreeMap allowed (integration)
- GIVEN a scratch `std::collections::BTreeMap` use in `gafira-doc`
- WHEN clippy runs with `-D warnings`
- THEN it passes

#### Scenario: Entries present in all copies (command)
- GIVEN the six `clippy.toml` files (`gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit`, `gafira-pathops`)
- WHEN each is searched for `path = "std::collections::HashMap"` and `path = "std::collections::HashSet"`
- THEN every file contains both, and V12 exits 0

#### Scenario: Clean tree (command)
- GIVEN the final tree
- WHEN `cargo fmt --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` run
- THEN both exit 0

#### Scenario: Pathops copy is byte-identical (C4 D1, command)
- GIVEN the canonical `crates/gafira-geom/clippy.toml` and `crates/gafira-pathops/clippy.toml`
- WHEN they are compared with `diff -q`
- THEN the command exits 0
