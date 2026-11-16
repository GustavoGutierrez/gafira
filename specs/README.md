# Specs — Gafira

Índice del ciclo de vida de las specs. Este archivo **no es una spec**; las reglas completas viven en [`.agents/skills/gafira-specs/SKILL.md`](../.agents/skills/gafira-specs/SKILL.md).

## Reglas rápidas

- Numeración de tres dígitos (`008`, `009`…); el ID no cambia al mover la spec de carpeta.
- `specs/` — spec en progreso o en ejecución estable.
- `specs/backlog/` — no iniciada, pendiente de revisión o decisión, bloqueada antes de empezar o solo una idea.
- `specs/done/` — completada y verificada. No se reabre en el sitio: un cambio de alcance crea una spec nueva.
- **TDD solo para lógica crítica** (modelo de documento, carga/guardado JSON y validación, comandos e historial de edición, tiempo y timecode, color, geometría y path ops, límites, parsing de formato); el resto se valida con tests de integración por feature; suite completa y gates al cerrar una spec. Sin tests y evidencia no se mueve a `done/`.

## Documentos de producto (referencia, no specs de ciclo de vida)

Conservan su nombre de dos dígitos y su propia fila `Estado`. Las specs nuevas citan sus IDs y secciones.

| Doc | Contenido |
| --- | --- |
| [00](00-decisiones.md) | Decisiones del titular (`D-nn`); prevalecen sobre 01-07 y no se reabren sin el titular |
| [01](01-producto-y-calidad.md) | Producto, calidad y criterios de aceptación (`RF-`, `RNF-`, `CAL-`) |
| [02](02-arquitectura-software.md) | Arquitectura de software por capas y crates (`ARQ-`) |
| [03](backlog/03-diseno-visual-e-interaccion.md) | Diseño visual e interacción (`DIS-`); en `backlog/` |
| [04](04-funcionalidad-diseno-imagen-exportacion.md) | Diseño, imagen y exportación (`FD-`) |
| [05](05-funcionalidad-video-y-distribucion.md) | Video y distribución (`FV-`) |
| [06](backlog/06-licencia-y-activacion.md) | Licencia y activación; en `backlog/` |
| [07](07-deuda-abierta.md) | Inventario de deuda abierta; no crea requisitos |

## En progreso / ejecución estable

| ID | Spec | Estado |
| --- | --- | --- |
| — | `gafira-engine` (change del flujo anterior, [carpeta](done/legacy/changes/gafira-engine/)) | in-progress: crate de capa 3 de 02; E1, E2, E3a-E3d y E5 verificadas e integradas en `develop`; E10 cerrada por la spec 009, E9 por la 010 y E6 (importación embebida) por la 011; E4 en parte por la 012 (identidad, «Guardar como», «Guardar copia», `.bak`); pendientes el resto de E4 (`recover`, `RecoveryAvailable`, bloqueos, versiones), E6 Fase 2 (vínculos y `relocate`), E7 (necesita E2-W7 y decidir cómo llega al governor, E5-S1), E8 y E11 (necesita SP3); los huecos SP1-SP8 están en [02 §5.6](02-arquitectura-software.md#56-huecos-abiertos-del-engine). Al retomarlo, abrir una spec con ID nuevo que cite esa carpeta |

## Done

| ID | Spec | Evidencia |
| --- | --- | --- |
| 009 | [`SettingsService` del engine e idioma persistido](done/009-engine-settings-service.md) | Puerto `SettingsRepository`, `SettingsService::{start, get, set, set_language, locale}`, `LocaleChanged` al arrancar y al cambiar idioma; 13 tests nuevos, 3 mutantes a mano muertos, 2649 tests en el workspace |
| 010 | [`LicenseService` del engine y licencia opcional](done/010-engine-license-service.md) | `LicenseService::{start, status, can_export, is_required, activate, validate, deactivate}`; licencia deshabilitable por build con `NoLicensing` → `NotRequired` (D-58); 9 tests nuevos, 4 mutantes a mano muertos, 2658 tests |
| 011 | [`AssetService::import` del engine](done/011-engine-asset-import.md) | Comando `InsertAsset` deduplicado por hash y `AssetService::import` (sesión → archivo → store → comando, deshacer incluido); 9 tests nuevos, 4 mutantes a mano muertos, 2667 tests |
| 012 | [Identidad del proyecto, «Guardar como», «Guardar copia» y `.gaf.bak`](done/012-engine-save-as-and-backup.md) | `ProjectId` en el documento (D-63), puerto `IdSource` y `SystemPorts`, ranuras de recuperación por id, `save_as`, `save_copy` y una generación `.bak` atómica; 14 tests nuevos, 6 mutantes a mano muertos, 2681 tests |
| 013 | [Mutantes críticos de `gafira-text` y `gafira-raster`](done/013-mutant-debt-text-raster.md) | D-66, ítems 74 y 75 de 07, solo tests: de 98 críticos, 57 muertos, 34 equivalentes excluidos en `.cargo/mutants.toml` y 7 sin matar en la frontera de una tolerancia; 24 tests nuevos, 2691 tests |
| 014 | [`gafira-theme`: tokens de diseño y tipografías](done/014-theme-tokens.md) | `assets/theme/gafira.tokens.json` (03 §3) cargado y validado (schema, referencias, modos, contraste DIS-10, pares rechazados, movimiento reducido) en tipos neutros; Inter y Manrope embebidas (subconjuntos provisionales) con prueba `cmap` de ARQ-105; 24 tests nuevos, 9 de 10 mutantes a mano muertos (1 equivalente), 2718 tests |
| 015 | [`gafira-i18n`: catálogos es/en y regla de D-08](done/015-i18n-catalogs.md) | `assets/i18n/{es,en}.toml` compilados en build; `D08LocaleResolver` (D-08, región D-64) y `CatalogTranslator` (parámetros, plurales CLDR, respaldo de §14.6, números y cantidades con ICU4X, tiempo relativo) y `PseudoTranslator` (feature `pseudo`); fechas como interim ISO 8601 UTC; 27 tests nuevos, 8 de 9 mutantes a mano muertos (1 equivalente), 2718 tests |
| 017 | [Primer shell de escritorio](done/017-ui-desktop-shell.md) | `gafira-ui` (egui/eframe) y `apps/gafira`: Pantalla de arranque, Inicio y regiones del Editor de diseño con tokens y catálogos reales; 12 tests; la app abre en Linux |
| 018 | [La UI sobre el engine y `gafira-platform`](done/018-ui-engine-platform.md) | Adaptadores reales de archivos, reloj, UUID, idioma y ajustes; la ventana crea el documento, dibuja sus páginas y deshace/rehace por el engine; 8 tests nuevos |

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

Siguiente ID libre: **019** (la 016, segunda parte de E4, está reservada).
