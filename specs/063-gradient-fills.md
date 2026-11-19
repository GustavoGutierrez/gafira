# 063 — Relleno de color sólido o degradado

> **Estado:** in-progress — 2026-11-19 — pedida por el titular con capturas de referencia; D-101.

## Objetivo

Que el relleno de una forma sea un color sólido o un degradado de dos o más colores, lineal en cualquier dirección (izquierda a derecha, derecha a izquierda, arriba a abajo, abajo a arriba, diagonal) o circular, elegido en el Inspector con dos pestañas, «Color sólido» y «Degradado» (D-101).

## Alcance / no alcance

Alcance:

- `gafira-doc`: `ShapeNode.gradient: Option<Gradient>` (escrito solo cuando existe; si existe, rellena en lugar de `fill`). `Gradient { kind, stops }`: `Linear { angle_deg }` (0° de izquierda a derecha, 90° de arriba a abajo, 180° de derecha a izquierda, 270° de abajo a arriba) o `Radial` (desde el centro de la caja hasta su esquina más lejana); de 2 a 16 paradas con posición de 0 a 1 en orden y su color. `Gradient::color_at` da el color en un punto de la caja, interpolando en sRGB. Validación: número de paradas, posiciones finitas, en orden y en 0–1, colores y ángulo finitos.
- `gafira-edit`: `Property::Gradient(Option<Gradient>)`.
- `gafira-ports`: `RenderOp::FillGradient { path, gradient, size, transform }`.
- `gafira-render`: una forma con degradado emite `FillGradient` en su caja; el degradado entra en la clave.
- `gafira-render-cpu`: rellena la cobertura del trazado con el color de `color_at` en el centro de cada píxel, llevado a la caja de la forma.
- `gafira-ui`: el lienzo dibuja la forma con degradado por la ruta de textura de los trazados (tiny-skia, la misma geometría); el Inspector muestra «Color sólido» y «Degradado»; en «Degradado», los colores (cada uno con su selector), «+» para añadir uno y cinco estilos: lineal izquierda a derecha, arriba a abajo, diagonal, circular y derecha a izquierda; volver a «Color sólido» quita el degradado.

No alcance: degradado en contornos, en textos o en el fondo de la página; mover las paradas a mano; ángulos libres en la interfaz.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S63-01 | Modelo (D-101). | Una forma con degradado se guarda y se carga igual; una sin él escribe el mismo JSON que antes; menos de 2 o más de 16 paradas, posiciones fuera de orden o de 0–1, o un valor no finito dan error. |
| S63-02 | Color en un punto. | Lineal a 0°: rojo a la izquierda, azul a la derecha, la mezcla en el medio; a 90°, de arriba a abajo; a 180°, al revés que a 0°; circular: el primer color en el centro, el último en las esquinas. |
| S63-03 | Render. | Una forma con degradado exporta los colores de sus extremos donde corresponden. |
| S63-05 | Transparencia (pedido del titular). | El selector de color conserva la transparencia elegida, así que un degradado puede ir de un color a transparente; elegido sobre «sin color», un color transparente queda opaco. Vale para relleno, colores del degradado, contorno y color del texto. |
| S63-04 | Inspector. | Elegir un estilo o un color pone el degradado en un paso de deshacer; «Color sólido» lo quita. |

## TDD

1. `s63_01_gradient_round_trip`, `s63_01_gradient_validation`, `s63_02_color_at` (`gafira-doc`).
2. `s63_03_a_gradient_fills_its_shape` (`gafira-render-cpu` o `gafira-render`).
3. `s63_04_the_gradient_is_set_and_removed` (`gafira-ui`).

## Evidencia esperada

Tests en verde, gates y prueba del titular.

## Avance

- Hecho: modelo y validación, propiedad de edición, `RenderOp::FillGradient` y su pintor en el backend CPU, el grafo y la clave, el lienzo (textura con el shader de tiny-skia, también para rectángulos con esquinas y elipses) y el Inspector con «Color sólido» / «Degradado», colores (clic derecho quita uno), «+» y siete estilos: izquierda a derecha, derecha a izquierda, arriba abajo, abajo arriba, dos diagonales y circular.
- Lapso de TDD: el pintor del backend CPU y la propiedad de edición se escribieron antes que sus tests; se compensó con mutantes a mano (sin el inverso de la transformación, con `x` fija y sin la validación), que los tests detectan.
- Falta: la prueba del titular.

## Riesgos y decisiones abiertas

- Una versión anterior de Gafira no abre un `.gaf` con degradados (D-101).
