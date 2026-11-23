# 039 — Un diálogo «Exportar» más completo

> **Estado:** done — 2026-11-17 — S39-01..S39-08 cubiertos por `gafira-io-image` (`jpeg_export::tests` 2, `tests/jpeg_webp.rs` 5), `gafira-engine` (`export::tests` s39, 2), `gafira-platform` (`reveal::tests` 1) y `gafira-ui` (`export::tests` s39, 8); 7 mutantes a mano muertos; 2988 tests.

## Objetivo

Que «Exportar» se parezca al diálogo de 03 §7.5. Hasta ahora ofrecía PNG de la página actual a 1× o 2× y PDF de todas las páginas (spec 030). Con esta spec la persona elige entre PNG, JPG, WebP y PDF. También elige la escala, el fondo transparente, la calidad del JPG y las páginas (todas, la actual o un rango como `1-3, 5`). Ve una vista previa de la página y el tamaño en píxeles que va a salir. Al terminar, el aviso ofrece «Mostrar en carpeta» (03 §7.5, 04 FD-55, FD-57, 02 §6.4).

## Alcance / no alcance

Alcance:

- `gafira-io-image`:
  - `JpegExporter` (FD-55): escala de 0,5× a 4×; calidad de 1 a 100, 90 por defecto; sin transparencia: la página se compone sobre blanco. Usa el codificador JPEG de `image`, que ya está en el workspace.
  - `WebpExporter` (FD-55): WebP sin pérdida con `image-webp` (ya en el workspace por la spec 033); escala de 0,5× a 4×; fondo transparente opcional.
  - Los dos escriben un archivo por página, como el PNG, con la clase legal `RoyaltyFree`.
- `gafira-engine`: `ExportService::preview(page, transparent)` dibuja una página con el `PageRenderers` de la exportación, a 512 px como mucho en el lado largo, y la devuelve en sRGB 8 bits sin premultiplicar.
- `gafira-platform`: `show_in_folder(path)` abre la carpeta del archivo con el gestor de archivos del sistema (`xdg-open` en Linux, `explorer /select,` en Windows, `open -R` en macOS). Lanza el proceso y no espera por él.
- `gafira-ui`:
  - El diálogo lista los formatos del registro y construye sus opciones desde `Exporter::options_schema`: escala en pasos 1×–4× dentro del rango del esquema, casillas para las opciones booleanas y un deslizador para las enteras (la calidad).
  - Páginas: «Todas», «Página actual» o «Rango». El rango se comprueba al escribir y un error explica qué escribir; con un rango inválido no se puede exportar.
  - Una vista previa pequeña de la página, sobre un damero cuando el fondo es transparente.
  - El tamaño en píxeles que sale con la escala elegida y, si salen varios archivos, cuántos.
  - Al terminar, una tarjeta no modal con «Tu diseño está listo para compartir», «Mostrar en carpeta» y «Cerrar». Mostrar en carpeta llama a una función que inyecta la raíz de composición por `UiConfig` (el shell no depende de `gafira-platform`, 02 §2.1).
- `apps/gafira`: registra PNG, JPG, WebP y PDF, en ese orden, y compone `show_in_folder`.

No alcance:

- WebP con pérdida: `image-webp` 0.2 solo codifica sin pérdida y no hay otro codificador permisivo en Rust puro en el árbol.
- JPEG: submuestreo elegible (4:4:4 o 4:2:0), progresivo y el perfil sRGB incrustado.
- La componenda del JPEG sobre el color de fondo de la página: el documento todavía no tiene color de fondo, así que se usa blanco.
- GIF, SVG, «Selección» de FD-57, el patrón de nombres configurable, «Limitar tamaño» y la calidad del PNG.
- El tamaño estimado en MB, «Recordar estos ajustes», «Guardar en» dentro del diálogo, avanzado del PDF (sangrado, marcas, contraseña), la navegación por páginas de la vista previa (◀ 1 / 3 ▶) y el aviso por encima de 16 384 px antes de exportar (el exportador lo rechaza igual).
- Omitir páginas ocultas (FD-02): el documento todavía no tiene páginas ocultas.
- Dibujar la vista previa fuera del hilo de la ventana: se dibuja una vez al abrir el diálogo y al cambiar el fondo, a 512 px.

## Dependencias

- 03 §7.5; 04 FD-55, FD-57, FD-58; 02 §2.1, §4.5, §6.4.
- Specs 030 (exportadores PNG y PDF), 033 (WebP sin pérdida), 035 (exportación en segundo plano).
- Sin crates nuevos: `image` 0.25 (MIT OR Apache-2.0) e `image-webp` 0.2 (MIT OR Apache-2.0) ya están en el workspace.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S39-01 | JPG. | Un JPG de la página tiene el tamaño de la página por la escala y el color de una forma conocida en su sitio, dentro de la tolerancia de la compresión. El fondo transparente no existe: una página pintada con alfa sale sobre blanco. La calidad fuera de 1–100 bloquea la exportación y una calidad menor da un archivo más pequeño. |
| S39-02 | WebP. | Un WebP sin pérdida se lee con los mismos píxeles que el PNG de la misma página; con fondo transparente, alfa 0 donde no hay contenido. |
| S39-03 | Rango de páginas. | `1-3, 5` da las páginas 1, 2, 3 y 5; los espacios no cuentan y una página repetida sale una vez. Un rango vacío, una página 0, una página más allá de la última, un rango invertido o un texto que no es un número es un error. |
| S39-04 | La petición desde el diálogo. | El formato, las páginas (todas, la actual o el rango, en el orden del documento) y las opciones del esquema (escala, fondo transparente, calidad) llegan a la `ExportRequest`. El PDF conserva su resolución de 2× por defecto. Las opciones que el formato no tiene no se envían. |
| S39-05 | Vista previa. | `preview` dibuja la página con 512 px en el lado largo como mucho y conserva la proporción; con fondo transparente, alfa 0 donde no hay contenido; sin documento o con una página que no existe, es un error. |
| S39-06 | Tamaño en píxeles. | El diálogo muestra el tamaño de la página por la escala, redondeado, y el número de archivos cuando sale más de uno. |
| S39-07 | «Mostrar en carpeta». | Tras una exportación correcta la tarjeta ofrece «Mostrar en carpeta», que llama a la función de `UiConfig` con la ruta del primer archivo. En Linux la orden es `xdg-open` sobre la carpeta del archivo. Lanzarla no bloquea la ventana. |
| S39-08 | Registro. | La app registra PNG, JPG, WebP y PDF; el engine de la app escribe un JPG y un WebP que empiezan por sus firmas. |

## TDD

Lógica crítica (deben fallar primero): la componenda sobre blanco del JPG (S39-01), el análisis del rango (S39-03), la construcción de la petición desde el esquema (S39-04) y el tamaño de la vista previa (S39-05). S39-02, S39-07 y S39-08, con tests de integración.

## Evidencia

- Rojo verificado: `tests/jpeg_webp.rs` no compilaba sin `JpegExporter`, `WebpExporter` ni `QUALITY`; los dos tests de la componenda fallaron con la función vacía; los de `preview` no compilaban sin el método. En `gafira-ui` los tests del rango y de la petición se escribieron en el mismo paso que el código; su capacidad de fallar se comprobó con los mutantes de abajo.
- Mutantes a mano, todos muertos:
  - sin el término del blanco en la componenda (muere `s39_01_pixels_are_composed_over_white`);
  - la calidad del JPEG fija en 90 (muere `s39_01_the_jpeg_quality_is_checked_and_used`);
  - la vista previa siempre opaca (muere `s39_05_the_preview_draws_the_page_at_512_px_on_its_long_side`);
  - sin rechazar un rango invertido (muere `s39_03_a_wrong_range_is_refused_with_its_reason`);
  - la petición envía todo valor del diálogo, también los que el esquema no tiene (muere `s39_04_the_options_come_from_the_schema_of_the_format`);
  - al cambiar de formato no se vuelve a los valores por defecto (muere el mismo test);
  - las páginas no siguen al formato de varias páginas (mueren `s39_04_the_pages_follow_the_format_until_they_are_chosen` y el de opciones).
- `cargo fmt --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 2988 tests, 0 fallos.
- `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` y `cargo test -p gafira-testkit --features ports --locked`: en verde.
- Pasos de política de `ci.yml` en local (ARQ-07, `forbid(unsafe_code)`, V5, ARQ-06, V12, ARQ-107 ×2, V13, V14): todos en verde.
- `cargo deny check licenses bans sources`: `bans ok, licenses ok, sources ok`. Sin crates nuevos en el árbol: `gafira-io-image` usa ahora `image-webp` (MIT OR Apache-2.0) y `gafira-engine` pasa `gafira-raster` de dev a normal (arista ya permitida en su fila V5).
- `python3 tools/sync_agents.py --check`: 0.
- Prueba manual pendiente del titular: abrir el diálogo en la app, ver la vista previa y pulsar «Mostrar en carpeta» en los tres sistemas.

## Decisiones y límites registrados

- Los tres exportadores de imagen comparten un bucle (`raster.rs` de `gafira-io-image`): un archivo por página con el nombre `page.<ext>` o `page-<n>.<ext>`, y el engine los numera junto a la ruta elegida (FD-57 (c)).
- El JPEG pide el render sobre el fondo de la página y compone lo que queda translúcido sobre blanco; ignora `transparent` aunque llegue.
- El shell no depende de `gafira-platform`: `UiConfig.show_in_folder` (`ShowInFolder`) lo da la raíz de composición. `xdg-open`, `explorer` y `open` vienen con el sistema operativo; no son binarios que Gafira instale.
- La vista previa se dibuja en el hilo de la ventana a 512 px y se guarda como textura mientras no cambien la página ni el fondo.
- Los textos `export.png` y `export.pdf` pasan a ser solo «PNG» y «PDF»: la página ya no va en el nombre del formato.

## Riesgos y decisiones abiertas

- Ninguna decisión del titular abierta. El color de fondo de la página no existe todavía: el JPG se compone sobre blanco hasta que exista.
