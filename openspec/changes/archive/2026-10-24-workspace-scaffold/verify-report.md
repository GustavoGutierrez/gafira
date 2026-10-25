# Verify report: workspace-scaffold (C1)

Pre-archive verification of a change merged before the lite flow existed (PR #1, merge `2e3d456`). Checked on current `origin/main` at `aa4280c` in a detached worktree; no source was modified. Toolchain rustc 1.93.1, cargo-deny 0.20.2.

**Verdict: PASS.** CRITICAL 0, WARNING 5, SUGGESTION 2. Both slices (A and B) are on `main`; no delivery remains. `next` = archive (first in the order C1 -> domain-color -> domain-doc). Archive must first request the whole-crate mutation run (`gafira-geom`, `gafira-cli`); it was not run here, per instruction.

## 1. Gates (on `aa4280c`)

| Gate | Result |
|---|---|
| `cargo fmt --check` | rc 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | rc 0 |
| `cargo test --workspace --locked` | rc 0 (cli 5+6, geom 31+7+11, plus color/doc suites; 0 failed) |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| ARQ-07 / V6 step (`ci.yml:60-66`) | rc 0 |
| V11 forbid step (`ci.yml:67-74`) | rc 0 |
| V5 layer table (`ci.yml:75-101`) | rc 0; xtrace shows `cargo tree -p gafira-geom` ran |
| V12 (`ci.yml:102-118`) | rc 0 |
| V13 (`ci.yml:119-144`, not owned by C1) | rc 0 |
| `gafira-cli --version` | `gafira-cli 0.1.0`, rc 0 |
| `rustc --version` (V4) | 1.93.1 |
| GitHub CI on `aa4280c` (run 40150102530) | test windows/linux/macos-aarch64 and policy: success |
| `gh api .../actions/permissions` | `sha_pinning_required: true` |

CI steps were extracted from `.github/workflows/ci.yml` with a YAML parser and run under `bash --noprofile --norc -eo pipefail`.

## 2. Requirements and scenarios

### cli-entrypoint

| Scenario | Evidence | Status |
|---|---|---|
| Version printed | `apps/gafira-cli/tests/cli.rs:17` `version_prints_workspace_version`; `src/main.rs:10` uses `CARGO_PKG_VERSION` | covered |
| Version tracks workspace | `tests/cli.rs:28` `version_tracks_workspace_manifest` (parses root `Cargo.toml`) | covered |
| Unknown flag | `tests/cli.rs:51` | covered |
| No arguments | `tests/cli.rs:59` | covered |
| Help | `tests/cli.rs:40` | covered |
| No `validate` in C1 | `src/main.rs:24-30` parses only `--help`/`--version` | covered |
| Dependency tree (no UI) | `ci.yml:60-66` rc 0 | covered |

### geometry-primitives

| Scenario | Evidence | Status |
|---|---|---|
| Dependency purity | V5 row `gafira-geom -` (`ci.yml:80`), rc 0; `crates/gafira-geom/Cargo.toml:10-11` kurbo+serde; `src/lib.rs:5` forbid | covered |
| Re-exports usable | `tests/api.rs:6`, `:15`, `:26` | covered |
| Unit set | `src/length.rs:136` | covered |
| Unit retained | `src/length.rs:152` | covered |
| Reference conversions | `src/length.rs:162`, `:179` | covered |
| Invalid DPI | `src/length.rs:204`, `:216` | covered |
| Non-finite length | `src/length.rs:227`, `:245` | covered |
| Same-unit conversion | `src/length.rs:262` (bit-exact) | covered |
| Round trip (property) | `tests/properties.rs:47` (1e-9 rel + 1e-12 abs) | covered |
| Identity and composition | `src/transform.rs:132`, `:141`, `:147` | covered |
| Inverse (property) | `tests/properties.rs:67` | covered |
| Singular inverse | `src/transform.rs:190`, `:199` | covered |
| Affine interop | `src/transform.rs:216`, `:225` | covered |
| Serde round trip (property) | `tests/properties.rs:79-129`, guard `:136` | covered |
| Wire names | `tests/api.rs:42`, `:52` | covered |

### workspace-tooling (C1 base spec)

| Scenario | Evidence | Status |
|---|---|---|
| Members inherit policy (`spec.md:13`) | `cargo metadata --no-deps`: all four members edition 2024, rust_version 1.93, publish `[]`; every member has `[lints] workspace = true` | covered |
| Toolchain pin (`:18`) | `rust-toolchain.toml` 1.93.1 + rustfmt, clippy; rustc 1.93.1 | covered |
| Unsafe rejected (`:27`) | `Cargo.toml:21-22` `unsafe_code = "deny"`, `crates/gafira-geom/src/lib.rs:5` `#![forbid(unsafe_code)]`; scratch run recorded in apply-progress (B6). Not re-run here (see W5) | covered (prior evidence) |
| Profiles (`:32`) | `Cargo.toml:29-39`: release opt 3, lto thin, panic unwind; dev.package."*" opt 2 | covered |
| Disallowed I/O flagged (`:41`) | `crates/gafira-geom/clippy.toml:7-44`; scratch `std::fs::read` re-run in the domain-doc D1 verify report (rc 101 `disallowed_methods`) | covered (prior evidence) |
| Clean tree (`:46`) | section 1 | covered |
| Offline gate passes (`:55`) | `cargo deny check` all ok; `deny.toml` allowlist, `confidence-threshold = 0.93`, `[licenses.private] ignore = true`, `allow-git = []` | covered |
| Git dependency rejected (`:60`) | apply-progress A3 (scratch git dep -> `source-not-allowed`, exit 8); `deny.toml` `[sources]`. Not re-run here | covered (prior evidence) |
| Advisories (`:65`) | `cargo deny check` advisories ok locally and in CI | covered |
| Geom is a leaf (`:74`) | V5 rc 0 | covered; superseded by domain-color MODIFIED (see 3) |
| Matrix exact (`:85`) | `ci.yml:31-37` exactly 3 triples; CI jobs on `aa4280c` are the 3 `test` jobs + `policy` | covered |
| Actions pinned (`:90`) | `sha_pinning_required: true`; `ci.yml:39,55,145` all `@<40-hex> # <tag>` | covered |
| Four gates present (`:95`) | clippy/test run per matrix job (`ci.yml:46-49`); fmt and deny run in `policy` (`ci.yml:58`, `:145`) | covered with W3 |
| Merge gating (`:100`) | Deferred: branch protection HTTP 403 on the plan (tasks P0.6) | deferred, allowed by the scenario ("MAY be deferred") |

Uncovered items: none.

Superseded on `main`: "Layer purity interim proof" (replaced by domain-color MODIFIED), "Formatting and per-crate Clippy policy" (replaced by domain-doc MODIFIED, adds the HashMap/HashSet ban). V12 is now the hardened domain-doc body. C1's tooling text is still satisfied by the current tree.

## 3. Archive merge notes (destructive deltas)

- C1 is the base: all three capabilities are new (no `openspec/specs/` exists), so archive is `cp` + `diff`. No REMOVED or RENAMED blocks anywhere in C1, domain-color or domain-doc.
- Order is mandatory: C1, then domain-color, then domain-doc (notes at `domain-color/specs/workspace-tooling/spec.md:3` and `domain-doc/specs/workspace-tooling/spec.md:3-7`; `AGENTS.md:84`).
- Later deltas that MODIFY C1 blocks and drop scenarios (warn at archive time):
  - domain-color MODIFIED "Layer purity interim proof" (`domain-color/specs/workspace-tooling/spec.md:88-96`) drops C1 scenario "Geom is a leaf" (`workspace-scaffold/.../spec.md:74`), replaced by "Domain crates are leaves" (content subsumed).
  - domain-doc MODIFIED "Formatting and per-crate Clippy policy" keeps both C1 scenarios ("Clean tree" now with `--locked`); not destructive.

## CRITICAL

None.

## WARNING

1. Task A3.4 (D16c PR checklist) was never ticked in PR #1; items 1-4 re-checked statically here (`ci.yml:9-10,31-37,39,47,49,55,58,145`).
2. ARQ-80 merge gating not enforceable (branch protection 403, P0.6 deferred). Allowed by the scenario, still an open product item.
3. Spec wording "matrix ... runs `cargo fmt --check` ... and `cargo deny check`" and scenario "Four gates present" ("each job") do not match the design D-policy split: fmt and deny run once in `policy`, not per matrix job. Behavior follows the design; spec text should be clarified when next touched.
4. C1 proposal chain table (C2 = `domain-color-doc` with `gafira-cli validate`) is out of date: split into C2a/C2b, `validate` deferred to `gafira-format` (domain-doc design A15).
5. Scratch negatives (unsafe block, `std::fs::read`, git dependency) were not re-run in this pass (a scratch-copy run was refused by the permission system); coverage relies on apply-progress and the domain-doc D1 verify report.

## SUGGESTION

1. Follow-ups listed in apply-progress (near-zero samples in the conversion property, DPI `-0.0`/subnormal tests) are still open.
2. When archiving, drop the stale "Not a git repo" risk text only in archived copies if desired; no spec impact.

## Result

PASS. CRITICAL 0, WARNING 5, SUGGESTION 2. `next`: archive (after the whole-crate mutation run), before domain-color.
