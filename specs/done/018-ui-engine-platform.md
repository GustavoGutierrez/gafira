# 018 — La UI sobre el engine y el adaptador `gafira-platform`

> **Estado:** done — 2026-11-16 — S18-01..S18-07 cubiertos por `gafira-platform` (6 tests) y `gafira-ui` (2 tests con el engine real); comprobado a mano: la app pasa sola de la Pantalla de arranque a Inicio y el editor crea, añade páginas, deshace y rehace.

## Objetivo

Que la ventana trabaje sobre el engine de verdad: «Empezar a diseñar» crea un documento con la página del formato, el lienzo dibuja la instantánea del documento, y deshacer, rehacer y «Añadir página» pasan por `EditService` (02 §5.4, §5.5).

## Alcance / no alcance

Alcance:

- `crates/gafira-platform` (capa 4): `StdFileSystem` (renombrado atómico y sincronización de la carpeta en Unix), `SystemClock`, `RandomIds` (UUID v4, D-63), `EnvLocale`, `StderrNotifier`, `JsonSettings` (`<datos_app>/settings.json`, escritura atómica) y `data_dir()` por sistema; `compile_error!` fuera de los tres targets (ARQ-03).
- `gafira-engine`: `EditService::document()` pasa a ser pública (instantánea inmutable, 02 §5.5) y se añade `can_undo_redo()`.
- `gafira-ui`: el shell recibe una fábrica del engine, la construye con un `Waker` que repinta egui, arranca `SettingsService` (idioma guardado o del sistema; `--lang` lo cambia solo para esa ejecución), crea el documento con `new_document` + `InsertPage`, dibuja todas las páginas apiladas, muestra «Cambios sin guardar»/«Cambios guardados.», y atiende Deshacer/Rehacer (botones, `Ctrl+Z`, `Ctrl+Shift+Z`, `Ctrl+Y`) y «Añadir página».
- `apps/gafira`: raíz de composición con los adaptadores reales y `NoLicensing` (D-62).

No alcance:

- Abrir y guardar `.gaf`: el repositorio llega con `gafira-format` (008, D-68); hasta entonces la app compone un repositorio y un almacén que responden «no disponible en esta compilación».
- Selección, elementos, texto e imágenes en el lienzo; menús con contenido.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S18-01 | `StdFileSystem` cumple el puerto. | Crear, escribir, sincronizar, reemplazar y leer de vuelta; los errores vuelven como `io::Error`. |
| S18-02 | `SystemClock` da hora real y monótona. | `now` es posterior a 2026; `monotonic` no retrocede. |
| S18-03 | `RandomIds` da UUID v4. | Distintos, nunca nulos, con los bits de versión 4 y de variante RFC 9562. |
| S18-04 | `EnvLocale` convierte valores POSIX en etiquetas. | `es_MX.UTF-8` → `es-MX`; `C`, `POSIX` y vacío no dicen nada. |
| S18-05 | `JsonSettings` guarda y lee los ajustes. | Sin archivo da los valores por defecto; lo guardado se lee igual; un archivo roto es `Corrupt`. |
| S18-06 | «Empezar a diseñar» crea el documento del formato. | Una página del tamaño del formato (A4 en mm a 96 dpi) y un id de proyecto no nulo. |
| S18-07 | Las acciones del editor pasan por el engine. | «Añadir página» añade una del tamaño de la última; deshacer y rehacer la quitan y la ponen; el documento queda modificado; «Inicio» vuelve a Inicio. |

## Notas

- La primera página es la primera edición del documento, así que «Deshacer» puede quitarla. Un `new_document` que reciba el tamaño de la primera página es un hueco del engine para la siguiente spec.
- `datos_app` sigue a XDG en Linux, `%APPDATA%` en Windows y `~/Library/Application Support` en macOS.
