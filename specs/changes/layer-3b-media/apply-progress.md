## Slice card
Flow: standard | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the media stream of layer-3b opens with the playback clock that keeps the audio and the video together; O8 carries O4 and O5 of layer 3 to this cut
Product IDs: FV-11, FV-10 (c) (source: specs/05-funcionalidad-video-y-distribucion.md §A6, specs/02-arquitectura-software.md §6.5)
Scope: `ME4` of `specs/history/2026-11-04-layer-3/design.md`: in `gafira-media`, the playback clock whose master is the audio, the A/V drift that follows from it, and the drop decision of §6.5 with its 40 ms tolerance.      Non-goals: the exporter (`ME5`), the frame cache (`ME2`, done), the keyframe index (`ME1`, done), the audio mixer itself (layer 2, `gafira-audio`), and any change to `gafira-ports`.
Acceptance:
  AC1 Given the playback, When the clock is read, Then the master is the audio and the video never blocks it (02 §6.5) [unit]
  AC2 Given a video frame that arrives 39 ms late, When the drop decision runs, Then no frame is dropped; given one that arrives 40 ms late or later, Then that frame is dropped and the next one is presented (02 §6.5) [unit]
  AC3 Given the preview, When the drift between the audio and the video is measured, Then it does not exceed 40 ms, the same tolerance that FV-12 (b) fixes for the exported file (02 §6.5) [unit]
  AC4 Given the speeds of FV-11, When the playback runs at 0,5×, 1× and 2×, Then the presentation time of every frame scales with the speed and the master clock stays the audio [unit]
  AC5 Given a frame stepped one by one in both directions, When the clock advances, Then the step follows the direction and stays inside the range (FV-11) [unit]
  AC6 Given a zero-length range, When it plays, Then it completes without panicking and without dropping any frame (02 §6.5) [unit]
  AC7 Given the clock, When it is read twice, Then the reading is monotonic and comes from the injected `ports::Clock`, not from a clock of the crate (ARQ-02, ARQ-138) [unit]
  AC8 Given a long playback, When it runs, Then the cancel token is checked per frame and the progress only reaches `1.0` on a normal end (ARQ-139) [unit]
  AC9 Given the crate, When the CI policy steps run, Then `gafira-media` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the crate has no `std::fs` and no clock of its own [command]
Plan: the delivery adds the playback clock module to `crates/gafira-media/src/`, on top of the frame cache of `ME2`. It adds no dependency, touches no shared file, and reaches the time through `ports::Clock`. The critical code is the monotonicity of the clock, the drop decision, the drift computation and the zero-length range (design of layer 3, "Critical code"), so those carry the hand mutants.
Owner decisions touched: O8 of 2026-11-05 (the rigor of layer 3 carries over); no D-nn is contradicted.

# Apply progress: layer-3b-media

## ME4 (tdd-apply)

Commits:

- `31fc79b` `feat(gafira-media): add the playback clock and the frame dropping`: the clock module of the crate (`src/playback.rs`), its exports (`src/lib.rs`), its tests (`tests/playback.rs`) and the clock and frame doubles those tests inject (`tests/playback_support.rs`). 4 files changed, 897 insertions(+).

Measured at `31fc79b`: `git diff --shortstat 414bef4...HEAD` -> 4 files changed, 897 insertions(+), that is 897 authored lines against the ~420 forecast of the design of record (590 with the factor of the plan): 326 lines for the clock, 457 for the tests, 110 for the doubles and 4 for the exports. The delivery passes the ~700 signal of `AGENTS.md` by 197 lines: the coordinator decides whether the size is accepted or the delivery is cut. No file outside `crates/gafira-media/**` was touched, the testkit needed no new double, and neither a manifest nor `Cargo.lock` changed.

Gates per commit, each run alone, exit 0: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0 (188 binaries, 2371 tests, 0 failed), `cargo deny check` 0 (`advisories ok, bans ok, licenses ok, sources ok`), and the nine CI policy steps, each `run:` extracted literally from `.github/workflows/ci.yml` and run with `bash -euo pipefail`: Format 0, CLI has no UI dependencies (ARQ-07) 0, Domain crates forbid unsafe code 0, the layer table (V5) 0, the `clippy.toml` copies (V12) 0, Layers 0-2 know no language (ARQ-107) 0, EditToken only in the edit crates (V13) 0, Every core command has a property test (V14) 0 and Testkit ports feature compiles (ADR 0013) 0. Plus the crate gate: `cargo clippy -p gafira-media --all-targets --locked -- -D warnings` 0 and `cargo test -p gafira-media --locked` 0, 40 tests (12 cache, 3 contract, 7 keyframe, 9 playback, 9 proxy).

### RED evidence

The tests were written against the acceptance criteria of the card and the implementation was then reduced to a skeleton, in six replays, to capture the RED. Every failure below is an assertion failure, not a compile error, and the implementation was restored afterwards (`cargo test -p gafira-media --test playback --locked`: 9 passed, 0 failed).

- AC1 `arq_147_the_master_is_the_audio_and_the_video_never_blocks_it`, with the master frozen at its anchor: `assertion left == right failed: the master follows the audio with no video frame, left: Flicks(0), right: Flicks(70560000)`.
- AC2 `arq_147_the_drop_boundary_is_39_ms_late_and_40_ms_late`, with `decide` presenting everything: `assertion left == right failed: a frame 40 ms late against the tolerance of 40 ms, left: Present, right: Drop`.
- AC3 `arq_147_the_preview_drift_never_passes_40_ms`, with `decide` presenting everything: `assertion left == right failed: a frame 40 ms late, left: Present, right: Drop`. The second assertion of the same test holds the bound itself (`drift < SYNC_TOLERANCE`) and it is the one that fires on H1 b below.
- AC4 `fv_11_the_speed_scales_the_presentation_time`, with the master frozen: `assertion left == right failed: 500 ms of audio are 250 ms of presentation at Half, left: Flicks(0), right: Flicks(176400000)`.
- AC5 `fv_11_the_step_follows_the_direction_inside_the_range`, with `step` answering the current position: `assertion left == right failed: one frame forward is one frame of 30 fps, left: Flicks(0), right: Flicks(23520000)`.
- AC6 `arq_147_a_zero_length_range_completes_without_dropping`, twice: with a frame at or past the end dropped, `assertion left == right failed: the frame at the start of a range of zero length is presented, left: Drop, right: Present`; with the empty range not treated as over (`>` instead of the empty-range guard), `a range of zero length is over at once`.
- AC7 `arq_138_the_master_is_monotonic_and_comes_from_the_clock_port`, with the master frozen: `assertion left == right failed: the reading is the reading of the injected clock, left: Flicks(0), right: Flicks(352800000)`.
- AC8 `arq_139_the_playback_checks_the_cancel_token_per_frame`, with `play` without the per-frame checkpoints nor the per-frame reports: `assertion left == right failed: the fraction is the frames done over the frames planned, and 1.0 is the end, left: [1.0], right: [0.2, 0.4, 0.6, 0.8, 1.0]`. `check_progress_contract` does not fail in that batch, for the reason ME3 already recorded: a run that reports only its end gives the checker no report to cancel in, so the test asserts the values of the reports of a plain run before it hands the operation to the checker.
- AC9 is a command: the policy steps above all exit 0, and `grep -rnE` over `crates/gafira-media/src` and `crates/gafira-media/tests` for `std::fs`, `std::net`, `std::thread::spawn`, `std::thread::Builder`, `std::time::Instant::now`, `std::time::SystemTime::now` and `std::env` gives no hit (exit 1). The crate keeps `#![forbid(unsafe_code)]` and its row of the layer table (V5).

### Negative controls

The hand mutants of the list, applied one at a time to the clock on the green code and reverted after each run. None of them was committed and `31fc79b` is the green one.

| # | The wrong behaviour | The killing test | The assertion |
|---|---|---|---|
| H1 a | the boundary moved to the other side: a frame 39 ms late was dropped (`late + 1 ms >= 40 ms`) | `arq_147_the_drop_boundary_is_39_ms_late_and_40_ms_late` | `assertion left == right failed: a frame 39 ms late against the tolerance of 40 ms, left: Drop, right: Present` |
| H1 b | the boundary moved to the other side: a frame 40 ms late was kept (`late > 40 ms`) | `arq_147_the_drop_boundary_is_39_ms_late_and_40_ms_late`, `arq_147_the_preview_drift_never_passes_40_ms` and `fv_11_the_speed_scales_the_presentation_time` | `a frame 40 ms late against the tolerance of 40 ms, left: Present, right: Drop`; `a frame 40 ms late, left: Present, right: Drop`; `the frame at 960 ms after 500 ms of audio at Double, left: Present, right: Drop` |
| H2 | the reading followed the clock back (`self.last = position`, without the monotonic guard) | `arq_138_the_master_is_monotonic_and_comes_from_the_clock_port` | `assertion left == right failed: a clock that goes back does not move the master back, left: Flicks(0), right: Flicks(352800000)` |
| H3 | the video became the master (`master` answered the last presented frame instead of the audio clock) | `arq_147_the_master_is_the_audio_and_the_video_never_blocks_it` (and AC4, AC7 and the drop boundary) | `assertion left == right failed: the master follows the audio with no video frame, left: Flicks(0), right: Flicks(70560000)` |
| H4 | a frame at or past the end of the range was dropped, so a range of zero length dropped its own frame | `arq_147_a_zero_length_range_completes_without_dropping` | `assertion left == right failed: the frame at the start of a range of zero length is presented, left: Drop, right: Present` |
| H5 | the speed was ignored and the master always advanced in real time (`scale` returned the elapsed time) | `fv_11_the_speed_scales_the_presentation_time` | `assertion left == right failed: 500 ms of audio are 250 ms of presentation at Half, left: Flicks(352800000), right: Flicks(176400000)` |

H1 is listed on both sides because the card fixes the boundary in both directions and one mutant would leave half of it unproven. No item survived, so no test had to be added for a survivor.

### Choices and deviations

- **The master is the audio, read from `ports::Clock` and scaled in exact integers.** The crate adds no clock of its own (ARQ-02, ARQ-138): the position is the anchor plus the time the injected clock advanced since it, halved at 0,5× and doubled at 2×. The conversion from the `Duration` of the port to flicks is integer (nanoseconds over 1e9 times the flicks of a second), so 39 ms and 40 ms are exact and no float takes part in a decision.
- **The tolerance is stated once.** `SYNC_TOLERANCE` is 40 ms in flicks and the comparison is `>=`. §6.5 says "más de la tolerancia" and then fixes the boundary in the two directions —39 ms drops nothing, 40 ms or later drops— and ARQ-147 and FV-11 (d) repeat that reading, so 40 ms is inside the tolerance's far side and is dropped. The boundary lives in one constant and one comparison, which is what H1 attacks.
- **The drift is measured at the decision and not recomputed later.** A presented frame reports the lateness it had when it was decided, so the drift does not grow with the time; a frame ahead of the master reports 0. That is the reading of "la deriva entre el audio y el video en la vista previa no pasa de 40 ms" for the presented frame, and it makes the bound provable: a frame 40 ms late is dropped, so the presented drift stays strictly below 40 ms.
- **A frame ahead of the master is presented, not waited for.** The video never blocks the audio (02 §6.5, FV-11 (c)), and the crate can wait on nothing anyway: it has no clock or thread of its own (ARQ-02). A late frame is dropped and the next one is decided.
- **The step moves the master and re-anchors the clock there**, so the playback continues from the frame stepped to; the stepped frame counts as presented at its own time, so the drift of a step is 0. A step back also sets the monotonic guard to the new position: an explicit step is not a reading going backwards, and the guard only covers the readings of the clock (ARQ-138).
- **`PlaybackSource` is a trait, not a slice.** The caller decodes while the playback runs, as the proxy store takes its chunks from `ProxyProducer` (ME3), and `frames()` is the plan the progress counts (ARQ-139). The loop runs at most `frames()` times, which also bounds it when a source never ends.
- **`play` reports once per frame and once at the end**, with the finish as one more unit of work (as the proxy store does), so the fraction never reaches 1,0 before the range is over; the last cancel check sits before that report, so a cancel after the last frame never reaches 1,0 either.
- **No deviation of scope.** The delivery touched only `crates/gafira-media/**`: no testkit file, no `Cargo.toml`, no `Cargo.lock`, and `gafira-ports`, `gafira-doc` and the testkit are untouched.
- **Size.** 897 lines against the 420 (590) forecast, 457 of them tests. Recorded for the coordinator's decision, as ME3 did.

## ME5
Flow: standard | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the media stream of layer-3b closes with the video exporter that the export queue reaches; O8 carries O4 and O5 of layer 3 to this cut
Product IDs: FV-12 (partial: the pipeline, the sync, the cancellation and the audio track here; the real codecs and the matrix land with the layer-4 adapters), FV-24 (partial), ARQ-43, CAL-16 (partial), CAL-17 (partial) (source: specs/05-funcionalidad-video-y-distribucion.md §A6, specs/02-arquitectura-software.md §6.4, §4.5)
Scope: `ME5` of `specs/history/2026-11-04-layer-3/design.md`: in `gafira-media`, the video exporter behind `ports::Exporter`: the loop of decode to compose to encode to mux, the progress, the cancellation and the audio track.      Non-goals: the MJPEG or AV1 encoders themselves and the container writers (layer-4 adapters), the export queue (03 §7.5), the codec matrix of CAL-17, `PageRenderer` (`RE5`, the exporter composes through the port), and any change to `gafira-ports`.
Acceptance:
  AC1 Given a range and an export frame rate, When the export runs, Then the pipeline is decode to compose to encode to mux in that order, and the number of encoded frames equals the frames of the range at that rate (FV-12) [unit]
  AC2 Given a sequence with audio, When it is exported, Then the audio track is muxed in and the A/V difference of the result stays below 40 ms (FV-12 (b)) [unit]
  AC3 Given an export cancelled part way, When it stops, Then no partial final file is left and the queue stays usable (FV-12 (d), the same rule as ME3's AC20) [integration]
  AC4 Given the export, When it runs, Then it checks the cancel token per frame, reports a non-decreasing fraction in `[0, 1]` that only reaches `1.0` on success, and enqueues with `Background` priority (ARQ-43, 02 §6.3) [unit]
  AC5 Given a medium with a lightweight copy, When it is exported, Then the export reads the original and never the copy (ME3's rule, §6.5) [unit]
  AC6 Given the export, When it composes a frame, Then it goes through `ports::PageRenderer` and is proved against the testkit double, so the delivery does not depend on `RE5` (design of layer 3, `ME5`) [integration]
  AC7 Given a lost or unavailable encoder, When the export runs, Then it fails with the error the port fixes and does not leave a partial file [unit]
  AC8 Given the crate, When the CI policy steps run, Then `gafira-media` still has `#![forbid(unsafe_code)]`, its V5 row still passes and the crate has no `std::fs` [command]
Plan: the delivery adds the exporter module to `crates/gafira-media/src/`, on top of the frame cache of `ME2` and the playback clock of `ME4`, and implements `ports::Exporter`. The codecs and the container writers arrive as layer-4 adapters; this delivery owns the loop, the ordering, the progress, the cancellation and the audio track. The critical code is the frame loop, the A/V alignment, the cancellation checkpoints, the mux ordering and the frame count (design of layer 3, "Critical code"), so those carry the hand mutants. `CAL-16` (`bench_export_reel_60s` under 180 s) and `CAL-17` (the export matrix of at least 40 combinations) are measurements of the layer-4 adapters: they stay partial here.
Owner decisions touched: O8 of 2026-11-05 (the rigor of layer 3 carries over); D-02 (the codecs and their licences) is respected by not shipping an encoder of our own.

## ME5 (tdd-apply)

Commits:

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations

