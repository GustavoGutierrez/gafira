# Crate gafira-color

## Propósito

Vocabulario de color y conversiones analíticas de la capa 0. Código de dominio puro: sin I/O, sin `unsafe`. Los canales se guardan como `f32`; las conversiones se amplían a `f64`, calculan y se reducen una sola vez. No tiene dependencias internas y usa `serde`.

## API pública

| Elemento | Descripción |
|---|---|
| `ColorSpaceId` | Identificador de un espacio soportado; `ColorSpaceId::ALL` tiene tres entradas |
| `Rgba`, `LinearRgba`, `DisplayP3Rgba` | Tipos de valor RGBA `f32` con alfa recto, uno por espacio; `new(r, g, b, a)`, `is_finite` y conversiones como `Rgba::to_linear`, `LinearRgba::to_srgb`, `LinearRgba::to_display_p3`, `DisplayP3Rgba::to_linear_srgb` |
| `Color` | Enum etiquetado con su espacio: `Srgb`, `LinearSrgb`, `DisplayP3`; `space`, `alpha`, `is_finite`, `convert(to)`, `to_linear_srgb`; `From` para los tres tipos de valor |
| `ColorTransform` | Trait (`Send + Sync`): `src`, `dst`, `apply_f32(&self, px, layout)` |
| `AnalyticTransform` | `ColorTransform` de forma cerrada entre dos cualesquiera de los tres espacios: `new(src, dst)` |
| `PixelLayout` | `Rgb` (3 canales) o `Rgba` (4 canales, alfa recto); `channels()` |
| `PixelBufferError` | `LengthNotMultiple { len, channels }` (no exhaustivo) |

## Forma en disco de Color

`Color` se serializa como un único objeto plano, con `space` primero y luego `r`, `g`, `b`, `a`:

```json
{"space":"srgb","r":1.0,"g":0.5,"b":0.0,"a":1.0}
```

Los nombres de espacio son `srgb`, `srgb-linear` y `display-p3`. Un `space` desconocido o ausente falla al deserializar; los campos extra se ignoran. El enum es exhaustivo a propósito: añadir un espacio hace que el compilador señale cada `match`.

## Invariantes

- La conversión nunca recorta. Los valores fuera de gamut o de rango pasan intactos.
- El alfa se copia bit a bit. Convertir al mismo espacio devuelve un valor idéntico bit a bit.
- La igualdad es IEEE y estructural, así que `Srgb(x) != LinearSrgb(x)` para los mismos números.
- `ColorTransform::apply_f32` convierte píxeles entrelazados in situ y deja el alfa intacto para `PixelLayout::Rgba`. Un buffer cuya longitud no es múltiplo del número de canales devuelve `PixelBufferError::LengthNotMultiple` y queda sin modificar; un buffer vacío es válido. Es la decisión D-11 del titular, una desviación documentada de la spec 02 sección 4.10, que devolvía `()`.
- La matemática de conversión (curva de transferencia y matrices CSS Color 4 a través de XYZ) es interna del crate.

## Pruebas

Las pruebas de integración están en `crates/gafira-color/tests/` (`api.rs`, `properties.rs`).
