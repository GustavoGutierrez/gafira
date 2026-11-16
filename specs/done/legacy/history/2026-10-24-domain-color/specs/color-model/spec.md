# Color Model Specification

## Purpose

`gafira-color` (layer 0, domain): pure, serializable color vocabulary used by the document model and later engines (02 §2, §4.10). New capability; no existing spec. Conversions live in `color-conversions`. ICC, gradients, palettes, `Interpolate`, `ColorRegistry`, gamut mapping and clamping are out of scope.

Scenario type tags: (unit), (property), (integration), (command), (manual).

## Deferred to the next change

`Lab` and `Cmyk` (nominal data types and nominal conversions) are NOT part of this change. The design cut them for size (about 180 measured lines, nothing consumes them before print/PDF). They are not `ColorSpaceId` variants and not `Color` variants. When they return, they MUST be documented as NOMINAL (Lab D50 per CSS Color 4; naive CMYK on clamped encoded sRGB; not valid for production ΔE2000 or print) and their requirements are specified in that change. The proposal's cut line for them is therefore already applied.

## Requirements

### Requirement: Purity and dependencies

`gafira-color` MUST have no I/O, MUST use `#![forbid(unsafe_code)]`, MUST NOT depend on any `gafira-*` crate, and MUST depend on `serde` only (normal dependencies). `proptest` and `serde_json` MAY appear as dev-dependencies only. No other third-party crate MAY be added. Every public value type MUST implement `Serialize` and `Deserialize`.

#### Scenario: Normal dependency tree (command)
- GIVEN the crate
- WHEN `cargo tree -p gafira-color -e normal,build --target all --prefix none -f '{p}'` runs
- THEN the only `gafira-*` line is `gafira-color` itself, `serde` is the only direct dependency, and no UI, engine or I/O crate appears

#### Scenario: Dev dependencies bounded (command)
- GIVEN the crate
- WHEN `cargo tree -p gafira-color -e dev` runs
- THEN the only additions over the normal tree are `proptest` (with its transitive crates) and `serde_json`

#### Scenario: I/O and unsafe rejected (integration)
- GIVEN a scratch `std::fs::read` call and, separately, a scratch `unsafe {}` block in `gafira-color`
- WHEN `cargo clippy -p gafira-color -- -D warnings` and `cargo check -p gafira-color` run
- THEN the first fails citing `disallowed_methods` and the second fails with an unsafe-code error

### Requirement: Channel types and precision

The crate MUST expose `Rgba` (encoded sRGB), `LinearRgba` (linear-light sRGB primaries) and `DisplayP3Rgba` (encoded Display-P3), each with public `f32` fields `r`, `g`, `b`, `a`. Alpha MUST be straight (non-premultiplied). A `const` constructor `new(r, g, b, a)` MUST be total (never fail, never panic) and MUST store the values unchanged. The types MUST NOT implement `Default`, `Eq`, `Hash` or `Ord`. Each MUST offer `is_finite()` (true only when all four channels are finite) as the validation hook for later document invariants.

#### Scenario: Values retained (unit)
- GIVEN `Rgba::new(0.25, 0.5, 0.75, 1.0)`
- WHEN each channel is read back
- THEN the values are bit-identical to the inputs

#### Scenario: Unvalidated construction (unit)
- GIVEN channels `-0.5`, `1.5`, `f32::NAN`, `f32::INFINITY`, `-0.0`
- WHEN each is used to build all three channel types
- THEN construction does not panic, each channel reads back bit-identical, and `is_finite()` is false exactly when a NaN or infinity is present

### Requirement: ColorSpaceId

`ColorSpaceId` MUST have exactly the variants `Srgb`, `LinearSrgb`, `DisplayP3`, MUST be exhaustive (not `#[non_exhaustive]`), MUST derive `Copy`, `Eq`, `Hash`, `Ord`, and MUST expose `ALL` listing the three variants. It MUST serialize as the CSS Color 4 identifiers `"srgb"`, `"srgb-linear"`, `"display-p3"`. Any other name, including `"SRGB"` and `"linear-srgb"`, MUST fail deserialization.

#### Scenario: Variant set and wire names (unit)
- GIVEN `ColorSpaceId::ALL`
- WHEN each variant is serialized to JSON
- THEN the output is `"srgb"`, `"srgb-linear"`, `"display-p3"` and the set has exactly three members

#### Scenario: Unknown names rejected (unit)
- GIVEN the JSON strings `"rec2020"`, `"SRGB"`, `"linear-srgb"`
- WHEN deserialized as `ColorSpaceId`
- THEN each fails

#### Scenario: Exhaustive match (unit)
- GIVEN a test that matches on `ColorSpaceId` and on `Color` without a wildcard arm
- WHEN the crate tests compile
- THEN compilation succeeds (a future added space breaks it deliberately)

### Requirement: Color enum and wire format

`Color` MUST be an exhaustive enum with exactly `Srgb(Rgba)`, `LinearSrgb(LinearRgba)` and `DisplayP3(DisplayP3Rgba)`, serialized with `#[serde(tag = "space")]` as a bare value: one flat JSON object with `space` first, then `r`, `g`, `b`, `a` in that order, with no wrapper. Identical values MUST produce identical bytes (ARQ-10). `Color` MUST provide `space()` and `alpha()` and `From` impls from the three channel types. Deserialization MUST fail for an unknown `space` value, a missing `space`, a missing channel (including `a`), and a channel that is `null` or not a number. Extra fields MUST be ignored (02 §3.5). A bare channel type (not `Color`) MUST serialize as `{"r":..,"g":..,"b":..,"a":..}` with no tag.

#### Scenario: Exact JSON shape (unit)
- GIVEN `Color::Srgb(Rgba::new(1.0, 0.5, 0.0, 1.0))`
- WHEN serialized with `serde_json`
- THEN the output is exactly `{"space":"srgb","r":1.0,"g":0.5,"b":0.0,"a":1.0}`

#### Scenario: Tag per variant (unit)
- GIVEN one `Color` per variant
- WHEN serialized
- THEN `space` is `srgb`, `srgb-linear`, `display-p3` respectively and equals the serialization of `space()`

#### Scenario: Bare channel type (unit)
- GIVEN `Rgba::new(1.0, 0.5, 0.0, 1.0)`
- WHEN serialized
- THEN the output is `{"r":1.0,"g":0.5,"b":0.0,"a":1.0}`

#### Scenario: Unknown, case-changed or missing tag (integration)
- GIVEN JSON with `"space":"rec2020"`, with `"space":"SRGB"`, and with no `space` key
- WHEN deserialized as `Color`
- THEN all three fail

#### Scenario: Missing and non-numeric channels (integration)
- GIVEN an sRGB object missing `a`, and one with `"r":null`
- WHEN deserialized as `Color`
- THEN both fail

#### Scenario: Extra field ignored (integration)
- GIVEN `{"space":"srgb","r":0.1,"g":0.2,"b":0.3,"a":1.0,"x":1}`
- WHEN deserialized as `Color`
- THEN it succeeds and equals `Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 1.0))`

#### Scenario: Untagged fallback compatibility (integration)
- GIVEN the object `{"keys":[]}`
- WHEN deserialized as `Color`
- THEN it fails because `space` is required (so a future untagged `Animatable` can fall through to its keyed form)

### Requirement: Serde round trip

JSON round trip of every finite value of `Rgba`, `LinearRgba`, `DisplayP3Rgba`, every `Color` variant and `ColorSpaceId` MUST be bit-identical (`to_bits()`), including negative values, values above 1, `-0.0` and f32 subnormals. Non-finite channels are not persistable: serialization MUST NOT panic and the resulting document (JSON `null`) MUST fail to deserialize.

#### Scenario: Finite round trip (property)
- GIVEN arbitrary finite `f32` channels (full range, subnormals, negatives, > 1) for each channel type and each `Color` variant
- WHEN serialized to JSON and deserialized
- THEN every channel is bit-identical, and a guard test shows the generator reaches `-0.0` and subnormals

#### Scenario: Signed zero and subnormal (unit)
- GIVEN `r = -0.0`, `g = 1e-45` (smallest subnormal), `b = f32::MAX`
- WHEN round-tripped
- THEN `r.is_sign_negative()` holds and all three are bit-identical

#### Scenario: Non-finite not persistable (unit)
- GIVEN a `Color` with a NaN, `+inf` or `-inf` channel
- WHEN serialized and the output deserialized
- THEN serialization does not panic and deserialization fails

### Requirement: Value policy for NaN, infinity and out-of-range

Construction, deserialization of finite values and conversion MUST be infallible and MUST NOT clamp, validate or normalize. Finite values below 0 or above 1 are legal (extended range) and MUST be conserved. If any input channel is non-finite, converted color channels are unspecified (NaN or infinite) but the call MUST NOT panic. No clamping API is part of this change; rejecting non-finite values and range rules are document invariants of a later change.

#### Scenario: Conservation (unit)
- GIVEN `Rgba::new(-0.25, 1.75, 0.5, 2.0)`
- WHEN stored, serialized and deserialized
- THEN all four values are unchanged

#### Scenario: Non-finite never panics (property)
- GIVEN channels drawn from `{-inf, +inf, NaN, finite}`
- WHEN a `Color` is built, serialized, and converted to every `ColorSpaceId`
- THEN no panic occurs

### Requirement: Equality

Channel types and `Color` MUST derive IEEE `PartialEq` (`NaN != NaN`, `-0.0 == 0.0`), compared structurally so that `Color::Srgb(x) != Color::LinearSrgb(x)` for identical channel numbers. Tests that claim identity MUST compare `to_bits()` per channel.

#### Scenario: IEEE equality (unit)
- GIVEN two `Rgba` differing only by `-0.0` versus `0.0` in `r`, and one with a NaN channel
- WHEN compared with `==`
- THEN the zeros compare equal and the NaN value is not equal to itself, while `to_bits()` of the zeros differ

#### Scenario: Variants never equal (unit)
- GIVEN `Color::Srgb` and `Color::LinearSrgb` built from the same four numbers
- WHEN compared with `==`
- THEN the result is false

### Requirement: Deferred ΔE2000 acceptance test

The 02 §4.10 acceptance test (ΔE2000 < 1.0 over a 1,000-color chart) is DEFERRED to a later change that introduces an independent reference (an ICC path or a published colorimetric dataset). Reason: this change has no independent oracle (no ICC, no new dependency), the analytic transforms would be compared against themselves, and `Lab` is itself deferred. This change verifies against published matrices, reference points and round trips instead (see `color-conversions`).

#### Scenario: Deferral recorded (manual)
- GIVEN the change's verify report
- WHEN its coverage notes are reviewed
- THEN the ΔE2000 test is listed as deferred with the reason above and no ΔE2000 test exists in `gafira-color`
