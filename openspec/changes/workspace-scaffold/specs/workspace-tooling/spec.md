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

Layer purity MUST be proven in this change by `cargo tree` only; the `xtask` crate is deferred to a later change.

#### Scenario: Geom is a leaf (command)
- GIVEN the workspace
- WHEN `cargo tree -p gafira-geom` runs
- THEN no `gafira-*` crate other than itself, and no I/O, UI or engine crate, appears

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
