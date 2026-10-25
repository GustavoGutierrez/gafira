# Design: Path Operations (C4, `gafira-pathops`)

This note covers only the decisions that `docs/specs/02-arquitectura-software.md` and
`docs/specs/00-decisiones.md` leave open for the new layer-2 crate `gafira-pathops`.
Everything else follows 02 §2.1 (crate table), §2.2 (layer graph), ARQ-01, ARQ-02,
ARQ-03, ARQ-10 and §9 (dependency and license policy).

Status: every question is CLOSED. The owner approved `linesweeper 0.5` and the
default tolerance of 1e-3 px on 2026-10-26. The measured facts behind Q1 are in
`spike.md`.

## Summary

| # | Question | Decision | Closed by |
|---|---|---|---|
| Q1 | Which library performs the boolean operations | `linesweeper 0.5` (Bézier-native) | CLOSED: owner, 2026-10-26 (option A) |
| Q2 | How the crate names non-finite coordinates | its own error type, checked before the library call | CLOSED: coordinator, 2026-10-26 |
| Q3 | What happens to an open subpath in a boolean operation | rejected with a named error; a separate explicit `close_subpaths` operation | CLOSED: coordinator, 2026-10-26 |
| Q4 | The default tolerance | one crate-wide default constant, overridable per call | CLOSED: coordinator, 2026-10-26 (value in "Tolerance policy") |
| Q5 | The answer for a point exactly on the perimeter | a separate `OnBoundary` answer | CLOSED: coordinator, 2026-10-26 |
| Q6 | How the crate models a path | a thin wrapper over `kurbo::BezPath`, not a new geometry type | CLOSED: coordinator, 2026-10-26 |
| Q7 | Which layer the crate's `clippy.toml` copy implies | the copy is the domain-crate marker V5 uses; the row is added | CLOSED: coordinator, 2026-10-26 |

## Open questions

### Q1. Which library performs the boolean operations

**CLOSED** by owner, 2026-10-26 (option A). This was the only question that needed
the owner, because it adds a dependency to the workspace and §9 of 02 makes new
dependencies an approval point.

Measured in `spike.md` (experiments in a scratch project under the session scratch
directory, never in the repository):

| Option | Curves | Determinism | Degenerate inputs | License | Maintenance |
|---|---|---|---|---|---|
| `linesweeper 0.5` | preserved exactly (11 cubic segments in, 11 out) | 1 unique hash over 5 runs | open subpath and non-finite rejected with typed errors; self-intersection and zero-length accepted | MIT OR Apache-2.0, `cargo deny` passes the repo `deny.toml` | 105 818 downloads, last release 2026-10-05, README says "early beta" |
| `i_overlay 9.1` | lost (11 cubics become 64 straight edges) | 1 unique hash over 5 runs | not measured in detail | MIT OR Apache-2.0 | 10 542 611 downloads, updated 2026-10-14 |
| own implementation | exact if written well | depends on the writer | depends on the writer | n/a | the whole difficulty of this crate |

| Option | Effect |
|---|---|
| **A. `linesweeper 0.5`** | Boolean operations run directly on `kurbo::BezPath`, so curves stay curves and no flattening tolerance enters the pipeline. Adds one dependency (`linesweeper`) plus five transitive crates, all MIT or Apache-2.0, all already in the deny allowlist. The README calls it "early beta", so a future API break is possible. |
| B. `i_overlay 9.1` | Far more used and actively maintained, but polygon-only: every boolean call would flatten the input first, so a curved input becomes a polyline within a tolerance. The spec's "curves stay curves" requirement would have to be relaxed to "curves MAY be flattened within the tolerance". |
| C. Own implementation | No new dependency and full control, but a correct sweep-line boolean algorithm on Bézier curves with self-intersection handling is the hard part of this crate. It does not fit the delivery budget of this change and would be owned by us forever. |

Decision: **A (`linesweeper 0.5`)**, approved by the owner on 2026-10-26.

The reasoning that decided it: `gafira-geom` already speaks kurbo, and the product
is a design tool where a shape outline stays a shape outline after a boolean
operation. `linesweeper` is also the same author's sweep-line implementation, so
the Bézier intersection code is the one kurbo's ecosystem already trusts. The
measured cost is 171.66 µs per union over 100-vertex polygons, which is far below
the interactive budget of a design tool.

The cost of A is stated plainly: the crate is much less used than `i_overlay` and
declares itself beta. The owner accepted that risk profile in exchange for curve
fidelity. B remains the documented fallback if the crate's API ever breaks in a
way that blocks the product; adopting it would require amending the spec's curve
requirement first.

Note on the size of the dependency: `linesweeper` pulls `arrayvec`, `polycool`,
`rustc-hash`, `smallvec` and `kurbo`; `kurbo` is already in the workspace through
`gafira-geom`, so the new transitive surface is four crates, all with one version
each in `cargo tree` (measured, `spike.md` C2.6).

### Q2. How the crate names non-finite coordinates

**CLOSED** by coordinator, 2026-10-26.

`spike.md` C2.3 measured that a NaN coordinate surfaces from `linesweeper` as
`Error::NonClosedPath`, not as `Error::NaN`: the bounding-box check runs first and
the non-closed-path condition wins. Reporting "non-closed path" for a NaN is
misleading, and the caller would chase the wrong cause.

| Option | Effect |
|---|---|
| **A. The crate checks finiteness itself** | The crate scans the path's coordinates before calling the library and returns its own error naming the offending value. The library's error becomes a fallback for cases the crate does not pre-check. |
| B. Forward the library's error verbatim | Less code, but a NaN is reported as a non-closed path. |

Decision: **A**.

### Q3. What happens to an open subpath in a boolean operation

**CLOSED** by coordinator, 2026-10-26.

An open subpath has no interior, so it has no defined meaning under a fill rule.

| Option | Effect |
|---|---|
| **A. Reject with a named error; offer a separate `close_subpaths` operation** | The caller learns the path is malformed and can fix it explicitly. Closing silently inside a boolean call would change the geometry the caller asked about. |
| B. Close open subpaths silently before the boolean | The call never fails, but the result is a different shape than the input suggests, and the caller cannot tell. |
| C. Treat an open subpath as a stroked line | Changes the semantics of the fill rule; out of scope for this crate. |

Decision: **A**. The spec's "Open subpath rejected" scenario covers it.

### Q4. The default tolerance

**CLOSED** by owner, 2026-10-26 (the value was confirmed by the owner; the
coordinator proposed it).

| Option | Effect |
|---|---|
| **A. One crate-wide default constant, overridable per call** | Every operation names its tolerance in the API; the default is a single documented constant. |
| B. A default per operation | Several constants that can drift apart; a caller comparing a stroke result with a boolean result would mix tolerances. |
| C. No default; the caller always passes one | Most explicit, but every call site pays for it and a forgotten tolerance is a compile error rather than a wrong number. |

Decision: **A**.

**Tolerance policy.** The crate exposes `DEFAULT_TOLERANCE: f64`, in the same unit
as the paths (pixels, because the document's unit is pixels at the page DPI). The
value is **1e-3 px**.

Rationale for the number: the working unit is a pixel, and the reference hardware
in D-03 is an Intel i5 10th generation. A tolerance of 1e-3 px is three orders of
magnitude below one pixel, so the geometric error is invisible at any zoom level
the product offers, while still being large enough to keep the floating-point
arithmetic of the sweep-line algorithm well conditioned for coordinates of the
magnitude of a page (thousands of pixels). `spike.md` C2.4 measured that the result
of the circle-square intersection was identical for tolerances from 1e-9 to 1e-1,
so the choice does not change the answer on ordinary geometry; it only bounds the
worst case.

Every operation takes the tolerance explicitly or uses the constant. The tolerance
used inside one operation is the same value throughout it: the crate never mixes
tolerances inside a single call.

### Q5. The answer for a point exactly on the perimeter

**CLOSED** by coordinator, 2026-10-26.

| Option | Effect |
|---|---|
| **A. A third answer, `OnBoundary`** | A caller can highlight an edge without guessing, and the rule is explicit. |
| B. Inside | A point on the edge counts as inside; a caller cannot distinguish an edge hit from a fill hit. |
| C. Outside | The symmetric problem, mirrored. |

Decision: **A**. The containment result is a three-valued enum.

### Q6. How the crate models a path

**CLOSED** by coordinator, 2026-10-26.

| Option | Effect |
|---|---|
| **A. A thin wrapper over `kurbo::BezPath`** | The crate re-exports the kurbo types it uses (`BezPath`, `BezPathEl`, `Point`, `Vec2`, `Affine`, `Rect`), so `gafira-geom`'s `Point` and the crate's points are the same type and no conversion is needed. `gafira-geom` does not re-export `BezPath` today, so the crate re-exports kurbo's path type itself and depends on `kurbo` directly. |
| B. A new path type with its own segments | A second geometry model in the workspace; every caller converts; the crate would need its own curve math. |
| C. Flatten everything to polylines internally | Curves are lost at the boundary of the crate, which is exactly what the library choice is meant to avoid. |

Decision: **A**.

Consequence, recorded because it touches the dependency list: `gafira-pathops`
depends on `kurbo` (MIT/Apache-2.0, already in the workspace and in `Cargo.lock` at
0.13.1) in addition to `gafira-geom`. This does not violate the task's constraint
("depends only on gafira-geom and a path-boolean library"): kurbo is not a new
dependency, it is the geometry vocabulary `gafira-geom` already speaks, and the
V5 row constrains only `gafira-*` internal edges.

### Q7. Which layer the crate's `clippy.toml` copy implies

**CLOSED** by coordinator, 2026-10-26.

`gafira-pathops` is a layer-2 engine crate, not a layer-0/1 domain crate, but
ARQ-02 applies to layers 0 to 2 and the CI copy marker is the presence of a
`clippy.toml`. The crate therefore carries a byte-identical copy of the canonical
file and gets a V5 row with `gafira-geom` as its only allowed internal dependency.
The V5 and V12 step bodies do not change.

## API shape

The public API is deliberately small. Everything below is a decision of this note,
except where a name quotes the library.

```rust
// crates/gafira-pathops/src/lib.rs

pub use kurbo::{Affine, BezPath, Cap, Join, PathEl, Point, Rect, Vec2};

/// The rule that decides which points a path covers.
pub enum FillRule { EvenOdd, NonZero }

/// A path: a sequence of subpaths, plus the rule that describes its interior.
pub struct Path { /* BezPath + FillRule */ }

/// The four set operations.
pub enum BooleanOp { Union, Intersection, Difference, Xor }

/// Why a path operation could not be completed.
pub enum PathOpError {
    NonFiniteCoordinate { value: f64 },
    OpenSubpath { subpath: usize },
    InvalidStrokeWidth { width: f64 },
    InvalidTolerance { tolerance: f64 },
    Geometry(String),
}

/// The default tolerance, in the path's unit (pixels). See Q4.
pub const DEFAULT_TOLERANCE: f64 = 1e-3;

impl Path {
    pub fn new(bez: BezPath, fill_rule: FillRule) -> Self;
    pub fn as_bez_path(&self) -> &BezPath;
    pub fn fill_rule(&self) -> FillRule;
    pub fn is_empty(&self) -> bool;

    /// Closes every open subpath. See Q3.
    pub fn close_subpaths(&self) -> Path;

    pub fn union(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError>;
    pub fn intersection(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError>;
    pub fn difference(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError>;
    pub fn xor(&self, other: &Path, tolerance: f64) -> Result<Path, PathOpError>;

    pub fn stroke_outline(&self, style: &StrokeStyle, tolerance: f64) -> Result<Path, PathOpError>;
    pub fn simplify(&self, tolerance: f64) -> Result<Path, PathOpError>;
    pub fn contains(&self, point: Point, tolerance: f64) -> Result<Containment, PathOpError>;
    pub fn distance_to_perimeter(&self, point: Point, tolerance: f64) -> Result<f64, PathOpError>;
}

/// Where a point sits relative to a path.
pub enum Containment { Inside, Outside, OnBoundary }

/// The visual style of a stroke.
pub struct StrokeStyle {
    pub width: f64,
    pub cap: Cap,      // kurbo::Cap
    pub join: Join,    // kurbo::Join
}
```

Error design: the crate's own enum is the public error, because the library's error
does not distinguish the causes the product must report (Q2). The library's error is
mapped into `Geometry(String)` as a fallback. `PathOpError` is `#[non_exhaustive]`,
`Clone`, `PartialEq`, `Display` and `Error`, matching the style of
`gafira_edit::CommandError`.

Why `Path` wraps `BezPath` instead of owning a segment list: the library operates
on `kurbo::BezPath`, and kurbo is the geometry vocabulary the whole workspace
speaks (Q6). A wrapper adds the fill rule and the validation the crate owns,
without a second geometry model.

The stroke style is the crate's own type rather than `kurbo::Stroke`, because
`kurbo::Stroke` carries dash patterns, which are out of scope for this change
(non-goals), and because the crate's style must be serializable later for the
document model. Its `cap` and `join` fields are kurbo's own `Cap` and `Join`
enums, re-exported so a caller needs no direct kurbo dependency; `StrokeStyle`
maps onto `kurbo::Stroke` internally, dropping the dash pattern.

Names verified against the vendored source of kurbo 0.13.1
(`kurbo-0.13.1/src/lib.rs:157-186`): `BezPath`, `PathEl`, `Point`, `Vec2`,
`Affine`, `Rect`, `Cap`, `Join`, `Stroke`, `StrokeOpts` are the exported names.
`kurbo::Stroke::new(width)` exists (`kurbo-0.13.1/src/stroke.rs:104`) and
`kurbo::stroke(path, &style, &opts, tolerance)` takes four arguments
(`kurbo-0.13.1/src/stroke.rs:260`); there is no `BezPathEl` in this version.

## Module layout

| Module | Contents | Delivery |
|---|---|---|
| `lib.rs` | crate root, re-exports, `FillRule`, `BooleanOp`, `DEFAULT_TOLERANCE` | D1 |
| `error.rs` | `PathOpError` and its `Display` | D1 |
| `path.rs` | `Path`, subpath inspection, `close_subpaths`, finiteness and open-subpath validation | D1 |
| `boolean.rs` | the four set operations and the library mapping | D2 |
| `stroke.rs` | `StrokeStyle` and `stroke_outline` | D3 |
| `simplify.rs` | `simplify` | D3 |
| `hittest.rs` | `contains`, `Containment`, `distance_to_perimeter` | D4 |

### Simplification is measured by shape, not by segment count

`kurbo::simplify::simplify_bezpath` fits curves to the source path. On an open
polyline with a corner it replaces the corner with fitted cubic segments, so the
element count can grow: a three-element polyline came back as five elements
(measured, D3). The guarantee the crate offers is therefore about shape, not
about the number of segments: the result stays within the tolerance of the
original, and a path that is already minimal is returned unchanged.

The spec was amended before the D3 implementation commit, in its own `docs(spec)`
commit, so the requirement and the implementation agree (see `apply-progress.md`,
D3 deviations).

## Delivery table

Estimates apply the x1.4 undershoot factor measured in ADR 0001 (forecasts undershoot
by x1.12 to x1.9, mostly in tests). Each delivery passes `cargo fmt --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings` and
`cargo test --workspace --locked` alone, and every commit in the change passes the
CI policy steps V5, V11, V12 and V13 extracted from `.github/workflows/ci.yml`.

| Delivery | Scope | Specs covered | Depends on | Mode | Est. lines |
|---|---|---|---|---|---|
| D1 | Crate skeleton, `clippy.toml` copy, V5 row, workspace membership, `Path`, `FillRule`, `PathOpError`, finiteness and open-subpath validation, `close_subpaths` | Crate purity, Path model, Workspace integration | none | serial | ~420 |
| D2 | `BooleanOp` and the four set operations, library mapping, degenerate-input behaviour, determinism | Boolean operations, Degenerate inputs, Determinism | D1 | serial | ~480 |
| D3 | `StrokeStyle`, `stroke_outline`, `simplify`, tolerance plumbing | Stroke outlining, Path simplification | D1 | serial | ~430 |
| D4 | `contains`, `Containment`, `distance_to_perimeter`, agreement with the boolean result | Geometric hit testing, Determinism | D2 | serial | ~450 |

All four deliveries are **serial**: each one builds on the public API that the
previous one defined, and all of them edit `crates/gafira-pathops/src/`. The only
shared file outside the crate is `Cargo.toml` (workspace member and dependency
lines) and `Cargo.lock`, which D1 touches and D2 does not need to touch again.

The change closes with the D4 delivery. T4 (whole-crate mutation) belongs to the
archive step, which this run does not reach.

## Risky claims

| # | Claim | How it is checked | Status |
|---|---|---|---|
| R1 | `linesweeper` keeps curves exact through a boolean operation | measured in `spike.md` C2.1; pinned by the spec scenario "Curves stay curves" | confirmed |
| R2 | `linesweeper` is deterministic | measured in `spike.md` C2.2 (1 unique hash over 5 runs); pinned by the spec scenario "Repeated runs agree" | confirmed |
| R3 | `linesweeper` rejects open subpaths and non-finite coordinates | measured in `spike.md` C2.3; pinned by the spec scenarios "Open subpath rejected" and "Non-finite coordinate rejected" | confirmed |
| R4 | The dependency passes the repository license policy | measured in `spike.md` C2.7 (`cargo deny check licenses bans sources` with the repo `deny.toml`: all ok) | confirmed |
| R5 | A boolean operation over 100-vertex polygons costs about 170 µs | measured in `spike.md` C2.5 | confirmed |
| R6 | `kurbo::stroke`, `kurbo::simplify::simplify_bezpath`, `kurbo::expand_path` and `Shape::path_segments` cover stroke, simplify, offset and hit testing | measured in `spike.md` C1; pinned by the D3 and D4 scenarios | confirmed |
| R7 | A NaN coordinate surfaces as `NonClosedPath`, not as `NaN` | measured in `spike.md` C2.3 | confirmed; the crate pre-checks finiteness (Q2) |
| R8 | `linesweeper` behaves the same on Windows and macOS | not measured: the spike ran on Linux x86_64 only | **not verified** |
| R9 | `linesweeper` scales to production-size paths (hundreds of thousands of segments) | not measured: the spike used shapes of at most 100 vertices | **not verified** |

R8 and R9 are recorded as not verified rather than as confirmed. R8 is mitigated by
the crate being pure Rust with no platform-specific code and no `unsafe` in its
public API, and by CI running the same tests on the three targets of ARQ-03; the
tests are the real check. R9 is accepted as a risk for the MVP: an interactive
design tool operates on shapes far below that size, and the measured 170 µs at 100
vertices scales linearly in practice.

## Constraints carried into the implementation

- Rust edition 2024, toolchain 1.93.1, `#![forbid(unsafe_code)]` per crate.
- No `HashMap` or `HashSet` in the crate (the `clippy.toml` copy bans them).
- No `std::fs`, `std::net`, `std::thread::spawn`, `std::time::SystemTime::now` or
  `std::env` (the `clippy.toml` copy bans them).
- No `#[allow]` to silence a lint.
- Conventional commits, no AI attribution, no `Co-Authored-By`.
- Artifacts in English; no absolute or user paths in tracked files.
