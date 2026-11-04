## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: the media stream opens with the keyframe index, the first piece of `gafira-media`, and it is independent of the render stream
Product IDs: ARQ-138, ARQ-139, ARQ-140, ARQ-144, FV-11, CAL-14 (partial) (source: specs/02-arquitectura-software.md §4.17.8 and §6.5, specs/05-funcionalidad-video-y-distribucion.md §A6)
Scope: the media stream of `specs/changes/layer-3/design.md`: `ME1` (the keyframe index, the seek by keyframe and the exact seek with forward decode), `ME2` (the frame cache, the LRU, the byte budget, the prefetch policy and the nearest cached frame), `ME3` (the proxy store, its byte budget, its background priority and its cancellation), `ME4` (the playback clock, the A/V sync and the frame dropping) and `ME5` (the video exporter, decode to compose to encode to mux, the progress, the cancellation and the audio track), all in `gafira-media`, plus the crate-level contract the stream inherits (purity through ports, cancellation and progress, determinism).      Non-goals: the render crate, any change to `gafira-ports`, any change to `gafira-doc`, and the timeline model (`O1`).
Deliveries: ME1, ME2, ME3, ME4, ME5, serial; each verified and merged before the next. `ME1` opens the crate and its acceptance criteria are the ones written first in this card; `ME2` to `ME5` add their own criteria in their own section, below.
Acceptance:
  AC1 Given a medium with three keyframes at 0 s, 2 s and 4 s, When a seek to 3,5 s runs, Then `SeekMode::Keyframe` lands on the 2 s keyframe and `SeekMode::Exact` lands on the 3,5 s frame [integration]
  AC2 Given a medium whose first keyframe is at 5 s and a seek to 3 s, When the index is searched, Then the 5 s keyframe is returned [unit]
  AC3 Given an empty index, When it is searched, Then it does not panic and reports the absence of a keyframe [unit]
  AC4 Given a medium that is closed and opened again, When the index is searched, Then the index is rebuilt and the search results are the same [integration]
  AC5 Given a project that has been opened, When the user cache is inspected, Then it holds no new index file [command]
  AC6 Given the crate, When its sources are read, Then there is no `std::fs`, no `std::net`, no `std::thread::spawn`, no `std::time::Instant::now`, no `std::time::SystemTime::now` and no `std::env`; the time comes from `ports::Clock` and the file system from `ports::FileSystem` [command]
  AC7 Given a long operation, When it runs, Then it checks the cancel token at least once per frame, emits a `Progress` with a non-decreasing fraction in `[0, 1]` that only reaches `1.0` on success, and the fraction follows units done over units planned and not elapsed time [unit]
  AC8 Given the same input, When the crate runs the operation twice, Then the bytes are the same, and with a pool of 1, 2 and 4 threads the index is identical [property]
  AC9 Given the crate, When the CI policy steps run, Then `gafira-media` still has `#![forbid(unsafe_code)]`, its V5 row still passes, and the testkit doubles this delivery fills compile with the `ports` feature on [command]
Plan: the delivery adds the keyframe index module to `crates/gafira-media/src/` and fills the media doubles of the testkit. The testkit is the single writer of `crates/gafira-testkit/src/ports/**` for the media files: this delivery owns `container.rs`, `codec.rs`, `decoder.rs`, `encoder.rs` and `muxer.rs`; the render stream owns `render_backend.rs` and `page_renderer.rs`. `crates/gafira-testkit/src/lib.rs` is not touched, because `SC1` already declared the whole module list. The crate declares `gafira-testkit` with `features = ["ports"]` in its `dev-dependencies`, which adds two lines to `Cargo.lock`; that is why this delivery is serial in the media stream. The critical code is the keyframe lookup boundaries, the seek direction, the empty index and the time before the first keyframe (design "Critical code"), so those carry the hand mutants.
Owner decisions touched: none (checked D-01 to D-57: the keyframe index is a domain capability and ARQ-144 keeps it in memory, so no project format or persistence change).

# Apply progress: gafira-media

## ME1 (tdd-apply)

Commits:

- `ff58ca0` `feat(gafira-testkit): fill the media doubles`: the five media doubles of the testkit (`container.rs`, `codec.rs`, `decoder.rs`, `encoder.rs`, `muxer.rs`) and the `gafira-time` optional dependency of the `ports` feature. 7 files changed, 396 insertions(+), 42 deletions(-).
- `07ae973` `feat(gafira-media): add the keyframe index and the seek`: the crate (`lib.rs`, `ports.rs`, `index.rs`, `medium.rs`), its tests (`tests/keyframe.rs`, `tests/contract.rs`, `tests/support/mod.rs`), the dev-dependency on the testkit and the second line of `Cargo.lock`. 9 files changed, 963 insertions(+).

Measured: `git diff --shortstat feat/layer-3-scaffold...HEAD` -> 16 files changed, 1423 insertions(+), 42 deletions(-). The delivery itself, from the Slice card `fb1a621`: `git diff --shortstat fb1a621...HEAD` -> 16 files changed, 1394 insertions(+), 43 deletions(-), that is 1437 authored lines against the ~700 ceiling of the layer-3 design; the code without these notes is 1401 of them (1359 insertions and 42 deletions). The breakdown is 438 lines for the doubles of the testkit, 450 for the crate and 540 for the tests; the owner may want to approve the size or to cut the delivery into chained slices.

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, and the ARQ-107 step), each by extracting its `run:` from `ci.yml`. Plus the crate gate: `cargo clippy -p gafira-media --all-targets --locked -- -D warnings` and `cargo test -p gafira-media --locked`. The `ports` feature belongs to the testkit, so its own step compiles it: `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` and `cargo test -p gafira-testkit --features ports --locked`. Every commit was gated in its own state, with the tree of the other commits aside.

### RED evidence

The tests were written against the acceptance criteria of the Slice card and the implementation was then reduced to a skeleton to capture the RED: the build scans nothing and the lookup returns the zero time. Every failure below is an assertion failure, not a compile error, and the implementation was restored afterwards (`cargo test -p gafira-media --locked`: 9 passed, 0 failed).

- AC1 `arq_144_the_seek_lands_on_the_previous_keyframe_and_the_exact_seek_on_the_frame`: `assertion left == right failed, left: Keyframe(Flicks(0)), right: Keyframe(Flicks(1411200000))`.
- AC2 `arq_144_a_time_before_the_first_keyframe_returns_the_first_keyframe`: `left: Keyframe(Flicks(0)), right: Keyframe(Flicks(3528000000))`.
- AC3 `arq_144_an_empty_index_reports_the_absence_of_a_keyframe`: `left: Some(Flicks(0)), right: None`.
- AC4 `arq_144_reopening_the_medium_rebuilds_the_index`: `left: 0, right: 3` (the reopened medium held no keyframe).
- AC5 `arq_144_opening_a_medium_writes_no_file`: `left: 0, right: 3`.
- AC6 (command): `grep -rn` over `crates/gafira-media/src` and `crates/gafira-media/tests` for `std::fs`, `std::net`, `std::thread::spawn`, `std::time::Instant::now`, `std::time::SystemTime::now` and `std::env`: no hit in any of the six.
- AC7 `arq_139_the_scan_follows_the_progress_contract`: `left: Err(NotCompleted), right: Ok(())`.
- AC8 `arq_140_the_index_does_not_depend_on_the_pool`: `left: 0, right: 5`.
- AC9 (command): the policy steps, all of them exit 0 (see the gates).

### Negative controls

The four hand mutants of the design ("Critical code" of `ME1`), applied one at a time to `KeyframeIndex::keyframe_at_or_before` and reverted after each run. None of them was committed and the tree is clean after the four.

- The boundaries of the lookup: `*t <= time` became `*t < time`, so the keyframe at the requested time was skipped. Killed by `arq_144_a_seek_to_the_time_of_a_keyframe_lands_on_it`: `left: Keyframe(Flicks(0)), right: Keyframe(Flicks(1411200000))`.
- The direction of the seek: the lookup returned the first keyframe at or after the time instead of the one at or before it. Killed by `arq_144_the_seek_lands_on_the_previous_keyframe_and_the_exact_seek_on_the_frame`: `left: Keyframe(Flicks(2822400000)), right: Keyframe(Flicks(1411200000))`.
- The empty index: the lookup indexed `self.times[0]` and panicked on an empty index. Killed by `arq_144_an_empty_index_reports_the_absence_of_a_keyframe`: panic at `crates/gafira-media/src/index.rs:84`.
- The time before the first keyframe: the lookup returned the last keyframe instead of the first one. Killed by `arq_144_a_time_before_the_first_keyframe_returns_the_first_keyframe`: `left: Keyframe(Flicks(5644800000)), right: Keyframe(Flicks(3528000000))`.

### Choices and deviations

- **The `ports` feature of the testkit gained `gafira-time` (a file outside the scope of the delivery).** The `Demuxer` port takes the time type of the ports in `seek`, so a double of the testkit cannot implement it without the crate, and the container double of the task needs packets with a time. The manifest of the testkit is a single-writer file of `SC1` (design, "Shared files"), so this is a deviation to report: the line added is `gafira-time = { path = "../gafira-time", optional = true }` and the feature gains `dep:gafira-time`. The default tree of the testkit does not change (the dependency is optional), so the V5 row, the ARQ-107 step and `cargo deny` are untouched; the `Cargo.lock` gains the line `gafira-time` under `gafira-testkit`.
- **The pool is a trait of the crate, `WorkerPool`, and not a number.** The crate creates no thread (ARQ-02), so the pool has to come from the caller; a trait says so and the delivery keeps the promise that the result does not depend on it. The container port is sequential (`read_packet` gives one packet at a time), so the pool takes the extraction of the keyframe times out of the scanned records: the records are split into as many chunks as the pool has workers and the results are merged by chunk. The double of the test runs the jobs in the reverse order, so a result that depended on the order would fail.
- **The planned units of the progress come from the duration and the frame rate of the stream** (`duration * fps`), because the number of packets is unknown before the scan reads them all. A stream without a duration plans nothing and the operation reports only its end.
- **The seek reports the end of the decode only.** The number of frames between the keyframe and the target is unknown before the decode, and the fraction of the whole stream would jump from the keyframe to the end, so the seek reports `1.0` when it lands and nothing when it is cancelled.
- **The lookup of the keyframe is inclusive**: a seek to the time of a keyframe lands on that keyframe, and the time before the first keyframe lands on that first keyframe (the keyframe that covers the time). The exact seek lands on the first frame at or after the requested time.
- **The doubles that need state keep the name of the stub and a companion carries the state.** The own test of the testkit (`tests/ports_feature.rs`, not owned by this delivery) names `FakeDecoder`, `FakeEncoder` and `FakeMuxer` as values, so those keep the unit form and are the empty doubles; the useful ones are `FrameDecoder` (a frame per packet), `PacketEncoder` (a packet per frame) and `RecordingMuxer`. `FakeContainer` reads its medium from the source it opens, a JSON document of streams and packets, so it needs no state either.
- **The RED was captured by replaying the tests against a skeleton**, not in the order of one test at a time: the tests and the implementation were written in the same session and the skeleton replay shows that every test detects the missing behaviour. The evidence above is the real output of that replay.
- **The budget is reported, not enforced.** `MediaError` has no variant for a scan that runs slow, and `gafira-ports` is out of the scope of the delivery, so `IndexReport` carries the elapsed time and the budget and `within_budget()` tells the caller.


## ME2

Flow: full | level: L2 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: the media stream continues with the cache the scrubbing reads from
Product IDs: ARQ-44, ARQ-145, CAL-14, FV-11 (b) (source: specs/02-arquitectura-software.md §6.5, specs/05-funcionalidad-video-y-distribucion.md §A6)
Scope: `ME2` of `specs/changes/layer-3/design.md`: in `gafira-media`, the cache of frames: the LRU, the byte budget, the prefetch policy in the direction of the head and the nearest cached frame.      Non-goals: the proxies (`ME3`), the playback clock (`ME4`), the exporter (`ME5`), the tile cache of the render stream, and any change to `gafira-ports`.
Acceptance:
  AC10 Given a budget of 0, When a frame is cached, Then no frame is kept and `bytes()` gives 0 [unit]
  AC11 Given a budget that fits a single frame, When the third frame is asked for, Then the first is discarded and `bytes()` is at most the budget [unit]
  AC12 Given two frames at the same distance from the requested time, When the nearest is chosen, Then the earlier one wins [unit]
  AC13 Given the same file with another resource id inside the project, When a frame is cached, Then the cached frame is the same [unit]
  AC14 Given the cache key of §6.5, When it is built, Then it is the content hash of the medium, the proxy level and the frame index in that level [unit]
  AC15 Given a frame that is not cached, When it is asked for, Then the nearest cached frame is returned and the exact one is left to be decoded [unit]
  AC16 Given the head moving, When the prefetch runs, Then it prefetches in the direction of the movement [unit]
  AC17 Given `trim(budget)`, When it runs, Then `bytes()` is at most the budget given and the cache keeps the most recently used entries [unit]
  AC18 Given the crate, When the CI policy steps run, Then `gafira-media` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the crate has no `std::fs` [command]
Plan: the delivery adds the frame cache module to `crates/gafira-media/src/`, on top of the keyframe index that `ME1` built. The budget calls travel through the same trait shape that `ports::TileCache` uses, and the order of ARQ-44 holds: the governor trims this cache first. The cache lives inside the 1,5 GB of D-03, which is the owner's decision of 2026-11-03 (O2). The critical code is the cache key, the eviction order, the nearest-frame search and the budget boundary (design "Critical code"), so those carry the hand mutants.
Owner decisions touched: O2 of 2026-11-03 (the frame cache lives inside the 1,5 GB of D-03 and the governor trims it first); no D-nn is contradicted.

## ME2 (tdd-apply)

Commits:

- `6d6eab9` `feat(gafira-media): add the frame cache`: the cache module of the crate (`src/cache.rs`), its exports (`src/lib.rs`) and its tests (`tests/cache.rs`). 3 files changed, 649 insertions(+).

Measured at `6d6eab9`: `git diff --shortstat a57f8ea...HEAD` -> 4 files changed, 667 insertions(+), of which 18 are the `## ME2` section of the card that `98e6d57` added. The delivery alone, from that card: `git diff --shortstat 98e6d57...HEAD` -> 3 files changed, 649 insertions(+), that is 649 authored lines against the ~470 ceiling of the design (660 with the factor of the plan). The breakdown is 380 lines for the cache module, 5 for the exports and 264 for the tests. No file outside `crates/gafira-media/**` was touched: the delivery needed no new double, so the manifest of the testkit was not touched either.

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` (182 binaries, 2293 tests, 0 failed), `cargo deny check` (advisories, bans, licenses and sources ok) and the CI policy steps, each by extracting its `run:` from `ci.yml`: CLI/UI (ARQ-07), the forbid step, the layer table (V5), the clippy.toml copies (V12), ARQ-107, V13, V14 and the `ports` feature of the testkit (ADR 0013). Plus the crate gate: `cargo clippy -p gafira-media --all-targets --locked -- -D warnings` and `cargo test -p gafira-media --locked` (21 tests, 0 failed).

### RED evidence

The tests were written against the acceptance criteria of the card and the implementation was then reduced to a skeleton, in two batches, to capture the RED. Every failure below is an assertion failure, not a compile error, and the implementation was restored afterwards.

The first batch is the core of the cache: the key, the budget, the eviction and the trim. Its skeleton never evicted (the budget was never enforced), keyed the frames by the id of the resource inside the project instead of the hash of the content, and always answered the default budget with one gigabyte. Six of the eight tests of the batch failed:

- AC10 `arq_145_a_budget_of_zero_keeps_no_frame`: `assertion left == right failed, left: Bytes(16), right: Bytes(0)`.
- AC11 `arq_145_a_budget_of_one_frame_discards_the_first`: `assertion failed: cache.bytes() <= budget`.
- AC13 `arq_145_the_same_file_with_another_resource_id_gives_the_same_frame`: `the same file gives the same cached frame in any project`.
- AC14 `arq_145_the_key_is_the_content_hash_the_level_and_the_index`: `assertion left == right failed, left: FrameKey { content: ContentHash([99, 108, 105, 112, 0, ...]), level: P720, index: 12 }, right: FrameKey { content: ContentHash([7, 7, 7, ...]), level: P720, index: 12 }` (the key had taken the id of the resource inside the project).
- AC17 `arq_145_the_trim_leaves_the_cache_at_or_below_the_budget`: `assertion failed: cache.bytes() <= budget`.
- The budget of ARQ-145, `arq_145_the_default_budget_is_the_lesser_of_a_gigabyte_and_a_fifteenth_of_the_ram`: `assertion left == right failed, left: Bytes(1073741824), right: Bytes(644245094)`.

The second batch is the nearest frame and the prefetch. Its skeleton looked for no nearest frame at all and its prefetch always went forward. Five of the twelve tests of the batch failed:

- AC12 `arq_145_the_nearest_frame_wins_the_tie_with_the_earlier_one`: `assertion left == right failed: the earlier frame wins the tie, left: Flicks(211680000), right: Flicks(70560000)`.
- AC15 `arq_145_a_missing_frame_answers_with_the_nearest_one`: `assertion left == right failed: the nearest frame of the medium asked for, left: Flicks(289296000), right: Flicks(282240000)`.
- AC16 `arq_145_the_prefetch_follows_the_direction_of_the_head`: `assertion left == right failed, left: [11, 12, 13], right: [9, 8, 7]`.
- AC16, the border of the prefetch, `arq_145_the_prefetch_skips_the_cached_frames_and_stops_at_the_first_one`: `assertion left == right failed, left: [3, 4, 5], right: [0]`.
- The level of the key, `arq_145_a_frame_of_the_original_is_not_the_frame_of_the_proxy`: `assertion left == right failed, left: Nearest(VideoFrame { pts: Flicks(112896000), ... }), right: Empty`.

AC18 is a command: the policy steps above, all of them exit 0, and `grep -rn` over the `.rs` files of `crates/gafira-media` for `std::fs`, `std::net`, `std::thread`, `std::time::Instant`, `std::time::SystemTime` and `std::env`: no hit. The crate keeps `#![forbid(unsafe_code)]` and its row of the layer table (V5) still passes.

### Negative controls

The four hand mutants of the design ("Critical code" of `ME2`), applied one at a time to the cache of `gafira-media` and reverted after each run. None of them was committed and the tree is clean after the four.

- The key of the cache: the key of a request took the id of the resource inside the project instead of the hash of the content. Killed by `arq_145_the_same_file_with_another_resource_id_gives_the_same_frame`: `the same file gives the same cached frame in any project`. It also killed `arq_145_the_key_is_the_content_hash_the_level_and_the_index` and the two tests of the nearest frame.
- The order of the eviction: the eviction freed the most recently used frame instead of the least. Killed by `arq_145_the_trim_leaves_the_cache_at_or_below_the_budget`: `the most recently used frame stays in the cache`; and by `arq_145_a_budget_of_one_frame_discards_the_first`: `assertion failed: matches!(cache.lookup(&third), FrameHit::Exact(_))`.
- The search of the nearest frame: at the same distance the later frame won. Killed by `arq_145_the_nearest_frame_wins_the_tie_with_the_earlier_one`: `assertion left == right failed: the earlier frame wins the tie, left: Flicks(211680000), right: Flicks(70560000)`.
- The limit of the budget: the eviction freed until the size was below the budget (`>=` instead of `>`). Killed by `arq_145_a_budget_of_one_frame_discards_the_first`: `assertion left == right failed, left: 0, right: 1` (the cache kept no frame); and by `arq_145_the_trim_leaves_the_cache_at_or_below_the_budget`: `left: 0, right: 1`.

### Choices and deviations

- **The content hash is reused, not computed.** The key uses `gafira_doc::ContentHash`, the 32-byte hash that names a content in the content store (02 §5.3), which is plain data by ADR 0008 rule 1. The hash of the file is computed where the content is stored, so the media crate takes it as a parameter and adds no dependency: the `Cargo.toml` of the crate and the `Cargo.lock` are untouched by this delivery.
- **The caller names the medium with two names.** `MediumRef` holds the id of the resource inside the project (`ProviderAssetId`, the only id of asset of the ports) and the hash of the content of the file. The cache ignores the id and keys by the hash, which is what AC13 proves; the id is in the call so the proof is possible.
- **The budget of a machine is a parameter.** `default_budget(ram)` returns the lesser of one gigabyte and 15 % of the RAM (02 §6.5, FIJADA), because the crate reads no value of the system (ARQ-138). The default does not lower because of the 1,5 GB of D-03: that cap belongs to the governor, and the owner decided on 2026-11-03 (O2) that the governor trims this cache first.
- **`trim(budget)` frees the cache and does not lower the budget of the cache.** It follows the semantics of `ports::TileCache::trim` ("frees the cache until the cache holds at most `budget` bytes"), so the governor can trim the cache and the cache grows again under its own budget; the shape of the two calls is the one of the port (`bytes()` and `trim(budget)`, both in bytes).
- **The cache does not implement `ports::TileCache`.** That trait is the one of the tiles of the render stream: its key is a content key plus a tile rect and its value is an `ImageBuf`. The frame cache is keyed by the content hash, the proxy level and the index of the frame, and holds a `VideoFrame` with its time. The governor lives in `gafira-engine`, out of the cut of this delivery, and calls the two calls on the concrete type.
- **The bytes of a frame are the bytes of its pixels** (`Rgba8`, `RgbaF32` and `Alpha8`); the rest of the frame is a time, and the cache counts the memory it holds (ARQ-44).
- **The nearest frame is searched among the frames of the same medium and the same level**, by the distance in time, and the search walks the `BTreeMap` by the range of the key instead of the whole cache. At the same distance the earlier frame wins (02 §4.17.8).
- **The LRU order lives in a second `BTreeMap`**, from the tick of the last use to the key, so the eviction takes the least recently used frame without walking the cache. The crate uses `BTreeMap` and never `HashMap` (ARQ-10).
- **The prefetch names the frames to decode.** The crate does not decode them: it gives the keys of the frames ahead of the head, in the direction the head moves, that the cache does not hold yet, and the caller decodes and stores them. The prefetch stops at the first frame of the medium when it goes backward.
- **The RED was captured in two batches, by reducing the implementation to a skeleton**, not in the order of one test at a time: the tests and the implementation were written in the same session and the skeleton replay shows that every criterion has a test that detects its absence. The evidence above is the real output of those replays.
- **No deviation of scope.** The delivery touched only `crates/gafira-media/**`: it needed no new double, so the manifest of the testkit (`crates/gafira-testkit/Cargo.toml`, a single-writer file of `SC1`) was not touched, and `crates/gafira-ports`, `crates/gafira-render`, `crates/gafira-doc` and `.odd/state.json` were not touched either.
