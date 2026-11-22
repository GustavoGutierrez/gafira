# 074 — Exportar SVG, recordar la configuración y calidad de imagen

> **Estado:** in-progress — 2026-11-21. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Exportar a SVG con vectores y texto real; recordar la configuración de exportación de cada formato; dar a PNG, JPEG y WebP una calidad elegible y una escala que se arrastra.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S74-01 | SVG. | Adaptador `gafira-io-svg`: un archivo por página (`page.svg` o `page-N.svg`) desde las operaciones de la página (spec 073): trazados con su transformación, degradados lineales y radiales, imágenes como PNG, recortes, capas con opacidad y fusión, y el texto como texto real con su fuente incrustada (`@font-face`); un lector SVG lo abre (`s74_01_the_page_as_vectors_and_text`). Una página con máscara o efecto se incrusta como imagen. |
| S74-02 | Opciones del SVG. | «Fondo transparente» quita el blanco de la página; «Texto como trazados» dibuja el texto como formas (`s74_02_outlines_and_transparency`). |
| S74-03 | Recordar la configuración. | Interruptor «Recordar esta configuración» en el diálogo: al exportar, guarda las opciones del formato en `export-presets.json` (nunca la contraseña) o las olvida; al elegir ese formato, el diálogo sale con ellas y el interruptor activado; un valor que el formato ya no acepta se ignora (`s74_03_the_settings_of_a_format_are_kept`). |
| S74-04 | Calidad. | PNG: «Alta» o «Comprimir» (sin pérdida; la segunda con la compresión más fuerte). JPEG: Baja, Media o Alta (60, 80, 92). WebP: Baja, Media o Alta; el codificador solo escribe sin pérdida, así que Baja y Media guardan 5 y 6 bits por canal de color para reducir el archivo, y Alta es exacta (`fewer_bits_round_to_their_levels`, `s39_01_the_jpeg_quality_is_checked_and_used`). Cada calidad explica su efecto en su «i». |
| S74-05 | Escala. | La escala es un deslizador de pasos de 0,25× en su rango (0,5× a 4× en imágenes; 1× a 4× en el PDF aplanado). Sustituye a los pasos 1×–4× de S39-04. |
