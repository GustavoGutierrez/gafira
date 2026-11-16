# 026 — `gafira-assets`: almacén por contenido y recursos embebidos en el `.gaf`

> **Estado:** in-progress — 2026-11-16 — rama `feat/026-content-store`.

## Objetivo

Que los recursos importados tengan dónde vivir y viajen dentro del proyecto: un `ContentStore` real en disco (02 §2.1, §5.3) y la carpeta `assets/` del `.gaf` (02 §7.1, D-68), para que las imágenes puedan llegar después.

## Alcance / no alcance

Alcance:

- `crates/gafira-assets` (capa 4): `FsStore` implementa `gafira_ports::store::ContentStore` con archivos direccionados por contenido bajo una carpeta raíz, `<raíz>/<2 primeros hex>/<hash hex>`, y hash BLAKE3 como `ContentHash`. `put` lee por bloques, calcula el hash, escribe un temporal, lo sincroniza y lo renombra; si el contenido ya está, no lo duplica. `open` y `contains`. Sin dependencia de `gafira-engine` (ARQ-06).
- `crates/gafira-format`:
  - Guardar escribe cada recurso `Embedded` de la tabla del documento en `assets/<blake3-hex>.<ext>`, con los bytes que da el `ContentStore` del repositorio (`GafRepository::new(limits, store)`). La extensión sale del nombre de la entrada; STORE si el formato ya va comprimido (jpg, png, webp, mp4…), DEFLATE si no. El manifiesto `assets` sigue con el hash como clave (D-68) y suma `bytes` en los embebidos.
  - Abrir comprueba que el BLAKE3 de cada entrada `assets/` es el de su nombre antes de meterla en el almacén; si no, es `Corrupt`. Un recurso embebido de la tabla sin entrada en el archivo es `Corrupt`. Siguen las reglas de archivo hostil (nombres, tamaños, relación de compresión), con un tope nuevo de tamaño por recurso.
  - La escritura sigue siendo determinista.
- `apps/gafira`: compone `FsStore` en `<datos_app>/store` en lugar del marcador `StorePending` y se lo pasa a `GafRepository`.
- CI: un paso que comprueba ARQ-06 en los adaptadores de capa 4 sin fila V5 (`gafira-assets`, `gafira-format`, `gafira-platform`).

No alcance:

- Insertar imágenes en el lienzo (spec siguiente).
- Recolector de recursos, índice de biblioteca, miniaturas, plantillas y `AssetProvider` de 02 §2.1.
- `mime` en el manifiesto, medios vinculados (Fase 2), `masks/`, `thumbs/`, `versions/`.

## Dependencias

- `gafira-ports` (`ContentStore`, `StoreError`, `RepositoryError`), `gafira-doc` (`ContentHash`, `AssetRef::hex`, `AssetTable`).
- Spec 019 (`gafira-format`) y 011 (`AssetService::import`, que ya escribe en el almacén).
- `blake3` 1 (ya en el workspace por `gafira-render`; `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`, aceptada por `deny.toml`).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S26-01 | `put` devuelve el BLAKE3 del contenido y lo guarda en `<raíz>/<2 hex>/<hex>`. | El hash coincide con `blake3::hash`; el archivo existe en esa ruta con esos bytes. |
| S26-02 | `open` y `contains` encuentran lo guardado. | `open` devuelve los mismos bytes; un hash desconocido es `NotFound` y `contains` da `false`. |
| S26-03 | Mismo contenido, un solo archivo. | Dos `put` iguales dan el mismo hash y un solo archivo. |
| S26-04 | Un `put` que falla no deja nada. | Un lector que falla a mitad da `Io` y no queda ningún archivo en la raíz. |
| S26-05 | Guardar escribe los recursos embebidos. | `assets/<hex>.<ext>` con los bytes del almacén, STORE para `mp4`, DEFLATE para `wav`; los vinculados no tienen entrada; el manifiesto lleva `bytes`. |
| S26-06 | Ida y vuelta con un recurso embebido. | Abrir en un almacén vacío devuelve el mismo documento y los mismos bytes en el almacén; dos escrituras dan los mismos bytes. |
| S26-07 | Bytes de un recurso cambiados son `Corrupt`. | Y no entran en el almacén. |
| S26-08 | Un recurso embebido que falta es `Corrupt`. | Sin su entrada `assets/`. |
| S26-09 | Nombres de `assets/` malformados son `Corrupt`. | Hash corto o en mayúsculas, sin extensión. |
| S26-10 | Guardar sin los bytes en el almacén falla. | `Io`, sin escribir un archivo incompleto como si fuera bueno. |

## TDD

Casos que deben fallar primero:

1. `gafira-assets`: S26-01..S26-04 en `tests/store.rs`.
2. `gafira-format`: S26-05..S26-10 en `tests/assets.rs`.

## Evidencia esperada

Tests nuevos en verde, `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `python3 tools/sync_agents.py --check`, `cargo deny check licenses bans sources` y los pasos de política de `ci.yml`.

## Riesgos y decisiones abiertas

- La relación de compresión de bomba ZIP (más de 200x en una entrada de más de 1 MiB) también se aplica a `assets/`: un BMP o un WAV de silencio muy grande podría rechazarse. Se mantiene la regla; si aparece un caso real se revisa con el titular.
- Abrir lee dos veces cada recurso (hash y copia al almacén) para no meter en el almacén bytes que no coinciden; cuesta tiempo en archivos grandes.
- Un `assets/` que no está en la tabla se verifica y se guarda igual, porque las versiones futuras (`versions/`) compartirán recursos por hash.
