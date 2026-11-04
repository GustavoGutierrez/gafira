# Verify report: gafira-media — ME1 round 2 (closure) and ME2 round 1

Two rounds in one file, verified at the head of `feat/layer-3-media`, commit `c9d696c` (the merge of `develop`), in the worktree `/tmp/gafira-wt/me`. Gates and inspection at `c9d696c`; the tree was clean before and after the run.

| Round | Delivery | Kind | Depth | Verdict |
|---|---|---|---|---|
| A | `ME1` | closure round 2 of 3 | V2, on the open findings only | **PASS** |
| B | `ME2` | first verify, round 1 of 3 | V1 (owner decision O5) | **PASS** |

Round A: **PASS.** CRITICAL 0, WARNING 6 (all from round 1, still open), SUGGESTION 4. Classes: implementation 5, claim 1.
Round B: **PASS.** CRITICAL 0, WARNING 3, SUGGESTION 1. Classes: implementation 1, ambiguity 2.

Both verdicts are mechanical: no CRITICAL is open in either delivery, and the gates are green at HEAD. Two cautions for the coordinator, stated up front:

- **ME1 still carries two real code defects** found by round 1's probes and untouched by the fix round: **W1** (the progress step breaks ARQ-139's 0.02 bound when the packet count does not match `duration x fps`) and **W2** (the keyframe times are never sorted, so the binary search can land wrong). A closure round does not re-severity earlier findings, so they stay WARNING and ME1 is PASS; the coordinator decides whether to route them to `tdd-apply` before merging ME1 or to carry them as follow-ups. Round 1 already routed them.
- **ME2 ships no bench**, so CAL-14's FIJADA figure for the cache search (< 50 ms) is unmeasured (**W8**), and the card's ME2 Flow line still says `verify: V2 | mutation: archive` while the owner decisions O5 and O4 say V1 and `mutation: off` (**W9**).

---

# Round A — `ME1`, closure round 2 (depth V2, open findings only)

The open findings are the ones of round 1 (`specs/changes/gafira-media/verify-report.md` at `c1ba024`, recorded in `verification.json` round 1): C1 (CRITICAL) and W1 to W6. The implementer answered with `9bc20f1 fix(gafira-media): check the cancel token in the keyframe seek` and recorded the round in `2c3dff5`.

Per the depth rule for closure rounds, this round runs the gates and the artifact-integrity checks, checks only that each earlier finding is closed, and spends no independent-mutant budget. No probe was written from scratch; every closure below rests on the code, the implementer's test, or the gate run at HEAD.

## Gates at HEAD (`c9d696c`)

Each command ran alone, in the worktree, with `CARGO_TARGET_DIR=/tmp/gafira-verify-me-target` and `CARGO_INCREMENTAL=0`. The exit code is the command's own (read through `PIPESTATUS` where the output was filtered).

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (183 binaries, 2311 tests passed, 0 failed) |
| `cargo deny check` | 0 (`advisories ok, bans ok, licenses ok, sources ok`) |
| `cargo test -p gafira-media --locked` | 0 (22 tests: `cache` 12, `contract` 3, `keyframe` 7) |
| Format (CI step) | 0 |
| CLI has no UI dependencies (ARQ-07) | 0 |
| Domain crates forbid unsafe code | 0 |
| Domain crates respect the layer table (V5) | 0 |
| Domain clippy.toml copies are in sync (V12) | 0 |
| Layers 0-2 know no language (ARQ-107) | 0 |
| EditToken only in the edit crates (V13) | 0 |
| Every core command has a property test (V14) | 0 |
| Testkit ports feature compiles (ADR 0013) | 0 |

The nine policy steps were extracted from the `run:` blocks of `.github/workflows/ci.yml` with a YAML parse and run one by one with `bash -euo pipefail`. The `cargo-deny` step is a `uses:` action in CI, so `cargo deny check` ran locally instead.

## Closure of the earlier findings

| Id | Severity | Class | Status now | Evidence |
|---|---|---|---|---|
| C1 | CRITICAL | implementation | **closed** | See below. |
| W1 | WARNING | implementation | open | `Work::step` (`crates/gafira-media/src/index.rs:176-182`) and `planned_frames` (`:224-237`) are unchanged by the fix round; the plan is still `duration x fps` and the jump on a mismatch still happens. |
| W2 | WARNING | implementation | open | `keyframe_times` (`crates/gafira-media/src/index.rs:188-220`) still concatenates each chunk's times in packet order and sorts nothing, while the type documents ascending order (`:44-47`) and `keyframe_at_or_before` uses `partition_point` (`:82`). |
| W3 | WARNING | implementation | open | `crates/gafira-testkit/Cargo.toml:10,19` still carries `gafira-time` and `dep:gafira-time` in the `ports` feature, added by `ff58ca0`, a single-writer file of `SC1`. |
| W4 | WARNING | implementation | open | `git grep ARQ-138` over `crates/gafira-media/tests` reaches only the module header of `tests/support/mod.rs:1`. |
| W5 | WARNING | implementation | open | The delivery is still 1401 code lines (measured below); the owner's OK on the size is still not recorded. |
| W6 | WARNING | claim | open | `specs/changes/gafira-media/` has no `spike.md` and `crates/gafira-media/` has no `benches/` directory, so XP6 is still unmeasured. |

### C1 (closed): the cancel token is checked in the keyframe seek

The defect was that `Medium::seek` checked the token only inside the forward-decode loop, so `SeekMode::Keyframe` never checked it and a cancelled seek answered `Ok`.

The fix, `crates/gafira-media/src/medium.rs:109`:

```rust
cancel.check()?;
let Some(keyframe) = self.index.keyframe_at_or_before(time) else {
    return Ok(SeekOutcome::NoKeyframe);
};
```

Evidence that it holds in both seek modes and in both routes of the keyframe mode:

| Route | Where the token is checked | Answer with a cancelled token |
|---|---|---|
| `SeekMode::Keyframe`, medium with keyframes | `medium.rs:109`, before the lookup, the container seek and the `progress.report(1.0)` at `:115` | `Err(MediaError::Cancelled(_))`, no output, no report |
| `SeekMode::Keyframe`, empty index | the same line, before the `NoKeyframe` early return at `:110-112` | `Err(MediaError::Cancelled(_))`, not `NoKeyframe` |
| `SeekMode::Exact`, cancelled before the call | the same line, `:109` (no mode branch precedes it) | `Err(MediaError::Cancelled(_))`, no output, no report |
| `SeekMode::Exact`, cancelled during the decode | `:109` and once per packet at `:119` | unchanged from round 1 |

The check is the first statement of the method and no branch precedes it, so it is mode-independent: the exact path keeps its per-packet check and gains the early one. The implementer's test `arq_139_the_keyframe_seek_checks_the_cancel_token` (`crates/gafira-media/tests/contract.rs:79`) drives both routes — a medium with keyframes and a medium with an empty index — and asserts `Err(MediaError::Cancelled(_))`, the absence of `Ok`, and that no progress was reported. Its RED evidence in `apply-progress.md` (the same call at `c1ba024` answering `Ok(Keyframe(Flicks(1693440000)))` and `Ok(NoKeyframe)`) is the mutant of the rule, which is the proof that the test kills it.

No criterion that passed in round 1 regressed: the seven tests of `tests/keyframe.rs` (AC1 to AC4) are green, including `arq_144_an_empty_index_reports_the_absence_of_a_keyframe`, which keeps its `NoKeyframe` answer under a live token; the contract test of the scan (`arq_139_the_scan_follows_the_progress_contract`) and the determinism test (`arq_140_the_index_does_not_depend_on_the_pool`) are green.

### Suggestion added by this round

**S4.** The new test exercises `SeekMode::Keyframe` only; a cancelled `SeekMode::Exact` call is proved by inspection of the check order (and was proved by round 1's probe P10), but no test of the delivery asserts it. One more arm in `arq_139_the_keyframe_seek_checks_the_cancel_token` would name it.

## Criterion map (AC1 to AC9), re-confirmed at HEAD

| AC | Type | Status | Proof at `c9d696c` |
|---|---|---|---|
| AC1 | integration | covered | `arq_144_the_seek_lands_on_the_previous_keyframe_and_the_exact_seek_on_the_frame` (green) |
| AC2 | unit | covered | `arq_144_a_time_before_the_first_keyframe_returns_the_first_keyframe` (green) |
| AC3 | unit | covered | `arq_144_an_empty_index_reports_the_absence_of_a_keyframe` (green, live token) |
| AC4 | integration | covered | `arq_144_reopening_the_medium_rebuilds_the_index` (green) |
| AC5 | command | covered | `arq_144_opening_a_medium_writes_no_file` (green) |
| AC6 | command | covered | `grep -rn` over `crates/gafira-media/src` and `tests` for the six forbidden paths: no hit (exit 1); clippy gate 0; `#![forbid(unsafe_code)]` at `src/lib.rs:9` |
| AC7 | unit | covered | Both halves: the scan by `arq_139_the_scan_follows_the_progress_contract`, the cancellation of the seek by `arq_139_the_keyframe_seek_checks_the_cancel_token` (the C1 defect). ARQ-139's separate acceptance of the 0.02 step is **not** met on a medium whose packet count differs from its plan (**W1**, open); AC7's own wording does not state that bound. |
| AC8 | property | covered | `arq_140_the_index_does_not_depend_on_the_pool` (pools 1, 2 and 4, green) |
| AC9 | command | covered | forbid 0, V5 0, testkit `ports` 0 (table above) |

Nine of nine criteria have a proof at HEAD. AC7 is covered as written and still subject to W1.

## Artifact integrity (step 3c)

| Check | Result |
|---|---|
| `fb1a621` `docs(gafira-media): slice card` touches only `apply-progress.md` | pass (`1 file changed, 30 insertions`) |
| It precedes the first implementation commit (`ff58ca0`) | pass |
| No implementer commit changed the card | pass: `ff58ca0`, `07ae973`, `6d6eab9` and `9bc20f1` touch no file of `specs/` (0 `specs/` paths each) |
| The `Flow:` line is intact | pass: line 2 is byte-identical to `fb1a621`'s line 2 |
| The amends `a57f8ea` and `98e6d57` touch only that file and state why | pass (`2 insertions, 1 deletion`; `18 insertions`) |
| The fix round changed only ME1 files | pass: `9bc20f1` is `src/medium.rs` and `tests/contract.rs`; the ME2 files (`cache.rs`, `tests/cache.rs`, `lib.rs`) are untouched by it |

## Size (measured, not judged)

`git diff --shortstat`:

| Scope | Lines |
|---|---|
| ME1 delivery, card `fb1a621` to the round-1 commit `c299ae7` | 16 files, 1394 insertions + 43 deletions = **1437** (code only 1401) |
| The fix round `c1ba024...9bc20f1` | 2 files, 85 insertions + 5 deletions = **90** |
| The whole change `fb1a621...c9d696c` | 30 files, 4202 insertions + 48 deletions = **4250** |

ME1 remains at **2.0x the ~700-line guide** and 2.3x its adjusted forecast of 600 (**W5**, open). The fix round adds 90 lines, inside any reading of the guide.

## Risks carried out of round A

- W1 and W2 are code defects, not test gaps: a scrub over material whose packets arrive out of presentation order can land on the wrong keyframe (`keyframe_at_or_before` is a binary search over unsorted data), and a stream whose packet count does not match `duration x fps` breaks the 0.02 step of ARQ-139. Both are inside ARQ-144 and ARQ-139, both are one small change each.
- W3 and W5 outlive the delivery: the testkit manifest is a shared file of the scaffold that two streams read, and the size needs the owner's OK.
- W6 has no owner inside `gafira-media`; it belongs to `explorer`.

---

# Round B — `ME2`, first verify (depth V1)

Commit `6d6eab9 feat(gafira-media): add the frame cache`, notes in `a66ee11 docs(gafira-media): record the ME2 progress`. Base of the delivery: the card amend `98e6d57`.

**Depth.** Per the owner decision O5 in `specs/changes/layer-3/design.md:295-305`, `ME2` and the deliveries after it are verified at **V1**: the gates at HEAD plus a map of every acceptance criterion to its test or command. V1 drops the independent probes, the independent hand mutants and the scratch-export runs of V2, and it keeps the gates, the criterion map and the artifact-integrity checks. This round holds to that: no probe was written and no mutant was run. (`mutation: off` by O4 as well, so there is no `mutants` report and its absence is not a finding.)

## Gates at HEAD (`c9d696c`)

Same commands, same working tree and target dir as round A; the table of round A is the table of this round, since both deliveries are verified at the same commit. In short: `fmt` 0, `clippy` 0, `test` 0 (2311 tests), `deny` 0, and all nine CI policy steps 0, the `Testkit ports feature compiles (ADR 0013)` step included.

## Criterion map (AC10 to AC18)

| AC | Type | Status | Proof at `c9d696c` | Strength |
|---|---|---|---|---|
| AC10 | unit | covered | `arq_145_a_budget_of_zero_keeps_no_frame` (`tests/cache.rs:51`) | Strong: asserts `bytes() == 0`, `len() == 0` and `lookup == Empty`, so a cache that kept the frame but reported 0 bytes fails too. |
| AC11 | unit | covered | `arq_145_a_budget_of_one_frame_discards_the_first` (`:65`) | Strong: budget of one frame, three inserts, `bytes() <= budget`, `len() == 1`, third `Exact`, first not `Exact`. |
| AC12 | unit | covered | `arq_145_the_nearest_frame_wins_the_tie_with_the_earlier_one` (`:195`) | Adequate: 100 ms and 300 ms cached, request at 200 ms, asserts the earlier one. It does not also assert the later one is not chosen, but the `Nearest` value carries the time. |
| AC13 | unit | covered | `arq_145_the_same_file_with_another_resource_id_gives_the_same_frame` (`:120`) | Strong: two different `ProviderAssetId`s, the same `ContentHash`, second lookup is `Exact`, and `len() == 1` (no duplicate entry). |
| AC14 | unit | covered | `arq_145_the_key_is_the_content_hash_the_level_and_the_index` (`:85`) plus `arq_145_a_frame_of_the_original_is_not_the_frame_of_the_proxy` (`:104`) | Strong: each of the three parts varies alone and the key differs; the level test proves the parts are not collapsed. |
| AC15 | unit | covered | `arq_145_a_missing_frame_answers_with_the_nearest_one` (`:216`) | Adequate: a nearer frame of another medium is in the cache and is not returned, so the search is scoped to the medium; the `Nearest` variant is the proof that the exact one is left to the decode. |
| AC16 | unit | covered | `arq_145_the_prefetch_follows_the_direction_of_the_head` (`:238`) and `arq_145_the_prefetch_skips_the_cached_frames_and_stops_at_the_first_one` (`:252`) | Strong: both directions, the cached frame skipped, and the backward walk stops at index 0 instead of wrapping. |
| AC17 | unit | covered | `arq_145_the_trim_leaves_the_cache_at_or_below_the_budget` (`:137`) and `arq_145_a_trim_with_a_bigger_budget_changes_nothing` (`:163`) | Strong: the touched frame survives the trim to one frame's budget, `bytes() <= budget`, and a bigger budget changes nothing. |
| AC18 | command | covered | The policy steps above (forbid 0, V5 0), plus `grep -rnE 'std::fs|std::net|std::thread::spawn|std::time::Instant::now|std::time::SystemTime::now|std::env' crates/gafira-media/{src,tests}` with no hit (exit 1) and `#![forbid(unsafe_code)]` at `src/lib.rs:9` | Command; the crate tree is walked, not one file. |

Nine of nine criteria have a proof. No criterion is proved only by a weak test.

## Implementation read against the criteria

`crates/gafira-media/src/cache.rs` (380 lines) is an LRU over `FrameKey` with a `BTreeMap` of entries and a second `BTreeMap` from the use tick to the key, which is the deterministic shape ARQ-10 and the canonical `clippy.toml` demand (no hash types; clippy gate 0).

- The key is `FrameKey(ContentHash, ProxyLevel, u32)`; `FrameRequest::key()` builds it from the content hash and drops the `ProviderAssetId`, which is exactly §6.5 and AC13/AC14.
- The budget is enforced on insert (`put` ends in `evict_to(self.budget)`) and by `trim`, both in bytes; the loop condition is `size > budget`, so the cache lands *at or below* the budget, not below it (AC11, AC17; the implementer's mutant of `>=` is killed by both tests).
- The byte accounting is the pixels of the frame (`frame_size`), with `RgbaF32` counted at 4 bytes per sample; the rest of the frame is a time, as the notes say.
- The nearest search walks only `FrameKey(content, level, 0)..=FrameKey(content, level, u32::MAX)`, so it never leaves the medium and the level; the tie is broken towards the earlier presentation time (AC12). It compares distances in `i128`, so a signed `Flicks` difference cannot overflow.
- The prefetch emits the keys ahead of the head in the direction of travel, skips what the cache holds, and stops at index 0 going backward (or `u32::MAX` going forward) instead of wrapping (AC16).
- `default_budget(ram)` is the lesser of 1 GiB and 15 % of the RAM, with the RAM injected (ARQ-138), and it does not lower for the 1.5 GB of D-03, which is O2's call on the governor.

The one deviation of design is judged sound: the cache does **not** implement `ports::TileCache`, because that trait's key is a tile rect plus a content key and its value is an `ImageBuf`, while this cache is keyed by the frame index and holds a `VideoFrame`. The card asked for "the same trait shape", and `bytes()`/`trim(budget)` in bytes are that shape, which is what ARQ-44 requires of a cache the governor can trim.

## Product IDs (step 3c)

`git grep <ID>` over `crates/*/tests/` and `crates/*/src/`.

| ID | Proof | Named by |
|---|---|---|
| ARQ-145 | Twelve tests of `crates/gafira-media/tests/cache.rs`; the budget default (`arq_145_the_default_budget_is_the_lesser_of_a_gigabyte_and_a_fifteenth_of_the_ram`), the boundary, the eviction, the nearest and the prefetch. | The tests themselves. **Covered.** |
| ARQ-44 | `bytes()` and `trim(budget)` in bytes, and the "trim to half leaves `bytes()` ≤ half; a bigger budget changes nothing" acceptance, are proved by `arq_145_the_trim_leaves_the_cache_at_or_below_the_budget` and `arq_145_a_trim_with_a_bigger_budget_changes_nothing`. The process-level parts of the row (1 GB limit, 20-page document) belong to `gafira-engine` and are out of this cut. | **Nothing in `gafira-media`**: `git grep ARQ-44` reaches only doc comments of `src/cache.rs` (and the tile-cache tests of `gafira-ports`). **W7.** |
| CAL-14 | Its mechanism (`bench_scrub_1080`) does not exist and needs the proxies of `ME3`; the cache-search half (< 50 ms) has no bench either. | **Nothing.** **W8.** |
| FV-11 (b) | (b) is "after 10 minutes of continuous playback neither the smoothness degrades nor memory grows more than 10 %": it needs the playback clock of `ME4` and a long run, neither of which `ME2` has. What `ME2` contributes to FV-11 is the cache and the nearest frame. | **Nothing.** **W8.** |

## Findings

### WARNING

**W7. `ARQ-44`, a Product ID this card lists, has no test in `gafira-media` that names it.** class: implementation. Ref: `crates/gafira-media/tests/cache.rs`; `specs/02-arquitectura-software.md:1490` (ARQ-44).

Read literally, step 3c makes a missing name CRITICAL. It is read as a WARNING here, the same judgement round 1 of ME1 recorded for ARQ-138 (its W4) and the layer-3-scaffold round recorded for ARQ-01, ARQ-03 and ARQ-06: the row is proved in substance by two `arq_145_` tests, but the ID does not trace to the proof. Fix: one comment line naming ARQ-44 in the trim tests, or a test `arq_44_the_trim_leaves_bytes_at_or_below_the_budget`.

**W8. `CAL-14` and `FV-11 (b)` are listed by the ME2 card with no test or command that proves them, and the card does not mark them partial.** class: ambiguity. Ref: the `## ME2` block of `specs/changes/gafira-media/apply-progress.md:70`; `specs/01-producto-y-calidad.md:311` (CAL-14); `specs/05-funcionalidad-video-y-distribucion.md:143` (FV-11); `specs/changes/layer-3/design.md:248` (P20 counts the cache bench of `gafira-media`).

`CAL-14` fixes "búsqueda en caché < 50 ms" and names `bench_scrub_1080` as its mechanism; `crates/gafira-media/` has no `benches/` directory, so nothing measures the cache lookup. `FV-11 (b)` needs the playback clock of `ME4`. The card inherited from ME1 the shape "FV-11, CAL-14 (partial)" but for ME2 it lists them without the qualifier, and no criterion of the ME2 block (AC10 to AC18) covers either. Fix: mark them partial with the part `ME2` covers (the cache mechanism) and the delivery that closes each (ME3 for the scrubbing bench, ME4 for (b)), or add the bench to the delivery. This is the same shape as round 1's W6 for ARQ-144 and as its S3.

**W9. The ME2 `Flow:` line contradicts the owner decisions O4 and O5.** class: ambiguity. Ref: `specs/changes/gafira-media/apply-progress.md:69` (`verify: V2`, `mutation: archive`); `specs/changes/layer-3/design.md:285` (O4, `mutation: off`) and `:295` (O5, V1 from ME2).

The ME2 block was committed in `98e6d57` before the coordinator recorded O4 and O5 (`93f562d`), and it was never amended. As the coordinator's own routing did, this round ran at V1 and no mutants ran; the card is the file the next agents read, so the stale fields can mislead. Fix: a `slice card amend` that sets `verify: V1 | mutation: off` in the ME2 Flow line and states why (O4, O5). Not a defect of the delivery; a stale acceptance artifact.

### SUGGESTION

**S5.** `FrameCache::lookup` marks a frame as recently used only on an `Exact` hit (`cache.rs:249-252`); a `Nearest` hit returns `Arc::clone` without touching the recency (`:254-257`). The frame the scrub just displayed is therefore the next one the eviction may take. No criterion demands otherwise (AC17 speaks of "the most recently used entries", which the exact path and `put` establish), but a nearest hit is a use in the sense §6.5 describes.

## Artifact integrity (step 3c)

| Check | Result |
|---|---|
| The ME2 criteria were committed before the first implementation commit | pass: `98e6d57` (`slice card amend`, 18 insertions) precedes `6d6eab9` (`git merge-base --is-ancestor 98e6d57 6d6eab9` holds) |
| It touches only `apply-progress.md` | pass (`1 file changed, 18 insertions`) |
| Its body states why | pass: "Adds the ME2 section ... so the criteria of the second delivery of this stream are committed before its first implementation commit." |
| No implementer commit changed the ME2 block | pass: `6d6eab9` touches no file of `specs/`; `2c3dff5` and `a66ee11` only append below the card (0 deletions) |
| The ME2 Flow line is present and readable | pass, and stale: **W9** |
| The delivery touches only `crates/gafira-media/**` | pass: `6d6eab9` is `src/cache.rs`, `src/lib.rs`, `tests/cache.rs`; the testkit manifest, `gafira-ports`, `gafira-doc` and `.odd/state.json` are untouched, so the shared-file rule of the design is respected and the W3 deviation of ME1 is not repeated |

## The notes of the delivery, checked

- "649 insertions, 3 files, `98e6d57...HEAD`": measured `git diff --shortstat 98e6d57 6d6eab9` = `3 files changed, 649 insertions(+)`. Exact.
- "380 lines for the cache module, 5 for the exports, 264 for the tests": the three files are 380 + 5 + 264 = 649. Exact.
- "21 tests, 0 failed" at the commit: at HEAD `cargo test -p gafira-media --locked` gives 22 (the 22nd is ME1's `arq_139_the_keyframe_seek_checks_the_cancel_token`); the cache binary gives the 12 the note claims. Consistent.
- "No deviation of scope": confirmed above.
- The four negative controls: the design marks the cache key, the eviction order, the nearest-frame search and the budget boundary as critical, and the notes record one mutant for each, killed by the named test. This round did not re-run them (V1), and the two `arq_145` tests that kill the budget boundary and the eviction order are green at HEAD.

## Size (measured, not judged)

| Scope | Lines |
|---|---|
| The delivery, card amend `98e6d57` to `6d6eab9` | 3 files, 649 insertions = **649** |
| The notes `a66ee11` (`apply-progress.md`) | 56 insertions (not code) |
| The whole change so far, `fb1a621...c9d696c` | 30 files, 4202 insertions + 48 deletions = **4250** |

ME2 is **649 lines against the ~700-line guide**: below it, and within the x1.4 band of its forecast (470 authored, 660 adjusted). The delivery does not exceed the guide, so no owner approval is needed for the size.

## Risks carried out of round B

- CAL-14's FIJADA figure for the cache search is unmeasured, and P20 of the design says `gafira-media` contributes the cache bench. The risk is that the figure is met only by argument until a bench exists (**W8**).
- The nearest hit does not refresh the LRU position (**S5**): a scrub that leans on nearest hits can evict the frames it is showing. Cheap to change if the design wants it.
- The cache's default budget is not capped by the 1.5 GB of D-03 inside the crate; that is O2's design (the governor trims first), and the governor does not exist yet, so the cap is unproven end to end.

---

## Notes on this run

- The shared worktree `/tmp/gafira-wt/me` was deleted by something outside this session while `cargo test --workspace` ran (the directory disappeared entirely; `git worktree list` then showed it as `prunable`). It was recreated with `git worktree prune` and `git worktree add /tmp/gafira-wt/me feat/layer-3-media` in the main repository, at the same commit `c9d696c`, with a clean tree; `cargo fmt`, `clippy`, `deny`, the crate tests and the policy steps then ran there against the same `CARGO_TARGET_DIR`. The `cargo test --workspace` run finished before the deletion, on the same clean tree and commit, and its 2311 passing tests are the ones recorded. No file of the tree was modified by this session: only this report and `verification.json` are written.
- The target dir `/tmp/gafira-verify-me-target` is freed with `cargo clean --target-dir` at the end of the round.

next: tdd-apply (ME1: W1, W2, W3, W4, W5; ME2: W7; then ME3), with explorer for W6 (XP6) and the coordinator for W8 and W9 (card attribution and the stale Flow line)
