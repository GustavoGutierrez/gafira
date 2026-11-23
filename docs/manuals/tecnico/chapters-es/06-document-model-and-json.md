# Modelo de documento y JSON canónico

## Puntos de entrada

| Llamada | Comportamiento |
|---|---|
| `Document::from_json(&[u8]) -> Result<Document, LoadError>` | Lee JSON UTF-8 y luego valida. `LoadError::Json` para errores de sintaxis, forma o codificación; `LoadError::Invalid` para JSON bien formado que rompe una regla del documento |
| `Document::to_json(&self) -> Result<String, serde_json::Error>` | Escribe la forma canónica compacta, sin salto de línea final |

La deserialización de `Document` pasa por un tipo de disco sin comprobar y por `validate()`, así que `serde` por sí solo no puede producir un documento inválido. Esta frontera de E/S de archivos pertenece al crate `gafira-format`, **Planificado**; hoy el crate de documento solo convierte bytes y valores.

## Forma canónica

La salida es determinista: guardar el mismo documento produce siempre los mismos bytes, sea cual sea el orden en que se construyó.

- Las claves se escriben en orden de declaración y se escribe cada campo; `None` se escribe como `null`.
- El orden es: documento `schema_version`, `next_id`, `dpi`, `pages`; página `id`, `name`, `size`, `locked`, `hidden`, `roots`, `nodes`; nodo `base`, `kind`, `children`; base `id`, `name`, `placement`, `opacity`, `blend`, `visible`, `locked`; placement `x`, `y`, `w`, `h`, `rotation_deg`, `skew`, `flip`.
- `NodeKind` se etiqueta internamente con `type`, con los valores `group`, `shape` y `extension`. Una forma escribe `geometry` (`rect` o `ellipse`) y luego `fill`; una extensión escribe `type_id`, `schema_version` y `payload`.
- `NodeId`, `PageId` y `Dpi` son números simples. `Length` es `{"value":..,"unit":..}`. `Color` usa la forma plana de `gafira-color`.
- `nodes` es un arreglo en orden ascendente de id, sea cual sea el orden de inserción.
- Los modos de mezcla usan los nombres CSS en kebab-case (`color-dodge`).

Un documento mínimo, tal como se guarda en `crates/gafira-doc/tests/golden/minimal.json`:

```json
{"schema_version":1,"next_id":3,"dpi":96,"pages":[{"id":1,"name":"P","size":{"width":{"value":100.0,"unit":"px"},"height":{"value":200.0,"unit":"px"}},"locked":false,"hidden":false,"roots":[],"nodes":[]}]}
```

## Reglas de lectura

| Regla | Detalle |
|---|---|
| Campos obligatorios | `schema_version`, `next_id`, `pages`; página `id`, `size`; nodo `base`, `kind`; base `id`, `placement`; placement `x`, `y`, `w`, `h`; kind `type`; extensión `type_id`, `schema_version`, `payload`; forma `geometry`; color `space`. La ausencia de uno es un error |
| Valores por defecto al leer | `dpi` 96; `name`, `locked`, `hidden`, `roots`, `nodes`, `children`; `opacity` 1,0; `blend` `normal`; `visible` verdadero; `rotation_deg` 0; `skew` 0; `flip` falso; `fill` ninguno. Un nuevo guardado los escribe explícitamente |
| Campos desconocidos | Se ignoran y se descartan al guardar; no se usa `deny_unknown_fields`, por compatibilidad hacia adelante |
| Vocabulario desconocido | Un `type`, `geometry`, `blend`, `unit` o `space` desconocido, o una capitalización incorrecta, es un error |
| Normalización | Un entero para un campo `f64` se acepta y se escribe como decimal; los nodos fuera de orden de id se aceptan y se escriben en orden ascendente |
| Ids duplicados | Los ids de nodo duplicados en una página fallan al cargar; los duplicados entre páginas los detecta la validación |

## Payloads de extensión

Un nodo `Extension` conserva `type_id`, `schema_version` y `payload` en la carga y el guardado. El payload es un `serde_json::Value`. Un payload canónico es exactamente la salida de `serde_json::to_string(&Value)` (claves ordenadas, sin espacios, números más cortos que ida y vuelta) y regresa byte a byte. Cualquier otro payload se carga como el mismo valor y se escribe en forma canónica; una segunda ida y vuelta no cambia nada. Normalizaciones conocidas: las claves duplicadas conservan el último valor y un entero mayor que `u64` pasa a decimal.

El `schema_version` de la extensión es propio y no guarda relación con el del documento. Las features `preserve_order` y `arbitrary_precision` de `serde_json` no deben activarse en ninguna parte del workspace, porque romperían el orden de claves y la identidad de bits de `f64`. El workspace activa `float_roundtrip`.

## Límites

| Límite | Valor | Motivo |
|---|---|---|
| Profundidad del payload de extensión | 64 niveles (`MAX_EXTENSION_DEPTH`) | `serde_json` no puede recargar un documento cuyo payload llega a unos 122 niveles; un documento guardado debe poder reabrirse |
| Lado de página | De 1 a 16 384 px, inclusive, al DPI del documento | `PageSize::MIN_PX` y `MAX_PX` |
| DPI | De 1 a 10 000 | `Dpi::MIN` y `Dpi::MAX` |
| Versión de esquema | 1 | La carga solo acepta la versión 1 |

## Entrada hostil

La carga devuelve un error y nunca entra en pánico, desborda la pila ni devuelve un documento inválido ante entradas vacías, truncadas o con basura al final, UTF-8 inválido, tipos incorrectos, números erróneos, campos duplicados, árboles profundos y payloads profundos. Las pruebas de propiedades corrompen documentos válidos y comprueban que la corrupción se detecta.

## Archivos golden

`crates/gafira-doc/tests/golden/` contiene `minimal.json`, `full.json`, `extension.json` y `extension_noncanonical.json`. Los golden canónicos deben regresar byte a byte en cualquier checkout, por lo que `crates/gafira-doc/tests/golden/.gitattributes` marca `*.json -text`. Eso evita que un checkout en Windows convierta LF en CRLF y cambie los bytes.

## Planificado

La lectura y escritura de archivos `.gaf`, los guardados atómicos, los medios vinculados, las versiones dentro del archivo (`versions/`) y las migraciones de esquema por pasos están especificados (spec 02, sección 7) para `gafira-format` y no están implementados. El subcomando `gafira-cli validate` se mueve con él (D-13).
