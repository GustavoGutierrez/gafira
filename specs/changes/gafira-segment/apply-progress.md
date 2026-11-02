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
- Not covered here and left to SG3: `reliable`, the IoU gate, the ARQ-112 cancel-at-k and 0.02 step tests, the benches, and tuning of gamma, regularisation, compactness and the guided-filter epsilon on the gate scenes. Correction (verify S1): the border band is not a hard background. `grabcut` uses it only to initialise the first GMMs (`cut.rs`); band nodes stay free in the graph cut, as P12 says, and a subject touching the left edge gives IoU 1.000 in the verifier's probe. There is nothing to soften in SG3.

## Fix round after verify round 1 (SG1 + SG2)

Commits: cdce5f4 (C1), 5995482 (C2), 501ab7b (tests: C4, C5, alpha8, hints shape, input alpha, ramp, W2), ea95b74 (C3, C6, C7 tests, seeds and zones tests, S2), 8da7ef1 (exact cut of extreme strokes, found by the sweep). Spec commits a2a2924 and 314f3ba (specifier) were read and covered.
Measured: `git diff --shortstat f4eb195..HEAD -- crates`: 5 files changed, 858 insertions(+), 162 deletions(-) (mostly tests). Over 700 lines in total with SG1/SG2: the coordinator decides.
Gates on the final tree, each exit code alone: `cargo fmt --check` 0; `cargo clippy --workspace --all-targets --locked -- -D warnings` 0; `cargo test --workspace --locked` 0. Commits cdce5f4 and 5995482 were gated one by one; 501ab7b was gated as part of the full tree (its tests are green on the old code, see below) and ea95b74 and 8da7ef1 after.

Done, by finding:
- C1: `Segmenter: Describe + Send + Sync`; `static DESCRIPTOR` `segment.classic`, version 1.
- C2: `SegReq.ram_bytes` = `side^2 * 16 + 8 * 4 * 282 * side` for the 2048 side (85,590,016); a first estimate, SG3 calibrates it (Q-48).
- C3: stroke geometry in `f64`; the stroke is cut to the image box first, with the coordinate across the cut set to the box side (a + t*d at 1e19 loses 2048 pixels).
- C4: tables run over the four zones; pixels over every element of a 3x2 `f32` image (alpha included); hints over x and y of both lists, the radius, and a bad value in the second point.
- C5: the size test loops over the four zones. S3: `SegHints::default()` stays zone `All` (the spec does not ask for another); the type doc now says so.
- C6: per-axis scale `(ww/w, wh/h)`; the brush radius uses the smaller one.
- C7: Elemento test with a side object larger than the central one, in the same and in another colour.
- W2: the reduction test uses a block with a white pixel in one row only (mean 64).
- S1: note corrected above. S2: strokes are worked in bands of at most 256 rows; the token is checked and progress reported at each band (cost planned from the bands, both the pin stamp and the final one).
- Coordinator's spec commit a2a2924: Fondo exact complement with no tolerance, and bounds under seeds; Todo ignores seeds (solid 255); fg wins an overlap and neighbours keep their bounds; Elemento and Primer plano seeds; Alpha8 Err in every zone; `SegHints` with only zone and strokes; ramp scene 800x600 (graded alpha); same RGB with another alpha gives the same mask bit for bit (Rgba8 and RgbaF32).

RED (failing line first, against the code before the fix; stubs, never removal):
- C1: `arq_130_segmenter_describes_itself`: `left: "" right: "segment.classic"` (supertrait added with an empty descriptor).
- C2: `arq_130_requirements_estimate_ram_from_the_working_size`: `left: 0 right: 85590016` (field added with 0).
- C3: `arq_110_extreme_finite_strokes_are_applied` (f32 code): `[-1e19, 10.0] [1e19, 10.0] x=0` assertion `>= 243`.
- C6: `arq_130_a_seed_steers_the_cut_on_elongated_images_in_both_orientations`: `tall=true: beyond the dot: 234 242`.
- S2: `fd_38_a_stroke_reports_progress_and_checks_the_token_per_chunk`: `69 against 69` (reports with and without the stroke).
- Cut of extreme strokes: `arq_110_extreme_strokes_are_applied_along_a_long_image` (first version, f64 without the cut): `tall=false -13000000000000000000 27000000000000000000 at 1024,1: 0`. The version in the tree is `..._along_their_whole_line`; its RED is the two mutants below (the code of ea95b74 is the no-cut mutant). A first cut with `a + t*d` passed the first test and failed the sweep (100x100 at 2,1); the exact-side version was then required.

Green from the start, proved by a mutant (none of these failed before the change; the code was already right):
- C4 tables, C5 sizes, Alpha8 per zone, input alpha, ramp, W2, C7, Fondo exact, Todo ignores seeds, overlap and neighbours, Elemento/Primer plano seeds.

Hand mutants (all killed; every one applied, run, reverted, none committed):
- SG1-A hints unchecked in zone All: `arq_110_non_finite_hint_is_err_before_any_work`.
- SG1-B alpha of `f32` pixels unchecked: `arq_110_non_finite_pixel_is_err`.
- SG1-C2 pipeline panics on 1x1 and width 2048: `arq_130_mask_has_the_input_dimensions`.
- SG2-D seeds not scaled (scale 1.0): `arq_130_a_seed_steers_the_cut_on_elongated_images_in_both_orientations`. Old uniform ratio (the C6 defect): the same test.
- SG2-E Elemento keeps the largest region: only `arq_130_element_keeps_the_central_object_even_when_the_other_is_larger` (the old test survived it).
- SG2-F reduction samples the first pixel: `fd_38_reduction_averages_source_blocks` (also first row only, first column only).
- Stamps swapped (spot): `arq_130_seed_strokes_set_the_alpha_under_them`; stamp order swapped (background wins an overlap): `arq_130_foreground_wins_an_overlap_and_neighbours_keep_their_bounds`.
- Fondo off by one: `arq_130_background_is_the_exact_complement_and_all_is_solid`. Todo applies a background seed: `arq_130_all_ignores_the_seeds`.
- Alpha used in the colour (`Rgba8`; `RgbaF32`): `arq_130_input_alpha_does_not_enter_the_mask`. Alpha8 accepted in Elemento: `arq_130_alpha8_is_unsupported`. Pipeline binarised after the filter: `fd_38_a_ramp_edge_gives_graded_alpha`.
- Descriptor id wrong: the descriptor test. RAM formula changed: the RAM test.
- No cut of strokes; cut with `a + t*d`: `arq_110_extreme_strokes_are_applied_along_their_whole_line`.
- Stroke progress as no-op in the pin stamp, in the final stamp, or once per stroke: `fd_38_a_stroke_reports_progress_and_checks_the_token_per_chunk`.
- Scratch note: a first run of the f64 stroke without the cut survived on 64-pixel images (rounding luck), which is why the whole-line test uses 2049, 4000 and 33x4000.

Sweep (throwaway test, not committed, release): 14 sizes (1x1, 1xN, Nx1, 3x5, 33x4000, 4000x33, 1x5000, 5000x1, 2049x9, 9x2049, 2048x2, 2x2048, 100x100, 7x4097) x 3 images (uniform, noise, subject) x 4 zones x 13 strokes (empty, negative, radius 1e9 and MAX, radius 0, denormals, 1e30, row and column through the image at 1e19, diagonal at +-MAX with radius 2 and MAX, 50 duplicate points, 400 points) x the stroke on each list = 2912 runs. Checked: `Ok`, input dimensions, non-decreasing progress ending in a single 1.0, All solid, Fondo the exact complement of Primer plano, a brush of radius 1e9 or MAX covers everything, and that row, column, duplicate and many-point strokes set their pixels in Primer plano and Elemento. First pass: 18 findings (row and column strokes at 1e19, dropped beyond pixel 1024); after the cut, 12 (the cut endpoint was off by up to 2048 pixels); after the exact side, 0. Each was fixed with a test first (above).

Deviations and notes:
- Extreme diagonal strokes (both coordinates beyond about 1e16) are as exact as an `f64`; no error is returned, and nothing is non-finite. Axis-parallel ones are exact. The sweep checked them for `Ok` only.
- Two commits bundle test and fix of several findings (ea95b74); the pieces share `lib.rs` and `hints.rs`.
- Stroke work now adds to the planned progress and reports at each band; SG3 owns the step size tests (ARQ-112).
- The size test runs 4000x3000 in the four zones; the segment contract test file takes about 40 s in debug.
- A shared scratch script of another agent overwrote one of my helpers mid-run (it ran another worktree's tests); the three stroke mutants of that moment were re-run with my own helper and killed.
- W1 (RED by removal in SG1) stays with the owner; nothing changed there.
- Unchanged: W3/W4 are the specifier's (spec commits a2a2924, 314f3ba).
