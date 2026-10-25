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

## D1 - Flicks, constants and checked arithmetic

Status: done.

RED evidence (first run of `cargo test -p gafira-time --test flicks` against compile-clean stubs that returned zero; no compile errors):

- `seconds_are_exact` -> `left: 0, right: 705600000`.
- `milliseconds_are_exact` -> `left: 0, right: 705600` (and the 1,234 ms case exposed a wrong expected value in the spec; see deviation).
- `seconds_and_milliseconds_overflow_instead_of_wrapping` -> expected `Err(Overflow)`, got `Ok(0)`.
- `addition_overflow_is_an_error`, `subtraction_overflow_is_an_error`, `multiplication_overflow_is_an_error`, `negation_overflow_is_an_error` -> expected `Err(Overflow)`, got `Ok(0)`.
- `whole_seconds_floor_towards_negative_infinity` -> `left: 0, right: 1` / `right: -1`.
- `f64_seconds_are_the_lossy_display_value` -> `left: 0.0, right: 1.0`.
- `serde_uses_a_plain_integer` -> stub value serialized as `0` instead of `705600000`.
- Proptest: `add_then_subtract_is_an_identity`, `seconds_round_trip`, `milliseconds_round_trip`, `seconds_floor_matches_euclidean_division` failed with minimal input `value = 1`.
- 4 tests passed on the stubs (constants, tick divisibility, negative tick preservation), which is expected: those rules are constants or trivial accessors, not arithmetic.
- Result of the RED run: `15 failed; 4 passed`.

GREEN evidence:

- `cargo test -p gafira-time --test flicks` -> `19 passed; 0 failed` (unit tests plus 5 proptest properties).
- `cargo fmt --check` -> exit 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` -> exit 0; `cargo test --workspace --locked` -> exit 0 (all workspace suites); `cargo deny check` -> all sections ok; local CI policy replay -> `policy status=0`.

Measured size: 5 files, 435 insertions under `crates/` (including the proptest regression seed).

Deviation: the spec example for 1,234 ms said 870,854,400 flicks; the correct product is 870,710,400 (1,234 x 705,600). The RED unit test exposed it before the implementation commit. The spec was corrected in commit `3d5ace9` (`docs(spec): correct the 1,234 ms example to 870,710,400 flicks`), and the test asserts the corrected value. No other criterion changed.

