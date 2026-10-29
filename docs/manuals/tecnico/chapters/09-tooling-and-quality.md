# Tooling and quality gates

## Local gates

Every commit must pass these three commands on its own, not only the last commit of a branch:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

The toolchain is pinned to `1.93.1` with `rustfmt` and `clippy` in `rust-toolchain.toml`. `--locked` keeps `Cargo.lock` authoritative.

## Workspace lints

Defined in the root `Cargo.toml` and inherited by each crate with `[lints] workspace = true`:

| Lint | Level |
|---|---|
| `unsafe_code` (rustc) | `deny`; each crate also declares `#![forbid(unsafe_code)]` |
| `clippy::dbg_macro`, `clippy::todo`, `clippy::unimplemented` | `deny` |

`#[allow]` must not be used to silence lints. Domain crates add a `clippy.toml` with the ARQ-02 and ARQ-10 bans described in the architecture chapter.

## Release profile

`opt-level = 3`, thin LTO, `codegen-units = 4`, `panic = "unwind"`, line-table debug info, packed split debuginfo and stripped symbols. Dependencies are built with `opt-level = 2` in the dev profile.

## CI

**Current status.** CI is disabled manually on GitHub because of a billing problem, so no workflow runs on pushes or pull requests. The local gates are the only barrier: before a delivery is accepted, the three commands above, `cargo deny check` and the `policy` steps of `ci.yml` (ARQ-07, forbid `unsafe`, V5, V12, V13) are run locally, the steps under `bash --noprofile --norc -eo pipefail`, and recorded in the change folder. The Windows and macOS legs of the matrix are therefore not exercised today. The description below is of the workflow file as it stands in the repository, to be re-enabled by the owner.

The workflow is `.github/workflows/ci.yml`. It runs on pushes to `main`, pull requests and manual dispatch, with read-only `contents` permission and cancel-in-progress for pull requests. Third-party actions are pinned to 40-hex commit SHAs. Its V5 table has a row for each library crate, `gafira-testkit` included.

| Job | Runs on | Steps |
|---|---|---|
| `test` | Matrix: `windows-2022` (`x86_64-pc-windows-msvc`), `ubuntu-24.04` (`x86_64-unknown-linux-gnu`), `macos-14` (`aarch64-apple-darwin`) | Install the pinned toolchain; check the host triple equals the matrix target; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` |
| `policy` | `ubuntu-24.04` | `cargo fmt --check`; CLI has no UI dependencies (ARQ-07); every `lib.rs` and `main.rs` has `#![forbid(unsafe_code)]`; layer table (V5); `clippy.toml` copies in sync (V12); `EditToken` only in the edit crates (V13); `cargo-deny check` |

The `policy` job is where architecture rules become executable. See the architecture chapter for V5, V12 and V13.

## Dependency and license policy

`deny.toml` drives `cargo-deny`:

- Licenses: only permissive ones (MIT, Apache-2.0 with or without the LLVM exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016, BSL-1.0, CC0-1.0, MIT-0, 0BSD). Workspace crates use `LicenseRef-Gafira-Proprietary` and are private (`publish = false`). Allowlist changes for transitive dependencies need a per-crate `[[licenses.exceptions]]` entry with a written rationale.
- Bans: wildcard versions are denied (path dependencies are exempt); duplicate versions warn. A deny list blocks FFmpeg bindings, `symphonia`, persistent-collection crates (`imbl`, `im`, `rpds`), `gstreamer`, `openssl-sys`, patent-encumbered or LGPL encoders (`openh264`, `x264`, `mp3lame-encoder`) and GNU gettext wrappers.
- Sources: unknown registries and git sources are denied.
- Advisories: yanked crates are denied. The advisory database needs network access, so it runs in CI and may be skipped locally.
- Targets checked: the three supported triples.

## Testing policy

- Tests cover the behavior of the public contract, not the implementation.
- Test layers: unit tests next to the code, integration tests in each crate's `tests/` directory, property tests with `proptest` (a workspace dependency with default features off and `std` on), compile-fail tests with `trybuild`, and golden files for the document format. No end-to-end layer exists yet.
- Every core command runs the six-law `prop_command` harness of `gafira-testkit` (see below and the edit chapter).
- Critical code, for which a missed mutant is a failure: validation, JSON load and save, ids and the arena, transforms, size and DPI limits, and any rule with an invariant. Non-critical: `Display` and `Error` text unless a spec fixes it, trivial accessors and test helpers.
- Compile-fail doctests guard capability rules (for example that `EditToken` is not `Clone`). They are regression aids, not proof; the interim evidence is CI step V13 plus the `trybuild` fixtures.

### gafira-testkit

A support crate (dev-dependency only, `publish = false`, V5 row, no `clippy.toml`). It provides:

- `document()`: a strategy for valid documents, bounded (at most 3 pages, with capped node count and tree depth), or the empty document.
- `corrupted()`, `corrupt(doc, Corruption, pick)` and the 24-variant `Corruption` enum: a valid document broken in exactly one way, with the validation error it must produce.
- `prop_command` and `try_prop_command`: the six-law command harness (1 000 cases by default; the `PROPTEST_CASES` environment variable overrides it).

Use it from integration tests (`tests/`) only, where each crate has one copy of its types. The generator started in the tests of `gafira-doc` and moved here so `gafira-edit` can share it.

### trybuild

`trybuild` (a dev-dependency of `gafira-edit`) compiles the fixtures in `crates/gafira-edit/tests/ui/` and compares the compiler output with the stored `.stderr` files. Those files quote compiler text, so a change of the toolchain pin or of a quoted signature needs them regenerated with `TRYBUILD=overwrite` and reviewed. The fixtures are the interim evidence for ARQ-30; the edit chapter says what they prove.

## Mutation testing

`cargo-mutants` is an optional developer tool, like `cargo-deny`; it is not a CI gate. The `mutants` agent runs it when the `mutation` value of the `Flow:` line asks for a run, and the `verify` agent fails on a missed or timed-out mutant in critical code and documents the rest. There is no score target. The run mutates a scope set by the mutation tier (T0 to T4, see the [methodology chapter](10-methodology.md)). By default (`mutation: archive`) there is no per-delivery run, and the change closes with one whole-crate run (T4) of every crate it touched, before archive. `mutation: delivery` adds the per-delivery tiers T2 and T3, and `mutation: off` runs no `cargo-mutants`.

A second practice complements it. When a test is written after the code, the delivery shows that the test can fail by applying one hand mutation at a time (for example `<` for `<=`, no early `break`, compacting the newest entry), running the tests and restoring the code. Hand mutations are recorded as a selection in the change's `apply-progress.md`, never as a complete run. A surviving mutant found by `cargo-mutants` in critical code sends the delivery back for a fix round with new tests.

```bash
cargo mutants -p <crates> --in-diff <diff> --jobs 8 -o <scratch dir>
```

This is the T2 command, used only with `mutation: delivery`. The other tiers that run `cargo-mutants` use these shapes (`AGENTS.md`):

| Tier | Command shape |
|---|---|
| T3 | `cargo mutants -p <crate> --file <file> --jobs 8 -o <scratch dir>`, one `--file` per changed non-test source file; only with `mutation: delivery` |
| T4 | `cargo mutants -p <crate> --jobs 8 -o <scratch dir>`; the default run, once per touched crate at change close |

`--iterate` is allowed on T3 only for test-only fix rounds with the same file set, never on the closing PASS round.

Rules, from `AGENTS.md` and `.cargo/mutants.toml`:

- Use the default `cargo test` runner (nextest skips doctests) and no `RUSTFLAGS=-D warnings`.
- `--locked` already comes from `additional_cargo_test_args` in `.cargo/mutants.toml`; do not pass it again, because cargo rejects a duplicate.
- `.cargo/mutants.toml` sets `profile = "mutants"`. The `[profile.mutants]` table in `Cargo.toml` inherits `test` and sets `debug = "none"`, which shortens the build of each mutant. Do not pass `--profile`.
- Use `--jobs 8`. A fast linker such as mold or wild is optional and not installed.
- `#[mutants::skip]` is forbidden because it needs a dependency. Equivalent mutants are excluded in `.cargo/mutants.toml` with a regex that matches exactly one mutant and a written reason. The current entry excludes `Document::schema_version -> u32 with 1`, equivalent while `SCHEMA_VERSION` is 1 and load accepts only version 1.
- Send output to a scratch directory with `-o`; `/mutants.out/` is gitignored.

## Delivery size

Measure each delivery with `git diff --shortstat`. Split when it exceeds about 700 lines; estimates have undershot by 1.4 to 1.9 times, mostly in tests. Stacked pull requests are cut at commit boundaries and each prefix must pass the gates alone.
