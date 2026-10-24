# Design: Workspace Scaffold (C1 of `workspace-foundations`)

## Technical Approach

Create a virtual Cargo workspace that encodes policy once (edition, MSRV, lints, profiles, license gate, CI). Two members prove it:
- `gafira-geom`: layer-0 domain, pure, f64.
- `gafira-cli`: binary, std-only.

Layer purity is enforced by a per-crate `clippy.toml` (ARQ-02) and `cargo tree` checks (ARQ-01 interim, ARQ-07). Licensing and sources are gated by `deny.toml` (ARQ-70, ARQ-71).

CI is a workflow with exactly the three ARQ-03 targets. It is verified by four things (D16):
- GitHub's native SHA-pinning policy
- its first real run on GitHub
- a manual PR review checklist
- optionally, a local `actionlint`

All project tooling is Rust (holder instruction). Requirements come from `specs/{workspace-tooling,geometry-primitives,cli-entrypoint}/spec.md`, which another agent is amending in parallel. This design follows the holder's decisions.

**Scope note**: C1 has **no `xtask`**. `xtask` arrives in a later change with `check-layers` (ARQ-01), which will absorb the interim checks V5, V6, V11 and V12.

**Repository state**: a git repository exists:
- branch `main`
- remote `origin` = `https://github.com/GustavoGutierrez/gafira.git`
- private and empty, with no commits yet

## Architecture Decisions

| # | Decision | Choice | Rejected alternatives | Rationale |
|---|---|---|---|---|
| D1 | MSRV (02 §2.1 deviation) | **Confirmed**: `rust-version = "1.93"`, toolchain pinned to `1.93.1` | Pin 1.95 now (not installed; needs network and holder approval); no pin (non-reproducible) | Builds with the installed toolchain. Edition 2024 needs 1.85 and resolver 3 needs 1.84, so 1.93 covers both. Raising it later changes two lines. |
| D2 | `unsafe_code` (deviation) | **Confirmed**: `deny` in `[workspace.lints.rust]`. `#![forbid(unsafe_code)]` goes in each domain crate **and in `gafira-cli`**, and check V11 requires it in every existing C1 member. | `forbid` for the whole workspace | When a crate sets `lints.workspace = true`, Cargo does not let it override single lints. A workspace-wide `forbid` would make future FFI crates (`gafira-codec-os`, `gafira-platform`) drop workspace lints entirely. With `deny`, those crates can use a scoped `#[allow]`. A `deny` can be relaxed by a local `#[allow]`, so every crate that needs no unsafe code also carries the source-level `forbid`, which cannot be relaxed. V11 fails if that attribute is missing. Putting it in `gafira-cli` goes beyond the spec, which only requires it in domain crates. |
| D3 | Own license (deviation) | **Confirmed**: `license = "LicenseRef-Gafira-Proprietary"`, `publish = false`, `[licenses.private] ignore = true` | Adding the LicenseRef to `allow`; `license-file` | This is valid SPDX. Unpublished crates are skipped by cargo-deny, so the allowlist stays limited to third-party licenses. The `LICENSE`/`LICENCIA.md` texts (02 §9.4) are out of scope. |
| D4 | Third-party licenses outside the list | Unicode-3.0 is already allowed (`unicode-ident` needs it), so no action is needed. Any other license is added through a per-crate `[[licenses.exceptions]]` with a written rationale comment, never a global `allow`. Set `unused-allowed-license = "allow"`. | Widening `allow` | An exception is the narrowest way to amend the list and is easy to audit. The allowlist is policy for future dependencies, so unused entries must not produce warnings. |
| D5 | Clippy layer policy location | One `clippy.toml` inside each domain crate, as the spec requires. `crates/gafira-geom/clippy.toml` is the canonical copy, and interim check V12 (`diff`) requires every other domain copy to be byte-identical to it. | Root file (would also apply to apps); `CLIPPY_CONF_DIR` (global); `crates/domain/clippy.toml` with layered folders | Clippy uses the first `clippy.toml` it finds walking up from the manifest directory, and it does **not merge** files. A file per crate keeps binaries and adapters free to do I/O. Copies could drift. V12 catches that until `xtask check-layers` exists. In C1 there is only one copy, so V12 passes trivially; it starts to matter in C2. The layered-folder option is revisited in C2 (see C2 relation). |
| D6 | Workspace clippy lints | `dbg_macro`, `todo` and `unimplemented` set to `deny`. Policy on `unwrap`/`expect` is deferred. | Copy figcraft's `unwrap_used`/`panic` deny set | No Gafira spec mandates a no-panic policy yet (02 §7.3 belongs to later crates). Adding it now would also need test allowances in every `clippy.toml`. |
| D7 | DPI argument | Raw `f64`, validated on every conversion | A `Dpi` newtype with a fallible constructor | The spec scenarios pass invalid DPI values straight to the conversion. Keeps the API small. C2 may add a project-level `Dpi` newtype on top of it. |
| D8 | Validation scope | Validate only at fallible operations (conversion, inverse). `Length::new`, `Size::new` and `Transform2D` constructors do not validate. | Fallible constructors everywhere | Matches the spec ("no validation in C1"). serde_json cannot encode NaN or infinity anyway: they are written as `null`, and `null` fails to decode as `f64`. |
| D9 | `Transform2D` representation | Newtype over `kurbo::Affine`. The wire format is `[a,b,c,d,e,f]`. | Decomposed form (x, y, w, h, rotation_deg, skew, flip) | The spec requires bit-equal round trips to and from `Affine`. The decomposed placement in 02 §3.2 belongs to C2/anim as a separate type. |
| D10 | Composition order | `a.then(b)` means "apply `a`, then `b`". No `Mul` impl. | Exposing kurbo's `*` (right-to-left) | Gives one unambiguous spelling. The spec scenario `translate(2,3).then(scale(2))` maps (1,1) to (6,8). |
| D11 | serde in geom | Always on, not behind a feature | An optional `serde` feature | The spec makes it mandatory. The document model always needs it. |
| D12 | Errors | Hand-written `Display` + `Error` | `thiserror` | Two small types do not justify a proc-macro dependency in a leaf crate. |
| D13 | CLI arguments | `std::env::args_os` with a hand-written parser | `clap` (about 10 crates), `lexopt`, `pico-args` | Three flags do not need a parser crate, and std adds no surface for cargo-deny. Revisit `lexopt` (MIT) in C2 when `validate <path>` arrives. |
| D14 | CLI integration tests | `std::process::Command` with `env!("CARGO_BIN_EXE_gafira-cli")` | `assert_cmd` + `predicates` | No dependencies are needed. Cargo resolves the `.exe` suffix on Windows. |
| D15 | Property tests in C1 | `proptest` as a dev-dependency (`default-features = false, features = ["std"]`) | Deferring to C3; fixed tables only | The spec has three "(property)" scenarios, and 04 FD-01(d) requires a property test. Turning off `fork`/`timeout` avoids `rusty-fork`/`tempfile`. |
| D16 | How `ci.yml` is verified | No project code. Four mechanisms, (a) to (d) (see "CI-file verification"): (a) GitHub's native repository policy "Require actions to be pinned to a full-length commit SHA"; (b) the first real CI run on GitHub on the three systems; (c) a manual PR review checklist; (d) optional `actionlint`, local and manual. | `xtask check-ci` with a Rust YAML parser (holder decision: not in C1; about 270 extra lines and one more dependency); `rg`/`grep` line checks (they count commented lines and cannot prove the file parses); scripts in another language (holder rule: Rust-only tooling) | GitHub itself is the authority on whether a workflow is valid and whether every action is pinned: under the policy, a run with an unpinned action fails. The first run proves the matrix really executes on Windows x86_64, Linux x86_64 and macOS aarch64, which no static check can do. The remaining structural points (exact matrix, four gates) are small enough for a review checklist. |
| D17 | `--locked` in CI | **Refinement**: CI runs `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked`. The PR review checklist (D16c) accepts the spec's literal command with or without one `--locked` token. | The exact spec strings without `--locked` | Makes CI fail if `Cargo.lock` is stale, instead of updating it silently. It does not weaken the spec's gate. |
| D18 | Third-party actions | Only `actions/checkout` and `EmbarkStudios/cargo-deny-action`. The toolchain is installed through the runner's rustup and `rust-toolchain.toml`. | `dtolnay/rust-toolchain`, `Swatinem/rust-cache` | Fewer pins means less supply-chain trust to manage. Build caching is deferred. |
| D19 | Where deny runs | One Linux job, with `[graph] targets` listing all three triples | cargo-deny on each runner (the action is Docker/Linux-only) | With the targets listed, cfg-gated Windows and macOS dependencies are still evaluated. |

## Data Flow

    apps/gafira-cli ──(no internal deps in C1)
    crates/gafira-geom ──► kurbo(serde) ──► arrayvec/smallvec
                       └─► serde(derive) ─► serde_derive, proc-macro2, quote, syn, unicode-ident
    dev only: proptest(std), serde_json(float_roundtrip)

CLI: `args_os` → `parse(&[OsString]) -> Action{Version, Help, Usage(arg)}` → write to stdout or stderr → `ExitCode`.

## File Changes (all Create; line estimates)

| File | Slice | Est. lines | Content |
|---|---|---|---|
| `Cargo.toml` | A | 40 | Workspace manifest (below), with members `crates/*` and `apps/*`; no third-party dependencies in A |
| `rust-toolchain.toml` | A | 4 | `channel = "1.93.1"`, `components = ["rustfmt","clippy"]`, `profile = "minimal"`. **No** `targets` entry, so rustup does not download other standard libraries. |
| `rustfmt.toml` | A | 4 | `newline_style = "Unix"`, `use_field_init_shorthand = true`, `use_try_shorthand = true`. Default width. |
| `deny.toml` | A | 45 | Below |
| `.github/workflows/ci.yml` | A | 72 | Slice-A version: test matrix, then fmt, V6, V11, deny |
| `apps/gafira-cli/Cargo.toml` | A | 12 | Inherited fields; `[[bin]] name = "gafira-cli"`; no dependencies |
| `apps/gafira-cli/src/main.rs` | A | 50 | `#![forbid(unsafe_code)]`, parser, output, exit codes + parser unit tests |
| `apps/gafira-cli/tests/cli.rs` | A | 55 | Spawns the binary |
| `Cargo.toml` (modify) | B | +6 | Adds `gafira-geom` (path), `kurbo`, `serde`, `serde_json`, `proptest` to `[workspace.dependencies]`. They are added in B, so A never names a path that does not exist yet. |
| `.github/workflows/ci.yml` (modify) | B | +8 | Adds the V5 (`cargo tree -p gafira-geom`) and V12 (`clippy.toml` diff) steps to `policy` |
| `crates/gafira-geom/Cargo.toml` | B | 15 | `[package]` fields `*.workspace = true`; deps `kurbo`, `serde`; dev-deps `proptest`, `serde_json`; `[lints] workspace = true` |
| `crates/gafira-geom/clippy.toml` | B | 45 | ARQ-02 lists (below): about 35 paths, one `{ path, reason }` per line, plus headers and comments |
| `crates/gafira-geom/src/lib.rs` | B | 20 | `#![forbid(unsafe_code)]`, modules, re-exports |
| `crates/gafira-geom/src/length.rs` | B | 190 | `Unit`, `Length`, `ConversionError` + unit tests (including the NaN-not-serializable case) |
| `crates/gafira-geom/src/size.rs` | B | 35 | `Size` + tests |
| `crates/gafira-geom/src/transform.rs` | B | 150 | `Transform2D`, `[f64; 6]` conversions, `SingularTransform` + unit tests |
| `crates/gafira-geom/tests/api.rs` | B | 40 | Re-exports and serde wire names, used from outside the crate |
| `crates/gafira-geom/tests/properties.rs` | B | 120 | proptest: conversion round trip, inverse, serde round trip of 8 types |
| **Slice A** (tooling, CLI, CI) | | **~282** | 40+4+4+45+72+12+50+55 |
| **Slice B** (`gafira-geom`) | | **~629** | 6+8+15+45+20+190+35+150+40+120 |
| **Total authored** | | **~911** | |

**Budget assumption (explicit)**: `Cargo.lock` is committed because the workspace has a binary, but it is **excluded** from the line count because it is generated (~200 lines).

**Budget result**: about 911 lines in total, which is **over** the 800-line review budget in `openspec/config.yaml`. Each slice fits on its own: A is ~282 and B is ~629. No requirement is cut to fit.
- The holder's guide figure for A was ≈376. The row-by-row count above gives ~282, with no tooling crate, no alias and no extra CI step in A.
- If apply runs over, slice A still has about 500 lines of margin.

**Recommendation**: deliver C1 in **two deliveries, A then B**. The final decision (two deliveries, or one under an explicit size exception) belongs to the review workload guard after `sdd-tasks`.

**Slice independence**:
- **A has no dependency on B.** Its `ci.yml` never mentions `gafira-geom`. `members = ["crates/*", "apps/*"]` tolerates a missing `crates/` directory. V11 covers `crates/*/src/lib.rs` only when such files exist, so an empty glob passes.
- **B depends on A**: it needs A's workspace manifest, lints, `deny.toml` and `ci.yml`. B edits the root `Cargo.toml` and `ci.yml`, and does not rewrite them.

**CI per delivery**:
- **After A**:
  - `test` matrix: clippy and test, covering the `gafira-cli` tests.
  - `policy`: fmt, V6, V11 (cli), deny.
  - The first push of A is also the first real CI run, which is D16(b).
- **After B**:
  - The same jobs, now also covering `gafira-geom` tests and clippy with its `clippy.toml`.
  - `policy` gains V5 and V12.

### Root `Cargo.toml`

    [workspace]  resolver = "3"; members = ["crates/*", "apps/*"]
    [workspace.package]  version = "0.1.0"; edition = "2024"; rust-version = "1.93"
                         license = "LicenseRef-Gafira-Proprietary"; publish = false
    [workspace.dependencies]   # empty in slice A; slice B adds:
      gafira-geom = { path = "crates/gafira-geom" }
      kurbo      = { version = "0.13", features = ["serde"] }      # resolve latest compatible at apply; check MSRV
      serde      = { version = "1", features = ["derive"] }
      serde_json = { version = "1", features = ["float_roundtrip"] }   # also C2 guidance
      proptest   = { version = "1", default-features = false, features = ["std"] }
    [workspace.lints.rust]    unsafe_code = "deny"
    [workspace.lints.clippy]  dbg_macro = "deny"; todo = "deny"; unimplemented = "deny"
    [profile.release]  opt-level = 3; lto = "thin"; codegen-units = 4; panic = "unwind"
                       debug = "line-tables-only"; split-debuginfo = "packed"; strip = "symbols"
    [profile.dev.package."*"]  opt-level = 2

`[workspace.dependencies]` has no entries for future crates. Cargo would reject paths that do not exist yet, so the C2/C3 slots are reserved by naming convention (`crates/gafira-<name>`).

### `deny.toml`

    [graph]  targets = ["x86_64-pc-windows-msvc","x86_64-unknown-linux-gnu","aarch64-apple-darwin"]
    [advisories]  yanked = "deny"                      # network; CI-only locally
    [licenses]  allow = [02 §9.1 list, 13 ids]; confidence-threshold = 0.93
                unused-allowed-license = "allow"
    [licenses.private]  ignore = true
    [bans]  multiple-versions = "warn"; wildcards = "deny"; allow-wildcard-paths = true
            deny = [02 §9.1 list verbatim]
    [sources]  unknown-registry = "deny"; unknown-git = "deny"; allow-git = []

`allow-wildcard-paths` keeps the private path dependency (no version) from being rejected by `wildcards = "deny"`. The schema keys must be checked against the installed cargo-deny version, because the `advisories` and `licenses` tables changed in version 2 of its schema.

### `crates/gafira-geom/clippy.toml` (ARQ-02)

- `disallowed-methods`:
  - `std::fs::{read, read_to_string, write, read_dir, metadata, create_dir, create_dir_all, remove_file, remove_dir, remove_dir_all, copy, rename}`
  - `std::fs::File::{open, create}`, `std::fs::OpenOptions::open`
  - `std::thread::spawn`, `std::thread::Builder::spawn`
  - `std::time::SystemTime::now`, `std::time::Instant::now` (stricter than ARQ-02, for determinism)
  - `std::env::{var, var_os, vars, args, args_os, current_dir, temp_dir}`
  - `std::process::Command::new`, `std::process::exit`
- `disallowed-types`: `std::fs::{File, OpenOptions}`, `std::net::{TcpStream, TcpListener, UdpSocket}`, `std::process::Command`, `std::thread::Builder`.

Clippy cannot disallow a whole module, so every path is listed one by one. Each path must resolve, otherwise clippy 1.93 warns about the config. Each entry carries `reason = "ARQ-02"`.

### `.github/workflows/ci.yml`

- **Triggers and settings**:
  - `on: push (main), pull_request, workflow_dispatch`
  - `permissions: contents: read`
  - `concurrency` cancels superseded PR runs
  - `defaults.run.shell: bash`
  - `env: CARGO_TERM_COLOR=always, CARGO_INCREMENTAL=0`
- **Job `test`**, with matrix `include`:
  - `{target: x86_64-pc-windows-msvc, os: windows-2022}`
  - `{x86_64-unknown-linux-gnu, ubuntu-24.04}`
  - `{aarch64-apple-darwin, macos-14}`

  Steps:
  1. `actions/checkout@PIN-SHA # v4.x`
  2. `rustup toolchain install` (reads `rust-toolchain.toml`)
  3. Host check: `rustc -vV` host equals `matrix.target`
  4. `cargo clippy --workspace --all-targets --locked -- -D warnings` (`--locked` is the D17 refinement)
  5. `cargo test --workspace --locked`
- **Job `policy`** (ubuntu-24.04). Steps:
  1. checkout
  2. toolchain
  3. `cargo fmt --check`
  4. ARQ-07 check (V6)
  5. `forbid` check (V11)
  6. `EmbarkStudios/cargo-deny-action@PIN-SHA # v2.x` with `command: check`
  7. *(slice B adds)* geom leaf check (V5) and `clippy.toml` sync check (V12)

The V5, V6, V11 and V12 steps are interim shell commands (`cargo tree`, `grep`/`rg`, `diff`). They migrate to `xtask check-layers` (ARQ-01) in a later change.

To resolve each `PIN-SHA` (needs network): run `git ls-remote https://github.com/<owner>/<repo> 'refs/tags/<tag>^{}'`, falling back to the unpeeled ref for lightweight tags. Write it as `uses: owner/repo@<40-hex> # <tag>`. Apply MUST NOT finish with any `PIN-SHA` left. While one remains, the workflow cannot run under the SHA-pinning policy (D16a).

### CI-file verification (D16; replaces a static parse check)

| Mechanism | What it proves | How | Owner and timing |
|---|---|---|---|
| (a) GitHub policy "Require actions to be pinned to a full-length commit SHA" | Every `uses` is pinned. Any workflow with an unpinned action fails. | Repository Actions setting. Today `sha_pinning_required` is `false`. Check with `gh api repos/GustavoGutierrez/gafira/actions/permissions`, which must report `"sha_pinning_required": true`. | **Holder pre-task**, before the first push of slice A |
| (b) First real CI run | The workflow is valid YAML and a valid workflow, and the matrix really runs on Windows x86_64, Linux x86_64 and macOS aarch64 (ARQ-03) | Push slice A to `origin` and inspect the run: three `test` jobs plus `policy`, all green | After the first push; it is also the first ARQ-80 evidence |
| (c) Manual PR review checklist | Exact matrix and required gates | Reviewer ticks four items, listed below | Each PR that touches `ci.yml` |
| (d) `actionlint` (optional, not required) | Static workflow lint | Run locally by hand. It is an external developer tool, never part of the project or of CI. | Optional |

Checklist items for (c):
1. `jobs.test.strategy.matrix.include` lists exactly `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`: no more, no fewer, no duplicates.
2. Each gate is present as a real step, not a comment:
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo fmt --check`
   - `cargo deny check` (the action with `command: check` counts)
   
   A single extra `--locked` is accepted (D17).
3. Every `uses:` is `owner/repo@<40-hex> # <tag>`, with no `PIN-SHA` left.
4. `permissions: contents: read` is present.

## Interfaces / Contracts (`gafira-geom`, illustrative signatures)

```rust
pub use kurbo::{Affine, Point, Rect, Vec2};          // only these four; not `pub use kurbo`

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit { Px, Mm, Cm, In, Pt }
impl Unit { pub const ALL: [Unit; 5]; }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]  // {"value":..,"unit":".."}
pub struct Length { value: f64, unit: Unit }
impl Length {
    pub const fn new(value: f64, unit: Unit) -> Self;
    pub const fn value(self) -> f64;
    pub const fn unit(self) -> Unit;
    pub fn convert(self, to: Unit, dpi: f64) -> Result<Length, ConversionError>;
    pub fn to_px(self, dpi: f64) -> Result<f64, ConversionError>;
}
#[non_exhaustive] #[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConversionError { InvalidDpi(f64), NonFiniteValue(f64), NonFiniteResult }

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Size { width: f64, height: f64 }
impl Size { pub const fn new(w: f64, h: f64) -> Self; pub const fn width(self) -> f64; pub const fn height(self) -> f64; }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "[f64; 6]", into = "[f64; 6]")]
pub struct Transform2D(Affine);
impl Transform2D {
    pub const IDENTITY: Self;
    pub fn translation(offset: Vec2) -> Self;
    pub fn scale(factor: f64) -> Self;
    pub fn scale_xy(sx: f64, sy: f64) -> Self;
    pub fn rotation(radians: f64) -> Self;
    pub fn then(self, next: Self) -> Self;
    pub fn apply(self, p: Point) -> Point;
    pub fn determinant(self) -> f64;
    pub fn inverse(self) -> Result<Self, SingularTransform>;
    pub const fn coeffs(self) -> [f64; 6];
}
impl From<Affine> for Transform2D {}  impl From<Transform2D> for Affine {}
// Required by #[serde(from/into)]; both are lossless (Affine::new / Affine::as_coeffs):
impl From<[f64; 6]> for Transform2D {}  impl From<Transform2D> for [f64; 6] {}
pub struct SingularTransform;  // Debug, Clone, Copy, PartialEq, Display, Error
```

**Conversion algorithm**:
1. If `dpi` is not finite or is ≤ 0, return `InvalidDpi`. This check runs on every conversion, including same-unit ones.
2. If the value is not finite, return `NonFiniteValue`.
3. If the target unit equals the source unit, return `self` (bit-exact).
4. Otherwise compute `value / upi(from) * upi(to)`, with units per inch: px = dpi, mm = 25.4, cm = 2.54, in = 1, pt = 72.
5. If the result is not finite, return `NonFiniteResult`.

**`inverse`**: returns `Err` when the determinant is 0 or not finite, or when any resulting coefficient is not finite. It never returns NaN.

**f64 policy**:
- The public API uses only `f64`.
- No type implements `Eq`, `Ord` or `Hash` over floats.
- The derived `PartialEq` uses IEEE 754 comparison, **not** bit patterns: NaN ≠ NaN and `-0.0 == 0.0`.
- Tests that claim "bit-identical" or "bit-equal" results (Affine interop in T4, serde round trip in T6) MUST compare each `f64` field with `to_bits()`. For example, `coeffs().map(f64::to_bits)` for `Transform2D`/`Affine`, and `x.to_bits()`/`y.to_bits()` for `Point`/`Vec2`. They MUST NOT rely on `==`.
- `-0.0` round-trips with its sign intact, which only a `to_bits()` comparison can show.
- `Length::new(f64::NAN, _)` (or ±inf) can be built but **cannot be persisted**: serde_json writes `null`, and `null` fails to decode as `f64`. A unit test in T3 records this: serialize, then deserialize must return `Err`.
- Re-exported kurbo types are not validated, and their wire format belongs to kurbo, pinned by `Cargo.lock`.

**Not exposed**:
- the whole `kurbo` crate, `kurbo::Size`, `BezPath`
- Length arithmetic, unit-normalizing equality, rounding to whole pixels, any default or ambient DPI
- the decomposed placement, interpolation
- any I/O

## CLI contract

| Invocation | stdout | stderr | Exit |
|---|---|---|---|
| `--version` (sole argument) | `gafira-cli {CARGO_PKG_VERSION}\n` | empty | 0 |
| `--help` (sole argument) | usage listing `--help`, `--version` | empty | 0 |
| no args | empty | usage | 2 |
| unknown arg or extra args | empty | `error: unexpected argument '<arg, lossy UTF-8>'` + usage hint | 2 |
| write failure on stdout | — | best effort | 1 |

There is no `-V` or `validate`; those come in C2. The version comes from `version.workspace = true`.

**Scope notes**:
- `--help` and the no-argument behavior go beyond the proposal, which says "`--version` only". They come from `specs/cli-entrypoint/spec.md`, and the proposal is not edited.
- Exit code 1 on a stdout write failure is a design **extension** that the spec does not cover. No RED test is planned for it, because there is no portable way to force a write failure. It is implemented as plain error mapping (no panic on a broken pipe).

## Testing Strategy (Strict TDD; RED → GREEN → REFACTOR per task)

| Order | Requirement | RED first | GREEN |
|---|---|---|---|
| T1 | Workspace + toolchain | V1/V2 fail (no manifest) | Root files, empty members |
| T2 | CLI version/help/usage | `tests/cli.rs` (5 scenarios) + `parse` unit tests fail to compile | `main.rs` |
| T3 | Unit/Length | unit set, retained, reference 25.4 mm (1e-12 relative), invalid DPI ×4, NaN/±inf, same-unit (bit-exact, compared with `to_bits()`), A4@300 DPI → 2480×3508 after `round()` (04 FD-01a). Also: a NaN `Length` serializes to `null` and decoding it fails. | `length.rs` |
| T4 | Size, Transform2D | composition (6,8), identity, singular `scale(0)`, Affine → `Transform2D` → Affine and `[f64;6]` round trips compared coefficient by coefficient with `to_bits()` | `size.rs`, `transform.rs` |
| T5 | Re-exports, serde names | `tests/api.rs`: kurbo parity, `"in"`, `"km"` rejected | `lib.rs` re-exports, serde attributes |
| T6 | Properties | `tests/properties.rs`: A→B→A in [-1e9, 1e9] and DPI [1, 1e4]; inverse of transforms composed from rotation, translation and scale with \|s\| in [0.1, 10]. JSON round trip of **`Length`, `Unit`, `Size`, `Transform2D`, `Point`, `Vec2`, `Rect`, `Affine`**, with arbitrary finite f64 (including `-0.0` and subnormals) and every field compared with `to_bits()`. | (passes when T3–T5 are correct; RED is shown by temporarily breaking a factor or a serde attribute) |
| T7 | Purity, clippy, unsafe | V5 grep; scratch `std::fs::read` passes clippy **before** `clippy.toml` (RED), fails after; scratch `unsafe {}` fails `cargo check` in `gafira-geom` and `gafira-cli`; V11 fails before the `forbid` attributes exist; revert the scratch code. V12 is run here too, but it is **trivial in C1** (one copy only), so it is not counted as a RED. | `clippy.toml`, `#![forbid]` in both crates |
| T8 | deny + CI | No automated RED in the project. `ci.yml` is accepted through D16: checklist (c) before the commit, then (a) and (b) on the first push. Under policy (a), a draft with `PIN-SHA` cannot run, which is the observable failure until the pins are resolved. `deny.toml` is verified by V8 once cargo-deny is installed. | `deny.toml`, `ci.yml` |

**Task order by slice**:
- Slice A: T1, T2, the cli parts of T7, then T8.
- Slice B: T3–T6, the geom parts of T7 (including V5 and V12), and the B edits to `ci.yml`, reviewed with checklist (c).

Commit `proptest-regressions/` files if any are produced. Layer rules are checked in CI by V5, V6, V11, V12 (interim shell commands) and by clippy with the per-crate `clippy.toml`. They migrate to `xtask check-layers` (ARQ-01) in a later change.

## Verification Plan

| ID | Criterion | Command | Today? |
|---|---|---|---|
| V1 | Tests | `cargo test --workspace` | Yes (needs crates.io fetch once) |
| V2 | Lint, format, build | `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --check`; `cargo check --workspace --all-targets` | Yes |
| V3 | Inherited policy | `cargo metadata --no-deps --format-version 1` → `rg` both members show `"edition":"2024"`, `"rust_version":"1.93"`, `"publish":[]` | Yes |
| V4 | Toolchain | `rustc --version` reports 1.93.1 | Yes |
| V5 | Geom is a leaf | `cargo tree -p gafira-geom -e normal,build --target all --prefix none -f '{p}'` with grep: no `gafira-` line other than `gafira-geom`, no `egui\|eframe\|winit\|wgpu\|tokio\|mio\|socket2\|reqwest\|ureq` | Yes |
| V6 | ARQ-07 | `cargo tree -p gafira-cli --no-default-features -e normal,build --target all` with no `gafira-ui\|eframe\|winit\|wgpu` | Yes |
| V7 | CI file | D16: (a) `gh api repos/GustavoGutierrez/gafira/actions/permissions` reports `"sha_pinning_required": true`; (b) the first CI run is green on the three systems; (c) the PR checklist is ticked; (d) optional `actionlint` | (a) after the holder enables it; (b) after the first push; (c) yes |
| V8 | License gate | `cargo deny check licenses bans sources`; scratch git dependency fails `sources` | **No locally**: cargo-deny is not installed. Runs in CI through the action. |
| V9 | Advisories, CI run, ARQ-80 merge gate | Advisories and the CI run: the `policy` and `test` jobs on GitHub. ARQ-80 (green CI as a merge condition) also needs a branch-protection rule on `main`. | After the first push of slice A; ARQ-80 becomes verifiable once the first CI run exists |
| V10 | Line budget | `git add --intent-to-add .` then `git diff --stat -- . ':!Cargo.lock'`. The repository has no commits yet, so for later slices use `git diff --stat <base>...HEAD -- . ':!Cargo.lock'`. | Yes |
| V11 | `forbid` in every C1 member | For each existing file among `crates/*/src/lib.rs` and `apps/*/src/main.rs`, `rg -q '^#!\[forbid\(unsafe_code\)\]'` must succeed. Fail if any file misses it. An empty glob (slice A has no `crates/`) passes. | Yes |
| V12 | `clippy.toml` copies in sync (slice B) | For each `crates/*/clippy.toml` other than the canonical one, `diff -q crates/gafira-geom/clippy.toml <copy>` must exit 0 | Yes (trivial in C1) |

V5, V6, V11 and V12 are interim shell commands. They migrate to `xtask check-layers` (ARQ-01) in a later change.

**Unverifiable at design time (verify during apply)**:

| Item | How to verify |
|---|---|
| kurbo `0.13` exists, has a `serde` feature, and supports MSRV ≤ 1.93 | `cargo add`/`cargo check` resolves it. Read `rust-version` in `~/.cargo/registry/src/*/kurbo-*/Cargo.toml`. Use the newest compatible minor version. |
| cargo-deny keys (`[graph] targets`, `unused-allowed-license`, `allow-wildcard-paths`, advisories v2 keys) | `cargo deny --version`, then `cargo deny check` has no config errors. Check against the docs for that version. |
| Every `clippy.toml` path resolves in std 1.93 | `cargo clippy -p gafira-geom` emits no "does not refer to an existing" config warnings. The scratch `std::fs::read` RED (T7) fails as expected. |
| proptest `default-features = false, features = ["std"]` is a valid and enough set | `cargo tree -p gafira-geom -e dev` shows no `rusty-fork` or `tempfile`. The T6 tests compile and run. |
| Action tags and SHAs | `git ls-remote` as described above. Then V7 (a)–(c). |
| `members = ["crates/*", "apps/*"]` accepts a glob that matches nothing (slice A has no `crates/`) | `cargo metadata --no-deps` succeeds in slice A. If it does not, list `apps/gafira-cli` explicitly in A and switch to globs in B. |

## Threat Matrix

N/A. The change has no routing, no shell, no subprocess in shipped code, no VCS or PR automation, and no executable-file classification. All five rows are N/A for that reason. The CLI only reads argv, and its tests spawn the binary they build only as a test harness. The CI workflow runs checks and does not push, commit or open PRs.

## Platforms

- **Windows**: `CARGO_BIN_EXE_*` adds `.exe`. `split-debuginfo = "packed"` produces a PDB. The workflow uses bash, so steps behave the same on all three runners.
- **macOS**: `macos-14` is arm64, so it builds `aarch64-apple-darwin`. Packed debug info produces a dSYM.
- **Linux**: `ubuntu-24.04` runs fmt, purity checks and deny once.
- **All three**: the first real run (D16b) is the proof that the matrix executes on all three platforms.
- No code path depends on the platform.

## Relation to C2 / C3

- **C2 `domain-color-doc`**:
  - Inherits the workspace lints and profiles.
  - Each new domain crate needs its own copy of `clippy.toml` (D5). Decide in C2 whether to keep copies or move to `crates/domain/`.
  - `PageSize` should be built on `Length`. The project DPI stays a raw `f64`, or C2 adds a `Dpi` newtype (D7).
  - The 1–16 384 px limits (FD-01c) are document invariants, not part of `gafira-geom`.
  - The ElementBase placement (x, y, w, h, rotation_deg, skew, flip) is a **new type** that produces a `Transform2D` (D9).
  - `serde_json` with `float_roundtrip` and ids as `u64` are already recorded.
  - `validate <path>` decides the parser choice (D13).
- **C3 `edit-history`**: reuses the `proptest` workspace dependency (D15). Equality-based undo properties must take into account that `PartialEq` on f64 is IEEE (NaN ≠ NaN, `-0.0 == 0.0`), and compare with `to_bits()` where identity matters (see **f64 policy** under Interfaces / Contracts).

## Migration / Rollout

No migration required. Every path is new, and rollback means deleting them.

## Risks

| Risk | Mitigation |
|---|---|
| Total ~911 lines is over the 800 budget | Two deliveries (A ~282, B ~629). The guard decides after `sdd-tasks`. |
| Matrix and gates are checked only by human review, with no automated structural test | Checklist (c) on every PR touching `ci.yml`. The first real run (b) catches an invalid workflow or a missing platform. Automating it is left to the future `xtask`. |
| The SHA-pinning policy is not enabled yet (`sha_pinning_required: false`) | Holder pre-task before the first push, checked by V7(a). Without it, an unpinned action would run silently. |
| `PIN-SHA` placeholders need network | Resolve with `git ls-remote` during apply. Under policy (a), the workflow cannot run while any placeholder remains. |
| Repository is empty; the first push publishes the first commit | Push slice A only after the local checks (V1–V6, V10, V11) and checklist (c) pass. |
| `deny` relaxable by `#[allow]` | `#![forbid(unsafe_code)]` in every C1 member, enforced by V11 |
| `clippy.toml` copies drift | V12 `diff` (from slice B), until `xtask check-layers` |
| cargo-deny not installed locally | V8 runs in CI through the action. Installing it locally is optional (open question). |
| Unverified kurbo 0.13, cargo-deny keys, clippy paths, proptest features | "Unverifiable at design time" table |

## Open Questions

- [ ] Accept MSRV 1.93, or install 1.95 first (rustup needs network)?
- [ ] May apply install `cargo-deny` (`cargo install cargo-deny --locked`) and fetch crates.io and GitHub refs (needed for kurbo, serde, proptest and the SHA pins)?
- [ ] Holder pre-task: enable "Require actions to be pinned to a full-length commit SHA" on `GustavoGutierrez/gafira` (today `sha_pinning_required` is false).
- [ ] Add a branch-protection rule on `main` requiring the CI jobs, so that ARQ-80 holds after the first run?
- [ ] The recommendation is two deliveries (slice A ~282 lines, slice B ~629; ~911 total). The alternative is one delivery under a size exception. The review workload guard decides after `sdd-tasks`. `Cargo.lock` is not counted.
- [ ] Transform pivot and degree-based rotation are left to C2. Confirm that `rotation(radians)` in C1 is acceptable.
