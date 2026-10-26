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
The owner closed the two questions this spec raised (OQ1, OQ2) on 2026-10-25,
the coordinate-magnitude question of verify round 2 (W1) the same day, and the
tolerance floors of verify round 3 (W2) on 2026-10-26; they are recorded under
"Owner decisions" at the end. Scenario types: unit, property,
integration, command, manual.

Terms used below:

- **Segment**: a line, quadratic or cubic element. Moves and closes are not
  segments.
- **Empty path**: a path that holds no segment: no elements at all, or only moves
  (and closes). It covers no area (owner decision OQ2, 2026-10-25).
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

Each operand is read with its own fill rule (owner decision OQ1, 2026-10-25): A
covers a point under A's fill rule and B covers it under B's fill rule, whether the
two rules are equal or not. The definitions above therefore hold for every pair of
fill rules. The result of an operation between two non-empty operands MUST carry
the `NonZero` fill rule, whatever the operands' rules and whatever their order,
including when that result is empty.

The point-by-point definition is exact only away from the boundaries: near a
boundary the computed result deviates from the exact one by the amount stated in
"Determinism and numeric tolerance" (boolean tolerance). The testable form of the
definition is therefore stated for operands built from axis-aligned rectangles,
where the measured deviation is far below the tolerance, with a margin equal to
the tolerance around the perimeter of both operands.

Empty operands. When at least one operand is empty, the operation MUST NOT compute
the result geometrically; it MUST return:

- `A ∪ ∅` and `∅ ∪ A`: a clone of `A`;
- `A Δ ∅` and `∅ Δ A`: a clone of `A`;
- `A \ ∅`: a clone of `A`;
- `∅ \ A`, `A ∩ ∅` and `∅ ∩ A`: an empty path;
- any operation over two empty paths: an empty path.

A clone is equal in value to the surviving operand: the same elements in the same
order, the same start point and the surviving operand's own fill rule (not
`NonZero`), whichever operand position it held and whatever fill rule the empty
operand carries. A moves-only operand is empty and takes this rule. An empty
result returned by this rule holds no elements; its fill rule is not asserted,
because an empty path covers no point under either rule. The input validation
of "Degenerate and invalid inputs" runs before this rule, so an invalid input is
rejected even when the other operand is empty.

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

#### Scenario: Union with an empty path returns a clone (D2, unit)
- GIVEN a rectangle path R, a circle path C, and an empty path E, all with the `NonZero` fill rule
- WHEN `R ∪ E`, `E ∪ R`, `C ∪ E` and `E ∪ C` are computed
- THEN the first two results are equal in value to R and the last two are equal in value to C (same elements in the same order, same start point, same fill rule)

#### Scenario: The surviving operand keeps its own fill rule (D2, unit)
- GIVEN an empty path E with the `NonZero` fill rule, and a ring path G with the `EvenOdd` fill rule made of an outer square (0,0)-(10,10) and an inner square (3,3)-(7,7) wound the same way
- WHEN `E ∪ G`, `G ∪ E`, `E Δ G` and `G \ E` are computed
- THEN each result is equal in value to G, carries the `EvenOdd` fill rule, and the point (5,5) in the hole is reported outside it

#### Scenario: A moves-only path is empty in either operand position (D2, unit)
- GIVEN a moves-only path M holding `MoveTo(1,1)` and `MoveTo(4,4)` with the `NonZero` fill rule, a second moves-only path M2 holding one `MoveTo(2,2)`, and the ring path G (`EvenOdd`) above
- WHEN `M ∪ G`, `G ∪ M`, `M Δ G`, `G \ M`, `M \ G`, `M ∩ G`, `G ∩ M` and `M ∪ M2` are computed
- THEN none returns the open-subpath error; the first four results are equal in value to G (with `EvenOdd`), and the last four are empty paths with no elements

#### Scenario: A moves-only path covers no point (D4, unit)
- GIVEN the moves-only path M
- WHEN the containment of the point (1,1) is tested
- THEN the call succeeds and answers `Outside`, not the open-subpath error

#### Scenario: Difference and xor with an empty path return a clone (D2, unit)
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

#### Scenario: Validation runs before the empty-operand rule (D2, unit)
- GIVEN the rectangle path R and the empty path E
- WHEN `R ∪ E` is computed with tolerance 0
- THEN the call returns the invalid-tolerance error, not a clone of R

#### Scenario: Crossing bars have overlapping difference boxes (D2, unit)
- GIVEN A, the rectangle from (0,-10) to (2,10), and B, the rectangle from (-10,0) to (10,2)
- WHEN `A \ B` and `B \ A` are computed
- THEN both are non-empty, their bounding boxes overlap by an area of 4, and the point (1,1) is covered by neither

#### Scenario: Xor box can be smaller than the union box (D2, unit)
- GIVEN A, the rectangle from (0,0) to (10,10), and B, the rectangle from (0,0) to (10,5)
- WHEN `A Δ B` and `A ∪ B` are computed
- THEN the bounding box of the xor spans y from 5 to 10 and the bounding box of the union spans y from 0 to 10

#### Scenario: Box containment holds (D2, property)
- GIVEN two generated paths, each made of one to three axis-aligned rectangles, each with a fill rule chosen independently
- WHEN the four operations are computed
- THEN, for every result that is not empty: the bounding box of `A \ B` is contained in the bounding box of `A`; the bounding box of `A ∩ B` is contained in the intersection of the two operand boxes; the bounding box of `A Δ B` is contained in the bounding box of `A ∪ B`; each within the tolerance

#### Scenario: Point-by-point definition holds for rectangles (D2, property)
- GIVEN two generated paths, each made of one to three possibly overlapping axis-aligned rectangles wound the same way, each path with a fill rule chosen independently (so all four combinations occur), and a generated point farther than the tolerance from every rectangle edge of both operands
- WHEN the four operations are computed, the point's containment is tested in each result, and the oracle decides whether each operand covers the point by counting the operand's rectangles that contain it (odd for `EvenOdd`, non-zero for `NonZero`)
- THEN each result covers the point exactly when the definition above says it does from the oracle's answers, no point is covered by both `A \ B` and `B \ A`, and every result carries `NonZero`

#### Scenario: Mixed rules never cover a point neither operand covers (D2, unit)
- GIVEN A, the rectangle (20,0)-(30,10) with the `NonZero` fill rule, and B, the nested squares (0,0)-(10,10) and (3,3)-(7,7) wound the same way with the `EvenOdd` fill rule
- WHEN `A ∪ B` and `B ∪ A` are computed
- THEN in both results (5,5) is outside, (1,1) is inside and (25,5) is inside, and both results carry `NonZero`

#### Scenario: Intersection, difference and xor with mixed rules (D2, unit)
- GIVEN A, the rectangle (2,2)-(8,8) with the `NonZero` fill rule, and B, the same `EvenOdd` nested squares
- WHEN `A ∩ B`, `A \ B`, `B \ A` and `A Δ B` are computed and the points (5,5), (2.5,2.5) and (1,1) are tested in each
- THEN `A ∩ B` covers (2.5,2.5) only; `A \ B` covers (5,5) only; `B \ A` covers (1,1) only; `A Δ B` covers (5,5) and (1,1) but not (2.5,2.5); and every result carries `NonZero`

#### Scenario: The rule of each operand is used in both directions (D2, unit)
- GIVEN A, the nested squares (0,0)-(10,10) and (3,3)-(7,7) wound the same way with the `NonZero` fill rule (A covers the whole square (0,0)-(10,10)), and B, the nested squares (2,0)-(12,10) and (5,3)-(9,7) wound the same way with the `EvenOdd` fill rule (B has a hole spanning x 5..9, y 3..7)
- WHEN `A ∩ B`, `B ∩ A` and `A \ B` are computed and the points (6,5) and (4,5) are tested in each
- THEN (6,5), which A covers and B does not, is outside `A ∩ B` and `B ∩ A` and inside `A \ B`; (4,5), which both cover, is inside `A ∩ B` and `B ∩ A` and outside `A \ B`

#### Scenario: Result of non-empty operands carries NonZero (D2, unit)
- GIVEN two `EvenOdd` rectangles (0,0)-(10,10) and (5,0)-(15,10), and the same pair with their rules swapped to `EvenOdd` and `NonZero`
- WHEN each of the four operations is computed in both operand orders, and the intersection of two disjoint `EvenOdd` rectangles is computed
- THEN every result carries the `NonZero` fill rule, including the empty intersection

#### Scenario: Empty operand with mixed rules keeps the survivor's rule (D2, unit)
- GIVEN an empty path E with the `EvenOdd` fill rule and the rectangle R with the `NonZero` fill rule, and the empty path E2 with the `NonZero` fill rule and the ring G with the `EvenOdd` fill rule
- WHEN `E ∪ R`, `R Δ E`, `R \ E`, `E2 ∪ G` and `G Δ E2` are computed
- THEN the first three are equal in value to R (`NonZero`) and the last two are equal in value to G (`EvenOdd`)

### Requirement: Degenerate and invalid inputs (D2, D3)

The crate MUST define the behaviour of an operation over inputs that are not well
formed. The cases are: a path with an open subpath; a path with a subpath that
crosses itself; a path with a zero-length segment; a path with a non-finite
coordinate; a path with a coordinate whose absolute value exceeds the coordinate
limit; a tolerance or stroke width that is not a finite positive number.

Every operation (the four boolean operations, the stroke outline, the
simplification, the containment test and the distance to the perimeter) MUST
reject a path with a non-finite coordinate with an error that names the kind of
non-finite value (NaN, positive infinity or negative infinity), before any
geometric computation (Q2). The operation MUST return that error; it MUST NOT loop,
abort the process or return a result.

Coordinate limit (owner decision W1, 2026-10-25). Every operation (the same eight)
MUST reject a path in which the absolute value of any x or y coordinate of any
point (move points, end points and control points) is greater than `1e9`, with a
named error that carries the offending coordinate value and the limit `1e9`,
before any geometric computation. The error contents (value and limit) are
mandatory; its name is a suggestion (`CoordinateOutOfRange { value, limit }`).
When several coordinates exceed the limit, the value carried is that of the first
one in path order, x before y within a point. A coordinate of exactly `1e9` or
`-1e9` is accepted. The operation MUST return that error; it MUST NOT loop, abort
the process or return a result. The limit applies to the input paths only: a
result MAY hold coordinates beyond it (for example the stroke outline of a path
that lies at the limit). The public documentation of each of the eight
operations MUST state the limit `1e9` and the error returned above it.

The query point of the containment test and of the distance to the perimeter is
subject to the same rules (owner decision W2, 2026-10-26): a point with a
non-finite x or y MUST be rejected with the same non-finite-coordinate error naming
the kind of value, and a point with an x or y whose absolute value is greater than
`1e9` MUST be rejected with the same coordinate-out-of-range error carrying that
value and the limit `1e9`. A coordinate of exactly `1e9` or `-1e9` is accepted.
When both the x and the y of the point are defective, x is reported first, and a
non-finite value is reported before a value beyond the limit. These checks run
after all the checks of the path (the error order below), and they run before any
geometric computation. The query point takes no part in `m` and no too-fine rule
applies to the containment test or to the distance to the perimeter.

Motivation (measured, verify round 2 W1): above about `1.34e154` (the square root
of the largest finite f64) the squared length of a segment overflows; the
distance to the perimeter becomes wrong (41% off at `1e154`) and then infinite
(from `1e155`), and the simplification never returns or returns garbage. The
limit `1e9` is set by precision, not by overflow: with the too-fine rules below,
the minimum tolerance of the simplification and of a stroke of width up to `2e9`
is at most `1e-12 × 1e9`, which equals the default tolerance `1e-3` in f64, so the
default tolerance is accepted by them for every path within the limit. The
boolean operations have a ten times higher floor, `1e-11 × m`, so the default
tolerance is accepted by them only when `m` is at most `1e8`; at the limit their
minimum is `1e-11 × 1e9`, which is `1e-2`. A page is at most 16 384 px per side (`document-model`),
so the limit is about 61 000 page widths.

An operation that depends on the path's interior (the four boolean operations and
the containment test) MUST reject an open subpath with an error that names the
index of the open subpath, counted from zero in path order. A subpath is open when
it holds at least one segment and does not end with a close; a subpath that holds
no segment (a lone move) is not open, and a path made only of such subpaths is
empty (OQ2). A segment drawn after a close with no new move starts a new subpath
(the SVG convention: it begins at the start point of the subpath just closed);
that subpath takes the next index in order of appearance, and it is open until a
close ends it. Such an operation MUST
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
4. a coordinate beyond the coordinate limit in the first operand, then in the
   second operand;
5. an open subpath in the first operand, then in the second operand (interior
   operations only; a moves-only subpath is never open, so a moves-only operand
   never reaches this step's error);
6. a tolerance too fine for the operands (boolean operations, the simplification
   and the stroke outline, see below);
7. a non-finite coordinate of the query point (containment test and distance to
   the perimeter only), x before y;
8. a coordinate of the query point beyond the coordinate limit (the same two
   operations), x before y.

For an operation over one path (the stroke outline, the simplification, the
containment test and the distance to the perimeter) that path is the first
operand and there is no second. A non-finite value is never reported as beyond
the limit, because step 3 comes before step 4 and step 7 before step 8. Steps 6 to
8 do not compete: the too-fine rule applies to the first three operations only and
the query point to the last two. All the checks run before any geometric
computation, and before the empty-operand rule of "Boolean operations". A
moves-only operand still goes through steps 3, 4 and 6: a non-finite move
coordinate is rejected, a move coordinate beyond the limit is rejected, and its
move points count in `m`.

A boolean operation MUST reject a tolerance below `1e-11 × m`, where `m` is the
largest absolute value of any x or y coordinate over all the points (move points,
end points and control points) of both operands, with a named error that carries
that minimum (for example `ToleranceTooFine { minimum }`). A tolerance equal to
the minimum is accepted. The rule applies to every boolean operation whatever the
shape of the operands (straight or curved). When both operands are empty, `m`
is 0 and every finite positive tolerance is accepted.

The simplification MUST apply a too-fine rule to its one path (owner decision W1,
2026-10-25): it MUST reject a tolerance below `1e-12 × m`, where `m` is the
largest absolute value of any x or y coordinate over all the points of the path,
with the same too-fine-tolerance error carrying that minimum. A tolerance equal to
the minimum is accepted. For an empty path, or a path whose every coordinate is
0, `m` is 0 and every finite positive tolerance is accepted. Its factor stays
`1e-12`; only the boolean operations use `1e-11`.

The stroke outline MUST also reject a tolerance that is too fine (owner decision
W2, 2026-10-26): it MUST reject a tolerance below `1e-12 × max(m, w / 2)`, where
`m` is the largest absolute value of any x or y coordinate over all the points of
the path, and `w` is the stroke width, with the same too-fine-tolerance error
carrying that minimum. A tolerance equal to the minimum is accepted. The width
takes part because the number of segments of a round cap or join grows with the
ratio of the width to the tolerance, and `m` alone can be 0. The minimum is
positive for every valid width, so the rule has no exemption. The containment
test and the distance to the perimeter have no too-fine rule.

Motivation (measured, `design.md` "Tolerance policy"): below about this bound the
boolean library recurses without limit on curved inputs. In a release build the
process overflows its stack and aborts (an abort that cannot be caught); in a debug
build a library assertion panics earlier. A larger stack does not help. The bound
`1e-11 × m` is empirical: in a release build no failure was observed at or above
`1e-12 × m`, but in a debug build the library's assertions panicked for shapes
centred near the origin (92 of 700 cases per operation at `1e-12 × m`, about
`1e-11` times the extent of the shapes) and none at `1e-11 × m` (0 of 3 000 cases
in union). For the stroke outline (measured): the arcs of round caps and joins
grow in number with the ratio of the width to the tolerance and have no cap; the
process aborted for lack of memory from a ratio of about `1e40`, so a floor
relative to the width bounds the ratio at about `1e12`. For the simplification (measured, verify round 2 W1 follow-up, on
straight polylines in a release build): the call never returned for tolerances of
about `1e-16 × m` and `3e-16 × m` (60 and 43 of 105 cases) and always returned
from `1e-15 × m` up (0 of 105 per level). Examples of hangs: the default
tolerance on a path near `1e13`, and tolerance `1e-9` on a path near `1e7`. The
second path is inside the coordinate limit, so the limit alone does not prevent
the hang. The bound `1e-12 × m` sits a thousand times above the last observed
hang.

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

#### Scenario: Drawing after a close without a move is the next subpath (D2, D4, unit)
- GIVEN the path `MoveTo(0,0)`, `LineTo(10,0)`, `LineTo(10,10)`, `ClosePath`, `LineTo(5,20)`, `LineTo(0,20)` (a closed triangle, then a tail drawn with no new move and no close)
- WHEN its union with a valid rectangle is computed in either operand position, as are its intersection, difference and xor, and its containment of the point (1,1) is tested
- THEN every call returns the open-subpath error naming subpath 1 (the triangle is subpath 0, the tail is subpath 1)

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

#### Scenario: Too fine a tolerance is rejected, never aborts (D2, unit)
- GIVEN the circle of centre (1000,1000) and radius 300 and the rectangle from (900,900) to (1500,1500), both closed with the `NonZero` fill rule (so m = 1500)
- WHEN their intersection is computed with tolerance 1e-13, in a debug and in a release build
- THEN the call returns the too-fine-tolerance error carrying the minimum `1e-11 × 1500` (as computed in f64), and the process neither panics nor aborts

#### Scenario: A tolerance at or above the minimum works (D2, unit)
- GIVEN the same circle and rectangle
- WHEN their intersection is computed with tolerance `1e-11 × 1500` and with tolerance 1e-3
- THEN both calls succeed and return a non-empty result

#### Scenario: The minimum applies to straight operands too (D2, unit)
- GIVEN the rectangles (0,0)-(10,10) and (5,0)-(15,10) (so m = 15)
- WHEN their union is computed with tolerance 1e-13 and with the default tolerance
- THEN the first call returns the too-fine-tolerance error carrying `1e-11 × 15`, and the second returns the same result as the scenario "Union of overlapping rectangles"

#### Scenario: Too fine comes after open subpath (D2, unit)
- GIVEN the same circle and an open path with finite coordinates
- WHEN their union is computed with tolerance 1e-13
- THEN the error is the open-subpath error

#### Scenario: The coordinate limit is inclusive (D2, D3, D4, unit)
- GIVEN the closed rectangle path with corners (-1e9,-1e9) and (1e9,1e9), the valid rectangle path (0,0)-(10,10) and a valid stroke style of width 10
- WHEN the four boolean operations are computed with the large rectangle in either operand position and with tolerance `1e-11 × 1e9` (their minimum), and the large rectangle is passed to the stroke outline, the simplification, the containment test of the point (0,0) and the distance from the point (0,0) to the perimeter, those four with the crate's default tolerance
- THEN every call succeeds (the minimum tolerance of the stroke and of the simplification, `1e-12 × 1e9`, equals the default tolerance and is accepted)

#### Scenario: A coordinate beyond the limit is rejected by every operation (D2, D3, D4, unit)
- GIVEN closed paths that are valid except for one coordinate of `next_up(1e9)` or of `-next_up(1e9)`, placed once in an end point, once in a curve control point and once in a move, and the valid rectangle path (0,0)-(10,10)
- WHEN each path is passed, in either operand position, to union, intersection, difference and xor, and alone to the stroke outline, the simplification, the containment test and the distance to the perimeter, with the default tolerance
- THEN every call returns the coordinate-out-of-range error carrying the offending value (reading it back gives the same bits) and the limit `1e9`, and no result

#### Scenario: The overflow cases return an error instead of hanging (D3, D4, unit)
- GIVEN the open polyline (0,0), (1e154,1e154), (2e154,0) and the open line from (0,0) to (1e155,1e155)
- WHEN the polyline is simplified with tolerance 1e-3, and the distance from the point (1,0) to the line's perimeter is computed with tolerance 1e-3
- THEN both calls return the coordinate-out-of-range error, carrying `1e154` and `1e155` respectively; the simplification returns (it does not loop) and the distance is not infinite

#### Scenario: Non-finite is reported before out of range (D2, D3, unit)
- GIVEN an open polyline holding a coordinate of `1e10` followed by a NaN coordinate, and separately a closed first operand holding a coordinate of `1e10` and a closed second operand holding a positive infinite coordinate
- WHEN the polyline is simplified, and the union of the two operands is computed
- THEN the first call returns the non-finite-coordinate error naming NaN and the second the non-finite-coordinate error naming positive infinity, not the coordinate-out-of-range error

#### Scenario: Out of range is reported before open subpath (D2, unit)
- GIVEN an open path with finite coordinates, one of them `1e10`, and the valid rectangle path (0,0)-(10,10)
- WHEN their union is computed
- THEN the error is the coordinate-out-of-range error carrying `1e10`, not the open-subpath error

#### Scenario: A moves-only operand beyond the limit is rejected (D2, unit)
- GIVEN the moves-only path holding `MoveTo(2e9,0)` and the valid rectangle path (0,0)-(10,10)
- WHEN their union is computed, in either operand position
- THEN the error is the coordinate-out-of-range error carrying `2e9`, not a clone of the rectangle

#### Scenario: Too fine a simplification tolerance is rejected, never hangs (D3, unit)
- GIVEN the open polyline (1e7,1e7), (1e7+1,1e7+1), (1e7+2,1e7+2), so m = 1e7+2 and the minimum is `1e-12 × (1e7+2)` as computed in f64
- WHEN it is simplified with tolerance 1e-9, with `next_down(minimum)` and with the minimum
- THEN the first two calls return the too-fine-tolerance error carrying the minimum, and the third succeeds and every vertex of the input lies within the minimum of the result; no call loops

#### Scenario: Too fine comes after out of range in the simplification (D3, unit)
- GIVEN an open polyline with finite coordinates, one of them `2e9`
- WHEN it is simplified with tolerance 1e-15
- THEN the error is the coordinate-out-of-range error carrying `2e9`, not the too-fine-tolerance error

#### Scenario: The default tolerance is below the boolean minimum for large operands (D2, unit)
- GIVEN the closed rectangle (0,0)-(2e8,2e8) and the valid rectangle path (0,0)-(10,10), so m = 2e8
- WHEN their union is computed with the default tolerance and with tolerance `1e-11 × 2e8`
- THEN the first call returns the too-fine-tolerance error carrying `1e-11 × 2e8` (as computed in f64) and the second succeeds

#### Scenario: Too fine a stroke tolerance is rejected (D3, unit)
- GIVEN the open line from (0,0) to (100,0) with a round cap and round join, so m = 100, and the open line from (0,0) to (1,0) of width 1e6, so w / 2 = 5e5 is above m
- WHEN the first is stroked with width 10 and the second with width 1e6, each with `next_down(minimum)` and with the minimum, where the minimum is `1e-12 × max(m, w / 2)` as computed in f64 (`1e-10` and `5e-7`)
- THEN the calls with `next_down(minimum)` return the too-fine-tolerance error carrying the minimum, and the calls with the minimum succeed; no call aborts the process or runs out of memory

#### Scenario: A degenerate path still has a stroke minimum (D3, unit)
- GIVEN the path made of one move to (0,0) and one line to (0,0) (m = 0) and a stroke width of 8
- WHEN it is stroked with tolerance 1e-13 and with tolerance `1e-12 × 4`
- THEN the first call returns the too-fine-tolerance error carrying `1e-12 × 4`, and the second succeeds

#### Scenario: Too fine comes after out of range in the stroke (D3, unit)
- GIVEN an open line with a coordinate of `2e9`
- WHEN it is stroked with width 10 and tolerance 1e-15
- THEN the error is the coordinate-out-of-range error carrying `2e9`, not the too-fine-tolerance error

#### Scenario: No too-fine rule for the hit tests (D4, unit)
- GIVEN the valid rectangle path (0,0)-(10,10)
- WHEN its containment of the point (5,5) and the distance from the point (12,5) to its perimeter are computed with tolerance 1e-30
- THEN both calls succeed

#### Scenario: A query point beyond the limit or not finite is rejected (D4, unit)
- GIVEN the valid rectangle path (0,0)-(10,10) and query points with one coordinate NaN, positive infinite, negative infinite, `next_up(1e9)` or `-next_up(1e9)`, placed in x and in y
- WHEN the containment test and the distance to the perimeter are called with each point and the default tolerance
- THEN every call returns the non-finite-coordinate error naming the kind of value, or the coordinate-out-of-range error carrying the value (reading it back gives the same bits) and the limit `1e9`, and no result; and the points (1e9, -1e9) and (-1e9, 1e9) are accepted

#### Scenario: The query point is checked after the path and in order (D4, unit)
- GIVEN a path holding a coordinate of `2e9`, an open path with finite coordinates, and the query points (NaN, 0), (1e10, 0) and (1e10, NaN)
- WHEN the path with a coordinate of `2e9` and the point (NaN, 0) are passed to the distance to the perimeter, the open path and the point (NaN, 0) to the containment test, and a valid rectangle and the point (1e10, NaN) to the containment test
- THEN the first call returns the coordinate-out-of-range error carrying `2e9`, the second the open-subpath error, and the third the non-finite-coordinate error naming NaN (non-finite before beyond the limit, though y comes after x)

#### Scenario: Public docs state the coordinate limit (D2, D3, D4, manual)
- GIVEN the public documentation of union, intersection, difference, xor, the stroke outline, the simplification, the containment test and the distance to the perimeter
- WHEN a reviewer reads it
- THEN each states the coordinate limit `1e9` and names the error returned above it; the containment test and the distance to the perimeter state that the query point is subject to the same limit; the boolean operations state their minimum tolerance `1e-11 × m`, the simplification its minimum `1e-12 × m` and the stroke outline its minimum `1e-12 × max(m, w / 2)`

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

This includes a subpath whose every segment has zero length (verify round 2,
follow-up of W1). Such a subpath is not moves-only, so it is not empty (OQ2), and
it MUST NOT disappear from the result or become moves-only: its output subpath MUST
start with a move to the input's start point and MUST hold at least one segment,
and every end point and control point of that output subpath MUST equal that
start point. Both the input subpath unchanged and a move followed by one
zero-length line satisfy this.

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

#### Scenario: A subpath of zero-length segments is kept (D3, unit)
- GIVEN the path `MoveTo(5,5)`, `LineTo(5,5)`, `LineTo(5,5)` with the `EvenOdd` fill rule, and the path made of the closed square (0,0)-(10,10) followed by that same subpath
- WHEN each is simplified with tolerance 1e-3
- THEN the first result is not empty (it does not come back with 0 elements): it holds exactly one subpath, whose first element is `MoveTo(5,5)`, which holds at least one segment, and every end point and control point of it is (5,5), with the `EvenOdd` fill rule; the second result holds two subpaths, the square first and then a subpath meeting the same conditions

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
under that rule. An empty path, including a moves-only path, covers no point: its
containment test answers `Outside`. A point within the tolerance of the perimeter MUST be reported as
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
- GIVEN two generated well-formed paths made of axis-aligned rectangles, each with a fill rule chosen independently, and a point farther than the tolerance from every rectangle edge of both operands
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

Boolean tolerance. The tolerance of a boolean operation is the accuracy of the
sweep that computes it: a comparison tolerance along the sweep direction, not a
distance in the plane. The crate MUST NOT claim, in a requirement or in its public
documentation (the boolean operations and the default tolerance constant), that the
boolean result lies within the tolerance of the exact answer; that claim is false
(verify-report C4: a circle-rectangle difference deviated 78 times the tolerance).
The public documentation MUST state what was measured, and MUST say that it is
empirical, not proven:

- for straight-edged operands, and for curves that meet the other boundary at an
  angle, the result matched the exact boundary to about 1e-11, far below the
  tolerance;
- the worst case is a crossing where a curve runs parallel to the sweep direction;
  there the deviation from the exact boundary grew as the square root of the
  tolerance and stayed below `sqrt(2 × R × tolerance)`, where R is the largest
  radius of curvature of the operands at a crossing (largest observed ratio of
  deviation to `sqrt(R × tolerance)`: 1.414);
- the tolerance with which a curved input was itself produced (for example the
  accuracy of a circle's approximation by cubics) adds to that deviation;
- the tolerance is meaningful only well below the thinnest feature of the
  operands.

The only testable boolean accuracy scenario is the point-by-point property for
rectangle operands ("Boolean operations"). No test may assert a boolean deviation
bound for curved operands.

#### Scenario: Repeated runs agree (D2, property)
- GIVEN a fixed pair of paths
- WHEN the union is computed 100 times
- THEN all 100 results are equal to each other

#### Scenario: Tolerance is visible in the API (D3, unit)
- GIVEN a valid rectangle path, a valid circle path and a valid stroke style
- WHEN a test calls each of union, intersection, difference, xor, the stroke outline, the simplification, the containment test and the distance to the perimeter, once with an explicit tolerance of 0.5 and once with the crate's documented default tolerance constant
- THEN every call compiles, passes the tolerance as an argument and succeeds

#### Scenario: Public docs make no boolean bound claim (D2, manual)
- GIVEN the public documentation of the four boolean operations and of the default tolerance constant
- WHEN a reviewer reads it
- THEN no sentence says the tolerance bounds the deviation of a boolean result, and the documentation states the measured behaviour listed above and calls it empirical

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

## Owner decisions

Closed by the owner on 2026-10-25 and 2026-10-26, after verify round 1 (findings C3
and W5), verify round 2 (finding W1) and verify round 3 (finding W2); also recorded in `apply-progress.md`. Like "Closed decisions", this section is not a
requirement and archive MUST NOT merge it as one.

| # | Question | Decision | Where it constrains this spec |
|---|---|---|---|
| OQ1 | Which fill rule applies when the two operands of a boolean operation carry different ones | A: each operand is read with its own fill rule (the explorer measured an exact, same-cost way to do it with the approved library) | "Boolean operations", "Geometric hit testing" |
| OQ1 follow-up | Which fill rule a result between non-empty operands carries | `NonZero` always, independent of operand order (the same rule as stroke outlines) | "Boolean operations" |
| OQ2 | Is a path holding only moves empty | A: yes; it holds no segment and covers no area, so it takes the empty-operand rule and is never an open subpath | Terms, "Boolean operations", "Degenerate and invalid inputs", "Geometric hit testing" |
| W1 (verify round 2) | How to bound coordinate magnitude, given that the simplification hangs and the distance overflows on large finite coordinates | A: every operation rejects a path with a coordinate whose absolute value exceeds `1e9`, with a named error carrying the value and the limit; and the simplification also rejects a tolerance below `1e-12 × m` with the too-fine-tolerance error | "Degenerate and invalid inputs" |
| W2 (verify round 3) | The floors found by the explorer's measurements: in debug builds the boolean library panics up to about `1e-11 × m` for shapes centred near the origin; a stroke with a tiny tolerance relative to the width grows without bound and can abort for lack of memory; the hit tests accept a query point of any magnitude | A for all: the boolean minimum becomes `1e-11 × m` (the simplification keeps `1e-12 × m`); the stroke outline rejects a tolerance below `1e-12 × max(m, w / 2)` with the too-fine-tolerance error (the hit tests get no too-fine rule); the query point of the containment test and of the distance is subject to the coordinate limit and the non-finite rule, checked after the path checks | "Degenerate and invalid inputs" |
