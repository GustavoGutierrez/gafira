# Design: the rest of layer 2

This plan finishes `gafira-text`, `gafira-raster`, `gafira-audio` and `gafira-segment`, plus the prerequisite `color-lab`, with one writer per worktree. The key move is a scaffold delivery, `SC1`, which lands every shared-file change once. After it, each stream touches only its own crate, so the four crates merge without conflicts.

Evidence: 00 (D-09, D-40 to D-44), 02 §2, §4.17, §9.2, §10, 04, ADR 0005, ADR 0007, ADR 0008, ADR 0010, `specs/history/2026-11-01-layout-core/design.md` (L8, L9), `specs/changes/engine-contracts/design.md`.

## Quick path

1. Finish `CL1`, beside `engine-contracts`. Nothing blocks it.
2. Finish `D2a`, `D2b` and `D3`.
3. Run `SC1` alone.
4. Open one worktree per stream: `TX`, `RS`, `AU`. Open `SG` when `RS1` is on `develop`.
5. Ask the owner O4 before `SG3`, and run `explorer` for XP11 before `SG2`.
6. Merge each delivery after its verify PASS, one at a time. Run T4 per crate at close, one crate at a time.

## Dependency state

State at `develop` b642c1e. `D1` is merged. SP17 and SP18 go with the `AU1` fix round, and SP19 blocks `AU2` and `AU3`. SP20 blocks `RS3a`, and SP21 and SP22 block `RS3b`. O4 blocks only `SG3`; O5 blocks no `RS` delivery (P15).

| Delivery | State | Blocked by |
|---|---|---|
| `CL1` | In progress on `feat/color-lab` | Nothing |
| `D2a`, `D3` | In progress on their branches | Nothing |
| `D2b` | Blocked | `D2a` |
| `SC1` | Blocked | `D2b`, `D3` |
| `TX1` | Blocked | `SC1` |
| `TX2` | Blocked | `TX1` |
| `TX3`, `TX4` | Blocked | `TX2` |
| `RS1` | Blocked | `SC1` |
| `RS2` | Blocked | `RS1` |
| `RS3a` | Blocked | `RS2`, SP20 |
| `RS3b` | Blocked | `RS3a`, SP21, SP22 |
| `RS4` | Blocked | `RS3b` |
| `RS5` | Blocked | `RS4` |
| `RS6` | Blocked | `RS5`, `CL1` |
| `AU1` | Blocked | `SC1` |
| `AU2`, `AU3`, `AU4` | Blocked | `AU1` |
| `SG1` | Blocked | `RS1` |
| `SG2` | Blocked | `SG1`, XP11 |
| `SG3` | Blocked | `SG2`, `CL1`, O4 |

## Delivery table

Forecast is authored lines, with the x1.4 planning figure in brackets. Every delivery stays under ~700 lines at x1.4. "Merge" is the order onto `develop`; `any` means first PASS, first merged.

| Delivery | Crate | Product IDs | Depends on | Mode | Forecast | Merge |
|---|---|---|---|---|---|---|
| `D1`..`D3` | doc, geom, pathops, layout | ARQ-102, ARQ-112 (on doubles) | see `engine-contracts` | see `engine-contracts` | 1 200 (1 685) | 1; `D1` merged |
| `CL1` | `gafira-color` | ARQ-122 (Lab, ΔE2000, ΔE76) | None | parallel, beside `D1`..`D3` | 280 (390) | any, before `RS6` |
| `SC1` | all four, testkit, shared files | ARQ-01, ARQ-02, ARQ-70 (rows, lints, licences), ARQ-107 (CI step) | `D2b`, `D3` | serial | 180 (250) | 2 |
| `TX1` | `gafira-text` | FD-41 (a, d), ARQ-115, ARQ-116, ARQ-75 (partial file), ARQ-110, ARQ-111 | `SC1` | parallel stream `TX` | 465 (650) | any |
| `TX2` | `gafira-text` | FD-41 (c, e to j) | `TX1` | serial in `TX` | 440 (615) | after `TX1` |
| `TX3` | `gafira-text` | FD-45 (a, c, d, e), FD-46 (a, b, c, e, f), ARQ-14 (`FitBox`) | `TX2` | serial in `TX` | 400 (560) | after `TX2` |
| `TX4` | `gafira-text` | FD-44 (a, c, d, e), FD-43 (a), ARQ-114 (text) | `TX2` | serial in `TX`; merged alone, for `skrifa` (P14) | 350 (490) | after `TX3` |
| `RS1` | `gafira-raster` | ARQ-117, ARQ-121, ARQ-110, ARQ-111 | `SC1` | parallel stream `RS` | 450 (630) | any, before `SG1` |
| `RS2` | `gafira-raster` | ARQ-119, FD-12 (c, CPU) | `RS1` | serial in `RS` | 340 (475) | after `RS1` |
| `RS3a` | `gafira-raster` | ARQ-118 (`progress`, Cut, Dissolve, Fade), ARQ-25 (ends), ARQ-110 | `RS2` | serial in `RS` | 420 (590) | after `RS2` |
| `RS3b` | `gafira-raster` | ARQ-118 (Slide, Zoom, Wipe), ARQ-25 (ends), ARQ-110 | `RS3a` | serial in `RS` | 450 (630) | after `RS3a` |
| `RS4` | `gafira-raster` | ARQ-120, FD-35 (a, d; Paralela only, D-42), ARQ-114 (`bench_shadow_1080`) | `RS3b` | serial in `RS` | 420 (590) | after `RS3` |
| `RS5` | `gafira-raster` | FD-33 (a, d to h), except colours detected by ΔE | `RS4` | serial in `RS` | 480 (670) | after `RS4` |
| `RS6` | `gafira-raster` | ARQ-122, FD-26, FD-28 (b), FD-33 (detected colours), FD-34 (a, b; D-40, D-41), ARQ-114 (`bench_dhash_12mp`) | `RS5`, `CL1` | serial in `RS` | 480 (670) | after `RS5` |
| `AU1` | `gafira-audio` | ARQ-123, ARQ-124, ARQ-129, FV-10 (a, d), ARQ-110, ARQ-111, `params()` (P13) | `SC1` | parallel stream `AU` | 480 (670) | any |
| `AU2` | `gafira-audio`, testkit test | ARQ-125, ARQ-23, CAL-15, FV-20 (c) | `AU1` | serial in `AU` | 400 (560) | after `AU1` |
| `AU3` | `gafira-audio` | ARQ-126, ARQ-127, FV-10 (b), FV-20 (d) | `AU1` | serial in `AU` | 480 (670) | after `AU2` |
| `AU4` | `gafira-audio` | ARQ-128, FV-04 (b) | `AU1` | serial in `AU` | 350 (490) | after `AU3` |
| `SG1` | `gafira-segment` | ARQ-130 (shape, sizes, hints), ARQ-110 | `RS1` | parallel stream `SG` | 450 (630) | after `RS1` |
| `SG2` | `gafira-segment` | ARQ-130 (seeds, zones), FD-38 (reduction) | `SG1`, XP11 | serial in `SG` | 480 (670) | after `SG1` |
| `SG3` | `gafira-segment`, testkit | ARQ-131, ARQ-132, FD-32 (e), FD-38 (a, b, c), ARQ-43, CAL-06, ARQ-111, ARQ-112 | `SG2`, `CL1`, O4 | serial in `SG` | 490 (685) | after `SG2` |

Total after `engine-contracts`: 8 285 lines (x1.4: 11 600). The largest delivery is 490 lines (x1.4: 685).

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
| `Cargo.toml` (root) | `SC1`, `TX4` (P14) | A later dependency is a stop: mark that delivery serial and merge it alone. Keep both lines |
| `Cargo.lock` | `SC1`, `TX4` (P14) | Never hand-merge. Take `develop`'s file, run `cargo check --workspace --all-targets` to regenerate, then the gates with `--locked` |
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
| `EffectCx` | `gafira-raster` | `EffectCx { pub scale: f32, pub tile: Option<TileWindow> }` (P16). No pool (L8). No token: `gafira-render` checks per tile (02:1145) |
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
| Every test font | XP3: upstream repository and commit, OFL-1.1, subset command (ARQ-115) |
| `assets/splash_screen.jpg` | D-09, "Procedencia de la imagen": origin, tool, licence basis, AI notice, copyright note, provenance, similarity check. The similarity check stays VERIFICAR (pending item 6 of `00`) |

ARQ-02 bans `std::fs` under `--all-targets`, so `gafira_testkit::fonts` exposes the fonts with `include_bytes!`. The ARQ-115 check is a testkit integration test, which may read files: testkit has no `clippy.toml`. The same test asserts the splash entry, so the second mandatory entry cannot go missing.

Subset recipe, measured by XP3. It gives the same behaviour as E4, not the same bytes, so nothing may compare against E4's files.

| Item | Value |
|---|---|
| Fonts | Inter, Manrope, Noto Sans Arabic, Noto Emoji, all OFL-1.1 |
| Source | `google/fonts`, commit `6c1dbe27dcf7d817d4b62cec033ba77fa8e2a5ec` |
| Tool | `pyftsubset`, fonttools 4.51.0, `--layout-features='*' --glyph-names` |
| Latin set | `U+0020-007E,U+00A0-017F,U+2010-2022,U+2026,U+20AC` |
| Arabic set | `U+0020-007E,U+0600-06FF,U+200C-200F` |
| Emoji set | `U+1F600,U+1F469,U+1F4BB,U+200D,U+FE0F` |

Two rules for `TX1` with parley 0.12.0, which loads all four:

- Turn a feature off with the CSS form `"liga" 0`. `liga=0` is silently ignored.
- Resolve families by registration id, not by name.

`gafira-text` must not use `StyleProperty::Locale`: it is the only parley API that needs that word, and ARQ-107 forbids it (XP10). Only `specifier` can relax that.

### P8. Benchmarks that need a clock

`std::time::Instant::now` is banned under `--all-targets` in domain crates, so `bench_cancel_segment` cannot time cancellation directly. `SG3` benches the longest work unit between two checkpoints with plain `criterion`. That bounds ARQ-43 from above, and ARQ-43 now says so. The input is a 4000 px image. The name stays. XP8 checks that the guided filter fits.

### P9. The ARQ-107 CI step

- Confirmed by XP10.
- Options: (a) add the ICU crates to `forbidden` in the V5 step: one line, but V5 walks `-e normal,build`, and ARQ-107 says normal dependencies; (b) a new step in the `policy` job, in `SC1`.
- Decision: (b). `SC1` owns it because it adds the crates and edits `ci.yml` once.
- The step reads its crate list from the V5 table, as the V12 step does, so a new crate needs no second list. `gafira-testkit` is left out: it is dev-only support, not a layer 0 to 2 crate.
- Per crate: `gafira-i18n`, `icu_decimal`, `icu_datetime` and `icu_plurals` must be absent from the transitive tree of `cargo tree -e normal`, and `Locale` must have 0 matches. One banned dependency therefore fails every crate that depends on it. `icu_segmenter` stays allowed in `gafira-text`.
- `verify` of `SC1` runs the negative control: add `icu_plurals` to one of those crates, and the step fails.

### P10. Input alpha in the ARQ-131 means

- Coordinator's choice, in `SG3`, now in ARQ-131 (SP16).
- Input alpha weights the mean: each side uses `Σ(a × c) / Σa`, with `c` the unpremultiplied colour in linear light (ARQ-117, D-44). A fully transparent pixel adds no colour.
- If `Σa` is 0 on one side, that side counts as having no pixels: not reliable, no NaN (ARQ-131).
- The 1 % and 95 % area rule still counts `mask` pixels. Input alpha does not change it.

### P11. The shadow blur

- Measured by XP4, for `RS4`.
- Options: (a) three-box: refuted, σ is off beyond ±2 % below σ 10; (b) direct separable kernel, dot-product form: 137 ms, refuted; (c) direct separable kernel with tap-outer loops.
- Decision: (c), truncated at 3σ, with a 3σ `roi`.
- Evidence: 46 to 52 ms single-threaded at 1920×1080 under load; σ −0.6 % to −1.25 %; tile match max diff 0.
- Not measured: RGBA and non-x86. `RS4` runs `bench_shadow_1080` for that.

### P12. The segmentation pipeline

- Measured by XP5, XP7, XP8 and XP9, on one prototype. This is the starting point for `SG1` to `SG3`.

| Part | Value |
|---|---|
| Superpixels | Own SLIC, about 1530 |
| Colour model | 5-component GMM |
| Cut | Dinic max-flow, up to 5 iterations |
| Initialisation | Border band |
| Upscale | Guided filter, radius 8 |

Binding rules for the `SG` deliveries:

- Post-processing runs in serial chunks, and graph adjacency uses a hash table. ARQ-43 holds only with both. XP11 covers the table.
- The fraction follows a work model, not time share (ARQ-112, XP9).
- Start from `gamma` about 500 or GMM regularisation about 25 (02 §4.7). Both were tuned on the gate scenes, so passing ARQ-132 does not prove they generalise. O4 covers the known gap.

### P13. `AudioProcessor::params()`, amendment to `AU1`

- Context: the C12 decision (`specs/history/2026-11-02-engine-contracts/design.md`) requires a test per compiled schema. `AudioProcessor` (02 §4.9) has no way to reach one, so nothing can run `validate_definition` on the table of 02 §4.17.5. C12 lives in that design and in the "how to add" lists of 02 §4.4 and §4.13, not in the text of ARQ-110, and the §4.9 list lacks the step.
- Options: (a) a required trait method, as in `Effect` (02 §4.4) and `Transition` (02 §4.13); (b) a static per-type schema, `fn schema() -> ParamSchema` with `where Self: Sized`, or an inherent function.
- Decision: (a), as a required method with no default body:

```rust
pub trait AudioProcessor: Describe + Send {
    fn params(&self) -> &ParamSchema;
    fn prepare(&mut self, sample_rate: u32, max_block: usize, channels: u16);
    fn process(&mut self, block: &mut AudioBlock<'_>, params: &ParamSnapshot);
    fn latency(&self) -> u32 { 0 }
    fn tail(&self) -> u32 { 0 }
    fn reset(&mut self);
}
```

- Why: `ProcessorChain` holds `Box<dyn AudioProcessor>` (`crates/gafira-audio/src/chain.rs` on `feat/gafira-audio`) and 02 §11 lists an `AudioRegistry` open to plugins. Option (b) cannot be called through `dyn`, and a new processor that forgets it still compiles. With (a) the compiler enforces the method and only the test can be forgotten, which is the risk C12 accepts.
- `process` never calls `params()`, so ARQ-23 is unaffected. `params()` only borrows.

Storage:

- Each processor keeps a `schema: ParamSchema` field. `ParamSchema` holds a `Vec` (`crates/gafira-doc/src/contract/params.rs`), so it cannot be a plain `static` like `DESCRIPTOR`.
- A `LazyLock` is rejected: ARQ-31 excepts only the logging `OnceLock`, and a field needs no global at all. Making `ParamSchema` const-constructible is rejected: it changes `gafira_doc::contract` for one stream.
- Cost: `Gain::new`, `Fade::new` and `Pan::new` stop being `const fn`. Nothing on `feat/gafira-audio` uses them in a const context. Build processors off the audio thread.
- One source for the numbers: `dsp::Param` gains `pub(crate) fn def(&self, label: MessageKey, animatable: bool) -> ParamDef`, called in `new` before any `read`, because `value` holds the default until then. The ranges are not written twice.
- `ProcessorChain` returns an empty schema. It owns no parameter, and merging its stages would repeat ids: two `Fade` stages give `BadDefinition`. Its stages keep their own schemas.

| Processor | Id | `ParamKind` | `animatable` | Label (proposed) | Ground |
|---|---|---|---|---|---|
| `Gain` | `volume` | `Float { min: 0.0, max: 200.0, default: 100.0 }` | `true` | `audio.gain.volume` | Stated: FV-14 lists volume as animatable, 02 §3 has `Clip.volume: Animatable<f32>`, FV-10 has an editable volume graph |
| `Fade` | `duration` | `Float { min: 0.0, max: f64::MAX, default: 0.0 }` | `false` | `audio.fade.duration` | Inferred: ARQ-124 reads it once per ramp and a change waits for the next `reset`, so a keyframed value could not act. FV-14 does not list it |
| `Pan` | `pan` | `Float { min: -1.0, max: 1.0, default: 0.0 }` | `false`, provisional | `audio.pan.pan` | Unstated: see below |

Not stated in the specs, sent to `specifier` as SP18:

- **`animatable`.** Neither FV-10 nor the table of 02 §4.17.5 has it. FV-10 only supports `volume`, through its graph; the list is in FV-14.
- **`pan`.** FV-14 lists clip parameters, without pan, but ends with "parámetros de efectos", which could cover it. Pan is a track strip control (FV-20), and neither `Clip` nor `Track` has a pan field (02 §3), so a keyframe has no home. `false` costs nothing: the flag is not persisted and can flip later.
- **`duration` bound.** 02 says "sin máximo", but `is_consistent` requires a finite `max`, so `f64::INFINITY` gives `BadDefinition`. `f64::MAX` admits every finite value, which keeps the meaning, and `Fade::new` already uses it. A schema-generated control still needs a real limit, such as the clip length.
- **Labels and units.** 02 gives no label key, and ARQ-98 requires one per `ParamDef`. The keys above follow `Descriptor`'s `audio.gain`. `ParamDef` has no unit field, so the % and s of the table have no place in the schema. No catalogue exists yet, so the keys get their entries with `gafira-i18n`.

`AU1` obligations, in its fix round:

- A table test in `crates/gafira-audio/tests/` over `Gain`, `Fade` in both directions, `Pan` and `ProcessorChain`, called through `dyn AudioProcessor`: `validate_definition` is `Ok`, and id, bounds, default and `animatable` equal the table above.
- Add the method to the `Op` double in `tests/chain.rs`.
- This closes W2 of the `AU1` verify report. About 90 lines, mostly test. `AU1` is already 2 094 lines against 670 forecast (`specs/changes/gafira-audio/verification.json` on its branch), so this is not a split trigger, but the forecast no longer holds.

### P14. `skrifa` as a direct dependency of `gafira-text`

- Context: `gafira-text` reaches `skrifa` only through `parley_engine`, and `parley` does not re-export it, so production code cannot call it. 02 §9 already lists it and its licence.
- Decision: when `TX4` needs a real outline, its first commit makes `skrifa` a direct dependency.

| File | Change |
|---|---|
| `Cargo.toml` (root) | `skrifa = { version = "0.44", default-features = false, features = ["std"] }` |
| `crates/gafira-text/Cargo.toml` | `skrifa.workspace = true` |
| `Cargo.lock` | Regenerated by the P2 rule |

- `0.44` is the requirement of `parley_engine` 0.12.0, and the lock holds 0.44.0. A `parley` bump moves this line too: `multiple-versions = "warn"` in `deny.toml` would not stop a second copy. `std` is required, because `skrifa` fails to compile without `std` or `libm`.
- Serial per P2: a root `Cargo.toml` change is a stop. `TX4` merges alone, against all streams, not only inside `TX`.

Does it need to come sooner? No, by delivery:

| Delivery | Needs `skrifa`? | Evidence |
|---|---|---|
| `TX1` | No | Apply-progress deviation 1 and verify ruling on `feat/gafira-text`: FD-41 (a) is accepted through a test-only `glyf` reader, with extraction left to `TX4`. Verify S4 points at `TX4`. `layout.rs` uses only `parley` |
| `TX2` | No, provisional | "Fuente sin métricas" (verify W6) reads parley line metrics. FD-41 (g) picks the font that covers most code points: `parley_engine` tracks best coverage in `shape/cluster.rs`, and `parley::fontique` exposes `QueryFont::charmap` as a fallback. That parley meets the rule across a fallback list is not proved: XP12 |
| `TX3` | No | FD-45 and FD-46 use layout measures only |
| `TX4` | Yes, for FD-43 (a) | FD-44 (a, c, d, e) work from advances, and FD-44 (b) is out of the cut. FD-43 (a) is only meaningful on a stroked real outline: FD-43 Técnica and 02 §2.1 give `gafira-text` the contours |

If XP12 fails, the same step moves to the first commit of `TX2`, with the same serial rule.

### P15. The `RS3` transitions

- Context: items 1 to 9 of `## RS3` in `specs/changes/gafira-raster/apply-progress.md` on `feat/gafira-raster`. ARQ-118 gives the maths. C12 needs a schema per transition, and the rest below is unstated.
- Status: items 3 to 7 and 9 are decided here. Items 1 and 2 are proposals until `specifier` writes them (SP20 to SP22). Item 8 is provisional until O5.

Split: `RS3` becomes `RS3a` (contract, shared mix, Cut, Dissolve, Fade) and `RS3b` (Slide, Wipe, Zoom). `RS1` and `RS2` came in at about 3 times their forecast (`verification.json`: 1 696 against 450, 1 068 against 340), so one delivery of six transitions would not stay reviewable. `Mode` stays serial: both edit `lib.rs`, `error.rs` and `transitions/`.

**Shape.** `Transition`, as in 02 §4.13, lives in `gafira_raster::transitions`, beside `GpuTransition`, which P5 already puts in this crate. `gpu()` keeps its default `None`. Each transition is one file, and each keeps a `schema: ParamSchema` field, as in P13.

**Order inside `run_cpu`,** in one shared guard, before any write (ARQ-110):

1. `cx.scale` is finite.
2. `progress` is finite.
3. `self.params().resolve(p)`.
4. Formats (item 6).
5. Dimensions.
6. `RgbaF32` pixels of `from` and `to` are finite.
7. Size 0 gives `Ok`.
8. `p` is clipped. `p == 0` copies `from` and `p == 1` copies `to`.

Step 1 holds even though no transition reads `scale`: ARQ-110 checks every number, also those that do not take part in the calculation. A non-finite `scale` gives `Buffer(BufferError::NonFinite { field: "cx.scale", index: 0 })`. `Transition::run_cpu` and `Effect::run_cpu` check it the same way, as the first step, so `RS4` inherits the rule. Only non-finite values are rejected: no row gives `scale` a range (02 §4.17.1, "Rangos").

With step 8, no sampling or colour conversion runs at the ends, so ARQ-118 and ARQ-25 hold byte-exact for all six, Zoom included. Near the ends, ARQ-118 binds only Dissolve, Fade and Wipe with `s = 1`: at `p` 0.0001 and 0.9999 they stay within 1/255 of their end. Cut, Slide, Zoom and Wipe with another softness have no such bound: Slide moves whole pixels, and Zoom measured up to 4/255 (`RS3b` verify round 1, W3).

#### Item 1, schemas (proposal)

02 §4.13 says only "dirección, color, suavidad", and FV-06 gives the catalogue. Stated facts: Fade goes to "negro, blanco o color", Slide has 4 directions, Wipe has 4 plus circular with softness in `[0, 1]`, and Zoom is "acercar o alejar".

| Transition | Id | `ParamKind` | Default | Label |
|---|---|---|---|---|
| Cut, Dissolve | None | Empty schema | | |
| Fade | `color` | `Color` | Opaque black, `Srgb` | `transition.fade.color` |
| Slide | `direction` | `Choice`: `left`, `right`, `up`, `down` | `left` | `transition.slide.direction` |
| Zoom | `direction` | `Choice`: `in`, `out` | `in` | `transition.zoom.direction` |
| Wipe | `direction` | `Choice`: `right`, `left`, `down`, `up`, `circular` | `right` | `transition.wipe.direction` |
| Wipe | `softness` | `Float { min: 0.0, max: 1.0 }` | 0 | `transition.wipe.softness` |

- Black and white are values of `color`, not a separate choice: they are UI shortcuts.
- Fade converts `color` once per call: to sRGB bytes, then premultiplied, for `Rgba8`, and through `Color::to_linear_srgb`, then premultiplied, for `RgbaF32`, the buffer meanings used since `RS1`. Its own alpha is kept.
- Choice labels are `transition.direction.<id>`, shared by the three.
- `animatable` is `false` on all, by the P13 reasoning: FV-14 does not name transition parameters, and the flag is not persisted. `TransitionInstance.params: AnimatableParams` (02 §4.13) would allow `true` later.

#### Item 2, direction conventions (proposal)

ARQ-118 fixes neither. The `direction` id always names the way the moving edge travels, with `y` down.

| Transition | Id | Rule |
|---|---|---|
| Slide | `left` | `to` enters at the right edge. `lado` is the width. Screen `x` shows `to(x − d)` when `x ≥ d`, else `from(x)` |
| Slide | `right` | `to` enters at the left edge. `lado` is the width |
| Slide | `up` | `to` enters at the bottom edge. `lado` is the height |
| Slide | `down` | `to` enters at the top edge. `lado` is the height |
| Wipe | `right` | `u = (x + 0.5) / W` |
| Wipe | `left` | `u = 1 − (x + 0.5) / W` |
| Wipe | `down` | `u = (y + 0.5) / H` |
| Wipe | `up` | `u = 1 − (y + 0.5) / H` |
| Wipe | `circular` | Centre `(W / 2, H / 2)`, half diagonal `√(W² + H²) / 2`, so `to` opens from the centre |

- Wipe mask `m` is the weight of `to`: ARQ-118 gives 1 for `u < p`, and `p = 1` leaves nothing of `from`.
- Slide replaces pixels. It does not composite `to` over `from`, because a translucent `to` would otherwise jump at `p = 1`.
- `d = redondeo((1 − p) × lado)` in `f64`, half away from 0.

#### Item 3, Zoom

- Pivot: the image centre, `(W / 2, H / 2)`, in continuous coordinates, so an odd side needs no rounding.
- Scale: linear in `p`, as ARQ-118 lists only the ends. `in`: `from` `1 + p`, `to` `(1 + p) / 2`. `out`, the mirror (proposal, SP22): `from` `1 − p / 2`, `to` `2 − p`.
- Sampling: for a screen pixel, `sx = cx + (x + 0.5 − cx) / scale − 0.5`, and the same in `y`.
- Filter: bilinear, 4 taps per image, on premultiplied channels. Nearest shimmers during a slow zoom, and bicubic overshoots, which breaks `c ≤ a`.
- Outside the image: clamp to the edge pixel. `to` at scale 0.5 leaves a ring, and ARQ-118 does not fill it. ARQ-120's transparent black would make two opaque clips translucent mid-way, and Zoom would be the only transition doing so. Clamp keeps opaque inputs opaque, and it is the usual GPU sampler mode for FV-06 (b).
- No mip level is used, so the `to` side aliases slightly near scale 0.5. This is accepted for a blend lasting under 5 s.

#### Item 4, ids and display

| Transition | `Descriptor.id`, also `transition_id` | Name key | `version` |
|---|---|---|---|
| Cut | `transition.cut` | `glossary.transition_cut` | 1 |
| Fade | `transition.fade` | `glossary.transition_fade` | 1 |
| Dissolve | `transition.dissolve` | `glossary.transition_dissolve` | 1 |
| Slide | `transition.slide` | `glossary.transition_slide` | 1 |
| Zoom | `transition.zoom` | `glossary.transition_zoom` | 1 |
| Wipe | `transition.wipe` | `glossary.transition_wipe` | 1 |

The ids name the concept, not the crate, because `transition_id` is persisted in `TransitionInstance`. Audio's `audio.gain` is the same two-part form. `version` rises when the output changes.

#### Item 5, `HandleReq`

| Transition | Value | Why |
|---|---|---|
| Cut | `None` | Instantaneous in FV-06. No extra frames are shown |
| Fade, Dissolve, Slide, Zoom, Wipe | `Both` | `handles` does not receive `TransitionAlign`, so the value must hold for `Centered`, `Start` and `End` |

No catalogue transition uses `From` or `To`. A short clip still works: the builder repeats the edge frame and warns (02 §4.13).

#### Item 6, formats

- `Rgba8` and `RgbaF32` only, and `from`, `to` and `dst` must share the format. No transition converts: that belongs to the render graph.
- `Alpha8` gives `UnsupportedFormat` for all six. Frames are 8-bit or `f32` (02 §6.2), `Alpha8` is mask data with no caller here, and Fade has no meaning on it. ARQ-118's "por formato" therefore means two formats (SP24).
- A mix gives `FormatMismatch`, naming the first side that differs from `from`.

#### Item 7, `EffectError`

In `crates/gafira-raster/src/error.rs`, shared with `RS4`:

```rust
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectError {
    Buffer(BufferError),
    Param(ParamError),
    SizeMismatch { field: &'static str, expected: (u32, u32), actual: (u32, u32) },
    FormatMismatch { field: &'static str, expected: PixelFormat, actual: PixelFormat },
    UnsupportedFormat { format: PixelFormat },
}
```

- `Buffer` and `Param` have `From`, and `to_message` delegates (02 §7 error rules, ADR 0012).
- `field` is `to` or `dst`; `from` is the reference.
- A non-finite `progress` is `Buffer(BufferError::NonFinite { field: "progress", index: 0 })`, `cx.scale` the same with its own field, and a pixel the same with `field` `from` or `to`. A new variant would need the key `error.raster.non_finite`, which `BufferError` already holds with other arguments.
- New keys: `error.raster.size_mismatch`, `error.raster.format_mismatch`, `error.raster.unsupported_format`. Formats go in `Text`, sizes in `Count`.

#### Item 8, the mixing space

- **Decided:** Dissolve, Fade, Wipe and Zoom mix premultiplied RGBA directly, alpha included. It is the only form where the result keeps `c ≤ a`, and it matches `Color` interpolation (02 §3.4). ARQ-117's "sin premultiplicar" binds colour operations, not compositing, so mutant 10 tests the premultiplied path.
- **Open:** O5, the same question as W1 of RS2.
- **It is one switch in one function.** Every mix goes through `weighted(samples: &[(Pixel, f64)]) -> Pixel`, in one private module with an arm per format. Dissolve, Fade and Wipe pass 2 samples, and Zoom passes 8. The space is `const MIX_SPACE: MixSpace` in that module. Cut and Slide only copy, so no answer touches them.
- A sample of weight exactly 1 returns its pixel unconverted, so `s = 0` Wipe and Fade at `p = 0.5` stay exact.

| O5 answer | `Rgba8` | `RgbaF32` | Changes in `RS3` |
|---|---|---|---|
| (a) Linear light | Decode, mix, encode | As stored | None: this is the provisional build |
| (b) Encoded sRGB everywhere | As stored | Encode, mix, decode | The constant, and the midpoint test rows |
| (c) Each format as stored | As stored | As stored | The constant, and the `Rgba8` midpoint rows |

- `RS3a` and `RS3b` can proceed before O5, on (a). Exact ends, ids, schemas, errors and geometry hold under every answer. Only test rows at mid `p` on the affected format depend on it, so they sit in one table beside the constant.
- (a) is provisional because it follows what 02 and 05 state: the same graph in both modes to avoid divergence (02 §6.2), a linear working space (FV-19), linear `Color` interpolation (02 §3.4), and linear mips (ARQ-121).
- `Transfer` in `mip.rs` moves to a `pub(crate)` module for this. `blend_u8` does not change in `RS3`.
- **Cost of (a), measured (XP13):** 154 to 156 ms per 1080p `Rgba8` Dissolve in `--release`, on a scratch export, 5 runs at `p` 0.37 (`RS3a` verify round 1). That is about 74 ns per pixel, and about 19 times the 8 ms share of 02 §6.2. `f32` frames and the ends are not affected.

What would bring it down. Every figure in this table is an estimate, not a measurement, and assumes the same machine:

| Step | Estimate at 1080p, 1 thread | Note |
|---|---|---|
| Today | 154 to 156 ms (measured) | `weighted_u8_in` unpremultiplies, then `Transfer::encode` runs `AnalyticTransform` for 3 channels per pixel |
| Encode lookup table | 40 to 70 ms | Replaces the per-pixel transform, which should be the largest cost. It needs a test against `encode` over a dense input sweep, within 1/255: sRGB is steep near 0, so a uniform table needs enough entries there |
| Table plus opaque fast path | 20 to 40 ms | With `a = 255` there is no unpremultiply and no alpha division. Mid-`p` mixes of opaque video are the common case |
| The same on 4 threads | 5 to 12 ms | By rows, with ARQ-111 byte identity across pools. `gafira-raster` declares no `rayon` today, so this is a `Cargo.toml` change in that crate |
| The same at quarter-area viewport size | 5 to 10 ms, 1 thread | 02 §6.2 runs Preview effects at viewport resolution, so 1080p is the worst case, not the usual one |

No single step reaches 8 ms by this estimate; the table, the fast path and either threads or viewport size together should. The work stays inside `weighted_u8_in` and `Transfer`, so it is a later fix and changes no contract. `RS3a` keeps the XP13 bench, so these estimates can be replaced by numbers.

#### Item 9, rounding

- `Rgba8`: the weighted sum in `f64`, then `(v + 0.5).floor()` clamped to `[0, 255]`, the same rule as `blend_u8`. Every channel uses the same weights and rounding is monotonic, so `c ≤ a` survives.
- `RgbaF32`: the weighted sum in `f64`, cast to `f32`. Admitted values beyond `[0, 1]` (02 §4.17.4) stay finite, because a convex sum of `f32` values cannot overflow in `f64`.
- `p` enters as `f64::from(p)`.

**Limit:** `Transition` has no `roi`, and Slide, Wipe and Zoom read the whole image, so `run_cpu` takes whole frames, never tiles (SP23).

### P16. Where a tile sits in its image

- Context: W4 of `specs/changes/gafira-raster/verify-report.md`. `EffectCx` holds only `scale` (`crates/gafira-raster/src/view.rs:294`). `Texture::run_cpu` centres Viñeta on `src` (`effects/texture.rs:283`, `:301`), so a tile differs from the whole image by 47/255, against ARQ-120 "Ejecutar por tiles con `roi` coincide con la imagen entera". `run_cpu` has no equivalent input: `src` and `dst` have the same size (`check_io`, `effects/pointwise.rs:40`) and carry no position.
- The tile shape is already fixed by the tests (`effects/shadow.rs:977`, `effects/texture.rs:765`): the caller cuts `roi(out)`, intersected with the image, runs on that crop, and keeps `out` from the result. So the position that matters is that of `src`, not of `out`.
- Decision: `EffectCx` gains an optional window. `None` means `src` is the whole image.

```rust
/// Where `src` and `dst` sit in the full image, in the pixels of `src`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileWindow {
    /// Full image width.
    pub image_width: u32,
    /// Full image height.
    pub image_height: u32,
    /// Column of the first pixel of `src` in the full image.
    pub x: u32,
    /// Row of the first pixel of `src` in the full image.
    pub y: u32,
}

/// Context handed to an effect. Plain data: it does not validate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectCx {
    /// Logical-to-physical pixel ratio (ARQ-120).
    pub scale: f32,
    /// Set when `src` is a cut of a larger image. `None`: `src` is the whole image.
    pub tile: Option<TileWindow>,
}

impl EffectCx {
    /// A whole-image run.
    #[must_use]
    pub fn whole(scale: f32) -> Self {
        Self { scale, tile: None }
    }
}
```

- Whole-image default: `EffectCx::whole(scale)`, with `tile: None`. It needs no image size, so `roi`, `cost_hint`, `FilterPreset::apply` and every transition call stay as they are. `Some` with `x = y = 0` and the size of `src` gives the same output as `None`, bit for bit.
- `gafira-render`, per tile: `x`, `y` are the origin of the `roi` after intersecting it with the image, not of `out`. On a mip level or a reduced copy, image size and origin are in that level's pixels, the same as `src`.

| Who | Must do |
|---|---|
| `Texture` | Read the window through one helper, `(x, y, image_w, image_h)`, which gives `(0, 0, w, h)` for `None`. Viñeta: `x_c = x + (i % w) + 0.5 - image_w / 2`, the same for `y`, `half_diagonal = hypot(image_w, image_h) / 2`, all in `f64`. `roi` does not change: Viñeta reads only its own pixel. Update the comment at `texture.rs:203` |
| `Texture`, check | After `check_io`, before the empty-image return and before writing: `x + w <= image_w` and `y + h <= image_h`, in `u64`. Otherwise `EffectError::SizeMismatch { field: "cx.tile", expected: (image_w, image_h), actual: (x + w, y + h) }`, saturating to `u32`. This also keeps `half_diagonal` non-zero for a non-empty `src`, so the output stays finite (ARQ-110). Shared as `check_tile` in `effects/pointwise.rs` |
| `Light`, `ColorAdjust`, `WhiteBalance`, `DetectedColors`, `ParallelShadow` | Nothing. They read no position: per-pixel closures ignore the index, and `ParallelShadow` already matches on crops. They ignore `tile` and do not check it |
| `FilterPreset::apply` | Nothing. It hands the same `cx` to each step with same-size buffers (`effects/filter.rs:276`). `dst` is only written by the last step, so a `cx.tile` `Err` from `Texture` leaves it intact |
| Any new effect | If the output depends on the position or the size of the image, it must use the helper and `check_tile`, and add a tile test with that parameter active |
| `Transition` | Nothing. It takes whole frames, never tiles (P15 limit, SP23), and reads only `cx.scale`. It ignores `tile` and does not check it |

- Doc comment of `Effect::run_cpu` (`effects/mod.rs:54`): replace "Runs on a whole image" with "Runs on `src`, a whole image or a tile described by `cx.tile`".
- Tests for the fix: `arq_120_texture_tiles_with_roi_match_the_whole_image` gets a Viñeta case with a window, in `Rgba8` and `RgbaF32`, including a tile touching no corner and one touching the top-left; `Some` at origin equals `None`; a window that does not fit gives the `Err` with `dst` unchanged. The 41 `EffectCx { scale }` literals in `src` and `benches` become `EffectCx::whole(scale)`, and `lib.rs` exports `TileWindow`. Estimate: 40 lines of code, 80 of tests, plus the mechanical literals.
- Alternatives considered:
  - Four flat fields, 0 by default: a forgotten size reads as a 0x0 image and gives NaN in Viñeta; a whole-image run must know its size twice.
  - State in 02 that Viñeta runs on whole images: Textura could not be tiled when Viñeta is active, and export uses parallel tiles (02 §6.7 row `bench_export_png_1080`). It also leaves the next position-dependent effect with the same gap.
  - New arguments on `run_cpu`: changes the signature of 02 §4.4 and of every implementer for one reader. `EffectCx` is the context already passed for this.
- Consequences: no implementers exist outside `gafira-raster`, so the change stays in one crate. No ADR: the contract is `EffectCx`, inside layer 2. `GpuEffect::bind` takes only `p`, so a tiled GPU Viñeta will need this window too; no `GpuEffect` exists in this cut, and it is left to the delivery that adds one. Spec changes: SP26.

## Critical code

Hand mutants apply on all of these. T4 at close covers every crate in the table.

| Delivery | Critical | Not critical |
|---|---|---|
| `CL1` | Lab conversion, ΔE formulas (ARQ-122) | None |
| `SC1` | None; config only, checked by V5, V12, the ARQ-107 step and `cargo deny` | All |
| `TX1`..`TX4` | Range checks, FD-46 search bounds, curve geometry, overflow and `.notdef` flags | Benches |
| `RS1`..`RS3b` | Length and overflow checks, premultiply, mip levels, blend formulas, transition ends | None |
| `RS4`..`RS6` | Parameter checks, `roi`, alpha preservation, dHash threshold | Benches |
| `AU1`..`AU4` | `process` totality, subnormals, offset arithmetic, peak ceiling, output length | Benches |
| `SG1`..`SG3` | Size checks, ARQ-131 bounds, IoU, cancellation | Benches, scene generators' visuals |

T4 at close: `gafira-color`, `gafira-text`, `gafira-raster`, `gafira-audio`, `gafira-segment`, `gafira-testkit`.

## Open questions (owner)

O4 and O5 are open. O1 is in the `engine-contracts` design.

| # | Result |
|---|---|
| O2 | The owner chose (a): CI re-enabled, run by `workflow_dispatch`. The first run on `develop` (41373916250) failed only on Windows golden paths. `fix/golden-windows-paths` then passed run 41375746735 and is merged in 3b700dc, so `TX3` no longer waits on it |
| O3 | The owner chose (a): D-40 and D-41. The filter mechanism and 1 test preset are in the cut |

Open, for `SG3`:

| # | Question | Options and tradeoffs | Recommendation |
|---|---|---|---|
| O4 | Q-45: a textured background at ΔE76 6 is outside ARQ-132. It passes at `gamma` 50 and fails at 500. Is that gap accepted, and what ΔE76 floor do textured scenes get? Affects D-20 and the `SG3` scene list | (a) Accept the gap and keep the textured scenes as measured: no rework, weak on subtle textures. (b) Add that scene to the gate: stronger, but neither `gamma` value passes everything, regularisation 25 has no result on that scene, and `SG3` may fail. (c) Retune first: costs a new prototype run, with an unknown result | (a), with the floor set by the textured scenes that pass, recorded as PROPUESTA. Real photos stay a warning (pending item 9) |

Open, no `RS` delivery waits for it:

| # | Question | Options and tradeoffs | Recommendation |
|---|---|---|---|
| O5 | W1 of RS2: in which colour space do blending and transitions run? Today `blend_u8` uses encoded bytes and `blend_f32` linear values, with grey gaps up to 232/255. A black-to-white Dissolve at 0.5 gives 128 encoded and 188 linear. It must be ruled before `gafira-render-cpu` uses these functions | (a) Linear light in both: Preview matches Export (02 §6.2), but blend modes look unlike tools that blend encoded values, and CPU Preview pays a decode and encode per pixel: 154 to 156 ms per 1080p `Rgba8` Dissolve today, with an estimated 5 to 12 ms after tables, a fast path and threads (P15 item 8). (b) Encoded sRGB in both: familiar blend modes, but it contradicts the linear working space. (c) Each format as stored: no cost, but Preview and Export visibly differ | (a) |

XP5 reached ARQ-132 only in-sample. If `SG3` cannot reach the gate, D-20 is the owner's decision, and that becomes a new question then.

## Needs explorer

XP1 to XP10 are resolved; XP11 and XP12 are open, and XP11 comes from those results. XP13 is measured. XP5, XP7, XP8 and XP9 ran on one prototype: i7-12700H under load, not D-03 hardware. `E1` to `E5` are the older spikes.

| # | Claim | Check | Blocks |
|---|---|---|---|
| XP1 | Measured, confirmed: see `engine-contracts` | | Nothing |
| XP2 | Measured: the parley 0.12 tree with `rayon` passes `cargo deny` and the V5 script. The banned crates of ARQ-107 are absent by a one-off `cargo tree` grep; `SC1` turns that into a step (P9) | `cargo deny` exit 0 on the 3 targets; V5 script exit 0 with the 4 rows; builds on 1.93.1; non-Linux targets not compiled | Nothing |
| XP3 | Resolved, confirmed with limits: E4's behaviour is regenerated, not its bytes. Recipe in P7. `TX1` uses it | Sizes, advances, Arabic joining and Manrope `liga` hold; parley 0.12.0 loads all four fonts | Nothing |
| XP4 | Resolved: three-box and the dot-product kernel are refuted; the tap-outer direct kernel meets 100 ms. Decision in P11 | 46 to 52 ms single-threaded, σ −0.6 % to −1.25 %, tile match max diff 0. Not measured: RGBA, non-x86 | Nothing |
| XP5 | Resolved with limits: IoU ≥ 0.90 on 16 of 16 scenes, only with `gamma` 500 or GMM regularisation 25; 14 of 16 at 50. `gamma` was chosen on those scenes, so the pass is in-sample. Textured background at ΔE 6, not a gate scene, passes at 50 and fails at 500 | 4000×3000 in 0.3 to 1.6 s by thread count, peak RSS 585 MB. D-03 estimate 1.3 to 3 s, inferred, so CAL-06 still needs its bench | Nothing; see O4 |
| XP6 | Measured: `SC1` with unused dependencies is allowed | No unused-dependency rule in the repo; `cargo deny` result as in XP2 | Nothing |
| XP7 | Resolved: noise is not reliable, so the ARQ-131 rule stands. ARQ-131 now defines uniform noise as per-pixel (bbadc85) | 0 of 48 reliable with per-pixel noise | Nothing |
| XP8 | Resolved with a condition: ARQ-43 holds only with chunked serial post-processing and hash adjacency. `SG` deliveries must chunk them (P12) | ≤ 100 ms measured with both, on the prototype | Nothing; see XP11 |
| XP9 | Resolved with a limit: ≥ 50 reports are reached. The ≤ 0.02 step is missed by time share, set by `sp_stats`, so the fraction follows a work model (P12) | Time share per step 0.033 to 0.086 | Nothing; ARQ-112 now says so (ba7fee8) |
| XP10 | Resolved, confirmed: the P9 step is exact. `StyleProperty::Locale` is the only parley API that needs the word (P7) | Passes clean, fails with `icu_plurals`, works under gawk and mawk | Nothing |
| XP11 | Hash adjacency can meet ARQ-43 without `std::collections::HashMap`, which `clippy.toml` bans in domain crates, and it stays deterministic (ARQ-111). The prototype's table type is not recorded here | Re-time the longest stretch between token checks with an allowed table: own open addressing, or sorted pairs | `SG2` |
| XP12 | `gafira-text` can meet FD-41 (g) without a direct `skrifa`, and adding `skrifa` as in P14 adds no package to `Cargo.lock` | Fixture with two partial fonts on one cluster and a tie through `layout_box`; a throwaway branch with the P14 entry, then `cargo tree -p gafira-text`, `cargo deny` and the V5 and ARQ-107 steps | Start of `TX2` |
| XP13 | Measured, the claim fails: with O5 (a), a CPU `Rgba8` Dissolve at 1080p takes 154 to 156 ms, about 19 times the 8 ms share of 02 §6.2. Remedies and estimates in P15 item 8 | `RS3a` verify round 1: 5 runs, `--release`, `p` 0.37, scratch export. `RS3a` still owes the bench | Nothing; informs O5 |

## Needs specifier

Open:

| # | ID | Request | Blocks |
|---|---|---|---|
| SP17 | 02 §4.9 | Add `fn params(&self) -> &ParamSchema;` to the trait and the `validate_definition` step to the "how to add" list (P13) | `AU1` fix round |
| SP18 | 02 §4.17.5, FV-14 | State `animatable` per parameter, the finite bound of `duration`, the label keys, and whether a unit belongs in the schema. Settle whether `pan` is animatable (P13) | `AU1` fix round; proceed on the provisional P13 table if it comes later |
| SP19 | 02 §4.17.5 | Add the `Eq3Band` and `LoudnessNormalizer` parameters to the table: neither has a row, and P13 makes `params()` mandatory for both | `AU2`, `AU3` |
| SP20 | 02 §4.13 or §4.17.4 | Transition parameter table: P15 item 1, and whether `animatable` stays `false` | `RS3a` |
| SP21 | ARQ-118 | Slide and Wipe conventions: P15 item 2, `lado` per axis, replace instead of composite, softness default | `RS3b` |
| SP22 | ARQ-118, FV-06 | Zoom: linear scale law, centre pivot, bilinear, clamp outside the image, and the `out` mode that ARQ-118 lacks and FV-06 requires (P15 item 3) | `RS3b` |
| SP23 | 02 §4.17, §4.13 | Add `Transition` and `Effect` to the type-home table, and state that a transition runs on whole frames | Nothing |
| SP24 | 02 §4.13, ARQ-118, 02 §7 | Record the persisted `transition_id` values, `handles` per transition, the two formats of "por formato", and `EffectError` and its keys (P15 items 4 to 7) | Nothing |
| SP25 | ARQ-118 | After O5: the mixing space, premultiplied mixing, and the `Rgba8` rounding (P15 items 8, 9), with the mid `p` test values | Nothing |
| SP26 | 02 §4.4, §4.13, §4.17 table, ARQ-120 | Record `EffectCx.tile` and `TileWindow` (P16): the window in 02 §4.4, its home in the §4.17 table, that a transition ignores it, and the ARQ-120 sentence and test with the `cx.tile` `Err` | Fix of W4 (`gafira-raster`) |

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
| SP16 | ARQ-131 | Input alpha weights the means; a side with no alpha is not reliable | c25488b |

CAL-06 and CAL-15 now live in `01`, and the FV IDs in `05`. That promotion is merged, so no stream carries the rename.

## Next

Finish `D2a`, `D2b` and `D3`, then run `SC1`. Ask the owner O4 before `SG3`. `explorer` takes XP11 before `SG2` and XP12 before `TX2`. `specifier` takes SP20 to SP22 before `RS3a` and `RS3b`, and the owner takes O5. `specifier` takes SP17 and SP18, then `AU1` takes P13 in its fix round.
