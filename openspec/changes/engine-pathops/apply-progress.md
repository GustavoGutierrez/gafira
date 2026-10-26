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

## Verify fix round 2 (owner-authorized 2026-10-25)

Scope: verify round 2 findings C1, C2, W2. W1 (huge coordinates) waits for a spec
decision and was not implemented; W4 (`.cargo/mutants.toml`) stays for archive.
`origin/main` was merged first (`ea1b17f`); no conflict.

Commits:

- `aecd57d` fix(pathops): fit the arcs of round caps and joins at the stroke tolerance (C1)
- `b7e00ca` fix(pathops): treat drawing after a close as an open subpath (C2)
- `b711725` test(pathops): cover both stroke directions and untouched curve offsets
- `baa1a0f` refactor(pathops): derive the sweep of a round arc from its middle point

### C1 round caps and joins exceed the tolerance

RED (`tests/stroke_accuracy.rs`, samples the outline and compares each point with
the exact boundary at half the width; widths 0.1..1000, tolerances 1e-1..1e-5):

- `a_round_cap_lies_within_the_tolerance_of_the_exact_cap`: "width 0.1, tolerance
  0.00001: the cap deviates by 0.000013341929513722983".
- `a_round_join_lies_within_the_tolerance_of_the_exact_join`: same message for the
  join (closed square and triangle, both directions; only the outer contour is
  compared, because kurbo's inner join cuts across the interior of the stroke and is
  not a boundary of the covered area).

Deviation: the suggested fix (pass `tolerance x min(1, 2/width)` to kurbo) does not
work. kurbo 0.13.1 hardcodes an internal tolerance of `1e-3` on the unit circle for
round caps and joins (`stroke.rs` `finish` and `do_join`, marked `TODO: scale`), so
the tolerance argument never reaches the arcs (verified: the output element count
stayed at 8 for every tolerance). The fix post-processes the outline: each cubic
whose end points and middle point lie at half the width from a vertex of the input
path is replaced by an arc of the same circle and sweep, fitted by
`kurbo::Arc::to_cubic_beziers` at the caller's tolerance. Offset curves of curved
segments are untouched (tests with a curved and a looped centreline).

Measured worst deviation / tolerance after the fix (line with round caps, widths
0.1..1000, tolerances 1e-1..1e-7): at most 0.87 (before: up to 1.36e6). Cost, in
outline elements for the capsule (was 8 everywhere): 8 at tolerance 1e-1..1e-3 for
widths up to 10, 12 for width 1000 at 1e-1, 16 at width 10 and 1e-5, 24 at width
1000 and 1e-5, and 48 at width 1000 and 1e-7. `DEFAULT_TOLERANCE` and
`stroke_outline` docs now say the arcs are fitted at the tolerance.

### C2 drawing after a close without a move

RED (`tests/validation.rs`):

- `drawing_after_a_close_without_a_move_is_an_open_subpath`: `left: Ok(())`, `right:
  Err(OpenSubpath { subpath: 1 })` on `validate_closed`.
- `closing_the_subpaths_closes_the_tail_drawn_after_a_close`: `left: 1`, `right: 2`
  (`subpath_count`).

The segments after a `ClosePath` now start a new subpath (SVG and kurbo convention),
so `validate_closed`, `contains` and the four boolean operations return
`OpenSubpath { subpath: 1 }` for the example. The spec text covers `contains` ("an
operation that depends on the path's interior (the four boolean operations and the
containment test) MUST reject an open subpath"), so nothing was decided. The index
`1` follows the specifier suggestion S1 of the verify report; the spec text does not
pin it, so the specifier should confirm it. The stroke, the simplification and the
distance accept such a path (test).

### W2 release evidence for the too-fine tolerance

`a_tolerance_below_the_minimum_is_rejected_and_never_aborts` (tests/boolean.rs) is
profile independent. Commands (run from the workspace root):

- `cargo test -p gafira-pathops --release --locked --test boolean a_tolerance_below_the_minimum`
  passes with the validation in place (1 passed).
- With the check disabled by hand (`if false && tolerance < minimum`, reverted
  afterwards), the same command in release aborts: "thread ... has overflowed its
  stack / fatal runtime error: stack overflow, aborting ... (signal: 6, SIGABRT)";
  in debug it fails on `linesweeper curve/mod.rs:1301` `assertion failed`.
- `cargo test -p gafira-pathops --release --locked` (whole crate) passes.

Recommended gate note: run the release command above next to `cargo test
--workspace --locked`.

### Gates and mutation

Each commit: `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace
--locked`, `cargo deny check` (bans, licenses, sources; advisories) and the CI policy
steps (ARQ-07, forbid-unsafe, V5, V12, V13, V14) all returned 0.

Flaky-prone properties (`prop_boolean`, `prop_hittest`, `prop_stroke`): 30 runs in a
row, 0 failures, before and after the last commit.

Mutation T3 (`--file`, `--jobs 4`, default per-job target dirs, `Compiling
gafira-pathops` confirmed on every mutated tree):

- `stroke.rs`: first run 39 mutants, 9 missed (the negative sweep branch, the
  middle-point checks). Closed test-first with reversed shapes, a curved centreline,
  a looped curve and a short curve; one equivalent mutant (`+ PI` / `- PI` inside a
  `rem_euclid(TAU)`) was removed by rewriting the angle as `atan2(cross, dot)`.
  Final: 25 mutants, 18 caught, 7 unviable, 0 missed.
- `path.rs`: 32 mutants, 29 caught, 3 unviable, 0 missed.
- `lib.rs` changed only in a doc comment.

### Measured size

`git diff --shortstat ea1b17f..HEAD` before this docs commit: 5 files changed, 356
insertions, 9 deletions (source: `stroke.rs` +76 / -3, `path.rs` +14 / -5, `lib.rs`
+2 / -1; tests: `stroke_accuracy.rs` new, 213 lines, `validation.rs` +51).

## Owner decision after verify round 2 (2026-10-25)

Recorded by the specifier; the delta spec carries it in "Owner decisions" and in
"Degenerate and invalid inputs".

| # | Question | Owner decision |
|---|---|---|
| W1 | How to bound coordinate magnitude: the simplification never returned for finite coordinates of `1e154` or more, and the distance to the perimeter returned infinity from `1e155` | **A**: every operation rejects a path with a coordinate whose absolute value exceeds `1e9`, with a named error carrying the value and the limit (suggested name `CoordinateOutOfRange { value, limit }`); and the simplification also rejects a tolerance below `1e-12 × m` with the existing too-fine-tolerance error, because it also hung at ordinary magnitudes (tolerance `1e-9` on a path near `1e7`). The error order now has six steps: tolerance, stroke width, non-finite, beyond the limit, open subpath, too fine. |

The same spec commit also states two clarifications from the verify rounds: a
segment drawn after a close with no new move starts the next subpath in order of
appearance (round 2 S1; the implementation already reports index 1), and the
simplification keeps a subpath made only of zero-length segments (it came back
with 0 elements).

## Fix round 3 (owner decisions W1 and W2, 2026-10-25 and 2026-10-26)

Scope: implement the coordinate limit, the simplification too-fine rule, the
zero-length-subpath survival and the drawing-after-close evidence from spec commit
`38ee579`, plus the owner decisions W2 of 2026-10-26 (all A) recorded in the spec
commit `61fb23a` (single-purpose `docs`): the boolean minimum becomes `1e-11 x m`; the
stroke outline gets a too-fine rule `1e-12 x max(m, w / 2)`; the query point of
`contains` and `distance_to_perimeter` obeys the coordinate limit and the non-finite
rule, checked after the path checks; no too-fine rule for the hit tests; the
simplification keeps `1e-12 x m`. The error order has eight steps now.

Commits (each passed fmt, clippy `-D warnings`, `cargo test --workspace --locked`,
the crate in `--release`, `cargo deny check` and the CI policy steps ARQ-07,
forbid-unsafe, V5, V12, V13, V14, all rc 0):

- `61fb23a` docs(engine-pathops): spec for the floors and the query point
- `21131b1` feat(pathops): coordinate limit `COORDINATE_LIMIT = 1e9`,
  `PathOpError::CoordinateOutOfRange { value, limit }`, all eight operations
- `02e4ff5` feat(pathops): simplification too-fine rule `1e-12 x m`
- `c19eba8` feat(pathops): boolean minimum `1e-11 x m`
- `ac68d6e` feat(pathops): stroke minimum `1e-12 x max(m, w / 2)`
- `3839e6e` feat(pathops): query point checks
- `cecfa67` fix(pathops): zero-length subpath survives the simplification
- `eefaf1e` docs(pathops): minimums in `DEFAULT_TOLERANCE`, design "Minimum tolerance"

### RED evidence (failing assertion, then GREEN)

| Criterion | Test | RED |
|---|---|---|
| Limit rejected by every operation, in either position | `limits::a_coordinate_beyond_the_limit_is_rejected_by_every_operation` | `is_out_of_range` false: the call returned `Ok` for `next_up(1e9)` (end point, control point, move) |
| First offending coordinate in path order | `limits::the_first_offending_coordinate_in_path_order_is_reported` | assertion false (`Ok`) |
| Overflow cases return an error | `limits::the_overflow_cases_return_an_error_instead_of_hanging` | assertion false for `1e154` and `1e155` |
| Out of range before open subpath; moves-only beyond limit | `limits::a_coordinate_beyond_the_limit_is_reported_before_an_open_subpath`, `limits::a_moves_only_operand_beyond_the_limit_is_rejected` | assertion false (open-subpath error, a clone of the rectangle) |
| Limit inclusive; non-finite before out of range | `limits::a_coordinate_of_exactly_the_limit_is_accepted`, `limits::a_non_finite_coordinate_is_reported_before_one_beyond_the_limit` | pin behaviour that already held (the stub accepted everything and the old code reported the right non-finite error); they guard the order |
| Simplification too fine | `tolerance_floor::the_simplification_accepts_exactly_the_minimum_tolerance` | `left: Ok(())`, `right: Err(ToleranceTooFine { minimum: 4e-12 })` |
| Simplification never hangs (spec case, `1e-9` on a path near `1e7`) | `tolerance_floor::a_simplification_tolerance_below_the_minimum_is_rejected_and_never_hangs` | RED is a hang: `timeout 40` ended it (rc 124) before the rule; a test cannot guard a hang without a thread (ARQ-02 bans threads), same as HM4 |
| Boolean minimum `1e-11 x m` | `boolean::the_minimum_applies_to_straight_operands_too` | `left: Err(ToleranceTooFine { minimum: 1.5e-10 })`, `right: Err(ToleranceTooFine { minimum: 1.5e-11 })` after updating to the new floor; also `a_boolean_tolerance_is_accepted_exactly_at_the_minimum`, `the_default_tolerance_is_below_the_minimum_for_large_operands` (`left: Ok`, `right: Err(ToleranceTooFine { minimum: 0.002 })`) |
| Stroke minimum | `tolerance_floor::a_stroke_tolerance_is_checked_against_the_larger_of_the_extent_and_half_the_width`, `a_stroke_of_a_path_at_the_origin_still_has_a_minimum_tolerance` | `left: Ok(())`, `right: Err(ToleranceTooFine { minimum: 1e-10 })` and `{ minimum: 4e-12 }` |
| Too fine after out of range (simplification, stroke) | `tolerance_floor::a_too_fine_*_is_reported_after_a_coordinate_beyond_the_limit` | pin the order (the new rule sits after the range check); they pass at the first run because the range check landed first |
| Query point rejected | `limits::a_query_point_that_is_not_finite_or_beyond_the_limit_is_rejected` | `left: Ok(())`, `right: Err(NonFiniteCoordinate { value: Nan })` |
| Query point after the path, non-finite first | `limits::the_query_point_is_checked_after_the_path_and_non_finite_first` | `left: Ok(())`, `right: Err(NonFiniteCoordinate { value: Nan })` |
| No too-fine rule for the hit tests; point at the limit accepted | `limits::the_hit_tests_have_no_too_fine_rule`, `limits::a_query_point_at_exactly_the_limit_is_accepted` | pin behaviour that already held |
| Zero-length subpath kept | `stroke_and_simplify::a_subpath_of_zero_length_segments_is_kept_by_the_simplification` | `left: 0`, `right: 1` (`subpath_count`) |

Evidence row for the drawing-after-close index (spec "Drawing after a close without a
move is the next subpath"): `validation::drawing_after_a_close_without_a_move_is_an_open_subpath`
asserts `OpenSubpath { subpath: 1 }` from `validate_closed`, `contains` and the four
boolean operations in both operand positions; it was written in round 2 and passes
unchanged. No new test was needed.

### Existing tests that changed

Only `tests/boolean.rs`: the expected minimums moved from `1e-12 x m` to `1e-11 x m`
(six assertions). No existing test used a coordinate above `1e9`, or a stroke or
simplification tolerance below its new floor.

### Design of the code

`validate::not_too_fine(tolerance, per_unit, extent)` is shared by the three rules;
`Path::coordinates()` is the one iterator behind `validate_finite`, `validate_in_range`
and `max_abs_coordinate`; `validate::query_point` checks the point of the hit tests.
`simplify` now splits the path before each move and passes a subpath that holds only
zero-length segments through unchanged, because the underlying simplifier drops it.

### Deviations and decisions to confirm

1. The inclusive-limit scenario in the spec asked every call to use the default
   tolerance; with the boolean floor at `1e-11` the minimum at `m = 1e9` is `1e-2`, so
   the spec commit changes the boolean calls of that scenario to the minimum and adds the
   scenario "The default tolerance is below the boolean minimum for large operands".
   This follows arithmetically from the owner's choice; the owner may want to know that
   the default tolerance no longer works for boolean operations above `m = 1e8`.
2. The too-fine error for the stroke is the existing `ToleranceTooFine { minimum }`; it
   does not carry the offending tolerance (the caller holds it), because the earlier
   rule for the boolean operations and the simplification does the same.
3. The query point is checked after the open-subpath check in `contains` (step 7 and 8
   of the order), so an open path with a bad point reports the open subpath.
4. The spec line "stroke has no too-fine rule" named in the task does not exist in the
   committed spec; the stroke rule was added as new text.
5. Mutation T3 was not run in this round (no `mutants` report); `verify` or `mutants`
   should cover `validate.rs`, `path.rs`, `simplify.rs`, `stroke.rs`, `hittest.rs` and
   `boolean.rs`.
6. `verify-report.md` and `verification.json` carry uncommitted verifier edits in this
   worktree; they were left untouched and are not part of these commits.

### Measured size

`git diff --shortstat 38ee579..HEAD` (the previous round's last commit to here): 14
files changed, 931 insertions, 91 deletions (net +840). The crate code alone
(`git diff --shortstat 61fb23a..HEAD -- crates`): 12 files, 824 insertions, 63
deletions, of which `src/` is 260 insertions and 55 deletions and the rest is tests
(`limits.rs` 329 lines and `tolerance_floor.rs` 143 lines are new). This is past the
~700-line ceiling; the coordinator decides.

## Fix round 4 (final, owner-authorized)

Items from the coordinator's final-round brief; decisions were made upstream and not reopened.

### Done

- Docs commit of the verifier's round 3 edits (`verify-report.md`, `verification.json`).
- W1 spec (docs commit first): the simplification drops a subpath made only of a move;
  the requirement and a new scenario say so, and the implicit-move rule is stated for the
  simplification. Code and tests agree (`a_subpath_made_only_of_a_move_is_dropped`).
- C1: `simplify` now works on the path with its implicit moves written out
  (`Path::with_explicit_moves`), so a segment drawn after a close is simplified from the
  start of the closed subpath. A zero-length tail comes back as `MoveTo(0,0)`,
  `LineTo(0,0)` instead of `MoveTo(10,10)`, `LineTo(0,0)`.
- Sweep for siblings, audited operation by operation:
  - simplify: fixed (above). The same input also made `kurbo::simplify_bezpath` panic in
    debug builds (`uninitialized subpath`) on a degenerate closed subpath followed by a
    tail; that goes away because every slice now starts with its move.
  - boolean: the sweep library gave a different result for the same geometry with and
    without the written-out move on zero-area operands (a sliver survived). `boolean_op`
    now feeds the sweep the explicit-move paths, so both read the same.
  - stroke, distance to the perimeter, containment, `validate_closed`, `close_subpaths`:
    already read the implicit move correctly (tested against the explicit form on
    generated paths; no change needed).
  - A path that starts with a segment and no move at all is not a valid `BezPath`
    (kurbo debug-asserts); `with_explicit_moves` starts it at the origin, and
    `close_subpaths` does not close it (no start point). Not part of the spec; reported.
- New shared generator and properties in `tests/after_close.rs`: a closed polygon, then a
  tail with or without a `MoveTo`, closed or not, checked for simplify, stroke outline,
  distance, the four boolean operations, containment, `close_subpaths` and the
  open-subpath error index.
- W2: the docs of union, intersection, difference and xor state the `1e9` limit,
  `CoordinateOutOfRange` and the `1e-11 * m` minimum.
- W3 is void (the T3 report exists). Mutants re-run on `simplify.rs`, `path.rs` and
  `boolean.rs` (see below); the old `simplify.rs:70:38` mutant disappeared with the
  rewrite of `subpaths`.
- W5: `.cargo/mutants.toml` excludes `hittest.rs` `distance_to_perimeter` `<` to `<=`
  (tie-break between equally near segments gives the same distance); `--list` confirms
  it no longer appears and the `==` and `>` mutants still do.

### RED evidence

- `a_tail_after_a_close_is_simplified_from_the_start_of_the_closed_subpath`: failed
  before the fix.
- `a_zero_length_tail_after_a_close_stays_at_the_start_of_the_closed_subpath`: left
  `... MoveTo(10,10), LineTo(0,0)`, right `... MoveTo(0,0), LineTo(0,0)`.
- `the_simplification_reads_the_implicit_move`: panic `uninitialized subpath (missing
  MoveTo)` in kurbo, then a value mismatch on the implicit versus the written-out form.
- `the_interior_operations_read_the_implicit_move`: union of the closed-tail path differed
  from union of the written-out path (a zero-area sliver in one result only).
- `a_subpath_made_only_of_a_move_is_dropped`: already green on the old code (the
  simplifier dropped it); the gap was only the spec, now written.

### Process notes

- C2: spec commit `61fb23a` was made by `tdd-apply` on the coordinator's routing; its
  content was ratified by owner decision A (2026-10-26), so no implementer-authored spec
  change remains unratified. This round's W1 spec edit is ratified by the owner's
  instruction to close the change.
- Size: the whole change is past ~700 lines; the owner approved a single PR for it.

### Mutants (T3, no `--locked`, default per-job dirs)

- `simplify.rs`, `path.rs`, `boolean.rs` together: 64 mutants, 51 caught, 12 unviable,
  1 timeout (`simplify.rs` `delete !` in the moves-only filter of `subpaths`). The
  mutation changed no behavior: the simplifier already returns a moves-only slice empty,
  so the filter was redundant and was removed (refactor commit).
- `simplify.rs` after that: 7 mutants, 5 caught, 2 unviable, 0 missed, 0 timeouts.
  `path.rs` and `boolean.rs` are unchanged since the first run (0 missed there).
- The old `simplify.rs:70:38` mutant no longer exists (the `subpaths` rewrite removed the
  `index > 0` filter); the first slice of a path starting with a move is covered by
  every simplify test.

### Measured size (round 4)

`git diff --shortstat 7814c10..HEAD -- crates .cargo`: 5 files, 319 insertions, 13
deletions (most of it the new `tests/after_close.rs`). Whole change against `main`:
35 files, 7744 insertions, 1 deletion (crates only: 23 files, 4510 insertions). Past the
~700-line ceiling; the owner approved a single PR for it.

### Not settled

- A path whose first element is a segment (no `MoveTo` at all) is not a valid `BezPath`;
  `close_subpaths` leaves it open. The spec is silent; no change made.

## Closing pass (verify round 4 findings, owner asked to close the change)

- Verifier edits (`verify-report.md`, `verification.json`, round 4) committed as docs.
- C1 (class implementation), strict TDD:
  - RED: `tests/empty_subpaths.rs`
    `a_simplification_never_starts_with_a_close_and_holds_no_empty_subpath` panicked in
    kurbo (`uninitialized subpath (missing MoveTo)`) on `M Z`; after the first fix it
    failed on `square Z Z` (`left` ended `ClosePath, ClosePath`, `right` one close).
  - GREEN: `Path::simplify` skips a subpath that holds no segment (`is_segment`, now
    `pub(crate)`) and collapses a close right after a close before calling kurbo.
  - Sweep, per operation, on `M Z`, `M Z M Z`, square + `M Z`, `M Z` + square, mixes,
    `Z Z`, `M Z` + tail and `Z Z` + tail, each compared with the same path without the
    empty subpaths: `validate_closed`, `close_subpaths`, `contains`,
    `distance_to_perimeter`, `is_empty`, the four boolean operations (both positions),
    `stroke_outline`, `simplify`, and the simplified output reused in every operation.
    Only `simplify` was broken; the other operations already passed (no change).
  - Generators: `tests/after_close.rs` `Shape` now emits `M Z` before and after the
    polygon; `tests/empty_subpaths.rs` has a generated property over squares, `M Z` and
    bare `Z` in any order. Debug and release both green.
  - Mutants (`--test-workspace=true`, no `--locked`, per-job dirs). First run at `--jobs 4`
    had timeouts from machine load (load average 7 to 10, 20 s auto timeout, also on
    mutants unrelated to the change); every test binary runs in under 1 s on a mutated
    `simplify.rs`, so I re-ran with `--jobs 2 --minimum-test-timeout 120`:
    `simplify.rs` 8 mutants, 6 caught, 2 unviable, 0 missed, 0 timeouts;
    `path.rs` 43 mutants, 40 caught, 3 unviable, 0 missed, 0 timeouts.
- W2/W1: `origin/main` merged (README status row: main's rows kept, `engine-pathops`
  added as active in the 02 row). The `workspace-tooling` delta now holds only the two
  MODIFIED requirements (V5 table and the clippy policy) as they stand on main, plus the
  `gafira-pathops` row, note, copy and scenarios; `gafira-time`, N1-N5, H-N1 and the
  BTreeMap scenario are kept.
- S2: the simplify rule reads "holds no segment (moves and closes only)" in the delta
  spec. No scenario was added for `M Z`; the requirement text covers it and the tests
  above cite it. The coordinator may add one if wanted.
- S5: `a_closed_tail_after_a_close_unions_as_its_written_out_form` (fixed unit test). It
  fails with the `boolean_op` fix reverted (`assertion left == right`) and passes now.
  The shape that fails is not a zero-area sliver: without the written-out move the
  sweep read the tail from another start point; a zero-area tail agreed in every
  variant I tried.
- S1: accepted. The `OpenSubpath` message does not point to `close_subpaths` (only the
  variant doc does); the real gap is a path with no initial move, already under "Not
  settled".
- S3, S4: accepted as they are (message wording when the query point is at fault;
  `distance_to_perimeter` of an empty path is `+inf`, undefined by the spec).
- C2 (process): spec commit `f89f883` was made on the coordinator's routing and is
  ratified by the owner's instruction to close the change (2026-10-26).
- Spec commits `951e777` (workspace-tooling rebase onto main) and `5f9ead0` (simplify
  wording) are likewise ratified by the owner's instruction to close the change
  (2026-10-26), as done for `f89f883` and `61fb23a`.
- Measured size: `git diff --shortstat 7814c10..HEAD -- crates .cargo`: 6 files, 685
  insertions, 16 deletions. Whole change against `origin/main`: 36 files, 8648 insertions, 1 deletion. Past the ~700-line ceiling; the owner approved a single PR.
