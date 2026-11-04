## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer 3 scaffold of two new crates whose shared-file changes (root Cargo.toml, CI V5 rows, lockfile, a public `ports` trait change) land once, so the render and media streams can run beside each other afterwards
Product IDs: ARQ-01, ARQ-02, ARQ-03, ARQ-06, ARQ-44 (partial), ARQ-70, ARQ-71, ARQ-107 (source: specs/02-arquitectura-software.md). ARQ-05 was dropped from this card on 2026-11-03 by `docs(layer-3-scaffold): slice card amend`: its only stated mechanism, `cargo public-api` in CI, does not exist in the repository and names `gafira-doc`, `gafira-ports` and `gafira-engine`, none of which this delivery touches, so the property is vacuous for two empty crates (verify round 1, W2).
Scope: `SC1` of `specs/changes/layer-3/design.md`: the `gafira-render` and `gafira-media` crates with their `clippy.toml` copies, the root `Cargo.toml` entries, the CI V5 rows, `Cargo.lock`, the `ports::TileCache` budget calls, and the testkit `ports` feature with its double stubs.      Non-goals: any production logic in either crate, the visitor contract (`DC1`), the render graph and planner, the media pipeline, `gafira-render-cpu`, the timeline model (`O1`), and the persistence of the keyframe index (`SP13`).
Acceptance:
  AC1 Given the two new crates and their V5 rows, When the V5 step runs, Then it exits 0, and a row without a crate's dependency exits 1 [command]
  AC2 Given each new `clippy.toml`, When V12 runs, Then it is byte-identical to `crates/gafira-geom/clippy.toml` [command]
  AC3 Given each new `lib.rs`, When the policy step runs, Then it has `#![forbid(unsafe_code)]` [command]
  AC4 Given the ARQ-107 step, When it runs on the clean tree, Then it exits 0, and with `icu_plurals` added to one of the two crates it exits 1 [command]
  AC5 Given `deny.toml`, When `cargo deny check` runs, Then it exits 0 [command]
  AC6 Given `ports::TileCache`, When the trait is read, Then it declares `bytes(&self) -> Bytes` and `trim(&mut self, budget: Bytes)`, and every implementor in the workspace compiles [unit]
  AC7 Given the testkit, When it is built with the `ports` feature off, Then its default tree contains neither `gafira-ports` nor `gafira-raster`, and with the feature on, the port doubles are available to a dev-dependency [command]
  AC8 Given the branch, When CI is dispatched, Then all jobs pass, new dependencies compiling on Windows and macOS [command]
Plan: ADR 0013 and ADR 0015 in the commits they require. The two crates and their V5 rows, their `clippy.toml` copies, the root `Cargo.toml` entries and the lockfile land together, so no later delivery edits `ci.yml`, the root manifest or the lockfile. The `ports::TileCache` change lands in `crates/gafira-ports/src/render.rs` in its own commit, with the testkit `ports` feature and its stubs in another. The V5 rows list the dependency set of the whole cut, not only of `SC1`, so they stay final: `gafira-render` gets `gafira-ports gafira-doc gafira-geom gafira-color gafira-raster gafira-time gafira-anim` and `gafira-media` gets `gafira-ports gafira-doc gafira-raster gafira-time gafira-geom gafira-color`. The V5 step only checks that the actual internal dependencies are a subset of the row, so a listed dependency that the crate does not use yet is not an error.
Owner decisions touched: none (checked D-01 to D-57: the scaffold adds no production logic and no data, licence or project-format change; the `ARQ-44` budget calls were fixed by `specifier` under `O2`).

# Apply progress: layer-3-scaffold

## SC1 (tdd-apply)

Commits:

- `14409a6` feat(layer-3): scaffold the gafira-render and gafira-media crates
- `efc84c0` feat(gafira-ports): add the budget calls to TileCache
- `d5caa7b` feat(gafira-testkit): add the ports feature and the double stubs

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step), each by extracting its `run:` from `ci.yml`. The third commit adds two more gates, because the `ports` feature is off by default: `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` and `cargo test -p gafira-testkit --features ports --locked`.

Hand mutants: none. The design lists `SC1` as "None; configuration only", so no mutant list came with the delivery.

### RED evidence

The RED of a scaffold is the CI step that fails for the declared reason before the change that satisfies it, the precedent of the layer-2 scaffold. AC6 and AC7 have a real behaviour RED, and AC6 is the compile error the Slice card allows for that criterion.

| AC | RED | Command and exit | GREEN |
|---|---|---|---|
| AC1 | V5 rows added, crates absent | V5 step: `error: package ID specification 'gafira-render' did not match any packages`, exit 101 | crates added, exit 0 |
| AC2 | rows and crates added, no `clippy.toml` copies | V12 step: `::error::gafira-render is in the V5 table but has no crates/gafira-render/clippy.toml` and the same for `gafira-media`, exit 1 | byte-identical copies of `crates/gafira-geom/clippy.toml` (same md5 `eb4b5378...`), exit 0 |
| AC3 | crates with a `lib.rs` lacking the attribute | forbid step: `missing forbid(unsafe_code): crates/gafira-media/src/lib.rs` and `missing forbid(unsafe_code): crates/gafira-render/src/lib.rs`, exit 1 | `#![forbid(unsafe_code)]`, exit 0 |
| AC4 | no failing state on the clean tree: the ARQ-107 step exists since layer 2 and reads the crate list from the V5 table, so it covers the two new crates with no edit | clean tree exit 0 | the failing state is the negative control below |
| AC5 | no failing state: `cargo deny check` passes with the two new crates and `blake3` (CC0-1.0 OR Apache-2.0, both in the allowlist) | exit 0 | exit 0 |
| AC6 | the trait without the budget calls | `cargo test -p gafira-ports --test render`: `error[E0407]: method 'bytes' is not a member of trait 'TileCache'` and `error[E0407]: method 'trim' is not a member of trait 'TileCache'`, exit 101 | `arq_44_tile_cache_reports_bytes_and_trims_to_a_budget` passes (13 passed, 0 failed) |
| AC7 | the testkit manifest without `[features]` | `cargo test -p gafira-testkit --test ports_feature`: `arq_112_the_ports_feature_brings_the_port_crates_in` FAILED, `cargo tree ["--features", "ports"] failed: error: the package 'gafira-testkit' does not contain this feature: ports`, exit 101 | the three tests of `tests/ports_feature.rs` pass; the third one only with `--features ports` |
| AC8 | the coordinator's `workflow_dispatch` on this branch; not run here | | |

### Negative controls

Run and reverted; the tree was clean afterwards and the V5, V12 and ARQ-107 steps exit 0 again.

- AC1, row lacking a dependency: the `gafira-render` row without `gafira-ports` -> `::error::gafira-render must not depend on gafira-ports`, exit 1.
- AC1, `clippy.toml` without a row: the `gafira-render` row deleted -> `::error::gafira-render has clippy.toml but no V5 row`, exit 1.
- AC4, an ICU crate in the tree: `icu_plurals = "2"` in the dependencies of `gafira-render` -> `::error::gafira-render depends on icu_plurals v2.4.0 (ARQ-107)`, exit 1.
- AC4, the word in the sources: `Locale` in `crates/gafira-render/src/lib.rs` -> `::error::gafira-render mentions Locale (ARQ-107): crates/gafira-render/src/lib.rs:5://! The Locale of a render.`, exit 1. The two new crates are layer 3 but not `gafira-ports`, so the search of the step applies to them.

### Choices and deviations

- The module list of the doubles is declared in `lib.rs`, behind `#[cfg(feature = "ports")]`, as the inline module `ports`; the files of the doubles live in `crates/gafira-testkit/src/ports/`, the path the design names. A later delivery fills the file of its own double; a new double still needs the coordinator to fold the `lib.rs` edit into the next serial delivery.
- The stubs panic. `unimplemented!()` and `todo!()` are denied by the workspace lints, so a stub that does nothing yet says so with a `panic!` and a doc comment. The stubs are never called by a test.
- The optional dependencies of the testkit are declared with `path = "../gafira-ports"` and `path = "../gafira-raster"`, as the delivery asked, and not with `workspace = true` as the other crates do: the path stays local to the manifest that owns the optional edge, and the root manifest keeps only the two new crates and `blake3`.
- The `ports` feature is off by default, so `cargo test --workspace` compiles the testkit without the doubles. The run with the feature (`cargo test -p gafira-testkit --features ports --locked`) and its clippy are extra gates of this delivery.
- The test of the feature reads the tree of the testkit with `cargo tree` as a subprocess, run from the crate directory with `--locked`: no network, no change of the lockfile, and no deadlock inside `cargo test`.
- V5 row-without-crate exits 101, not 1 (AC1 reads "exits 1"): the existing V5 step, which this delivery does not change.
- `Cargo.lock` grew 48 lines: `blake3` with its tree (`arrayvec`, `cc`, `cfg-if`, `constant_time_eq`, `cpufeatures`) and the two new crates.
- The notes of this delivery are a separate `docs` commit, as in the layer-2 scaffold (`60f39cf`), because the notes name the hash of the commit they describe.

Measured: `git diff --shortstat develop...HEAD` -> 24 files changed, 675 insertions(+), 6 deletions(-). Authored lines without `Cargo.lock` and without the two `clippy.toml` copies: about 504 (forecast 200, x1.4 = 280). Over the forecast, under the ~700 ceiling of a full run; the coordinator decides.

## SC1 (fix round 1)

Round that closes the warning W3 of verify round 1 (`implementation`): "No CI job compiles the testkit ports feature, so the nine doubles (~300 lines) are outside the remote gate: run 41836317734 never compiled them. SC1 owns `ci.yml`, so the feature-on job belongs here."

Commit:

- `a18096f` ci(layer-3): compile the testkit ports feature in CI

The change adds one step to the `policy` job of `.github/workflows/ci.yml`, `Testkit ports feature compiles (ADR 0013)`, which runs `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` and `cargo test -p gafira-testkit --features ports --locked`.

### RED evidence

The RED of a CI configuration fix is the state the gate is missing from, and its real output.

| AC | RED | Command and exit | GREEN |
|---|---|---|---|
| AC8 | no job of the workflow compiles the `ports` feature | `grep -n 'features ports' .github/workflows/ci.yml` -> no match, exit 1. The only cargo runs of the workflow were `cargo clippy --workspace --all-targets --locked -- -D warnings` (line 47) and `cargo test --workspace --locked` (line 49), neither with `--features ports`, plus `cargo fmt --check` and the `cargo tree` of the policy steps | the `policy` job carries the step `Testkit ports feature compiles (ADR 0013)`, exit 0 |
| AC7 | the default suite compiles the doubles and their test out | `cargo test -p gafira-testkit --test ports_feature --locked` -> `running 2 tests`, `arq_112_the_port_doubles_are_reachable_with_the_feature_on` absent, exit 0 | `cargo test -p gafira-testkit --features ports --locked` -> `running 3 tests`, `arq_112_the_port_doubles_are_reachable_with_the_feature_on ... ok`, exit 0 |

### Negative control

The signature of a double was broken on purpose: `fn now(&self) -> UnixTime` became `fn now(&self) -> u64` in `crates/gafira-testkit/src/ports/clock.rs`, a signature that does not match the `Clock` trait. The step body was then run as CI runs it.

```
error[E0053]: method `now` has an incompatible type for trait
  --> crates/gafira-testkit/src/ports/clock.rs:14:22
   |
14 |     fn now(&self) -> u64 {
   |                      ^^^ expected `gafira_ports::common::UnixTime`, found `u64`
error: could not compile `gafira-testkit` (lib) due to 2 previous errors
```

Step exit: 101.

The same defect with the default features, which is what the workflow ran before this fix, passed:

```
$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
EXIT: 0
```

That is the substance of W3: the defect was invisible to the remote gate. The change was reverted with `git checkout -- crates/gafira-testkit/src/ports/clock.rs`, the tree was clean apart from `.github/workflows/ci.yml`, and the step exited 0 again.

Hand mutants: none. The delivery carries no mutant list (the design lists `SC1` as "None; configuration only"); the control above is its configuration equivalent.

### Gates

Run in order, each on its own, exit 0:

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`) | 0 |
| CI policy steps of `ci.yml`, each body extracted from its `run:` and run in bash: toolchain install, Format, CLI has no UI dependencies (ARQ-07), Domain crates forbid unsafe code, V5, V12, ARQ-107, V13, V14, and the new `Testkit ports feature compiles (ADR 0013)` | 0 each (10 of 10) |
| `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` | 0 |
| `cargo test -p gafira-testkit --features ports --locked` | 0 |

### Choices and deviations

- The step lives in the `policy` job and not in the `test` matrix: the matrix is the three product targets, and the Slice card's Plan gives `ci.yml` to `SC1`, so the feature-on gate belongs to the delivery that owns the file. The job runs on a single target, so the step checks the host itself (`rustc -vV` must report `x86_64-unknown-linux-gnu`, the Linux entry of the matrix) and states in its comment that the feature is compiled on Linux while the three product targets are covered by the `test` matrix through the default suite, whose tree carries neither `gafira-ports` nor `gafira-raster` (ADR 0013, decision 4). No new matrix was invented.
- The step runs before the `cargo-deny` action, which stays last.
- The `policy` job warms no cache and none was added: the step adds a full build of the testkit and its tree to that job, and correctness comes before the time.
- The `gafira-testkit` dev-dependency of `gafira-render` and `gafira-media` was **not** added here. Neither `crates/gafira-render/Cargo.toml` nor `crates/gafira-media/Cargo.toml` has a `[dev-dependencies]` section, which is the state XP9 records ("the premise is not in the tree"); XP9 also records that the dev-dependency adds two lines to `Cargo.lock`, so it belongs to the delivery that adds it (`RE1`/`ME1`), serial and alone. Out of this round.
- The V5 row of `gafira-testkit` still describes only its default tree (suggestion S1 of verify round 1). Unchanged here: V5 and the ARQ-107 step resolve the graph without `--features`, by design.

Measured: the fix round itself is `git diff --shortstat 02ff7bb..a18096f` -> 1 file changed, 14 insertions(+). The branch at the time of writing is `git diff --shortstat develop...HEAD` -> 27 files changed, 1364 insertions(+), 6 deletions(-).
