# 019 — `gafira-format`: leer y escribir `.gaf`, y el menú Archivo

> **Estado:** done — 2026-11-16 — S19-01..S19-11 cubiertos por `gafira-format` (10 tests), `gafira-ui` (2) y `gafira-engine` (`path`); 5 mutantes a mano muertos (uno tras reforzar el test del `mimetype`). La app abre y guarda proyectos `.gaf` reales.

## Objetivo

Que Gafira guarde y abra proyectos: el adaptador del formato `.gaf` con la disposición híbrida de D-68 (D-67, D-69) y el menú Archivo de 03 §2.2 en la ventana.

## Alcance / no alcance

Alcance:

- `crates/gafira-format` (capa 4, `zip` 2 con DEFLATE):
  - Escritura: `mimetype` primero y sin comprimir; `manifest.json` en camelCase con `format`, `formatVersion` `1.0.0`, `schemaVersion`, `generator`, `projectId`, `profile`, `features`, `packageCompleteness`, `missingResources`, `pages`, `sequences`, `assets` (hash → nombre) y `versions`; `document.json` sin páginas ni secuencias; `pages/<id>.json` y `sequences/<id>.json`. Misma entrada, mismos bytes (fecha fija en cada entrada).
  - Lectura como archivo hostil (008 §11): nombres de entrada seguros (sin `..`, rutas absolutas, `\`, `:` ni segmentos vacíos), límites de entradas y de tamaño de JSON, rechazo de una relación de compresión de bomba ZIP, `mimetype` exacto, manifiesto de Gafira, páginas y secuencias que coinciden con el manifiesto, y el mismo proyecto en manifiesto y documento.
  - `formatVersion` de mayor versión o una capacidad requerida desconocida abren en solo lectura (ARQ-55); un `schemaVersion` mayor es `TooNew` (el modelo no lo puede representar).
  - `GafRepository` implementa `ProjectRepository` y deja el archivo sincronizado (SP6).
- `gafira-engine`: `ProjectService::path()`.
- `gafira-ui`: menú Archivo (Nuevo, Abrir…, Guardar, Guardar como…, Guardar copia…), `Ctrl+O`, `Ctrl+S`, `Ctrl+Shift+S`, «Abrir archivo» en Inicio, diálogo del sistema (`rfd` con el portal XDG), extensión `.gaf` añadida si falta, nombre del proyecto en la barra superior y aviso de fallo en la línea de estado.
- `apps/gafira`: compone `GafRepository`.

No alcance:

- Recursos embebidos en `assets/` (llega con el `ContentStore` real), `thumbs/`, `masks/`, `versions/` (spec 016), `extensions/`.
- Migraciones (no hay `schemaVersion` 2), `createdAt`/`updatedAt`, perfiles distintos de `portable-full` y medios vinculados (Fase 2).
- Los puntos que D-68 deja a la spec del formato (forma de la tabla de recursos, `originalPathHint`, portada y miniaturas de secuencia, `masks/`/`extensions/`/`metadata/`, perfil de Guardar): esta entrega escribe `portable-full` porque todo va embebido.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S19-01 | La disposición es la de D-68. | Entradas `mimetype`, `manifest.json`, `document.json`, `pages/<id>.json`, `sequences/<id>.json`; `mimetype` sin comprimir con el texto exacto. |
| S19-02 | El manifiesto lleva los campos de D-68. | Claves camelCase con los valores de arriba. |
| S19-03 | Ida y vuelta exacta y determinista. | Leer lo escrito da el mismo `Document`; dos escrituras dan los mismos bytes. |
| S19-04 | Lo que no es un `.gaf` es `Corrupt`. | No ZIP, sin `mimetype` primero, comprimido, distinto o con cola, sin manifiesto. |
| S19-05 | Nombres de entrada inseguros se rechazan. | `..`, absolutos, `\`, `:` y `//`. |
| S19-06 | Una bomba ZIP se rechaza. | Una entrada de 8 MiB de ceros comprimida. |
| S19-07 | Las partes deben coincidir con el manifiesto. | Página que falta, otro proyecto, otro id, otro `format`. |
| S19-08 | Lo futuro abre en solo lectura o es `TooNew`. | `formatVersion` 2 y capacidad desconocida: solo lectura; 1.4: normal; `schemaVersion` 2: `TooNew`. |
| S19-09 | El repositorio escribe y lee archivos reales. | Guardar, leer de vuelta; un archivo que falta es `Io`. |
| S19-10 | La extensión `.gaf` se añade una vez. | `poster` → `poster.gaf`; `poster.gaf` y `.GAF` se quedan. |
| S19-11 | Guardar como y abrir conservan el proyecto. | Con el engine real: páginas, id y ruta iguales tras reabrir; sin cambios pendientes. |
