# Verify report: gafira-raster, RS4, round 2

- Commit: `6d1f1a6` (branch `feat/gafira-raster`), base `develop`. Fix commits: `d499754`, `cd16756`, `4a5f7f8`; notes `6d1f1a6`; spec amend `249789a`.
- Closure round, depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS4 round 1, which stays in git at `271f7c6`.
- Scope, set by the coordinator: C1, C2, C3, W1 to W5, S1, S3 of round 1, the text of ARQ-120 and FD-35 in `249789a`, and the four commits. S2 was not re-checked.
- Decisions taken as given: below radius × scale of about 1e-21 the response is the identity, not a finding. `Rgba8` σ at small radii is measured in `RgbaF32`.
- Probes ran on a scratch export of `6d1f1a6`, through the public API. Neither the export nor its target dir touched the worktree. The build folder is cleaned.

**Verdict: PASS for RS4.** 0 CRITICAL, 2 WARNING (both follow-up), 2 SUGGESTION. All 10 scoped findings are closed.
Per class: implementation 1 (1 WARNING), ambiguity 2 (1 WARNING, 1 SUGGESTION), architecture 0, claim 1 (1 SUGGESTION).

One result needs the owner's eye: W6. On translucent premultiplied `Rgba8` images with a tinted shadow, the half-size criterion reaches 1.5/255 on a few values. It is rated WARNING, not CRITICAL. The reasons are under W6, and the owner can overrule them.

## Gates

Each run alone, exit code read directly, `CARGO_TARGET_DIR` not passed, at `6d1f1a6`.

| Gate | Exit | Note |
|---|---|---|
| `cargo fmt --check` | 0 | |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | |
| `cargo test --workspace --locked` | 0 | 1 491 passed, 0 failed, 0 ignored (round 1 had 1 484, plus the 7 new tests) |
| `cargo deny check` | 0 | advisories, bans, licenses and sources ok |
| CI policy (7 steps of the `policy` job besides Format, read from `ci.yml` with a YAML parser) | 0 | ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13, V14 |

`git status` at the end: only `crates/gafira-raster/proptest-regressions/` untracked (known pending, deviation 9).

## Artifact integrity

- Slice card unchanged: `271f7c6..HEAD` adds 35 lines to `apply-progress.md` and removes none.
- `249789a` touches only `specs/02-arquitectura-software.md` and `specs/04-funcionalidad-diseno-imagen-exportacion.md`, one row each, and precedes the fixes. It is the only commit since `07ba7d3` that touches `specs/` outside the change folder.
- `6d1f1a6` touches only `apply-progress.md`. `d499754` touches only the two bench headers.

## Closure

| Finding | Status | Evidence |
|---|---|---|
| C1 σ at small radius | **Closed** | Probe below: worst error 1.254 % over 3 scales and both axes |
| C2 underflow | **Closed** | Blur `5e-324`, `1e-200`, `1e-170`, `1e-162`, `2e-162`, `3e-162`, and `1e-160` at scale `1e-30`: `Ok` in both formats, `RgbaF32` bit-equal to blur 0, `Rgba8` equal to blur 0 |
| C3 composite overflow | **Closed** | Survivor M1 re-applied: killed by `arq_110_composite_overflow_is_err_and_dst_is_untouched` |
| W1 sub-pixel offsets | **Closed** | `249789a`: whole pixels, half away from zero, no sub-pixel offset; the criterion names the 0.5, −0.5, −1.5 and scale 0.5 cases; odd offsets are out of the half-size bound. Code and test match |
| W2 half-size domain | **Closed** as a spec gap | `249789a` names a smooth image, radius ≥ 16, over 3σ from the edge. Test rewritten. A new residue is in W6 |
| W3 offset rounding | **Closed** | Survivor M2 re-applied: killed by `fd_35_offsets_round_half_away_from_zero` |
| W4 opacity ties | **Closed** | Re-ran the round 1 probe: 0 of 51 456 mismatches (round 1 had 20) |
| W5 test-first | **Closed** | RED table present in `apply-progress.md`, with honest entries for tests that were green at the start (C3, W3, W1 and W2). Replay below. Git cannot show the order, because `cd16756` holds tests and code together |
| S1 cap 4800 | **Closed** | `MAX_HALF_WIDTH` is 4800; σ −1.343 % at 3 200. The comment cites the wrong ID: S4 |
| S3 nightly claim | **Closed** | Both raster bench headers reworded; no `nightly` under `crates/gafira-raster` |

**σ probe (C1, S1).** `RgbaF32`, second moment of an impulse.

| Radius × scale | Error |
|---|---|
| About 40 grid points from 0.02 to 40, also 1.16, 1.19, 1.1999, 1.2001, each at scale 1, 0.5 and 2, along x and y | Worst 1.254 % |
| 0.6, 1, 1.1999 (three-tap branch) | 0.000 % |
| 1.2 (Gaussian branch) | −1.176 % |
| 100, 400, 1 600 | −1.306 %, −1.334 %, −1.341 % |
| 2 880, 3 000, 3 200 | −1.340 %, −1.341 %, −1.343 % |

Round 1 measured −7.26 % at 1, −89.6 % at 0.5 and −3.96 % at 3 200.

**Other probes.**

| Probe | Result |
|---|---|
| Tiles with `roi` at small radii: blur 0.25 to 2.4, scales 1, 0.5 and 2, 3 offset pairs, 3 tiles, both formats | Max diff 0 |
| Constant opaque, centre pixel, blur 0.5, 1, 1.19, 2 | 255 and 1.0 |
| Half-size, `RgbaF32`, tinted, translucent, blur 16 to 100, even offsets | At most 0.02/255 |
| Half-size, `Rgba8`, alpha only or opaque image | At most 1.00/255 |
| Half-size, `Rgba8`, translucent premultiplied, tinted shadow | Up to 1.5/255, see W6 |

**Mutants.** No independent budget on a closure round. Only the two round 1 survivors were re-applied, and both die.

| Mutant | Killed by |
|---|---|
| M1 `RgbaF32` composite written straight into `dst` | `arq_110_composite_overflow_is_err_and_dst_is_untouched` |
| M2 offset `floor(v + 0.5)` | `fd_35_offsets_round_half_away_from_zero` |

**RED replay (W5).** With the four fixes reverted on the export, exactly the five tests the record lists as RED fail: `arq_120_sigma_holds_for_small_radius_times_scale`, `arq_110_a_radius_whose_square_underflows_is_the_identity`, `arq_110_a_blur_too_small_to_square_is_the_same_as_blur_zero`, `arq_120_the_cap_covers_blur_100_at_zoom_32`, `fd_35_a_opacity_ties_round_up_exactly`.

**Rulings.** The change to `arq_120_radius_zero_is_the_identity` (radius `1e-12` now within 1e-6) is accepted: ARQ-120 makes only radius 0 the identity, and radius 0 stays bit-exact. Size was left aside by the owner in round 1; the fix round is 239 insertions and 42 deletions under `crates`, which matches the record.

## Findings

### WARNING

- **W6**, class: `ambiguity`, follow-up. ARQ-120, half-size criterion, `Rgba8`, as amended in `249789a`.
  - *Measured:* on a 1024×900 image with neighbour steps ≤ 1/255 per channel, translucent and premultiplied, blur 16 or 32, shadow colour (0.8, 0.3, 0.1), offsets (4, −6) and (−6, 8), over 3σ plus the offset from the edge, 26 to 45 channel values, over about 190 000 pixels, exceed 1/255 on red or green. The worst is 1.5/255. This happens when the half is made on its own: a rounded 2×2 mean (worst 1.5, 26 values at blur 16) or the same function sampled at half size (1.5, 45 values at blur 16). When the large image is the half repeated 2×2, the worst is 0.50/255. Alpha never exceeds 1.00, and alpha-only or opaque images stay within 1.00.
  - *Why not CRITICAL:* read literally, the bound is exceeded inside the named domain. But the same images reach at most 0.02/255 in `RgbaF32`, so the blur and scale logic are right. The excess is 8-bit rounding that adds up across the input, the shadow alpha, the premultiply and the composite. The criterion does not say how the half is made, in which format the bound applies, or whether it covers colour. On the one construction with no input rounding it holds.
  - *Fix:* the specifier states the format or the construction, for example "in `RgbaF32`, or in `Rgba8` with a tolerance of 2/255 for translucent images". No code change is expected. This is the one item for the owner to overrule.
- **W7**, class: `implementation`, follow-up. `shadow.rs`, `arq_120_a_half_size_image_at_half_scale_matches_after_rescaling`. New test gap: the test checks only alpha, with black shadow, in `Rgba8`. Its bump is steeper than the amended definition of smooth, so it is a harder case on alpha but does not exercise colour channels or `RgbaF32`. *Fix:* add a smooth tinted case in `RgbaF32` once W6 is settled.

### SUGGESTION

- **S4**, class: `ambiguity`. `blur.rs:4` and the comment at `blur.rs:374` cite FD-07 for zoom 32. FD-07 is the Cuadrícula; zoom is FD-08 (5 to 3200 %). The error came from round 1's own S1 text. The value 4800 is right. *Fix:* change the two citations to FD-08.
- **S5**, class: `claim`. Outside this scope, noted only: `crates/gafira-layout/benches/layout.rs:4` keeps the same nightly-job claim as S3.

S2 (round 1) was not re-checked and stays open as a suggestion.

## Criteria

| Criterion | Evidence | State |
|---|---|---|
| ARQ-120 impulse σ ±2 % | Probe, `arq_120_sigma_holds_*`, `arq_120_the_cap_*`, `arq_120_impulse_*` | Covered |
| ARQ-120 offsets round (scale 1 and 0.5) | `fd_35_offsets_round_half_away_from_zero` | Covered |
| ARQ-120 half image, amended | `arq_120_a_half_size_*`, probe | Covered; residue in W6, W7 |
| ARQ-120 tiles at scale 1, 0.5, 2 | `arq_120_a_tile_*`, probe at small radii | Covered, bit-exact |
| ARQ-120 radius 0 identity | `arq_120_radius_zero_is_the_identity` | Covered |
| FD-35 (a) | `fd_35_a_*`, probe | Covered |
| FD-35 offset scales and rounds | `fd_35_offsets_*` | Covered |
| ARQ-110 intermediate non-finite | `arq_110_a_blur_too_small_*`, `arq_110_a_radius_whose_square_*`, probe | Covered |
| ARQ-110 write nothing on composite overflow | `arq_110_composite_overflow_*`, M1 | Covered |

The other criteria of round 1 were not re-opened. Every ID of the Slice card still has tests naming it.

## Next

RS4 is done except for T4 at close, which stays owed. RS5 and RS6 remain, so the change is not done. W6 and W7 are follow-ups and do not hold the merge.

next: tdd-apply (next delivery, RS5)
