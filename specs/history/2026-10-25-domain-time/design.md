# Design: domain-time (`gafira-time`)

## Context

New layer-0 crate for exact time. The contract is fixed by the delta specs `time-model` and `timecode`; `docs/specs/02-arquitectura-software.md` §2.1 names the crate, its responsibility and its `NO debe` (use system clocks), and §3 uses `Flicks`, `TimeRange` and `FrameRate` in the document model. `docs/specs/backlog/05-funcionalidad-video-y-distribucion.md` V-2 fixes the user-visible form: flicks internally, `HH:MM:SS:FF` and seconds in the interface, cuts on exact frames. No owner decision in `docs/specs/00-decisiones.md` is touched or reopened.

Constraints from the repository: layer 0, no other `gafira-*` dependency, `#![forbid(unsafe_code)]`, no I/O, threads, environment or clocks (`ARQ-02`, enforced by the domain `clippy.toml` copy), deterministic serde (`ARQ-10`), valid `TimeRange` invariants (`ARQ-13`), permissive dependencies only. `serde` is already a workspace dependency and is the only runtime dependency.

Risky arithmetic claims were measured in `spike.md` (divisibility, frame durations, drop-frame mapping, `i64` headroom, serde validation shape). All confirmed; no decision below rests on an unmeasured claim.

## Decisions

### D-1: `Flicks` is a signed 64-bit newtype, exact integer arithmetic only

Options:

| Option | Tradeoff |
|---|---|
| `i64` newtype | ±414 years at flick resolution; signed for pre-roll and offsets; `serde` as a JSON integer; overflow must be checked |
| `u64` | Cannot represent negative offsets the edit layer needs |
| `i128` | Unneeded range; larger wire values and slower arithmetic |
| `f64` seconds | Cannot represent 1/705,600,000 exactly; drift and non-determinism |

Choice: `i64`. 705,600,000 x `u32::MAX` fits in `i64` (spike C3), so all rate products are safe; an `i128` intermediate is used in rate validation anyway. No floating point participates in storage or in any exact conversion. All arithmetic is checked and returns `Result`; no `Add`/`Sub` operator implementations are provided, because they panic on overflow in debug builds and the domain policy is to return an error.

### D-2: `FrameRate` is a normalized rational `u32/u32` with an exactly integral frame duration

Options:

| Option | Tradeoff |
|---|---|
| Rational `num/den` (`u32`) | 23.976 and 29.97 are exact; must define normalization and validation |
| `f32`/`f64` fps | Cannot represent 30000/1001 exactly; comparisons and serde drift |
| Fixed enum of known rates | Rejects legitimate rates and needs a migration for every new one |

Choice: rational. `FrameRate::new` rejects zero numerator or denominator, normalizes by `gcd` (so 48/2 equals 24/1), requires at least 1 fps, and requires `(705_600_000 * den) % num == 0`. The last rule is the crate's central guarantee: every accepted frame rate has a whole-flick frame duration, so every frame boundary is an exact instant and V-2 ("cuts land on an exact frame") is enforceable without tolerances. Rates whose duration is not integral (11 fps, 23.98) are rejected instead of rounded. Constants cover 23.976, 24, 25, 29.97, 30, 48, 50, 59.94, 60 and 120.

`nominal_fps()` rounds to the nearest integer with ties upward (`(num + den/2) / den`); it is used only by timecode labels. The spec is amended to state this rule, so NDF timecode is testable at rational rates.

### D-3: `SampleRate` is a validated positive hertz value

Options: a raw `u32` argument (no invariant), or a validated newtype. Choice: newtype, validated with `705_600_000 % hz == 0`, which accepts 44,100, 48,000, 96,000 and their exact multiples and rejects fractional flick-per-sample rates. The type makes audio alignment a first-class exact operation instead of a documented hope.

### D-4: `TimeRange` stores start, duration and end, validated once

Options:

| Option | Tradeoff |
|---|---|
| `start` + `duration` | Natural for editing operations; `end()` needs an invariant-based addition |
| `start` + `end` | Natural for containment; `duration()` can overflow on extreme spans |
| Both plus `duration` | Redundant 24-byte value, but no arithmetic in accessors and no panic path |

Choice: store `start`, `duration` and `end`, all validated in the only constructor. `duration >= 0`, `start + duration` must not overflow, and `from_bounds` rejects `end < start`. An inverted or overflowing range is not representable (ARQ-13). The range is half-open: contains `t` iff `start <= t < end`; empty ranges contain nothing and never intersect. Negative starts are allowed because pre-roll arithmetic is legitimate; consumers impose non-negativity. Serde uses `{"start","duration"}` and revalidates through the constructor.

### D-5: `Timecode` carries its format and is valid by construction

Options:

| Option | Tradeoff |
|---|---|
| Label + format in one value | A label valid for DF may be invalid for NDF; self-contained formatting; conversion back needs `self.format` |
| Bare label frames | Format must be passed to every call and can contradict the label; invalid labels representable |
| Formatted `String` as the value | Parsing on every operation; loses the frame count; invalid text representable |

Choice: `Timecode { label_frames, format }`, constructed only through `new` (validated), `parse` or `from_flicks`. `format()` is infallible because the label was validated for its format. `to_flicks` is exact and checked. `from_flicks` floors to the containing frame (same rule as `time-model`) and is infallible for a valid format: the index and label bounds are proven by the frame duration. Timecode is never serialized; flicks are the persisted value.

Non-drop and drop-frame are two labeling modes over the same time base. Drop-frame is accepted only for 30000/1001 and 60000/1001; the skipped-number rule is the standard one measured in `spike.md` (skip 00–01 or 00–03 at every minute not divisible by 10). Negative labels are formatted with a leading `-` and mapped symmetrically; hours are a running count and never wrap at 24, because this crate models durations, not time of day.

### D-6: One non-exhaustive error enum, no panics

`TimeError` is a `#[non_exhaustive]` enum implementing `Display` and `std::error::Error`. Constructors and checked operations return `Result`; no public function panics on malformed input. The variants separate cause enough for tests and callers: `Overflow`, `ZeroDenominator`, `ZeroFrameRate`, `FrameRateBelowOneFps`, `FrameRateNotIntegral`, `ZeroSampleRate`, `SampleRateNotIntegral`, `NegativeDuration`, `InvertedRange`, `DropFrameUnsupported`, `DroppedFrameLabel`, `MalformedTimecode`, `InvalidTimecodeField`, `WrongTimecodeSeparator`.

### D-7: Module layout

```text
crates/gafira-time/
  Cargo.toml            serde; dev: proptest, serde_json
  clippy.toml           byte-identical copy of the canonical domain file
  src/lib.rs            crate docs, #![forbid(unsafe_code)], modules, re-exports
  src/error.rs          TimeError
  src/flicks.rs         Flicks, constants, constructors, checked arithmetic
  src/rate.rs           FrameRate, SampleRate, frame/sample conversions on Flicks
  src/range.rs          TimeRange
  src/timecode.rs       Timecode, TimecodeFormat, TimecodeMode
  tests/flicks.rs       public-API tests for D1
  tests/rates.rs        public-API tests for D2
  tests/ranges.rs       public-API tests for D3
  tests/timecode.rs     public-API tests for D4
```

One integration-test file per delivery keeps deliveries independent and avoids edits to another delivery's test file.

### D-8: Serde contract

`Flicks` is `#[serde(transparent)]` over `i64`. `FrameRate`, `SampleRate` and `TimeRange` use `#[serde(try_from = "...", into = "...")]` with private wire structs, so deserialization runs the same validation as construction and rejects invalid values at load time (spike C5). The wire shapes are those fixed by the spec. `Timecode` deliberately has no serde implementation.

### D-9: Public API (shape, not final doc prose)

```rust
pub const FLICKS_PER_SECOND: i64 = 705_600_000;
pub const FLICKS_PER_MILLISECOND: i64 = 705_600;

pub struct Flicks(i64);
impl Flicks {
    pub const ZERO: Self;
    pub const fn from_ticks(ticks: i64) -> Self;
    pub const fn ticks(self) -> i64;
    pub fn from_seconds(seconds: i64) -> Result<Self, TimeError>;
    pub fn from_millis(millis: i64) -> Result<Self, TimeError>;
    pub fn seconds_floor(self) -> i64;
    pub fn as_seconds_f64(self) -> f64;
    pub fn checked_add(self, other: Self) -> Result<Self, TimeError>;
    pub fn checked_sub(self, other: Self) -> Result<Self, TimeError>;
    pub fn checked_mul(self, factor: i64) -> Result<Self, TimeError>;
    pub fn checked_neg(self) -> Result<Self, TimeError>;
}

pub struct FrameRate { /* num, den, normalized, >= 1 fps, integral frame */ }
impl FrameRate {
    pub const FPS_23_976: Self;  pub const FPS_24: Self;   pub const FPS_25: Self;
    pub const FPS_29_97: Self;   pub const FPS_30: Self;   pub const FPS_48: Self;
    pub const FPS_50: Self;      pub const FPS_59_94: Self; pub const FPS_60: Self;
    pub const FPS_120: Self;
    pub fn new(num: u32, den: u32) -> Result<Self, TimeError>;
    pub const fn num(self) -> u32;
    pub const fn den(self) -> u32;
    pub fn fps_f64(self) -> f64;
    pub fn nominal_fps(self) -> u32;
    pub fn flicks_per_frame(self) -> Flicks;
}

pub struct SampleRate(u32);
impl SampleRate {
    pub const HZ_44_100: Self; pub const HZ_48_000: Self; pub const HZ_96_000: Self;
    pub fn new(hz: u32) -> Result<Self, TimeError>;
    pub const fn hz(self) -> u32;
    pub fn flicks_per_sample(self) -> Flicks;
}

impl Flicks {
    pub fn from_frames(frames: i64, rate: FrameRate) -> Result<Self, TimeError>;
    pub fn to_frames_floor(self, rate: FrameRate) -> i64;
    pub fn is_frame_boundary(self, rate: FrameRate) -> bool;
    pub fn from_samples(samples: i64, rate: SampleRate) -> Result<Self, TimeError>;
    pub fn to_samples_floor(self, rate: SampleRate) -> i64;
}

pub struct TimeRange { /* start, duration, end */ }
impl TimeRange {
    pub fn new(start: Flicks, duration: Flicks) -> Result<Self, TimeError>;
    pub fn from_bounds(start: Flicks, end: Flicks) -> Result<Self, TimeError>;
    pub const fn start(self) -> Flicks;
    pub const fn duration(self) -> Flicks;
    pub const fn end(self) -> Flicks;
    pub const fn is_empty(self) -> bool;
    pub fn contains(self, at: Flicks) -> bool;
    pub fn intersects(self, other: Self) -> bool;
    pub fn intersection(self, other: Self) -> Option<Self>;
}

pub enum TimecodeMode { NonDrop, DropFrame }
pub struct TimecodeFormat { /* rate, mode */ }
impl TimecodeFormat {
    pub fn new(rate: FrameRate, mode: TimecodeMode) -> Result<Self, TimeError>;
    pub const fn rate(self) -> FrameRate;
    pub const fn mode(self) -> TimecodeMode;
}
pub struct Timecode { /* label_frames, format */ }
impl Timecode {
    pub fn new(label_frames: i64, format: TimecodeFormat) -> Result<Self, TimeError>;
    pub fn from_flicks(at: Flicks, format: TimecodeFormat) -> Self;
    pub fn parse(text: &str, format: TimecodeFormat) -> Result<Self, TimeError>;
    pub fn to_flicks(self) -> Result<Flicks, TimeError>;
    pub const fn label_frames(self) -> i64;
    pub const fn format_spec(self) -> TimecodeFormat;
    pub fn format(self) -> String;
}
```

`to_frames_floor` and `to_samples_floor` are infallible: Euclidean division of `i64` by a positive `i64` cannot overflow. `format()` is infallible for the same reason: the label and format pair was validated.

### D-10: Exact algorithms

- `Flicks::from_seconds/millis`: `checked_mul` by the exact factor.
- `Flicks::seconds_floor`: `div_euclid(FLICKS_PER_SECOND)`, so negative values floor towards minus infinity.
- `FrameRate::new`: zero checks, `gcd` reduction, `num >= den`, integrality check in `i128`.
- `Flicks::to_frames_floor`: `ticks.div_euclid(frame_duration)`; boundary: `ticks.rem_euclid(frame_duration) == 0`.
- `TimeRange` intersection: `start = max(starts)`, `end = min(ends)`, `Some` only when `start < end`; an empty operand needs no separate check because it has `start == end`, so the single test covers it (the earlier non-empty check was removed in `15eb6cd`, behaviour-preserving). The duration cannot overflow because it is bounded by an operand's duration.
- Timecode NDF: label == real frame index; `to_flicks = label * frame_duration`.
- Timecode DF: the bidirectional mapping in `spike.md` C4, applied symmetrically to the magnitude for negative labels; skipped-label validation is `minutes % 10 != 0 && seconds == 0 && frames < drop`.
- Timecode fields: `hh` unbounded, `mm < 60`, `ss < 60`, `ff < nominal_fps`; parse uses checked arithmetic and rejects malformed shapes and contradictory separators.
- Timecode canonical text (owner decisions Q1 = A and Q2 = A, `specs/timecode` "Timecode text is canonical"): `mm` and `ss` have exactly two digits; `hh` and `ff` have at least two digits with no leading zero beyond that minimum (a value below 10 has one leading zero, 10 or more has none); a `-` appears only before a label whose frame count is not zero. `format()` emits only this text, and `parse()` rejects any text that formatting cannot produce, so a parsed text formats back unchanged. Negative zero (`-00:00:00:00`, `-00:00:00;00`) is rejected.

## Delivery table

All deliveries are `serial`: they share the crate, `lib.rs` re-exports and the workspace lockfile. Estimates include tests and apply the x1.4 undershoot factor from `AGENTS.md`.

| Delivery | Scope | Specs covered | Depends on | Mode | Est. lines |
|---|---|---|---|---|---|
| D0 | Crate scaffold and shared wiring: `crates/gafira-time/Cargo.toml`, canonical `clippy.toml` copy, `src/lib.rs` with `#![forbid(unsafe_code)]` and crate docs, `gafira-time` in root `[workspace.dependencies]`, `Cargo.lock`, V5 row in `.github/workflows/ci.yml`, status row in `docs/specs/README.md` | `time-model` purity (D0) | - | serial | 80 |
| D1 | `error.rs` + `flicks.rs` + `tests/flicks.rs`: `TimeError`, `Flicks`, constants, exact constructors, floor, `f64` display, checked arithmetic, transparent serde | `time-model` flicks, construction, arithmetic, display, serde (D1) | D0 | serial | 450 |
| D2 | `rate.rs` + `tests/rates.rs`: `FrameRate` (normalization, validation, constants, nominal fps), `SampleRate`, frame and sample conversions on `Flicks`, serde for both rates | `time-model` frame rates, frame alignment, sample alignment, serde (D2) | D1 | serial | 520 |
| D3 | `range.rs` + `tests/ranges.rs`: `TimeRange`, invariants, half-open semantics, intersection, serde | `time-model` ranges, half-open semantics, serde (D3) | D1 | serial | 360 |
| D4 | `timecode.rs` + `tests/timecode.rs`: `TimecodeMode`, `TimecodeFormat`, NDF and DF labels, parse/format, conversions, negative and beyond-24 hours, reference vectors and property tests | `timecode` all requirements (D4) | D1, D2 | serial | 650 |

D3 and D4 do not depend on each other, but both are serial because they touch the same crate and `lib.rs`; the coordinator does not need a second worktree. Total estimate about 2,060 lines including tests.

## Gates per delivery

`cargo fmt --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo deny check`; and, locally (GitHub Actions is disabled), the `policy` job steps: ARQ-07 `cargo tree` for `gafira-cli`, `forbid(unsafe_code)` scan, V5 layer table, V12 clippy-copy sync, V13 `EditToken` scan, `cargo fmt --check`. Mutation tier T3 with `cargo mutants -p gafira-time --file <each changed source file> --jobs 4 -o <scratch outside the repo>`; each run uses its own `CARGO_TARGET_DIR`, never shared between mutated trees.

## Risks

| Risk | Mitigation |
|---|---|
| Drop-frame off-by-one at minute blocks and negative labels | Reference vectors from `spike.md` C4, exhaustive round-trip property over 20 blocks, negative-label cases |
| Overflow hidden by `i64` products | Checked arithmetic everywhere; `i128` in rate validation; explicit overflow tests at the extremes |
| Serde bypassing invariants | Private wire structs and `try_from`; invalid-JSON tests for every validated type |
| Timecode becoming a second time base | `Timecode` has no serde and `to_flicks` is the only bridge; the spec states flicks are authoritative |
| Non-integral rates sneaking in | Construction-time integrality check; rejected-rate tests (11 fps, 23.98) |
| Delivery size above the ~700 ceiling | D4 is the largest at ~650; if measured above ~700, stop and ask the owner before continuing |

## Open questions

None. `time-model` and `timecode` close every behavior this design needs; the one rule discovered while designing (nominal frame rate = nearest integer, ties up) is added to `timecode` before implementation. No owner decision is reopened.
