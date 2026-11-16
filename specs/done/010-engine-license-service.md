# 010 — `LicenseService` del engine y licencia opcional

> **Estado:** done — 2026-11-16 — E9 del change `gafira-engine` ([carpeta legacy](legacy/changes/gafira-engine/design.md)) con el interruptor de D-58; S10-01..S10-08 cubiertos por `gafira-ports/tests/license.rs` (2 nuevos) y `gafira-engine` (`license::tests`, 6, y `project::tests::s10_08_*`); 4 mutantes a mano muertos; 2658 tests en el workspace.

## Objetivo

Que el engine exponga el estado de la licencia y sus acciones (02 §5.4 línea 1363, §4.14, §13.3) y publique `LicenseChanged`, y que la licencia se pueda **deshabilitar** por build sin tocar el engine ni la UI (D-58): Gafira puede publicarse sin licencia.

## Alcance / no alcance

Alcance:

- `gafira-ports::license`: `LicenseStatus::NotRequired`, `LicenseStatus::is_required`, `LicenseError::Disabled` y el adaptador `NoLicensing`.
- `gafira-engine`: `LicenseService::{start, status, can_export, is_required, activate, validate, deactivate}`, `Gafira::license()`, `EngineError::License`.

No alcance:

- `gafira-license` (token Ed25519, cliente de red, llavero): capa 4, solo si se habilita la licencia.
- La pantalla de activación y el estado en Ajustes (03), y la política de validación diaria (§13.3, PROPUESTA): la decide el host.
- La lista de funciones bloqueadas de D-12 (spec 06).

## Dependencias

- E3d del change `gafira-engine` (en `develop`); `Ports.license` y `Ports.clock` ya existen.
- `EngineEvent::LicenseChanged(LicenseStatus)` (E1).
- D-12, D-55, D-58. ADR 0016: el engine no hace red ni hilos; las llamadas de red del puerto son bloqueantes y el host las ejecuta fuera del hilo de UI.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S10-01 | `NotRequired` permite exportar y no pide licencia (D-58). | `NotRequired.can_export()` y `!NotRequired.is_required()`; los otros cinco estados dan `is_required()`; la tabla de `can_export` de §4.14 no cambia para ellos. |
| S10-02 | `NoLicensing` es la licencia deshabilitada. | `status` devuelve `NotRequired` para cualquier `now`; `activate`, `validate` y `deactivate` devuelven `LicenseError::Disabled`, con clave `error.license.disabled`. |
| S10-03 | `start` lee el estado sin red con la hora del `Clock` y publica `LicenseChanged` una vez. | Con un verificador en `Active`, `start` devuelve `Active`, `status()` es `Active` y hay un único `LicenseChanged(Active)`. |
| S10-04 | `activate` y `validate` adoptan el estado que devuelve el puerto y publican solo si cambió. | `Unlicensed` → `activate` → `Active`: publica `LicenseChanged(Active)`; un `validate` que vuelve a dar `Active` igual no publica. |
| S10-05 | Un error del puerto no cambia el estado (ARQ-92: la red no invalida un token). | `validate` con `Err(Network)` devuelve `Err(EngineError::License(Network))`, `status()` sigue igual y no se publica nada. |
| S10-06 | `deactivate` correcto vuelve a leer el estado y lo publica. | `Active` → `deactivate` → `status()` es lo que da `status(now)` (`Unlicensed`) y se publica `LicenseChanged(Unlicensed)`. |
| S10-07 | Con la licencia deshabilitada, el servicio no pide nada. | Con `NoLicensing`: `start` da `NotRequired`, `can_export()` y `!is_required()`; `activate` da `Err(EngineError::License(Disabled))` sin evento. |
| S10-08 | La licencia nunca bloquea guardar (D-55). | Con `Unlicensed`, `projects().save` de una sesión con ruta escribe igual que con `Active` (el servicio no participa en guardar). |

## TDD

Lógica crítica (TDD estricto): S10-01 y S10-02 (predicados y adaptador, política de licencia) y S10-03..S10-07 (transiciones y eventos). Tests que fallan primero:

- `gafira-ports`: `tests/license.rs` → `s10_01_*`, `s10_02_*`.
- `gafira-engine`: `license::tests::s10_03_*` a `s10_08_*`, con un verificador falso guionizado.

## Evidencia esperada

- Gates de `AGENTS.md` en verde; mutantes a mano en el cambio de estado y el filtro de eventos.
- Banner a `done` y fila en `specs/README.md`.

## Riesgos y decisiones abiertas

- Activar o validar no se reintenta en el engine: la política de reintento es del llamador (§13.2).
- `can_export` aún no tiene consumidor: `ExportService` (E7) lo consultará.
