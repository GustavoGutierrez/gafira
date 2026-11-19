# 058 — Líneas con puntas que se conectan a las formas

> **Estado:** in-progress — 2026-11-19 — pedida por el titular con capturas de referencia; D-98.

## Objetivo

Que Formas tenga una sección «Líneas» y que una línea se pueda conectar a una forma: al arrastrar un extremo cerca de una forma aparecen sus puntos de conexión (arriba, derecha, abajo, izquierda y centro), el extremo se pega como un imán, y si la forma se mueve la línea la sigue sin despegarse (D-98).

## Alcance / no alcance

Alcance:

- `gafira-doc`: `NodeKind::Line(LineNode)` con dos extremos; cada extremo tiene un punto en la caja unidad del `Placement` (como un trazado, D-95), una punta (`none`, `arrow`, `circle`, `diamond`, `diamond_filled`, `square`) y, si está conectado, la forma y el punto (`top`, `right`, `bottom`, `left`, `center`); la línea tiene color y grosor. `LineNode::ends` da los dos extremos en px de página: el del punto de conexión de la forma si existe en la página, si no el de su punto en la caja. Una conexión a una forma que ya no está no invalida el documento.
- `gafira-edit`: `Property::Line(LineNode)`.
- `gafira-render`: la línea se dibuja como un trazo de su grosor entre sus extremos resueltos, con sus puntas; entra en la exportación.
- `gafira-ui`:
  - sección «Líneas» con cinco líneas de la referencia (círculos, rombos, rombos rellenos, sin puntas, cuadrado y flecha), con clic y arrastre como las formas;
  - una línea seleccionada muestra sus dos extremos como tiradores; arrastrar uno lo mueve y muestra el largo y el ángulo junto al puntero;
  - cerca de una forma (a 40 px de pantalla de su caja) se ven sus cinco puntos de conexión; a 12 px de uno el extremo se engancha y queda conectado al soltar; lejos, queda suelto;
  - mover, girar o redimensionar una forma mueve con ella los extremos conectados.

No alcance: líneas curvas o en codo, elegir la punta y el grosor en el Inspector, texto sobre la línea, conectar a puntos que no sean esos cinco.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S58-01 | Modelo (D-98). | Una línea con extremos conectados se valida, se guarda y se carga igual; un grosor no finito, negativo o mayor que 1000 da error; una conexión a un nodo que no está no da error. |
| S58-02 | Extremos resueltos. | Un extremo suelto está en su punto de la caja; uno conectado, en el punto de la forma, también girada; si la forma no está, en su punto de la caja. |
| S58-03 | Render. | La línea se dibuja entre sus extremos resueltos; mover la forma conectada mueve la línea dibujada. |
| S58-04 | Conectar en el lienzo. | Soltar un extremo a 12 px de pantalla de un punto de conexión lo conecta; más lejos, queda suelto en ese punto; un paso de deshacer. |
| S58-05 | Biblioteca. | La sección «Líneas» va antes de «Formas básicas» con sus cinco líneas. |

## TDD

1. `s58_01_line_round_trips`, `s58_01_line_validation` y `s58_02_line_ends` (`gafira-doc`).
2. `s58_03_a_connected_line_follows_its_shape` (`gafira-render`).
3. `s58_04_nearest_anchor` y `s58_04_a_dropped_end_connects` (`gafira-ui`).

## Evidencia esperada

Tests en verde, gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Una versión anterior de Gafira no abre un `.gaf` con líneas (D-98).
