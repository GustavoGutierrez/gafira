# 031 — Versiones dentro del `.gaf`

> **Estado:** in-progress — 2026-11-16 — tercera parte de E4 del change `gafira-engine` ([carpeta legacy](done/legacy/changes/gafira-engine/design.md)), tras las specs 012, 016 y 028.

## Objetivo

Que el proyecto guarde instantáneas de sí mismo dentro del propio archivo (02 §7.1 «Versiones», D-67, D-68): versiones nombradas que crea la persona con Archivo → Guardar versión y versiones automáticas al guardar, que se listan en «Versiones» y se restauran con un `Command` que se puede deshacer (03 §2.2).

## Alcance / no alcance

Alcance:

- `gafira-ports::repository`: `VersionId`, `VersionKind` (`Named`, `Auto`), `VersionInfo` (`id`, `name`, `kind`, `created`, `app`, `schema`) y `Version` (la información y el `Arc<Document>` de ese momento). `LoadedProject` gana `versions` y `ProjectRepository::save` recibe las versiones que el archivo debe conservar.
- `gafira-format`: `versions/index.json` y `versions/<id>/document.json`, `pages/*.json` y `sequences/*.json`; el manifiesto lista los ids en `versions`. Los recursos embebidos de una versión se escriben una sola vez en `assets/`, compartidos por hash con el documento. Al leer, las mismas comprobaciones de archivo hostil que el resto (nombres, tamaños, proporción, ids coherentes).
- `gafira-doc`: `DocMut::raise_next_id`, para que restaurar no reutilice ids.
- `gafira-edit`: el comando `RestoreVersion`, que sustituye el contenido del documento (páginas, secuencias, recursos, dpi) por el de una versión, conserva la identidad del proyecto y se deshace.
- `gafira-engine`: `ProjectService::{save_version(name), list_versions(), restore_version(id)}` (02 §5.4); la regla de la versión automática en `save` y `save_as`; las versiones viajan con la sesión, con «Guardar copia», con el autoguardado y con la recuperación.
- `gafira-ui`: «Versiones» en la barra superior del editor abre la lista (nombre, fecha, «Restaurar»); Archivo → «Guardar versión…» pide un nombre.

No alcance:

- Borrar o renombrar una versión a mano, y comparar dos versiones.
- El recolector de recursos (02 §7.1): todavía no existe; cuando llegue, no debe borrar un recurso que use una versión.
- Guardado incremental (02 §7.2): cada guardado reescribe todas las versiones.
- Las cifras de la versión automática en Ajustes → Guardado (03): quedan fijas en el código.
- `masks/` y `thumbs/` de una versión.

## Dependencias

- Specs 012 (guardado atómico, `.bak`), 016 (recuperación), 019 y 026 (`.gaf` y recursos embebidos), 028 (bloqueos).
- D-67 (versiones dentro del `.gaf`), D-68 (disposición), D-71 (30 min y 20 versiones automáticas, confirmadas).
- ARQ-02: la hora sale del puerto `Clock`.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S31-01 | El puerto lleva las versiones. | `LoadedProject.versions` y `save(path, document, versions)`; un doble en memoria hace ida y vuelta. |
| S31-02 | El `.gaf` guarda las versiones. | `versions/index.json` con `id`, `name`, `kind`, `created`, `app`, `schema` (y los ids de páginas y secuencias), una carpeta por versión con su `document.json`, `pages/` y `sequences/`; el manifiesto las lista; leer devuelve las mismas versiones; dos escrituras dan los mismos bytes. Sin versiones el archivo no cambia respecto a la spec 019. |
| S31-03 | Los recursos se comparten. | Un recurso embebido que usan el documento y una versión se escribe una vez; uno que solo usa una versión también se escribe y vuelve al almacén al abrir. |
| S31-04 | Una versión hostil es corrupta. | Índice que no coincide con el manifiesto, id repetido, `kind` desconocido, nombre de más de 256 caracteres, `schema` distinto del documento de la versión o más nuevo que el soportado, página que guarda otro id o recurso embebido de una versión que falta: `Corrupt`. |
| S31-05 | La versión automática sigue la regla. | Al guardar se crea una automática si no hay ninguna o si la última tiene más de 30 min; dentro de los 30 min no; se conservan las 20 automáticas más recientes y ninguna nombrada se borra. «Guardar copia» y el autoguardado no crean versiones. |
| S31-06 | «Guardar versión» guarda una nombrada. | `save_version(name)` añade una versión `Named` con ese nombre, la hora del reloj, la versión de la app y el schema, y escribe el archivo; la sesión queda limpia. Sin sesión, solo lectura o sin archivo se rechaza antes de llamar a un puerto. |
| S31-07 | Las versiones viajan con el proyecto. | Abrir lista las versiones del archivo; sobreviven a guardar, «Guardar como», «Guardar copia», y a recuperar desde una ranura. |
| S31-08 | Restaurar es un comando reversible. | `RestoreVersion` deja páginas, secuencias, recursos y dpi de la versión, conserva el `project_id`, no baja el contador de ids; deshacer y rehacer funcionan; con el mismo contenido es `Unchanged`. |
| S31-09 | El engine restaura. | `restore_version(id)` publica un `DocumentChanged`, deja la sesión modificada y se deshace con `edit().undo()`; un id desconocido es `NoVersion`. |
| S31-10 | La UI ofrece las versiones. | «Versiones» abre la lista con nombre y fecha y «Restaurar»; Archivo → «Guardar versión…» pide el nombre; los textos nuevos existen en es y en. |

## TDD

Lógica crítica (formato, comandos, regla con invariante): S31-02..S31-09 con tests que fallan primero en `gafira-format` (`tests/versions.rs`), `gafira-edit` (`restore.rs`) y `gafira-engine` (`project::tests`, `versions::tests`) con los dobles en memoria y el reloj manual. S31-10 con el test de claves de `gafira-ui`.

## Evidencia esperada

Tests nuevos en `gafira-ports`, `gafira-format`, `gafira-doc`, `gafira-edit`, `gafira-engine` y `gafira-ui`; gates de `AGENTS.md` («Before calling something done») en verde.

## Riesgos y decisiones abiertas

- Todas las versiones viven en memoria con la sesión y se reescriben en cada guardado: es lo más simple que mantiene el guardado atómico (temporal, `FileSystem::replace`, `.bak`) sin que el adaptador lea el archivo anterior. Un proyecto con muchas versiones grandes guarda más lento; el guardado incremental lo resolverá.
