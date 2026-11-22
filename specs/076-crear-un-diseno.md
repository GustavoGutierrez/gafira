# 076 — Crear un diseño: tamaños por categoría, búsqueda y guías

> **Estado:** in-progress — 2026-11-21. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Que Inicio sea «Crear un diseño»: encontrar rápido el tamaño de lienzo, predefinido o propio, por categoría, por grupo o por búsqueda, verlo dibujado en su proporción, y empezar con una página que ya trae las guías de su formato (pliegues, márgenes de seguridad, zona segura).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S76-01 | Categorías. | Lista lateral con icono propio en color: Para ti, Redes sociales, Presentaciones, Videos, Imprimir, Documentos, Fotos, Web y correo, Elegir el tamaño, y «Abrir archivo». «Para ti» muestra Más usados, Popular en Gafira y Prueba algo distinto en filas, y Recientes. |
| S76-02 | Catálogo. | 56 tamaños reales (`design_sizes`), cada uno con nombre en español e inglés, palabras de búsqueda, medida y unidad («21 × 29,7 cm», «8,5 × 11 in», «1080 × 1080 px»); cada uno es una página válida (`every_size_is_a_page`, `every_name_is_written`). |
| S76-03 | Filtros y búsqueda. | Redes sociales se filtra por plataforma, e Imprimir por Marketing, Tarjetas e invitaciones, Documentos y papelería, Para regalar, y Etiquetas y stickers; ningún filtro queda vacío. La búsqueda encuentra por palabras en los dos idiomas (`sizes_are_found_and_filtered`). |
| S76-04 | Miniaturas. | Cada tamaño se dibuja en SVG con los colores de Gafira y en su proporción: la hojita con la esquina doblada y la flor, tarjeta, afiche, invitación, tríptico, postal, etiqueta, volante, infografía, certificado, diapositiva, móvil, post, video, web, correo y foto. Se rasterizan una vez con resvg (`every_picture_draws`). La tarjeta se ilumina al pasar el ratón y tiene la mano. |
| S76-05 | Tamaño propio. | Ancho, alto, unidades (px, in, mm, cm), candado de proporción y «Crear un diseño», todos con la altura del botón y en una línea. Cada lado va de 40 px a 8.000 px; fuera de eso, en rojo: «El tamaño no puede ser menor a 40 px ni mayor a 8.000 px.», con los límites en la unidad elegida (0,42 a 83,33 in; 10,59 a 2.116,66 mm; 1,06 a 211,66 cm) (`sides_are_bounded_in_every_unit`). Debajo, Formatos populares. |
| S76-06 | Guías del formato. | La página nueva trae sus guías: pliegues bloqueados y discontinuos (tríptico en tercios, tarjeta plegable, talón del certificado), márgenes de seguridad de impresión, zona segura de historias y videos, área visible del banner de YouTube, columna de 1200 px del sitio web y tercios de las fotos (`sizes_start_with_their_guides`). Una guía bloqueada se dibuja discontinua. |
| S76-08 | Reglas en la unidad de la página. | Las reglas miden en la unidad del tamaño de la página (px, in, mm, cm, pt), con pasos de décimas en pulgadas y centímetros al acercar (`s76_rulers_count_in_the_page_unit`). |
| S76-09 | Guías al arrastrar. | Mientras se arrastra una guía, una etiqueta sobre la regla dice su posición en la unidad de la página («1.666» in, «120» px, «10.2» mm) y cambia con el puntero (`a_dragged_guide_tells_its_position`). Soltarla sobre su regla o más allá (por encima de la regla superior, a la izquierda de la izquierda) la borra del documento, en la página donde estaba (`a_guide_past_its_ruler_is_deleted`). |
| S76-07 | Atajos. | Ctrl+N crea un post cuadrado; Ctrl+O abre un archivo. |
