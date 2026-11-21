# 069 — Fondo de la página

> **Estado:** in-progress — 2026-11-21 — D-108. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Que la página tenga un fondo propio —color, degradado, patrón o imagen— que se elige desde el Inspector cuando no hay nada seleccionado, igual en el lienzo y en la exportación.

## Alcance / no alcance

Alcance: `Page.background` (aditivo, esquema 1) con validación (P7 para la imagen); `Property::PageBackground` con deshacer; el fondo como primer subgrafo del render (bajo todo) y en el lienzo; el panel «Fondo de la página»; la búsqueda de colores por nombre (es/en) o hexadecimal; «Aplicar a todas las páginas».

No alcance: la paleta de la Marca (Kit de Marca); degradados o patrones editables más allá de sus colores; fondos de video.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S69-01 | Modelo. | `PageBackground` es `solid`, `gradient`, `pattern` (tipo, color, fondo) o `image` (recurso, tamaño natural, `fill`/`fit`/`tile`); ida y vuelta sin pérdida; una página sin fondo no escribe `background`; una imagen que no está en la tabla, un color no finito o una imagen sin tamaño no validan. |
| S69-02 | Patrones vectoriales. | Catorce patrones: puntos, cuadrícula, rayas, cuadros, zigzag, cruces y, de papel (pedidos por el titular con muestras de referencia, recreados como geometría propia), ladrillo, ladrillo grande, cuadrícula fina (cuadrillé), cuadrícula con diagonales, hexágonos, isométrico (±30°), rombos (±45°) y triangular; todos cubren la página hasta sus cuatro bordes (`s69_02_every_pattern_reaches_every_edge`); en su color sobre su fondo, con una repetición de 32 px de página. Son formas vectoriales (`gafira-raster::pattern_path`: subtrayectos convexos): la exportación las rellena como vectores y el lienzo las dibuja como polígonos aplanados a un cuarto de píxel de pantalla, nítidos a cualquier zoom. El color por defecto es el verde de la marca (`brand.mint`). |
| S69-03 | Igual en exportación y lienzo. | El grafo dibuja el fondo como primer subgrafo: color, degradado y patrón como rellenos vectoriales de la página, imagen como imagen (mosaico repetido a su tamaño, lo que sobra recortado). El lienzo dibuja lo mismo como textura. |
| S69-04 | Buscar colores. | «Prueba con «azul» o «#00c4cc»»: un código hexadecimal (`#RGB`, `#RRGGBB`, `#RRGGBBAA`, con o sin `#`) y los nombres en español o inglés que contienen la búsqueda (sin tildes ni mayúsculas) muestran sus muestras. |
| S69-05 | Aplicar. | Elegir un color, degradado, patrón o imagen cambia el fondo de la página actual en un paso de deshacer; «Aplicar a todas las páginas» copia el de la actual a las demás en un paso; «Quitar fondo» la deja en blanco. |

## Tests

- `gafira-doc/tests/background.rs`: `s69_01_round_trip`, `s69_01_validation`.
- `gafira-raster` (`pattern.rs`): `s69_02_vector_patterns_cover_their_shapes`.
- `gafira-render-cpu/tests/backgrounds.rs`: `s69_03_*`.
- `gafira-ui`: `s69_04_hex_codes`, `s69_04_colours_by_name`, `s69_05_a_background_for_one_page_or_all`.

## Evidencia

Tests en verde; panel, patrón y degradado vistos en la app.
