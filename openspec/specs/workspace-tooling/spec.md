# Workspace Tooling Specification

## Purpose

Shared workspace policy (edition, MSRV, lints, profiles), license/source gate and CI for all Gafira crates (ARQ-02, ARQ-03, ARQ-70, ARQ-71, ARQ-80). New capability; no existing spec.

## Requirements

### Requirement: Workspace manifest

The root manifest MUST declare resolver 3, edition 2024, `rust-version = "1.93"` (deviation from 02 §2.1's 1.95, accepted pending review), `publish = false`, license `LicenseRef-Gafira-Proprietary`, and internal crates under `[workspace.dependencies]` by path. Every member MUST inherit package fields and lints from the workspace. `rust-toolchain.toml` MUST pin `1.93.1` with `rustfmt` and `clippy` components.

#### Scenario: Members inherit policy (command)
- GIVEN the workspace with `gafira-geom` and `gafira-cli`
- WHEN `cargo metadata --no-deps` is parsed
- THEN both members report edition 2024, rust-version 1.93, and `publish` empty (`[]`)

#### Scenario: Toolchain pin (command)
- GIVEN `rust-toolchain.toml`
- WHEN `rustc --version` runs in the repo root
- THEN it reports 1.93.1

### Requirement: Lints and profiles

`[workspace.lints]` MUST set `unsafe_code = "deny"`; each domain crate MUST additionally carry `#![forbid(unsafe_code)]`. `[profile.release]` and `[profile.dev.package."*"]` MUST match 02 §10.2 (`profile.dist` is out of scope).

#### Scenario: Unsafe rejected (integration)
- GIVEN a temporary `unsafe` block added to `gafira-geom`
- WHEN `cargo check -p gafira-geom` runs
- THEN compilation fails with an unsafe-code error

#### Scenario: Profiles (command)
- GIVEN the root manifest
- WHEN its profile tables are inspected
- THEN release has `opt-level = 3`, `lto = "thin"`, `panic = "unwind"`, and dev dependencies use `opt-level = 2`

### Requirement: Formatting and per-crate Clippy policy

`rustfmt.toml` MUST exist. Each domain crate MUST have a `clippy.toml` whose `disallowed-methods`/`disallowed-types` cover `std::fs`, `std::net`, `std::thread::spawn`, `std::time::SystemTime::now` and `std::env` (ARQ-02).

#### Scenario: Disallowed I/O flagged (integration)
- GIVEN a scratch call to `std::fs::read` in `gafira-geom`
- WHEN `cargo clippy -p gafira-geom -- -D warnings` runs
- THEN it fails citing `disallowed_methods`

#### Scenario: Clean tree (command)
- GIVEN the final tree
- WHEN `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` run
- THEN both exit 0

### Requirement: Dependency and license gate

`deny.toml` MUST allow only: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016, BSL-1.0, CC0-1.0, MIT-0, 0BSD; set `confidence-threshold = 0.93`; ban the 02 §9.1 crates; set `unknown-registry = "deny"`, `unknown-git = "deny"`, `allow-git = []`. The private proprietary workspace license MUST be exempted via `[licenses.private] ignore = true` and MUST NOT be added to `allow`. MPL-2.0 and copyleft MUST NOT be allowed. Allowlist amendments for transitive dependencies MUST be justified in writing.

#### Scenario: Offline gate passes (command)
- GIVEN `cargo-deny` installed
- WHEN `cargo deny check licenses bans sources` runs
- THEN it exits 0 and the private workspace crates raise no license error

#### Scenario: Git dependency rejected (command)
- GIVEN a scratch git dependency
- WHEN `cargo deny check sources` runs
- THEN it fails

#### Scenario: Advisories (command)
- GIVEN network access (CI)
- WHEN `cargo deny check` runs
- THEN it exits 0; locally, `advisories` MAY be skipped

### Requirement: Layer purity interim proof

Layer purity MUST be proven by `cargo tree` only, applied to every domain crate through the per-crate allowed-dependency table (V5, see "Per-crate allowed-dependency table (V5)"); the `xtask` crate remains deferred to a later change.

#### Scenario: Domain crates are leaves (command)
- GIVEN the workspace
- WHEN `cargo tree -p gafira-geom` and `cargo tree -p gafira-color` run
- THEN in each, no `gafira-*` crate other than itself, and no I/O, UI or engine crate, appears

### Requirement: CI workflow

`.github/workflows/ci.yml` MUST be a standalone workflow whose matrix contains exactly `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `cargo deny check`. Every third-party `uses:` MUST reference a full 40-hex commit SHA. The repository has git with a private `origin` remote, so the workflow is exercised by GitHub itself. Verification in this change is manual or by CI run; no programmatic check of the file exists.

Note: programmatic checks of the CI file and layer rules arrive with the `xtask` crate in a later change (ARQ-01); they are not part of C1.

#### Scenario: Matrix exact (manual / CI run)
- GIVEN the first real CI run on GitHub
- WHEN the run's jobs are listed
- THEN exactly Windows x86_64, Linux x86_64 and macOS aarch64 ran, and the PR review checklist confirms no other target in the matrix

#### Scenario: Actions pinned (manual / CI run)
- GIVEN the repository setting "Require actions to be pinned to a full-length commit SHA"
- WHEN `gh api repos/GustavoGutierrez/gafira/actions/permissions` is queried
- THEN `sha_pinning_required` is `true`, and a workflow using a tag or branch ref is rejected by GitHub

#### Scenario: Four gates present (manual / CI run)
- GIVEN the first CI run on the three systems
- WHEN each job's steps are inspected, with the PR review checklist as cross-check
- THEN fmt, clippy (`-D warnings`), test and deny each executed and passed

#### Scenario: Merge gating (manual)
- GIVEN branch protection on the remote
- WHEN the PR checklist is reviewed
- THEN a green CI on all three systems is recorded as required (ARQ-80); enabling it MAY be deferred and is not an automated test

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
