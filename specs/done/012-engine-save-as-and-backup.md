# 012 — Identidad del proyecto, «Guardar como», «Guardar copia» y `.gafira.bak`

> **Estado:** done — 2026-11-16 — primera parte de E4 del change `gafira-engine` ([carpeta legacy](legacy/changes/gafira-engine/design.md)), desbloqueada por D-63; S12-01..S12-09 cubiertos por `gafira-doc` (`project_id::tests` 5, `json::tests` 1) y `gafira-engine` (`project::tests` 8 nuevos, más los de `save` y autoguardado ajustados al `.bak` y a la carpeta por id); 6 mutantes a mano muertos; 2681 tests. Quedan para otra spec `recover`, `RecoveryAvailable`, bloqueos y versiones.

## Objetivo

Dar a cada proyecto la identidad de D-63 (un UUID guardado en el documento) y completar las salidas de guardado de 02 §5.4 línea 1358 y §7.2: «Guardar como», «Guardar copia» y la generación `.gafira.bak` del paso 4 del guardado atómico.

## Alcance / no alcance

Alcance:

- `gafira-doc`: tipo `ProjectId` (128 bits, texto `8-4-4-4-12` en minúsculas) y campo `project_id` del `Document`, siempre escrito (ADR 0006, regla 3); un documento anterior al campo se lee con el id nulo.
- `gafira-ports`: puerto `IdSource` (el engine no genera azar, ARQ-02).
- `gafira-engine`:
  - `SystemPorts { filesystem, clock, ids }` agrupa los puertos para que `Ports::new` no pase de siete argumentos.
  - La sesión toma el id del documento, o le asigna uno nuevo si es nulo (proyecto nuevo o archivo anterior al campo).
  - Las ranuras de recuperación pasan a `recovery/<project_id>/` (sustituye la clave provisional de P13).
  - `save_as(path)` y `save_copy(path)`.
  - `.gafira.bak` en cada guardado sobre un archivo existente.

No alcance (la segunda parte de E4, en otra spec):

- `recover`, `RecoveryAvailable`, los bloqueos `.gafira.lock` y el checksum del journal.
- `save_version`, `list_versions`, `restore_version` y `SaveState`.
- El adaptador real de `IdSource` (UUID v4 con `getrandom`, en `gafira-platform`) y el de `ProjectRepository` (`gafira-format`, spec 008).

## Dependencias

- E3c/E3d del change `gafira-engine` (en `develop`): `save` atómico, autoguardado por ranuras.
- D-63 (UUID del proyecto; «Guardar copia» asigna uno nuevo y «Guardar como» conserva el del proyecto).
- `FileSystem` no tiene `copy` (design, tabla de huecos, fila `.gafira.bak`): la copia de seguridad se escribe con `open`, `create`, `sync_all` y `replace`, sin cambiar el puerto.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S12-01 | `ProjectId` tiene una forma de texto única. | Se escribe en minúsculas `8-4-4-4-12`; se lee también en mayúsculas; cualquier otro texto se rechaza; el nulo es el valor por defecto. |
| S12-02 | El documento guarda su identidad (ADR 0006). | `project_id` se escribe siempre, después de `next_id`; un JSON sin el campo carga con el nulo; un valor mal formado no carga. |
| S12-03 | Cada sesión tiene identidad. | `new_document` asigna un id nuevo del `IdSource`; `open` conserva el id del archivo y asigna uno nuevo solo si es nulo; el documento de la sesión lleva ese id y no queda modificado por ello. |
| S12-04 | La recuperación usa el id del proyecto (D-63). | Las ranuras se escriben en `recovery/<project_id>/slot-<n>.gafira-journal`; reabrir el mismo archivo usa la misma carpeta. Un resultado de autoguardado de una sesión ya reemplazada se ignora aunque sea del mismo proyecto. |
| S12-05 | `save_as` escribe en otra ruta y la adopta. | Con el mismo orden atómico que `save` (temporal en la carpeta destino, `replace` solo tras `Ok`), deja el documento en la ruta nueva, la sesión pasa a esa ruta, deja de ser de solo lectura, queda limpia y conserva el id. Es la salida de una sesión de solo lectura y de un documento nuevo (ARQ-55). |
| S12-06 | `save_copy` escribe una copia con identidad nueva y no toca la sesión. | La copia lleva un id nuevo del `IdSource`; la sesión conserva ruta, id, estado de solo lectura y estado modificado. |
| S12-07 | Un guardado sobre un archivo existente deja una generación `.gafira.bak` (02 §7.2 paso 4). | Tras `save`, `save_as` o `save_copy` sobre un archivo que existía, `<ruta>.bak` tiene los bytes anteriores y la ruta los nuevos; sin archivo previo no hay `.bak`. La copia se escribe en un temporal propio, se sincroniza y se mueve con `replace`, siempre antes del `replace` del archivo principal. |
| S12-08 | Ningún fallo deja el archivo a medias (ARQ-50). | Un fallo en cualquier operación del guardado (incluidas las del `.bak`) deja la ruta con los bytes anteriores o los nuevos completos, devuelve el primer error, borra los temporales y la sesión no cambia. |
| S12-09 | Las salidas se rechazan antes de tocar un puerto. | Sin sesión, `save_as` y `save_copy` devuelven `NoSession` sin llamadas al repositorio ni al sistema de archivos. |

## TDD

Lógica crítica: S12-01, S12-02 (forma de cable), S12-05..S12-08 (orden de los puertos, atomicidad, estado de la sesión). Tests que fallan primero en `gafira-doc` (`project_id::tests`, `json::tests`) y `gafira-engine` (`project::tests`, `autosave::tests`).

## Notas de diseño

- El id nulo como «sin identidad» evita un `Option` que la regla 3 de ADR 0006 no admite: el campo se escribe siempre y el engine nunca guarda un documento con id nulo.
- El `.bak` es parte del guardado: si no se puede escribir, el guardado falla y el archivo del usuario queda intacto (preferimos no guardar a guardar sin copia).
- `save_copy` no limpia el estado modificado: el archivo de la sesión no cambió.
