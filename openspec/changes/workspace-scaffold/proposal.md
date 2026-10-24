# Proposal: Workspace Scaffold (C1 of `workspace-foundations`)

## Intent

Gafira has specs (`docs/specs/00`–`05`) but no code. Every later crate needs a shared, policy-enforcing workspace: edition, MSRV, lints, license gate, CI on the three targets (ARQ-03, ARQ-70, ARQ-71, ARQ-80). This change lays that foundation plus the first domain crate (`gafira-geom`) and a minimal `gafira-cli`, so C2/C3 only add crates.

## Change Chain

| Order | Change | Content |
|---|---|---|
| C1 | `workspace-scaffold` (this) | Workspace, tooling, CI, `gafira-geom`, minimal `gafira-cli` (~800 lines) |
| C2 | `domain-color-doc` | `gafira-color`, `gafira-doc`, `gafira-cli validate` |
| C3 | `edit-history` | `gafira-edit`, property tests |

C1 MUST NOT pre-decide C2/C3 APIs; it only reserves workspace slots and conventions (ids `u64`, geometry `f64`, color `f32`, `serde_json` with `float_roundtrip`) as recorded guidance for C2.

## Scope

### In Scope
- Root `Cargo.toml`: resolver 3, edition 2024, `[workspace.package]`, `[workspace.dependencies]` (internal crates by path), `[workspace.lints]` (`unsafe_code = "deny"`), dev/release profiles (02 §10.2).
- `rust-toolchain.toml`, `rustfmt.toml`, per-domain-crate `clippy.toml` (ARQ-02 disallowed I/O), `deny.toml` (02 §9.1, `allow-git = []`, `[licenses.private] ignore = true`).
- `.github/workflows/ci.yml`: own workflow, actions pinned by SHA, matrix of exactly the three ARQ-03 targets; fmt, clippy, test, deny.
- `gafira-geom`: re-export `kurbo` (`serde`) `Point`, `Vec2`, `Affine`, `Rect`; own `Length`, `Unit` (px, mm, cm, in, pt) with DPI conversion, `Size`, `Transform2D`; `f64` public API; `#![forbid(unsafe_code)]`.
- `apps/gafira-cli`: `--version` only.

### Out of Scope
- Bézier paths, rasterization, `gafira-color`/`-doc`/`-edit` (C2/C3).
- `xtask check-layers`, `gafira-testkit`, nextest, llvm-cov coverage gate, packaging, `profile.dist` size jobs.
- Any change to `docs/`.

## Capabilities

### New Capabilities
- `workspace-tooling`: workspace policy, lint/format/license gates, CI matrix.
- `geometry-primitives`: `gafira-geom` types, units and DPI conversion.
- `cli-entrypoint`: `gafira-cli` binary and `--version`.

### Modified Capabilities
None.

## Approach

Strict TDD per crate (unit-conversion tests first). Layer purity enforced now by `clippy.toml` plus a `cargo tree` check; `xtask` comes later.

**PROPOSAL deviations from 02 (design must confirm):**
- `rust-version = "1.93"`, toolchain `1.93.1` (02 §2.1 says MSRV 1.95; installed is 1.93.1).
- `unsafe_code` = `deny` at workspace, `forbid` per domain crate (FFI crates need override later).
- Workspace license `LicenseRef-Gafira-Proprietary`, `publish = false`.
- License allowlist may need adjustment for transitive deps (Unicode-3.0 already listed).

## Affected Areas

| Area | Impact |
|---|---|
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `rustfmt.toml`, `deny.toml` | New |
| `crates/gafira-geom/` (+ `clippy.toml`) | New |
| `apps/gafira-cli/` | New |
| `.github/workflows/ci.yml` | New |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| `cargo-deny` not installed; advisories need network | High | Install locally in apply; run `licenses bans sources` offline, `advisories` in CI |
| Not a git repo, no remote: CI unrunnable, 800-line budget unmeasurable | High | Validate YAML structurally; count lines from file sizes; init git before apply |
| MSRV mismatch with 02 | Med | Record deviation; bump later is one line |
| Transitive license outside allowlist | Low | Inspect `cargo deny` output; amend only with rationale |

## Rollback Plan

All files are new: delete the created paths. No existing behavior or data is affected.

## Dependencies

- `kurbo` (MIT/Apache-2.0, VERIFIED in 02 §9.2), `serde`; `clap` for CLI if design chooses it (MIT/Apache-2.0).
- Local `cargo-deny` install.

## Success Criteria

- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo deny check` passes (advisories may be CI-only).
- [ ] `cargo tree -p gafira-geom` contains no `gafira-*` crate and no I/O, UI or engine crate (ARQ-01, ARQ-02).
- [ ] `cargo tree -p gafira-cli` contains no `eframe`, `winit`, `wgpu` (ARQ-07).
- [ ] CI matrix lists exactly the three ARQ-03 targets; all actions pinned by SHA.
- [ ] `gafira-cli --version` prints the workspace version.
- [ ] Unit conversions round-trip at a given DPI (e.g. 25.4 mm = 1 in = 96 px at 96 DPI).

## Proposal question round

Not asked interactively (executor cannot prompt). Assumptions needing user review:

1. Is accepting MSRV 1.93 (vs. 02's 1.95) acceptable, or should the toolchain be upgraded first?
2. Should `gafira-geom` expose `Length` with an explicit unit always, or normalize to px internally with DPI supplied per conversion?
3. Is a GitHub remote planned, so CI is a real merge gate (ARQ-80), or is CI only a prepared file for now?
4. Should `xtask check-layers` (ARQ-01) stay deferred, with `cargo tree` as the interim proof?
