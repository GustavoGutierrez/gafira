# Verify report: layer-2-scaffold, SC1, round 1

Commit `60f39cf` on `feat/layer-2-scaffold`, base `8b3cf93`. Depth V2, mode full, level L3, `mutation: archive` (no `mutants` report, not a finding). Date 2026-11-02.

**Verdict: PASS.** CRITICAL 0, WARNING 4, SUGGESTION 2. Classes: implementation 2, claim 1, ambiguity 1, architecture 0.

Probes and mutants ran on scratch exports outside the repo. The worktree stayed clean apart from these two files.

## Gates

Each command ran alone and its own exit code was read, with no pipe. `CARGO_INCREMENTAL=0`, one target dir per export.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (106 binaries `ok`, 0 `FAILED`) |
| `cargo deny check` (0.20.2, the version CI built) | 0 |
| ARQ-07, forbid, V5, V12, ARQ-107 steps | 0 each |
| V13, V14 steps | 0 each, on a scratch clone |

The steps were extracted from `ci.yml` and run with `bash --noprofile --norc -e -o pipefail`. V13 and V14 first gave exit 1 on the plain export because they call `git`, which needs a repository: an artifact of the export, gone on a clone. Those two runs are not evidence against the tree.

## CI run (AC6)

Run 41393718000, `workflow_dispatch`, branch `feat/layer-2-scaffold`, SHA `60f39cf`: completed, success.

| Job | Result |
|---|---|
| `policy` | success. Every step ran, including ARQ-107 and cargo-deny |
| `test (x86_64-pc-windows-msvc)` | success, Clippy and Test both success |
| `test (aarch64-apple-darwin)` | success, Clippy and Test both success |
| `test (x86_64-unknown-linux-gnu)` | success, Clippy and Test both success |

This closes the XP2 gap: the new dependencies compile on Windows and macOS.

## Acceptance criteria

| AC | Status | Proof |
|---|---|---|
| AC1 | covered | V5 exit 0 clean. Row lacking a dependency: exit 1 (V1, V2, V3, V8) |
| AC2 | covered | V12 exit 0, `cmp` with the canonical copy; drift gives exit 1 (C3) |
| AC3 | covered | forbid step exit 0; attribute removed gives exit 1 (C4) |
| AC4 | covered | ARQ-107 step exit 0 clean, exit 1 with `icu_plurals` in `gafira-raster` (A1), under gawk and mawk |
| AC5 | covered | `cargo deny check` exit 0, locally and in CI |
| AC6 | covered | run above |

## Product IDs (step 3c)

| ID | Proof | Named by |
|---|---|---|
| ARQ-01 | V5, mutants V1 to V8 | V5 does not name it: only an unrelated `gafira-doc` test does (W1) |
| ARQ-02 | Each `clippy.toml` copy. On the new crates `std::fs::read`, `HashMap`, `std::env::var` and `std::thread::spawn` are all rejected (C1, C2) | `clippy.toml` header |
| ARQ-70 | cargo-deny job; removing `Unicode-3.0` gives exit 4 (D4), so the new tree is checked | `deny.toml` header |
| ARQ-107 | P9 step, mutants below | step name in `ci.yml` |

None left out. ARQ-01's `cargo xtask check-layers` does not exist yet; V5 stands in for it. The card's scope is the rows, as the delivery table says.

## Artifact integrity

- `544b503` touches only `apply-progress.md`, and precedes the first implementation commit.
- `git diff 544b503..HEAD` on that file removes no line: the card is unchanged.
- ADR 0007: row, `clippy.toml` and `Cargo.toml` entries land together in `7f7e588`.
- `deny.toml` is its own commit. `Cargo.lock` adds 553 lines and removes none.
- No attribution lines in the commits.
- The four V5 rows match ADR 0007, and every P1 row is present.

## Owner's questions

- **`parley` in `gafira-text`: correct.** `layout-core` design L9 (Text stack, the `gafira-text` wrapper) and 02 §4.17.3. Normal-tree check: present in text only, of the crates sampled.
- **`rayon` in `gafira-segment`: correct.** L9 row "`rayon` in `gafira-segment` only", and L8 (`ClassicSegmenter::new(pool: Arc<rayon::ThreadPool>)`). Absent from the normal trees of text, raster, audio and geom. Layer-2 design P1 names no crate, so this rests on L8 and L9, as the implementer said.
- **`rayon = "1"`: correct.** Resolves to 1.12.0, the current crates.io version (02 §9.2 says "última"), MSRV 1.80 under the pinned 1.93.1. Major-only matches `serde` and `proptest`.
- **V5 exit 101 versus AC1's 1: not a defect, and not the AC1 case.** AC1 is about a row lacking a crate's *dependency*, which exits 1. Exit 101 (mutant V5) is a row for a crate that does *not exist*: cargo's error passes through `set -e`. AC1 does not state that case, and ADR 0007 only says V5 "fails". It fails closed. See S2.

## Mutants and probes (step 1c, 4b)

Spot-check of the recorded list: V3, killed. 27 of 32 detected. Of the other 5, 3 are equivalent and 2 are the gaps W2 and W3.

| ID | Mutation | Expected | Exit |
|---|---|---|---|
| V1 | text row without `gafira-pathops` | fail | 1 |
| V2 | audio row without `gafira-time` | fail | 1 |
| V3 | segment row without `gafira-raster` (spot-check) | fail | 1 |
| V4 | raster row deleted | fail | 1 |
| V5 | ghost-crate row | fail | 101 |
| V6 | edge `raster -> anim` (ADR 0007) | fail | 1 |
| V7 | edge `layout -> text` (ADR 0007) | fail | 1 |
| V8 | text row set to `-` | fail | 1 |
| A1 | P9 control: `icu_plurals` in raster; fails raster and segment | fail | 1 |
| A2 | `icu_decimal` in geom | fail | 1 |
| A3 | `icu_datetime` in time | fail | 1 |
| A4 | stub `gafira-i18n` in audio | fail | 1 |
| A6 | `cfg(windows)`-only `icu_plurals` in color | fail | 1 |
| A8 | `Locale` in a test file of time | fail | 1 |
| A10 | `icu_plurals` renamed in `Cargo.toml` | fail | 1 |
| A12 | `icu_plurals` in layout | fail | 1 |
| S1 | step: `icu_plurals` dropped from `banned`, AC4 input | control misses it | 0, detected |
| S2 | step: first crate only, AC4 input | control misses it | 0, detected |
| S3 | step: `-e build` for `-e normal`, AC4 input | control misses it | 0, detected |
| S4 | step: wrong `Locale` pattern, `Locale` input | control misses it | 0, detected |
| D2 | `unused-license-exception = "deny"` | fail | 4 |
| D4 | `Unicode-3.0` out of the allowlist | fail | 4 |
| D6 | `assert_no_alloc` normal dep, no exception | fail | 4 |
| C1 | `fs::read`, `HashMap`, `env::var` in raster, text, audio | fail | 101 |
| C2 | `thread::spawn` in segment, alone | fail | 101 |
| C3 | `clippy.toml` drift | fail | 1 |
| C4 | `forbid` removed | fail | 1 |

C2 was re-run alone: in the first combined run segment never compiled, because raster failed before it.

Not detected:

| ID | Mutation | Exit | Reading |
|---|---|---|---|
| A7 | optional `icu_plurals` in segment, feature not enabled | ARQ-107 0, V5 0 | W3 |
| D5 | `assert_no_alloc` as a normal dep of audio, exception kept | deny 0, V5 0 | W2 |
| D1 | exception and key removed | deny 0 | equivalent: the entry is never matched |
| D3 | key removed | deny 0, with a warning | equivalent for the exit code |
| D7 | exception pointed at `rayon` | deny 0 | equivalent, same reason |

D2 proves the key is read and the entry is unused, which confirms the design's claim. So the P1 `deny.toml` row has no failing state today. AC5 proves only that deny exits 0.

Behaviour controls that correctly pass: A5 (dev-only `icu_plurals`, ARQ-107 says normal dependencies), A9 (`icu_segmenter` in text), A11 (`gafira-testkit` exempt, P9).

Other check: an empty stub does fail `cargo fmt --check` (exit 1), so the one-line `//!` in the testkit stubs is a justified deviation from P1's "Empty".

## Findings

### CRITICAL

None.

### WARNING

**W1. ARQ-01 is not named by the check that proves it.** class: implementation. Ref: `ci.yml` V5 step.
`git grep ARQ-01` traces to a `gafira-doc` test that does not cover the new crates. V5 proves it but does not name it. Read literally, step 3c makes a missing name CRITICAL. I read it as WARNING because all six ACs are typed `[command]`, the card excludes tests with its "no production logic" non-goal, and the proof exists and was mutated. That is a judgement call on a rule written for tests, stated here so the coordinator or the owner can overrule it. Fix: name ARQ-01 in a comment on the V5 table.

**W2. Nothing enforces "dev-dependency only" for `assert_no_alloc`.** class: claim. Ref: `deny.toml` `[[licenses.exceptions]]`, 02 §9.2.
`deny.toml` says it "never ships in a product binary", and 02 §9.2 says "Solo `dev-dependency`". Today that holds: it is in no normal or build tree sampled. But D5 shows that moving it to normal dependencies gives deny 0 and V5 0, while D6 shows the same move fails without the exception. The exception is inert for its intended use, and active only for the case it must not allow. D-37 and P1 require the entry, so this is not an implementer error. Options: a CI check that the crate is out of every `-e normal` tree, or drop the entry (D1 passes), which needs the owner because of D-37.

**W3. An optional banned dependency passes the ARQ-107 step.** class: ambiguity. Ref: `ci.yml` ARQ-107 step, design P9, ARQ-107.
A7: `icu_plurals` with `optional = true`, feature off, gives ARQ-107 exit 0 and V5 exit 0. The step follows P9's command exactly, and ARQ-107 does not say whether an inactive optional dependency counts, so the gap is upstream. It is a real route: `gafira-segment` will have an `onnx` feature (ADR-01). `cargo tree --all-features` shows 0 banned crates at HEAD on the 5 crates sampled, not all of them. Candidate fix: add `--all-features` to the step, which also bears on `ort`.

**W4. Delivery size is above the ceiling by the rule's measure, with no recorded OK.** class: implementation. Ref: `apply-progress.md`, AGENTS "Delivery rules".
`git diff --shortstat 8b3cf93..HEAD` gives 21 files, 944 insertions; 391 without `Cargo.lock`, about 340 authored and 188 of those are `clippy.toml` copies. "Under ~700" is true only without the lockfile, and AGENTS names no such exclusion. Authored size is also above the forecast (180, ×1.4 = 250). No stop or owner OK is recorded. The lockfile is generated and P1 requires it, so the risk is low. The ceiling is a signal for the coordinator, not a hard block.

### SUGGESTION

**S1.** `deny.toml` comments cite "layer-2 design E2/S20" and "L9", but those live in the `layout-core` design. The owner's ruling is D-37, which it does not cite.

**S2.** The last bullet of "Choices and deviations" in `apply-progress.md` reads 101 as a deviation from AC1. It is a different case (see above). No action beyond knowing that when reading the note.

## Notes

- `gafira-text`'s tree holds `memmap2` through `fontique`. It is unused today; for `TX1`, fonts arrive as bytes (02 §2.1).
- Mutation: T4 is still owed before archive for every touched crate, `gafira-testkit` included (`mutation: archive`). The four new crates have no logic yet.

next: archive
