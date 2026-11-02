# Verify report: gafira-raster, RS1, round 1

- Commit: `10f328b` (branch `feat/gafira-raster`, base `9124700`). Worktree clean.
- Depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding).
- Round and depth came from the coordinator. Steps 1c and 4b ran in full, on scratch exports.

**Verdict: FAIL.** 5 CRITICAL, 3 WARNING, 4 SUGGESTION.
Per class: implementation 6 (5 CRITICAL, 1 WARNING), ambiguity 2 (2 WARNING), architecture 0, claim 0.

## Gates

Each exit code was read alone, from its own file, on a clean export of `10f328b`.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 302 passed, 0 failed, 29 in `gafira-raster`) |
| CLI has no UI dependencies (ARQ-07) | 0 |
| Crates forbid unsafe code | 0 |
| Layer table (V5) | 0 |
| `clippy.toml` copies (V12) | 0 |
| Layers 0-2 know no language (ARQ-107) | 0 |
| EditToken (V13) | 0 |
| Core command property test (V14) | 0 |
| `cargo deny check` | 0 |

Artifact integrity (step 3c): `b64090c` touches only `apply-progress.md` and comes before the first implementation commit. The card lines are unchanged in `10f328b`. No commit touches `specs/02-arquitectura-software.md`, `specs/changes/layer-2/`, `docs/adr/`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.cargo/` or `.github/`.

Disk: free space fell from 25 GB to 19 765 MiB during the run because of the two other writers. The gate run had already started at 23 GB and finished. No new build started under 20 GB: `cargo clean` on this verify's own target brought it back to 23 094 MiB first. No other worktree's folder was touched.

## CRITICAL

### C1. `RgbaF32` mip: a pixel with `a = 0` adds colour

- class: `implementation`
- ref: `crates/gafira-raster/src/mip.rs:124` (`reduce_f32`), ARQ-121 "El nivel de un píxel `a = 0` no aporta color"
- Evidence: 2x2 `[(1,1,1,0), (0.1,0.2,0.3,1), (0.1,0.2,0.3,1), (1,0,0,0)]` gives level 1 `[0.55, 0.35, 0.4, 0.5]`; with the two `a = 0` pixels zeroed it gives `[0.05, 0.1, 0.15, 0.5]`. All-`(1,1,1,0)` gives `[1, 1, 1, 0]`, colour with alpha 0 in a premultiplied buffer. Probes `p13`, `p14` fail.
- Why it is a defect: the criterion has no format limit, and it only has content for such pixels, because a valid premultiplied `a = 0` pixel is already 0. `reduce_rgba8` skips them (`mip.rs:92`) and its test says "whatever bytes it holds"; `unpremultiply` for `f32` zeroes them too. The same picture gives different colour by format.
- Test gap: `arq_121_transparent_pixel_adds_no_colour` covers `Rgba8` only.

### C2. Survivor V1: `premultiply` and `unpremultiply` input check on `a = 0` pixels

- class: `implementation`
- ref: `crates/gafira-raster/src/pixel.rs`, `convert`; ARQ-110
- Mutant: the input check ignores non-finite values in pixels with `a = 0`. All 29 unit tests pass. Probe `p05` fails on it: `unpremultiply` of `(NaN, 0, 0, 0)` gives `Ok`, because `a = 0` writes zeros.
- The code is correct today (`p05` passes on it). The gap is the test: `arq_110_non_finite_pixel_is_err_and_buffer_unchanged` only uses `a = 0.5`.

### C3. Survivor V2: `build_mips` input check on `a = 0` pixels

- class: `implementation`
- ref: `crates/gafira-raster/src/mip.rs`, `build_mips`; ARQ-110
- Mutant: the same skip in `build_mips`. All 29 unit tests pass. Probe `p25` fails on it, with a non-finite value in the output. The code is correct today. `arq_110_non_finite_pixel_is_err_for_f32` only uses `a = 0.5`.

### C4. Survivor V3: ARQ-111 does not run every operation twice

- class: `implementation`
- ref: `crates/gafira-raster/src/pixel.rs:103` onward; ARQ-111 "ejecuta dos veces cada operación"
- `arq_111_premultiply_is_deterministic` covers `premultiply` on `f32` only. Nothing repeats `unpremultiply`, or either conversion on `Rgba8`.
- Mutant: entropy (`RandomState`) in `unpremultiply`, for `|a| != 0.5` only, so it stays away from the exact-value assertions. That limit makes it deliberately narrow, but the coverage fact does not depend on it. All 29 unit tests pass; `p26` fails on it. The code is correct today.

### C5. ARQ-110 property test is missing

- class: `implementation`
- ref: ARQ-110 "Prueba de propiedad: con entrada finita dentro del dominio, toda salida numérica es finita"
- `crates/gafira-raster` has no `proptest!` block, although `proptest` is a dev-dependency and `gafira-layout` has ARQ-110 property tests. The example tests do not replace the named property.
- The code holds as far as probed: `p18` ran 2 000 random finite inputs plus `±f32::MAX`, with no non-finite output. That is verify's evidence, not a test the repository keeps.

## WARNING

### W1. ARQ-121 does not decide what an odd side does

- class: `ambiguity`
- ref: ARQ-121; `apply-progress.md` assumption 2; `crates/gafira-raster/src/mip.rs`, `pair`
- Reading: dropping does not contradict "promedia caja de 2×2". Every output pixel is an exact 2x2 mean, and sizes round down as stated. But the text does not say that source pixels may be left out, and no acceptance row fixes it. Other readings fit the same text.
- Measured: the loss adds up along the chain. In 1000x600 the 1x1 level sees 512 of 1000 columns. Left 512 columns black, right 488 white: 1x1 gives 0, the exact linear mean gives 185.5. Worst measured width is 1023 (50.0 %). 3x3 `Alpha8` with the last row and column at 255 gives 0 against an exact 141.7. At level `k` the dropped part is under one output pixel per side, so the near levels only lose a thin edge; the top levels are the ones that change much.
- Why WARNING: the code follows a defensible reading and the choice is recorded, so it is not a code defect. It should still be settled before merge, because goldens and consumers will fix these bytes. `tdd-apply` should have stopped and reported this as `blocked` rather than choosing.
- Question for the owner, through `specifier`: keep the drop and state it in ARQ-121, or require every source pixel to count, as the top level's mean suggests.

### W2. One error enum per crate against two names in raster

- class: `ambiguity`
- ref: 02:1308 ("Un enum de error por crate"), 02:529 and 02:742 (`EffectError`), P4 (`BufferError`)
- Raster is headed for two enums at the first `Effect` or `Transition`. RS1 itself is fine; it has only `BufferError`. Settle it before RS3, which builds the first `Transition::run_cpu`, or RS4, whichever needs `EffectError` first.

### W3. Weak tests on two ARQ-121 rows

- class: `implementation`
- ref: `crates/gafira-raster/src/mip.rs` tests
- "Imagen constante" runs on `Rgba8` only, and the odd-side test covers a dropped column only, not a row. Probes `p10` and `p22` show the code behaves as chosen; the repository does not hold that proof.

## SUGGESTION

- S1. `crates/gafira-raster/src/pixel.rs:53-55`: `if a == 0 { p[3] = 0; }` is dead code, since `a` is already 0. T4 may report it.
- S2. `mip_level_count(0, 5)` gives 0 while the literal formula gives 3, and `mip_size(0, 5)` gives `(1, 2)`. Both are defensible; one spec line on empty images would close this.
- S3. An invalid `Rgba8` pixel with `c > a` is not kept: constant `(200, 100, 50, 100)` gives `(100, 100, 50, 100)`. It is outside premultiplied alpha, and ARQ-117 caps the result at `a`, so this is not a finding. The owner may want it recorded when ARQ-121 is touched for W1.
- S4. `apply-progress.md` assumption 5 speaks of "the two constructor variants of P4"; P4 lists no variants. Assumption 3 says the formats are not spelled out, but 02 §6.2 gives 8-bit sRGB premultiplied for Preview and `f32` linear for Export.

## Implementer's assumptions

| Assumption | Class | Basis | Result |
|---|---|---|---|
| Empty image gives 0 levels | Derivable | `log2(0)` is undefined; 02 §4.17.4 "Imagen de tamaño 0: `Ok` sin cambios"; no pixels to make a level from. The row is tagged ARQ-118 but sits in the raster table | No finding; S2 for 0xN |
| Odd side drops the last row or column | Ambiguity | Not contradicted by ARQ-121, and not decided by it | W1 |
| Side of 1 repeats a pixel | Derivable | `(a+a+b+b)/4 = (a+b)/2`, the only mean available on a side of 1 | No finding |
| `RgbaF32` averaged as is | Derivable, with one exception | 02 §6.2 and ARQ-117 give linear premultiplied, so the plain mean is right. The exception is `a = 0`, where ARQ-121 states the rule | C1 |
| `Alpha8` plain rounded mean | Derivable | Coverage has no colour or transfer curve | No finding |
| Extra API beyond P4 | Derivable, partly stated | `ImageViewMut` is in 02 §4.17 and `EffectCx` in P5, with the exact shape. `PixelDataMut`, `ImageBuf`, `PixelBuf`, `TileGrid` and the helpers follow from what RS1 must return. P4's frozen shape is intact, and the additions are only additions | No finding |
| `NonFinite` variant | Stated, its home derivable | ARQ-110 requires a typed `Err` naming the field; P4 lists no variants, and `#[non_exhaustive]` leaves `SG` safe. Key and arguments meet ADR 0012 | No finding; W2 on the wider question |
| 8-bit mip ignores the transform's `Err` | Derivable | 02:692 gives `LengthNotMultiple` as its error, and the buffer has 4 elements. `p09` and `p16` agree | No finding |

## Criteria

| # | Criterion | Status | Test or probe |
|---|---|---|---|
| 1 | ARQ-117, 65 536 pairs, `<= a`, exact rounding | covered | `arq_117_premultiply_is_exact_rounding_and_at_most_alpha`, `p01` |
| 2 | ARQ-117, unpremultiply formula | covered | `arq_117_unpremultiply_rounds_clamps_and_zero_alpha_is_zero`, `p02` |
| 3 | ARQ-117, `a = 255` identity | covered | `arq_117_opaque_is_identity_and_transparent_is_zero`, `p02` |
| 4 | ARQ-117, `a = 0` gives 0 | covered | same, plus `arq_117_view_*`, `p02` |
| 5 | ARQ-117, wrong length is `Err` | covered | `arq_117_length_mismatch_is_err_for_each_format`, `p03` |
| 6 | ARQ-117, overflow is `Err`, no allocation | covered | `arq_117_size_overflow_is_err_without_allocation`, `p03` |
| 7 | ARQ-121, 256 px tiles, 257x1 gives 2 | covered | `arq_121_tile_counts_at_the_256_boundary`, `p15` |
| 8 | ARQ-121, 1000x600, 10 levels, last 1x1 | covered | `arq_121_chain_of_1000_by_600_has_ten_levels_ending_1x1`, `p08` |
| 9 | ARQ-121, constant image | covered, weak (W3) | `arq_121_constant_image_is_equal_at_all_levels`; `p09` all valid `(c, a)`, `p10` |
| 10 | ARQ-121, checkerboard 188 ±1 | covered | `arq_121_checkerboard_level_one_is_188`; `p11` gives 188 |
| 11 | ARQ-121, `a = 0` adds no colour | **failing** for `RgbaF32` (C1) | `arq_121_transparent_pixel_adds_no_colour`, `p12`, `p13`, `p14` |
| 12 | ARQ-121, linear premultiplied mean | covered | `arq_121_level_one_matches_a_linear_premultiplied_reference`; `p16`, 200 000 boxes, colour within 1, alpha exact |
| 13 | ARQ-110, table, `Err` names the field, buffer unchanged | weak (C2, C3) | `arq_110_*`, `p04` to `p07`, `p25` |
| 14 | ARQ-110, finite input gives finite output, property | **missing** (C5) | `p18` only |
| 15 | ARQ-110, constructors do not validate | covered | `arq_110_constructors_accept_non_finite_values`, `p24` |
| 16 | ARQ-111, each operation twice | weak (C4) | `arq_111_*`, `p19`, `p26`. No `rayon` in the crate, so no pool test applies |
| 17 | P4, shape and derives | covered | `p24`, code read |
| 18 | ADR 0005, type homes | covered | `lib.rs` exports |
| 19 | ADR 0012, keys and arguments | covered | `buffer_error_maps_to_message_keys`, `p20` |

Covered 15 of 19. Product IDs: ARQ-117, ARQ-121, ARQ-110 and ARQ-111 all have named tests (`git grep`). None were left out of RS1. `Tile`, `TileSet`, `Cost`, `HandleReq` and the GPU traits are outside the RS1 row.

## Probes (step 1c)

26 probes were derived from the rows before the tests were read (`p25`, `p26` added afterwards, to confirm the survivors). 24 pass, 2 fail (`p13`, `p14`, C1). They live only under the scratchpad.

## Hand mutants (step 4b)

| Mutant | Rule | Result |
|---|---|---|
| Spot-check, recorded item 3: `min(255)` removed | ARQ-117 | Killed by `arq_117_unpremultiply_rounds_clamps_and_zero_alpha_is_zero`, as recorded |
| V1: input check skips `a = 0` pixels in `convert` | ARQ-110 | **Survived** (C2); `p05` fails on it |
| V2: input check skips `a = 0` pixels in `build_mips` | ARQ-110 | **Survived** (C3); `p25` fails on it |
| V3: entropy in `unpremultiply` | ARQ-111 | **Survived** (C4); `p26` fails on it |

Each survivor changes observable behaviour, so none is equivalent. The real code passes all three probes.

## Size

1 367 lines, about 670 of them tests. The owner said size does not concern him, so this is noted and is not a finding.

next: specifier (W1, W2), then tdd-apply (C1 to C5, W3)
