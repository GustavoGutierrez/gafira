# Verify report: gafira-raster, RS3a, round 1

- Commit: `dc6fced` (branch `feat/gafira-raster`, base `8c0ff09`). Code commits: `54d8e7b`, `8059490`, `09847aa`.
- Depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS2 round 2, which stays in git at `1909734`. RS2 round 1 stays at `0b563fe`, RS1 round 2 at `e5e1d0e`. All rounds are also in `verification.json`.

**Verdict: FAIL for RS3a.** 3 CRITICAL, 5 WARNING, 5 SUGGESTION.
Per class: implementation 7 (3 CRITICAL, 4 WARNING), ambiguity 1 (1 WARNING), architecture 0, claim 0.

The contract, the ends, the error table, Cut, Dissolve and Fade hold on every in-domain probe. The failures are one real defect in Fade (`inf` and NaN in `dst` with no `Err`), and two sets of surviving hand mutants: the P15 check order, and the Fade boundary, which the record calls equivalent and is not.

## Gates

Each exit code was read alone, on a clean clone of `dc6fced`, with no `proptest-regressions` folder.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1 413 passed, 0 failed, 0 ignored; 95 in `gafira-raster`) | 0 |
| `cargo deny check` | 0 |
| policy: ARQ-07 CLI | 0 |
| policy: forbid unsafe | 0 |
| policy: V5 layers | 0 |
| policy: V12 clippy copies | 0 |
| policy: ARQ-107 language | 0 |
| policy: V13 EditToken | 0 |
| policy: V14 property tests | 0 |

The 14 seeds in the untracked `proptest-regressions/` also pass on the unmutated code, on a scratch copy. The CI test job's Windows and macOS targets were not run here. Disk: 36 GB free at the start, 32 GB at the lowest reading, 36 GB after `cargo clean` of all five build folders.

## Artifact integrity (step 3c)

- The card is `b64090c`, amended by `75e360d`. `dc6fced` only adds lines to `apply-progress.md` (54 added, none removed). No RS3a commit changed the card.
- `54d8e7b`, `8059490` and `09847aa` touch only `crates/gafira-raster/src/`. No implementer commit touched `specs/02-arquitectura-software.md`, `specs/changes/layer-2/` or `.cargo/`.
- The card's Deliveries line still says `RS3`; the split is in P15 (S2).

## Criteria

| Criterion | Status | Proof |
|---|---|---|
| ARQ-118 `progress` clip, ends exact, both formats | covered | `arq_118_ends_are_exact_copies_for_every_transition`; own probe with ghost colour, subnormals, `-0.0`, `f32::MIN`, `f32::MAX` |
| ARQ-25 (ends) | covered | same test, which names it. `extension-demo` is out of the cut (02 §4.17.4) |
| ARQ-118 `progress` NaN and `±∞`, named, before writing | covered | `arq_110_non_finite_progress_is_err_and_writes_nothing` |
| ARQ-118 sizes, formats, `Alpha8`, size 0, before writing | covered | `arq_118_sizes_*`, `arq_118_formats_*`, `arq_118_height_alone_*`, `arq_118_size_zero_*` |
| ARQ-118 Cut | covered | `arq_118_cut_switches_at_exactly_one_half` |
| ARQ-118 Dissolve | covered | the 0.25 and 8-bit tests; own probe, 200 000 random pixels against the formula at 1e-5 |
| ARQ-118 Fade, 0.5 exact, halves | **weak** at the boundary | `arq_118_fade_*`; own probe agrees with the code; C3 |
| ARQ-118 near the ends, ≤ 1/255 | covered | one pixel pair in the test; own probe, 3 000 000 random premultiplied pairs at `p` 0.0001 and 0.9999: worst 1/255 for both |
| ARQ-110 pixels, parameters, unknown ids | covered | `arq_110_*` |
| ARQ-110 no non-finite in the output, computed non-finite is `Err` | **failing** | C1 |
| ARQ-110 `cx.scale` | **open** | W4 |
| ARQ-111 determinism | covered | `arq_111_transitions_are_deterministic` (no `rayon` in the crate); own probe also varies the old `dst` |
| 02 §4.13 identity, schema, handles, trait defaults | covered | `*_has_the_identity_and_schema_of_the_table`, `transition_defaults_*` |
| 02 §7.3, 02:1308, ADR 0012 | covered | `effect_error_*`; exhaustive `match`, keys, `Count` and `Text` as stated. No `Cancelled` variant: ARQ-112 leaves `Transition` out, correct |
| P15 order, 7 steps | **weak** | the code follows it; three swaps survive, C2 |
| P15 shape, items 4 to 9, `Transfer` move | covered | `blend.rs` untouched; `mip.rs` tests pass |

12 of 16 fully covered. Left out on purpose: Slide, Zoom, Wipe (`RS3b`); GPU variants; `TransitionRegistry`; `extension-demo`.

## Independent probes (step 1c)

13 probes on a scratch export, through the public API, written before the tests were read. 12 pass on unmutated code; the failing one is C1. After reading the tests I added 5 more, one per surviving order or boundary mutant, all passing on unmutated code, and one in-crate probe of the `Encoded` arm (W3).

| Probe | Result |
|---|---|
| Ends and clip, bit for bit, awkward `f32` | pass |
| P15 order with several faults at once, `dst` untouched | pass |
| Bad `color`, `WrongType`, unknown ids with NaN and `∞` | pass |
| Formula checks, Fade colour exact, default black, translucent bytes | pass |
| Validity `c ≤ a`, self-mix within 1/255, near the ends | pass |
| `f32::MAX`, subnormal and mixed-sign pixels stay finite | pass |
| Fade with a huge finite `color` | **fail**, C1 |
| Error messages, descriptors, determinism | pass |
| `EffectCx { scale: NaN }` | `Ok` on all three, W4 |

XP13, timed on a scratch export in `--release`: Dissolve, `Rgba8`, 1080p, `p` 0.37, five runs: 154 to 156 ms per frame. The 8 ms share of 02 §6.2 is missed by a factor of about 19. See W1.

## Hand mutants (step 4b)

Spot-check of the recorded list: item 3, Cut `<=` at 0.5: killed by `arq_118_cut_switches_at_exactly_one_half`.

| # | Mutant | `gafira-raster` tests | Own probe | Result |
|---|---|---|---|---|
| I1 | `progress` checked after `resolve` (P15 steps 1, 2) | pass | `kill_i1` fails | **survives**, C2 |
| I2 | `resolve` after dimensions (P15 steps 2 to 4) | pass | `kill_i2` fails | **survives**, C2 |
| I3 | Pixels checked before dimensions (P15 steps 4, 5) | pass | `kill_i3` fails | **survives**, C2 |
| I3b | Dimensions before formats | `arq_118_height_alone_*` fails | | killed |
| I4 | Fade threshold 0.4999 | pass | `kill_i4` fails | **survives**, C3 |
| I4b | Fade threshold 0.5001 | pass | `kill_i4` fails | **survives**, C3 |
| I4c | Fade `p <= 0.5` | pass | pass | equivalent, confirmed |
| I5 | `colour_f32` alpha clamp to 2 | pass | `kill_i5` fails | **survives**, W2 |
| I5b | `colour_f32` no lower clip | pass | `kill_i5` fails | **survives**, W2 |
| I6 | Linear arm drops `* a` | `equal_samples_give_that_sample` fails | | killed |
| I7 | Guard's `p == 0` and `p == 1` branches removed | pass | | equivalent for these three, confirmed by reading |

Run 12 (spot-check plus 11), killed 3, equivalent 2, survivors 7. The budget of 2 to 3 was exceeded on purpose: the owner asked for each claimed equivalent to be examined.

## Claimed equivalents

| Claim | Finding |
|---|---|
| Fade `p <= 0.5` | True. Both give the colour at 0.5 |
| Fade threshold 0.4999 | **False.** Continuity at 0.5 does not cover `[0.4999, 0.5)`: there `t` is negative and the mix extrapolates. At `p` 0.49995, `from` 1, black colour, `to` 0: 1e-4 expected, and the mutant gives 0. A 1e-5 test sees it. C3 |
| Fade colour alpha clamp to 2 | True for bytes (`as u8` saturates). **False for `f32`**: `colour_f32` gives alpha 2. W2 |
| Final byte clamp | True: `as u8` saturates |
| Guard's `p == 0` and `p == 1` for these three | True: weights `(1, 0)` and `(0, 1)` return the pixel exactly |

## RED order

Git holds only the final commits, so the order itself cannot be read from it. I replayed the stubs each commit body states, on an export of that commit.

| Commit | Body claims | Replay | Reading |
|---|---|---|---|
| `54d8e7b` | 3 failing | 2 failing, with the stated `left` and `right` values | `effect_error_displays_for_the_log` passes: it uses neither stub, so it needs a `Display` stub the body does not state |
| `8059490` | 9 failing; height-alone Not RED | 10 failing | The tenth is height-alone: it fails on any stub returning `Ok`, so it was not in the RED run and came after the code. `arq_111_*` is among the nine, while `apply-progress.md` lists determinism as Not RED |
| `09847aa` | 12 failing | 11 failing | `dissolve_has_the_identity_and_schema_of_the_table` passes: it needs a stub the body does not state |

The order stubs, tests, RED, code is **not contradicted**: 22 of the 24 recorded failures reproduce, and they fail for the stated reasons. It is **not proved** for three tests: one came after the code, as its own note implies, against the "tests first" summary, and two need an unstated stub. W5.

## Deviations

| # | Finding |
|---|---|
| 1, two typed functions | Accepted. Same one place, same weight-1 rule, a slice fits Zoom's 8 samples. P15 can keep its generic wording |
| 2, colour alpha clipped | Accepted: an out-of-range alpha is finite and has no row, so 02 §4.17 leaves it unspecified. "In both formats" is proved only for bytes, W2. Wide gamut as stated |
| 3, `debug_assert!` over `MixSpace::ALL` | Works, but it is an always-true assertion that exists to satisfy `dead_code`, close to the spirit of "no `#[allow]` to silence lints". S1 |
| 4, XP13 not done | **"No bench was asked for" is wrong**: `design.md` XP13 says "in a bench of `RS3a`". W1 |
| 5, 6 | Accepted. Seeds: S3 |

## Findings

### CRITICAL

- **C1. Fade writes `inf` and NaN to `dst` with `Ok`.** class: implementation. `crates/gafira-raster/src/transitions/fade.rs:125`. `colour_f32` overflows on a huge finite colour, and nothing checks the result. Observed, `RgbaF32`: `Srgb(1e30, 0, 0, 1)` at `p` 0.25 gives `[inf, 0.05, 0.05, 0.55]`; `Srgb(1e30, 0, 0, 0)` at 0.5 gives `[NaN, 0, 0, 0]` (`inf × 0`); `DisplayP3(MAX, −MAX, MAX, 1)` gives `±inf`. The same colours give `Ok` on `Rgba8`, with clamped bytes. ARQ-110: a computed non-finite gives `Err`, no non-finite in the output; 02 §4.17 "Rangos": it holds for a value outside any range. Needs a check before any write, naming `color`, and a test. No test reaches this: the finite property sits on `weighted_f32`, not on the transition.
- **C2. The P15 check order is not proved.** class: implementation. `crates/gafira-raster/src/transitions/guard.rs:30`, `:38`, `:68`. I1, I2 and I3 survive. The code is right today. No test combines a bad `progress` or bad `color` with a later fault, or a size fault with a non-finite pixel, so steps 1, 2 and 5 can move unnoticed. Validation is critical code.
- **C3. Fade's `p < 0.5` boundary is not proved, and the equivalent claim is false.** class: implementation. `crates/gafira-raster/src/transitions/fade.rs:82`. I4 and I4b survive and are observable through the public API. Add `p` values within 1e-4 of one half on each side, with `from` and `to` different from the colour. ARQ-118 fixes the rule as `2p` if `p < 0,5`.

### WARNING

- **W1. XP13 is skipped, and the measurement goes against it.** class: implementation. `specs/changes/layer-2/design.md` XP13. The row asks for a bench in `RS3a`; none exists, and the deviation's reason is wrong. My timing is 154 ms per 1080p frame, against 8 ms. XP13 blocks nothing, but its claim does not hold as built, and the row says a table may be needed. It belongs in front of the owner with O5. This is no finding on `MIX_SPACE`, which is provisional by decision.
- **W2. `f32` colour alpha clip is untested.** class: implementation. `crates/gafira-raster/src/transitions/fade.rs:127`, test at `mod.rs:923`. I5 and I5b survive. Deviation 2 says both formats; the test holds bytes only. WARNING and not CRITICAL because no spec rule fixes an alpha outside `[0, 1]`.
- **W3. The dormant `Encoded` arm can give `inf`.** class: implementation. `crates/gafira-raster/src/transitions/mix.rs:314`. Finite pixels `[1e30, 0, 0, 1e-40]` and `[1.5e38, 0, 0, 1e-3]`, mixed at 0.5, give `inf` under `MixSpace::Encoded`, from `px / a`; the property excludes that arm. It does not run today, so this is no defect of the build, but P15 says answer (b) changes only the constant and the midpoint rows, and it would also need this check.
- **W4. `cx.scale` NaN is accepted, and the specs disagree.** class: ambiguity. ARQ-110 says every public function checks its numbers, and its table test covers "cada parámetro numérico"; P15's closed order has no such step, and the transitions never read `scale`. All three return `Ok` on NaN. Either ARQ-110 excludes `EffectCx`, or the guard gains a step. `RS4` meets the same question with `Effect::run_cpu`.
- **W5. The RED record does not match the replay.** class: implementation. Commit bodies of `54d8e7b`, `8059490`, `09847aa`, and `apply-progress.md` line 200. See "RED order". State the stubs in full, and say which tests came after the code.

### SUGGESTION

- S1. Replace the `debug_assert!` of deviation 3 with something that does not exist only for a lint, or say so beside it.
- S2. Amend the Slice card for the split: its Deliveries line and IDs still say `RS3`.
- S3. `proptest-regressions/` is untracked for the third delivery, now with 14 seeds, and 9 such paths are tracked in other crates. Decide once: commit them or ignore them. The two new seeds pass, and no deterministic test pins their case.
- S4. `from` `Rgba8` with `to` `Alpha8` gives `FormatMismatch`, and a test pins it. ARQ-118 and P15 do not rank the two rules. Settle it when W4 goes to `specifier`.
- S5. Comments cite `02:1308`; the rule now sits at line 1349 of `specs/02-arquitectura-software.md`. A section reference (§7.3) would not move.

## Size

1 992 lines added in `crates/gafira-raster`, about 1 100 of them tests, against a forecast of 420 (590). Noted, no finding, as the coordinator said.

next: specifier (ambiguity: W4), then tdd-apply (implementation: C1, C2, C3, W1, W2, W3, W5)
