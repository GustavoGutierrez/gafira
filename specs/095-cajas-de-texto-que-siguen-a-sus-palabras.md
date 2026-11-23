# 095 — Cajas de texto que siguen a sus palabras

> **Estado:** in-progress — 2026-11-23 — implementada y probada en Xephyr (D-126); pendiente de la prueba del titular.

## Objetivo

La caja de un texto se ajusta a sus palabras: un título corto tiene una caja corta, que crece a lo ancho al escribir y se encoge al borrar. Un texto curvo nunca se sale de su caja. Al mover un elemento se ve el puntero de mover de Gafira.

## Alcance / no alcance

Alcance:

- **Ancho automático**: los textos nuevos (panel Texto, arrastrar al lienzo, estilos de Marca) son tan anchos como su línea más larga y no la parten. Al escribir o borrar, el ancho sigue a la línea más larga; Enter abre otra línea. El lado al que se alinean las líneas (izquierda, centro o derecha) queda quieto.
- **Ancho fijo**: arrastrar un lado de la caja fija el ancho; desde entonces las líneas se parten en él y la caja crece hacia abajo (como hasta ahora).
- **Texto curvo**: su caja toma el alto del arco y se ensancha por igual a ambos lados si los extremos del arco la pasan. Se ajusta al escribir, al cambiar la curva, el tamaño o la fuente, y al redimensionar.
- **Puntero de mover**: mientras se arrastra un elemento, el puntero del sistema se cambia por el de Gafira, en tinta de marca con un borde blanco.

No alcance:

- Cambiar los textos de documentos ya guardados: conservan su modo.
- Un control visible para elegir el modo de tamaño.

## Dependencias

- 02 §3.1 (`TextSizing`), spec 061 (la caja crece con su texto), spec 027 (edición de texto), FD-44 (texto curvo).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S95-01 | Propiedad de tamaño. | `SetProperty(TextSizing)` cambia el modo de un texto en un paso, se deshace exacto e ignora lo que no es texto. |
| S95-02 | Ancho que sigue a las líneas. | La caja toma el ancho dado y conserva el borde izquierdo, el centro o el borde derecho según la alineación. |
| S95-03 | Ancho natural. | El ancho natural es el de la línea más larga; con él la exportación no parte ningún párrafo. |
| S95-04 | Texto nuevo. | Un texto nuevo usa ancho automático y su caja mide su ancho natural, centrada. |
| S95-05 | Texto curvo dentro de su caja. | La caja ajustada contiene el contorno de todas las letras y conserva el centro; una caja ya ajustada no cambia. |
| S95-06 | Lado fija el ancho. | Arrastrar un lado de un texto con ancho automático lo pasa a ancho fijo en el mismo paso. |
| S95-07 | Puntero de mover. | Al arrastrar un elemento se dibuja el icono de mover en lugar del puntero del sistema. |

## TDD

1. `s95_01_text_sizing_is_set_and_reverted` (gafira-edit).
2. `s95_02_a_box_as_wide_as_its_lines_keeps_its_anchor`.
3. `s95_03_the_natural_width_is_the_widest_line_and_wraps_nothing`.
4. `s95_04_a_new_text_is_as_wide_as_its_words`.
5. `s95_05_a_curved_text_fits_inside_its_box`.

## Evidencia

- Pruebas S95-01 a S95-05 en verde; `s61_02_text_fit` cubre también el alto con ancho automático. `s27_03` y `s61_03` ajustadas al nuevo ancho.
- Xephyr: «Añadir un título» crea una caja del ancho de «Título»; al escribir crece a la derecha, Enter abre otra línea y al borrar se encoge a la línea más larga. «Arco arriba» queda dentro de su caja; al alargarlo, la caja crece en alto y ancho y contiene todo el arco. Al arrastrar se ve el puntero de mover con su borde blanco.
- Iconos propios en `assets/icons/gafira/` (`cursor.svg`, `move.svg`), registrados en `assets/LICENSES.toml` con su hash; licencia pendiente de confirmar por el titular.

## Riesgos y decisiones abiertas

- El ancho natural se mide con la maquetación de la exportación; el lienzo dibuja con egui sin partir las líneas, así que una diferencia mínima de medidas no parte líneas en ninguno de los dos.
- La licencia de los dos iconos (uno parece de Noun Project) está por confirmar.
