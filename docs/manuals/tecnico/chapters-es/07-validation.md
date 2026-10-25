# Pasadas de validación

## Contrato

`Document::validate(&self) -> Result<(), ValidationError>` devuelve la primera regla que el documento rompe, o `Ok(())`. Nada se altera ni se recorta. El resultado es determinista: el mismo documento produce siempre el mismo error.

Las pasadas se ejecutan en un orden fijo, de P1 a P6. Una pasada anterior siempre prevalece sobre una posterior y, dentro de una pasada, las comprobaciones siguen el orden de las variantes de `ValidationError`. En P1 a P3 cada comprobación recorre todas las páginas (en orden de página) antes de la siguiente; P4 avanza nodo por nodo en orden ascendente de id.

`ValidationError` es `#[non_exhaustive]`, así que un `match` externo necesita un brazo comodín (una doctest compile-fail lo vigila). Cada variante lleva los ids necesarios para localizar la falla.

## Pasadas

| Pasada | Comprobaciones, en orden | Errores |
|---|---|---|
| P1 Cabecera | La versión de esquema es `Document::SCHEMA_VERSION`; el DPI está en `Dpi::MIN..=Dpi::MAX` | `UnsupportedSchemaVersion`, `InvalidDpi` |
| P2 Ids | Los ids de página y de nodo son únicos en todo el documento; cada clave de la arena es igual al id del nodo que guarda; `next_id` es mayor que el id más alto en uso | `DuplicateId`, `ArenaKeyMismatch`, `NextIdNotAboveMax` |
| P3 Páginas | Cada lado de página está dentro de los límites en píxeles al DPI del documento (primero el ancho); toda raíz existe en su página; ninguna raíz se repite | `PageSize`, `DanglingRoot`, `DuplicateRoot` |
| P4 Nodos | Los campos numéricos son finitos y luego están en rango; el `type_id` de extensión no está vacío; la profundidad del payload de extensión es como máximo 64; solo los grupos tienen hijos; todo hijo existe en la página | `NonFinite`, `OutOfRange`, `EmptyExtensionTypeId`, `ExtensionTooDeep`, `ChildrenOnLeaf`, `DanglingChild` |
| P5 Padres | Ningún nodo es hijo de más de un padre; ninguna raíz es además hija; ningún nodo queda sin ser raíz ni hijo de otro | `MultipleParents`, `RootHasParent`, `Orphan` |
| P6 Alcanzabilidad | Búsqueda en profundidad desde las raíces; el id más bajo no alcanzado se reporta como dentro o debajo de un ciclo | `Cycle` |

## Rangos numéricos en P4

| Campo | Regla |
|---|---|
| `placement.x`, `placement.y`, `placement.rotation_deg` | Cualquier valor finito |
| `placement.w`, `placement.h` | Al menos 0 |
| `placement.skew` | Estrictamente entre -90 y 90 grados |
| `opacity` | De 0 a 1, inclusive |
| Rojo, verde y azul del relleno de una forma | Cualquier valor finito (pasan valores de gamut amplio y HDR) |
| Alfa del relleno de una forma | De 0 a 1, inclusive |

`NonFinite` se reporta para cualquier campo antes que cualquier `OutOfRange`. El campo lo nombra `NodeField` (`X`, `Y`, `W`, `H`, `RotationDeg`, `Skew`, `Opacity`, `FillChannel`, `FillAlpha`).

## Robustez

- P6 y la medida de profundidad del payload son iterativas, así que un árbol profundo o un payload hostil no pueden desbordar la pila.
- El límite en píxeles sigue al DPI del documento: cambiar el DPI puede invalidar una página antes válida, por lo que los comandos que cambian el DPI deben revalidar.
- Un ciclo es representable en memoria (el modelo no lo impide) pero nunca sobrevive a `validate` y, por tanto, nunca sobrevive a la carga.

## Dónde se ejecuta la validación

| Lugar | Estado |
|---|---|
| `Document::from_parts` y `Document::from_json` / `Deserialize` | Implementado: por estas rutas no se puede construir un documento inválido |
| Tras cada comando (`debug_assert!(doc.validate().is_ok())`, ARQ-13) | Implementado en `History` (compilaciones de depuración); los comandos comprueban sus propias entradas en todas las compilaciones |

## Pruebas de propiedades

`crates/gafira-doc/tests/properties.rs` genera documentos válidos (todos deben validar), aplica corrupciones dirigidas (cada una debe detectarse como la variante exacta esperada) y comprueba que la corrupción sobrevive a una ida y vuelta por JSON. Unas guardas de cobertura aseguran que los generadores alcanzan cada variante.
