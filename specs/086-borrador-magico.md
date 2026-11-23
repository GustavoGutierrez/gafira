# 086 — Borrador mágico y quitar marca de agua

> **Estado:** in-progress — 2026-11-22 — implementada y probada en Linux; pendiente de la prueba del titular. Pedida por el titular con capturas de referencia.

## Objetivo

Con una imagen seleccionada, borrar lo que se elija (un objeto, un texto, un logotipo, una marca de agua) y reconstruir el fondo, con modelos locales y sin destruir la imagen.

## Alcance / no alcance

Alcance: página «Borrador mágico» de «Editar imagen» (Área, Elemento, Primer plano), «Quitar marca de agua» con la misma página, y los modelos con la descarga, la recomendación y la medición de la spec 084.

No alcance: encontrar marcas de agua por sí solo, marcas invisibles o de procedencia (no se tocan ni se buscan), rellenar con un texto escrito, vídeo.

## Dependencias

D-117 (esta spec); D-115, specs 084 (modelos, original) y 085 (zonas, pincel). Sin dependencias de código nuevas.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S86-01 | Puerto. | `Inpainter::fill(imagen, máscara)`; otro modelo entra sin tocar el motor ni la interfaz. |
| S86-02 | Relleno. | Fuera de la máscara y su borde suave, píxeles idénticos; conserva tamaño y opacidad. La zona se trabaja a su resolución, por piezas con contexto; un hueco grande, reducido. |
| S86-03 | Modelos. | MI-GAN (28 MB, MIT) ligero y LaMa (Apache-2.0) de calidad, descargables, con BLAKE3 y URL fija; se recomienda según el equipo y se usa el motor más rápido medido. |
| S86-04 | Página. | Área con pincel, detección automática, Elemento y Primer plano; «Borrar», «Desmarcar», Comparar y «Deshacer los cambios»; sin modelo, ofrece descargarlo ahí mismo. La marca se ve en el menta de Gafira y la acción en coral. |
| S86-05 | No destructivo. | Cada borrado es un paso de deshacer; el original se conserva; los ajustes y el fondo quitado siguen puestos. |
| S86-06 | Trabajo. | En otro hilo, con progreso y el escáner de la spec 084; se puede cancelar. |
| S86-07 | Uso. | Al quitar una marca se recuerda que sea de una imagen propia o con permiso. |

## TDD

`gafira-vision` (relleno, catálogo, motores) y `gafira-ui` (página y pasos).

## Evidencia

- Gates de 086 a 089 juntas: `cargo fmt --check`, `clippy -D warnings`, `cargo deny` y `cargo test --workspace --locked` en verde, 3636 tests. Fallan desde antes V14 y `forbid(unsafe_code)` en `gafira-photos`.

- Medido (i7-12700H, RTX 4070 portátil), por pieza de 512 px: MI-GAN 0,44 s en `tract`, 0,46 s en ONNX Runtime CPU y 0,06 s en GPU; LaMa 3,0 s, 1,7 s y 0,27 s. Preparar LaMa la primera vez cuesta unos 11 s en `tract` y 16 s al medir los motores.
- Probado en la app (Linux, datos temporales): descarga real de LaMa desde la página, marca de agua borrada, Área, Elemento y deshacer un trazo. Los dos modelos se compararon sobre dos fotos: LaMa reconstruye mejor los objetos grandes. Sin probar a mano: Primer plano, Cancelar, Comparar; ni Windows ni macOS.
- Se probó borrar solo los trazos de la marca dentro de lo pintado y se descartó: sobre fondos con textura dejaba letras.
- Tests: `s86_*` en `gafira-vision` y `gafira-ui`.

## Riesgos y decisiones abiertas

- Los modelos inventan el fondo: un hueco muy grande puede quedar difuso.
