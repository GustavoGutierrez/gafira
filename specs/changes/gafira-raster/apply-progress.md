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

Survivor below tolerance: the decode join `0.04045` moved to `0.04` (the curves differ by up to 4.8e-7, under the 1e-5 of the test, in an arm not in use). Ends: the guard copies at 0 and 1 for all six. Corrected in fix round 1: removing them is an equivalent mutant for all six, not only for Cut, Dissolve and Fade (at `p` 0 Slide has `d` = side, at 1 `d` = 0; Zoom has scale 1 and a tap of weight exactly 1; Wipe saturates), so the claim here that Slide and Zoom depend on them and that the tests fail without them was false. The branches stay because P15 step 8 asks that no conversion runs at the ends.

### Deviations and assumptions

1. P15 item 3 leaves open what happens to `to` pixels exactly at `p` near 0 for Zoom (no near-end bound is stated for Zoom, Slide, or hard Wipe, only for Dissolve, Fade and soft Wipe); I tested none. A 1080p frame sampled at `p = 0.0001` moves up to about 0.1 pixel.
2. The test reference for bytes uses `weighted_u8_in` with the taps written in the test, so it checks geometry and weights, not the mixing space (O5).
3. `Slide` with `lado` 1 and `p` below 0.5 shows `from` entirely (d = 1), above shows `to` (d = 0).
4. `proptest-regressions/` is untracked and not committed.

Lines: `git diff --shortstat fadac62..HEAD -- crates/gafira-raster`.

### Fix round 1 (RS3b verify round 1: C1, C2, W1, W2, W4, S2, S3)

Commits: 30769fb (Wipe mask), 3b9e626 (tests), then this note. Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each, read alone and checked before each commit. `gafira-raster` tests: 130 to 136.

- C1 (RED first, no stub: the old code): the mask `clamp((p (1 + s) - u) / s, 0, 1)` is now `clamp(p + (p - u) / s, 0, 1)`, taking the quotient only while `|p - u| <= s`; beyond that the mask has saturated. No cancellation, and no division of a normal number by a subnormal. RED: `arq_118_wipe_tiny_softness_keeps_the_edge_pixel_at_p` (s = 1e-10, p 0.4375: pixel 3 was 0.43750003) and `arq_118_wipe_mask_equals_exact_integer_arithmetic_for_dyadic_softness` (k=49, p=0.53125, w=16, x=8: 0.5 against 0.53125). The first is hand-worked on 8 columns (pixel 3 has `u = p`) for 16 softnesses from 1e-2 to 5e-324, in four directions and circular; the second uses exact `i128` arithmetic for every `s = 2^-k`, k up to 60, and shares no expression with the code.
- W1: with the saturation test, no division by a subnormal reaches infinity. Observable output is the same as before away from `u == p`, so dropping the test is an equivalent mutant at the output; the code analysis above is the evidence, and the 5e-324 row of the hand table covers the result.
- C2: `arq_118_slide_offset_is_computed_in_f64` (side 5 at 0.1 gives `d` 4; 10 at 0.05 gives 9; 10 at 0.55 gives 4; 7 at 0.3 gives 5; both axes). Not RED.
- W2: `arq_111_every_configuration_is_deterministic` and the proptests `every_configuration_keeps_premultiplied_pixels_valid` and `every_configuration_gives_finite_output_for_finite_input` run Slide in four directions, Wipe in five directions at softness 0, 0.4, 1 and 1e-16, Zoom `in` and `out`, Cut, Dissolve and Fade through `run_cpu`. Not RED.
- S3: the ends test carries `FV-06 (a)` in its comment.
- Not touched: S1 (`samples` allocates a `Vec` per pixel; XP13 and O5 own the cost), the Slice card and `proptest-regressions/`.

Mutants of this round (all killed unless noted): Wipe softness below 1e-9 treated as hard (I2), the old expression, flipped saturation, flipped gap, `p` dropped from the ramp, no clamp; Slide `d` in `f32` (I1); entropy in the soft ramp and in Zoom `out`; a soft Wipe pixel that breaks `c <= a`; a Zoom `f32` pixel divided into NaN. Equivalent: the saturation test dropped (the infinity is clamped) and `>=` for `>` at `|gap| == s`.

Deviations: none new. P15 as amended (d67674b) binds near-end bounds only to Dissolve, Fade and Wipe with `s = 1`; this delivery adds none for Slide, Zoom, Cut or the other softnesses. Lines: `git diff --shortstat 608b196..HEAD -- crates/gafira-raster`.


### Fix round 3 (RS3b verify round 2: C1 on `circular`, C3)

Commit: the Wipe fix with its tests (one commit, tests written and seen RED first), then this note. Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0 each, read alone. `gafira-raster` tests: 136 to 138.

- Cause: circular `u` divided two `hypot` results, each rounded. Now `u = sqrt((a^2 + b^2) / (w^2 + h^2))` with `a = 2 cx - w`, `b = 2 cy - h` (exact in `f64`), one quotient and one sqrt; an exact `u == p` stays exact. Axis path, the saturation test and the non-finite guarantees (W1) are untouched.
- RED (before the production change): `arq_118_wipe_circular_exact_edge_pixels_keep_the_rule`, "hard 8x8 p 0.875 pixel (0,0)", left 1.0, right 0.0; `arq_118_wipe_circular_exact_edge_on_1080_square`, "hard i=276979", left 1.0, right 0.0. The first also scans every exact-edge pixel on squares 4 to 64 (found by `i128` arithmetic, at least 88) for hard 0 and soft (`s` 1e-16) `p`, and the 8x8 hand case for `s` 1e-16 to 1e-3. The second covers C3 at 1080x1080, p 3/8.
- Hand mutants: `u <= p` in the hard Wipe, killed by `arq_118_wipe_defaults_are_right_and_hard`, `arq_118_wipe_edge_is_strict_and_at_the_pixel_centre` and both new tests; two-`hypot` quotient restored, killed by both new tests; ramp quotient taken regardless of `|p - u| <= s`, SURVIVES: equivalent at the output (the quotient is then at worst infinite, `p + inf` or `p - inf` is clamped to 1 or 0, never NaN since `gap` is finite), as recorded in round 1 for W1; clamp removed, killed by `arq_118_wipe_circular_centre_and_half_diagonal`, `arq_118_wipe_mask_equals_exact_integer_arithmetic_for_dyadic_softness`, `arq_118_wipe_soft_edge_values`, `arq_118_wipe_matches_the_criterion_for_every_direction`.
- W5 not touched (follow-up). `proptest-regressions/` not committed. Deviations: none. Lines: `git diff --shortstat HEAD~2..HEAD -- crates/gafira-raster` (69 insertions, 2 deletions in `wipe.rs`).

## RS4

Branch `feat/gafira-raster`, from 07ba7d3 (the amend of ARQ-110 and ARQ-120 that gives `Effect::roi` and `cost_hint` a `cx`). Commits: 3d400cd (`Effect`, `GpuEffect`, `Cost`), e2358e1 (blur and Paralela, with tests), 5243b16 (`bench_shadow_1080`). Gates (fmt, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each, read alone. Effects get no `CancelToken` (ARQ-112): no cancel checkpoint.

Size: 1 660 lines added in 6 files (`git diff --shortstat 07ba7d3..HEAD -- crates`), of which about 1 590 in `src/effects/` and about 1 000 are tests (rustfmt expands the test tables). Forecast was 420 (590): over the ~700 mark; the coordinator decides.

### Done

`effects::{Effect, GpuEffect, Cost, ParallelShadow}`. `GpuEffect` copies the spec signature (no `cx` in `bind`: recorded follow-up). `Cost { units: u64 }`. `ParallelShadow` (id `effect.shadow.parallel`): `offset_x`, `offset_y` (-500..500), `blur` (0..100), `opacity` (0..100), `color`; output is the element over the shadow, `Rgba8` and `RgbaF32`; `Alpha8` gives `UnsupportedFormat`. Blur: direct separable kernel, tap-outer loops over rows, truncated at `ceil(3 sigma)` with `sigma = radius / 2`, normalised, zero outside, no per-pixel allocation (P11). `roi` = `out` joined with `out` moved by minus the offset and grown by the taps. Check order in `run_cpu`: `cx.scale`, parameters, formats, sizes, `f32` pixels, colour overflow, size 0.

### RED per criterion (stub: schema empty, `run_cpu` returned `Ok(())`, `roi` returned `out`, cost 0)

| Criterion | Failing test |
|---|---|
| ARQ-120 sigma, edges, identity, 3 sigma | blur: `arq_120_impulse_response_has_sigma_of_half_the_radius` ("sigma NaN, want 5"), `arq_120_matches_the_reference_gaussian_with_transparent_outside` ("0 vs 0.1065"), `arq_120_constant_image_keeps_its_value_inside_and_fades_at_the_edge`, `arq_120_radius_zero_is_the_identity`, `arq_120_half_width_is_three_sigma_rounded_up` ("radius 10", left 0, right 15) |
| ARQ-120 roi | `arq_120_roi_grows_the_output_by_the_scaled_kernel` (left = out, right = out grown) |
| ARQ-120 scale, tiles | `arq_120_radius_and_offset_are_multiplied_by_the_scale`, `arq_120_a_tile_with_its_roi_matches_the_whole_image` (the stub left the sentinel; guard added) |
| FD-35 (a) | `fd_35_a_blur_zero_shadow_is_the_moved_alpha_times_opacity`, `fd_35_offset_signs_move_the_shadow_right_and_down`, `fd_35_the_element_is_composited_over_the_shadow` |
| FD-35 (d) | `fd_35_d_extremes_pass_and_just_outside_is_err` (`Ok` instead of `Err`), `fd_35_d_opacity_zero_does_not_change_the_image` (sentinel left), `fd_35_schema_has_the_parameters_ranges_and_a_valid_definition` (panics on the empty schema) |
| ARQ-110 | `arq_110_non_finite_parameter_is_err_naming_it_and_writes_nothing`, `arq_110_non_finite_scale_is_checked_first_and_writes_nothing`, `arq_110_checks_run_in_order_params_formats_sizes_pixels`, `arq_110_non_finite_source_pixel_is_err_and_writes_nothing`, `arq_110_cost_hint_never_falls_with_twice_the_pixels`, `arq_110_finite_input_gives_finite_output` |
| ARQ-111 | `arq_111_two_runs_give_the_same_bytes` (after the sentinel guard) |
| 8-bit composite | `fd_35_the_8_bit_composite_equals_the_normal_blend` ("[0,0,255,0] under [0,0,0,5]"), RED against a stub `over_u8` |

Not RED, proved by mutants: `arq_110_unknown_ids_are_ignored` (before its guard), `arq_110_roi_and_cost_hint_do_not_panic_at_the_extremes` (a no-panic property), `arq_120_a_half_size_image_at_half_scale_matches_after_rescaling` (after the guard it fails on the stub), the blur `shifted_origin` and `tile` tests, `fd_35_the_colour_tints_the_shadow_and_its_alpha_scales_it` (added for a surviving mutant).

### Hand mutants (all killed)

| Mutant | Killed by |
|---|---|
| sigma = radius | `arq_120_impulse_response_has_sigma_of_half_the_radius`, `arq_120_matches_the_reference_gaussian_*` |
| radius 0 not the identity (one tap) | `arq_120_radius_zero_is_the_identity`, `arq_120_half_width_*`, `arq_120_shifted_origin_*` |
| outside is the clamped edge | `arq_120_matches_the_reference_gaussian_*`, `arq_120_constant_image_keeps_its_value_*` |
| radius not multiplied by scale | `arq_120_radius_and_offset_are_multiplied_by_the_scale`, `arq_120_roi_grows_*` |
| `roi` short by one tap; one side short | `arq_120_roi_grows_*`, `arq_120_a_tile_with_its_roi_matches_the_whole_image` |
| `roi` ignores scale | same two |
| offset sign flipped (run, roi x, roi y) | `fd_35_offset_signs_*`, `fd_35_a_*`, `arq_120_roi_grows_*`, the tile test |
| shadow over the element, `u8` and `f32` | `fd_35_the_element_is_composited_over_the_shadow` |
| scale check after params / after format / not at all | `arq_110_non_finite_scale_is_checked_first_and_writes_nothing` |
| params resolved after the format check / after sizes | `arq_110_checks_run_in_order_params_formats_sizes_pixels` and the parameter table |
| `f32` pixel check removed / NaN only | `arq_110_non_finite_source_pixel_is_err_and_writes_nothing` |
| others: Alpha8 accepted, unknown id read, 2 sigma, floor, no normalisation, opacity not /100, `roi` without `out`, offset not scaled, cost ignores kernel, no transparent-shadow skip, colour alpha ignored (first survivor), colour not linear in `f32`, five `over_u8` variants | named tests above |

### Sweep (throwaway test, not committed)

2 895 configurations: sizes 1x1, 1x5, 5x1, 2x2, 3x7, 17x13, 64x3, 40x31; scales 0.5, 1, 2; offsets 0, +-500, +-1, 499.99, mixed; blur 0, 0.01, 1, 7.5, 100; opacity 0, 1, 50, 100; both formats. Each: finite `f32` output, two runs equal, `u8` and `f32` alpha within 2.5/255, opacity 0 identity, integer reference for blur 0 at scale 1, five tile splits per configuration (bit-exact in `f32`, <= 1 in `u8`), `roi` contains `out`; plus constant opaque images unchanged. No finding.

### Bench

`bench_shadow_1080` (`benches/shadow.rs`, `Rgba8` 1920x1080, radius 40 px, offset 8): 78.8 ms (78.3 to 79.2) single thread on the prototype machine, budget < 100 ms. The blur alone measured 47 ms. The first version, using `blend_u8` per pixel, took 117 ms, so the 8-bit composite is `over_u8`, proved equal to `blend_u8(Normal)` by test. Not measured: non-x86.

### Deviations and assumptions (spec silent; verify may check)

1. Offsets are in logical px and are multiplied by `cx.scale`, rounded half away from zero to whole device pixels.
2. Defaults: offset 4, 4; blur 8; opacity 50; colour black. All five parameters animatable. Ids of the descriptor and label keys are mine.
3. The kernel half width is capped at 4096 taps (`MAX_HALF_WIDTH`), so a huge `cx.scale` cannot ask for a huge allocation; `roi`, cost and pixels use the same cap.
4. Where the shadow is zero the pixel is the element as it is (so opacity 0 is byte-exact, ghost colour of a transparent pixel included); where the element is transparent and the shadow is not, the result is the shadow.
5. The colour parameter's alpha scales the shadow. `Rgba8` uses the sRGB bytes of the colour, `RgbaF32` linear light.
6. For `Rgba8` the alpha plane is blurred in 0..255 units so that blur 0 is exact.
7. `roi`, `cost_hint` read missing, wrong-typed or non-finite numbers as 0 and a non-finite scale as 0; no value is pinned for them.
8. A non-finite value cannot be seen by a mutant that checks `cx.scale` after computing but before writing (nothing differs in output): equivalent.
9. `proptest-regressions/` untracked and not committed.

### RS4 fix round 1 (verify round 1: C1, C2, C3, W3, W4, W5, S1, S3, W1 and W2 test)

Commits: d499754 (bench headers, S3), cd16756 (code and tests), plus a test-only commit for the amended scale 0.5 offsets (3 and -3 move 2 and -2 px, spec commit 249789a). Gates (fmt, clippy `-D warnings`, test `--workspace --locked`), each exit code read alone, are 0 on each commit. Lines: `git diff --shortstat 271f7c6..HEAD -- crates` is about 240 insertions and 40 deletions, mostly tests.

Changes: C1 `kernel` uses `[s2/2, 1 - s2, s2/2]` (s2 = sigma squared) for sigma < 0.6, outer taps 0, `half_width`, `roi` and cost unchanged. C2 `half_width` returns 0 unless `radius * radius > 0`. S1 `MAX_HALF_WIDTH` is 4800 (1.5 x blur 100 x zoom 32). W4 `cover * opacity * tint.alpha / 100.0` in both formats. S3 the two bench headers no longer mention a nightly job.

#### RED (W5: every new test written first, run against the code as it stood, no stub)

| Item | Test | Result before the fix |
|---|---|---|
| C1 | `arq_120_sigma_holds_for_small_radius_times_scale` | FAILED: "radius 0.25: sigma 1.59e-7, want 0.125" |
| C2 | `arq_110_a_radius_whose_square_underflows_is_the_identity` | FAILED: `half_width(1e-170)` left 1, right 0 |
| C2 | `arq_110_a_blur_too_small_to_square_is_the_same_as_blur_zero` | FAILED: `Rgba8` differs from blur 0 (shadow lost) |
| S1 | `arq_120_the_cap_covers_blur_100_at_zoom_32` | FAILED: left 4096, right 4800 |
| W4 | `fd_35_a_opacity_ties_round_up_exactly` | FAILED: "alpha 50 opacity 29" left 14, right 15 |
| C3 | `arq_110_composite_overflow_is_err_and_dst_is_untouched` | Green at HEAD (the code was right; missing test). Killed by mutant M3 below. First draft failed on `r.is_err()` because the default `offset_y` 4 moved the shadow off the image; fixed in the test, not the code. |
| W3 | `fd_35_offsets_round_half_away_from_zero` | Green at HEAD (rule already implemented). Killed by M4. |
| W1, W2 | `arq_120_a_half_size_image_at_half_scale_matches_after_rescaling` (smooth raised-cosine bump, blur 16, 24, 32, even offsets) | Green at HEAD and after; rewritten to the amended criterion. |

One existing test changed: `arq_120_radius_zero_is_the_identity` asserted bit identity for radius 1e-12. The variance-preserving kernel keeps variance 2.5e-25, so it now asserts the identity within 1e-6. Spec says only radius 0 is the identity. Deviation, recorded.

#### Hand mutants (each reverted, none committed)

| Mutant | Killed by |
|---|---|
| kernel side taps `sigma^2` instead of `sigma^2 / 2` | `arq_120_sigma_holds_for_small_radius_times_scale` |
| radius squared guard removed | `arq_110_a_radius_whose_square_underflows_is_the_identity` (`half_width(1e-170)`). The shadow-level C2 test does not catch it any more, because the new kernel is NaN free (sides underflow to 0): the guard is now only the identity shortcut. |
| `RgbaF32` composite written straight into `dst` | `arq_110_composite_overflow_is_err_and_dst_is_untouched` |
| offset rounded toward zero (`trunc`) | `fd_35_offsets_round_half_away_from_zero` |
| `opacity / 100` factor restored | `fd_35_a_opacity_ties_round_up_exactly` |

#### Sweep (throwaway test, deleted)

Radius x scale from 1e-170 to 3200 (blur up to 100, scale = target / blur): 1e-170, 1e-160, 1e-30, 1e-8, 1e-3, then a 1.37 geometric grid from 0.05 to 3000 plus 0.5, 0.75, 1, 1.16, 1.19999, 1.2, 2, 2880, 3000, 3200. One-row images (the vertical pass reads only the centre tap, so sigma is along x), alpha impulse 0.01, `RgbaF32`: sigma from the shadow response. Also `Rgba8` against `RgbaF32` alpha on non-centre pixels, and a tile (7 px with its `roi`) against the whole row, bit-exact in `f32`. Results: no `Err` anywhere; tiles identical at all radii; `Rgba8` within 0.5/255 of `RgbaF32` (impulse of 255 against 1); sigma error 0.000 % up to 1.2, -1.18 % at 1.2 (Gaussian branch), -0.2 % at 2, -1.0 % at 20, -1.34 % from 100 to 3200 (the 3 sigma truncation, expected 1.35 %), so ARQ-120 holds across the sweep, also at the cap 4800. Not a fix: for blur x scale below about 1e-21 (sigma squared under the smallest `f32`) the response cannot be stored in `f32` and the output is the identity (measured sigma 0 at 1e-30 and 1e-160). A 2 % criterion has no meaning there, and the result is the identity, not NaN or `Err`.

## RS5

Branch `feat/gafira-raster` (from `develop` d64a41f). Commits: ba8fa71 (white balance and eyedropper), 6b5cfec (light), 5f9bb26 (color), 69a3406 (texture), 82f8034 (auto adjust), and 5dca6d4 (test only: W7 and S4 of RS4, asked by the coordinator during the run). Gates (fmt `--check`, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each commit, read one by one. The RS5 and RS6 sweep, the size and the bench figure are at the end of the RS6 section.

Strict TDD as the owner asked: for each criterion the test was written first and run against a compile-only stub (`run_cpu` left `dst` at its sentinel, `eyedropper` and `auto_adjust` returned zeros), the failing line is in the tables, then the code. A test that passed from the start says so and names its mutant.

### Done

FD-33 (a) every slider at its default is the identity, difference 0, both formats (also over the 32 896 valid pairs `(c, a)` of 8 bits), (d) `auto_adjust`, (e) the analytic values of the table, (f) alpha kept and alpha 0 stays, flat image, centre of the vignette, Invertir twice, (g) the eyedropper, (h) bad parameters give `Err` and `dst` is untouched. Public API in `gafira_raster::effects`: `WhiteBalance`, `Light`, `ColorAdjust`, `Texture` (all `Effect`), `eyedropper`, `TemperatureTint`, `auto_adjust`, `AutoAdjust`. The detected colours (ΔE) are RS6.

### Unit 1: shared runner, White balance (Temperatura, Matiz) and the eyedropper

RED (stub `run_cpu` returned `Ok(())` and left `dst` at the sentinel; stub `eyedropper` returned (0, 0); all run before any implementation):

| Criterion | Failing test and line |
|---|---|
| FD-33 (a) identity | `fd_33_a_white_balance_defaults_are_identity`: "7x5", left `[171, 171, ...]` (sentinel), right = the source |
| FD-33 (e) factors, f32 | `fd_33_e_temperature_and_tint_scale_the_linear_channels`: "t 100 m 0 a 1 ch 0: NaN vs 0.2828427166893039" |
| FD-33 (e) 8 bit | `fd_33_e_8_bit_pixels_go_through_linear_light`: "ch 0 t 100 m -40", left 171, right 150 |
| FD-33 (f) alpha | `fd_33_f_white_balance_keeps_alpha`: "alpha at 0 ... temperature -100", left 59, right 171 |
| FD-33 (g) values | `fd_33_g_eyedropper_values_and_clipping`: `TemperatureTint { temperature: 0.0, tint: 0.0 }` |
| FD-33 (g) neutral | `fd_33_g_the_eyedropper_values_make_the_grey_neutral`: `[NaN, NaN, NaN, NaN]` (dst untouched) |
| FD-33 (h) | `fd_33_h_bad_parameters_are_err_and_dst_is_untouched`: "temperature -100.0000001", left `Ok(())`, right `Err(Param(OutOfRange { param: "temperature" }))` |
| ARQ-110 | `arq_110_eyedropper_rejects_non_finite_channels`: left `Ok(TemperatureTint {..})`, right `Err(NonFinite { field: "gray", index: 0 })`; `arq_110_overflowing_result_is_err_and_dst_is_untouched`: left `Ok(())`, right `Err(Buffer(NonFinite { field: "result", index: 4 }))`; `arq_110_non_finite_scale_and_pixels_are_err`: "scale NaN" |
| Formats | `white_balance_rejects_other_formats_and_sizes`: left `Ok(())`, right `Err(UnsupportedFormat { format: Alpha8 })` |

Passed from the start (stub already gave the answer), proved by a mutant: `fd_33_white_balance_schema_has_two_sliders_of_100` (range 100 to 99, default 0 to 5, id), `fd_33_g_eyedropper_with_a_zero_or_negative_channel_changes_nothing` (guard `<= 0` to `< 0`, third channel unchecked), `arq_111_white_balance_is_deterministic` (`RandomState` entropy in the `f32` and in the 8-bit write), `white_balance_empty_images_are_ok` (size 0 returns `Err`), `white_balance_roi_is_the_output_region` (x0 + 1, x1 - 1), `white_balance_cost_grows_with_pixels` (`px + 1`).

### Unit 2: Light (Brillo, Contraste, Luces, Sombras, Blancos, Negros)

RED (stub `run_cpu` left `dst` at the sentinel):

| Criterion | Failing test and line |
|---|---|
| FD-33 (a) | `fd_33_a_light_defaults_are_identity`: "7x5", left the sentinel bytes, right the source |
| FD-33 (e) | `fd_33_e_light_analytic_values`: "brightness +100: NaN vs 0.5" (the sentinel is a NaN) |
| FD-33 Luces, Sombras | `fd_33_light_highlights_and_shadows_weights`: left NaN, right NaN |
| FD-33 Blancos, Negros | `fd_33_light_whites_and_blacks_move_the_ends`: "NaN vs 1" |
| 02 4.17.4 clip | `fd_33_light_contrast_clips_and_brightness_does_not`: `[NaN, NaN, NaN]` |
| formulas, f32 | `fd_33_light_matches_the_reference_in_f32`: "[0.0, 0.0, 0.0] [0.3, 0.0, ...] ch 0: NaN vs 0" |
| formulas, 8 bits | `fd_33_light_matches_the_reference_in_8_bits`: "[10, 100, 200, 255] [0.2, 0.5, ...] ch 0: 171 vs 0" |
| FD-33 (f) | `fd_33_f_light_keeps_alpha`: "alpha at 0 ... brightness -100", left 59, right 171 |
| FD-33 (h) | `fd_33_h_light_bad_parameters_are_err_and_dst_is_untouched`: "brightness -100.0000001", left `Ok(())`, right `Err(Param(OutOfRange { param: "brightness" }))` |
| ARQ-110 | `arq_110_light_non_finite_scale_and_pixels_are_err`: "scale NaN", left `Ok(())`; `arq_110_light_overflowing_result_is_err_and_dst_is_untouched`: `Ok(())` |
| Formats | `light_rejects_other_formats_and_sizes`: left `Ok(())`, right `Err(UnsupportedFormat { format: Alpha8 })` |

Test fix at GREEN: the reference applied the Blancos and Negros stage at zero sliders, which clips above 1; guarded like the code (the spec says the default is the identity).

Passed from the start, proved by a mutant: the schema test, `arq_111_light_is_deterministic`, `light_empty_images_are_ok`, `light_roi_is_the_output_region`, `light_cost_grows_with_pixels` (same mutants as unit 1 on `Light`).

### Unit 3: Color (Tono, Saturación, Vitalidad, Invertir)

RED (stub `run_cpu` left `dst` at the sentinel):

| Criterion | Failing test and line |
|---|---|
| FD-33 (a) | `fd_33_a_color_defaults_are_identity`: "7x5", left the sentinel bytes, right the source |
| FD-33 (e) Saturación -100 | `fd_33_e_saturation_minus_100_gives_equal_channels`: `[NaN, NaN, NaN]` |
| Tono | `fd_33_color_hue_rotates_the_linear_channels`: `[NaN, NaN, NaN] vs [0.0, 0.356, -0.574]` |
| Vitalidad | `fd_33_color_vibrance_saturates_dull_colours_more`: `[NaN, NaN, NaN] [0.57083, 0.38083, 0.38083]` |
| FD-33 (f) Invertir x2 | `fd_33_f_invert_twice_is_the_original_and_flips_v`: `[NaN, NaN, NaN]` |
| formulas | `fd_33_color_matches_the_reference_in_f32`: "[0.0, 0.0, 0.0] 45 0 0 false ch 0: NaN vs 0"; `fd_33_color_matches_the_reference_in_8_bits`: "[10, 100, 200, 255] ch 0: 171 vs 179" |
| FD-33 (f) alpha | `fd_33_f_color_keeps_alpha`: "alpha at 0 ... hue -180", left 59, right 171 |
| FD-33 (h) | `fd_33_h_color_bad_parameters_are_err_and_dst_is_untouched`: "hue -180.00000018", left `Ok(())`, right `Err(Param(OutOfRange { param: "hue" }))` |
| ARQ-110 | `arq_110_color_non_finite_scale_and_pixels_are_err`: "scale NaN", left `Ok(())`; `arq_110_color_overflowing_result_is_err_and_dst_is_untouched`: `Ok(())` |
| Formats | `color_rejects_other_formats_and_sizes`: left `Ok(())`, right `Err(UnsupportedFormat { format: Alpha8 })` |

Passed from the start, proved by a mutant: the schema test, `arq_111_color_is_deterministic`, `color_empty_images_are_ok`, `color_roi_is_the_output_region`, `color_cost_grows_with_pixels`.

Equivalent survivors: `unwrap_or(false)` to `unwrap_or(true)` on `invert` (the value is always present after `resolve`), `changed.then_some(c)` to `Some(c)` (with every stage off the pixel goes through decode and encode unchanged; proved over the 32 896 pairs `(c, a)` by the exhaustive identity test), and `Cost { units: 2 * px }` (cost units are abstract, only comparisons mean something; the test requires proportionality to `px`).

### Unit 4: Texture (Nitidez, Claridad, Viñeta)

RED (stub `run_cpu` left `dst` at the sentinel, `roi` returned `out`, cost was `px`):

| Criterion | Failing test and line |
|---|---|
| FD-33 (a) | `fd_33_a_texture_defaults_are_identity`: "7x5", left the sentinel bytes, right the source |
| FD-33 (f) flat image | `fd_33_f_sharpness_and_clarity_do_not_change_a_flat_image`: "1x1 [120, 60, 200, 255] ... sharpness 100 scale 1", left `[171, 171, 171, 171]`, right `[120, 60, 200, 255]` |
| FD-33 (f) centre | `fd_33_f_vignette_does_not_change_the_centre`: "21x21 px 71 d 0.49", left `[171, ...]`, right the source |
| FD-33 Viñeta | `fd_33_texture_vignette_matches_the_formula`: "kv 1 px 0 d 0.96: NaN vs 0.79" |
| FD-33 Nitidez, Claridad f32 | `fd_33_texture_sharpness_and_clarity_match_the_reference_in_f32`: "scale 1 [1.0, 0.0, 0.0] px 0 ch 0: NaN vs 0.09" |
| FD-33 8 bits | `fd_33_texture_matches_the_reference_in_8_bits`: "scale 1 [1.0, 0.0, 0.0] px 0 ch 0: 171 vs 0" |
| Nitidez edge | `fd_33_texture_sharpness_steepens_an_edge`: "171 171" |
| FD-33 (f) alpha | `fd_33_f_texture_keeps_alpha`: "alpha at 0 ... sharpness 0", left 59, right 171 |
| FD-33 (h) | `fd_33_h_texture_bad_parameters_are_err_and_dst_is_untouched`: "sharpness -0.000000001", left `Ok(())`, right `Err(Param(OutOfRange { param: "sharpness" }))` |
| ARQ-110 | `arq_110_texture_non_finite_scale_and_pixels_are_err`: "scale NaN", left `Ok(())`; `arq_110_texture_overflowing_result_is_err_and_dst_is_untouched`: `Ok(())` |
| ARQ-120 roi | `arq_120_texture_roi_grows_by_the_largest_active_kernel`: "sharp 1", left `IRect { 10, 20, 50, 60 }`, right `IRect { 7, 17, 53, 63 }` |
| ARQ-120 tiles | `arq_120_texture_tiles_with_roi_match_the_whole_image`: "nothing was written" (first version passed against the stub because both runs left the sentinel; the guard was added before GREEN) |
| cost | `texture_cost_grows_with_pixels_and_with_the_blur`: "c(1.0, [1,0,0], 1000) > c(1.0, [0,0,1], 1000)" |
| Formats | `texture_rejects_other_formats_and_sizes`: left `Ok(())`, right `Err(UnsupportedFormat { format: Alpha8 })` |

Test fixes at GREEN: the overflow test used Claridad, which cannot overflow with a 3e38 pixel; it uses Nitidez (`c + 1.5 (c - G)` passes 3.4e38). The 8-bit reference test had a case with Nitidez at scale 0.25 (radius 0.5, sigma 0.25): the crate uses its three-tap kernel below sigma 0.6 and the written-again reference does not model it, so the case is Nitidez at scale 2 and Claridad at scale 0.25.

Passed from the start, proved by a mutant: the schema test, `arq_111_texture_is_deterministic`, `texture_empty_images_are_ok`.

Equivalent survivors: `(out != c).then_some(out)` to `Some(out)` and dropping the `a == 0` skip in `Planes::linear` (`unpremultiply_u8(_, 0)` is 0, ARQ-117); the cost weights (`blur(.., 2)` to 4, `blur(.., 4)` to 1): units are abstract and the test pins proportionality to `px` and the order sharp/clarity above no blur.

### Unit 5: Ajuste automático (FD-33 (d))

RED (stub `auto_adjust` returned `AutoAdjust::default()` for every input):

| Criterion | Failing test and line |
|---|---|
| FD-33 (d) Brillo | `fd_33_d_brightness_moves_the_median_luminance_to_0_18`: "dark by one stop: 0 vs 100" |
| FD-33 (d) Blancos, Negros | `fd_33_d_whites_and_blacks_come_from_the_0_5_and_99_5_percentiles`: "blacks: 0 vs -41.28" |
| FD-33 (d) T, M | `fd_33_d_temperature_and_tint_use_the_mean_colour`: "temperature: 0 vs 16.99" |
| FD-33 (d) Intensidad | `fd_33_d_intensity_scales_every_value`: "assertion failed: full.brightness != 0.0 && ..." |
| FD-33 (d) idempotent | `fd_33_d_auto_adjust_is_idempotent`: `assertion left != right`: the stub gave an image equal to the source |
| FD-33 (h) | `fd_33_h_intensity_outside_0_100_or_not_finite_is_err`: "-0.01", left `Ok(AutoAdjust {..})`, right `Err(Param(OutOfRange { param: "intensity" }))` |
| ARQ-110 | `arq_110_auto_adjust_rejects_non_finite_pixels_and_alpha8`: left `Ok(..)`, right `Err(Buffer(NonFinite { field: "src", index: 5 }))` |

Passed from the start (the stub returns zeros), proved by a mutant: `fd_33_d_a_black_image_keeps_brightness_temperature_and_tint_at_zero` (`median > 0.0` to `>= 0.0`, no empty-list return, no alpha filter) and `fd_33_d_the_values_are_valid_for_white_balance_and_light` (`temperature` read as `tint`).

Added after the first GREEN, each RED against the mutant that showed the gap: `fd_33_d_translucent_pixels_count_with_their_straight_colour` (the `/ a` of `f32` and the unpremultiply of 8 bits survived because ratios cancel; the 40th percentile survived on short lists).

Equivalent survivor: `px[3] != 0` to `px[3] > 0` (a `u8` is never negative).


### Deviations and assumptions of RS5 (spec silent; verify may check)

1. **Question for the owner, kept as built.** FD-33 gives Invertir the range "-100 to 100" with the rest, but its formula `v' = 1 - v` has no `k`, and (f) says "Invertir dos veces". Built as a `Bool` switch (`invert`, default off). If the owner wants a slider, the formula needs a `k`. Recommendation: keep the switch.
2. One `Effect` per group of the table (`WhiteBalance`, `Light`, `ColorAdjust`, `Texture`), ids `effect.adjust.*`. Ajuste automático is not an effect (its Intensidad default of 60 would break (a)); it is a function of the original image whose result goes to the sliders. T and M are Balance de blancos sliders, so `AutoAdjust` has `white_balance_params()` and `light_params()` (the spec says "Luz y Color").
3. Stages run only when their sliders are not 0, in the order of the spec (Brillo, Contraste, Luces y Sombras, Blancos y Negros; Tono, Saturación, Vitalidad, Invertir). A pixel no stage changed is written back byte for byte.
4. `v` is the encoded value clipped to `[0, 1]` (as the notation says): Contraste, Blancos, Negros and Invertir clip their input; Brillo, Luces y Sombras, matrices do not. Results beyond `[0, 1]` stay in `f32` (02 4.17.4) and are clamped by the 8-bit encode.
5. A pixel with alpha 0 is copied as it is (a colour it holds is kept). Premultiplied pixels are unpremultiplied (ARQ-117 rounding), worked in linear light and premultiplied again.
6. Nitidez and Claridad: "radio 2 px" and "40 px" are CSS radii (σ = r / 2, ARQ-120), times `cx.scale`; both read the input image, not each other's output. The colour is blurred premultiplied in linear light and divided by the blurred alpha, so the border of the image does not darken (a flat image is unchanged, edges included). Claridad adds `0.5 k (Y - Y_G)` to the three channels, which moves Y by exactly that. Radii below σ 0.6 use the kernel of RS4.
7. `Texture::roi` grows the output by the largest blur in use (`ceil(1.5 r)` taps, 0 for sliders at 0); the vignette needs the size of the whole image, so `Texture` runs on whole images (as the transitions, SP23) and `roi` does not model it.
8. Ajuste automático: percentiles by nearest rank on `v_Y` of the pixels with alpha above 0 (`ceil(p n / 100)`-th value); the median of Y is the lower middle value for an even count; Brillo is `log2(0.18 / median)`, Negros `-4 n`, Blancos `4 (1 - b)`, all clipped to the slider range, T and M from the eyedropper on the mean of R, G, B; then times Intensidad / 100. A fully transparent or empty image gives all 0.
9. The eyedropper treats a channel at 0 or below as "0" (the spec says 0): the adjustment does not change.
10. `smoothstep` needs `a < b`; the degenerate edge comes with the tolerance of RS6.
11. Test-only commit 5dca6d4 (coordinator's W7 and S4): two new tests, `arq_120_half_size_tinted_rgba_f32_matches_within_1_255` and `arq_120_half_size_translucent_tinted_rgba8_matches_within_2_255`, on the full image made of the half repeated 2x2, compared at 3σ plus the offset from the edge. Both passed from the start. Mutants that change the scale handling were killed (blur radius ignoring `scale`, offset shifted by 1); mutants that scale the tint or the alpha equally in both sizes survive, because the half-size property holds for them (their tests are the RS4 ones). The comment of the first half-size test says what it covers, and `blur.rs` cites FD-08 for zoom 32.

### Hand mutants of RS5 (each reverted, none committed; all killed unless marked)

| Rule | Wrong behaviour | Killed by |
|---|---|---|
| Factors of Temperatura, Matiz | sign of the exponent; `/ 100` for `/ 200`; B and R factors swapped | `fd_33_e_temperature_and_tint_scale_the_linear_channels`, `fd_33_e_8_bit_pixels_go_through_linear_light` |
| Eyedropper | `<= 0` guard as `< 0` or on two channels; `log2(r / b)`; `g / (r + b)`; clamp at 99 or removed | `fd_33_g_eyedropper_values_and_clipping`, `fd_33_g_eyedropper_with_a_zero_or_negative_channel_changes_nothing`, `fd_33_g_the_eyedropper_values_make_the_grey_neutral` |
| Alpha preserved | alpha written as `a - 1`; `a == 0` pixel not copied; premultiply dropped; division by alpha dropped | the `fd_33_f_*_keeps_alpha` tests and the formula tests |
| Parameter check after work | the `f32` result written before the finite check; `resolve` errors swallowed; `check_scale` and `check_io` removed | the `arq_110_*_overflowing_result_is_err_and_dst_is_untouched`, `fd_33_h_*` and `arq_110_*_non_finite_scale_and_pixels_are_err` tests |
| Identity value not identity | default of a slider 1 or 5; Bool default true; skip of the identity removed (equivalent, see below) | the `fd_33_a_*_defaults_are_identity` and schema tests |
| `roi` too small | `out` shrunk or moved by 1 (pointwise); Texture `taps - 1`, only the sharpness taps, `min` instead of `max` | the `*_roi_is_the_output_region` tests, `arq_120_texture_roi_grows_by_the_largest_active_kernel`, `arq_120_texture_tiles_with_roi_match_the_whole_image` |
| Luz | 24 mutants on the Brillo gain, the Contraste exponent and fraction, clip on or off, Y weights, smoothstep edges, Blancos and Negros sign and denominator, stage guards, three stage-order swaps | `fd_33_e_light_analytic_values`, the `fd_33_light_*` tests |
| Color | each of 18 matrix coefficients and signs, `to_radians` removed, Vitalidad `max - min` and `1 - spread`, Invertir without the clip or without the curve | `fd_33_color_hue_rotates_the_linear_channels`, `fd_33_color_vibrance_saturates_dull_colours_more`, `fd_33_f_invert_twice_is_the_original_and_flips_v`, the reference tests |
| Textura | radii 2 and 40, weights `1.5` and `0.5`, the mask `e` and its edges (0.02, 0.1, `abs`), blur divided by alpha, premultiplied planes, vignette centre (`+ 0.5`), half diagonal, edges 0.5 and 1.0, scale ignored | the `fd_33_texture_*`, `fd_33_f_*` and `arq_120_texture_*` tests |
| Ajuste automático | percentiles 0.5 and 99.5 moved (1.0, 99.0), nearest rank (`floor`, no `- 1`), median (40th percentile, mean), target 0.18 as 0.2, sign of Negros, factor 4 as 2, unsorted list, mean of the first pixel, intensity `/ 50`, intensity bounds | the `fd_33_d_*` tests |

Equivalent survivors: skipping the identity stage (proved by the exhaustive identity over the 32 896 pairs), `unwrap_or(true)` on a value `resolve` always fills, `Cost { units: 2 * px }`, `changed.then_some(c)` as `Some(c)`, dropping the `a == 0` skip in the blur planes, the plane weights in `Texture::cost_hint`, `px[3] != 0` as `> 0` on a `u8`.

## RS6

Branch `feat/gafira-raster`. Commits: e1ed7c4 (palette), 9ccc1cb (dHash and `bench_dhash_12mp`), f452f97 (detected colours), f61c637 (filters), e8dd6d9 and 1a12da2 (the two fixes the sweep found, test first). Gates (fmt `--check`, clippy `-D warnings`, test `--workspace --locked`) exit 0 on each commit, read one by one. One commit (the palette) had a clippy error that I saw after the first `git commit`; it was amended before any further work, so no commit on the branch fails alone.

### Done

ARQ-122 palette (`dominant_colors`, `PaletteEntry`, `PALETTE_SIZE`) and dHash (`dhash`, `hamming_distance`, `is_similar`, `SIMILAR_DISTANCE`), FD-26 (the palette; the search by ΔE2000 below 15 belongs to the library), FD-28 (b), FD-33 detected colours (`DetectedColors`, `DETECTED_SLOTS`), FD-34 (a, b) with the mechanism (`FilterPreset`, `FilterStep`, `FilterError`, `mix_params`, `test_preset`, D-40, D-41), ARQ-114 `bench_dhash_12mp`. FD-34 (c) is out, as the card says.

**Bench:** `cargo bench -p gafira-raster --bench dhash`, `Rgba8` 4000 x 3000, on this machine: 10.25 to 11.77 ms (median 10.9 ms) against < 100 ms.

### Unit 6: palette (ARQ-122, FD-26)

RED (stub `dominant_colors` returned `Ok(vec![])`):

| Criterion | Failing test and line |
|---|---|
| 5 flat colours, order of area | `arq_122_five_flat_colours_in_order_of_area`: left 0, right 5 (`p.len()`) |
| 2 colours | `arq_122_two_colours_give_two_entries`: left 0, right 2 |
| alpha 0 ignored | `arq_122_alpha_zero_pixels_do_not_count`: left 0, right 2 |
| translucent | `arq_122_translucent_pixels_count_with_their_straight_colour`: left 0, right 2 |
| at most 5 | `arq_122_at_most_five_entries_the_largest_areas`: left 0, right 5 |
| noise | `arq_122_noise_around_a_colour_is_one_entry`: left 0, right 2 |
| `f32` | `arq_122_rgba_f32_gives_the_same_palette`: left 0, right 3; `arq_122_f32_values_beyond_the_range_are_clipped`: left 0, right 1 |
| ARQ-110 | `arq_110_palette_rejects_non_finite_pixels_and_alpha8`: left `Ok([])`, right `Err(Buffer(NonFinite { field: "src", index: 5 }))` |
| ARQ-111 | `arq_111_palette_is_deterministic`: left 0, right 4 (the length of the first run) |

Passed from the start (the stub returns an empty list), proved by a mutant: `arq_122_transparent_and_empty_images_give_an_empty_list` (alpha filter inverted to `!= 255`, and `filter(|_| true)` for `f32`).

Added after the first GREEN, each because a mutant survived: `arq_122_colours_that_differ_in_one_channel_are_apart` (blue bit of the bin replaced by green), `arq_122_a_noisy_large_colour_comes_before_a_small_flat_one` (final sort by area removed), `arq_122_f32_values_beyond_the_range_do_not_split_a_colour` (clamp of the straight `f32` colour removed); `arq_122_at_most_five_entries_the_largest_areas` now asserts the literal 5 (a constant of 6 survived when it compared with the constant).

### Unit 7: dHash (ARQ-122, FD-28 (b), ARQ-114 `bench_dhash_12mp`)

RED (stubs: `dhash` returned `Ok(0)`, `hamming_distance` 0, `is_similar` true):

| Criterion | Failing test and line |
|---|---|
| 64 bits, ramps | `arq_122_dhash_ramps_set_all_or_no_bits`: left 0, right 18446744073709551615 |
| bit rows | `arq_122_dhash_bit_rows_follow_the_bands_of_the_image`: "row 0", left 0, right 255 |
| bit columns | `arq_122_dhash_bit_columns_follow_the_cells`: "column 0", left 0, right 72340172838076673 |
| tiny and odd sizes | `arq_122_dhash_tiny_images_cover_every_cell`: left 0, right 0x2424242424242424; `arq_122_dhash_odd_sizes_split_the_image_evenly`: left 0, right 0x2020202020202020 |
| premultiplied luminance | `arq_122_dhash_reads_premultiplied_luminance`: left 0, right 0x1010101010101010 |
| `f32` | `arq_122_dhash_rgba_f32_matches_rgba8`: `assertion left != right`, left 0, right 0 |
| half size similar | `arq_122_dhash_an_image_and_its_half_are_similar`: `assertion left != right`, left 0, right 0 (the stub's distance 0 passed the bound, so the test also demands a non-zero hash) |
| noise not similar | `arq_122_dhash_noise_with_different_seeds_is_not_similar`: "seed 2: distance 0" |
| threshold | `arq_122_hamming_distance_and_the_threshold`: left 0, right 64 |
| ARQ-110 | `arq_110_dhash_rejects_non_finite_pixels_and_alpha8`: left `Ok(0)`, right `Err(Buffer(NonFinite { field: "src", index: 5 }))` |
| ARQ-111 | `arq_111_dhash_is_deterministic`: `assertion left != right`, left 0, right 0 |

Test fixes before GREEN (written from my own arithmetic, wrong): the odd-size expectation (bit 5, not 4) and the transparent-over-opaque cases (the boundary at x = 45 sits inside a cell; it is 50). One GREEN-time RED: `arq_122_dhash_reads_premultiplied_luminance` failed on "alpha 0 with colour 90" until alpha 0 was read as black.

Added after the first GREEN because a mutant survived: `arq_122_dhash_channel_weights` (both formats; the red and blue weights were not pinned) and `arq_122_dhash_f32_alpha_zero_is_black`. Dead code removed after mutants showed it: `.max(lo + 1)` and `.min(n)` in the cell split (`ceil((i + 1) n / cells)` is already above `lo` and at most `n`).

Equivalent survivors: `round()` to `floor()` of the `f32` luminance (both whole numbers, a flat image has equal cells), and the split of a side that is not a multiple of the grid by `floor` (cells that do not share border pixels) instead of `ceil`: the spec fixes no split, both are equal-sized cells within one pixel.

Bench `bench_dhash_12mp` (`cargo bench -p gafira-raster --bench dhash`, `Rgba8` 4000 x 3000, this machine): 10.25 to 11.77 ms, median 10.9 ms, against < 100 ms.

### Unit 8: detected colours (FD-33)

RED (stub `run_cpu` left `dst` at the sentinel; `smoothstep` had no case for equal edges):

| Criterion | Failing test and line |
|---|---|
| FD-33 (a) | `fd_33_a_detected_defaults_are_identity`: "7x5", left the sentinel bytes, right the source |
| sample moves like Color | `fd_33_detected_the_sample_colour_moves_like_color`: `[NaN, NaN, NaN] vs [0.3018, 0.3000, 0.0903]` |
| weight `1 - w(tau/2, tau, dE2000)` | `fd_33_detected_weight_follows_delta_e_and_tolerance`: "tol 8 t 0/40 dE 0 w 1: [NaN, NaN, NaN] vs [0.1318, 0.3881, -0.2815]" |
| tolerance 0, reach | `fd_33_detected_tolerance_zero_selects_nothing_and_more_reaches_further`: "t 0", left `[2143294004, ...]` (the sentinel bits), right the colour |
| 8 samples | `fd_33_detected_eight_samples_each_move_their_own_colours`: "sample 0: [NaN, NaN, NaN] vs [0.6277, 0.1411, -0.0482]" |
| overlap, order | `fd_33_detected_overlapping_samples_apply_in_order`: `[NaN, NaN, NaN] vs [0.5486, 0.2593, -0.0068]` |
| 8 bits | `fd_33_detected_8_bit_pixels_match_color`: left `[171, 171, 171, 171]`, right `[61, 142, 0, 255]` |
| FD-33 (f) | `fd_33_f_detected_keeps_alpha`: "alpha at 0 ... hue_1 -180", left 59, right 171 |
| FD-33 (h) | `fd_33_h_detected_bad_parameters_are_err_and_dst_is_untouched`: "hue_1 -180.00000018", left `Ok(())`, right `Err(Param(OutOfRange { param: "hue_1" }))` |
| ARQ-110 | `arq_110_detected_non_finite_scale_and_pixels_are_err`: "scale NaN", left `Ok(())`; `arq_110_detected_overflowing_result_is_err_and_dst_is_untouched`: `Ok(())` |
| smoothstep, equal edges | `fd_33_smoothstep_values_and_the_step_of_equal_edges`: left NaN, right 1.0 |
| Formats | `detected_rejects_other_formats_and_sizes`: left `Ok(())`, right `Err(UnsupportedFormat { format: Alpha8 })` |

Test fix before GREEN: the eight sample colours of the "eight samples" test were not apart (its own precondition, dE2000 above 12, failed); new colours chosen.

Passed from the start, proved by a mutant: the schema test (ranges, default 15, slot ids), `arq_111_detected_is_deterministic`, `detected_empty_images_are_ok`, `detected_roi_is_the_output_region`, `detected_cost_grows_with_pixels`.

Equivalent survivors: `weight <= 0.0` to `< 0.0` (a weight of exactly 0 applies the identity matrices, whose product is exact in `f32` as the bit-for-bit "outside is untouched" assertion shows) and `changed.then_some(out)` to `Some(out)` (round trip is exact).

### Unit 9: filters (FD-34 (a), (b); D-40, D-41)

RED (stubs: `mix_params` returned the defaults, `resolve` an empty list, `apply` `Ok(())` leaving `dst` at its sentinel, `test_preset` an empty preset):

| Criterion | Failing test and line |
|---|---|
| (a) thumbnail on mip level 1 | `fd_34_a_the_thumbnail_matches_level_one_of_the_full_result`: "nothing written"; `fd_34_a_the_thumbnail_matches_in_rgba_f32`: `unwrap` on `Err(NonFinite { field: "pixels", index: 0 })` (the NaN sentinel) |
| (b) Float `d 0, p 40, i 25` gives 10 | `fd_34_b_float_intensity_formula`: `assertion failed: (f(40.0, 25.0) - 10.0).abs() < 1e-12` |
| (b) Int 3, -3, 2 | `fd_34_b_int_rounds_half_away_from_zero`: left 0, right 3 |
| (b) Color black to white at 50 | `fd_34_b_color_blends_in_linear_light`: `LinearRgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }` |
| (b) Bool, Choice | `fd_34_b_bool_and_choice_jump_above_zero`: "0.01", left `Some(false)`, right `Some(true)` |
| (b) 0 is no filter, 100 is the preset | `fd_34_b_intensity_zero_is_no_filter_byte_for_byte`: left the sentinel bytes, right the source; `fd_34_b_the_ends_are_exact`: "b", left `Bool(true)`, right `Bool(false)`; `fd_34_b_intensity_100_runs_the_preset_in_order`: left the sentinel bits |
| (b) bad intensity | `fd_34_b_bad_intensity_is_err_and_the_ends_pass`: "NaN", left `Ok(ParamValues {..})`, right `Err(Effect(Buffer(NonFinite { field: "intensity", index: 0 })))` |
| (b) preset out of range | `fd_34_b_a_preset_value_out_of_range_is_err`: "f at 0", left `Ok(..)`, right `Err(Effect(Param(OutOfRange { param: "f" })))`; `fd_34_errors_leave_dst_untouched`: left `Ok(())`, right `Err(.. OutOfRange { param: "brightness" })` |
| test preset | `fd_34_the_test_preset_is_contrast_and_saturation`: left `[]`, right `["effect.adjust.light", "effect.adjust.color"]` |
| ARQ-110 | `arq_110_filter_rejects_bad_inputs_before_writing`: `assertion failed: matches!(r, Err(FilterError::Effect(EffectError::Buffer(_))))` |
| ARQ-111 | `arq_111_filter_is_deterministic`: "nothing written" |

Test fixes before GREEN: the two (a) tests passed against the stub (zero buffers on both sides); they now start from the sentinel and demand that the filter changed level 1.

Passed from the start, proved by a mutant: `filter_error_messages` (key `error.raster.unknown_effect` to another, `Effect` not delegating) and `filter_empty_images_are_ok` (`check_io` returning `Err` on size 0).

Added after the first GREEN because a mutant survived: `fd_34_b_color_alpha_and_space_follow_the_interpolation_rule` (premultiplied weights, `alpha <= 0` straight blend, space kept or linear sRGB) and the `big` parameter of the ends test (an `i64` beyond 2^53).

Equivalent survivors: the NaN filler of the scratch buffer (every value is overwritten before it is read) and the `convert` of the `from` end before `to_linear_srgb` (it converts anyway).


### Sweep (throwaway test, deleted)

A test module of about 330 lines, kept out of the commits, ran: every effect (`WhiteBalance`, `Light`, `ColorAdjust`, `Texture`, `DetectedColors`) over about 800 parameter sets each (every parameter at its minimum, maximum, default and a middle value, singly and in pairs, plus all at minimum and all at maximum, colours including `(5, -3, 1e30)` and alpha 0), on 1x1, 1x9, 9x1, 3x3, 17x5, 257x2 and 2x257 images, both formats, scales 0.01, 0.5, 1, 2 and 7: always `Ok`, alpha unchanged, alpha 0 pixels unchanged, 8-bit `c <= a`, `f32` finite or `Err` with `dst` untouched. Extreme pixels (denormals, 3e38, negative alpha, alpha 3e38). The four pointwise effects over two column halves equal the whole image bit for bit. `Texture` by 16 x 16 tiles with `roi` over a 70 x 50 image with edge tiles, scales 0.25, 0.5, 1, 2, within 1/255. Scales -1, 0, `MIN_POSITIVE`, 1e-30, 1e6 and `f32::MAX`. The eyedropper over 300 random grey values (zero, denormal, 3e38). `auto_adjust` over four sizes, denormal and ordinary data, intensities 0, 1e-300, 59.99..., 100. The palette and the hash over eight sizes, both formats, and a 12 MP flat image (hash 0, one palette entry of 12 000 000). `mix_params` over every float of every effect, 14 intensities, six preset values; ints with `i64::MIN` and `i64::MAX`; the test preset at awkward intensities.

It found two defects, both fixed with the test first:

| Finding | RED | Fix |
|---|---|---|
| `auto_adjust` returned `Err(NonFinite { field: "gray" })` for a finite image whose straight colour passes `f32` (premultiplied 1.0 at alpha 1e-40) | `arq_110_a_straight_colour_beyond_f32_is_not_an_error`: `unwrap` on `Err(Buffer(NonFinite { field: "gray", index: 0 }))` | e8dd6d9: the mean goes to the eyedropper in `f64` (`sliders_for`) |
| `mix_params` could leave the range of the schema: a preset equal to the default at the end of the range, at intensity 0.04, gave 100.00000000000001, so the effect rejected its own parameters | `fd_34_b_a_float_mix_stays_between_its_ends`: `unwrap` on `Err(OutOfRange { param: "x" })` | 1a12da2: the mix is clamped between the two values |

Mutants for both: casting the mean to `f32` again, removing the clamp or shrinking it are killed by those tests.

### Hand mutants of RS6 (each reverted, none committed; all killed unless marked)

| Rule | Wrong behaviour | Killed by |
|---|---|---|
| Palette, order and size | final sort removed or ascending; `PALETTE_SIZE` 4 or 6; bins ordered by count ascending or by bin descending; nearest cluster taken as the farthest | `arq_122_five_flat_colours_in_order_of_area`, `arq_122_at_most_five_entries_the_largest_areas`, `arq_122_a_noisy_large_colour_comes_before_a_small_flat_one`, `arq_111_palette_is_deterministic` |
| Palette, merge threshold | dE 0.5, 60, or the edge `<` as `<=` | `arq_122_noise_around_a_colour_is_one_entry`, `arq_122_five_flat_colours_in_order_of_area` |
| Palette, alpha and colour | alpha filter inverted or off (`u8`, `f32`); no unpremultiply; `f32` without `/ a` or without the clamp; blue bit of the bin read from green; sum `+ 1`; channels swapped; alpha 0.5 in the result | `arq_122_alpha_zero_pixels_do_not_count`, `arq_122_translucent_pixels_count_with_their_straight_colour`, `arq_122_rgba_f32_gives_the_same_palette`, `arq_122_f32_values_beyond_the_range_do_not_split_a_colour`, `arq_122_colours_that_differ_in_one_channel_are_apart` |
| dHash threshold | `SIMILAR_DISTANCE` 9 or 11; `<=` as `<` | `arq_122_hamming_distance_and_the_threshold` |
| dHash bits | `^` as `\|` or `&`; bit index `8 c + r` or `63 - ..`; comparison `>=`; 8 columns or 9 rows; start of the cell by `ceil` | the `arq_122_dhash_*` tests |
| dHash luminance | R and B weights swapped, B dropped, G and B swapped, in 8 bits and in `f32`; alpha 0 not black (`u8`, `f32`) | `arq_122_dhash_channel_weights`, `arq_122_dhash_reads_premultiplied_luminance`, `arq_122_dhash_f32_alpha_zero_is_black` |
| Detected colours | tolerance `/ 3`, `* 1.5` or edges from 0; ΔE76 for ΔE2000; weight not inverted; cut at 0.3; hue not scaled by the weight; saturation not scaled or negated; 7 slots; `\|\|` for `&&` in the active test; samples read from slot 0; default tolerance 16; Lab without the clamp; weights from the changed colour | the `fd_33_detected_*` tests |
| smoothstep | `b <= a` as `<`; `x < a` as `<=` | `fd_33_smoothstep_values_and_the_step_of_equal_edges` |
| Filters, intensity | bounds `..100`, `..=99`, `0.001..`; NaN-only check; `/ 99`; mix reversed or without `d`; the two ends computed instead of copied; Int `floor`, `trunc`, `ceil`, without `d`; Bool and Choice take the default; preset not checked | the `fd_34_b_*` tests |
| Filters, colour | alpha not mixed; weights not premultiplied; `alpha <= 0` as `< 0`; space not kept; always kept | `fd_34_b_color_blends_in_linear_light`, `fd_34_b_color_alpha_and_space_follow_the_interpolation_rule` |
| Filters, chain | scale check removed; `check_io` removed; unknown step index 0; last step taken as the first; test preset values 0 | `arq_110_filter_rejects_bad_inputs_before_writing`, `fd_34_errors_leave_dst_untouched`, the `fd_34_a_*` tests |

Equivalent survivors: the final `Some(out)` of the pointwise closures, `weight <= 0` as `< 0` in the detected colours, the NaN filler of the filter scratch buffer, `convert` of the `from` colour before `to_linear_srgb`, the `floor` split of the dHash cells against `ceil` (the spec fixes no split), `round` against `floor` of the `f32` luminance.

### Deviations and assumptions of RS6 (spec silent; verify may check)

1. **Palette.** ARQ-122 fixes the five-colour result, not the algorithm. Built: pixels go to 5-bit sRGB bins; bins, fullest first, join the nearest cluster when it is within ΔE2000 10 (FD-26 matches within 15), otherwise open a new cluster up to five; a colour far from all five when they are taken is left out. The entry is the mean colour in linear light, returned as opaque straight sRGB, with its pixel count. A translucent pixel counts as one pixel of its straight colour. Ties go in bin order.
2. **dHash.** 9 x 8 cells of mean luminance (0.2126, 0.7152, 0.0722 on the premultiplied sRGB values, so alpha 0 is black); bit `8 r + c` is set when cell `c` of row `r` is brighter than cell `c + 1`. A side that is not a multiple of the grid gives cells that share their border pixels; a side smaller than the grid gives each cell at least one pixel. `Alpha8` is `UnsupportedFormat`; size 0 hashes to 0.
3. **Detected colours.** The spec gives no parameter list. Built as one effect, `effect.adjust.detected_colors`, with eight slots of `color_n`, `hue_n` (-180 to 180), `saturation_n` (-100 to 100) and `tolerance_n` (ΔE2000, 0 to 100, default 15). A slot with hue and saturation at 0 does nothing. The weight `1 - w(τ/2, τ, ΔE)` comes from the colour as it was before earlier slots and scales the hue angle and the saturation of that slot; slots apply in order. Tolerance 0 selects nothing (`w` with equal edges is a step). The Lab of the pixel clamps the channels to `[0, 1]`.
4. **Filters.** The crate has no effect registry, so `apply` and `resolve` take a slice of `&dyn Effect` and find each step by `descriptor().id`. `FilterError` is new (`Effect`, `UnknownEffect { step }`, key `error.raster.unknown_effect`). A bad intensity is `Buffer(NonFinite { field: "intensity" })` or `Param(OutOfRange { param: "intensity" })`. The Color blend of D-41 is written in the crate (premultiplied linear light, as `Interpolate for Color` in `gafira-anim`) because the layer table of V5 does not list `gafira-anim` for `gafira-raster`; if the owner wants one implementation, the table needs the row. The test preset is `filter.test`: Contraste 40 and Saturación 60.
5. `Cargo.toml` of `gafira-raster` gains the `dhash` bench target (as `shadow` did in RS4); no new dependency, `Cargo.lock` unchanged.
6. `crates/gafira-raster/proptest-regressions/` is untracked and stays so.

### Size

`git diff --shortstat` over `crates/`: RS5 (ba8fa71 to 82f8034, with the test-only commit 5dca6d4) 3 366 lines added; RS6 (82f8034 to the fix commits) 2 746 added. Both are far above the ~700 mark of the card; about half of each are tests. Authored non-test lines: white balance 132, light 125, color 147, texture 272, auto adjust 136, detected colours 239, filters 313, pointwise runner 158, palette 139, dHash 118, bench 42. The forecast was 480 (670) lines for each delivery; the owner said size is no limit for serial work.

## Fix round of RS5 and RS6 (verify round 1: C1 to C6, W1 to W4, S2, S7)

Commits, in order (each passes `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` alone, exit codes read separately):

| Commit | Items |
|---|---|
| `b3e7ae8` | P16 types: `TileWindow`, `EffectCx.tile`, `EffectCx::whole`; the 40 literals replaced; `Effect::run_cpu` doc |
| `eabbbd1` | W4: Viñeta from the window, `window_of`, `check_tile` |
| `d3facb1` | C4: Claridad reads the result of Nitidez; `roi` is the sum of both tap counts |
| `84d9db7` | C3, C5 (filter), W3, S2, S7 |
| `4347022` | C5 (detected colours) |
| `d0039c8` | W1, W2 |
| `f3dbfa5` | C1, C2 |
| `4a82f6d` | C6 |

Test counts: 1 769 at the start of the round, 1 803 at the end, 0 failed.

### RED evidence and hand mutants per item

Each test below was written against the code of the commit before it and run to see the assertion fail; the failing line is quoted. C6 has no RED by design: the behaviour was right (verify probe), so its tests are green and each is named by the mutant it kills. Mutants are made by hand on the working copy, the named tests fail, then the change is reverted; none is committed.

#### W4 (P16) Texture window
RED: arq_120_texture_vignette_tiles_with_a_window_match_the_whole_image: `[0.5, 0.0, 0.8] (21,12) [194, 149, 242, 255] [215, 166, 255, 255]` (8-bit diff > 1); arq_120_texture_window_that_does_not_fit_is_err_and_dst_is_untouched: `left: Ok(()) right: Err(SizeMismatch { field: "cx.tile", expected: (10, 10), actual: (11, 3) })`.
Pass from start (property of API): arq_120_texture_window_at_the_origin_equals_none_bit_for_bit (mutants below).
Mutants killed: M-W4a vignette x ignores x0 -> vignette_tiles test; M-W4b half diagonal from src size -> vignette_tiles; M-W4c check_tile `right < iw` -> 16 tests incl window_that_does_not_fit; M-W4d `bottom < ih` -> same (all 16/1 via window tests); M-W4e check sum ignores x,y -> window_that_does_not_fit; M-W4f check after empty return -> window_that_does_not_fit.
#### C4 Texture order
RED (reference corrected + new tests, old code): fd_33_texture_sharpness_and_clarity_match_the_reference_in_f32: `scale 1 [0.6, 0.8, -0.5] px 0 ch 0: 0.024895364 vs 0.024929540051087787`; fd_33_texture_matches_the_reference_in_8_bits: `scale 2 [0.5, 0.5, -0.8] px 7 ch 2: 0 vs 9`; fd_33_texture_sliders_in_one_run_equal_the_sliders_in_sequence: `scale 1 [1.0, 1.0, 0.0] value 0: 0.039661754 vs 0.039735384`; arq_120_texture_roi_grows_by_the_active_kernels: `left IRect{-50,-40,110,120} right IRect{-53,-43,113,123}`.
Mutants killed: Claridad delta from input luma (luma(c)) -> sequence + 2 reference tests; Y plane from input -> same 3; roi max instead of sum -> arq_120_texture_roi_grows_by_the_active_kernels; Claridad without Nitidez -> 6 tests.
#### Filter: C3, C5 (mix), W3, S2, S7
RED: fd_34_b_intensity_zero_is_no_filter_for_an_effect_whose_defaults_change_the_image: assertion `left == right` failed (dst has the shadow, e.g. left [.., 0,0,0,6,..] right [..,7,0,0,0,..]); arq_110_a_colour_mix_that_overflows_is_err_naming_the_parameter: `Srgb(r: 1e30..) at 1: left Ok(.. "col": Color(Srgb(Rgba { r: inf ..`, right Err(Effect(Buffer(NonFinite { field: "col", index: 0 }))); fd_34_b_color_blend_is_the_interpolation_of_02_3_4_bit_for_bit: `[0.2,0.4,0.6,0.05] to [0.9,0.1,0.5,0.02] at 33 left [.., 1051952363, ..] right [.., 1051952362, ..]` (expected bits from gafira_anim::Interpolate for Color, taken by a throwaway test in gafira-anim, deleted); arq_110_mix_params_keeps_the_unknown_ids_of_the_preset: panicked at filter.rs:771 `0` (id lost); fd_33_the_default_intensity_is_60: left 0.0 right 60.0 (constant placeholder 0.0 added to compile).
Mutants killed: intensity-0 copy removed -> fd_34_b_intensity_zero_is_no_filter_for_an_effect_whose..; DEFAULT_INTENSITY 50 -> fd_33_the_default_intensity_is_60; alpha in f64 -> fd_34_b_color_blend_is_the_interpolation..; t perturbed -> same; result without finite check position(skip 1)-> arq_110_a_colour_mix_that_overflows..; returning unconverted `linear` -> fd_34_b_color_alpha_and_space_follow_the_interpolation_rule; unknown ids not copied -> arq_110_mix_params_keeps_the_unknown_ids..
#### DetectedColors C5
RED: arq_110_a_sample_that_overflows_is_err_naming_its_slot: `slot 0 left: Ok(()) right: Err(Buffer(NonFinite { field: "color_1", index: 0 }))`.
Mutants killed: finite check only for NaN -> that test; check applied to idle slots (skip of the hue/sat filter removed) -> that test (idle half).
#### dHash W1, W2 (spec 1b159f9 landed: min size 128 px for 2x2 halves, 640 for bilinear/nearest, no bound below; no code behaviour needed)
RED (old code): arq_122_dhash_translucent_picture_has_the_same_hash_in_both_formats: `seed 0: distance 6 left 13158266634104048221` (before margin filter; same code path); arq_122_dhash_cells_are_weighted_by_the_area_they_cover: `38x30 seed 1 left: 13209995958845039898 right: <reference>`.
Mutants killed: f32 arm encodes the premultiplied value (old) -> translucent test; weights = 1 -> 4 tests (area, odd sizes, tiny, noise); weights +1 -> area + odd sizes; only first cell touched -> 3 tests. Equivalent: alpha*0.5 uniform scale (hash is scale invariant).
Half-size test gains 131x101, 157x139, 203x129 (pass from start; spec bound >=128 px).
#### Palette C1, C2
RED (old code): arq_122_close_colours_are_one_entry_each: `[[200, 60, 60], [215, 70, 60]]: [PaletteEntry { color: Srgb(0.8128738, 0.25473788, 0.23529412), pixels: 190 }] left: 1 right: 2`; arq_122_a_large_noisy_colour_beats_five_small_flat_ones: `noise 16: PaletteEntry { color: Srgb(0.1176, 0.6274, 0.2352), pixels: 800 }` (first entry is a flat, not the noise); arq_122_a_gradient_beats_five_small_flat_ones: `left: 4399 right: 10000` (entries cover 4399 of 10000).
Design: exact mode for <=5 distinct straight colours (C2); otherwise up to 32 clusters (bin merge dE<10), clusters within dE 15 (the FD-26 radius) merged, cut to five by area, every bin of a dropped cluster joins the nearest kept one, re-sort (C1). The second merge radius is the "look at the merge threshold": with only the cut, noise +-32 and +-40 took three or four of the five places (first test version failed at noise 40 with a 3470-pixel entry holding flats).
Old tests changed (deviations): arq_122_at_most_five_entries_the_largest_areas (the colours left out now join, sum 258, was "left out"); arq_122_f32_values_beyond_the_range_do_not_split_a_colour (1e30 vs 1.0 is one colour, 1e30 vs 0.8 are two entries now that up to five colours are one each).
Mutants killed: exact mode off -> close_colours_are_one_entry_each + f32 test; exact list `<=` PALETTE_SIZE -> six_colours_are_five_entries; dropped bins not redistributed -> 4 tests; stage-2 merge off / radius 12 -> a_large_noisy_colour_beats_five_small_flat_ones (+ six, gradient); dropped bins to cluster 0 instead of the nearest -> noisy_colour test; no re-sort after redistribution -> arq_122_the_order_follows_the_areas_after_the_pixels_left_out_join.
#### C6 generic ARQ-110 helpers
No RED by design: the behaviour was right (verify probe); tests are green on the code and killed by mutants. Helpers in testing.rs: assert_unknown_ids_are_ignored, assert_roi_and_cost_extremes, finite_output_case; one proptest per effect (white_balance, light, color_adjust, texture, detected) plus the filter (unknown ids in a step, proptest).
Mutants killed: G1 `px * work` in Texture::cost_hint -> arq_110_texture_roi_and_cost_hint_survive_the_extremes (overflow panic); G2 `assert!(cx.scale.is_finite())` in Texture::roi -> same test (NaN scale); G3 DetectedColors returns early on an unknown id -> arq_110_detected_unknown_ids_are_ignored; same style for light, color_adjust, white_balance, texture (unknown ids tests); light cost `px * 3` -> arq_110_light_roi_and_cost_hint_survive_the_extremes; filter reads an unknown id -> arq_110_filter_ignores_unknown_ids_in_a_step; finite property: map_straight overflow check removed + vignette x1e4 -> arq_110_texture_finite_input_gives_finite_output.

### Sweep (throwaway, deleted, not committed)

Release build, through the public API only. Every effect (Texture, Light, ColorAdjust, WhiteBalance, DetectedColors, ParallelShadow) in `Rgba8` and `RgbaF32`:

- Tiles against the whole image: 320 x 240, 9 tiles (corners, strips, 1 x 1) with `roi` and `cx.tile`, parameters at minimum, maximum, alternating and middle (detected colours also with eight set samples), scale 0.1, 0.25, 0.5, 1 and 2: 125 combinations, largest difference 0 in `u8` and 0 relative in `f32`. A negative control (Texture at 100 / 100 / 100, scale 0.25, same tile without the window) differs by 56/255, so the sweep is not vacuous. A second pass with 360 random parameter sets, tiles and scales on 200 x 150: largest difference 0 and 0.
- Extreme scale (`f32::MAX`, `MIN_POSITIVE`, 1e-30, 1e30, 0, -1) and extreme parameters: no non-finite output, no panic, `Rgba8` and `RgbaF32` agree on `Ok` or `Err`.
- Intensity 0 for every effect as the only step of a preset (parameters at ends, scale 0.5, 1, 2): byte-for-byte copy in `Rgba8` and bit-for-bit in `RgbaF32` (all 6 effects, including ParallelShadow); intensities 1e-300, 1e-9, 0.5, 99.999999, 100 give no non-finite output.
- Palette, 300 random images with 0 to 8 small flat colours (1 to 40 pixels), noise of amplitude 0 to 48 over 0 to 20 000 pixels and transparent pixels: entries sum to the opaque pixel count, sorted, at most five, one entry per colour when there are at most five distinct, and the noisy mass first when it holds the image: 0 bad cases. Tiny images (1 x 1 to 3 x 3) and `RgbaF32` noise with flats sum to the pixel count. `Rgba8` and `RgbaF32` palettes of the same image agree. 12 MP, one thread, release: uniform noise 288 ms, gradient 112 ms.
- Found: nothing. No fix was needed beyond the items above.

### Deviations and notes of the fix round

1. **Old tests changed** because they pinned the rejected behaviour: `reference` and the roi test of Textura (C4, order and sum of taps); `arq_122_at_most_five_entries_the_largest_areas` (the colours left out now join the nearest one, so areas add up to the image); `arq_122_f32_values_beyond_the_range_do_not_split_a_colour` (a value beyond 1 equals 1; 0.8 is another colour and, with at most five colours, its own entry).
2. **Palette merge threshold.** Fixing only the cut left a noisy colour taking three or four of the five places at noise 32 and 40. A second merge of clusters within ΔE2000 15 (the FD-26 radius, constant `CLUSTER_DELTA_E`) was added; up to 32 clusters are opened (`MAX_CLUSTERS`) so the cost stays at `BINS x 32` distances. With more than five colours, colours within 15 of each other become one entry (six greys 100 to 160 give one entry); with at most five, nothing merges (C2). The spec does not bound this.
3. **dHash.** Area weights make every cell cover the same area, so sums compare directly; `bench_dhash_12mp` reads 25.8 ms (was 9.4 to 11 ms), against the 100 ms of ARQ-114. W2's minimum size landed in `1b159f9` (128 px for 2 x 2 halves, 640 px for bilinear or nearest, no bound below), so no code branch; the half-size test gains 131 x 101, 157 x 139 and 203 x 129. The W1 test only checks pictures whose neighbouring cells differ by more than 1 % of the range, so 8-bit rounding cannot flip a bit (without that filter seed 12 of 24 differed by one bit in the 8-bit premultiply, not in the order of operations).
4. **DetectedColors C5** checks only the samples that move something (hue or saturation not 0); a non-finite conversion of an idle slot is not read, so not an error.
5. **Intensity 0 (C3)** copies after `check_scale`, `resolve` and `check_io`, so a step that would fail at 100 (for example a `cx.tile` that does not fit Texture) does not fail at 0; the preset is still validated first.
6. **S3** (even-count median takes the lower middle) is not changed: it is a recorded deviation pinned by `fd_33_d_*` tests, and the spec does not say which. **S1, S4, S5** are notes; **S6** stays untracked as told.
7. **P16 against `1b159f9`.** The rows written by the specifier match the code: `size_mismatch` with field `cx.tile`, also for a `src` of size 0, `dst` untouched, sums in `u64`, the other effects and transitions ignore the window.
