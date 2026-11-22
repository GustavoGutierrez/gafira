# 088 — Recortar imagen y Propiedades de imagen

> **Estado:** in-progress — 2026-11-22 — implementada y probada en Linux; pendiente de la prueba del titular. Pedida por el titular.

## Objetivo

Recortar una imagen de forma fácil: con marco y tiradores, proporciones, formas o a mano, sin perder nada de ella; y ordenar Propiedades de una imagen con los mismos controles de «Editar imagen».

## Alcance / no alcance

Alcance: recorte no destructivo; formas Rectángulo, Redondeado, Círculo (elipse) y A mano; proporciones Libre, Original, 1:1, 4:3, 3:4, 3:2, 2:3, 16:9 y 9:16; Propiedades de imagen; quitar «Iconos» de Elementos y apagar Marca y Plantillas (D-119).

No alcance: enderezar, perspectiva, recortar varias a la vez; la foto de un marco sigue con su propio recorte (spec 066).

## Dependencias

D-119 (esta spec); specs 084 y 085 (original, ajustes, sombra). Sin dependencias nuevas.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S88-01 | Modelo. | `ImageNode.crop` (proporciones de la imagen entera y forma), validado, aditivo al esquema 1; sin recorte no se escribe. |
| S88-02 | Dibujo. | Lienzo, miniaturas y exportación muestran solo lo recortado, con bordes suaves; ajustes, zonas, filtro y sombra siguen el recorte. |
| S88-03 | La imagen no se mueve. | Al aplicar, lo que se ve queda donde estaba, también girada o reflejada; es un paso de deshacer y se puede volver a la imagen entera. |
| S88-04 | Edición. | Página «Recortar» con tiradores, mover dentro, tercios, proporciones, formas y trazo a mano; Intro aplica y Esc cancela; doble clic en la imagen la abre. |
| S88-05 | Propiedades. | Con una imagen: secciones con el mismo estilo, y Opacidad, Giro y demás con los controles de Ajustar; nada pegado a los bordes. |

## TDD

`gafira-doc` (modelo y colocación), `gafira-raster` (recorte y formas), `gafira-ui` (tiradores y pasos).

## Evidencia

- Gates de 086 a 089 juntas: `cargo fmt --check`, `clippy -D warnings`, `cargo deny` y `cargo test --workspace --locked` en verde, 3636 tests. Fallan desde antes V14 y `forbid(unsafe_code)` en `gafira-photos`.

- Probado en la app (Linux, datos temporales): doble clic, tiradores, círculo, a mano, Intro, deshacer, miniaturas y sombras que siguen el recorte; Propiedades de imagen. Sin probar a mano: imagen girada o reflejada (con tests), Esc; ni Windows ni macOS.
- Marca y Plantillas pasan al final de la barra, apagadas y con «Próximamente».
- Tests: `s88_*` en `gafira-doc`, `gafira-raster` y `gafira-ui`.
