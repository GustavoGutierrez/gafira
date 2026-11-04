# Verify report: layer-3-scaffold, SC1, round 1

Commit `059d51d` on `feat/layer-3-scaffold`, base `develop` at `ed1dc7d`. Depth V2, mode full, level L2, tier T4, `mutation: archive` (no `mutants` report, not a finding). Date 2026-11-04.

**Verdict: PASS.** CRITICAL 0, WARNING 3, SUGGESTION 3. Classes: implementation 2, ambiguity 1, claim 0, architecture 0.

Probes and hand mutants ran on a scratch export outside the repo (`git archive` of `059d51d` into `/tmp/gafira-verify-sc1`, own `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL=0`). The repo worktree stayed clean apart from these two files.

Precedent compared: `specs/history/2026-11-02-layer-2-scaffold/` (same card shape, same RED form, same negative controls). The layer-2 verdict was PASS with 4 WARNINGs; this delivery has the same profile.

## Gates

Each command ran alone and its own exit code was read. `CARGO_INCREMENTAL=0`. The CI policy steps were extracted from the `run:` blocks of `.github/workflows/ci.yml` with a YAML parse and run with `bash --noprofile --norc -o pipefail -e`, in the repo worktree at `059d51d` (clean).

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (179 binaries, 2271 tests passed, 0 failed, 0 ignored) |
| `cargo deny check` (0.20.2, the version CI built) | 0 (`advisories ok, bans ok, licenses ok, sources ok`) |
| CLI has no UI dependencies (ARQ-07) | 0 |
| Domain crates forbid unsafe code | 0 |
| Domain crates respect the layer table (V5) | 0 |
| Domain clippy.toml copies are in sync (V12) | 0 |
| Layers 0-2 know no language (ARQ-107) | 0 |
| EditToken only in the edit crates (V13) | 0 |
| Every core command has a property test (V14) | 0 |

Extra gates, because the `ports` feature of the testkit is off by default:

| Gate | Exit |
|---|---|
| `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` | 0 |
| `cargo test -p gafira-testkit --features ports --locked` | 0 |
| `cargo clippy --workspace --all-targets --locked --features gafira-testkit/ports -- -D warnings` | 0 |
| `cargo test --workspace --locked --features gafira-testkit/ports` | 0 (179 binaries, 2272 tests passed) |
| `cargo clippy -p gafira-render -p gafira-media --all-targets --features ports --locked -- -D warnings` | **101** — `none of the selected packages contains this feature: ports`. The `ports` feature belongs to `gafira-testkit`; the two new crates have no testkit dev-dependency yet (XP9's finding: that edge arrives with `RE1`/`ME1`). The command as written cannot apply to this delivery. See S3. |

## CI run (AC8)

Run `41836317734`, `workflow_dispatch`, branch `feat/layer-3-scaffold`, SHA `059d51d`: **completed, success**. Watched to the end (`gh run watch --exit-status`, exit 0).

| Job | Result |
|---|---|
| `policy` | success, 1m6s. Every step ran: Format, CLI/UI (ARQ-07), forbid, V5, V12, ARQ-107, V13, V14, cargo-deny |
| `test (x86_64-unknown-linux-gnu)` | success, 13m35s. Clippy and Test both success |
| `test (x86_64-pc-windows-msvc)` | success, 20m20s. Clippy and Test both success |
| `test (aarch64-apple-darwin)` | success, 4m0s. Clippy and Test both success |

This closes the unverified half of XP8: the native Windows and macOS compile that the spike could not measure on this host. The only annotations are the Node.js 20 deprecation notices on `actions/checkout`, pre-existing and unrelated.

## Acceptance criteria

| AC | Type | Status | Proof |
|---|---|---|---|
| AC1 | command | covered | V5 step exit 0 clean (repo and export). Row lacking a dependency: exit 1 (P1, P3). A real dependency not in the row: exit 1 (P2) |
| AC2 | command | covered | V12 step exit 0; `cmp` against the canonical copy. All three `clippy.toml` files share md5 `eb4b53786378c56edb32f75f8ab7ed88`. Drift: exit 1 (P4, P4b) |
| AC3 | command | covered | forbid step exit 0; the attribute removed: exit 1 (P5) |
| AC4 | command | covered | ARQ-107 step exit 0 clean, exit 1 with `icu_plurals` in `gafira-media` (P6) and with the word `Locale` in it (P7). Both probes hit the crate the implementer did not use |
| AC5 | command | covered | `cargo deny check` exit 0 locally and in the remote `policy` job. The new crates are inside the checked graph: `[licenses.private] ignore = false` gives exit 4 and names `gafira-render` (P8a) |
| AC6 | unit | covered | `crates/gafira-ports/tests/render.rs::arq_44_tile_cache_reports_bytes_and_trims_to_a_budget` names ARQ-44 and exercises the calls through `Box<dyn TileCache>`. Independent probe (P9): two tests of my own that pin the exact signatures (`fn bytes(&self) -> Bytes`, `fn trim(&mut self, Bytes)`) through fn-pointer coercions and drive them through the trait object. Hand mutant HM3 proves the implementer's test is load-bearing on the trait change |
| AC7 | command | covered | The `ports` feature is off by default: the testkit's default tree (default edges, normal+build+dev) holds neither `gafira-ports` nor `gafira-raster` (P10); with `--features ports` both appear (P11). The nine doubles are reachable from an outside consumer that takes the testkit as a dev-dependency with `features = ["ports"]` (P12). Hand mutant HM2 kills `arq_112_default_testkit_tree_carries_neither_ports_nor_raster` |
| AC8 | command | covered | The run above, job by job |

All eight criteria are covered.

## Product IDs (step 3c)

`git grep <ID>` over `crates/*/tests/` and `crates/*/src/`, plus the artifacts that actually prove each row.

| ID | Proof | Named by |
|---|---|---|
| ARQ-01 | The V5 step plus the two manifests, cross-checked against the layer table of 02 §2.1: `gafira-render` (layer 3) depends on layers 3, 1, 0, 0, 2, 0, 2 and `gafira-media` (layer 3) on 3, 1, 2, 0, 0, 0; neither depends on `gafira-engine` (layer 3), `gafira-i18n` (layer 4) or the other. `cargo xtask check-layers` does not exist | Nothing relevant. The V5 step does not name ARQ-01; `git grep ARQ-01` traces only to `crates/gafira-doc/tests/content_hash.rs` and `crates/gafira-doc/tests/contract_imports.rs`, which do not cover the new crates. **W1** |
| ARQ-02 | The canonical `clippy.toml` copies (byte-identical, md5 above) plus the clippy gate; HM1 shows the lint really binds `gafira-render` (`use of a disallowed method std::fs::read_to_string`). Note the row scopes the lint to layers 0–2; the layer-3 rule that binds `gafira-render` is ARQ-138 | The canonical `clippy.toml` header: `# ARQ-02 layer purity for domain crates` |
| ARQ-03 | The CI matrix holds exactly the three targets (`ci.yml` unchanged by the delivery) and the remote run built all three. `gafira-platform` does not exist yet (layer 4), so its `compile_error!` half is out of scope | Nothing. `git grep ARQ-03` over the tests returns nothing. **W1** |
| ARQ-05 | Only vacuously: neither `crates/gafira-render/src/lib.rs` nor `crates/gafira-media/src/lib.rs` exposes a single `pub` item, so the property holds by construction. The row's own mechanism — `cargo public-api` running in CI for `gafira-doc`, `gafira-ports` and `gafira-engine` — does not exist in the repo, and none of those three crates is touched here | Nothing. **W2** |
| ARQ-06 | The V5 rows and the two manifests: neither new crate declares `gafira-engine`, and neither declares the other (ADR 0015). The row's stated check (`check-layers` over the layer-4 adapters) has no subject yet: no layer-4 adapter exists and `check-layers` does not exist | Nothing in the proving artifact. `git grep ARQ-06` traces only to `crates/gafira-ports/tests/`. **W1** |
| ARQ-44 (partial) | `crates/gafira-ports/tests/render.rs:407` names ARQ-44 above `arq_44_tile_cache_reports_bytes_and_trims_to_a_budget`. Independent probe P9; mutant HM3 | The test itself |
| ARQ-70 | `cargo deny check` exit 0 locally and in the remote `policy` job; P8a shows the new crates are in the checked graph | `deny.toml` header: `# cargo-deny policy (ARQ-70, ARQ-71)` |
| ARQ-71 | `deny.toml` has `allow-git = []` and `unknown-git = "deny"`; the deny job is green in the remote run | `deny.toml` header |
| ARQ-107 | The CI step, whose name carries `(ARQ-107)`, reads the crate list from the V5 table, so it covers the two new crates with no edit; P6 and P7 confirm the step bites on `gafira-media` | The step name in `ci.yml` |

**Left out, with the reason:**

- `ARQ-05` is the one ID the card lists without any mechanism that can prove it. It is reported as **W2** (class `ambiguity`), not as a missing test: the row names a CI step the repo does not have and three crates this delivery does not touch, and the property is vacuous for two empty crates. The card's ID list is the thing that is imprecise.
- `ARQ-01`, `ARQ-03` and `ARQ-06` are proved by commands that do not name them. Reported as **W1** (class `implementation`), the same judgement the layer-2 verify recorded for ARQ-01.
- `ARQ-44` is marked *partial* in the card, and it is partial here too: the trait declares the calls and the one implementor in the workspace compiles, but the production cache (`gafira-render`, `RE3`) does not exist yet, so only the contract and a test double are proved.
- `ARQ-112` is not in the card's ID list but is named by `crates/gafira-testkit/tests/ports_feature.rs`; it is the row behind AC7 and is proved by that test. Its own row is about cancellation and progress in layer 2, of which the testkit doubles are a small part.

## Artifact integrity

- `c5f6b12 docs(layer-3-scaffold): slice card` touches only `specs/changes/layer-3-scaffold/apply-progress.md` (1 file, 29 insertions) and precedes the first implementation commit `14409a6`.
- `git diff c5f6b12..HEAD -- apply-progress.md` starts at line 20: the card block (lines 1–15) is unchanged. The two later `docs` commits only add the apply-progress notes and `spike.md`. No implementer commit touched the card.
- Conventional commit prefixes throughout: `feat`, `docs`. No AI attribution line in any of the five commits (grep for `co-authored-by`, `generated with`, `claude`, `anthropic`: none).
- The V5 rows match the Slice card's Plan exactly: `gafira-render` gets `gafira-ports gafira-doc gafira-geom gafira-color gafira-raster gafira-time gafira-anim` and `gafira-media` gets `gafira-ports gafira-doc gafira-raster gafira-time gafira-geom gafira-color`. The manifests declare subsets of those rows, which is what the step checks.
- `Cargo.lock` grew 48 lines and removed none: `blake3` 1.8.7 with `arrayvec`, `cc`, `cfg-if`, `constant_time_eq`, `cpufeatures`, plus the two new crates. `kurbo` was already in the tree through `gafira-geom`/`gafira-ports`; `gafira-render` only makes it a direct dependency.
- The root manifest gained exactly the two crates and `blake3 = "1"`, as the delivery notes say.
- The two new crates declare no edge to each other and none to `gafira-engine` (ADR 0015, ARQ-01, ARQ-06).

## Probes (step 1c)

My own checks, derived from the acceptance criteria, run on the scratch export. Each one mutates the export, reads an exit code and reverts; the export was clean at the end.

| ID | Probe | Expected | Exit |
|---|---|---|---|
| P0 | baseline: forbid, V5, V12, ARQ-107 on the untouched export | 0 | 0 each |
| P1 | the `gafira-render` V5 row without `gafira-ports` | 1 | 1 (`::error::gafira-render must not depend on gafira-ports`) |
| P2 | `gafira-render` gains a real dependency not in its row (`gafira-edit`) | 1 | 1 (`::error::gafira-render must not depend on gafira-edit`) |
| P3 | the `gafira-media` V5 row without `gafira-raster` | 1 | 1 (`::error::gafira-media must not depend on gafira-raster`) |
| P4 | one line of drift in `crates/gafira-media/clippy.toml` | 1 | 1 (`clippy.toml differs from crates/gafira-geom/clippy.toml`) |
| P4b | `crates/gafira-render/clippy.toml` without the `HashMap` ban | 1 | 1 (the step compares the whole file, not only the array) |
| P5 | `forbid(unsafe_code)` removed from `crates/gafira-media/src/lib.rs` | 1 | 1 (`missing forbid(unsafe_code)`) |
| P6 | `icu_plurals = "2"` in `crates/gafira-media` | 1 | 1 (`::error::gafira-media depends on icu_plurals v2.4.0 (ARQ-107)`) |
| P7 | the word `Locale` in `crates/gafira-media/src/lib.rs` | 1 | 1 (`::error::gafira-media mentions Locale (ARQ-107)`) |
| P8a | `deny.toml` `[licenses.private] ignore = false` | non-zero | 4 (`failed to satisfy license requirements`, names `gafira-render`) |
| P8b | `ring = "0.17"` added to `gafira-render` | 0 | 0 — `ring` is `Apache-2.0 AND ISC`, both allowlisted. The check does not fire on an allowlisted licence. (My first assumption, that `ring` carries OpenSSL, was wrong; the licence expression is `Apache-2.0 AND ISC`.) |
| P9 | my own TileCache probe test on the export: two tests pinning `fn bytes(&self) -> Bytes` and `fn trim(&mut self, Bytes)` through fn-pointer coercions and driving them through `Box<dyn TileCache>` | 0 | 0 (2 passed) |
| P10 | the testkit's default tree with the default edges (normal, build **and** dev) | neither crate | neither `gafira-ports` nor `gafira-raster` appears |
| P11 | the testkit's tree with `--features ports` | both crates | `gafira-ports` and `gafira-raster` present |
| P12 | an outside crate that takes the testkit as a dev-dependency with `features = ["ports"]` and imports all nine doubles | 0 | 0 (1 passed) |

P2 and P4b are the probes the implementer did not run: a *real* dependency missing from a row, and a `clippy.toml` whose banned type is gone rather than a byte of drift. P9, P10 and P12 are the checks I derived from AC6 and AC7 independently of the implementer's tests.

## Hand mutants (step 4b)

`tdd-apply` recorded no list (the design marks SC1 as "configuration only"), so there is no spot-check and no repeat. Three independent mutants, chosen from the spec rules that apply to this delivery.

| ID | Mutation | Expected | Result |
|---|---|---|---|
| HM1 | `crates/gafira-render/src/lib.rs` calls `std::fs::read_to_string` | the canonical `clippy.toml` rejects it | **killed**: `cargo clippy -p gafira-render --all-targets --locked -- -D warnings` exit 101, `error: use of a disallowed method std::fs::read_to_string` (`-D clippy::disallowed-methods`) |
| HM2 | the testkit's `ports` feature becomes a default feature | AC7 breaks | **killed**: `cargo test -p gafira-testkit --test ports_feature --locked` exit 101, `arq_112_default_testkit_tree_carries_neither_ports_nor_raster` FAILED at `crates/gafira-testkit/tests/ports_feature.rs:36` |
| HM3 | the `ports::TileCache` trait loses `fn bytes(&self) -> Bytes` | the ARQ-44 test stops compiling | **killed**: `cargo test -p gafira-ports --test render --locked` exit 101, `error[E0407]: method bytes is not a member of trait TileCache` |

Three run, three killed, no survivors. HM3 is the one that matters most for AC6: it shows the implementer's test is load-bearing on the trait change and not a weaker restatement of it.

## Findings

### CRITICAL

None.

### WARNING

**W1. ARQ-01, ARQ-03 and ARQ-06 are proved by commands that do not name them.** class: implementation. Ref: `.github/workflows/ci.yml` V5 step and the CI matrix; `specs/02-arquitectura-software.md:72,76,169`.
The V5 step proves the edge rule (ARQ-01, ARQ-06) and the CI matrix proves the target set (ARQ-03), but neither artifact mentions the ID, and `git grep` over the tests traces each of the three only to unrelated crates (`gafira-doc/tests/content_hash.rs`, `gafira-doc/tests/contract_imports.rs`, `gafira-anim/tests/golden.rs`, `gafira-ports/tests/export.rs`). Read literally, step 3c makes a missing name CRITICAL. I read it as WARNING, the same judgement the layer-2 verify recorded for ARQ-01: all eight ACs of this card are typed `[command]`, the card's "no production logic" non-goal excludes tests from this cut, and the proof exists and was mutated (P1, P2, P3 for the V5 rule; the remote run for the matrix). Stated here so the coordinator or the owner can overrule. Fix: name the three IDs in the V5 step and in the matrix block of `ci.yml`.

**W2. ARQ-05 is listed as a covered Product ID, but nothing in the repo can prove it.** class: ambiguity. Ref: `specs/02-arquitectura-software.md:76`; `specs/changes/layer-3-scaffold/apply-progress.md:3`.
The row's only stated acceptance mechanism is "`cargo public-api` runs in CI for `gafira-doc`, `gafira-ports` and `gafira-engine`". There is no `public-api` step in `ci.yml`, no `cargo public-api` in any manifest, and no `cargo xtask` in the repo (only the ARQ-107 step's comment says "until `check-layers` exists"). None of the three crates the row names is touched by SC1. For the two new crates the property holds vacuously: neither `lib.rs` exposes a single `pub` item. The defect is in the card's ID list, not in the code, so this is not routed to `tdd-apply`. Fix, one of: drop ARQ-05 from the card's ID list for this cut, or state in the card that for SC1 it is proved by the emptiness of the two crates. The `specifier` owns the product row; the coordinator owns the card.

**W3. No CI job compiles the `ports` feature, so the nine doubles are outside the remote gate.** class: implementation. Ref: `.github/workflows/ci.yml:46-49`; `crates/gafira-testkit/Cargo.toml`; `crates/gafira-testkit/tests/ports_feature.rs:52`.
`ci.yml` runs `cargo clippy --workspace --all-targets --locked` and `cargo test --workspace --locked` with default features, and `ports` is not a default feature. So `#[cfg(feature = "ports")] pub mod ports` — the nine doubles, about 300 authored lines — and the `#[cfg(feature = "ports")]` test are compiled out in CI. The run `41836317734` that closes AC8 never compiled them; the plain workspace test run shows `ports_feature.rs` with 2 tests, not 3. The delivery's own extra gate (`cargo clippy`/`cargo test -p gafira-testkit --features ports`) covers this, but it runs only on the implementer's machine. SC1 is the delivery that owns `ci.yml` (the card's Plan: "no later delivery edits `ci.yml`"), so the missing job belongs here: a defect in a double would land in `develop` behind a green CI. Fix: add the feature-on gate to the `policy` job of `ci.yml` (one line).

### SUGGESTION

**S1.** The V5 row of `gafira-testkit` (`gafira-geom gafira-color gafira-doc gafira-edit`) describes only its default tree. With the `ports` feature on, that tree also holds `gafira-ports`, `gafira-raster` and `gafira-time`, none of which the row lists — the three are reachable through `gafira-ports`. No step ever resolves the graph with the feature on (V5 and the ARQ-107 step both run `cargo tree` without `--features`), so the discrepancy is invisible today. The choice is deliberate and recorded in `d5caa7b` ("its V5 row does not change"); this is a note, not a defect. It is the same shape as the layer-2 warning about an optional dependency escaping a step.

**S2.** The forecast was 200 authored lines (x1.4 = 280). The measured delivery is 675 insertions in 24 files at `d5caa7b`, of which about 533 are authored after excluding `Cargo.lock` (48) and the two `clippy.toml` copies (94) — roughly x2.7 over the forecast, above the x1.4–x1.9 band AGENTS describes, though under the ~700 ceiling. `HEAD` is 1161 insertions because `spike.md` (+449) landed in `059d51d`, outside the delivery. Nothing to stop for; the coordinator may want to re-forecast the rest of the cut, whose per-delivery estimates are 430–500.

**S3.** `cargo clippy -p gafira-render -p gafira-media --all-targets --features ports --locked -- -D warnings` exits 101 with `none of the selected packages contains this feature: ports`: the `ports` feature belongs to `gafira-testkit`, and the two new crates have no testkit dev-dependency yet (the XP9 finding: that edge arrives with `RE1`/`ME1`). The equivalent that does apply is `cargo clippy --workspace --all-targets --features gafira-testkit/ports -- -D warnings`, exit 0. Recorded so the coordinator does not read the 101 as a gate failure.

## Notes

- The apply-progress note quotes "24 files changed, 675 insertions(+), 6 deletions(-)" for `git diff --shortstat develop...HEAD`. That is the measurement at `d5caa7b`; the two `docs` commits add 37 lines of notes and 449 lines of `spike.md` on top. The number in the note is the delivery's own, and it is correct for the delivery.
- The note's "about 504" authored lines is 675 − 48 (lockfile) − 94 (the two `clippy.toml` copies) − 29 (the notes file itself). Both readings (533 without the notes, 504 with) are consistent with "about".
- The AC1 exit-code deviation already recorded in the notes ("a row without a crate exits 101, not 1") concerns a case AC1 does not state. AC1's case — a row without a crate's *dependency* — exits 1, confirmed by P1 and P3.
- The T4 mutants for the four touched crates (`gafira-render`, `gafira-media`, `gafira-ports`, `gafira-testkit`) are still owed before the archive, per the Flow line's `mutation: archive`. The spike already warns that `gafira-render` has 0 mutants today, so its T4 will report "no mutants found".
- The XP8 gap that the spike could not close (the native Windows and macOS compile) is closed by run `41836317734`.

next: archive
