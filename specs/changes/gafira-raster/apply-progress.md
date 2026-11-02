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
- W2: `EffectError` does not exist in RS1 (no `Effect` or `Transition`), so there is nothing to wrap. Reported: RS3 or RS4, whichever adds `EffectError` first, must add a variant wrapping `BufferError` with `From`, with `to_message` delegating (02 §7.3).
- C2, C3: `arq_110_non_finite_colour_in_transparent_pixel_is_err` (pixel.rs, for `premultiply` and `unpremultiply`, alpha 0 and -0.0, NaN and both infinities, buffer unchanged) and the same name in mip.rs for `build_mips`. Not RED; proved by V1 and V2.
- C4: `arq_111_every_conversion_is_deterministic` runs `premultiply` and `unpremultiply` twice on `f32` and `Rgba8`. Not RED; proved by V3.
- C5: `proptest!` blocks `arq_110_finite_input_gives_finite_output`, `arq_110_unit_range_premultiply_is_ok_and_finite`, `arq_110_finite_mips_are_finite`. Not RED.
- W3: `arq_121_constant_image_all_formats` (f32 and Alpha8, odd sizes), plus the odd-row case in the Alpha8 and Rgba8 odd-side tests (1x3 and 3x3). S2: `arq_121_zero_side_images_have_no_levels` (0xN, Nx0, 0x0, all formats), and `arq_121_one_by_one_has_one_level_equal_to_input`. S1: dead code removed.

Mutants of this round (all killed): V1 (input check skips `a = 0` pixels in `convert`) by `arq_110_non_finite_colour_in_transparent_pixel_is_err`; V2 (same in `build_mips`) by the mip test of that name; V3 (`RandomState` entropy in `unpremultiply` f32, in 8-bit premultiply and in 8-bit unpremultiply) by `arq_111_every_conversion_is_deterministic`; f32 `a = 0` colour by `arq_121_f32_transparent_pixel_adds_no_colour`; last box not widened (odd side dropped), area ignored, cover sum wrong, sRGB mean, equal weights, Rgba8 `a = 0` raw colour, alpha rounding down, unchecked overflow result: all by the ARQ-121 and ARQ-110 tests above.

Deviations: a mutant run left an untracked `crates/gafira-raster/proptest-regressions/` folder (proptest's failure seeds). I could not delete it here (the `rm` was denied) and did not commit it; delete it before merge. S3 and S4 need no code. Lines for the round: see `git diff --shortstat 10f328b..HEAD -- crates/`.

## RS2

Branch `feat/gafira-raster` (from `develop` plus the card amend). Commit: see `git log` (`feat(raster): the 16 W3C blend modes ...`). Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0, read alone. `gafira-raster` tests: 41 to 55.

Source of the formulas: W3C *Compositing and Blending Level 1*, fetched from the W3C site (sections 5.1, 9 and 10). The test reference (`blend::tests::reference`) transcribes them in `f64` on straight colours with the general formula `αs(1 − αb)Cs + αs αb B + (1 − αs) αb Cb`, with its own structure (closures, `set_sat` by value matching); the code under test works on premultiplied values with another `set_sat`.

### Done

`blend_f32(mode, backdrop, source) -> Result<[f32; 4], BufferError>` and `blend_u8(mode, backdrop, source) -> [u8; 4]`, premultiplied RGBA in and out, all 16 `BlendMode::ALL` (the `match` has no `_` arm: a new mode does not compile). 8-bit blends the stored byte values as the formula inputs (no linearisation), result rounded to nearest.

### RED per criterion

| Criterion | Failing test (stub returned the backdrop) |
|---|---|
| 16 modes vs `f64` reference, f32 <= 1e-5 | `arq_119_every_mode_matches_the_reference_in_f32` (400 triples per mode) |
| 8 bits <= 1/255 | `arq_119_every_mode_matches_the_reference_in_8_bits` (also checks nearest-byte rounding, <= 0.5/255) |
| Formulas by hand | `arq_119_spot_values_of_the_w3c_formulas` (both `D(x)` branches, Overlay and HardLight asymmetry, four non-separable modes with ClipColor) |
| `Normal` at alpha 1 gives the source | `arq_119_normal_opaque_source_is_the_source` (also outside [0, 1]) |
| Backdrop a = 0 gives the source | `arq_119_transparent_backdrop_gives_source` |
| Clip only before other modes | `arq_119_clip_applies_only_before_other_modes` |
| Dodge/Burn boundaries | `arq_119_dodge_and_burn_boundaries` |
| ARQ-110 | `arq_110_non_finite_input_is_err_for_every_mode` (16 modes, 3 values, 8 positions, alpha 0.5 and 0), `arq_110_overflowing_normal_result_is_err`, proptest `arq_110_finite_input_gives_finite_output` |

Not RED, proved by mutants: `arq_119_transparent_source_leaves_backdrop` (source a = 0 with colour and -0.0), `arq_119_boundary_inputs_give_finite_output_for_every_mode` (0, 1, 0.25, 0.5 with alphas down to 1e-45), `arq_111_blend_is_deterministic`, proptest `arq_119_u8_result_is_premultiplied_valid`.

### Hand mutants (all killed unless noted)

| # | Mutants | Killed by |
|---|---|---|
| 1 | Per mode: SoftLight `D(x)` join 0.25 to 0.3 and to 0.2; `16` to `15`; Cs split 0.5 to 0.6; SoftLight arguments swapped; Overlay with source (arguments unswapped); HardLight swapped, split 0.4, constant `2s - 0.9`; Multiply `cb*cb`; Screen; Darken and Lighten swapped; Difference without `abs`; Exclusion; Dodge and Burn arguments swapped | the f32 and 8-bit tables, `arq_119_spot_values_of_the_w3c_formulas` |
| 2 | `Lum` three coefficients; `ClipColor` sign, factor, thresholds, denominators; `SetLum` sign and no clip; `Sat`; `SetSat` mid, max, min/max order; argument order of Hue, Saturation, Color, Luminosity; Hue's `Lum` argument | tables and spot values |
| 3 | `Normal` clipped | `arq_119_clip_applies_only_before_other_modes`, `arq_110_overflowing_normal_result_is_err` |
| 4 | No clip before other modes; no alpha cap (f32 and 8-bit); alpha not clipped | `arq_119_clip_applies_only_before_other_modes`, proptest |
| 5 | Source a = 0 changes the backdrop; backdrop a = 0 keeps its colour; `Normal` source scaled by alpha | `arq_119_transparent_source_leaves_backdrop`, `arq_119_transparent_backdrop_gives_source`, `arq_119_clip_applies_only_before_other_modes` |
| 6 | Composite on straight values; alpha union `as + ab` and `max`; `(1 - as)` term and `(1 - ab)` factor missing | the tables, identity tests |
| 7 | Dodge without the `Cb = 0` guard; Dodge and Burn guard order; unpremultiply without the `a > 0` guard (0/0) | `arq_119_dodge_and_burn_boundaries`, tables, `arq_119_transparent_backdrop_gives_source` |
| 8 | 8-bit truncation instead of rounding; straight colour quantised to 4 bits | `arq_119_every_mode_matches_the_reference_in_8_bits`, identity tests |
| ARQ-110/111 | Backdrop not checked; `±∞` not checked; check skipped for a = 0; result check reduced to NaN; `RandomState` entropy in the alpha | `arq_110_*`, `arq_111_blend_is_deterministic` |

Survivors, all equivalent: (a) `ClipColor` recomputing `x` or `L` after the `n < 0` step: both branches need spread above 1, and the spread of any colour reaching `ClipColor` is at most 1 (SetLum shifts equally; SetSat gives spread `s <= 1`), so the branches never both run. (b) Dropping the `Cs == 0` guard of ColorBurn: `(1 - cb) / 0` is `+inf` in IEEE, `min(1, inf) = 1`, result 0, the same value; the `Cb == 1` guard (0/0) is the one that matters and is killed. (c) `Normal` in `separable()` is never reached (the `Normal` path is premultiplied); kept for exhaustiveness.

### Assumptions and deviations (spec silent; verify may check)

1. ARQ-119 gives no signature. Chosen: per-pixel functions on premultiplied RGBA (`[f32; 4]`, `[u8; 4]`); no buffer-level blend, so no size-mismatch rule had to be invented. `Cost`/`HandleReq` unchanged. `gafira-render-cpu` can loop.
2. "Triples (backdrop, source, alpha)": read as backdrop pixel, source pixel and their two alphas.
3. "Clip channels to [0, 1]": all four inputs are clipped, and a colour channel above its alpha is capped to the alpha (so the straight colour is at most 1; needed to keep `D(x)`, `sqrt` and the division safe). In 8 bits the same cap applies before non-`Normal` modes.
4. A pixel with `a = 0` is treated as transparent black in both arguments and in every mode, including `Normal` (consistent with RS1's mip rule).
5. Non-finite check covers all eight inputs first, also for `a = 0` pixels; `Err` field is `"backdrop"`, `"source"` or `"result"` and the index is the channel. A non-finite result only arises in `Normal` (unclipped), which gives `Err`.
6. `Normal` out of range is not clipped (ARQ-119): output may leave [0, 1].
7. 8-bit values go into the formulas as `v / 255`, with no sRGB linearisation: the criterion compares with the `f64` formula on the same values.
8. `crates/gafira-raster/proptest-regressions/` is again untracked on disk (failure seeds from the mutant runs). Not committed; the RS1 coordinator note said so; I cannot delete it here.

Size: `blend.rs` is 840 lines (about 560 tests).

### Fix round 1 (RS2 verify round 1: C1 to C4, W2 to W5, S2)

Commits: 07dd1e2 (amended ARQ-119 and W3, with RED), then the gap-closing test commit. Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each, read alone. Tests in `gafira-raster`: 55 to 63.

- Amended ARQ-119 (RED first): a source with `a = 0` returns the backdrop bit for bit, an early return after the non-finite check, in `f32` and 8 bits, for all modes, with `(1,5; 0,5; 0,2; 1)`, `-0.0` and out-of-range backdrops. RED: `arq_119_transparent_source_leaves_the_backdrop_bit_for_bit` (`Normal` turned `-0.0` into `+0.0`; Multiply clipped 1,5). A NaN or infinity still gives `Err` (`arq_119_transparent_source_still_checks_for_non_finite`, not RED). When both pixels have `a = 0` the source rule wins; the old backdrop test no longer includes `a = 0` sources.
- W3 (RED first): the mixing result is clamped to [0, 1]. RED: `arq_119_mixing_result_is_clamped` (Saturation gave `-2.7755576e-17`). Checked first: the unclamped reference tables still pass with the clamp (max error unchanged), and the full 55 tests pass.
- C1: `arq_119_transparent_backdrop_gives_source` compares bits on 3 000 sources whose `c / a` is not exact (low mantissa bits scrambled; for plain `c = u * a` the f32 round trip returns `c` anyway, which is why my first attempt at the mutant survived).
- C2: `arq_119_source_colour_is_capped_to_its_alpha`. C3: `arq_110_overflowing_alpha_is_err`. C4: `arq_110_color_burn_with_negative_zero_source`; my claim (b) "equivalent" was wrong for `-0.0`: `(1 - cb) / -0.0` is `-inf`, `min(1)` keeps it and `1 - (-inf)` is `+inf`. The guard stays and is tested.
- W4: two proptests, `arq_110_interior_input_gives_output_in_unit_range` (inputs in [-2, 2], tiny values, [0, 1]) and `arq_119_valid_interior_input_stays_in_unit_range`. W5: the two table tests carry `// ARQ-119 (FD-12 c, CPU)`. S2: `index` doc in `error.rs`.
- W1 (colour space of blending): not touched. `blend_u8` still blends the stored byte values.

Mutants of this round (all killed): M1 (output rebuilt from an `f32` straight source, two variants) by the bit-exact backdrop test; M2a (`f32` source not capped) and M2b (8-bit source not capped) by the cap test; M3 (result check skips alpha) by `arq_110_overflowing_alpha_is_err`; ColorBurn without the `Cs == 0` guard by the `-0.0` test; early return removed (`f32` and 8 bits), placed before the finite check, or returning the clipped backdrop, by the bit-for-bit and still-checks tests; clamp removed by `arq_119_mixing_result_is_clamped` and the interior property; backdrop ghost colour kept by the backdrop test. Equivalent survivors, corrected: (a) ClipColor recompute and (c) the `Normal` arm of `separable` stand (verify confirmed); (b) is withdrawn.

Deviations: `proptest-regressions/` is still untracked on disk and not committed; I cannot delete it here. Lines of the round: `git diff --shortstat 2a2b517..HEAD -- crates/`.

## RS3

Status: partial. W6 and W7 of RS2's report are done (commit fe02abf). The transitions are blocked on the questions below; no transition code or stub was written.

### Done

- W6: `arq_119_transparent_source_leaves_the_backdrop_bit_for_bit` holds the 8-bit case of both pixels transparent (ghost colour in the backdrop). W7: `arq_110_finite_input_gives_finite_output` asserts the backdrop bit for bit at source alpha 0 and the range only above it; `arq_110_interior_input_gives_output_in_unit_range` assumes a non-zero source alpha. Not RED (the code was right); mutants: 8-bit early return skipped for a transparent backdrop, same returning the source, `f32` early return returning the clipped backdrop. All killed by the first test above. Gates exit 0.

### Blocked: what ARQ-118, 02 §4.13 and 02 §7.3 do not state

ARQ-118 gives the maths of `progress`, Cut, Dissolve, Fade, Slide offset, Zoom scales and Wipe mask. It leaves open what a `Transition` needs to exist, and C12 needs a schema per transition:

1. `ParamSchema` of each transition: ids, kinds, ranges, defaults, animability, labels, choice ids. 02 says only "dirección, color, suavidad". Needed: Fade colour (default black? "a negro, blanco o color"), Slide direction (4 ids), Wipe direction (4 plus circular) and softness (default, range [0, 1] by ARQ-118), Zoom (in or out? ARQ-118 fixes one: `from` 1 to 2, `to` 0.5 to 1; is "alejar" a second mode?).
2. Direction conventions (spike R-b named it, nothing closed it): for Slide, from which side `to` enters (left/right/up/down) and whether `lado` is the width for horizontal and the height for vertical; for Wipe, where `u = 0` is for each direction (left to right, right to left, top to bottom, bottom to top) and the centre of the circular wipe (image centre?).
3. Zoom resampling: the scale of `from` 1 to 2 and `to` 0.5 to 1 about which point (the centre?), and the filter (nearest or bilinear). ARQ-118 fixes none, and `p = 0` or 1 must be byte-exact.
4. Identifiers and display: `transition_id` strings and `MessageKey` names for the `Descriptor` of each of the six, and their `version`.
5. `HandleReq` for each transition (`Both`, `From`, `To`, `None`; 02 gives the type, not the values per transition).
6. Pixel formats: which of `Rgba8`, `RgbaF32`, `Alpha8` each transition accepts, and the error for a mix (from `Rgba8`, to `RgbaF32`, destination `Alpha8`). ARQ-118 tests "por formato".
7. `EffectError`: 02 §7.3 states the `BufferError` wrapper only. The typed `Err` variants for a dimension mismatch (ARQ-118) and for a non-finite or invalid `ParamValues` (ARQ-110, `ParamError`) are not named. `BufferError` has no mismatch variant.
8. Dissolve and Fade on premultiplied values: the formula is stated, and mutant 10 reads "colour operations without premultiplying". Confirm that Dissolve and Fade interpolate the premultiplied RGBA channels directly (the only linear crossfade that keeps alpha), and in which space 8-bit pixels are mixed (stored sRGB bytes, as `blend_u8`, or linear light as the mip levels; W1 of RS2 is the same open question).
9. `Rgba8` rounding of Dissolve and Fade (round to nearest?).

Hand mutants 1 to 11 for the transitions are not run: there is no code.

## RS3a

Branch `feat/gafira-raster` (`develop` merged). Commits: 54d8e7b (`EffectError`), 8059490 (contract, guard, Cut), 09847aa (mix, Dissolve, Fade). Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each, read alone. `gafira-raster` tests: 63 to 95. Order: stubs of the signatures, then tests, then RED, then code; the RED of each unit is in its commit body. The earlier W6 and W7 items (fe02abf) stand.

### Done

- `EffectError { Buffer, Param, SizeMismatch, FormatMismatch, UnsupportedFormat }` with `From` for `BufferError` and `ParamError`, `to_message` delegating, keys `error.raster.size_mismatch|format_mismatch|unsupported_format` (sizes in `Count`, formats in `Text`).
- `gafira_raster::transitions`: `Transition`, `GpuTransition`, `UniformBlock`, `HandleReq`; `Cut`, `Dissolve`, `Fade`. Duration stays out of the crate (Q-47).
- Shared guard in the P15 order, all before any write: `progress` finite; `schema.resolve`; formats (`Alpha8` gives `UnsupportedFormat`, a mix gives `FormatMismatch` naming `to` then `dst`); sizes (`to` then `dst`); `f32` pixels of `from` and `to` finite; size 0 gives `Ok`; clip; exact copies at 0 and 1.
- Mix: `MIX_SPACE = Linear`, an arm per format for each of the three spaces, a weight of exactly 1 returns the pixel unconverted, a pixel with `a = 0` adds no colour, `Rgba8` rounds half up and clamps. The linear `Rgba8` arm mixes premultiplied linear colour and alpha, then encodes the straight colour and multiplies by the mixed alpha once before rounding (c <= a survives). `Transfer` moved from `mip.rs` to `transfer.rs` (`pub(crate)`, shared tables built once in a `OnceLock`).

### RED per criterion (stubbed bodies, tests first)

| Criterion | Failing test |
|---|---|
| 02 §7.3 wrapper, delegation | `effect_error_wraps_buffer_and_param_errors_and_delegates` (left `UnsupportedFormat`, right `Buffer(NonFinite progress)`); `effect_error_new_variants_have_keys_and_arguments` |
| ARQ-118 ends, clip | `arq_118_ends_are_exact_copies_for_every_transition` |
| ARQ-110 `progress`, pixels, params | `arq_110_non_finite_progress_is_err_and_writes_nothing`, `arq_110_non_finite_pixels_are_err_and_write_nothing`, `arq_110_fade_rejects_a_bad_colour_before_writing` |
| ARQ-118 formats, sizes, size 0 | `arq_118_formats_are_checked_before_writing`, `arq_118_sizes_are_checked_before_writing`, `arq_118_size_zero_is_ok_after_the_checks` |
| Cut | `arq_118_cut_switches_at_exactly_one_half`, `cut_has_the_identity_and_schema_of_the_table` (handles `None`) |
| Dissolve | `arq_118_dissolve_at_a_quarter_over_zero_and_one_is_a_quarter`, `arq_118_dissolve_8_bit_rows_follow_the_mix_space`, `dissolve_has_the_identity_and_schema_of_the_table` |
| Fade, C12 | `arq_118_fade_at_one_half_is_the_colour_exactly`, `arq_118_fade_halves_use_2p_and_2p_minus_1`, `fade_has_the_identity_and_schema_of_the_table` (calls `validate_definition`; Cut and Dissolve have the same test with the empty schema) |
| Near the ends | `arq_118_dissolve_and_fade_stay_within_1_255_of_the_ends_near_them` |
| Mix | `a_sample_of_weight_one_is_returned_as_it_is`, `midpoint_rows_per_space_and_format` (the O5 table beside the constant), `a_transparent_pixel_adds_no_colour`, `rgba8_rounds_half_up_and_clamps` |

Not RED, proved by mutants: unknown parameter ids ignored, trait defaults (`gpu()` none, `audio()` equal power), height-alone and format-before-size, determinism (ARQ-111) and the premultiplied-validity properties.

### Hand mutants (killed unless noted)

| # | Mutants | Killed by |
|---|---|---|
| 1 | NaN not `Err`; `±∞` not `Err`; no clip upper or lower | the `progress` test, `arq_118_ends_are_exact_copies_for_every_transition` |
| 2 | Weight-1 path converting; end copy dropped or swapped | `a_sample_of_weight_one_is_returned_as_it_is`, the ends test. The guard's own `p == 0` and `p == 1` branches are equivalent for these three (weights `(1, 0)` and `(0, 1)` already return the pixel exactly); they matter for RS3b |
| 3 | `<=` at 0.5, swapped, threshold 0.4999 | `arq_118_cut_switches_at_exactly_one_half` |
| 4 | Dissolve weights swapped, `from`/`to` swapped, constant weights | 8-bit rows (137/225 against 64/191), the 0.25 test, the near-ends test |
| 5 | Fade `2p` and `2p - 1` branches wrong, weights swapped, colour not premultiplied, alpha dropped, colour not converted, default white, animatable | the half and quarter tests, `fade_has_the_identity_and_schema_of_the_table` |
| 9 | No size check on `to` or `dst`; width only; no format check; `Alpha8` allowed; pixel check skipped; size 0 before the checks; sizes before formats; parameters not resolved; `dst` written before `Err` | the size, format, pixel, size-0 and `color` tests |
| 10 | Linear arm without unpremultiply, without premultiplied output, alpha dropped; ghost colour kept | the mix tests |
| 11 | `RandomState` entropy in Cut, Dissolve, Fade | determinism and value tests |
| Arms | `Linear` `Rgba8` as stored; `Encoded` `RgbaF32` as stored; `Encoded` `Rgba8` through the linear arm; `MIX_SPACE` changed; rounding `floor` | `midpoint_rows_per_space_and_format`, `mix_space_is_linear_until_the_owner_answers`, the rounding test. Swapping arms between spaces does not compile (the `match` is exhaustive) |

Equivalent survivors: Fade `p <= 0.5` and a threshold at 0.4999 (the function is continuous at one half, both branches give the colour there); Fade colour alpha clamp to 2 and the final byte clamp (Rust `as u8` saturates); the guard's `p == 0` and `p == 1` branches for these three (see 2).

### Deviations and assumptions

1. P15 names `weighted(samples: &[(Pixel, f64)])`. I wrote one typed function per format, `weighted_u8` and `weighted_f32` (plus `_in(space, ..)` for the tests), because a mixed-format sample list cannot occur and would need an impossible arm.
2. The colour alpha of Fade is clipped to `[0, 1]` in both formats (P15 says "its own alpha is kept"; an alpha outside the range has no meaning). Display-P3 and other wide-gamut colours keep their out-of-gamut values in `f32` and clamp in bytes.
3. `MixSpace::ALL` is checked by a `debug_assert!` in `weighted_u8` so that all three variants are constructed in non-test code (no `#[allow]` for dead code).
4. XP13 (time `weighted` on 2 Mpx) is not done: no bench was asked for in this delivery.
5. The `Transition` for `Cut` has `HandleReq::None` as in the table; the others `Both`. Q-47 is untouched: no duration handling.
6. `crates/gafira-raster/proptest-regressions/` is untracked again; not committed.

Lines: `git diff --shortstat baf5e59..HEAD -- crates/gafira-raster` (1 992 added, about 1 100 of them tests).

### Fix round 1 (RS3a verify round 1: C1 to C3, W1 to W5, S1, S2, S5, ARQ-31)

Commits: 219ffb7 (cx.scale, order, Fade colour, Fade boundary, alpha clip), 9d54263 (Encoded finite, no global state), 2f0480f (XP13 bench), then this note. Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each, read alone. The first of them was committed once with a clippy failure (too many arguments in a test helper) and amended before anything else; the final commit passes. `gafira-raster` tests: 95 to 104.

- W4, P15 step 1 (RED first): a non-finite `cx.scale` gives `Buffer(NonFinite { field: "cx.scale", index: 0 })` before every other check; the steps are renumbered as in the amended P15. RED: `arq_110_non_finite_cx_scale_is_err_first_and_writes_nothing` (unwrap_err on Ok). A finite scale of any sign or size is no error.
- C1 (RED first): Fade converts its colour for the format in use and returns `Buffer(NonFinite { field: "color", index })` if the conversion overflows, before any write. RED: `arq_110_fade_colour_that_overflows_is_err_and_writes_nothing` (`Srgb` 1e30 at 0.25 wrote `[inf, 0.0005, -0.1245, 0.6505]`). The colour is built only when it is used (mid `progress`), so at 0 and 1 the copy is still `Ok`. Bytes: `arq_110_fade_colour_that_overflows_is_err_in_bytes_too` (not RED: the bytes path already clamped; it holds only once the conversion check exists, so it is a gap-closing test for that check, killed by removing it).
- W3 (RED first): the `Encoded` arm for `f32` works in `f64` and clamps to the `f32` range. RED: `every_space_stays_finite_on_finite_pixels` (gave `inf`) and the property `f32_result_is_finite_in_every_space` (gave `-inf`). The finite property now covers all three spaces.
- ARQ-31: no `static` is left in the crate. `Transfer::new()` runs once per `run_cpu` that mixes and is passed down (`weighted_u8(&t, ..)`), at the price of building 256 table entries per call. The unused `f32` transform methods are gone; the `Encoded` arm has its own sign-odd `f64` curve in `Transfer` (a second copy of the sRGB formula of `gafira-color`, which keeps its own private).
- Not RED (gap-closing, proved by mutants): C2 `p15_order_of_the_checks_with_two_faults_at_once`; C3 `arq_118_fade_boundary_is_exactly_one_half` (public API, `[0.4999, 0.5)` and `(0.5, 0.5001]`); W2 `arq_118_fade_colour_alpha_is_clipped_in_f32`; `out_of_range_colours_keep_their_sign_in_every_space` (sign mutants of the `f64` curve survived the first tests).
- W1: `crates/gafira-raster/benches/transitions.rs`, `bench_dissolve_rgba8_1080p`, a criterion bench; nothing times anything in `cargo test`. One run: 161 ms per frame (verify: 154 to 156 ms), against the 8 ms share. My earlier deviation "no bench was asked for" was wrong: XP13 asks for it.
- S1: the `debug_assert!` over `MixSpace::ALL` stays, with a comment that says it exists so that the arms of the other O5 answers stay constructed without `#[allow]`. S5: the comment cites 02 §7.3, not a line. S2: the `RS3a` name is in the commit subjects and headings; the Slice card still says `RS3` and I do not edit it.
- Kept: `from` `Rgba8` with `to` `Alpha8` is `FormatMismatch` (S4 goes to `specifier`).

Mutants of this round (all killed): I1 (progress after parameters), I2 (parameters after sizes), I3 (pixels before sizes), scale unchecked, scale checked for NaN only, scale after progress: `arq_110_non_finite_cx_scale_*`, `p15_order_*`. I4 (Fade threshold 0.4999) and I4b (0.5001): `arq_118_fade_boundary_is_exactly_one_half`. I5 (`f32` colour alpha clamp 2) and I5b (no lower clip): `arq_118_fade_colour_alpha_is_clipped_in_f32`. C1: `f32` check removed, `u8` check removed, wrong field: the two overflow tests. W3: `encode_f64` constant, sign dropped in encode and decode, decode exponent, no clamp, alpha 0 not zeroed, premultiply dropped: the mix tests. Equivalent: Fade `p <= 0.5` (confirmed by verify) and the guard's `p == 0` / `p == 1` branches for Cut, Dissolve and Fade.

### RED record, corrected (W5)

The stubs, in full, and what came after the code:
- `54d8e7b`: `From<BufferError>` and `From<ParamError>` returned `UnsupportedFormat { format: Alpha8 }`, `to_message` returned the key `error.raster.stub`, and `Display` wrote nothing. With those three stubs all three tests fail; the body did not name the `Display` stub.
- `8059490`: `plan` returned `Ok(Plan::Done)`, `Cut::run_cpu` returned `Ok(())`, and `Cut::handles` returned `Both`. The mix module did not exist in that commit. `arq_118_height_alone_differs_and_format_is_checked_before_size` was written after the code, to kill survivors, so it was not RED, and it was listed as such; `arq_111_transitions_are_deterministic` was RED (it compares `dst` after the stub left it untouched) and my note listed it as not RED in error.
- `09847aa`: `Dissolve::handles` and `Fade::handles` returned `None`, both `run_cpu` copied `from` for any `progress`, the `Fade` schema was empty, and the mix functions returned the first sample. `dissolve_has_the_identity_and_schema_of_the_table` fails only because of the `None` handles, which the body did not state. The mix tests were run RED before the guard and transitions existed; a later edit loosened one assertion (the near-ends test) before the code, while it still failed.

Deviations of this round: `proptest-regressions/` is untracked again and not committed (S3 is the coordinator's decision); XP13 on this machine reads 161 ms, so the table of O5 stays as the design records it. Lines: `git diff --shortstat dc6fced..HEAD -- crates/gafira-raster`.

## RS3b

Branch `feat/gafira-raster` (merged at fadac62 plus this work). Commits: bbd49c5 (Slide, W6), 3b316f6 (Wipe), f78c829 (Zoom). Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each, read alone, and checked before each commit. `gafira-raster` tests: 104 to 130. Order per transition: the stub, the tests, RED, the code. The stub of each is stated exactly in its commit body.

### Done

- `Slide`, `Wipe`, `Zoom` in `gafira_raster::transitions`, each with the id, name key, version and `HandleReq::Both` of the 02 §4.13 table; every schema passes `validate_definition` in its own test (C12) with the ids, kinds, defaults, option labels and `animatable: false` of the table.
- Slide: `d = round((1 - p) * side)` in `f64` (halves away from zero), `left` shows `to(x - d)` for `x >= d`, `right` shows `to(x + d)` for `x < W - d`, `up` and `down` on `y` with the height; replaces pixels.
- Wipe: `m = clamp((p (1 + s) - u) / s, 0, 1)`, `u < p` at `s = 0` with no division, `u` at the pixel centre, circular from `(W / 2, H / 2)` over half the diagonal; defaults `right` and `0`.
- Zoom: `in` (`from` `1 + p`, `to` `(1 + p) / 2`) and `out` (`1 - p / 2`, `2 - p`), pivot `(W / 2, H / 2)`, `s = c + (x + 0.5 - c) / scale - 0.5`, four taps on premultiplied channels, edge clamp, 8 samples in one call of the shared mix.
- W6: `the_f64_curves_agree_with_the_analytic_transform` sweeps `encode_f64` and `decode_f64` against `AnalyticTransform` from -2 to 3 with the joins inside, plus oddness and round trip.
- `guard.rs`: `Frames` and `render` for transitions that read pixels at other positions.

### RED per criterion (stubs as in the commit bodies)

| Criterion | Failing test |
|---|---|
| Slide table, bad direction, 10x4 acceptance, rounding, replace, f32 bits | `slide_has_the_identity_and_schema_of_the_table`, `arq_110_slide_rejects_an_unknown_direction`, `arq_118_slide_acceptance_rows_on_10_by_4`, `arq_118_slide_offset_rounds_half_away_from_zero`, `arq_118_slide_replaces_and_does_not_composite`, `arq_118_slide_copies_f32_pixels_bit_for_bit` |
| Wipe | `wipe_has_the_identity_and_schema_of_the_table`, `arq_110_wipe_rejects_bad_parameters_before_writing`, `arq_118_wipe_acceptance_rows_on_4_by_2`, `arq_118_wipe_edge_is_strict_and_at_the_pixel_centre`, `arq_118_wipe_soft_edge_values`, `arq_118_wipe_matches_the_criterion_for_every_direction`, `arq_118_wipe_circular_centre_and_half_diagonal`, `arq_118_wipe_defaults_are_right_and_hard`, `arq_118_wipe_hard_edge_copies_pixels_and_soft_edge_mixes_premultiplied`, `arq_118_wipe_soft_stays_within_1_255_of_the_ends_near_them` |
| Zoom | `zoom_has_the_identity_and_schema_of_the_table`, `arq_110_zoom_rejects_an_unknown_direction`, `arq_118_zoom_ramp_values_for_in_and_out`, `arq_118_zoom_pivot_is_the_continuous_centre`, `arq_118_zoom_filters_premultiplied_values`, `arq_118_zoom_matches_the_criterion_tap_by_tap`, `arq_118_zoom_out_mirrors_zoom_in` |

Not RED, proved by mutants: the six transitions in the generic tests of `mod.rs` (ends, sizes, formats, pixels, determinism; any copy of `from` satisfies them), `arq_118_wipe_ends_are_exact_with_any_softness`, `arq_118_zoom_constant_images_stay_constant_and_opaque_images_opaque`, and W6.

### Hand mutants (killed unless noted)

| Mutants | Killed by |
|---|---|
| Slide: `from` moves; offset floor, ceil, half-even; `(1 - p)` reversed; `d` on the wrong axis (width for all, height for all); `x > d`; `right` off by one; up and down swapped; left and right swapped; default `right`; composite instead of replace | the slide tests above |
| Wipe: `u` not at the centre; `s = 0` with `<=`; no `s = 0` branch (division by 0, NaN at `u == p`); half diagonal as the full one or the longer side; circular centre off; left, down, up wrong; softness default 0.5; direction default `left`; weights swapped; `(1 + s)` dropped; no low clamp; end copy swapped; entropy | the wipe tests, `arq_118_ends_are_exact_copies_for_every_transition` |
| Zoom: `in` and `out` ranges reversed; each scale wrong; pivot at the index centre or the origin; no `+ 0.5`; no `- 0.5`; scale multiplied instead of divided; nearest instead of bilinear; tap weights swapped; edge transparent (out-of-range tap) instead of clamp; straight (unpremultiplied) filtering; `from` and `to` weights swapped; default `out`; entropy | the zoom tests; the edge-transparent mutant also fails the generic tests (an out-of-range index panics) |
| W6: `encode_f64` threshold, decode slope, encode offset | `the_f64_curves_agree_with_the_analytic_transform` |

Equivalent survivor: the decode join `0.04045` moved to `0.04` (the curve is continuous there; the change is below the 1e-5 of the test). Ends: the guard copies at 0 and 1 for all six; Slide and Zoom depend on them (no mix reaches the exact end), and the tests fail if they are removed.

### Deviations and assumptions

1. P15 item 3 leaves open what happens to `to` pixels exactly at `p` near 0 for Zoom (no near-end bound is stated for Zoom, Slide, or hard Wipe, only for Dissolve, Fade and soft Wipe); I tested none. A 1080p frame sampled at `p = 0.0001` moves up to about 0.1 pixel.
2. The test reference for bytes uses `weighted_u8_in` with the taps written in the test, so it checks geometry and weights, not the mixing space (O5).
3. `Slide` with `lado` 1 and `p` below 0.5 shows `from` entirely (d = 1), above shows `to` (d = 0).
4. `proptest-regressions/` is untracked and not committed.

Lines: `git diff --shortstat fadac62..HEAD -- crates/gafira-raster`.

