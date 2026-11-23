# 089 — Efectos de texto

> **Estado:** in-progress — 2026-11-22 — implementada y probada en Linux; pendiente de la prueba del titular. Pedida por el titular con capturas de referencia.

## Objetivo

Con un texto seleccionado, «Efectos» abre un panel para darle un efecto, ajustar sus parámetros y quitarlo, con el texto siempre editable.

## Alcance / no alcance

Alcance: los nueve efectos de D-120, Curvar, doce estilos listos con fuentes incluidas; lienzo y todas las exportaciones.

No alcance: efectos en el texto de formas, tablas y gráficos; estilos propios; animación.

## Dependencias

FD-43, FD-44; D-120 (esta spec); specs 073 (texto real en PDF y SVG) y 085 (controles). Sin dependencias nuevas.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S89-01 | Modelo. | `TextNode.effects`, hasta 4, validado y aditivo al esquema 1; sin efectos no se escribe. |
| S89-02 | Dibujo. | Cada efecto se dibuja igual en el lienzo y en la exportación; la maquetación no cambia. Todo es vectorial menos lo desenfocado. |
| S89-03 | Texto real. | En PDF y SVG las palabras siguen siendo texto, también con «Sin relleno»; en PowerPoint, lo más parecido que tiene. |
| S89-04 | Panel. | Botón en la barra y en Propiedades; baldosas con muestra; al elegir un efecto, sus controles aparecen bajo su fila; un control es un paso de deshacer; pulsar el elegido lo quita. |
| S89-05 | Estilos. | Un clic cambia fuente, color y efectos en un paso. |

## TDD

`gafira-doc`, `gafira-raster` (forma suave), `gafira-render` (piezas), `gafira-io-pdf`, `gafira-io-svg`, `gafira-io-pptx` y `gafira-ui`.

## Evidencia

- Gates de 086 a 089 juntas: `cargo fmt --check`, `clippy -D warnings`, `cargo deny` y `cargo test --workspace --locked` en verde, 3636 tests. Fallan desde antes V14 y `forbid(unsafe_code)` en `gafira-photos`.

- Probado en la app (Linux, datos temporales): panel, Paralela, Contorno, Desalineado, Neón, los controles bajo su fila y el estilo Láser. Los demás efectos y estilos se vieron en sus muestras, que se dibujan con el mismo código. Sin probar a mano: Curvar desde el panel ni los archivos exportados en otros programas.
- El titular encontró que Sin relleno y Desalineado trazaban las piezas solapadas de las letras en lugar de su silueta; corregido con la unión de `gafira-pathops` y con test (`s89_02_the_line_is_of_the_outline`).
- Con Sin relleno y Desalineado, las palabras quedan en PDF y SVG como texto transparente; con un texto curvo no hay palabras, como antes.
- Tests: `s89_*` en `gafira-doc`, `gafira-raster`, `gafira-render`, `gafira-io-pptx` y `gafira-ui`.

## Riesgos y decisiones abiertas

- Los valores iniciales de efectos y estilos son una primera propuesta, sujeta a la revisión visual del titular.
