# Workspace Tooling Specification (delta for `domain-time`)

## Purpose

`domain-time` D0 added the layer-0 crate `gafira-time` to the CI policy: a V5 table row with no allowed `gafira-*` dependency and a byte-identical copy of the canonical domain `clippy.toml`. This delta records that wiring as its own requirement. It deliberately does not modify the shared requirements "Per-crate allowed-dependency table (V5)", "Formatting and per-crate Clippy policy" or "Policy checks cover every domain crate (V11, V12)": the change `edit-history` modified those, and the main spec it archived (`openspec/specs/workspace-tooling/spec.md`) already lists the `gafira-time` row and copy as `.github/workflows/ci.yml` has them. With no requirement in common, archiving this delta only appends the requirement below. V14 (property test per core command) does not apply: `gafira-time` defines no `Command`. Requirements not listed here are unchanged. Scenario types: unit, property, integration, command, manual.

## ADDED Requirements

### Requirement: gafira-time domain crate wiring (V5 row, clippy.toml copy) (ARQ-01, ARQ-02, ARQ-10) (D0)

The V5 table in `.github/workflows/ci.yml` MUST contain a row whose only fields are `gafira-time` and `-` (fields are whitespace-separated; the file writes it as `gafira-time  -`), so `gafira-time` may depend on no other `gafira-*` crate and V5 checks its normal and build tree for the forbidden UI, async and network prefixes. `crates/gafira-time/clippy.toml` MUST exist, MUST be byte-identical to the canonical copy `crates/gafira-geom/clippy.toml` (produced with `cp`, never edited by hand), and so carries the ARQ-02 and ARQ-10 bans. The row and the copy MUST stay together: a copy without a row fails V5's reverse check, and the copy is one of those V12 requires (`workspace-tooling`, "Policy checks cover every domain crate (V11, V12)"). `crates/gafira-time/src/lib.rs` MUST contain a line that starts with the crate attribute `#![forbid(unsafe_code)]`; the `//!` crate documentation may precede it. V11 checks this through its `crates/*/src/lib.rs` wildcard by matching each line of the file against `^#!\[forbid\(unsafe_code\)\]`.

#### Scenario: Row and copy present (D0, command)
- GIVEN the final tree
- WHEN the V5 table in `ci.yml` is read and `diff -q crates/gafira-geom/clippy.toml crates/gafira-time/clippy.toml` runs
- THEN the table has a row with the fields `gafira-time` and `-`, `diff` exits 0, and V5, V11 and V12 exit 0

#### Scenario: gafira-time takes no internal edge (D0, command)
- GIVEN the final tree
- WHEN `cargo tree -p gafira-time -e normal,build --target all --prefix none -f '{p}'` is filtered with `grep -Eo '^gafira-[a-z0-9-]+' | sort -u`, as V5 does
- THEN the only name printed is `gafira-time` itself, and V5 exits 0

#### Scenario: Missing forbid fails V11 (D0, manual negative test)
- GIVEN a scratch copy of the final tree with the line `#![forbid(unsafe_code)]` removed from `crates/gafira-time/src/lib.rs`
- WHEN V11 runs
- THEN it exits 1 with "missing forbid(unsafe_code): crates/gafira-time/src/lib.rs"

#### Scenario: Copy without a row fails V5 (D0, manual negative test)
- GIVEN a scratch copy of the final tree with the `gafira-time` row removed from the V5 table
- WHEN V5 runs
- THEN it exits 1 with "gafira-time has clippy.toml but no V5 row"

#### Scenario: Forbidden edge fails V5 (D0, manual negative test)
- GIVEN a scratch copy of the final tree with `gafira-geom.workspace = true` added to the `[dependencies]` of `crates/gafira-time/Cargo.toml`
- WHEN V5 runs
- THEN it exits 1 with "gafira-time must not depend on gafira-geom"
