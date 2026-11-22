# 055 — Texto dentro de las formas y escribir en el lienzo

> **Estado:** in-progress — 2026-11-19 — pedida por el titular con una captura de referencia; D-96.

## Objetivo

Que cada forma pueda llevar un texto dentro, o quedar vacía, alineado a la izquierda, al centro o a la derecha y arriba, al medio o abajo (D-96). Que todo texto, el de un elemento de texto y el de una forma, se escriba directamente sobre el lienzo con un doble clic, sin ir al panel.

## Alcance / no alcance

Alcance:

- `gafira-doc`: `ShapeNode.text: Option<ShapeText>` con `content: RichText`, `align: TextAlign` y `valign: VerticalAlign` (`top`, `middle`, `bottom`; por defecto centrado y al medio). Se escribe solo cuando existe, así que una forma sin texto conserva sus bytes. La validación de los tamaños y colores de sus tramos es la del texto.
- `gafira-edit`: `Property::ShapeText(Option<ShapeText>)`.
- `gafira-render`: el texto de una forma se dibuja sobre su relleno y su contorno, dentro de la caja menos un margen de un décimo del lado corto, con su alineación horizontal y vertical, y entra en la clave de la forma.
- `gafira-ui`:
  - doble clic en un texto o en una forma abre un campo de edición sobre el elemento en el lienzo, con su tamaño de letra, su color y su alineación; Esc o un clic fuera lo cierra; todo lo escrito en una edición es un paso de deshacer;
  - una forma sin texto empieza con el estilo por defecto (Inter, 24 px, la tinta de la marca);
  - borrar todo el texto de una forma la deja vacía (`None`);
  - en el Inspector, una forma muestra la alineación horizontal y vertical de su texto.

No alcance: negrita y cursiva (otra spec, necesita más pesos de fuente), estilos por tramo, editar dentro de un elemento girado en sus propios ejes (el campo se abre en la caja alineada), ajuste automático del tamaño de la forma al texto.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S55-01 | Modelo (D-96). | Una forma con texto se valida, se guarda y se carga igual; una sin texto escribe el mismo JSON que antes; un tamaño de letra fuera de rango en su texto da error de validación. |
| S55-02 | Comando. | `Property::ShapeText` pone, cambia y quita el texto de una forma en un paso de deshacer, y no aplica a otro tipo de elemento. |
| S55-03 | Render. | El texto de una forma queda dentro de la caja con su margen; arriba, al medio y abajo lo colocan donde dice; cambiar el texto cambia la clave. |
| S55-04 | Escribir en el lienzo. | Doble clic en un texto o en una forma abre la edición; lo escrito llega al documento; cerrar la edición es un paso de deshacer con todo lo escrito; vaciar una forma deja su texto en `None`. |
| S55-05 | Alineación en el Inspector. | Con una forma seleccionada, seis botones ponen la alineación horizontal y vertical de su texto. |

## TDD

1. `s55_01_shape_text_round_trips`, `s55_01_shape_without_text_is_unchanged`, `s55_01_shape_text_validation` (`gafira-doc`).
2. `s55_02_shape_text_is_set_and_reverted` (`gafira-edit`).
3. `s55_03_shape_text_sits_where_it_is_aligned` (`gafira-render`).
4. `s55_04_text_offset` (posición vertical en el lienzo) y `s55_04_editing_writes_one_undo_step` (`gafira-ui`).

## Evidencia esperada

Tests en verde, gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Una versión anterior de Gafira no abre un `.gaf` con texto dentro de una forma (D-96).
