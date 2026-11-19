# 050 — Borde de la página activa

> **Estado:** done — 2026-11-19 — S50-01..S50-03 cubiertos por `gafira-ui` (`editor::tests` s50, 2); prueba del titular en la app; 3192 tests.

## Objetivo

Que se vea qué página está activa: un clic en una página la activa y la rodea con un borde; un clic fuera de toda página quita el borde (03 §7.3, FD-02).

## Alcance / no alcance

Alcance (`gafira-ui`): estado de página activa en el editor; un clic o el inicio de un arrastre sobre una página (en un elemento o en su fondo) la activa, y sobre el espacio de trabajo fuera de las páginas la desactiva; borde de 2 px en `canvas.selection` alrededor de la página activa; la página activa es la página actual (inserción de piezas, Capas, reglas, «n de N») cuando no hay selección.

No alcance: la cabecera de página con sus acciones (FD-02), activar con el teclado, el estado entre sesiones.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S50-01 | Qué activa un clic. | Sobre un elemento, su página; sobre el fondo de una página, esa página; fuera de toda página, ninguna. Tomar un tirador o una guía no cambia la página activa. |
| S50-02 | Borde. | La página activa se dibuja con un borde de 2 px por fuera, en `canvas.selection`; sin página activa, ninguna lo tiene. |
| S50-03 | Página actual. | La página de la selección manda; sin selección, la activa; sin las dos, la primera. Una página activa que ya no existe no cuenta. |

## TDD

1. `s50_01_active_after_a_click`: los casos de S50-01 con dos páginas apiladas.
2. `s50_03_current_page`: los casos de S50-03.

## Evidencia

- TDD: `active_after` y `current_page_id` se escribieron vacíos (`None`); los 2 tests fallaron por sus aserciones y pasaron con la implementación.
- `Editor.active`, puesto en cada clic o inicio de arrastre que no toma un tirador ni una guía; borde de 2 px `canvas.selection` por fuera de la página activa; `current_page` usa `current_page_id` (selección, activa, primera).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` y `cargo test --workspace --locked --no-fail-fast` (3192 passed) en verde; el titular lo probó en Linux.

## Riesgos y decisiones abiertas

Ninguna abierta.
