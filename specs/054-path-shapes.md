# 054 — Formas de trazado: polígonos, estrellas, flechas y más

> **Estado:** in-progress — 2026-11-19 — D-95; sigue a la 053.

## Objetivo

Que el panel Formas ofrezca las secciones de la referencia del titular (polígonos, estrellas, flechas, diagramas de flujo, globos de diálogo, nubes, corazones, banners, lágrimas, engranajes y formas orgánicas) con una geometría nueva de trazado vectorial en el documento (D-95), que se rellena, se contornea, se escala, se gira y se exporta como las formas de hoy.

## Alcance / no alcance

Alcance:

- `gafira-doc`: `ShapeGeometry::Path(ShapePath)`, una lista de órdenes (`move_to`, `line_to`, `quad_to`, `cubic_to`, `close`) en coordenadas de la caja unidad (0 a 1 en cada eje, escalada al `Placement`), con relleno de regla no nula. Validación: al menos una orden, la primera es `move_to`, como mucho 4096 órdenes, coordenadas finitas entre −1 y 2. En JSON es aditiva: `"rect"` y `"ellipse"` siguen igual y el trazado es `{"path": {"commands": [...]}}`.
- `gafira-edit` y `gafira-render`: la geometría deja de ser `Copy`; el render convierte el trazado en un `BezPath` dentro de la caja, con la misma clave de caché por contenido.
- `gafira-ui`: el lienzo rasteriza las formas de trazado con `tiny-skia` (relleno y contorno) en una textura del tamaño en pantalla y la dibuja en los ejes del elemento; la prueba de toque usa el trazado; la biblioteca genera las formas como funciones puras y las agrupa por secciones.

No alcance: editar los puntos de un trazado, Líneas (necesitan extremos y puntas), herramientas de dibujo, «Ver todo», búsqueda, «Usado recientemente».

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S54-01 | Modelo (D-95). | Un documento con un trazado se valida, se guarda y se carga igual; un documento sin trazados escribe el mismo JSON que antes. |
| S54-02 | Validación. | Sin órdenes, sin `move_to` al principio, más de 4096 órdenes, una coordenada no finita o fuera de −1 a 2 dan error de validación con el campo. |
| S54-03 | Render. | Un triángulo de trazado rellena los píxeles de su interior y no los de fuera en una exportación PNG; su contorno usa el de `gafira-pathops`. |
| S54-04 | Lienzo. | Las formas de trazado se ven rellenas, contorneadas, giradas y volteadas como en la exportación; un clic dentro del trazado las toma y uno en la caja pero fuera del trazado, no. |
| S54-05 | Biblioteca. | Cada forma de la biblioteca es un trazado válido dentro de la caja unidad que toca sus cuatro lados; las secciones siguen el orden de la referencia y ninguna está vacía. |

## TDD

1. `s54_01_path_round_trips` y `s54_01_old_json_is_unchanged` (`gafira-doc`).
2. `s54_02_path_validation` (`gafira-doc`).
3. `s54_03_a_path_triangle_fills_its_inside` (`gafira-render`).
4. `s54_04_a_path_hits_inside_only` (`gafira-ui`).
5. `s54_05_library_paths_are_valid_and_fill_the_box` (`gafira-ui`).

## Evidencia esperada

Tests en verde, gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Una versión anterior de Gafira no abre un `.gaf` con formas de trazado (D-95).
