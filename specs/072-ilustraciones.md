# 072 — Ilustraciones

> **Estado:** in-progress — 2026-11-21 — D-111. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Que Elementos → Ilustraciones ofrezca ilustraciones vectoriales, a color y en blanco y negro, que se pueden usar libremente (también con fines comerciales y sin citar al autor en el diseño), en filas por tema como la referencia del titular, y que se inserten como formas de Gafira editables.

## Alcance / no alcance

Alcance: el puerto `IllustrationCatalog` (`gafira-ports::photos`), el adaptador de Iconify en `gafira-photos` (sin clave), el panel con búsqueda, estilo (color, blanco y negro o ambos), filas por tema y por colección, «Ver todo», clic y arrastre (D-105), e inserción como grupo de formas vectoriales.

No alcance: ilustraciones realistas de estilo «stock» (las libres de derechos que se encontraron, recortes de dominio público de rawpixel en Openverse, dependen de un servicio que falla a menudo); colecciones con licencia que exige atribución (CC BY, CC BY-SA: Twemoji, Streamline, OpenMoji, Emoji One) ni logotipos de marcas (aunque el archivo sea CC0, la marca está protegida); iconos de interfaz (van en Iconos); favoritos.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S72-01 | Colecciones libres. | Solo Fluent Emoji Flat, Fluent Emoji y Fluent Emoji High Contrast (Microsoft, MIT), Noto Emoji (Google, Apache 2.0), Firefox OS Emoji (Mozilla, Apache 2.0), Flat Color Icons (Icons8, MIT), Meteocons (MIT) y Health Icons (MIT), cada una marcada a color o en blanco y negro. Una respuesta con otra colección o un nombre que no sea de letras minúsculas, cifras y guiones se descarta, y la dirección del SVG no se puede desviar (`only_our_collections_and_plain_names`). |
| S72-02 | Estilo. | «Color y blanco y negro», «A color» o «Blanco y negro»: la búsqueda pide solo las colecciones de ese estilo y las filas de una colección se ocultan si no lo son. Los dibujos en blanco y negro se ven sobre un fondo claro también en el tema oscuro. |
| S72-03 | Panel. | Búsqueda (espera de 400 ms), filas por tema (Flores, Hojas, Árboles, Corazones, Estrellas, Clima, Gatos, Perros, Pasteles y dulces, Música, Libros, Coches, Oficina y objetos, Dibujos de línea, Salud y personas) que se piden al entrar en vista, «Ver todo» con flecha para volver, miniaturas de los SVG (8 a la vez) y el aviso de que se usan libremente, con las colecciones y sus licencias (`a_row_and_its_drawings_arrive`). |
| S72-04 | Inserción. | Un clic o un arrastre inserta la ilustración como un grupo de formas vectoriales (colores y degradados editables), al 30 % del lado corto de la página, con su nombre, colección, autor y licencia como nombre del grupo, en un paso de deshacer (`an_illustration_is_a_group_of_shapes`). |
| S72-05 | Red. | Iconify (`api.iconify.design`) por el mismo transporte de D-111; un 429 en una búsqueda deja en pausa solo Iconify; 2 MB por SVG. Comprobado en vivo (`live_illustrations`, ignorada). |
| S72-06 | Open Peeps y Open Doodles. | Los 93 personajes de Open Peeps y los 33 dibujos de Open Doodles (Pablo Stanley, CC0 1.0), tal como se publican en sus webs, van incluidos en Gafira (`assets/illustrations/`, con su licencia y una entrada en `LICENSES.toml` por archivo) y se ven y se insertan sin red. Son las dos primeras filas; una búsqueda los encuentra por su nombre o por «persona», «gente», «people», y van antes que los de Iconify; sin red siguen saliendo (`bundled_drawings_are_found_offline`). Las miniaturas se dibujan a 160 px. |
| S72-07 | Ilustraciones detalladas. | Una sección de recortes de dominio público (CC0) de rawpixel con fondo transparente, a través de Openverse (`PhotoQuery.cutouts`: `source=rawpixel` y «png sticker»), en filas por tema (helechos, hojas tropicales, árboles, flores, rocas, setas, aves, mariposas), oculta con «Blanco y negro». Un clic o un arrastre la inserta como imagen por las descargas de Fotos (también en un marco). Openverse falla a veces; la fila lo dice (`a_cutout_row_asks_for_free_cutouts`). |
| S72-08 | Proporciones. | Una miniatura conserva sus proporciones en su casilla (`a_thumbnail_keeps_its_proportions`; fallo del titular: Open Peeps se veían deformados). |

