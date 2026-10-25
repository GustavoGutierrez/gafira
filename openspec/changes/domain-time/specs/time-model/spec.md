# Time Model Specification

## Purpose

`gafira-time`: layer-0 rational time for Gafira. Time is an exact integer count of flicks (1/705,600,000 s), with exact frame-rate and audio-sample alignment and a validated `TimeRange`. The crate supplies primitives only: it never reads a clock and never chooses a current instant. Timecode is specified in `timecode`.

New capability; no existing main spec and no dependency on another `gafira-*` crate. Scenario types: unit, property, integration, command, manual. Delivery tags: D0 (crate and workspace wiring), D1 (flicks), D2 (rates), D3 (ranges).

## ADDED Requirements

### Requirement: Crate purity and layering (ARQ-01, ARQ-02) (D0)

`gafira-time` MUST be a layer-0 domain crate with `#![forbid(unsafe_code)]`. It MUST NOT depend on any other `gafira-*` crate and MUST NOT perform I/O, spawn threads, read environment variables or read a system clock (no `std::time::SystemTime::now`, no `std::time::Instant::now`, no `std::fs`, `std::net`, `std::process` or `std::thread` API). Its only runtime dependency MUST be `serde`. Every instant MUST be supplied by the caller; the crate MUST NOT obtain "now" from any source.

#### Scenario: Dependency and lint purity (command)
- GIVEN the workspace with `gafira-time` as a member
- WHEN `cargo tree -p gafira-time -e normal,build` runs and the CI policy steps apply (V5 layer table, `forbid(unsafe_code)` per crate, V12 `clippy.toml` copies byte-identical)
- THEN no other `gafira-*` crate appears, no forbidden crate appears, and clippy reports zero `disallowed_methods` or `disallowed_types`

#### Scenario: No clock, thread or I/O API (command)
- GIVEN the crate sources
- WHEN searched for `SystemTime`, `Instant`, `std::thread`, `std::fs`, `std::net`, `std::process` and `std::env` outside documentation and test fixtures
- THEN zero occurrences are found

### Requirement: Flicks as the exact rational time unit (D1)

Time MUST be represented as a signed integer count of flicks where 705,600,000 flicks equal one second. Storage and the conversions defined in this spec MUST be exact integer arithmetic; no floating-point value MAY participate in storage or in any exact conversion. The tick MUST be exactly divisible by 24, 25, 30, 48, 50, 60, 120 fps and by 44,100, 48,000 and 96,000 Hz. Negative values MUST be representable and MUST NOT be an error at this layer; consumers MAY impose non-negativity.

#### Scenario: Divisibility of the tick (unit)
- GIVEN the flicks-per-second constant
- WHEN divided by each of 24, 25, 30, 48, 50, 60, 120, 44,100, 48,000 and 96,000
- THEN every division has remainder zero

#### Scenario: Negative time is representable (unit)
- GIVEN a flicks value of −1
- WHEN it is read back and used in checked arithmetic
- THEN the value is preserved and no error is returned

### Requirement: Exact construction from whole seconds and milliseconds (D1)

A flicks value MUST be constructible exactly from whole seconds and from whole milliseconds, using the exact factors 705,600,000 flicks/s and 705,600 flicks/ms. A construction whose result is not representable in the signed 64-bit range MUST return an error; it MUST NOT wrap, saturate or lose precision. Sub-flick construction (microseconds, nanoseconds, floating-point seconds) is not part of this change.

#### Scenario: Seconds and milliseconds are exact (unit)
- GIVEN 1 s, 0 s, −1 s, 1 ms and 1,234 ms
- WHEN each is converted to flicks
- THEN the results are 705,600,000; 0; −705,600,000; 705,600; and 870,854,400 flicks

#### Scenario: Overflow on construction (unit)
- GIVEN a seconds count whose product by 705,600,000 exceeds the signed 64-bit range
- WHEN it is converted to flicks
- THEN an overflow error is returned and no value is produced

### Requirement: Checked arithmetic (D1)

Addition, subtraction, multiplication by an integer and negation MUST be checked. On overflow each MUST return an error and MUST NOT wrap or saturate. Addition and subtraction of zero MUST be identities, and subtracting a value from itself MUST yield zero. When the mathematical result is representable, addition and multiplication MUST be associative and commutative.

#### Scenario: Overflow on addition (unit)
- GIVEN the maximum representable flicks
- WHEN one flick is added
- THEN an overflow error is returned

#### Scenario: Add then subtract is an identity (property)
- GIVEN arbitrary flicks `a` and `b` whose sum and difference are representable
- WHEN `b` is added and then subtracted
- THEN `a` is returned exactly

#### Scenario: Negation at the minimum (unit)
- GIVEN the minimum representable flicks
- WHEN it is negated
- THEN an overflow error is returned

#### Scenario: Zero is an identity (unit)
- GIVEN an arbitrary representable flicks value
- WHEN zero is added, and separately subtracted
- THEN the value is unchanged in both cases

### Requirement: Declared lossy and exact display conversions (D1)

Conversion to `f64` seconds MUST be available for display and MUST be documented as lossy and unsuitable for persistence or comparison. Conversion to whole seconds MUST floor towards negative infinity and MUST be exact. No public conversion MAY round to nearest implicitly.

#### Scenario: Floor to whole seconds (unit)
- GIVEN +1.5 s in flicks and −0.5 s in flicks
- WHEN each is converted to whole seconds by floor
- THEN the results are 1 s and −1 s

#### Scenario: Lossy f64 is declared (manual)
- GIVEN the public API documentation
- WHEN the `f64` conversion is read
- THEN it states that the conversion is lossy and that flicks are the exact representation

### Requirement: Frame rates are exact rationals of at least one frame per second (D2)

A frame rate MUST be an exact rational number, stored without floating point, whose value is at least 1 frame per second. A frame rate MUST be accepted only when 705,600,000 multiplied by its denominator is exactly divisible by its numerator, so that one frame is a whole number of flicks; otherwise it MUST be rejected, never rounded. Zero numerators and zero denominators MUST be rejected. The rates 24000/1001 (23.976), 24/1, 25/1, 30000/1001 (29.97), 30/1, 48/1, 50/1, 60000/1001 (59.94), 60/1 and 120/1 MUST be accepted. Equivalent fractions MUST normalize so that they compare equal.

#### Scenario: Standard rates accepted (unit)
- GIVEN 24000/1001, 24/1, 25/1, 30000/1001, 30/1, 48/1, 50/1, 60000/1001, 60/1 and 120/1
- WHEN each is constructed
- THEN each is accepted and its frame duration in flicks is a whole number

#### Scenario: NTSC frame durations are exact (unit)
- GIVEN 24000/1001, 30000/1001 and 60000/1001
- WHEN the frame duration is read
- THEN it is 29,429,400; 23,543,520 and 11,771,760 flicks

#### Scenario: Non-integral frame duration rejected (unit)
- GIVEN 11/1 (705,600,000 leaves remainder 6 when divided by 11) and 2398/100
- WHEN each is constructed
- THEN each returns an error naming the non-integral frame duration and no value is produced

#### Scenario: Below one frame per second rejected (unit)
- GIVEN 1/2 fps, and rates with a zero numerator or a zero denominator
- WHEN each is constructed
- THEN each returns an error

#### Scenario: Equivalent fractions normalize (unit)
- GIVEN 48/2 and 24/1
- WHEN they are compared and their frame durations are read
- THEN they are equal and the frame durations are equal

### Requirement: Frame alignment conversions floor to the containing frame (D2)

Conversion from a whole frame count at a rate to flicks MUST be exact and checked. Conversion from flicks to a frame index MUST floor towards negative infinity: the index returned MUST be that of the frame interval that contains the instant, so an instant inside a frame never moves to the next frame. An instant MUST be reported as a frame boundary exactly when it is an exact multiple of the frame duration, including zero and negative multiples. No conversion MAY round to nearest.

#### Scenario: Frames to flicks and back (property)
- GIVEN an arbitrary frame index whose flicks value is representable and an accepted frame rate
- WHEN the index is converted to flicks and back to a frame index
- THEN the original index is returned

#### Scenario: Instant inside a frame floors (unit)
- GIVEN one flick less than the end of frame 5 at 24 fps, and the last flick before zero
- WHEN each is converted to a frame index
- THEN the results are 5 and −1

#### Scenario: Frame boundary detection (unit)
- GIVEN an exact multiple of the 24 fps frame duration and one flick on each side
- WHEN each is tested for being a frame boundary
- THEN only the exact multiple is a boundary

#### Scenario: Cross-rate conversion is deterministic (unit)
- GIVEN an instant that is an exact 24 fps frame boundary but not a 25 fps one
- WHEN it is converted to a 25 fps frame index
- THEN the containing 25 fps frame is returned (floor), never the nearest one

#### Scenario: Frame conversion overflow (unit)
- GIVEN a frame index whose flicks value would exceed the signed 64-bit range
- WHEN it is converted to flicks
- THEN an overflow error is returned

### Requirement: Audio sample alignment is exact (D2)

A sample rate MUST be a positive integer in hertz for which 705,600,000 is exactly divisible, so one sample is a whole number of flicks; other rates MUST be rejected, never rounded. 44,100, 48,000 and 96,000 Hz MUST be accepted. Conversion between a sample index and flicks MUST be exact in both directions for accepted rates, and floor to the containing sample for instants between samples.

#### Scenario: Standard sample rates accepted (unit)
- GIVEN 44,100, 48,000 and 96,000 Hz
- WHEN each is accepted and its flicks per sample is read
- THEN they are 16,000; 14,700 and 7,350 flicks

#### Scenario: Fractional flick per sample rejected (unit)
- GIVEN a positive rate that does not exactly divide 705,600,000, and a rate of zero
- WHEN each is constructed
- THEN each returns an error

#### Scenario: Sample round trip (property)
- GIVEN an arbitrary sample index whose flicks value is representable and an accepted sample rate
- WHEN the index is converted to flicks and back
- THEN the original index is returned

#### Scenario: Instant between samples floors (unit)
- GIVEN one flick less than the second sample at 48,000 Hz
- WHEN it is converted to a sample index
- THEN sample 1 is returned

### Requirement: Time ranges are ordered and have a non-negative duration (D3)

A time range MUST have a start instant and a duration of zero or more. The end instant MUST be representable in the signed 64-bit range and MUST be validated at construction. An empty range (zero duration) MUST be representable. Construction from an end that precedes its start MUST fail; an inverted range MUST NOT be representable. A negative start MUST be allowed at this layer because consumers may need pre-roll arithmetic; consumers own any non-negativity rule.

#### Scenario: Range construction and end (unit)
- GIVEN a start of 1 s and a duration of 500 ms
- WHEN the range is built
- THEN its end is 1.5 s and its duration is 500 ms

#### Scenario: Empty range (unit)
- GIVEN a range with zero duration
- WHEN it is queried
- THEN it is empty, contains no instant (not even its start) and intersects itself to no value

#### Scenario: Inverted range rejected (unit)
- GIVEN an end before the start
- WHEN a range is built from those bounds
- THEN an error is returned and no value is produced

#### Scenario: Overflowing end rejected (unit)
- GIVEN a start near the maximum representable flicks and a duration whose sum would exceed the range
- WHEN the range is built
- THEN an error is returned

#### Scenario: Negative start is allowed (unit)
- GIVEN a start of −1 s and a duration of 2 s
- WHEN the range is built
- THEN it is valid, its end is 1 s and it contains 0 s

### Requirement: Half-open range semantics (D3)

A time range MUST be half-open: it contains an instant exactly when `start <= instant < end`. Adjacent ranges (the end of one equals the start of the next) MUST NOT overlap. Intersection MUST return the common part for overlapping and containing ranges and MUST return no value for disjoint or empty inputs. Containment, overlap and intersection MUST be exact; no tolerance is permitted.

#### Scenario: Containment at the boundaries (unit)
- GIVEN a range from 1 s to 2 s
- WHEN 1 s, 2 s and 1.5 s are tested for containment
- THEN 1 s and 1.5 s are contained and 2 s is not

#### Scenario: Adjacent ranges do not overlap (unit)
- GIVEN a range that ends where the next starts
- WHEN overlap and intersection are queried
- THEN there is no overlap and no intersection

#### Scenario: Intersection cases (unit)
- GIVEN two partially overlapping ranges, a range containing another, two disjoint ranges and one empty range
- WHEN each pair is intersected
- THEN the first returns the common part, the second the inner range, and the last two return no value

#### Scenario: Symmetry and containment (property)
- GIVEN arbitrary valid ranges
- WHEN they are intersected in both orders
- THEN both results are equal or both are absent, and any intersection is contained in both operands

### Requirement: Deterministic serde wire format (ARQ-10) (D1, D2, D3)

`Flicks`, frame rates, sample rates and `TimeRange` MUST serialize deterministically and round-trip exactly. Flicks MUST serialize as a plain integer count. A frame rate MUST serialize as its normalized numerator and denominator. A sample rate MUST serialize as its integer hertz. A `TimeRange` MUST serialize as its start and duration in flicks. Deserialization MUST reject values that violate this spec: zero or non-integral rates, rates below 1 fps, inverted or overflowing ranges, and negative durations. `Timecode` is a derived display value and MUST NOT be serialized by this crate.

#### Scenario: Round trip (property)
- GIVEN arbitrary valid values of each serialized type
- WHEN each is serialized to JSON and deserialized
- THEN the result equals the original value

#### Scenario: Wire shapes (unit)
- GIVEN 1 s, the 30000/1001 fps rate, 48,000 Hz and a range starting at 1 s with a duration of 2 s
- WHEN each is serialized
- THEN the JSON is `705600000`, `{"num":30000,"den":1001}`, `{"hz":48000}` and `{"start":705600000,"duration":1411200000}`

#### Scenario: Invalid values rejected on deserialization (unit)
- GIVEN JSON with a zero denominator, a non-integral rate, a rate below 1 fps, a negative duration and an overflowing range end
- WHEN each is deserialized
- THEN each is rejected

## Non-goals

- Wall-clock time, dates, calendars, time zones, timestamps and monotonic clocks.
- Sleeping, scheduling, timers or any API that waits for time.
- Sub-flick precision (microseconds, nanoseconds, floating-point ticks) and round-to-nearest conversions.
- Speed changes, time stretching, reverse playback, ramps and variable frame rate.
- Frame-accurate trimming, snapping and edit-layer policies; those belong to `gafira-edit`.
- Localized formatting and user-facing labels (`gafira-i18n`).
- Serialization of `Timecode`.
- Drop-frame timecode; specified in `timecode`.

## Edge cases

| Case | Required behavior |
|---|---|
| Overflow in construction or arithmetic | Return an error; never wrap, saturate or lose precision |
| Negative time | Representable in flicks and in range starts; frame and sample conversions floor towards negative infinity |
| Rounding between rates | Floor to the containing frame or sample; exact on boundaries; never round to nearest |
| Empty range | Valid, contains no instant, has no intersection |
| Inverted range | Not representable; construction from inverted bounds fails |
| Zero frame or sample rate | Rejected |
| Non-integral frame or sample duration | Rejected, never rounded |
| Equivalent frame-rate fractions | Normalized; compare equal |
| Adjacent ranges | Do not overlap (half-open) |
| Overflowing range end | Rejected at construction |
| `f64` seconds | Lossy, display only; flicks remain the exact value |

## Open questions

None. Checked against `docs/specs/00-decisiones.md`: D-03 (reference hardware) and D-08 (product languages) do not constrain this crate; no owner decision is reopened. The crate is a pure primitive and decides no product default. The visual and interaction document (03) owns how the interface displays time once the UI exists; this spec fixes only the exact values and conversions behind that display.
