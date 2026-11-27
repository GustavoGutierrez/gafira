# 109 — Formas grunge: pinceladas, salpicaduras y rasgaduras

> **Estado:** done — 2026-11-27 — aceptada por el titular en la app; 26 pinceladas (simuladas cerda a cerda y vectorizadas), 13 salpicaduras y 11 rasgaduras, 10 de ellas dibujadas por el titular (GPL, en `assets/LICENSES.toml`); S109-02/03 con pruebas.

## Objetivo

Que la biblioteca de Formas tenga trazos de estilo grunge: pinceladas de pincel seco, salpicaduras y rasgaduras de garra, negras, sólidas y con fondo transparente, para usarlas como cualquier otra forma (color, tamaño, giro, efectos).

## Alcance / no alcance

Alcance:

- Tres secciones nuevas al final de Formas: «Pinceladas» (26), «Salpicaduras» (13) y «Rasgaduras» (11).
- Diez formas dibujadas por el titular y vectorizadas (`assets/shapes/grunge/`, licencia de Gafira), al principio de su sección: dos pinceladas, cinco salpicaduras y tres rasgaduras, una con las garras.
- Pinceladas de pincel seco simuladas cerda a cerda sobre una rejilla y vectorizadas: inicio cargado, bordes ásperos, cerdas que se quedan sin pintura una a una hacia un final seco abierto, vetas (huecos) y motas sueltas; trazos rectos, curvos y en S, horizontales, verticales y diagonales.
- Salpicaduras: mancha central con picos, gotas en sus puntas y rocío alrededor.
- Rasgaduras: tres o cuatro tajos largos y paralelos de garra, en diagonal, finos y afilados en ambos extremos, más anchos pasada la mitad, con dientes angulosos de desgarro en los bordes y alguna astilla suelta.
- Las generadas son trazados (`ShapeGeometry::Path`) con ruido de semilla fija: el mismo resultado en cada ejecución.

No alcance: pinceles para dibujar a mano alzada, texturas de mapa de bits, variantes de color dentro de la forma.

## Dependencias

- Biblioteca de Formas y sus trazados (specs 053, 054, 096), `ShapePath` y sus límites (`gafira-doc`).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S109-01 | Secciones nuevas | «Pinceladas», «Salpicaduras» y «Rasgaduras» aparecen en Formas con sus 26, 13 y 11 formas, con nombre en español e inglés |
| S109-03 | Formas del titular | Las diez aparecen en su sección, enteras, con sus huecos (el par-impar del SVG pasa a la regla no-cero) y registradas en `assets/LICENSES.toml` |
| S109-02 | Formas válidas y fijas | Cada una es un trazado válido del documento que llena su caja, con área, varias piezas y, en las pinceladas, vetas como huecos; iguales en cada ejecución |

## Pruebas

- `s109_02_grunge_shapes_are_fixed_and_filled` y la prueba general de la biblioteca `s54_05_library_paths_are_valid_and_fill_the_box`, que ya recorre todas las formas.

## Evidencia para cerrar

- Pruebas en verde, gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Son vectoriales: el borde seco se imita con contorno irregular y huecos, no con textura de píxeles.
