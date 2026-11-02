## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: scaffold of four layer-2 crates under the owner's parallel full route; CI policy and dependency changes
Product IDs: ARQ-01, ARQ-02, ARQ-70 (rows, lints, licences), ARQ-107 (CI step) (source: specs/02-arquitectura-software.md)
Scope: `SC1` of `specs/changes/layer-2/design.md`: P1 table in full, P9, the four V5 rows of ADR 0007, one `workflow_dispatch` on the branch.      Non-goals: any production logic, fonts or `assets/LICENSES.toml` (`TX1`), any crate API.
Acceptance:
  AC1 Given the four new crates, When the V5 step runs, Then it exits 0, and a row without a crate's dependency exits 1 [command]
  AC2 Given each new `clippy.toml`, When V12 runs, Then it is byte-identical to `crates/gafira-geom/clippy.toml` [command]
  AC3 Given each new `lib.rs`, When the policy step runs, Then it has `#![forbid(unsafe_code)]` [command]
  AC4 Given the ARQ-107 step of P9, When it runs on the clean tree, Then it exits 0, and with `icu_plurals` added to a layer 0-2 crate it exits 1 [command]
  AC5 Given `deny.toml` per P1, When `cargo deny check` runs, Then it exits 0 [command]
  AC6 Given the branch, When CI is dispatched, Then all jobs pass, new dependencies compiling on Windows and macOS [command]
Plan: design P1, P2 and P9, in the commits ADR 0007 requires: row, `clippy.toml` and `Cargo.toml` entries together.
Owner decisions touched: none (checked D-20: untouched; the segment crate is scaffolded only).

# Apply progress: layer-2-scaffold
