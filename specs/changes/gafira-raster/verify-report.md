# Verify report: gafira-raster, RS1, round 2

- Commit: `888f54b` (branch `feat/gafira-raster`, contains `develop`). Closure round; round 1's report is at `e77d0e9`.
- Depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- Round and depth came from the coordinator. Steps 1c and 4b do not run in a closure round. V1 to V3 were re-applied to check closure, as the coordinator asked.

**Verdict: PASS for RS1.** 0 CRITICAL, 2 WARNING (both follow-up), 3 SUGGESTION. RS2 to RS6 remain, so the change is not done.
Per class: implementation 1 (1 WARNING), ambiguity 1 (1 WARNING), architecture 0, claim 0.
All eight findings of round 1 are closed: C1 to C5, W1, W2, W3.

## Gates

Each exit code was read alone, from its own file, on a clean export of `888f54b`.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1 314 passed, 0 failed, 41 in `gafira-raster`) |
| CLI has no UI dependencies (ARQ-07) | 0 |
| Crates forbid unsafe code | 0 |
| Layer table (V5) | 0 |
| `clippy.toml` copies (V12) | 0 |
| Layers 0-2 know no language (ARQ-107) | 0 |
| EditToken (V13) | 0 |
| Core command property test (V14) | 0 |
| `cargo deny check` | 0 |

Disk: 34 GB free at the start, 30 GB at the lowest. Both scratch targets were cleaned afterwards.

## Artifact integrity (steps 3, 3c)

- `b64090c` touches only `apply-progress.md`, before the first implementation commit. The 8 card lines at HEAD are identical to that commit.
- No implementer commit changed the card or a spec. `cfdd474` and `87148cd` touch only `crates/gafira-raster/src/`; `888f54b` touches only `apply-progress.md`.
- `specs/02-arquitectura-software.md` changed only in `a7726de` and `4e29a87`, one file each, through `docs/rs1-specs` and `develop`, not from an implementer commit. `cfdd474` comes after both.
- No delivery commit touches `.cargo/`, `.github/`, `Cargo.toml`, `Cargo.lock` or `deny.toml`. `develop` is fully merged.

## Closure of round 1

| Id | Status | Evidence |
|---|---|---|
| C1 | closed | `mip.rs:228` zeroes `a = 0` pixels, -0.0 included. The round 1 case gives `[0.05, 0.1, 0.15, 0.5]`, all-`(1,1,1,0)` gives zeros. Probe `p4` matches an exact mean bit for bit on 137 sizes |
| C2 | closed | V1 re-applied: killed by `pixel::arq_110_non_finite_colour_in_transparent_pixel_is_err`, the only failure |
| C3 | closed | V2 re-applied: killed by `mip::arq_110_non_finite_colour_in_transparent_pixel_is_err`, the only failure. `p8` holds at every index of a 3x3, with alpha 0 and not |
| C4 | closed | V3 re-applied, entropy for `|a| != 0.5`: killed by `arq_111_every_conversion_is_deterministic`, the only failure. That test runs both conversions twice on `f32` and `Rgba8`. `p9` agrees |
| C5 | closed | `proptest!` blocks `arq_110_finite_input_gives_finite_output`, `arq_110_unit_range_premultiply_is_ok_and_finite`, `arq_110_finite_mips_are_finite`. The named property is held by the repository. Weakness: W5, new |
| W1 | closed | 02:951 now decides the odd side, and `mip.rs` follows it. See "Amended ARQ-121" |
| W2 | closed | Ruling below |
| W3 | closed | `arq_121_constant_image_all_formats` covers `f32` and `Alpha8` on odd sizes. Odd rows are in the `Alpha8` and `Rgba8` odd-side tests. M4, M5, M6 below killed |

S1 and S2 of round 1 were also acted on: the dead code is gone, and `arq_121_zero_side_images_have_no_levels` fixes the zero-side behaviour.

### Ruling on W2: deferral accepted, finding closed

The finding was a contradiction in the spec, and `4e29a87` removes it. 02:1308 now names `BufferError` as the exception for a type several crates share, lists `EffectError`, and says `EffectError` wraps `BufferError` in a variant with `From`, with `to_message` delegating. That matches the `Cancelled` pattern of ADR 0012 item 4.

RS1 owes no code for it. `EffectError` does not exist, no `Effect` or `Transition` exists to return it, and `BufferError` already has `ToMessage`. Adding an unused enum now would be speculative.

What remains is an implementation duty of the delivery that adds `EffectError`, not an open question. The first signature that needs it is `Transition::run_cpu` (02:742), so RS3 by the delivery order. One gap: that duty is recorded only in `apply-progress.md` and in 02:1308, which no RS3 product ID cites, so verify's per-ID check would not reach it. See S5.

## Amended ARQ-121

Probes ran on a scratch export, written from the row, and all 9 pass (`p1` to `p9`).

| Criterion | Result | Test | Probe |
|---|---|---|---|
| 3x1 and 1x3 `Alpha8` with 0, 0, 255 give 85 ±1 | covered | `arq_121_odd_side_counts_every_pixel_alpha8` | `p1` |
| 3x3 `Alpha8`, last row and column 255, gives 142 ±1 | covered | same | `p1` |
| 5x1 `Alpha8`, level 1 gives 0 and 170 ±1 | covered | same | `p1` |
| 3x1 `Rgba8` opaque, sRGB 0, 0, 255, gives 156 ±1 | covered, also 1x3 | `arq_121_odd_side_counts_every_pixel_rgba8` | `p2` |
| 1000x600 opaque, 512 at 0 and 488 at 255, 1x1 level gives 185 ±2 | covered, measured 185; exact 185.486 | `arq_121_wide_image_top_level_is_the_exact_mean` | `p2` |
| 1x1 has 1 level, equal to the input | covered, all formats | `arq_121_one_by_one_has_one_level_equal_to_input` | `p1` |
| Every level pixel is the equal-weight mean of the originals it covers | covered | the rows above | `p3`, `p4`, `p5` |
| Constant image | covered, every valid `(c, a)` pair in `Rgba8` | `arq_121_constant_image_*` | `p6` |

`p3` to `p5` derive the covered set on their own, by mapping each original index down the chain, and compare every level on 137 sizes, among them 1x1025, 1023x3 and 33x65. `Alpha8` matches exactly, `f32` matches bit for bit, `Rgba8` alpha matches exactly and colour within 1. Each level is a partition of the originals, so no pixel is lost. Earlier criteria were not retested beyond the gates.

Mutants, killed and each confirmed for the right reason. None is new budget: V1 to V3 are the survivors of round 1, and M4 to M6 check W3.

| Mutant | Killed by |
|---|---|
| V1, V2, V3 | as in the table above |
| M4: last row not joined to the last box, column kept | `arq_121_odd_side_counts_every_pixel_alpha8`, `..._rgba8`, the wide image test and both constant tests |
| M5: `f32` divides by 4, not by the area | `arq_121_constant_image_all_formats`, `arq_121_alpha8_and_f32_levels` |
| M6: `Alpha8` divides by 4, not by the area | `arq_121_constant_image_all_formats`, `arq_121_odd_side_counts_every_pixel_alpha8` |

## Ruling on memory

Measured with `VmHWM` in a release build on the development machine, on a real image in each row. Sizes are MiB. "Added" is the peak above the source.

| Input | Added | Process peak | Levels kept | Time |
|---|---|---|---|---|
| `Rgba8` 4000x3000 (12 MP) | 503 | 551 | 61 | 0.5 s |
| `Alpha8` 4000x3000 | 469 | 483 | 15 | 0.2 s |
| `RgbaF32` 4000x3000 | 641 | 826 | 244 | 0.3 s |
| `Rgba8` 4000x4000 (16 MP) | 671 | 735 | 81 | 0.7 s |
| `Rgba8` 8192x8192 (67 MP) | 2 816 | 3 074 | 341 | 2.8 s |
| `Rgba8` 16384x16384 (268 MP) | 11 264 | 12 291 | 1 365 | 10.5 s |

The "about 380 MB" in `apply-progress.md` is right for the base sums alone and low for the peak. `halve` holds level 0 and level 1 sums together, and the copy of level 0 is live too, so the added peak at 12 MP is about 500 MiB. "Then a quarter" holds only after that first step.

Budgets stated in `specs/`:

- CAL-26: at most 1.5 GB with 20 pages and images (FIJADA, D-03), on the 8 GB profile.
- CAL-22, ARQ-44, 02 §6.7: the configured limit plus 10 %, tested at 1 GB, default 50 % of RAM.
- 04 FD on import: images over 16 384 px per side are reduced with a notice (PROPUESTA), so 16 384 is the stated maximum. Pages use the same limit.
- No budget names the mip builder. ARQ-121 has no number, §6.7 has no mip row, and ARQ-114 has no mip benchmark.

Ruling:

- **12 MP and 4000 px: no stated budget is broken.** One build takes 37 % of CAL-26 and fits a 1.1 GiB address limit, confirmed by a run under `ulimit -v`. Concurrent builds are the limit: about two reach the 1.1 GB ceiling and three pass 1.5 GB.
- **Maximum size: it does not fit.** 12.3 GiB is 8 times CAL-26, 3 times the default limit on the 8 GB profile, and more than that profile's RAM. Under a 4 GiB limit the 16 384 px run aborted with SIGABRT, exit 134, `memory allocation of 8589934592 bytes failed`. 8192x8192 aborted the same way under 1.1 GiB. An abort is no `Err`, and `catch_unwind` (ADR-05, ARQ-54) cannot turn it into a failed job. These runs are a limit on one machine, not a run on the reference hardware.
- **Why this is not CRITICAL.** The budgets are process-wide, nothing calls `build_mips` yet, and no RS1 criterion covers memory. No spec says a whole image of maximum size is built in one pass. Verify cannot invent that requirement, so this goes to the spec as W4.
- The source and kept levels alone come to about 2.4 GiB at 16 384 px, over CAL-26 with no working memory at all. Maximum-size images need a decision on tiles or limits whatever the builder does, so this is not only a code fix.

## WARNING

### W4. No memory bound for `build_mips`, and the maximum image size cannot be built

- class: `ambiguity`, follow-up
- ref: ARQ-121 (02:951), CAL-26, CAL-22, 04 FD import limit, `crates/gafira-raster/src/mip.rs` `Sums`
- Evidence: "Ruling on memory". Peak 551 MiB at 12 MP and 12.3 GiB at 16 384 px, against budgets that never name the builder.
- Settle before the first caller of `build_mips` (`gafira-render` or the engine). The spec should state a bound or a maximum for one build, and whether maximum-size images use tiles or a reduced base, with a §6.7 row and benchmark if a number is set.
- Part of the peak is avoidable with no change to the output bytes. Building level 1 straight from the view, without storing `[f64; 4]` for level 0, should cut the added peak at 12 MP from about 500 to about 160 MiB. `Alpha8` could also hold one channel, not four. These are estimates, not measured, and they do not make 16 384 px fit. The trigger the implementer noted ("if a speed or memory test needs less") is the right one, and no such test exists yet.
- Time: 0.5 s at 12 MP on this machine equals the whole 04 budget for showing a 12 MP JPEG (< 500 ms, PROPUESTA) if the build ever ran inline. §6.1 refines in the background, so nothing is broken today.

### W5. ARQ-110 pixel property runs on at most one pixel

- class: `implementation`, follow-up
- ref: `crates/gafira-raster/src/pixel.rs:302`, and `:327` in the same way
- `pixels()` draws 0 to 7 values and cuts to a multiple of 4, so each case has 0 or 1 pixel, and 3 of 8 cases are empty. The range `±1e6` almost never gives an overflow, so the `Err` arm is hardly reached. C5 stays closed: the property is there, each pixel is independent, and `arq_110_overflowing_result_is_err_and_buffer_unchanged` covers the overflow. A longer vector and a mix of tiny alpha values would close it. New test gap after round 1, so not FAIL.

## SUGGESTION

- S5. Before RS3, the coordinator adds the `EffectError` duty of 02:1308 to the card with a `slice card amend`, so RS3's verify checks it.
- S6. `mip.rs:536`: the comment says "even in a dropped column"; no column is dropped now.
- S7. `crates/gafira-raster/proptest-regressions/pixel.txt` is untracked in the worktree, from a mutant run. It is not in `888f54b`, its seed passes on the code, and verify may not delete it. Remove it, and leave it out of the merge.

## Criteria

25 of 25 covered: the 19 of round 1, with 9, 11, 13, 14 and 16 no longer weak, missing or failing, plus the 6 new odd-side and 1x1 rows of ARQ-121. Product IDs ARQ-117, ARQ-121, ARQ-110 and ARQ-111 all have named tests. None was left out of RS1.

## Size

1 696 lines added in `crates/` since `9124700`, 430 of them in this fix round. The forecast was 450. The owner said size does not concern him, so this is noted and is not a finding.

next: tdd-apply (RS2). Open follow-ups: W4 (ambiguity), W5 (implementation).
