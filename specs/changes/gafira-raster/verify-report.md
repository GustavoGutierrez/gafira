# Verify report: gafira-raster, RS5 and RS6, round 1

- Commit: code at `d79f9ea` (branch `feat/gafira-raster`), base `develop`, merge base `d64a41f`.
- HEAD moved twice during the run, to `72d248e` and `8a8d2f3`. Both only touch specs (Invertir as a switch, D-46). `git diff d79f9ea HEAD -- crates Cargo.toml Cargo.lock` is empty, so the code verified is the code at HEAD, and the spec text read for FD-33 is the current one.
- Round 1, depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- Probes ran on a scratch export through the public API, written from the spec rows before the implementer's tests were read. Mutants ran on a second clean export. Build folders are cleaned.
- Invertir as a switch is out of scope as told, and D-46 has since settled it. Size is not a finding.

**Verdict: FAIL.** 6 CRITICAL, 4 WARNING, 7 SUGGESTION.
Per class (CRITICAL and WARNING): implementation 9 (6 CRITICAL, 3 WARNING), architecture 1 (1 WARNING). SUGGESTION: implementation 5, ambiguity 1, claim 1.

The value tests are strong: 20 of 20 value mutants died and the formula probes agree to rounding. The defects sit where no test looks: the palette on images that are not flat, a filter on an effect whose defaults are not the identity, the Textura order, colour conversion overflow, and the ARQ-110 tables of the new effects.

## Gates

Each run alone, exit code read directly, `CARGO_TARGET_DIR` not passed.

| Gate | Exit | Note |
|---|---|---|
| `cargo fmt --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 1 769 passed, 0 failed, 0 ignored; 320 in the `gafira-raster` lib |
| `cargo deny check` | 0 | advisories, bans, licenses and sources ok |
| CI policy (7 steps of the `policy` job besides Format, read from `ci.yml` with a YAML parser) | 0 | ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13, V14, each 0 |

`git status` at the end: only `crates/gafira-raster/proptest-regressions/` untracked (S6).

## Artifact integrity

- Slice card: commits `b64090c` and `75e360d` only. No commit since `d64a41f` removes a line of `apply-progress.md`, so no implementer commit changed the card.
- Spec commits in the range: `b9a7442`, `72d248e`, `8a8d2f3`. None carries code; each states its purpose in its subject.
- `.cargo/mutants.toml` is untouched, so there is no exclusion to re-check. `Cargo.lock` is unchanged; `crates/gafira-raster/Cargo.toml` gains only the `dhash` bench target.
- `develop` is at `65a7bb5`, ahead of the merge base. The branch needs `develop` merged before integration (not a finding).

## Rulings on recorded deviations

| Deviation | Ruling |
|---|---|
| `DetectedColors` with 8 slots as its own effect (RS6 dev. 3) | **Accepted.** The formula holds: 34 698 pixels inside the tolerance match an independent reference within 3e-8, and 65 302 outside are unchanged bit for bit. FD-33 lists it inside the Color group, so the stack order now depends on the caller: S4 |
| Palette algorithm (RS6 dev. 1) | **Rejected as built.** The algorithm is free, but this one fails ARQ-122: C1 and C2 |
| dHash algorithm (RS6 dev. 2) | **Accepted**, 9 x 8 cells on stored sRGB values. Distance is at most 7 on 2x2, bilinear and nearest halves from 640 px up, and at most 6 on 2x2 halves from 128 px, the 12 MP bench reads 9.4 ms, noise pairs are 20 to 43 apart. Two repairs: W1, W2 |
| Local colour blend (RS6 dev. 4) | **Accepted as a local copy**, since V5 does not list `gafira-anim` for raster. It is not the same rule as written, though: W3 |
| Textura blurs read the input (RS5 dev. 6) | **Rejected.** It is recorded as "spec silent", but FD-33 states the order: C4 |
| Vignette only on whole images (RS5 dev. 7) | **Open:** W4 |

## Findings

### CRITICAL

- **C1**, class: `implementation`. ARQ-122 ("dominantes, ordenados por área"), FD-26 (a). `palette.rs:124`.
  - *Measured:* 6 000 pixels of noise around (200, 60, 60) plus 5 flat colours of 800 pixels each: the palette holds the 5 flat colours, and the colour with 60 % of the image is missing, at noise ±16, ±24, ±32 and ±40. A 70 % red-to-blue gradient with five 6 % flats gives only the flats. On a 1000x750 natural image, entries cover 234 740 of 750 000 pixels.
  - *Root cause:* bins go in fullest first and the list closes at five clusters. A noisy colour spreads over many thin bins, so five small flat colours fill the list first, and every later bin is thrown away. A search by colour (FD-26) cannot find that image.
  - *Test gap:* `arq_122_a_noisy_large_colour_comes_before_a_small_flat_one` has only one flat colour, so the list never fills.
  - *Fix:* keep opening clusters, sort by area, cut to five, then add each dropped bin to its nearest kept cluster so the areas are real. Tried on the export without the last step: all 320 tests pass, the noisy colour comes first at ±16, ±24 and ±32, and the gradient now gives five gradient entries. At ±32 the colour splits into three clusters, and at ±40 into four, so the merge threshold needs a look at the same time. Palette time at 12 MP goes from 97 to 193 ms.
- **C2**, class: `implementation`. ARQ-122 criteria "Imagen de 2 colores: 2 entradas" and "5 colores planos: los 5 colores con ΔE2000 < 1". `palette.rs:12`, `:123`.
  - *Measured:* two flat colours with ΔE2000 from 2.4 to 4.8 give one entry, for example (200, 60, 60) and (215, 70, 60) give (205, 63, 60). Five greys 100 to 148 give two entries. Five pastels give four. At ΔE 10.1 two entries come back.
  - *Root cause:* merging below ΔE 10 is the implementer's choice; the criteria have no such bound. Tests use only far-apart colours.
  - *Fix:* when the image has at most `PALETTE_SIZE` distinct straight colours, return each as its own entry, without binning or merging, and test it with two close colours and the five greys. If the owner prefers near colours to count as one, the criterion must say so; that is a spec amend, not code.
- **C3**, class: `implementation`. FD-34 (b) "intensidad 0 equivale a sin filtro, byte a byte". `filter.rs:257`.
  - *Measured:* a preset with one `effect.shadow.parallel` step at intensity 0 changes 596 bytes in `Rgba8` and 768 values in `RgbaF32` of a 32x32 image with `Ok`.
  - *Root cause:* `apply` runs each effect at its defaults, and `ParallelShadow` defaults are not the identity (offset 4, opacity 50). FD-33 effects are the identity, and the test uses only `test_preset`.
  - *Fix:* after the checks, when `intensity == 0.0` copy `src` to `dst`. Keep validating the preset first (mutant I10 proves that is tested). Add a test with `ParallelShadow`.
- **C4**, class: `implementation`. FD-33 "Orden de cálculo: ... Textura (Nitidez, Claridad, Viñeta)". `texture.rs:243`, `:258`.
  - *Measured:* on a 256x192 image, `Texture` with both sliders against the same sliders run in order differs by up to 63/255 at 100 and 100, with 22 314 of 196 608 values above 1/255, and up to 23/255 at 50 and 50. In `f32` the gap reaches 0.09. Sliders alone match.
  - *Root cause:* Claridad's `Y` and `Y_G` read the input, not the output of Nitidez. The implementer's reference does the same, so the tests fix the deviation in place, and mutant I2 shows them holding it.
  - *Fix:* compute Nitidez over the image, build Claridad's blur from that result, then the Viñeta. `roi` must then be the sum of both tap counts, not the larger. Update `reference` in the tests and the note in `apply-progress.md`.
- **C5**, class: `implementation`. ARQ-110 ("un resultado intermedio no finito", "un no finito en la salida"), same rule as C1 of RS3a. `filter.rs:90`, `detected.rs:189`.
  - *Measured:* `mix_params` with a preset colour `Srgb(1e30, ...)` at intensity 50 returns `Ok` holding `r: inf`; with Display P3 `(1e30, 1e30, -1e30)` it returns `inf, inf, -inf`. `DetectedColors` with those samples returns `Ok` and hides the overflow: `lab_of` clips it and the colour moves nothing.
  - *Root cause:* a finite `Color` passes `check_values`, and `to_linear_srgb` then overflows. Neither site checks the result. `apply` is safe only because the next effect rejects the value.
  - *Fix:* check `is_finite` on the converted colour in `lerp_color` and in the sample, and return `Err` naming the parameter, as Fade does. Add both to the tests.
- **C6**, class: `implementation`. ARQ-110 acceptance. `white_balance.rs`, `light.rs`, `color_adjust.rs`, `texture.rs`, `detected.rs`, `filter.rs` tests.
  - *Missing for all five new effects and the filter:* the table with an unknown id holding NaN or `±∞`, `roi` and `cost_hint` at their extremes (`px = u64::MAX`, `out` at the ends of `i32`, `scale = f32::MAX`, unresolved values, NaN scale), and the finite-output property. These exist only for `ParallelShadow` and the transitions. The RS5 and RS6 sweep covered much of this and was deleted.
  - *Three survivors in critical code:* G1 `px * work` in `Texture::cost_hint`, G2 `assert!` on a finite scale in `Texture::roi`, G3 `DetectedColors` behaving differently for an unknown id.
  - *The behaviour itself is right:* a probe with overflow checks on passes every `roi` and `cost_hint` case over 86 parameter sets and all 8 scales, and unknown ids leave `Rgba8` and `RgbaF32` output bit-identical.
  - *Fix:* add generic helpers to `testing.rs` (`assert_unknown_ids_are_ignored`, `assert_roi_and_cost_extremes`) called from each effect, and one `proptest!` per effect, as in `shadow.rs`.

### WARNING

- **W1**, class: `implementation`. ARQ-122, FD-28 (b). `dhash.rs:81`.
  - *Measured:* a translucent image (alpha 55 to 255) in `Rgba8` and the same image in `RgbaF32` are up to 11 apart, median 6, so `is_similar` can say no. Opaque images and uniform alpha give 0.
  - *Root cause:* bytes are `encode(c) × a`; `f32` hashes `encode(c × a)`. Mutant I12 dies, but no test has varying alpha across formats.
  - *Fix:* in the `f32` arm, divide by alpha, encode, multiply by alpha.
- **W2**, class: `implementation`. ARQ-122 "una imagen y su versión al 50 %: distancia ≤ 10". `dhash.rs:16`.
  - *Measured:* at 2x2 halves with no noise, 38x30 exceeds 10 on 13 of 40 scenes, worst 33; 37x29 on 22 of 40, worst 37; 91x81 on 1 of 40, worst 11. From 128 px up the worst is 6, and multiples of the grid (36x32, 72x64) give at most 2. The tests start at 128x96.
  - *Root cause:* a side that is not a multiple of the grid gives cells sharing whole border pixels, so cells carry uneven weight. Recording the `floor` split as an equivalent mutant hid this.
  - *Fix:* weight border pixels by area. A prototype gives 38x30 0 of 40 over 10 (worst 5) and 91x81 worst 10. It does not rescue 37x29 or 17x13, where the half is close to the grid size, so ARQ-122 also needs a minimum size (`specifier`).
- **W3**, class: `implementation`. FD-34 "regla de `Color` de 02 §3.4", alpha "con la regla de `f32` (bit a bit)". `filter.rs:93`.
  - *Measured:* against `gafira_anim::Interpolate` with the same `t`, alpha differs in 16.6 % of cases at alpha ≥ 0.1 (up to 10 ulps) and 39 % below 0.1 (up to 140 ulps). RGB is within 4.77e-7 at alpha ≥ 0.1 and reaches 5.5e-6 below 0.01, over the 4.8e-7 of §3.4.
  - *Root cause:* alpha is computed in `f64` and cut to `f32`, and it divides unrounded; `gafira-anim` computes it in `f32`. The `f64` result is more exact but is not the stated rule.
  - *Fix:* compute `alpha` in `f32` as `a.a + (b.a - a.a) * t as f32` and divide by that value, with `t` passed as `f32`. Pin it with values taken from `gafira-anim`.
- **W4**, class: `architecture`. ARQ-120 "Ejecutar por tiles con `roi` coincide con la imagen entera", 02 "Paralelismo de datos" (tiles belong to `gafira-render`). `view.rs:294`, `texture.rs:202`.
  - *Measured:* the top-left 64x64 tile with Viñeta 80 differs from the whole image by 47/255, because the vignette centres on the tile.
  - *Root cause:* `EffectCx` holds only `scale`, so an effect cannot know the image or where the tile sits. `Effect::run_cpu` says "on a whole image"; 02 only says so for `Transition` (SP23).
  - *Fix, a decision:* add the image size and tile origin to `EffectCx`, or state in 02 that a `Texture` with Viñeta runs on whole images. It must be settled before `gafira-render` uses effects.

### SUGGESTION

- **S1**, class: `implementation`. FD-33 (f). Claridad at 100 on a flat `f32` image changes 428 491 of 614 400 values by up to 2.09e-7, which is blur rounding and well under the ±1e-5 of (e). `Rgba8` is unchanged, and so is `f32` with Nitidez alone.
- **S2**, class: `implementation`. FD-33 table: Intensidad "por defecto 60" lives nowhere in the crate. Making it not an effect is right, because (a) could not hold otherwise. *Fix:* a public constant.
- **S3**, class: `implementation`. RS5 dev. 8: the median of an even count is the lower middle value. Two pixels 50 and 200 take 50. This matters only on very small images.
- **S4**, class: `ambiguity`. FD-33 puts detected colours inside Color, before Textura. As a separate effect, that order is the caller's. *Fix:* one line in 02 §4.17.4, or a stack-order constant.
- **S5**, class: `claim`. CPU time at 12 MP, one thread: Contraste 2 959 ms, Claridad 1 796 ms, detected colours 2 638 ms with 1 slot and 13 574 ms with 8. The budgets of FD-33 and FD-34 are outside the card, and preview is GPU, so this is information for `gafira-render`, not a miss. `powf` per pixel is the cost.
- **S6**, class: `implementation`. `crates/gafira-raster/proptest-regressions/` is still untracked. Delete it before the merge.
- **S7**, class: `implementation`. ARQ-110 says an unknown id "se conserva intacto"; `mix_params` drops it (`filter.rs:168`), unlike `resolve`. Output does not change. *Fix:* copy unknown ids through.

## Criteria

| Criterion | Evidence | State |
|---|---|---|
| FD-33 (a) identity, switch off | `fd_33_a_*` (exhaustive over 32 896 pairs), probe | Covered |
| FD-33 (d) idempotent | `fd_33_d_*`, probe (`Rgba8` and `f32` agree to 3e-6) | Covered; S2, S3 |
| FD-33 (e) analytic values | `fd_33_e_*`, probe: all exact to 1e-7; random sliders against an `f64` reference within 2.4e-7 | Covered |
| FD-33 (f) alpha, flat, centre, invert twice | `fd_33_f_*`; probe: every (c, a) inverts back with difference 0, alpha kept | Covered; S1 |
| FD-33 (g) eyedropper | `fd_33_g_*`; probe: 113 064 greys, 8-bit spread 0 | Covered |
| FD-33 (h) bad parameters | `fd_33_h_*`, probe | Covered |
| FD-33 Orden de cálculo, Textura | Tests pin the wrong order | **Failing, C4** |
| FD-33 detected colours | `fd_33_detected_*`, probe | Covered; S4 |
| ARQ-122 five flat colours in area order | `arq_122_five_flat_colours_in_order_of_area` | **Failing on close colours, C2** |
| ARQ-122 two colours, two entries | `arq_122_two_colours_give_two_entries` | **Failing on close colours, C2** |
| ARQ-122 transparent image | `arq_122_transparent_and_empty_images_give_an_empty_list` | Covered |
| ARQ-122 dHash half | `arq_122_dhash_an_image_and_its_half_are_similar`, probe | Covered from 128 px; W1, W2 |
| ARQ-122 dHash noise | `arq_122_dhash_noise_with_different_seeds_is_not_similar`; probe minimum 20 | Covered |
| FD-26 palette | Tests above | **Failing, C1** |
| FD-28 (b) resized image | Probe: at most 4 at 73 % and at 25 %, from 640 px up | Covered; W2 |
| FD-34 (a) thumbnail | `fd_34_a_*`; probe with translucent blocks, intensities 50 and 100, both formats: difference 0 | Covered |
| FD-34 (b) | `fd_34_b_*` | **Failing at intensity 0, C3**; rest covered; W3 |
| FD-34 preset mechanism, test preset | `fd_34_the_test_preset_*`, `fd_34_errors_*`; probe: overflow in step 2 and a bad value leave `dst` untouched | Covered |
| ARQ-114 `bench_dhash_12mp` | `benches/dhash.rs`, run on the export: 9.4 ms against 100 ms | Covered |
| ARQ-110 | Tables, property, extremes for the new effects | **Missing, C6; failing, C5** |
| ARQ-111 | `arq_111_*` per operation; no threads in the crate | Covered |
| W7 of RS4 (`5dca6d4`) | `arq_120_half_size_tinted_rgba_f32_*`, `arq_120_half_size_translucent_tinted_rgba8_*`, against the amended bound of `b9a7442` | Closed |
| S4 of RS4 (`5dca6d4`) | `blur.rs:4`, `:374` cite FD-08 | Closed |

Product IDs left out, as the card says: FD-33 (b, c), FD-34 (c).

## Mutants

No `cargo-mutants` run (`mutation: archive`). All on a scratch export. The baseline is 320 passed.

**Sample of the recorded list.** 8 drawn at random, seed 20261102, from 22 concrete items that stand for the RS5 and RS6 tables. All 8 are killed by the test the record names or one of its family, so the record holds up: Int `floor`, Contraste without the clip, merge 60, a hue coefficient, Luces edge 0.4, `SIMILAR_DISTANCE` 11, temperature `/ 100`, `alpha < 0`.

**Independent.**

| # | Delivery | Mutant | Result |
|---|---|---|---|
| I1 | RS5 | Saturación before Tono | Killed, `fd_33_detected_*` and color tests |
| I2 | RS5 | Viñeta before Claridad | Killed by the Textura reference tests |
| I3 | RS5 | Negros from linear `Y`, not `v_Y` | Killed, `fd_33_d_whites_and_blacks_*` |
| I4 | RS5 | Vitalidad spread read before Tono | Killed by the color reference tests |
| I5 | RS5 | `check_io` skips the alpha channel | Killed, `arq_110_*_non_finite_scale_and_pixels_are_err` |
| I6 | RS6 | Every step reads `src`, no chaining | Killed, `fd_34_b_intensity_100_runs_the_preset_in_order` |
| I7 | RS6 | Empty preset leaves `dst` unwritten | Killed, `fd_34_errors_leave_dst_untouched` |
| I8 | RS6 | dHash cell areas crossed | Killed, `arq_122_dhash_odd_sizes_*` and two more |
| I9 | RS6 | Palette check skips alpha 0 pixels | Killed, `arq_110_palette_*` |
| I10 | RS6 | Preset not validated at intensity 0 | Killed, `fd_34_b_a_preset_value_out_of_range_is_err` |
| I11 | RS6 | Slot Saturación before Tono | Killed, `fd_33_detected_*` |
| I12 | RS6 | dHash `f32` without the transfer curve | Killed, `arq_122_dhash_channel_weights` |
| G1 | RS5 | `px * work` in `Texture::cost_hint` | **Survives, C6** |
| G2 | RS5 | `assert!` finite scale in `Texture::roi` | **Survives, C6** |
| G3 | RS6 | `DetectedColors` differs on an unknown id | **Survives, C6** |

## Next

RS5 and RS6 do not pass; nothing merges. C1 to C6 and W1 to W3 go to `tdd-apply`. W4 needs its decision first, and the minimum size in W2 goes to `specifier` with it. T4 stays owed at close.

next: architect (W4), then tdd-apply (implementation)
