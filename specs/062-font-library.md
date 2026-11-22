# 062 — Biblioteca de fuentes y panel «Fuente»

> **Estado:** in-progress — 2026-11-19 — pedida por el titular con una captura de referencia; D-100.

## Objetivo

Que un texto (una caja de texto o el texto de una forma) pueda cambiar de fuente entre doce familias incrustadas, con negrita y cursiva, desde un panel «Fuente» como el de la referencia: buscador, fuentes del documento, familias con su muestra y sus variantes desplegables, y la actual marcada (D-100).

## Alcance / no alcance

Alcance:

- Recursos: veinticinco familias libres de google/fonts (commit `cc5661d4`), en `assets/fonts/library/`: Open Sans, Montserrat, Poppins, DM Sans, Roboto, Lato, Inter, Manrope, EB Garamond, Source Serif 4, Anton, Pacifico y, en una segunda tanda de la lista del titular, Arimo, League Spartan, Archivo Black, Questrial, Bebas Neue, Alice, Bree Serif, Shrikhand, Chau Philomene One, Belleza, Cormorant Garamond, Chewy (Apache 2.0) y DM Serif Display, en normal, negrita, cursiva y negrita cursiva cuando la familia las tiene. De esa lista no entran las comerciales o de uso restringido (Canva Sans, Garet, Gagalin, Montaser Arabic, Codec Pro, More Sugar, Brittany, The Seasons, Marykate, Arial MT Pro, Bukhari Script, Intro Rust, Montaine) ni las libres que solo se publican variables con nombre reservado (Raleway, Quicksand, Lora, Libre Baskerville, Playfair Display, Merriweather). Playfair Display y Merriweather, de la propuesta, se cambiaron por EB Garamond y Source Serif 4: declaran un nombre reservado y solo se publican variables, así que instanciarlas obligaría a renombrarlas. Una tercera tanda (2026-11-21, spec 070) añade las manuscritas Kaushan Script, Satisfy (Apache 2.0), Alex Brush, Dancing Script, Allura, Great Vibes, Merienda y Courgette; Dancing Script y Merienda entran con su archivo variable sin modificar (solo cambia el nombre del archivo a `-Regular.ttf`) y se usan en su peso por defecto, 400, así que el nombre reservado de Dancing Script no se toca.
- `gafira-doc`: `SpanStyle` gana `bold` e `italic` (escritos solo cuando son verdaderos, así que un texto sin ellos conserva sus bytes).
- `gafira-theme`: el catálogo de la biblioteca (familia, variante, bytes).
- Render y exportación: la cara de cada estilo es la variante de su familia; si la familia no tiene esa variante, la más cercana (sin cursiva, sin negrita).
- `gafira-ui`: el lienzo dibuja cada texto con su familia y variante; botón «Fuente» en el Inspector de un texto o de una forma con texto que abre el panel «Fuente» en el panel lateral; elegir una familia o una variante la aplica a todo el texto en un paso de deshacer.

- Fuentes del sistema (pedido del titular): `gafira-platform` busca en segundo plano los `.ttf` y `.otf` de las carpetas de fuentes del sistema y del usuario (no las colecciones `.ttc`), una cara por familia, peso y estilo; el panel las lista en su propia sección, por nombre; el lienzo instala en egui solo las familias que el documento usa, y la exportación lee una cara del sistema solo cuando un texto la pide (`FontBook::with_loader`). Las fuentes del sistema que no son libres son responsabilidad del usuario.
- El panel es una lista de filas de igual alto de la que solo se dibujan las visibles.

No alcance: aplicar una fuente o negrita a una parte del texto (spec 063), subir fuentes, más pesos que normal y negrita, colecciones `.ttc`.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S62-01 | Modelo (D-100). | Un tramo en negrita y cursiva se guarda y se carga igual; uno sin ellas escribe el mismo JSON que antes. |
| S62-02 | Catálogo. | Las doce familias con sus variantes; cada archivo es el de disco; buscar la cara de una familia y un estilo da su variante o la más cercana. |
| S62-03 | Render. | Un texto en negrita usa la cara negrita de su familia en la exportación. |
| S62-05 | Fuentes del sistema. | La búsqueda da la familia y la cara de cada archivo; el catálogo excluye las familias de Gafira; el render carga una cara del sistema una sola vez, al pedirla. |
| S62-06 | Panel. | Secciones del documento, de Gafira y del sistema (o «Buscando…»), con caras al abrir una familia y búsqueda por nombre. |
| S62-04 | Cambiar la fuente. | Elegir una familia o una variante cambia la fuente de todos los tramos del texto (caja o forma) en un paso de deshacer. |

## TDD

1. `s62_01_bold_italic_round_trip` (`gafira-doc`).
2. `s62_02_library_faces` (`gafira-theme`).
3. `s62_03_bold_uses_the_bold_face` (`gafira-render`).
4. `s62_04_the_font_changes_in_one_step` (`gafira-ui`).

## Evidencia esperada

Tests en verde, gates y prueba del titular.

## Riesgos y decisiones abiertas

- La app crece unos 17 MB con las fuentes.
- Dos lapsos de TDD: `s62_01` se escribió junto con los campos, sin verlo fallar; y un commit se hizo con clippy fallando (dos tests leían archivos en crates de dominio), corregido antes de publicar.
