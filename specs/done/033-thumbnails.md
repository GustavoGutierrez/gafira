# 033 — Miniaturas de los recientes

> **Estado:** done — 2026-11-16 — S33-01..S33-07 cubiertos por `gafira-ports`, `gafira-format`, `gafira-engine` y `gafira-ui` (23 tests nuevos); `thumbs/<page_id>.webp` WebP sin pérdida, como dicen 02 y 03; 8 mutantes a mano muertos y 1 equivalente; 2940 tests. Las miniaturas de las demás páginas y de las versiones, y la lectura en segundo plano, quedan fuera.

## Objetivo

Que las tarjetas de «Recientes» de Inicio muestren la primera página del proyecto, leída del `.gaf` sin abrirlo (03 §7.2: «Las miniaturas salen de `thumbs/<page_id>.webp` (02 §7.1), sin abrir el proyecto»; 02 §7.1 y D-68, carpeta `thumbs/`).

## Alcance / no alcance

Alcance:

- `gafira-ports::repository`: `Thumbnail` (la página y su `ImageBuf`), `THUMBNAIL_SIDE` (512 px) y `ProjectRepository::save_with_thumbnail`, cuya implementación por defecto llama a `save` sin miniatura.
- `gafira-raster`: `straight_srgb8` pasa aquí desde `gafira-io-image` (que la reexporta), para que `gafira-format` la use sin depender de otro adaptador (ARQ-06).
- `gafira-format`: escribe `thumbs/<page_id>.webp` (WebP sin pérdida, RGBA de 8 bits en sRGB, sin comprimir dentro del ZIP); al leer, las entradas de `thumbs/` pasan las comprobaciones de archivo hostil; `read_thumbnail` lee solo el `mimetype`, el manifiesto y la miniatura de la primera página.
- `gafira-engine`: `save`, `save_as`, `save_copy` y `save_version` dibujan la primera página con `PageRenderers` (spec 030) a 512 px como máximo en el lado largo y la pasan al repositorio. Un dibujo que falla no hace fallar el guardado.
- `gafira-ui`: `RecentStore::thumbnail` y la tarjeta de «Recientes» con la miniatura como textura de egui; sin miniatura queda el marcador. `apps/gafira` lo compone sobre `gafira-format`.

No alcance:

- Miniaturas de las demás páginas, de las versiones (spec 031) y de las secuencias de video, y la portada `previews/cover.webp` de 008.
- Leer las miniaturas en segundo plano tras la Pantalla de arranque (03 §8.4 las clasifica como paso diferible): en este corte se leen al mostrar Inicio, a lo sumo cuatro.
- El autoguardado: las ranuras de recuperación no llevan miniatura.
- La vista del explorador de archivos del sistema.

## Dependencias

- Specs 019 y 026 (`.gaf`), 025 (recientes), 030 (`PageRenderers`, `GraphPageRenderers` sobre `gafira-render-cpu`), 031 (versiones, misma ruta de guardado).
- D-68 (`thumbs/` en lugar de `previews/pages/`), 02 §7.1, 03 §7.2.
- ARQ-06: `gafira-format` no ve `gafira-render`; la UI no depende de `gafira-format` (el puerto `RecentStore` lo inyecta la raíz de composición).
- `image-webp` (MIT o Apache-2.0), que ya estaba en el árbol por `krilla`.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S33-01 | El puerto lleva la miniatura. | `Thumbnail { page, image }`, `THUMBNAIL_SIDE = 512`; `save_with_thumbnail` por defecto llama a `save`, así que un repositorio que no guarda miniaturas sigue funcionando. |
| S33-02 | El `.gaf` guarda la miniatura. | `thumbs/<page_id>.webp`, WebP sin pérdida, `Stored`; leerla devuelve los mismos píxeles; dos escrituras dan los mismos bytes; sin miniatura el archivo es el de antes. Una miniatura que no se puede escribir (máscara, lado 0 o mayor que 512) se omite y el guardado sigue. |
| S33-03 | Las miniaturas hostiles son corruptas al abrir. | Una entrada bajo `thumbs/` cuyo nombre no es `<número>.webp` o que pasa de `Limits::max_thumb_bytes` hace que `read_gaf` devuelva `Corrupt`. |
| S33-04 | La miniatura se lee sin cargar el proyecto. | `read_thumbnail` devuelve la miniatura de la primera página que lista el manifiesto aunque `document.json` o las páginas estén rotos; `None` sin páginas o sin la entrada; `Corrupt` con un manifiesto que no es de Gafira, una entrada demasiado grande, bytes que no son WebP, una imagen animada o un lado 0 o mayor que 512. |
| S33-05 | Guardar escribe la miniatura. | `save`, `save_as`, `save_copy` y `save_version` pasan la miniatura de la primera página: el lado largo mide 512 px (o el de la página, si es menor), con la proporción de la página; un documento sin páginas no lleva. El autoguardado no la pasa. |
| S33-06 | Un dibujo fallido no estropea el guardado. | Si `PageRenderer::render` falla, el archivo se escribe sin miniatura y el guardado devuelve `Ok`. |
| S33-07 | Inicio muestra la miniatura. | `RecentStore::thumbnail(path)` (por defecto `None`); la tarjeta dibuja la textura dentro del marcador manteniendo la proporción; sin miniatura queda el marcador `surface`. Las texturas se guardan por ruta y por un hash de los píxeles: la misma miniatura no se vuelve a subir. |

## TDD

Lógica crítica (formato de archivo, lectura hostil, regla de tamaño): S33-02..S33-06 con tests que fallan primero en `gafira-format` (`tests/thumbs.rs`) y `gafira-engine` (`thumbnail::tests`, `project::tests`) con los dobles en memoria. S33-07 con tests de la caché en `gafira-ui` (`recent::tests`).

## Evidencia esperada

Tests nuevos en `gafira-format`, `gafira-engine` y `gafira-ui`; gates de `AGENTS.md` («Before calling something done») en verde, con `cargo deny check licenses bans sources`.

## Evidencia

- Rojo verificado: con `write_gaf` aceptando la miniatura sin escribirla y `read_thumbnail` devolviendo `None`, fallaron 6 de los 8 tests de `tests/thumbs.rs` (los otros dos prueban la ausencia de miniatura); con `thumbnail::size` devolviendo `None` y el engine llamando a `save` sin miniatura fallaron 5 de los 7 tests de `thumbnail::tests`; sin `save_with_thumbnail` en el puerto, el test de `gafira-ports` no compiló.
- Mutantes a mano muertos: sin la comprobación del nombre en `read_gaf` (muere `s33_03_a_misnamed_thumbs_entry_is_corrupt`); sin la comprobación de tamaño de `read_gaf`, y con `>=` en lugar de `>` en `read_gaf` y en `read_thumbnail` (mueren por `s33_03_an_oversized_thumbs_entry_is_corrupt`, que prueba también el límite exacto); sin la comprobación de lados al decodificar (muere `s33_04_a_hostile_thumbnail_is_corrupt`); sin `.min(1.0)` en la escala y con `floor` en lugar de `round` (muere `s33_05_a_small_page_keeps_its_size_and_a_thin_side_one_pixel`); la última página en lugar de la primera (muere `s33_05_every_save_passes_the_thumbnail_of_the_first_page`).
- Mutante equivalente: quitar a la vez las dos comprobaciones de tamaño de `read_thumbnail` sigue dando `Corrupt`, porque la lectura se corta en `max_thumb_bytes + 1` bytes y un WebP cortado no decodifica; solo cambia el texto del error.
- 23 tests nuevos: `gafira-ports` 1, `gafira-format` 11 (`tests/thumbs.rs` 9, `thumbs::tests` 2), `gafira-engine` 7 (`thumbnail::tests`), `gafira-ui` 4 (`recent::tests`). Los que escriben en disco borran su carpeta temporal.
- `cargo fmt --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 2940 tests, 0 fallos.
- `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` y `cargo test -p gafira-testkit --features ports --locked`: limpios (115 tests).
- `cargo deny check licenses bans sources`: `bans ok, licenses ok, sources ok`.
- Pasos de política de `.github/workflows/ci.yml` en local (ARQ-07, `forbid(unsafe_code)`, V5, ARQ-06, V12, ARQ-107 x2, V13, V14, ADR 0013): todos con salida 0.
- `python3 tools/sync_agents.py --check`: 0.
- La UI no se probó a mano en una ventana: S33-07 queda cubierto por los tests de la caché y del encaje; el dibujo de la textura en la tarjeta no tiene test automático.

## Decisiones

- **WebP**: `image-webp` 0.2 (MIT o Apache-2.0) tiene codificador sin pérdida y decodificador, y ya estaba en el árbol por `krilla`; `cargo deny` lo acepta. Por eso el archivo lleva `thumbs/<page_id>.webp`, como dicen 02 §7.1 y 03 §7.2, sin desviarse a PNG. Se guarda `Stored` en el ZIP porque ya va comprimido.
- **Capas**: el engine dibuja la página con el puerto `PageRenderers` de la exportación (spec 030) y pasa los píxeles (`ImageBuf`) al repositorio; `gafira-format` los convierte a sRGB de 8 bits sin premultiplicar y los codifica. Para eso `straight_srgb8` pasa a `gafira-raster` (capa 1), y `gafira-format` no depende de `gafira-io-image` ni de `gafira-render` (ARQ-06). `gafira-io-image` la reexporta, así `gafira-io-pdf` no cambia.
- **Puerto**: `save_with_thumbnail` es un método nuevo con implementación por defecto que llama a `save`, en lugar de cambiar la firma de `save`: los dobles de prueba que solo implementan `save` (en `gafira-ports`, `gafira-engine` y `gafira-ui`) siguen igual.
- **Tamaño**: la escala es `min(1, 512 / lado largo en px)`; cada lado se redondea y queda entre 1 y 512. Una página más pequeña no se amplía. Se dibuja sobre el fondo de la página, sin transparencia.
- **Fallos**: el engine no tiene registro propio; un dibujo que falla da `None` y el archivo se escribe sin miniatura, como el bloqueo de archivo (spec 028), que también es de mejor esfuerzo. Una miniatura que el formato no puede codificar (máscara, lado 0 o mayor que 512) también se omite.
- **Lectura**: `read_thumbnail` abre el ZIP, comprueba el `mimetype` y el manifiesto (`format`), toma el primer id de `pages` y lee solo `thumbs/<id>.webp` con el tope `Limits::max_thumb_bytes` (4 MiB); el WebP debe ser una imagen fija de 1 a 512 px por lado. Un WebP sin alfa se devuelve como RGBA opaco. `read_gaf` no carga las miniaturas: solo comprueba el nombre y el tamaño de cada entrada de `thumbs/`.
- **UI**: `RecentStore::thumbnail` tiene implementación por defecto `None`; `apps/gafira` la implementa con `read_thumbnail` y escribe los errores en stderr. El shell lee las miniaturas de las tarjetas visibles al crear Inicio y guarda las texturas en una `ThumbCache` por ruta y hash de los píxeles, que olvida los proyectos que ya no se muestran.

## Riesgos y decisiones abiertas

- WebP sin pérdida ocupa más que con pérdida; a 512 px son decenas o cientos de KiB por proyecto. `image-webp` no codifica con pérdida.
- Las miniaturas se leen en el hilo de la UI al mostrar Inicio (a lo sumo cuatro archivos, solo el directorio del ZIP, el manifiesto y una entrada); 03 §8.4 pide leerlas en segundo plano.
- Guardar dibuja la primera página en el hilo que guarda, con el backend de CPU; en documentos pesados añade ese tiempo a cada guardado.
- El hash de la caché (`DefaultHasher`) recorre los píxeles cada vez que se crea Inicio, no en cada fotograma.
