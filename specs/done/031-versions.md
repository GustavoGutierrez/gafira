# 031 — Versiones dentro del `.gaf`

> **Estado:** done — 2026-11-16 — tercera parte de E4 del change `gafira-engine` ([carpeta legacy](legacy/changes/gafira-engine/design.md)), tras las specs 012, 016 y 028; S31-01..S31-10 cubiertos por `gafira-ports`, `gafira-format`, `gafira-doc`, `gafira-edit`, `gafira-engine` y `gafira-ui` (33 tests nuevos); 5 mutantes a mano muertos; 2882 tests. Borrar o renombrar versiones y el guardado incremental quedan fuera.

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

## Evidencia

- Rojo verificado: los 8 tests de `tests/versions.rs` que escriben o leen versiones fallaron con `write_gaf` aceptando las versiones sin escribirlas; `raise_next_id` no existía; con `apply` de `RestoreVersion` devolviendo `Unchanged` fallaron los 3 tests que esperan el cambio; con `on_save` devolviendo las versiones sin tocar fallaron los 5 de `versions::tests` del engine.
- El test de propiedades de V14 (`restore_version_obeys_the_command_laws`) mostró que deshacer tras un `apply` sin cambios debe ser un no-op, como en los demás comandos; corregido.
- Mutantes a mano, todos muertos: sin las versiones en la comprobación de recursos embebidos al leer (muere `s31_04_a_version_asset_missing_from_the_file_is_corrupt`), sin `restore_next_id` en `revert` (muere `s31_08_revert_puts_back_the_exact_document`), sin guardar en la sesión las versiones escritas (mueren los tests `s31_05`, `s31_06`, `s31_07` y `s31_09` del engine), un trabajo de autoguardado que no escribe las versiones (muere `s31_07_versions_travel_with_the_file_its_copies_and_its_recovery`), sin la comprobación de ids repetidos (muere `s31_04_a_repeated_version_id_is_corrupt`).
- 33 tests nuevos: `gafira-ports` 1, `gafira-format` 11 (`tests/versions.rs` 9, `versions::tests` 2), `gafira-doc` 1, `gafira-edit` 7 (`restore::tests` 6 y la propiedad de V14), `gafira-engine` 12 (`versions::tests` 5, `project::tests` 7), `gafira-ui` 1; las claves nuevas están en la lista de `text.rs`.
- `cargo fmt --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 2882 tests, 0 fallos (la primera pasada falló en ARQ-105: la flecha «→» de `versions.empty` no está en el subconjunto de Inter; el texto se cambió).
- `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` y `cargo test -p gafira-testkit --features ports --locked`: limpios.
- Pasos de política de `.github/workflows/ci.yml` (ARQ-07, `forbid(unsafe_code)`, V5, ARQ-06, V12, ARQ-107 x2, V13, V14, ADR 0013) en local: todos con salida 0.
- `python3 tools/sync_agents.py --check`: 0.
- La UI no se probó a mano en una ventana: S31-10 queda cubierto por el test de filas (`versions::tests`) y el de claves de `text.rs`.

## Decisiones

- **Puerto**: la sesión guarda las versiones en memoria (`Vec<Version>`, cada una con su `Arc<Document>`); `load` las devuelve en `LoadedProject.versions` y `save(path, document, versions)` escribe exactamente las que recibe. Es el cambio más pequeño que conserva el guardado atómico del engine (el repositorio escribe un temporal y lo sincroniza, el engine hace `.bak` y `FileSystem::replace`): el adaptador no necesita leer el archivo anterior para copiar sus versiones, y el engine decide cuáles se conservan.
- **Índice**: cada entrada de `versions/index.json` lleva además `pages` y `sequences` (los ids en orden), igual que el manifiesto para el documento: sin ellos el orden de las páginas de una versión no se puede reconstruir.
- **Ids**: un `u64` por versión, el mayor más uno; nunca se reutiliza porque solo se descartan las automáticas más antiguas. `created` son segundos Unix del puerto `Clock`; `app` es la versión del crate del engine (la del workspace).
- **Regla automática**: `save` y `save_as` añaden una versión automática del documento guardado si no hay ninguna automática o si la más reciente tiene más de 30 min (estrictamente más); después se descartan las automáticas más antiguas hasta dejar 20. Por eso el primer guardado de un proyecto crea su primera versión automática.
- **«Guardar versión»** escribe el archivo con la versión nueva (las versiones viven dentro del `.gaf`, D-67), con los mismos rechazos que `save`; no añade una automática. En la UI, un proyecto sin archivo pide primero «Guardar como».
- **«Guardar copia»** conserva las versiones tal cual, con el `project_id` nuevo solo en el documento: el lector no exige que una versión tenga el mismo `project_id`.
- **Recursos**: un recurso embebido se escribe una vez aunque lo usen el documento y varias versiones; si el documento ya no lo tiene, el nombre (y su extensión) sale de la primera versión que lo tiene.
- **`RestoreVersion`** sustituye páginas, secuencias, tabla de recursos y dpi; conserva `schema_version` y `project_id`; sube el contador de ids con `DocMut::raise_next_id` y lo baja al deshacer. Marca como tocadas todas las páginas y nodos de antes y de después. Restaurar no cambia la lista de versiones.

## Riesgos y decisiones abiertas

- Todas las versiones viven en memoria con la sesión y se reescriben en cada guardado: es lo más simple que mantiene el guardado atómico (temporal, `FileSystem::replace`, `.bak`) sin que el adaptador lea el archivo anterior. Un proyecto con muchas versiones grandes guarda más lento y ocupa más memoria; el guardado incremental (02 §7.2) y la carga perezosa de las versiones lo resolverán.
- Las ranuras de recuperación también llevan las versiones, así que cada autoguardado crece con ellas (O5 sigue en vigor).
- La lista de versiones muestra la fecha ISO (UTC) y el tiempo relativo; la hora local espera a la fecha localizada de la spec 015.
- No hay forma de borrar ni renombrar una versión nombrada; un proyecto con muchas nombradas solo crece.
