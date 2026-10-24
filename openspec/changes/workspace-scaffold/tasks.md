# Tasks: Workspace Scaffold (C1 of `workspace-foundations`)

Conventions: Strict TDD (RED -> GREEN -> REFACTOR). A RED step is never committed alone; the commit is made after GREEN and holds tests plus code (work-unit-commits). Commits are conventional, in English, with no `Co-Authored-By` and no AI attribution. Test IDs (T1-T8) and checks (V1-V12) refer to `design.md` (decisions D1-D19). Lines are authored lines; `Cargo.lock` is excluded. C1 has no `xtask`: CI verification is D16 (GitHub SHA policy, first real run, PR checklist, optional `actionlint`).

## Phase 0: Holder / environment prerequisites (no code, no lines)

- [x] P0.1 Install cargo-deny: `cargo install cargo-deny --locked`. Done when `cargo deny --version` prints a version.
- [x] P0.2 Confirm network to crates.io (`cargo search kurbo --limit 1`) and GitHub (`git ls-remote https://github.com/actions/checkout HEAD`). Done when both succeed; otherwise stop (kurbo, serde, proptest, PIN-SHA need it).
- [x] P0.3 First commit on `main` with the existing `.gitignore` only (`chore: initial commit`), then `git push -u origin main` (remote is private and empty). Done when `git ls-remote origin main` shows the commit.
- [x] P0.4 Create branch `feat/workspace-scaffold-a` from `main`. (Done as the single branch `feat/workspace-scaffold`, per the one-PR size:exception decision.)
- [ ] P0.5 HOLDER: enable "Require actions to be pinned to a full-length commit SHA" on `GustavoGutierrez/gafira` before the first push of slice A. Done when `gh api repos/GustavoGutierrez/gafira/actions/permissions` reports `"sha_pinning_required": true` (V7a; today it is `false`).
- [ ] P0.6 HOLDER, optional: add a branch-protection rule on `main` requiring the CI jobs (ARQ-80). It can only select the checks after the first CI run exists. May be deferred.

## Slice A: tooling, CLI, CI (~282 lines)

### A1: Workspace and toolchain (T1) ~48 lines

- [x] A1.1 RED: with no manifest, `cargo metadata --no-deps` and `cargo test --workspace` fail. Verifies V1/V2/V3. Done when both fail ("could not find Cargo.toml"). 0 lines.
- [x] A1.2 GREEN: create `Cargo.toml` (40: resolver 3, `members = ["crates/*", "apps/*"]`, `[workspace.package]`, empty `[workspace.dependencies]`, lints, profiles), `rust-toolchain.toml` (4, 1.93.1, no `targets`), `rustfmt.toml` (4). No `xtask`, no alias. Done when `cargo metadata --no-deps` succeeds with no `crates/` directory (if not, list `apps/gafira-cli` explicitly in A and switch to globs in B), `rustc --version` reports 1.93.1 (V4), and `cargo fmt --check` passes.
- [x] A1.3 Commit: `build: add cargo workspace and toolchain policy`.

### A2: `gafira-cli` (T2, cli part of T7) ~117 lines

- [x] A2.1 RED: create `apps/gafira-cli/Cargo.toml` (12, inherited fields, `[[bin]] name = "gafira-cli"`, `[lints] workspace = true`), `apps/gafira-cli/tests/cli.rs` (55: `--version`, `--help`, no args, unknown arg, extra args; spec `cli-entrypoint`), and `parse` unit tests in `src/main.rs`. Before adding `#![forbid(unsafe_code)]`, run the V11 loop and see it fail. Done when `cargo test -p gafira-cli` fails.
- [x] A2.2 GREEN: implement `apps/gafira-cli/src/main.rs` (50: `#![forbid(unsafe_code)]`, `args_os` parser, `Action`, exit codes 0/2/1). Done when `cargo test -p gafira-cli` passes, V11 passes, and V6 (`cargo tree -p gafira-cli --no-default-features -e normal,build --target all` has no `gafira-ui|eframe|winit|wgpu`) is clean.
- [x] A2.3 REFACTOR + scratch check: keep `parse` pure; add a temporary `unsafe {}` and confirm `cargo check -p gafira-cli` fails, then revert it. Done when `cargo clippy --workspace --all-targets -- -D warnings` passes. Commit: `feat(cli): add gafira-cli with --version and --help`.

### A3: Deny policy and CI workflow (T8) ~117 lines

- [x] A3.1 RED: write `deny.toml` (45; check keys against the installed cargo-deny version). Add a scratch git dependency and confirm `cargo deny check sources` fails; revert it. Done when `cargo deny check licenses bans sources` passes on the clean tree (V8).
- [x] A3.2 Draft `.github/workflows/ci.yml` (72: triggers, `permissions: contents: read`, `test` matrix with the three ARQ-03 triples, host check, clippy and test with `--locked`; `policy` job with fmt, V6, V11, cargo-deny action `command: check`) with `PIN-SHA` markers. Under policy P0.5 this draft cannot run, which is the observable failure.
- [x] A3.3 GREEN: resolve each pin with `git ls-remote https://github.com/<owner>/<repo> 'refs/tags/<tag>^{}'` (fallback: unpeeled ref) for `actions/checkout` (v4.x) and `EmbarkStudios/cargo-deny-action` (v2.x); write `uses: owner/repo@<40-hex> # <tag>`. Done when `rg PIN-SHA` finds nothing.
- [ ] A3.4 Manual PR checklist (D16c), ticked in the PR: (1) matrix exactly the three triples, no extras or duplicates; (2) fmt, clippy, test and deny are real steps, not comments, one extra `--locked` accepted; (3) every `uses` is `owner/repo@<40-hex> # <tag>`; (4) `permissions: contents: read` present. Optional: `actionlint` run by hand (not project code).
- [x] A3.5 Commit: `ci: add cargo-deny policy and three-target CI workflow`.

### A4: Slice A close-out

- [x] A4.1 Run V1, V2, V3, V4, V6, V11 and V10 (`git add --intent-to-add .` then `git diff --stat -- . ':!Cargo.lock'`; expected ~282, must be <= 800). Commit `Cargo.lock`: `build: add Cargo.lock`.
- [ ] A4.2 Push `feat/workspace-scaffold-a`, open PR 1. Done when the first real CI run shows three `test` jobs (Windows x86_64, Linux x86_64, macOS aarch64) plus `policy`, all green (V7b, V9; also covers the "four gates" scenario) and `sha_pinning_required` is true (V7a).

## Slice B: `gafira-geom` (~629 lines), starts after PR 1 is merged (or per chain strategy)

### B1: Crate skeleton ~41 lines

- [x] B1.1 Create the branch per chain strategy. Modify root `Cargo.toml` (+6: `gafira-geom`, `kurbo`, `serde`, `serde_json`, `proptest`); resolve `kurbo` 0.13 or newest compatible (`serde` feature, `rust-version` <= 1.93). Create `crates/gafira-geom/Cargo.toml` (15) and `src/lib.rs` (20, `#![forbid(unsafe_code)]`, empty modules). Done when `cargo check --workspace --all-targets` passes and `cargo tree -p gafira-geom -e dev` shows no `rusty-fork`/`tempfile`.

### B2: `Unit` / `Length` (T3) ~190 lines

- [x] B2.1 RED: tests in `crates/gafira-geom/src/length.rs` (unit set, 25.4 mm = 1 in within 1e-12 relative, invalid DPI x4, NaN/+-inf, same-unit bit-exact via `to_bits()`, A4@300 DPI = 2480x3508 after `round()`, NaN `Length` serializes to `null` and decoding fails). Done when `cargo test -p gafira-geom` fails.
- [x] B2.2 GREEN + REFACTOR: implement `Unit`, `Length`, `ConversionError` (hand-written `Display`/`Error`), the 5-step conversion algorithm. Commit: `feat(geom): add Length and Unit with DPI conversion`.

### B3: `Size` / `Transform2D` (T4) ~185 lines

- [x] B3.1 RED: tests in `size.rs` (35) and `transform.rs` (150): `translate(2,3).then(scale(2))` maps (1,1) to (6,8), identity, `scale(0).inverse()` is `Err`, Affine -> `Transform2D` -> Affine and `[f64;6]` round trips compared with `to_bits()`. Done when they fail.
- [x] B3.2 GREEN + REFACTOR: implement `Size`, `Transform2D`, `SingularTransform`, the four `From` impls. Commit: `feat(geom): add Size and Transform2D`.

### B4: API surface (T5) ~40 lines

- [x] B4.1 RED: `crates/gafira-geom/tests/api.rs` (kurbo parity of the four re-exports, `"in"` accepted, `"km"` rejected). GREEN: re-exports in `lib.rs`, serde attributes. Commit: `feat(geom): re-export kurbo primitives and fix serde wire names`.

### B5: Property tests (T6) ~120 lines

- [x] B5.1 `crates/gafira-geom/tests/properties.rs`: conversion round trip ([-1e9, 1e9], DPI [1, 1e4]); inverse of composed transforms (|s| in [0.1, 10]); JSON round trip of `Length`, `Unit`, `Size`, `Transform2D`, `Point`, `Vec2`, `Rect`, `Affine` with `to_bits()`. RED shown by temporarily breaking a factor or a serde attribute, then reverting. Done when `cargo test -p gafira-geom` passes; commit any `proptest-regressions/`. Commit: `test(geom): add property tests for conversions and serde round trips`.

### B6: Layer purity (T7 geom part) ~53 lines

- [x] B6.1 RED: scratch `std::fs::read` passes clippy before `clippy.toml`; scratch `unsafe {}` fails `cargo check -p gafira-geom`. GREEN: create `crates/gafira-geom/clippy.toml` (45, ARQ-02 lists, `reason = "ARQ-02"`); the scratch call now fails with `disallowed_methods`; no "does not refer to an existing" warnings. Revert scratch code. Run V5, V11, V12 (trivial in C1).
- [x] B6.2 Modify `.github/workflows/ci.yml` (+8: V5 and V12 steps in `policy`), keeping pins as 40-hex SHAs. Tick checklist D16c again. Commit: `ci: enforce geom layer purity and clippy policy sync`.

### B7: Slice B close-out

- [ ] B7.1 Run V1-V6, V10 (`git diff --stat <base>...HEAD -- . ':!Cargo.lock'`, ~629), V11, V12. Commit the `Cargo.lock` update, push, open PR 2. Done when CI is green on the three systems plus `policy` (with V5 and V12).

## Suggested Work Units

| Unit | Goal | Likely PR | Focused test command | Runtime harness | Rollback boundary |
|------|------|-----------|----------------------|-----------------|-------------------|
| 1 (A) | Workspace policy, `gafira-cli`, deny and CI | PR 1 | `cargo test -p gafira-cli` | `cargo run -p gafira-cli -- --version`; `cargo deny check licenses bans sources`; first GitHub CI run | Remove all slice A paths (nothing else exists) |
| 2 (B) | `gafira-geom` and its purity checks | PR 2 | `cargo test -p gafira-geom` | V5, V11, V12 shell checks; `cargo clippy -p gafira-geom -- -D warnings` | Remove `crates/gafira-geom/`; revert the +6 and +8 line edits |

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | Slice A ~282; Slice B ~629; total ~911 (`Cargo.lock` excluded) |
| 400-line budget risk | High (total exceeds 800; B alone exceeds 400 but fits the 800 session budget) |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (A) -> PR 2 (B) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending (proposed: stacked-to-main) |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

Notes:
- Against the 800-line session budget, A (~282) and B (~629) each fit; the ~911 total does not. Against the 400-line default, B exceeds it, so a chain is recommended.
- Cut A then B. After PR 1, CI must pass: three `test` jobs (clippy and tests on windows-2022, ubuntu-24.04, macos-14) and `policy` (fmt, V6, V11, cargo-deny), with `sha_pinning_required: true`. After PR 2, the same jobs plus `gafira-geom` tests and clippy, and the V5 and V12 steps.
- Branches (proposed stacked-to-main): `feat/workspace-scaffold-a` -> `main`; after merge, `feat/workspace-scaffold-b` from updated `main` -> `main`. If the holder picks feature-branch-chain: draft tracker `feat/workspace-scaffold`; PR 1 base = tracker, PR 2 base = `feat/workspace-scaffold-a`.
- The holder must choose the chain strategy before apply.
