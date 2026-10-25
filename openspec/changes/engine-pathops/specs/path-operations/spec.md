# Path Operations Specification (delta for C4 `engine-pathops`)

## Purpose

C4 adds the layer-2 engine crate `gafira-pathops`: path boolean operations (union,
intersection, difference, xor), stroke outlining, path simplification and geometric
hit testing. The crate does not paint pixels and depends only on `gafira-geom` plus
the boolean-operations library approved in `design.md`. It never depends on
`gafira-doc`, `gafira-edit` or any adapter.

Scope: the WHAT. The HOW (API shape, tolerance policy, error types, library
choice) lives in `design.md`. Decisions are closed there; a requirement that depends
on one names it ("Q1", "Q2"...). Scenario types: unit, property, integration,
command, manual.

## ADDED Requirements

### Requirement: Crate purity and dependencies (ARQ-01, ARQ-02, ARQ-03) (D1)

`gafira-pathops` MUST be a layer-2 engine crate: `#![forbid(unsafe_code)]`, a
byte-identical copy of the canonical `clippy.toml`, and normal dependencies limited
to `gafira-geom`, the approved boolean-operations library and the standard library.
It MUST NOT read clocks, the environment, files, the network or spawn threads
(ARQ-02), and MUST NOT use `HashMap` or `HashSet` (ARQ-10 ban). It MUST NOT depend
on `gafira-doc`, `gafira-edit`, `gafira-color`, `gafira-time`, `gafira-anim` or any
adapter. It MUST NOT paint pixels: it returns geometry, never raster data.

#### Scenario: Dependency set (D1, command)
- GIVEN the tree after D1
- WHEN `cargo tree -p gafira-pathops -e normal,build --target all --prefix none -f '{p}'` is filtered for `gafira-*`
- THEN the set is exactly `gafira-pathops`, `gafira-geom` and the approved boolean library, and the V5 step exits 0

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
paths. Each operation MUST take the fill rule to apply and MUST return the result
as a path. The result MUST preserve the Bézier structure of the input: a boolean
operation over a curved input MUST NOT degrade it to a polyline (Q1).

The four operations MUST satisfy the algebraic identities of set difference over the
chosen fill rule, for any two inputs where the operands are well formed:

- `A ∪ B` and `A ∩ B` are each other's complements with respect to `A ∪ B`;
- `A \ B` and `B \ A` are disjoint;
- `A \ B` is contained in `A`;
- `A Δ B` (xor) equals `A \ B` together with `B \ A`.

A boolean operation over an empty path MUST return the other operand unchanged.
A boolean operation over two empty paths MUST return an empty path.

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

#### Scenario: Union with an empty path (D2, unit)
- GIVEN a rectangle path and an empty path
- WHEN their union is computed
- THEN the result equals the rectangle path

#### Scenario: Union of two empty paths (D2, unit)
- GIVEN two empty paths
- WHEN their union is computed
- THEN the result is empty and contains no contour

#### Scenario: Set identities hold (D2, property)
- GIVEN two generated well-formed paths
- WHEN union, intersection, difference and xor are computed
- THEN the bounding box of `A \ B` is contained in the bounding box of `A`, the bounding boxes of `A \ B` and `B \ A` do not overlap, and the bounding box of the xor equals the bounding box of the union

### Requirement: Degenerate and invalid inputs (D2, D3)

The crate MUST define the behaviour of an operation over inputs that are not well
formed. The cases are: a path with an open subpath; a path with a subpath that
crosses itself; a path with a zero-length segment; a path with a non-finite
coordinate.

A non-finite coordinate MUST be rejected with an error that names the non-finite
value, before any geometric computation (Q2). An open subpath MUST be rejected with
an error that names the open subpath, except when the caller states that open
subpaths are to be closed first (Q3). A self-intersecting path and a path with a
zero-length segment MUST be accepted and MUST NOT be reported as an error.

An error MUST leave no partial result: the operation returns either a complete
result or an error, never a truncated path.

#### Scenario: Non-finite coordinate rejected (D2, unit)
- GIVEN a path containing a NaN coordinate and a path containing an infinite coordinate
- WHEN their union is computed
- THEN both return an error naming the non-finite value, and no result is returned

#### Scenario: Open subpath rejected (D2, unit)
- GIVEN a path whose only subpath is not closed
- WHEN its union with a rectangle is computed
- THEN it returns an error naming the open subpath

#### Scenario: Self-intersecting path accepted (D2, unit)
- GIVEN a path shaped as a bowtie, whose subpath crosses itself
- WHEN its union with a rectangle is computed
- THEN it succeeds and returns a result

#### Scenario: Zero-length segment accepted (D3, unit)
- GIVEN a path whose subpath contains a segment whose start and end coincide
- WHEN its intersection with a rectangle is computed
- THEN it succeeds and returns a result

#### Scenario: Error leaves no partial result (D3, unit)
- GIVEN an operation that fails on its second operand
- WHEN it returns
- THEN the error carries no path data and the inputs are unchanged

### Requirement: Stroke outlining (D3)

The crate MUST convert a path plus a stroke style into the outline path that the
stroke covers. The stroke style MUST express the stroke width and the cap and join
behaviour, and the operation MUST take a tolerance that bounds the deviation
between the outline and the exact stroke.

The outline MUST be a closed path. It MUST be closed even when the stroked input is
an open path, because a stroke always encloses an area.

#### Scenario: Stroke of a straight line is a closed capsule (D3, unit)
- GIVEN a horizontal straight path and a stroke style with a width and a round cap
- WHEN the outline is computed
- THEN the result is a closed path, and its bounding box is wider than the line by the stroke width on each side

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
- THEN each returns an error and no result

### Requirement: Path simplification (D3)

The crate MUST simplify a path within a given tolerance: the simplified path MUST
stay within the tolerance of the original, and MUST NOT contain more segments than
the original. Simplification MUST preserve the path's subpath structure: every
subpath of the input MUST appear in the output, in the same order.

A path that is already minimal MUST be returned unchanged.

#### Scenario: Redundant point removed (D3, unit)
- GIVEN a path with a point that lies on the straight line between its neighbours, and a tolerance
- WHEN the path is simplified
- THEN the output has fewer segments than the input and the deviation stays within the tolerance

#### Scenario: Subpath structure preserved (D3, unit)
- GIVEN a path with two subpaths
- WHEN it is simplified
- THEN the result still has two subpaths, in the same order

#### Scenario: Already minimal path is unchanged (D3, unit)
- GIVEN a square path and a tolerance
- WHEN it is simplified
- THEN the result equals the input

#### Scenario: Deviation stays within tolerance (D3, property)
- GIVEN a generated path and a tolerance
- WHEN the path is simplified
- THEN no point of the original path is farther than the tolerance from the simplified path

### Requirement: Geometric hit testing (D4)

The crate MUST answer whether a point lies inside a path, under a stated fill rule.
It MUST also report the distance from a point to the path's perimeter, which a
caller uses to hit test a stroke.

The containment test MUST use the same fill rule as the boolean operations, so a
point reported inside a path is a point the path's interior covers under that rule.
A point exactly on the perimeter is a boundary case: the test MUST define it
deterministically and the same rule MUST apply to every path.

#### Scenario: Point inside a rectangle (D4, unit)
- GIVEN a rectangle path from (0,0) to (10,10)
- WHEN the containment of the point (5,5) is tested
- THEN the point is reported inside

#### Scenario: Point outside a rectangle (D4, unit)
- GIVEN the same rectangle path
- WHEN the containment of the point (15,5) is tested
- THEN the point is reported outside

#### Scenario: Fill rule changes the answer (D4, unit)
- GIVEN a path of two nested squares, both wound in the same direction
- WHEN the containment of a point inside the inner square is tested under `EvenOdd` and under `NonZero`
- THEN the answers differ, and `EvenOdd` reports the point outside

#### Scenario: Distance to perimeter (D4, unit)
- GIVEN a circle path of radius 10 centred at the origin
- WHEN the distance from the point (12,0) to the perimeter is computed
- THEN the result is 2 within the tolerance

#### Scenario: Containment agrees with the boolean result (D4, property)
- GIVEN two well-formed paths and a point
- WHEN the point's containment in the union is tested and the union is computed
- THEN the point is reported inside the union exactly when it is inside at least one of the two operands, within the tolerance

#### Scenario: Distance is zero inside (D4, unit)
- GIVEN a circle path of radius 10
- WHEN the distance from the point (0,0) to the perimeter is computed
- THEN the result is 10, not 0

### Requirement: Determinism and numeric tolerance (D2, D3, D4)

Every operation MUST be deterministic: the same inputs MUST produce the same
outputs, byte for byte, across runs and across machines. No operation may depend on
hash iteration order, on the system clock or on any ambient state.

Every geometric operation MUST take or use a documented tolerance. The tolerance
MUST be an explicit part of the operation's inputs or a documented default, never an
implicit constant hidden inside a helper. The same tolerance MUST be used
consistently inside one operation, so a single operation never mixes tolerances.

A result MUST be within the stated tolerance of the exact geometric answer.

#### Scenario: Repeated runs agree (D2, property)
- GIVEN a fixed pair of paths
- WHEN the union is computed 100 times
- THEN all 100 results are equal to each other

#### Scenario: Tolerance is visible in the API (D3, command)
- GIVEN the crate's public API
- WHEN the signatures of the boolean, stroke, simplify and distance operations are read
- THEN each names a tolerance, either as a parameter or through a documented default constant

#### Scenario: Result within tolerance (D3, property)
- GIVEN a straight path and a stroke width
- WHEN the outline is computed with a given tolerance
- THEN the distance from the path's midpoint to the outline's boundary is the stroke width, within the tolerance

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

## Open questions

These belong to the owner or to `architect`; the requirements above name them where
they constrain behaviour.

1. **Q1 — Curve fidelity of the boolean result.** The spike (`spike.md`, C2, C3)
   measured two viable options: `linesweeper`, which keeps Bézier curves exact, and
   `i_overlay`, which is better maintained but works on polygons and would flatten
   every input first, degrading curves to polylines within a tolerance. The
   requirement above assumes curve fidelity. **Recommendation: `linesweeper`**, and
   approve it as the new dependency. The alternative is `i_overlay` plus flattening,
   which changes the requirement to say the result MAY be a polyline.
2. **Q2 — Error wording for non-finite coordinates.** The measured library reports
   a NaN coordinate as a non-closed-path error rather than as a NaN error
   (`spike.md`, C2.3). The wrapper must check finiteness itself. **Recommendation:
   the crate's own error type names the non-finite value**, so the caller learns the
   real cause instead of a misleading one.
3. **Q3 — Open subpaths in boolean operations.** An open subpath has no interior,
   so it has no defined meaning under a fill rule. The measured library rejects it.
   **Recommendation: reject open subpaths with a named error, and offer a separate
   explicit "close the subpaths" operation** rather than closing them silently
   inside a boolean call, because silently closing changes the geometry the caller
   asked about.
4. **Q4 — Default tolerance.** Every operation needs a default tolerance when the
   caller does not give one. **Recommendation: one documented crate-wide default
   constant, overridable per call**, in the same unit as the paths. The value
   depends on the working unit of the product (pixels) and on the reference
   hardware in D-03; `architect` proposes a number and the owner confirms it.
5. **Q5 — Boundary case of the containment test.** A point exactly on the perimeter
   is inside, outside, or a separate "on boundary" answer. **Recommendation: a
   separate, explicit answer for the boundary**, so a caller can draw a highlighted
   edge without guessing.
