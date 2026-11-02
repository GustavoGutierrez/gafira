## Slice card
Flow: full | level: L3 | pack: product-spec slice | mutation: archive | tier: T4 | verify: V2 | decided-by: owner | reason: layer-2 crate in the owner's parallel full route; one writer per worktree
Flow-change: no closure verify for SG3 (trigger: owner order to stop rounds unless grave; decided-by: owner). Gate 16/16, XP5 fresh 9/12, gates green; verify round 1 findings fixed in fd4b99f, 8ca7fb4, 8d0a3b5 and not re-verified.
Flow-change: T4 skipped at close (trigger: owner order; decided-by: owner; G5 waived for this change). T4 ran in part and was stopped by owner order: 1 271 of 1 421 tested, 983 caught, 236 missed (gmm 73, slic 51, cut 41, hints 31, refine 21), 34 timeouts, unclassified.
Product IDs: SG1 ARQ-130 (shape, sizes, hints), ARQ-110; SG2 ARQ-130 (seeds, zones), FD-38 (reduction); SG3 ARQ-131, ARQ-132, ARQ-133, FD-32 (e), FD-38 (a, b, c), ARQ-43, CAL-06, ARQ-111, ARQ-112 (source: specs/02-arquitectura-software.md, specs/04-funcionalidad-diseno-imagen-exportacion.md, specs/01-producto-y-calidad.md)
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

## SG3: gate, reliability, progress, benches (ARQ-131, ARQ-132, FD-32 e, FD-38 a b c, ARQ-43, CAL-06, ARQ-111, ARQ-112)

Commits: 553b11f (W5), 17d36fa (W6), 5cbd7a5 (scenes), 1444da9 (dev profile), 98acb1a (reliable), a7f4d77 (chunks, progress tests), 1642222 (gate, regularisation), 5d4230a (benches, RAM).
Measured: `git diff --shortstat 65a7bb5..HEAD`: 16 files, 1606 insertions, 47 deletions (src 937, tests and benches 658). Over the ~700 line mark: the coordinator decides.
Gates on every code commit, each alone on the staged tree: `cargo fmt --check` 0, `cargo clippy --workspace --all-targets --locked -- -D warnings` 0, `cargo test --workspace --locked` 0.

### SG2 follow-ups
- W5: `hints::tests::arq_110_a_stroke_from_beyond_1e16_ends_where_it_ends`, RED `[1e19, 4.5] -> [1500.5, 4.5]`, left `(Some(1500), Some(2999), 954)` right `(Some(1500), Some(2999), 1500)`. Fix: an uncut end is `a` or `b` itself. The table of the verify report (six rows) is the test; the note "axis-parallel exact" now holds whatever the far end.
- W6: the ramp test counts only pixels 1 to 15 away from the subject's square. It was green before (the code was right); mutant: binarise after the filter, killed (`graded pixels in the ramp: 0`).

### Scenes (`crates/gafira-testkit/src/scenes.rs`, mine)
- 16 recipes: flat x7, gradient x4, textured x5; centred and offset; ellipse, rounded rectangle, C (concave), cross (concave), L (concave), ring and rectangle ring (holes); two flat low-contrast (ΔE76 7.82 and 9.10). Subject 6.5 % to 19.1 % of the area; ΔE76 7.82 to 79.24 at 4000x3000. Fresh set: 12 of the 16 recipes (indices `i * 16 / 12`) from `FRESH_SEED`; same constraints (test at 800x600).
- Colours by construction: the subject colour is sampled against the exact mean of the background, so the ΔE76 range holds; the tests recompute it from the final pixels.
- RED (all on assertions, stub with wrong values): `left: (0, 0) right: (4000, 3000)` for the 16 scene tests; `arq_132_iou_090_passes_and_08999_fails`, message `90/100`; `arq_132_empty_masks_have_iou_one` `left: 0.0 right: 1.0`; `arq_132_fresh_report_counts_and_never_fails` `left: (0, 0) right: (0, 12)`; `arq_132_scenes_are_deterministic_and_the_seeds_differ` `other seed, other pixels`.
- Q-49: gate at 4000x3000 (reduced to 2048x1536). ARQ-112 reference at 1024x768 (`reference_scene`, recipe 0).

### reliable (ARQ-131, D-44)
- `reliable.rs`: `decide` (area 1 % to 95 % inclusive in integers, ΔE76 >= 5) and `assess` (serial chunks of rows, token per chunk, linear-light sums weighted by input alpha). The colour is read as stored, over black (ARQ-130); alpha 0 adds nothing; no weight gives no colour and no NaN. `reliable` is computed on the returned mask, `All` and `Background` included.
- RED: `arq_131_rule_bounds_are_inclusive` `100/10000 dE Some(9.0) left: false right: true`; `arq_131_means_are_averaged_in_linear_light` `a mean colour`; `arq_131_a_fixed_scene_is_reliable` `assertion failed: out.reliable`; `arq_131_input_alpha_weights_reliability_not_the_mask` `assertion failed: opaque.reliable`; `arq_112_assess_stops_on_a_cancelled_token` `left: Ok(false) right: Err(Cancelled)`.
- Green from the start, proved by a mutant: uniform, noise, 0.5 % subject, 1x1, `All` (mutant: `decide` returns true).

### Gate and tuning (D-45)
- RED: `arq_132_gate_scene_06` IoU 0.1917 (gradient-ellipse), `_07` 0.1431 (gradient-roundrect), `_08` 0.0980 (gradient-c), with gamma 500, regularisation 25, compactness 25; 13 of 16 passed. The foreground took the half of the gradient closest in colour to the subject.
- Tuning ran on the 16 gate scenes only, colour only, working copy 2048. Sweeps (min over the 16 scenes, failing scenes): gamma 50, 100, 500, 2000 at reg 25: no change (the unary terms are weighted by superpixel size, so gamma barely moves the cut); reg 5: scenes 6 to 9 fail; reg 100: 6 and 8 fail; reg 150: 8 fails; reg 250: 8 fails; reg 300, 400, 600, 1000: 16 of 16; compactness 8 (reg 25): scenes 10 to 14 drop to 0.89 to 0.95 and 6 to 8 still fail. Chosen: gamma 500 and compactness 25 unchanged, regularisation 400 (inside the 300 to 1000 plateau). Spec text says "about 25": it is PROPUESTA and the spec allows changing it while ARQ-132 and CAL-06 hold.
- Result with 400, IoU per scene at 4000x3000: 00 flat-ellipse-centred 1.0000; 01 flat-roundrect-offset 1.0000; 02 flat-c-centred 1.0000; 03 flat-ring-centred 0.9972; 04 flat-ellipse-low-contrast 0.9979; 05 flat-cross-offset-low-contrast 0.9955; 06 gradient-ellipse-centred 1.0000; 07 gradient-roundrect-offset 1.0000; 08 gradient-c-offset 1.0000; 09 gradient-rectring-centred 1.0000; 10 textured-ellipse-centred 0.9909; 11 textured-roundrect-offset 0.9896; 12 textured-cross-centred 0.9963; 13 textured-ring-offset 0.9926; 14 textured-l-centred 0.9961; 15 flat-rectring-offset 1.0000. 16 of 16 and all reliable.
- Fresh set: run once, after the last parameter change and the sweep, with `--nocapture`: 12 of 12 at IoU >= 0.90 (0.9806 to 1.0000; lowest 0.9806 fresh scene 4 flat-cross-offset-low-contrast, 0.9860 fresh scene 9 textured-cross-centred). Nothing was tuned on it and no scene or parameter changed after it. Caution: the scenes are mine (flat subject, hard edges, no photographic noise), so 12 of 12 is not comparable with the 9 of 12 of D-45 and is no evidence of generalisation. The tuning and the generator were written by the same agent; a reader should treat both counts as in-sample for the generator.

### Progress and cancellation (ARQ-112)
- RED: `arq_112_progress_and_cancellation_on_the_reference_input` `Err(StepTooLarge { at: 3 })` (steps of 0.0387 with chunks of 256 rows, 0.0504 for 250-row filter bands); `arq_112_one_thread_gives_the_same_fractions_twice` `47 reports` (the contract needs at least 50 for a 0.02 step).
- Fix: `CHUNK_ROWS` 256 to 64 and `BAND_ROWS` 250 to 80. Reference run: 170 reports, largest step 0.0196 (the final reserve), then 0.0161. XP11 named 256 and 250 rows; smaller chunks only shorten the unit between token checks. The guided-filter margin overhead grows from 13 % to 40 %.
- Green from the start (proved by mutant): determinism across 1, 2, 4, 7 threads (4000x3000 and 640x480), report count independent of the pool.

### Benches and memory (release, this machine: i7-class 20 threads, not D-03)
- `bench_segment_4000` (textured cross, 4000x3000, criterion, 10 samples): 1 thread 969 ms, 2 threads 510 ms, 4 threads 416 ms. CAL-06 < 5 s holds with a margin of 5x on one thread.
- `bench_cancel_segment` (longest gap between two reports, which come with every token check; criterion's `WallTime`, no `Instant::now`): 4 threads 16.9 ms (14.7 to 19.0), 1 thread 11.3 ms, 2 threads 10.8 ms. ARQ-43 <= 100 ms holds. Strokes are checked per band of 64 rows; no stroke stretch is chunked further.
- Peak memory, `/usr/bin/time -v` on a release test binary, 4000x3000 textured scene, resident set minus the same process generating the scene: 34 MB (1 thread), 71 MB (4), 127 MB (8), 293 MB (20). CAL-26 <= 1.5 GB holds (the process total of that test is 90 to 349 MB). The peak grows by one guided-filter band per thread (8 buffers x 112 rows x 4000 x 4 B = 14.3 MB).
- `SegReq.ram_bytes` = 30,000,000 + 14,336,000 x pool threads (44 MB, 87 MB, 145 MB, 317 MB for 1, 4, 8, 20), above each measurement; it replaces the 85.6 MB first estimate. `arq_130_requirements_estimate_ram_from_the_measured_peak` pins the formula and the measured figures.
- Dev profile: `[profile.dev.package.gafira-segment]` and `gafira-testkit` at `opt-level = 2` in the root `Cargo.toml`: the gate's debug run fell from about 66 s to 2.4 s for the progress file. Overflow checks stay on.

### Hand mutants (each applied, run, reverted; none committed)
- IoU `>=` to `>` (and `meets_gate` strict): `arq_132_iou_090_passes_and_08999_fails`, `arq_132_empty_masks_have_iou_one`. Binarise at `> 128`: `arq_132_iou_binarises_at_alpha_128`.
- Area limits 1 % and 95 % exclusive, ΔE76 5 strict: `arq_131_rule_bounds_are_inclusive` (lower bound also `arq_131_foreground_is_alpha_128_or_more`). Foreground at `> 128` in `reliable`: `arq_131_foreground_is_alpha_128_or_more`.
- Averages in sRGB (D-44), in `reliable`: first run SURVIVED (my mutant was an identity table, which still averages linear values); the real mutant (average encoded values, decode after) died on `arq_131_means_are_averaged_in_linear_light` and `arq_131_means_are_weighted_by_input_alpha`. In the scenes' `mean_delta_e76`: SURVIVED because generator and check shared the function; test `d_44_scene_means_are_averaged_in_linear_light` added (hand scene, L 76.07 against grey 188); then killed.
- Input alpha not weighting the colours: `arq_131_means_are_weighted_by_input_alpha`, `arq_131_input_alpha_weights_reliability_not_the_mask`. No token check in `assess`: `arq_112_assess_stops_on_a_cancelled_token`. Group with weight 0 giving a colour: `arq_131_means_are_weighted_by_input_alpha`. `decide` always true: three tests of each file.
- Fresh set made blocking (the warning asserts on its count): `arq_132_a_segmenter_failing_every_fresh_scene_reports_0_of_12`; `fresh_report` with `>`: `arq_132_fresh_report_counts_and_never_fails`.
- Fresh set tuned: not a code mutant. It is held by process: the fresh set was run once, after the last parameter change; the commit order shows the tuning (1642222) before the run. `FRESH_SEED` equal to the fixed seed: `arq_132_scenes_are_deterministic_and_the_seeds_differ`.
- Generator non-deterministic (a clock in the seed): `arq_132_scenes_are_deterministic_and_the_seeds_differ`. Scene sizes out of range (subject 50 to 55 %): `arq_132_fixed_scene_04_area_and_contrast` and the fresh test; low-contrast range widened: `arq_132_fixed_scene_04_area_and_contrast`.
- Progress decreasing (a fraction scaled by 0.9 on some reports): `arq_112_progress_and_cancellation_on_the_reference_input`. Report after cancel (`step` reports when cancelled): same test. No token check in `step`: same test. Result delivered after cancel (`let _ = meter.finish()`; also `finish` without a check): same test. Step above 0.02 (chunks back to 256 or 250 rows): same test.
- Thread count changing the mask (SLIC compactness scaled by the pool size): `arq_130_the_mask_does_not_depend_on_the_thread_count`.
- RAM per-thread part or fixed part changed: `arq_130_requirements_estimate_ram_from_the_measured_peak`.

### Sweep (throwaway test, not committed, release)
- Cancel at every report of the reference input (1024x768, 170 reports; 14 for `All`) for 1, 2, 4 and 7 threads and the zones Foreground, Background, Element and All, plus every fifth report of a 4000x3000 scene (377 reports) and a 1x1: 0 findings; each cancel before the last report gave the cancelled error with at most one report after it, no decrease, no 1.0 after the cancel.
- Determinism with 1, 2, 4 and 7 threads over all 28 scenes (16 fixed and 12 fresh) at 4000x3000, 1100x800 and 333x777, zones Foreground and Element: mask and `reliable` identical in every case; 0 findings. IoU was not read on the fresh scenes there.

### Deviations and questions
- `reliable` is computed on the returned mask, as ARQ-131 words it. So `Background` (the complement) is unreliable when the subject is under 5 % of the image (the mask exceeds 95 %), and `All` (solid) is always unreliable. FD-32 (f) accepts the second. Whether `Background` should judge the subject mask instead is a question for the specifier.
- Colour read as stored for `reliable` (premultiplied bytes decoded to linear, weighted by alpha), consistent with ARQ-130: the examples "white alpha 0.25 and black alpha 0.75 gives L 57.08" hold with straight-looking `f32` pixels `[1,1,1,0.25]` and `[0,0,0,0.75]`.
- GMM regularisation is 400, not "about 25" (PROPUESTA, tuned on the gate; see above). The `* size` weighting of the unary terms in `cut.rs` makes gamma almost inert; left as is.
- Two commits outside the delivery's files: the root `Cargo.toml` dev profile (1444da9) and tests touched in `tests/hints.rs` (the stroke test counted 5 chunks of 256 rows, now 17 of 64).
- Q-48: CAL-06, CAL-26 and ARQ-43 hold on this machine; nothing goes to the owner.
- Disk: the sweep and benches used the worktree `target`; scratch scripts live in the session scratch folder `sg3sweep`.

## SG3 fix round after verify round 1 (ARQ-131, ARQ-132, ARQ-133; spec commit e04ea34)

Commits: fd4b99f (scenes), 8ca7fb4 (cut), 8d0a3b5 (reliable, warning, W5, W6).
Measured: `git diff --shortstat 3ff2279..HEAD`: 18 files, 1586 insertions, 494 deletions, spec commit e04ea34 included; `src` only, 944 and 405. Over the ~700 line mark: the coordinator decides.
Gates, each exit code alone: fd4b99f fmt 0, clippy 0, test 0 (1812 passed); 8d0a3b5 fmt 0, clippy 0, test 0 (1826 passed). 8ca7fb4: fmt 0, test 0 (1824 passed), clippy 101 on a type in `tests/pipeline.rs`; after the type alias, clippy 0 and the `pipeline` test file re-run. The full test gate was not repeated on that exact tree.

### Quality, 4000x3000, zone Foreground
- Gate (table of 02 §4.17): 16/16, min 0.9679. IoU 01 to 16: 1.0000, 1.0000, 1.0000, 1.0000, 1.0000, 1.0000, 0.9997, 1.0000, 1.0000, 1.0000, 1.0000, 0.9999, 1.0000, 0.9679, 1.0000, 1.0000.
- Fresh set, after the last parameter change: **11/12**. IoU: 0.9965, 0.9751, **0.0000 fresh-texture-frame-soft (dE76 7 to 8)**, 1.0000, 0.9988, 0.9933, 0.9995, 1.0000, 0.9930, 0.9881, 0.9972, 1.0000. Caution: the recipes are mine and their subjects are flat like the gate's, so this count is easier than the two below and is not comparable with 9 of 12.
- XP5 fresh, the verifier's port: **9/12**, the D-45 figure. F7, F9 and F12, all textured, give an empty mask, not reliable.
- Verifier's hard set: **22/24**, min 0.5315. V07, hue gradient at dE76 14, hard and soft: IoU 0.53, foreground 28.8 %, reported reliable.
- Nothing was tuned on these three sets. The verifier's logs on them, with their counts per parameter pair, were in hand before this round, so the choice of reg 25 and gamma 500 was not blind to them; ARQ-133 fixes both values anyway.

### Parameters
- Regularisation 25, gamma 500, inside ARQ-133. On the XP5 gate, gamma 500, 1500 and 5000 all gave 16/16.
- Band width: min side / 64, unchanged. /64, /37 (about 2 % of the long side) and /32 gave identical IoU on all 16 gate scenes, so the gate gives no reason to change it.

### Border-touching subject: a rule the spec does not name
The band pin alone gave an empty mask on all 8 border scenes tried: the subject's colours in the band train the background model. Pins are unchanged, as ARQ-133 requires. Band colours are grouped by nearness (dE76 6, 4 neighbours); the largest group is the background; a smaller group under half of the band, and isolated colours, stay out of the background GMM (`strays` in `cut.rs`). A subject on an edge, a corner or a whole side now keeps its inner part (`arq_133_a_subject_touching_the_border_does_not_vanish`). Chosen on border scenes of mine, not on the three sets. Limits: a subject over half of the band still vanishes, not reliable, and a seed recovers it. A low-contrast subject touching the border was not tried with the final rule. A background of two colours whose smaller one is under half of the band can now go to the foreground: not measured, no such scene exists. Specifier to record or reject.

### RED (failing line, on the code before the fix)
- Scenes, stub `Recipe` on the old generator: `arq_132_the_fixed_set_is_the_table_of_the_spec` `scene 01 left: (Flat, [], [], Centred, 0.0) right: (Flat, [Ellipse(0.667, 0.5, 0.34, 0.25)], [], Centred, 40.0)`; `arq_132_each_set_has_every_class` `concave`; `arq_132_shape_classes_hold_in_the_masks` `flat-ring-centred: hole left: true right: false`; `arq_132_fresh_scenes_cover_the_whole_range` `small subject`; `arq_132_texture_is_per_pixel` `textured-ellipse-centred: 0.0832`; `arq_132_no_fresh_mask_is_a_fixed_mask` `flat-ellipse-low-contrast and flat-ellipse-centred: 0.9600`.
- Cut: `arq_133_parameters_are_in_their_ranges` `regularisation 400`; `arq_133_twice_the_border_is_twice_the_capacity` `0 7: 8000 8000`; `arq_133_a_longer_border_pulls_the_cut` `long on the background`; `arq_133_pinned_nodes_stay_background_unless_seeded` `pinned red node`; `arq_133_a_subject_touching_the_border_does_not_vanish` `left edge: kept 0.0000`.
- Reliable: `arq_131_background_gives_the_reliable_of_foreground` `2 %: Background`.
- Green from the start: `arq_131_area_counts_pixels_not_alpha` (H5 kills it); `arq_133_half_of_the_pixels_in_the_band_pins`, `arq_133_all_pinned_gives_no_foreground`, `arq_133_all_pinned_is_ok_empty_and_unreliable`, `arq_133_a_seed_on_the_band_leaves_it_in_the_foreground` (no mutant of their own). `arq_133_a_gradient_does_not_swallow_the_image` also passed on the old code, so it proves nothing about this fix.
- Written with the plumbing, not before it: `arq_133_a_pair_counts_its_insertions`, `arq_133_the_shared_border_counts_pixel_pairs`.
- C6: no test. A plain `cargo test` log now holds `WARNING: fresh scenes at IoU >= 0.90: 11/12 (a warning, not a gate)`. No `ci.yml` change.

### Hand mutants (applied, run, reverted, none committed)
H1 `arq_132_the_fixed_set_is_the_table_of_the_spec`, and with every gradient gone `arq_132_each_set_has_every_class`; H2 `arq_132_each_set_has_every_class`, `arq_132_shape_classes_hold_in_the_masks`; H3 `arq_132_low_contrast_is_flat_on_both_sides`; H4 `arq_131_background_gives_the_reliable_of_foreground`; H5 `reliable::tests::arq_131_area_counts_pixels_not_alpha`; H6 `arq_112_progress_and_cancellation_on_the_reference_input`; border weight off `arq_133_a_longer_border_pulls_the_cut`, `arq_133_twice_the_border_is_twice_the_capacity`; band pin off `arq_133_pinned_nodes_stay_background_unless_seeded`; stray rule off `arq_133_a_subject_touching_the_border_does_not_vanish`. The mutant run covered only the test files named; each was edited in place, so no stale build.

### Budgets, indicative (i7-12700H under load, not D-03; Q-48 stays open)
- CAL-06: `bench_segment_4000` 1.02 s on 4 threads. Worst input on 1 thread: 1784 ms in a quieter run, **3468 ms** in a run sharing the machine with this round's own builds. Round 1 measured 1055 ms. Margin to 5 s: 2.8x, or 1.4x contended.
- ARQ-43: `bench_cancel_segment` 27.9 ms. Worst gap on 1 thread: 25.4 ms quieter, **96.8 ms** contended, against 100 ms. Round 1 measured 43.2 ms. 4 threads: 30 ms or less.
- CAL-26: heap peak 34.7, 72.8, 130.3, 302.3 MB for 1, 4, 8, 20 threads, unchanged. Segmenter's share only.

### Deviations and notes
- `Scene` lost `low_contrast` and `textured`; they are on `Recipe`, with the class fields. The 16 per-scene area tests became one test over both sets at 4000x3000.
- `ram_bytes` states its limit (W5) and does not scale: 44 MB against 96 MB measured at 8000x6000.
- The fresh test runs in every gate; its count was first read after the last parameter change, then read again from the same deterministic run for the per-scene figures.
- Scenes repeat run to run. `powf` and the Lab conversions remain, so identity across platforms is not claimed. Shapes and texture use a polynomial sine and T5, not `sin` and `atan2`.
- Sweep and S3, S5 not done: owner's order to finish.
