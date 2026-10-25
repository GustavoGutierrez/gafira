# Crate gafira-color

## Purpose

Layer-0 color vocabulary and analytic conversions. Pure domain code: no I/O, no `unsafe`. Channels are stored as `f32`; conversions widen to `f64`, compute and narrow once. It has no internal dependencies and uses `serde`.

## Public API

| Item | Description |
|---|---|
| `ColorSpaceId` | Identifier of a supported space; `ColorSpaceId::ALL` has three entries |
| `Rgba`, `LinearRgba`, `DisplayP3Rgba` | Straight-alpha `f32` RGBA value types, one per space; `new(r, g, b, a)`, `is_finite`, and conversions such as `Rgba::to_linear`, `LinearRgba::to_srgb`, `LinearRgba::to_display_p3`, `DisplayP3Rgba::to_linear_srgb` |
| `Color` | Enum tagged with its space: `Srgb`, `LinearSrgb`, `DisplayP3`; `space`, `alpha`, `is_finite`, `convert(to)`, `to_linear_srgb`; `From` for the three value types |
| `ColorTransform` | Trait (`Send + Sync`): `src`, `dst`, `apply_f32(&self, px, layout)` |
| `AnalyticTransform` | Closed-form `ColorTransform` between any two of the three spaces: `new(src, dst)` |
| `PixelLayout` | `Rgb` (3 channels) or `Rgba` (4 channels, straight alpha); `channels()` |
| `PixelBufferError` | `LengthNotMultiple { len, channels }` (non-exhaustive) |

## Wire form of Color

`Color` serializes as one flat object, `space` first and then `r`, `g`, `b`, `a`:

```json
{"space":"srgb","r":1.0,"g":0.5,"b":0.0,"a":1.0}
```

Space names are `srgb`, `srgb-linear` and `display-p3`. An unknown or missing `space` fails to deserialize; extra fields are ignored. The enum is exhaustive on purpose: adding a space makes the compiler flag every `match`.

## Invariants

- Conversion never clamps. Out-of-gamut and out-of-range values pass through.
- Alpha is copied bit for bit. Converting to the same space returns a bit-identical value.
- Equality is IEEE and structural, so `Srgb(x) != LinearSrgb(x)` for the same numbers.
- `ColorTransform::apply_f32` converts interleaved pixels in place and leaves alpha untouched for `PixelLayout::Rgba`. A buffer whose length is not a multiple of the channel count returns `PixelBufferError::LengthNotMultiple` and is left unmodified; an empty buffer is valid. This is owner decision D-11, a documented deviation from spec 02 section 4.10, which returned `()`.
- The conversion math (transfer curve, CSS Color 4 matrices through XYZ) is crate-internal.

## Tests

Integration tests live in `crates/gafira-color/tests/` (`api.rs`, `properties.rs`).
