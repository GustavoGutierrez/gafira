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
Product IDs: ARQ-10, ARQ-13, ARQ-30, ARQ-53, V-5, D-36, FV-02 (source: specs/02-arquitectura-software.md §3.1, §3.5, §7.1; specs/05-funcionalidad-video-y-distribucion.md FV-02; ADR 0006)
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

Gates per commit, each run alone, exit 0: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` (`gafira-doc` is not a leaf), `cargo test --workspace --locked`, `cargo deny check`, and the CI policy steps by extracting their `run:` from `ci.yml`.

### RED evidence

### Negative controls

### Choices and deviations
