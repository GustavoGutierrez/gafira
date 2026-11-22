# 079 — Dibujo a mano con pinceles

> **Estado:** in-progress — 2026-11-21. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Dibujar a mano en la página con pinceles sencillos, para cualquier persona, y que cada trazo quede como un único elemento vectorial que se mueve, se recolorea y cambia su transparencia como cualquier forma.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S79-01 | Caja de herramientas. | El panel Dibujo es solo herramientas en tarjetas: Seleccionar (icono de flecha), 16 pinceles y el Borrador (icono de borrador: goma rosa y funda azul); la explicación de cada una está en su tooltip. Cada tarjeta de pincel es una muestra del trazo dibujada por el propio pincel, en su color. Pinceles: Esfero, Marcador, Resaltador (multiplica), Rotulador fino, Lápiz, Pincel, Caligrafía, Crayón, Neón (aclara), Pluma estilográfica, Rotulador pincel, Tinta china, Acuarela (multiplica), Tiza, Carboncillo y Pincel seco. Debajo, el color, el grosor, la transparencia y el estilo del pincel en uso. Añadir un pincel es añadir una entrada a `draw::BRUSHES` y su nombre. |
| S79-02 | Estilo propio. | Cada pincel tiene su color, grosor, transparencia y estilo: suavizado, inicio fino, final fino, afinar con la velocidad, textura y, en Caligrafía, ángulo de la plumilla. Todo se ajusta en «Estilo del pincel» y «Restablecer pincel» lo devuelve a su valor inicial (`styles_shape_the_line`). |
| S79-03 | Un trazo, un elemento. | Mientras se dibuja, el trazo se ve en vivo. Al soltar, queda un único `ShapeNode` con el contorno del trazo como trazado relleno y suavizado, con su color, su transparencia como opacidad del elemento, el modo de fusión del pincel y `drawn_with` con el id del pincel. Un clic sin mover deja un punto (`every_brush_makes_one_path`, `a_click_is_a_dot`, `s79_a_stroke_is_one_element_and_the_eraser_removes_it`). |
| S79-04 | Borrador. | El borrador quita trazos enteros: los que toca o roza al pasar; se atenúan mientras pasa y se borran en un solo paso que «Deshacer» devuelve. Solo borra lo dibujado a mano (con `drawn_with`) y no bloqueado (`the_eraser_finds_strokes`). |
| S79-05 | Cursor. | Sobre el lienzo, el cursor es la herramienta en la mano: esfero, marcador, resaltador, lápiz, pincel, caligrafía, crayón o neón, con la punta en el puntero y la tinta del color elegido; el borrador, como un borrador rosa. |
| S79-06 | Modelo. | `ShapeNode::drawn_with` (opcional; solo se escribe si existe) marca un trazo a mano; los archivos anteriores no cambian. |
