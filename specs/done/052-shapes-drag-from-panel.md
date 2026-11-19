# 052 — Formas: el nombre y arrastrar desde el panel

> **Estado:** done — 2026-11-19 — S52-01..S52-04 cubiertos por `gafira-ui` (`canvas` s52, 2; `editor::tests` s52, 1); D-94; prueba del titular en la app; 3192 tests.

## Objetivo

Que la pestaña «Piezas» se llame «Formas» (D-94) y que una forma se pueda arrastrar del panel al lienzo y soltarla donde se quiera, viendo mientras tanto la forma semitransparente bajo el puntero. Un clic sigue añadiéndola en el centro de la página actual.

## Alcance / no alcance

Alcance (`gafira-ui`, catálogos y glosario):

- La pestaña del riel pasa a «Formas» («Shapes») y el título de la sección del panel, a «Formas básicas» («Basic shapes»).
- Arrastrar el botón de una forma muestra una vista previa semitransparente del tamaño que tendrá en el lienzo, centrada en el puntero, por encima de todo.
- Soltarla sobre una página la añade a esa página, encima de sus elementos, centrada en el punto donde se soltó, seleccionada y con la página activa (spec 050). Es un paso de deshacer.
- Soltarla fuera de toda página no añade nada.

No alcance: otras formas (líneas, polígonos, estrellas, flechas), la búsqueda, «Usado recientemente», y arrastrar textos e imágenes desde sus pestañas.

## Dependencias

Specs 020 (`new_piece`), 049 y 050. D-94.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S52-01 | Nombre (D-94). | La pestaña dice «Formas» en español y «Shapes» en inglés. |
| S52-02 | Dónde cae. | El punto de la página bajo el puntero, en px de página. Fuera de toda página, ninguno. |
| S52-03 | La forma soltada. | El tamaño de `new_piece` (un tercio del lado corto de la página), centrada en el punto. Encima de los elementos de la página, seleccionada, con la página activa. Un paso de deshacer. |
| S52-04 | Vista previa. | Mientras se arrastra, la forma se ve semitransparente bajo el puntero, con el tamaño que tendrá a ese zoom. |

## TDD

1. `s52_02_drop_point`: dos páginas apiladas con zoom 0,5; el punto en px de página de cada una; nada en el espacio entre ellas.
2. `s52_03_piece_at`: la caja de `new_piece` centrada en el punto.

Integración: `s52_03_a_dropped_shape_lands_where_it_was_dropped`.

## Evidencia

- TDD: `drop_point` (vacío) y `piece_at` (sin centrar) fallaron por sus aserciones y pasaron con la implementación.
- Botón de forma con `click_and_drag`: clic añade al centro; arrastre pinta la vista previa en la capa `Tooltip` (acento al 50 %, tamaño de un tercio del lado corto de la página bajo el puntero) y al soltar pide `drop_piece`; `Editor.page_rects` guarda las cajas de las páginas del último frame.
- Catálogos: `tab.pieces` = «Formas»/«Shapes», `pieces.shapes` = «Formas básicas»/«Basic shapes»; glosario de 03 actualizado (D-94).
- Gates de `AGENTS.md` en verde (3192 passed); el titular lo probó en Linux.

## Riesgos y decisiones abiertas

Ninguna abierta.
