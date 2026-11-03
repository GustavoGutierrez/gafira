## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: scaffold of four layer-2 crates under the owner's parallel full route; CI policy and dependency changes
Product IDs: ARQ-01, ARQ-02, ARQ-70 (rows, lints, licences), ARQ-107 (CI step) (source: specs/02-arquitectura-software.md)
Scope: `SC1` of `specs/changes/layer-2/design.md`: P1 table in full, P9, the four V5 rows of ADR 0007, one `workflow_dispatch` on the branch.      Non-goals: any production logic, fonts or `assets/LICENSES.toml` (`TX1`), any crate API.
Acceptance:
  AC1 Given the four new crates, When the V5 step runs, Then it exits 0, and a row without a crate's dependency exits 1 [command]
  AC2 Given each new `clippy.toml`, When V12 runs, Then it is byte-identical to `crates/gafira-geom/clippy.toml` [command]
  AC3 Given each new `lib.rs`, When the policy step runs, Then it has `#![forbid(unsafe_code)]` [command]
  AC4 Given the ARQ-107 step of P9, When it runs on the clean tree, Then it exits 0, and with `icu_plurals` added to a layer 0-2 crate it exits 1 [command]
  AC5 Given `deny.toml` per P1, When `cargo deny check` runs, Then it exits 0 [command]
  AC6 Given the branch, When CI is dispatched, Then all jobs pass, new dependencies compiling on Windows and macOS [command]
Plan: design P1, P2 and P9, in the commits ADR 0007 requires: row, `clippy.toml` and `Cargo.toml` entries together.
Owner decisions touched: none (checked D-20: untouched; the segment crate is scaffolded only).

# Apply progress: layer-2-scaffold

## SC1 (tdd-apply)

Commits: `ba48ef6` deny.toml exception; `7f7e588` four crates, V5 rows, clippy.toml copies, Cargo.toml entries, testkit stubs, Cargo.lock (ADR 0007: one commit); `19bddef` ARQ-107 step (P9).

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step on `19bddef`), each by extracting its `run:` from `ci.yml`. First run of `7f7e588` had `cargo fmt --check` exit 1 (the two empty testkit stubs); fixed with a one-line doc comment each.

### RED evidence (scaffolding: the CI step fails for the stated reason before the change that satisfies it)

| AC | RED | Command and exit | GREEN |
|---|---|---|---|
| AC1 | V5 rows added, crates absent | V5 step: `package ID specification gafira-text did not match any packages`, exit 101 | crates added, exit 0 |
| AC2 | rows added, no copies | V12 step: `gafira-text is in the V5 table but has no crates/gafira-text/clippy.toml` (x4), exit 1 | copies of `gafira-geom/clippy.toml`, exit 0 |
| AC3 | crates with `lib.rs` lacking the attribute | forbid step: `missing forbid(unsafe_code): crates/gafira-{audio,raster,segment,text}/src/lib.rs`, exit 1 | `#![forbid(unsafe_code)]`, exit 0 |
| AC4 | step absent: exit 99 (not found). With the step written and `icu_plurals = "2"` in `gafira-raster`: `gafira-raster depends on icu_plurals v2.4.0` and `gafira-segment depends on icu_plurals v2.4.0`, exit 1 | clean tree: exit 0 (gawk and mawk) | |
| AC5 | no failing state: `cargo deny check licenses` without `unused-license-exception` is exit 0 with a `license-exception-not-encountered` warning | with the key: `cargo deny check` exit 0 | |

AC6 is the coordinator's `workflow_dispatch` on this branch; not run here.

### Negative controls (run, then reverted)

- AC1, row lacking a dependency: `gafira-segment` row without `gafira-raster` -> `::error::gafira-segment must not depend on gafira-raster`, exit 1.
- AC1, `clippy.toml` without a row: `gafira-audio` row deleted -> `::error::gafira-audio has clippy.toml but no V5 row`, exit 1.
- AC1, row without a crate: row `gafira-ghost` -> cargo error, exit 101 (set -e passes cargo's code; non-zero, but not 1).
- AC4: `icu_plurals` in `gafira-raster` -> exit 1 (fails raster and segment, as P9 says). `// Locale` in `gafira-audio/src/lib.rs` -> `gafira-audio mentions Locale`, exit 1. `icu_segmenter` in `gafira-text` -> exit 0 (allowed).

### Choices and deviations

- Dependencies the design leaves implicit: `parley` goes to `gafira-text` (02 §9.2) and `rayon` to `gafira-segment` (layout-core design L8/L9: "rayon in gafira-segment only"); `rayon = "1"`. Neither is stated by P1 for a crate, so the coordinator may check them.
- `assert_no_alloc` is a dev-dependency of `gafira-audio` and `gafira-testkit`, as P1 says. `deny.toml` is its own commit (`ba48ef6`); `unused-license-exception = "allow"` only silences a warning, it does not change an exit code.
- The testkit stubs carry a one-line `//!` comment because an empty file fails `cargo fmt --check`.
- The ARQ-107 step also has a positive control (`^<crate> v` in the tree, `crates/<name>/src` exists), and `Locale` is a substring search over every file of the crate (stricter than a word search).
- `cargo update --workspace` regenerated `Cargo.lock`: 553 added lines, 0 removed or changed.
- V5 row-without-crate exits 101, not 1 (AC1 reads "exits 1"): the existing V5 step, which this delivery does not change.

Measured: `git diff --shortstat 544b503 HEAD` before this note: 20 files, 893 insertions, of which 553 are `Cargo.lock`; about 340 authored lines (forecast 180, x1.4 = 250). Under ~700.
