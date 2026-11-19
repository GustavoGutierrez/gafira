# 051 — Mover elementos entre páginas

> **Estado:** in-progress — 2026-11-19 — pedida por el titular con una captura de referencia; D-92.

## Objetivo

Que un elemento se arrastre de una página a otra y quede en la página donde se suelta. Que lo que sobresale de una página se vea recortado a ella, con el marco de selección completo por encima de todo y tomable también fuera de la página (D-92).

## Alcance / no alcance

Alcance (`gafira-ui`):

- Cada página dibuja sus elementos recortados a su rectángulo (D-92).
- La selección (marco, tiradores y asa de giro) se dibuja después de todas las páginas, por encima de ellas y de los espacios entre ellas.
- Un clic o un arrastre dentro de la caja de un elemento seleccionado lo toma, aunque el punto esté fuera de su página.
- Soltar un arrastre de movimiento con el puntero sobre otra página pasa los elementos movidos a esa página, en la misma posición de pantalla, encima de sus elementos. El movimiento y el traspaso son un solo paso de deshacer. Los elementos quedan seleccionados en la nueva página, que pasa a ser la activa (spec 050).
- Soltar fuera de toda página, o sobre la misma página, solo mueve.

No alcance: arrastrar a otra página desde Capas, un aviso de contenido fuera de la página (FD-09), y la exportación (ya recorta a la página).

## Dependencias

Specs 040, 043 (subárboles, `spec_of`, `top_ids`), 044 y 050. D-92.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S51-01 | Recorte (D-92). | Lo dibujado de una página no sale de su rectángulo. |
| S51-02 | Selección encima. | El marco y los tiradores de la selección se dibujan después de todas las páginas. |
| S51-03 | Tomar fuera de la página. | Un clic en la parte de la caja de un elemento seleccionado que queda fuera de su página lo mantiene seleccionado, y un arrastre lo mueve. |
| S51-04 | Destino al soltar. | Sobre otra página, esa página; sobre la propia o fuera de toda página, ninguna. |
| S51-05 | Traspaso. | Los subárboles seleccionados salen de la página de origen y entran como raíces encima de los de la de destino, con su posición trasladada por la diferencia entre las esquinas de las páginas, y con los mismos datos. Un solo paso de deshacer, y deshacer devuelve el documento al de antes del arrastre. |

## TDD

1. `s51_04_drop_target`: los casos de S51-04 con dos páginas apiladas.
2. `s51_05_transfer`: un rectángulo y un grupo con dos hijos pasan de la página 1 a la 2. Las posiciones se trasladan, el orden se conserva, los ids nuevos son los que da `top_ids` y la página 1 queda sin ellos. Deshacer los devuelve.

## Evidencia esperada

Tests en verde, gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Los elementos traspasados reciben ids nuevos (`InsertNodes`), como al cortar y pegar.
