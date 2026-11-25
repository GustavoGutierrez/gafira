# Apply Progress: domain-color (PR1 T1-T6, PR2 T7)

Mode: Strict TDD (runner `cargo test`). Backend: hybrid (Engram not available to the executor; the orchestrator persists it). Branch: `feat/domain-color` from `main` (d02bb6f). PR1 was merged as #3-#5; this file now also covers PR2 on `feat/domain-color-transform`.

Status: tasks 0.1, 0.2, 1.1-6.3, 7.1-7.4, 8.1 done (8.2 by the coordinator). Pending: 0.3 `Result` deviation confirmation.

PR1 measured 1718 lines (limit 800) and was split and merged as #3-#5 (see "PR1 delivery note").

## Commits (oldest first)

| Hash | Message |
|---|---|
| e400d4b | `chore(sdd): add domain-color change artifacts` |
| dfb4dbc | `feat(color): add gafira-color crate skeleton with clippy policy` (includes the `Cargo.lock` change) |
| d3e524b | `ci: make layer check table-driven per domain crate` |
| d51cee4 | `feat(color): add color value types and ColorSpaceId` |
| 9ddbfd2 | `feat(color): add sRGB transfer curve and CSS Color 4 matrices` |
| dbbc925 | `feat(color): add typed conversions between sRGB, linear and Display-P3` |
| 1e30812 | `feat(color): add Color enum with CSS space names` |
| 0952c36 | `test(color): add property tests for curves, conversions and serde` |

## Environment (task 0.2)

`cargo deny --version` = cargo-deny 0.20.2; `rustc --version` = rustc 1.93.1 (01f6ddf75 2026-02-11); `gh api repos/GustavoGutierrez/gafira/actions/permissions` = `{"enabled":true,"allowed_actions":"all","sha_pinning_required":true}`. P0.6/ARQ-80 (branch protection, HTTP 403 on the GitHub plan) stays deferred; not re-queried.

## Task 4.1: constants verified against the published source

Source fetched: `https://raw.githubusercontent.com/w3c/csswg-drafts/main/css-color-4/conversions.js` (509 lines) and `https://www.w3.org/TR/css-color-4/`.

- `lin_sRGB_to_XYZ`, `XYZ_to_lin_sRGB`, `lin_P3_to_XYZ`, `XYZ_to_lin_P3`: every rational compared token by token (script diff of the numerator/denominator sets against the design table). **All four identical; no constant corrected.** (The only textual difference is `0 / 1` in the source versus `0` in the design for `P3_TO_XYZ[2][0]`.)
- Curve: `abs <= 0.04045 -> val / 12.92`, else `sign * ((abs + 0.055) / 1.055)^2.4`; encode `abs > 0.0031308 -> sign * (1.055 * abs^(1/2.4) - 0.055)`, else `12.92 * val`. Matches D1 (thresholds, constants, sign-symmetric extension, linear branch keeps `-0.0`).
- D65 = `[0.3127/0.3290, 1, (1-0.3127-0.3290)/0.3290]`: matches.
- Reference values: the spec text literally publishes `red == color(display-p3 0.91748756 0.20028681 0.13856059)`, which confirms sRGB red -> P3 `(0.91749, 0.20029, 0.13856)`. The other references (green, blue, P3 red -> sRGB, decode(0.5)) are NOT printed in the spec; they were reproduced by an independent f64 evaluation of the verified CSS matrices (scratch Rust program, outside the repo): green `(0.45840, 0.98526, 0.29829)`, blue `(0, ~0, 0.95959)`, P3 red -> sRGB `(1.09307, -0.22674, -0.15013)`, `decode(0.5) = 0.214041140`, P3 red -> linear sRGB `(1.224940, -0.042057, -0.019638)`. Design values confirmed. The "literal agreement with CSS examples" item is therefore met for red only; the rest rely on the verified matrices plus f64 evaluation (recorded in the test comments).
- Mutation proof that the tests detect transcription errors: a typo in `SRGB_TO_XYZ[0][1]` (87881 -> 87891) failed 6 tests (inverse pairs, white point, composite white, references); a typo in `P3_TO_XYZ[2][1]` (32229 -> 32239) failed 4 tests. Both reverted.

## TDD Cycle Evidence

| Task | Test file | Layer | Safety net | RED | GREEN | TRIANGULATE | REFACTOR |
|---|---|---|---|---|---|---|---|
| 1.1-1.2 | (no unit test; command RED) | Command | N/A (new) | `cargo check -p gafira-color` exit 101, "did not match any packages" | `cargo check -p gafira-color` ok; metadata edition 2024, rust 1.93, publish []; V11/V12 ok | scratch `std::fs::read`: clippy passes without `clippy.toml` (exit 0), fails with it ("use of a disallowed method `std::fs::read`"); scratch `unsafe {}` fails check ("usage of an `unsafe` block") | N/A |
| 2.x | `$SCRATCH` shell scripts (N0-N7) | Command | existing V5 | old V5 exits 0 on the N1 tree that must fail | new V5 N1 exits 1 | N0-N7 (below) | step body identical to design plus `set -euo pipefail` |
| 3.1-3.2 | `src/space.rs`, `src/rgb.rs` | Unit | N/A (new) | compile failure: unresolved imports `space::ColorSpaceId`, `rgb::{Rgba,...}` (27 errors in the test build) | 20 passed | 3 types via one test macro; 7 sample values x 4 slots; 4 non-finite cases | clean |
| 4.2-4.4 | `src/math.rs` | Unit | 20/20 | compile failure: `decode`, `encode`, `mul`, `mul_vec`, `Mat3`, constants, `convert_rgb` not found (80 errors) | 43 passed (after fixing one over-strict test: `encode(1.0)` is `0.9999999999999999` in f64, now `< 1e-15`) | 23 tests: references, continuity both branches both signs, odd symmetry, inverse pairs x4, white point x2, composites, references via `convert_rgb`, structure of every pair, edge values | mutation 2.4 -> 2.2: 6 failed; typo in two rationals: 6 and 4 failed; reverted |
| 5.1-5.3 | `src/rgb.rs`, `src/color.rs`, `tests/api.rs` | Unit + integration | 43/43 | compile failure: no `to_linear`, `to_srgb`, `to_display_p3`, `to_linear_srgb`, `Color` | 63 lib + 10 api passed | all nine pairs, same-space bit identity, non-finite x 144 cases, alpha bits, variants unequal, 10 wire-format cases | mutation: remove `copysign` in both curves: 9 failed (incl. P3-red reference and `convert_matches_the_published_references`); reverted |
| 6.1 | `tests/properties.rs` | Property | 73/73 | the properties passed on first run (they test behaviour already implemented), so RED is shown by mutation, below | 21 passed; also 3 runs at `PROPTEST_CASES=20000` all green | generators include near-zero, `-0.0`, subnormals, both thresholds; guard tests prove it (the first run of the guard tests failed twice and exposed two generator gaps, fixed) | clean |

Mutations for T6 (properties only, then reverted; `git status` clean afterwards):

| Mutation | Result |
|---|---|
| M1 narrow intermediate to `f32` (`c as f32`) | NOT detected (21 passed). Equivalent within tolerance: the design already narrows once per conversion and the tolerances were sized for it. Recorded, not a gap. |
| M2 truncate the mantissa of the intermediate (clear 12 low bits) | 9 properties failed (P1, P2, P3 round trips, P8) |
| M3 rename `Color::LinearSrgb` tag to `linear-srgb` | properties NOT affected (rename is symmetric on write and read, so a round trip survives); caught by `tests/api.rs` (2 failed) |
| M3b rename `ColorSpaceId::DisplayP3` wire name | properties NOT affected; caught by `space.rs` tests (2 failed) |
| M4 scale alpha by 0.999 in `Color::convert` | `alpha_survives_every_conversion`, `non_finite_channels_never_panic` failed (and `convert_reaches_every_target...` in unit tests) |
| M5 drop `copysign` from `decode` | 7 properties failed (P5 odd symmetry, P3 monotonic, round trips, P8) |

Test summary: 154 workspace tests pass (gafira-color: 63 lib + 10 api + 21 properties = 94). Layers: unit, integration, property. Pure functions created: `decode`, `encode`, `mul`, `mul_vec`, `convert_rgb`.

## Manual negatives (task 2.3, `bash --noprofile --norc -eo pipefail`, each reverted, `git status` clean)

Scripts extracted from `ci.yml` itself (`$SCRATCH/extract.sh`), so the tested text is the committed text. `$SCRATCH` = the session's scratch directory, outside the repository.

| Case | Mutation | Result |
|---|---|---|
| N0 | clean tree | exit 0; xtrace shows `cargo tree -p gafira-geom ...` and `cargo tree -p gafira-color ...` both ran; `^gafira-color v` matches `cargo tree -f '{p}'` output |
| N1 | `gafira-geom.workspace = true` in color deps | new V5: `::error::gafira-color must not depend on gafira-geom`, exit 1. Old V5 on the same tree: exit 0 (the RED). |
| N2 | row `gafira-colour -` | `::error::gafira-color has clippy.toml but no V5 row`, then `error: package ID specification gafira-colour did not match any packages`, exit 101 |
| N3 | row deleted | `::error::gafira-color has clippy.toml but no V5 row`, exit 1 |
| N4 | `serde\|` prepended to `forbidden` | `::error::gafira-color: forbidden serde v1.0.229` plus `serde_core`, `serde_derive` lines, exit 1 |
| N5 (spec, prefix) | pattern `^(egui\|...)` against captured lines | `wgpu-core v0.1.0`, `tokio-util v0.1.0`, `winit-x v1.0.0` all match; the exact-name-with-space form (`^(...) `) misses `wgpu-core` (exit 1), confirming the prefix form is stronger. Side effect: a crate named `miow` also matches the `mio` prefix (by design of the prefix rule). |
| N5 (design, scratch crate) | `crates/scratch/` with manifest and `forbid`, no `clippy.toml`, no row | exit 0 = known limitation, requirement NOT met; closes with `xtask check-layers` (ARQ-01). Scratch removed. |
| N6 | remove `forbid` from color `lib.rs`, run V11 body | `missing forbid(unsafe_code): crates/gafira-color/src/lib.rs`, exit 1 |
| N7 | append a line to color `clippy.toml`, run V12 body | `diff -q` reports the files differ, exit 1 |

Discrepancy recorded (no spec or design edit): spec and design give the label N5 to two different cases (spec: prefix match for `wgpu-core`/`tokio-util`; design: scratch crate without `clippy.toml`). Both were run; the first passes the requirement, the second documents the unmet limitation.

D16c checklist on `ci.yml`: matrix still 3 triples (windows-2022, ubuntu-24.04, macos-14); fmt, clippy, test, deny are real steps; the three `uses` are `@<40-hex> # <tag>`; `permissions: contents: read`; `rg PIN-SHA` empty; the YAML parses.

## Gates (end of PR1, on commit 0952c36)

| Gate | Result |
|---|---|
| `cargo fmt --check` | rc 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | rc 0 |
| `cargo test --workspace --locked` | 154 passed, 0 failed |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |
| `cargo metadata --no-deps` (V3) | gafira-color: edition 2024, rust_version 1.93, publish [] |
| V5 (table, from `ci.yml`) | rc 0 (N0) |
| V6 (ARQ-07 step) | rc 0 |
| V11 | rc 0 |
| V12 | rc 0 (`diff -q` of the two `clippy.toml`) |
| `cargo tree -p gafira-color -e normal,build,dev --target all --prefix none -f '{p}'` | only `gafira-color` among `gafira-*`; no forbidden prefix; no `rusty-fork`/`tempfile`; direct normal dependency: `serde` only |

## Work Unit Evidence (unit 1)

| Evidence | Value |
|---|---|
| Focused test command | `cargo test -p gafira-color` : lib 63 passed, api 10 passed, properties 21 passed, 0 failed; also `PROPTEST_CASES=20000 cargo test -p gafira-color --test properties` x3 green |
| Runtime harness | V5 N0-N7 scratch runs (table above); `cargo tree -p gafira-color -e normal,build,dev`; `cargo deny check` ok |
| Rollback boundary | delete `crates/gafira-color/`; remove the `gafira-color = ...` line in root `Cargo.toml` and the 9 `Cargo.lock` lines; restore the previous V5 step in `.github/workflows/ci.yml` (commit d3e524b isolates it) |

## Size checkpoint (task 6.3)

`git diff --shortstat main...HEAD -- . ':!Cargo.lock' ':!openspec'` = **11 files changed, 1711 insertions(+), 7 deletions(-) = 1718 changed lines** (forecast ~720; limit 800). `Cargo.lock` +9 and `openspec/` excluded.

| Part | Lines |
|---|---|
| Production code (`space` 30, `rgb` 101, `math` 143, `color` 104, `lib` 15) | ~393 |
| In-crate unit tests (`space` 72, `rgb` 185, `math` 303, `color` 212) | ~772 |
| `tests/api.rs` + `tests/properties.rs` | 130 + 328 = 458 |
| `Cargo.toml` (crate 17, root 1), `clippy.toml` (copy) | 63 |
| `ci.yml` (+25 -7) | 32 |

The overrun is tests (about 1230 lines, versus ~190 forecast for the same files). The design's fallback (move T2 to PR2) would remove only 32 lines (PR1 would still be ~1686), so it cannot bring PR1 under 800 and it would also leave `gafira-color` outside the V5 layer table in PR1's CI. **I did not apply it and did not rewrite history**; the decision belongs to the holder. The V5 change is isolated in commit d3e524b and can be moved with one cherry-pick. No requirement was cut.

## Deviations (design and tasks untouched)

1. Task 0.1: the branch `feat/domain-color` did not exist; I created it from `main` and made the `chore(sdd)` commit first.
2. `Cargo.lock` is committed inside the skeleton commit (dfb4dbc), not as a late `build:` commit, so every commit passes `--locked`. Task 6.4's lock commit is therefore a no-op.
3. `cargo tree -e dev` (as written in tasks 1.3 and V5-dev) lists only direct dev dependencies; transitive checks use `-e normal,build,dev`.
4. In the math commit `mod math;` is declared as `#[cfg(test)]` so the lib build has no dead code without any `#[allow]`; the gate is removed in the typed-conversions commit that consumes it.
5. P3 (monotonic) for `encode` is asserted as `encode(lo) <= encode(hi) + 5e-8`. The published encode constants have a genuine downward step of 2.85e-8 just above 0.0031308 (f64: 0.040449936 linear side, 0.040449907 power side), which survives `f32` narrowing, so the strict claim in the spec is false inside a ~2e-9 wide window. `decode` is asserted strictly (its step is upward). A guard test (`encode_really_has_the_published_downward_step`) proves the slack is needed and is not larger than the gap. Spec wording ("`f(x) <= f(y)`") should be amended in a later change.
6. V5 step: the committed body is the design body plus a leading `set -euo pipefail` (lesson b) and the step name `Domain crates respect the layer table (V5)`. V6, V11, V12 were not touched (they already run under `shell: bash`, which is `bash -eo pipefail`).
7. `encode(1.0)` equals `0.9999999999999999` in f64; the unit test uses `< 1e-15` instead of equality.
8. The commit for T5 is split in two (allowed): typed conversions, then `Color` + `tests/api.rs`. The last commit also contains a small bit-comparison tidy of two `color.rs` tests.
9. P7 does not detect a symmetric rename of a wire name (write and read change together); that is covered by `tests/api.rs` and `space.rs` exact-JSON tests.

## Open items

- Holder: decide PR1 size (1718 vs 800): `size:exception`, or split differently (the design's fallback is insufficient). Task 0.3 (chain strategy, 02 4.10 `Result` deviation) still open.
- Push and PR1 (task 6.4), CI green on the three `test` jobs and `policy`: orchestrator.
- PR2 (T7, `ColorTransform`, P6) not started. Pending items carried: ΔE2000 (deferred), `Result` deviation confirmation, N5 limitation (not met), ARQ-80 (403), spec wording of P3 (item 5).

## PR1 delivery note

PR1 (measured 1718 lines against the 800 forecast cut) was merged as three PRs (#3-#5) instead of one; chain strategy was stacked-to-main, so the feature-branch-chain tracker of task 0.3 was never needed. The `Result` deviation from 02 §4.10 is still pending owner confirmation.

## PR2 (T7, tasks 7.1-7.4, 8.1)

Branch `feat/domain-color-transform` from `origin/main` (0bb420b), in a separate worktree. Not pushed by the executor (8.2 is the coordinator's).

Origin: the pre-archive verify found `ColorTransform` (PR2) was never built, so it was implemented now.

### Scenario -> test

| Scenario | Test |
|---|---|
| Channel counts | `transform::tests::layouts_report_their_channel_counts` |
| Object safety and thread bounds | `boxed_transform_is_object_safe_send_and_sync` |
| All nine pairs, Rgba slice | `all_nine_pairs_match_color_convert_on_rgba` |
| Rgb slice | `rgb_layout_converts_triples_without_an_alpha_channel` |
| Wrong length | `wrong_length_is_an_error_and_leaves_the_buffer_untouched` |
| Empty buffer | `empty_buffer_succeeds_for_both_layouts` |
| Error rendering | `error_names_length_and_channels_and_is_a_std_error` |
| Identity transform | `identity_transform_keeps_every_bit` |
| Agreement with the typed API (P6) | `tests/properties.rs::apply_f32_agrees_with_the_typed_api` |

Not covered as a separate test: "Slice round trip (property)"; it follows from P6 (`apply_f32_agrees_with_the_typed_api`, `tests/properties.rs:156-169`, bit identity with the typed API) plus the typed round trips: `display_p3_round_trip_in_unit_range` (`tests/properties.rs:199`) and P4 `alpha_survives_every_conversion` (`tests/properties.rs:171-173`) for alpha.

### RED

Stub `apply_f32` returning `Ok(())` without touching the buffer: 4 unit tests failed on assertions (nine pairs, Rgb, wrong length, object safety value check) and P6 failed with a minimal failing input. The `Err`-path tests failing proves the length check is not vacuous. Empty buffer, identity and error-rendering pass against the stub by construction (the stub is a no-op that returns `Ok`).

### GREEN and mutation

Implementation: `PixelLayout`, `ColorTransform`, `AnalyticTransform` (`const fn new`, `chunks_exact_mut`, shared `convert_rgb`), `PixelBufferError`, re-exports. Mutation (scaling a channel after `convert_rgb`, equivalent to bypassing the shared pipeline): 4 unit tests and P6 failed; reverted. Generated `proptest-regressions` files were deleted, not committed.

### Deviations

1. The spec scenario "Object safety and thread bounds" says the transform is moved to another thread. `clippy.toml` (ARQ-02) disallows `std::thread::spawn`, and `#[allow]` is forbidden, so the test proves `Send + Sync` at compile time through a generic bound on `Box<dyn ColorTransform>` and calls it through the trait object.
2. `Slice round trip` has no dedicated test (see above).
3. Clippy `manual_is_multiple_of` required `usize::is_multiple_of`.
4. P3 spec wording amended (see below).

### Gates (commit `feat(color): add ColorTransform with analytic implementation`)

`cargo fmt --check` ok; `cargo clippy --workspace --all-targets --locked -- -D warnings` ok; `cargo test --workspace --locked` 312 passed, 0 failed; `cargo deny check` ok (advisories, bans, licenses, sources).

### Size

`git diff --shortstat origin/main..HEAD -- crates .github` = 3 files changed, 272 insertions(+), 1 deletion(-) = 273 changed lines (forecast ~175; the whole of the overrun is tests).

### Pre-archive fixes

- Spec "Monotonic" scenario amended: encode is asserted `encode(x) <= encode(y) + 5e-8` (the published constants step down by 2.85e-8), matching `tests/properties.rs`; decode stays strict. Behavior unchanged. This resolves deviation 5 of PR1.
- C1-era note: `specs/workspace-tooling/spec.md:3` and `proposal.md:41,67` say C1 (`workspace-scaffold`) is merged but not archived. `openspec/specs/` and `changes/archive/` are still empty, so the note is accurate and was left as is. `workspace-scaffold` untouched.
- Archive order stays C1, then `domain-color`, then `domain-doc`.

### Open items

Holder: confirm the `Result` deviation. Coordinator: push and open PR2 (8.2). Carried: ΔE2000 (deferred), N5 limitation (not met), ARQ-80 (403).
