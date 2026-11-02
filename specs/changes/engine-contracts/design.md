# Design: `engine-contracts`

`engine-contracts` adds the message types, the shared engine contracts and `IRect`, then retrofits `LayoutError` and `PathOpError` to `ToMessage`. It unblocks the four remaining layer-2 crates (`specs/changes/layer-2/design.md`).

Evidence: 02 §4.17, §6.3, §7.3, §14.2, ADR 0005, ADR 0007, `crates/gafira-doc/src/msg.rs`, `crates/gafira-layout/src/error.rs`, `crates/gafira-pathops/src/error.rs`, `.github/workflows/ci.yml`.

## Quick path

1. ADR 0012 is accepted (O1, resolved) and XP1 is confirmed.
2. Coordinator records the G0 entry with the "Acceptance" list below.
3. `tdd-apply` runs D1. Then D2a and D2b in order, and D3 in parallel in its own worktree.

Flow line: `Flow: full | level: L2 | new capability | mutation: archive | T4 | V2 | decided-by: owner | reason: parallel deliveries in layer 2`.

## Scope

| In | Out |
|---|---|
| `gafira_doc::msg`: `Message`, `MsgArg`, `Unit`, `ToMessage`, `msg!` (02:1847, 02:1871-1890) | `UserError` (engine, 02:1299) |
| `gafira_doc::contract`: `Describe`, `Descriptor`, `ParamSchema`, `ParamValues`, `CancelToken`, `Progress`, `AudioCrossfade` | Catalogs `assets/i18n/*.toml`, `xtask check-i18n` |
| `gafira_geom::IRect` | `gafira-ui` lint |
| `ToMessage` for `LayoutError`, `PathOpError` | `EffectInstance`, `AnimatableParams`, any wire form |
| `pathops -> doc` edge and its V5 row (ADR 0007) | `Registry<T>` (`gafira-ports`) |

## Decisions at a glance

| # | Open point | Decision |
|---|---|---|
| C1 | `msg!` body and check | `macro_rules!` at the crate root; a `const` assertion over `msg::is_valid_key` (XP1 confirmed) |
| C2 | Key grammar | Two or three dot-separated segments, each `[a-z][a-z0-9_]*`, from 02 §14.2 |
| C3 | `Progress` methods | One method, `report(&self, fraction: f32)`; `Send + Sync` supertraits |
| C4 | `CancelToken` shape | `Arc<AtomicU64>` plus the generation the handle is bound to |
| C5 | Cancelled error | Shared `Cancelled` value; each cancellable engine error wraps it (ADR 0012) |
| C6 | `Descriptor` fields | `id`, `name`, `version`; `#[non_exhaustive]`, built by `const fn new` |
| C7 | `ParamSchema` variants | `Float`, `Int`, `Bool`, `Choice`, `Color` |
| C8 | `ParamValues` | `BTreeMap<String, ParamValue>`; no serde now |
| C9 | `AudioCrossfade` | One variant, `EqualPower` |
| C10 | `IRect` | `i32` corners, half-open, signed; plain data |
| C11 | Key names and arguments | `error.<area>.<variant>` (ADR 0012); tables below |
| C12 | Who validates a `ParamSchema` | A precondition, not a run-time check: a test per compiled schema, the plugin adapter for the rest |

## Public API

These signatures are the contract for `tdd-apply`. Derives are exact. Doc comments and private helpers are free.

### `gafira_doc::msg` (D1)

`MessageKey` and `CommandLabel` stay as they are (`crates/gafira-doc/src/msg.rs`).

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unit { Px, Mm, Cm, In, Pt, Percent, Bytes, Seconds, Fps, Db, Lufs, Degrees }

#[derive(Clone, Debug, PartialEq)]
pub enum MsgArg {
    Text(String),
    Count(u64),
    Int(i64),
    Decimal(f64),
    Quantity(f64, Unit),
    Timestamp(i64),
    Elapsed(i64),
    Key(MessageKey),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub key: MessageKey,
    pub args: Vec<(&'static str, MsgArg)>,
}

pub trait ToMessage {
    fn to_message(&self) -> Message;
}

/// True for two or three dot-separated segments of `[a-z][a-z0-9_]*`.
#[must_use]
pub const fn is_valid_key(key: &str) -> bool;
```

`msg::Unit` is not `gafira_geom::Unit`: it holds display units and has no conversion. No `From` between them until a caller needs one.

The macro, at the crate root, so `gafira_doc::msg!` and `use gafira_doc::msg;` work as `std::vec` does for module and macro:

```rust
#[macro_export]
macro_rules! msg {
    ($key:literal $(, $name:ident = $variant:ident ( $($value:expr),* $(,)? ))* $(,)?) => {{
        const _: () = assert!($crate::msg::is_valid_key($key));
        $crate::msg::Message {
            key: $crate::msg::MessageKey($key),
            args: ::std::vec![$((::core::stringify!($name), $crate::msg::MsgArg::$variant($($value),*))),*],
        }
    }};
}
```

It accepts 02's form, `msg!("export.pages_selected", count = Count(n))`, with bare variant names. The assertion has no message on purpose: ARQ-107 forbids literals other than keys in `gafira-doc::msg`.

### `gafira_doc::contract` (D2a, D2b)

Layout: `contract/mod.rs` (re-exports, `AudioCrossfade`), `contract/describe.rs`, `contract/params.rs`, `contract/cancel.rs`. `lib.rs` gains `pub mod contract;`. The import rule of ADR 0005 applies: `crate::msg`, `gafira_color`, `serde`, `std` only.

D2a:

```rust
pub trait Describe {
    fn descriptor(&self) -> &Descriptor;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Descriptor {
    pub id: &'static str,
    pub name: MessageKey,
    pub version: u32,
}

impl Descriptor {
    #[must_use]
    pub const fn new(id: &'static str, name: MessageKey, version: u32) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCrossfade { EqualPower }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParamChoice {
    pub id: &'static str,
    pub label: MessageKey,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParamKind {
    Float { min: f64, max: f64, default: f64 },
    Int { min: i64, max: i64, default: i64 },
    Bool { default: bool },
    Choice { options: &'static [ParamChoice], default: &'static str },
    Color { default: Color },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamDef {
    pub id: &'static str,
    pub label: MessageKey,
    pub kind: ParamKind,
    pub animatable: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParamSchema { /* params: Vec<ParamDef> */ }

impl ParamSchema {
    #[must_use]
    pub fn new(params: Vec<ParamDef>) -> Self;
    #[must_use]
    pub fn params(&self) -> &[ParamDef];
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ParamDef>;
    #[must_use]
    pub fn defaults(&self) -> ParamValues;
    pub fn validate_definition(&self) -> Result<(), ParamError>;
    pub fn check_values(&self, values: &ParamValues) -> Result<(), ParamError>;
    pub fn resolve(&self, values: &ParamValues) -> Result<ParamValues, ParamError>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    Choice(String),
    Color(Color),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParamValues { /* values: BTreeMap<String, ParamValue> */ }

impl ParamValues {
    #[must_use]
    pub fn new() -> Self;
    pub fn set(&mut self, id: impl Into<String>, value: ParamValue) -> Option<ParamValue>;
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ParamValue>;
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ParamValue)>;
    #[must_use]
    pub fn float(&self, id: &str) -> Option<f64>;
    #[must_use]
    pub fn int(&self, id: &str) -> Option<i64>;
    #[must_use]
    pub fn bool(&self, id: &str) -> Option<bool>;
    #[must_use]
    pub fn choice(&self, id: &str) -> Option<&str>;
    #[must_use]
    pub fn color(&self, id: &str) -> Option<Color>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamError {
    NonFinite { param: &'static str },
    OutOfRange { param: &'static str },
    WrongType { param: &'static str },
    BadDefinition { param: &'static str },
}
// Display (English, log only), std::error::Error, ToMessage.
```

Rules of `params.rs`:

**Domain.** `get`, `defaults`, `check_values` and `resolve` are defined only for a schema that passed `validate_definition`. On any other schema their result is unspecified and no test pins it. They do not run the check themselves (C12). `new`, `params` and `validate_definition` are defined for every schema.

| Function | Rule |
|---|---|
| `new`, `ParamValues::set` | Plain data, no validation (ARQ-110) |
| `validate_definition` | `BadDefinition` for: repeated `id`; non-finite bound or default; `min > max`; default outside `[min, max]`; a `Choice` default not in `options`, or a repeated option id; a non-finite `Color` default. First fault in definition order |
| Caller of `validate_definition` | A schema compiled into the build: one test per schema in the crate that implements `Effect`, `Transition` or `Exporter`, asserting `Ok`, added by the delivery that adds the schema. No run-time call. A schema from outside the build: the `gafira-plugin` adapter (02 §4.12), once, when it wraps the contribution and before it is registered; on `Err` the plugin is disabled and the document stays intact (02 §7.3, row "Plugin") |
| `get` | The definition with this id, or `None` |
| `defaults` | Every parameter at its default, so `resolve` on empty values equals `defaults` |
| `check_values` | Walks the schema in definition order and stops at the first fault. A missing id passes. The kind decides first: a value of another kind is `WrongType` whatever it holds, NaN and `±∞` included. Only a value of the parameter's kind goes on. Non-finite `Float` or `Color` is `NonFinite`, checked before range. Outside `[min, max]`, inclusive, or a `Choice` not in `options`, is `OutOfRange` |
| Unknown ids | Ignored and kept whatever they hold, so `resolve` returns them, NaN and `±∞` included. A saved filter from a newer build keeps its data, as ADR-02 does for extensions. ARQ-110 does not forbid it: it binds layer-2 engines, `resolve` is layer 0, an engine reads only the ids of its own schema, and no trait in 02 §4.4 or §4.13 returns `ParamValues`, so such a value reaches no calculation and no engine output (SP16) |
| `resolve` | `check_values`, then fills each missing id with its default. In `Ok`, every id of the schema is present, of its kind and inside its range, and every `Float` and `Color` among them is finite |
| Accessors `float`..`color` | `None` when absent or of another kind; no conversion between `Int` and `Float` |

D2b:

```rust
#[derive(Clone, Debug, Default)]
pub struct CancelToken { /* generation: Arc<AtomicU64>, issued: u64 */ }

impl CancelToken {
    #[must_use]
    pub fn new() -> Self;
    /// Starts a new generation. Every token bound to an older one is cancelled.
    #[must_use]
    pub fn next_generation(&self) -> CancelToken;
    /// Cancels this token's generation. A no-op from a stale token.
    pub fn cancel(&self);
    #[must_use]
    pub fn is_cancelled(&self) -> bool;
    pub fn check(&self) -> Result<(), Cancelled>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancelled;
// Display (English, log only), std::error::Error, ToMessage.

pub trait Progress: Send + Sync {
    /// `fraction` is non-decreasing in `[0, 1]`; 1.0 only on success (ARQ-112).
    fn report(&self, fraction: f32);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoProgress;

/// Checks `cancel`, then reports. On `Err` it reports nothing.
pub fn checkpoint(
    cancel: &CancelToken,
    progress: &dyn Progress,
    fraction: f32,
) -> Result<(), Cancelled>;
```

Token rules: `Clone` shares the atomic and keeps `issued`. `is_cancelled` is `load != issued`. `next_generation` is `fetch_add(1)`. `cancel` is `fetch_max(issued + 1)`, so a stale handle cannot cancel a newer generation. All use `Ordering::Relaxed`: the token guards no data, and the only requirement is that the flag is seen eventually.

### `gafira_testkit::progress` (D2b)

The test double of ARQ-112, reused by every engine with a long operation. Used from integration tests only (`crates/gafira-testkit/src/lib.rs`).

```rust
pub struct ProgressLog { /* Mutex<Vec<f32>> */ }
impl ProgressLog {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn fractions(&self) -> Vec<f32>;
}
impl Progress for ProgressLog { .. }

#[derive(Clone, Debug, PartialEq)]
pub enum ProgressViolation {
    Decreasing { at: usize },
    OutOfRange { at: usize },
    EarlyCompletion { at: usize },
    NotCompleted,
    StepTooLarge { at: usize },
    NotCancelled { k: usize },
    OutputAfterCancel { k: usize },
    ReportsAfterCancel { k: usize, extra: usize },
}

/// Runs `op` once for the report count `n`, then cancels at report 1, `n / 2`
/// and `n - 1`. `op` returns whether it ended in the engine's cancelled error;
/// a successful run returns `Ok(false)`, and any other failure is the caller's.
pub fn check_progress_contract<E>(
    max_step: f32,
    op: impl Fn(&CancelToken, &dyn Progress) -> Result<bool, E>,
) -> Result<Result<(), ProgressViolation>, E>;
```

Tests give the checker one conforming fake and one fake per violation.

### `gafira_geom::IRect` (D1)

New `crates/gafira-geom/src/irect.rs`, exported from `lib.rs`.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    #[must_use]
    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self;
    #[must_use]
    pub const fn width(self) -> u32;
    #[must_use]
    pub const fn height(self) -> u32;
    #[must_use]
    pub const fn area(self) -> u64;
    #[must_use]
    pub const fn is_empty(self) -> bool;
    #[must_use]
    pub const fn contains(self, x: i32, y: i32) -> bool;
    #[must_use]
    pub fn intersect(self, other: IRect) -> IRect;
    #[must_use]
    pub fn inflate(self, dx: i32, dy: i32) -> IRect;
}
```

| Rule | Value |
|---|---|
| Interval | Half-open: `x0 <= x < x1` |
| Inverted rect | Legal plain data. `width` and `height` are 0, `is_empty` is true, `contains` is false |
| `width` | `x1.abs_diff(x0)` when `x1 > x0`, else 0, so `i32::MIN..i32::MAX` is `u32::MAX` with no overflow |
| `intersect` | Max of the lows, min of the highs. Disjoint inputs give an empty rect, possibly inverted |
| `inflate` | Saturating, moves each side outward by `dx` or `dy`. A negative value deflates |

Kept out until a caller exists: `union`, conversion to or from `Rect`. No serde.

## Key tables (C11)

Argument names are stable: they are the catalog's `{parameters}`.

### `LayoutError`

| Variant | Key | Arguments |
|---|---|---|
| `NonFinite { field }` | `error.layout.non_finite` | `field = Text` |
| `InvalidBox { field }` | `error.layout.invalid_box` | `field = Text` |
| `OutOfRange { field }` | `error.layout.out_of_range` | `field = Text` |
| `NoArea` | `error.layout.no_area` | none |
| `UnknownNode(id)` | `error.layout.unknown_node` | `node = Text(id.to_string())` |
| `Cycle(id)` | `error.layout.cycle` | `node = Text(id.to_string())` |

### `PathOpError`

| Variant | Key | Arguments |
|---|---|---|
| `NonFiniteCoordinate { .. }` | `error.pathops.non_finite_coordinate` | none |
| `CoordinateOutOfRange { value, limit }` | `error.pathops.coordinate_out_of_range` | `value = Decimal`, `limit = Decimal` |
| `OpenSubpath { subpath }` | `error.pathops.open_subpath` | `subpath = Int` (`i64::try_from`, saturating at `i64::MAX`) |
| `InvalidStrokeWidth { NotPositive(v) }` | `error.pathops.stroke_width_not_positive` | `value = Decimal` |
| `InvalidStrokeWidth { NonFinite(_) }` | `error.pathops.stroke_width_non_finite` | none |
| `InvalidTolerance { NotPositive(v) }` | `error.pathops.tolerance_not_positive` | `value = Decimal` |
| `InvalidTolerance { NonFinite(_) }` | `error.pathops.tolerance_non_finite` | none |
| `ToleranceTooFine { minimum }` | `error.pathops.tolerance_too_fine` | `minimum = Decimal` |
| `Geometry(_)` | `error.pathops.geometry` | none |

The NaN or infinity class and the `Geometry` string stay in `Display`, for the local log. A person cannot act on them, and the string is English text from a library, which ARQ-102 keeps out of messages.

### Shared contract errors

| Type | Key | Arguments |
|---|---|---|
| `Cancelled` | `error.job.cancelled` | none |
| `ParamError::NonFinite` | `error.param.non_finite` | `param = Text` |
| `ParamError::OutOfRange` | `error.param.out_of_range` | `param = Text` |
| `ParamError::WrongType` | `error.param.wrong_type` | `param = Text` |
| `ParamError::BadDefinition` | `error.param.bad_definition` | `param = Text` |

Both `to_message` bodies are exhaustive `match`es with no `_` arm, so a new variant does not compile until it has a key. `#[non_exhaustive]` on `PathOpError` does not force a wildcard inside its own crate.

## Decisions

Short form. ADR 0012 holds the cross-cutting part of C5 and C11.

### C1. `msg!` and its check

- Status: decided; XP1 confirmed it.
- Options: (a) `macro_rules!` with a const check; (b) a proc-macro crate; (c) no check, leaving it to `xtask check-i18n`.
- Decision: (a). It fails at compile time, needs no new crate, and the check is a plain `const fn` that unit tests can call. `xtask` does not exist yet, so (c) checks nothing today. (b) adds a crate, a V5 row and `syn`.
- `MessageKey("...")` stays public: 21 distinct keys use it today (`git grep` over `crates`), and `const` labels need it. That conflicts with "única forma" in 02:1889 (SP2).

### C3. `Progress`

- Decision: `&self`, because every caller holds `&dyn Progress` (02:562, 02:602). `Send + Sync`, because `ClassicSegmenter` runs on a pool. `f32`, as `EngineEvent::JobProgress` (02:1037). Named `report` after "informe" in ARQ-112.
- `checkpoint` puts the ARQ-112 order in one place: a double that cancels inside report `k` leaves 0 reports after `k`, within the limit of 1.
- Rejected: `&mut self` makes `dyn Progress` unusable from parallel workers.

### C4. `CancelToken`

- Context: 02:1145 fixes `AtomicU64` generations and says a new interaction invalidates the previous generation. It leaves `Clone` and the new-generation API open.
- Decision: one type, as signatures take only `&CancelToken`. The `Arc` makes clones share the generation, and `issued` binds a handle to it.
- Rejected: a separate `CancelSource` adds a second type that no spec names. A `bool` flag cannot express a generation. `fetch_add` in `cancel` would let a stale handle cancel the running generation.
- All three targets are 64-bit with native `AtomicU64`.

### C5. Cancelled shape

- Options: (a) a shared `Cancelled` plus one wrapping variant per engine error; (b) an unrelated `Cancelled` variant per enum, as today's "one enum per crate" implies; (c) a single shared error enum.
- Decision: (a). `CancelToken::check` has one return type, `?` works through `From<Cancelled>`, and there is one key. (c) contradicts 02:1298.
- `LayoutError` and `PathOpError` gain no variant: they have no long operation.

### C6. `Descriptor`

- Context: 02:429 says "id estable, nombre, versión, capacidades". Capabilities have no consumer, and each trait already exposes its own typed form: `gpu()`, `requirements()`, `caps()`.
- Decision: leave `capabilities` out. `#[non_exhaustive]` and `new` keep adding it later source compatible. `id` is `&'static str` because `Registry::by_id` keys on it (02:423). `name` is a `MessageKey` (ARQ-98). SP3 aligns 02.

### C7. `ParamKind`

- Sources: the five kinds cover FD-33 and FD-35 sliders and colours, Wipe direction and softness (ARQ-118), and integer filter or export options. 02:523 asks for types, ranges, defaults and animability, and nothing more.
- `f64` for `Float`: the persisted form will be JSON with `float_roundtrip` (`Cargo.toml`), and engines narrow to `f32` where they compute.
- Left out: unit and slider step. The Inspector does not exist; a new field is a mechanical edit inside the workspace.

### C8. `ParamValues` and serde

- Context: ADR 0005 says it will be persisted, through `EffectInstance` and saved filters. No such field exists. `EffectInstance` holds `AnimatableParams` (02:533), which has no definition yet.
- Options: (a) derive serde now; (b) add it with the first persisted owner.
- Decision: (b). A derive now fixes a JSON form with no reader and no golden, and may not match `AnimatableParams`. ADR 0006 keeps schema 1 open, and a later derive changes no public signature, so waiting costs nothing. This change adds no format surface.
- `BTreeMap`: domain crates ban `HashMap`, and iteration order must be deterministic (ARQ-111). `String` keys: a deserialized key cannot be `&'static str`.

### C9. `AudioCrossfade`

- Decision: one variant. 02:740 and ARQ-124 name only `EqualPower`, and FV-06 sets no other audio rule. Not `#[non_exhaustive]`, so `gafira-audio` matches it exhaustively and a new variant fails to compile there.

### C10. `IRect`

- Decision: signed, because a blur `roi` can leave the image (ARQ-120). `i32`, because tile and page coordinates are far below it. Corners rather than origin and size, to mirror `gafira_geom::Rect`. Half-open, so tiles abut without overlap (ARQ-121). Constructors do not validate (ARQ-110).

### C12. Who validates a `ParamSchema`

- Status: decided, from W2 and W3 of the D2a verify.
- Context: `new` cannot validate (ARQ-110). A schema is written once and read on every `run_cpu`. `Registry<T>` is bound only by `Describe` (02:424), so it cannot see `params()`.
- Options: (a) a precondition, met by a test per compiled schema and by the plugin adapter; (b) `check_values` and `resolve` call `validate_definition` each time; (c) a validated schema type from a fallible constructor; (d) `Registry::register` checks.
- Decision: (a). A fault in a compiled schema is a bug in our code, so `cargo test` is the right place to stop it and no user ever meets it. A plugin schema is foreign input, so it needs a real check, once.
- Rejected: (b) repeats a static check on every call, and `validate_definition` allocates (`seen`). (c) changes `fn params(&self) -> &ParamSchema` (02:523, 02:735) and `options_schema` (02:559). (d) needs a schema bound on a generic registry that also holds items with no schema.
- Consequence: a forgotten test is the remaining risk. SP17 puts the step in the "how to add" lists. Nothing calls `validate_definition` at run time until `gafira-plugin` exists (Phase 3).

## Delivery table

Mode rule: D2a and D2b share `contract/mod.rs` and `lib.rs`. D3 touches only `gafira-pathops`, `gafira-layout`, `ci.yml` and `Cargo.lock`, and no other delivery touches those, so it can run beside them. D1 comes first because all three use its types.

| Delivery | Scope | Specs covered | Depends on | Mode | Est. lines |
|---|---|---|---|---|---|
| D1 | `msg` types, `is_valid_key`, `msg!`; `IRect` | ARQ-102 (types), ARQ-107 | None | serial | 370 (x1.4: 520) |
| D2a | `contract`: `Describe`, `Descriptor`, `AudioCrossfade`, params, `ParamError` | ARQ-98 (labels), ARQ-110 (params) | D1 | serial | 360 (x1.4: 505) |
| D2b | `CancelToken`, `Cancelled`, `Progress`, `NoProgress`, `checkpoint`, `gafira_testkit::progress` | ARQ-112 (on doubles) | D2a | serial | 270 (x1.4: 380) |
| D3 | `ToMessage` for both errors; `gafira-pathops -> gafira-doc`; V5 row `gafira-pathops gafira-geom gafira-color gafira-doc` | ARQ-102 (engines) | D1 | parallel | 200 (x1.4: 280) |

Total forecast 1 200 lines (x1.4: 1 685). D3 merges in any order relative to D2a and D2b. The V5 row, the `Cargo.toml` line and `Cargo.lock` land in one commit (ADR 0007).

## Acceptance

Coordinator: copy to the G0 record. Each item is testable now.

- [ ] Every `MsgArg` variant and every `msg::Unit` builds; `Message` is `Clone` and `PartialEq`.
- [ ] `msg!` with 0, 1 and 2 arguments, and a trailing comma, gives those names in order. An invalid key does not compile (`compile_fail` doctest).
- [ ] `is_valid_key` table: accepts `edit.set_property`, `error.format.too_new`, `a.b2`; rejects `""`, `a`, `.a`, `a.`, `a..b`, `a.b.c.d`, `A.b`, `a.2b`, `a b.c`, `a.é`.
- [ ] ARQ-102, per-variant fixtures: every variant of both errors gives its table key and argument names; keys are valid and distinct within the error; the test never calls `Display`.
- [ ] ARQ-112 on doubles: `check_progress_contract` passes the conforming fake and names each violation.
- [ ] `CancelToken`: a clone sees `cancel`; `next_generation` cancels older tokens, and the new token is live; a stale `cancel` leaves the new generation live; `cancel` twice is harmless; the type is `Send + Sync + Clone`.
- [ ] `checkpoint`: on a cancelled token, `Err` and no report; otherwise one report and `Ok`.
- [ ] `ParamSchema`: each rule of the table above, with one fixture per fault and per bound, boundary values included.
- [ ] `IRect`: the rules above, including `i32::MIN..i32::MAX` width, saturation, and inverted rects.
- [ ] V5 rows pass; `gafira_doc::contract` imports only what ADR 0005 allows; `git grep Locale` finds nothing in `gafira-doc`.

Still pending after this change, by design (ARQ-102 "Cumplimiento"): catalog presence, UI text, the `gafira-ui` lint, `UserError`. ARQ-112 on a real operation arrives with `gafira-segment`.

## Critical code

Hand mutants and T4 apply. T4 at close: `gafira-doc`, `gafira-geom`, `gafira-pathops`, `gafira-layout`, `gafira-testkit`.

| Code | Why | Delivery |
|---|---|---|
| `is_valid_key` | Validation | D1 |
| `IRect` arithmetic | Size limits, overflow | D1 |
| `ParamSchema` checks | Validation, ARQ-110 | D2a |
| `CancelToken` generation rule, `checkpoint` | Invariant, ARQ-112 | D2b |
| `to_message` tables | Spec rule, ARQ-102 | D3 |

`Display` text is not critical. The `gafira_testkit::progress` checker is a test helper, but its negative controls are still required.

## Open questions (owner)

| # | Question | Options | Recommendation |
|---|---|---|---|
| O1 | Accept ADR 0012? | (a) Accept. (b) Keep `Text` out of technical identifiers. (c) One unrelated `Cancelled` per enum | Resolved: (a), owner confirmation 2026-11-01 |

## Needs explorer

| # | Claim | Check | Blocks |
|---|---|---|---|
| XP1 | `msg!` above compiles from a second crate, and a `const _` assertion over `is_valid_key` rejects a bad literal | Measured and confirmed on rustc 1.93.1: consumer crate build exit 0; a bad key gives E0080, build exit 101; the `compile_fail` doctest passes; clippy with the repo lints exit 0; no author message text in the macro | Nothing |

## Needs specifier

| # | Where | Edit | Blocks |
|---|---|---|---|
| SP2 | 02:1876, 02:1889, 02 §14.2 | `msg!` is the way to build a `Message`; `MessageKey` literals stay legal for labels. Say the key has 2 or 3 segments. Widen `Text`: stable technical identifiers also travel there, untranslated | None |
| SP3 | 02:429 | `Descriptor` has `id`, `name`, `version`; capabilities live on each trait | None |
| SP4 | 02:1298, 02 §9.2 | Errors are hand-written: no crate uses `thiserror` and `Cargo.lock` does not list it | None |
| SP16 | ARQ-110 (02:866) | "Sus números" does not say whether an id the schema does not know counts. Say it does not: it is kept untouched and the engine never reads it. Not a contradiction, the rule above already follows this reading | None |
| SP17 | 02 §4.4, §4.5, §4.12, §4.13, "Cómo añadir" lists | Add the step of C12: a test that the schema passes `validate_definition`; the plugin adapter checks foreign schemas | None |
| SP18 | ARQ-110 (02:866), 02:524-525, 02:530, 02:742 | `roi`, `cost_hint` and both `bind` take `ParamValues` and return no `Result`, so they cannot give the `Err` that ARQ-110 asks of every public function, and its only exception is `AudioProcessor::process`. Options: give them a `Result`, or name them as exceptions and require values that passed `resolve` | `RS4` |

SP numbers are shared with `specs/changes/layer-2/design.md`.

## Next

`tdd-apply` for D1.
