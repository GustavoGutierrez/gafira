# Spike: `gafira-render-cpu`

Facts only. Claims `XP1`, `XP3` and `XP4` of `design.md` ("Needs explorer"), measured
2026-11-05. `XP2` is not in this round.

Scratch: three throwaway crates outside the repo, `<scratch>/xp3`, `<scratch>/xp1`,
`<scratch>/xp4`, built with `CARGO_TARGET_DIR=<scratch>/target CARGO_INCREMENTAL=0`. Toolchain
`rustc 1.93.1` / `cargo 1.93.1` (the repo's `rust-version`). The probe manifests mirror the
repo's `[profile.release]` verbatim (`opt-level = 3`, `lto = "thin"`, `codegen-units = 4`,
`panic = "unwind"`), because that profile decides whether `gafira-raster`'s per-pixel functions
are inlined across the crate boundary. No repo file was modified: the probes read
`crates/gafira-raster` through a path dependency, and `deny.toml` was copied, not edited.

## Summary

| Claim | Verdict | Number |
|---|---|---|
| `XP3` resolves, builds, passes `cargo deny` on the 0.12 line | **confirmed** | `tiny-skia 0.12.0`, BSD-3-Clause, `advisories ok, bans ok, licenses ok, sources ok` |
| `XP3` builds for the two non-host targets | **confirmed** for `cargo build --lib` (rlib, no linker) | both targets finish in ~1.7 s |
| `XP3` runs and is byte-identical on the two non-host targets | **unmeasured** | linking fails locally on both; only the remote CI can run them |
| `XP1` coverage (`Mask::fill_path`, 40 tiles, 1080p) | **confirmed cheap** | 0.78 ms (page rect), 1.12 ms (256-segment path) |
| `XP1` coverage with the `D6` cull | **premise contradicted**, cull harmless | 0.69 ms / 1.27 ms; dense page 4.19 → 2.12 ms |
| `XP1` `blend_f32`, 2.07 M px × 3 layers | **confirmed**, and not the expensive one | 90.7 ms (Normal), 213.1 ms (Multiply) |
| `XP1` `blend_u8`, same work | **contradiction of the risk's direction** | 640.7 ms (Normal) — **7.1×** `blend_f32` |
| `XP1` total against CAL-11's 2 s | **confirmed, ample margin** | ≈ 115 ms of the 2 000 ms (17× margin) |
| `XP4` surface + pool arithmetic | **confirmed** | 33.2 MB / 132.7 MB + `depth × 1 MiB`; ≤ 15.8 % of the 1.5 GB cap |
| `XP4` FD-55's 16 384 px side | **contradicted as feasible** → stated limit | 4 GiB `RgbaF32` = 2.9× the cap; the allocation fails under it |

## 1. XP3 — `tiny-skia` 0.12 builds and passes `cargo deny`

### What was built

`<scratch>/xp3`, one library crate and the repo's `deny.toml` copied next to it:

```toml
[dependencies]
tiny-skia = { version = "0.12", default-features = false, features = ["std", "simd"] }
```

`src/lib.rs` fills a page-covering rect into a 256×256 `Mask`, which is one tile task of `D6`.

### Resolved version and tree

```
$ cargo tree
xp3-tiny-skia-probe v0.0.0
└── tiny-skia v0.12.0
    ├── arrayref v0.3.9
    ├── arrayvec v0.7.8
    ├── bytemuck v1.25.2
    ├── cfg-if v1.0.5
    ├── log v0.4.35
    └── tiny-skia-path v0.12.0
        ├── arrayref v0.3.9
        ├── bytemuck v1.25.2
        └── strict-num v0.1.1
```

- It resolves to **`tiny-skia 0.12.0`** (and `tiny-skia-path 0.12.0`) with `--offline`: the
  local registry has it, no network is needed. `Locking 8 packages to latest compatible versions`.
- `png-format` really is out of the graph: no `png`, `miniz_oxide` or `flate2` appears, so `D2`'s
  "dropping `png-format` keeps the `png` codec out of the product graph" holds.
- **The tree is seven crates, not five.** `D2` (design.md:245-247) lists `tiny-skia-path`,
  `arrayref`, `arrayvec`, `bytemuck`, `log`; it omits **`cfg-if`** and **`strict-num`**.
- Three of the seven are already in the workspace's `Cargo.lock` at exactly these versions —
  `bytemuck 1.25.2`, `arrayvec 0.7.8`, `cfg-if 1.0.5` (checked with `git grep` on `Cargo.lock`).
  `log`, `arrayref`, `strict-num` are new. So `SC1`'s `Cargo.lock` gains **five** entries
  (`tiny-skia`, `tiny-skia-path`, `log`, `arrayref`, `strict-num`) and no `multiple-versions`
  warning is triggered, because no version is duplicated.

### Licence and advisories

```
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
```

Per-crate licences seen by `cargo metadata`:

```
tiny-skia 0.12.0: BSD-3-Clause        tiny-skia-path 0.12.0: BSD-3-Clause
arrayref 0.3.9: BSD-2-Clause          arrayvec 0.7.8: MIT OR Apache-2.0
bytemuck 1.25.2: Zlib OR Apache-2.0 OR MIT   cfg-if 1.0.5: MIT OR Apache-2.0
log 0.4.35: MIT OR Apache-2.0         strict-num 0.1.1: MIT
```

Every one is on `deny.toml`'s allow-list (`BSD-3-Clause` at `deny.toml:21`, `BSD-2-Clause` at
`:20`, `MIT`, `Apache-2.0`, `Zlib` at `:23`). No advisory fires; the advisory database was
available for the run. `deny.toml` needs no amendment, as `D2` and the "Shared files" table say.

### The three targets

`rustup target list --installed` already holds all three (`aarch64-apple-darwin`,
`x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`), so **the two non-host targets compile
locally** — a library target is an rlib and never invokes a linker:

```
$ cargo build --lib --release --target aarch64-apple-darwin
   Compiling tiny-skia v0.12.0
   Compiling xp3-tiny-skia-probe v0.0.0
    Finished `release` profile [optimized] target(s) in 1.69s
$ cargo build --lib --release --target x86_64-pc-windows-msvc
    Finished `release` profile [optimized] target(s) in 1.79s
$ cargo build --lib --release --target x86_64-unknown-linux-gnu
    Finished `release` profile [optimized] target(s) in 1.68s
```

What does **not** work locally, and must not be claimed:

```
$ cargo build --bin probe --target aarch64-apple-darwin
  = note: cc: error: unrecognized command-line option '-arch'
          cc: error: unrecognized command-line option '-mmacosx-version-min=11.0.0'
$ cargo build --bin probe --target x86_64-pc-windows-msvc
  = note: please ensure that Visual Studio 2017 or later, or Build Tools for Visual Studio were
          installed with the Visual C++ option.
```

There is no macOS SDK and no MSVC linker here, so nothing can be linked, executed or hashed for
those targets. `XP3`'s third clause — *byte-identical output across the three targets for the same
input at the same `simd` level* — is **unmeasured**: proving it needs a run on each platform, and
the repo's `test` matrix (`.github/workflows/ci.yml:40-61`, native runners, `cargo clippy
--workspace --all-targets`) is the only place that can. The local evidence stops at "it compiles".

**Verdict:** confirmed for resolving, building and `cargo deny` on the host and for compiling the
library on all three targets; **unmeasured** for execution and byte-identity on the two non-host
targets. **Blocks:** `SC1`'s verify (the licence and lock questions are settled); it does not block
`SC1` from starting.

## 2. XP1 — the cost of a 1080p page on the CPU

### What was measured

`<scratch>/xp1` depends on the **real** `gafira-raster` (path dependency) and on `kurbo 0.13` +
`tiny-skia 0.12`, and mirrors the workspace release profile. Three separate bodies, each timed as
one full page pass = 40 tiles (8 columns × 5 rows, `256×256`, the last column and row clipped),
21 repetitions after a warm-up, median and min reported, `std::hint::black_box` around the result,
`taskset -c 3` so the numbers are not moving between cores:

1. no cull — for each of the 40 tiles: `mask.clear()`, convert the page-space `kurbo::BezPath` to
   `tiny_skia::Path` (`D2`'s ~40-line conversion), `mask.fill_path(&p, Winding, true,
   Transform::identity())`;
2. `D6`'s cull — for each tile: skip when the path's `bounding_box` misses the tile's region,
   otherwise convert with `-tile_origin` added to every coordinate and fill;
3. `gafira_raster::blend_u8` and `blend_f32` over 2 073 600 pixels (1920 × 1080) × 3 layers,
   premultiplied data, one surface-sized pass per layer.

### Raw output

```
tiles: 40 (1920x1080 page, 256x256 tiles)
clear 40 masks (256x256)                     median    0.039 ms   min    0.039 ms
convert page rect x40 (identity)             median    0.002 ms   min    0.002 ms
convert page rect x40 (tile-translated)      median    0.002 ms   min    0.002 ms
convert 256-segment path x40 (identity)      median    0.103 ms   min    0.102 ms
convert 256-segment path x40 (tile-translated) median  0.103 ms   min    0.102 ms
XP1.1 fill 40 tiles, rect, no cull           median    0.778 ms   min    0.695 ms
XP1.1 fill 40 tiles, 256-seg path, no cull   median    1.120 ms   min    1.080 ms
XP1.2 fill 40 tiles, rect, cull              median    0.686 ms   min    0.638 ms
XP1.2 fill 40 tiles, 256-seg path, cull      median    1.267 ms   min    1.215 ms
scene: 20 shapes x 40 tiles, no cull         median    1.045 ms   min    1.031 ms
scene: 20 shapes x 40 tiles, cull            median    0.894 ms   min    0.883 ms
dense: 200 shapes x 40 tiles, no cull        median    4.190 ms   min    4.054 ms
dense: 200 shapes x 40 tiles, cull           median    2.118 ms   min    2.003 ms
XP1.3 blend_u8 Normal, 2.07 M px x 3 layers  median  640.724 ms   min  620.569 ms
XP1.3 blend_f32 Normal, 2.07 M px x 3 layers median   90.660 ms   min   87.251 ms
XP1.3 blend_u8 Multiply, 2.07 M px x 3 layers median 718.200 ms  min  709.945 ms
XP1.3 blend_f32 Multiply, 2.07 M px x 3 layers median 213.065 ms min  212.237 ms
PNG encode 1920x1080 RGBA, Compression::Fast median   23.357 ms   min   22.814 ms
pixels per layer: 2073600, blend calls per format: 6220800
```

Three further runs on the same core stayed inside ±3 % on every row but `blend_u8 Multiply`,
which reached 1 132 ms once on a loaded machine (min 710 ms across runs; the medians above are the
fourth run, the `min` column is the stable figure). The 256-segment path is a `kurbo::BezPath` of
256 cubics around an ellipse that covers the page; "20 shapes" is twenty rounded rects, "200 dense"
two hundred small rects on a 20 × 10 grid.

The measurement is only worth something if the cheap fill really fills, so it is pinned:

```
$ cargo test --release --lib         # in xp3
test tests::a_page_rect_fills_every_byte_of_a_tile_mask ... ok
```

The page rect covers all 65 536 bytes of a tile at `255`; translated to the tile at (1024, 768),
column 127 is `255` and column 128 is `0`, i.e. exactly the page's right edge at `x = 1920`.

### (1) and (2): coverage, and what the cull of `D6` actually buys

- The whole 1080p page costs **0.78 ms** of coverage with the page rect, **1.12 ms** with a
  256-segment path, **1.05 ms** for a 20-shape scene.
- That is ~19 µs per tile for 65 536 pixels, ≈ 0.3 ns/pixel. Forty tiles scanned at page size
  would be 83 M pixels in 0.78 ms — 10 Gpx/s, impossible. **`tiny-skia` already confines the
  scan to the destination mask**: the page-sized path is not scanned 40 times, only the tile is.
  The per-tile cost is the mask area, not the path's page size.
- The cull therefore moves very little on a page-covering op: −12 % for the rect (0.778 → 0.686),
  and **+13 % slower** for the 256-segment path (1.120 → 1.267), because translating the path into
  tile-local coordinates makes most of it land far outside the mask and the edge clipper does the
  extra work. On a sparse page (20 shapes) it is −14 % (1.045 → 0.894); on a dense one (200
  shapes) it is −49 % (4.190 → 2.118), which is the cull doing what it is for: **skipping the ops
  that miss the tile**, not shrinking the scan.
- Conversion is negligible either way: 0.002 ms for the page rect ×40, 0.103 ms for the
  256-segment path ×40 (~2.6 µs per tile). Mask clearing is 0.039 ms ×40.

**Verdict on `D6` (design.md:325-328): the premise is contradicted, the decision is still fine.**
The sentence "without a cull, a 1080p export converts and scans the same page-sized path 40 times;
with it, each tile converts a path and scans only its own 256×256 window. This is also the
difference between CAL-11 and CAL-11 × the tile count" does not survive measurement: the scan is
already the tile's window, and the page-background rect — the example the paragraph is built on —
is the case where the cull saves nothing (4 ms of difference on a 200-shape page is 0.2 % of
CAL-11). Keep the cull: it is cheap, it bounds the dense-page case and it skips ops that cannot
contribute, but it is not what makes CAL-11 and should not be sold as such.

### (3): the blends, and the direction of the risk

| 6 220 800 calls | `Normal` | `Multiply` | per call (`Normal`) |
|---|---|---|---|
| `blend_f32` (the export path, `RgbaF32`) | **90.7 ms** | 213.1 ms | 14.6 ns |
| `blend_u8` (the preview path, `Rgba8`) | **640.7 ms** | 718.2 ms | 103 ns |

**`XP1`'s stated worry — "`blend_f32` is a per-pixel `f64` call with a finite check" — is refuted
in direction: `blend_f32` is 7.1× cheaper than `blend_u8`.** The reason is in the code, not in the
call: `blend_u8` (`crates/gafira-raster/src/blend.rs:215`) decodes with a 256-entry table but
encodes through `Transfer::encode_unit` (`crates/gafira-raster/src/transfer.rs:46`), which calls
`AnalyticTransform::apply_f32` → `convert_rgb` → `math::encode` → `powf(1.0 / 2.4)`
(`crates/gafira-color/src/math.rs:103-111`): **three `f64` `powf` per pixel**, ~18.6 M of them,
i.e. ~33 ns each. `blend_f32`'s finite checks and `f64` composite are cheap next to that.

Consequence for the change, stated plainly: **the export is the cheap path and the preview is the
expensive one.** CAL-11 is an export number, so it is safe. But a full-page `Rgba8` composite of
three layers is 640 ms, 46× the 14 ms visible-render budget of 02 §6.2 — the preview can only
afford it through culling, mips, priority and the deadline. That is consistent with the 154-156 ms
already recorded for a 1080p `Rgba8` dissolve in 02 §6.2 (D-48); it is not a new defect, but it is
the number that `CB7` should be argued from, and it is not the number `XP1` predicted.

### Does the total leave margin under CAL-11's 2 s?

Measured single-threaded components for one 1080p export page:

```
coverage, 40 tiles, 20-shape scene, cull          0.9 ms
blend_f32, 2.07 M px × 3 full-page layers        90.7 ms
PNG encode, 1920×1080 RGBA, Compression::Fast    23.4 ms
                                                ---------
                                                ≈ 115 ms   of 2 000 ms  (≈ 17× margin)
```

Ten full-page layers instead of three would still be ~325 ms (6× margin). The sum is a budget of
measured parts, not a measurement of `CB1`: it excludes the per-pixel mask multiply, the
premultiply/unpremultiply of ARQ-117 and the layer bookkeeping, all of which are a few ns per
pixel (the blend rows give the scale) and none of which is 1.8 s. The frame is generous in the
other direction too: three *full-page* layers is more than "a few groups and a page background".

**Verdict: confirmed.** CAL-11 is not at risk from the CPU arithmetic; `CB6` should measure well
under 2 s, and `CB7` has no performance case from this measurement on the export path (its case,
if any, is the `Rgba8` preview path, which CAL-11 does not measure). This is also the measurement
`O4` was waiting for: on these numbers `CB7` is not needed to reach CAL-11, and the owner's
decision (sequential for the MVP) holds without a margin problem.

**Blocks:** `CB1` (nothing measured contradicts it), `CB6`, and `O4`/`CB7` (the margin exists
without parallelism).

## 3. XP4 — the memory arithmetic

### Arithmetic (computed and printed, not estimated)

```
1920x1080:   RgbaF32 33177600 B =   31.6 MiB =   33.2 MB;  Rgba8  8294400 B =   7.9 MiB =   8.3 MB
3840x2160:   RgbaF32 132710400 B = 126.6 MiB =  132.7 MB;  Rgba8 33177600 B =  31.6 MiB =  33.2 MB
7680x4320:   RgbaF32 530841600 B = 506.2 MiB =  530.8 MB;  Rgba8 132710400 B = 126.6 MiB = 132.7 MB
16384x16384: RgbaF32 4294967296 B = 4096.0 MiB = 4295.0 MB; Rgba8 1073741824 B = 1024.0 MiB = 1073.7 MB

one layer buffer per depth (256x256 tile):
  Rgba8:   262144 B = 256 KiB = 0.250 MiB
  RgbaF32: 1048576 B = 1024 KiB = 1.000 MiB

surface + pool against D-03's 1.5 GB cap:
  1080p RgbaF32 + depth 1   =   34.2 MB =  2.28 % of the cap
  1080p RgbaF32 + depth 10  =   43.7 MB =  2.91 %
  1080p RgbaF32 + depth 100 =  138.0 MB =  9.20 %
  4K    RgbaF32 + depth 1   =  133.8 MB =  8.92 %
  4K    RgbaF32 + depth 10  =  143.2 MB =  9.55 %
  4K    RgbaF32 + depth 100 =  237.6 MB = 15.84 %
```

- `D4`'s "33 MB at 1080p, 133 MB at 4K" and "`depth × 1 MiB` (`RgbaF32`) / `depth × 256 KiB`
  (`Rgba8`)" are exactly right (33.2 MB, 132.7 MB, 1 048 576 B, 262 144 B).
- Inside 02 §6.6 / D-03's 1.5 GB: even a 4K export surface with 100 open groups is **15.8 %** of
  the cap, and the remaining 1.27 GB is what `ARQ-44`'s governor has for the tile cache and the
  frame cache (whose default alone is 1 GB). The arithmetic makes the governor's job tight but
  real; it does not make the surface a problem.

### FD-55's 16 384 px side

Allocation test, one `Vec<u8>` filled page by page so the pages are resident, peak RSS read from
`/proc/self/status`:

```
this machine (30 GB):   xp4-memory f32   → VmHWM 2.0 -> 4098.0 MiB (peak delta 4096.0 MiB)   ok
this machine (30 GB):   xp4-memory u8    → VmHWM 2.0 -> 1026.0 MiB (peak delta 1024.0 MiB)   ok
under a 1.5 GB cap (ulimit -v 1572864, D-03):
  xp4-memory f32 → memory allocation of 4294967296 bytes failed    (the process dies)
  xp4-memory u8  → allocated 1.00 GiB, ok  → 68 % of the 1.5 GB cap gone, ~0.47 GB left
```

So the 16 384 px side, in the numbers `D4` uses:

- `RgbaF32`: **4 GiB = 4.295 GB**, which is **2.9× D-03's 1.5 GB cap** and **54 % of the 8 GB
  reference machine** (02 §6.6). One surface of that size cannot exist inside the app's own memory
  contract, and the encoder needs its output on top. Under the cap it does not allocate at all.
- `Rgba8`: 1 GiB, 68 % of the cap — it allocates, but leaves ~0.47 GB for the tile cache, the
  frame cache, the encoder and everything else, so it is not a working shape either.

**Verdict: `XP4`'s first half is confirmed; FD-55's 16 384 px side is contradicted as feasible and
must be a stated limit.** The arithmetic decides it: one page-sized surface is the `D4` model
("The surface is `TargetDesc`-sized", design.md:268) and that model stops working well before
16 384 px — 8K (`7680×4320`, 530.8 MB) still fits, 16 384 px does not. FD-55 already says
"límite de 16 384 px por lado (PROPUESTA), exportación por tiles", so the requirement is not
broken; what is missing is the line in the design that says the tiled/streaming path is what that
limit costs, and that `CB1` neither builds it nor may claim it.

**Blocks:** `CB1` (the model is fine at 1080p and 4K) and `CB1`'s "cannot prove" table, which
must state the 16 384 px limit. Nothing here blocks `SC1`, `PC1` or `CB2`–`CB6`.

## 4. What contradicts the design, and where

| # | Contradiction | File:line | Suggested fix |
|---|---|---|---|
| 1 | `D6`'s "scans the same page-sized path 40 times … the difference between CAL-11 and CAL-11 × the tile count". The scan is already the tile's window; the cull saves ≤ 4 ms on a 200-shape page and nothing on the page background. | `design.md:325-328` | Say what the cull is for: skipping ops that miss the tile (−14 % sparse, −49 % dense) and bounding the pathological page. Do not tie it to CAL-11's margin. |
| 2 | `XP1`'s risk is placed on `blend_f32`; measured, `blend_f32` is 7.1× **faster** than `blend_u8` (640.7 ms), whose cost is three `f64` `powf` per pixel. `D8`'s "CAL-11's margin comes from culling, buffer reuse and avoiding copies" understates how much margin the blend already leaves. | `design.md:538`, `:379-380` | Record `blend_u8` as the expensive primitive and the `Rgba8` preview as the path to watch; `CB7`'s justification, if any, is the preview, not the export. |
| 3 | `D2`'s cost lists five transitive crates; the tree has seven (`cfg-if`, `strict-num` missing). | `design.md:245-247` | Add both; note that `bytemuck 1.25.2`, `arrayvec 0.7.8` and `cfg-if 1.0.5` already sit in `Cargo.lock`, so `SC1` adds five entries. |
| 4 | `D4`'s one page-sized surface cannot cover FD-55's 16 384 px side: 4 GiB in `RgbaF32`, 2.9× the 1.5 GB cap of D-03. | `design.md:268`, `:541` | State the limit and the tiled/streaming path it would need; keep `CB1` at 1080p/4K. |

## 5. What this round does not settle

- `XP2` (preview/export agreement at 1/255) — not run here, still blocks `CB3`.
- `XP3`'s byte-identity across targets — needs a run on macOS and Windows; only the CI matrix can
  produce it.
- `CB1`'s real pixel cost — the backend does not exist. The budget above is measured parts, and
  the parts that are missing (mask multiply, premultiply, layer bookkeeping) are per-pixel work at
  the scale the blend rows show, not a second of work.
- The `blend_u8` figure was taken with `lto = "thin"`, the workspace profile. A debug build is far
  slower, which is why `D10`'s 2 s guard only runs optimised.
