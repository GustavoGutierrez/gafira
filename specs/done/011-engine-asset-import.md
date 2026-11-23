# 011 — `AssetService::import` del engine (recursos embebidos)

> **Estado:** done — 2026-11-16 — E6 del change `gafira-engine` ([carpeta legacy](legacy/changes/gafira-engine/design.md)), parte del MVP; S11-01..S11-06 cubiertos por `gafira-edit` `assets::tests` (4) y `gafira-engine` `assets::tests` (5); 4 mutantes a mano muertos (uno, tras añadir el test directo de `revert`); 2667 tests. Medios vinculados, `relocate` y `LinksMissing` siguen en Fase 2.

## Objetivo

Que el engine importe un archivo como recurso **embebido** del documento abierto (02 §5.4 línea 1360, §3.1, §7.1): los bytes van al `ContentStore`, la tabla de recursos gana una entrada con el hash como identidad, y la importación se deshace como cualquier edición (ARQ-30).

## Alcance / no alcance

Alcance:

- `gafira-edit`: comando `InsertAsset` (añade una entrada a `AssetTable`, se deshace; si el hash ya está, no cambia nada).
- `gafira-engine`: `AssetService::import(path) -> AssetRef`, `Gafira::assets()`.

No alcance (Fase 2, 02 §7.1 línea 1554 y §11):

- Medios vinculados (`AssetStorage::Linked`), `AssetService::relocate` y `EngineEvent::LinksMissing`. El modelo de `gafira-doc` todavía no guarda `rel_path`, `bytes` ni `blake3` del vínculo; se añaden con esa fase.
- Decodificar, validar el formato o extraer metadatos del archivo (importadores de capa 4); qué hace cada formato con las fuentes incrustables (D-60, spec 008).
- Insertar el recurso en una página o en la línea de tiempo: eso lo hacen otros comandos que citan el `AssetRef`.

## Dependencias

- E3d del change `gafira-engine` (en `develop`): sesión, `History`, versión y `DocumentChanged`.
- Puertos `FileSystem` y `ContentStore` (ya en `Ports`). ADR 0016: el engine no abre archivos por su cuenta.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S11-01 | `InsertAsset` añade una entrada y se deshace. | `apply` sobre un hash nuevo da `Changed` y la entrada; `revert` la quita; `apply` sobre un hash que ya está da `Unchanged` y conserva la entrada existente; `revert` sin `apply` da `NotApplied`. |
| S11-02 | `import` guarda los bytes y registra el recurso embebido. | Con una sesión abierta, `import("/in/photo.png")` deja en el store los bytes del archivo, devuelve el `AssetRef` de su hash, añade `AssetEntry { name: "photo.png", storage: Embedded }` y publica un `DocumentChanged`. |
| S11-03 | Importar el mismo contenido dos veces no duplica (identidad por hash, 02 §3.1). | La segunda importación devuelve el mismo `AssetRef`, deja una sola entrada y no publica nada. |
| S11-04 | La importación se deshace (ARQ-30). | `undo` quita la entrada y publica `DocumentChanged`; `redo` la vuelve a poner. |
| S11-05 | Sin sesión no se toca nada. | `import` sin documento abierto devuelve `EngineError::NoSession` sin abrir el archivo ni escribir en el store. |
| S11-06 | Un fallo de un puerto no cambia el documento. | Un archivo que no existe da `EngineError::Io`; un store que falla da `EngineError::Store`; en los dos casos no hay entrada nueva ni evento. |

## TDD

Lógica crítica: S11-01 (comando sobre el documento) y S11-02..S11-06 (identidad por hash, orden de los puertos, deshacer). Tests que fallan primero:

- `gafira-edit`: `assets::tests::s11_01_*`.
- `gafira-engine`: `assets::tests::s11_02_*` a `s11_06_*`, con un `MemoryStore` falso.

## Evidencia esperada

- Gates de `AGENTS.md` en verde; mutantes a mano en la deduplicación y en el orden sesión → archivo → store.
- Banner a `done` y fila en `specs/README.md`.

## Riesgos y decisiones abiertas

- El store puede quedarse con bytes huérfanos si la sesión se cierra entre `put` y el comando; es inocuo porque el store es por contenido, y la limpieza llega con el recolector de la biblioteca.
- `relocate` y `LinksMissing` necesitan ampliar `AssetStorage::Linked` en `gafira-doc` (Fase 2).
