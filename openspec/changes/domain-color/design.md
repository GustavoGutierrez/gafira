# Design: Domain Color (C2a)

## Technical Approach

Add `crates/gafira-color`, a layer-0 crate that is pure and serializable. Its only normal dependency is `serde`. It contains:
- four value types: `Rgba` (encoded sRGB), `LinearRgba`, `DisplayP3Rgba` and `ColorSpaceId`;
- one wire enum, `Color`;
- analytic conversions (sRGB transfer curve plus 3×3 matrices);
- the `ColorTransform` trait with a single implementation, `AnalyticTransform`.

The crate follows the `gafira-geom` patterns:
- `#![forbid(unsafe_code)]`;
- a `clippy.toml` that is a byte-identical copy of geom's (V12);
- constructors that do not validate (C1 D8);
- IEEE `PartialEq` on floats, with bit comparison in tests;
- hand-written errors (C1 D12).

CI step V5 becomes a table of allowed dependencies per crate.

Requirements come from `specs/{color-model,color-conversions,workspace-tooling}/spec.md`. Where the specs and this design state a number differently (reference values, tolerances, the V5 forbidden pattern), the corrected values here are the ones to carry into the specs; see "Spec alignment" below.

**Design-time cut (already triggered).** The proposal's cut line is "if measured lines exceed ~650". The projected size without `Lab`/`Cmyk` is already ~890 measured lines (see Budget), so **`Lab` and `Cmyk` move to the change that first needs them**. Notes for adding them back are in D12.

## Architecture Decisions

| # | Decision | Choice | Rejected | Rationale |
|---|---|---|---|---|
| D1 | Transfer curve | IEC 61966-2-1:1999 sRGB curve, written on the absolute value and re-signed with `copysign`. Decode: `|c| ≤ 0.04045 → c/12.92`, else `copysign(((|c|+0.055)/1.055)^2.4, c)`. Encode: `|l| ≤ 0.0031308 → 12.92·l`, else `copysign(1.055·|l|^(1/2.4) − 0.055, l)`. This gives the **sign-symmetric extension** `f(−x) = −f(x)` of CSS Color 4 §"extended transfer". Display-P3 uses the same curve. | A pure 2.2 gamma; a curve that clamps at 0 and 1; branching on `c < 0` | The curve is the published standard. Without the extension, P3 → sRGB → P3 cannot round-trip, because out-of-gamut colors have negative channels. Testing `|c|` makes a negative input take exactly the mirror branch of its positive counterpart, so the symmetry is exact (P5). `-0.0` stays `-0.0`: it takes the linear branch, and `-0.0/12.92` and `12.92·(-0.0)` keep the sign. |
| D2 | Matrices | Store the CSS Color 4 *rational* matrices as `const` f64 expressions: `lin_sRGB↔XYZ` and `lin_P3↔XYZ` (D65). The composites `SRGB_TO_P3 = XYZ_TO_P3·SRGB_TO_XYZ` and `P3_TO_SRGB` are computed by a `const fn mul` (float arithmetic in `const fn` is stable since 1.82, and MSRV is 1.93). Table below. | 17-digit decimal literals; deriving the matrices from chromaticities at compile time (needs a const 3×3 inverse, ~40 extra lines) | Rationals come straight from the published text, are more precise than decimals, and avoid `clippy::excessive_precision` under `-D warnings`. Transcription errors are caught by the tests for M·M⁻¹ = I and for white-point row sums (T4). |
| D3 | Precision | Types store `f32` (proposal). Every conversion widens to `f64`, does the arithmetic, and narrows to `f32` once. | f32 arithmetic throughout | Gives one code path and tighter round trips. `powf` comes from the platform libm (UCRT, glibc, libSystem) and is not bit-identical across platforms, so all reference checks use tolerances (see the **Tolerances** table under Testing Strategy). Conversion results are never persisted (see the domain-doc notes). |
| D4 | Representation | `#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]`. Channel fields are `pub r, g, b, a: f32`. No `#[repr]`, no `Default`, no `Eq`/`Hash`/`Ord`. One private `macro_rules!` generates the three RGBA structs. | Private fields with accessors (like geom's `Length`); `#[repr(C)]`; deriving `Default` | The types have no invariant to protect, so public fields work like kurbo's `Point`. `#[repr(C)]` only helps zero-copy casts, which need `unsafe` or `bytemuck`; the bulk path is `apply_f32` on `&mut [f32]`, and the attribute can be added later without breaking anything. `Default` would silently mean transparent black. |
| D5 | Equality | Derived IEEE `PartialEq`: NaN ≠ NaN and `-0.0 == 0.0`. Tests that claim identity compare `to_bits()`. Variants are compared structurally, so `Srgb(x) != LinearSrgb(x)`. | Bit-pattern `PartialEq` | Same as geom's f64 policy, so domain-doc and edit get one rule. |
| D6 | NaN, infinities, out of gamut | Construction and conversion are **infallible** and **never clamp**. If any input channel is non-finite, the output channels are unspecified (NaN or inf) but the call never panics. Alpha is copied bit for bit by every conversion. `is_finite()` is the validation hook for C2c. serde_json writes NaN and inf as `null`, which fails to decode. | `Result` on conversion; clamping to [0,1] | Conversions run in pixel loops, where a `Result` per pixel is not acceptable. Gamut mapping is a separate future feature. Rejecting non-finite values is a document invariant (C2c), not a property of the color vocabulary. |
| D7 | `Color` | Internally tagged enum, `#[serde(tag = "space")]`, with variants `Srgb(Rgba)`, `LinearSrgb(LinearRgba)` and `DisplayP3(DisplayP3Rgba)`. The enum is **exhaustive** (not `#[non_exhaustive]`). An unknown `space` value is an **error**. A missing `space` or a missing channel (including `a`) is an error. Extra fields are ignored. | Adjacent tagging (`{"space":..,"value":{..}}`); `#[serde(other)]` (impossible with data); `#[non_exhaustive]` | The flat shape is a bare value, which is what `Animatable::Static` (untagged) needs: an object like `{"keys":[...]}` fails as `Color` because `space` is required, so the untagged fallback reaches `Keyed`. Rejecting unknown spaces matches geom's `"km"` rejection; a new space arrives with a format migration (`gafira-format`). An exhaustive enum makes the compiler flag every `match` when a space is added (ADR-02). Ignoring extra fields follows 02 §3.5. |
| D8 | Wire names | CSS Color 4 predefined-space identifiers: `"srgb"`, `"srgb-linear"`, `"display-p3"`. The same names serve `ColorSpaceId`. | `snake_case` (`linear_srgb`, `display_p3`) | A public, externally documented vocabulary that SVG export can emit directly. One vocabulary for both the tag and the id. |
| D9 | `ColorTransform` | Trait from 02 §4.10 with `src`, `dst` and `apply_f32`. **Deviation (PROPOSAL)**: `apply_f32` returns `Result<(), PixelBufferError>`. It errors when `len % channels != 0` and leaves the buffer untouched. `bake_lut3d` is **omitted**, with no placeholder `Lut3d`. `AnalyticTransform::new(src, dst)` is infallible because all 9 pairs are supported; `src == dst` leaves the buffer bit-identical. | 02's `()` return with a panic or a silently skipped remainder; a placeholder `Lut3d` | A panic conflicts with the future no-panic policy, and skipping the remainder hides corruption. Adding a *provided* `bake_lut3d` later does not break implementors or callers. A placeholder would freeze the LUT size, layout and domain before its GPU consumer exists. |
| D10 | `PixelLayout` | `enum PixelLayout { Rgb, Rgba }`: interleaved `f32`, straight (non-premultiplied) alpha, exhaustive, with `channels()`. | Depending on raster; adding premultiplied now | Layer 0 cannot depend on raster. Premultiplied pixels need unpremultiplying before the non-linear curve, so raster adds `RgbaPremultiplied` when it needs it, and the exhaustive enum forces every implementor to handle it. |
| D11 | No ICC / `moxcms` | Analytic transforms only. | `moxcms` now | No ICC input exists yet (no importer). Analytic math is exact for these three spaces. `moxcms` would be a new dependency (licence/MSRV review) that the proposal excludes. A future `IccTransform` implements the same trait. |
| D12 | `Lab`/`Cmyk` | Deferred (design-time cut). If they come back: Lab D50 per CSS Color 4 (Bradford D65→D50, ε = 216/24389, κ = 24389/27), and naive CMYK on clamped encoded sRGB (`K = 1 − max`), documented as NOMINAL. Neither is a `Color` variant. | Keeping them in C2a | Together they add ~180 measured lines. Nothing consumes them until print/PDF (Phase 2). Leaving them out of `Color` keeps the cut clean. |
| D13 | Internal API | The curve functions and matrices are `pub(crate)` in `math.rs`. | Making them public | ARQ-05 spirit. Exposing them later is additive. |

**D2 constants** (CSS Color 4, "Sample code for color conversions"; D65 = `[0.3127/0.3290, 1, (1−0.3127−0.3290)/0.3290]`). Display-P3 primaries come from SMPTE EG 432-1 with D65 white:

| Matrix | Rows |
|---|---|
| `SRGB_TO_XYZ` | `[506752/1228815, 87881/245763, 12673/70218]`, `[87098/409605, 175762/245763, 12673/175545]`, `[7918/409605, 87881/737289, 1001167/1053270]` |
| `XYZ_TO_SRGB` | `[12831/3959, −329/214, −1974/3959]`, `[−851781/878810, 1648619/878810, 36519/878810]`, `[705/12673, −2585/12673, 705/667]` |
| `P3_TO_XYZ` | `[608311/1250200, 189793/714400, 198249/1000160]`, `[35783/156275, 247089/357200, 198249/2500400]`, `[0, 32229/714400, 5220557/5000800]` |
| `XYZ_TO_P3` | `[446124/178915, −333277/357830, −72051/178915]`, `[−14852/17905, 63121/35810, 423/17905]`, `[11844/330415, −50337/660830, 316169/330415]` |

These were transcribed from memory without network access. Apply MUST check them against the spec text, and the T4 tests must pass.

## Data Flow

    Color ──space()──► ColorSpaceId
      │ convert(to) / to_linear_srgb()
      ▼
    [f32;3] ─widen─► f64 ─► to_linear_srgb(space): decode? ─► P3_TO_SRGB?
                              from_linear_srgb(dst): SRGB_TO_P3? ─► encode? ─narrow─► [f32;3]
    alpha ───────────────────────── copied bit-exact ─────────────────────────►
    AnalyticTransform::apply_f32(px, layout) ─► chunks_exact(channels) ─► same pipeline

Both the typed API and `apply_f32` use one private pipeline, `math::convert_rgb(src, dst, [f32; 3])`, so they give bit-identical results (property P6).

## File Changes

| File | Action | Content |
|---|---|---|
| `crates/gafira-color/Cargo.toml` | Create | Same `[package]` block as geom, with `name = "gafira-color"` and `version/edition/rust-version/license/publish.workspace = true`; `[dependencies] serde.workspace = true`; `[dev-dependencies] proptest.workspace = true`, `serde_json.workspace = true`; `[lints] workspace = true` |
| `crates/gafira-color/clippy.toml` | Create | `cp crates/gafira-geom/clippy.toml`, byte-identical (V12). Never edited by hand. |
| `crates/gafira-color/src/lib.rs` | Create | Crate docs (pure, f32 storage, NOMINAL scope), `#![forbid(unsafe_code)]`, modules, re-exports |
| `crates/gafira-color/src/space.rs` | Create | `ColorSpaceId` + tests |
| `crates/gafira-color/src/math.rs` | Create | Curve, `Mat3`, the D2 constants, `const fn mul`, `convert_rgb` + reference tests |
| `crates/gafira-color/src/rgb.rs` | Create | The macro and the three structs, typed conversions + tests |
| `crates/gafira-color/src/color.rs` | Create | `Color`, `From` impls + tests |
| `crates/gafira-color/src/transform.rs` | Create | `PixelLayout`, `ColorTransform`, `AnalyticTransform`, `PixelBufferError` + tests |
| `crates/gafira-color/tests/api.rs` | Create | Wire format seen from outside the crate |
| `crates/gafira-color/tests/properties.rs` | Create | proptest suite |
| `Cargo.toml` | Modify | +1 line `gafira-color = { path = "crates/gafira-color" }`. `members = ["crates/*", "apps/*"]` already covers the crate. |
| `.github/workflows/ci.yml` | Modify | Replace the V5 step (below). V11 and V12 already loop over `crates/*` and need no change. |

## Interfaces / Contracts (illustrative)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ColorSpaceId { #[serde(rename="srgb")] Srgb, #[serde(rename="srgb-linear")] LinearSrgb,
                        #[serde(rename="display-p3")] DisplayP3 }
impl ColorSpaceId { pub const ALL: [ColorSpaceId; 3]; }

pub struct Rgba { pub r: f32, pub g: f32, pub b: f32, pub a: f32 }        // encoded sRGB
pub struct LinearRgba { /* same */ }  pub struct DisplayP3Rgba { /* same, encoded P3 */ }
// each: pub const fn new(r, g, b, a) -> Self; pub fn is_finite(self) -> bool;
impl Rgba          { pub fn to_linear(self) -> LinearRgba; }
impl LinearRgba    { pub fn to_srgb(self) -> Rgba; pub fn to_display_p3(self) -> DisplayP3Rgba; }
impl DisplayP3Rgba { pub fn to_linear_srgb(self) -> LinearRgba; }

#[serde(tag = "space")] pub enum Color { Srgb(Rgba), LinearSrgb(LinearRgba), DisplayP3(DisplayP3Rgba) }
impl Color { pub const fn space(self) -> ColorSpaceId; pub const fn alpha(self) -> f32;
             pub fn convert(self, to: ColorSpaceId) -> Color;   // same space: bit-identical
             pub fn to_linear_srgb(self) -> LinearRgba; pub fn is_finite(self) -> bool; }
// From<Rgba>, From<LinearRgba>, From<DisplayP3Rgba> for Color

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelLayout { Rgb, Rgba }  impl PixelLayout { pub const fn channels(self) -> usize; }
pub trait ColorTransform: Send + Sync {
    fn src(&self) -> ColorSpaceId;  fn dst(&self) -> ColorSpaceId;
    fn apply_f32(&self, px: &mut [f32], layout: PixelLayout) -> Result<(), PixelBufferError>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct AnalyticTransform { /* private src, dst */ }
impl AnalyticTransform { pub const fn new(src: ColorSpaceId, dst: ColorSpaceId) -> Self; }
#[non_exhaustive] #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelBufferError { LengthNotMultiple { len: usize, channels: usize } }  // Display + Error
```

**Wire format (exact)**:
- `Color::Srgb(Rgba::new(1.0, 0.5, 0.0, 1.0))` → `{"space":"srgb","r":1.0,"g":0.5,"b":0.0,"a":1.0}`. The tag comes first, then the fields in declaration order (ARQ-10).
- A bare `Rgba` → `{"r":..,"g":..,"b":..,"a":..}`.
- `ColorSpaceId::DisplayP3` → `"display-p3"`.

**Not exposed**: the curve and matrix functions, XYZ, `Lab`, `Cmyk`, `Lut3d`/`bake_lut3d`, gamut mapping or clamping, interpolation (`gafira-anim` implements `Interpolate for Color`, which the orphan rule allows), hex parsing, palettes, ICC, any I/O.

## V5: table-driven layer check (`ci.yml`)

```bash
# crate  allowed-internal-deps ("-" = none). Add a row per domain crate; keep the step unchanged.
table='
gafira-geom  -
gafira-color -
'
forbidden='egui|eframe|winit|wgpu|tokio|mio|socket2|reqwest|ureq'
status=0
listed=$(awk 'NF{print $1}' <<<"$table")
for c in crates/*/clippy.toml; do            # domain crates = crates with a clippy.toml copy (D5/V12)
  name=$(basename "$(dirname "$c")")
  grep -qx "$name" <<<"$listed" || { echo "::error::$name has clippy.toml but no V5 row" >&2; status=1; }
done
while read -r crate allowed; do
  [ -n "$crate" ] || continue
  tree=$(cargo tree -p "$crate" -e normal,build --target all --prefix none -f '{p}' </dev/null)  # -e aborts on failure
  grep -q "^$crate v" <<<"$tree" || { echo "::error::empty cargo tree for $crate" >&2; status=1; continue; }
  for dep in $(grep -Eo '^gafira-[a-z0-9-]+' <<<"$tree" | sort -u); do
    [ "$dep" = "$crate" ] && continue
    case " $allowed " in *" $dep "*) ;; *) echo "::error::$crate must not depend on $dep" >&2; status=1;; esac
  done
  if bad=$(grep -E "^($forbidden)" <<<"$tree"); then echo "::error::$crate: forbidden $bad" >&2; status=1; fi
done <<<"$table"
exit "$status"
```

Safety under `bash -eo pipefail`:
- The `cargo tree` output is captured *before* any `grep`, as in C1.
- The loop runs in the main shell, fed by a here-string, so `status` survives the loop.
- `</dev/null` stops `cargo` from reading the table from stdin.
- The `if bad=$(...)` and `|| { }` forms keep `-e` from triggering on "no match".
- The `^$crate v` sanity line rejects an empty tree.
- The forbidden check is a **prefix** match, `^($forbidden)` with no trailing space. A package is forbidden when its name *starts with* a listed name, so `wgpu-core`, `tokio-util` and `winit-*` are caught too. An exact-name pattern (`^(…) `) would let those through. The C1 step matched anywhere on the line; the prefix form is anchored to the package name (it can no longer hit a path or version), but it still catches every derived crate. The spec MUST word this as "a package whose name starts with one of …", not "a package named …".

`gafira-doc` is added later as one row, for example `gafira-doc gafira-geom gafira-color gafira-time gafira-anim`.

**Manual negative tests** (T2). Copy the step body to `$SCRATCH/v5.sh`, where `$SCRATCH` is the session scratchpad, outside the repo. Each case is run with `bash --noprofile --norc -eo pipefail "$SCRATCH/v5.sh"; echo "exit=$?"`, then reverted with `git checkout -- crates/gafira-color Cargo.lock` (and `rm -r crates/scratch` for N5). For N6 and N7, run the V11 or V12 step body the same way. `git status` must be clean afterwards. The expected results are:

| Case | Mutation | Expected |
|---|---|---|
| N1 | Add `gafira-geom.workspace = true` to color's `[dependencies]` | exit 1, `gafira-color must not depend on gafira-geom`. The current geom-only step exits 0 on the same tree, which is the RED. |
| N2 | Row `gafira-colour -` | `cargo tree` fails ("did not match any packages"), the step exits non-zero, and the coverage check reports the missing `gafira-color` |
| N3 | Delete the `gafira-color` row | exit 1, "has clippy.toml but no V5 row" |
| N4 | Add `serde` to the `forbidden` pattern (no network needed) | exit 1, forbidden `serde v1…` and `serde_derive v1…` (the prefix match also catches the derived name) |
| N5 | Scratch crate `crates/scratch/` with a valid manifest and `#![forbid(unsafe_code)]`, but no `clippy.toml` and no row | exit 0. This is the **recorded known limitation** from the spec, not a requirement that is met: V5 identifies domain crates by their `clippy.toml`. The gap closes with `xtask check-layers` (ARQ-01). Remove the scratch crate afterwards. |
| N6 | Remove `#![forbid(unsafe_code)]` from `crates/gafira-color/src/lib.rs` and run the V11 loop | exit 1, `missing forbid(unsafe_code): crates/gafira-color/src/lib.rs` |
| N7 | Append a scratch line to `crates/gafira-color/clippy.toml` and run the V12 loop | `diff -q` reports a difference, non-zero exit |
| N0 | Clean tree | exit 0 |

## Testing Strategy (Strict TDD)

**Tolerances** (absolute bound `tol·max(1, |x|)` on `f32` output):

| Check | Tolerance |
|---|---|
| Curve round trips, inputs in [0,1] and [−4,4] | `2e-6` |
| Round trips through Display-P3, inputs in [0,1] | `1e-5` |
| Round trips through Display-P3, inputs in [−4,4] | `1e-4` (narrowing to f32 reaches ~1.01e-5, so `1e-5` has no margin) |
| Composition (a→b)∘(b→c) ≈ a→c (P8) | `1e-4` |
| Curve continuity at the thresholds (f64, private functions) | decode at 0.04045: `1e-8` (measured 2.3e-9); encode at 0.0031308: `5e-8` (measured 2.85e-8) |
| Published 4-decimal references | `1e-4` |
| f64 matrix identities | `1e-12` |
| Bit identity (same-space conversion, alpha, serde) | `to_bits()` |

**Reference values**:

| Value | Source |
|---|---|
| decode(0) = 0, decode(1) = 1, decode(0.5) ≈ 0.2140411 | IEC 61966-2-1 formula; independent f64 evaluation written into the test comment |
| Branch difference below `1e-8` at 0.04045 (decode) and below `5e-8` at 0.0031308 (encode) | IEC 61966-2-1 thresholds. The published constants leave a small gap between the two branches: the encode gap is 2.85e-8, so a `1e-8` bound would fail. |
| M·M⁻¹ = I for both pairs | D2 matrices |
| `SRGB_TO_XYZ·[1,1,1]` = D65 and `P3_TO_XYZ·[1,1,1]` = D65 | D65 chromaticities |
| Y row of `SRGB_TO_XYZ` ≈ (0.2126, 0.7152, 0.0722) | ITU-R BT.709 |
| sRGB red, green, blue → P3 ≈ (0.91749, 0.20029, 0.13856), (0.45840, 0.98526, 0.29829), (0, 0, 0.9596) (exact f64 blue: 0.95959), checked within `1e-4` | Exact f64 evaluation (verified by the orchestrator for blue). Literal agreement with the CSS Color 4 / WebKit `color(display-p3 …)` examples is still unverified (no network). |
| P3 red → extended sRGB ≈ (1.0931, −0.2267, −0.1501) within `1e-4`, unclamped (exact f64: 1.09307, −0.22674, −0.15013) | Same as the row above |

**Properties** (`tests/properties.rs`):

| ID | Property |
|---|---|
| P1 | sRGB → linear → sRGB, for inputs in [0,1] and in [−4,4] |
| P2 | linear → sRGB → linear |
| P3 | Monotonic non-decreasing decode and encode on [−4,4] |
| P4 | Alpha is bit-preserved by every conversion |
| P5 | `decode(−x)` has the same bits as `−decode(x)` |
| P6 | `apply_f32` is bit-identical to the per-pixel typed API; the identity transform leaves the buffer bit-identical |
| P7 | serde round trip of `Rgba`, `LinearRgba`, `DisplayP3Rgba`, all `Color` variants and `ColorSpaceId` is bit-identical, for any finite `f32` including `-0.0` and subnormals |
| P8 | (a→b)∘(b→c) ≈ a→c within `1e-4` |

Fixed-table unit tests cover 0, ±0, 1, the two thresholds and `f32` subnormals, so edge coverage does not depend on the random seed.

**Seeds**: proptest default config (256 cases, random seed). Failure persistence goes to committed `proptest-regressions/`. A `TestRunner::deterministic()` guard test proves the generators reach `-0.0` and subnormals, copying geom.

| T | RED | GREEN | REFACTOR / mutation |
|---|---|---|---|
| T1 | `cargo check -p gafira-color`: "did not match any packages". Scratch `std::fs::read` passes clippy *before* `clippy.toml` exists. Scratch `unsafe {}` in `gafira-color` (the `unsafe` branch of the I/O-purity scenario). | Manifest, `lib.rs` with `forbid`, `clippy.toml` copy, root dependency. V11 and V12 pass. The scratch read now fails clippy ("disallowed method `std::fs::read`", ARQ-02). The scratch `unsafe {}` fails `cargo check` ("usage of an `unsafe` block"). `cargo tree -p gafira-color -e dev` shows only `proptest`, `serde_json` and their transitive crates: no `gafira-*`, none of the forbidden prefixes, and no `rusty-fork` or `tempfile`. | Revert scratch; `git status` clean |
| T2 | N1 passes under the current V5 | New V5; N0–N4 as expected. N5 recorded as the known limitation (not met). N6 (V11) and N7 (V12) negatives fail as expected. | Commit `ci: make layer check table-driven per domain crate` |
| T3 | `space.rs`/`rgb.rs` tests fail to compile | Types, `new`, `is_finite`, wire names. Tests: an exhaustive `match` over `ColorSpaceId::ALL` **with no wildcard arm** (adding a variant breaks compilation); IEEE equality (`-0.0 == 0.0` is true, a NaN channel makes a value unequal to itself); `is_finite` is false for NaN and ±inf in any channel, including alpha. | — |
| T4 | Reference and matrix tests fail | `math.rs` | Mutations: changing 2.4 to 2.2 fails decode(0.5); a typo in one rational fails M·M⁻¹ |
| T5 | Conversion and `Color` tests plus `tests/api.rs` fail. Conversion cases: NaN and ±inf channels convert to every space **without panic** (output unspecified, alpha bit-identical); `convert` to the same space is bit-identical for NaN, `-0.0` and out-of-range channels; variants are never equal (`Srgb(x) != LinearSrgb(x)` and `!= DisplayP3(x)` for the same channels). API cases: exact JSON; unknown, missing or `"SRGB"` space rejected; missing `a` rejected; non-numeric channel (`"r":"x"`) rejected; `{"keys":[]}` rejected; NaN → `null` → decode error; extra field ignored. | `rgb.rs` conversions, `color.rs` | Removing the sign mirroring fails the P3-red reference |
| T6 | Properties P1–P5, P7, P8 | Pass. RED is shown by mutation (truncating through `f32` mid-pipeline; renaming a variant), then reverted. | **Size checkpoint** |
| T7 | `transform.rs` tests fail: object safety (`Box<dyn ColorTransform>` moved to another thread), `Send + Sync`, `src`/`dst`, length error (10 with `Rgba`, 7 with `Rgb`) leaves the buffer bit-identical, empty slice ok, `Rgb` and `Rgba`. `Display` of `PixelBufferError::LengthNotMultiple` names both `len` and `channels`, and the error is usable as `&dyn std::error::Error`. | `transform.rs`, P6 | — |
| T8 | — | All gates (Verification) | — |

## Budget (rows × 1.5 for Rust; exact files × 1.0)

| File | Raw | Expected measured |
|---|---|---|
| `crates/gafira-color/Cargo.toml` / `clippy.toml` | 18 / 45 | 18 / 45 |
| `lib.rs` / `space.rs` | 20 / 35 | 30 / 53 |
| `math.rs` / `rgb.rs` / `color.rs` | 100 / 80 / 60 | 150 / 120 / 90 |
| `transform.rs` | 100 | 150 |
| `tests/api.rs` / `tests/properties.rs` | 40 / 85 | 60 / 128 |
| root `Cargo.toml` / `ci.yml` (+32 −8, ×1.2) | 1 / 40 | 1 / 48 |
| **Total** | **624** | **~893** (Lab+Cmyk would add ~180 → ~1 070) |

The total is over the 800 budget even after the cut. Two slices are recommended:

| Slice | Tasks | Expected measured | Content |
|---|---|---|---|
| PR1 | T1–T6 | ~720 | Types, conversions, `Color`, V5 |
| PR2 | T7 | ~175 | `ColorTransform`, `PixelLayout`, P6 |

The alternative is one PR under `size:exception`. The review workload guard decides after `sdd-tasks`.

**Measurement point**: after T6's commit, run `git diff --shortstat main...HEAD -- . ':!Cargo.lock' ':!openspec'`. If PR1 is over 800, move T2 (V5) to PR2.

## Verification Plan

| ID | Command | Locally today? |
|---|---|---|
| Gates | `cargo fmt --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo deny check` | Yes (cargo-deny 0.20.2 is installed; advisories need network) |
| V3 | `cargo metadata --no-deps`: gafira-color has edition 2024, rust_version 1.93, publish [] | Yes |
| V5 | New step; N0–N4; N5 recorded as the known limitation (crate without `clippy.toml` is not checked; not met) | Yes, through the scratch copy |
| V5-dev | `cargo tree -p gafira-color -e dev --target all --prefix none -f '{p}'`: no `gafira-*` other than itself, no forbidden prefix, no `rusty-fork`/`tempfile` | Yes |
| V6 | Unchanged ARQ-07 step | Yes |
| V11 | Existing loop covers `crates/gafira-color/src/lib.rs`; negative N6 fails | Yes |
| V12 | `diff -q crates/gafira-geom/clippy.toml crates/gafira-color/clippy.toml`; negative N7 fails | Yes, first non-trivial run |
| Purity | Scratch `std::fs::read` fails clippy; scratch `unsafe {}` fails `cargo check` (T1) | Yes |
| Unit/API extras | Non-finite inputs without panic; same-space `convert` bit-identical for NaN and `-0.0`; exhaustive `match` on `ColorSpaceId`; variants never equal and IEEE equality; `"r":"x"` rejected; `PixelBufferError` `Display` (T3, T5, T7) | Yes, in `cargo test` |
| CI | Three ARQ-03 targets + `policy`, green | After push |
| Not today | 02 §4.10 ΔE2000 < 1.0 on a 1 000-color chart against a production reference (needs an ICC or reference path) | Deferred |

## Not verifiable at design time (check during apply)

| Item | How |
|---|---|
| CSS Color 4 rationals transcribed correctly | Compare with the spec's sample code. The T4 identity and white-point tests must pass. |
| Literal agreement of the reference P3 and extended-sRGB values with the CSS Color 4 / WebKit examples (the f64 values themselves are verified) | Compare with the published examples when network is available; the independent f64 evaluation is recorded in the test comment |
| `serde_json` f32 decode (f64 parse then `as f32`) is bit-exact for shortest output, including subnormals and the internally-tagged `Content` path | P7 on all three variants |
| `const fn` float `mul` compiles on 1.93; no `excessive_precision` warnings | `cargo clippy -p gafira-color -- -D warnings` |
| `grep -E "^$crate v"` matches the `cargo tree -f '{p}'` format, for example `gafira-color v0.1.0 (/path)` | N0 run |

## Spec alignment

The specs and this design agree on the reference values, tolerances, curve thresholds and the V5 prefix match (aligned after the first design validation, 2026-10-24).

## Threat Matrix

| Boundary | Applicability |
|---|---|
| Documentation-like paths | N/A: no file classification or execution |
| Git repository selection | N/A: V5 runs only `cargo tree` in the checkout cwd |
| Commit state | N/A: no git automation |
| Push state | N/A: no git automation |
| PR commands | N/A: no git or PR automation |

The only shell change is a read-only CI check. Its own failure modes are covered by N0–N4.

## Platforms

- No platform-specific code.
- `powf` differs by libm across Windows, Linux and macOS, so only tolerances are used.
- Persisted bytes are user input, never converted values, so ARQ-10 holds on all three.

## Decisions that constrain `domain-doc`

1. The stored format is the bare `Color` (`space` + `r,g,b,a`). Field order and the CSS names are part of the format; changing either needs a migration.
2. `Color` is `Copy + PartialEq` with no `Eq`/`Hash`, so `ElementBase` and nodes cannot derive `Eq`/`Hash`. Identity checks (undo, ARQ-10 tests) compare bits.
3. Store the user's chosen space and values; never persist conversion output.
4. Finiteness (`is_finite`) and any [0,1] range rule are C2c invariants. The domain allows extended values.
5. `Animatable<Color>` (untagged) works because `space` is required.
6. `ColorRef::Swatch` and `NamedColor` belong to doc, not color.

## Migration / Rollout

No migration required. Rollback: delete `crates/gafira-color/`, remove the root dependency line, and restore the previous V5 step.

## Open Questions

Already settled by the specs (no longer open): `apply_f32 -> Result<(), PixelBufferError>` as a documented 02 §4.10 deviation; CSS Color 4 wire names; unknown extra fields ignored; `Lab`/`Cmyk` deferred; `bake_lut3d`/`Lut3d` omitted; ΔE2000 acceptance deferred.

- [ ] Two PRs (~720 + ~175) or one under `size:exception` (~890)? The review workload guard decides after `sdd-tasks`.
- [ ] Holder confirmation of the 02 §4.10 deviation (`Result` return), recorded as PROPOSAL in the spec.
