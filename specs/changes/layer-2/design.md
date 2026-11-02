# Design: the rest of layer 2

This plan finishes `gafira-text`, `gafira-raster`, `gafira-audio` and `gafira-segment`, plus the prerequisite `color-lab`, with one writer per worktree. The key move is a scaffold delivery, `SC1`, which lands every shared-file change once. After it, each stream touches only its own crate, so the four crates merge without conflicts.

Evidence: 02 §2, §4.17, §9.2, §10, 04, ADR 0005, ADR 0007, ADR 0008, ADR 0010, `specs/history/2026-11-01-layout-core/design.md` (L8, L9), `specs/changes/engine-contracts/design.md`.

## Quick path

1. Finish `engine-contracts`. `CL1` can run beside it.
2. Clear the blockers below, one at a time: O2, O3, then `specifier` for SP1, SP5, SP6, SP7, SP9 and SP10, then `explorer` for XP2 to XP5.
3. Run `SC1` alone.
4. Open one worktree per stream: `TX`, `RS`, `AU`. Open `SG` when `RS1` is on `develop`.
5. Merge each delivery after its verify PASS, one at a time. Run T4 per crate at close, one crate at a time.

## Delivery table

Forecast is authored lines, with the x1.4 planning figure in brackets. Every delivery stays under ~700 lines at x1.4. "Merge" is the order onto `develop`; `any` means first PASS, first merged.

| Delivery | Crate | Product IDs | Depends on | Mode | Forecast | Merge |
|---|---|---|---|---|---|---|
| `D1`..`D3` | doc, geom, pathops, layout | ARQ-102, ARQ-112 | see `engine-contracts` | see `engine-contracts` | 1 200 (1 685) | 1 |
| `CL1` | `gafira-color` | ARQ-122 (Lab, ΔE2000), ARQ-131 (ΔE76) | SP1 | parallel, beside `D1`..`D3` | 280 (390) | any, before `RS6` |
| `SC1` | all four, testkit, shared files | ARQ-01, ARQ-02, ARQ-70 (rows, lints, licences) | `D3`, XP2 | serial | 150 (210) | 2 |
| `TX1` | `gafira-text` | FD-41 (a, d), ARQ-115, ARQ-116, ARQ-110, ARQ-111 | `SC1`, XP3 | parallel stream `TX` | 450 (630) | any |
| `TX2` | `gafira-text` | FD-41 (c, e to j) | `TX1` | serial in `TX` | 440 (615) | after `TX1` |
| `TX3` | `gafira-text` | FD-45 (a, c, d, e), FD-46 (a, b, c, e, f), ARQ-14 (`FitBox`) | `TX2`, O2 | serial in `TX` | 400 (560) | after `TX2` |
| `TX4` | `gafira-text` | FD-44 (a, c, d, e), FD-43 (a), ARQ-114 (text) | `TX2` | serial in `TX` | 350 (490) | after `TX3` |
| `RS1` | `gafira-raster` | ARQ-117, ARQ-121, ARQ-110, ARQ-111 | `SC1` | parallel stream `RS` | 450 (630) | any, before `SG1` |
| `RS2` | `gafira-raster` | ARQ-119, FD-12 (c, CPU) | `RS1` | serial in `RS` | 340 (475) | after `RS1` |
| `RS3` | `gafira-raster` | ARQ-118, ARQ-25 (ends) | `RS1` | serial in `RS` | 450 (630) | after `RS2` |
| `RS4` | `gafira-raster` | ARQ-120, FD-35 (a, d), ARQ-114 (`bench_shadow_1080`) | `RS3`, XP4 | serial in `RS` | 420 (590) | after `RS3` |
| `RS5` | `gafira-raster` | FD-33 (a, d to h), except colours detected by ΔE | `RS4` | serial in `RS` | 480 (670) | after `RS4` |
| `RS6` | `gafira-raster` | ARQ-122, FD-26, FD-28 (b), FD-33 (detected colours), FD-34 (a, b), ARQ-114 (`bench_dhash_12mp`) | `RS5`, `CL1`, O3, SP9 | serial in `RS` | 420 (590) | after `RS5` |
| `AU1` | `gafira-audio` | ARQ-123, ARQ-124, ARQ-129, FV-10 (a, d), ARQ-110, ARQ-111 | `SC1` | parallel stream `AU` | 480 (670) | any |
| `AU2` | `gafira-audio`, testkit test | ARQ-125, ARQ-23, CAL-15, FV-20 (c) | `AU1` | serial in `AU` | 400 (560) | after `AU1` |
| `AU3` | `gafira-audio` | ARQ-126, ARQ-127, FV-10 (b), FV-20 (d) | `AU1`, SP11 | serial in `AU` | 480 (670) | after `AU2` |
| `AU4` | `gafira-audio` | ARQ-128, FV-04 (b) | `AU1` | serial in `AU` | 350 (490) | after `AU3` |
| `SG1` | `gafira-segment` | ARQ-130 (shape, sizes, hints), ARQ-110 | `RS1`, SP6, XP5 | parallel stream `SG` | 450 (630) | after `RS1` |
| `SG2` | `gafira-segment` | ARQ-130 (seeds, zones), FD-38 (reduction) | `SG1` | serial in `SG` | 480 (670) | after `SG1` |
| `SG3` | `gafira-segment`, testkit | ARQ-131, ARQ-132, FD-32 (e), FD-38 (a, b, c), ARQ-43, CAL-06, ARQ-111, ARQ-112 | `SG2`, `CL1`, SP5, SP7 | serial in `SG` | 480 (670) | after `SG2` |

Total after `engine-contracts`: 7 750 lines (x1.4: 10 850). The largest delivery is 480 lines (x1.4: 670).

### Why each mark holds

| Mark | Ground |
|---|---|
| `CL1` beside `engine-contracts` | It touches only `crates/gafira-color/`. The owner's order is still met if `CL1` starts after D1; running it beside saves time and shares no file |
| `SC1` serial | It edits every shared file, and `ci.yml` and `Cargo.lock` also change in `D3` |
| `TX`, `RS`, `AU` parallel | ADR 0007: no edge between them. `text -> pathops` needs only the merged `gafira-pathops` |
| `SG` starts after `RS1` | `segment -> raster`: it needs `ImageView` and `AlphaMask` merged. It needs nothing else from raster (L8), so it runs beside `RS2`..`RS6` |
| Serial inside a stream | Each delivery edits the same `lib.rs` and `error.rs`. Inside one stream, the owner may run them as one flow |
| `RS6` late | It needs `CL1` (ARQ-122). `SG3` needs `CL1` too, for ΔE76 |

## Decisions

### P1. A scaffold delivery first

- Options: (a) each crate adds its own shared lines, and merges fix the conflicts; (b) one delivery adds them all.
- Decision: (b). `SC1` holds what ADR 0007 requires in one commit (row, `clippy.toml`, `Cargo.toml` entries), for all four crates.

| `SC1` adds | Detail |
|---|---|
| Crates | `crates/gafira-{text,raster,audio,segment}`: `Cargo.toml`, `clippy.toml` byte-identical to `crates/gafira-geom/clippy.toml`, `lib.rs` with `#![forbid(unsafe_code)]` |
| Internal deps | Those of the V5 rows below, declared now so the rows are exercised |
| Workspace entries | Four path entries; `parley = { version = "0.12", default-features = false, features = ["std"] }`; `rayon`; `assert_no_alloc = "1.1"` |
| Dev-deps | `proptest`, `criterion`, `gafira-testkit` per crate; `assert_no_alloc` in `gafira-audio` and in `gafira-testkit` (L9) |
| V5 rows | The four rows of ADR 0007 |
| `deny.toml` | BSD-1-Clause `[[licenses.exceptions]]` for `assert_no_alloc`, and `unused-license-exception` as in the L9 note |
| testkit stubs | Empty `pub mod fonts;` and `pub mod scenes;` |
| `Cargo.lock` | Regenerated once, with all of the above |

Unused dependencies do not fail `clippy` here: `unused_crate_dependencies` is off by default, and `Cargo.toml` does not enable it.

### P2. How shared files resolve

After `SC1` no stream should touch these. If one must, this table applies.

| File | Touched by | Resolution at merge |
|---|---|---|
| `Cargo.toml` (root) | `SC1` | A later dependency is a stop: mark that delivery serial and merge it alone. Keep both lines |
| `Cargo.lock` | `SC1` | Never hand-merge. Take `develop`'s file, run `cargo check --workspace --all-targets` to regenerate, then the gates with `--locked` |
| `.github/workflows/ci.yml` | `D3`, `SC1` | Serial by order. Keep both rows |
| `deny.toml` | `SC1` | Single writer |
| `clippy.toml` copies | `SC1` | New files. A change to the canonical copy needs every copy in one commit (V12), so it is serial |
| `.cargo/mutants.toml` | T4 fix rounds | Keep both entries. Its regexes hold line numbers, so re-check them after a merge that moves that file. T4 runs one crate at a time, which keeps these serial |
| `crates/gafira-testkit/` | `TX1` (`fonts.rs`), `AU2` (one test file), `SG3` (`scenes.rs`), `D2b` | Separate files. The stubs in `SC1` remove the `lib.rs` conflict |
| `assets/LICENSES.toml` | `TX1` | Single writer: the file does not exist today |
| `.odd/state.json` | Coordinator, each branch | Union by `outcome_id`: keep every entry, and for the same id take the one with the newest commit. Use one outcome per stream |
| `specs/README.md` | Coordinator, at archive | One archive at a time |

Change folders: each stream gets `specs/changes/gafira-{text,raster,audio,segment}/` for its `apply-progress.md`, `verify-report.md` and `verification.json`, and cites this design. Separate folders avoid conflicts on those fixed file names.

### P3. Parallel run costs

- T4 runs one crate at a time. Follow-ups 24 and 31 show load timeouts with `--jobs 8`, and parallel worktrees would make that worse.
- `verify` runs per delivery in its worktree. After each merge the coordinator runs the gates once on `develop` (ADR 0003).
- Each worktree builds its own `target/`.

### P4. Interfaces between streams

Only `SG` consumes another stream, so only this shape is frozen across streams. It follows L8 and ADR 0005.

```rust
// gafira-raster, RS1
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat { Rgba8, RgbaF32, Alpha8 }

#[derive(Clone, Copy, Debug)]
pub enum PixelData<'a> { Rgba8(&'a [u8]), RgbaF32(&'a [f32]), Alpha8(&'a [u8]) }

#[derive(Clone, Copy, Debug)]
pub struct ImageView<'a> { /* width, height, data */ }

impl<'a> ImageView<'a> {
    pub fn new(width: u32, height: u32, data: PixelData<'a>) -> Result<Self, BufferError>;
    pub fn width(&self) -> u32;
    pub fn height(&self) -> u32;
    pub fn data(&self) -> PixelData<'a>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlphaMask { /* width, height, data: Vec<u8> */ }

impl AlphaMask {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self, BufferError>;
    pub fn width(&self) -> u32;
    pub fn height(&self) -> u32;
    pub fn data(&self) -> &[u8];
}
```

Fields are private, because length is an invariant (ARQ-117). The check uses `checked_mul`, so an overflow is `Err` with no allocation.

### P5. Types 02 names but no crate owns

| Type | Home | Decision |
|---|---|---|
| `Cost` | `gafira-raster` | `Cost { pub work: u64 }`, abstract work units from pixels and kernel size. No time: ARQ-02 |
| `EffectCx` | `gafira-raster` | `EffectCx { pub scale: f32 }`. No pool (L8). No token: `gafira-render` checks per tile (02:1145) |
| `HandleReq` | `gafira-raster` | `Both`, `From`, `To`, `None` (02:736) |
| `GpuEffect`, `GpuTransition`, `UniformBlock` | `gafira-raster` | Traits as in 02; `UniformBlock` is a `Vec<u8>` newtype. No implementation in this cut |
| `AudioBlock<'_>` | `gafira-audio` | Planar: `channels: &mut [&mut [f32]]`, equal lengths |
| `ParamSnapshot` | `gafira-audio` | A fixed `Copy` array of up to 16 `(id, value)` pairs, so reading it never allocates (ARQ-23) |
| `SegHints`, `SegReq`, `SegError` | `gafira-segment` | Per ARQ-130 |

SP12 records these homes in 02 §4.17.

### P6. `color-lab`

- `crates/gafira-color/src/lab.rs`, exported from `lib.rs`:

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lab { pub l: f64, pub a: f64, pub b: f64 }

impl Lab {
    #[must_use]
    pub fn from_linear_srgb(c: LinearRgba) -> Lab;
}

#[must_use]
pub fn delta_e76(a: Lab, b: Lab) -> f64;
#[must_use]
pub fn delta_e2000(a: Lab, b: Lab) -> f64;
```

- As L10: value type, D50, Bradford, `f64`. Alpha is ignored. No inverse conversion: no ID needs one. Non-finite input follows domain plain-data rules (no `Result`), because `gafira-color` is layer 0 and ARQ-110 binds layer 2.
- Consumers `RS6` and `SG3` check their own inputs.

### P7. Text test fonts

`git ls-files` finds no font, and `assets/LICENSES.toml` does not exist, so E4's subsets were never committed. `TX1` regenerates them under `assets/fonts/test/` and creates the file with font entries only. ARQ-02 bans `std::fs` under `--all-targets`, so `gafira_testkit::fonts` exposes them with `include_bytes!`. The ARQ-115 check is a testkit integration test, which may read files: testkit has no `clippy.toml`.

### P8. Benchmarks that need a clock

`std::time::Instant::now` is banned under `--all-targets` in domain crates, so `bench_cancel_segment` cannot time cancellation directly. `SG3` benches the longest work unit between two checkpoints with plain `criterion`. That bounds ARQ-43 from above. The name stays.

## Critical code

Hand mutants apply on all of these. T4 at close covers every crate in the table.

| Delivery | Critical | Not critical |
|---|---|---|
| `CL1` | Lab conversion, ΔE formulas (ARQ-122) | None |
| `SC1` | None; config only, checked by V5, V12 and `cargo deny` | All |
| `TX1`..`TX4` | Range checks, FD-46 search bounds, curve geometry, overflow and `.notdef` flags | Benches |
| `RS1`..`RS3` | Length and overflow checks, premultiply, mip levels, blend formulas, transition ends | None |
| `RS4`..`RS6` | Parameter checks, `roi`, alpha preservation, dHash threshold | Benches |
| `AU1`..`AU4` | `process` totality, subnormals, offset arithmetic, peak ceiling, output length | Benches |
| `SG1`..`SG3` | Size checks, ARQ-131 bounds, IoU, cancellation | Benches, scene generators' visuals |

T4 at close: `gafira-color`, `gafira-text`, `gafira-raster`, `gafira-audio`, `gafira-segment`, `gafira-testkit`.

## Open questions (owner)

Asked one at a time. O1 is in the `engine-contracts` design.

| # | Question | Options and tradeoffs | Recommendation |
|---|---|---|---|
| O2 | CI is disabled (follow-up 2), so ARQ-14's "same size on 3 platforms" cannot run, and ADR 0003 merges never reach the matrix. What happens at `TX3`? | (a) Re-enable CI first, and run it by `workflow_dispatch` on the branch: real proof, needs billing fixed. (b) Merge on Linux evidence; ARQ-14 stays open as a follow-up: fast, unproven on Windows and macOS. (c) Stop `TX3`: `verify` reports BLOCKED | (a). If billing cannot be fixed soon, (b), recorded by the owner |
| O3 | FD-34 has no filter catalogue: no names, no values, and no rule for what intensity does. These are product choices | (a) `RS6` ships the mechanism and 1 fixture preset; the catalogue comes later with Q-34: tests FD-34 (a, b) now. (b) Owner gives the 12 presets now: complete, slower. (c) Move FD-34 out of the cut | (a), with intensity as a linear blend of each parameter from its default to the preset value (SP9) |

If XP5 shows ARQ-132 is out of reach, D-20 is the owner's decision, and that becomes a new question then.

## Needs explorer

Nothing below is measured. `E1` to `E5` are the older spikes.

| # | Claim | Check | Blocks |
|---|---|---|---|
| XP1 | See `engine-contracts` | | D1 |
| XP2 | The parley 0.12 tree on all three targets passes `cargo deny check`, the V5 forbidden regex and ARQ-107's banned crates, and our code needs no `Locale` identifier. E1 covered parley, not this combined workspace with `rayon` | Throwaway `SC1`, with `cargo tree --target all` | `SC1` |
| XP3 | E4's subsets can be regenerated with the same results: sizes, advances, Arabic joining, Manrope `liga`. Inputs were not kept, and upstream Manrope returned 404 | `pyftsubset` from pinned upstream files | `TX1` |
| XP4 | `bench_shadow_1080` < 100 ms single-threaded at radius 40 with a direct kernel. Near 120 taps × 2 passes × 2 M pixels, this looks tight. An IIR blur likely breaks the ARQ-120 tile match; a three-box blur must hold σ within ±2 % | Prototype direct and three-box on one alpha channel; compare σ and time | `RS4` (provisional) |
| XP5 | Own SLIC, GMM, max-flow and GrabCut reach IoU ≥ 0.90 on every ARQ-132 scene, low contrast included, and 4000 px in < 5 s on D-03 hardware (R-04). This is the largest technical risk of the plan | Prototype on generated scenes, before `SG1` | `SG1` |
| XP6 | `SC1` with unused dependencies passes every gate, with cargo-deny on 3 targets | Same run as XP2 | `SC1` |

## Needs specifier

| # | ID | Gap and proposed edit | Blocks |
|---|---|---|---|
| SP1 | ARQ-122, ARQ-131 (old S11) | S11 is half done: 02:945 now states D50, CSS Color 4 and the Sharma source. Still missing: the pair count and tolerance, a Lab reference value, a ΔE76 definition and a criterion for it. Proposed: all 34 Sharma pairs ±1e-4; sRGB white gives `L` 100, `a` and `b` 0, ±1e-3; plus CSS Color 4 sample values; ΔE76 is the Euclidean distance | `CL1` |
| SP2 to SP4 | ARQ-102, 02:429, 02:1298 | See `engine-contracts` | None |
| SP5 | ARQ-112 | No list of which operations are long, and "entrada de referencia" is undefined. Only `Segmenter::segment` has a token in layer 2 signatures. Proposed: `segment` only, with a reference of 1 scene from ARQ-132; say whether loudness and resampling count | `SG3` |
| SP6 | ARQ-131, 02:602 | Contradiction: "el resultado indica si es fiable", but `segment` returns `Result<AlphaMask, SegError>`. Proposed: `Result<Segmentation, SegError>`, with `Segmentation { mask, reliable }` | `SG1` |
| SP7 | ARQ-131, ARQ-132 | Scenes must include "contraste bajo", yet all must be reliable, which needs ΔE76 ≥ 5. Proposed: low contrast means ΔE76 between 5 and 10 | `SG3` |
| SP8 | FD-12 (c) | The criterion is CPU against GPU only, so the "mitad CPU" has no test of its own. Proposed: it is ARQ-119 | None |
| SP9 | FD-34 (a, b) | No preset format, intensity rule or thumbnail path. Depends on O3 | `RS6` |
| SP10 | FD-35 (a, d) | Only Paralela has a criterion. Brillo and Contorno have ranges but no formula or test. Proposed: state them, or leave them out of the cut | `RS4` |
| SP11 | ARQ-126 | 6-channel order is unstated. Proposed: L, R, C, LFE, Ls, Rs | `AU3` |
| SP12 | 02 §4.17, §4.4, §4.13 | Add the homes of P5 to the table of shared types. `Cost` has no definition | None |
| SP13 | ARQ-43 | Its 100 ms cannot be timed in a domain crate's bench (P8). Say what `bench_cancel_segment` measures | None |
| SP14 | ARQ-115, ARQ-75 | `assets/LICENSES.toml` must cover all of `assets/`, but ARQ-115 needs only fonts now. Say that a partial file is fine until `xtask` exists | None |

Coordinator, not `specifier`: CAL-06 and CAL-15 live in `backlog/01`, and the FV IDs in `backlog/05`. The rule in `specs/README.md` promotes a backlog document when a change of 02 needs it, in its own commit. Do both before `SC1`, on `develop`, so no stream carries the rename.

## Next

Ask O2, then O3. Run `specifier` for the blocking SP items and `explorer` for XP2 to XP5. `tdd-apply` starts with `D1`.
