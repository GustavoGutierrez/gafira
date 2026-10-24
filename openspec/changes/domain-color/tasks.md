# Tasks: Domain Color (C2a)

Conventions: Strict TDD (RED -> GREEN -> REFACTOR). A RED step is never committed alone; one conventional commit per task or coherent unit, made after GREEN, holding tests plus code (work-unit-commits). Commits in English, no `Co-Authored-By`, no AI attribution. IDs T1-T8, P1-P8, V*, N0-N7 and decisions D1-D13 refer to `design.md`. Lines are measured lines (`git diff --shortstat`, `Cargo.lock` and `openspec/` excluded), already at the C1 factor (~x1.43-1.5 over raw rows). Tooling: only Rust and cargo; no `sdd-attempt acquire`, no verification-report validator (gentle-ai 3.4.0 lacks both). `$SCRATCH` is the session scratchpad, outside the repo. Network is available.

Gates after every GREEN: `cargo fmt --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`.

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | PR1 ~720 (T1-T6); PR2 ~175 (T7); total ~893 |
| 400-line budget risk | High against 400 (both PRs of the 800 session cut: PR1 fits with ~80 margin, PR2 fits); Medium against the 800 session budget (total over 800; PR1 margin thin) |
| Chained PRs recommended | Yes |
| Suggested split | PR1 (T1-T6) -> PR2 (T7) |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending (proposed: stacked-to-main) |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

Cut and CI:
- After PR1: three ARQ-03 `test` jobs (Windows x86_64, Linux x86_64, macOS aarch64) and `policy` green (fmt, clippy, tests, deny, table-driven V5, V6, V11, V12). `cargo test -p gafira-color` covers types, conversions, `Color`, P1-P5, P7, P8.
- After PR2: the same jobs, plus `transform.rs` tests and P6.
- Branches, proposed stacked-to-main: `feat/domain-color` -> `main`; after merge, `feat/domain-color-transform` from updated `main` -> `main`. If the holder picks feature-branch-chain: draft tracker `feat/domain-color`; PR1 base = tracker, PR2 base = PR1 branch. Branch protection is unavailable (HTTP 403, GitHub plan), so ARQ-80 cannot be required; merge order is by convention.
- Fallback: measure after T6 (task 6.3). If PR1 is over 800, move T2 (~48) to PR2 and record it. Alternative for the holder: one PR under `size:exception` (~893).

### Suggested Work Units

| Unit | Goal | Likely PR | Focused test command | Runtime harness | Rollback boundary |
|------|------|-----------|----------------------|-----------------|-------------------|
| 1 | Crate, table-driven V5, types, conversions, `Color`, wire format, properties | PR1 | `cargo test -p gafira-color` | V5 scratch run N0-N7; `cargo tree -p gafira-color -e dev`; `cargo deny check` | Delete `crates/gafira-color/`, drop the root dependency line, restore the previous V5 step |
| 2 | `PixelLayout`, `ColorTransform`, `AnalyticTransform`, `PixelBufferError`, P6 | PR2 | `cargo test -p gafira-color transform` and `--test properties` | Gate trio on the stacked tree | Remove `transform.rs`, its re-exports and the P6 test |

## Phase 0: Prerequisites (no code, 0 lines)

- [ ] 0.1 Confirm `main` is at d02bb6f (`git rev-parse main`) and the tree is clean (`assets/splash_screen.jpg` stays untracked, never committed). Create `feat/domain-color` from `main` (`git switch -c feat/domain-color main`). Done when `git branch --show-current` prints it.
- [ ] 0.2 Record `cargo deny --version` (0.20.2) and `rustc --version` (1.93.1); `gh api repos/GustavoGutierrez/gafira/actions/permissions` shows `sha_pinning_required: true`. Holder: P0.6/ARQ-80 stays deferred (403). Done when recorded in apply-progress.
- [ ] 0.3 Holder decision: chain strategy (stacked-to-main proposed) or `size:exception`; also confirm the 02 §4.10 deviation (`Result` return), kept as PROPOSAL.

## PR1 (T1-T6, ~720 lines)

### Phase 1: Crate skeleton and layer purity (T1, V3, V11, V12) ~94 lines

- [ ] 1.1 RED: `cargo check -p gafira-color` fails ("did not match any packages"). Done when it fails. 0 lines.
- [ ] 1.2 GREEN: create `crates/gafira-color/Cargo.toml` (18; geom `[package]` block with `name = "gafira-color"`, `serde.workspace`, dev `proptest`/`serde_json`, `[lints] workspace = true`), `src/lib.rs` (~10 now: `#![forbid(unsafe_code)]`, crate docs), `cp crates/gafira-geom/clippy.toml crates/gafira-color/clippy.toml` (45, never hand-edited), and +1 root `Cargo.toml` line `gafira-color = { path = "crates/gafira-color" }`. Done when `cargo check -p gafira-color` passes, `cargo metadata --no-deps` shows edition 2024, rust_version 1.93, publish [] (V3), V11 and `diff -q` of the two `clippy.toml` (V12) pass.
- [ ] 1.3 Purity proof (T1): a scratch `std::fs::read` passes clippy in a copy without `clippy.toml` (RED), fails with `disallowed method` with it (ARQ-02); a scratch `unsafe {}` fails `cargo check` ("usage of an `unsafe` block"). `cargo tree -p gafira-color -e dev --target all --prefix none -f '{p}'` shows only `proptest`, `serde_json` and transitive crates: no `gafira-*` besides itself, no forbidden prefix, no `rusty-fork`/`tempfile`. Revert scratch; `git status` clean. Verify the `cargo tree -f '{p}'` line format (`gafira-color v0.1.0 (/path)`) matches `^gafira-color v`.
- [ ] 1.4 Commit: `feat(color): add gafira-color crate skeleton with clippy policy`.

### Phase 2: Table-driven V5 (T2, N0-N7) ~48 lines

- [ ] 2.1 RED: copy the current V5 step body to `$SCRATCH/v5-old.sh`; add `gafira-geom.workspace = true` to color's `[dependencies]` (N1). Done when the old step exits 0 on the tree that must fail. Revert with `git checkout -- crates/gafira-color Cargo.lock`.
- [ ] 2.2 GREEN: replace the V5 step in `.github/workflows/ci.yml` with the design's table-driven body (rows `gafira-geom -`, `gafira-color -`; prefix forbidden match `^(...)`; here-string loop; `</dev/null`; captured `cargo tree`; coverage check on `crates/*/clippy.toml`). Keep every `uses` as `@<40-hex> # <tag>`, `permissions: contents: read`, the 3-triple matrix. Done when the step is a real step (not a comment) and `rg PIN-SHA` is empty.
- [ ] 2.3 Manual negatives from `$SCRATCH/v5.sh` under `bash --noprofile --norc -eo pipefail`, reverting each time: N0 exit 0 with both crates examined; N1 exit 1 "gafira-color must not depend on gafira-geom"; N2 row `gafira-colour -` exits non-zero and the coverage check names `gafira-color`; N3 deleted row exits 1 "has clippy.toml but no V5 row"; N4 `serde` added to `forbidden` exits 1 (`serde v1...`, `serde_derive v1...`); spec N5 (prefix): run the pattern against the captured line `wgpu-core v0.1.0` and `tokio-util v0.1.0`, both match; design N5 (scratch crate `crates/scratch/` without `clippy.toml` or row) exits 0, recorded as the known limitation NOT met (close with `xtask check-layers`, ARQ-01), then `rm -r crates/scratch`. N6: remove `forbid` from color `lib.rs`, V11 loop exits 1 "missing forbid". N7: append a line to color `clippy.toml`, V12 `diff -q` fails. `git status` clean after each. Record that spec and design number the prefix and scratch cases both as N5.
- [ ] 2.4 Re-verify the D16c checklist on `ci.yml` (matrix = 3 triples; fmt, clippy, test, deny real steps; pins as 40-hex; permissions). Commit: `ci: make layer check table-driven per domain crate`.

### Phase 3: Types and `ColorSpaceId` (T3, D4, D5, D7, D8) ~103 lines

- [ ] 3.1 RED: tests in `src/space.rs` (53 with code) and `src/rgb.rs` (types part): variant set and wire names `"srgb"`, `"srgb-linear"`, `"display-p3"`; `"SRGB"` and `"linear-srgb"` rejected; exhaustive `match` over `ColorSpaceId::ALL` with no wildcard arm; values retained; unvalidated construction (NaN, 2.0, -1.0); IEEE equality (`-0.0 == 0.0`, NaN channel unequal to itself); `is_finite` false for NaN or +-inf in any channel including alpha. Done when `cargo test -p gafira-color` fails to compile.
- [ ] 3.2 GREEN: implement `ColorSpaceId` (+ `ALL`), the private `macro_rules!` generating `Rgba`, `LinearRgba`, `DisplayP3Rgba` (`pub const fn new`, `is_finite`, derives per D4), modules and re-exports in `lib.rs`. Done when tests pass and gates are clean. Commit: `feat(color): add color value types and ColorSpaceId`.

### Phase 4: Math core (T4, D1-D3, D13) ~150 lines

- [ ] 4.1 GREEN-first verification of constants (prerequisite for 4.2): fetch the CSS Color 4 "Sample code for color conversions" (https://www.w3.org/TR/css-color-4/#color-conversion-code, network available; fall back to the W3C repo `w3c/csswg-drafts` `css-color-4/conversions.js`) and compare every rational of `SRGB_TO_XYZ`, `XYZ_TO_SRGB`, `P3_TO_XYZ`, `XYZ_TO_P3` with the design table (transcribed from memory). Also compare the published `color(display-p3 ...)` examples with the test references (sRGB red/green/blue to P3, P3 red to extended sRGB). Done when each constant is confirmed or corrected in code, and the result (including any correction to the design table) is written to apply-progress. Do not edit design or specs.
- [ ] 4.2 RED: tests in `src/math.rs`: decode(0)=0, decode(1)=1, decode(0.5) ~ 0.2140411 (f64 evaluation in a test comment); continuity at 0.04045 (`1e-8`) and 0.0031308 (`5e-8`); M*M^-1 = I for both pairs (`1e-12`); `SRGB_TO_XYZ*[1,1,1]` and `P3_TO_XYZ*[1,1,1]` = D65; Y row ~ (0.2126, 0.7152, 0.0722); sRGB R/G/B to P3 ~ (0.91749, 0.20029, 0.13856), (0.45840, 0.98526, 0.29829), (0, 0, 0.9596) at `1e-4`; P3 red to extended sRGB ~ (1.0931, -0.2267, -0.1501), unclamped; fixed tables for 0, -0.0, 1, thresholds, f32 subnormals; `convert_rgb` never panics on non-finite. Done when they fail.
- [ ] 4.3 GREEN: implement `Mat3`, the rational constants, `const fn mul` for `SRGB_TO_P3`/`P3_TO_SRGB`, `decode`/`encode` on `|x|` with `copysign`, and `convert_rgb(src, dst, [f32; 3])` (widen to f64, narrow once), all `pub(crate)`. Verify during apply: `const fn` float arithmetic compiles on 1.93; no `clippy::excessive_precision`; round trip of f32 subnormals; `-0.0` stays `-0.0`. Done when tests pass.
- [ ] 4.4 REFACTOR + mutation: 2.4 -> 2.2 must fail decode(0.5); a typo in one rational must fail M*M^-1; revert both. Commit: `feat(color): add sRGB transfer curve and CSS Color 4 matrices`.

### Phase 5: Typed conversions, `Color`, wire format (T5, D6-D8) ~220 lines

- [ ] 5.1 RED: unit tests in `rgb.rs` (~70) and `color.rs` (~90), plus `tests/api.rs` (60). Unit: all nine pairs via `Color::convert`; `convert` to the same space is bit-identical for NaN, `-0.0`, out-of-range; out-of-gamut conserved; NaN and +-inf convert to every space without panic with alpha bit-identical; `Srgb(x) != LinearSrgb(x) != DisplayP3(x)`; `From` impls, `space()`, `alpha()`, `is_finite()`. API: exact JSON `{"space":"srgb","r":1.0,"g":0.5,"b":0.0,"a":1.0}` (tag first, declaration order); bare `Rgba` JSON; `"display-p3"` id; unknown, missing or `"SRGB"` `space` rejected; missing `a` rejected; `"r":"x"` rejected; `{"keys":[]}` rejected as `Color`; NaN serializes to `null` and decoding fails; extra field ignored. Done when they fail.
- [ ] 5.2 GREEN: implement the typed methods (`to_linear`, `to_srgb`, `to_display_p3`, `to_linear_srgb`) over `convert_rgb`, and `Color` (`#[serde(tag = "space")]`, exhaustive, `From` impls, `convert`, `to_linear_srgb`). Done when `cargo test -p gafira-color` passes.
- [ ] 5.3 REFACTOR + mutation: removing sign mirroring must fail the P3-red reference; revert. Commit (may be two): `feat(color): add typed conversions between sRGB, linear and Display-P3`; `feat(color): add Color enum with CSS space names`.

### Phase 6: Properties and size checkpoint (T6) ~105 lines

- [ ] 6.1 `tests/properties.rs`: P1 (sRGB -> linear -> sRGB, [0,1] and [-4,4], `2e-6`), P2, P3 (monotonic decode and encode on [-4,4]), P4 (alpha bit-preserved), P5 (`decode(-x)` bits = `-decode(x)`), P7 (serde round trip bit-identical for all three types, all `Color` variants and `ColorSpaceId`, finite `f32` incl. `-0.0`/subnormals; verify the internally-tagged `Content` path is bit-exact), P8 (composition, `1e-4`); P3 round trips `1e-5` in [0,1] and `1e-4` in [-4,4]; add near-zero samples. Guard test with `TestRunner::deterministic()` proves the generators reach `-0.0` and subnormals. RED by mutation (truncate through `f32` mid-pipeline; rename a variant), then revert. Done when properties pass; commit any real `proptest-regressions/` (not mutation artifacts). Commit: `test(color): add property tests for curves, conversions and serde`.
- [ ] 6.2 Run all gates, V3, V5 (N0), V6, V11, V12, `cargo tree -p gafira-color -e dev` check, `cargo deny check`.
- [ ] 6.3 Size checkpoint: `git diff --shortstat main...HEAD -- . ':!Cargo.lock' ':!openspec'`. Done when the figure is in apply-progress; if over 800, move task 2 to PR2 and tell the orchestrator before pushing.
- [ ] 6.4 Commit `Cargo.lock` if changed (`build: add gafira-color to Cargo.lock`), push `feat/domain-color`, open PR1 (chain context; dependency diagram if chained). Done when the three `test` jobs and `policy` are green.

## PR2 (T7, ~175 lines), after PR1 merges (or per chain strategy)

### Phase 7: `ColorTransform` (T7, D9-D11, P6) ~175 lines

- [ ] 7.1 Create the branch per chain strategy (stacked-to-main: `feat/domain-color-transform` from updated `main`).
- [ ] 7.2 RED: tests in `src/transform.rs` (~150 with code) and P6 in `tests/properties.rs` (~25): `PixelLayout::channels` (Rgb 3, Rgba 4); `Box<dyn ColorTransform>` moved to another thread, `Send + Sync`; `src`/`dst`; all nine pairs; `Rgb` and `Rgba` slices; wrong length (10 with `Rgba`, 7 with `Rgb`) returns `Err` and leaves the buffer bit-identical; empty slice ok; identity transform bit-identical; `Display` of `LengthNotMultiple` names `len` and `channels`; usable as `&dyn std::error::Error`; P6 `apply_f32` bit-identical to the per-pixel typed API. Done when they fail.
- [ ] 7.3 GREEN: implement `PixelLayout`, `ColorTransform` (no `bake_lut3d`, no `Lut3d`), `AnalyticTransform` (`const fn new`, `chunks_exact`, shared `convert_rgb`), `PixelBufferError` (`#[non_exhaustive]`, hand-written `Display`/`Error`), re-exports. Done when tests pass; the P6 mutation (bypassing `convert_rgb`) fails; revert.
- [ ] 7.4 Commit: `feat(color): add ColorTransform with analytic implementation`.

### Phase 8: Final verification (T8)

- [ ] 8.1 Run `cargo fmt --check`, clippy with `-D warnings`, `cargo test --workspace --locked`, `cargo deny check`, V3, V5 (N0), V6, V11, V12; record the measured size of PR2 and of the whole change. Done when green locally.
- [ ] 8.2 Push, open PR2, wait for the three `test` jobs and `policy` green. Pending items stay recorded: ΔE2000 (deferred), holder confirmation of the `Result` deviation, N5 limitation (not met), ARQ-80 (403).
