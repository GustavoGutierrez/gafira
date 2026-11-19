# 053 — Biblioteca de Formas: secciones y botones con la forma

> **Estado:** in-progress — 2026-11-19 — pedida por el titular con una captura de referencia (panel de formas por secciones).

## Objetivo

Que el panel Formas muestre las formas agrupadas por secciones, cada una como un botón que dibuja la forma (no un texto), y que se pueda añadir con un clic o arrastrándola (spec 052). Primero las formas que el modelo ya dibuja; las que necesitan geometría nueva quedan para una spec siguiente.

## Alcance / no alcance

Alcance (`gafira-ui`, catálogos):

- Sección «Formas básicas» con Cuadrado, Cuadrado redondeado y Círculo. El cuadrado redondeado es un rectángulo con radio de esquina de un octavo del lado (spec 038).
- Cada forma es una baldosa de 56 px con la forma dibujada en `text` sobre el fondo del panel, `secondary.bg` al pasar por encima, nombre accesible y tooltip con su nombre.
- La vista previa del arrastre tiene la forma de la baldosa (con sus esquinas redondeadas).
- Las secciones son una lista de datos; una sección sin formas no se muestra.

No alcance: Líneas, Polígonos, Estrellas, Flechas, diagramas de flujo, globos, nubes, corazones, banners, lágrimas, engranajes y formas orgánicas o abstractas (necesitan geometría de trazado en el documento: spec siguiente con decisión del titular); «Ver todo»; búsqueda; «Usado recientemente».

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S53-01 | Las formas básicas. | Cuadrado: rectángulo sin radio; Cuadrado redondeado: rectángulo con radio = lado / 8; Círculo: elipse; las tres de lado un tercio del lado corto de la página. |
| S53-02 | Secciones. | El panel lista las secciones en orden con su título y sus formas en filas de baldosas; una sección vacía no aparece. |
| S53-04 | Relleno por defecto. | Una forma nueva, con clic o arrastre, se rellena de gris claro #CCD1D9, el mismo en tema claro y oscuro (pedido del titular). |
| S53-03 | Baldosas. | Cada baldosa dibuja su forma, tiene nombre accesible y tooltip, y responde a clic (añadir al centro) y arrastre (soltar donde se quiera). |

## TDD

1. `s53_01_basic_pieces`: geometría, radio y tamaño de cada forma básica.
3. `s53_04_default_fill_is_light_grey`: el relleno de una forma nueva es #CCD1D9.
2. `s53_02_sections`: la sección de formas básicas, en orden, y ninguna sección vacía.

## Evidencia esperada

Tests en verde, gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Las demás secciones de la referencia necesitan una geometría nueva en el documento (polígono o trazado), un cambio de formato: se decide con el titular antes de la spec siguiente.
