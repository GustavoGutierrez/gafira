# 070 — Propiedades, barra de la selección y panel Texto

> **Estado:** in-progress — 2026-11-21 — FD-41, FD-44, D-26, D-110. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Que el Inspector muestre las propiedades de lo seleccionado (o de la página) en una sola pestaña, que la selección tenga a mano sus acciones (agrupar, desagrupar, bloquear, duplicar, eliminar) y que el panel Texto añada texto curvo y combinaciones de fuentes.

## Alcance / no alcance

Alcance: pestañas «Propiedades» y «Capas» (se quitan «Posición» e «Imagen»); el título de Propiedades dice qué es el elemento; una imagen muestra su archivo y «Usar como fondo de la página»; la barra flotante sobre la selección; `TextNode.curve` (aditivo, esquema 1) con validación, `Property::TextCurve`, render y lienzo; el control «Curva»; alineación, negrita, cursiva, espaciado entre letras, interlineado y color del texto (sólido o degradado) en Propiedades; elegir lo bloqueado con un clic para desbloquearlo (D-110); las secciones «Texto curvo» y «Combinaciones de fuentes» del panel Texto, con clic y arrastre (D-105).

No alcance: listas con viñetas o numeradas en el panel Texto (el motor de texto las tiene, el modelo aún no); editar el texto curvo sobre su arco (se edita recto); varios estilos por texto.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S70-01 | Pestañas del Inspector. | «Propiedades» (la activa, subrayada) y «Capas». Propiedades muestra el fondo de la página sin selección y, con un elemento, su tipo (Texto, Imagen, Forma, Línea, Marco, Luz, Grupo) y «Posición y tamaño». |
| S70-02 | Imagen como fondo. | «Usar como fondo de la página» pone la imagen de fondo (`fill`) y la quita de la página en un paso de deshacer (`an_image_becomes_the_page_background`). |
| S70-03 | Barra de la selección. | Sobre la selección (o debajo si no cabe), oculta al arrastrar, escribir o recortar: «Agrupar» con varios elementos o «Desagrupar» con un grupo, Bloquear/Desbloquear, Duplicar, Eliminar y «···» (Copiar, Cortar). |
| S70-04 | Texto curvo. | `curve` en `[-100, 100]` (FD-44): positivo es un arco, negativo una copa, 0 recto y no se escribe. La exportación y el lienzo usan el mismo contorno (`text_outline`); en un arco la letra del medio queda en la línea y los extremos bajan, en una copa suben (`s70_04_curved_text_follows_its_arc`). «Curva» cambia el texto y la altura de su caja en un paso (`s70_04_curving_a_text_fits_its_box`). |
| S70-05 | Combinaciones de fuentes. | Catorce: Clásica, Moderna, Editorial, Divertida, Cartel, Elegante y, con las fuentes manuscritas añadidas a la biblioteca (Kaushan Script, Satisfy, Alex Brush, Dancing Script, Allura, Great Vibes, Merienda y Courgette, de google/fonts, OFL 1.1 salvo Satisfy, Apache 2.0), Bienvenida, Romántica, Invitación, Celebración, Firma, Informal, Cafetería y Menú; todas con fuentes de la biblioteca; cada una entra como un grupo seleccionado de un título y una línea de texto, en un paso (`a_font_pairing_is_added_as_a_group`, `a_pairing_is_a_group_of_two_texts`). |
| S70-06 | Estilo del texto. | Propiedades de un texto: tamaño, negrita, cursiva, alineación (izquierda, centro, derecha, justificado), espaciado entre letras (`[-100, 500]` milésimas de em), interlineado (`[0,5, 4]`), «Curva» y color: selector, colores del diseño, 28 sólidos y 14 degradados predeterminados. `TextNode.letter_spacing`, `line_height` y `gradient` son aditivos al esquema 1 y se validan. La exportación y el lienzo usan el mismo contorno; el degradado rellena las letras sobre la caja (`s70_06_spacing_line_height_and_gradient_of_a_text`). Cada cambio es un paso de deshacer; un color sólido quita el degradado (`s70_06_text_styles_from_the_inspector`). |
| S70-07 | Desbloquear desde el lienzo (D-110). | Un clic sobre algo bloqueado lo selecciona, o el grupo bloqueado de más arriba (`a_click_selects_what_is_locked`); la barra muestra solo el candado cerrado con «Desbloquear». |
| S70-08 | Escalar un grupo. | Una esquina de un grupo escala sus elementos y el tamaño de letra de cada texto, también el de las formas, por un mismo factor: una letra grande y una pequeña conservan su proporción. El factor se limita para que ninguna letra baje del mínimo ni pase del máximo, y el grupo toma ese factor. Un paso de deshacer (`a_group_corner_scales_its_letters_in_proportion`). |
