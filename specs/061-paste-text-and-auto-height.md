# 061 — Pegar texto, altura automática de las cajas y arrastrar textos

> **Estado:** in-progress — 2026-11-19 — pedida por el titular.

## Objetivo

Que pegar texto copiado de cualquier lugar cree una caja de texto con él; que una caja de texto crezca o encoja en alto con su texto al cambiar su ancho o al escribir en ella; y que los textos del panel Texto se arrastren a la página como las formas (spec 052).

## Alcance / no alcance

Alcance (`gafira-ui`):

- Ctrl+V con texto que no es el de una copia de Gafira crea en la página actual una caja de texto del estilo «cuerpo» con ese texto, seleccionada, a la que después se ajusta el alto (dos pasos de deshacer: pegar y ajustar).
- Una caja de texto con `TextSizing::AutoHeight` toma el alto de su texto: al redimensionarla (en el mismo paso de deshacer que el arrastre) y al escribir en ella en el lienzo (en el mismo paso que lo escrito).
- Los tres textos del panel Texto se arrastran con una vista previa y se sueltan centrados donde se sueltan; un clic los sigue añadiendo en el centro.

No alcance: fuentes y estilos por tramo (specs 062 y 063), cajas de ancho automático.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S61-01 | Pegar texto. | Un pegado con texto ajeno crea una caja de texto con ese texto en la página actual; uno con el texto de una copia de Gafira sigue pegando los elementos (spec 043). |
| S61-02 | Alto automático. | El alto pedido es el del texto maquetado; una caja de alto fijo no cambia; una diferencia de medio px o menos no cambia nada. |
| S61-03 | Arrastrar textos. | Un texto soltado en una página queda centrado en el punto, seleccionado, en un paso de deshacer. |

## TDD

1. `s61_01_foreign_text_pastes_a_text_box` (se ajusta `s43` que comprobaba que el texto ajeno no se pegaba).
2. `s61_02_text_fit`.
3. `s61_03_a_dropped_text_lands_where_it_was_dropped`.

## Evidencia esperada

Tests en verde, gates y prueba del titular.
