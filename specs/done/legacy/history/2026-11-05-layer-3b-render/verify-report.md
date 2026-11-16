# Verify report: layer-3b — RE4, RE5, ME4, ME5 (combined round)

## Current state

Round 1 of the four deliveries in **one combined round**, by owner decision **O9**
(`specs/changes/layer-3b/design.md:44`): `RE4` and `RE5` of `gafira-render`, `ME4` and
`ME5` of `gafira-media`. Commit `4731379` (branch `chore/layer-3b-v1`), depth **V1**,
mode `standard`, tier `T0`, `mutation: off` (O8). The gates are green at HEAD.

Verdict: **FAIL** — 1 CRITICAL, 3 WARNING, 2 SUGGESTION. Per delivery: `RE4` **PASS**,
`RE5` **PASS**, `ME4` **FAIL** (C1), `ME5` **PASS**.

Per class: implementation 4 (3 WARNING, 1 SUGGESTION), ambiguity 1 (1 CRITICAL),
architecture 0, claim 1 (1 WARNING).

The single CRITICAL is not in code: `ME4`'s Slice card lists the product ID `FV-10 (c)`,
which no test or command in the tree names, while the criterion it means (`FV-11 (c)`) is
proved. It is a one-line card row, fixed by a `slice card amend`; the delivery's own code
and tests are sound. `next: specifier` (see "Next").

## What this round proves, and what it does not

V1 is the gates at HEAD plus the map of each acceptance criterion to a test or a command.
Per O8 this round runs **no independent probes and no independent hand mutants**; the hand
mutants of the card belong to `tdd-apply` and are recorded there. Nothing here re-judges
them.

The round is combined, so the reading was budgeted. `ME5` (2 285 lines) and `RE4`
(1 277) got less depth than a dedicated round would give:

- `ME5`: `src/exporter.rs` was read in full (code, options, sink and the memory writer) and
  `tests/exporter.rs` in full, but `tests/exporter_support.rs` (640 lines of doubles) was
  read only where a criterion depends on it (the `CountingRenderer` that delegates to the
  testkit `FakePageRenderer`, the `MemSink` that records the file names, `LogCodec`). The
  codecs and the container are doubles by design (D-02), so the intra-double assertions are
  not judged beyond what each criterion names.
- `RE4`: `budget.rs`, the selector and the frame loop of `frame.rs` were read in full with
  the deadline, degrade, progress and cancel seams; the test helpers (`RecordingBackend`,
  `ScriptClock`) and the testkit `FakeRenderBackend` were read only as the tests use them.

The three things the cards already refuse to claim stay refused, and are **not** findings
of this round: the `1/255` golden of ARQ-42 needs the CPU backend of `gafira-render-cpu`
(O3); `CAL-16` and `CAL-17` are measurements of the layer-4 codecs and container writers;
the `100 ms` bound of ARQ-43 has no bench. No card claims any of them as proved — checked,
and they read `partial` in every Product IDs row and in the Size/Choices notes.

## Gates at HEAD (`4731379`)

Each gate ran alone; exit codes are the real ones. The nine CI policy steps were extracted
literally from `.github/workflows/ci.yml` (the `policy` job) into
`/tmp/gafira-v1-ci/steps/` and each run with `bash -euo pipefail`; the extraction is
byte-for-byte (checked against the workflow). `CARGO_TARGET_DIR=/tmp/gafira-v1-target`,
`CARGO_INCREMENTAL=0`; `df -h /` showed 73 GB free before the build.

| Gate | Exit | Notes |
|---|---|---|
| `cargo fmt --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | 25 s |
| `cargo test --workspace --locked` | 0 | 301 s; 191 `test result: ok` blocks, **2 405 tests**, 0 failed, 0 ignored |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI · Format | 0 | |
| CI · CLI has no UI dependencies (ARQ-07) | 0 | |
| CI · Domain crates forbid unsafe code | 0 | both crates keep `#![forbid(unsafe_code)]` (`gafira-render/src/lib.rs:11`, `gafira-media/src/lib.rs:9`) |
| CI · Domain crates respect the layer table (V5) | 0 | both rows of the table are present and unchanged |
| CI · Domain clippy.toml copies are in sync (V12) | 0 | |
| CI · Layers 0-2 know no language (ARQ-107) | 0 | |
| CI · EditToken only in the edit crates (V13) | 0 | |
| CI · Every core command has a property test (V14) | 0 | scoped to `crates/gafira-edit/tests/prop_commands.rs` only |
| CI · Testkit ports feature compiles (ADR 0013) | 0 | 43 s |

The test totals match the cards: `ME4` says the workspace reached 2 371 at its commit,
`ME5` 2 380, `RE5` 2 405 — the HEAD number is exactly `RE5`'s, `191` blocks included.

## RE4 — the frame budget, the degradation and the progressive refinement

**Verdict: PASS.** All eight criteria are covered by a test or a command; the tests name the
IDs of the card; the code matches the reading of each criterion.

| AC | Type | Proof | Verdict |
|---|---|---|---|
| AC1 cumulative deadlines | unit | `budget::tests::arq_41_the_stages_are_cumulative_deadlines` (`budget.rs:175`) — offsets 6/14 ms, margin `114..116`, and a stage that runs into the margin does not move the deadlines | covered |
| AC2 equality of the deadline, `Unbounded` | unit | `frame::tests::arq_41_the_left_tiles_are_reported_at_the_exact_deadline` (`frame.rs:735`) plus `arq_41_a_tile_the_backend_leaves_is_not_counted` (`frame.rs:869`); `deadline_passed` delegates to `Deadline::has_passed` (`ports/render.rs:175`, `now >= t`) | covered |
| AC3 the instant from the injected clock | unit | `budget::tests::arq_02_the_deadline_comes_from_the_injected_clock` (`budget.rs:202`); `FrameBudget::begin` reads `clock.monotonic()` | covered |
| AC4 best mip, refinement, never blocks | unit | `frame::tests::arq_41_a_late_frame_presents_the_best_mip_and_refines` (`frame.rs:792`): `left > 0`, `refine`, `mip` = the last finished level, and a deadline already past returns at once | covered |
| AC5 degradation to CPU | unit | `frame::tests::arq_22_a_lost_device_degrades_the_session_to_the_cpu` (`frame.rs:687`): `DeviceLost` on the second task; graph key and node count unchanged; two later frames on CPU; the switch reported | covered, except the "unavailable" branch (W3) |
| AC6 cancel per tile, progress | unit | `arq_43_the_cancel_stops_the_frame_between_tiles` (`frame.rs:960`), `arq_43_the_frame_checks_the_cancel_per_tile` (`frame.rs:932`), `arq_43_the_progress_is_the_units_done_over_the_units_planned` (`frame.rs:896`) | covered |
| AC7 determinism `[property]` | property | `budget::tests::arq_02_the_budget_takes_the_same_decisions_twice` (`budget.rs:225`) and `frame::tests::arq_02_the_same_frame_takes_the_same_decisions` (`frame.rs:988`) — fixed-input double runs (S1) | covered |
| AC8 the crate gate | command | the gates above; `git grep` for `std::fs`, `std::net`, `std::thread::spawn`, `std::time::Instant::now`, `std::time::SystemTime::now`, `std::env` over `crates/gafira-render/{src,tests}` exits 1; `#![forbid(unsafe_code)]` | covered |

Findings:

- **W3**, WARNING, `class: implementation`, `crates/gafira-render/src/frame.rs:56`.
  AC5 says "a backend that fails **or is unavailable**". The failing half is tested; the
  unavailable half is implemented (`BackendSelector::new` sets
  `Degradation::AdapterMissing` when no GPU is given) and **no test asserts it**: the ARQ-22
  test builds a selector with `Some(Box::new(LostDevice))`, and the RE5 integration test that
  passes `None` (`tests/page_renderer.rs:214`) never reads `degradation()`. A mutation that
  dropped the `AdapterMissing` report would survive. Small, static, untested branch.
- **S1**, SUGGESTION, `class: implementation`, `crates/gafira-render/src/budget.rs:225`.
  AC7 is tagged `[property]` and is proved by two fixed-input double runs. `gafira-render`
  has no `proptest` dev-dependency and the delivery adds none; other crates that carry
  `[property]` criteria use `proptest!` (`gafira-raster`, `gafira-anim`, `gafira-audio`). The
  V14 policy does not reach this crate, so no gate requires it. The criterion as worded ("the
  same input twice") is satisfied; a generated-input property would be stronger.

## RE5 — the `PageRenderer` and the page scope

**Verdict: PASS.** All eight criteria are covered. The amended AC4 is the one the round was
asked to check, and the code matches it.

| AC | Type | Proof | Verdict |
|---|---|---|---|
| AC1 `ImageBuf` through the port | unit | `page::tests::arq_42_a_page_renders_through_the_graph_the_plan_and_the_cache` (`page.rs:632`) plus the port conformance `tests/page_renderer.rs:332` (`&dyn PageRenderer`) | covered |
| AC2 same graph and ops, quality only in the keys | integration | `tests/page_renderer.rs:247` (builds both graphs through the public API: same nodes, same kind, different keys, same ops) and `tests/page_renderer.rs:275` (both renderers reach the backend with the same tiles and ops, different `key.content`) | covered, strong |
| AC3 the two formats | unit | `page::tests::arq_42_the_preview_is_rgba8_and_the_export_is_rgba_f32` (`page.rs:703`): `Rgba8` at level 1 for a 2× preview, `RgbaF32` at level 0 for the export. `PixelFormat::Rgba8` is documented as "sRGB-encoded, premultiplied" and `RgbaF32` as "linear light, premultiplied" (`gafira-raster/src/view.rs:5-14`), so the format assertion carries the whole criterion | covered |
| AC4 scope, `OutOfRange`, total `render` | unit | `scope::tests::fd_58_an_out_of_range_page_is_rejected_before_any_render` (`scope.rs:204`), `fd_57_a_single_page_and_a_range_map_to_requests` (`scope.rs:159`), `page::tests::fd_57_render_is_total_over_the_page_id` (`page.rs:734`) | covered, matches the amend |
| AC5 cancelled render, no buffer | unit | `page::tests::arq_43_a_cancelled_render_answers_cancelled_without_a_buffer` (`page.rs:753`): cancelled before the frame and cancelled by the backend after the first tile | covered |
| AC6 determinism `[property]` | property | `page::tests::arq_02_two_renders_of_the_same_page_are_identical` (`page.rs:791`) over preview 1×, preview 2× and export | covered, no proptest (S2) |
| AC7 transparent background | unit | `page::tests::arq_42_a_transparent_render_paints_no_background` (`page.rs:817`): surface channels 4, the background `FillPath` present only when `transparent == false` | covered |
| AC8 the crate gate | command | as `RE4`'s AC8 | covered |

**The amended AC4, checked.** The card now says the out-of-range page is rejected by the
scope mapping and that `render` stays total. The code does exactly that:
`PageScope::resolve` (`scope.rs`) checks every page and every selection page against the
document and answers `ScopeError::OutOfRange { page, pages: document.pages().len() }` before
any request exists; `GraphPageRenderer::render` (`page.rs:160`) looks the page up with
`self.document.page(request.page)` and answers `blank(region, format)` on `None`, in the
quality's format.

**Can `render` panic on a page id?** No, as far as reading and the tests reach. The lookup is
total; `extent` maps a non-finite page size to `RenderError::Buffer` and `side_px` clamps to
`≥ 1`, so `TileGrid::level_count` cannot see 0; `level_for_zoom` returns 0 on `levels == 0`,
`NaN` or `≤ 0`; `blank` uses `checked_mul` and answers `SizeOverflow`; `whole` and `side_px`
saturate instead of wrapping; both `Mutex` locks recover poisoning with
`PoisonError::into_inner`; `paint_background` builds one finite `Rect`. The only infallible
paths are the blank answer and the pointer-free arithmetic. A `PageId` the document does not
hold never opens a frame (`targets.is_empty()` and `ran.is_empty()` in the AC4 test).

Finding:

- **S2**, SUGGESTION, `class: implementation`, `crates/gafira-render/src/page.rs:791`.
  Same as S1 on the other side: AC6 is tagged `[property]` and is a triple double-render, not
  a generated-input property; `gafira-render` has no `proptest`.

## ME4 — the playback clock and the frame dropping

**Verdict: FAIL**, on the card's Product IDs row (C1); the code and the tests themselves are
sound and every acceptance criterion is proved.

| AC | Type | Proof | Verdict |
|---|---|---|---|
| AC1 the master is the audio | unit | `arq_147_the_master_is_the_audio_and_the_video_never_blocks_it` (`tests/playback.rs:44`): 100 ms with no frame, a dropped frame does not move it, a presented one neither, and a playback that drops all four frames still reaches 800 ms | covered |
| AC2 the 39/40 ms boundary | unit | `arq_147_the_drop_boundary_is_39_ms_late_and_40_ms_late` (`playback.rs:114`): 0/1/38/39 ms present, 40/41/1000 drop, and the frame after a dropped one is presented | covered |
| AC3 the drift ≤ 40 ms | unit | `arq_147_the_preview_drift_never_passes_40_ms` (`playback.rs:164`): 39 ms reported as the drift, and a sweep of 0..=120 ms asserts both the decision and `drift < SYNC_TOLERANCE` | covered; the ahead-of-master case is untested (W1) |
| AC4 the speeds | unit | `fv_11_the_speed_scales_the_presentation_time` (`playback.rs:206`): 500 ms of audio are 250/500/1000 ms of presentation, and a frame at 960 ms is ahead at 0,5×/1× and 40 ms late at 2× | covered |
| AC5 the step | unit | `fv_11_the_step_follows_the_direction_inside_the_range` (`playback.rs:248`): forward, back, the clock re-anchoring, the stop at both ends | covered |
| AC6 the zero-length range | unit | `arq_147_a_zero_length_range_completes_without_dropping` (`playback.rs:302`): `is_finished` at once, the frame at the start presented, `play` reports `(0, 0)` and a single `1.0` | covered |
| AC7 monotonic, from the port | unit | `arq_138_the_master_is_monotonic_and_comes_from_the_clock_port` (`playback.rs:342`): two readings of one instant, a clock that goes back twice, and a re-take when it passes the last reading | covered |
| AC8 cancel per frame, progress | unit | `arq_139_the_playback_checks_the_cancel_token_per_frame` (`playback.rs:399`, exact fractions `[0.2 … 1.0]`, a token cancelled before the first frame reporting nothing) and `arq_139_the_playback_follows_the_progress_contract` (`playback.rs:445`, testkit) | covered |
| AC9 the crate gate | command | as `RE4`'s AC8; the grep includes `std::thread::Builder` and `std::env` and exits 1; no `Instant::now`/`SystemTime::now`, so the crate keeps no clock of its own | covered |

Findings:

- **C1**, CRITICAL, `class: ambiguity`, `specs/changes/layer-3b-media/apply-progress.md:3`.
  The card's Product IDs row lists `FV-11, FV-10 (c)`. `git grep FV-10` over
  `crates/gafira-{render,media,testkit}/{src,tests}` finds **nothing**, and over the whole
  tree it finds only `FV-10 (a)`, `(b)` and `(d)` in `gafira-audio`. `FV-10 (c)` is
  "Separar audio conserva la sincronía exacta (diferencia 0 muestras)"
  (`specs/05-…:137`), which is not in this delivery's scope (`apply-progress.md:4`), and the
  `(c)` the delivery actually proves is `FV-11 (c)` "el audio nunca se corta al saltar
  fotogramas" — named that way by the test itself (`tests/playback.rs:89`). The row also
  cites `§A6`, the section of `FV-11`, not of `FV-10`. Step 3c of `verify.md` makes a listed
  product ID with no test that names it CRITICAL; the root cause is the card row (a
  contradiction inside the card, hence `ambiguity`, not a code gap). **Fix:** a one-line
  `slice card amend` to `FV-11 (c)`; no code moves. If `FV-10 (c)` was meant (Separar audio
  sync), it is out of this delivery and needs its own slice.
- **W1**, WARNING, `class: implementation`, `crates/gafira-media/src/playback.rs:198`.
  `drift()` is documented as "0 … when that frame was ahead of the master"
  (`playback.rs:194`) and the card repeats it (`apply-progress.md:63`), but the code returns
  `master.checked_sub(frame).unwrap_or(ZERO)`; `Flicks` is a signed `i64`
  (`gafira-time/src/flicks.rs:21`) and `checked_sub` only fails on overflow, so a frame ahead
  of the master yields a **negative** drift, not `Flicks::ZERO`. No test decides a frame
  ahead and then reads `drift()` (the 0..=120 sweep only goes backwards). AC3's bound still
  holds (a negative drift is below 40 ms), so no criterion fails; a caller that displays the
  drift would show a negative number.

## ME5 — the video exporter

**Verdict: PASS.** All eight criteria are covered; AC3's deviation (the name created only
after the mux, the bytes buffered in memory) is real and the cancellation path is clean.

| AC | Type | Proof | Verdict |
|---|---|---|---|
| AC1 the pipeline order and the frame count | unit | `fv_12_the_pipeline_orders_the_stages_and_encodes_the_frames_of_the_range` (`tests/exporter.rs:87`): the journal's first four steps are decode → compose → encode → mux, 50 encodes and 50 muxes for 2 s at 25 fps, one file named `reel.mp4`, non-zero bytes | covered |
| AC2 the audio track and the A/V difference | unit | `fv_12_the_audio_track_is_muxed_and_the_av_difference_is_below_40_ms` (`exporter.rs:149`): two streams, audio packets non-empty, first audio at the range start, `|first_audio − first_video| < SYNC_TOLERANCE` | covered for an aligned source (W2) |
| AC3 a cancelled export leaves no partial file | integration | `fv_12_a_cancelled_export_leaves_no_partial_file_and_the_next_one_runs` (`exporter.rs:223`): the progress cancels at report 3, the export answers `ExportError::Cancelled`, `sink.names()` is empty, and the same exporter then writes the file | covered |
| AC4 cancel per frame, progress, background | unit | `arq_139_the_export_honours_the_progress_contract_at_background_priority` (`exporter.rs:290`) and `arq_139_the_export_checks_the_cancel_token_per_frame` (`exporter.rs:339`, with `DeafRenderer` so only the loop's checkpoint can react); `priority() == Priority::Background` | covered |
| AC5 the original, never the copy | unit | `fv_12_an_export_reads_the_original_and_never_the_copy` (`exporter.rs:383`): the source was opened at `original.mov` although a copy exists | covered |
| AC6 through `ports::PageRenderer` | integration | `fv_12_the_composition_goes_through_the_page_renderer` (`exporter.rs:429`): 25 compositions, the page of the scope, the region from the width/height options; `CountingRenderer` delegates to the testkit `FakePageRenderer` (`exporter_support.rs:216`), so the delivery does not depend on `RE5` | covered |
| AC7 a lost or unavailable encoder | unit | `arq_43_a_lost_or_unavailable_encoder_fails_without_a_file` (`exporter.rs:490`): (a) `SystemCodecUnavailable` before any frame, (b) `Corrupt` on the 4th `send`; no file in either | covered |
| AC8 the crate gate | command | as `ME4`'s AC9 | covered |

**The AC3 deviation, checked.** `Exporter::export` (`exporter.rs:428`) creates the muxer on a
`SharedOutput` over an in-memory `Vec`, runs the frame loop and the audio track with a
`cancel.check()` before every frame, before every audio buffer and once more after
`muxer.finish()`; only then does it clone the bytes and call `sink.create(&self.output)`
(`exporter.rs:484-488`). There is no path from a cancellation to a created name, and the
test drives exactly that path. The consequence the card records is real and is not a defect
of this cut: **the whole reel is buffered until the end** (`exporter.rs:437-440`), which
belongs to the layer-4 `ExportSink` (`create`/rename) and to `CAL-16`. The same buffering is
why an encoder lost after `send` leaves nothing: the bytes never reach a file.

Finding:

- **W2**, WARNING, `class: claim`, `crates/gafira-media/src/exporter.rs:503`.
  `write_audio`'s doc claims the first audio packet "shares its presentation time with the
  first video packet and the A/V difference of the result stays below 40 ms". The exporter
  anchors only the **audio** (`time = range.start + (buffer.pts − first_buffer.pts)`,
  `exporter.rs:523-529`); the video `pts` are the source's, unchanged (`exporter.rs:470`). The
  A/V difference is therefore 0 only when the source's first video frame sits at the range
  start, which the test's `timeline(500, 2_500, 25 fps)` guarantees by construction. The
  exporter neither aligns nor checks. At 24 fps a range that starts between frames can put
  the first video packet 41,7 ms away, past the 40 ms of FV-12 (b). The fix is a sentence in
  the doc (and a layer-4 note) or a real offset; not a criterion failure today.

## Artifact integrity

| Check | Result |
|---|---|
| Every card commit touches only its file | `7c1decd` (RE5 card) and `443fd37` (ME5 card) each touch only `specs/changes/layer-3b-render/apply-progress.md` / `…-media/apply-progress.md`. `414bef4` ("open the change and the two slice cards that are ready") creates `design.md` and both ready cards — the change-opening commit, not a `slice card` commit; both cards still precede their first implementation commit (`31fc79b`, `a1bb4b7`). No implementer commit touches `specs/` |
| No implementer commit changed a card block or a `Flow:` line | `git diff 414bef4~1 HEAD -- …` over both files has no `-` line for `Flow:`, `Product IDs:`, `Scope:`, `Plan:`, `Owner decisions touched:` or any `ACn`. The only deletions in the four notes commits (`320a0d6`, `5de29a6`, `dc74e81`, `58c7f7a`) are the placeholder `Gates per commit …` line that sits in the `(tdd-apply)` section, replaced by the measured evidence |
| A card amend states why | `1878e0b` (`docs(layer-3b-render): amend the RE5 card on the out-of-range page`) changes AC4 and the Plan line; its body names the reason (`RenderError` has no variant, `render` documented as `Cancelled`-only, the ARQ-112 double reads it as total) and it precedes the first RE5 implementation commit (`4c3c85a`). The amended AC4 is the one the code implements |
| Notes in the right sections | `RE4`/`RE5`: `Commits`, `RED evidence`, `Negative controls` (six mutants each, with assertions), `Choices and deviations`, `### The size`. `ME4`/`ME5`: the same, except that the size is the last bullet of `Choices and deviations` instead of its own `### The size` section — format only, the number is recorded in both |
| Scope of the commits | the nine implementation commits touch only `crates/gafira-render/**` or `crates/gafira-media/**`; no `Cargo.toml`, no `Cargo.lock`, no `gafira-ports`, no testkit file |

## Sizes (measured, not decided)

`git diff --shortstat`, per delivery, code only and with the notes:

| Delivery | Code only | With notes | Forecast (design, ×1,4) | Over the ~700 guide |
|---|---|---|---|---|
| `RE4` | `414bef4..436299d` 3 files, **1 232** +, 5 − | 4 files, **1 277** +, 6 − | 645 | yes, +577 (code alone is over) |
| `RE5` | `1878e0b..bf939a8` 5 files, **1 479** +, 1 − | 6 files, **1 528** +, 2 − | 615 | yes, +828 (code alone is over) |
| `ME4` | `414bef4..31fc79b` 4 files, **897** + | 5 files, **938** +, 1 − | 590 | yes, +238 (code alone is over) |
| `ME5` | `443fd37..38aaedf` 4 files, **2 242** + | 5 files, **2 285** +, 1 − | 700 | yes, +1 585 (code alone is over) |

All four pass the signal. Each card records its own number and puts the decision in the
coordinator's hands, as `AGENTS.md` asks; this round only measures. The bulk is tests and
doubles in every case (the cards' own splits: `RE4` ≈ 435 code of 1 232, `RE5` ≈ 430 of 1 479,
`ME4` 326 of 897, `ME5` 965 of 2 242), which is the pattern `RE1`, `RE3` and `ME3` already
set.

## Product IDs

| Delivery | ID | Named by a test? | Proves its criterion? |
|---|---|---|---|
| `RE4` | ARQ-22 | yes (14 mentions) | yes, the DeviceLost half (W3 for AdapterMissing) |
| `RE4` | ARQ-41 | yes (8) | yes |
| `RE4` | ARQ-43 | yes (18) | yes, as a shape (no bench, by design) |
| `RE5` | ARQ-42 (partial) | yes (13) | yes, the shared graph and the quality switch; the 1/255 golden is O3, deliberately left out |
| `RE5` | FD-57 (partial) | yes (11) | yes, the page scope; naming, multi-file output and PDF are layer 4, deliberately left out |
| `ME4` | FV-11 | yes (15) | yes |
| `ME4` | **FV-10 (c)** | **no (0)** | **no — C1** |
| `ME5` | FV-12 (partial) | yes (22) | yes for the pipeline, the sync, the cancellation and the audio; the codecs are layer 4 |
| `ME5` | FV-24 (partial) | yes (26) | yes, the eleven options and the pre-export rejection |
| `ME5` | ARQ-43 | yes (18) | yes, as a shape (no bench, by design) |
| `ME5` | CAL-16 (partial) | no (0) | deliberately left out: a bench of the layer-4 codecs (the card and the design say so) |
| `ME5` | CAL-17 (partial) | no (0) | deliberately left out: the codec matrix of the layer-4 adapters |

Left out with a reason, and therefore not findings: `CAL-16`, `CAL-17`, the `1/255` golden of
`ARQ-42`, the naming/PDF half of `FD-57`, and the `100 ms` bound of `ARQ-43`. `FD-58` is
cited by `RE5`'s AC4 but is not in its Product IDs row; the delivery proves only the
"reject before the export starts" reading of it (the validate list, the `.tmp` write and the
rename are `ME5`'s and layer 4's) — a narrow citation, recorded as an ambiguity of the card
rather than an unproved ID.

## Risks this round leaves

1. `ME5` buffers the whole reel before the name exists. Correct for cancellation, expensive
   for `CAL-16`; the card flags it and it belongs to the layer-4 sink. No measurement here.
2. `ME4`'s drift can be negative (W1) and `RE4`'s `AdapterMissing` report is untested (W3).
   Neither breaks an acceptance criterion; both are cheap to close.
3. The A/V guarantee of `ME5` is the source's alignment, not the exporter's (W2).
4. The two `[property]` criteria are deterministic replays (S1, S2). If the repo wants the
   `[property]` tag to mean `proptest`, the cards should say so and V14 (edit-only) does not
   cover it.
5. Depth: a dedicated `V2` round for `ME5` and `RE4` would probe the doubles' internals and
   run independent mutants. O8/O9 deliberately do not, and this report says where the reading
   was lighter.

## Next

C1 is the only blocker and it is the card's row, not the code: `next: specifier` — amend
`FV-10 (c)` to `FV-11 (c)` in `ME4`'s Product IDs (or open a slice if Separar audio is
meant). With that amend, all four deliveries are PASS at V1 and the round can route to
`archive`; W1, W2, W3, S1 and S2 are `tdd-apply` follow-ups that do not block it.
