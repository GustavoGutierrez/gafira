# Delta for Workspace Tooling

> **Base spec note.** This delta applies to requirements of the `workspace-scaffold` change (C1). C1 is merged but NOT yet archived, so there is no `openspec/specs/workspace-tooling/spec.md`. The full base spec lives at `openspec/changes/workspace-scaffold/specs/workspace-tooling/spec.md`. This delta is merged into it when the changes are archived (C1 first, then `domain-color`); the MODIFIED block below replaces the requirement of the same name in that file. Requirement blocks not named here are unchanged.

Scenario type tags: (unit), (integration), (command), (manual).

## ADDED Requirements

### Requirement: Per-crate allowed-dependency table (V5)

CI step V5 MUST be driven by one table of allowed internal dependencies per domain crate, not by a step hard-coded for `gafira-geom`. The table in this change:

| Crate | Allowed `gafira-*` dependencies |
|---|---|
| `gafira-geom` | none (`-`) |
| `gafira-color` | none (`-`) |

For each table row, V5 MUST run `cargo tree -p <crate> -e normal,build --target all --prefix none -f '{p}'` with standard input redirected from `/dev/null` (so `cargo` cannot consume the table), capture its output before any filtering, and fail when:
- `cargo tree` fails, or the captured tree has no line starting with `<crate> v` (empty tree);
- the tree contains a `gafira-*` crate other than the crate itself and its allowed list;
- the tree contains a line whose crate name STARTS WITH any of `egui`, `eframe`, `winit`, `wgpu`, `tokio`, `mio`, `socket2`, `reqwest`, `ureq`. The match is a prefix match with no trailing space (`^(egui|eframe|...)`), so derived names such as `wgpu-core`, `tokio-util` or `winit-x` are also detected. It MUST NOT be weaker than the pre-change step, which matched these names anywhere in a line.

The table MUST be iterated in the main shell, fed by a here-string, so the failure status survives the loop; the step MUST run correctly under `bash -eo pipefail` and MUST NOT pass because a `grep` found no match. V5 MUST also fail when a crate that has a `clippy.toml` under `crates/*/` (the domain-crate marker, shared with V12) has no table row. Adding a crate or an allowed edge MUST be an explicit edit of the table; the step body stays unchanged.

Known limitation: a crate under `crates/` without a `clippy.toml` is not recognized as a domain crate by V5's coverage check and is therefore not required to have a row. V11 still requires `#![forbid(unsafe_code)]` in its `src/lib.rs`, but neither the layer table nor V12 constrains it; closing this gap is deferred to `xtask check-layers` (ARQ-01).

#### Scenario: Clean tree passes (N0, command)
- GIVEN the final tree with `gafira-geom` and `gafira-color`
- WHEN V5 runs from a scratch copy under `bash --noprofile --norc -eo pipefail`
- THEN it exits 0 and both crates were examined

#### Scenario: Forbidden internal edge fails (N1, manual negative test)
- GIVEN `gafira-geom.workspace = true` temporarily added to `gafira-color`'s `[dependencies]`
- WHEN V5 runs
- THEN it exits 1 with "gafira-color must not depend on gafira-geom"; the previous geom-only V5 exits 0 on the same tree (the RED); after `git checkout -- crates/gafira-color/Cargo.toml Cargo.lock` V5 exits 0

#### Scenario: Misspelled row fails closed (N2, manual negative test)
- GIVEN the table row `gafira-colour -` instead of `gafira-color -`
- WHEN V5 runs
- THEN `cargo tree` fails ("did not match any packages"), the step exits non-zero, and the coverage check reports that `gafira-color` has no row

#### Scenario: Missing row fails (N3, manual negative test)
- GIVEN the `gafira-color` row deleted
- WHEN V5 runs
- THEN it exits 1 with "gafira-color has clippy.toml but no V5 row"

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

### Requirement: Policy checks cover every domain crate (V11, V12)

V11 (`#![forbid(unsafe_code)]` in `crates/*/src/lib.rs`) and V12 (`clippy.toml` identical to the canonical copy) MUST cover `gafira-color` through their existing `crates/*` wildcards without per-crate edits. `crates/gafira-color/clippy.toml` MUST be byte-identical to `crates/gafira-geom/clippy.toml` (the canonical copy) and MUST never be edited by hand. V12 MUST fail when any copy differs.

#### Scenario: Color copy identical (command)
- GIVEN the final tree
- WHEN `diff -q crates/gafira-geom/clippy.toml crates/gafira-color/clippy.toml` runs
- THEN it exits 0

#### Scenario: Drifted copy fails V12 (manual negative test)
- GIVEN a scratch extra line appended to `crates/gafira-color/clippy.toml`
- WHEN V12 runs
- THEN it exits non-zero; reverted afterwards

#### Scenario: Missing forbid fails V11 (manual negative test)
- GIVEN `#![forbid(unsafe_code)]` temporarily removed from `crates/gafira-color/src/lib.rs`
- WHEN V11 runs
- THEN it exits non-zero naming that file; reverted afterwards

#### Scenario: Workspace membership (command)
- GIVEN the final tree
- WHEN `cargo metadata --no-deps` is parsed
- THEN `gafira-color` is a member with edition 2024, rust-version 1.93 and `publish` empty, and the root `[workspace.dependencies]` declares it by path

## MODIFIED Requirements

### Requirement: Layer purity interim proof

Layer purity MUST be proven by `cargo tree` only, applied to every domain crate through the per-crate allowed-dependency table (V5, see the ADDED requirement above); the `xtask` crate remains deferred to a later change.
(Previously: proof by `cargo tree` for `gafira-geom` alone, with V5 hard-coded to that crate.)

#### Scenario: Domain crates are leaves (command)
- GIVEN the workspace
- WHEN `cargo tree -p gafira-geom` and `cargo tree -p gafira-color` run
- THEN in each, no `gafira-*` crate other than itself, and no I/O, UI or engine crate, appears
