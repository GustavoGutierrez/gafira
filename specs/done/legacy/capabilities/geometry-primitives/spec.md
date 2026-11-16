# Geometry Primitives Specification

## Purpose

`gafira-geom`: domain-layer geometry and unit types with explicit-DPI conversion. New capability; no existing spec. Bézier paths are out of scope.

## Requirements

### Requirement: Purity and public API

`gafira-geom` MUST have no I/O, MUST use `#![forbid(unsafe_code)]`, MUST expose a public API in `f64`, and MUST NOT depend on any `gafira-*` crate. It MUST depend on `kurbo` (with `serde`) and `serde`. Its public types MUST implement `Serialize` and `Deserialize`.

#### Scenario: Dependency purity (command)
- GIVEN the crate
- WHEN `cargo tree -p gafira-geom` runs
- THEN no `gafira-*` (besides itself), UI, engine or I/O crate appears

### Requirement: kurbo re-exports

The crate MUST re-export `kurbo` `Point`, `Vec2`, `Affine` and `Rect`.

#### Scenario: Re-exports usable (unit)
- GIVEN a test importing the four names from `gafira_geom`
- WHEN a `Point` is translated by a `Vec2` and an `Affine` is applied to a `Rect` corner
- THEN the results equal the same operations done via `kurbo`

### Requirement: Units and Length

`Unit` MUST have exactly `px`, `mm`, `cm`, `in`, `pt`. `Length` MUST always retain its value and its own unit; it MUST NOT be normalized to px implicitly. Fixed physical ratios: 1 in = 25.4 mm = 2.54 cm = 72 pt; px relates to in by the supplied DPI.

#### Scenario: Unit set (unit)
- GIVEN `Unit`
- WHEN its variants are enumerated
- THEN exactly the five units exist

#### Scenario: Unit retained (unit)
- GIVEN `Length` 10 mm
- WHEN read back without conversion
- THEN value is 10.0 and unit is mm

### Requirement: DPI conversion

Conversion between units MUST take the DPI as an explicit argument; no default or ambient DPI MAY exist. A DPI that is zero, negative, NaN or infinite MUST return an error, as MUST a non-finite length value. Physical-to-physical conversions MUST be exact to within 1e-12 relative error.

#### Scenario: Reference conversions (unit)
- GIVEN 25.4 mm
- WHEN converted to in, px at 96 DPI, and pt
- THEN results are 1 in, 96 px, 72 pt (within 1e-12 relative)

#### Scenario: Invalid DPI (unit)
- GIVEN any length
- WHEN converted with DPI 0, -96, NaN or infinity
- THEN each returns an error and no value

#### Scenario: Non-finite length (unit)
- GIVEN a length of NaN or infinity
- WHEN converted with DPI 96
- THEN an error is returned

#### Scenario: Same-unit conversion (unit)
- GIVEN 3 px
- WHEN converted to px at DPI 300
- THEN the result is 3 px

#### Scenario: Round trip (property)
- GIVEN any finite length in [-1e9, 1e9], any unit pair, DPI in [1, 10000]
- WHEN converted A→B→A
- THEN the result is within 1e-9 relative error of the original (absolute 1e-12 near zero) and unit is A

### Requirement: Size and Transform2D

`Size` MUST hold `f64` width and height with accessors (no validation in C1). `Transform2D` MUST provide identity, translation, scale, rotation, composition, point application, and inverse, the inverse returning an explicit failure for a singular transform. It MUST convert losslessly to and from `kurbo::Affine`.

#### Scenario: Identity and composition (unit)
- GIVEN translate(2,3) then scale(2)
- WHEN applied to (1,1)
- THEN the point maps per the stated composition order and identity leaves it unchanged

#### Scenario: Inverse (property)
- GIVEN a non-singular transform T and point p
- WHEN T⁻¹ is applied after T
- THEN p is recovered within 1e-9 relative error

#### Scenario: Singular inverse (unit)
- GIVEN scale(0)
- WHEN inverted
- THEN failure is returned, never NaN output

#### Scenario: Affine interop (unit)
- GIVEN a `Transform2D`
- WHEN converted to `Affine` and back
- THEN the coefficients are bit-equal

### Requirement: Serde round trip

Serialization to JSON (with `float_roundtrip`) MUST round-trip `Length`, `Unit`, `Size`, `Transform2D` and re-exported types. `Unit` MUST serialize as its lowercase name; unknown names MUST fail deserialization.

#### Scenario: Round trip (property)
- GIVEN arbitrary finite values of each type
- WHEN serialized to JSON and deserialized
- THEN the value is bit-identical

#### Scenario: Wire names (unit)
- GIVEN `Unit` in
- WHEN serialized
- THEN the JSON is `"in"`; `"km"` fails to deserialize
