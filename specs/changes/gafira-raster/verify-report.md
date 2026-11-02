# Verify report: gafira-raster, RS5 and RS6, round 2 (closure)

- Commit: `a4ac3ae` (branch `feat/gafira-raster`), base `develop`. Depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding).
- Scope: C1 to C6 and W1 to W4 of round 1, P16, ARQ-120 and ARQ-122 at `1b159f9`, D-46, commits `b3e7ae8` to `a4ac3ae`.
- Severity follows the owner's bar: a CRITICAL must break an explicit criterion within its domain, with root cause and a concrete fix. New test gaps are follow-up warnings.
- Probes and mutants ran on a scratch export through the public API (`rs56r2_probe.rs`, `rs56r2_mutants.py`, next to this report). The repo was not edited; `HEAD` is unchanged.

**Verdict: PASS.** 0 CRITICAL, 5 WARNING (all follow-up), 6 SUGGESTION. All 10 findings of round 1 are closed.
Per class: ambiguity 2 WARNING, implementation 2 WARNING, claim 1 WARNING. No class blocks.

RS5 and RS6 were the last deliveries of the card. T4 is still owed at close (`mutation: archive`).

## Gates

Each run alone in the worktree, exit code read directly, `CARGO_TARGET_DIR` not passed.

| Gate | Exit | Note |
|---|---|---|
| `cargo fmt --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 1 803 passed, 0 failed, 0 ignored |
| `cargo deny check` | 0 | advisories, bans, licenses and sources ok |
| CI policy (7 steps of the `policy` job besides Format, read from `ci.yml` with a YAML parser) | 0 | ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13, V14, each 0 |

## Artifact integrity

- Slice card: `b64090c` and `75e360d` only. `ced4673..HEAD` removes no line of `apply-progress.md`; `a4ac3ae` only appends.
- Spec commits `0c05f00` and `1b159f9` carry no code. No commit after `1b159f9` touches `specs/` except `apply-progress.md`.
- `Cargo.lock`, `.cargo` and `crates/gafira-raster/Cargo.toml` are unchanged in the range.
- `git status`: only `crates/gafira-raster/proptest-regressions/` untracked (S6).

## Closure of round 1

| ID | Class | State | Evidence |
|---|---|---|---|
| C1 | implementation | **Closed** | Noise ±16 to ±40, 6 000 pixels with 5 flats of 800, 3 seeds each: first entry is the noisy colour at ΔE2000 0.19 to 1.28, 6 000 pixels. Gradient and natural image: entries sum to the whole image (10 000, 750 000). Test kill not reapplied: W9 |
| C2 | implementation | **Closed** | Two colours at ΔE 2 to 5, five greys 100 to 148, five pastels, and (50,50,50) with (51,50,50): one entry each, ΔE 0.0000, area order, same count in `RgbaF32` |
| C3 | implementation | **Closed** | `ParallelShadow` step at intensity 0 and -0.0: byte-identical in `Rgba8`, bit-identical in `RgbaF32`. Bad preset value and NaN scale stay `Err`. Copy removed: killed by `fd_34_b_intensity_zero_is_no_filter_*` |
| C4 | implementation | **Closed** | One run against Nitidez then Claridad in `f32`, 4 images, 4 slider pairs, 3 scales: largest gap 2.38e-7 (was 0.09). `Rgba8` run against the `f32` run of the same pixels: within byte rounding plus 0.5/255. Input luma: killed by 3 tests. `roi` max: killed by `arq_120_texture_roi_grows_by_the_active_kernels` |
| C5 | implementation | **Closed** | `mix_params` with `Srgb(1e30, ...)` and P3 `(1e30, 1e30, -1e30)` at 50, and at 1e-300: `Err` naming the parameter. `DetectedColors`, each of 8 slots, both colours, hue or saturation active: `Err` |
| C6 | implementation | **Closed** | G1, G2, G3 reapplied, each killed (`arq_110_texture_roi_and_cost_hint_survive_the_extremes` twice, `arq_110_detected_unknown_ids_are_ignored`) |
| W1 | implementation | **Closed** | 160x120 translucent, alpha 55 to 255, 120 seeds on each of 4 picture kinds: median 0, worst 1 (was 11). Near-flat noise: median 1, worst 5 |
| W2 | implementation | **Closed** | At the bounds of `1b159f9`, smooth and 16-cell block pictures: worst 7 with 2x2 means, 4 with nearest or bilinear. 38x30, below the bound: 0 of 40 over 10, worst 2 (was 13 of 40, worst 33). Residue on noise: W6 |
| W3 | implementation | **Closed** | Against `gafira_anim::Interpolate`, 2 000 000 cases in three spaces with alpha down to 0: 0 differences with `t` strictly inside `(0, 1)`. Ends: S8 |
| W4 | architecture | **Closed** | 320x240, 5 slider sets, 3 scales, 5 tiles (corner, middle, far corner, strip, 1 px): difference 0 in `u8` and `f32`; without the window 67/255. Origin window equals `None` bit for bit. Exact fit is `Ok`; 1 px over, `u32::MAX` and size 0 give `SizeMismatch` `cx.tile` with `dst` intact. `Light`, `ParallelShadow`, `DetectedColors` ignore a bad window |

Also closed: S2 (`DEFAULT_INTENSITY`, 60) and S7 (`mix_params` keeps unknown ids). P16 matches the code: `TileWindow`, `EffectCx::whole`, `check_tile` before the empty return, sums in `u64`, the doc of `run_cpu`. D-46 holds: `invert` is `ParamKind::Bool { default: false }`, and `fd_33_f_invert_twice_*` covers it.

## Rulings on the deviations

| Deviation | Ruling |
|---|---|
| Second merge at ΔE2000 15, cap of 32 clusters | **Accepted.** The radius is the FD-26 search radius. The cap was attacked with 20 to 200 small flats over a noisy mass, 750 runs: the noisy colour came first every time, nearest entry at most ΔE 12.2. The spec does not state what it does above five colours: W5 |
| Idle slot in `DetectedColors` | **Accepted.** A slot with hue and saturation 0 is not converted, so no intermediate exists, and 1e30 is a finite, valid parameter. Output is the identity; the same slot active gives `Err` |
| Intensity 0 copy order | **Accepted.** A `cx.tile` that does not fit gives `Ok` with a copy at 0 and `Err` at 50. FD-34 (b) says intensity 0 is no filter, byte for byte, and no effect runs. Preset, scale and buffers are still checked |
| W1 test margin | **Accepted.** The filter drops only pictures where 8-bit rounding can flip a bit. Measured with no filter: worst 1 on real pictures and 5 on near-flat noise, and no pair stops being similar. The strict equality on the rest is the stronger test |

The dHash bench at 25.8 ms against 100 ms is fine, as told.

## Findings

No CRITICAL.

### WARNING (all follow-up)

- **W5**, class: `ambiguity`, follow-up. ARQ-122, FD-26. `palette.rs:172`.
  - *Measured:* with more than five colours, a dropped colour moves its host. Five flats and a noisy mass give (177,193,51), a blend of green (30,160,60) and yellow (240,220,40), neither in the image. Six far flats give (91,91,91) from grey and black. Five greys give five entries, but one stray green pixel turns them into (125,125,125) x 5000. Six greys 100 to 160 give one entry.
  - *Root cause:* no criterion covers more than five colours. The join came from the fix proposed in round 1, so areas are real and colours are not always.
  - *Fix:* `specifier` states the rule, for example that a dropped colour joins only within ΔE 15 and otherwise leaves the list, with areas no longer adding up. Not a criterion broken today.
- **W6**, class: `ambiguity`, follow-up. ARQ-122 at `1b159f9`.
  - *Measured:* the amended bound says any image. On random pixel noise: 2x2 means reach 11 at 131x101 (4 of 40) and 157x139 (2 of 40); `build_mips` reaches 13 to 17 from 128 px on; nearest reaches 31 and bilinear 22 at 640 px and up. A 40-cell block picture via `build_mips` reaches 13 at 157x139 (1 of 40).
  - *Root cause:* in noise, neighbouring cells have nearly equal means, so bits follow the sampling. No dHash can meet this, and the code is not the cause.
  - *Fix:* exclude noise and fine random texture from the row, as ARQ-120 does with "suave".
- **W7**, class: `implementation`, follow-up. New test gap. `dhash.rs:333`.
  - *Measured:* M3, the `f32` arm without area weights, passes all 354 tests. `arq_122_dhash_rgba_f32_matches_rgba8` uses 36x32, a multiple of the grid. The code is right: W1 ran at 160x120.
  - *Fix:* run that test at a size that is not a multiple, for example 131x101.
- **W8**, class: `implementation`, follow-up. New test gap. `palette.rs:17`, `:21`.
  - *Measured:* M4 (`MAX_CLUSTERS` 6) and M5 (`CLUSTER_DELTA_E` 25) pass all 354 tests. The cap and the upper side of the radius are unpinned.
  - *Fix:* add a test with more flats than the cap over a noisy mass, and one with two colours at ΔE 20 among more than five.
- **W9**, class: `claim`, follow-up. Four closure mutants not reapplied.
  - *Measured:* C5 check removed, C1 dropped bins not joined, W4 vignette ignores `x0`, W1 old `f32` order. The behaviour is proved by the probes above, and `apply-progress.md` records each as killed, so only the kill by the tests is unconfirmed.
  - *Root cause:* free disk fell from 24G to 19G in `df -h` during the run, mainly from another `gafira-text` build on the machine. The hygiene rule stops builds below 20 GB, so the run was stopped.
  - *Fix:* covered by the T4 run at close, or reapply them when disk allows.

### SUGGESTION

- **S8**, class: `implementation`. New. `filter.rs:109`. When `t` rounds to 1 in `f32`, from intensity about 99.999997, `gafira-anim` returns the end colour and `lerp_color` computes it, so the space or last bits can differ (279 210 of the probe's cases aimed there). *Fix:* return `to` when `t == 1.0` and `from` when `t == 0.0`.
- **S1, S3, S4, S5**: unchanged from round 1, open as notes.
- **S6**: `proptest-regressions/` is still untracked. Delete it before the merge. All its seeds pass at HEAD.

The `Rgba8` run of Nitidez then Claridad as two separate calls differs from one call by up to 152/255. That is the clip between two 8-bit calls, not the order; the one-call result is what FD-33 asks for, so it is not a finding.

## Mutants

Closure round, on a scratch export, baseline 354 passed in the `gafira-raster` lib.

| # | Mutant | Result |
|---|---|---|
| M1 | W3 alpha as `a(1 - t) + b t` in `f32` | Killed, `fd_34_b_color_blend_is_the_interpolation_of_02_3_4_bit_for_bit` |
| M2 | W3 divide by the unrounded `f64` alpha | Killed, same test |
| M3 | dHash `f32` arm only, no area weights | **Survives, W7** |
| M4 | Palette `MAX_CLUSTERS` 6 | **Survives, W8** |
| M5 | Palette `CLUSTER_DELTA_E` 25 | **Survives, W8** |
| M6 | Palette exact list, smallest area first | Killed, 7 tests |
| G1, G2, G3 | Round 1 survivors | Killed |
| C3, C4 (2) | Copy removed, input luma, `roi` max | Killed |

12 run, 9 killed, 3 survivors, all new test gaps. 4 not run: W9.

## Criteria

23 of 23 covered at HEAD, the six rows that failed in round 1 included. The dHash half row is covered within the domain W6 proposes. Product IDs left out, as the card says: FD-33 (b, c), FD-34 (c).

## Cleanup

The export's build folder is cleaned. The export's source stays under `exp/` beside this report, because `rm -rf` is denied. Free disk at the end: 19G, in `df -h`, with the other build still running.

## Next

RS5 and RS6 pass and may merge. T4 is owed at close, and W5 to W9 go to the follow-up list.

next: archive
