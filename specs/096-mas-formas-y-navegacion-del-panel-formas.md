# 096 — Más formas y navegación del panel Formas

> **Estado:** in-progress — 2026-11-23 — implementada y probada en Xephyr (D-127); pendiente de la prueba del titular.

## Objetivo

El panel Formas ofrece muchas más formas y se recorre con comodidad: cada sección muestra una fila de cinco, con flechas para ver las siguientes y «Ver todo» para verlas todas.

## Alcance / no alcance

Alcance:

- **Navegación**: fila de cinco formas por sección. Con más de cinco, «Ver todo» abre la sección entera (tres por línea, con flecha para volver) y unas flechas redondas a los lados de la fila la mueven de cinco en cinco, sin pasar de los extremos.
- **Formas nuevas** (de 48 a 173): líneas (cuadrados, cuadrados rellenos, círculos rellenos, barras, discontinua, punteada, flechas simple, doble y punteadas); básicas (triángulo invertido, rombo, más, barril, ticket, paralelogramos, trapecios, forma de U, arco, semicírculo, cuarto de círculo, cuarto y medio anillo); polígonos (7, 9 y 12 lados); estrellas (7, 12 puntas y explosión); flechas (vertical doble, chevrón, con muesca); diagrama de flujo (retraso, entrada manual, tarjeta, base de datos, datos almacenados, pantalla, varios documentos, proceso predefinido, almacenamiento interno, límite de bucle, ordenar, intercalar, O lógico, unión de suma, cinta perforada, acceso secuencial, datos de acceso directo, cubo, paso); globos (de nube, ovalado, de pensamiento); nubes (esponjosa, pequeña, grande); corazones (estrecho, redondo, puntiagudo, suave); banners (etiquetas clásica, festoneada, vertical y redondeada; cintas curva, recta y ondulada); lágrimas (alargada, gruesa, invertida, de lado, llama, con hueco); engranajes (10, 16 dientes, dientes agudos, macizo); estrellas cuadradas (8, 5, 12 brazos, grueso, de bloque, con barra, redondeado, aspa, molinete); orgánicas (flores, trébol, cruz de pétalos, hoja, ondas, mancha, piedra).
- **Secciones nuevas**: «Formas abstractas» (cojín, pastillas apiladas, medias lunas, bloques, bloque inclinado, molino, destellos, flor con ventana, escalones, judía) y «Colgantes» (16 etiquetas con su agujero para colgar).
- **Líneas**: el modelo gana el trazo discontinuo y punteado y los extremos círculo relleno, cuadrado relleno y barra, en el lienzo, la exportación y PowerPoint.

No alcance:

- Buscar formas por nombre.
- Elegir el trazo o los extremos de una línea ya puesta desde el inspector.

## Dependencias

- Specs 053, 054 (formas por trayectoria) y 058 (líneas).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S96-01 | Fila paginada. | Cinco por fila; las flechas mueven cinco y no pasan de los extremos; «Ver todo» solo con más de cinco. |
| S96-02 | Secciones ampliadas. | Básicas, polígonos, estrellas, flechas, diagrama de flujo, globos, corazones y engranajes tienen más de cinco; nubes, cinco. |
| S96-03 | Trazo y extremos nuevos. | `LineDash` y los extremos nuevos se guardan y leen; una línea antigua es continua; la exportación dibuja guiones, puntos, extremos rellenos y barra. |
| S96-04 | Formas válidas. | Toda forma nueva es una trayectoria válida que llena su caja, con área y proporción entre 1:4 y 4:1 (S54-05). |

## TDD

1. `s96_01_a_row_pages_by_five_and_stops_at_the_ends`.
2. `s96_02_the_grown_sections_have_more_than_five`.
3. `s96_03_dashes_and_new_caps_round_trip` (gafira-doc) y `s96_03_dashes_dots_and_new_caps` (gafira-render).
4. `s54_05_library_paths_are_valid_and_fill_the_box` sobre las 173 formas.

## Evidencia

- Pruebas S96 en verde y S54-05 sobre todas las formas.
- Revisión visual de una lámina con todas las formas: se corrigieron la cola del globo de nube, el molinete, el agujero del colgante de corazón, el hueco de la flor con ventana y el grosor de las cintas.
- Xephyr: las secciones muestran «Ver todo» y flechas; la flecha de Líneas pasa a las cinco siguientes; «Ver todo» de Líneas muestra las 15 en tres columnas; una línea punteada se inserta y se dibuja en puntos.

## Riesgos y decisiones abiertas

- Los extremos y trazos nuevos no existen en versiones anteriores de Gafira: un documento que los use no se abre en ellas.
