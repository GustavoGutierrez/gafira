# Workspace Tooling Specification

## Purpose

Shared workspace policy (edition, MSRV, lints, profiles), license/source gate and CI for all Gafira crates (ARQ-02, ARQ-03, ARQ-70, ARQ-71, ARQ-80). New capability; no existing spec.

## Requirements

### Requirement: Workspace manifest

The root manifest MUST declare resolver 3, edition 2024, `rust-version = "1.93"` (deviation from 02 §2.1's 1.95, accepted pending review), `publish = false`, license `LicenseRef-Gafira-Proprietary`, and internal crates under `[workspace.dependencies]` by path. Every member MUST inherit package fields and lints from the workspace. `rust-toolchain.toml` MUST pin `1.93.1` with `rustfmt` and `clippy` components.

#### Scenario: Members inherit policy (command)
- GIVEN the workspace with `gafira-geom` and `gafira-cli`
- WHEN `cargo metadata --no-deps` is parsed
- THEN both members report edition 2024, rust-version 1.93, and `publish` empty (`[]`)

#### Scenario: Toolchain pin (command)
- GIVEN `rust-toolchain.toml`
- WHEN `rustc --version` runs in the repo root
- THEN it reports 1.93.1

### Requirement: Lints and profiles

`[workspace.lints]` MUST set `unsafe_code = "deny"`; each domain crate MUST additionally carry `#![forbid(unsafe_code)]`. `[profile.release]` and `[profile.dev.package."*"]` MUST match 02 §10.2 (`profile.dist` is out of scope).

#### Scenario: Unsafe rejected (integration)
- GIVEN a temporary `unsafe` block added to `gafira-geom`
- WHEN `cargo check -p gafira-geom` runs
- THEN compilation fails with an unsafe-code error

#### Scenario: Profiles (command)
- GIVEN the root manifest
- WHEN its profile tables are inspected
- THEN release has `opt-level = 3`, `lto = "thin"`, `panic = "unwind"`, and dev dependencies use `opt-level = 2`

### Requirement: Formatting and per-crate Clippy policy

C3 delivery: D1. Scenarios without a delivery tag below belong to it.

`rustfmt.toml` MUST exist. Each domain crate MUST have a `clippy.toml` whose `disallowed-methods`/`disallowed-types` cover `std::fs`, `std::net`, `std::thread::spawn`, `std::time::SystemTime::now` and `std::env` (ARQ-02). Every `clippy.toml` copy (`gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit`; the `gafira-time` copy comes from `domain-time`) MUST additionally list `std::collections::HashMap` and `std::collections::HashSet` in `disallowed-types`, with the reasons "ARQ-10: unstable iteration order; use BTreeMap" and "ARQ-10: unstable iteration order; use BTreeSet". The copies MUST stay byte-identical (V12). The ban applies to all targets (`--all-targets`), including tests. `gafira-geom` and `gafira-color` MUST still pass clippy after the change with no code edit (neither uses the types today). The only ordering constraint is that the commit adding the `HashMap`/`HashSet` ban to the `gafira-geom` and `gafira-color` copies precedes the `gafira-doc` skeleton commit; the hardened V12 is activated later, together with the `gafira-doc` copy (see V12 above). The `gafira-edit` copy (C3 D1) is produced with `cp` from the canonical copy in the crate's skeleton commit, together with its V5 row. `gafira-testkit` has no `clippy.toml`: it is a support crate that will read golden files (`design.md` Q10).

#### Scenario: Disallowed I/O flagged (integration)
- GIVEN a scratch call to `std::fs::read` in `gafira-geom`
- WHEN `cargo clippy -p gafira-geom -- -D warnings` runs
- THEN it fails citing `disallowed_methods`

#### Scenario: HashMap and HashSet flagged in every domain crate (H-N1, integration)
- GIVEN a scratch `let _m: std::collections::HashMap<u8, u8> = Default::default();` and a scratch `std::collections::HashSet<u8>` binding, in turn in `gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc` and `gafira-edit`
- WHEN `cargo clippy -p <crate> --all-targets -- -D warnings` runs
- THEN each fails citing `disallowed_types` and ARQ-10, and the run shows no "does not refer to an existing" configuration warning

#### Scenario: BTreeMap allowed (integration)
- GIVEN a scratch `std::collections::BTreeMap` use in `gafira-doc`
- WHEN clippy runs with `-D warnings`
- THEN it passes

#### Scenario: Entries present in all copies (command)
- GIVEN the five `clippy.toml` files (`gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit`)
- WHEN each is searched for `path = "std::collections::HashMap"` and `path = "std::collections::HashSet"`
- THEN every file contains both, and V12 exits 0

#### Scenario: Clean tree (command)
- GIVEN the final tree
- WHEN `cargo fmt --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` run
- THEN both exit 0

### Requirement: Dependency and license gate

`deny.toml` MUST allow only: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016, BSL-1.0, CC0-1.0, MIT-0, 0BSD; set `confidence-threshold = 0.93`; ban the 02 §9.1 crates; set `unknown-registry = "deny"`, `unknown-git = "deny"`, `allow-git = []`. The private proprietary workspace license MUST be exempted via `[licenses.private] ignore = true` and MUST NOT be added to `allow`. MPL-2.0 and copyleft MUST NOT be allowed. Allowlist amendments for transitive dependencies MUST be justified in writing.

#### Scenario: Offline gate passes (command)
- GIVEN `cargo-deny` installed
- WHEN `cargo deny check licenses bans sources` runs
- THEN it exits 0 and the private workspace crates raise no license error

#### Scenario: Git dependency rejected (command)
- GIVEN a scratch git dependency
- WHEN `cargo deny check sources` runs
- THEN it fails

#### Scenario: Advisories (command)
- GIVEN network access (CI)
- WHEN `cargo deny check` runs
- THEN it exits 0; locally, `advisories` MAY be skipped

### Requirement: Layer purity interim proof

Layer purity MUST be proven by `cargo tree` only, applied to every domain crate through the per-crate allowed-dependency table (V5, see "Per-crate allowed-dependency table (V5)"); the `xtask` crate remains deferred to a later change.

#### Scenario: Domain crates are leaves (command)
- GIVEN the workspace
- WHEN `cargo tree -p gafira-geom` and `cargo tree -p gafira-color` run
- THEN in each, no `gafira-*` crate other than itself, and no I/O, UI or engine crate, appears

### Requirement: CI workflow

`.github/workflows/ci.yml` MUST be a standalone workflow whose matrix contains exactly `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` and `cargo deny check`. Every third-party `uses:` MUST reference a full 40-hex commit SHA. The repository has git with a private `origin` remote, so the workflow is exercised by GitHub itself. Verification in this change is manual or by CI run; no programmatic check of the file exists.

Note: programmatic checks of the CI file and layer rules arrive with the `xtask` crate in a later change (ARQ-01); they are not part of C1.

#### Scenario: Matrix exact (manual / CI run)
- GIVEN the first real CI run on GitHub
- WHEN the run's jobs are listed
- THEN exactly Windows x86_64, Linux x86_64 and macOS aarch64 ran, and the PR review checklist confirms no other target in the matrix

#### Scenario: Actions pinned (manual / CI run)
- GIVEN the repository setting "Require actions to be pinned to a full-length commit SHA"
- WHEN `gh api repos/GustavoGutierrez/gafira/actions/permissions` is queried
- THEN `sha_pinning_required` is `true`, and a workflow using a tag or branch ref is rejected by GitHub

#### Scenario: Four gates present (manual / CI run)
- GIVEN the first CI run on the three systems
- WHEN each job's steps are inspected, with the PR review checklist as cross-check
- THEN fmt, clippy (`-D warnings`), test and deny each executed and passed

#### Scenario: Merge gating (manual)
- GIVEN branch protection on the remote
- WHEN the PR checklist is reviewed
- THEN a green CI on all three systems is recorded as required (ARQ-80); enabling it MAY be deferred and is not an automated test

### Requirement: Per-crate allowed-dependency table (V5)

C3 delivery: D1, D2. Scenarios without a delivery tag below belong to it.

CI step V5 MUST be driven by one table of allowed internal dependencies per domain crate, not by a step hard-coded for `gafira-geom`. The table after this change, in the order `ci.yml` lists it:

| Crate | Allowed `gafira-*` dependencies |
|---|---|
| `gafira-geom` | none (`-`) |
| `gafira-color` | none (`-`) |
| `gafira-time` | none (`-`) (added by `domain-time`) |
| `gafira-doc` | `gafira-geom`, `gafira-color` (and no other `gafira-*` crate) |
| `gafira-edit` | `gafira-geom`, `gafira-color`, `gafira-doc` |
| `gafira-testkit` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit` (no `clippy.toml`; the row constrains it anyway, `design.md` Q10) |

The `gafira-doc` row MUST land in the same commit as the crate's `clippy.toml`, because a `clippy.toml` without a row fails V5; the same holds for the `gafira-edit` row (C3 D1). The `gafira-testkit` row lands with that crate (C3 D2). The `gafira-time` row and its `clippy.toml` copy landed together in `domain-time` D0. Dev-dependencies (`trybuild`, `proptest`, `gafira-testkit`) are outside `-e normal,build` and MUST NOT appear in a row.

For each table row, V5 MUST run `cargo tree -p <crate> -e normal,build --target all --prefix none -f '{p}'` with standard input redirected from `/dev/null` (so `cargo` cannot consume the table), capture its output before any filtering, and fail when:
- `cargo tree` fails, or the captured tree has no line starting with `<crate> v` (empty tree);
- the tree contains a `gafira-*` crate other than the crate itself and its allowed list;
- the tree contains a line whose crate name STARTS WITH any of `egui`, `eframe`, `winit`, `wgpu`, `tokio`, `mio`, `socket2`, `reqwest`, `ureq`. Main's V5 already uses this prefix match (`^($forbidden)`, `.github/workflows/ci.yml:98`), with no trailing space, so derived names such as `wgpu-core`, `tokio-util` or `winit-x` are also detected. The prefix match MUST be kept and MUST NOT be weakened (for example by a trailing-space anchor or an exact-name match).

The table MUST be iterated in the main shell, fed by a here-string, so the failure status survives the loop; the step MUST run correctly under `bash -eo pipefail` and MUST NOT pass because a `grep` found no match. V5 MUST also fail when a crate that has a `clippy.toml` under `crates/*/` (the domain-crate marker, shared with V12) has no table row. Adding a crate or an allowed edge MUST be an explicit edit of the table; the step body stays unchanged. V5 constrains only `gafira-*` edges and the forbidden prefixes above, not other third-party crates: a direct third-party dependency of a domain crate (for example `serde_json` in `gafira-edit`, allowed by `edit-commands` "Crate purity and dependencies") is governed by that crate's purity requirement, `cargo deny check` and review, and never appears in a row.

Known limitation: a crate under `crates/` without a `clippy.toml` is not recognized as a domain crate by V5's coverage check and is therefore not required to have a row. V11 still requires `#![forbid(unsafe_code)]` in its `src/lib.rs`, but neither the layer table nor V12 constrains it; closing this gap is deferred to `xtask check-layers` (ARQ-01).

#### Scenario: Clean tree passes (N0 / D-N0, command)
- GIVEN the final tree with `gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit` and `gafira-testkit`
- WHEN V5 runs from a scratch copy under `bash --noprofile --norc -eo pipefail` with xtrace
- THEN it exits 0 and the trace shows `cargo tree` ran for each of the six rows, `gafira-edit`, `gafira-testkit` and `gafira-time` included

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

#### Scenario: Edit row constrains the edge (E-N1, manual negative test)
- GIVEN the `gafira-edit` row reduced to `gafira-edit gafira-geom gafira-color`
- WHEN V5 runs
- THEN it exits 1 with "gafira-edit must not depend on gafira-doc"; reverted afterwards

#### Scenario: Missing edit row fails (E-N2, manual negative test)
- GIVEN the `gafira-edit` row deleted
- WHEN V5 runs
- THEN it exits 1 with "gafira-edit has clippy.toml but no V5 row"

#### Scenario: Dev-dependencies are not layer edges (command)
- GIVEN the final tree, where `gafira-edit` dev-depends on `trybuild`, `proptest` and `gafira-testkit`
- WHEN `cargo tree -p gafira-edit -e normal,build --target all --prefix none -f '{p}'` runs
- THEN none of them appears, and V5 exits 0

### Requirement: Policy checks cover every domain crate (V11, V12)

C3 delivery: D1, D2, D11. Scenarios without a delivery tag below belong to D1 and D2.

V11 (`#![forbid(unsafe_code)]` in `crates/*/src/lib.rs`) MUST cover `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit` and `gafira-testkit` through its existing `crates/*` wildcard without per-crate edits. V12 MUST run under `set -euo pipefail` and MUST: (1) fail unless the canonical copy `crates/gafira-geom/clippy.toml` contains, inside the `disallowed-types` array, an uncommented entry anchored with `^[[:space:]]*\{ path = "std::collections::HashMap"` and the same for `HashSet` (a positive control, so removing the ban consistently from every copy, commenting it out, or moving it under `disallowed-methods` cannot pass); (2) for every crate in the V5 table except the exempt one, fail when its `crates/<crate>/clippy.toml` differs from the canonical copy (byte comparison), naming the crate; (3) fail, naming the crate, when a crate listed in the V5 table has no `crates/<crate>/clippy.toml`. A `clippy.toml` of a crate outside the table fails V5's reverse check instead. After C3 the table requires five copies: `gafira-geom` (canonical), `gafira-color`, `gafira-time`, `gafira-doc` and `gafira-edit`. The set of required copies MUST be derived from the V5 table, the one list that V5 iterates, so editing that table is enough to change what V12 requires; V12 MUST NOT keep a second list of crates. The only exception is `gafira-testkit`, a support crate without a `clippy.toml` (`design.md` Q10), which V12 MUST name explicitly as exempt. The reverse check stays in V5: a crate that has a `clippy.toml` but no V5 row fails V5 (see "Per-crate allowed-dependency table (V5)"). The fixed count floor of C2b (`>= 3` copies) is removed (owner decision, 2026-10-25, C3 D11): with a floor of three, deleting one copy passed both V12 and V5. History: in C2b the hardened V12 became active in the commit that added the `gafira-doc` copy, after the commit that added the ban to the `gafira-geom` and `gafira-color` copies. `crates/gafira-color/clippy.toml`, `crates/gafira-time/clippy.toml`, `crates/gafira-doc/clippy.toml` and `crates/gafira-edit/clippy.toml` MUST be byte-identical to the canonical copy and MUST never be edited by hand (they are produced with `cp`).

#### Scenario: Copies identical (command)
- GIVEN the final tree
- WHEN `diff -q crates/gafira-geom/clippy.toml crates/gafira-color/clippy.toml` and the same for `crates/gafira-time/clippy.toml`, `crates/gafira-doc/clippy.toml` and `crates/gafira-edit/clippy.toml` run
- THEN all exit 0

#### Scenario: Drifted copy fails V12 (H-N2, manual negative test)
- GIVEN a scratch extra line appended to `crates/gafira-doc/clippy.toml`, and separately to `crates/gafira-color/clippy.toml`
- WHEN V12 runs
- THEN it exits non-zero in both cases; reverted afterwards

#### Scenario: Ban removed consistently fails V12 (H-N3, manual negative test)
- GIVEN both ban lines removed from every copy (three before C3, five after C3)
- WHEN V12 runs
- THEN it exits 1 with "lacks the std::collections::HashMap ban"; the previous V12 exits 0 on the same tree (the RED)

#### Scenario: Commented-out ban fails V12 (H-N5, manual negative test)
- GIVEN the `HashMap` line turned into a comment (`# { path = "std::collections::HashMap", ... },`) in every copy
- WHEN V12 runs
- THEN it exits 1 with "lacks the std::collections::HashMap ban"

#### Scenario: Ban under the wrong key fails V12 (H-N6, manual negative test)
- GIVEN the `HashMap` and `HashSet` entries moved from `disallowed-types` into `disallowed-methods` in every copy
- WHEN V12 runs
- THEN it exits 1 with the same message

#### Scenario: Hardened V12 timing (C2b history, command)
- GIVEN the commit history of D1
- WHEN V12 runs on the commit that adds the ban to the geom and color copies, and on the commit that adds the `gafira-doc` copy
- THEN it passes on both (the first with the previous body, the second with the hardened body and three copies)

#### Scenario: Clean tree passes the derived V12 (D11, command)
- GIVEN the final tree, where `gafira-testkit` has no `clippy.toml`
- WHEN V12 runs from a scratch copy under `bash --noprofile --norc -eo pipefail`
- THEN it exits 0, having required the copies of `gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc` and `gafira-edit` and not one for `gafira-testkit`

#### Scenario: A deleted or edited gafira-time copy fails V12 (D11, manual negative test)
- GIVEN a scratch copy of the final tree with `crates/gafira-time/clippy.toml` removed, and separately with one line appended to it
- WHEN V12 runs
- THEN it exits 1 naming `gafira-time` in both cases

#### Scenario: A deleted copy fails V12 (D11, manual negative test)
- GIVEN a scratch copy of the final tree with `crates/gafira-edit/clippy.toml` removed
- WHEN V12 runs
- THEN it exits 1 with a message naming `gafira-edit`; the V12 before D11 exits 0 on the same tree (the RED)

#### Scenario: An edited copy fails V12 (D11, manual negative test)
- GIVEN a scratch copy of the final tree with one line appended to `crates/gafira-edit/clippy.toml`
- WHEN V12 runs
- THEN it exits 1 with a message naming `gafira-edit`

#### Scenario: A V5 crate without a copy fails V12 (D11, manual negative test)
- GIVEN a scratch crate `crates/gafira-scratch/` (valid manifest, `#![forbid(unsafe_code)]`, workspace member, no `clippy.toml`) and the row `gafira-scratch -` added to the V5 table
- WHEN V12 runs
- THEN it exits 1 with a message naming `gafira-scratch`; the V12 before D11 exits 0 on the same tree (the RED); removed afterwards

#### Scenario: A whitespace-only line does not hide table rows (D11, manual negative test)
- GIVEN a scratch copy of the final tree with a line of only spaces inserted in the V5 table before the `gafira-edit` row, and `crates/gafira-edit/clippy.toml` removed
- WHEN V12 runs
- THEN it exits 1 naming `gafira-edit` (V12 reads the same rows V5 iterates)

#### Scenario: gafira-testkit is the named exception (D11, manual negative test)
- GIVEN the final tree, and separately the `gafira-testkit` exemption removed from the step in a scratch copy
- WHEN V12 runs
- THEN it exits 0 on the final tree, and exits 1 naming `gafira-testkit` without the exemption (so the exemption is what lets it pass)

#### Scenario: A copy without a V5 row still fails V5 (D11, manual negative test)
- GIVEN a scratch copy of the final tree with the `gafira-color` row removed from the V5 table
- WHEN V5 runs
- THEN it exits 1 with "gafira-color has clippy.toml but no V5 row" (the existing reverse check, unchanged)

#### Scenario: Missing forbid fails V11 (manual negative test)
- GIVEN `#![forbid(unsafe_code)]` temporarily removed from `crates/gafira-doc/src/lib.rs`, and separately from `crates/gafira-color/src/lib.rs`
- WHEN V11 runs
- THEN it exits non-zero naming that file; reverted afterwards

#### Scenario: Workspace membership (command)
- GIVEN the final tree
- WHEN `cargo metadata --no-deps` is parsed
- THEN `gafira-color`, `gafira-doc`, `gafira-edit` and `gafira-testkit` are members with edition 2024, rust-version 1.93 and `publish` empty, and the root `[workspace.dependencies]` declares `gafira-doc`, `gafira-edit` and `gafira-testkit` by path

#### Scenario: Missing forbid in the new crates fails V11 (manual negative test)
- GIVEN `#![forbid(unsafe_code)]` temporarily removed from `crates/gafira-edit/src/lib.rs`, and separately from `crates/gafira-testkit/src/lib.rs`
- WHEN V11 runs
- THEN it exits non-zero naming that file; reverted afterwards

### Requirement: V13 EditToken step (D5b)

C3 delivery: D1, D5. Scenarios without a delivery tag below belong to it.

The `policy` job MUST contain step V13, which fails when `EditToken` or the hidden constructor's identifier appears in Rust sources outside `gafira-doc`, `gafira-edit` and `gafira-testkit`, with a positive control, `set -euo pipefail`, explicit `grep` exit-code handling and a re-export check for `gafira-edit`. The full rule, what Rust does and does not guarantee, the known textual limits (two-step alias, public function or wrapper handing out the token, macro-generated re-export, `;` inside comments or strings, type inference, crate or module re-export such as `pub use gafira_doc as d;`) and the negative runs E0 to E15 (including the multi-line, `pub type` and glob re-export cases, which V13 does catch) are specified in `edit-capability` (requirement "V13 CI step") and are not repeated here; this requirement makes the step part of the tooling contract. V13 is evidence for ARQ-30 together with the guarantees table, review and, from C3, the `trybuild` fixtures (`edit-capability`, "trybuild evidence for ARQ-30"). The fixtures live under `crates/gafira-edit/tests/ui/`, an allowed path, so the allowlist is unchanged.

#### Scenario: Step present and green (command)
- GIVEN `.github/workflows/ci.yml` and the final tree
- WHEN the `policy` job runs on GitHub, or the step body runs from a scratch copy under `bash --noprofile --norc -eo pipefail`
- THEN V13 exists, exits 0, and its positive control found the definition

#### Scenario: Each commit stays green (command)
- GIVEN every commit of C2b D5b and of C3
- WHEN `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace --locked`, `cargo deny check`, V5, V6, V11, V12, V13 and (from C3 D10) V14 run
- THEN all pass on each commit

### Requirement: Compile-test dependency (trybuild) (ARQ-30, ARQ-70, ARQ-71) (D5)

`trybuild` MUST be declared once in the root `[workspace.dependencies]` with a version requirement (no git source) and used only as a dev-dependency of `gafira-edit`. The release chosen MUST be at least two weeks old when added. Its whole dependency tree MUST pass `cargo deny check` with the current `deny.toml`; if a transitive license needs an entry, it MUST be a per-crate `[[licenses.exceptions]]` with a written rationale, never a change of the global `allow` list. `Cargo.lock` MUST change only by `trybuild`'s tree and the new workspace crates. The `trybuild` test MUST run inside the ordinary `cargo test --workspace --locked` step of the `test` job (no separate CI job), and MUST NOT need network access beyond the registry download that every build already does (explorer R1).

#### Scenario: Dev-only (D5, command)
- GIVEN the final tree
- WHEN `cargo tree -e normal,build -i trybuild --workspace` runs
- THEN no workspace crate is listed as a dependent of `trybuild` (or cargo reports that `trybuild` is not in the normal/build graph); the scenario judges the printed output, not the exit code, because `cargo tree -i trybuild -e normal,build` exits 0 both when clean ("nothing to print") and when violated (spike R13)

#### Scenario: Lock delta is bounded (D5, command)
- GIVEN `Cargo.lock` before and after D5
- WHEN the added `[[package]]` names are listed
- THEN each is a registry package (none from a git source) that appears in `cargo metadata --format-version 1` as a transitive dependency of `trybuild` or is a new workspace crate, and `cargo deny check` passes; the comparison uses `cargo metadata`, not `cargo tree -p trybuild`, because the lock also gains `equivalent`, `hashbrown` and `indexmap` (through `toml`'s optional `indexmap`), which `cargo tree` does not list (spike R2)

#### Scenario: Deny passes unchanged (D5, command)
- GIVEN `cargo-deny` installed
- WHEN `cargo deny check licenses bans sources` runs
- THEN it exits 0, and `git diff deny.toml` is empty or adds only justified per-crate exceptions

### Requirement: V14 command property coverage (ARQ-81) (D10, Q9)

The `policy` job MUST contain step V14, run under `set -euo pipefail`, that lists the core commands and fails when one of them does not appear in `crates/gafira-edit/tests/prop_commands.rs`. A core command is the type named in an `impl` of `Command` in a Rust file under `crates/gafira-edit/src` (tracked or untracked, listed with a plain directory pathspec as in V13). The step MUST:
- read impl headers as text with line breaks folded, so that a header split over several lines, generic parameters with nested bounds (`impl<T: Into<Vec<u8>>> Command for G<T>`), a path before the trait (`impl crate::Command for Q`) and a trailing `where` clause are all recognized;
- leave out every item under a `#[cfg(test)]` attribute (a module, an impl or any other item), finding its end by counting braces, and leave out lines whose first non-blank characters are `//`;
- leave out the files that a `#[cfg(test)]` item declares (`#[path = "..."] mod x;` names the file, `mod x;` names `x.rs`), matched by file name; a file is not left out because of its name alone (a `*_tests.rs` file that no `#[cfg(test)]` item declares is searched);
- require the named type to be a plain identifier, optionally with generics, for which a `struct`, `enum` or `union` of that name appears in a searched file under `src` (a textual check); any other target (`Box<T>`, `&X`, `crate::X`, a blanket `T`) MUST fail the step with a message naming it;
- count a type as covered when its name appears as a whole word in `prop_commands.rs`, and fail when the property file is missing;
- never treat a failure of `git ls-files`, `awk`, the impl search, the definition search or the coverage search as a clean result: their exit codes are captured, and only exit 1 of `grep` means "no match";
- run a positive control that fails unless at least one implementation is found (so a wrong working directory or a renamed trait cannot pass);
- print the missing types.

Known limits, recorded in the verify report: braces inside strings or comments confuse the `#[cfg(test)]` skip; implementations generated by macros are invisible; the trait imported under an alias (`use crate::Command as Cmd;` then `impl Cmd for X`) hides the impl; a type name that appears in `prop_commands.rs` only in a comment counts as covered; a file whose name equals that of a file a `#[cfg(test)]` item declares elsewhere under `src` is skipped; an impl inside a block comment or a string literal is read as an impl (this fails closed). V14 stands in for the `xtask` check of ARQ-81 until `xtask` exists.

#### Scenario: Clean tree passes (D10, command)
- GIVEN the final tree
- WHEN V14 runs from a scratch copy under `bash --noprofile --norc -eo pipefail`
- THEN it exits 0, and the positive control found the implementations `InsertNodes`, `InsertPage`, `RemoveNodes`, `RemovePage`, `Reorder`, `Reparent`, `SetProperty` and `Transaction`

#### Scenario: Command without a property fails (D10, manual negative test)
- GIVEN a scratch `pub struct Noop; impl Command for Noop { .. }` in `crates/gafira-edit/src`
- WHEN V14 runs
- THEN it exits 1 naming `Noop`; removed afterwards

#### Scenario: Header forms are recognized (D10, manual negative test)
- GIVEN, in turn, a scratch file in `crates/gafira-edit/src` with a struct and an `impl Command` for it written with nested generic bounds, with `crate::Command`, with the header split over three lines, with a `where` clause on its own lines, inside a non-test inline module (indented), and in a file named `fake_tests.rs` that no `#[cfg(test)]` item declares
- WHEN V14 runs on each
- THEN it exits 1 naming the type each time; each file removed afterwards

#### Scenario: A target that is not a local type fails (D10, manual negative test)
- GIVEN a scratch `impl<T: Send> Command for Box<T>` in `crates/gafira-edit/src`
- WHEN V14 runs
- THEN it exits 1 with a message that the type is not defined under `src`; removed afterwards

#### Scenario: Test-only commands are outside the search (D10, manual negative test)
- GIVEN a scratch `#[cfg(test)] mod tests { struct T1; impl Command for T1 { .. } }` in a file under `crates/gafira-edit/src`
- WHEN V14 runs
- THEN it exits 0; removed afterwards

#### Scenario: Every core type is checked (D10, manual negative test)
- GIVEN every core type name changed in `prop_commands.rs`
- WHEN V14 runs
- THEN it exits 1 and the message lists all eight core types; restored afterwards

#### Scenario: Positive control (D10, manual negative test)
- GIVEN V14 run from `crates/`
- WHEN it runs
- THEN it exits 1 with the positive-control message
