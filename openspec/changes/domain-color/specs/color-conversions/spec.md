# Color Conversions Specification

## Purpose

Analytic conversions between sRGB, linear sRGB and Display-P3, the `ColorTransform` strategy trait (02 §4.10), a minimal `PixelLayout`, and the single `AnalyticTransform` implementation. New capability; depends on `color-model`. Values are never compared bit-for-bit across platforms (`powf` comes from the platform libm and may differ); only tolerances are used, except for the bit-identity checks listed. Scenario type tags: (unit), (property), (integration), (command), (manual).

## Deferred to the next change

Nominal `Lab` and `Cmyk` conversions are NOT part of this change (see `color-model`, "Deferred to the next change"). `bake_lut3d` and `Lut3d` are omitted entirely (see "ColorTransform trait").

## Tolerance table

Tolerances are absolute bounds of the form `tol * max(1, |expected|)` on the `f32` output. Conversions widen to `f64`, compute, and narrow once to `f32`.

| Check | Tolerance | Justification |
|---|---|---|
| Curve round trips (sRGB -> linear -> sRGB, linear -> sRGB -> linear) | 2e-6 | One `powf` each way in f64 plus a single f32 narrowing (about 6e-8 relative); the bound leaves headroom for libm differences and the inverse slope near 0 |
| Gamut round trips and composition (through the 3x3 matrices), inputs in `[0,1]` | 1e-5 | Two curve evaluations and two matrix products, each narrowed once |
| Gamut round trips and composition, inputs in `[-4,4]` | 1e-4 | Narrowing to f32 at an intermediate step with amplified out-of-range values reaches about 1.01e-5, which exceeds 1e-5; 1e-4 leaves a safe margin |
| Curve continuity at the thresholds (f64, in-crate) | decode below 1e-8; encode below 5e-8 | The published encode formula has a genuine discontinuity of about 2.85e-8 at 0.0031308 |
| Published 4-decimal references | 1e-4 | Precision of the published values |
| f64 matrix identities (M times M inverse = I, white-point row sums) | 1e-12 | Pure f64 arithmetic on exact rationals |
| Same-space conversion, alpha, serde | bit identity (`to_bits()`) | No arithmetic is involved |

A tolerance MAY be tightened by the implementation; it MUST NOT be loosened without recording the reason.

## Requirements

### Requirement: Transfer curve

The sRGB transfer curve (IEC 61966-2-1) MUST be used for sRGB and, unchanged, for Display-P3. Decode: `|c| <= 0.04045` gives `c / 12.92`, else `copysign(((|c| + 0.055) / 1.055)^2.4, c)`. Encode: `|l| <= 0.0031308` gives `12.92 * l`, else `copysign(1.055 * |l|^(1/2.4) - 0.055, l)`. The thresholds MUST be compared on the absolute value, so a negative input never takes the linear branch merely because it is negative. The curve MUST be sign-symmetric, `f(-x) = -f(x)` (CSS Color 4 extended transfer, realized with `copysign`), and MUST NOT clamp, so out-of-range finite inputs are conserved and `-0.0` stays `-0.0`. Alpha MUST never pass through a curve. Curve functions are crate-internal; they are tested through the typed API and in-crate unit tests.

#### Scenario: Reference decode values (unit)
- GIVEN encoded sRGB 0, 0.5 and 1
- WHEN converted to linear
- THEN the results are 0, about 0.2140411 (independent f64 evaluation recorded in the test comment) and 1, within 2e-6

#### Scenario: Continuity at thresholds (unit)
- GIVEN the thresholds 0.04045 (decode) and 0.0031308 (encode), each evaluated by both branches in f64, for positive and negative arguments
- WHEN evaluated
- THEN the decode branches differ by less than 1e-8 and the encode branches by less than 5e-8 (the encode formula has a genuine discontinuity of about 2.85e-8)

#### Scenario: Monotonic (property)
- GIVEN `x <= y` in `[-4, 4]`
- WHEN both are decoded (and, separately, encoded)
- THEN decode satisfies `decode(x) <= decode(y)` strictly, and encode satisfies `encode(x) <= encode(y) + 5e-8` (the encode formula with the published constants has a genuine downward step of about 2.85e-8 just above 0.0031308, which survives `f32` narrowing; the tolerance matches `tests/properties.rs`)

#### Scenario: Odd symmetry (property)
- GIVEN any finite `x`
- WHEN decoded and encoded
- THEN `f(-x).to_bits()` equals `(-f(x)).to_bits()`

#### Scenario: Zero, subnormal and large inputs (unit)
- GIVEN `0.0`, `-0.0`, `1e-45`, `-1e-45`, `-0.5`, `2.0`, `1e30`
- WHEN converted sRGB to linear
- THEN `-0.0` keeps its sign bit, the subnormals give finite results within 2e-6 of 0, `-0.5` is the negation of the result for `0.5`, `2.0` gives a finite value above 1 (not clamped), and `1e30` does not panic

### Requirement: Round trips within tolerance

sRGB -> linear -> sRGB and linear -> sRGB -> linear MUST be identities within 2e-6 for finite inputs in `[0,1]` and in `[-4,4]`. Round trips through Display-P3 (sRGB -> P3 -> sRGB and linear -> P3 -> linear) MUST be identities within 1e-5 for inputs in `[0,1]` and within 1e-4 for inputs in `[-4,4]`, including out-of-gamut values. Composition MUST hold: (a -> b) then (b -> c) equals (a -> c) within 1e-5 for inputs in `[0,1]` and within 1e-4 for inputs in `[-4,4]`. Alpha MUST be bit-identical after every conversion, including NaN alpha, `-0.0` and values above 1.

#### Scenario: sRGB <-> linear (property)
- GIVEN arbitrary `Rgba` with color channels in `[0,1]` (second run: `[-4,4]`)
- WHEN converted to `LinearRgba` and back
- THEN each color channel is within 2e-6 of the original

#### Scenario: Linear -> sRGB -> linear (property)
- GIVEN arbitrary `LinearRgba` in `[0,1]` and `[-4,4]`
- WHEN converted to `Rgba` and back
- THEN each color channel is within 2e-6

#### Scenario: sRGB <-> P3 (property)
- GIVEN arbitrary `Rgba` in `[0,1]` and, in a second run, in `[-4,4]`
- WHEN converted to `DisplayP3Rgba` and back
- THEN each color channel is within 1e-5 for the `[0,1]` run and within 1e-4 for the `[-4,4]` run

#### Scenario: Composition (property)
- GIVEN arbitrary values in the three spaces, in `[0,1]` and in `[-4,4]`
- WHEN converting a -> b -> c versus a -> c for all ordered triples
- THEN results agree within 1e-5 for `[0,1]` and within 1e-4 for `[-4,4]`

#### Scenario: Alpha preserved (property)
- GIVEN arbitrary `f32` alpha including NaN, `-0.0`, subnormals and 1.5
- WHEN any typed conversion is applied
- THEN `a.to_bits()` is unchanged

### Requirement: Published matrices and reference points

Linear sRGB <-> CIE XYZ (D65) and linear Display-P3 <-> XYZ (D65) MUST use the CSS Color 4 rational matrices (Display-P3 primaries from SMPTE EG 432-1 with D65 white); conversion between sRGB and P3 composes them. The reference constants (the matrices, the primaries expressed in P3, and P3 red expressed in sRGB) were written from memory at design time and MUST be verified against their published source during application. The matrix-inverse and white-point tests below exist to detect transcription errors, and the reference-value tests MUST record in comments the independent f64 evaluation they rely on.

Reference data (each checked within 1e-4): sRGB red, green, blue in encoded Display-P3 are `(0.91749, 0.20029, 0.13856)`, `(0.45840, 0.98526, 0.29829)`, `(0, 0, 0.9596)` (exact blue 0.95959); encoded P3 red in extended sRGB is `(1.0931, -0.2267, -0.1501)` (exact 1.09307, -0.22674, -0.15013), unclamped (CSS Color 4 and WebKit `color(display-p3 ...)` examples).

#### Scenario: Matrix inverse pairs (unit)
- GIVEN each matrix and its published inverse (sRGB and P3 vs XYZ)
- WHEN multiplied in f64
- THEN the product is the identity within 1e-12 (a typo in any rational fails this)

#### Scenario: White point and luminance row (unit)
- GIVEN the XYZ matrices of sRGB and P3
- WHEN applied in f64 to `[1,1,1]`
- THEN both yield the D65 white `[0.3127/0.3290, 1, (1-0.3127-0.3290)/0.3290]` within 1e-12, and the Y row of the sRGB matrix is about `(0.2126, 0.7152, 0.0722)` (ITU-R BT.709) within 1e-4

#### Scenario: White, black and grays (unit)
- GIVEN `(1,1,1)`, `(0,0,0)` and sRGB grays 0.1, 0.5, 0.9
- WHEN converted to linear and to Display-P3
- THEN white stays `(1,1,1)` and black `(0,0,0)` within 1e-5 and grays keep equal channels within 1e-5

#### Scenario: sRGB primaries into P3 (unit)
- GIVEN encoded sRGB red, green and blue
- WHEN converted to Display-P3
- THEN each result matches the reference triple above within 1e-4

#### Scenario: P3 red out of the sRGB gamut (unit)
- GIVEN encoded Display-P3 red `(1,0,0)`
- WHEN converted to sRGB
- THEN the result matches `(1.0931, -0.2267, -0.1501)` within 1e-4, with no clamping (negative channels and a channel above 1 are kept)

### Requirement: Typed conversions

`Rgba::to_linear`, `LinearRgba::to_srgb`, `LinearRgba::to_display_p3` and `DisplayP3Rgba::to_linear_srgb` MUST exist, and `Color::convert(to: ColorSpaceId)` MUST convert between every ordered pair of the three spaces, returning a `Color` of the target variant. `Color::to_linear_srgb` MUST exist. Converting to the same space MUST return a bit-identical value. The typed API and `AnalyticTransform::apply_f32` MUST share one pipeline and therefore agree bit for bit on the same platform. Out-of-gamut finite inputs MUST yield out-of-range finite outputs (never clamped, never an error).

#### Scenario: All nine pairs via Color (unit)
- GIVEN one `Color` of each variant
- WHEN `convert` targets each `ColorSpaceId`
- THEN nine results of the correct variant exist and agree with the reference values and round trips above

#### Scenario: Same-space identity (property)
- GIVEN any `Color`
- WHEN converted to its own space
- THEN every channel is bit-identical (including NaN and `-0.0`)

#### Scenario: Out-of-gamut conserved (unit)
- GIVEN `Rgba::new(-0.25, 1.75, 0.5, 1.0)`
- WHEN converted to `DisplayP3Rgba`
- THEN the call succeeds and `is_finite()` holds on the result

#### Scenario: Non-finite input (property)
- GIVEN color channels from `{-inf, +inf, NaN, finite}` with at least one non-finite
- WHEN converted through every pair
- THEN no panic occurs and alpha is bit-identical

### Requirement: PixelLayout

`PixelLayout` MUST be defined in `gafira-color` (layer 0 cannot depend on raster) as an exhaustive enum `Rgb` (3 channels) and `Rgba` (4 channels, straight alpha), interleaved `f32`, deriving `Clone, Copy, Debug, PartialEq, Eq, Hash`, with `channels()`. Premultiplied and planar layouts are out of scope.

#### Scenario: Channel counts (unit)
- GIVEN `PixelLayout::Rgb` and `PixelLayout::Rgba`
- WHEN `channels()` is queried
- THEN the results are 3 and 4

### Requirement: ColorTransform trait

`ColorTransform` MUST be `Send + Sync`, object-safe (usable as `Box<dyn ColorTransform>`), and expose `src() -> ColorSpaceId`, `dst() -> ColorSpaceId` and `apply_f32(&self, px: &mut [f32], layout: PixelLayout) -> Result<(), PixelBufferError>`. This is a documented deviation from 02 §4.10 (which returns `()`). `apply_f32` converts interleaved pixels in place from `src` to `dst`; for `Rgba` layouts alpha MUST be untouched. If `px.len()` is not a multiple of `layout.channels()` it MUST return `Err(PixelBufferError::LengthNotMultiple { len, channels })` and MUST leave the buffer completely unmodified; it MUST NOT panic and MUST NOT silently skip a remainder. An empty buffer MUST succeed and stay empty. `PixelBufferError` MUST be `#[non_exhaustive]`, `Clone, Copy, Debug, PartialEq, Eq`, and implement `Display` and `std::error::Error`. `apply_f32` MUST NOT perform I/O. `bake_lut3d` and any `Lut3d` type are omitted entirely, with no placeholder; a provided (defaulted) `bake_lut3d` MAY be added later without breaking implementors or callers.

#### Scenario: Object safety and thread bounds (unit)
- GIVEN a `Box<dyn ColorTransform>` built from `AnalyticTransform`
- WHEN moved to another thread and called there
- THEN it compiles and runs

#### Scenario: Rgba slice (unit)
- GIVEN 3 sRGB RGBA pixels and an sRGB -> linear transform
- WHEN `apply_f32` runs with `PixelLayout::Rgba`
- THEN it returns `Ok(())`, each pixel equals the typed conversion, and every alpha is bit-identical

#### Scenario: Rgb slice (unit)
- GIVEN 2 pixels (6 floats) and an sRGB -> P3 transform
- WHEN `apply_f32` runs with `PixelLayout::Rgb`
- THEN it returns `Ok(())` and each triple equals the typed conversion

#### Scenario: Wrong length (unit)
- GIVEN a buffer of 10 floats with `Rgba` (10 % 4 = 2), and another of 7 floats with `Rgb`
- WHEN `apply_f32` runs
- THEN each returns `Err(LengthNotMultiple { len: 10, channels: 4 })` and `{ len: 7, channels: 3 }`, and the buffers are bit-identical to their input

#### Scenario: Empty buffer (unit)
- GIVEN an empty slice
- WHEN `apply_f32` runs with either layout
- THEN it returns `Ok(())` and the slice stays empty

#### Scenario: Error rendering (unit)
- GIVEN a `PixelBufferError`
- WHEN formatted with `Display` and used as `&dyn std::error::Error`
- THEN a non-empty message naming the length and the channel count is produced

#### Scenario: Agreement with the typed API (property)
- GIVEN arbitrary buffers whose length is a multiple of the channel count
- WHEN `apply_f32` runs for each of the nine pairs and each layout
- THEN each pixel is bit-identical to the per-pixel typed conversion

### Requirement: Analytic transform implementation

`AnalyticTransform` MUST implement `ColorTransform` for every ordered pair of `Srgb`, `LinearSrgb`, `DisplayP3` (nine pairs). `AnalyticTransform::new(src, dst)` MUST be infallible (`const fn`, no `Result`), because every pair is supported; `src()` and `dst()` MUST report the requested pair. When `src == dst`, `apply_f32` MUST leave the buffer bit-identical.

#### Scenario: All nine pairs (unit)
- GIVEN each of the nine pairs
- WHEN a transform is built and applied to a fixed 4-pixel RGBA buffer
- THEN `src()`/`dst()` match the pair and results agree with `Color::convert`

#### Scenario: Identity transform (property)
- GIVEN an arbitrary buffer (including NaN and `-0.0`) and a transform with `src == dst`
- WHEN `apply_f32` runs
- THEN every float is bit-identical

#### Scenario: Slice round trip (property)
- GIVEN arbitrary buffers with channels in `[0,1]`
- WHEN transformed sRGB -> P3 then P3 -> sRGB
- THEN each color channel is within 1e-5 and alpha is bit-identical
