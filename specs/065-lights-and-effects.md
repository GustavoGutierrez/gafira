# 065 — «Luces y efectos»: cinco luces personalizables

> **Estado:** in-progress — 2026-11-21 — D-103, D-104, D-105. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Que Elementos tenga una categoría «Luces y efectos» con luces personalizables que una persona no diseñadora entienda: «Brillo», una estrella de luz; «Destello de lente», el brillo más el reflejo de una cámara a lo largo de la página (D-103); «Destello de cine», «Rayos de sol» y «Luces bokeh» (D-104). Como en el programa de referencia, cualquier forma se puede convertir en una fuente de luz, y el destello se calcula sobre el encuadre (la página) como lo haría una lente real.

## Origen y licencia

El comportamiento y las opciones se tomaron del manual público de un programa existente (capítulos «Sparkle» y «Lens Flare»). Su licencia prohíbe la ingeniería inversa, así que no se analizó su binario: la implementación es propia, con técnicas gráficas estándar (degradados gaussianos, rayos que se afinan, fantasmas a lo largo del eje luz-centro).

## Alcance / no alcance

Alcance: el campo `ShapeNode.effect` (cambio aditivo del esquema 1) con su validación; la propiedad editable con deshacer; los píxeles de las luces en `gafira-raster`, compartidos por la exportación y el lienzo; la categoría del panel; los ajustes del Inspector.

No alcance: «Pasar por detrás» (descartado por el titular, D-103); animar las luces; otros efectos del mismo programa (lentes, filtros).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S65-01 | Modelo (D-103). | `ShapeNode.effect` es `sparkle` o `lens_flare`, con rayos 4–16, grosor e intensidad 0–1, ángulo y halo de color opcional. Una forma sin efecto conserva sus bytes; un efecto fuera de rango no valida. Se cambia y se quita con deshacer. |
| S65-02 | Brillo. | Núcleo blanco, rayos con su número, grosor, intensidad y ángulo, y halo exterior del color elegido; la caja de la forma da el centro y el tamaño. Alfa = canal más brillante (premultiplicado): sobre fondo oscuro suma luz casi como «Trama». |
| S65-03 | Destello de lente. | El brillo más un resplandor y un anillo del color del halo (rojo cálido si no tiene), y fantasmas de colores y un aro arcoíris a lo largo de la línea de la luz al centro de la página; se recalcula al mover o cambiar el tamaño de la luz. Con la luz fuera de la página (un sol fuera de cuadro), sus fantasmas siguen cruzando la página. |
| S65-04 | Igual en exportación y lienzo. | El grafo de render emite la imagen de la luz en lugar de la forma; el lienzo dibuja la misma imagen como textura. |
| S65-05 | Panel e Inspector. | «Luces y efectos» en Elementos con «Brillo» y «Destello de lente»; al pulsarlos se insertan en el centro de la página, seleccionados. El Inspector muestra rayos, grosor, intensidad, ángulo, halo y «Quitar efecto» en lugar de relleno y contorno. |
| S65-06 | Cualquier forma es una luz. | El Inspector de cualquier forma (no texto, imagen, grupo ni línea) tiene la fila «Luz: Ninguna / Brillo / Destello de lente». Elegir una luz la convierte, con los ajustes por defecto; cambiar de tipo conserva los ajustes; «Ninguna» la devuelve a su relleno, que se conserva. Todo con deshacer. |
| S65-08 | Destello de cine (D-104). | Núcleo blanco y una línea de luz larga a lo largo del ángulo, con un resplandor ancho, blanca junto al núcleo y del color elegido lejos; cruza la página. Sin ajuste de rayos; «Grosor» la engrosa. |
| S65-09 | Rayos de sol (D-104). | Haces de luz (el doble de los rayos) de anchura y brillo desiguales que salen de la luz, giran con el ángulo y se apagan con la distancia; cruzan la página, con huecos oscuros entre ellos. |
| S65-10 | Luces bokeh (D-104). | Tantos círculos de luz como la «Cantidad», dentro de la caja de la forma, siempre en los mismos sitios para los mismos ajustes; «Tamaño» los agranda y el ángulo los gira. Tonos del color elegido. |
| S65-11 | Arrastrar y soltar (D-105). | Cada luz del panel se añade con un clic o arrastrándola al lienzo, centrada donde se suelta, seleccionada, en un paso de deshacer. |
| S65-12 | Inspector legible. | «Luz» es una lista desplegable del ancho del Inspector; cada ajuste lleva su nombre encima y el deslizador debajo, del ancho del Inspector, sin cortes; los nombres cambian con la luz («Cantidad», «Tamaño», «Grosor», «Color»). |
| S65-07 | Opacidad y giro. | La opacidad del elemento atenúa la luz, en el lienzo y en la exportación; el giro de la forma gira los rayos del Brillo. |

## Casos TDD

- `gafira-doc/tests/effect.rs`: `s65_01_effect_round_trip`, `s65_01_effect_validation`.
- `gafira-edit`: `s65_01_the_effect_is_set_and_removed`.
- `gafira-raster` (`light.rs`): `s65_02_*`, `s65_03_*`, `s65_04_which_lights_span_the_page`, `s65_08_the_streak_runs_along_its_angle`, `s65_09_sun_rays_fan_out_with_gaps`, `s65_10_bokeh_discs_stay_in_the_box_and_repeat`.
- `gafira-ui` (`editor.rs`): `d105_a_dropped_light_lands_where_it_was_dropped`.
- `gafira-render` (`graph.rs`): `s65_04_a_light_draws_a_picture_instead_of_the_shape`.
- `gafira-render-cpu/tests/lights.rs`: `s65_04_lights_render_on_a_black_page`.

## Evidencia

Tests anteriores en verde; render de exportación sobre página negra revisado visualmente; panel, inserción e Inspector probados en la app; iconos Phosphor `sparkle`, `sun`, `arrows-out-line-horizontal`, `sun-horizon` y `circles-three` registrados en `assets/LICENSES.toml`.

## Riesgos y decisiones abiertas

- Cada movimiento de un deslizador del Inspector es un paso de deshacer, como los demás campos numéricos del Inspector; agruparlos en un gesto es una mejora común a todos.
- Las texturas de luz se limitan a 1024 px por lado: la luz es difusa y a más resolución solo cuesta tiempo.
- Solo las formas pueden ser luces: el efecto vive en `ShapeNode`. Convertir textos o imágenes en luz necesitaría llevarlo a `ElementBase`.
- Sobre fondo claro, las partes de color de una luz tiñen ligeramente el fondo en lugar de desaparecer (no es una suma pura de luz). Es lo que permite ver la luz sobre blanco.
