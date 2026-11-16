# 032 — Imágenes en la exportación PNG y PDF

> **Estado:** done — 2026-11-16 — S32-01..S32-06 cubiertos por `gafira-render` (`graph::tests` 3), `gafira-io-image` (`tests/decode.rs` 3, `tests/png.rs` 2), `gafira-io-pdf` (`tests/pdf.rs` 1) y `gafira-engine` (`export::tests` 3); 7 mutantes a mano muertos; 2929 tests. Recorte, ajustes, otros formatos y la caché de imágenes decodificadas quedan fuera.

## Objetivo

Que el PNG y el PDF de «Exportar» (spec 030) dibujen también los elementos de imagen (spec 029: `NodeKind::Image { asset }`, bytes en el almacén de contenidos), escalados a la caja del elemento y con su opacidad. Hoy la exportación los omite: el grafo de `gafira-render` dejó un enganche para `visit_image` y nadie le da píxeles.

## Alcance / no alcance

Alcance:

- `gafira-ports::export`:
  - El puerto `ImageDecoder`: bytes de un recurso → `ImageBuf` (8 bits sRGB premultiplicado, ARQ-117); un archivo que no decodifica es `MediaError::Corrupt`.
  - `ImagePixels`: el mapa `AssetRef` → `Arc<ImageBuf>` de los recursos que una exportación dibuja.
  - `PageRenderers::for_document(document, images)`: la fábrica recibe los píxeles ya decodificados junto a la instantánea.
- `gafira-engine`:
  - `Ports` gana el puerto `images`; `Ports::new` recibe `ExportPorts { pages, images }` en lugar de `pages`, como `SystemPorts` y `SettingsPorts` (siete argumentos de clippy).
  - `ExportService::export`, después de `validate` y antes de abrir ningún archivo, resuelve las imágenes de las páginas del alcance (todas las de esas páginas, visibles o no): lee los bytes del almacén y los decodifica por el puerto. Un recurso sin bytes en el almacén es `EngineError::Export(ExportError::Store(StoreError::NotFound { hash }))` («error.store.not_found» con el hash); un archivo que no decodifica, `ExportError::Media`. En los dos casos no se abre ningún archivo ni se construye el renderizador.
- `gafira-render`:
  - `RenderGraphBuilder::with_images` y `GraphPageRenderer::with_images`; `GraphPageRenderers::for_document` las pasa.
  - `visit_image` registra el nodo; `subtree` emite `RenderOp::DrawImage` con la transformación de la colocación (con la escala de exportación delante) por el escalado imagen → caja (`w / ancho`, `h / alto`), dentro de la capa del nodo, cuya opacidad y mezcla aplica `PopLayer` como en formas y textos.
  - La clave del nodo lleva el tipo nuevo `NodeType::Image` (5) y el hash del recurso (`KeyParam::Asset`, etiqueta 8).
  - Una imagen sin píxeles en el mapa (la vista previa no los pasa) o con una caja de lado 0 no dibuja nada.
- `gafira-io-image`: `RasterDecoder`, el `ImageDecoder` sobre `image` 0.25 con solo `png` y `jpeg` (ya en el workspace, ARQ-66); premultiplica a `Rgba8`.
- `apps/gafira`: compone `RasterDecoder` en `ExportPorts`.

No alcance:

- Recorte, ajustes y máscaras de imagen (RF-20); filtros distintos del bilineal del backend CPU (ARQ-121).
- Otros formatos de entrada (WebP, GIF, TIFF...), medios vinculados (Fase 2) y la caché de imágenes decodificadas entre exportaciones: cada exportación decodifica sus recursos una vez.
- La vista previa del lienzo, que sigue dibujando las imágenes con egui (spec 029).
- Límites de tamaño propios del decodificador: rigen los de `image` por defecto (512 MiB por asignación).

## Dependencias

- 02 §2.1 (capas: `gafira-raster` no decodifica, `gafira-io-image` sí), §4.5, §6.1, §6.2; ARQ-06, ARQ-66, ARQ-117, ARQ-121.
- Specs 026 (`ContentStore`), 029 (`ImageNode`), 030 (`ExportService`, `PageRenderers`, exportadores).
- Sin aristas nuevas en la tabla V5: `gafira-engine` ya puede depender de `gafira-raster`; `image` es un crate de terceros en un adaptador de capa 4.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S32-01 | La imagen en el grafo. | Una imagen con píxeles emite `DrawImage` entre `PushLayer` y `PopLayer` con la opacidad y la mezcla del nodo; su transformación lleva la imagen a la caja (esquinas en (x, y) y (x + w, y + h)) y la escala de exportación va delante; el recurso entra en la clave; sin píxeles o con un lado 0 no emite nada. |
| S32-02 | Decodificación. | `RasterDecoder` decodifica PNG y JPEG a `Rgba8` premultiplicado del tamaño del archivo; bytes que no son imagen dan `MediaError::Corrupt`. |
| S32-03 | El engine resuelve las imágenes. | La exportación entrega a `for_document` un mapa con los píxeles decodificados de cada recurso de imagen de las páginas del alcance, una vez por recurso. |
| S32-04 | Recurso ausente. | Sin bytes en el almacén la exportación falla con `StoreError::NotFound` del hash, antes de abrir ningún archivo y sin construir el renderizador; un archivo que no decodifica, igual con `ExportError::Media`. |
| S32-05 | PNG con imagen. | Un PNG de una página con una imagen tiene sus píxeles en la caja del elemento (escalados a ella) y blanco fuera; con opacidad 0,5 sobre blanco el rojo se lee mezclado en luz lineal (255, 188, 188). |
| S32-06 | PDF con imagen. | Un PDF de una página con imagen tiene una página del tamaño de la página en puntos. |

## TDD

Lógica crítica (transformaciones): S32-01 en `graph::tests`, debe fallar primero. S32-03 y S32-04 (invariante de no escribir nada ante un error) en `export::tests` del engine con dobles, también primero. S32-02, S32-05 y S32-06 con tests de integración (`gafira-io-image`, `gafira-io-pdf`) sobre `CpuBackend`.

## Evidencia esperada

Tests nuevos en `gafira-render`, `gafira-io-image`, `gafira-io-pdf` y `gafira-engine`; gates de `AGENTS.md` («Before calling something done»), `cargo deny check licenses bans sources` y los pasos de política de `ci.yml` en verde.

## Evidencia

- Rojo verificado: los tests de `graph` no compilaban sin `ImagePixels` ni `with_images`; los de `export` del engine fallaron sin la resolución (mapa vacío, exportación con éxito ante un recurso ausente o que no decodifica).
- Mutantes a mano, todos muertos: la colocación sin la escala de exportación delante (muere `s32_01_the_scale_goes_in_front_of_an_image`); un lado de la caja cambiado por el otro (mueren `s32_01_an_image_draws_its_pixels_into_its_box_inside_its_layer` y el de la escala); sin el recurso en la clave y sin comprobar el alto 0 (muere `s32_01_an_image_without_pixels_draws_nothing_and_its_asset_keys_it`); sin quitar recursos repetidos (muere `s32_03`); un recurso ausente o que no decodifica que se salta (mueren los dos `s32_04`).
- `cargo fmt --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 2929 tests, 0 fallos.
- `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` y `cargo test -p gafira-testkit --features ports --locked`: en verde.
- Pasos de política de `ci.yml` ejecutados en local (ARQ-07, `forbid(unsafe_code)`, V5, ARQ-06, V12, ARQ-107 x2, V13, V14, ADR 0013): todos en verde; la tabla V5 no cambió.
- `cargo deny check licenses bans sources`: `bans ok, licenses ok, sources ok`. Sin crates nuevos: `image` 0.25 ya estaba en el workspace (`gafira-ui`); `gafira-io-image` lo toma como dependencia normal y `gafira-io-pdf` gana `gafira-raster` solo en pruebas.
- `python3 tools/sync_agents.py --check`: 0.

## Decisiones y límites registrados

- El motor resuelve y decodifica; el render solo dibuja lo que recibe (ARQ-06, 02 §2.1). `PageRenderers::for_document` gana el mapa `ImagePixels`; `Ports::new` toma `ExportPorts { pages, images }`.
- Un recurso ausente usa el error existente `StoreError::NotFound` (mensaje `error.store.not_found` con el hash) dentro de `ExportError::Store`; no hay variante nueva en `EngineError`.
- Se resuelven todas las imágenes de las páginas del alcance, también las ocultas: una oculta sin bytes también bloquea la exportación.
- La imagen se estira a la caja (sin conservar proporción): la caja ya la fija la colocación (spec 029). El muestreo es el bilineal del backend CPU.
- El `README.md` no se tocó: su línea de estado ya no sigue a las specs.

## Riesgos y decisiones abiertas

- El cambio de firma de `PageRenderers::for_document` y de `Ports::new` toca a todos los que componen el engine (app, tests de UI y de adaptadores); es mecánico.
- Una imagen muy grande se decodifica entera en memoria para exportar; la caché y los niveles de mip quedan para cuando la vista previa use `gafira-render`.
