# 025 — Proyectos recientes en Inicio

> **Estado:** in-progress — 2026-11-16 — rama `feat/025-recent-projects`.

## Objetivo

Que «Recientes» de Inicio (03 §7.2, RF-01) muestre los proyectos que la persona abrió o guardó, del más nuevo al más antiguo, y que un clic en uno lo abra.

## Alcance / no alcance

Alcance:

- `gafira-platform`: `RecentProjects`, una lista JSON en `<data_dir>/recent.json` con hasta 12 entradas `{path, name, opened_at}` (segundos Unix), la más nueva primero y sin rutas repetidas. Escritura atómica como `JsonSettings`; un archivo ausente o ilegible se lee como lista vacía. Fuera del engine.
- `gafira-ui`: el puerto `RecentStore`, que el shell recibe en `UiConfig` desde la raíz de composición (`apps/gafira`). Se registra un proyecto tras Abrir, Guardar, Guardar como, «Abrir el último guardado» y Recuperar con archivo de origen; Guardar copia no cambia la lista.
- Inicio muestra hasta 4 tarjetas con el nombre y el tiempo relativo de los catálogos (`Translator::relative`); un clic abre el archivo, con «¿Guardar los cambios?» si hay una sesión modificada. Una ruta que ya no existe sale de la lista y no se muestra. Sin recientes sigue el texto vacío.

No alcance:

- Miniaturas (`thumbs/<page_id>.webp`, 02 §7.1): la tarjeta lleva un marcador del color `surface`.
- «Ver todos», la pestaña Proyectos y Archivo → Abrir reciente.
- Duplicar, renombrar, archivar o eliminar desde Inicio (resto de RF-01).

## Dependencias

- Specs 016 (recuperación y «¿Guardar los cambios?»), 017 (Inicio), 018 (`gafira-platform`), 019 (Abrir y Guardar).
- `gafira-i18n`: `Translator::relative` y las claves `time.*` (spec 015).
- 02 §2.1 (fila de `gafira-ui`): la UI no hace I/O directo; el archivo lo lee y escribe `gafira-platform` detrás del puerto.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S25-01 | Registrar. | La entrada nueva va primero; una ruta repetida sube al frente sin duplicarse; nunca hay más de 12. |
| S25-02 | Persistir. | Un archivo ausente o con basura se lee vacío; lo escrito se lee igual; quitar una ruta la borra; las rutas que ya no existen se podan y el archivo se reescribe. |
| S25-03 | Qué registra la ventana. | Abrir, Guardar y Guardar como registran la ruta del proyecto con su nombre sin `.gaf`; Guardar copia no. |
| S25-04 | Tarjetas. | Como mucho 4, en el orden de la lista; el tiempo relativo sale del catálogo y un reloj atrasado se muestra como «ahora mismo». |

## TDD

Casos que deben fallar primero (lógica con invariante: orden, tope y duplicados):

1. `s25_01_*` en `gafira-platform` (`recent::tests`): orden, duplicados y tope de 12.
2. `s25_02_*` en `gafira-platform`: ausente, basura, ida y vuelta, quitar y podar.

S25-03 y S25-04 son de interfaz y se validan con tests de `gafira-ui` sin ventana.

## Evidencia esperada

`cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` y `python3 tools/sync_agents.py --check` en verde; los tests no dejan archivos en la carpeta temporal.

## Riesgos y decisiones abiertas

- Una ruta que no es UTF-8 no se puede escribir en JSON; esa grabación falla, se registra en el log y la lista sigue como estaba.
- Sin miniaturas hasta que exista `thumbs/` en el `.gaf`.
