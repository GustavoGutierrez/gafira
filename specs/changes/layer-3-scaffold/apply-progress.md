## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer 3 scaffold of two new crates whose shared-file changes (root Cargo.toml, CI V5 rows, lockfile, a public `ports` trait change) land once, so the render and media streams can run beside each other afterwards
Product IDs: ARQ-01, ARQ-02, ARQ-03, ARQ-05, ARQ-06, ARQ-44 (partial), ARQ-70, ARQ-71, ARQ-107 (source: specs/02-arquitectura-software.md)
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

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
