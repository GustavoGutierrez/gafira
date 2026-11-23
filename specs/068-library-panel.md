# 068 — Biblioteca: archivos importados con vista previa

> **Estado:** in-progress — 2026-11-21 — pedida por el titular con capturas de referencia. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Que la pestaña Biblioteca muestre los archivos que la persona ya importó, con su vista previa, para volver a usarlos sin buscarlos en el disco: un clic, un doble clic o arrastrarlos al lienzo.

## Alcance / no alcance

Alcance: «Importar archivo…» (PNG, JPEG, SVG, PDF; varios a la vez); la lista de importados en `<data_dir>/library.json` (`gafira-platform::ImportedFiles`), más recientes primero, sin repetidos (un SVG puede estar en las dos pestañas), con su fecha, hasta 200; búsqueda por nombre; pestañas «Imágenes» (PNG, JPEG) y «Diseños» (SVG, PDF: contornos de marcos, spec 066 S66-10); miniaturas; añadir con clic, doble clic o arrastre (D-105).

No alcance: carpetas, etiquetas, la búsqueda por colores, borrar de la lista desde el panel.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S68-01 | Lista de importados. | Toda importación (diálogo, soltar o pegar, spec 067) se recuerda; la lista va más reciente primero, sin repetidos, hasta 200; un archivo borrado o movido deja de mostrarse (se relee cada 3 s y se reescribe sin él); un `library.json` roto se lee como vacío. |
| S68-02 | Pestañas. | Cada archivo se muestra en la pestaña con la que entró: «Imágenes» muestra PNG, JPEG y los SVG traídos como formas vectoriales (spec 067) como miniaturas; «Diseños» muestra los SVG traídos como marco y los PDF como la silueta gris de su contorno, con el nombre del archivo y su tamaño en px (SVG: su tamaño; PDF: la página a 96 ppp). |
| S68-03 | Búsqueda. | «Buscar en tus archivos» deja los archivos de la pestaña cuyo nombre contiene el texto, sin distinguir mayúsculas. |
| S68-05 | Seleccionar y quitar. | En las dos pestañas, al pasar el puntero una miniatura muestra una casilla (arriba a la izquierda) y «···» (arriba a la derecha). Con alguna marcada, todas muestran su casilla y abajo aparece «× N seleccionados» con la papelera, que quita los marcados de la lista (los archivos siguen en el disco); «×» desmarca. «···» ofrece «Más información» y «Quitar de la lista». |
| S68-06 | Más información. | Muestra el nombre con una flecha para volver, la vista previa grande y «Detalles»: carpeta («Guardado en»), tipo (Imagen o Diseño), formato (PNG, JPEG, SVG, PDF), tamaño en disco (MB o KB), dimensiones en px y fecha de importación (día/mes/año), además de «Quitar de la lista» y la ruta completa. |
| S68-04 | Añadir. | Un clic o un doble clic (una sola vez) añade el archivo a la página actual; arrastrarlo lo deja centrado donde se suelta; sobre un marco, una imagen pasa a ser su foto; un diseño crea un marco personalizado. |

## Tests

- `gafira-platform`: `s68_01_newest_first_once_and_only_existing_files`, `s68_01_broken_or_long_lists`.
- `gafira-ui`: `s68_03_tabs_and_search`, `s68_06_date_and_size`, `s68_04_imports_are_remembered_and_added_again` (también quitar de la lista sin borrar el archivo).
- `gafira-io-vector`: el tamaño del documento en `s66_10_*`.

## Riesgos

- Las miniaturas se crean la primera vez que se muestran, en el hilo de la interfaz; con muchas imágenes grandes la primera apertura puede tardar.
- La lista guarda rutas: si el archivo cambia en el disco, la miniatura de la sesión no se actualiza hasta reabrir Gafira.
