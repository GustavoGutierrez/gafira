# 022 — Tiradores de tamaño en el lienzo

> **Estado:** done — 2026-11-16 — S22-01..S22-03 cubiertos por `gafira-ui` (`canvas::tests` 2, `editor::tests` 1); 2804 tests.

## Objetivo

Cambiar el tamaño de una forma con el mouse (03 §9.1): ocho tiradores en la selección, y arrastrar uno redimensiona con el borde opuesto fijo, en una sola entrada de deshacer.

## Alcance / no alcance

Alcance (`gafira-ui`): ocho tiradores (esquinas y puntos medios) con `surface` y `canvas.selection`; un tirador de la selección tiene prioridad sobre lo que haya debajo; el arrastre usa el mismo gesto que mover (ARQ-33); el lado mínimo es 1 px.

No alcance: mantener la proporción con `Mayús`, redimensionar desde el centro con `Alt`, rotar, cursores por tirador, ajuste a guías.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S22-01 | Tiradores en su sitio. | Esquinas y puntos medios del rectángulo; el centro no es un tirador. |
| S22-02 | Redimensionar. | El borde opuesto no se mueve; el desplazamiento se divide por el zoom; ningún lado baja de 1 px. |
| S22-03 | Por el engine, en un deshacer. | Arrastrar la esquina inferior derecha agranda sin mover la superior izquierda; un deshacer vuelve al tamaño inicial. |
