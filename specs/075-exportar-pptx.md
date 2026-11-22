# 075 — Exportar a PowerPoint (PPTX)

> **Estado:** in-progress — 2026-11-21. Implementada y probada; pendiente de la prueba del titular.

## Objetivo

Exportar el diseño a una presentación PPTX editable en PowerPoint, Keynote o LibreOffice: una diapositiva por página, con formas, textos, imágenes, marcos, líneas y grupos que siguen siendo elementos editables.

## Origen

El titular dio como referencia DeckCraft (MIT o Apache 2.0). Su escritor de PPTX vive en `craft-libs`, que DeckCraft trae de un repositorio git que no está en este equipo y que la política de `cargo deny` no admite (`unknown-git = "deny"`). El adaptador se escribió desde el estándar público ECMA-376 (PresentationML y DrawingML), sin copiar código de DeckCraft.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S75-01 | Paquete. | Adaptador `gafira-io-pptx`: un `presentation.pptx` (OPC en ZIP) con tipos de contenido, relaciones, presentación, patrón, diseño en blanco, tema, propiedades y una diapositiva por página; cada parte XML está bien formada; LibreOffice lo abre y lo convierte (`s75_01_an_editable_presentation`). El tamaño de diapositiva es el de la primera página (de 1 a 56 pulgadas). |
| S75-02 | Elementos. | Formas con su geometría (rectángulo, redondeado, elipse o trazado propio), relleno, degradado lineal o radial, contorno, giro, volteo y opacidad, y su texto interior; textos como cuadros de texto con fuente, tamaño, color, negrita, cursiva, alineación, espaciado, interlineado y degradado, y el texto curvo con el arco de PowerPoint; imágenes con su archivo (PNG o JPEG tal cual; otros, como PNG); marcos como imagen recortada por su forma con el encuadre de la foto; líneas como conectores con sus puntas; grupos como grupos; el fondo de la página (color, degradado, imagen o el patrón de PowerPoint más parecido). |
| S75-03 | Límites. | Las fuentes no se incrustan: la diapositiva las nombra y el equipo que la abre debe tenerlas. Las luces y los elementos de extensiones no tienen forma en PowerPoint: se dejan fuera y el informe lo avisa. |
