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

---

# Round C — `ME3`, first verify (depth **V1**)

Delivery `ME3` of `layer-3`, crate `gafira-media`, verified at `5a50147` (the code tree is `bfcc9e5`). Mode full, `mutation: off` (owner decision O4), tier T0, depth **V1** (owner decision O5: the gates at HEAD plus the map of every criterion to its test or command; no independent probes from a scratch export and no independent hand mutants). This is round 1 of the delivery, so step 1c and step 4b do not apply.

## Gates at HEAD (`5a50147`), each run alone

`CARGO_TARGET_DIR=/tmp/gafira-verify-me3-target`, `CARGO_INCREMENTAL=0`, `df -h /` at 115 GB free before the first build.

| Gate | Command | Exit |
|---|---|---|
| Format | `cargo fmt --check` | 0 |
| Clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| Test | `cargo test --workspace --locked` | 0 |
| Deny | `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`) | 0 |
| CI policy | the nine `run:` blocks of the `policy` job, extracted literally and run with `bash -euo pipefail` | 0 (each) |

`cargo test --workspace --locked`: 185 test binaries, **2320 tests passed, 0 failed**. The four binaries of the delivery: `proxy` 9, `cache` 12, `contract` 3, `keyframe` 7 (31 in the crate), all green, which matches the implementer's count.

The nine policy steps, each in its own `bash -euo pipefail`: Format 0, `CLI has no UI dependencies (ARQ-07)` 0, `Domain crates forbid unsafe code` 0, `Domain crates respect the layer table (V5)` 0, `Domain clippy.toml copies are in sync (V12)` 0, `Layers 0-2 know no language (ARQ-107)` 0, `EditToken only in the edit crates (V13)` 0, `Every core command has a property test (V14)` 0, `Testkit ports feature compiles (ADR 0013)` 0. No `head` was applied to a cargo pipe, so no exit code is a `SIGPIPE` 101.

AC24's own command, run here: `#![forbid(unsafe_code)]` at `crates/gafira-media/src/lib.rs:9`; `grep -rn` over `crates/gafira-media/src` and `crates/gafira-media/tests` for `std::fs`, `std::net`, `std::thread`, `std::time::Instant`, `std::time::SystemTime`, `std::env` and `unsafe ` returns no hit (exit 1 in each of the seven searches). The `std::io::Write` and `std::path` imports that `proxy.rs` does use are not in the list ADR 0016 bans. The V5 row is at `.github/workflows/ci.yml:95` and passes.

## Criterion map (AC19 to AC24)

| AC | Type | Proof | Result |
|---|---|---|---|
| AC19 | integration | `arq_146_the_copy_lands_in_the_user_cache_and_not_in_the_project` (`tests/proxy.rs:151`), `arq_146_the_two_levels_of_a_medium_are_two_copies` (`:442`) | covered |
| AC20 | integration | `arq_146_a_cancelled_generation_leaves_no_half_written_file` (`:198`) plus the cancel points of `check_progress_contract` inside `arq_146_the_generation_follows_the_progress_contract` (`:372`) | covered |
| AC21 | integration | `arq_146_an_export_reads_the_original_and_never_the_copy` (`:326`) proves the reader, not the exported file | **partial** (W11) |
| AC22 | unit | `arq_146_the_default_budget_of_the_copies_is_ten_gigabytes` (`:244`), `arq_146_the_budget_discards_the_least_used_copies` (`:254`), `arq_146_a_budget_of_zero_keeps_no_copy` (`:300`) | covered, with W13 and S4 |
| AC23 | unit | `arq_146_the_generation_runs_in_the_background_with_progress` (`:415`), `arq_146_the_generation_follows_the_progress_contract` (`:372`) | covered, with S5 |
| AC24 | command | the gates above plus the seven `grep`s | covered |

**AC19, the copy lives in the user cache and not in the `.gafira`.** The store writes only `root.join("proxies")` (`proxy.rs:381-383`), and the only root a caller can pass is the cache: nothing in the crate reaches the `.gafira`. The test does what the criterion asks: the copy is readable at `store.path(&key)`, that path starts with `/home/user/.cache/gafira`, every file the double holds is under the cache root or is the project file itself, and the project file is byte-identical afterwards. It also pins the naming half (§7.1, "the proxies do not go in the file"): the two levels are two files and a second store on the same cache names the same copy. The test's first version compared the expected name against `store.path(&key)` and H1 survived it; the current assertion is against the cache root, so the mutant dies (checked by reading `:176-187`).

**AC20, a cancel leaves the cache usable and no half-written final file.** This is the strongest test of the delivery, and the rule it proves is real in the code: `generate` writes to `temporary_path(&final_path)` (`proxy.rs:427-432`), and `replace` is the only statement that can put bytes under the final name (`:287-290`); every early return between the create and the replace removes the temporary name (`:283`, `:288`), and `write_chunks` returns before writing when `create` fails (`:314`). The cancel checkpoints are at `:317` (before every chunk) and `:328` (after the last one, before the rename), so a cancel can land at any frame boundary and never reaches `1.0`. The three assertions of the test cover the criterion: `Err(MediaError::Cancelled(_))`, `paths() == [PROJECT]` (no final file *and* no leftover `.part`), `producer.asked() == 1` (it stops at the cancel), `reports() == [0.2]` (no `1.0` after the cancel) and `visible() == [false]`, a `Progress` double that asks the file system whether the final name is taken at every frame. The last part of the test generates the same copy again, which is "the cache stays usable". The one file-level gap is that the *success* path never asserts the `.part` is gone (S6).

**AC21, an export never uses a proxy.** `read` answers `ProxySource { level: Original, path: original }` when `use_of == ProxyUse::Export`, before it even looks at the registry (`proxy.rs:231`), and `ProxyKey::of` refuses to build a key at `ProxyLevel::Original` (`:57-62`), so an export cannot even name a copy. `arq_146_an_export_reads_the_original_and_never_the_copy` proves both halves of that rule and also that a preview of the same key does answer the copy, so the test would fail if the branch were inverted (H4 kills it). The criterion's second clause — "the exported file has the resolution of the original" — has no proof in this delivery (W11).

**AC22, the budget, the 10 GB default and the least-used eviction.** `DEFAULT_PROXY_BUDGET = Bytes(10 << 30)` (`:29`) is asserted exactly; the eviction walks the `recency` map from its lowest tick (`:366-378`), and the test makes the *older* copy the most recently used one through `read`, so insertion order alone cannot pass it. The `>` boundary is proved: three 4-byte copies under an 8-byte budget leave two, and `bytes() <= budget()`; the `>=` mutant empties the cache and fails the same test. The reservation in the budget of what the store wrote (W13) and the GiB reading of "10 GB" (S4) are the two caveats.

**AC23, background priority, progress and the cancel token.** The report carries `Priority::Background` (`:296`) and the test asserts the field. It is a label: `ports::WorkerPool` has no priority, so the store cannot enqueue its work at that priority, and the implementer recorded the choice. Progress is the frames done over the frames planned (`planned = frames + 1`, `:275`), which keeps the fraction below `1.0` until the rename, and the test asserts the exact sequence `[0.2, 0.4, 0.6, 0.8, 1.0]` and the report count, which is what stops the contract checker from passing vacuously (the implementer's note on that is correct: `check_progress_contract` cancels at reports `{1, n/2, n-1}`, and `n = 5` here, so it does exercise cancels at the first, the middle and the last frame). The cancel checkpoints are the two at `:317` and `:328`; the contract check at `k = 4` reaches the post-loop checkpoint, which the file-system assertions of the AC20 test do not.

## Product IDs (step 3c)

| ID | Named by a test? | Proves its acceptance? |
|---|---|---|
| ARQ-146 | yes, 10 hits in `tests/proxy.rs`, and every test name carries it | yes for the cache, the cancel and the "export never uses a copy" rules; not for the resolution of the exported file (W11) |
| FV-22 | only the module header of `tests/proxy.rs:1` | partially: (b) is proved, (a) and (c) are not (W12) |
| ARQ-43 | **no** — `git grep ARQ-43 -- crates/gafira-media` is empty | the per-frame checkpoint is proved under ARQ-146; the ≤ 100 ms bound has no bench (W10) |

## Artifact integrity (step 3c)

- `cbe83c5 docs(gafira-media): slice card for ME3` touches only `specs/changes/gafira-media/apply-progress.md` (+27 lines): the `## ME3` block, its `Flow:` line at `:193` and AC19 to AC24. Nothing else in the commit.
- `bfcc9e5 feat(gafira-media): add the proxy store` touches only `crates/gafira-media/src/lib.rs` (+4), `src/proxy.rs` (440), `tests/proxy.rs` (467) and `tests/proxy_support.rs` (197). No file outside the crate.
- `5a50147 docs(gafira-media): record the ME3 progress` touches only `apply-progress.md`, and `git diff cbe83c5..HEAD -- apply-progress.md` starts at line 207: every changed line is inside `## ME3 (tdd-apply)`. The card block (191-204) is byte-identical to `cbe83c5`, including the `Flow:` line and the `Deliveries`/`Scope`/`Non-goals`/`Plan` fields. The notes sit in the right section and the placeholder "Gates per commit" line the card left is the 1 deletion.
- The `Flow:` line of this delivery, `full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1`, matches O4 and O5 of the design, unlike the stale ME2 line W9 already recorded.
- The delivery adds no `.cargo/mutants.toml` exclusion (no file of that or `Cargo.toml`/`Cargo.lock` is touched), and `mutation: off` needs none.
- The ME3 card is byte-identical on `develop` and on this branch (checked with `diff`), so the amend that fixed the ME2 card does not disturb this round.

## The notes of the delivery, checked

The six hand mutants of the list are the four critical rules of the design for `ME3` (naming, budget, the checkpoints, "an export never uses a proxy") split into six behaviours, and each names the test that kills it. The two survivors that forced a test to be strengthened (H1, H2) are visible in the committed tests: `:176-187` for H1 and the `visible()` double at `:131-146` for H2, which does die on the first frame against a store that writes to the final name. The RED evidence is recorded output, not something git can reproduce (no skeleton commit exists), and V1 does not ask for that re-run.

## Size (measured, not judged)

`git diff --shortstat cbe83c5...HEAD` -> **5 files changed, 1163 insertions(+), 1 deletion(-)**. Without the notes commit the delivery is 1108 insertions in 4 files, of which 440 are the store, 467 the tests, 197 the extra double and 4 the exports.

Against the `AGENTS.md` signal of ~700 lines per delivery that is **1,58×**, and against the design's forecast for `ME3` (`design.md:103`: 460, or 645 with the x1,4 factor) it is **2,4×** and **1,7×**. Above the ceiling the delivery needs the owner's explicit OK; that is W14, and the decision is the coordinator's. The 197-line double is the avoidable part (S7).

## Findings

### CRITICAL

None.

### WARNING

**W10. ARQ-43, a Product ID of the card, has no test, bench or command that names it.** class: implementation. Ref: `apply-progress.md:194`; `git grep ARQ-43 -- crates/gafira-media` (empty); `specs/02-arquitectura-software.md:1489`.

Read literally, step 3c makes a missing name CRITICAL. It is read as a WARNING here, the same judgement ME1 round 1 (W4, ARQ-138) and ME2 round 1 (W7, ARQ-44) recorded, for the same reason: the rule is proved in substance and only the trace is missing. The mechanism of ARQ-43 in this delivery is the per-frame checkpoint, which the AC20 and AC23 tests do prove; the figure itself, "cancel ≤ 100 ms", is measured by `bench_cancel_segment` in `gafira-segment` (§6.7), and the crate has no `benches/` directory, so nothing here can measure it. A per-frame checkpoint does not by itself bound the response to 100 ms, because one chunk write is outside the check.

Fix: name ARQ-43 in the test that proves the checkpoint (one comment line is enough), and say in the card that the 100 ms figure is the bench of `gafira-segment` and not this crate's.

**W11. AC21's second clause, "the exported file has the resolution of the original", has no proof in this delivery.** class: ambiguity. Ref: `apply-progress.md:199`; `specs/02-arquitectura-software.md:1252` (ARQ-146, third acceptance); `crates/gafira-media/tests/proxy.rs:326`.

The store owns and proves the half that is its own: an export reads the original path at `ProxyLevel::Original` and never the copy. The written file does not exist until the exporter of `ME5` exists, and the implementer declared this as "a deviation of the evidence, not of the behaviour". The criterion as written cannot be observed in `ME3`, so the card attributes to this delivery an outcome of another one. If the coordinator wants the card literal it needs the same "partial" marker the ME2 card got in `1da2b21`; a strict reading of step 3c would make this CRITICAL and route to `specifier`, and it is not graded that way because the behaviour the store owns is proved and the missing half is scheduled by this same change.

Fix: mark ARQ-146 and AC21 as partial in the card (export half to `ME5`), as ME2 did with CAL-14 and FV-11 (b).

**W12. FV-22 is listed without a partial marker, and only (b) of its acceptance is proved.** class: ambiguity. Ref: `apply-progress.md:194`; `specs/05-funcionalidad-video-y-distribucion.md:350-357`; `crates/gafira-media/tests/proxy.rs:1`.

FV-22 asks for four things: automatic generation for material over 1080p or with expensive codecs, MJPEG at 540p/720p in the user cache, a 10 GB limit discarding the least used, and the export always on the original. The delivery has the store, the format name, the limit and the export rule; the *automatic* trigger depends on the codec and the asset metadata (`gafira-engine`, `ME5`), and the manual create/delete of the Copies belongs to the Phase 2 UI. The only appearance of FV-22 in the tests is the module header at `:1`, which is not a test name. (c) "deleting the cache loses no project data" has no test, although `evict_to` removing only its own files is the code that makes it true.

Fix: scope FV-22 in the card to the store half, and let `ME5`/the engine own the trigger.

**W13. The 10 GB budget only counts the copies this process wrote; a cache left by an earlier session is neither counted nor collected.** class: implementation. Ref: `crates/gafira-media/src/proxy.rs:286-345` (`size` starts at `Bytes(0)`), `:366-378` (eviction removes only registered keys); the implementer's note at `apply-progress.md:267`.

The store keeps no manifest and never enumerates the proxy directory, so a second session starts from zero, regenerates what it needs and can push the real directory well past 10 GB; a `.part` file left behind by a killed process is never collected either, because `read_dir` is not used anywhere in the crate. The note's justification, "`ports::FileSystem` has no call that gives the size of a file", overstates the port: `open` returns `Box<dyn ReadSeek + Send>` (`crates/gafira-ports/src/system.rs:34`), and `Seek::seek(SeekFrom::End(0))` gives the length, so the store could seed itself from `read_dir` at construction and also drop stale `.part` files. The criterion is therefore proved only within one `ProxyStore` instance, and FV-22 (c) is met by accident rather than by test.

Fix: seed the store from `read_dir` (size by `open` + `seek(End)`, dropping `*.part`) or record the manifest, and say in the card that the budget is per-session until that lands.

**W14. The delivery is 1,58× the ~700-line signal.** class: implementation. Ref: the size section above; `specs/changes/layer-3/design.md:103` (forecast 460, 645 adjusted); `AGENTS.md`, "Delivery rules".

1108 code lines (1163 with the notes) against ~700, and 1,7× its own adjusted forecast. Above the ceiling the delivery needs the explicit OK of the owner; the split is 440 store, 467 tests, 197 double, 4 exports, so the tests are not the outlier — the extra double is (S7).

### SUGGESTION

**S4.** `DEFAULT_PROXY_BUDGET` is `10 << 30`, that is 10 GiB, while FV-22 and ARQ-146 say "10 GB (PROPUESTA)" without stating whether the gigabyte is decimal. If the owner reads GB as 10^9 the constant is 7 % above the spec. It follows `DEFAULT_BUDGET` of `ME2` (1 GB = `1 << 30`), and the test pins the binary reading, so this is a spec wording question, not a defect: one line to change if the owner answers.

**S5.** The contract test passes `max_step = 0.25` to `check_progress_contract`, while ARQ-139's acceptance fixes a rise of at most 0,02 between two reports *on the reference input*. The finest step the model can give is `1/(frames+1)`, so 0,02 holds from 50 frames up and a 4-frame copy (the test's producer) reports 0,2. Naming the reference copy of this operation (≥ 50 frames) or saying in ARQ-146 that the per-frame model is the reference would close the gap between the row and its proof.

**S6.** The success path never asserts that the `.part` name is gone: the AC19 test only checks that every held file is under the cache root, and a leftover `...540p.mjpeg.part` satisfies that. `replace` cannot leave it (the double's and the OS's rename both remove the source), so this is a cheap extra assertion rather than a defect. Likewise, no test injects a failing `FileSystem` or a producer that returns `Err`, so the three error paths of the temporary rule (`create`, `write_all`, `replace`) are proved by reading `:278-290`, not by a test.

**S7.** The 197-line writable double duplicates a home that already exists: `crates/gafira-testkit/src/ports/file_system.rs` is a `FakeFileSystem` that panics ("A stub: the media stream fills it"), and the design's shared-files table gives `crates/gafira-testkit/src/ports/**` to each stream for "their own files" but never assigns `file_system.rs`. Filling that stub, or declaring the file in the table, would cut the largest avoidable part of the over-size — and would let `ME4`/`ME5` reuse it. The implementer's stated reason (`tests/support/mod.rs` must stay read-only for ARQ-144 and `#[allow(dead_code)]` is banned) does not cover the testkit copy, which lives behind the `ports` feature.

**S8.** `write_chunks` reads `producer.frames()` a second time (`proxy.rs:316`) although `generate` already read it for `planned` (`:271`). A producer whose `frames()` changes between the two calls can make the fraction reach `1.0` before the rename, which is exactly what ARQ-139 forbids. Reading it once and passing it down, or documenting `frames()` as a stable count, removes the hole; the `Chunks` double is stable, so no test sees it today.

**S9.** `§6.3` of `02` puts "generación de proxies" on the I/O thread with priority "Normal", while FV-22 and ARQ-146 demand `Background`. The delivery follows the latter, but the store can only *report* the priority (`ports::WorkerPool` carries none), so the contradiction will surface when `gafira-engine` enqueues the work. Reconciling the two rows in the spec would decide who owns the enqueue.

## Notes on this run

- Both deliverables were written on `develop`'s copies of the two files, which hold the ME1 round 2 and ME2 round 1 rows, because this branch's copies predate them. The ME1 and ME2 rounds are byte-identical to `develop`'s; the report gains this section and nothing else (131 insertions, 0 deletions against the copy it was built on), and `verification.json` gains the ME3 round plus the one-line `delivery` field, which now reads `ME3` and no longer `ME1+ME2` (each round keeps its own `delivery` label).
- `develop` already contains the three ME3 commits (`33715bf Merge branch 'feat/layer-3-media-3' into develop`) while this first verify of the delivery was still to run, and that merge kept `develop`'s `verification.json`. Recorded because it changes what a fix round has to rebase on; not a finding of this round.
- Nothing outside the two files was touched: the worktree is clean apart from them, no commit was made, `.odd/state.json` was not opened for writing, and the build dir `/tmp/gafira-verify-me3-target` is freed with `cargo clean --target-dir` at the end of the round.
- Verdict reasoning: no CRITICAL, so PASS; the five WARNINGs stay open and become follow-ups on PASS. `next` is `tdd-apply` for `ME4`, which is the next delivery of the card.
- The verified tree is `5a50147`. After this section was written, `03cf92a` recorded the round and amended the ME3 card (ARQ-43 and FV-22 marked partial, the second clause of AC21 deferred to the exporter of `ME5`, which moved to `layer-3b` by O6); that commit touches the three files of the round and no code, so the gates above still describe the tree they were run on. The `open` status of W10 to W12 in `verification.json` is the coordinator's to update; nothing in the code changed for them.

next: tdd-apply (ME4), with the coordinator to amend the ME3 card for ARQ-43, ARQ-146/AC21 and FV-22 if it wants the card literal (W10, W11, W12) and for the size (W14)
