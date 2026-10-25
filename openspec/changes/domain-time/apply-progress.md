Flow: full | pack: new capability or crate | tier: T3 | verify: V3 | decided-by: coordinator | reason: new layer-0 crate (full floor: new crate, V5 row, ordered deliveries); T3 per changed source file, T4 at archive; verify is left to the independent verifier

# Apply progress: domain-time (`gafira-time`)

Append-only. Each delivery records its RED evidence, the gates, the measured size and any deviation. Specs: `openspec/changes/domain-time/specs/time-model/spec.md` and `specs/timecode/spec.md`. Design and delivery table: `openspec/changes/domain-time/design.md`.

## D0 - crate skeleton and shared wiring

Status: done.

RED evidence (D0 has no Rust behavior; the acceptance criteria are command-level):

- Before the crate existed, `cargo tree -p gafira-time -e normal,build` failed with `error: package ID specification 'gafira-time' did not match any packages`.
- Before the V5 row existed, `grep -n gafira-time .github/workflows/ci.yml` returned no match while `crates/gafira-time/clippy.toml` already existed, which is exactly the V5 rejection the CI step is written for.

GREEN evidence:

- `cargo check -p gafira-time` -> `Finished dev profile`; `Cargo.lock` gained only the `gafira-time` package (serde, serde_json and proptest were already in the lock).
- `cargo tree -p gafira-time -e normal,build --target all --prefix none -f '{p}'` -> only `gafira-time`, `serde`, `serde_core`, `serde_derive`, and their proc-macro build dependencies. No other `gafira-*` crate.
- `cp crates/gafira-geom/clippy.toml crates/gafira-time/clippy.toml` + `diff -q` -> identical (V12).
- V5 row added: `gafira-time  -`.
- Root `[workspace.dependencies]` gained `gafira-time = { path = "crates/gafira-time" }`.
- `docs/specs/README.md` status row records C2c `domain-time`.

Gates (all from the repository root, before commit):

- `cargo fmt --check` -> exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -> exit 0.
- `cargo test --workspace --locked` -> exit 0 (gafira-time has no tests yet; the crate compiles).
- `cargo deny check` -> `advisories ok, bans ok, licenses ok, sources ok`, exit 0.
- Local replay of the CI `policy` job -> `policy status=0`: ARQ-07 tree clean, `forbid(unsafe_code)` present, V5 table clean with 6 crates and 5 clippy copies, V12 sync clean, V13 `EditToken` scan clean, `fmt` clean.

Measured size: `git diff --cached --shortstat -- crates` = 3 files, 70 insertions; whole D0 change = 7 files, 82 insertions, 1 deletion.

Deviation: none.
