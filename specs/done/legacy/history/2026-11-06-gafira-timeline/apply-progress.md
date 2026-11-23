## Slice card
Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the timeline opens with the edge that lets the document name time, which every later delivery of this change needs; the rigor is the one the owner fixed for the previous two cuts (O4, O8, O10): hand mutants, gates per commit, one V1 round per group, no T4 at close
Product IDs: ARQ-01, ARQ-02, ARQ-10, ARQ-13, ARQ-30, ARQ-136 (source: specs/02-arquitectura-software.md §2.1, §2.2, §3.1, §3.2, §4.14; ADR 0007; V-2 of specs/05-funcionalidad-video-y-distribucion.md)
Scope: `TL1` of `specs/changes/gafira-timeline/design.md`: in `gafira-doc`, the dependency on `gafira-time` and `Page.duration: Option<Flicks>` with its wire form; in `ci.yml`, the V5 rows that must gain `gafira-time` because the V5 step reads the whole `cargo tree`; and the goldens the new field moves.      Non-goals: `Sequence`, `Track` and `Clip` (`TL2`), the persistence of the model (`TL3`), the commands and the resolver (later changes), and any change to `gafira-ports` or to an engine crate beyond the `Cargo.toml` edge and its V5 row.
Acceptance:
  AC1 Given `gafira-doc`, When its manifest is inspected, Then it declares `gafira-time` and the crate builds, and the edge is the one ADR 0007 named as arriving with `Page.duration` (ARQ-01, V-5) [command]
  AC2 Given a `Page`, When it carries a duration, Then it is an `Option<Flicks>`, absent when the page has none, and the type comes from `gafira-time` and not from a unit of the crate's own (V-2, ARQ-136) [unit]
  AC3 Given a page without a duration, When it is written and read back, Then the field is absent in the wire form and the round trip gives `None`, so a document written before this change still loads (ARQ-10, ARQ-13, ADR 0006 rule 3) [unit]
  AC4 Given the V5 step, When the workspace is checked, Then every crate whose `cargo tree` now reaches `gafira-time` has it in its row, and the step passes (V-5) [command]
  AC5 Given the byte-pinned golden, When the tests run, Then it moves with the new field and the artifact-integrity review confirms it changed only for that reason (ARQ-10, ADR 0006 rule 3) [unit]
  AC6 Given the crate, When the CI policy steps run, Then `gafira-doc` still has `#![forbid(unsafe_code)]`, its V5 row passes, V12 keeps its `clippy.toml` identical and ARQ-107 still holds (ARQ-02, ARQ-107) [command]
  AC7 Given a `Document` with pages and their durations, When the document is cloned or compared, Then the new field takes part in equality and in the deterministic ordering the crate already fixes (ARQ-10, ARQ-13) [unit]
Plan: the delivery is an edge and a field, not a model. It touches the crates' manifests and the V5 table as one work unit, because the V5 step reads the whole `cargo tree`: a row without the manifest entry, or the reverse, fails the build. `Page.duration` is the trigger ADR 0007 named, and it is additive in the wire form, so no migration is needed while `Document::SCHEMA_VERSION` stays at 1 (owner decision Q1). The critical code is the wire form of the new field and the V5 rows (design, "Critical code"), so those carry the hand mutants. The layer rule that matters: `gafira-doc` may depend on `gafira-time` after this edge and still on nothing else.
Owner decisions touched: the rigor the owner fixed for O4, O8 and O10, applied here as the same pattern; Q1 of the timeline design (schema 1, additive) if accepted; ADR 0007 (the edge and its trigger); no D-nn is contradicted.

# Apply progress: gafira-timeline

## TL1 (tdd-apply)

Commits:

- `aee6122` `test(doc): fail on the page duration the timeline edge brings` (RED)
- `d023416` `feat(doc): give a page an optional duration from gafira-time` (GREEN: the edge, the field, the V5 rows, the goldens)
- `8a5b71c` `test(doc): bound the ARQ-136 guard to the data of the timeline` (the guard, resolved below)
- `93172c8` `docs(gafira-timeline): record the TL1 progress, its mutants and the open guard`
- `b0f6c44` `docs(gafira-timeline): resolve the ARQ-136 guard in the TL1 notes`
- the head commit of the branch, `docs(gafira-timeline): align the TL1 notes with the head of the branch` (this section)

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` (the edge touches many manifests, so the workspace scope applies), `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps (forbid, CLI/UI, V5, V12, V13, V14, ARQ-107, and the testkit `ports` step), each by extracting its `run:` from `ci.yml`.

Status of the commits:

- `aee6122` is red by design (three failing assertions, listed below); a RED commit cannot pass alone.
- `d023416`: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo deny check` 0 (`advisories ok, bans ok, licenses ok, sources ok`), and the nine policy steps 0 (V5, V12 and ARQ-107 among them). `cargo test --workspace --locked` failed at one test only: `arq_136_the_measure_context_carries_no_time`, the guard this delivery had to narrow (commit `8a5b71c`).
- `8a5b71c`: the same gates at the HEAD of the branch, all 0, and `cargo test --workspace --locked --no-fail-fast` exit 0 with 194 test binaries passing and none failing.

### RED evidence

| AC | Test | The assertion that fails before the implementation |
|---|---|---|
| AC1 (ARQ-01, V-5) | `arq_01_the_crate_declares_the_gafira_time_edge` | `the crate must declare \`gafira-time.workspace = true\`` against a manifest that does not. |
| AC2 (V-2, ARQ-136) | `arq_136_the_duration_is_a_flicks_of_gafira_time` | None of its own: naming `page.duration` before the field exists is a compile error, which the testing policy forbids as RED. Its RED is AC1's manifest assertion; its evidence is the typed binding `let none: Option<Flicks> = page.duration;`, which only compiles with the type of `gafira-time`. |
| AC3 (ARQ-10, ARQ-13) | `arq_10_a_page_duration_round_trips_through_the_wire_form` | `assert_eq!(resaved, DOC_WITH_DURATION)`: the re-save drops `"duration":3528000000`, so the field is not in the wire form. |
| AC3 (ARQ-10, ARQ-13) | `arq_13_a_page_without_a_duration_writes_no_field` | Passes before it too (the field does not exist), and it is the guard of the omission after it: the old bytes must re-save byte for byte. |
| AC3 (ARQ-10, ADR 0006 rule 1) | `adr_0006_old_bytes_resave_in_the_canonical_form` (`tests/layout.rs`) | The `pre_layout/full.json` bytes gain no `duration`; the test compares them to the golden with the duration removed. |
| AC4 (V-5) | the V5 step of `ci.yml`, run literally | Eight `::error::<crate> must not depend on gafira-time` lines (`gafira-doc`, `gafira-edit`, `gafira-testkit`, `gafira-pathops`, `gafira-layout`, `gafira-text`, `gafira-raster`, `gafira-segment`), exit 1. |
| AC5 (ARQ-10, ADR 0006 rule 3) | `json::tests::full_golden_equals_the_built_document` | `assert_eq!(d.to_json()?, golden)` with the new `full.json` and a page that carries no duration yet. |
| AC6 (ARQ-02, ARQ-107) | the `forbid unsafe`, V12 and ARQ-107 steps | No RED: they pass before and after, which is their point. All three exit 0. |
| AC7 (ARQ-10) | `arq_10_the_duration_takes_part_in_the_equality_of_the_document` | `assert_ne!(without, with)`: the two documents compare equal because the duration is not a field of `Page`. The canonical order is pinned by AC3's round trip (the field sits after the grid and before `locked`). |

### Negative controls

Hand mutants, applied on the code of `d023416`, each one confirmed to fail, then reverted (`git status` clean after each):

| # | Wrong behaviour | Killing test and observed failure |
|---|---|---|
| H1 | the wire form writes `null` instead of omitting the field (drop `skip_serializing_if`) | `arq_13_a_page_without_a_duration_writes_no_field`, `tests/duration.rs:70`: the re-save writes `"duration":null`. The byte-pinned golden fails too (`json.rs`: `full_golden_equals_the_built_document`). |
| H2 | an absent field reads as zero (`#[serde(default = "zero_duration")]` with `Some(Flicks::ZERO)`) | the same test, `tests/duration.rs:70`: `a document without a duration must re-save without the field`, the re-save writes `"duration":0`. |
| H3 | the duration is an integer of the page's own (`Option<i64>`) | `arq_136_the_duration_is_a_flicks_of_gafira_time` does not compile: `tests/duration.rs:119`, `expected Option<Flicks>, found Option<i64>`. A type rule has no runtime observable here: `Flicks` is `#[serde(transparent)]` over `i64`, so both write the same bare number; the typed binding is the guard. |
| H4 | drop `gafira-time` from a V5 row whose tree reaches it (`gafira-doc`) | the V5 step: `::error::gafira-doc must not depend on gafira-time`, exit 1. |
| H5 | add a time field to the measurement context (`instant: Option<Flicks>`) | `arq_136_the_measure_context_carries_no_time`, `tests/visit_contract.rs:212`: `the context declares only the borrowed page and the target's DPI`, before the crate-wide scan is reached. |

### Choices and deviations

1. **The V5 rows are eight, not the design's nine.** Nine crates' trees reach `gafira-time` after the edge
   (`gafira-doc`, `gafira-edit`, `gafira-testkit`, `gafira-pathops`, `gafira-layout`, `gafira-text`,
   `gafira-raster`, `gafira-segment`, `gafira-render-cpu`), but `gafira-render-cpu`'s row already listed it
   (it reaches the crate through `gafira-render`), so only eight rows change. Verified crate by crate with
   `cargo tree -p <crate> -e normal,build --target all`, in both directions: no row lists a crate its tree
   does not reach. `gafira-audio`, `gafira-ports`, `gafira-render` and `gafira-media` already listed it.
2. **Only one golden moves.** The design expected all seven, but AC3 asks for the field to be omitted when the
   page has none, so `minimal.json`, `extension.json`, `extension_noncanonical.json` and the three
   `pre_layout/` fixtures keep their bytes; `tests/golden/full.json` gains exactly `,"duration":3528000000`
   after the grid of page 2 (artifact-integrity review: `git diff` of `tests/golden/` shows that token and
   nothing else). `tests/layout.rs` compares the old `pre_layout/full.json` bytes to the golden with the
   duration removed, the way the same test already treats page 1's layout keys.
3. **`gafira-time` left the ban list of `arq_137_the_domain_depends_on_no_crate_above_it`.** ARQ-137 forbids
   edges to a crate **above** the domain; `gafira-time` is layer 0, below it, and ARQ-136 names the edge as
   expected and bounded. The test failed on the new manifest for that reason, not because of the guard.
   `gafira-render`, `gafira-media`, `gafira-layout` and `gafira-ports` stay banned.
4. **The crate doc of `lib.rs`** now names `gafira-time` among the dependencies of the domain, as the design's
   critical-code list asks; `#![forbid(unsafe_code)]` is untouched (AC6).
5. **`Cargo.lock`** gains one line for the new dependency of `gafira-doc`; `cargo deny` adds no licence.

### The ARQ-136 guard: the row decides, not the brief

The row ARQ-136 of `specs/02-arquitectura-software.md` (rewritten by `4f79cb1`) and the brief of the
delivery read differently, so the guard needed a decision. **The row decides**, and it is the stricter
of the two, so nothing of the brief's intent is lost.

- The brief said: keep `Instant`, `Duration` and `SystemTime` in the list and drop `Flicks` and
  `FrameRate` from it.
- The row says: the crate names `Flicks` and `FrameRate` **only where the data of the timeline of §3.1
  lives** (`Page.duration`, `Sequence`, `Track`, `Clip` and `TimeRange`), and the test
  `arq_136_the_measure_context_carries_no_time` "fails if one of `Flicks`, `FrameRate`, `Instant`,
  `Duration` or `SystemTime` appears **outside those types**".

The row keeps the five names and bounds the first two by scope; the brief's reading would have let
the crate name `Flicks` anywhere, which is weaker than what the acceptance column asks for. The
guard is therefore the row's: the five names stay in the list, the scan skips the sources that hold
the data of the timeline (`TIMELINE_SOURCES`, today `page.rs` only, and the timeline files when
`TL2` brings them), and every other source of the crate may not name any of the five.

Two consequences, both inside the crate:

- The assertion of `MeasureCx` is untouched: the context declares exactly `page` and `dpi`, and it
  names no time type of the five (H5 kills that mutant there).
- The fixture of the byte-pinned golden builds the duration of its second page **without naming the
  type** (`serde_json::from_value` on the number, with the type inferred from the field), because
  `src/json.rs` is not a source of the timeline's data. That is the reading the row allows, not a new
  boundary: the boundary it states is the data of the timeline, and the fixture lives outside it.

The delivery is therefore green at the HEAD of the branch, with a guard that is strictly stronger
than the brief's and exactly the row's. If the coordinator prefers the brief's letter after all, the
change is one line in `tests/visit_contract.rs`: `TIMELINE_SOURCES` becomes empty and the five names
are dropped from the crate-wide scan; the `MeasureCx` assertion stays as it is either way.

### Measured lines

`git diff --shortstat develop` for `aee6122..d023416`: 10 files changed, 166 insertions(+), 14 deletions(-).
At the head of the branch: 11 files changed, 278 insertions(+), 17 deletions(-), of which this
file is 95 of the insertions and the delivery's code and tests 183. The guard commit adds the scope of the ARQ-136 scan and the
fixture of the golden; the notes commits change this file only.

## TL3
Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the timeline reaches the disk when the model it persists exists; the document a user opens is the one the laws must hold on, and the golden of the timeline is the evidence that the wire form writes every field
Product IDs: ARQ-10, ARQ-13, ARQ-30, ARQ-53, V-5, D-36, FV-02 (source: specs/02-arquitectura-software.md §3.1, §3.5, §7.1; specs/05-funcionalidad-video-y-distribucion.md FV-02; ADR 0006). Three of them name no test and that is deliberate: `V-5`'s canonical form is proven by AC2 and AC7's byte-set tests, which the V1 round verified; `D-36` is the schema-version decision, proven by AC2's head check and by the fact that pre-`TL3` bytes still load; `FV-02`'s privacy clause is about what a document *contains*, and this delivery's job is the table that will hold it — its test arrives with the asset import of `TL3`'s successor, not here.
Scope: `TL3` of `specs/changes/gafira-timeline/design.md`: in `gafira-doc`, the persistence of the model `TL2` defined — `Document.sequences` and the asset table keyed by `AssetRef` —, the wire form of both, the validation of the new ids, of the clip sources and of the laws of `TL2` on a loaded document, and the goldens: a new timeline golden plus the seven old byte sets, which must still re-save byte for byte.      Non-goals: the commands `MoveClip`, `TrimClip` and `SplitClip` and the media resolver (later changes, by owner decision Q4), the import metadata of the assets (Q5), any change to `gafira-time` or to `gafira-ports`, and the `Project` root of 02 §3.1, which is layer 4 (`gafira-format`).
Acceptance:
  AC1 Given a sequence, a track, a clip and a marker, When the document is written, Then their ids are plain numbers in the wire form, the enums carry their tags, every field is written and the defaults are read (ADR 0006 rule 3, ARQ-10) [unit]
  AC2 Given the root of the document, When it is written and read, Then `sequences` is always written, a document that predates the field reads as empty, and the error path of `Document::from_json` is unchanged (ADR 0006 rule 1, ARQ-13) [unit]
  AC3 Given the ids of the sequences, the tracks, the clips and the markers, When the document is validated, Then they are unique in the whole document and below `next_id`, the rule P2 of the existing validation extended to the three new id spaces (ARQ-10) [unit]
  AC4 Given a clip, When the document is validated, Then a `Scene` clip names an existing page, and a `Media` or `Audio` clip names an entry of the asset table (P9 of the design, FV-02) [unit]
  AC5 Given the assets of a document, When they are written and read, Then the table is keyed by `AssetRef`, the 64-character lowercase hex hash of the content, and each entry carries its name, its content hash and its storage (P9 of the design) [unit]
  AC6 Given a document with a timeline, When it is loaded, Then the laws of `TL2` hold on it — the cuts on the grid, the clips inside their source and their sequence, no lane holding two clips at an instant, the cap of the audio tracks and the relation of the speed — and a document that breaks one fails to load (ARQ-13, V-2, FV-01) [unit]
  AC7 Given the goldens, When the tests run, Then the new timeline golden is the canonical bytes of a document with a clip of each source kind, and the seven old byte sets still re-save byte for byte (ADR 0006 rules 1 and 3, ARQ-53) [unit]
  AC8 Given the crate, When the CI policy steps run, Then its V5 row passes, V12 keeps its `clippy.toml` identical and the crate still forbids `unsafe` (ARQ-02, V-5) [command]
Plan: the delivery persists what `TL2` defined and validates what it wrote: the wire form of the sequences and of the assets, the root that gains `sequences`, the validation extended to the three new id spaces and to the clip sources, and the laws of `TL2` applied to a loaded document, so a document that breaks one fails to load instead of reaching the editor. The critical code is the design's row for `TL3`: the wire form of the sequences and of the assets, the wire of the root, the validation of the ids and of the clip sources, and the goldens. Those carry the hand mutants: one per rule, each confirmed to fail and reverted. The `Scene` rule has a consequence worth naming: `RemovePage` on a page a clip shows fails with `InvalidDocument`, which is the safe half-state; FV-05 (c) is the commands change deleting both before it validates. The `AssetRef` is the 64-character lowercase hex hash of the content (the design's P9), and the minimum table carries the id, the name, the content hash and the storage; the metadata of the import arrives with the import.
Owner decisions touched: Q1 of the design (schema 1, additive: no migration while `Document::SCHEMA_VERSION` stays at 1, because no build has written a `.gafira` yet), Q5 (the minimum asset table, the metadata with the import), and the rigor of O4/O8/O10 (hand mutants, gates per commit, one V1 round for `TL2` and `TL3` after this lands).

## TL2
Flow: full | level: L2 | pack: product-spec slice | mutation: off | tier: T0 | verify: V1 | decided-by: owner | reason: the timeline exists when the model and its laws do; every later delivery and the commands rest on them
Product IDs: ARQ-01, ARQ-10, ARQ-13, ARQ-30, ARQ-136, V-1, V-2, FV-01, FV-03, FV-04 (source: specs/02-arquitectura-software.md §3.1, §3.2, §4.14; specs/05-funcionalidad-video-y-distribucion.md FV-01, FV-03, V-1, V-2)
Scope: `TL2` of `specs/changes/gafira-timeline/design.md`: in `gafira-doc`, the types `Sequence`, `Track`, `TrackKind`, `Clip`, `ClipSource`, `Marker` and the ids `SequenceId`, `TrackId`, `ClipId`, `AssetRef`; the laws that make them a timeline; the query a resolver needs; and the `DocMut` accessors under the `EditToken` of ARQ-30.      Non-goals: the persisted form of the model (`TL3`), the commands `MoveClip`, `TrimClip` and `SplitClip` and the media resolver (later changes, by owner decision Q4), the import metadata of the assets (Q5), and any change to `gafira-time` or to `gafira-ports`.
Acceptance:
  AC1 Given a project, When a sequence is built, Then it carries its rate, its size and its sample rate, its tracks are `Video`, `Overlay`, `Audio` or `Caption` and no further kind exists, and a clip names its source, its offsets in the sequence and in the source, its speed and its volume (FV-01, ARQ-13) [unit]
  AC2 Given the ids, When a sequence, a track, a clip or a marker is created, Then its id comes from the document's counter, unique across the whole document and not per page (ARQ-10, V-1) [unit]
  AC3 Given a cut, When it is placed, Then it lands on an exact frame of the sequence at its rate, including 23,976, where the frame duration in flicks is exact and not rounded (V-2, FV-03) [unit]
  AC4 Given a clip, When it is placed, Then its range is half-open, lies inside the range of its source and inside the sequence, and a range outside either is rejected with the error the crate fixes (FV-01) [unit]
  AC5 Given two clips in one track, When they would overlap at any instant, Then the model rejects it, and the stacking of the lanes is the order of `tracks`, not the order of insertion (P11 of the design) [unit]
  AC6 Given a sequence whose rate changes, When the clips are retimed, Then their positions and durations land on the new grid by the function FV-01 (c) fixes, and the retiming is deterministic (FV-01 (c), ARQ-10) [unit]
  AC7 Given a track count, When the sixteenth audio track is added, Then it is accepted and the seventeenth is rejected (FV-01) [unit]
  AC8 Given a speed, When a clip's duration is computed, Then it follows the relation between speed, source duration and sequence duration, and a speed of zero or a negative one is rejected (FV-03) [unit]
  AC9 Given an instant, When the resolver query is asked, Then it answers which clip each track shows at that instant, and it answers `None` where a track shows nothing (the contract P11 and Q4 name, without implementing the resolver) [unit]
  AC10 Given a held `EditToken`, When a timeline mutation is attempted without it, Then it is refused, and every mutation that holds it is undoable and redoable like the rest of the document (ARQ-30) [unit]
Plan: the delivery fills `crates/gafira-doc/src/` with the model and its laws, on top of the edge and `Page.duration` that `TL1` brought. The laws are the point: a model without them is data that breaks the first time someone persists it, and `TL3` is what persists it. The design's P11 (one clip per track at an instant) and the frame grid are the two with the most edge cases, and they carry the hand mutants. The resolver query is a query over the model, not the resolver that `gafira-media` will implement. The critical code is everything in the design's "Critical code" row for `TL2`: the grid arithmetic, the bounds check, the overlap rule, the audio cap, the retiming and the speed relation.
Owner decisions touched: Q3 of the design (bare values for the animated volume and transform, recorded as a deviation here), Q4 (the commands and the resolver go to later changes), Q5 (the minimum asset table arrives in `TL3`), and the rigor of O4/O8/O10.

## TL2 (tdd-apply)

Commits:

| Commit | What |
|---|---|
| `da35130` feat(gafira-doc): the timeline types, ids and the edit view | The types `Sequence`, `Track`, `TrackKind`, `Clip`, `ClipSource`, `Marker`, `Speed`, `AssetRef`, `AssetEntry`, `AssetStorage`, `PixelSize`; the ids `SequenceId`, `TrackId`, `ClipId`, `MarkerId` from the document's counter; the `DocMut` accessors of ARQ-30; the model's query surface. No laws yet: the surface the laws come on top of. |
| `b00b845` feat(gafira-doc): the laws of the timeline, each with its test | The seven laws with the battery of `crates/gafira-doc/tests/timeline_laws.rs` (10 tests, one per AC). |
| `f103281` fix(gafira-doc): snap both cuts of a retimed clip | Round 2: each cut of a retimed clip snaps on its own instead of the start and the duration snapping independently (see "Round 2 closure"). |

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` (`gafira-doc` is not a leaf), `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps by extracting their `run:` from `ci.yml`.

### RED evidence

The battery was written against the types of `da35130`, before the laws existed. RED run
(`cargo test -p gafira-doc --locked --test timeline_laws`), one line per AC:

```
test fv_01_a_sequence_and_a_clip_carry_their_fields ... ok
test arq_10_and_v_1_the_timeline_ids_come_from_one_counter ... ok
test arq_30_the_timeline_mutators_ask_for_the_token ... ok
test v_2_and_fv_03_a_cut_lands_on_an_exact_frame_including_23_976 ... FAILED
test fv_01_a_clip_lies_inside_its_source_and_its_sequence ... FAILED
test p_11_no_lane_holds_two_clips_at_any_instant ... FAILED
test fv_01_the_retime_snaps_cuts_to_the_new_grid ... FAILED
test fv_01_the_audio_cap_sixteenth_enters_seventeenth_is_refused ... FAILED
test fv_03_the_duration_follows_the_speed_relation ... FAILED
test p_7_the_lane_answers_what_it_shows_at_an_instant ... FAILED
test result: FAILED. 3 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
```

GREEN run after `b00b845`: `test result: ok. 10 passed; 0 failed`.

| AC | Requirement | Test | RED | GREEN |
|---|---|---|---|---|
| AC1 | FV-01, ARQ-13: the sequence carries its rate, size and sample rate, the four track kinds, the clip carries its source, its offsets, its speed and its volume | `fv_01_a_sequence_and_a_clip_carry_their_fields` | green (the types are the surface) | ok |
| AC2 | ARQ-10, V-1: the ids come from the document's counter, unique across the document | `arq_10_and_v_1_the_timeline_ids_come_from_one_counter` | green (the ids are the surface) | ok |
| AC3 | V-2, FV-03: a cut lands on an exact frame; 23,976 gives exactly 29 429 400 flicks | `v_2_and_fv_03_a_cut_lands_on_an_exact_frame_including_23_976` | FAILED | ok |
| AC4 | FV-01: the clip lies inside its source and the sequence, half-open, at least one frame | `fv_01_a_clip_lies_inside_its_source_and_its_sequence` | FAILED | ok |
| AC5 | P11: no overlap on a lane; the stacking is the order of `tracks` | `p_11_no_lane_holds_two_clips_at_any_instant` | FAILED | ok |
| AC6 | FV-01 (c): the retime snaps to the nearest frame of the new grid, loses no clip, is deterministic | `fv_01_the_retime_snaps_cuts_to_the_new_grid` | FAILED | ok |
| AC7 | FV-01: the sixteenth audio track enters, the seventeenth is refused | `fv_01_the_audio_cap_sixteenth_enters_seventeenth_is_refused` | FAILED | ok |
| AC8 | FV-03: the relation between speed, source and sequence duration; zero or negative refused | `fv_03_the_duration_follows_the_speed_relation` | FAILED | ok |
| AC9 | P7: the model answers which clip each lane shows at an instant, `None` where nothing | `p_7_the_lane_answers_what_it_shows_at_an_instant` | FAILED | ok |
| AC10 | ARQ-30: the mutation needs the token; the counter never repeats | `arq_30_the_timeline_mutators_ask_for_the_token` | green (the edit view is the surface) | ok |

### Hand mutants

Nine behaviours applied to the green code, one test each, reverted after every one.
Nine dead, none weakened. `H1` to `H7` are the round-0 list; `H8` and `H9` close `W2` of
the round-1 verify.

| # | Rule | Wrong behaviour applied | Test that killed it |
|---|---|---|---|
| H1 | the grid: a cut lands on an exact frame | the cut between two frames is rounded instead of reported | `v_2_and_fv_03_a_cut_lands_on_an_exact_frame_including_23_976` |
| H2 | 23,976 gives an exact frame duration | the frame rounded to the millisecond (29 635 200 flicks) | `v_2_and_fv_03_a_cut_lands_on_an_exact_frame_including_23_976` |
| H3 | the bounds: inside the source and the sequence | a clip starting before its source is accepted | `fv_01_a_clip_lies_inside_its_source_and_its_sequence` |
| H4 | P11: no overlap on any lane | two clips may meet on the video lane | `p_11_no_lane_holds_two_clips_at_any_instant` |
| H5 | P11: the stacking is the order of `tracks` | the stacking follows the order of insertion (the lane ids) | `p_7_the_lane_answers_what_it_shows_at_an_instant` |
| H6 | the audio cap | the seventeenth audio track is accepted | `fv_01_the_audio_cap_sixteenth_enters_seventeenth_is_refused` |
| H7 | the speed relation | the relation divided the other way round | `fv_03_the_duration_follows_the_speed_relation` |
| H7 | the speed relation | a speed of zero is accepted | `fv_03_the_duration_follows_the_speed_relation` |
| H8 | the retiming law | `snap_nearest`'s frame index is used where a count of ticks belongs (the defect `C1` of the round-1 verify) | `fv_01_the_retime_snaps_cuts_to_the_new_grid` |
| H9 | the speed rule's grid step | the floor of one frame is dropped from `place_span_for` | `fv_03_the_place_of_a_clip_is_the_source_over_the_speed_on_the_grid` |

Nine mutants over the eight laws plus the speed rule's grid step; both of the first
two needed a stronger test, and the test was strengthened instead of the mutant being
weakened:

- **H2** first survived because the test read the frame from the rate
  (`FrameRate::flicks_per_frame`) and not from the sequence. The test now reads
  `sequence.frame()` and asserts the exact 29 429 400 flicks, which is the frame
  the model's own grid uses.
- **H5** first survived because the mutant ordered by the *clip* ids and the test's
  clip ids ran in the same order as the lanes. The test now puts two lanes over
  the same instant, with the lane ids and the clip ids both in the opposite
  order of the tracks list, and asserts the answer follows the list.

### Round 1 closure (verify findings)

The round-1 report (`/tmp/tl2-verify-round1.md`) failed the delivery on one CRITICAL and
three implementation warnings. This round fixes those four and leaves the rest where the
coordinator put them:

| Finding | What was wrong | What changed |
|---|---|---|
| `C1` (CRITICAL) | `Sequence::retimed` fed `snap_nearest`'s **frame index** to `Flicks::from_ticks`, so every clip longer than one frame collapsed to one frame: 2.5 s at 24->30 fps became 23 520 000 ticks instead of 1 764 000 000. | The index is multiplied by the frame; `.max(frame)` and `.max(last_end)` stay. Commit `cfbadc7`. |
| `W1` | AC6's test passed over `C1`: two one-frame clips, a modulo assertion, no absolute tick. | A 60-frame clip with the absolute ticks of both cuts (0 and 1 764 000 000) and a tie case (58 800 000 -> 70 560 000, the later frame). It failed on the bug before the fix. Commit `cfbadc7`. |
| `W2` | The hand-mutant list skipped the retiming law. | `H8` added and run: the mutant reproduces `C1` and the strengthened test kills it (`left: 23520000, right: 1764000000`). `H9` covers the new grid step of the speed rule (`left: 0, right: 29400000`). |
| `W3` | The design's speed rule ("rounded to the grid, remainder into `source_in`") had no code, and `clip.rs` linked a method that did not exist. | `Sequence::place_span_for` answers `PlaceSpan { place, remainder }`: the place snapped to the grid and floored at one frame, the signed remainder reported. The reading of "the source offset absorbs the remainder" is in the method's doc; the alternative (shifting `source_in` by the remainder) moves content and needs an owner decision. Commit `60b69a0`. |
| `S2` | `cargo doc` warned about five unresolved intra-doc links. | The links carry their path; `cargo doc -p gafira-doc --no-deps` is clean. Commit `60b69a0`. |

Left open by the coordinator's instruction, not by oversight:

- `W7` (the manual serde drops `sequences`/`assets`): TL3's, where the timeline reaches the wire.
- `W4`, `W5`, `W6`, `S3` (card and spec wording: AC4's two bounds, `ARQ-13`/`FV-04` in the card's
  ID list, AC10's undo clause, the `tier: T0` label): the specifier's, not TL2 code.
- `W8` (1 794 insertions against a 480/670 forecast and the ~700 ceiling): the coordinator's call
  once this round lands.
- `S1` (`AssetRef`'s hex serde and `Speed`'s reduction untested): untouched; neither is reachable
  from an AC in this cut.

### Round 2 closure (both cuts of a retimed clip)

The design (`design.md:481-482`) says the retime moves "every cut to the nearest frame". A clip
has two cuts, and rounding is not additive, so `snap(start) + snap(duration) != snap(start + duration)`:
the end cut drifted by up to one frame, in both directions. `Sequence::retimed` now snaps each
cut on its own and derives the span with `TimeRange::from_bounds(start, end)`. The one-frame
floor and the push-right survive because the start is still floored at `last_end` and the end
at `start + frame`.

| Commit | What |
|---|---|
| `f103281` fix(gafira-doc): snap both cuts of a retimed clip | Both cuts snap independently; the span is derived from them. Test `fv_01_the_retime_moves_both_cuts_of_a_clip`. |

RED on `4a24b8b`, before the fix
(`cargo test -p gafira-doc --locked --test timeline_laws fv_01_the_retime_moves_both_cuts_of_a_clip`):

- grow, 24 -> 30 fps, clip `[29 400 000..58 800 000)`: `the end cut snaps on its own: 2.5 frames
  of 30 fps goes to the later frame`, `left: 47040000, right: 70560000`.
- shrink, 30 -> 24 fps, clip `[47 040 000..94 080 000)`: `the end cut snaps to 3 frames of 24 fps,
  not the snapped span added to the start`, `left: 117600000, right: 88200000`. (The grow
  assertion was temporarily set to its buggy value to reach this one; restored before the commit.)

The test also guards the two rules the round had to keep: a span below one frame keeps one frame
(two one-frame 60 fps clips retimed to 24 fps) and the clip that would overlap is pushed right.

Hand mutant, applied on the green `f103281` and reverted (tree clean after):

| # | Rule | Wrong behaviour applied | Test that killed it |
|---|---|---|---|
| H10 | the retime moves every cut to the nearest frame | the end is `start + snap(duration)` instead of `snap(end)`, the defect's own shape | `fv_01_the_retime_moves_both_cuts_of_a_clip`, `left: 47040000, right: 70560000` |

Gates at `f103281`, each alone, exit 0: `cargo fmt --check`, `cargo clippy -p gafira-doc
--all-targets --locked -- -D warnings`, `cargo test -p gafira-doc --locked` (all binaries ok),
`cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`), the nine policy steps of
`ci.yml` (`POLICY OK: every step exited 0`), and `RUSTDOCFLAGS="-D warnings" cargo doc -p
gafira-doc --no-deps`. Measured: 2 files changed, 103 insertions(+), 8 deletions(-) (this file
not counted).

### Negative controls

- The guard of ARQ-136 still bites: with the new sources out of
  `TIMELINE_SOURCES`, `arq_136_the_measure_context_carries_no_time` fails with
  "the domain names the time type Flicks in src/timeline/error.rs, outside the
  data of the timeline". It is the guard the delivery had to satisfy, not a
  formality: the error type carries plain `i64` ticks so the law's values can
  be reported without the time type leaking out of the timeline's own files.
- The module coverage of the same test fails on a module the list does not cover:
  `src/lib.rs declares the module timeline, which the list of sources does not
  cover: add it`. That is why `timeline/{mod,asset,clip,error,sequence,track}.rs`
  are in `DOMAIN_SOURCES`.
- The wire did not move: the golden files and `tests/serde.rs` pass untouched
  (18 passed), and the timeline is not on the wire yet (see the deviations).

### Choices and deviations

- **The timeline is in memory, not on the wire (UD-3).** `Document` keeps
  `sequences` and `assets`, and its `Serialize` writes the four canonical fields
  of schema 1, so the golden bytes do not move; the persisted form arrives with
  the tables change (TL3). The derive of `Serialize`/`Deserialize` left the
  struct for a manual pair, because the model types are not serde-ready in this
  cut and the derive would have leaked them into bytes the schema version has
  not declared. `Deserialize` keeps ARQ-13's validate-while-loading rule
  (`try_from`, as the derive's `try_from` did).
- **`Speed` is a rational (`num`, `den`), not a float.** FV-03's relation is
  exact (`source × den / num`), a float would round the duration and the
  tolerance of ARQ-42 would measure the rounding instead of the raster. The
  ratio is reduced at construction, so the same speed has one form.
- **The volume is a bare `f32` (Q3 of the design).** The animation of the
  volume (`Animatable<f32>`) is out of this cut.
- **`TimelineError` is its own type, not a `ValidationError` variant.** The
  laws of the model are not the document's validation (that pass is ARQ-13's,
  over the document's structure), and the timeline is not on the wire yet, so a
  variant there would promise a persisted error that does not exist. Its values
  carry plain `i64` ticks so that no file outside the timeline's own names a
  time type (ARQ-136).
- **The frame grid of the sequence comes from `gafira-time`'s `FrameRate`,**
  whose own law rejects a rate whose frame is not a whole number of flicks.
  The sequence never rounds: a cut that is not on the grid is reported
  (`OffGrid`), and the 23,976 case is checked against 29 429 400 flicks, not
  against a rounded millisecond.
- **The retime is a pure function** (`Sequence::retimed(&self, new_rate) ->
  Sequence`), as P6 of the design says: it moves nothing, because only a
  command may write (ARQ-30), and the commands are the next change. A clip
  that would fall below one frame keeps one frame and pushes the rest of its
  lane right, so no clip is lost (FV-01 (c)); the tie of the nearest frame
  breaks towards the later frame.
- **`AssetTable` is keyed by `AssetRef`** (P9), with the two fields the
  document needs (`name`, `storage`); the import metadata (duration, rate,
  legal class) arrives with the import feature (Q5).
- **The `TIMELINE_SOURCES` list grew from one entry to four:**
  `page.rs` (TL1's `Page.duration`), `timeline/clip.rs`, `timeline/sequence.rs`
  and `timeline/track.rs` — the four files that hold the timeline's data and
  name the time types. `timeline/error.rs` and `timeline/asset.rs` are in
  `DOMAIN_SOURCES` (so the module coverage passes) but not in
  `TIMELINE_SOURCES`, because they name no time type: the error carries plain
  ticks and the asset table carries hashes. The list stays a list of the
  sources that hold the data, not a wildcard.

## TL3 (tdd-apply)

Commits (branch `feat/gafira-timeline-tl3`, cut from `develop` at `d89bb4e`):

| Commit | What |
|---|---|
| `9924741` feat(gafira-doc): the document writes its timeline and its asset table | The wire form of the model (`Sequence`, `Track`, `Clip`, `Marker` derive `Serialize`/`Deserialize`; `TrackKind`, `ClipSource` and `AssetStorage` carry their tag in `snake_case`), `DocumentWire` gaining `sequences` and `assets` with `#[serde(default)]`, `Document` writing six fields, the new timeline golden and the three canonical goldens moved. |
| `e0439b8` feat(gafira-doc): the timeline ids share the document's counter | P2 extended: the ids of the sequences, the tracks, the clips and the markers enter the uniqueness check and the `next_id` bound, with the battery of `tests/timeline_validation.rs`. |
| `485028a` feat(gafira-doc): a loaded timeline is checked against its laws and its sources | P7 (a `Scene` clip names a page, a `Media` or `Audio` clip names an asset) and P8 (`Sequence::check_laws`: the cut on the grid, the span, the clip inside its sequence and its source, the speed, one strip per lane, the audio cap), with `BeforeSequence` and the three `ValidationError` variants they report. |
| `68c8783` Merge branch 'develop' into feat/gafira-timeline-tl3 | `develop` moved while this delivery ran (TL2's verify round: `cfbadc7` the retime in flicks, `60b69a0` `place_span_for`). The merge keeps `check_laws` and `place_span_for` side by side in `timeline/sequence.rs` and changes nothing of TL3. |
| `c9661c0` docs(gafira-timeline): record the TL3 delivery, its mutants and its goldens | This section: the commits, the RED evidence per criterion, the seventeen hand mutants, the deviations and the measured lines. |
| `f801c9e` docs(gafira-doc): point the clip's law link at the sequence's path | One intra-doc link of `timeline/clip.rs` prefixed, so `cargo doc -p gafira-doc --no-deps` is warning-free (the round-1 report of TL2 raised the same class as S2). |

No commit is red by design: the RED of every criterion was observed in the working
tree before the implementation (recorded below) and each committed unit carries its
test with its code, so the three commits of the delivery pass every gate alone — and
the merge and the two notes commits pass them too, `git status` clean at the head.

Gates per commit, each run alone, exit 0: `cargo fmt --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings` (`gafira-doc` is not
a leaf, ADR 0017), `cargo test --workspace --locked` (`2474`, `2476`, `2480` and
`2481` tests passed, 0 failed), `cargo deny check` (`advisories ok, bans ok, licenses
ok, sources ok`) and the nine policy steps of `.github/workflows/ci.yml`, extracted
from the `policy` job's `run:` blocks into `/tmp/gafira-tl3-ci/steps2/` and run with
`bash -euo pipefail` (AC8): format, CLI without UI crates (ARQ-07), `forbid unsafe`,
V5, V12, ARQ-107, V13, V14 and the testkit `ports` feature. The full workspace suite
ran once per commit (the counts above) and once more at the head of the branch.

### RED evidence

Wire form, `cargo test -p gafira-doc --locked --test timeline_wire` before the
implementation: `0 passed; 6 failed`.

| AC | Requirement | Test | The assertion that fails before the implementation |
|---|---|---|---|
| AC1 | ARQ-10: the ids as plain numbers, the enums tagged, every field written | `arq_10_the_timeline_ids_are_plain_numbers_and_every_field_is_written` | `"sequences":[{"id":3, is not in {"schema_version":1,...,"pages":[...]}` — the root writes four fields and the timeline is not on the wire. |
| AC2 | ARQ-13: `sequences` always written, an older document reads it as empty, the error path unchanged | `arq_13_a_document_that_predates_sequences_reads_them_as_empty` | `the saved bytes write both fields: {...}` — the re-save of the old bytes carries no `"sequences":[],"assets":{}`. |
| AC5 | P9: the table keyed by the 64-character lowercase hex hash | `p9_the_asset_table_is_keyed_by_the_lowercase_hex_hash` | `the table is keyed by the hex hash: {...}` — no `"assets"` in the bytes. |
| AC7 | ARQ-53: the timeline golden is the canonical bytes | `arq_53_the_timeline_golden_is_the_canonical_bytes` | `left: "{...\"pages\":[...]}"` against the placeholder golden `{}`. |
| AC7 | ADR 0006 rule 1: the old byte sets load and re-save canonically | `adr_0006_the_old_bytes_read_the_timeline_as_empty` | `extension_noncanonical writes both fields: {...}`. |
| AC1 | ARQ-10, ARQ-13: the timeline survives the round trip | `arq_10_the_timeline_round_trips_through_the_document` | `assert_eq!(back, doc)`: the re-read document has `sequences: []` where the built one has the sequence. |

`arq_10_the_timeline_ids_are_plain_numbers_and_every_field_is_written` is also AC1's
only honest RED: a test that named `serde_json::to_string(&clip)` before the derive
would have been a compile error, which the testing policy forbids as RED (the same
situation as TL1's AC2).

Validation, `cargo test -p gafira-doc --locked --test timeline_validation` before the
implementation: `0 passed; 5 failed`, every one at
`crates/gafira-doc/tests/timeline_validation.rs:67`:

```
a sequence id used twice: the document loaded and it must not
a cut between two frames: the document loaded and it must not
a scene clip with no page: the document loaded and it must not
the second sequence is checked: the document loaded and it must not
the second sequence names a page that is gone: the document loaded and it must not
```

| AC | Requirement | Tests | RED |
|---|---|---|---|
| AC3 | ARQ-10: the timeline ids unique in the document and below `next_id` | `arq_10_the_timeline_ids_are_unique_in_the_whole_document`, `arq_10_a_timeline_id_at_the_counter_is_refused` | `a sequence id used twice: the document loaded and it must not` |
| AC4 | P9, FV-02: a `Scene` names a page, a `Media`/`Audio` names an asset | `p9_a_clip_names_a_page_and_an_asset_the_document_holds`, `arq_13_every_clip_of_the_document_names_its_source` | `a scene clip with no page: the document loaded and it must not` |
| AC6 | ARQ-13: the laws of `TL2` on a loaded document | `arq_13_a_document_that_breaks_a_law_does_not_load`, `arq_13_every_sequence_of_the_document_is_checked` | `a cut between two frames: the document loaded and it must not` |

AC8 has no RED of its own: the nine policy steps pass before and after this delivery,
which is their point. They ran literally, exit 0, at each commit.

### Hand mutants

Seventeen behaviours of the delivery's critical code, applied one at a time to the
green tree at the head of the branch (re-run after the merge of `develop`), each one
confirmed to fail a test and reverted; nothing was committed. Seventeen killed, none
survived, and every kill is a failing assertion, not a compile error.

| # | Rule | Wrong behaviour applied | Test that killed it, and the observed failure |
|---|---|---|---|
| M1 | The root writes `sequences` always (ADR 0006 rule 3) | `Document::serialize` drops the `sequences` field | `timeline_wire::arq_10_the_timeline_ids_are_plain_numbers_and_every_field_is_written`: `"sequences":[{"id":3, is not in {...}` |
| M2 | A document that predates the field reads it as empty (rule 1) | `#[serde(default)]` removed from `DocumentWire.sequences` | `timeline_wire::arq_13_a_document_that_predates_sequences_reads_them_as_empty`: `old bytes still load: Json(Error("missing field \`sequences\`"))` |
| M3 | The asset table is keyed by the lowercase hex hash (P9) | `AssetRef::hex` writes uppercase | `timeline_wire::p9_the_asset_table_is_keyed_by_the_lowercase_hex_hash`: `the table is keyed by the hex hash: {...}` |
| M4 | A reference is exactly 64 hex characters | the length check of `AssetRef::from_hex` removed | the same test: the short key loads instead of being a JSON error |
| M5 | Every field is written, defaults included (rule 3) | `#[serde(skip_serializing_if = "Not::not")]` on `Track::ripple` | `arq_10_the_timeline_ids_are_plain_numbers...`: `"kind":"overlay","ripple":false is not in {...}` |
| M6 | The enums carry their tag (ARQ-10) | `rename_all = "snake_case"` removed from `TrackKind` | the same test: `{"id":4,"kind":"video" is not in {...}` |
| M7 | The timeline ids are unique in the whole document (P2) | the `timeline` iterator dropped from `check_ids`' `ids()` | `timeline_validation::arq_10_the_timeline_ids_are_unique_in_the_whole_document`: `a sequence id used twice: the document loaded and it must not` |
| M8 | The timeline ids are below `next_id` (P2) | the counter bound computed over the pages only | `arq_10_a_timeline_id_at_the_counter_is_refused`: `a clip id at the counter: the document loaded and it must not` |
| M9 | A `Scene` clip names an existing page (P9) | the `Scene` arm of `check_clip_sources` removed | `p9_a_clip_names_a_page_and_an_asset_the_document_holds`: `a scene clip with no page: the document loaded and it must not` |
| M10 | A `Media`/`Audio` clip names an asset entry (P9) | the `Media`/`Audio` arm removed | the same test: `a media clip with no asset: the document loaded and it must not` |
| M11 | Every sequence of the document is checked (ARQ-13) | `check_timeline` reads only `sequences[0]` | `arq_13_every_sequence_of_the_document_is_checked`: `the second sequence is checked: the document loaded and it must not` |
| M12 | Every clip of every sequence names its source (AC4) | `check_clip_sources` reads only `sequences[0]` | `arq_13_every_clip_of_the_document_names_its_source`: `the second sequence names a page that is gone: ...` |
| M13 | One strip per lane at any instant (P11) | the `windows(2)` overlap loop of `check_laws` removed | `arq_13_a_document_that_breaks_a_law_does_not_load`: `two clips at one instant: the document loaded and it must not` |
| M14 | The audio cap: sixteen tracks (FV-01) | the cap check of `check_laws` removed | the same test: `the seventeenth audio track: the document loaded and it must not` |
| M15 | The speed divides (FV-03) | the `InvalidSpeed` check of `check_laws` removed | the same test: `a zero speed: the document loaded and it must not` |
| M16 | A clip starts inside its sequence (FV-01) | the `BeforeSequence` rule of `Track::check_clip` removed | the same test: `a clip before the sequence starts: the document loaded and it must not` |
| M17 | A document is validated while it loads (ARQ-13) | `doc.validate()` dropped from `TryFrom<DocumentWire>` | the same test: `a cut between two frames: the document loaded and it must not` |

### Choices and deviations

- **The wire order is the one 02 §7.1 lists:** `schema_version`, `next_id`, `dpi`,
  `pages`, `sequences`, `assets`, so the canonical bytes end
  `..."nodes":[]}],"sequences":[],"assets":{}}`. The asset table is a JSON object
  keyed by the hex hash, so two entries cannot hold the same content and the order is
  the `BTreeMap`'s (ARQ-01).
- **The three new enums are spelled in `snake_case`** (`"video"`, `{"scene":1}`,
  `{"media":"abab…"}`, `"embedded"`), the lowercase vocabulary the rest of the
  document's wire form already uses (`"normal"`, `"lines"`, `"group"`, `"srgb"`).
  A unit variant is a bare string and a variant with a value is a one-key object, so
  every enum carries its tag.
- **The defaults are the root's, not the model's.** ADR 0006 rule 1 is about a field
  the document gained after a document could exist: `sequences` and `assets` read with
  `#[serde(default)]` and every field of the model itself is required, because no
  bytes of a timeline exist yet that a default would have to serve.
- **`Track::check_clip` gained the `BeforeSequence` rule.** `TL2`'s AC4 says "its
  range ... lies inside the range of its source and inside the sequence, and a range
  outside either is rejected", but `check_clip` only checked the grid, the span and
  `source_in >= 0`. The validation of a loaded document needs the missing half, so it
  lives in the model's own predicate, and `Track::insert` now refuses a cut at a
  negative tick too. That is the law AC4 already states, not a new rule.
- **`TimelineError` gained `BeforeSequence`, and `ValidationError` gained three
  variants.** The two referential rules (`DanglingScene`, `DanglingAsset`) are the
  document's, not the model's: only the document can see the pages and the asset
  table, which is why they are not `TimelineError` variants. `TimelineLaw {
  sequence, error }` wraps a break of the model's own laws, so the error the load
  returns says which sequence and which law. `ValidationError` stays
  `#[non_exhaustive]`.
- **`ClipSource::Nodes` is not referentially checked.** AC4 names the `Scene` clip
  and the `Media`/`Audio` clip; the node list of a `Nodes` clip belongs with the node
  commands, which do not exist yet.
- **A non-reduced speed off the wire is accepted.** `Speed` reduces at construction,
  so the writer always spells 2/1 and never 4/2; a hand-written 4/2 behaves as the
  same ratio. Only a zero numerator or denominator is refused (FV-03), which is the
  one value that divides nothing and would make the relation panic.
- **The seven byte sets: what moves and what stays.** Rule 3 moves the three
  canonical goldens (`minimal.json`, `full.json`, `extension.json`), which gain
  exactly `,"sequences":[],"assets":{}` at the root, and `tests/duration.rs` (TL1's
  two pinned byte strings) gains the same two fields, for the same reason. The four
  old byte sets (`extension_noncanonical.json` and `pre_layout/{minimal,full,
  extension}.json`) stay as input-only fixtures and are the evidence of rule 1: they
  load with an empty timeline and re-save in the new canonical form.
  "The seven old byte sets re-save byte for byte" cannot be literally true while rule
  3 holds — writing a field the old bytes do not carry changes them — so the
  achievable form is the one the tests pin: the three canonical goldens round-trip
  byte for byte (`tests/golden.rs`), the four old byte sets land on them
  (`adr_0006_the_old_bytes_read_the_timeline_as_empty`, and `tests/layout.rs` for the
  page-level differences of `pre_layout/full.json`), and the new timeline golden
  round trips too.
- **`RemovePage` over a shown page fails, and that is the safe half-state.** With
  P9's rules in place, removing a page a `Scene` clip shows leaves the document with
  `ValidationError::DanglingScene` on the next load: `RemovePage` alone cannot produce
  a document that validates. FV-05 (c) — "deleting the scene deletes its clip, and it
  is reversible" — is the commands change deleting both before it validates. Noted
  here because it is a consequence of this delivery that a later change must own.
- **`develop` moved under the delivery.** TL2's verify round landed `cfbadc7`
  (retime in flicks) and `60b69a0` (`Sequence::place_span_for`) plus its notes while
  this delivery ran. The merge `68c8783` brings them in; the three conflicts
  (`timeline/sequence.rs`, `timeline/track.rs` doc comment) were resolved by keeping
  both sides, and the mutants were re-run at the merged head.

### Measured lines

`git diff --shortstat develop` without this section: 17 files changed, 955
insertions(+), 47 deletions(-), which is 1 002 changed lines. Of those, 702 are the two
new test files (`tests/timeline_wire.rs` 365, `tests/timeline_validation.rs` 337) and
253 are code, the three goldens and the two byte strings of `tests/duration.rs`; the
new golden is one line of 2 221 bytes and each moved golden changes by the two fields.
The design forecast 330 (460) and the delivery ceiling is ~700 lines, so the measured
size is **above the ceiling** and rests on the tests: 702 of the 1 002 lines are the
battery, and the code the delivery writes is 253. The coordinator decides whether that
is the delivery or whether the battery is a review slice of its own.

## TL1 fix (tdd-apply)

Closes the two implementation warnings of the retroactive `TL1` round on
`fix/gafira-timeline-tl1-verdict`, cut from `develop` at `316d247` (the commit
that amended ADR 0006 rule 3). The record lives here, in the change folder,
because `specs/changes/gafira-timeline/` has not moved to `history/` yet; had it
moved, the notes would go to the `specs/07-deuda-abierta.md` entry instead.

### Commits

- `04ebea9` `test(doc): refuse a clock inside the timeline sources` — `W1`, the
  ARQ-136 guard (`crates/gafira-doc/tests/visit_contract.rs`, +215/−11).
- `418b300` `feat(doc): refuse a page duration before zero on load` — `W2`, the
  range rule (`src/page.rs`, `src/layout.rs`, `tests/duration.rs`, +58/−9).
- `73690c1` `test(doc): pin the present side of the excepted page duration` —
  the amended ADR's other side (`src/page.rs`, +40).
- the record commit of this section (this file).

### `W1` — the guard refuses a clock inside a timeline source

The old rule skipped each of the four `TIMELINE_SOURCES` whole, so
`std::time::Instant`, `Duration` and `SystemTime` were legal in `page.rs`,
`timeline/clip.rs`, `timeline/sequence.rs` and `timeline/track.rs`. A timeline
source may now name `Flicks` and `FrameRate` and nothing else: the three clock
types are refused there too. Every other source still refuses all five. The five
names are checked to be exactly two data types plus three clock types, so a name
added to one list and not the other is a hole.

How the guard tells a type from a word: `code_only` strips line comments, nested
block comments, string literals and raw strings before the whole-word search
runs. A type is an identifier in code; the word in a doc comment, in an
assertion message or inside `r#"…"#` is prose. The search is case-sensitive and
bound at identifier edges, so the field `duration`, the identifier `durations`
and the English word `Duration` in prose are not the type, while
`std::time::Duration`, `Duration::from_secs` and `fn wait(clock: Instant)` are.
The last piece is a naming constraint, not a guard exception:
`PageField::DurationTicks` (below) is the only page-field variant that could
have collided with the banned name, and it is named for its unit so the crate
holds no free-standing `Duration` identifier outside the timeline's data files.

RED, before the fix (`cargo test -p gafira-doc --test visit_contract`):

```text
---- arq_136_a_clock_type_inside_a_timeline_source_is_refused ----
src/page.rs must refuse the clock type Instant
  left: []  right: ["Instant"]
---- arq_136_the_guard_tells_a_type_from_a_word ----
assertion `left == right` failed
  left: []  right: ["Duration"]
```

The literal probe from the finding: `#[cfg(any())] fn probe_time_type(_now:
std::time::Instant) {}` in `src/page.rs` then failed
`arq_136_the_measure_context_carries_no_time` with `src/page.rs names the time
type(s) ["Instant"]`; the probe was removed and is not committed.

### `W2` — a negative page duration fails to load

The design of `TL1` says a page duration is a `Flicks` value `>= 0`, but `Flicks`
is signed: `"duration":-5` loaded and re-saved. `Page::check_layout` now checks
the duration after the grid, with the error the page's other ranges already use,
`PageLayoutError::OutOfRange`, reported by `Document::validate` as
`ValidationError::PageLayout`. `"duration":-5` fails with
`page 1: duration is -5, out of range`; `"duration":0` loads and is
`Some(Flicks(0))`, an absence being `None`, not zero. `gafira-edit` calls the
same `check_layout`, so a negative duration set through a command is refused
there too.

RED, before the fix (`cargo test -p gafira-doc --test duration`):

```text
---- arq_13_a_negative_page_duration_fails_to_load ----
a negative page duration must fail to load, got Ok(Document { …, duration: Some(Flicks(-5)), … })
```

The page-field variant is `PageField::DurationTicks`, not `Duration`: see the
guard note above. It is named for the unit (`flicks`) it holds.

### The amended ADR's two-sided test

ADR 0006 rule 3's sentence — "absent when `None`, present and in canonical
position when `Some`, never `null`" — points at the pair
`page::tests::page_wire_form_writes_every_field` (absent: the whole page's bytes
have no `duration`) and
`page::tests::adr_0006_a_present_duration_sits_between_grid_and_locked`
(present: the whole page's bytes with `"duration":705600000`,
`assert_eq!` on the three bytes after the grid's closing brace and before
`locked`, and `!json.contains("null")`). `tests/duration.rs`'s
`arq_13_a_page_without_a_duration_writes_no_field` is the same absent side one
level up, on the document; the new test is the side `TL1` lacked.

### Hand mutants

| # | The rule | The wrong behaviour | Killed by |
|---|---|---|---|
| M1 | the guard's allow-list | a timeline source is exempt from every time type (`&[]` in the timeline branch), the old whole-file skip | `arq_136_a_clock_type_inside_a_timeline_source_is_refused` (also `arq_136_the_guard_tells_a_type_from_a_word`) |
| M2 | the negative-duration check | the bound is `i64::MIN`, so every value is accepted | `arq_13_a_negative_page_duration_fails_to_load` (`:150`, "must fail to load, got Ok") |
| M3 | type versus word | `code_only` is replaced by the raw source, so comments and strings count | `arq_136_the_guard_tells_a_type_from_a_word` (`:458`) |
| M4 | the duration boundary | `< 0` becomes `<= 0`, so zero is refused | `arq_13_a_negative_page_duration_fails_to_load` (`:154`, zero must load) |

Each mutant was applied, run, seen failing, and reverted; `git status` is clean
and no mutant is committed. M1 and M2 are the two the finding asked for; M3 and
M4 are the boundary and the type/word trap.

### Gates

Per commit, each run alone at that commit, exit 0: `cargo fmt --check`;
`cargo clippy -p gafira-doc --all-targets --locked -- -D warnings`;
`cargo test -p gafira-doc --locked`; `cargo deny check` (`advisories ok, bans
ok, licenses ok, sources ok`); and the nine `policy` steps of
`.github/workflows/ci.yml` re-read from the file (Format, CLI/UI, forbid,
V5, V12, ARQ-107, V13, V14, testkit `ports`), all exit 0. The full workspace
suite runs once at the HEAD of the branch.

### Measured lines

`git diff --shortstat 316d247 HEAD` before this section: 4 files changed,
313 insertions(+), 20 deletions(−), 333 changed lines — 72 of code, 261 of
tests and the guard. Under the ~700 delivery ceiling.

### Deviations

- **The duration's range error is the page's layout error.** Duration is not
  layout data, but `PageLayoutError` is the error the crate already uses for a
  page field out of range and `Page::check_layout` is where `validate.rs` and
  `gafira-edit` already look for those failures; a second page-range error type
  would duplicate the path. The docs of `PageField` and `PageLayoutError` were
  widened from "page-layout data" to "page field" to say so.
- **`PageField::DurationTicks`.** ARQ-136 bans the name `Duration` outside the
  timeline's data files, and the guard reads identifiers in code; naming the new
  variant `Duration`, `Self::Duration` or `PageField::Duration` would have put
  that identifier in `src/layout.rs`, outside the timeline. The variant keeps
  the unit instead, and the message still reads `duration is -5, out of range`.
