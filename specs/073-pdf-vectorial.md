# 073 — PDF vectorial, comprimir, aplanar, contraseña y perfil de color

> **Estado:** in-progress — 2026-11-21 — D-112. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Que el PDF guarde los vectores y el texto como texto real, con su fuente incrustada, en vez de rasterizar cada página, y que el diálogo de exportación ofrezca comprimir las imágenes, aplanar el PDF, protegerlo con contraseña y elegir el perfil de color, con un diálogo cómodo de usar.

## Alcance / no alcance

Alcance: `RenderOp::Text` (contorno para los backends de píxeles y tiras de glifos con su fuente y sus palabras para el PDF); `PageRenderer::vectors` (las operaciones de una página a 1×); el escritor vectorial de `gafira-io-pdf` (krilla: trazados, degradados lineales y radiales, imágenes, recortes, capas con opacidad y modo de fusión, texto con subconjunto de fuente incrustado); las opciones del PDF; el tipo de opción `ParamKind::Text`; el diálogo de exportación rediseñado.

No alcance: perfil ICC de imprenta (la conversión a CMYK es directa); texto curvo y texto con degradado como texto (salen como contornos vectoriales); máscaras, efectos y transiciones como vectores (esa página sale como imagen, con aviso); sangrado y marcas de corte.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S73-01 | Vectores y texto real. | Por defecto cada página se escribe con vectores; el texto recto es texto real con su fuente incrustada (subconjunto) y su mapa a Unicode: `pdftotext` lo lee y `pdffonts` muestra la fuente incrustada. El texto curvo o con degradado sale como contorno vectorial (`s73_01_text_stays_text`, `s73_01_a_text_carries_its_glyph_runs`). |
| S73-02 | Aplanar PDF. | Con «Aplanar PDF» cada página es una imagen, como antes (spec 030), a la resolución elegida, que solo se muestra entonces (`s73_02_flatten_draws_pixels`). Una página con máscara, efecto o transición se aplana sola y el informe lo avisa. |
| S73-03 | Contraseña. | «Proteger con contraseña» muestra un campo oculto; el PDF se cifra con AES-256 (lopdf, D-112) y solo se abre con esa contraseña (`s73_03_a_password_protects_the_pdf`). |
| S73-04 | Perfil de color. | «Perfil de color»: RGB (más apropiado para uso digital) o CMYK (mejor para impresión profesional); en CMYK los colores y degradados se escriben en CMYK por la regla directa, sin perfil ICC (`s73_04_cmyk_colours`, `cmyk_by_the_simple_rule`). |
| S73-05 | Comprimir PDF. | Con «Comprimir PDF» las imágenes opacas se guardan como JPEG (calidad 78) no mayores que el doble del tamaño al que se muestran; su icono «i» dice «Comprime las imágenes para reducir el tamaño del archivo. Esto podría reducir la calidad de las imágenes». |
| S73-06 | Diálogo. | Margen interior de 24 px; formato, páginas, perfil de color y resolución en listas desplegables; las opciones de sí o no como interruptores, con su «i» cuando tienen explicación; rejilla de dos columnas con 14 px entre filas; «Exportar» destacado y «Cancelar» abajo a la derecha. El selector de archivo del sistema se abre sobre la ventana de Gafira (`set_parent`), no detrás. |
| S73-07 | Progreso. | La exportación sigue en su hilo, sin bloquear la ventana; el PDF informa del avance página a página y se puede cancelar entre páginas. Escribir vectores evita dibujar cada página en píxeles, así que es más rápido que el PDF aplanado. |
