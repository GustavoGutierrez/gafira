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

