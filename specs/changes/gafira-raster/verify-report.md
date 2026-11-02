# Verify report: gafira-raster, RS2, round 2

- Commit: `068f17b` (branch `feat/gafira-raster`). Fix commits: `07dd1e2` and `1781250`. Closure round: gates, artifact integrity and the earlier findings only.
- Depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS2 round 1, which stays in git at `0b563fe`, with its probe tables and the W1 measurements. RS1 round 2 stays at `e5e1d0e`. All rounds are also in `verification.json`.

**Verdict: PASS for RS2.** 0 CRITICAL, 3 WARNING (all follow-ups), 4 SUGGESTION. RS3 to RS6 remain.
Per class: implementation 2 (2 WARNING), ambiguity 1 (1 WARNING), architecture 0, claim 0. No class routes this round; all three are follow-ups.

C1 to C4 and W2 to W5 are closed. W1 stays open as an owner question. The fix round left one test gap, W6: no test holds the 8-bit case of both pixels transparent.

## Gates

Each exit code was read alone, on a clean clone of `068f17b`, with no `proptest-regressions` folder.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1 381 passed, 0 failed, 0 ignored; 63 in `gafira-raster`) | 0 |
| `cargo deny check` | 0 |
| policy: ARQ-07 CLI | 0 |
| policy: forbid unsafe | 0 |
| policy: V5 layers | 0 |
| policy: V12 clippy copies | 0 |
| policy: ARQ-107 language | 0 |
| policy: V13 EditToken | 0 |
| policy: V14 property tests | 0 |

The 12 seeds in the untracked `proptest-regressions/` (11 in `blend.txt`, 1 in `pixel.txt`) also pass on the unmutated code, on a scratch copy. The CI test job's Windows and macOS targets were not run here.

Disk: 26 GB free at the start. It fell to 17 GB after the gate build, with other work on the same disk, so the gate target was freed at once. It then read 20G, and the mutant run, already under way and building one crate, went on. Free space was 36 GB at the end. No gate result depends on this.

## Artifact integrity (step 3c)

- `b64090c` (slice card) and `75e360d` (slice card amend) are as checked in round 1.
- `07dd1e2` touches only `blend.rs`. `1781250` touches only `blend.rs` and `error.rs`. `068f17b` only adds 15 lines to `apply-progress.md`. No implementer commit changed the card.
- The ARQ-119 amendment is `125d2ab`, merged through `9dc2ed1` and `c1d8504`. No implementer commit touched `specs/02-arquitectura-software.md`.
- No `.cargo/mutants.toml` change.
- As in round 1, step 3c binds the fast lane and this flow is `full`. It was applied because the card is the acceptance source.

## Closure of earlier findings

Mutants were reapplied one at a time on a scratch clone, with the same patterns as round 1 where they exist. No independent-mutant budget was spent: W3, W2 and B are closure checks.

| Finding | Status | Mutant | Implementer's 63 tests |
|---|---|---|---|
| C1 identity difference 0 | **Closed** | M1 | Killed by `arq_119_transparent_backdrop_gives_source` |
| C2 source cap | **Closed** | M2a, M2b | Both killed by `arq_119_source_colour_is_capped_to_its_alpha` |
| C3 alpha result check | **Closed** | M3 | Killed by `arq_110_overflowing_alpha_is_err` |
| C4 ColorBurn guard | **Closed** | Guard dropped | Killed by `arq_110_color_burn_with_negative_zero_source`; claim (b) is withdrawn in `apply-progress.md` |
| W2 transparent source against clip | **Closed** | Early return removed, `f32` and 8 bits | Killed by `arq_119_transparent_source_leaves_the_backdrop_bit_for_bit` |
| | | Early return before the finite check | Killed by `arq_119_transparent_source_still_checks_for_non_finite` and `arq_110_non_finite_input_is_err_for_every_mode` |
| W3 mixing result clamped | **Closed** | Clamp removed | Killed by `arq_119_mixing_result_is_clamped` |
| W4 property only at corners | **Closed** | None | New interior draws reach [-2, 2] and tiny values. Their [0, 1] assertion has an edge: W7 |
| W5 no test names FD-12 | **Closed** | None | `git grep FD-12` finds three comments in `blend.rs` |

S2 is also closed: the `index` doc in `error.rs` now names the channel.

Closure probes on the amended text, separate from the implementer's tests, for all 16 modes: any source colour with `a = 0` or `-0.0` keeps the backdrop bits, with backdrop `(1,5; 0,5; 0,2; 1)`, `-0.0`, `f32::MAX` and negative alpha. NaN and both infinities in that source or backdrop give `Err`. All pass.

The round 1 probes were re-run to catch regressions from the clamp and the early return. The reference tables (32 000 `f32` and 32 000 8-bit triples) and the rest pass, with one expected exception below.

## Ruling: both pixels transparent returns the backdrop

**Accepted. The code is right; the deviation is derivable.**

- The text does not state a precedence. The criteria say both "fuente con alfa 0 [...] deja el fondo igual" and "fondo con alfa 0 da la fuente".
- On valid premultiplied input they do not conflict: `a = 0` means colour 0 (ARQ-117), so both give zero. Only ghost colour, or the sign of a zero, tells them apart.
- With ghost colour the source rule must win. It says "sea cual sea su color", so the result may not depend on the source colour, and returning the source would make it depend on exactly that. It is also the rule in the requirement column, stated "en todo modo" and "bit a bit". W3C agrees: zero alpha means the colour does not contribute.
- The cost is that a ghost backdrop stays ghost: `blend_u8(m, [9,7,5,0], [0,0,0,0])` gives `[9,7,5,0]`, not transparent black. That is what "bit a bit" asks for, and the input was already invalid. It narrows round 1's "`a = 0` as transparent black", and the `blend_f32` doc says so.
- Evidence: 16 modes, `f32` and 8 bits, ghost colour on both sides, signed zeros: the backdrop every time (probe r3). In `f32` the rule is pinned: B1 and B2 are killed. In 8 bits it is not: W6.
- Round 1's probe `p4_identities_u8_exhaustive` now fails on exactly this case, 16 modes by one input. Its expectation was written before the amendment; it is stale, not a regression. Its other cases pass.

## CRITICAL

None.

## WARNING

### W1. No stated colour space for blending in Preview

- class: `ambiguity`, `follow-up`
- ref: ARQ-119, 02 §6.2, RS1 `Rgba8` rule, W3C section 10.2
- Open, unchanged, owner question: `blend_u8` blends encoded bytes and `blend_f32` linear values, with grey gaps up to 232/255. Measurements are in the round 1 report (`0b563fe`). It does not block RS2. It must be ruled before `gafira-render-cpu` uses these functions.

### W6. The 8-bit case of both pixels transparent has no test

- class: `implementation`, `follow-up`
- ref: `crates/gafira-raster/src/blend.rs:201`, ARQ-119
- B3 (8 bits, both transparent falls through to transparent black) and B4 (gives the source) pass all 63 tests; probe r3 kills them. The 8-bit transparent-source tests use backdrop alpha 100, and the property uses valid pixels only.
- The code is right today. It only shows with ghost colour in an 8-bit backdrop. New test gap after round 1, so not FAIL.
- Fix: add a backdrop with `a = 0` and ghost colour to the 8-bit cases of `arq_119_transparent_source_leaves_the_backdrop_bit_for_bit`.

### W7. Two properties assert [0, 1], which the amended rule makes false at source alpha 0

- class: `implementation`, `follow-up`
- ref: `crates/gafira-raster/src/blend.rs:1001`, `:1015`
- `arq_110_finite_input_gives_finite_output` and `arq_110_interior_input_gives_output_in_unit_range` expect output in [0, 1] for non-`Normal` modes. Their draws can include a source alpha of exactly 0 with an out-of-range backdrop, and then the right answer is outside that range: `blend_f32(Multiply, [2,0,0,1], [0;4])` gives `[2,0,0,1]`.
- Not seen in practice: the interior property passed 20 million cases, and 256 run per test. The odds are too low to measure from that, so this is a possible rare false failure, not an observed one.
- Fix: `prop_assume!` a non-zero source alpha, or exclude it from the [0, 1] assertion.

## SUGGESTION

- S1. As in round 1: a per-pixel call has no destination, so "`Err` antes de escribir en destino" moves to the loop in `gafira-render-cpu`, which should check the whole buffer first.
- S3. `crates/gafira-raster/proptest-regressions/` is still untracked, now with 11 seeds in `blend.txt`, and `.gitignore` does not cover it. Verify may not delete it. Remove it and leave it out of the merge.
- S4. Add the ruling to ARQ-119 in one clause, for example "con los dos en alfa 0, gana la fuente: queda el fondo". This is a clarification of a derivable reading, not a gap that blocks.
- S5. A source with negative alpha is not "alfa 0", so a non-`Normal` mode still clips: Multiply with alpha -0,5 over `(1,5; 0,5; 0,2; 1)` gives `(1,0; 0,5; 0,2; 1)`, and so does a source of alpha 1e-45. Both follow the amended text, but W2's effect remains for an out-of-range source. Worth a thought when `gafira-render-cpu` defines its inputs.

## Criteria

16 criteria: **16 covered, 0 weak, 0 failing, 0 missing.** The amendment adds the `Err` criterion.

| Criterion | State | Proof |
|---|---|---|
| 16 modes per W3C | Covered | Tables, spot values |
| Premultiplied source-over | Covered | Tables, identities |
| Clip before non-`Normal`, not `Normal`, except source `a = 0` | Covered | `arq_119_clip_applies_only_before_other_modes`, `arq_119_source_colour_is_capped_to_its_alpha`, bit-for-bit test |
| At least 256 triples, `f32` <= 1e-5 | Covered | `arq_119_every_mode_matches_the_reference_in_f32` |
| 8 bits <= 1/255 | Covered | `arq_119_every_mode_matches_the_reference_in_8_bits` |
| Source `a = 0` leaves the backdrop bit for bit, `(1,5; 0,5; 0,2; 1)`, `-0.0` | Covered | `arq_119_transparent_source_leaves_the_backdrop_bit_for_bit` |
| NaN in that source or backdrop gives `Err` | Covered | `arq_119_transparent_source_still_checks_for_non_finite` |
| Backdrop `a = 0` gives source | Covered | `arq_119_transparent_backdrop_gives_source` |
| `Normal` at alpha 1 gives source | Covered | `arq_119_normal_opaque_source_is_the_source` |
| Identities, difference 0 | Covered | Bit comparisons in the tests above |
| ARQ-110 inputs | Covered | `arq_110_non_finite_input_is_err_for_every_mode` |
| ARQ-110 result | Covered | `arq_110_overflowing_normal_result_is_err`, `arq_110_overflowing_alpha_is_err`, `arq_110_color_burn_with_negative_zero_source` |
| ARQ-110 property | Covered (W7) | `arq_110_finite_input_gives_finite_output`, `arq_110_interior_input_gives_output_in_unit_range` |
| ARQ-111 | Covered | `arq_111_blend_is_deterministic` |
| ADR 0012 | Covered | No new `BufferError` variant |
| FD-12 (c, CPU) | Covered | Named on the ARQ-119 table tests |

Left out on purpose: "`gafira-render-cpu` las usa, no las redefine", since that crate does not exist yet, and the GPU half of FD-12 (c), which is not in this delivery.

Product IDs: ARQ-119 and FD-12 (c, CPU), both named by tests. ARQ-110 and ARQ-111 tests are extended here.

## Size

226 lines added in `crates/gafira-raster/` this round, for about 1 068 in the delivery, against a forecast of 340 (475). Most are tests. Noted, not a finding.

Open from RS1, unchanged: W4 (ambiguity, `build_mips` memory) and W5 (implementation, pixel property), both follow-ups.

next: tdd-apply (RS3)
