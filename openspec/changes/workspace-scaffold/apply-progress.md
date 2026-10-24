# Apply Progress: workspace-scaffold (batch 1, Slice A)

Mode: Strict TDD. Delivery: single PR, `size:exception` (exception-ok). Branch: `feat/workspace-scaffold`.
Batch scope: A1-A4.1. Pending: A4.2 (push/PR, orchestrator), P0.5/P0.6 (holder), Slice B (batch 2).

## Task status
- P0.1-P0.4 done before this batch (P0.4 as the single branch `feat/workspace-scaffold`).
- P0.5 PENDING (HOLDER): enable "Require actions to be pinned to a full-length commit SHA"; V7a (`sha_pinning_required: true`) cannot be verified until then. Needed before the first CI run counts as D16(a) evidence.
- P0.6 PENDING (HOLDER, optional).
- A1.1-A1.3, A2.1-A2.3, A3.1-A3.3, A3.5, A4.1 done. A3.4 is the PR checklist, left unticked for the PR (items 1-4 verified locally, see below). A4.2 left to the orchestrator.

## Commits
| Hash | Message |
|---|---|
| 3d30eb7 | build: add cargo workspace and toolchain policy |
| 81f2a34 | feat(cli): add gafira-cli with --version and --help |
| 549dadc | ci: add cargo-deny policy and three-target CI workflow |
| 9a8feba | build: add Cargo.lock |

## TDD Cycle Evidence
| Task | Test File | Layer | Safety Net | RED | GREEN | TRIANGULATE | REFACTOR |
|---|---|---|---|---|---|---|---|
| A1 | (cargo metadata/test) | Command | N/A (new) | `cargo metadata --no-deps` and `cargo test --workspace`: "could not find `Cargo.toml`" | `cargo metadata` ok, `rustc` 1.93.1 | Skipped: purely structural config files | None needed |
| A2 | `apps/gafira-cli/tests/cli.rs`, `src/main.rs` unit tests | Integration (6) + Unit (5) | N/A (new) | `cargo test -p gafira-cli`: 10 errors E0425/E0433 (`parse`, `Action` undeclared); V11 loop: "MISSING forbid: apps/gafira-cli/src/main.rs" | 5 unit + 6 integration pass | Version, help, no args, unknown, extra args (distinct paths) | `parse` kept pure; scratch `unsafe {}` -> `cargo check` exit 101 "usage of an `unsafe` block", reverted; clippy clean |
| A3 | `deny.toml` (V8) | Command | N/A (new) | scratch git dep: `cargo deny check sources` -> `source-not-allowed`, "sources FAILED", exit 8; reverted | clean tree: "bans ok, licenses ok, sources ok" | Skipped: config only | None needed |

Pins (A3.2/A3.3): the draft with `PIN-SHA` was resolved in the same step, so the "draft cannot run" observation was not recorded separately. `rg -n PIN-SHA .github` finds nothing.

## Work Unit Evidence
| Unit | Focused test | Runtime harness | Rollback boundary |
|---|---|---|---|
| A (all) | `cargo test -p gafira-cli`: 5 + 6 passed | `cargo run -p gafira-cli -- --version` -> `gafira-cli 0.1.0`; `cargo deny check` -> advisories ok, bans ok, licenses ok, sources ok | Remove all Slice A paths (Cargo.toml, Cargo.lock, rust-toolchain.toml, rustfmt.toml, deny.toml, .github/, apps/) |

## Gates (final tree)
- `cargo fmt --check`: exit 0
- `cargo clippy --workspace --all-targets -- -D warnings` (also with `--locked`): clean
- `cargo test --workspace --locked`: 11 passed, 0 failed
- `cargo check --workspace --all-targets`: ok
- `cargo deny check`: all four ok (advisories worked, network available)
- V3: only member `gafira-cli`: edition 2024, rust_version 1.93, publish []. V4: rustc 1.93.1. V6: no gafira-ui/eframe/winit/wgpu. V11: pass.
- A3.4 checklist verified locally: matrix is exactly the three triples (YAML parse), fmt/clippy/test/deny are real steps (clippy/test use `--locked`), every `uses` is `owner/repo@<40-hex> # <tag>`, `permissions: contents: read` present. `actionlint` not installed (optional).
- Pins: `actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0`; `EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25 # v2.1.1` (peeled tag).

## Size (V10)
`git diff --stat 60de00a...HEAD -- . ':!Cargo.lock'` (excluding openspec artifacts, which are not in that range): 371 added, 0 deleted, 8 files. Estimate was ~282; difference is comments and blank lines in deny.toml (66 vs 45), main.rs (104 vs 50 incl. unit tests), cli.rs (72 vs 55), ci.yml (76 vs 72). Within the 800 budget. Running total with Slice B estimate: ~371 + 629 = ~1000 (size:exception).

## DEVIATIONS
1. `members = ["crates/*", "apps/*"]` does not work without the directories: cargo fails with "failed to load manifest for workspace member `crates/*`" (glob matching nothing is an error when the parent dir is absent). Fallback from the design applied: Slice A uses `members = ["apps/gafira-cli"]` (A1 commit had `members = []`, switched in A2 when the crate was created). Slice B must switch to `["crates/*", "apps/*"]` once `crates/` exists.
2. A1 verification: `cargo fmt --check` and `cargo test --workspace` cannot succeed with zero members (fmt: "Failed to find targets"; test: "workspace has no members"). They were verified at A2 instead; A1 verified with `cargo metadata` and `rustc --version`.
3. `rust-toolchain.toml` is read by rustup at the repo root; the 1.93.1 toolchain was already installable (rustup synced components on first run).
4. Pin tags: `actions/checkout` v4.4.0 (latest v4.x; v6/v7 exist but design says v4.x); deny action v2.1.1.
5. `deny.toml` comments and `[advisories]` keys accepted by cargo-deny 0.20.2 as designed; no key changes were needed.
6. Cosmetic: `Action::Usage(Option<OsString>)` models "no args" and "unexpected arg" in one variant.

## Issues / notes
- Remaining pending: A4.2 (push, PR, first CI run: three `test` jobs + `policy`, V7a/V7b/V9), holder P0.5, A3.4 PR checklist.
- Hygiene incident (no impact): two scratch commands hung (`python3 -` waiting on stdin; `rg` with no path); no files were harmed, the main.rs rewrite was redone with the Write tool.
