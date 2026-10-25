# Path Operations Specification (delta for C4 `engine-pathops`)

## Purpose

C4 adds the layer-2 engine crate `gafira-pathops`: path boolean operations (union,
intersection, difference, xor), stroke outlining, path simplification and geometric
hit testing. The crate does not paint pixels. Its internal dependency is
`gafira-geom` alone; its other normal dependencies are `kurbo` (the geometry
vocabulary `gafira-geom` already speaks, `design.md` Q6) and the boolean-operations
library approved in `design.md` Q1. It never depends on `gafira-doc`, `gafira-edit`
or any adapter.

Scope: the WHAT. The HOW (API shape, tolerance policy, error types, library
choice) lives in `design.md`. Decisions Q1-Q7 are closed there (see "Closed
decisions" below); a requirement that depends on one names it ("Q1", "Q2"...).
Decisions still open for the owner are listed under "Open questions" at the end,
and the requirements they affect say so. Scenario types: unit, property,
integration, command, manual.

Terms used below:

- **Empty path**: a path with no elements at all. (Whether a path holding only
  moves or closes also counts as empty is open question OQ2.)
- **Covers**: a path covers a point when its containment test answers `Inside`
  for that point under the path's own fill rule.
- **Equal in value**: two paths are equal in value when they hold the same
  sequence of elements with the same coordinates and the same fill rule.

## ADDED Requirements

### Requirement: Crate purity and dependencies (ARQ-01, ARQ-02, ARQ-03) (D1)

`gafira-pathops` MUST be a layer-2 engine crate: `#![forbid(unsafe_code)]`, a
byte-identical copy of the canonical `clippy.toml`, and direct normal dependencies
limited to `gafira-geom`, `kurbo` (Q6), the approved boolean-operations library
(Q1) and the standard library. Its only `gafira-*` dependency MUST be
`gafira-geom`. It MUST NOT read clocks, the environment, files, the network or
spawn threads (ARQ-02), and MUST NOT use `HashMap` or `HashSet` (ARQ-10 ban). It
MUST NOT depend on `gafira-doc`, `gafira-edit`, `gafira-color`, `gafira-time`,
`gafira-anim` or any adapter. It MUST NOT paint pixels: it returns geometry, never
raster data.

#### Scenario: Internal dependency set (D1, command)
- GIVEN the tree after D1
- WHEN `cargo tree -p gafira-pathops -e normal,build --target all --prefix none -f '{p}'` is filtered for `gafira-*`
- THEN the set of crate names is exactly `gafira-pathops` and `gafira-geom`, and the V5 step exits 0

#### Scenario: Direct dependency set (D1, command)
- GIVEN the tree after D1
- WHEN `cargo tree -p gafira-pathops -e normal --depth 1 --prefix none -f '{p}'` runs
- THEN the crate names below the root line are exactly `gafira-geom`, `kurbo` and the approved boolean-operations library (`linesweeper`, Q1)

#### Scenario: No document or edit dependency (D1, command)
- GIVEN the tree after D1
- WHEN the same tree filter is searched for `gafira-doc`, `gafira-edit`, `gafira-color`, `gafira-time` and `gafira-anim`
- THEN no match is found

#### Scenario: Clock call rejected (D1, integration)
- GIVEN a scratch `std::time::Instant::now()` call in a `gafira-pathops` test
- WHEN `cargo clippy -p gafira-pathops --all-targets -- -D warnings` runs
- THEN it fails citing `disallowed_methods` and ARQ-02

#### Scenario: HashMap use rejected (D1, integration)
- GIVEN a scratch `let _m: std::collections::HashMap<u8, u8> = Default::default();` in a `gafira-pathops` test
- WHEN `cargo clippy -p gafira-pathops --all-targets -- -D warnings` runs
- THEN it fails citing `disallowed_types` and ARQ-10

### Requirement: Path model (D1)

`gafira-pathops` MUST expose a path type built on the `kurbo` primitives that
`gafira-geom` re-exports (`Point`, `Vec2`, `Affine`, `Rect`), plus the Bézier path
and stroke style types it needs. The crate MUST NOT define a second, incompatible
point or vector type. A path MUST be able to carry several subpaths, each of which
MAY be open or closed, and MUST carry the fill rule that describes its interior.

The fill rule MUST have exactly the variants `EvenOdd` and `NonZero`, with the same
meaning as in the vector-graphics convention: `EvenOdd` counts crossings and
alternates, `NonZero` accumulates winding direction.

#### Scenario: Path carries subpaths and a fill rule (D1, unit)
- GIVEN a path built from two subpaths, one closed and one open
- WHEN its subpath count, each subpath's closedness and its fill rule are read
- THEN they report 2 subpaths, the first closed and the second open, and the fill rule as set

#### Scenario: Fill rule set is exactly two variants (D1, unit)
- GIVEN the fill rule type
- WHEN its variants are enumerated
- THEN exactly `EvenOdd` and `NonZero` exist

#### Scenario: Points come from gafira-geom (D1, unit)
- GIVEN a path point
- WHEN it is compared with a `gafira_geom::Point` built from the same coordinates
- THEN the two are equal, and the path crate defines no separate point type

### Requirement: Boolean operations (D2)

The crate MUST provide union, intersection, difference and exclusive-or over two
paths, and MUST return the result as a path. The result MUST preserve the Bézier
structure of the input: a boolean operation over a curved input MUST NOT degrade it
to a polyline (Q1).

Each operation is defined point by point, from which operand covers the point:

- a point is in `A ∪ B` when A covers it or B covers it;
- a point is in `A ∩ B` when A covers it and B covers it;
- a point is in `A \ B` when A covers it and B does not;
- a point is in `A Δ B` (xor) when exactly one of A and B covers it.

Consequences that MUST hold: `A \ B` is contained in `A`; `A ∩ B` is contained in
both operands; no point is covered by both `A \ B` and `B \ A`; `A Δ B` covers
exactly the points that `A \ B` or `B \ A` covers. The bounding boxes of `A \ B`
and `B \ A` MAY overlap (two crossing bars give two non-empty differences whose
boxes overlap), and the bounding box of `A Δ B` MAY be smaller than the bounding
box of `A ∪ B` (when one operand contains the other along a side).

The definitions above hold when both operands carry the same fill rule. **PENDING
owner decision OQ1**: what an operation does when the two operands carry different
fill rules. Until OQ1 is closed, no test may assert a result for mixed fill rules.

**PENDING explorer (C4)**: the distance from the perimeter of either operand
within which a point is exempt from the point-by-point definition (the geometric
bound of a boolean result). Until it is measured, the point-by-point property is
stated only for operands built from axis-aligned rectangles, with a margin equal to
the tolerance.

Empty operands:

- `A ∪ ∅` and `∅ ∪ A` MUST return a path equal in value to `A`;
- `A \ ∅` MUST return a path equal in value to `A`;
- `A Δ ∅` and `∅ Δ A` MUST return a path equal in value to `A`;
- `A ∩ ∅`, `∅ ∩ A` and `∅ \ A` MUST return an empty path;
- any operation over two empty paths MUST return an empty path.

"Equal in value" means the operand is not re-segmented, its start point is not
moved and no element is added: an empty operand contributes nothing to compute.

#### Scenario: Union of overlapping rectangles (D2, unit)
- GIVEN a 10x10 rectangle at the origin and a 10x10 rectangle offset by 5 on the x axis
- WHEN their union is computed
- THEN the result is a single closed contour whose bounding box spans x from 0 to 15 and y from 0 to 10

#### Scenario: Intersection of overlapping rectangles (D2, unit)
- GIVEN the same two rectangles
- WHEN their intersection is computed
- THEN the result is a single closed contour whose bounding box spans x from 5 to 10 and y from 0 to 10

#### Scenario: Difference removes the second shape (D2, unit)
- GIVEN the same two rectangles
- WHEN the difference of the first by the second is computed
- THEN the result is a single closed contour whose bounding box spans x from 0 to 5 and y from 0 to 10

#### Scenario: Xor is symmetric (D2, unit)
- GIVEN the same two rectangles
- WHEN the exclusive-or is computed in both operand orders
- THEN both results are equal

#### Scenario: Curves stay curves (D2, unit)
- GIVEN a circle path and a rectangle path
- WHEN their intersection is computed
- THEN the result contains at least one cubic Bézier segment, and no operation turns a curved input into a polyline

#### Scenario: Union with an empty path returns the operand (D2, unit)
- GIVEN a rectangle path R, a circle path C, and an empty path E, all with the `NonZero` fill rule
- WHEN `R ∪ E`, `E ∪ R`, `C ∪ E` and `E ∪ C` are computed
- THEN the first two results are equal in value to R and the last two are equal in value to C (same elements in the same order, same start point, same fill rule)

#### Scenario: Difference and xor with an empty path return the operand (D2, unit)
- GIVEN the rectangle path R and the empty path E
- WHEN `R \ E`, `R Δ E` and `E Δ R` are computed
- THEN each result is equal in value to R

#### Scenario: Intersection with an empty path is empty (D2, unit)
- GIVEN the rectangle path R and the empty path E
- WHEN `R ∩ E`, `E ∩ R` and `E \ R` are computed
- THEN each result is an empty path with no contour

#### Scenario: Operations on two empty paths are empty (D2, unit)
- GIVEN two empty paths
- WHEN their union, intersection, difference and xor are computed
- THEN each result is empty and contains no contour

#### Scenario: Crossing bars have overlapping difference boxes (D2, unit)
- GIVEN A, the rectangle from (0,-10) to (2,10), and B, the rectangle from (-10,0) to (10,2)
- WHEN `A \ B` and `B \ A` are computed
- THEN both are non-empty, their bounding boxes overlap by an area of 4, and the point (1,1) is covered by neither

#### Scenario: Xor box can be smaller than the union box (D2, unit)
- GIVEN A, the rectangle from (0,0) to (10,10), and B, the rectangle from (0,0) to (10,5)
- WHEN `A Δ B` and `A ∪ B` are computed
- THEN the bounding box of the xor spans y from 5 to 10 and the bounding box of the union spans y from 0 to 10

#### Scenario: Box containment holds (D2, property)
- GIVEN two generated well-formed paths with the same fill rule
- WHEN the four operations are computed
- THEN, for every result that is not empty: the bounding box of `A \ B` is contained in the bounding box of `A`; the bounding box of `A ∩ B` is contained in the intersection of the two operand boxes; the bounding box of `A Δ B` is contained in the bounding box of `A ∪ B`; each within the tolerance

#### Scenario: Point-by-point definition holds for rectangles (D2, property)
- GIVEN two generated paths, each made of one to three axis-aligned rectangles, with the same fill rule, and a generated point farther than the tolerance from the perimeter of both operands
- WHEN the four operations are computed and the point's containment is tested in each operand and each result
- THEN each result covers the point exactly when the definition above says it does, and no point is covered by both `A \ B` and `B \ A`

### Requirement: Degenerate and invalid inputs (D2, D3)

The crate MUST define the behaviour of an operation over inputs that are not well
formed. The cases are: a path with an open subpath; a path with a subpath that
crosses itself; a path with a zero-length segment; a path with a non-finite
coordinate; a tolerance or stroke width that is not a finite positive number.

Every operation (the four boolean operations, the stroke outline, the
simplification, the containment test and the distance to the perimeter) MUST
reject a path with a non-finite coordinate with an error that names the kind of
non-finite value (NaN, positive infinity or negative infinity), before any
geometric computation (Q2). The operation MUST return that error; it MUST NOT loop,
abort the process or return a result.

An operation that depends on the path's interior (the four boolean operations and
the containment test) MUST reject an open subpath with an error that names the
index of the open subpath, counted from zero in path order. Such an operation MUST
NOT close the subpath itself; a caller closes open subpaths explicitly with a
separate operation first (Q3). The stroke outline, the simplification and the
distance to the perimeter MUST accept open subpaths.

A self-intersecting path and a path with a zero-length segment MUST be accepted and
MUST NOT be reported as an error.

A tolerance or a stroke width that is zero or negative MUST be rejected with an
error that says the value is not positive and states the offending number. It MUST
NOT be reported as NaN or as any other non-finite value. A tolerance or a stroke
width that is NaN or infinite MUST be rejected with an error that names the kind of
non-finite value. The error for a tolerance and the error for a stroke width MUST be
distinguishable.

When an input has more than one defect, the reported error MUST follow this order,
first match wins:

1. an invalid tolerance;
2. an invalid stroke width (stroke outline only);
3. a non-finite coordinate in the first operand, then in the second operand;
4. an open subpath in the first operand, then in the second operand (interior
   operations only).

An error MUST leave no partial result: the operation returns either a complete
result or an error, never a truncated path, and the inputs are unchanged.

#### Scenario: Non-finite coordinate rejected by every operation (D2, D3, D4, unit)
- GIVEN three closed paths, holding respectively a NaN, a positive infinite and a negative infinite coordinate, and a valid rectangle path
- WHEN each one is passed, in either operand position, to union, intersection, difference and xor, and alone to the stroke outline, the simplification, the containment test and the distance to the perimeter
- THEN every call returns the non-finite-coordinate error naming NaN, positive infinity or negative infinity respectively, and no result

#### Scenario: Simplification of a non-finite path returns an error (D3, unit)
- GIVEN an open polyline whose second point has a NaN x coordinate
- WHEN it is simplified with tolerance 1e-3
- THEN the call returns the non-finite-coordinate error naming NaN (it returns; it does not loop)

#### Scenario: Stroke of a non-finite path returns an error (D3, unit)
- GIVEN an open line whose end point has an infinite y coordinate
- WHEN its outline is computed with a valid style and tolerance
- THEN the call returns the non-finite-coordinate error naming positive infinity

#### Scenario: Open subpath rejected (D2, unit)
- GIVEN a path whose only subpath is not closed
- WHEN its union with a rectangle is computed, in either operand position
- THEN it returns the open-subpath error naming subpath 0

#### Scenario: Non-finite reported before open subpath (D2, unit)
- GIVEN a path whose only subpath is open and holds a NaN coordinate
- WHEN its union with a valid rectangle is computed
- THEN the error is the non-finite-coordinate error naming NaN, not the open-subpath error

#### Scenario: Non-finite in the second operand reported before open subpath in the first (D2, unit)
- GIVEN a first operand that is finite with an open subpath, and a second operand that is closed with an infinite coordinate
- WHEN their intersection is computed
- THEN the error is the non-finite-coordinate error naming positive infinity

#### Scenario: Invalid tolerance reported before path defects (D2, unit)
- GIVEN a path that is open and holds a NaN coordinate
- WHEN its union with a rectangle is computed with tolerance 0
- THEN the error is the invalid-tolerance error

#### Scenario: Non-positive tolerance and width are named as such (D2, D3, unit)
- GIVEN tolerances 0, -0.0 and -1, and stroke widths 0, -0.0 and -1
- WHEN a boolean operation, the stroke outline, the simplification, the containment test and the distance to the perimeter are called with each tolerance, and the stroke outline with each width and a valid tolerance
- THEN each call returns the invalid-tolerance or invalid-stroke-width error, whose cause is "not positive" and which carries the offending number (reading it back gives the input value) and whose message does not contain `NaN`

#### Scenario: Non-finite tolerance and width are named by kind (D2, D3, unit)
- GIVEN tolerances and stroke widths of NaN, positive infinity and negative infinity
- WHEN the same calls are made
- THEN each returns the invalid-tolerance or invalid-stroke-width error naming the kind of non-finite value

#### Scenario: Self-intersecting path accepted (D2, unit)
- GIVEN a path shaped as a bowtie, whose subpath crosses itself
- WHEN its union with a rectangle is computed
- THEN it succeeds and returns a result

#### Scenario: Zero-length segment accepted (D3, unit)
- GIVEN a path whose subpath contains a segment whose start and end coincide
- WHEN its intersection with a rectangle is computed
- THEN it succeeds and returns a result

#### Scenario: Error leaves no partial result (D3, unit)
- GIVEN a valid rectangle path A, a closed path B holding a NaN coordinate, and copies of both taken before the call
- WHEN `A ∪ B` is computed
- THEN the call returns the non-finite-coordinate error, the error carries no path data, and A and B are equal in value to their copies

### Requirement: Stroke outlining (D3)

The crate MUST convert a path plus a stroke style into the outline path that the
stroke covers. The stroke style MUST express the stroke width and the cap and join
behaviour, and the operation MUST take a tolerance.

The stroke width is the full width of the stroke: the stroke extends half the width
on each side of the path. For a straight segment, every point at a distance of at
most half the width from the segment, measured perpendicular to it, is covered by
the outline; a round cap extends the stroke by half the width beyond each end of an
open subpath.

The outline MUST be a closed path. It MUST be closed even when the stroked input is
an open path, because a stroke always encloses an area.

The outline MUST carry the `NonZero` fill rule, whatever fill rule the input path
carries, so that a region the stroke covers more than once (where the path crosses
itself, or where two subpaths cross) is covered by the outline and never appears as
a hole.

#### Scenario: Stroke of a straight line is a closed capsule (D3, unit)
- GIVEN the open path from (0,0) to (100,0) and a stroke style of width 10 with a round cap
- WHEN the outline is computed with tolerance 1e-3
- THEN the result is one closed subpath whose bounding box spans x from -5 to 105 and y from -5 to 5, each bound within the tolerance

#### Scenario: Stroke of an open path is closed (D3, unit)
- GIVEN an open path of three points and a stroke style
- WHEN the outline is computed
- THEN the result's subpaths are all closed

#### Scenario: Stroke width bounds the outline (D3, unit)
- GIVEN the same path and a style whose width is doubled
- WHEN the outline is computed
- THEN the bounding box of the wider outline is wider than the bounding box of the narrower one

#### Scenario: Degenerate stroke style rejected (D3, unit)
- GIVEN a stroke style whose width is zero, negative, NaN or infinite
- WHEN the outline is computed
- THEN each returns an error and no result (the error content is fixed by "Degenerate and invalid inputs")

#### Scenario: Outline fill rule is NonZero (D3, unit)
- GIVEN the open path from (0,0) to (100,0) carrying the `EvenOdd` fill rule
- WHEN its outline is computed with width 10
- THEN the outline carries the `NonZero` fill rule

#### Scenario: Crossing strokes stay solid (D3, unit)
- GIVEN an X path made of two open subpaths, from (0,0) to (10,10) and from (0,10) to (10,0), carrying the `EvenOdd` fill rule, and a stroke of width 2
- WHEN the outline is computed and the containment of the crossing point (5,5) in the outline is tested
- THEN the answer is `Inside`

#### Scenario: Self-crossing polyline stays solid (D3, unit)
- GIVEN one open polyline (0,0), (10,10), (10,0), (0,10) carrying the `EvenOdd` fill rule, whose first and third segments cross at (5,5), and a stroke of width 2
- WHEN the outline is computed and the containment of (5,5) in the outline is tested
- THEN the answer is `Inside`

### Requirement: Path simplification (D3)

The crate MUST simplify a path within a given tolerance: every point of the
original path MUST lie within the tolerance of the simplified path. Simplification
MUST preserve the path's subpath structure: every subpath of the input MUST appear
in the output, in the same order, and the fill rule MUST be kept.

The simplification MAY replace straight segments with fitted curves, and the
simplified path MAY hold more segments than the original. The guarantee is about
shape, not about the count of segments.

A path that is already minimal MUST be returned equal in value to the input. A path
is already minimal, for this requirement, when it is made only of straight
segments, every subpath is closed, and every vertex lies farther than the
tolerance from the straight line through its two neighbours. Paths outside this
definition (open polylines, curved paths) carry only the tolerance guarantee.

#### Scenario: Redundant point removed (D3, unit)
- GIVEN the closed path (0,0), (10,0), (20,0), (20,10), (0,10), whose vertex (10,0) lies on the straight line between its neighbours, and tolerance 1e-3
- WHEN the path is simplified
- THEN the output has fewer segments than the input and every vertex of the input lies within the tolerance of the output

#### Scenario: Subpath structure preserved (D3, unit)
- GIVEN a path with two subpaths
- WHEN it is simplified
- THEN the result still has two subpaths, in the same order, with the input's fill rule

#### Scenario: Already minimal path is unchanged (D3, unit)
- GIVEN a closed square path and a closed triangle path, and tolerance 1e-3
- WHEN each is simplified
- THEN each result is equal in value to its input

#### Scenario: Simplification may add segments (D3, unit)
- GIVEN an open polyline with a corner
- WHEN it is simplified
- THEN the call succeeds and every point of the original lies within the tolerance of the result, whatever the number of segments in the result

#### Scenario: Deviation stays within tolerance (D3, property)
- GIVEN a generated path and a tolerance
- WHEN the path is simplified
- THEN every point sampled along every segment of the original (not only its vertices) lies within the tolerance of the simplified path

### Requirement: Geometric hit testing (D4)

The crate MUST answer whether a point lies inside a path, under the path's fill
rule. It MUST also report the distance from a point to the path's perimeter, which
a caller uses to hit test a stroke. The distance is unsigned: it is the distance to
the nearest point of the perimeter, whether the point is inside or outside.

The containment test MUST use the same fill rule semantics as the boolean
operations, so a point reported inside a path is a point the path's interior covers
under that rule. A point within the tolerance of the perimeter MUST be reported as
on the boundary, a third answer distinct from inside and outside (Q5), and the same
rule MUST apply to every path.

#### Scenario: Point inside a rectangle (D4, unit)
- GIVEN a rectangle path from (0,0) to (10,10)
- WHEN the containment of the point (5,5) is tested
- THEN the point is reported inside

#### Scenario: Point outside a rectangle (D4, unit)
- GIVEN the same rectangle path
- WHEN the containment of the point (15,5) is tested
- THEN the point is reported outside

#### Scenario: Point on the perimeter (D4, unit)
- GIVEN the same rectangle path
- WHEN the containment of the point (10,5) is tested
- THEN the point is reported on the boundary

#### Scenario: Fill rule changes the answer (D4, unit)
- GIVEN a path of two nested squares, both wound in the same direction
- WHEN the containment of a point inside the inner square is tested under `EvenOdd` and under `NonZero`
- THEN the answers differ, and `EvenOdd` reports the point outside

#### Scenario: Distance to perimeter (D4, unit)
- GIVEN a circle path of radius 10 centred at the origin
- WHEN the distance from the point (12,0) to the perimeter is computed
- THEN the result is 2 within the tolerance

#### Scenario: Containment agrees with the boolean result (D4, property)
- GIVEN two generated well-formed paths with the same fill rule, made of axis-aligned rectangles, and a point farther than the tolerance from the perimeter of both operands
- WHEN the point's containment in the union is tested and the union is computed
- THEN the point is reported inside the union exactly when it is inside at least one of the two operands

#### Scenario: Distance from an interior point is the gap to the perimeter (D4, unit)
- GIVEN a circle path of radius 10 centred at the origin
- WHEN the distance from the point (0,0) to the perimeter is computed
- THEN the result is 10 within the tolerance, not 0

### Requirement: Determinism and numeric tolerance (D2, D3, D4)

Every operation MUST be deterministic: the same inputs MUST produce the same
outputs, byte for byte, across runs and across machines. No operation may depend on
hash iteration order, on the system clock or on any ambient state.

Every geometric operation MUST take or use a documented tolerance. The tolerance
MUST be an explicit part of the operation's inputs or a documented default, never an
implicit constant hidden inside a helper. The same tolerance MUST be used
consistently inside one operation, so a single operation never mixes tolerances.

The stroke outline MUST lie within the tolerance of the exact stroke (the w/2
geometry stated in "Stroke outlining"), and the simplification MUST lie within the
tolerance of the original path ("Path simplification").

**PENDING explorer (C4, C5)**: what geometric bound the tolerance gives for a
boolean result, and the smallest tolerance an operation accepts. The previous
statement "a result MUST be within the stated tolerance of the exact geometric
answer" is withdrawn for the boolean operations: it is false (verify-report C4).
No test may assert a boolean deviation bound until this requirement is completed.

#### Scenario: Repeated runs agree (D2, property)
- GIVEN a fixed pair of paths
- WHEN the union is computed 100 times
- THEN all 100 results are equal to each other

#### Scenario: Tolerance is visible in the API (D3, unit)
- GIVEN a valid rectangle path, a valid circle path and a valid stroke style
- WHEN a test calls each of union, intersection, difference, xor, the stroke outline, the simplification, the containment test and the distance to the perimeter, once with an explicit tolerance of 0.5 and once with the crate's documented default tolerance constant
- THEN every call compiles, passes the tolerance as an argument and succeeds

#### Scenario: Stroke outline within tolerance (D3, unit)
- GIVEN the open path from (0,0) to (100,0) and a stroke of width 10
- WHEN the outline is computed with tolerance 1e-3 and the distance from the point (50,0) to the outline's perimeter is computed
- THEN the distance is 5 (half the stroke width) within the tolerance

### Requirement: Workspace integration (D1)

The crate MUST be a member of the cargo workspace and MUST appear in the CI
allowed-dependency table (V5) with `gafira-geom` as its only allowed internal
dependency. Its `clippy.toml` MUST be byte-identical to the canonical copy (V12),
and the V12 step MUST accept the new copy. The crate MUST be reachable in
`cargo test --workspace --locked` and `cargo clippy --workspace --all-targets
--locked -- -D warnings`.

#### Scenario: V5 row exists (D1, command)
- GIVEN the CI allowed-dependency table
- WHEN it is searched for the `gafira-pathops` row
- THEN the row exists and lists `gafira-geom` as its only allowed internal dependency

#### Scenario: Clippy copies stay in sync (D1, command)
- GIVEN the canonical `clippy.toml` and the `gafira-pathops` copy
- WHEN they are compared byte for byte
- THEN they are identical

#### Scenario: Workspace gates pass (D1, command)
- GIVEN the tree after D1
- WHEN `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo test --workspace --locked` run
- THEN all three exit 0

## Non-goals

- Painting pixels, rasterization or any output image.
- Editing the document: `gafira-pathops` neither reads nor writes nodes, pages or
  commands.
- Text shaping, glyph outlines and font handling (`gafira-text`).
- Layout, snapping, guides or grid (`gafira-layout`).
- Importing or exporting file formats (`gafira-io-svg`, `gafira-io-pdf`).
- Adaptive or non-uniform stroking, variable-width strokes, and stroke dashes.
- Boolean operations on text runs or on images.
- Persisting paths to the document format; serialization belongs to `gafira-doc`.
- Multi-operand (n-ary) boolean operations in one call.
- A minimum segment count or a canonical form for simplified paths.

## Closed decisions

Recorded for traceability only; these are not requirements and archive MUST NOT
merge this section into the main spec as requirements. The full record is in
`design.md`.

| # | Decision | Where it constrains this spec |
|---|---|---|
| Q1 | `linesweeper 0.5`, Bézier-native boolean operations | "Boolean operations" (curves stay curves), "Crate purity" |
| Q2 | the crate checks finiteness itself and names the non-finite value | "Degenerate and invalid inputs" |
| Q3 | open subpaths rejected with a named error; a separate explicit close operation | "Degenerate and invalid inputs" |
| Q4 | one crate-wide default tolerance constant, overridable per call (1e-3 px) | "Determinism and numeric tolerance" |
| Q5 | a separate on-boundary answer | "Geometric hit testing" |
| Q6 | a thin wrapper over the kurbo Bézier path; direct `kurbo` dependency | "Crate purity", "Path model" |
| Q7 | the `clippy.toml` copy marks the crate for V5 and V12 | "Workspace integration" |

## Open questions

These belong to the owner. The requirements above mark what they block as PENDING.

1. **OQ1 — Fill rule when the two operands of a boolean operation carry different
   fill rules.** Each path carries its own fill rule, but the spec never said how
   two different rules combine. The implementation applies the first operand's
   rule to both operands, so a union can cover a point that neither operand covers
   (verify-report C3: a `NonZero` rectangle united with `EvenOdd` nested squares
   covers (5,5), which neither operand covers).

   | Option | Consequence |
   |---|---|
   | **A. Each operand is read with its own fill rule (recommended)** | The point-by-point definition holds for every input and the containment test agrees with every boolean result. The result's fill rule must then be one under which the result covers exactly the computed set. Whether the approved library can read two operands with two rules is not yet measured (needs explorer); if it cannot, the crate needs extra work (for example normalizing an operand first), which costs code and delivery budget. |
   | B. Reject operands whose fill rules differ, with a named error | Simple and honest: no silent wrong result. Callers must make the rules match before the call, which a design tool does often (imported SVG paths carry either rule). |
   | C. Keep the first operand's rule for both, documented | No code change, but the result is not the set the operation names; the point-by-point definition and the hit-test agreement must be weakened to "under the first operand's rule". |
   | D. The caller passes one fill rule that applies to both operands | Explicit, but the same semantic problem as C, and it duplicates the rule each path already carries. |

   Recommendation: **A**, with B as the fallback if the explorer finds that the
   library cannot read the operands with different rules.

2. **OQ2 — Is a path holding only moves (no segment) empty?** The empty-operand
   rules above are stated for a path with no elements. A path holding a lone move
   covers no area, yet the boolean operations reject it today as an open subpath
   (verify-report W5).

   | Option | Consequence |
   |---|---|
   | **A. Empty: a path with no segment is empty, and a subpath with no segment is not an open subpath (recommended)** | A path that covers no area behaves as the empty path in every operation; callers that build paths incrementally do not hit a spurious error. |
   | B. Not empty: a lone move is an open subpath and is rejected | Strict; the caller must strip such moves. The "is empty" query and the operations must then agree on this narrower meaning. |

   Recommendation: **A**.
