# 110 — «Efectos de luz»: nueve luces nuevas y tarjetas planas

> **Estado:** done — 2026-11-27 — aceptada por el titular en la app; «Luces y efectos» pasa a «Efectos de luz»; tarjetas planas; Luz de luna, Foco, Resplandor, Estrellas, Polvo de estrellas, Estela de cometa, Aurora boreal, Fuga de luz y Anillo luminoso; S110-01/02/04 con pruebas.

## Objetivo

Que la categoría de luces de Elementos diga lo que contiene y tenga más luces útiles para diseño, con tarjetas que se distingan de las de la cuadrícula principal.

## Alcance / no alcance

Alcance:

- La categoría «Luces y efectos» (spec 065, D-103) pasa a llamarse «Efectos de luz» («Light effects»), a petición del titular: contiene solo luces y efectos de luz.
- Su lista usa una tarjeta blanca plana con el icono de cada luz, sin el abanico de tres tarjetas de la cuadrícula de categorías; al pasar el puntero se oscurece un poco.
- Cuatro luces nuevas, personalizables como las de la spec 065 (`Light`: rayos, grosor, intensidad, ángulo, halo):
  - «Luz de luna»: disco con cráteres (tantos como los rayos), fase con el grosor, lado oscuro con el ángulo, halo frío; en la caja de la forma.
  - «Foco»: cono de luz de escenario desde la luz a lo largo del ángulo; el grosor abre el cono; ocupa la página.
  - «Resplandor»: brillo redondo y suave del color del halo; el grosor lo suaviza; en la caja de la forma.
  - «Estrellas»: estrellas de cuatro puntas, tres veces los rayos, de tamaños y brillos variados; en la caja de la forma.
- Cinco luces más, de la lista del titular (las que ya existían no se repiten: el resplandor lunar es «Resplandor» en azul, la luna creciente es «Luz de luna» con la fase al máximo, «Bokeh», «Destello de cine» como destello anamórfico y «Destello de lente»):
  - «Polvo de estrellas»: partículas de luz diminutas, doce veces los rayos, reunidas en una banda a lo largo del ángulo; en la caja.
  - «Estela de cometa»: cabeza brillante hacia el ángulo y cola de luz que se abre con el grosor; en la caja.
  - «Aurora boreal»: cortinas onduladas verde, azul y violeta (el halo es el color de arriba); el grosor las hace más altas y el ángulo las inclina; ocupa la página.
  - «Fuga de luz»: manchas cálidas que entran desde el borde al que apunta el ángulo, como una fuga de luz en la película; ocupa la página.
  - «Anillo luminoso»: anillo fino como un eclipse, oscuro dentro, con corona y una perla brillante donde apunta el ángulo; en la caja.
- Iconos de Phosphor 2.1.1 (MIT): `moon`, `flashlight`, `sun-dim`, `star-four`, `dots-nine`, `shooting-star`, `waves`, `flame`, `circle-notch`.

No alcance: luces animadas, cambiar las cinco luces existentes.

## Dependencias

- Spec 065 y D-103/D-104 (luces, `ShapeEffect`, `gafira-raster::light`), panel de Elementos (spec 064), iconos (spec 037).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S110-01 | Modelo | `ShapeEffect::Moon`, `Spotlight`, `Glow` y `Stars` con sus valores de inicio; guardan y cargan igual; validan como las demás |
| S110-02 | Dibujo | La luna tiene lado iluminado y lado en sombra con halo que no pasa de su caja; el foco ilumina a lo largo de su ángulo y no detrás; el resplandor y las estrellas no pasan de su caja; el mismo resultado cada vez |
| S110-04 | Cinco luces más | El polvo de estrellas y el anillo no pasan de su caja y el anillo es oscuro dentro y brillante en el borde; la cabeza del cometa brilla más que su cola; la aurora y la fuga de luz ocupan la página, la fuga es cálida en su borde |
| S110-03 | Panel | «Efectos de luz» lista las catorce luces en tarjetas planas; clic añade y arrastrar suelta; el Inspector muestra los controles de cada una |

## Pruebas

- `s110_01` en `gafira-doc/tests/effect.rs` (ida y vuelta), `s110_02_*` y `s110_04_*` en `gafira-raster/src/light.rs`.

## Evidencia para cerrar

- Pruebas en verde, gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Renombrar la categoría cambia el nombre que fija D-103; queda pedido por el titular el 2026-11-27.
