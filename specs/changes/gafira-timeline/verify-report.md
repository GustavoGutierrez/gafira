# Verify report: gafira-timeline

```json
{ "schema": 1, "change": "gafira-timeline", "delivery": "TL2", "rounds": [ {
  "round": 1, "commit": "4a0e856978b22469f0c7e3f02147d774e17c7905", "mode": "standard", "tier": "T0", "depth": "V1",
  "verdict": "FAIL", "next": "specifier",
  "counts": {"critical": 1, "warning": 8, "suggestion": 3},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/sequence.rs:237-249,276-281",
     "summary": "retimed() feeds snap_nearest's frame index into Flicks::from_ticks: a 2.5 s clip retimed 24->30 fps becomes 1 frame (23 520 000 ticks) instead of 75 (1 764 000 000). AC6's law is not enforced.",
     "follow_up": false, "status": "open"},
    {"id": "W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/tests/timeline_laws.rs:261-290",
     "summary": "AC6's test passes over C1: it asserts only `% frame == 0` and a clip count, on two one-frame clips whose buggy output coincides with the grid. No absolute position, tie or >1-frame clip.",
     "follow_up": false, "status": "open"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:188-202",
     "summary": "The retiming law has no hand mutant: the record's seven cover grid, 23,976, bounds, overlap, stacking, cap and speed, not the law that shipped broken (design TL2 critical code).",
     "follow_up": false, "status": "open"},
    {"id": "W3", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/clip.rs:54-70,57; specs/changes/gafira-timeline/design.md:479-480",
     "summary": "The speed relation's grid step is missing: Speed::sequence_span is unrounded, clip.rs:57 links a Sequence::place_span_for that does not exist, and nothing absorbs the remainder into source_in.",
     "follow_up": false, "status": "open"},
    {"id": "W4", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:133 (card AC4)",
     "summary": "AC4's 'inside its source'/'inside the sequence' cannot be checked: no source duration (Q5) and no stored sequence length (P4). Its half-open assertion is TimeRange arithmetic, not a model law.",
     "follow_up": false, "status": "open"},
    {"id": "W5", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:127,130 (card product IDs, AC1)",
     "summary": "ARQ-13 and FV-04 are card product IDs with no TL2 test naming or proving them for the timeline: validate() skips sequences/tracks/clips (TL3) and Clip has no invert/freeze (FV-04 c, d).",
     "follow_up": false, "status": "open"},
    {"id": "W6", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:139 (card AC10)",
     "summary": "AC10's 'undoable and redoable like the rest of the document' has no code or test: undo lives in gafira-edit::History and the timeline commands are deferred by Q4.",
     "follow_up": false, "status": "open"},
    {"id": "W7", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/src/json.rs:80-113; crates/gafira-doc/src/document.rs:15-27",
     "summary": "Document's manual Serialize/Deserialize drops sequences/assets; no test builds a document with a timeline and round-trips it, so Document's PartialEq and the wire disagree with nothing pinning UD-3.",
     "follow_up": false, "status": "open"},
    {"id": "W8", "severity": "WARNING", "class": "implementation",
     "ref": "git diff origin/develop HEAD --shortstat",
     "summary": "1 794 insertions in 14 files (1 673 code+tests) against the TL2 forecast 480 (670 at x1.4) and the ~700 ceiling; no owner OK for the size is recorded.",
     "follow_up": false, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/asset.rs:37-88; clip.rs:33-42,74-82",
     "summary": "AssetRef's custom 64-hex serde and Speed's gcd reduction are untested; the asset table is not on the wire yet, so a test gap, not a defect.",
     "follow_up": false, "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/clip.rs:57",
     "summary": "clip.rs links Sequence::place_span_for, which does not exist; cargo doc warns (broken intra-doc link) and no CI step runs cargo doc.",
     "follow_up": false, "status": "open"},
    {"id": "S3", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:126 (TL2 Flow line)",
     "summary": "The TL2 Flow line records tier T0 ('no Rust touched') for a delivery that writes critical Rust; inert under mutation: off and hand mutants were done, so a mislabel, not a missed gate.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": null,
  "criteria": {"total": 10, "covered": 6},
  "size": {"lines": 1794, "forecast": 670}, "report": "verify-report.md" } ] }
```

## Round 1 — TL2

- Delivery: `TL2` of `gafira-timeline`, verified at the merge commit
  `4a0e856978b22469f0c7e3f02147d774e17c7905` (`Merge branch
  'feat/gafira-timeline-tl2' into develop`), parents `2ff24c6` (origin/develop) and `0580c7f`
  (the delivery branch tip). The merge is contentwise a no-op over `2ff24c6`, so this round covers
  both. The commit is **not pushed**; this is its last gate before `develop` is pushed.
- Mode `standard`, depth **V1** (the TL2 Flow line says `verify: V1`; the owner grouped the rounds
  for this change). `mutation: off`: no `cargo-mutants` report exists for this delivery, and per the
  change's decision that is not a finding — the rigor is hand mutants and per-commit gates, no T4.
  V1 owes no independent mutant budget, so `hand_mutants` is null.
- Tier as recorded in the Flow line: `T0` (see `S3`; inert under `mutation: off`).
- Verdict: **FAIL** — 1 CRITICAL, 8 WARNING, 3 SUGGESTION. Every gate exits 0, but the retiming law
  of AC6 is not enforced in code (`C1`), and its test does not fail without the law (`W1`).
- Toolchain: cargo/rustc 1.93.1. Disk at start: 157 GB free.

### Gates (delivery HEAD, run once)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` |
| `cargo test --workspace --locked` | 0 | 195 test targets (178 unit/integration + 17 doctest), **2 468 passed, 0 failed, 0 ignored**; `tests/timeline_laws.rs` 10/10 |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0: Format; CLI has no UI deps (ARQ-07); `forbid(unsafe_code)`; V5 layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports` (its `clippy` and `test` both inside the step) |

The extractor `/tmp/gafira-policy-run.py` was re-checked against the current `ci.yml` before use: the
`policy` job has 10 `run:` steps, one of which is the toolchain install the extractor skips on
purpose, leaving the nine checks it ran literally under `bash -euo pipefail`. The job's last step is
`cargo-deny-action`, covered by the `cargo deny check` row.

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-tl2-v1-target` for the worktree and
`/tmp/tl2-probe-target` for the probe crate, both with `CARGO_INCREMENTAL=0`, both released with
`cargo clean --target-dir` when the round ended.

### Acceptance criteria (TL2 card, `apply-progress.md:129-139`)

| AC | Proves it | Verdict | Does it prove the criterion as worded? |
|---|---|---|---|
| AC1 sequence carries rate, size, sample rate; four track kinds; clip carries source, both offsets, speed, volume (FV-01, ARQ-13) | `timeline_laws::fv_01_a_sequence_and_a_clip_carry_their_fields` | covered | Yes for the fields and the four kinds. `TrackKind` is `#[non_exhaustive]` with exactly four variants; the clip exposes the six fields. The ARQ-13 half of the citation is not this delivery's — see `W5`. |
| AC2 ids come from the document's counter, unique across the document (ARQ-10, V-1) | `arq_10_and_v_1_the_timeline_ids_come_from_one_counter`, plus `DocMut::alloc_{sequence,track,clip,marker}_id` in `edit.rs:186-208` | covered | Yes: four ids draw distinct raw values and a page id joins the same set, so one counter serves pages and the timeline. |
| AC3 a cut lands on an exact frame at the rate, 23,976 exact (V-2, FV-03) | `v_2_and_fv_03_a_cut_lands_on_an_exact_frame_including_23_976`, `Track::check_clip` `track.rs:128-147` | covered | Yes: 29 429 400 flicks asserted twice (the rate and `Sequence::frame`), an off-grid start reported as `OffGrid`, the boundary accepted. |
| AC4 half-open, inside the source and the sequence, error on violation (FV-01) | `fv_01_a_clip_lies_inside_its_source_and_its_sequence` | **partial** | The span and source-offset laws are proved (`SpanTooShort`, `SourceBeforeStart`). "Inside the range of its source" and "inside the sequence" have no data to check (no source duration, no stored length) and the half-open assertion is `TimeRange` arithmetic — `W4`. |
| AC5 no overlap on a lane; stacking is the order of `tracks` (P11) | `p_11_no_lane_holds_two_clips_at_any_instant` (+ `p_7_…` for the stacking order) | covered | The overlap law is proved in both directions (inside an existing clip, and a cut exactly at its end accepted). The stacking assertion in this test is trivial (the vector keeps insertion order), but `p_7` proves the real rule with lane and clip ids in the opposite order. |
| AC6 retime snaps cuts to the new grid, deterministic, no clip lost (FV-01 (c)) | `fv_01_the_retime_snaps_cuts_to_the_new_grid` | **failing** | No. The test passes while `Sequence::retimed` is wrong: `C1`, probe below. The test can never fail for the collapse it should catch — `W1`. |
| AC7 sixteenth audio track accepted, seventeenth refused (FV-01) | `fv_01_the_audio_cap_sixteenth_enters_seventeenth_is_refused` | covered | Yes: sixteen `Audio` tracks, the seventeenth refused, a `Video` track still accepted, and the message names the seventeenth and the sequence. |
| AC8 speed relation between speed, source span and sequence span; zero/negative refused (FV-03) | `fv_03_the_duration_follows_the_speed_relation`, `Speed::sequence_span`/`source_span`, `clip.rs:54-70` | **partial** | The ratio is proved both ways and zero numerator/denominator are refused (a negative speed is unrepresentable through `u32`). The design's "rounded to the grid, the remainder into `source_in`" step has no code — `W3`. |
| AC9 the resolver query answers the clip per lane at an instant, `None` where nothing (P7) | `p_7_the_lane_answers_what_it_shows_at_an_instant` | covered | Yes: half-open at both ends, past the last clip, an empty lane, the per-lane order and index, and the exact source time at speed 1. |
| AC10 the mutation needs `EditToken`; undoable and redoable like the document (ARQ-30) | `arq_30_the_timeline_mutators_ask_for_the_token`, `DocMut` accessors, V13 step | **partial** | The token path and the non-repeating counter are proved (and V13 pins `EditToken` to the edit crates). The undo/redo clause has no code or test — `W6`. |

Product IDs of the card: `ARQ-01` (AC6's V5 step and the layer table), `ARQ-10` (AC2, the retime's
determinism half), `ARQ-13` (no TL2 test — `W5`), `ARQ-30` (AC10), `ARQ-136`
(`arq_136_the_measure_context_carries_no_time`, still green), `V-1` (AC2), `V-2` (AC3), `FV-01`
(AC1, AC4, AC5, AC6, AC7), `FV-03` (AC3, AC8), `FV-04` (no TL2 test — `W5`).

### Artifact integrity (fast lane, step 3c)

- `d70d475 docs(gafira-timeline): slice card for TL2` touches only
  `specs/changes/gafira-timeline/apply-progress.md` (1 file, 30 insertions), and precedes the first
  implementation commit `da35130`.
- The card block (lines 126-141) is unchanged by every later commit: the only implementer commit
  that touches the file, `0580c7f`, is a pure append (its diff has no removed line), and
  `da35130`/`b00b845` do not touch it.
- Delivery diff against `origin/develop` (`2ff24c6`): 14 files, 1 794 insertions, 9 deletions —
  exactly the figure the record states. Note the local `develop` ref has already been advanced to
  `4a0e856`, so `git diff develop` shows nothing; `origin/develop` is the base.

### Independent probe (beyond V1, for `C1`)

One targeted probe, in a crate outside the repository (`/tmp/tl2-probe`) path-depending on the
worktree's `gafira-doc`, used only to confirm the retiming defect:

```
one clip of 60 frames = 2.5 s at 24 fps, retimed to 30 fps
frame24=29400000 frame30=23520000
before: duration=1764000000 ticks (2.5 s)
after:  start=0 duration=23520000 end=23520000
expected: start=0 duration=1764000000 (75 frames at 30 fps, the same 2.5 s)
```

The clip loses 74 of its 75 frames. The cause is dimension: `snap_nearest` returns a **frame
index** (`q` or `q + 1`, `sequence.rs:276-281`) and `retimed` passes it to `Flicks::from_ticks` as
if it were ticks (`sequence.rs:237-249`). Only the single-frame case survives, by the coincidence
that `duration.max(frame)` is one frame and the second clip is then pushed by `last_end`.

### Hand mutants

Not run: depth V1 owes none, and `mutation: off` produces no report. The record's list
(`apply-progress.md:188-215`, seven items) was reviewed rather than repeated: its two claims of a
strengthened test hold against the current battery (H2 reads `sequence.frame()` and asserts
29 429 400; H5 puts two lanes over one instant with lane and clip ids in the opposite order), and
its `H1` is the assertion `v_2_…` still carries. The gap is that the **retiming law has no mutant
at all** — `W2`.

### Findings

| Id | Severity | Class | Ref | Summary | Status |
|---|---|---|---|---|---|
| C1 | CRITICAL | implementation | `sequence.rs:237-249`, `276-281` | `Sequence::retimed` treats a frame index as a number of ticks. `snap_nearest` returns `q`/`q+1`, and `retimed` builds `TimeRange` with `Flicks::from_ticks(start)` and `Flicks::from_ticks(duration)`. A 2.5 s clip retimed 24→30 fps becomes 1 frame (23 520 000 ticks) instead of 75 (1 764 000 000): every clip longer than one frame collapses to one frame and the following clips are pushed to `last_end`. The law of AC6 (and of the design's TL2 critical code, `design.md:481-487`) is not enforced. Two defects in one: the missing `* frame`, and snapping the duration as an independent quantity instead of snapping both cuts (`start` and `place.end`), which drifts when the two boundaries round inward. | open |
| W1 | WARNING | implementation | `timeline_laws.rs:261-290` | AC6's test is not a test that would fail without the law. It uses two clips of exactly one frame, starting at frames 0 and 1, and asserts only `start % frame_30 == 0`, a count of 2, purity and determinism. The buggy output — start 0 and 23 520 000, duration one frame each — satisfies every one of those assertions. It needs a clip longer than one frame, the absolute expected ticks of both cuts, and a tie case (the design breaks ties towards the later frame). | open |
| W2 | WARNING | implementation | `apply-progress.md:188-202` | The design says "the hand-mutant lists of `TL2` and `TL3` name the laws above; each law is one mutant". Seven mutants cover the grid, 23,976, the bounds, the overlap, the stacking, the cap and the speed relation; the retiming is missing, and it is the law that shipped broken. A retime mutant (return a frame index as ticks) would have been killed by a correct AC6 test. | open |
| W3 | WARNING | implementation | `clip.rs:54-70,57`; `design.md:479-480` | The design's speed rule is "the place of the clip is the source duration over the speed, **rounded to the grid**, and the source offset absorbs the remainder". The delivery has `Speed::sequence_span` (exact, unrounded) and `Clip::source_span`, but no grid step and no remainder absorption; `clip.rs:57` points at `Sequence::place_span_for`, which does not exist. A caller that uses `sequence_span` and hands the value to `Track::insert` gets `SpanTooShort` for any span that is not a whole number of frames. AC8's test proves the ratio only. | open |
| W4 | WARNING | ambiguity | `apply-progress.md:133` (card AC4) | AC4 asks that a clip lie "inside the range of its source and inside the sequence". The model has no source duration (owner decision Q5 puts the import metadata in the import change) and no stored sequence length (design P4: the length follows from the clips), so neither upper bound can be checked. The delivered laws are the design's narrower set (grid, at least one frame, `source_in >= 0`), which is defensible; the card over-words. The test's half-open assertion (`two.place().end().ticks() - frame == frame`) is arithmetic on `TimeRange`, not a law of the model. | open |
| W5 | WARNING | ambiguity | `apply-progress.md:127,130` | The card lists `ARQ-13` and `FV-04` (the design's TL2 row lists `FV-04` too), but no TL2 test names or proves either for the timeline. `Document::validate` does not inspect sequences, tracks or clips (the design puts that validation in TL3), and `Clip` has no invert/freeze metadata (FV-04 (c) and (d)); the only part proved is the speed half, under FV-03. Either the two IDs are over-listed for TL2 or the delivery is missing them; as written, the card's ID list and its ACs disagree. | open |
| W6 | WARNING | ambiguity | `apply-progress.md:139` (card AC10) | AC10's second clause, "every mutation that holds it is undoable and redoable like the rest of the document", has no code and no test. Undo lives in `gafira-edit::History` and needs a `Command` per edit; owner decision Q4 puts the timeline commands in a later change. The token half is proved and V13 backs it; the undo half belongs to the commands change, so the card should say so. | open |
| W7 | WARNING | implementation | `json.rs:80-113`; `document.rs:15-27` | The delivery replaced the derived `Serialize`/`Deserialize` on `Document` with a manual pair that writes only the four schema-1 fields, so `sequences` and `assets` are silently dropped on save and empty on load (the recorded deviation UD-3). That reading is right for this cut, but no test builds a document that carries a sequence and round-trips it, so nothing distinguishes the intended omission from a later regression; `Document`'s `PartialEq` now compares fields the wire discards, which no test states. The goldens prove only the timelineless bytes, which is why they stay green. | open |
| W8 | WARNING | implementation | diff vs `origin/develop` | 14 files, 1 794 insertions, 9 deletions — 1 673 of them code and tests, 121 the delivery record — against the TL2 forecast of 480 (670 at ×1.4) and the ~700-line ceiling. ×2.68 the forecast and ×2.5 the ceiling; no owner OK for the size is recorded in the card or the record. A signal for the coordinator to stop and ask, not a defect of the code. | open |
| S1 | SUGGESTION | implementation | `asset.rs:37-88`; `clip.rs:33-42,74-82` | `AssetRef`'s custom serde (64 hex characters, custom error) and `Speed`'s gcd reduction are untested. Neither is on the wire or reachable from an AC yet, so this is a test gap, not a defect; the reduction is a stated normal-form claim ("the same ratio has one form") with no test. | open |
| S2 | SUGGESTION | implementation | `clip.rs:57` | The intra-doc link `[`Sequence::place_span_for`]` points at a method that does not exist. `cargo doc` warns (`broken_intra_doc_links`); no CI step runs `cargo doc`, so no gate fails. It also marks the missing step of `W3`. | open |
| S3 | SUGGESTION | ambiguity | `apply-progress.md:126` | The TL2 Flow line records `tier: T0`, which the tier table defines as "no Rust touched", for a delivery that writes 1 673 lines of critical Rust. Judgment: a mislabelled metadata line, not a rigor gap — `mutation: off` governs and removes no gate that would otherwise run, and the hand mutants were done. It would be a real gap if the line were used to decide a T4. | open |

### Observations (not findings)

- **The wire did not move, and that is the right answer for this cut.** `json.rs` was edited, but no
  field was added: the two new `Document` fields are skipped on write and start empty on read. The
  TL1 evidence still holds — `adr_0006_old_bytes_resave_in_the_canonical_form`,
  `full_golden_equals_the_built_document` and `arq_10_a_page_duration_round_trips_through_the_wire_form`
  all ran green in this round — so a document written before this change still loads. The cost is
  the silent loss of a timeline on save, recorded as UD-3 and pinned by no test (`W7`).
- **The ARQ-136 guard still means something and was not loosened to pass.** `TIMELINE_SOURCES`
  (`visit_contract.rs:47-52`) grew from `page.rs` to the four files that hold the timeline's data
  and name its time types; `timeline/error.rs` and `timeline/asset.rs` are deliberately left in
  `DOMAIN_SOURCES` (checked by the same scan) because the error carries plain `i64` ticks, and the
  module-coverage loop forces any new module into the list. The narrow reading is the one the
  ARQ-136 row states after the owner's Q6 and the one `arq_136_the_measure_context_carries_no_time`
  enforces; the `MeasureCx` assertion is untouched. Nothing here was widened to make the delivery
  green.
- **Marker instants survive the retime.** `retimed` clones the markers unchanged; because a flick is
  an absolute unit, a marker at 2.5 s stays at 2.5 s even though the grid changed. That is correct
  for this law, which speaks of cuts.
- **Local `develop` is already at the merge.** `refs/heads/develop` points at `4a0e856`, so
  `git diff develop` is empty and `git status` is clean. The delivery's own base is `origin/develop`
  (`2ff24c6`); nothing is pushed.
- **The `-1` on a V5 row is not this delivery's.** The size and the eight-row V5 change belong to
  `TL1` and were verified there; TL2 adds no manifest edge.

next: specifier (ambiguity: W4, W5, W6, S3), then tdd-apply (implementation: C1, W1, W2, W3, W7, W8)

---

# Verify report: gafira-timeline — round 2

```json
{ "schema": 1, "change": "gafira-timeline", "delivery": "TL2", "rounds": [ {
  "round": 2, "commit": "3e4d553d5c3e9cb74084755d12860cb76215e218", "mode": "standard", "tier": "T0", "depth": "V1",
  "verdict": "FAIL", "next": "specifier",
  "counts": {"critical": 1, "warning": 6, "suggestion": 2},
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/sequence.rs:275-276",
     "summary": "Half closed: x frame on both, AC6's test now fails on the old code. Open: the end cut is snap(duration), not snap(place.end); up to one frame off (24->30 [1..2): 47 040 000 vs 70 560 000).",
     "follow_up": false, "status": "open"},
    {"id": "W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/tests/timeline_laws.rs:262-319",
     "summary": "Closed: a 60-frame clip with the absolute ticks of both cuts and a start-cut tie; it fails on the pre-fix code at timeline_laws.rs:296 (23 520 000 vs 1 764 000 000).",
     "follow_up": false, "status": "closed"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:204-205",
     "summary": "Closed: H8 (frame index as ticks) reproduces the old defect and the AC6 test kills it; H9 (the one-frame floor) also reproduces, left 0 right 29 400 000.",
     "follow_up": false, "status": "closed"},
    {"id": "W3", "severity": "WARNING", "class": "ambiguity",
     "ref": "crates/gafira-doc/src/timeline/sequence.rs:242-258; design.md:479-480",
     "summary": "Narrowed, open: place and remainder match the design's arithmetic; nothing folds the remainder into source_in and the doc redefines 'absorbs' as 'reports'. Direction is the design's to state.",
     "follow_up": false, "status": "open"},
    {"id": "W4", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:133",
     "summary": "Untouched; routed to the specifier by the record (apply-progress.md:222-228).",
     "follow_up": false, "status": "open"},
    {"id": "W5", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:127,130",
     "summary": "Untouched; ARQ-13 is still unnamed in timeline_laws.rs and FV-04's only naming test is gafira-audio's resample, not the timeline.",
     "follow_up": false, "status": "open"},
    {"id": "W6", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:139",
     "summary": "Untouched; routed to the specifier by the record.",
     "follow_up": false, "status": "open"},
    {"id": "W7", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/src/json.rs:80-113; document.rs:15-27",
     "summary": "Untouched; routed to TL3, where the timeline reaches the wire.",
     "follow_up": false, "status": "open"},
    {"id": "W8", "severity": "WARNING", "class": "implementation",
     "ref": "git diff --shortstat 2ff24c6 3e4d553",
     "summary": "Untouched; the fix adds 180 insertions on top of the delivery's 1 794; the coordinator's call.",
     "follow_up": false, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/asset.rs:37-88; clip.rs:33-42",
     "summary": "Untouched; neither the hex serde nor the gcd reduction is reachable from an AC in this cut.",
     "follow_up": false, "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/clip.rs:57",
     "summary": "Closed: cargo doc -p gafira-doc --no-deps with RUSTDOCFLAGS=-D warnings exits 0 on a clean export.",
     "follow_up": false, "status": "closed"},
    {"id": "S3", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:126",
     "summary": "Untouched; the tier label is still the specifier's.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 2, "killed": 2},
  "criteria": {"total": 10, "covered": 6},
  "size": {"lines": 180, "forecast": 670}, "report": "verify-report.md" } ] }
```

## Round 2 — TL2 closure

- Delivery `TL2`, verified at `3e4d553d5c3e9cb74084755d12860cb76215e218`
  (`Merge branch 'feat/gafira-timeline-tl2' into develop (the retime fix)`), parents `76b546f`
  (`origin/develop`) and `4a24b8b` (the delivery branch tip). A detached worktree; the merge is
  local only and nothing is pushed.
- **Round 2, a closure round.** Mode `standard`, depth `V1`, `mutation: off`. No independent mutant
  budget was spent and no probe beyond what closing a finding needs. The gates and the
  artifact-integrity checks of steps 3, 3b, 3c and 3d were re-run, then only the earlier findings
  were checked.
- Verdict: **FAIL** — `C1`'s first half is fixed, but its second half is not (the end cut is still
  snapped from the duration), and `W3`'s remainder is reported instead of absorbed. `W1`, `W2` and
  `S2` are closed. `W4`–`W8`, `S1` and `S3` are untouched and stay where the record put them.
- Toolchain: cargo/rustc 1.93.1. Disk at start: 154 GB free. The failure is not a gate failure:
  every gate exits 0 and the defect is a law the gates have no test for.

### Gates (delivery HEAD, run once this round)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` |
| `cargo test --workspace --locked` | 0 | 195 test targets, **2 469 passed, 0 failed, 0 ignored**; `tests/timeline_laws.rs` **11/11** (round 1: 2 468 and 10 — `60b69a0` adds the place test) |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0 |

The extractor `/tmp/gafira-policy-run.py` was re-checked against `ci.yml` before use: the `policy`
job has 12 steps, 10 of them `run:`; the extractor skips the toolchain install and the
`cargo-deny-action` step is covered by the `cargo deny check` row, leaving the nine checks it ran
under `bash -euo pipefail`: Format; ARQ-07; `forbid(unsafe_code)`; V5 layer table; V12
`clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit `ports`.

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-tl2-v2-target` for the worktree and
`/tmp/tl2v2-export-target` for the scratch export, both with `CARGO_INCREMENTAL=0`, both released
with `cargo clean --target-dir` when the round ended.

### Closure of the round-1 findings

| Id | Round 1 | Now | Evidence (the test or probe that decides it) |
|---|---|---|---|
| `C1` | open | **still open** | Half closed. The index is multiplied by the frame (`sequence.rs:275-276`) and AC6's test now fails on the pre-fix code. The end cut is still `snap(start) + snap(duration)` instead of `snap(start + duration)`: the probe shows one frame of drift in both directions. |
| `W1` | open | **closed** | `timeline_laws.rs:262-319` has a 60-frame clip, the absolute ticks of both cuts and a start-cut tie. Run against the pre-fix `retimed` it fails at `:296` with `left: 23 520 000, right: 1 764 000 000`. |
| `W2` | open | **closed** | `H8` is the exact old defect; my pre-fix run reproduces it and the AC6 test kills it, which is the record's claim. `H9` also reproduces: dropping the one-frame floor fails the place test with `left: 0, right: 29 400 000`. |
| `W3` | open | **still open** (narrowed) | The place arithmetic is right; the design's second clause is not implemented. See below. |
| `W4` | open | open, untouched | The record says the specifier has it (`apply-progress.md:222-228`); no code change in this round. Not a new finding. |
| `W5` | open | open, untouched | `ARQ-13` still does not appear in `timeline_laws.rs`; `FV-04`'s only naming test is `crates/gafira-audio/tests/resample.rs`. |
| `W6` | open | open, untouched | The record routes it to the specifier. |
| `W7` | open | open, untouched | The record routes it to `TL3`. |
| `W8` | open | open, untouched | The coordinator's call; this round adds 180 insertions on top of the 1 794, it does not shrink them. |
| `S1` | open | open, untouched | Still unreachable from an AC. |
| `S2` | open | **closed** | `RUSTDOCFLAGS="-D warnings" cargo doc -p gafira-doc --no-deps` exits 0 on a clean export of `3e4d553`. |
| `S3` | open | open, untouched | The specifier's. |

#### `C1` — half closed, the other half still open

The fix is `snap_nearest(...) * frame` on **both** the start and the duration
(`sequence.rs:275-276`), so the frame-index-as-ticks defect is gone and the new test catches it.
That test is honest: reverted to `cfbadc7^`'s two lines in a scratch export, it fails at

```
thread 'fv_01_the_retime_snaps_cuts_to_the_new_grid' panicked at timeline_laws.rs:296:
assertion `left == right` failed: 2.5 s at 30 fps is 75 frames, not one
  left: 23520000
 right: 1764000000
```

The assertion that catches it is `timeline_laws.rs:296`,
`video[0].place().duration().ticks() == 1_764_000_000`; the tie clip's start assertion at `:309-312`
is what would catch the start half on its own (the video clip starts at 0, where both halves
coincide).

What round-1 `C1` called its second defect is still there: the duration is snapped as an
independent quantity (`sequence.rs:276`), so the end cut lands on `snap(start) + snap(duration)`,
not on `snap(start + duration)`. Rounding is not additive, so the end cut is not the nearest frame
of the old end, which is what `design.md:481-482` says ("moves **every cut** to the nearest frame of
the new grid"). The probe (`/tmp/tl2v2-export`, path aside, no repo change):

```
24 fps, clip [frame 1..2) -> 30 fps
  got start=23 520 000 end=47 040 000 dur=23 520 000
  law start=23 520 000 end=70 560 000        (2.5 frames is a tie -> the later frame, 3)
30 fps, clip [frame 2..4) -> 24 fps
  got start=58 800 000 end=117 600 000 dur=58 800 000
  law start=58 800 000 end=88 200 000        (1.6 and 3.2 frames both round down)
```

The first case loses a frame the law keeps; the second keeps a frame the law drops. Both are one
frame, both are in the retiming, which `design.md:487` and the card's Plan name as critical code.
No test in the battery asserts an end cut whose nearest frame differs from `start + snapped
duration` — the 60-frame clip starts at 0, where the two readings agree, and the tie case is a
start. That gap is inside `C1` (round 1 named it), not a new finding.

#### `W3` — narrowed, still open

Checked against the design's wording (`design.md:479-480`), not against the new test's
expectations. The arithmetic holds:

| Input (24 fps, frame 29 400 000) | `exact = source × den / num` | `place` | Design's "source/speed rounded" | `remainder = source − speed × place` |
|---|---|---|---|---|
| 60 frames, speed 2/1 | 30 frames | 30 frames | 30 | 0 |
| 1 frame + 600 000 ticks, speed 1/1 | 1.02 frames | 1 frame | 1 (nearest) | +600 000 |
| 100 ticks, speed 1/1 | 0 frames | 1 frame | 0, floored by the one-frame law | −29 300 000 |

The place is the design's rounded place, the floor at one frame is the design's own law, and the
remainder's sign and dimension are right. What is missing is the design's second clause: nothing
**puts** the remainder anywhere. `place_span_for` returns it (`sequence.rs:242-258`) and its doc
redefines "the source offset absorbs the remainder" as "the window keeps it unread and it is
reported". The record says the same and calls the alternative — shifting `source_in` — an owner
decision (`apply-progress.md`, Round 1 closure). That is a fair reading of a terse sentence, but it
is not the design's sentence, and the residual is a question for the design, not for a test: the
*bound* on a negative remainder needs Q5, but the *direction* does not. Note that the write itself
is a command's under ARQ-30 and Q4, so the design should say whether `TL2`'s job is to report and a
command's job is to fold, or whether a pure constructor owes the adjusted `source_in`.

### Artifact integrity (fast lane, step 3c)

- `d70d475 docs(gafira-timeline): slice card` touches only
  `specs/changes/gafira-timeline/apply-progress.md` (+30) and precedes `da35130`.
- The `## TL2` card block is **byte-identical** between `d70d475` and `3e4d553` (extracted by
  heading and diffed). The file only grew: `git diff d70d475 3e4d553` on it has no removed line.
  The 16 lines that shift the block come from `TL3`'s card on `develop`, not from a `TL2` commit.
- No implementer commit touched the card. `cfbadc7` and `60b69a0` touch `gafira-doc` only;
  `4a24b8b` appends the record and the closure section (`+32/−4`, all inside `apply-progress.md`
  from line 190 on) and states why.
- The merge keeps the fix intact: `git diff 4a24b8b 3e4d553 -- crates/` shows only
  `gafira-render-cpu` (work that arrived from `develop`), nothing in `gafira-doc`, and no conflict
  markers survive.
- Product IDs of the card, unchanged from round 1: `ARQ-01`, `ARQ-10`, `ARQ-13`, `ARQ-30`,
  `ARQ-136`, `V-1`, `V-2`, `FV-01`, `FV-03`, `FV-04`. `ARQ-13` and `FV-04` still have no `TL2` test
  naming them — that is `W5`, untouched, not this round's finding.

### Probes run (only what closing a finding needed)

1. **AC6 against the pre-fix `retimed`** — scratch export of `3e4d553`, the two lines restored to
   `cfbadc7^`: the strengthened test fails as quoted above. `W1` closed, `C1`'s first half proved,
   and `H8` confirmed dead.
2. **Both-cuts probe** — two clips across two rate pairs, fixed code, in the same export:
   the drift table above. `C1`'s second half proved open.
3. **`H9`** — the one-frame floor removed from `place_span_for`, place test fails
   `left: 0, right: 29 400 000`, reverted. `W2`'s second claim confirmed.
4. **`cargo doc`** — `RUSTDOCFLAGS=-D warnings`, exit 0. `S2` closed.

No new mutant was designed; 2 of the record's 9 were re-run as spot-checks and both died.

### Findings

| Id | Severity | Class | Ref | Summary | Status |
|---|---|---|---|---|---|
| C1 | CRITICAL | implementation | `sequence.rs:275-276` | The frame-index defect is fixed on both fields and AC6's test now fails on the old code, but the end cut is still `snap(duration)` added to a snapped start instead of `snap(place.end)`. Rounding is not additive, so the end cut misses the nearest frame by up to one frame in either direction (24→30 fps `[1..2)` ends at 47 040 000 where the law says 70 560 000; 30→24 fps `[2..4)` ends at 117 600 000 where it says 88 200 000). `design.md:481-482` says every cut goes to the nearest frame; no test asserts an end cut where the two readings differ. | open |
| W1 | WARNING | implementation | `timeline_laws.rs:262-319` | Closed. 60-frame clip, absolute ticks of both cuts, start-cut tie; fails on the pre-fix code at `:296`. | closed |
| W2 | WARNING | implementation | `apply-progress.md:204-205` | Closed. `H8` reproduces the old defect and is killed by AC6's test; `H9` reproduces as recorded. | closed |
| W3 | WARNING | ambiguity | `sequence.rs:242-258`; `design.md:479-480` | Narrowed. The place, the floor and the signed remainder match the design's arithmetic; the remainder is reported, never folded into `source_in`, and the method's doc and the record both argue the design's "absorbs" away. The design owes the direction (and whether a command, not a pure function, does the write). | open |
| W4 | WARNING | ambiguity | `apply-progress.md:133` | Untouched; the specifier's. | open |
| W5 | WARNING | ambiguity | `apply-progress.md:127,130` | Untouched; `ARQ-13`/`FV-04` still have no `TL2` test naming them. | open |
| W6 | WARNING | ambiguity | `apply-progress.md:139` | Untouched; the specifier's. | open |
| W7 | WARNING | implementation | `json.rs:80-113`; `document.rs:15-27` | Untouched; `TL3`'s. | open |
| W8 | WARNING | implementation | `git diff --shortstat 2ff24c6 3e4d553` | Untouched; the coordinator's. The fix adds 180 insertions. | open |
| S1 | SUGGESTION | implementation | `asset.rs:37-88`; `clip.rs:33-42` | Untouched. | open |
| S2 | SUGGESTION | implementation | `clip.rs:57` | Closed; `cargo doc -D warnings` clean. | closed |
| S3 | SUGGESTION | ambiguity | `apply-progress.md:126` | Untouched; the specifier's. | open |

### Observations (not findings)

- **The delivery diff against `develop` is now the fix, not the delivery.** `git diff 76b546f
  3e4d553` is 6 files, 180 insertions, 35 deletions: the retime fix, the place rule, the tests and
  the record. The original 1 794-insertion figure of `W8` is measured from `2ff24c6` and is
  unchanged by this round.
- **The record's closure claims are accurate except for `C1` and `W3`.** `W1`, `W2` and `S2` hold
  under re-run, and the record does not overstate what `W3` did — it says "reported" and names the
  owner decision, which is why the residual is an ambiguity rather than a quiet regression.
- **The new place test is written to the implementation, not to the design.** It pins `place`,
  `remainder` and the floor, all of which are right; it cannot see the absorption clause because
  the implementation has no code for it.
- **`retimed`'s `.max(last_end)` is a second, smaller source of drift** beyond the snapped
  duration: a pushed start keeps the independently snapped duration, so a collision moves the end
  cut too. It is inside `C1` and worth fixing with the end-cut snap.

next: specifier (ambiguity: W3's direction, W4, W5, W6, S3), then tdd-apply (implementation: C1's end cut; W7 with TL3; W8 is the coordinator's)

---

# Verify report: gafira-timeline — round 3 (closure)

```json
{ "schema": 1, "change": "gafira-timeline", "delivery": "TL2", "rounds": [ {
  "round": 3, "commit": "5789465554461e3e4fc69e2581b0761e1ae5f056", "mode": "standard", "tier": "T0", "depth": "V1",
  "verdict": "PASS", "next": "tdd-apply",
  "counts": {"critical": 0, "warning": 6, "suggestion": 2},
  "classes": ["implementation", "ambiguity"],
  "findings": [
    {"id": "C1", "severity": "CRITICAL", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/sequence.rs:279-293",
     "summary": "Closed: each cut snaps on its own, the span comes from from_bounds. RED reproduced on 4a24b8b-era code (47 040 000/70 560 000 and 117 600 000/88 200 000); the merged tree answers the law.",
     "follow_up": false, "status": "closed"},
    {"id": "W1", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/tests/timeline_laws.rs:262-319",
     "summary": "Still closed: the 60-frame clip, the absolute ticks of both cuts and the start tie pass in the merged tree (timeline_laws 12/12).",
     "follow_up": false, "status": "closed"},
    {"id": "W2", "severity": "WARNING", "class": "implementation",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:290",
     "summary": "Still closed: H8/H9 reproduce and are killed; H10 (end = start + snapped duration) is the defect's own shape and is killed by the new test.",
     "follow_up": false, "status": "closed"},
    {"id": "W3", "severity": "WARNING", "class": "ambiguity",
     "ref": "crates/gafira-doc/src/timeline/sequence.rs:242-258; specs/changes/gafira-timeline/design.md:479-480",
     "summary": "Open, now labelled ambiguity in this report: the code reports the remainder and the doc redefines 'absorbs' as 'reports'. The design owes the direction; a specifier decision, no code change.",
     "follow_up": false, "status": "open"},
    {"id": "W4", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:133",
     "summary": "Untouched; the specifier's (AC4's two bounds need a source duration and a stored sequence length).",
     "follow_up": false, "status": "open"},
    {"id": "W5", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:127,130",
     "summary": "Untouched; ARQ-13 and FV-04 still have no TL2 test naming them (0 hits in timeline_laws.rs).",
     "follow_up": false, "status": "open"},
    {"id": "W6", "severity": "WARNING", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:139",
     "summary": "Untouched; the specifier's (AC10's undo clause, deferred with the commands by Q4).",
     "follow_up": false, "status": "open"},
    {"id": "W7", "severity": "WARNING", "class": "implementation",
     "ref": "crates/gafira-doc/src/json.rs:80-113; crates/gafira-doc/src/document.rs:15-27",
     "summary": "Untouched; TL3's, where the timeline reaches the wire.",
     "follow_up": false, "status": "open"},
    {"id": "W8", "severity": "WARNING", "class": "implementation",
     "ref": "git diff --shortstat 2ff24c6 5789465 -- crates/gafira-doc specs/changes/gafira-timeline",
     "summary": "Untouched; 2 089 insertions in 14 files against the 480/670 forecast and the ~700 ceiling; this round adds 142. The coordinator's call.",
     "follow_up": false, "status": "open"},
    {"id": "S1", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/asset.rs:37-88; clip.rs:33-42",
     "summary": "Untouched; neither the hex serde nor the gcd reduction is reachable from an AC in this cut.",
     "follow_up": false, "status": "open"},
    {"id": "S2", "severity": "SUGGESTION", "class": "implementation",
     "ref": "crates/gafira-doc/src/timeline/clip.rs:57",
     "summary": "Still closed: RUSTDOCFLAGS=\"-D warnings\" cargo doc -p gafira-doc --no-deps exits 0 on the merged tree.",
     "follow_up": false, "status": "closed"},
    {"id": "S3", "severity": "SUGGESTION", "class": "ambiguity",
     "ref": "specs/changes/gafira-timeline/apply-progress.md:126",
     "summary": "Untouched; the `tier: T0` label is still the specifier's.",
     "follow_up": false, "status": "open"} ],
  "gates": {"fmt": 0, "clippy": 0, "test": 0, "deny": 0, "ci_policy": 0},
  "mutation": null,
  "hand_mutants": {"run": 2, "killed": 2},
  "criteria": {"total": 10, "covered": 6},
  "size": {"lines": 142, "forecast": 670}, "report": "verify-report.md" } ] }
```

## Round 3 — TL2 closure (the owner's last authorised round)

- Delivery `TL2`, verified at `5789465554461e3e4fc69e2581b0761e1ae5f056`
  (`Merge branch 'feat/gafira-timeline-cuts' into develop (both cuts of the retime)`), a detached
  worktree. Parents: `1c9c3f8` (`develop`, a `gafira-render-cpu` design commit: +67/−1, not this
  change) and `d928ef5` (the delivery branch tip). The delivery branch carries exactly two commits
  for this round: `f103281` (the fix and its test) and `d928ef5` (its record). **Nothing is
  pushed**: `origin/develop` is still at `76b546f`, which carries the defect.
- **A closure round.** Mode `standard`, depth `V1`, `mutation: off`. No independent-mutant budget
  was spent. The gates and the artifact-integrity checks were re-run, then only the earlier
  findings were checked; the two probes below exist because closure of `C1` and of the two
  behaviours the round had to keep needs them.
- Verdict: **PASS** — `C1` is closed on both halves, every gate exits 0, and no new finding
  appeared. Six WARNINGs and two SUGGESTIONs stay open exactly where the record put them; none of
  them is TL2 code this round could have closed.
- Toolchain: cargo/rustc 1.93.1 (`01f6ddf75`). Disk at start: 148 GB free.

### Gates (merged commit, run alone)

| Gate | Exit | Evidence |
|---|---|---|
| `cargo fmt --check` | 0 | no diff |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | `Finished dev profile` |
| `cargo test --workspace --locked` | 0 | 195 test targets, **2 470 passed, 0 failed, 0 ignored**; `tests/timeline_laws.rs` **12/12** (round 2: 2 469 and 11 — `f103281` adds `fv_01_the_retime_moves_both_cuts_of_a_clip`) |
| `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| CI policy, job `policy` | 0 | 9/9 steps exit 0 (`POLICY OK`) |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p gafira-doc --no-deps` | 0 | `Generated .../gafira_doc/index.html` |

The extractor `/tmp/gafira-policy-run.py` was re-checked against `ci.yml` before use: the `policy`
job has 12 steps, of which `actions/checkout`, the toolchain install and
`EmbarkStudios/cargo-deny-action` carry no `run:`; the extractor skips the install and the nine
remaining checks ran under `bash -euo pipefail` — Format; ARQ-07; `forbid(unsafe_code)`; the V5
layer table; V12 `clippy.toml`; ARQ-107 language; V13 `EditToken`; V14 property test; testkit
`ports`. The deny action is covered by the `cargo deny check` row.

Build hygiene: `CARGO_TARGET_DIR=/tmp/gafira-tl2-v3-target` for the worktree and one target dir per
scratch export (`/tmp/tl2r3-export-target` for the RED tree, `/tmp/tl2r3-probe-target` for the
probes), all with `CARGO_INCREMENTAL=0`, all released with `cargo clean --target-dir` when the
round ended. `git status` in the worktree shows only this report: no source file was touched.

### Closure of every earlier finding

| Id | Round 2 | Now | Evidence (what decides it) |
|---|---|---|---|
| `C1` | open (half closed) | **closed** | Both cuts snap on their own (`sequence.rs:279-280`) and the span is derived with `from_bounds` (`:289-293`). Both recorded RED cases reproduced on pre-fix code; the merged tree answers the law. See below. |
| `W1` | closed | **closed** (stays) | The 60-frame clip, the absolute ticks of both cuts and the start tie are still asserted and pass (`timeline_laws.rs:262-319`). |
| `W2` | closed | **closed** (stays) | `H8`/`H9` unchanged; `H10` (the defect's own shape) is recorded and killed by the new test (`apply-progress.md:290`). |
| `W3` | open (narrowed) | **open**, class `ambiguity` | The place, the floor and the signed remainder match the design's arithmetic; the remainder is reported, nothing folds it into `source_in`. A design/specifier decision, not a code defect — see below. |
| `W4` | open | **open**, untouched | The specifier's; the record says so (`apply-progress.md:222-228`). |
| `W5` | open | **open**, untouched | `ARQ-13` and `FV-04` still have **0** occurrences in `timeline_laws.rs`. |
| `W6` | open | **open**, untouched | The specifier's (AC10's undo clause). |
| `W7` | open | **open**, untouched | `TL3`'s, where the timeline reaches the wire; `d928ef5` does not touch `json.rs` or `document.rs`. |
| `W8` | open | **open**, untouched | 2 089 insertions in 14 files against `2ff24c6` (round 2 measured 1 794 + 180); this round adds 142 (−8). The coordinator's call. |
| `S1` | open | **open**, untouched | Neither the `AssetRef` hex serde nor `Speed`'s reduction is reachable from an AC in this cut. |
| `S2` | closed | **closed** (stays) | `cargo doc` with `-D warnings` exits 0 on the merged tree. |
| `S3` | open | **open**, untouched | The `tier: T0` label is still the specifier's. |

Closed by this round: `C1`. Still open: `W3`–`W8`, `S1`, `S3` — each routed where the record and
round 2 routed it, none re-opened as a new finding.

#### `C1` — closed on both halves

The merged code is exactly the snapshot this round was asked to confirm
(`crates/gafira-doc/src/timeline/sequence.rs:279-293`):

```rust
let start = snap_nearest(clip.place.start().ticks(), frame) * frame;
let end = snap_nearest(clip.place.end().ticks(), frame) * frame;
let start = start.max(last_end);
let end = end.max(start.saturating_add(frame));
last_end = end;
// ...
moved.place = gafira_time::TimeRange::from_bounds(start, end)
```

**RED reproduction, independently, outside the repo.** `/tmp/tl2r3-export` is `git archive
4a24b8b` (the pre-fix tree, `snap(duration)`) plus the merged `timeline_laws.rs`; `f103281`'s diff
to that file is a pure +90 insertion, so the export is exactly "pre-fix code + new test", not a
reconstruction. Both recorded cases reproduce, and the assertion values match the record to the
tick:

```
thread 'fv_01_the_retime_moves_both_cuts_of_a_clip' panicked at timeline_laws.rs:354:
assertion `left == right` failed: the end cut snaps on its own: 2.5 frames of 30 fps goes to the later frame
  left: 47040000
 right: 70560000
```

and, with the grow assertion temporarily set to its buggy value (the record discloses the same
technique) to reach the shrink case:

```
thread 'fv_01_the_retime_moves_both_cuts_of_a_clip' panicked at timeline_laws.rs:381:
assertion `left == right` failed: the end cut snaps to 3 frames of 24 fps, not the snapped span added to the start
  left: 117600000
 right: 88200000
```

**GREEN, the merged tree answers exactly those numbers.** `5789465`, `cargo test -p gafira-doc
--test timeline_laws`: 12 passed, 0 failed, `fv_01_the_retime_moves_both_cuts_of_a_clip ... ok`,
whose assertions are `end == 70 560 000` for 24→30 fps `[29 400 000..58 800 000)` and
`end == 88 200 000` for 30→24 fps `[47 040 000..94 080 000)`
(`crates/gafira-doc/tests/timeline_laws.rs:331-413`). The start cuts (`23 520 000` and
`58 800 000`) are asserted too, so a regression that moves the pair together still fails.

#### The two behaviours that had to survive — both still pinned

The new test's last block (`timeline_laws.rs:387-412`) claims to guard both. I checked the claim
with two hand mutants on `/tmp/tl2r3-probe` (a clean export of `5789465`), each applied alone and
reverted:

| Mutant | Wrong behaviour | Result |
|---|---|---|
| the push-right guard removed (`.max(last_end)`) | an overlapping clip keeps its snapped start | **killed**: `assertion left == right failed: the clip that would overlap is pushed right`, `left: 0, right: 29400000` |
| the end floor removed (`.max(start + frame)`) | a span below one frame collapses to zero | **killed**: `assertion left == right failed: a span below one frame keeps one frame`, `left: 0, right: 29400000` |

Both mutations are invisible to the older `fv_01_the_retime_snaps_cuts_to_the_new_grid`, so the
behaviour really is pinned by the new test, not by coincidence of a 60-frame clip.

#### `W3` — the record's framing (checked, then left alone)

`W3` is an ambiguity, and this report says so. The record does **not** use that word: it presents
`W3` in the round-1 closure table as fixed by `60b69a0` and states that "the alternative (shifting
`source_in` by the remainder) moves content and needs an owner decision"
(`apply-progress.md:248`). That names the open direction — and it is the honest half — but it
files `W3` under "fixed", not under "the design owes a sentence". The classification lives only in
this report and in round 2's. Reading of the design left to the specifier: the code satisfies
"rounded to the grid" and reports the remainder; it does not do "the source offset absorbs the
remainder". No code change is asked for here, and nothing in this round re-opens it as a defect.

### Commit granularity — compatible with the per-commit gates rule

`f103281` carries the test and the fix in one commit, with the RED evidence captured on the
pre-fix tree; `d928ef5` carries the record. That is what `AGENTS.md` ("Strict TDD") requires, not a
deviation from it:

- "**Every commit must pass alone**, scoped to what it can break." A commit holding only the
  failing test would fail `cargo test -p gafira-doc --locked` and so could never be pushed. The
  shape that satisfies the rule is exactly test + minimal code, green at that commit.
- The RED obligation is a **run**, not an artefact: "The same scope applies to the RED run of
  every acceptance criterion and to the test that kills each hand mutant." The record holds both
  RED transcripts and the `H10` kill in `apply-progress.md`.
- "One commit per reviewable work unit": one law, one commit. The record commit is docs-only and
  needs no cargo gates; the record states the gates it ran on `f103281` (`cargo fmt`, `clippy -p
  gafira-doc`, `cargo test -p gafira-doc`, `deny`, the nine policy steps, `cargo doc`).
- One caveat that the rule does not create but is worth naming: a RED transcript is a
  self-reported artefact, so it is only as good as its reproduction. I reproduced both cases
  independently and they match to the tick, so the record's `C1` claim is verified, not merely
  asserted.

### Artifact integrity (fast lane, step 3c)

- `d70d475 docs(gafira-timeline): slice card` touches only
  `specs/changes/gafira-timeline/apply-progress.md` (+30) and precedes the first implementation
  commit.
- The `## TL2` card block is **byte-identical** between `d70d475` and `5789465` (extracted by
  heading and compared; 18 lines each). No implementer commit changed the card.
- The only commit in this round that touches the file is `d928ef5` (+39 lines appended after
  `## TL2`'s region, from the `Round 2 closure` heading on). It is the record, it states why, and
  it removes no line: the card cannot have moved.
- `f103281` touches `crates/gafira-doc` only (`sequence.rs` +13/−8, `timeline_laws.rs` +90).
- The merge is not purely TL2: `git diff 3e4d553 5789465` also carries `1c9c3f8`'s
  `gafira-render-cpu/design.md` (+67/−1) from `develop`, which is another change's artefact and no
  TL2 commit's doing. The TL2 part of `76b546f..5789465` is 6 files, +319/−40.
- No conflict markers survive in any `.rs` file.
- Card product IDs, unchanged: `ARQ-01`, `ARQ-10`, `ARQ-13`, `ARQ-30`, `ARQ-136`, `V-1`, `V-2`,
  `FV-01`, `FV-03`, `FV-04`. `ARQ-13` and `FV-04` still have no `TL2` test naming them — `W5`,
  untouched, not this round's finding.

### Findings

| Id | Severity | Class | Ref | Summary | Status |
|---|---|---|---|---|---|
| C1 | CRITICAL | implementation | `sequence.rs:279-293` | Closed. Each cut snaps on its own and the span is derived with `from_bounds`; both recorded RED cases reproduced on `4a24b8b`-era code (47 040 000/70 560 000 and 117 600 000/88 200 000) and the merged tree answers them. | closed |
| W1 | WARNING | implementation | `timeline_laws.rs:262-319` | Stays closed; the strengthened test passes. | closed |
| W2 | WARNING | implementation | `apply-progress.md:290` | Stays closed; `H8`, `H9`, `H10` all reproduce and are killed. | closed |
| W3 | WARNING | ambiguity | `sequence.rs:242-258`; `design.md:479-480` | Open. The remainder is reported, never absorbed; the design owes the direction. A specifier decision; the record calls it an owner decision but files it as fixed, not as an open ambiguity. | open |
| W4 | WARNING | ambiguity | `apply-progress.md:133` | Untouched; the specifier's. | open |
| W5 | WARNING | ambiguity | `apply-progress.md:127,130` | Untouched; `ARQ-13`/`FV-04` still unnamed in the TL2 tests. | open |
| W6 | WARNING | ambiguity | `apply-progress.md:139` | Untouched; the specifier's. | open |
| W7 | WARNING | implementation | `json.rs:80-113`; `document.rs:15-27` | Untouched; `TL3`'s. | open |
| W8 | WARNING | implementation | `git diff --shortstat 2ff24c6 5789465 -- crates/gafira-doc specs/changes/gafira-timeline` | Untouched; 2 089 insertions in 14 files; this round adds 142. The coordinator's call. | open |
| S1 | SUGGESTION | implementation | `asset.rs:37-88`; `clip.rs:33-42` | Untouched. | open |
| S2 | SUGGESTION | implementation | `clip.rs:57` | Stays closed; `cargo doc -D warnings` clean. | closed |
| S3 | SUGGESTION | ambiguity | `apply-progress.md:126` | Untouched; the specifier's. | open |

### Observations (not findings)

- **The merge is the smallest TL2 delta of the three rounds**: two commits, 142 insertions, and
  the only source change is `retimed`'s cut arithmetic. Nothing else in `gafira-doc` moved.
- **`d928ef5` keeps the record honest about the shrink case.** It discloses that the grow
  assertion was temporarily set to its buggy value to reach the shrink one, which is the same
  technique this round used; the committed assertion is the law's value (70 560 000), checked.
- **AC6 is now enforced end to end**, but I did not re-derive the criteria matrix: a closure round
  does not, so `criteria` stays at 10/6 as round 1 left it. The parts AC6 still does not decide
  are `W3` (absorption) and the card-level gaps `W4`/`W5`/`W6`, all ambiguities.
- **`W8` is now the largest open item by measure**, and it is a process question, not a defect:
  2 089 insertions against a 480-line forecast and a ~700 ceiling. It needs the owner's OK or a
  split before `TL3` lands on top, since `TL3` will touch the same files.
- **Nothing is pushed.** `origin/develop` at `76b546f` still carries the `C1` defect, so this
  merge is what a push would deliver; every gate above ran on the merged tree, not on the branch.

next: tdd-apply (TL3, the next delivery of `gafira-timeline`); W3, W4, W5, W6, S3 remain the specifier's (ambiguity), W7 is TL3's, W8 is the coordinator's
