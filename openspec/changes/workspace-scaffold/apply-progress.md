# Apply Progress: workspace-scaffold (batch 1 Slice A, batch 2 Slice B)

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

---

# Batch 2: Slice B (`gafira-geom`)

Mode: Strict TDD. Delivery: single PR, `size:exception`. Branch `feat/workspace-scaffold`. No push, no PR (orchestrator).
Done: B1.1, B2.1-B2.2, B3.1-B3.2, B4.1, B5.1, B6.1-B6.2. B7.1: all gates and size measured (below); push, PR and the first CI run with V5/V12 left to the orchestrator, so the box stays unticked.

## Commits (batch 2)
| Hash | Message |
|---|---|
| 5e81fd0 | build(geom): add gafira-geom crate skeleton and workspace dependencies |
| 7358fb7 | feat(geom): add Length and Unit with DPI conversion |
| 4486b15 | feat(geom): add Size and Transform2D |
| 1493ed9 | feat(geom): re-export kurbo primitives and fix serde wire names |
| 8fafed0 | test(geom): add property tests for conversions and serde round trips |
| e767a74 | ci: enforce geom layer purity and clippy policy sync (also holds `crates/gafira-geom/clippy.toml`) |

## TDD Cycle Evidence (batch 2)
| Task | Test file | Layer | Safety net | RED | GREEN | TRIANGULATE | REFACTOR |
|---|---|---|---|---|---|---|---|
| B1 | (cargo check/tree) | Command | 11 cli tests pass | Structural; no RED (skeleton, no logic) | `cargo check --workspace --all-targets` ok; `cargo tree -p gafira-geom -e dev`: only proptest, serde_json, no rusty-fork/tempfile | Skipped: config only | None needed |
| B2 | `src/length.rs` (15 unit) | Unit | N/A (new) | `cargo test -p gafira-geom`: 80 errors E0425/E0433 (`Length`, `Unit`, `ConversionError` undeclared) | 15 passed | Many inputs: 5 DPI kinds, 3 non-finite kinds, same-unit over 6 values x 5 units incl. -0.0 and 5e-324, overflow, error precedence | Re-exports added to lib.rs (removed dead-code warnings); fmt + clippy clean |
| B3 | `src/size.rs` (4), `src/transform.rs` (12) | Unit | 15/15 | `cargo test -p gafira-geom`: ~66 errors E0425/E0433 (`Size`, `Transform2D`, `SingularTransform` undeclared) | 31 passed | both composition orders, 4 singular/non-finite inverse cases, bit round trips of Affine and `[f64;6]`, wire shape | clippy clean |
| B4 | `tests/api.rs` (7) | Integration | 31/31 | `cargo test --test api`: E0432 unresolved imports Affine/Point/Rect/Vec2 | 7 passed | kurbo parity via `to_bits()`, `"in"` ok, `"km"`/`"IN"` rejected | clean |
| B5 | `tests/properties.rs` (11) | Property | 38/38 | Mutation 1: truncating the conversion result to f32: `conversion_round_trip_stays_within_tolerance` FAILED (`-437254935.44 Mm -> Px @ 1: back -437254944`). Mutation 2: perturbing `From<[f64;6]>`: `transform2d_json_round_trip_is_bit_identical` FAILED. Both reverted. A consistent wrong factor (cm = 2.5) does NOT fail the round trip (A->B->A cancels it); the fixed-reference unit tests cover factors. | 11 passed | Generator guard test proves -0.0 and subnormals are produced | Removed the `proptest-regressions` file created by the mutation run (artifact of deliberately broken code; no real regression) |
| B6 | clippy / cargo check (V11, V12, V5) | Command | 49/49 | Scratch `std::fs::read` passed `cargo clippy -p gafira-geom -- -D warnings` (no config); scratch `unsafe {}`: `cargo check` exit 101 "usage of an `unsafe` block" | With `clippy.toml`: scratch fails "use of a disallowed method `std::fs::read`" note ARQ-02; clean run emits 0 config warnings (all 34 paths resolve in 1.93) | Skipped: config only | Scratch reverted (git status clean) |

Tests: 31 unit + 7 api + 11 property = 49 in gafira-geom (60 in workspace with the 11 cli tests).

## Work Unit Evidence (Slice B)
| Unit | Focused test | Runtime harness | Rollback boundary |
|---|---|---|---|
| B (all) | `cargo test -p gafira-geom`: 31 + 7 + 11 passed, 0 failed | V5 (`cargo tree ... | grep`) no hits; V11 pass; V12 pass (single copy); scratch fs::read rejected by clippy; scratch `unsafe` rejected by cargo check | Remove `crates/gafira-geom/`; revert root `Cargo.toml` (members glob + 5 dep lines), the +13 lines in `.github/workflows/ci.yml`, and the Cargo.lock entries |

## Gates (final tree, HEAD e767a74)
- `cargo fmt --check`: exit 0
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean
- `cargo test --workspace --locked`: 60 passed, 0 failed
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok
- V3: `cargo metadata --no-deps` lists `gafira-cli` and `gafira-geom`, edition 2024, rust_version 1.93, publish [] on both. V4: rustc 1.93.1. V5: no hits. V6: no hits. V11: pass. V12: pass. ci.yml: matrix exactly the 3 triples, permissions contents: read, every `uses` is `@<40-hex> # <tag>`, no PIN-SHA (checklist D16c items 1-4 re-verified locally).
- Pending (needs push): first CI run showing V5/V12 steps green on GitHub; V7a (holder P0.5).

## Size (V10)
- Slice B: `git diff --shortstat 2f31e7e...HEAD -- . ':!Cargo.lock'`: 938 added, 1 deleted (10 files). Estimate ~629. Of this, 856 is Rust in `crates/gafira-geom/src` and `tests`. The overshoot comes from the tests (length.rs 313 vs 190, transform.rs 247 vs 150, properties.rs 151 vs 120, size.rs 63 vs 35, api.rs 68 vs 40), doc comments, and 13 CI lines vs 8.
- Whole change: `git diff --shortstat main...HEAD -- . ':!Cargo.lock' ':!openspec'`: 1308 added, 0 deleted (A 371 + B 938). Estimate ~911. Covered by the `size:exception` decision.

## DEVIATIONS (batch 2)
1. Verified assumptions: kurbo 0.13.1 exists, has `serde` feature, `rust-version = "1.85"` (<= 1.93). `proptest` 1.12.0 with `default-features = false, features = ["std"]` compiles and runs; no `rusty-fork`/`tempfile`. `Affine::as_coeffs` is a const fn returning `[f64;6]`; there is no `Affine::from_coeffs`, the constructor is `Affine::new([f64;6])` (as the design's contract comment says). kurbo pulls `arrayvec`, `smallvec`, `polycool` (extra transitive crate not named in the design diagram); all pass cargo-deny and V5.
2. Root `Cargo.toml` back to `members = ["crates/*", "apps/*"]` (resolves batch 1 deviation 1); `cargo metadata` lists both members.
3. B1 got its own commit (`build(geom): ...`); tasks.md names no commit for B1. Cargo.lock was committed inside it (no separate Cargo.lock commit needed; it was not modified afterwards).
4. B6.1 `clippy.toml` and B6.2 CI edit are in one commit (e767a74) because the task lists a single commit message for B6.2 and the clippy file is untestable alone from the CI side.
5. CI additions are +13 lines (design +8): multi-line V5 command and an error message.
6. Tests added beyond the listed ones: `Display` of errors, `to_px`, error precedence (invalid DPI before non-finite value), overflow to `NonFiniteResult`, non-finite and overflow cases for `inverse`, generator guard test for -0.0 and subnormals.
7. Design hardening noted, not changed: the V5/ARQ-07 shell steps run under `bash -eo pipefail`; a failing `cargo tree` inside `if cmd | grep` would pass silently. Same pattern as the batch-1 step. Left as is until `xtask check-layers`.
8. `assets/splash_screen.jpg` is an untracked file that existed before this batch; not touched, not committed.

## Remaining
- B7.1 tail: push, open the single PR (`size:exception`), first CI run (three `test` jobs + `policy` with V5 and V12), A3.4/D16c checklist ticked in the PR body, A4.2.
- Holder: P0.5 (`sha_pinning_required: true`, V7a), P0.6 optional.
