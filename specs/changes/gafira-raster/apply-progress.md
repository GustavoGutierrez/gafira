## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Product IDs: RS1 ARQ-117, ARQ-121, ARQ-110, ARQ-111; RS2 ARQ-119, FD-12 (c, CPU); RS3 ARQ-118, ARQ-25 (ends), 02:1308 (`EffectError` wraps `BufferError`); RS4 ARQ-120, FD-35 (a, d; Paralela only, D-42), ARQ-114 (bench_shadow_1080); RS5 FD-33 (a, d to h) except colours detected by ΔE; RS6 ARQ-122, FD-26, FD-28 (b), FD-33 (detected colours), FD-34 (a, b; D-40, D-41), ARQ-114 (bench_dhash_12mp) (source: specs/02-arquitectura-software.md, specs/01-producto-y-calidad.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/05-funcionalidad-video-y-distribucion.md)
Scope: RS1 to RS6 of specs/changes/layer-2/design.md, with P11 for the blur.      Non-goals: other streams; the filter catalogue (Q-34); Brillo and Contorno (D-42); `f16` buffers.
Deliveries: RS1 to RS6, serial; each verified and merged before the next.
Plan: specs/changes/layer-2/design.md, table and decisions; `crates/gafira-raster/` plus the files P2 assigns.
Owner decisions touched: D-40 to D-44 as recorded; none reopened.

# Apply progress: gafira-raster

## RS1

Branch `feat/gafira-raster`, from `develop` 9124700. Commits: e2f43d2 (views, mask, error), 3d1d62b (premultiply), 42ca239 (tiles, mips). Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each, read alone.

Size: 1 367 lines added in `crates/gafira-raster/src/` (`git diff --shortstat`), of which about 670 are tests. Over the ~700 mark; the delivery table forecast 450 (630).

### Done

ARQ-117 (views, length and overflow checks, 8-bit and `f32` premultiply), ARQ-121 (tile grid, level count and size, linear premultiplied box mean), ARQ-110 (`f32` pixel checks before any write, constructors do not validate), ARQ-111 (repeat-run tests). Public API: `ImageView`, `ImageViewMut`, `AlphaMask`, `ImageBuf`, `PixelBuf`, `PixelData`, `PixelDataMut`, `PixelFormat`, `EffectCx`, `BufferError` (with `ToMessage`, ADR 0012 keys `error.raster.*`), `premultiply`, `unpremultiply`, `premultiply_u8`, `unpremultiply_u8`, `tile_grid`, `TILE_SIZE`, `build_mips`, `mip_level_count`, `mip_size`.

### RED per criterion

| Criterion | Failing test and assertion (bodies stubbed) |
|---|---|
| ARQ-117 length | `arq_117_length_mismatch_is_err_for_each_format`: "rgba8 2x3 len 23", left true, right false |
| ARQ-117 overflow | `arq_117_size_overflow_is_err_without_allocation`: `unwrap_err` on `Ok` |
| ARQ-117 other views | `arq_117_mutable_view_mask_and_buf_check_length`: `is_err` failed |
| ARQ-117 premultiply | `arq_117_premultiply_is_exact_rounding_and_at_most_alpha`: "c=1 a=0", left 1.0, right 0.0 |
| ARQ-117 unpremultiply | `arq_117_unpremultiply_rounds_clamps_and_zero_alpha_is_zero`: "c=1 a=0", left 1, right 0 |
| ARQ-117 identity, zero | `arq_117_opaque_is_identity_and_transparent_is_zero`: left 1, right 0 |
| ARQ-117 views | `arq_117_view_premultiply_and_unpremultiply_8bit`, `arq_117_view_f32_values_and_zero_alpha` |
| ARQ-110 pixels | `arq_110_non_finite_pixel_is_err_and_buffer_unchanged`, `arq_110_overflowing_result_is_err_and_buffer_unchanged`, `arq_110_non_finite_pixel_is_err_for_f32` (`Ok` instead of `Err`) |
| ARQ-121 tiles | `arq_121_tile_counts_at_the_256_boundary`, `arq_121_tile_grid_of_the_largest_image` |
| ARQ-121 mips | `arq_121_level_count_at_powers_of_two`, `arq_121_mip_size_rounds_down_with_minimum_one`, `arq_121_chain_of_1000_by_600_has_ten_levels_ending_1x1`, `arq_121_checkerboard_level_one_is_188`, `arq_121_transparent_pixel_adds_no_colour`, `arq_121_level_one_matches_a_linear_premultiplied_reference`, `arq_121_mean_is_weighted_by_alpha`, `arq_121_odd_side_uses_the_first_even_part`, `arq_121_alpha8_and_f32_levels` |

Not RED, proved by a mutant: `arq_110_constructors_accept_non_finite_values`, `arq_117_view_round_trips_dimensions_and_pixels`, `buffer_error_maps_to_message_keys` (the code held by construction), `arq_111_premultiply_is_deterministic` and `arq_111_build_mips_is_deterministic` (hold with no entropy; mutants 10b), `arq_121_constant_image_is_equal_at_all_levels` (vacuous on the stub; now asserts 6 levels).

### Hand mutants

| # | Mutant | Killed by |
|---|---|---|
| 1 | `+ 127` to `+ 126`; to `+ 128`; `/ 255` to `>> 8` | `arq_117_premultiply_is_exact_rounding_and_at_most_alpha`; `arq_117_opaque_is_identity_and_transparent_is_zero` for `>> 8` |
| 2 | `+ 127` to `+ 255` (result above `a`) | `arq_117_premultiply_is_exact_rounding_and_at_most_alpha` and two more |
| 3 | `a = 0` returns `c`; `min(255)` removed; truncation | `arq_117_unpremultiply_rounds_clamps_and_zero_alpha_is_zero` (all three; the `min` one first survived because of a saturating `unwrap_or(MAX)`, so the fallback is now 0) |
| 4 | `a = 255` off by one, premultiply and unpremultiply | `arq_117_opaque_is_identity_and_transparent_is_zero` |
| 5 | `expected == actual` to `<=` and to `true`; `wrapping_mul` for the element count | `arq_117_length_mismatch_is_err_for_each_format`, `arq_117_mutable_view_mask_and_buf_check_length`, `arq_117_size_overflow_is_err_without_allocation`. `wrapping_mul` of `width × height` alone is an equivalent mutant on 64-bit (`u32 × u32` fits `usize`); it only matters on 32-bit targets |
| 6 | Round up; `max(1)` dropped on both sides; dropped on the height only | `arq_121_mip_size_rounds_down_with_minimum_one`, `arq_121_checkerboard_level_one_is_188` |
| 7 | Count `+ 1`; ceil log2; min side | `arq_121_level_count_at_powers_of_two` |
| 8 | sRGB mean (no decode); equal weights (unpremultiplied); `a = 0` pixel adds raw colour with weight 1 | `arq_121_level_one_matches_a_linear_premultiplied_reference`, `arq_121_constant_image_is_equal_at_all_levels`, `arq_121_transparent_pixel_adds_no_colour` (the last one needed the black-plus-bright-ghost case added). Removing only the `a == 0` skip is equivalent: the weight is 0 |
| 9 | 256 gives 2; 257 gives 1; tile 255 | `arq_121_tile_counts_at_the_256_boundary` |
| 10 | NaN-only check (no `±∞`) in `premultiply` and in `build_mips`; no result check; real entropy (`RandomState`) in `f32` mip, 8-bit mip and `premultiply` | `arq_110_non_finite_pixel_is_err_and_buffer_unchanged`, `arq_110_overflowing_result_is_err_and_buffer_unchanged`, `arq_110_non_finite_pixel_is_err_for_f32`, `arq_111_*_is_deterministic` |

Odd-side mutant (last row or column included): killed by `arq_121_odd_side_uses_the_first_even_part`.

### Deviations and assumptions (no spec text; verify may check)

1. ARQ-121 does not say what an empty image gives. Chosen: 0 levels, `build_mips` returns `Ok(vec![])` (ARQ-118: size 0 is `Ok`).
2. ARQ-121 does not say what an odd side does. Chosen: the box starts at `2x` and the last row or column is dropped; a side of 1 repeats its pixel.
3. Buffer semantics for mips: `Rgba8` is sRGB-encoded and premultiplied; `RgbaF32` is linear and premultiplied (02 §6, Export), averaged as is; `Alpha8` is a plain rounded mean. Not spelled out per format in the spec.
4. P4 lists no `ImageViewMut` methods and no owned type. Added `ImageViewMut::{new, width, height, data, data_mut}`, `PixelDataMut`, `ImageBuf`/`PixelBuf` (owned mip levels), `PixelData::format`, `PixelFormat::channels`. `Cost`, `HandleReq` and the GPU traits of P5 are not in the RS1 row and are not built.
5. `BufferError` carries `NonFinite { field, index }` besides the two constructor variants of P4, so one enum serves ARQ-110 for RS1. `field` is `"pixels"`.
6. `TileGrid { columns, rows }` and `tile_grid` are the minimum for "257×1 is 2 tiles"; `Tile` and `TileSet` of 02 §4.17 are not RS1.
7. The 8-bit mip decodes and encodes through `gafira_color::AnalyticTransform`, ignoring its impossible `Err` (a four-channel buffer is always whole).

### Fix round 1 (verify round 1: C1 to C5, W1 to W3, S1, S2)

Commits: cfdd474 (mips: C1, W1, W3 mips part, S2), then the test commit for C2 to C5 and S1. Gates exit 0 on each (fmt, clippy `-D warnings`, test `--workspace --locked`, read alone). Test count in `gafira-raster`: 29 to 41.

- C1: an `RgbaF32` pixel with `a = 0` now adds no colour. RED: `arq_121_f32_transparent_pixel_adds_no_colour`.
- W1 (amended ARQ-121): every source pixel counts. Each level pixel is the equal-weight mean of the original pixels it covers; an odd side joins its last row or column to the last box; a side of 1 does not divide. RED on the old code: `arq_121_odd_side_counts_every_pixel_alpha8` (3x1 gave 0, not 85), `arq_121_odd_side_counts_every_pixel_rgba8` (3x1 gave 0, not 156), `arq_121_wide_image_top_level_is_the_exact_mean` (1x1 gave 0, not 185). The test of the old rule (`arq_121_odd_side_uses_the_first_even_part`) is replaced.
- Choice on rounding: levels are not rebuilt from level 0 pixel by pixel (that costs a pass over the source per level). Each level is built from the unrounded f64 sums of the level above, which started from level 0, so a level equals the exact mean of the originals it covers and no rounding builds up. Cost: 32 bytes per source pixel while building (about 380 MB for 12 MP), then a quarter of that. If a speed or memory test needs less, level 1 could be computed straight from the 8-bit source; not done now.
- W2: `EffectError` does not exist in RS1 (no `Effect` or `Transition`), so there is nothing to wrap. Reported: RS3 or RS4, whichever adds `EffectError` first, must add a variant wrapping `BufferError` with `From`, with `to_message` delegating (02:1308).
- C2, C3: `arq_110_non_finite_colour_in_transparent_pixel_is_err` (pixel.rs, for `premultiply` and `unpremultiply`, alpha 0 and -0.0, NaN and both infinities, buffer unchanged) and the same name in mip.rs for `build_mips`. Not RED; proved by V1 and V2.
- C4: `arq_111_every_conversion_is_deterministic` runs `premultiply` and `unpremultiply` twice on `f32` and `Rgba8`. Not RED; proved by V3.
- C5: `proptest!` blocks `arq_110_finite_input_gives_finite_output`, `arq_110_unit_range_premultiply_is_ok_and_finite`, `arq_110_finite_mips_are_finite`. Not RED.
- W3: `arq_121_constant_image_all_formats` (f32 and Alpha8, odd sizes), plus the odd-row case in the Alpha8 and Rgba8 odd-side tests (1x3 and 3x3). S2: `arq_121_zero_side_images_have_no_levels` (0xN, Nx0, 0x0, all formats), and `arq_121_one_by_one_has_one_level_equal_to_input`. S1: dead code removed.

Mutants of this round (all killed): V1 (input check skips `a = 0` pixels in `convert`) by `arq_110_non_finite_colour_in_transparent_pixel_is_err`; V2 (same in `build_mips`) by the mip test of that name; V3 (`RandomState` entropy in `unpremultiply` f32, in 8-bit premultiply and in 8-bit unpremultiply) by `arq_111_every_conversion_is_deterministic`; f32 `a = 0` colour by `arq_121_f32_transparent_pixel_adds_no_colour`; last box not widened (odd side dropped), area ignored, cover sum wrong, sRGB mean, equal weights, Rgba8 `a = 0` raw colour, alpha rounding down, unchecked overflow result: all by the ARQ-121 and ARQ-110 tests above.

Deviations: a mutant run left an untracked `crates/gafira-raster/proptest-regressions/` folder (proptest's failure seeds). I could not delete it here (the `rm` was denied) and did not commit it; delete it before merge. S3 and S4 need no code. Lines for the round: see `git diff --shortstat 10f328b..HEAD -- crates/`.

