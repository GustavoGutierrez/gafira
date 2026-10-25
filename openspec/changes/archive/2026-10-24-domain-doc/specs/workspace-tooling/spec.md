# Delta for Workspace Tooling

> **Base spec note.** This delta applies to requirements of two changes that are merged but NOT archived, so no `openspec/specs/workspace-tooling/spec.md` exists yet:
> - `workspace-scaffold` (C1): full spec at `openspec/changes/workspace-scaffold/specs/workspace-tooling/spec.md`. The MODIFIED block "Formatting and per-crate Clippy policy" below replaces the requirement of that name there.
> - `domain-color` (C2a): its delta at `openspec/changes/domain-color/specs/workspace-tooling/spec.md` ADDED "Per-crate allowed-dependency table (V5)" and "Policy checks cover every domain crate (V11, V12)". The MODIFIED blocks of those two names below replace them as they stand after C1 and C2a are archived, in order: C1, `domain-color`, then this change.
>
> Requirement blocks not named here are unchanged. The complete specs of both changes remain in their change folders until archived.

Scenario type tags: (unit), (integration), (command), (manual). "Dn": delivery per the design table (D1 for V5, V12 and the `clippy.toml` ban; D5b for V13).

## ADDED Requirements

### Requirement: V13 EditToken step (D5b)

The `policy` job MUST contain step V13, which fails when `EditToken` or the hidden constructor's identifier appears in Rust sources outside `gafira-doc`, `gafira-edit` and `gafira-testkit`, with a positive control, `set -euo pipefail`, explicit `grep` exit-code handling and a re-export check for `gafira-edit`. The full rule, what Rust does and does not guarantee, the known textual limits (two-step alias, public function or wrapper handing out the token, macro-generated re-export, `;` inside comments or strings, type inference, crate or module re-export such as `pub use gafira_doc as d;`) and the negative runs E0 to E15 (including the multi-line, `pub type` and glob re-export cases, which V13 does catch) are specified in `edit-capability` (requirement "V13 CI step") and are not repeated here; this requirement makes the step part of the tooling contract. V13 is evidence for ARQ-30 only together with the guarantees table and review; `trybuild` arrives in C3, whose fixtures that name `EditToken` must live under an allowed path or extend the allowlist.

#### Scenario: Step present and green (command)
- GIVEN `.github/workflows/ci.yml` and the final tree
- WHEN the `policy` job runs on GitHub, or the step body runs from a scratch copy under `bash --noprofile --norc -eo pipefail`
- THEN V13 exists, exits 0, and its positive control found the definition

#### Scenario: Each commit stays green (command)
- GIVEN every commit of D5b
- WHEN `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace --locked`, `cargo deny check`, V5, V6, V11, V12 and V13 run
- THEN all pass on each commit

## MODIFIED Requirements

### Requirement: Per-crate allowed-dependency table (V5)

CI step V5 MUST be driven by one table of allowed internal dependencies per domain crate, not by a step hard-coded for `gafira-geom`. The table in this change:

| Crate | Allowed `gafira-*` dependencies |
|---|---|
| `gafira-geom` | none (`-`) |
| `gafira-color` | none (`-`) |
| `gafira-doc` | `gafira-geom`, `gafira-color` (and no other `gafira-*` crate) |

(Previously: two rows, `gafira-geom` and `gafira-color`; `gafira-doc` had no row.) The `gafira-doc` row MUST land in the same commit as the crate's `clippy.toml`, because a `clippy.toml` without a row fails V5.

For each table row, V5 MUST run `cargo tree -p <crate> -e normal,build --target all --prefix none -f '{p}'` with standard input redirected from `/dev/null` (so `cargo` cannot consume the table), capture its output before any filtering, and fail when:
- `cargo tree` fails, or the captured tree has no line starting with `<crate> v` (empty tree);
- the tree contains a `gafira-*` crate other than the crate itself and its allowed list;
- the tree contains a line whose crate name STARTS WITH any of `egui`, `eframe`, `winit`, `wgpu`, `tokio`, `mio`, `socket2`, `reqwest`, `ureq`. Main's V5 already uses this prefix match (`^($forbidden)`, `.github/workflows/ci.yml:98`), with no trailing space, so derived names such as `wgpu-core`, `tokio-util` or `winit-x` are also detected. The prefix match MUST be kept and MUST NOT be weakened (for example by a trailing-space anchor or an exact-name match).

The table MUST be iterated in the main shell, fed by a here-string, so the failure status survives the loop; the step MUST run correctly under `bash -eo pipefail` and MUST NOT pass because a `grep` found no match. V5 MUST also fail when a crate that has a `clippy.toml` under `crates/*/` (the domain-crate marker, shared with V12) has no table row. Adding a crate or an allowed edge MUST be an explicit edit of the table; the step body stays unchanged.

Known limitation: a crate under `crates/` without a `clippy.toml` is not recognized as a domain crate by V5's coverage check and is therefore not required to have a row. V11 still requires `#![forbid(unsafe_code)]` in its `src/lib.rs`, but neither the layer table nor V12 constrains it; closing this gap is deferred to `xtask check-layers` (ARQ-01).

#### Scenario: Clean tree passes (N0 / D-N0, command)
- GIVEN the final tree with `gafira-geom`, `gafira-color` and `gafira-doc`
- WHEN V5 runs from a scratch copy under `bash --noprofile --norc -eo pipefail` with xtrace
- THEN it exits 0 and the trace shows `cargo tree -p gafira-doc` ran

#### Scenario: Forbidden internal edge fails (N1, manual negative test)
- GIVEN `gafira-geom.workspace = true` temporarily added to `gafira-color`'s `[dependencies]`
- WHEN V5 runs
- THEN it exits 1 with "gafira-color must not depend on gafira-geom"; after `git checkout -- crates/gafira-color/Cargo.toml Cargo.lock` V5 exits 0

#### Scenario: Misspelled row fails closed (N2, manual negative test)
- GIVEN the table row `gafira-colour -` instead of `gafira-color -`
- WHEN V5 runs
- THEN `cargo tree` fails ("did not match any packages"), the step exits non-zero, and the coverage check reports that `gafira-color` has no row

#### Scenario: Missing row fails (N3 / D-N3, manual negative test)
- GIVEN the `gafira-color` row deleted, and separately the `gafira-doc` row deleted
- WHEN V5 runs
- THEN it exits 1 with "gafira-color has clippy.toml but no V5 row" and "gafira-doc has clippy.toml but no V5 row" respectively

#### Scenario: Forbidden third-party crate fails (N4, manual negative test)
- GIVEN `serde` temporarily added to the `forbidden` pattern (no network needed)
- WHEN V5 runs
- THEN it exits 1 reporting the forbidden `serde v...` line

#### Scenario: Prefixed forbidden crate names fail (N5, manual negative test)
- GIVEN `wgpu-core` (or `tokio-util`) temporarily added as a dependency of `gafira-color`, or the pattern exercised against a captured tree line `wgpu-core v0.1.0`
- WHEN V5 runs
- THEN it exits 1 reporting the forbidden line, although its name is not exactly `wgpu` or `tokio`

#### Scenario: Crate without clippy.toml is not checked (known limitation, manual)
- GIVEN a scratch crate `crates/scratch/` with a valid manifest, `#![forbid(unsafe_code)]`, no `clippy.toml` and no table row
- WHEN V5 runs
- THEN V5 does NOT fail on it; this documents a known limitation that is not met by this change and is recorded in the verify report

#### Scenario: Doc may depend on geom and color (command)
- GIVEN the final tree
- WHEN `cargo tree -p gafira-doc -e normal,build --target all --prefix none -f '{p}'` is filtered for `gafira-*`
- THEN the lines are exactly the set `gafira-doc`, `gafira-geom`, `gafira-color` (order not significant: `cargo tree` lists `gafira-color` before `gafira-geom`), direct dependencies are `gafira-geom`, `gafira-color`, `serde`, `serde_json`, and V5 exits 0

#### Scenario: Shrinking the doc row fails (D-N1, manual negative test)
- GIVEN the row reduced to `gafira-doc gafira-geom` (without `gafira-color`)
- WHEN V5 runs
- THEN it exits 1 with "gafira-doc must not depend on gafira-color"; this is the evidence that the row constrains the edge

#### Scenario: Forced edge to gafira-cli (D-N2, manual negative test, verified with a warning)
- GIVEN `gafira-cli = { path = "../../apps/gafira-cli" }` temporarily added to `gafira-doc`'s `[dependencies]`
- WHEN V5 runs
- THEN Cargo warns "ignoring invalid dependency ... which is missing a lib target" but `cargo tree` still lists the edge, so V5 exits 1 with "gafira-doc must not depend on gafira-cli"; reverted afterwards. D-N1 remains the primary negative for the row

### Requirement: Policy checks cover every domain crate (V11, V12)

V11 (`#![forbid(unsafe_code)]` in `crates/*/src/lib.rs`) MUST cover `gafira-color` and `gafira-doc` through its existing `crates/*` wildcard without per-crate edits. V12 MUST run under `set -euo pipefail` and MUST: (1) fail unless the canonical copy `crates/gafira-geom/clippy.toml` contains, inside the `disallowed-types` array, an uncommented entry anchored with `^[[:space:]]*\{ path = "std::collections::HashMap"` and the same for `HashSet` (a positive control, so removing the ban consistently from every copy, commenting it out, or moving it under `disallowed-methods` cannot pass); (2) fail when any `crates/*/clippy.toml` differs from the canonical copy; (3) count the copies and fail when fewer than three exist (so an empty or shrunken glob cannot pass). The hardened V12 (all three checks) becomes active in the commit that adds the `gafira-doc` copy of `clippy.toml`, not before: with only two copies the count check would fail the earlier commits. The commit that adds the ban to the `gafira-geom` and `gafira-color` copies comes first and keeps the previous V12 body. `crates/gafira-color/clippy.toml` and `crates/gafira-doc/clippy.toml` MUST be byte-identical to the canonical copy and MUST never be edited by hand (they are produced with `cp`).
(Previously: V12 only diffed the copies against the canonical one, named `gafira-color` only, and ran without `set -euo pipefail`, positive control or copy count.)

#### Scenario: Copies identical (command)
- GIVEN the final tree
- WHEN `diff -q crates/gafira-geom/clippy.toml crates/gafira-color/clippy.toml` and the same for `crates/gafira-doc/clippy.toml` run
- THEN both exit 0

#### Scenario: Drifted copy fails V12 (H-N2, manual negative test)
- GIVEN a scratch extra line appended to `crates/gafira-doc/clippy.toml`, and separately to `crates/gafira-color/clippy.toml`
- WHEN V12 runs
- THEN it exits non-zero in both cases; reverted afterwards

#### Scenario: Ban removed consistently fails V12 (H-N3, manual negative test)
- GIVEN both ban lines removed from all three copies
- WHEN V12 runs
- THEN it exits 1 with "lacks the std::collections::HashMap ban"; the previous V12 exits 0 on the same tree (the RED)

#### Scenario: Commented-out ban fails V12 (H-N5, manual negative test)
- GIVEN the `HashMap` line turned into a comment (`# { path = "std::collections::HashMap", ... },`) in all three copies
- WHEN V12 runs
- THEN it exits 1 with "lacks the std::collections::HashMap ban"

#### Scenario: Ban under the wrong key fails V12 (H-N6, manual negative test)
- GIVEN the `HashMap` and `HashSet` entries moved from `disallowed-types` into `disallowed-methods` in all three copies
- WHEN V12 runs
- THEN it exits 1 with the same message

#### Scenario: Hardened V12 timing (command)
- GIVEN the commit history of D1
- WHEN V12 runs on the commit that adds the ban to the geom and color copies, and on the commit that adds the `gafira-doc` copy
- THEN it passes on both (the first with the previous body, the second with the hardened body and three copies)

#### Scenario: Too few copies fails V12 (H-N4, manual negative test)
- GIVEN the `gafira-doc` and `gafira-color` copies removed (only the canonical one left)
- WHEN V12 runs
- THEN it exits 1 with "expected >= 3"

#### Scenario: Missing forbid fails V11 (manual negative test)
- GIVEN `#![forbid(unsafe_code)]` temporarily removed from `crates/gafira-doc/src/lib.rs`, and separately from `crates/gafira-color/src/lib.rs`
- WHEN V11 runs
- THEN it exits non-zero naming that file; reverted afterwards

#### Scenario: Workspace membership (command)
- GIVEN the final tree
- WHEN `cargo metadata --no-deps` is parsed
- THEN `gafira-color` and `gafira-doc` are members with edition 2024, rust-version 1.93 and `publish` empty, and the root `[workspace.dependencies]` declares `gafira-doc` by path

### Requirement: Formatting and per-crate Clippy policy

`rustfmt.toml` MUST exist. Each domain crate MUST have a `clippy.toml` whose `disallowed-methods`/`disallowed-types` cover `std::fs`, `std::net`, `std::thread::spawn`, `std::time::SystemTime::now` and `std::env` (ARQ-02). Every `clippy.toml` copy (`gafira-geom`, `gafira-color`, `gafira-doc`) MUST additionally list `std::collections::HashMap` and `std::collections::HashSet` in `disallowed-types`, with the reasons "ARQ-10: unstable iteration order; use BTreeMap" and "ARQ-10: unstable iteration order; use BTreeSet". The copies MUST stay byte-identical (V12). The ban applies to all targets (`--all-targets`), including tests. `gafira-geom` and `gafira-color` MUST still pass clippy after the change with no code edit (neither uses the types today). The only ordering constraint is that the commit adding the `HashMap`/`HashSet` ban to the `gafira-geom` and `gafira-color` copies precedes the `gafira-doc` skeleton commit; the hardened V12 is activated later, together with the `gafira-doc` copy (see V12 above).
(Previously: only I/O, network, thread, clock and environment entries; no collection types.)

#### Scenario: Disallowed I/O flagged (integration)
- GIVEN a scratch call to `std::fs::read` in `gafira-geom`
- WHEN `cargo clippy -p gafira-geom -- -D warnings` runs
- THEN it fails citing `disallowed_methods`

#### Scenario: HashMap and HashSet flagged in every domain crate (H-N1, integration)
- GIVEN a scratch `let _m: std::collections::HashMap<u8, u8> = Default::default();` and a scratch `std::collections::HashSet<u8>` binding, in turn in `gafira-geom`, `gafira-color` and `gafira-doc`
- WHEN `cargo clippy -p <crate> --all-targets -- -D warnings` runs
- THEN each fails citing `disallowed_types` and ARQ-10, and the run shows no "does not refer to an existing" configuration warning

#### Scenario: BTreeMap allowed (integration)
- GIVEN a scratch `std::collections::BTreeMap` use in `gafira-doc`
- WHEN clippy runs with `-D warnings`
- THEN it passes

#### Scenario: Entries present in all copies (command)
- GIVEN the three `clippy.toml` files
- WHEN each is searched for `path = "std::collections::HashMap"` and `path = "std::collections::HashSet"`
- THEN every file contains both, and V12 exits 0

#### Scenario: Clean tree (command)
- GIVEN the final tree
- WHEN `cargo fmt --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` run
- THEN both exit 0
