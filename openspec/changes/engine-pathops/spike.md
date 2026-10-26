# Spike: path boolean operations for `gafira-pathops`

Explorer role. Change: `engine-pathops`. Date: 2026-10-26. Machine: Linux x86_64.

All experiments run in a scratch cargo project under the session scratch directory
(`/tmp/gafira-pathops-spike`), never inside the repository. The repository was not
modified by any experiment.

## Question

Path boolean operations (union, intersection, difference, xor) need a library or an
own implementation. Which option satisfies the repo constraints: permissive licenses
only, local only, deterministic output, curve-aware results, and no dependency on
`gafira-doc`, `gafira-edit` or any adapter?

## Claim C1. kurbo 0.13 already provides stroke, simplify, offset and hit-test primitives

**Status: confirmed.**

kurbo 0.13.1 is already a dependency of the workspace (`crates/gafira-geom`), pinned
in `Cargo.lock` at 0.13.1, license `Apache-2.0 OR MIT`.

Public modules and items read from the vendored source
(`~/.cargo/registry/src/index.crates.io-*/kurbo-0.13.1/src/`):

| Need | kurbo item | Source |
|---|---|---|
| Stroke outline | `kurbo::stroke(path, &Stroke, &StrokeOpts, tolerance) -> BezPath` | `src/stroke.rs:260` |
| Stroke style | `Stroke`, `StrokeOpts`, `Cap`, `Join`, `StrokeOptLevel` | `src/stroke.rs:22-86` |
| Path simplification | `kurbo::simplify::simplify_bezpath(els, accuracy, &SimplifyOptions) -> BezPath` | `src/simplify.rs:298` |
| Offset / expand | `kurbo::expand_path(path, Diagonal2, Join, f64, f64) -> BezPath` | `src/expand.rs:127` |
| Hit test (distance to perimeter) | `Shape::path_segments(accuracy)` + `ParamCurveNearest::nearest` | `src/shape.rs`, `src/param_curve.rs:163` |
| Flatten | `kurbo::flatten`, `kurbo::segments` | `src/lib.rs` |

Command and output:

```
$ cargo run --release   # scratch project depending on kurbo 0.13
=== kurbo::simplify ===
  elements before=6 after(nonzero)=3
=== kurbo::stroke ===
  stroke outline elements=7 (expect a closed capsule)
=== kurbo hit testing via Shape::path_segments + nearest ===
  inside: distance to perimeter = 8.5858 (radius 10) => inside = true
  outside: distance to perimeter = 40.0000 (radius 10) => inside = false
=== kurbo::expand_path (offset) ===
  expanded elements=6
```

Conclusion: stroke outlining, simplification, offsetting and distance-based hit
testing need **no new dependency**. Only boolean operations are missing from kurbo.

## Claim C2. linesweeper performs boolean operations directly on kurbo Bézier paths

**Status: confirmed.**

`linesweeper 0.5.0`, published 2026-10-05, license `MIT OR Apache-2.0`, 105 818
downloads. Its API takes and returns `kurbo::BezPath` directly.

Source read at `src/lib.rs`:

```rust
pub fn binary_op(
    set_a: &kurbo::BezPath,
    set_b: &kurbo::BezPath,
    fill_rule: FillRule,
    op: BinaryOp,
) -> Result<topology::Contours, Error>

pub fn binary_op_with_eps(
    set_a: &kurbo::BezPath,
    set_b: &kurbo::BezPath,
    fill_rule: FillRule,
    op: BinaryOp,
    eps: f64,
) -> Result<topology::Contours, Error>
```

`FillRule` has exactly `EvenOdd` and `NonZero`. `BinaryOp` has exactly `Union`,
`Intersection`, `Difference`, `Xor`. `Error` has exactly `Infinity`, `NaN` and
`NonClosedPath(NonClosedPath)`. The result `topology::Contours` yields `Contour`s,
each with `path: BezPath`, `parent: Option<ContourIdx>` and `outer: bool`
(`src/topology.rs:2038-2184`).

**C2.1 Curve preservation.** A circle intersected with a square keeps its cubic
segments: 11 cubic segments in, 11 cubic segments out.

```
=== E1: curve preservation (circle vs square) ===
input circle: elements=13 cubics=11
result contours=1
  contour path: elements=13 cubics=11 segs=11 outer=true parent=None
```

**C2.2 Determinism.** The same input produces byte-identical output across 5 runs.

```
=== E2: determinism (same input 5x, compare bytes) ===
unique hashes across 5 runs = 1 (expect 1)
```

**C2.3 Degenerate inputs.** Zero-length segments and self-intersecting paths are
accepted; open subpaths, NaN and infinity are rejected with typed errors.

```
=== E3: degenerate inputs ===
zero-length seg: Ok, contours=1
empty path: Ok, contours=1
open subpath: Err(one of the inputs had a non-closed path)
self-intersecting bowtie: Ok, contours=1
NaN input: Err(one of the inputs had a non-closed path)
Inf input: Err(one of the inputs was infinite)
```

Note: a NaN coordinate surfaces as `NonClosedPath`, not as `Error::NaN`. The `NaN`
variant exists but the bbox check in `binary_op` reports the non-closed path first
for this input. The wrapper crate must therefore validate finiteness itself before
calling the library, so the error the caller sees names the real cause.

**C2.4 Accuracy parameter.** `binary_op_with_eps` accepts an explicit tolerance.
On the circle-square case the output is identical for every tolerance tried.

```
=== E4: eps sensitivity ===
  eps=1e-9 -> contours=1 elements=13 time=89.164µs
  eps=1e-6 -> contours=1 elements=13 time=75.027µs
  eps=1e-3 -> contours=1 elements=13 time=60.742µs
  eps=1e-1 -> contours=1 elements=13 time=73.989µs
```

**C2.5 Speed.** 1000 union operations between two 100-vertex polygons: 171.66 ms
total, 171.66 µs per operation.

```
=== E5: speed (1000 union ops on 100-vertex polygons) ===
  1000 ops in 171.659145ms => 171.659µs/op, contours total=1000
```

**C2.6 Dependency footprint.** `linesweeper` pulls `arrayvec`, `polycool`,
`rustc-hash`, `smallvec` and `kurbo` (already present). All are MIT or
Apache-2.0/MIT.

**C2.7 License check against the repo policy.** `cargo deny check licenses bans
sources` with the repository `deny.toml` copied verbatim into the scratch project:

```
$ cargo deny check licenses bans sources
bans ok, licenses ok, sources ok
```

Dependency tree:

```
spike-deny v0.0.0
├── kurbo v0.13.1
│   ├── arrayvec v0.7.8
│   ├── polycool v0.4.0
│   ├── serde v1.0.229
│   └── smallvec v1.16.4
└── linesweeper v0.5.0
    ├── arrayvec v0.7.8 (*)
    ├── kurbo v0.13.1 (*)
    ├── polycool v0.4.0 (*)
    ├── rustc-hash v2.1.3
    └── smallvec v1.16.4 (*)
```

The `cargo tree` output shows one version per crate, so `cargo deny`'s
`multiple-versions = "warn"` does not trigger.

**C2.8 Maintenance.** The crate is published by `jneem`, who is also the author of
kurbo; it is the same author's sweep-line implementation of the Bentley-Ottmann
algorithm, so the Bézier intersection code is shared in spirit with kurbo's. The
crate README states it is "currently in an early beta state". Latest release
2026-10-05, and 0.5.0 is a semver-compatible release, not a pre-release.

## Claim C3. i_overlay works on polygons only and loses curve information

**Status: confirmed (as a drawback).**

`i_overlay 9.1.0`, license `MIT OR Apache-2.0`, 10 542 611 downloads, last updated
2026-10-14. Its API (`SingleFloatOverlay::overlay`) takes arrays of contours of
points and returns `Vec<Shape>` of polylines. A curve must be flattened before the
call, so the result is a polyline, not a Bézier path.

```
i_overlay intersection contours = 1
  shape with 1 contours, first contour 64 pts
=> output is POLYLINE (64 pts), curves lost: circle had 11 cubics, now 64 straight edges
i_overlay unique hashes across 5 runs = 1
```

Deterministic, and much more actively maintained than `linesweeper` (10.5 M
downloads versus 105 k). But it cannot operate on curves without a flattening step,
which loses exactness and adds a tolerance parameter to every boolean operation.

## Claim C4. Alternative polygon libraries do not beat linesweeper for this use

**Status: confirmed by metadata; not benchmarked.**

| Crate | Version | License | Updated | Downloads | Note |
|---|---|---|---|---|---|
| `i_overlay` | 9.1.0 | MIT OR Apache-2.0 | 2026-10-14 | 10 542 611 | polygon-only (C3) |
| `linesweeper` | 0.5.0 | MIT OR Apache-2.0 | 2026-10-05 | 105 818 | Bézier-native (C2) |
| `geo-clipper` | 0.9.0 | — | 2025-02-08 | 1 018 723 | wraps `i_overlay`; polygon-only |
| `clipper2` | 0.6.0 | MIT OR Apache-2.0 | 2026-05-06 | 438 498 | polygon-only, adds offsetting we already have in kurbo |
| `flo_curves` | 0.8.1 | Apache-2.0 | 2026-08-25 | 382 226 | curve library, no boolean ops on paths |
| `lyon` | 1.0.19 | MIT/Apache-2.0 | 2026-03-08 | 7 991 552 | tessellation, no boolean ops |
| `tiny-skia-path` | 0.12.0 | MIT/Apache-2.0 | 2026-02-02 | 62 310 467 | path building, no boolean ops |

Every polygon-only option forces a flatten-then-boolean pipeline with an extra
tolerance knob. `linesweeper` is the only candidate found that operates on the
Bézier representation the product already uses (`gafira-geom` re-exports kurbo
types), and it is the only one that keeps curves exact through the operation.

`lyon`, `tiny-skia-path` and `flo_curves` were not benchmarked for booleans because
their published APIs do not expose boolean operations on paths.

## Conclusion

**Recommended: `linesweeper 0.5` for boolean operations, kurbo 0.13 for everything
else.**

- Stroke outlining: `kurbo::stroke` (C1).
- Path simplification: `kurbo::simplify::simplify_bezpath` (C1).
- Geometric hit testing: `kurbo::Shape::path_segments` + `ParamCurveNearest`
  (C1), plus an even-odd/non-zero containment test built on the same primitives.
- Boolean operations: `linesweeper::binary_op_with_eps` (C2), which preserves
  curves, is deterministic, rejects open subpaths, and passes `cargo deny` with
  the repository policy.

**Alternative considered and rejected: `i_overlay 9.1`.** Better maintained and far
more downloaded, but polygon-only: every boolean call would need a flattening step
with a tolerance, so curve output would be approximate (C3). For a design tool
whose `gafira-geom` already speaks kurbo, exact Bézier output is worth more than
download counts.

**Alternative considered and rejected: own implementation of the boolean
operations.** A correct sweep-line boolean algorithm on Bézier curves with
self-intersection handling is the hard part of this crate, not the easy part.
Writing it means owning numerical robustness, and it is far more code than the
whole delivery budget of this change. kurbo plus `linesweeper` already covers it.

**Risk to note for the owner.** `linesweeper` describes itself as "an early beta
state". Its license and API are stable enough for the MVP, but the crate is much
less used than `i_overlay`. If the owner prefers the lower-maintenance risk, the
fallback is `i_overlay` plus flattening, at the cost of approximate curve output.

## Not verified

- Behaviour of `linesweeper` on Windows and macOS targets. The experiments ran on
  Linux x86_64 only. The crate is pure Rust with no platform-specific code and no
  `unsafe` in its public API, but this was not measured on the other two CI
  targets.
- Numerical behaviour under pathological inputs at production scale (hundreds of
  thousands of segments). The spike used shapes of at most 100 vertices.
- Whether the `NonClosedPath` error variant carries enough context to report which
  subpath was open. The variant holds a `NonClosedPath` value; its fields were not
  inspected.
