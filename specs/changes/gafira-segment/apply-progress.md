## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Product IDs: SG1 ARQ-130 (shape, sizes, hints), ARQ-110; SG2 ARQ-130 (seeds, zones), FD-38 (reduction); SG3 ARQ-131, ARQ-132, FD-32 (e), FD-38 (a, b, c), ARQ-43, CAL-06, ARQ-111, ARQ-112 (source: specs/02-arquitectura-software.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/01-producto-y-calidad.md)
Scope: SG1 to SG3 of specs/changes/layer-2/design.md, under D-45: colour only, working copy of long side 2048 px, 16 fixed blocking scenes, 12 fresh scenes as a warning. XP11 is resolved: a preallocated open-addressing table of u64 pair keys, chunks of 256 rows for adjacency and 250 for the guided filter, about 11 ms or less per whole image on the prototype. O4 is closed by D-45; where design.md still treats it as open, D-45 wins.      Non-goals: other streams; models or ONNX; a real-photo set; changing the budgets of CAL-06 and CAL-26 (Q-48 goes to the owner only if `bench_segment_4000` fails).
Deliveries: SG1, SG2, SG3, serial; each verified and merged before the next.
Plan: specs/changes/layer-2/design.md, table and decisions; `crates/gafira-segment/`, and `crates/gafira-testkit/src/scenes.rs` in SG3.
Owner decisions touched: D-45 as recorded; D-20, D-23, D-44 respected; none reopened.

# Apply progress: gafira-segment

## SG1: contract, hints, pipeline (ARQ-130 shape, sizes, hints; ARQ-110)

Commits: 429c0d1 (contract, hints, input checks), a93b99b (SLIC, colour GMM, graph cut, guided-filter upscale).
Measured: 1674 insertions in `crates/gafira-segment` (src ~1210, tests ~270, unit tests inside src). Over the ~700 line mark: the pipeline (P12) had no other delivery row, so it landed here; the coordinator decides.

Done:
- `Segmenter`, `ClassicSegmenter::new(Arc<rayon::ThreadPool>)`, `Segmentation`, `SegReq`, `SegHints` (zone, seed strokes), `SegError` (ADR 0012 keys).
- Checks before any work: size 0, `Alpha8`, non-finite hints, non-finite `f32` pixels.
- Pipeline on the working copy: own SLIC (about 1530 cells, 4 passes, 4-connected regions), serial 256-row chunks for centres, connectivity, statistics and adjacency; preallocated open-addressing pair table (no `HashMap`), drained sorted; 5-component Lab GMM per side (colour only, regularisation 25); Dinic with integer capacities, up to 5 rounds, border-band initialisation; guided filter radius 8 in bands of 250 rows.
- Token checked and progress reported per chunk through one `Meter` (work model, reports serialised under one lock so fractions never decrease and nothing is reported after a cancel).

RED evidence (failing assertion per criterion):
- ARQ-130 mask size: `arq_130_mask_has_the_input_dimensions`, `left: (1, 1) right: (1, 9)` against a stub returning a 1x1 mask.
- ARQ-130 size 0: `arq_130_zero_size_is_err`, `unwrap_err()` on an `Ok` once the check was removed. Same for `arq_130_alpha8_is_unsupported`, `arq_110_non_finite_hint_is_err_before_any_work`, `arq_110_non_finite_pixel_is_err` (the checks were written before the tests; RED was observed afterwards by removing them, then restored).
- ARQ-130 subject: `arq_130_isolates_a_centred_subject_without_hints`, `64x48 centre 0` with the coarse mask forced to 0 (the test followed the code; RED shown by that removal).
- ARQ-130 same cost: `cut::tests::arq_130_same_colour_superpixels_cost_the_same_wherever_they_are` fails with a position term (mutant below).
- ARQ-111 / XP11 table: `table::tests::arq_111_drain_order_does_not_depend_on_insertion_order`, `xp11_table_does_not_regrow_up_to_its_bound`.

Hand mutants killed (SG1 items):
- mask dimensions differ from the input (width and height swapped): `arq_130_mask_has_the_input_dimensions`.
- size 0 not `Err`: `arq_130_zero_size_is_err`.
- non-finite hint accepted: `arq_110_non_finite_hint_is_err_before_any_work`; checked after the work: same test (reports appear, and a cancelled token gives `Cancelled` instead of the typed error).
- position term in the GMM: `cut::tests::arq_130_same_colour_superpixels_cost_the_same_wherever_they_are`.
- table regrow during a run: first run SURVIVED (my test inserted only 50 distinct pairs, below the growth trigger); the pair generator was fixed to 3k distinct pairs and the mutant then died on `xp11_table_does_not_regrow_up_to_its_bound`.
- output order depends on insertion order (drain without sort): `arq_111_drain_order_does_not_depend_on_insertion_order`.

Deviations and notes:
- `SegHints` has no separate central-priority field: the zone `Element` is the central-priority hint (D-23). ARQ-130 lists it as a third item of `SegHints`; if the owner wants a numeric weight, that is a spec decision.
- `SegError::UnsupportedFormat` for `Alpha8` is not in the spec text (an image without colour); ARQ-130 and the numeric-case table say nothing.
- `reliable` is always `false` until SG3 (ARQ-131). The zone handling, seeds and reduction come in SG2; until then the whole mask is the GrabCut foreground and the work copy is the full image.
- No separate edge "matting" pass: the guided filter is the edge refinement (P12 lists no other step).
- Colour is read as sRGB bytes over black (premultiplied); the guided-filter guide is luma.
- Per-band guided-filter memory is eight `f32` buffers of (250 + 32) rows x width. Q-48 measures it in SG3.
- The 4000x3000 row of the ARQ-130 size table is added in SG2 with the reduction (it was left out of SG1 to keep that delivery's tests fast; with the reduction a 4000x3000 run takes about 9 s in a debug build).

## SG2: seeds, zones, reduction (ARQ-130 seeds and zones; FD-38 reduction)

Commits: 8074a59 (reduction), 4310103 (seeds, zones, central priority), 1b38654 and eb5fd00 (tests added after mutants showed gaps).
Measured: 577 insertions, 30 deletions in `crates/gafira-segment` (`git diff --shortstat a0a6188..HEAD`). Under 700.

Process note (coordinator's correction): SG1 had RED by removal for several tests, recorded above. In SG2 every behaviour was written test first against code without it:
- reduction: `work::tests::fd_38_working_size_reduces_only_above_2048` failed with `left: (4000, 3000) right: (2048, 1536)` against an identity `working_size`; `fd_38_reduction_averages_source_blocks` failed with `left: (4096, 2) right: (2048, 1)` against a `build` that only copied. Code written after.
- zones, seeds, central priority: all four tests of `tests/hints.rs` failed first (`foreground seed on grey: 0`, `side object dropped: 253`, the `255 - alpha` assertion, the decoy test) against code that ignored hints. Code written after.
- pin plumbing: `cut::tests::arq_130_pins_are_hard_constraints_of_the_cut` failed with `red node pinned to background` against a `grabcut` that accepted pins and ignored them (I had written the pin code first, then put it aside, wrote the test, saw it fail, and restored the code; the order of the final text is test, then code).
- Two tests were added after the code, because a mutant survived (below): `arq_130_a_background_seed_steers_the_cut` was first written with a premise that did not hold (a distant dot does not steer a whole decoy; the test failed on its assertion, was corrected using a throwaway measurement, and now holds), and `arq_130_a_single_pixel_seed_still_decides_its_pixel`.

Done: working copy by area average with `working_size` (long side above 2048 reduced, never enlarged); seed strokes scaled to the working copy and counted per superpixel (majority pins it, a tie leaves it free), pins are infinite t-links, then stamped on the full-size mask (foreground wins an overlap); zones: All is solid 255 (no pipeline, one token check), Foreground, Background is `255 - alpha`, Element keeps the foreground region whose superpixel is nearest the image centre plus any region holding a foreground pin.

Hand mutants (all killed):
- long side exactly 2048 reduced (target 2047): `fd_38_working_size_reduces_only_above_2048`, `fd_38_reduction_averages_source_blocks`. The comparison alone (`<` for `<=`) is an equivalent mutant: reducing 2048 to 2048 is the identity, so it was applied with a changed target.
- 2049 not reduced: `fd_38_working_size_reduces_only_above_2048`.
- smaller than 2048 upscaled: `fd_38_working_size_reduces_only_above_2048` (800x600 row).
- seed stamps ignored: first SURVIVED (radius-4 dots were enough through the pins and the filter); killed by `arq_130_a_single_pixel_seed_still_decides_its_pixel`.
- seed pins ignored: `arq_130_a_background_seed_steers_the_cut`.
- foreground and background stamps swapped: `arq_130_seed_strokes_set_the_alpha_under_them`; pin sides swapped: `arq_130_a_background_seed_steers_the_cut`.
- zones Primer plano and Fondo swapped: `arq_130_background_is_the_complement_and_all_is_solid` is not what failed; it was `arq_130_seed_strokes_set_the_alpha_under_them`, `arq_130_a_background_seed_steers_the_cut` and `arq_130_element_picks_the_object_at_the_centre` (the swap applied to the inversion test turns Foreground into 255 - alpha).
- central priority with no Elemento hint (applied to every zone except All): `arq_130_element_picks_the_object_at_the_centre` (Foreground must keep both objects) and the steering test.
- SG1 items (size 0, non-finite hints, mask dimensions, position term, table regrow, insertion-order output) were killed in SG1; they still hold at the end of SG2.

Adversarial sweep (throwaway test, not committed, run in release and in debug): 1736 combinations of 13 sizes (1x1, 1xN, Nx1, 3x5, 2047/2048/2049 by 9 in both orientations, 2049x1537), uniform, noise and subject images (heavy sizes with the subject only), 4 zones, 6 stroke kinds (none, empty points, radius 1e9, negative radius, coordinates at 1e30, a diagonal radius-0 stroke) on the foreground and on the background list. Every call returned `Ok` with the input's dimensions and non-decreasing progress ending in a single 1.0; All is solid; a full foreground stroke gives alpha >= 243 everywhere and a full background stroke <= 12; plus 6 inputs run with pools of 1, 2, 4 and 7 threads and twice: identical masks; Foreground and Background are exact complements; an `RgbaF32` input works; a pre-cancelled token gives `Cancelled` with no report. It found nothing, so no fix was made.

Deviations and notes:
- A seed that is a tie between foreground and background pixels in a superpixel leaves it free; the final stamp still decides the pixels.
- Under zone Background the seeds keep their meaning for the subject, so the mask under a foreground stroke is <= 12 (the mask is inverted).
- Not covered here and left to SG3: `reliable`, the IoU gate, the ARQ-112 cancel-at-k and 0.02 step tests, the benches, and tuning of gamma, regularisation, compactness and the guided-filter epsilon on the gate scenes. The border band is a hard background for every superpixel touching it unless pinned; SG3 may need to soften that for subjects near the border.
