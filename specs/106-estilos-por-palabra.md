# 106 — Estilos por palabra dentro de un texto

> **Estado:** in-progress — 2026-11-26 — implementado y probado (S106-01 a 07); falta la revisión del titular en la app.

## Objetivo

Que una caja de texto pueda tener partes con otro estilo: seleccionar unas palabras mientras se escribe y ponerlas en negrita, en cursiva, en otro color, tamaño o fuente, sin cambiar el resto («Agregar **algo** de texto», con «algo» en rojo y negrita).

## Alcance / no alcance

Alcance:

- Modelo: aplicar un cambio de estilo a un rango de letras de `RichText` y conservar los estilos al escribir (`restyle`, `edited`, `style_at`, `char_count`).
- Dibujo: el motor de texto, el lienzo y las exportaciones (PNG, JPEG, PDF, SVG) dibujan cada tramo con su fuente, tamaño, color, negrita y cursiva; PowerPoint ya lo hace.
- Edición en el lienzo: el campo muestra los estilos; con letras seleccionadas, negrita, cursiva, color, tamaño y fuente actúan solo sobre ellas; sin selección, sobre todo el texto como hoy.
- Escribir conserva los estilos de lo que no se toca; lo nuevo toma el estilo de la letra anterior.

No alcance: subrayado y tachado, listas, estilos con nombre, interletrado o interlineado por tramo.

## Dependencias

- `RichText`/`Span`/`SpanStyle` (spec 027), motor `gafira-text`, renderizador `gafira-render`, exportadores, editor en el lienzo (specs 055, 061, 095), panel «Fuente» (spec 062), propiedades de texto (spec 070).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S106-01 | Estilo a un rango | Solo las letras del rango cambian; los tramos se cortan donde cambia el estilo |
| S106-02 | Rangos entre líneas y fusión | Un salto de línea cuenta como una letra; tramos iguales contiguos se unen |
| S106-03 | Escribir conserva estilos | Lo no tocado mantiene su estilo; lo escrito toma el de la letra anterior |
| S106-04 | Estilo en una posición | La barra puede mostrar el estilo de la selección |
| S106-05 | Dibujo por tramos | El lienzo y la exportación muestran cada tramo con su estilo |
| S106-06 | Edición por rango | Con letras seleccionadas, la barra cambia solo esas letras, en un paso de deshacer |

## TDD

Casos que deben fallar primero:

1. S106-01 a 04 en `crates/gafira-doc/tests/rich_range.rs` (hechos: fallaron con métodos vacíos y pasan).
2. La composición de un párrafo con dos tramos de tamaños y fuentes distintos da avances distintos para cada tramo.
3. Editar en el lienzo un texto con dos estilos no los aplana.

## Evidencia esperada

Pruebas `s106_*`, una exportación PNG con dos colores en el mismo texto y los gates de `AGENTS.md`.

## Riesgos y decisiones abiertas

- El motor de texto compone hoy un solo estilo por texto; componer por tramos toca `gafira-text` y `gafira-render`, la parte más grande del trabajo.

## Implementación (2026-11-26)

- Modelo: `RichText::{restyle, edited, style_at, char_count, mixed}` (S106-01 a 04).
- Motor: `gafira_text::layout_box_runs` con `FaceRun` (familia, negrita, cursiva y tamaño por rango sobre parley) (S106-05).
- Renderizador: un texto de varios estilos se compone por tramos y sus letras salen por color, con sus glifos, en operaciones `Text` separadas; las exportaciones PNG, JPEG, PDF y SVG las heredan. `text_parts` da las letras por color al lienzo (S106-06).
- Lienzo: un texto de varios estilos cuenta como `styled()` y se dibuja desde su contorno, cada color aparte.
- Editor: escribir conserva los estilos (`edited`); el campo muestra los estilos mientras se escribe. Con letras elegidas, los controles de siempre (Propiedades: tamaño, negrita, cursiva, color y degradado; panel «Fuente») actúan solo sobre ellas, y la escritura vuelve con las mismas letras elegidas para el siguiente cambio (S106-08).
- Degradado por tramo: `SpanStyle.gradient` (aditivo, solo se escribe si existe); el renderizador lo pinta sobre la caja como el degradado de todo el texto (S106-09, S106-10).
- Se quitó la barra flotante sobre el texto (pedido del titular: no funcionaba y el panel hace lo mismo).
- Fuera de esta entrega: elegir otra fuente para unas letras (el panel «Fuente» sigue cambiando todo el texto), subrayado y tachado.

