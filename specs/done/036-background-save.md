# 036 — Guardar en segundo plano

> **Estado:** done — 2026-11-17 — S36-01..S36-07 cubiertos por `gafira-engine` (`project::tests` s36, 5) y `gafira-ui` (`saving::tests`, 2); 1 mutante a mano muerto (15 tests); 2971 tests.

## Objetivo

Guardar nunca congela la ventana. Desde la spec 033 cada guardado dibuja la miniatura de la primera página y escribe el `.gaf` con su `.bak`, y todo eso ocurría en el hilo de la ventana. Ahora el guardado corre en su propio hilo, igual que la exportación de la spec 035.

## Alcance / no alcance

Alcance:

- `gafira-engine`:
  - `ProjectService::prepare_save(SaveRequest)` hace las comprobaciones y elige las versiones que guarda el archivo, sin tocar puertos. Devuelve un `SaveJob` que es `Send + 'static` y contiene la instantánea, las versiones y los puertos que escriben.
  - `SaveJob::run()` escribe de forma atómica, con la miniatura y el `.bak`, y devuelve un `SaveOutcome`.
  - `ProjectService::save_finished(outcome)` lo cuenta: la sesión queda limpia en la versión guardada y, en «Guardar como», adopta el archivo y le pasa el bloqueo. Si la sesión fue reemplazada mientras tanto, no la toca.
  - `SaveRequest`:
    - `Save`;
    - `SaveAs(path)`;
    - `SaveCopy(path)`;
    - `Version { name, at }`. Con `at`, es el «Guardar como» de un proyecto nunca guardado, con la versión con nombre y sin la automática.
  - `save`, `save_as`, `save_copy` y `save_version` son `prepare_save`, `run` y `save_finished` en el mismo hilo. Las reglas de las specs 012, 028, 031 y 033 no cambian.
  - El engine no crea hilos (ARQ-02).
- `gafira-ui`:
  - «Guardar», «Guardar como», «Guardar copia» y «Guardar versión…» muestran primero, si hace falta, el diálogo del sistema. Después el guardado va a un hilo `gafira-save`.
  - Mientras corre, una tarjeta no modal muestra «Guardando…» y la ventana sigue respondiendo.
  - Un segundo guardado mientras tanto avisa «Ya se está guardando el proyecto».
  - «¿Guardar los cambios?» → «Guardar» sigue a su destino (Inicio, Abrir, un reciente) solo cuando el guardado salió bien.
  - Un hilo que termina sin respuesta es un guardado fallido, nunca el cierre de la ventana.
  - Al cerrar la ventana se espera al guardado, que nunca se corta, y el engine lo cuenta.

No alcance: abrir un proyecto en segundo plano; cancelar un guardado; el diálogo del sistema no bloqueante.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S36-01 | Preparar no escribe. | `prepare_save` no llama a ningún puerto; rechaza como `save` (solo lectura); el trabajo es `Send + 'static`; contado, la sesión queda limpia. |
| S36-02 | Instantánea. | El archivo tiene el documento de `prepare_save`; una edición hecha mientras corre deja la sesión modificada. |
| S36-03 | Fallo. | `save_finished` devuelve el error, la sesión sigue modificada en su archivo y el archivo conserva sus bytes. |
| S36-04 | Versión de un proyecto nuevo. | El archivo elegido pasa a ser el de la sesión, con una única versión con nombre. |
| S36-05 | Copia. | Escribe la copia y deja la sesión como estaba. |
| S36-06 | Fuera del hilo de la ventana. | El hilo escribe el `.gaf` real; la ventana se despierta desde otro hilo; contado, la sesión vive en el archivo y está limpia. |
| S36-07 | Un pánico no cierra la app. | Un hilo que termina sin respuesta se lee como guardado perdido («No pudimos guardar»). |

## Evidencia

- `cargo test --workspace --locked`: 2971 tests.
- Mutante a mano: si `save_finished` no cuenta el guardado, fallan 15 tests.
