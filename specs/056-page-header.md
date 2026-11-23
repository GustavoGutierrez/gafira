# 056 — Cabecera de página: título, orden, ocultar, bloquear, duplicar, eliminar y añadir

> **Estado:** in-progress — 2026-11-19 — pedida por el titular con una captura de referencia (FD-02).

## Objetivo

Que cada página tenga encima una cabecera (FD-02, 03 §7.3) con «Página N - Agregar título de página», el título editable en el sitio, y a la derecha Subir, Bajar, Ocultar, Bloquear, Duplicar, Eliminar y Añadir página.

## Alcance / no alcance

Alcance:

- `gafira-edit`: `ReorderPage` (una página a otro índice) y `DuplicatePage` (una copia justo después, con id de página e ids de nodo nuevos, mismo contenido), cada uno un paso de deshacer.
- `gafira-ui`:
  - cabecera sobre cada página: «Página N» y, tras un guion, el título o «Agregar título de página» atenuado y subrayado con puntos; un clic lo edita en el sitio (Enter o un clic fuera guarda, Esc cancela) con `Property::PageName`;
  - a la derecha, con iconos Phosphor y su nombre accesible: Subir (deshabilitado en la primera), Bajar (deshabilitado en la última), Ocultar/Mostrar (`PageHidden`), Bloquear/Desbloquear (`PageLocked`), Duplicar, Eliminar (deshabilitado con una sola página) y Añadir página (una página vacía del mismo tamaño justo después);
  - una página oculta se titula «Página oculta» y no entra en «Todas» ni en un rango al exportar (FD-02 b);
  - una página bloqueada trata a todos sus elementos como bloqueados (D-91): no se eligen ni se mueven en el lienzo.

No alcance: reemplazar el contenido de una imagen en una página bloqueada (no existe todavía), la rejilla de Páginas, Notas, arrastrar páginas.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S56-01 | `ReorderPage`. | Mueve la página al índice pedido; un índice fuera de rango o una página que no existe es error; deshacer la devuelve. |
| S56-02 | `DuplicatePage`. | La copia queda justo después, con id de página e ids de nodo nuevos y sin colisiones (ARQ-13), mismo contenido, árbol y guías; el documento valida; deshacer la quita y devuelve el contador de ids. |
| S56-03 | Cabecera. | Título editable en el sitio; Subir y Bajar deshabilitados en los extremos; Eliminar deshabilitado con una página; cada acción es un paso de deshacer. |
| S56-04 | Oculta. | Se titula «Página oculta» y se excluye de «Todas» y de un rango al exportar. |
| S56-05 | Bloqueada. | Sus elementos no se eligen con clic, marco ni Ctrl+A, ni se mueven. |

## TDD

1. `s56_01_reorder_page` y `s56_02_duplicate_page` (`gafira-edit`).
2. `s56_04_hidden_pages_are_not_exported` y `s56_05_a_locked_page_locks_its_elements` (`gafira-ui`).

## Evidencia esperada

Tests en verde, gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- «Permitir solo reemplazar contenido» se aplica como página bloqueada (FD-02): como no existe aún reemplazar contenido, nada de la página se edita en el lienzo.
