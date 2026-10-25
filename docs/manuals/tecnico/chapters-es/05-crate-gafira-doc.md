# Crate gafira-doc

## Propósito

Modelo de documento de la capa 1: `Document -> Page -> Node`. Código de dominio puro: sin I/O, sin `unsafe`. Depende solo de `gafira-geom` y `gafira-color`; los tipos de geometría se importan siempre a través de `gafira_geom`. Externamente usa `serde` (feature `rc`) y `serde_json`.

La spec 02 describe un modelo más amplio (`Project`, secuencias, pistas, clips, estilos, recursos, animación). Solo existe el subconjunto siguiente; el resto está **Planificado**.

## API pública

| Elemento | Descripción |
|---|---|
| `Document` | Raíz: `SCHEMA_VERSION` (1), `schema_version`, `next_id`, `dpi`, `pages`, `page(id)`, `from_parts`, `validate`, `from_json`, `to_json`, `edit`; `Default` es un documento vacío con `next_id` 1 y 96 DPI |
| `Page` | `id`, `name`, `size`, `locked`, `hidden`, `roots`, `nodes` (un `BTreeMap<NodeId, Arc<Node>>`); `Page::new`, `node(id)` |
| `Node` | `base: ElementBase`, `kind: NodeKind`, `children: Vec<NodeId>`; `Node::new` |
| `NodeKind` | Enum cerrado: `Group(GroupNode)`, `Shape(ShapeNode)`, `Extension(ExtensionNode)` |
| `ShapeNode`, `ShapeGeometry` | Forma con `geometry` (`Rect` o `Ellipse`) y un `fill: Option<Color>` opcional |
| `ExtensionNode` | Datos opacos de un plugin o de una función futura: `type_id`, `schema_version`, `payload` (`serde_json::Value`); `MAX_EXTENSION_DEPTH` es 64 |
| `ElementBase` | Cabecera común: `id`, `name`, `placement`, `opacity`, `blend`, `visible`, `locked`; `ElementBase::new` |
| `Placement`, `Flip` | Posición y orientación en px lógicos: `x`, `y`, `w`, `h`, `rotation_deg`, `skew`, `flip`; `to_transform()` devuelve un `Affine` |
| `BlendMode` | Los 16 modos de composición W3C; `BlendMode::ALL` |
| `Dpi` | Píxeles por pulgada: `DEFAULT` 96, `MIN` 1, `MAX` 10 000; `new`, `get`, `is_valid` |
| `PageSize`, `Axis`, `PageSizeError` | Dimensiones de página como dos valores `Length`; `MIN_PX` 1,0, `MAX_PX` 16 384,0; `to_px(dpi)`, `check(dpi)` |
| `NodeId`, `PageId` | Newtypes `u64` distintos con `new`, `get`, `Display` |
| `IdSpaceExhausted` | Error cuando el contador de ids superaría `u64::MAX` |
| `ValidationError`, `NodeField` | Resultado de `validate` (véase el capítulo de validación) |
| `LoadError` | `Json(serde_json::Error)` o `Invalid(ValidationError)` (no exhaustivo) |
| `EditToken`, `DocMut` | La capacidad de edición (más abajo) |
| `msg::MessageKey`, `msg::CommandLabel` | Clave de mensaje neutra `MessageKey(&'static str)`; `CommandLabel` es un alias. Sin idioma ni texto traducido; nunca se serializa en un documento |

## Estructura y compartición

Un `Document` guarda sus páginas como `Arc<Page>`, y cada página guarda sus nodos como `Arc<Node>` en un `BTreeMap`. Clonar un documento no copia ninguna página ni nodo. Una mutación copia al escribir solo a lo largo del camino que toca (`Arc::make_mut` sobre la página y sobre el nodo). Esta compartición estructural es en la que se apoyan las instantáneas del historial.

Los ids salen de un único contador, `next_id`, compartido por páginas y nodos. Los ids de `roots` y de `children` se resuelven solo dentro de la arena de su propia página.

## Invariantes de los tipos del modelo

- La construcción nunca valida. Los valores fuera de rango (por ejemplo una opacidad de 2,0 o un DPI de 0) son representables; `Document::validate` es la comprobación, y `Document::from_parts` y la carga la invocan.
- La igualdad es IEEE y estructural: un NaN nunca es igual a sí mismo.
- Las banderas de página (`locked`, `hidden`) no se propagan a las banderas de los nodos.
- `NodeKind` es cerrado a propósito; añadir un tipo hace que el compilador señale cada `match`.
- El orden de iteración es determinista: la arena es un `BTreeMap` y `HashMap` está prohibido por `clippy.toml`.

## Geometría de la colocación

La caja local de un elemento es `[0,w] x [0,h]`; `w` y `h` dimensionan la caja y nunca la escalan. `Placement::to_transform` lleva las coordenadas locales al espacio de página, aplicando de derecha a izquierda: trasladar menos la mitad del tamaño, reflejar, inclinar x por `tan(skew)`, rotar en sentido horario (página con y hacia abajo) y trasladar al centro de la caja. La función es total: campos no finitos dan coeficientes no finitos.

## Capacidad de edición

`Document` tiene campos privados y no expone acceso `&mut`. La mutación exige un `EditToken` y pasa por `Document::edit(&token)`, que devuelve un `DocMut`:

| Método de `DocMut` | Efecto |
|---|---|
| `alloc_node_id`, `alloc_page_id` | Toman un id del contador compartido; `IdSpaceExhausted` en `u64::MAX`, con el contador sin cambios |
| `restore_next_id` | Baja el contador hasta un valor leído antes de las asignaciones que se deshacen; un valor igual o superior al contador se ignora, así que el contador nunca sube y los ids siguen siendo únicos en todo estado |
| `set_dpi` | Fija el DPI (sin validar) |
| `pages_mut` | La lista de páginas, para añadir, quitar o reordenar páginas |
| `page_mut`, `node_mut` | Acceso con copia al escribir a lo largo de un camino |
| `as_doc` | Acceso de lectura durante la edición |

`EditToken` tiene un campo privado y no implementa `Clone`, `Copy`, `Default` ni `Debug`, así que no puede duplicarse ni construirse con un literal. Su constructor es `#[doc(hidden)]`. Solo los crates de edición pueden nombrarlo; el paso V13 de la CI lo aplica. Nada se valida implícitamente: tras cualquier secuencia de operaciones de `DocMut` el documento puede ser inválido. Reemplazar un valor completo (`*doc = other`) por quien posee el documento es posible y está documentado, no impedido. Las doctests compile-fail de `edit.rs` son ayudas de regresión; los fixtures compile-fail con `trybuild` para ARQ-30 viven en `gafira-edit` (véase el capítulo de edición).

## Pruebas

Las pruebas de integración están en `crates/gafira-doc/tests/`: `serde.rs`, `golden.rs`, `properties.rs`, `edit.rs`, con los generadores compartidos tomados de `gafira-testkit` (una dev-dependency). Los archivos JSON golden están en `tests/golden/`.
