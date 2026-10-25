# Workspace Tooling Specification (delta for `domain-time`)

## Purpose

`domain-time` D0 added the layer-0 crate `gafira-time` to the CI policy: a V5 table row with no allowed `gafira-*` dependency and a byte-identical copy of the canonical domain `clippy.toml`. This delta records that wiring as its own requirement. It deliberately does not modify the shared requirements "Per-crate allowed-dependency table (V5)", "Formatting and per-crate Clippy policy" or "Policy checks cover every domain crate (V11, V12)": the change `edit-history` modifies those, and its versions already list the `gafira-time` row and copy as `.github/workflows/ci.yml` has them. With no requirement in common, the two deltas can be archived in either order and give the same main spec. Requirements not listed here are unchanged. Scenario types: unit, property, integration, command, manual.

## ADDED Requirements

### Requirement: gafira-time domain crate wiring (V5 row, clippy.toml copy) (ARQ-01, ARQ-02, ARQ-10) (D0)

The V5 table in `.github/workflows/ci.yml` MUST contain the row `gafira-time -`, so `gafira-time` may depend on no other `gafira-*` crate and V5 checks its normal and build tree for the forbidden UI, async and network prefixes. `crates/gafira-time/clippy.toml` MUST exist, MUST be byte-identical to the canonical copy `crates/gafira-geom/clippy.toml` (produced with `cp`, never edited by hand), and so carries the ARQ-02 and ARQ-10 bans. The row and the copy MUST stay together: a copy without a row fails V5's reverse check, and the copy is one of those V12 requires (`workspace-tooling`, "Policy checks cover every domain crate (V11, V12)"). `crates/gafira-time/src/lib.rs` MUST start with `#![forbid(unsafe_code)]`, which V11 checks through its `crates/*` wildcard.

#### Scenario: Row and copy present (D0, command)
- GIVEN the final tree
- WHEN the V5 table in `ci.yml` is read and `diff -q crates/gafira-geom/clippy.toml crates/gafira-time/clippy.toml` runs
- THEN the table has the row `gafira-time -`, `diff` exits 0, and V5, V11 and V12 exit 0

#### Scenario: gafira-time takes no internal edge (D0, command)
- GIVEN the final tree
- WHEN `cargo tree -p gafira-time -e normal,build --target all --prefix none -f '{p}'` is filtered for `gafira-*`
- THEN the only line is `gafira-time` itself, and V5 exits 0

#### Scenario: Copy without a row fails V5 (D0, manual negative test)
- GIVEN a scratch copy of the final tree with the `gafira-time` row removed from the V5 table
- WHEN V5 runs
- THEN it exits 1 with "gafira-time has clippy.toml but no V5 row"

#### Scenario: Forbidden edge fails V5 (D0, manual negative test)
- GIVEN a scratch copy of the final tree with `gafira-geom.workspace = true` added to the `[dependencies]` of `crates/gafira-time/Cargo.toml`
- WHEN V5 runs
- THEN it exits 1 with "gafira-time must not depend on gafira-geom"
