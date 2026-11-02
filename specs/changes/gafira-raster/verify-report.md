# Verify report: gafira-raster, RS2, round 1

- Commit: `2a2b517` (branch `feat/gafira-raster`, base `7c076d2`). The implementation is `738316d`. First verify of this code, so steps 1c and 4b ran in full.
- Depth V2, mode full, tier T4 at close, `mutation: archive`, so no `mutants` report (not a finding). T4 is still owed at close.
- This report replaces RS1 round 2, which stays in git at `e5e1d0e`.
- W3C text: *Compositing and Blending Level 1*, Candidate Recommendation Draft of 21 March 2024, fetched from the W3C site.

**Verdict: FAIL.** 4 CRITICAL, 5 WARNING, 3 SUGGESTION. RS3 to RS6 remain.
Per class: implementation 7 (4 CRITICAL, 3 WARNING), ambiguity 2 (2 WARNING), architecture 0, claim 0.

The code met every ARQ-119 and ARQ-110 criterion in the probes. All four CRITICAL items are test gaps: mutants that break a rule pass all 55 tests of the crate. One of them also shows a claimed equivalent mutant is not equivalent.

## Gates

Each exit code was read alone, on a clean clone of `2a2b517`, with no `proptest-regressions` folder.

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` (1 328 passed, 0 failed, 0 ignored) | 0 |
| `cargo deny check` | 0 |
| policy: ARQ-07 CLI | 0 |
| policy: forbid unsafe | 0 |
| policy: V5 layers | 0 |
| policy: V12 clippy copies | 0 |
| policy: ARQ-107 language | 0 |
| policy: V13 EditToken | 0 |
| policy: V14 property tests | 0 |

The ten seeds in the untracked `proptest-regressions/blend.txt` also pass on the unmutated code. The CI test job's Windows and macOS targets were not run here.

## Artifact integrity (step 3c)

- `b64090c` (slice card) touches only `apply-progress.md`. `75e360d` (slice card amend) changes one card line, states why and touches only that file.
- `738316d` touches only `blend.rs` and `lib.rs`. `2a2b517` only adds 54 lines to `apply-progress.md`. No implementer commit changed the card.
- Step 3c binds the fast lane and this flow is `full`. It was applied anyway, because the card is the acceptance source. See W5.
- No `.cargo/mutants.toml` change.

## Independent probes (step 1c)

The reference was written from the W3C text before the implementer's tests were read. It is a Python script with 60-digit `Decimal` values, in the two-step form of W3C section 10 (`Cr`, then source-over). The mixing result is clamped, as W3C requires. It shares no code or structure with `blend::tests::reference`.

Script and probes are in `/tmp/claude-1000/-home-meridian-Documentos-Proyectos-Personal-gafira/d1e76b26-3a3f-4305-b24f-7bfde6a07f61/scratchpad/verify-rs2/refgen/gen.py` and `/tmp/claude-1000/-home-meridian-Documentos-Proyectos-Personal-gafira/d1e76b26-3a3f-4305-b24f-7bfde6a07f61/scratchpad/verify-rs2/probe/crates/gafira-raster/tests/verify_probes.rs`.

| Probe | Result |
|---|---|
| 16 modes, 2 000 `f32` triples each (boundaries, greys, tied channels), 32 000 total | Max error 2.98e-8; limit 1e-5 |
| 16 modes, 2 000 8-bit triples each, 32 000 total | Max error 0.5/255; limit 1/255 |
| W3C section 5.1.1 worked examples | Exact |
| 8 bits: identities for all alphas, sampled colours, ghost colour in the `a = 0` pixel | Difference 0 |
| `f32`: backdrop `a = 0` gives the source, any `c <= a`, 500 000 pixels, all modes | Difference 0 |
| `f32`: source `a = 0` leaves the backdrop; `Normal` at alpha 1 gives the source | Difference 0 |
| Non-`Normal` result equals the call with clipped inputs; `Normal` is not clipped | Holds |
| ARQ-110: 3 values, 8 positions, 2 alphas, 16 modes; `field` and `index` | Holds |
| Any finite `f32` bit pattern, 1.5 M inputs, all modes | No non-finite output; `Err` only in `Normal` |
| `-0.0` against `0.0` in a source channel | Same value |
| Valid 8-bit input, 2 M pairs, all modes | `c <= a` always |
| ClipColor `0/0` from rounding: all `f32` greys on black and white, plus 40 M very dark greys | No `Err`. Not reached, not proved impossible |
| Valid `f32` input, 30 M calls in non-`Normal` modes | **76 856 channels negative, worst -4.4e-16 (W3)** |

The `0/0` probe exists because the `Decimal` reference hit that division on rounding residue. Its sweep flagged two cases at grey 1.0 on black; that was a wrong expected value in my probe, and the output was 1.1e-16.

## Hand mutants (step 4b)

The first pass replaced every match of its pattern, so S0 and Ec also changed the implementer's reference. Both were re-run on the code only; the table holds the corrected run.

| # | Mutant, rule | Implementer's 55 tests | My probes |
|---|---|---|---|
| S0 | Spot-check of the recorded list: Overlay arguments unswapped | **Killed**, recorded killers confirmed | Killed |
| M1 | Straight source colour held in `f32` (identities, difference 0, 02 table) | **Survives** | Killed by p20 |
| M2a | `f32` source not capped to alpha | **Survives** | Killed by p6, p8 |
| M2b | 8-bit source not capped to alpha | **Survives** | Killed by p21 |
| M3 | Result check skips alpha (ARQ-110) | **Survives** | Killed by p19, p8 |

### The three claimed equivalents

| Claim | Ruling |
|---|---|
| (a) ClipColor recomputing `x` or `L` | **Confirmed.** 120 M differential cases gave 0 bit differences, and both branches never ran together. The argument holds in rounding too: the first step only shrinks towards `L`, so `x` cannot pass 1 after it. |
| (b) Dropping ColorBurn's `Cs == 0` guard | **Refuted for `-0.0`.** See C4. It holds for `+0.0` only. |
| (c) `Normal` arm of `separable` | **Confirmed.** `composite` returns before `separable` for `Normal`. The arm changed to 0.25 passes all tests and probes. |

## Deviations

| Deviation | Class | Basis |
|---|---|---|
| Per-pixel signature | Derivable | ARQ-119 asks for the formulas and gives no signature; 02 §4.17 and the layer-2 design list no blend API. `gafira-render-cpu` "las usa", and a loop does that. See S1. |
| Colour capped to alpha | Derivable | ARQ-119 clips channels but does not state the cap. W3C 5.1 says all values are in [0, 1] and section 10 blends straight colours. Capping `c` to `a` is the same as clipping the straight colour to 1. The 8-bit cap follows from the same text; ARQ-119 only says `f32`. The source side is unproved (C2). |
| `a = 0` as transparent black | Derivable | W3C 5.1: "zero alpha meaning that the color does not contribute". 02 table: division by alpha 0 gives 0. It is needed for both ARQ-119 identities when the pixel holds ghost colour, `Normal` included. With ghost colour on both sides the two identities disagree, and this rule is the sensible way out. |
| 8-bit path with no sRGB linearisation | Derivable for the function; **ambiguity** for its use | ARQ-119's 8-bit criterion compares with the formula on the same values, so `blend_u8` is right as tested. D-27 only fixes the count of modes. How `gafira-render-cpu` must use it under 02 §6.2 is not stated: W1. |
| Non-finite check first | Stated | ARQ-110 ("antes de calcular"), 02:891 (all inputs, also those not used), 02 table. Holds, also for `a = 0`. Backdrop before source is unspecified and harmless. |
| Not `Normal`: everything clipped, also with source `a = 0` | **Ambiguity** | Not listed by the implementer: W2. |
| No clamp of the mixing result | **Stated, not followed** | Not listed by the implementer: W3. |

## CRITICAL

### C1. Identity "difference 0" is tested at 1e-6

- class: `implementation`
- ref: `crates/gafira-raster/src/blend.rs:576`, 02 table "Identidades, diferencia 0", ARQ-119
- `arq_119_transparent_backdrop_gives_source` allows 1e-6 although its comment says "exactly". The `f32` table allows 1e-5, and its sources are poor for this: a = 0.5 is exact.
- M1 changes results by one `f32` step, against the spec, and survives. The code holds today (p20). Critical: a spec rule with an invariant.
- Fix: assert equal values, with sources whose `c / a` is not exact in `f32`.

### C2. Source side of the alpha cap is unproved

- class: `implementation`
- ref: `crates/gafira-raster/src/blend.rs:153`, `:202`, ARQ-119
- `arq_119_clip_applies_only_before_other_modes` caps only the backdrop. The property test cannot reach it either (W4), because its source alpha clips to 0 or 1.
- M2a and M2b survive. M2a gives results outside [0, 1] and fails the finite-output probe, so this cap is what keeps the source in the domain. The code holds today (p6, p21).
- Fix: `f32` and 8-bit source cases with `c > a` in `0 < a < 1`.

### C3. Result check on alpha is unproved

- class: `implementation`
- ref: `crates/gafira-raster/src/blend.rs:182`, ARQ-110
- `arq_110_overflowing_normal_result_is_err` overflows channel 0 only.
- M3 survives, and with it `blend_f32(Normal, [0,0,0,MAX], [0,0,0,-1])` returns `Ok` with an infinite alpha. HEAD gives `Err` with field `result`, index 3 (p19). Critical: validation.
- Fix: a case where only alpha overflows.

### C4. ColorBurn's `Cs == 0` guard is not equivalent, and is untested

- class: `implementation`
- ref: `crates/gafira-raster/src/blend.rs:38`, `apply-progress.md` RS2 survivor (b), ARQ-110
- `-0.0` survives `clamp` and `min`, and `-0.0 / a` is `-0.0`. With no guard, `(1 - cb) / -0.0` is `-inf`, `min(1.0)` keeps it, and `1 - (-inf)` is `+inf`. A finite in-range input would then fail in ColorBurn.
- The mutant passes all 55 tests and fails p9. The guard is correct today; only the test and the claim are missing.
- Fix: a ColorBurn case with a `-0.0` source channel and `Cb < 1`, and correct note (b).

## WARNING

### W1. No stated colour space for blending in Preview

- class: `ambiguity`
- ref: ARQ-119, 02 §6.2, RS1 `Rgba8` rule, W3C section 10.2
- 02 §6.2 gives Preview as 8-bit sRGB premultiplied and Export as `f32` in linear light, and says the shared graph keeps them from diverging. RS1 treats `Rgba8` as sRGB-encoded when averaging. `blend_u8` works on the stored bytes, so in Preview it blends encoded values while `blend_f32` blends linear ones in Export.
- W3C names a "blending color space" and does not define it. D-27 says nothing on it.
- Measured on opaque grey pairs, every 5 levels, with the Export result encoded to sRGB:

| Mode | Worst gap (of 255) |
|---|---|
| ColorBurn | 232 |
| ColorDodge | 202.9 |
| Difference, Exclusion | 120 |
| Overlay, HardLight | 115 |
| SoftLight | 31.6 |
| Screen | 27 |
| Multiply | 7.8 (128 × 128: 64 against 60.95) |
| Normal, Darken, Lighten | 0 |
| The four non-separable modes | 0 on greys only; colours not measured |

- Decision needed before `gafira-render-cpu`: which space each quality blends in, and whether Preview must decode. `blend_u8` may need no change, but the owner should rule on it.

### W2. Transparent source against clipping an out-of-range backdrop

- class: `ambiguity`
- ref: ARQ-119, 02 table "Canal `f32` fuera de `[0, 1]`"
- "Fuente con alfa 0 deja el fondo igual" is stated per mode, and the 02 table admits out-of-range channels. "Se recortan" applies before any non-`Normal` mode.
- Measured: backdrop `(1.5, 0.5, 0.2, 1)` with a transparent source gives `(1.0, 0.5, 0.2, 1)` in Multiply and is unchanged in `Normal`. So a Multiply layer changes the backdrop where it paints nothing.
- The code follows the clip. The spec should say which rule wins. W3C's domain is [0, 1], so it does not settle this.

### W3. The mixing result is not clamped

- class: `implementation`
- ref: `crates/gafira-raster/src/blend.rs:140`, W3C section 10: "must be clamped"
- The four non-separable modes give slightly negative channels on valid input. Example: `Saturation`, backdrop `(0.7235856, 0, 0, 1)`, source `(0, 0, 1, 1)` gives `-2.8e-17`. Worst seen -4.4e-16, far inside 1e-5. `blend_u8` clamps at its output, so this is `f32` only.
- It breaks no ARQ-119 number, but the text is a "must" and the implementer's note does not list it.

### W4. The ARQ-110 property only reaches corners

- class: `implementation`
- ref: `crates/gafira-raster/src/blend.rs:807`
- Inputs are drawn from ±1e30, so after clipping every channel is 0 or 1. Its assertion that the output is in [0, 1] holds on all 3 840 corners (no flakiness), yet is false in the interior (W3). This is why C2 and W3 went unseen.
- Fix: add a draw over [-2, 2] and tiny values. It must follow the fix of W3.

### W5. No test names FD-12

- class: `implementation`
- ref: Slice card, FD-12 (c), 02:944
- FD-12 (c, CPU) "es ARQ-119", so it is covered in substance. Still, no test holds the ID, and `git grep` cannot trace it. Not CRITICAL, because step 3c binds the fast lane and this flow is `full`.
- Fix: one line of comment on the ARQ-119 table tests.

## SUGGESTION

- S1. A per-pixel call has no destination, so the rule "`Err` antes de escribir en destino" and "sin salida parcial" move to the loop in `gafira-render-cpu`. It should check the whole buffer first. ARQ-110 binds layer 2 only, so this is a note for that crate.
- S2. `crates/gafira-raster/src/error.rs:30` documents `index` as the element in the buffer; here it is the channel. Update the doc.
- S3. `crates/gafira-raster/proptest-regressions/` is still untracked, now with `blend.txt`. Verify may not delete it. Remove it and leave it out of the merge.

## Criteria

15 criteria: **11 covered, 4 weak, 0 failing, 0 missing.**

| Criterion | State | Proof |
|---|---|---|
| 16 modes per W3C | Covered | Tables, spot values |
| Premultiplied source-over | Covered | Tables, identities |
| Clip before non-`Normal`, not `Normal` | **Weak** (C2) | `arq_119_clip_applies_only_before_other_modes` |
| At least 256 triples, `f32` <= 1e-5 | Covered | `arq_119_every_mode_matches_the_reference_in_f32` (400) |
| 8 bits <= 1/255 | Covered | `arq_119_every_mode_matches_the_reference_in_8_bits` |
| Source `a = 0` leaves backdrop | Covered | `arq_119_transparent_source_leaves_backdrop` |
| Backdrop `a = 0` gives source | **Weak** (C1) | `arq_119_transparent_backdrop_gives_source` |
| `Normal` at alpha 1 gives source | Covered | `arq_119_normal_opaque_source_is_the_source` |
| Identities, difference 0 | **Weak** (C1) | Same; 8 bits exact through the table |
| ARQ-110 inputs | Covered | `arq_110_non_finite_input_is_err_for_every_mode` |
| ARQ-110 result | **Weak** (C3, C4) | `arq_110_overflowing_normal_result_is_err` |
| ARQ-110 property | Covered, poor domain (W4) | `arq_110_finite_input_gives_finite_output` |
| ARQ-111 | Covered | `arq_111_blend_is_deterministic` |
| ADR 0012 | Covered | Reuses `error.raster.non_finite`; `BufferError` has no new variant and `field` goes in `Text` |
| FD-12 (c, CPU) | Covered through ARQ-119 (W5) | Same tests |

Left out on purpose: "`gafira-render-cpu` las usa, no las redefine", since that crate does not exist yet, and the GPU half of FD-12 (c), which is not in this delivery.

Product IDs: ARQ-119 (named), FD-12 (c, CPU) (not named, W5). ARQ-110 and ARQ-111 tests are extended here.

## Size

842 lines added in `crates/` (840 in `blend.rs`, about 560 of them tests), against a forecast of 340 (475). Above the ~700 mark. Noted as the owner asked; not a finding.

Open from RS1, unchanged: W4 (ambiguity, `build_mips` memory) and W5 (implementation, pixel property), both follow-ups.

next: specifier (W1, W2), then tdd-apply (C1 to C4, W3 to W5)
