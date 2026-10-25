# Apply progress: engine-pathops (C4)

Flow: full | pack: new-capability-or-crate | tier: T3 | verify: V3 | decided-by: owner | reason: new crate and a new dependency edge, so ADR 0001 floors the mode at full

## D1: crate skeleton, path model, error type

Status: done. Base: `origin/main` e95e56d. Branch `feat/engine-pathops-1`.

Commits: `cd2c44c` (crate skeleton, `clippy.toml` copy, V5 row, `Path`,
`PathOpError`, validation).

### Commits and gates

Every commit passed alone: `cargo fmt --check`, clippy `--workspace --all-targets
--locked -D warnings`, `cargo test --workspace --locked`, `cargo deny check`
(advisories, bans, licenses, sources ok), and the `ci.yml` steps ARQ-07, V5, V12,
V13 plus the forbid-unsafe step, extracted from the file and run under
`bash --noprofile --norc -eo pipefail` (all rc 0).

### Scenario to test

| Scenario | Evidence |
|---|---|
| Dependency set (command) | V5 rc 0 with the `gafira-pathops` row; `cargo tree -p gafira-pathops` filtered for `gafira-*` gives exactly `gafira-pathops` and `gafira-geom` |
| No document or edit dependency (command) | the same filter finds no `gafira-doc`, `gafira-edit`, `gafira-color`, `gafira-time` or `gafira-anim` |
| Clock call rejected (integration) | scratch `Instant::now()` in `crates/gafira-pathops/tests/`: clippy fails with `disallowed_methods`, note ARQ-02; scratch file removed |
| HashMap use rejected (integration) | scratch `std::collections::HashMap` binding: clippy fails with `disallowed_types` and ARQ-10; scratch file removed |
| Path carries subpaths and a fill rule (unit) | `path_model::a_path_carries_its_subpaths_and_fill_rule` |
| Fill rule set is exactly two variants (unit) | `path_model::the_fill_rule_set_is_exactly_two_variants` (the match is exhaustive, so a new variant breaks the build) |
| Points come from gafira-geom (unit) | `path_model::the_points_are_the_geom_points` |
| V5 row exists (command) | V5 rc 0; the row `gafira-pathops gafira-geom` is in `ci.yml` |
| Clippy copies stay in sync (command) | V12 rc 0 with the new copy |
| Workspace gates pass (command) | fmt, clippy and test exit 0 |

### RED evidence

- Path model (stub that compiled and returned wrong values): 7 tests failed with
  assertions, not with compile errors: `a_path_carries_its_subpaths_and_fill_rule`
  (`left: 0, right: 2`), `an_empty_path_is_empty`, `closing_subpaths_closes_the_open_one`
  (`left: 0, right: 2`), `a_non_finite_coordinate_is_rejected` (`left: Ok(()), right:
  Err(NonFiniteCoordinate)`), `an_infinite_coordinate_is_rejected`,
  `an_open_subpath_is_rejected_with_its_index` (`left: Ok(()), right:
  Err(OpenSubpath { subpath: 0 })`), `the_second_open_subpath_is_the_one_reported`.

### Deviations and notes

- The error type carries `NonFiniteValue` (an enum: `Nan`, `Infinity`,
  `NegativeInfinity`) instead of a raw `f64`. A raw NaN is not equal to itself, so
  an error holding one could not be compared, and `PathOpError` must be
  `PartialEq`. The same reason applies to `InvalidTolerance` and
  `InvalidStrokeWidth`.
- `gafira-pathops` depends on `kurbo` in addition to `gafira-geom`: the crate
  re-exports the path types the boolean library and kurbo's own stroke and
  simplify operate on. `kurbo` is not a new dependency (it is already in the
  workspace and in `Cargo.lock` at 0.13.1) and the V5 row constrains only
  `gafira-*` edges.

## D2: path boolean operations

Status: done. Commit: `9a760e0`.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Union of overlapping rectangles (unit) | `boolean::the_union_of_two_overlapping_rectangles_spans_both` |
| Intersection of overlapping rectangles (unit) | `boolean::the_intersection_of_two_overlapping_rectangles_spans_the_overlap` |
| Difference removes the second shape (unit) | `boolean::the_difference_removes_the_second_shape` |
| Xor is symmetric (unit) | `boolean::the_exclusive_or_is_symmetric` |
| Curves stay curves (unit) | `boolean::curves_stay_curves_through_a_boolean_operation` |
| Union with an empty path (unit) | `boolean::the_union_with_an_empty_path_is_the_other_path` |
| Union of two empty paths (unit) | `boolean::the_union_of_two_empty_paths_is_empty` |
| Set identities hold (property) | `prop_boolean::the_difference_is_contained_in_its_first_operand`, `prop_boolean::the_exclusive_or_spans_the_same_box_as_the_union`, `prop_boolean::the_exclusive_or_is_symmetric`, `prop_boolean::the_intersection_is_contained_in_both_operands` |
| Non-finite coordinate rejected (unit) | `boolean::a_non_finite_coordinate_is_rejected_before_the_operation` |
| Open subpath rejected (unit) | `boolean::an_open_subpath_is_rejected_before_the_operation` |
| Self-intersecting path accepted (unit) | `boolean::a_self_intersecting_path_is_accepted` |
| Zero-length segment accepted (unit) | `boolean::a_zero_length_segment_is_accepted` |
| Error leaves no partial result (unit) | covered by the two tests above: the error carries no path data |
| Repeated runs agree (property) | `boolean::the_same_inputs_give_the_same_result_every_time` (100 runs) and `prop_boolean::the_operation_is_deterministic` |

### RED evidence

- Boolean operations (stub that returned the first operand unchanged): 7 tests
  failed with assertions: `the_union_of_two_overlapping_rectangles_spans_both`
  (`max_x was 10`), `the_intersection_of_two_overlapping_rectangles_spans_the_overlap`
  (`min_x was 0`), `the_difference_removes_the_second_shape`,
  `the_exclusive_or_is_symmetric` (`left: Path { bez: ... }` vs `right: Path { bez: ... }`),
  `a_non_finite_coordinate_is_rejected_before_the_operation`,
  `an_open_subpath_is_rejected_before_the_operation`,
  `an_invalid_tolerance_is_rejected`.

### Deviations and notes

- The tolerance check lives in `validate_tolerance`, shared by the boolean, the
  stroke, the simplification and the hit testing. It rejects a tolerance that is
  not a finite positive number, so the geometry engine never receives a zero
  tolerance.
- The property `the_intersection_is_contained_in_both_operands` and
  `the_difference_is_contained_in_its_first_operand` skip the empty result: an
  empty path reports the origin box, which is not inside the operand's box. This
  was a real defect in the first version of the property: it failed randomly
  (see the flakiness note below) and was fixed before the D2 commit.

## D3: stroke outlining and path simplification

Status: done. Commits: `754099b` (spec amendment), `f198f6a` (implementation).

### Scenario to test

| Scenario | Evidence |
|---|---|
| Stroke of a straight line is a closed capsule (unit) | `stroke_and_simplify::the_outline_of_a_straight_line_is_a_closed_capsule` |
| Stroke of an open path is closed (unit) | `stroke_and_simplify::the_outline_of_an_open_path_is_closed` |
| Stroke width bounds the outline (unit) | `stroke_and_simplify::the_outline_is_wider_than_the_line_by_the_stroke_width` and `::a_wider_stroke_gives_a_wider_outline` |
| Degenerate stroke style rejected (unit) | `stroke_and_simplify::a_degenerate_stroke_width_is_rejected` |
| Redundant point removed (unit) | `stroke_and_simplify::a_redundant_point_is_removed_by_the_simplification` |
| Subpath structure preserved (unit) | `stroke_and_simplify::the_simplification_keeps_the_subpath_structure` |
| Already minimal path is unchanged (unit) | `stroke_and_simplify::an_already_minimal_path_is_unchanged_by_the_simplification` |
| Deviation stays within tolerance (property) | `prop_stroke::the_simplification_stays_within_the_tolerance` |

### RED evidence

- Stroke and simplification (stub that returned the path unchanged): 8 tests
  failed with assertions: `the_outline_of_a_straight_line_is_a_closed_capsule`
  (`assertion failed: outline.is_subpath_closed(0)`),
  `the_outline_of_an_open_path_is_closed`,
  `the_outline_is_wider_than_the_line_by_the_stroke_width`,
  `a_wider_stroke_gives_a_wider_outline`,
  `a_degenerate_stroke_width_is_rejected`,
  `an_invalid_tolerance_is_rejected_by_the_stroke`,
  `an_invalid_tolerance_is_rejected_by_the_simplification`,
  `a_redundant_point_is_removed_by_the_simplification`.

### Deviations and notes

- **The spec was amended before the implementation commit**, in its own
  `docs(spec)` commit `754099b`. The requirement said the simplified path "MUST
  NOT contain more segments than the original". That is false against
  `kurbo::simplify::simplify_bezpath`, which fits curves to the source: a
  three-element polyline came back as five elements (measured). The requirement
  now states the guarantee the crate can give, that the result stays within the
  tolerance and that an already minimal path comes back unchanged.
- **Ratified by specifier** (after verify round 1, finding C1). The amendment in
  `754099b` is confirmed: the simplification MAY replace straight segments with
  fitted curves and MAY return more segments than it received; the guarantee is
  about shape (every point of the original within the tolerance of the result),
  not about the segment count. The verifier's probes agree (an open polyline
  gains elements, a circle goes from 17 elements to 6). The specifier also
  defined "already minimal", which the amendment left open: a path of straight
  segments only, every subpath closed, every vertex farther than the tolerance
  from the line through its neighbours. Such a path comes back equal in value;
  open polylines and curved paths carry only the tolerance guarantee. See
  `specs/path-operations/spec.md`, "Path simplification".
- A path with no segment strokes to an empty outline (measured with kurbo
  0.13.1): there is nothing to stroke, so there is no area to cover. The
  property test skips that case.
- The outline of a closed shape has two subpaths: the outer boundary and the
  inner one.

## D4: geometric hit testing

Status: done. Commit: `15f4b30`.

### Scenario to test

| Scenario | Evidence |
|---|---|
| Point inside a rectangle (unit) | `hittest::a_point_inside_a_rectangle_is_inside` |
| Point outside a rectangle (unit) | `hittest::a_point_outside_a_rectangle_is_outside` |
| Fill rule changes the answer (unit) | `hittest::the_fill_rule_changes_the_answer` |
| Distance to perimeter (unit) | `hittest::the_distance_to_the_perimeter_of_a_circle_is_the_gap` |
| Distance is zero inside (unit) | `hittest::the_distance_from_the_centre_of_a_circle_is_the_radius` and `::the_distance_to_the_perimeter_of_a_rectangle` |
| Containment agrees with the boolean result (property) | `prop_hittest::the_containment_agrees_with_the_union` |
| Point on the perimeter (unit) | `hittest::a_point_on_the_perimeter_is_on_the_boundary` |

### RED evidence

- Hit testing (stub that answered `Outside` and distance `0.0`): 9 tests failed
  with assertions: `a_point_inside_a_rectangle_is_inside`,
  `a_point_on_the_perimeter_is_on_the_boundary`,
  `the_fill_rule_changes_the_answer` (`left: Outside, right: Inside`),
  `the_distance_to_the_perimeter_of_a_circle_is_the_gap`,
  `the_distance_from_the_centre_of_a_circle_is_the_radius`,
  `the_distance_to_the_perimeter_of_a_rectangle`,
  `an_invalid_tolerance_is_rejected_by_the_containment_test`,
  `an_invalid_tolerance_is_rejected_by_the_distance_test`,
  `a_path_with_a_non_finite_coordinate_is_rejected`.

### Deviations and notes

- The containment test uses the winding number of the path around the point
  (`kurbo::Shape::winding`), so it answers with the same fill rule the boolean
  operations use.
- A point within the tolerance of the perimeter is `OnBoundary`, an answer of
  its own, so a caller can tell an edge hit from a fill hit.

## Owner decisions after verify round 1 (2026-10-25)

Recorded by the specifier; the delta spec carries them in "Owner decisions".

| # | Question | Owner decision |
|---|---|---|
| OQ1 (C3) | Which fill rule applies when the two operands of a boolean operation carry different ones | **A**: each operand is read with its own fill rule. The explorer measured that the approved library's public topology API does this exactly (0 wrong samples, all four operations) at the same cost as the current call. |
| OQ1 follow-up | Which fill rule the result of a boolean between non-empty operands carries | **A**: always `NonZero`, independent of operand order, the same rule as stroke outlines. A clone returned for an empty operand keeps the surviving operand's own rule. |
| OQ2 (W5) | Is a path holding only moves empty | **A**: yes. It holds no segment and covers no area, so it takes the empty-operand short circuit; a subpath without segments is never reported as an open subpath. |

## Mutation testing (T3)

Tier T3: `cargo mutants -p gafira-pathops --file <each changed source file>`.
Each run used its own copy of the tree and its own `target` directory, so no
`CARGO_TARGET_DIR` was shared between mutated trees, as AGENTS.md requires.

| File | Mutants | Caught | Unviable | Missed | Wall time |
|---|---|---|---|---|---|
| `src/path.rs` | 21 | 18 | 3 | 0 | 29 s |
| `src/error.rs` | 6 | 5 | 1 | 0 | 27 s |
| `src/boolean.rs` | 11 | 6 | 5 | 0 | 27 s |
| `src/stroke.rs` | 12 | 11 | 1 | 0 | 28 s |
| `src/simplify.rs` | 6 | 5 | 1 | 0 | 29 s |
| `src/hittest.rs` | 17 | 15 | 1 | 1 (equivalent) | 29 s |
| `src/lib.rs` | 0 | 0 | 0 | 0 | — |
| **Total** | **73** | **60** | **12** | **1 (equivalent)** | **~3 min** |

### The one survivor is an equivalent mutant

`src/hittest.rs:69:25: replace < with <= in Path::distance_to_perimeter`.

The code is `if distance < closest { closest = distance; }`, which keeps the
running minimum. With `<=`, the assignment happens when the distance equals the
current minimum, which stores the same value. A NaN distance is skipped by both
comparisons. The two versions return the same result for every input, so no
test can kill the mutant: it is equivalent, not a gap in the tests.

The repository excludes equivalent mutants through `.cargo/mutants.toml`, with a
reason. That file is outside the surfaces this change may touch, so the
exclusion is left to the archive step, which owns the whole-crate T4 run.

### Mutants that were caught by tests added after the first run

The first T3 run reported survivors that the tests killed by hand. Two causes,
both fixed:

1. **A shared `CARGO_TARGET_DIR` between mutated trees.** The runs that set
   `CARGO_TARGET_DIR` reused one directory for every copy of the tree, so cargo
   reused a stale build and the mutant was never compiled. The runs above give
   each mutated tree its own `target` directory, and the logs show the mutated
   tree being recompiled for every mutant.
2. **A flaky property test.** `prop_boolean::the_difference_is_contained_in_its_first_operand`
   failed when the difference was empty, because an empty path reports the
   origin box. The property now skips the empty result. The property tests were
   run 30 times in a row afterwards with no failure.

The two survivors that were real test gaps are now covered:

| Mutant | Test that kills it |
|---|---|
| `path.rs:90:24: delete ! in Path::close_subpaths` | `path_model::closing_a_path_with_two_open_subpaths_closes_both` |
| `path.rs:90:32: replace && with \|\| in Path::close_subpaths` | `path_model::closing_does_not_add_elements_to_a_closed_subpath` |
| `error.rs:21:9: replace <impl Display for NonFiniteValue>::fmt with Ok(())` | `error::each_non_finite_value_names_itself` and `error::the_error_names_the_kind_of_non_finite_value` |

Each was verified by hand: the mutation is applied to the source, the test
fails, the source is restored.

## Measured size

`git diff --shortstat origin/main...HEAD -- crates .github Cargo.toml`: 18 files
changed, 2138 insertions. The forecast for the change was about 1 800 lines
across four deliveries of 400 to 500 lines each (`design.md`).

## Next

`verify` (V3) and, at the archive, the whole-crate T4 run. This run stops before
both, as instructed.

## Correction to an earlier statement (append-only, verify fix round 1)

The D2 section above says the flaky property in
`prop_boolean::the_difference_is_contained_in_its_first_operand` "was fixed before
the D2 commit", and the mutation section says it was fixed by the time the T3 run
was made. Both statements are false. Verify round 1 (finding C8) measured the
defect in the commits `9a760e0`, `754099b`, `f198f6a` and `15f4b30` (fresh failure
rates of 9/40 and 6/40, and a failed gate run at `754099b`); the fix landed only in
`a8d2c0a`, after D4. So those four commits do not pass `cargo test --workspace
--locked` reliably.

History is not rewritten. The owner has not yet ruled on a waiver for C8; the
coordinator will ask. This round does not claim the finding closed.

### Owner decision on C8 (2026-10-25): waived

The owner waived finding C8, following the `gafira-time` C2 precedent:

- History is not rewritten. The commits `9a760e0`, `754099b`, `f198f6a` and
  `15f4b30` keep the flaky property; the fix stays in `a8d2c0a`.
- The final tree is correct: the property passed 30 consecutive runs without a
  failure after `a8d2c0a`.
- Consequence: no stacked PR may be cut at a commit boundary before `a8d2c0a`.
  The change is delivered as ONE PR.

The correction above stays as written; this decision closes C8 as waived, not as
fixed.

## Verify fix round 1

Base: `origin/main` merged first (`ec97f7b`: `gafira-time`, edit-history D1-D10 and
V14 came in; the only conflict was the `docs/specs/README.md` row, resolved by
keeping both sides). Gates after the merge: fmt, clippy, test, deny and the CI
policy steps ARQ-07, forbid-unsafe, V5, V12, V13, V14 (extracted from `ci.yml`), all
rc 0. Every commit below passed the same gates alone. Commits use the repository
identity and carry no AI attribution; test changes live with the fix they test and
no docs file is edited inside a test or fix commit.

### Commits

| Group | Finding | Commit |
|---|---|---|
| 1 | C7, OQ2 | `e3b9cbf` fix: clone of the surviving operand for an empty operand |
| 2 | C3, OQ1 | `9c0d95d` fix: each operand read with its own fill rule |
| 3 | C5 | `dcb06fa` fix: `ToleranceTooFine` below the safe minimum |
| 4 | C6 | `24cab46` fix: stroke outline carries `NonZero` |
| 5 | C9, HM4, W1, W4 | `f4438cd` fix: not-positive errors, one shared validation, input-validation tests |
| 6, 8 | W2, W3, test 18 | `fd81282` test: stronger tests and generators, false xor-box property removed |
| 7 | W5 | `8b467c5` docs: measured tolerance, real guarantees |
| - | mutant follow-up | `17a58c0` test: winding of every result, error messages |

### Design notes

- `PathOpError::InvalidTolerance` and `InvalidStrokeWidth` now carry
  `cause: InvalidNumber`, an enum with `NotPositive(f64)` (zero, `-0.0`, negative:
  the offending number) and `NonFinite(NonFiniteValue)`. `ToleranceTooFine
  { minimum }` is new. The old field `value: NonFiniteValue` is gone: this is an
  API change inside the unreleased crate.
- Boolean order of checks: tolerance, finite (A then B), open (A then B), too fine,
  then the empty-operand rule, then the sweep. The sweep uses
  `Topology::<BinaryWindingNumber>::from_paths_binary(..).contours(..)` with
  `rule_a.is_inside(w.shape_a)` and `rule_b.is_inside(w.shape_b)`.
- `Path::is_empty` now means "holds no segment" (moves and closes are not
  segments); `validate_closed` no longer reports a subpath without a segment.
- The four copies of the tolerance check are one (`validate.rs`); the unit tests of
  those private helpers are removed and the behaviour is tested through the public
  API in `tests/validation.rs` (suggestion S1).

### RED evidence per group

Each failing run was made before the code of its group; the assertion text is the
one printed.

1. C7, OQ2 (`tests/boolean.rs`): `the_union_with_an_empty_path_returns_a_clone`
   (left: rotated start point `MoveTo((0.0, 10.0)), LineTo((0.0, 0.0))...`, right:
   the input rectangle), `the_surviving_operand_keeps_its_own_fill_rule` (left:
   `fill_rule: NonZero` and a filled ring, right: `EvenOdd`),
   `a_moves_only_path_is_empty_in_either_operand_position` (left:
   `Err(OpenSubpath { subpath: 0 })`, right: the ring),
   `a_moves_only_path_covers_no_point` (left: `Err(OpenSubpath { subpath: 0 })`,
   right: `Ok(Outside)`), `difference_and_xor_with_an_empty_path_return_a_clone`,
   `an_empty_operand_with_mixed_rules_keeps_the_survivors_rule`. The two
   `path_model` tests added with it
   (`a_path_of_moves_only_is_empty_and_has_no_open_subpath`,
   `a_path_with_a_segment_is_not_empty`) were written after the code and have no RED
   run of their own; the boolean tests above are the RED for the same behaviour.
2. C3, OQ1: `mixed_rules_never_cover_a_point_neither_operand_covers` (left:
   `[true, true, true]`, right: `[false, true, true]`),
   `intersection_difference_and_xor_read_each_operand_with_its_own_rule` (left:
   `[true, true, false]`, right: `[false, true, false]`),
   `the_rule_of_each_operand_is_used_in_both_directions` (left: `[true, true]`,
   right: `[false, true]`), `the_result_of_non_empty_operands_carries_non_zero`
   (left: `EvenOdd`, right: `NonZero`, "Union"), and the property
   `prop_boolean::the_point_by_point_definition_holds_for_rectangles` (minimal
   failing input shrunk; the case is kept in `prop_boolean.proptest-regressions`).
3. C5: `a_tolerance_below_the_minimum_is_rejected_and_never_aborts` (the debug
   panic `assertion failed: solve_x_for_y(cubic0.c, y0) > solve_x_for_y(cubic1.c,
   y0)` at `linesweeper-0.5.0/src/curve/mod.rs:1301`; in release the same call
   aborts the process, not run here), `the_minimum_applies_to_straight_operands_too`
   (left: `Ok(Path ...)`, right: `Err(ToleranceTooFine { minimum: 1.5e-11 })`),
   `the_minimum_is_checked_before_the_empty_operand_rule` (right:
   `Err(ToleranceTooFine { minimum: 1e-11 })`),
   `the_minimum_is_computed_from_every_point_of_both_operands` (right:
   `Err(ToleranceTooFine { minimum: 4e-11 })`). The variant was added first so the
   tests compile.
4. C6: `the_outline_carries_non_zero_whatever_the_input_rule` (left: `EvenOdd`,
   right: `NonZero`), `crossing_strokes_stay_solid` and
   `a_self_crossing_polyline_stays_solid` (left: `Ok(Outside)`, right:
   `Ok(Inside)`).
5. C9/HM4, W1, W4 (`tests/validation.rs`): with the validation stubbed to the old
   behaviour (a non-positive number named NaN), `a_non_positive_tolerance_and_width_are_named_as_such`,
   `an_invalid_tolerance_is_reported_before_the_path_defects` (left:
   `Err(InvalidTolerance { cause: NonFinite(Nan) })`, right: `Err(InvalidTolerance {
   cause: NotPositive(0.0) })`) and `an_invalid_width_is_reported_before_the_path_defects`
   failed. HM4 re-run: with `validate_finite` deleted from `simplify`,
   `the_simplification_of_a_non_finite_path_returns_an_error` did not finish within a
   60 s timeout (the hang); with the guard restored it passes in milliseconds.
   The tests for the non-finite checks of every operation, the stroke of an infinite
   path, the non-finite second operand and the second-operand failure without a
   partial result pass against the code that already had the guards: they close
   missing evidence and were validated by the mutants above, not by a RED run.
6. W2/W3, test 18 (`fd81282`): these tests pin behaviour the code already had, so
   there is no failing run; they were validated by a hand mutant: scaling the
   tolerance passed to `simplify_bezpath` by 1000 makes
   `prop_stroke::the_simplification_stays_within_the_tolerance` fail ("a point moved
   84.5 away, over the tolerance 0.84"). The two scenarios without evidence now have
   tests: `the_outline_of_a_line_is_half_the_stroke_width_away` and
   `every_operation_takes_the_tolerance_as_an_argument`. The false property
   `the_exclusive_or_spans_the_same_box_as_the_union` is replaced by
   `the_boxes_of_the_results_are_contained` (xor box inside the union box).
7. W5: documentation only (the scenario is manual). Searched afterwards for
   "bounds the", "never holds" and "fill rule of the first": no match left except the
   stroke outline, whose tolerance guarantee the spec keeps.

### Flaky-prone properties

`cargo test -p gafira-pathops --test prop_boolean --test prop_hittest --test
prop_stroke` was run 30 times in a row after the last source change: 30 passes, 0
failures, no regression file created. One extra run with `PROPTEST_CASES=3000`
also passed.

### Mutation (T3), per changed non-test source file

`cargo mutants -p gafira-pathops --file <file> --jobs 2` on a clean `git archive`
export of the commit, one export and one default `target` per file (no shared
`CARGO_TARGET_DIR`); the logs show `Compiling gafira-pathops` for every mutant.

| File | Mutants | Caught | Unviable | Missed |
|---|---|---|---|---|
| `src/boolean.rs` | 18 | 11 | 7 | 0 |
| `src/error.rs` | 7 | 6 | 1 | 0 |
| `src/hittest.rs` | 12 | 10 | 1 | 1 (the known equivalent, `hittest.rs:69:25`) |
| `src/path.rs` | 28 | 25 | 3 | 0 |
| `src/stroke.rs` | 2 | 1 | 1 | 0 |
| `src/simplify.rs` | 1 | 0 | 1 | 0 |
| `src/validate.rs` | 8 | 8 | 0 | 0 |
| `src/lib.rs` | 0 | 0 | 0 | 0 |

Two misses on the first run were real gaps and were fixed test-first in `17a58c0`:

- `boolean.rs`: `Xor => in_a != in_b` to `==`. The mutant keeps every boundary but
  flips its winding, so coverage under either fill rule is identical and the point
  properties cannot see it. The behaviour that differs is the winding of the
  result: `every_operation_winds_the_result_the_same_way` (area `-100` against the
  union's winding `+100`).
- `error.rs`: `Display for InvalidNumber` replaced by an empty message. The spec says
  the error "says the value is not positive and states the offending number", so the
  test now checks the message names the cause and the number
  (`a_non_positive_tolerance_and_width_are_named_as_such`,
  `a_non_finite_tolerance_and_width_are_named_by_kind`).

`simplify.rs` has a single generated mutant (the whole function body), unviable.
The finiteness guard is a statement, which cargo-mutants does not delete, so it was
checked by the hand mutant HM4 above.

`.cargo/mutants.toml` is unchanged: the exclusion of the equivalent mutant
`hittest.rs:69:25` is added at archive, as before.

### Not addressed in this round

- W5, drawing after a `ClosePath` with no `MoveTo` is still treated as closed by the
  subpath model and reaches the library as a "non-closed path" error. The task
  listed only the documentation items of W5; the spec does not define the case.
- C8 stays as described in the correction above.

### Measured size

`git diff --shortstat ec97f7b..HEAD` (the fix round only, the merge excluded, before
this docs commit): 16 files changed, 1262 insertions, 385 deletions (net +877; 8
source files: 255 insertions, 213 deletions). This is past the 700-line ceiling and
past the roughly 1 000 changed lines the task named: the tests rewritten for the new
error shape and the new generators account for most of it. The coordinator decides.

## Open items for the archive notes

- Add the exclusion of the equivalent mutant `hittest.rs:69:25` (`<` to `<=` in
  `Path::distance_to_perimeter`) to `.cargo/mutants.toml`, with its
  justification: `Point::distance` never returns `-0.0`, so equal distances are
  bit-identical and assigning an equal value changes nothing; a NaN fails both
  comparisons; an infinite distance with an infinite running minimum stores the
  same value. No input distinguishes the two versions (proof in
  `verify-report.md`, round 1, "Mutation").
