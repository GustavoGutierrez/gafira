# Crate gafira-geom

## Propósito

Primitivas de geometría y unidades de la capa 0. Código de dominio puro: sin I/O, sin `unsafe`, `f64` en todas partes. No tiene dependencias internas; externamente usa `kurbo` y `serde`.

## API pública

| Elemento | Descripción |
|---|---|
| `Affine`, `Point`, `Rect`, `Vec2` | Reexportados de `kurbo` |
| `Unit` | `Px`, `Mm`, `Cm`, `In`, `Pt`; `Unit::ALL`; se serializa como el nombre en minúsculas |
| `Length` | Un valor con su unidad: `new`, `value`, `unit`, `convert(to, dpi)`, `to_px(dpi)` |
| `ConversionError` | `InvalidDpi(f64)`, `NonFiniteValue(f64)`, `NonFiniteResult` (no exhaustivo) |
| `Size` | Par ancho y alto: `new`, `width`, `height` |
| `Transform2D` | Envoltura de `kurbo::Affine`: `IDENTITY`, `translation`, `scale`, `scale_xy`, `rotation`, `then`, `apply`, `determinant`, `inverse`, `coeffs`; conversiones desde y hacia `Affine` y `[f64; 6]` |
| `SingularTransform` | Error de `Transform2D::inverse` para una transformación no invertible |

## Conversión de unidades

La conversión pasa por pulgadas. Las unidades físicas tienen factores fijos: 25,4 mm, 2,54 cm y 72 pt por pulgada. `Px` se relaciona con las pulgadas mediante el DPI dado en cada llamada, de modo que una longitud en píxeles no tiene significado físico sin un DPI.

```
value_in_target = value / per_inch(source) * per_inch(target)
```

## Invariantes

- `Length::convert` valida el DPI en cada llamada, incluidas las conversiones a la misma unidad. Un DPI cero, negativo, NaN o infinito devuelve `InvalidDpi`.
- Un valor no finito no se puede convertir (`NonFiniteValue`), y una conversión que desborda a infinito devuelve `NonFiniteResult`. La conversión a la misma unidad devuelve el valor sin cambios.
- `Length` nunca se normaliza implícitamente y su construcción no valida. Se puede construir un valor no finito, pero no se puede persistir: JSON lo codifica como `null`, que no se decodifica como `f64`.
- El DPI es siempre un argumento explícito; este crate no tiene un valor global por defecto.

## Pruebas

Las pruebas de integración están en `crates/gafira-geom/tests/` (`api.rs` para el contrato, `properties.rs` para pruebas de propiedades con `proptest`).
