# Crate gafira-geom

## Purpose

Layer-0 geometry and unit primitives. Pure domain code: no I/O, no `unsafe`, `f64` everywhere. It has no internal dependencies; externally it uses `kurbo` and `serde`.

## Public API

| Item | Description |
|---|---|
| `Affine`, `Point`, `Rect`, `Vec2` | Re-exported from `kurbo` |
| `Unit` | `Px`, `Mm`, `Cm`, `In`, `Pt`; `Unit::ALL`; serialized as the lowercase name |
| `Length` | A value with its unit: `new`, `value`, `unit`, `convert(to, dpi)`, `to_px(dpi)` |
| `ConversionError` | `InvalidDpi(f64)`, `NonFiniteValue(f64)`, `NonFiniteResult` (non-exhaustive) |
| `Size` | Width and height pair: `new`, `width`, `height` |
| `Transform2D` | Wrapper of `kurbo::Affine`: `IDENTITY`, `translation`, `scale`, `scale_xy`, `rotation`, `then`, `apply`, `determinant`, `inverse`, `coeffs`; conversions from and to `Affine` and `[f64; 6]` |
| `SingularTransform` | Error of `Transform2D::inverse` for a non-invertible transform |

## Unit conversion

Conversion goes through inches. Physical units have fixed factors: 25.4 mm, 2.54 cm and 72 pt per inch. `Px` relates to inches through the DPI given at call time, so a pixel length has no physical meaning without a DPI.

```
value_in_target = value / per_inch(source) * per_inch(target)
```

## Invariants

- `Length::convert` validates the DPI on every call, including same-unit conversions. A DPI that is zero, negative, NaN or infinite returns `InvalidDpi`.
- A non-finite value cannot be converted (`NonFiniteValue`), and a conversion that overflows to infinity returns `NonFiniteResult`. Same-unit conversion returns the value unchanged.
- `Length` is never normalized implicitly and construction does not validate. A non-finite value can be built but cannot be persisted: JSON encodes it as `null`, which fails to decode as `f64`.
- The DPI is always an explicit argument; there is no global default inside this crate.

## Tests

Integration tests live in `crates/gafira-geom/tests/` (`api.rs` for the contract, `properties.rs` for property tests with `proptest`).
