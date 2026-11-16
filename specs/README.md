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
| — | `gafira-engine` (change del flujo anterior, [carpeta](done/legacy/changes/gafira-engine/)) | in-progress: crate de capa 3 de 02; E1, E2, E3a-E3d y E5 verificadas e integradas en `develop`; pendientes E4 (necesita SP5), E6, E7 (necesita E2-W7 y decidir cómo llega al governor, E5-S1) y E8-E11 (E11 necesita SP3). Al retomarlo, abrir una spec con ID nuevo que cite esa carpeta |

## Done

| ID | Spec | Evidencia |
| --- | --- | --- |
| — | (ninguna todavía con el flujo nuevo) | — |

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

Siguiente ID libre: **009**.
