# 067 — Importar desde el sistema: arrastrar y pegar imágenes y formas

> **Estado:** in-progress — 2026-11-21 — pedida por el titular. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Que una imagen o una forma vectorial que esté fuera de Gafira entre en el lienzo sin pasar por un diálogo: soltándola desde el gestor de archivos o pegándola con Ctrl+V.

## Alcance / no alcance

Alcance: soltar en el lienzo archivos PNG o JPEG (se importan como imágenes) y SVG o PDF (crean un marco con su contorno, spec 066 S66-10); pegar con Ctrl+V una imagen del portapapeles del sistema o archivos copiados en el gestor de archivos (sus rutas o URI `file://`).

No alcance: soltar URLs o imágenes desde un navegador sin archivo; otros formatos (WebP, GIF, TIFF); pegar imágenes en Wayland (`arboard` sin la característica `wayland-data-control`, la misma configuración que egui-winit).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S67-01 | Soltar imágenes. | Un PNG o JPEG soltado en el lienzo se importa (`AssetService::import`) y queda centrado donde se suelta, seleccionado; varios, uno tras otro desplazados 24 px. Soltado sobre un marco, la primera imagen pasa a ser su foto (spec 066). |
| S67-02 | Soltar formas. | Un SVG o PDF soltado crea un marco vacío con su contorno y sus proporciones, centrado donde se suelta (S66-10). |
| S67-03 | Pegar una imagen. | Con una imagen en el portapapeles del sistema y sin texto, Ctrl+V la guarda como PNG (`AssetService::import_bytes`) y la pone en el centro de la página. La ventana no avisa de Ctrl+V cuando no hay texto; se detecta al soltar la V. |
| S67-05 | Un SVG pregunta cómo entra. | Todo SVG importado (diálogo, soltar, pegar o Biblioteca) abre «¿Cómo quieres importar el SVG?» —una sola pregunta para varios—: «Tal cual (vectorial)» lo trae como un grupo de formas de Gafira, una por trazado, con su relleno (color, degradado lineal o radial) y su contorno, a su tamaño o reducido al 80 % del lado corto de la página, centrado donde se suelta; «Como marco» crea un marco de su contorno (S66-10); «Cancelar» no importa nada. Un SVG sin trazados entra como imagen rasterizada al doble de su tamaño. El texto y las imágenes incrustadas del SVG no se traen como formas. |
| S67-06 | Colores del vector. | Con un grupo seleccionado (como un SVG traído tal cual), el Inspector muestra «Colores»: una muestra por cada color distinto de sus formas (rellenos, contornos y paradas de degradado). Cambiar una cambia ese color en todas las formas del grupo, en un paso de deshacer. |
| S67-04 | Pegar archivos. | Un texto pegado que es una lista de rutas o URI `file://` de imágenes o formas existentes se importa como en S67-01/S67-02 en lugar de crear una caja de texto; cualquier otro texto sigue creando una caja (spec 061). |

## Tests

- `gafira-engine`: `s67_import_bytes_stores_them_under_a_name`.
- `gafira-io-vector`: `s67_an_svg_comes_in_as_vector_shapes`, `s67_an_svg_is_drawn_in_colour`.
- `gafira-ui`: `s67_the_colours_of_a_group`, `s67_a_colour_changes_everywhere_in_the_group`, `s67_an_svg_as_it_is_is_a_group_of_shapes` (también el cambio de color), `s67_pasted_paths_of_images_are_images`, `s67_a_dropped_image_lands_at_the_point_or_in_the_frame`, `s66_10_a_dropped_svg_is_a_custom_frame`.

## Evidencia esperada

Tests en verde; prueba del titular soltando y pegando desde su gestor de archivos y su navegador.

## Riesgos

- Ctrl+V de una imagen depende de detectar la tecla al soltarse: si otra aplicación cambia el portapapeles entre medias, se pega lo que haya entonces.
- Wayland: pegar imágenes no está disponible con la configuración actual de `arboard`.
