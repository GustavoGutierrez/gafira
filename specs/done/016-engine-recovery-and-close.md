# 016 — Recuperación tras fallo, `close` y la primera página del documento

> **Estado:** done — 2026-11-16 — segunda parte de E4 del change `gafira-engine` ([carpeta legacy](legacy/changes/gafira-engine/design.md)); S16-01..S16-05 cubiertos por `gafira-engine` (`project::tests` 5 nuevos, trazas del autoguardado ajustadas al marcador) y `gafira-ui` (tests de 018 ajustados); 2794 tests. Las versiones dentro del `.gaf` y los archivos de bloqueo pasan a sus propias specs.

## Objetivo

Que el trabajo sin guardar sobreviva a un cierre inesperado (02 §7.2 «Recuperación», 03 §7.9), que salir de un proyecto lo cierre de verdad, y que la primera página de un documento nuevo no se pueda deshacer.

## Alcance / no alcance

Alcance:

- `gafira-engine`:
  - `ProjectService::new_document_with(size)`: documento con su primera página, sin entrada en el historial.
  - Marcador `recovery/<project_id>/latest` (dos líneas: ranura más reciente y archivo del usuario), escrito de forma atómica después de cada ranura.
  - `recoverable()` lista los proyectos con ranuras que no son de la sesión abierta y publica `EngineEvent::RecoveryAvailable(Vec<RecoveryEntry>)`.
  - `recover(project_id)` carga la ranura más reciente que se lea, con vuelta a la anterior (02 §7.2), en la ruta del usuario y como modificada; `EngineError::NoRecovery` si no hay marcador.
  - `discard_recovery(project_id)` y `close()`; un guardado que deja todo en el archivo borra las ranuras.
- `gafira-ui`:
  - El autoguardado corre en un hilo (el engine programa, el host ejecuta, P13).
  - Al aparecer Inicio por primera vez se buscan recuperaciones y se muestra Recuperación tras fallo: Recuperar (coral), Abrir el último guardado (si hay archivo) y Descartar con confirmación.
  - «¿Guardar los cambios?» antes de salir del editor a Inicio o de abrir otro archivo con cambios: Guardar, No guardar, Cancelar.

No alcance:

- Archivos de bloqueo `nombre.gaf.lock` con PID y bloqueo del SO: hace falta un puerto que diga si un proceso vive. Hasta entonces, una segunda instancia abierta ve las ranuras de la primera como recuperables.
- Versiones dentro del `.gaf` (`save_version`, `list_versions`, `restore_version`): cambian el puerto `ProjectRepository`.
- Journal solo JSON y la cifra de 200 ms (O5 sigue en vigor: cada ranura es un `.gaf` completo).
- La hora de la última copia en el diálogo (03 §7.9).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S16-01 | La primera página es del documento. | `new_document_with` da una página del tamaño pedido, id de proyecto, sin cambios pendientes y sin nada que deshacer. |
| S16-02 | El marcador nombra la ranura y el archivo. | Ida y vuelta con y sin archivo; texto vacío, ranura fuera de 0..3 o no numérica se rechazan. |
| S16-03 | Tras un fallo se ofrece y se recupera el trabajo. | Otro engine sobre el mismo disco lista el proyecto con su archivo, publica `RecoveryAvailable`, y `recover` deja el documento editado, en su ruta y modificado; sus propias ranuras ya no se ofrecen. |
| S16-04 | Una ranura que no carga cede a la anterior. | Con la más reciente rota se recupera la anterior; con todas rotas, error del repositorio; proyecto sin marcador, `NoRecovery`. |
| S16-05 | Guardar, cerrar y descartar borran las ranuras. | Tras cada uno no quedan marcador ni ranuras y nada se ofrece. |
