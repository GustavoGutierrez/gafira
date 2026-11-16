# 033 — Miniaturas de los recientes

> **Estado:** in-progress — 2026-11-16 — rama `feat/033-thumbnails`.

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

## Riesgos y decisiones abiertas

- WebP sin pérdida ocupa más que con pérdida; a 512 px son decenas o cientos de KiB por proyecto.
