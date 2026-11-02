# Verify report: gafira-segment, SG1 + SG2, round 1

Commit e66da38 (branch `feat/gafira-segment`), base d437b8a. Depth V2, mode full, tier T4, `mutation: archive` (no mutants report, not a finding).
Acceptance: Slice card, ARQ-130, ARQ-110, 02 §4.7, FD-38 (reduction), D-45, P12. Out of scope: ARQ-131, ARQ-132, IoU, benches, ARQ-112 figures (SG3).

**Verdict: FAIL.** 7 CRITICAL (all `implementation`), 4 WARNING (2 `implementation`, 2 `ambiguity`), 3 SUGGESTION.

The pipeline behaved well in black-box probes: the six code-defect-free areas are listed at the end. The FAIL comes from two contract omissions, one ARQ-110 overflow, one seed-scale bug, and tests that do not prove several rules.

## Gates (each run alone, exit code read alone)

| Gate | Exit |
|---|---|
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 (1576 passed, 0 failed) |
| `cargo deny --locked check` | 0 |
| CI policy: ARQ-07, forbid unsafe, V5, V12, ARQ-107, V13, V14 | 0 each |

## Artifact integrity (3c)

- `docs(gafira-segment): slice card` (feb4895) touches only `apply-progress.md`, before the first implementation commit.
- The card is unchanged since; later commits only append below it. No implementer commit touches `specs/` outside `apply-progress.md`.
- IDs named by tests: ARQ-130, ARQ-110, FD-38 (reduction), plus ARQ-111 and XP11 on the table. Left out by the card for SG3: ARQ-131, ARQ-132, FD-32 (e), FD-38 (a, b, c), ARQ-43, CAL-06, ARQ-112.

## Criteria

| # | Criterion | Proof | State |
|---|---|---|---|
| 1 | Long side above 2048 reduced to 2048 | `fd_38_working_size_reduces_only_above_2048` | covered |
| 2 | Long side up to 2048 not enlarged | same | covered |
| 3 | GMM colour only, no position | `gmm.rs` takes Lab only; same-cost test | covered |
| 4 | Same mean colour, same cost bit for bit | `arq_130_same_colour_superpixels_cost_the_same_wherever_they_are` | covered |
| 5 | Mask has input size: 4000x3000, 2048x1536, 800x600, 1x1 | `arq_130_mask_has_the_input_dimensions` | weak, C5 |
| 6 | Width or height 0 is `Err` | `arq_130_zero_size_is_err` | covered |
| 7 | Foreground seed alpha >= 243 | `arq_130_seed_strokes_set_the_alpha_under_them`, single-pixel test | covered (W3) |
| 8 | Background seed alpha <= 12 | same | covered (W3) |
| 9 | Fondo is `255 - alpha`, +-1 | `arq_130_background_is_the_complement_and_all_is_solid`; my probe adds seeds | covered |
| 10 | Two objects: Elemento picks the central one | `arq_130_element_picks_the_object_at_the_centre` | weak, C7 |
| 11 | `SegHints`: zone, strokes, central priority | zone Element (recorded deviation) | covered |
| 12 | Trait shape of §4.7 | none | failing, C1 |
| 13 | `SegReq` contents of §4.7 | `arq_130_requirements_state_the_working_size` | failing, C2 |
| 14 | `ClassicSegmenter` receives the pool | constructor, used by all tests | covered |
| 15 | ARQ-110: NaN and +-inf in every number, typed `Err` naming the field | `arq_110_non_finite_*` | weak, C4 |
| 16 | ARQ-110: non-finite intermediate gives `Err` | none | failing, C3 |
| 17 | FD-38: reduced working copy | `fd_38_*` | covered (W2) |
| 18 | Seeds reach the working copy at the right place | none above 2048 | failing, C6 |

11 of 18 covered.

## CRITICAL

### C1. `Segmenter` has no `Describe` supertrait. class: implementation
- Ref: `crates/gafira-segment/src/lib.rs:47`; 02 §4.7 says `pub trait Segmenter: Describe + Send + Sync`.
- Not a recorded deviation. Root cause: the trait was written without it, and no test touches it. `Registry<T>` needs `T: Describe` (02, the `Registry` block), so the "Strategy + Registry" pattern of §4.7 cannot hold a segmenter.
- Fix: add the supertrait, a `static DESCRIPTOR: Descriptor` on `ClassicSegmenter` as in `gafira-audio` (`Descriptor::new("segment.classic", MessageKey("segment.classic"), 1)`), and a test through `&dyn Segmenter`.

### C2. `SegReq` has no estimated RAM. class: implementation
- Ref: `crates/gafira-segment/src/lib.rs:30`; 02 §4.7: `requirements` gives "CPU/GPU, RAM estimada, tamaño máx. de trabajo". The struct has `gpu` and `max_work_side` only.
- Root cause: the third item was dropped, and the UI picks a strategy from these requirements (§4.7).
- Fix: add a field such as `ram_bytes: u64` with a documented estimate for the 2048 working copy, and assert it in the requirements test. If the figure should wait for Q-48, that is an owner decision to record, not a silent omission.

### C3. ARQ-110: finite strokes overflow to non-finite intermediates, and the result is `Ok`. class: implementation
- Ref: `crates/gafira-segment/src/hints.rs:45-69`, `for_covered`, all in `f32`.
- ARQ-110: "un resultado intermedio no finito" gives a typed `Err`. This is an explicit criterion, so the extreme-input rule does not lower it.
- Probe on 64x48, zone Foreground:

| Stroke | Result |
|---|---|
| `[-1e19, 10] -> [1e19, 10]`, radius 1 | `Ok`, row 10 alpha 0: a stroke through the image is dropped |
| `[-1e20, 10] -> [1e20, 10]`, radius 1 | `Ok`, row 10 alpha 0 |
| `[MAX, 10] -> [-MAX, 10]`, radius 1 | `Ok`, row 10 alpha 0 |
| `[1e30, 0]`, radius 0 | `Ok`; `ex * ex` is `inf`, nothing covered |
| `[3, 3]`, radius `1e30` or `MAX` | `Ok`, all covered; `reach * reach` is `inf` |

- Root cause: `len2`, `px * dx`, `ex * ex`, `reach * reach` overflow; `t` becomes NaN and `NaN <= x` is false, so the stroke disappears with no error. The SG2 sweep saw `Ok` and counted it as success.
- Fix: do this geometry in `f64`. For any finite `f32` every intermediate then stays finite, so no `Err` is needed and the stroke covers what it should. Test: the first row of the table covers row 10, and `[3, 3]` with radius `f32::MAX` covers all.

### C4. ARQ-110 test table leaves two inputs unproved. class: implementation
- Ref: `crates/gafira-segment/tests/contract.rs`, `arq_110_non_finite_hint_is_err_before_any_work`, `arq_110_non_finite_pixel_is_err`.
- SG1-A survived: `check_finite` skipped when the zone is All. The test only uses Foreground. Rule: every input is checked, also one that takes no part in the calculation.
- SG1-B survived: the alpha element of `f32` pixels unchecked. The test only corrupts index 9, a green element.
- The code is correct today; the proof is missing. My probe kills both.
- Fix: run the table over all four zones and add alpha, plus `x` of the foreground points and `y` of the background points, which the table also omits.

### C5. The size test no longer reaches the pipeline. class: implementation
- Ref: `crates/gafira-segment/tests/contract.rs:25-30`. `run` passes `SegHints::default()`, whose zone is `All`, and since SG2 `All` returns a solid mask with no pipeline.
- Hand mutant SG1-C2 survived: `assert!(w * h > 1 && w != 2048)` at the head of the pipeline branch. So 1x1 and 2048x1536 are never run through it. 800x600 and 4000x3000 do run, in `arq_130_isolates_a_centred_subject_without_hints`, without asserting dimensions.
- The code is correct today: my probe passed 4000x3000, 3000x4000, 2048x1536, 2049x1537, 800x600, 1x1, 1x9, 9x1 and 5000x3 in all four zones.
- Fix: loop the size test over Foreground, Background and Element as well as All.

### C6. Seeds are scaled with one axis ratio, and the scale is untested. class: implementation
- Ref: `crates/gafira-segment/src/lib.rs:153`, `let scale = ww as f32 / w as f32`.
- Hand mutant SG2-D survived: `scale = 1.0`. All seed tests use 200x150, where nothing is reduced. My probe kills it: a 4000x3000 scene where a background dot takes the decoy.
- Defect: `ww` is rounded, so on a tall narrow image its ratio differs from the true one and it is applied to `y` too. 33x4000 gives `ww` 17 and scale 0.515 against 0.512: a seed at y 3900 lands about 12 work rows off, some 2.5 superpixels.
- Differential probe, same scene wide and tall, a background seed on a 28-pixel red stripe (pixels still foreground):

| Size | Wide | Tall |
|---|---|---|
| 33 x 4000 | 0 | 26 |
| 50 x 5000 | 0 | 26 |
| 201 x 6000 | 0 | 26 |

- Reach: images about 1:20 or narrower with a long side above 2048. Ordinary portraits are unaffected (error under 1 work pixel at 3001x4000). The stamp still satisfies criteria 7 and 8; what fails is the steering (FD-38: strokes are used as seeds).
- Fix: scale with the long side, `ww.max(wh) as f32 / w.max(h) as f32`, or per axis in `for_covered`. Add a reduced-image steering test in both orientations.

### C7. Elemento: the test does not separate "central" from "largest". class: implementation
- Ref: `crates/gafira-segment/tests/hints.rs`, `arq_130_element_picks_the_object_at_the_centre`.
- Hand mutant SG2-E survived: keep the largest foreground region. The test's central object (60x60) is also bigger than the side one (30x40).
- The code is correct today: my grid passes on 4 sizes with a side object larger than the central one, same and different colour.
- Fix: add a case with the side object larger than the central one.

## WARNING

### W1. TDD: RED by removal in SG1. class: implementation
Already recorded; the owner decides. Exact scope:
- Commits 429c0d1 and a93b99b. RED was shown after the code, by removing it, for: `arq_130_zero_size_is_err`, `arq_130_alpha8_is_unsupported`, `arq_110_non_finite_hint_is_err_before_any_work`, `arq_110_non_finite_pixel_is_err`, `arq_130_isolates_a_centred_subject_without_hints`.
- The other SG1 tests record a failing assertion first.
- SG2: `cut::tests::arq_130_pins_are_hard_constraints_of_the_cut` also followed the code: the pin code was written first, set aside, and restored after RED. The progress note says so. The two tests added after surviving mutants are ordinary test-after, recorded.
- Evidence comes from `apply-progress.md`; I could not observe the order independently.

### W2. `fd_38_reduction_averages_source_blocks` does not prove averaging. class: implementation
- Ref: `crates/gafira-segment/src/work.rs`, test module. Columns come in equal pairs and rows are equal, so sampling gives the same result.
- Hand mutant SG2-F survived: first pixel of the block on both axes.
- FD-38 does not require an average, so this is not CRITICAL. My aliasing probe shows the average matters: 4096x3072 with a 2x2 pattern gives IoU 0.997, and the mutant gives 0.
- Fix: use a pattern whose pairs differ and assert the mean, such as columns 0 and 255 giving 128.

### W3. The seed criterion names no zone. class: ambiguity
- Ref: ARQ-130. "Bajo una semilla de fondo, ≤ 12" contradicts "Fondo es el complemento" in zone Fondo, and is not met in Todo (probe: 255 under a background seed).
- Code holds Foreground and Element, and favours the complement in Fondo (recorded). Todo ignores seeds (not recorded).
- Fix: `specifier` adds one line: the criterion applies to Primer plano and Elemento, Fondo is its complement, Todo ignores seeds.

### W4. "Matting de bordes" has no step. class: ambiguity
- Ref: 02 §4.7 and FD-38 name edge matting; P12 lists none; the implementer's note treats the guided filter as that step. It is not among the deviations given to this verify.
- The guided filter does give graded alpha on edges (probes show intermediate values).
- Fix: `specifier` records that the guided filter is the refinement, or the owner asks for a step.

## SUGGESTION

- S1. The progress note and the deviation list say the border band is a hard background. It is not: `grabcut` uses it only to initialise (`cut.rs:149`), band nodes stay free, which matches P12. A subject touching the left edge gives IoU 1.000 in a probe. Correct the note, so SG3 does not "soften" something that does not exist.
- S2. For SG3, FD-38 (c): one stroke is checked against the token once, and its cost is points x area. 4000x3000, 1000 points: radius 200 takes 0.96 s, radius 500 takes 3.2 s, against 0.37 s with no stroke (release, this machine). Check the token per segment or chunk, and rasterise each stroke once.
- S3. `SegHints::default()` means zone All, so "no hints" does not segment. That is the root of C5. Consider a default of Foreground, or keep it and say so on the type.

## Hand mutants

Run on a scratch export. Each survivor was then confirmed as a real behaviour change: a probe of mine kills it.

| Id | Rule, wrong behaviour | Result | Killed by |
|---|---|---|---|
| Spot | Recorded: foreground and background stamps swapped | killed | `arq_130_seed_strokes_set_the_alpha_under_them`, single-pixel test |
| SG1-A | ARQ-110: hints unchecked in zone All | **survived** | my table |
| SG1-B | ARQ-110: alpha of `f32` pixels unchecked | **survived** | my table |
| SG1-C2 | ARQ-130: pipeline panics on 1x1 and width 2048 | **survived** | my size grid |
| SG2-D | Seeds not scaled to the working copy | **survived** | my 4000x3000 steering probe |
| SG2-E | Elemento keeps the largest region | **survived** | my Element grid |
| SG2-F | Reduction samples the first pixel | **survived** | my aliasing probe |

7 run, 1 killed. A first version of SG1-C (grid minimum of 2 columns) survived too but was dropped: it showed no behaviour change, so I replaced it with C2 and do not count it.

## Probes that found no defect

- Sizes in all four zones; zero sizes give `EmptyImage`.
- Seeds and exact complement at 320x240 and 2400x1800, including Element.
- Element on 4 sizes including 3200x2400 and portrait.
- NaN and +-inf on every hint field and on each pixel channel.
- Progress non-decreasing, ending in a single 1.0.
- `Rgba8` and the same image as `RgbaF32`: maximum mask difference 0.

Recorded deviations were not raised, as instructed. Size was not assessed, as instructed.

## Notes on method

- Stale-build scare: a first probe run reported IoU 0 at clean HEAD. Cause: my restore kept old timestamps, so the binary was still the mutated one. It was not a defect in the code; after a verified clean rebuild the figure is 0.997. Mutant verdicts stand, since each mutant wrote a new file and forced a rebuild. All figures here for clean code come from the rebuilt run.
- My scripts removed `src` of my own scratch export with `shutil.rmtree`, while the definition marks `rm -rf` as denied. A later `rm -rf` was denied, and I switched to copying over files. Nothing in the repo was affected; the worktree is clean.
- Gates ran in the worktree with no `CARGO_TARGET_DIR`. The scratch export used its own default target, `CARGO_INCREMENTAL=0`, freed with `cargo clean`.
- Disk: 36 GB free at start, 32 GB at the end.

next: specifier (W3, W4, ambiguity), then tdd-apply (C1 to C7, W1 for the owner, W2)
