# Design: layout-core, and the path to all five layer-2 crates

`layout-core` adds page guides, margins, bleed and grid to `gafira-doc`, then creates `gafira-layout` with the primitives of FD-03..07, FD-09 and FD-11 (D-14). This document also settles the shared decisions that block `gafira-text`, `gafira-raster`, `gafira-audio` and `gafira-segment`, so each can get a Slice card.

Evidence: `spike.md`, `spike-layer2.md`, `spike-e.md`, 02 §2, §3, §4.17, §12.3, 04, D-14 to D-34.

## Quick path

1. ADRs 0005 to 0010 are accepted and the two owner questions are answered (see "Owner decisions").
2. `specifier` applies "Needs specifier". Items S1 to S6 and S8 block `layout-core`.
3. `tdd-apply` runs D1 to D7 in order. All are serial.
4. E1 to E5 are measured (`spike-e.md`). S16 and S17 now block `gafira-text` and `gafira-audio`.

## Decisions at a glance

Numbered `L1` to `L10` after the ten items of the request. `D-nn` stays the owner's, `D1` the deliveries'.

| # | Decision | Record |
|---|---|---|
| L1 | `Guide`, `GuideSet`, `Margins`, `GridSettings` live in `gafira-doc`; `gafira-layout` operates on them | ADR 0008 |
| L2 | Six shared contracts go to a new module `gafira_doc::contract`; the rest go to their owner crate. No new crate | ADR 0005 |
| L3 | Primitives take `Rect`; a thin `node_bounds` builds them. Every `Placement` is page-space, group children included | ADR 0009 |
| L4 | No bump: schema 1 stays open until a build can save. Every field is always written, `GridSettings.style` included | ADR 0006 |
| L5 | `duration`, `background` and `notes` are not added now | below |
| L6 | Five new rows listing transitive crates; every engine may use doc; no `layout -> text`, no `testkit -> raster` | ADR 0007 |
| L7 | `gafira-text` takes plain structs; `RichText` and `TextPath` wait for a later doc change | ADR 0008 |
| L8 | Raster owns `Tile`, `TileSet`, `AlphaMask` (8-bit, tightly packed). Goldens are PNG. The pool is a `ClassicSegmenter` constructor argument | ADR 0008, ADR 0010, below |
| L9 | No `half`, no `rstar` now; own Kaiser sinc resampler; no direct bidi crate; parley with only `std`; `assert_no_alloc`; Noto Sans Arabic and Noto Emoji subsets | below, `spike-e.md` |
| L10 | `Lab` (D50, CSS Color 4), ΔE76 and ΔE2000 in `gafira-color`, as a separate value type | below |

## What exists and what changes

| Requirement | Owner | Today | Change |
|---|---|---|---|
| FD-03(b), FD-05, FD-07 data | `gafira-doc` | `Page` has no such fields (`crates/gafira-doc/src/page.rs`) | D1, D2 |
| FD-03(d), FD-04(c) commands | `gafira-edit` | `Property` has page variants (`crates/gafira-edit/src/set_property.rs`) | D2 adds four |
| FD-11(c) one undo | `gafira-edit` | `Transaction` exists (`crates/gafira-edit/src/transaction.rs`) | None; the caller composes |
| FD-05, FD-09(d) rects | `gafira-layout` | Crate absent | D3 |
| FD-03(a), FD-04, FD-07 | `gafira-layout` | Crate absent | D4 |
| FD-03(c), FD-06 | `gafira-layout` | Crate absent | D5 |
| FD-11 | `gafira-layout` | Crate absent | D6 |
| ARQ-114 budgets | `gafira-layout` | No `criterion` in the workspace | D7 |

## Decisions

Short form for what stays in this change. The ADRs hold the cross-cutting ones.

### L1. Types stored in `Page`

- Status: accepted (ADR 0008).
- Decision: new module in `gafira-doc`, with `Page` fields in 02 §3.1 order after `size`: `bleed`, `margins`, `guides`, `grid`.

| Type | Shape | Default |
|---|---|---|
| `bleed: Length` | One value for all four sides (FD-05) | 0 mm |
| `Margins` | `top`, `right`, `bottom`, `left: Length`, `linked: bool` | 0 px, `linked` true |
| `Guide` | `axis: GuideAxis`, `position: Length`, `locked: bool` | none |
| `GuideAxis` | `Vertical`, `Horizontal` (snake_case on the wire) | none |
| `GuideSet` | `#[serde(transparent)]` newtype over `Vec<Guide>`, no invariant of its own | empty |
| `GridSettings` | `visible`, `step: Length`, `subdivisions: u8`, `style: GridStyle`, `snap` | false, 10 px, 1, `Lines`, true |

- `GuideAxis` is new because the existing `Axis` means page side (`Width`, `Height`, `page_size.rs`) and a guide axis is a line direction.
- `Document::validate` adds a page-field step to P3, after page sizes: guide positions and margins finite, margins ≥ 0, bleed in 0 to 10 mm, step in 1 to 500 in its unit, subdivisions 1 to 10. Nothing is clamped. Margins that leave no area are not a load error: the page can be resized later, so `gafira-layout` reports it (FD-05).
- `Length::new` does not validate, so `Page::new` and the serde defaults stay plain data, as elsewhere (`spike.md` item 3).
- Alternatives: see ADR 0008.

### L3. `gafira-layout` inputs

- Status: accepted (ADR 0009).
- Context: `MeasureCx` and `local_bounds` do not exist; text and image nodes do not exist (`spike.md` item 4).
- Decision: `snap`, `align`, `distribute` and `overflow` take `Rect`s, so they never depend on node kinds. One function, `node_bounds(page, id)`, maps a node to its `Rect`: `Placement::to_transform()` then `Affine::transform_rect_bbox` over `[0,w]×[0,h]` for a leaf (D-28), the union of children for a group, its own box for an empty group. Strokes and effects do not count: the box is the logical box.
- Consequences: FD-11(f), the 45° box, is testable at this level. Text and image nodes reuse the same function when their `Placement` holds their box. A non-finite `Placement` gives `Err` (ARQ-110).
- Alternatives: `Rect` inputs only moves the same code to every caller. `MeasureCx` now has no consumer.

### L5. `duration`, `background` and `notes` wait

- Status: proposed.
- Context: 02 §3.1 lists them. D-14 scopes this change to primitives. None of FD-03..07, 09, 11 reads them.
- Decision: do not add them. `duration` arrives with the first video change, and `doc -> time` with it. `background` arrives with the change that defines its type (02 §3.1 gives none). `notes` arrives with FD-02.
- Consequences: no V5 change to `gafira-doc`, `gafira-edit` or `gafira-testkit` now. With ADR 0006, adding them before the first saving build costs one golden update and no migration.
- Alternatives: adding `duration` now pays for a V5 edit across every row with no reader, and leaves `Option<Flicks>` untested by any behaviour.

### L8. Tiles, masks and the pool

- Status: proposed.
- Tile: `Tile` and `TileSet` belong to `gafira-raster` (02 §2.1); `TileSetRef` stays in `gafira-doc` (ADR 0008).
- `AlphaMask`: lives in `gafira-raster`, because effects (FD-32) and the masks of `gafira-segment` both use it, and `segment -> raster` exists. Layout is `width`, `height`, `Vec<u8>` of `width × height` bytes, row-major, top-down, no padding, 8 bits (ARQ-130). A pixel-format enum in raster covers `Rgba8`, `RgbaF32` and `Alpha8`. `gafira_color::PixelLayout` stays `Rgb`/`Rgba`: a mask is not colour, and `apply_f32` must never accept one.
- Pool: `ClassicSegmenter::new(pool: Arc<rayon::ThreadPool>)`. The trait of 02 §4.7 stays as written. The implementation runs inside `pool.install`, so it never uses the global pool. ARQ-111 tests build it with pools of 1, 2 and 4 threads.
- Effects run single-threaded per call: tile parallelism is `gafira-render`'s (02 §6.3). So `EffectCx` carries no pool, and `gafira-raster` needs no `rayon` in its first slice.
- Alternatives: a `pool` argument on `Segmenter::segment` couples every implementation, `OnnxSegmenter` included, to rayon. Ambient `install` by the caller cannot be enforced: one forgotten call uses the global pool, against 02 §6.3.

### L9. Third-party choices

Status: proposed; measured in `spike-e.md`. "Read" means read in the cached crate's `Cargo.toml`, not built.

| Need | Choice | Status |
|---|---|---|
| `half` | Not now. ARQ-117 leaves `f16` out; it arrives with `gafira-render-gpu`. `half` 2.7.1 is MIT OR Apache-2.0 | Read |
| Spatial index | No `rstar` in the first slice. Each snap axis is a 1-D nearest search over at most thousands of values: a plain scan, with `bench_snap_1000` as the check. 02:1156 keeps it for selection | Decided |
| Resampler | Own Kaiser-windowed sinc in `f64`, cutoff 0.95× lower Nyquist, stop edge 1.0×, 108 taps at ratio ≥ 1, scaled by 1/ratio (1080 at speed 10). `rubato` 5.0.1 was rejected: it returns 48 001 of 48 000 samples, and only reaches its figures with the stop band measured from 1.1× | Decided (E3) |
| Text stack | `parley` 0.12 with `default-features = false, features = ["std"]` | Decided (E1) |
| Bidi | No direct dependency; parley provides it. FD-41(a) pins the visual order in a test | Decided (E1) |
| Allocation guard (D-18) | `assert_no_alloc` 1.1.2 as a dev-dependency of `gafira-audio`, in its own integration test | Decided (E2) |
| Arabic/Hebrew font (D-19) | Noto Sans Arabic subset, 267 KB. Arabic exercises joining as well as RTL. `assets/fonts/test/`, one `LICENSES.toml` entry | Decided (E4) |
| Emoji subset (D-30) | Noto Emoji (monochrome) subset, 6.9 KB | Decided (E4) |
| Golden encoder | `png` 0.18.1 in `gafira-testkit` only (ADR 0010) | Decided (E5) |
| Data parallelism | `rayon` in `gafira-segment` only | 02 §9.2 |
| Ring buffer | `rtrb` waits for `gafira-media`; the ARQ-123 to ARQ-129 slice has no consumer | 02 §9.2 |
| Benchmarks | `criterion` as dev-dependency | 02 §9.2 |

Consequences of the measurements:

- **`gafira-text` wrapper.** It detects glyph id 0 for the ARQ-116 flag, subtracts parley's spacing after the last cluster of a line (FD-41(e)), compares advance with box width for cluster overflow, and implements columns itself. It passes the stack order explicitly. The FD-41(e) ligature test uses Manrope, because Inter has no `liga`.
- **Three-platform determinism stays unmeasured.** ARQ-14 needs the CI golden in `gafira-text`. CI runs on `pull_request` and on `push` to `main` only (`.github/workflows/ci.yml`), and ADR 0003 merges to `develop` without a PR. So the coordinator must run the workflow by `workflow_dispatch` on the branch before verify, or the matrix first sees it at the `develop` -> `main` PR.
- **Budgets.** The E1 timings have a margin near 10×, but were taken on a warm cache and not on D-03 hardware. ARQ-114 benches are still required.
- **Guard placement.** The guard test sits behind `#[cfg(debug_assertions)]`, since the default `disable_release` removes `AllocDisabler` in release. The negative control, proving the guard catches an allocation, needs a subprocess because an allocation aborts. `std::process::Command` is banned in domain crates under `--all-targets` (`crates/gafira-geom/clippy.toml`), so that control goes in a `gafira-testkit` integration test: testkit has no `clippy.toml`. It needs no `gafira-audio` edge. Negative probes use `black_box`.
- **Licence exception.** `deny.toml` gets the `[[licenses.exceptions]]` entry in the first `gafira-audio` delivery. cargo-deny 0.20.2 skips dev-only dependencies of private crates, so the entry is never matched, and `unused-license-exception` defaults to a warning. The check stays green; expect that warning, or set the key to `allow` as `unused-allowed-license` already is.
- **Goldens.** E5 fits ADR 0010, which compares decoded pixels only, so the encoder needs no pin.
- **Fonts.** Subsets total 584 KB, in `gafira-testkit` tests only. Manrope's upstream repo returned 404, so its entry cites the `google/fonts` copy.

MSRV: `parley`, `fontique` and `icu_segmenter` declare 1.88, below the pinned 1.93.1 (`rust-toolchain.toml`).

### L10. `Lab` and colour difference

- Status: proposed.
- Context: ARQ-122 needs Lab and ΔE2000, ARQ-131 needs ΔE76, and FD-33 needs ΔE for detected colours. `gafira-color` has none (02 §4.17.4). `ColorSpaceId` and `Color` are closed, serialized and exhaustive on purpose (`crates/gafira-color/src/space.rs`).
- Decision: a new module in `gafira-color`, in a prerequisite change `color-lab`. `Lab { l, a, b }` is a value type, not a `Color` variant. Conversion from linear sRGB goes through XYZ with the D50 white and Bradford adaptation of CSS Color 4, the source the crate already takes its published references from. `delta_e76` and `delta_e2000` compute in `f64`. ΔE2000 tests use the published Sharma, Wu and Dalal pairs.
- Consequences: no format change, no new `match` arm, no change to `ColorSpaceId`.
- Alternatives: D65 Lab needs no adaptation step, but gives values that differ from CSS `lab()` and from any published CSS test value. A `Color::Lab` variant adds a persisted space nothing needs.

## `gafira-layout` shape

| Module | Holds | IDs |
|---|---|---|
| `error` | `LayoutError`, one typed error naming the field | ARQ-110 |
| `bounds` | `node_bounds` | D-28 |
| `rects` | `bleed_rect`, `safe_rect`, `margin_rect`, `overflow` | FD-05, FD-09 |
| `guides` | `generate`, position checks | FD-03, FD-04 |
| `grid` | Visible lines for a viewport and zoom | FD-07 |
| `snap` | Per-axis snap, rotation snap | FD-06 |
| `align` | `align`, `distribute` | FD-11 |

`gafira-layout` has no serde and mutates nothing (02 §2.1). Every public function checks finiteness first and returns `Err` for the whole call (ARQ-110), as `crates/gafira-pathops/src/path.rs` does.

## Delivery table: `layout-core`

Forecast is authored lines; "x1.4" is the planning figure. Mode rule: all deliveries are serial. D1 and D2 share `page.rs` and `error.rs`. D3 creates the crate. D4 to D6 are separate modules but all edit `lib.rs` and `error.rs`, and D5 consumes D4's grid lines. D7 changes `Cargo.lock`.

| Delivery | Scope | Specs covered | Depends on | Mode | Est. lines |
|---|---|---|---|---|---|
| D1 | `gafira-doc`: L1 types, `Page` fields, always-written wire form, 7 tests and 3 goldens updated, old bytes kept as input fixture, testkit generators | FD-03(b), ADR 0006 | S1 to S3 | serial | 330 (x1.4: 460) |
| D2 | `gafira-doc` validate step; `gafira-edit` `Property::{Bleed, Margins, Guides, Grid}` with ranges and property tests | FD-03(d), FD-04(c), FD-05, FD-07(e) load side | D1, S6 | serial | 300 (x1.4: 420) |
| D3 | `gafira-layout` crate, V5 row, `clippy.toml`, `error`, `bounds`, `rects` | FD-05(b, d, e, f), FD-09(d), D-28, ARQ-110, ARQ-111 | D2, S8 | serial | 300 (x1.4: 420) |
| D4 | `guides`, `grid` | FD-03(a, d), FD-04(a, d, e, f), FD-07(a, c, d, e) | D3 | serial | 340 (x1.4: 475) |
| D5 | `snap`, rotation snap | FD-03(c), FD-06(a to i), D-16, D-28 | D4, S4, S5 | serial | 340 (x1.4: 475) |
| D6 | `align`, `distribute` | FD-11(a, b, d, e, f), D-17, D-28, D-29 | D3 | serial | 240 (x1.4: 335) |
| D7 | `criterion`, `bench_guides_generate`, `bench_layout_rects`, `bench_snap_1000`, `bench_grid_lines` | ARQ-114 | D4, D5 | serial | 90 (x1.4: 125) |

Total forecast 1 640 lines (x1.4: 2 310). The ceiling of ~700 holds for each delivery.

Out of this change, per the first-slice table of 02 §4.17.2: FD-04(b), FD-05(a, c), FD-07(b), FD-09(a, b, c). FD-11(c) needs no new code: the caller wraps the moves in a `Transaction`.

Critical code for hand mutants: L1 validation and the wire form. `T4` runs on `gafira-doc`, `gafira-edit` and `gafira-layout` at close.

## Roadmap after `layout-core`

Order follows the dependency, not the suggested order of 02 §4.17, which puts text before the prerequisites that raster needs. Text and audio are independent of raster, so their order can swap.

| # | Change | Product IDs | Depends on | State |
|---|---|---|---|---|
| 1 | `layout-core` | FD-03..07, FD-09, FD-11 (first slice) | S1 to S6, S8, S19 | Ready after those |
| 2 | `engine-contracts`: `gafira_doc::contract`, `IRect`, `Message`, `MsgArg`, `ToMessage`; retrofit to `LayoutError` and `PathOpError` | ARQ-102 (engines), ARQ-112 (types) | S7, S14 | Ready after those |
| 3 | `color-lab` | ARQ-122 (Lab part), ARQ-131 (ΔE76 part) | S11 | Ready after S11 |
| 4 | `testkit-golden` | ARQ-113 | None | Ready now |
| 5 | `gafira-text` | FD-41(a, c to j), FD-43(a), FD-44(a, c, d, e), FD-45(a, c, d, e), FD-46(a, b, c, e, f), ARQ-14 (`FitBox`), ARQ-115, ARQ-116 | 2, S16 | Blocked by 2 and S16. Determinism is proved inside the change, by CI golden |
| 6 | `gafira-raster` | ARQ-25, ARQ-117 to ARQ-122, FD-12(c, CPU), FD-26, FD-28(b), FD-33(a, d to h), FD-34(a, b), FD-35(a, d) | 2, 3, 4 | Blocked by 2 and 3 (4 is ready). Q-34 does not block |
| 7 | `gafira-audio` | ARQ-23, ARQ-123 to ARQ-129, CAL-15, FV-04(b), FV-10(a, b, d), FV-20(c, d) | 2, S17, S18 | Blocked by 2, S17 and S18 |
| 8 | `gafira-segment` | ARQ-130 to ARQ-132, ARQ-43, CAL-06, FD-32(e), FD-38(a, b, c) | First raster delivery | Blocked by 6 (buffers and `AlphaMask` only) |
| 9 | `doc-text-node`: `TextNode`, `RichText`, `TextPath` | FD-41(b), FD-46(d), FD-47, FD-49 | 5, S9 | Later |
| 10 | `layout-reflow` | FD-53, FD-54, FD-05(c) | 5, 9 | Later (D-14) |

Each engine change needs its own delivery table, but serial deliveries from a Slice card need no `architect` (ADR 0002). E1 kept parley, so text needs none. `gafira-format`, not yet on this list, carries T2, and T1 is checked when it is planned.

Sketch for change 9, not binding: `RichText` is a list of paragraphs, each with runs (`text`, style fields) and an optional list level. `TextPath` is an enum with one MVP variant, `Arc { bend: f32 }`, so the Fase 2 shape path adds a variant.

## Owner decisions

D-14 to D-34 are not reopened. These close the questions raised by this design:

| Topic | Decision |
|---|---|
| ADRs 0005 to 0010 | Accepted |
| Group children | Page-space (ADR 0009, option a), with task T1 |
| End of the open schema | Schema 1 stays open until the first `.gafira`-writing build reaches a user (ADR 0006, option a), with task T2 |
| `assert_no_alloc` | BSD-1-Clause is not in `deny.toml`; it gets a per-crate `[[licenses.exceptions]]` entry with a rationale |
| Safe-zone default | 5 % per side (FD-05) |

## Tracked tasks

Both outlive this change. `archive` moves this file, so the coordinator copies them to the follow-up list of `docs/specs/README.md` when `layout-core` closes.

| # | Task | Closes when | Owner of the check |
|---|---|---|---|
| T1 | Revisit ADR 0009 (page-space group children) against the video design for group keyframes | Before the first saving build; a change after that build needs a migration | Coordinator, when `gafira-format` or the first video change is planned, whichever comes first |
| T2 | Close schema 1: from the first `.gafira`-writing build that reaches a user, any new user-data field bumps the schema (ADR 0006 rule 2) | Closing task of the `gafira-format` change | `gafira-format` Slice card |

## Explorer results

All five are measured, in `spike-e.md`. None blocks a change.

| # | Verdict | Left over |
|---|---|---|
| E1 | parley stays, with `features = ["std"]` and a wrapper | Three-platform determinism, by CI golden in `gafira-text`; `.notdef` rule (S16) |
| E2 | `assert_no_alloc` stays, with the owner's exception | Spec wording (S18) |
| E3 | Own Kaiser sinc | Tie rule (S17) |
| E4 | Fonts stay; all OFL 1.1 with no Reserved Font Name | Hebrew not tested, which D-19 does not require |
| E5 | `png` stays | None |

## Needs specifier

These edit product specs; this design does not.

| # | Where | Edit | Blocks |
|---|---|---|---|
| S1 | 04:66 FD-03 Técnica | Replace "el diseño del change decide" with: `Guide` and `GuideSet` in `gafira-doc` | `layout-core` |
| S2 | 02 §3.1, 04 FD-05 | Name the types of L1: `bleed: Length`, `Margins` with `linked`, its default (proposed true), `Guide`, `GuideAxis`. Say which ranges `Document::validate` enforces on load | `layout-core` |
| S3 | 02 §3.2, 04 FD-13 | State ADR 0009: page-space children, group bounds by union, the Inspector offers no skew on groups | `layout-core` |
| S4 | 04 FD-06, 02:1182 | Drop `rstar` from the snap technique; the bench row keeps its figure | `layout-core` |
| S5 | 04 FD-06 "Quién no atrae" | Silent on nesting. Proposed: nested nodes attract; a node under a hidden or locked ancestor does not; a group attracts by its union | `layout-core` D5 |
| S6 | 04 FD-04 Técnica | `SetGuides` is `SetProperty` with `Property::Guides`, so V14 needs no new command | `layout-core` D2 |
| S7 | 02 §2.1, §4 | `gafira-doc` row gains `contract`; `gafira-geom` row gains `IRect`; raster row gains `AlphaMask`; §4 says where each ADR 0005 type lives | `engine-contracts` |
| S8 | 02 §2.2, 02:170, 02:839 | Write out the edges of ADR 0007; remove "y layout" from ADR-01 criterion 4; close the sentence on the contradiction | `layout-core` D3 |
| S9 | 02 §3.2 | Define `RichText`, `TextPath`, `TextAlign`, `VAnchor`, `TextSizing`, `StyleRef` | `doc-text-node` |
| S10 | 02:503, 04 FD-39, FD-54 | `PaintMask` swaps a `TileSetRef`; `ResizeDesign` receives the reflow result; `gafira-layout::reflow` takes an injected measure function | Later changes |
| S11 | 02 ARQ-122 | State the Lab white (D50, CSS Color 4) and the ΔE2000 reference pairs | `color-lab` |
| S12 | 02 §9.2 | Add `png` for testkit, `assert_no_alloc`, the test fonts; resampler "propio"; parley with `default-features = false, features = ["std"]`; `half` deferred | `gafira-text`, `gafira-audio` (not blocking) |
| S13 | 02 §4.7, §6.3 | Pool is a constructor argument of `ClassicSegmenter`; effects run single-threaded per call | `gafira-segment` |
| S14 | 02 ARQ-102, §14.2 | `gafira-doc::msg` holds only `MessageKey` today: no `Message`, `MsgArg` or `ToMessage`, and no error implements it. Say when existing crates comply | `engine-contracts` |
| S15 | 02:84 | MSRV says 1.95; `rust-toolchain.toml` pins 1.93.1 and `Cargo.toml` says 1.93 | None |
| S16 | 04 FD-41, ARQ-116 | Say which font supplies `.notdef` when no font covers a cluster. Proposed, matching parley: the font with the most coverage, ties to the earliest in the stack; uncovered code points use its `.notdef`, and the output flags them | `gafira-text` |
| S17 | 02 ARQ-128 | The length formula is ambiguous at ties: with `f64` speed it differs from exact in 183 of 8 386 147 grid cases. Proposed: the formula evaluated in `f64` is normative, which is deterministic and needs only IEEE operations | `gafira-audio` |
| S18 | 02:664, 02 ARQ-23 | The guard aborts on an allocation; it does not `panic`. State that, the debug-only scope, and that the negative control lives in `gafira-testkit` | `gafira-audio` |
| S19 | 04 FD-05, 02 §4.17.2 | Safe-zone default is fixed at 5 % per side: drop PROPUESTA, which D-34 had left out. `safe_rect` still takes the percentage as an input | `layout-core` D3 |
| S20 | 02 §9.1, D-18 | Add the BSD-1-Clause exception for `assert_no_alloc`. Correct "toda dependencia nueva pasa `cargo deny`": dev-only dependencies of private crates are not licence-checked, which also covers `png` in testkit, `proptest`, `criterion` and `trybuild` | None |
| S21 | `00-decisiones.md` | Record the owner decisions of this design as new entries: group children, end of the open schema, the BSD-1-Clause exception, safe zone. This file is the owner's, so the coordinator decides who writes it | None |

## Next

Run `specifier` for S1 to S6, S8 and S19; then `tdd-apply` for D1. `testkit-golden` is ready now and is independent of `layout-core`.
