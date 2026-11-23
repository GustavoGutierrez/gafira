# 020 — Primeras piezas en el lienzo: insertar, seleccionar, mover, borrar

> **Estado:** done — 2026-11-16 — S20-01..S20-05 cubiertos por `gafira-ui` (`canvas::tests` 4, `editor::tests` 1); mutantes a mano: el orden de impacto muere; quitar el gesto del arrastre sobrevive porque la historia ya fusiona movimientos seguidos del mismo elemento en un test rápido (equivalente con ese reloj). 2799 tests.

## Objetivo

Que el Editor de diseño edite: añadir formas desde la pestaña Piezas, dibujarlas, seleccionarlas con un clic, moverlas arrastrando, cambiar su posición y tamaño en el Inspector y borrarlas, todo con deshacer (03 §7.3, §9.1; 02 §5.5).

## Alcance / no alcance

Alcance (`gafira-ui`):

- `canvas`: formas visibles de una página en orden de dibujo (raíces y luego hijos; los grupos no dibujan), rectángulo en pantalla, impacto de la forma de arriba (la elipse solo dentro de su contorno), movimiento en espacio de página y caja de una pieza nueva (centrada, un tercio del lado menor).
- Piezas → Formas: «Rectángulo» y «Elipse» insertan con `InsertNodes` en la página del elemento seleccionado o en la primera, con el color de acento del tema, y quedan seleccionadas.
- Dibujo de rectángulos y elipses con su relleno; contorno de selección con `canvas.selection`.
- Clic selecciona la forma de arriba o limpia la selección; arrastrar mueve con `SetProperty` dentro de un gesto (`begin_gesture`/`end_gesture`, una sola entrada de deshacer, ARQ-33).
- Inspector → Posición: X, Y, Ancho y Alto editables.
- `Supr` o `Retroceso` borran la selección con `RemoveNodes` (fuera de un campo de texto).

No alcance:

- Rotación, sesgo y volteo al dibujar; tiradores de tamaño; selección múltiple y marco de selección; guías y ajuste magnético.
- Texto, imágenes, líneas, iconos y el resto de Piezas; el render del documento con `gafira-render` (el lienzo dibuja con egui).
- Barra contextual (03 §7.3).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S20-01 | Orden de dibujo. | Las formas salen de atrás hacia adelante, sin grupos ni ocultas. |
| S20-02 | Impacto. | Se elige la forma de arriba; fuera de la curva de una elipse se elige la de abajo; fuera de todo, nada. |
| S20-03 | Mover. | El desplazamiento en pantalla se divide por el zoom; el tamaño no cambia. |
| S20-04 | Pieza nueva. | Centrada y de un tercio del lado menor; el color va y vuelve sin pérdida. |
| S20-05 | Edición por el engine. | Insertar selecciona; un arrastre de varios pasos se deshace de una vez; el Inspector cambia el ancho; borrar quita la forma y se deshace. |
