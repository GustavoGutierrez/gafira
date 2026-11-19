# 062 — Biblioteca de fuentes y panel «Fuente»

> **Estado:** in-progress — 2026-11-19 — pedida por el titular con una captura de referencia; D-100.

## Objetivo

Que un texto (una caja de texto o el texto de una forma) pueda cambiar de fuente entre doce familias incrustadas, con negrita y cursiva, desde un panel «Fuente» como el de la referencia: buscador, fuentes del documento, familias con su muestra y sus variantes desplegables, y la actual marcada (D-100).

## Alcance / no alcance

Alcance:

- Recursos: doce familias OFL de google/fonts (commit `cc5661d4`), en `assets/fonts/library/`: Open Sans, Montserrat, Poppins, DM Sans, Roboto, Lato, Inter, Manrope, EB Garamond, Source Serif 4, Anton y Pacifico, en normal, negrita, cursiva y negrita cursiva cuando la familia las tiene. Playfair Display y Merriweather, de la propuesta, se cambiaron por EB Garamond y Source Serif 4: declaran un nombre reservado y solo se publican variables, así que instanciarlas obligaría a renombrarlas.
- `gafira-doc`: `SpanStyle` gana `bold` e `italic` (escritos solo cuando son verdaderos, así que un texto sin ellos conserva sus bytes).
- `gafira-theme`: el catálogo de la biblioteca (familia, variante, bytes).
- Render y exportación: la cara de cada estilo es la variante de su familia; si la familia no tiene esa variante, la más cercana (sin cursiva, sin negrita).
- `gafira-ui`: el lienzo dibuja cada texto con su familia y variante; botón «Fuente» en el Inspector de un texto o de una forma con texto que abre el panel «Fuente» en el panel lateral; elegir una familia o una variante la aplica a todo el texto en un paso de deshacer.

No alcance: aplicar una fuente o negrita a una parte del texto (spec 063), fuentes del sistema, subir fuentes, más pesos que normal y negrita.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S62-01 | Modelo (D-100). | Un tramo en negrita y cursiva se guarda y se carga igual; uno sin ellas escribe el mismo JSON que antes. |
| S62-02 | Catálogo. | Las doce familias con sus variantes; cada archivo es el de disco; buscar la cara de una familia y un estilo da su variante o la más cercana. |
| S62-03 | Render. | Un texto en negrita usa la cara negrita de su familia en la exportación. |
| S62-04 | Cambiar la fuente. | Elegir una familia o una variante cambia la fuente de todos los tramos del texto (caja o forma) en un paso de deshacer. |

## TDD

1. `s62_01_bold_italic_round_trip` (`gafira-doc`).
2. `s62_02_library_faces` (`gafira-theme`).
3. `s62_03_bold_uses_the_bold_face` (`gafira-render`).
4. `s62_04_the_font_changes_in_one_step` (`gafira-ui`).

## Evidencia esperada

Tests en verde, gates y prueba del titular.

## Riesgos y decisiones abiertas

- La app crece unos 12 MB con las fuentes.
