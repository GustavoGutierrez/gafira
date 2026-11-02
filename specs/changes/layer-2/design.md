# Design: the rest of layer 2

This plan finishes `gafira-text`, `gafira-raster`, `gafira-audio` and `gafira-segment`, plus the prerequisite `color-lab`, with one writer per worktree. The key move is a scaffold delivery, `SC1`, which lands every shared-file change once. After it, each stream touches only its own crate, so the four crates merge without conflicts.

Evidence: 00 (D-09, D-40 to D-44), 02 §2, §4.17, §9.2, §10, 04, ADR 0005, ADR 0007, ADR 0008, ADR 0010, `specs/history/2026-11-01-layout-core/design.md` (L8, L9), `specs/changes/engine-contracts/design.md`.

## Quick path

1. Run `CL1` now, beside `engine-contracts`. Nothing blocks it.
2. Finish `D2a`, `D2b` and `D3`.
3. Run `explorer` for XP10, XP3 and XP4, then for XP5, XP7, XP8 and XP9 as one prototype.
4. Run `SC1` alone.
5. Open one worktree per stream: `TX`, `RS`, `AU`. Open `SG` when `RS1` is on `develop`.
6. Merge each delivery after its verify PASS, one at a time. Run T4 per crate at close, one crate at a time.

## Dependency state

State at `develop` 3b700dc. `D1` is merged. No owner question and no `specifier` row blocks any delivery.

| Delivery | State | Blocked by |
|---|---|---|
| `CL1` | Ready | Nothing |
| `D2a`, `D3` | Ready, with worktrees open | Nothing |
| `D2b` | Blocked | `D2a` |
| `SC1` | Blocked | `D2b`, `D3`, XP10 |
| `TX1` | Blocked | `SC1`, XP3, XP10 |
| `TX2` | Blocked | `TX1` |
| `TX3`, `TX4` | Blocked | `TX2` |
| `RS1` | Blocked | `SC1` |
| `RS2`, `RS3` | Blocked | `RS1` |
| `RS4` | Blocked | `RS3`, XP4 |
| `RS5` | Blocked | `RS4` |
| `RS6` | Blocked | `RS5`, `CL1` |
| `AU1` | Blocked | `SC1` |
| `AU2`, `AU3`, `AU4` | Blocked | `AU1` |
| `SG1` | Blocked | `RS1`, XP5, XP7, XP8, XP9 |
| `SG2` | Blocked | `SG1` |
| `SG3` | Blocked | `SG2`, `CL1` |

## Delivery table

Forecast is authored lines, with the x1.4 planning figure in brackets. Every delivery stays under ~700 lines at x1.4. "Merge" is the order onto `develop`; `any` means first PASS, first merged.

| Delivery | Crate | Product IDs | Depends on | Mode | Forecast | Merge |
|---|---|---|---|---|---|---|
| `D1`..`D3` | doc, geom, pathops, layout | ARQ-102, ARQ-112 (on doubles) | see `engine-contracts` | see `engine-contracts` | 1 200 (1 685) | 1; `D1` merged |
| `CL1` | `gafira-color` | ARQ-122 (Lab, ΔE2000, ΔE76) | None | parallel, beside `D1`..`D3` | 280 (390) | any, before `RS6` |
| `SC1` | all four, testkit, shared files | ARQ-01, ARQ-02, ARQ-70 (rows, lints, licences), ARQ-107 (CI step) | `D2b`, `D3`, XP10 | serial | 180 (250) | 2 |
| `TX1` | `gafira-text` | FD-41 (a, d), ARQ-115, ARQ-116, ARQ-75 (partial file), ARQ-110, ARQ-111 | `SC1`, XP3, XP10 | parallel stream `TX` | 465 (650) | any |
| `TX2` | `gafira-text` | FD-41 (c, e to j) | `TX1` | serial in `TX` | 440 (615) | after `TX1` |
| `TX3` | `gafira-text` | FD-45 (a, c, d, e), FD-46 (a, b, c, e, f), ARQ-14 (`FitBox`) | `TX2` | serial in `TX` | 400 (560) | after `TX2` |
| `TX4` | `gafira-text` | FD-44 (a, c, d, e), FD-43 (a), ARQ-114 (text) | `TX2` | serial in `TX` | 350 (490) | after `TX3` |
| `RS1` | `gafira-raster` | ARQ-117, ARQ-121, ARQ-110, ARQ-111 | `SC1` | parallel stream `RS` | 450 (630) | any, before `SG1` |
| `RS2` | `gafira-raster` | ARQ-119, FD-12 (c, CPU) | `RS1` | serial in `RS` | 340 (475) | after `RS1` |
| `RS3` | `gafira-raster` | ARQ-118, ARQ-25 (ends) | `RS1` | serial in `RS` | 450 (630) | after `RS2` |
| `RS4` | `gafira-raster` | ARQ-120, FD-35 (a, d; Paralela only, D-42), ARQ-114 (`bench_shadow_1080`) | `RS3`, XP4 | serial in `RS` | 420 (590) | after `RS3` |
| `RS5` | `gafira-raster` | FD-33 (a, d to h), except colours detected by ΔE | `RS4` | serial in `RS` | 480 (670) | after `RS4` |
| `RS6` | `gafira-raster` | ARQ-122, FD-26, FD-28 (b), FD-33 (detected colours), FD-34 (a, b; D-40, D-41), ARQ-114 (`bench_dhash_12mp`) | `RS5`, `CL1` | serial in `RS` | 480 (670) | after `RS5` |
| `AU1` | `gafira-audio` | ARQ-123, ARQ-124, ARQ-129, FV-10 (a, d), ARQ-110, ARQ-111 | `SC1` | parallel stream `AU` | 480 (670) | any |
| `AU2` | `gafira-audio`, testkit test | ARQ-125, ARQ-23, CAL-15, FV-20 (c) | `AU1` | serial in `AU` | 400 (560) | after `AU1` |
| `AU3` | `gafira-audio` | ARQ-126, ARQ-127, FV-10 (b), FV-20 (d) | `AU1` | serial in `AU` | 480 (670) | after `AU2` |
| `AU4` | `gafira-audio` | ARQ-128, FV-04 (b) | `AU1` | serial in `AU` | 350 (490) | after `AU3` |
| `SG1` | `gafira-segment` | ARQ-130 (shape, sizes, hints), ARQ-110 | `RS1`, XP5, XP7, XP8, XP9 | parallel stream `SG` | 450 (630) | after `RS1` |
| `SG2` | `gafira-segment` | ARQ-130 (seeds, zones), FD-38 (reduction) | `SG1` | serial in `SG` | 480 (670) | after `SG1` |
| `SG3` | `gafira-segment`, testkit | ARQ-131, ARQ-132, FD-32 (e), FD-38 (a, b, c), ARQ-43, CAL-06, ARQ-111, ARQ-112 | `SG2`, `CL1` | serial in `SG` | 490 (685) | after `SG2` |

Total after `engine-contracts`: 7 865 lines (x1.4: 11 010). The largest delivery is 490 lines (x1.4: 685).

Scope of the cut, as closed by the owner:

- `RS4` ships Paralela only. Brillo and Contorno wait for approved formulas (D-42).
- `RS6` ships the FD-34 mechanism and 1 test preset (D-40). The catalogue comes with Q-34. FD-34 (c) stays out.
- ARQ-112 binds only `Segmenter::segment`. Loudness, Nivelar and resampling are out (D-43), so no `AU` delivery covers it.
- `RS4` keeps its forecast: the old 420 never held formulas for Brillo or Contorno.

### Why each mark holds

| Mark | Ground |
|---|---|
| `CL1` beside `engine-contracts` | It touches only `crates/gafira-color/`. The owner's order is still met if `CL1` starts after D1; running it beside saves time and shares no file |
| `SC1` serial | It edits every shared file, and `ci.yml` and `Cargo.lock` also change in `D3`. `D2b` touches testkit, so `SC1` follows it too |
| `TX3` proof | ARQ-14 needs the 3 platforms. CI runs on `main` and pull requests, not on pushes to `develop` (`.github/workflows/ci.yml`), so `verify` of `TX3` needs a `workflow_dispatch` run on its branch. This is a step, not a dependency |
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
| Internal deps | Those of the four V5 rows of ADR 0007, declared now so the rows are exercised |
| CI | The four V5 rows of ADR 0007 and the ARQ-107 step (P9) |
| Workspace entries | Four path entries; `parley = { version = "0.12", default-features = false, features = ["std"] }`; `rayon`; `assert_no_alloc = "1.1"` |
| Dev-deps | `proptest`, `criterion`, `gafira-testkit` per crate; `assert_no_alloc` in `gafira-audio` and in `gafira-testkit` (L9) |
| `deny.toml` | BSD-1-Clause `[[licenses.exceptions]]` for `assert_no_alloc`, and `unused-license-exception = "allow"`, because cargo-deny never matches that dev-only entry (L9) |
| testkit stubs | Empty `pub mod fonts;` and `pub mod scenes;` |
| `Cargo.lock` | Regenerated once, with all of the above |

Unused dependencies do not fail `clippy` here: `unused_crate_dependencies` is off by default, and `Cargo.toml` does not enable it.

`SC1` also runs one `workflow_dispatch` on its branch. It proves the new `ci.yml` steps, and its clippy step compiles the new dependencies on Windows and macOS, the gap left by XP2.

### P2. How shared files resolve

After `SC1` no stream should touch these. If one must, this table applies.

| File | Touched by | Resolution at merge |
|---|---|---|
| `Cargo.toml` (root) | `SC1` | A later dependency is a stop: mark that delivery serial and merge it alone. Keep both lines |
| `Cargo.lock` | `SC1` | Never hand-merge. Take `develop`'s file, run `cargo check --workspace --all-targets` to regenerate, then the gates with `--locked` |
| `.github/workflows/ci.yml` | `D3`, `SC1` | Serial by order. Keep both rows, and the ARQ-107 step of `SC1` |
| `deny.toml` | `SC1` | Single writer |
| `clippy.toml` copies | `SC1` | New files. A change to the canonical copy needs every copy in one commit (V12), so it is serial |
| `.cargo/mutants.toml` | T4 fix rounds | Keep both entries. Its regexes hold line numbers, so re-check them after a merge that moves that file. T4 runs one crate at a time, which keeps these serial |
| `crates/gafira-testkit/` | `TX1` (`fonts.rs`), `AU2` (one test file), `SG3` (`scenes.rs`), `D2b` | Separate files. The stubs in `SC1` remove the `lib.rs` conflict |
| `assets/LICENSES.toml` | `TX1` | Single writer: the file does not exist today (P7) |
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
| `Segmentation`, `SegHints`, `SegReq`, `SegError` | `gafira-segment` | `Segmentation { pub mask: AlphaMask, pub reliable: bool }` (02 §4.7, ARQ-130, ARQ-131). `segment` returns `Result<Segmentation, SegError>` |

These homes are now in 02 §4.17, with `Cost` defined there.

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

- As L10 and ARQ-122: value type, D50, Bradford, `f64`. Alpha is ignored. No inverse conversion: no ID needs one. Non-finite input is unspecified there (no `Result`), because `gafira-color` is layer 0 and ARQ-110 does not bind it (ARQ-122).
- Consumers `RS6` and `SG3` check their own inputs.

### P7. Text test fonts

`git ls-files` finds no font, and `assets/LICENSES.toml` does not exist, so E4's subsets were never committed. `TX1` regenerates them under `assets/fonts/test/`.

`TX1` also creates `assets/LICENSES.toml`. ARQ-75 allows a partial file until `xtask` exists, but its first commit needs both of these:

| Entry | Source of the fields |
|---|---|
| Every test font | XP3: upstream, version, licence (ARQ-115) |
| `assets/splash_screen.jpg` | D-09, "Procedencia de la imagen": origin, tool, licence basis, AI notice, copyright note, provenance, similarity check. The similarity check stays VERIFICAR (pending item 6 of `00`) |

ARQ-02 bans `std::fs` under `--all-targets`, so `gafira_testkit::fonts` exposes the fonts with `include_bytes!`. The ARQ-115 check is a testkit integration test, which may read files: testkit has no `clippy.toml`. The same test asserts the splash entry, so the second mandatory entry cannot go missing.

### P8. Benchmarks that need a clock

`std::time::Instant::now` is banned under `--all-targets` in domain crates, so `bench_cancel_segment` cannot time cancellation directly. `SG3` benches the longest work unit between two checkpoints with plain `criterion`. That bounds ARQ-43 from above, and ARQ-43 now says so. The input is a 4000 px image. The name stays. XP8 checks that the guided filter fits.

### P9. The ARQ-107 CI step

- Provisional until XP10.
- Options: (a) add the ICU crates to `forbidden` in the V5 step: one line, but V5 walks `-e normal,build`, and ARQ-107 says normal dependencies; (b) a new step in the `policy` job, in `SC1`.
- Decision: (b). `SC1` owns it because it adds the crates and edits `ci.yml` once.
- The step reads its crate list from the V5 table, as the V12 step does, so a new crate needs no second list. `gafira-testkit` is left out: it is dev-only support, not a layer 0 to 2 crate.
- Per crate: `gafira-i18n`, `icu_decimal`, `icu_datetime` and `icu_plurals` must be absent from the normal dependency tree, and `Locale` must have 0 matches. `icu_segmenter` stays allowed in `gafira-text`.
- `verify` of `SC1` runs the negative control: add `icu_plurals` to one of those crates, and the step fails.

### P10. Input alpha in the ARQ-131 means

- Coordinator's choice, in `SG3`. Not in the spec yet: see SP16.
- Input alpha weights the mean: each side uses `Σ(a × c) / Σa`, with `c` the unpremultiplied colour in linear light (ARQ-117, D-44). A fully transparent pixel adds no colour.
- If `Σa` is 0 on one side, that side counts as having no pixels: not reliable, no NaN (ARQ-131).
- The 1 % and 95 % area rule still counts `mask` pixels. Input alpha does not change it.

## Critical code

Hand mutants apply on all of these. T4 at close covers every crate in the table.

| Delivery | Critical | Not critical |
|---|---|---|
| `CL1` | Lab conversion, ΔE formulas (ARQ-122) | None |
| `SC1` | None; config only, checked by V5, V12, the ARQ-107 step and `cargo deny` | All |
| `TX1`..`TX4` | Range checks, FD-46 search bounds, curve geometry, overflow and `.notdef` flags | Benches |
| `RS1`..`RS3` | Length and overflow checks, premultiply, mip levels, blend formulas, transition ends | None |
| `RS4`..`RS6` | Parameter checks, `roi`, alpha preservation, dHash threshold | Benches |
| `AU1`..`AU4` | `process` totality, subnormals, offset arithmetic, peak ceiling, output length | Benches |
| `SG1`..`SG3` | Size checks, ARQ-131 bounds, IoU, cancellation | Benches, scene generators' visuals |

T4 at close: `gafira-color`, `gafira-text`, `gafira-raster`, `gafira-audio`, `gafira-segment`, `gafira-testkit`.

## Open questions (owner)

None open. O1 is in the `engine-contracts` design.

| # | Result |
|---|---|
| O2 | The owner chose (a): CI re-enabled, run by `workflow_dispatch`. The first run on `develop` (41373916250) failed only on Windows golden paths. `fix/golden-windows-paths` then passed run 41375746735 and is merged in 3b700dc, so `TX3` no longer waits on it |
| O3 | The owner chose (a): D-40 and D-41. The filter mechanism and 1 test preset are in the cut |

If XP5 shows ARQ-132 is out of reach, D-20 is the owner's decision, and that becomes a new question then.

## Needs explorer

XP1, XP2 and XP6 are measured; the rest are not. XP5, XP7, XP8 and XP9 share one prototype, so run them together. `E1` to `E5` are the older spikes.

| # | Claim | Check | Blocks |
|---|---|---|---|
| XP1 | Measured, confirmed: see `engine-contracts` | | Nothing |
| XP2 | Measured: the parley 0.12 tree with `rayon` passes `cargo deny` and the V5 script. The banned crates of ARQ-107 are absent by a one-off `cargo tree` grep; `SC1` turns that into a step (P9) | `cargo deny` exit 0 on the 3 targets; V5 script exit 0 with the 4 rows; builds on 1.93.1; non-Linux targets not compiled | Nothing |
| XP3 | E4's subsets can be regenerated with the same results: sizes, advances, Arabic joining, Manrope `liga`. Inputs were not kept, and upstream Manrope returned 404 | `pyftsubset` from pinned upstream files | `TX1` |
| XP4 | `bench_shadow_1080` < 100 ms single-threaded at radius 40 with a direct kernel. Near 120 taps × 2 passes × 2 M pixels, this looks tight. An IIR blur likely breaks the ARQ-120 tile match; a three-box blur must hold σ within ±2 % | Prototype direct and three-box on one alpha channel; compare σ and time | `RS4` (provisional) |
| XP5 | Own SLIC, GMM, max-flow and GrabCut reach IoU ≥ 0.90 on every ARQ-132 scene, including low contrast at the floor of ΔE76 6, and 4000 px in < 5 s on D-03 hardware (R-04). This is the largest technical risk of the plan | Prototype on generated scenes, before `SG1` | `SG1` |
| XP6 | Measured: `SC1` with unused dependencies is allowed | No unused-dependency rule in the repo; `cargo deny` result as in XP2 | Nothing |
| XP7 | ARQ-131: uniform noise with no subject gives `reliable` false. A segmenter can split noise into light and dark pixels, and those two means can reach ΔE76 ≥ 5 with an area inside 1 % to 95 % | Same prototype, on several noise seeds. If it reports reliable, the rule goes back to `specifier` | `SG1` |
| XP8 | ARQ-43: the guided filter at 4000 px fits between two token checks in ≤ 100 ms on D-03 hardware. As one pass over the full image, it may not | Same prototype: time the longest stretch between checks, at 4000 px | `SG1` |
| XP9 | ARQ-112: the fraction rises ≤ 0.02 per report on the 1024×768 reference, so at least 50 reports, each with a token check. Max-flow may offer too few natural points | Same prototype: count reports per stage on the flat background, centred subject scene | `SG1` |
| XP10 | ARQ-107: the exact CI command gives 0 `Locale` matches once `gafira-text` uses parley, and the tree check stays exact on the new crates. If parley's API needs that word in our source, the rule collides with `gafira-text` | Run the P9 step on a scratch branch with `parley` and a shaping call; run the negative control. A collision goes to `specifier` | `SC1`, `TX1` |

## Needs specifier

One row is open, and it blocks nothing. It records P10.

| # | ID | Gap and proposed edit | Blocks |
|---|---|---|---|
| SP16 | ARQ-131 | The rule does not say what input alpha does to the mean colours. Proposed: alpha weights the mean, so a fully transparent pixel adds no colour; a side with zero total alpha counts as having no pixels; the area rule still counts `mask` pixels | None |

Closed by `specifier`, all on `develop`:

| # | ID | Result | Commit |
|---|---|---|---|
| SP1 | ARQ-122, ARQ-131 | 34 Sharma pairs ±1e-4, Lab reference values, ΔE76 defined | 71605b9 |
| SP2 | 02 §14.2 | `msg!` and `MessageKey` literals aligned | 6ab29cf |
| SP3 | 02 §4 | `Descriptor` without capabilities | 129793e |
| SP4 | 02 §9.2 | Errors are hand-written | 8eff4c2 |
| SP5 | ARQ-112 | Only `Segmenter::segment`. Reference: flat background, centred subject, 1024×768. D-43 removes audio | 13622a6, 868fbab |
| SP6 | ARQ-131, 02 §4.7 | `Segmentation { mask, reliable }` | 76ae97a |
| SP7 | ARQ-131, ARQ-132 | Low contrast is ΔE76 6 to 10, not the proposed 5 to 10, to keep a margin over the ARQ-131 threshold. D-44 adds linear-light means | 9cfb217, ab55ac6 |
| SP8 | FD-12 (c) | The CPU half is ARQ-119 | 3a3637b |
| SP9 | FD-34 | Preset format, intensity by parameter type, thumbnail path (D-40, D-41) | 0e6738a, 27b3e2b |
| SP10 | FD-35 | Paralela bounded; Brillo and Contorno out of the cut (D-42) | 5e5f903, 92299f8 |
| SP11 | ARQ-126 | Order L, R, C, LFE, Ls, Rs | 3cf0e59 |
| SP12 | 02 §4.17 | Homes of the P5 types; `Cost` defined | a652373 |
| SP13 | ARQ-43 | What `bench_cancel_segment` measures | 1d606e5 |
| SP14 | ARQ-75 | Partial file allowed, with 2 mandatory entries | fb53eca |
| SP15 | ARQ-107 | CI step required until `check-layers` exists | 8d175b3 |

CAL-06 and CAL-15 now live in `01`, and the FV IDs in `05`. That promotion is merged, so no stream carries the rename.

## Next

Run `tdd-apply` for `CL1` beside `D2a` and `D3`, and `explorer` for XP10. `specifier` takes SP16 at any time before `SG3`.
