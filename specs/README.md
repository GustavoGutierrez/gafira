# Specs — Gafira

Índice del ciclo de vida de las specs. Este archivo **no es una spec**; las reglas completas viven en [`.agents/skills/gafira-specs/SKILL.md`](../.agents/skills/gafira-specs/SKILL.md).

## Reglas rápidas

- Numeración de tres dígitos (`008`, `009`…); el ID no cambia al mover la spec de carpeta.
- `specs/` — spec en progreso o en ejecución estable.
- `specs/backlog/` — no iniciada, pendiente de revisión o decisión, bloqueada antes de empezar o solo una idea.
- `specs/done/` — completada y verificada. No se reabre en el sitio: un cambio de alcance crea una spec nueva.
- **TDD solo para lógica crítica** (modelo de documento, carga/guardado JSON y validación, comandos e historial de edición, tiempo y timecode, color, geometría y path ops, límites, parsing de formato); el resto se valida con tests de integración por feature; suite completa y gates al cerrar una spec. Sin tests y evidencia no se mueve a `done/`.

## Documentos de producto (referencia, no specs de ciclo de vida)

Conservan su nombre de dos dígitos y su propia fila `Estado`; 01, 02, 04 y 05 están aprobados cuando no tienen ninguna decisión del titular abierta (D-83). Las specs nuevas citan sus IDs y secciones.

| Doc | Contenido | Estado |
| --- | --- | --- |
| [00](00-decisiones.md) | Decisiones del titular (`D-nn`); prevalecen sobre 01-07 y no se reabren sin el titular | Vigente (D-01 a D-86) |
| [01](01-producto-y-calidad.md) | Producto, calidad y criterios de aceptación (`RF-`, `RNF-`, `CAL-`) | Aprobado (D-83, 2026-11-16) |
| [02](02-arquitectura-software.md) | Arquitectura de software por capas y crates (`ARQ-`) | Aprobado (D-83, 2026-11-16) |
| [03](backlog/03-diseno-visual-e-interaccion.md) | Diseño visual e interacción (`DIS-`); en `backlog/` | Borrador para revisión |
| [04](04-funcionalidad-diseno-imagen-exportacion.md) | Diseño, imagen y exportación (`FD-`) | Aprobado (D-83, 2026-11-16) |
| [05](05-funcionalidad-video-y-distribucion.md) | Video y distribución (`FV-`) | Aprobado (D-83, 2026-11-16) |
| [06](backlog/06-licencia-y-activacion.md) | Licencia y activación; en `backlog/` | Esqueleto |
| [07](07-deuda-abierta.md) | Inventario de deuda abierta; no crea requisitos | Vigente (no ejecutable) |

## En progreso / ejecución estable

| ID | Spec | Estado |
| --- | --- | --- |
| — | `gafira-engine` (change del flujo anterior, [carpeta](done/legacy/changes/gafira-engine/)) | in-progress: crate de capa 3 de 02; E1, E2, E3a-E3d y E5 verificadas e integradas en `develop`; E10 cerrada por la spec 009, E9 por la 010 y E6 (importación embebida) por la 011; E4 por la 012 (identidad, «Guardar como», «Guardar copia», `.bak`) y la 016 (recuperación, `close`), y la cierran la 028 (bloqueos) y la 031 (versiones dentro del `.gaf`); pendientes E6 Fase 2 (vínculos y `relocate`), E7 (necesita E2-W7 y decidir cómo llega al governor, E5-S1), E8 y E11 (ya no la bloquea SP3, cerrado por D-75); los huecos SP1-SP8 están en [02 §5.6](02-arquitectura-software.md#56-huecos-abiertos-del-engine) y están todos cerrados (SP7 por D-76). Al retomarlo, abrir una spec con ID nuevo que cite esa carpeta |

## Done

| ID | Spec | Evidencia |
| --- | --- | --- |
| 009 | [`SettingsService` del engine e idioma persistido](done/009-engine-settings-service.md) | Puerto `SettingsRepository`, `SettingsService::{start, get, set, set_language, locale}`, `LocaleChanged` al arrancar y al cambiar idioma; 13 tests nuevos, 3 mutantes a mano muertos, 2649 tests en el workspace |
| 010 | [`LicenseService` del engine y licencia opcional](done/010-engine-license-service.md) | `LicenseService::{start, status, can_export, is_required, activate, validate, deactivate}`; licencia deshabilitable por build con `NoLicensing` → `NotRequired` (D-58); 9 tests nuevos, 4 mutantes a mano muertos, 2658 tests |
| 011 | [`AssetService::import` del engine](done/011-engine-asset-import.md) | Comando `InsertAsset` deduplicado por hash y `AssetService::import` (sesión → archivo → store → comando, deshacer incluido); 9 tests nuevos, 4 mutantes a mano muertos, 2667 tests |
| 012 | [Identidad del proyecto, «Guardar como», «Guardar copia» y `.gaf.bak`](done/012-engine-save-as-and-backup.md) | `ProjectId` en el documento (D-63), puerto `IdSource` y `SystemPorts`, ranuras de recuperación por id, `save_as`, `save_copy` y una generación `.bak` atómica; 14 tests nuevos, 6 mutantes a mano muertos, 2681 tests |
| 013 | [Mutantes críticos de `gafira-text` y `gafira-raster`](done/013-mutant-debt-text-raster.md) | D-66, ítems 74 y 75 de 07, solo tests: de 98 críticos, 57 muertos, 34 equivalentes excluidos en `.cargo/mutants.toml` y 7 sin matar en la frontera de una tolerancia; 24 tests nuevos, 2691 tests |
| 014 | [`gafira-theme`: tokens de diseño y tipografías](done/014-theme-tokens.md) | `assets/theme/gafira.tokens.json` (03 §3) cargado y validado (schema, referencias, modos, contraste DIS-10, pares rechazados, movimiento reducido) en tipos neutros; Inter y Manrope embebidas (subconjuntos provisionales) con prueba `cmap` de ARQ-105; 24 tests nuevos, 9 de 10 mutantes a mano muertos (1 equivalente), 2718 tests |
| 015 | [`gafira-i18n`: catálogos es/en y regla de D-08](done/015-i18n-catalogs.md) | `assets/i18n/{es,en}.toml` compilados en build; `D08LocaleResolver` (D-08, región D-64) y `CatalogTranslator` (parámetros, plurales CLDR, respaldo de §14.6, números y cantidades con ICU4X, tiempo relativo) y `PseudoTranslator` (feature `pseudo`); fechas como interim ISO 8601 UTC; 27 tests nuevos, 8 de 7 mutantes a mano muertos (1 equivalente), 2718 tests |
| 016 | [Recuperación tras fallo, `close` y la primera página](done/016-engine-recovery-and-close.md) | Marcador de ranuras, `recoverable`/`recover`/`discard_recovery`, `close`, `new_document_with`; diálogos de recuperación y «¿Guardar los cambios?»; autoguardado en hilo |
| 017 | [Primer shell de escritorio](done/017-ui-desktop-shell.md) | `gafira-ui` (egui/eframe) y `apps/gafira`: Pantalla de arranque, Inicio y regiones del Editor de diseño con tokens y catálogos reales; 12 tests; la app abre en Linux |
| 018 | [La UI sobre el engine y `gafira-platform`](done/018-ui-engine-platform.md) | Adaptadores reales de archivos, reloj, UUID, idioma y ajustes; la ventana crea el documento, dibuja sus páginas y deshace/rehace por el engine; 8 tests nuevos |
| 019 | [`gafira-format` y el menú Archivo](done/019-gaf-format-and-file-menu.md) | `.gaf` con la disposición de D-68: escritura determinista, lectura como archivo hostil, solo lectura para lo futuro; Abrir, Guardar, Guardar como y Guardar copia en la ventana; 12 tests nuevos |
| 020 | [Primeras piezas en el lienzo](done/020-canvas-first-pieces.md) | Rectángulos y elipses desde Piezas; selección, arrastre en un solo deshacer, posición y tamaño en el Inspector, borrar; 5 tests nuevos |
| 021 | [Menú Ver: modo oscuro](done/021-view-menu-dark-mode.md) | `Settings.appearance` y Ver → Modo oscuro con los tokens de D-82, recordado entre sesiones |
| 022 | [Tiradores de tamaño](done/022-resize-handles.md) | Ocho tiradores en la selección; redimensionar con el borde opuesto fijo en un solo deshacer |
| 023 | [Relleno y opacidad](done/023-fill-and-opacity.md) | Selector de color, «Sin relleno» y opacidad de la forma seleccionada en el Inspector |
| 024 | [Pestaña Capas](done/024-layers-tab.md) | Lista de elementos de la página, selección desde la lista, Subir y Bajar con deshacer |
| 025 | [Proyectos recientes en Inicio](done/025-recent-projects.md) | `recent.json` en la carpeta de datos (12 entradas, sin rutas repetidas, poda de archivos que ya no existen); hasta 4 tarjetas con tiempo relativo que abren el proyecto |
| 026 | [Almacén por contenido y recursos embebidos](done/026-content-store.md) | `gafira-assets` (`FsStore`, BLAKE3, escritura atómica, sin duplicados) compuesto en `<datos_app>/store`; el `.gaf` guarda los embebidos en `assets/<hash>.<ext>` y al abrir comprueba cada hash; 12 tests nuevos |
| 027 | [Texto en el lienzo](done/027-text-on-canvas.md) | `TextNode` en el documento (primer corte de 02 §3.1), `Property::TextContent`, pestaña Texto, dibujo e Inspector de texto |
| 028 | [Bloqueos de archivo y ranuras de otra instancia](done/028-file-locks.md) | Puerto `ProcessProbe` y `SystemProcesses` (`/proc` en Linux, baliza con `File::try_lock` fuera); PID en el marcador; `.gaf.lock` atómico, solo lectura ante un dueño vivo ajeno y toma de uno muerto; 13 tests nuevos, 2572 tests |
| 029 | [Imágenes en el lienzo](done/029-images-on-canvas.md) | `ImageNode` en el documento, «Importar imagen…» (PNG/JPEG) desde Biblioteca, dibujo desde el almacén |
| 030 | [Exportar PNG y PDF](done/030-export-png-pdf.md) | `ExportService` con licencia, registro y escritura por `*.tmp`; `gafira-io-image` (PNG 0,5×–4×, sRGB, 16 384 px) y `gafira-io-pdf` (krilla, una imagen por página); texto como contornos y escala de exportación en `gafira-render`; diálogo «Exportar» (PNG 1×/2× o PDF); 31 tests nuevos, 2880 tests |
| 031 | [Versiones dentro del `.gaf`](done/031-versions.md) | `versions/index.json` y una carpeta por versión, recursos compartidos por hash; el puerto lleva las versiones (`LoadedProject.versions`, `save(.., versions)`); versión automática al guardar (más de 30 min, 20 conservadas, nombradas nunca); `save_version`, `list_versions`, `restore_version` con el comando `RestoreVersion`; «Versiones» y «Guardar versión…» en el editor; 33 tests nuevos, 5 mutantes a mano muertos, 2882 tests |
| 032 | [Imágenes en la exportación PNG y PDF](done/032-export-images.md) | El engine lee y decodifica (puerto `ImageDecoder`, `RasterDecoder` PNG/JPEG en `gafira-io-image`) las imágenes del alcance antes de abrir archivos; un recurso ausente es `StoreError::NotFound`; el grafo emite `DrawImage` en la caja con la opacidad del elemento; 12 tests nuevos, 7 mutantes a mano muertos, 2929 tests |
| 033 | [Miniaturas de los recientes](done/033-thumbnails.md) | `thumbs/<page_id>.webp` (WebP sin pérdida, 512 px en el lado largo) en cada guardado salvo el autoguardado, dibujada con `PageRenderers`; un dibujo fallido guarda sin miniatura; `read_thumbnail` la lee sin abrir el proyecto, como archivo hostil; las tarjetas de «Recientes» la muestran con una caché de texturas; 23 tests nuevos, 8 mutantes a mano muertos, 2940 tests |
| 034 | [Pantalla de Ajustes](done/034-settings-screen.md) | General (idioma al momento), Apariencia (Claro, Oscuro, Sistema), Privacidad y Acerca de; desde Inicio y Archivo |
| 035 | [Exportar en segundo plano](done/035-background-export.md) | `ExportService::prepare` (comprobaciones sin dibujar ni abrir archivos) y `PreparedExport::run` (`Send + 'static`, cancelable); la app exporta en un hilo con tarjeta de progreso y «Cancelar»; un pánico del hilo es un error, no un cierre; 8 tests nuevos, 2 mutantes a mano muertos, 2964 tests |
| 036 | [Guardar en segundo plano](done/036-background-save.md) | `ProjectService::prepare_save(SaveRequest)` → `SaveJob::run` (`Send + 'static`) → `save_finished`; la app guarda en un hilo con tarjeta «Guardando…»; «¿Guardar los cambios?» sigue solo tras un guardado correcto; al cerrar se espera al guardado; 7 tests nuevos, 2971 tests |
| 039 | [Un diálogo «Exportar» más completo](done/039-export-dialog.md) | `JpegExporter` (calidad 1–100, sobre blanco) y `WebpExporter` (sin pérdida) en `gafira-io-image`; `ExportService::preview`; `show_in_folder` en `gafira-platform`; diálogo con PNG/JPG/WebP/PDF y opciones desde `options_schema`, páginas todas/actual/rango, vista previa, tamaño en px y «Mostrar en carpeta»; 18 tests nuevos, 7 mutantes a mano muertos, 2988 tests |

### Historial del flujo anterior

Antes del 2026-11-16 el trabajo seguía un flujo multiagente (ADR 0001, 0002, 0004 y 0011, reemplazados por ADR 0018). Su registro está en [`done/legacy/`](done/legacy/) y no se reescribe:

- [`done/legacy/history/`](done/legacy/history/) — changes cerrados (`<fecha>-<change>/`), desde `workspace-scaffold` (2026-10-24) hasta `gafira-render-cpu` (2026-11-07): capas 0 a 3 de 02.
- [`done/legacy/capabilities/`](done/legacy/capabilities/) — specs principales fusionadas desde delta specs (color, documento, tiempo, edición, geometría, path ops, testkit, CLI, workspace).
- [`done/legacy/changes/`](done/legacy/changes/) — la carpeta de `gafira-engine`, todavía abierta (arriba).
- [`done/legacy/README.md`](done/legacy/README.md) — índice anterior: tabla de estado por documento con los IDs cubiertos por cada change y la lista numerada «Pending follow-ups» (ítems 1-132) que cita `07-deuda-abierta.md`.

## Backlog

| ID | Spec | Motivo |
| --- | --- | --- |
| 008 | [Formato de proyecto GAF v1](backlog/008-gaf-format-v1.md) | Propuesta técnica del titular; pendiente de las decisiones de §17 y de la orden de empezar |

Siguiente ID libre: **037**.
