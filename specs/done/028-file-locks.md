# 028 — Bloqueos de archivo y ranuras de otra instancia

> **Estado:** done — 2026-11-16 — sigue a la spec 016; S28-01..S28-07 cubiertos por `gafira-engine` (`project::tests` 8 nuevos, `lock::tests` 1; trazas de `save_as` ajustadas al bloqueo) y `gafira-platform` (`processes::tests` 4); 4 mutantes a mano muertos; 2572 tests. El bloqueo del SO sobre el `.lock` y «abrir una copia» quedan fuera.

## Objetivo

Que dos instancias de Gafira en el mismo equipo no se pisen (02 §7.2 «Bloqueos de archivo» y «Recuperación»): una instancia viva no ve las ranuras de recuperación de otra instancia viva como recuperables, y un `.gaf` abierto en otra instancia se abre en solo lectura (ARQ-55, D-76) en lugar de que ambas lo sobrescriban.

## Alcance / no alcance

Alcance:

- `gafira-ports::system`: `ProcessId` y el puerto `ProcessProbe` (`current`, `host`, `is_alive`).
- `gafira-platform`: `SystemProcesses`, el adaptador real, solo con `std`.
- `gafira-engine`:
  - `SystemPorts` agrupa también `processes`, así `Ports::new` sigue en seis argumentos.
  - El marcador `recovery/<project_id>/latest` registra el PID de la instancia dueña; `recoverable()` omite las carpetas cuyo dueño vive y no es este proceso.
  - Abrir un `.gaf` (y recuperar uno, y «Guardar como») escribe `<archivo>.gaf.lock` con `{pid, host, inicio}`, de forma atómica a través de `FileSystem`; `close`, `new_document` y abrir otro archivo lo borran.
  - Un bloqueo de otro proceso vivo, o de otro equipo, abre el archivo en solo lectura y `ProjectService::locked_by_other()` lo nombra; un bloqueo muerto (PID que no vive) o ilegible se toma.
- `gafira-ui` y `apps/gafira`: solo la construcción de `SystemPorts`.

No alcance:

- El bloqueo consultivo del SO sobre el `.gaf.lock` (`fs4` en 02 §7.2) y el campo `usuario`: el bloqueo es solo el archivo auxiliar.
- Ofrecer «abrir una copia» desde la UI cuando el archivo está bloqueado: la sesión queda en solo lectura y «Guardar» devuelve el error de solo lectura que la UI ya muestra; «Guardar como» es la salida (ARQ-55).
- Unidades de red y almacenamiento en la nube (02 §7.2).

## Dependencias

- Spec 016 (marcador, `recoverable`, `close`) y 012 (`SystemPorts`, carpeta por id).
- ARQ-55 / D-76: la sesión de solo lectura existente.
- ARQ-02: el engine no lee PID, nombre de equipo ni procesos; los pide al puerto.
- Toolchain 1.93.1: `std::fs::File::try_lock` (estable desde 1.89) para el adaptador fuera de Linux.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S28-01 | El marcador nombra a su dueño. | `marker_text`/`parse_marker` hacen ida y vuelta de ranura, PID y archivo; un PID no numérico o ausente se rechaza. El autoguardado escribe el PID de `ProcessProbe::current`. |
| S28-02 | Las ranuras de otra instancia viva no se ofrecen. | Con el dueño vivo y distinto de este proceso, `recoverable()` no lista el proyecto ni publica `RecoveryAvailable`. |
| S28-03 | Las ranuras de un dueño muerto se ofrecen. | Con el dueño muerto, el proyecto se lista y se recupera como en S16-03. |
| S28-04 | Abrir escribe el bloqueo y cerrar lo borra. | Tras `open`, `<archivo>.lock` tiene `pid`, `host` e inicio en tres líneas, escrito por un temporal y `replace`; tras `close`, `new_document` u abrir otro archivo ya no existe. |
| S28-05 | Un bloqueo vivo ajeno abre en solo lectura. | Con un bloqueo de otro PID vivo del mismo equipo, o de otro equipo, `open` carga el documento, `save` devuelve `ReadOnly`, `locked_by_other()` da el dueño y el bloqueo no se toca, tampoco al cerrar. |
| S28-06 | Un bloqueo muerto se toma. | Con un bloqueo de un PID muerto del mismo equipo (o ilegible), `open` abre con escritura, `locked_by_other()` es `None` y el bloqueo pasa a este proceso. |
| S28-07 | El adaptador dice quién es y quién vive. | `SystemProcesses::current` es `std::process::id()`, el propio PID vive; en Linux un PID sin `/proc/<pid>` no vive; fuera de Linux una baliza bloqueada con `try_lock` vive y una libre o ausente no. |

## TDD

Lógica crítica (invariantes de 02 §7.2): S28-01..S28-06 en `gafira-engine` (`autosave::tests`, `project::tests`) con los dobles en memoria y un `FakeProcesses`; deben fallar primero. S28-07 con tests del adaptador en `gafira-platform`.

## Evidencia esperada

Tests nuevos en `gafira-engine` y `gafira-platform`; gates de `AGENTS.md` («Before calling something done») en verde.

## Evidencia

- Rojo verificado: `s28_02` falló con la ranura de la instancia viva listada antes de filtrar por dueño; los cinco tests de bloqueo (`s28_04` x3, `s28_05`, `s28_06`) fallaron con `locked_by_other` vacío y sin `.lock` antes de escribirlo.
- Mutantes a mano, todos muertos: sin la comparación de `host` (muere `s28_05`), sin soltar el bloqueo anterior al reemplazar la sesión (muere `s28_04_a_new_document_or_another_file_releases_the_lock`), sin `pid != current` en `recoverable` (muere `s16_03`), sin `is_alive` (muere `s28_03`).
- `cargo fmt --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 2572 tests, 0 fallos.
- `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` y `cargo test -p gafira-testkit --features ports --locked`: limpios.
- `python3 tools/sync_agents.py --check`: 0.

## Decisiones

- `locked_by_other() -> Option<FileLock>` en lugar de una variante de error: abrir no falla, la sesión queda en solo lectura (ARQ-55) y la UI puede leer el dueño si lo necesita; hoy no muestra nada nuevo.
- El marcador pasa a tres líneas (`ranura`, `pid`, `archivo`); un marcador de dos líneas de la spec 016 ya no se lee y sus ranuras quedan en disco sin ofrecerse (sin datos publicados que migrar).
- Escribir el bloqueo es de mejor esfuerzo: si falla, el archivo se abre igual y no hay bloqueo que borrar. «Guardar como» mueve el bloqueo al archivo nuevo.
- `SystemProcesses::new(<datos_app>/instances)`: en Linux no crea nada; fuera de Linux crea la baliza y la borra al soltarse.

## Riesgos y decisiones abiertas

- Sin `unsafe` ni dependencias nuevas no hay forma de preguntar por un PID en Windows o macOS. Fuera de Linux cada instancia mantiene una baliza `<datos_app>/instances/<pid>` bloqueada con `File::try_lock` mientras vive: otra instancia la ve ocupada (viva) o libre (muerta, el SO suelta el bloqueo al morir el proceso). Si no se puede comprobar (error de E/S distinto de «ocupado»), se supone viva: nada se pierde, a lo sumo se abre en solo lectura o no se ofrece una recuperación.
- Un bloqueo de otro equipo nunca se toma: no se puede saber si su proceso vive. La salida es «Guardar como» o borrar el `.lock` a mano.
- El nombre del equipo sale de `/proc/sys/kernel/hostname`, `COMPUTERNAME` o `HOSTNAME`; si no hay ninguno es vacío, y dos equipos sin nombre se tratan como el mismo.
- Sin bloqueo del SO sobre el `.lock`, dos instancias que abren el mismo archivo en el mismo instante pueden tomarlo las dos.
