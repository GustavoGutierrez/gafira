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
