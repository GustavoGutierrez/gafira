# 085 — Editar imagen: ajustes, zonas, filtros y sombras

> **Estado:** in-progress — 2026-11-22 — implementada y probada en Linux; pendiente de la prueba del titular. Pedida por el titular con capturas de referencia.

## Objetivo

Al seleccionar una imagen, «Editar imagen» abre un panel con Seleccionar (dónde), Herramientas, Ajustar, Filtros y Sombras. Nada cambia la imagen: son parámetros del documento.

## Alcance / no alcance

Alcance: FD-32 (Todo, Elemento, Área, Primer plano, Fondo), FD-33, FD-34 con catálogo propio y miniaturas de la propia imagen, FD-35 con ocho sombras, FD-37; el borrador de la spec 084 con barra de estado y botones con margen.

No alcance: zona Texto (D-23), LUT, filtros propios, varias sombras a la vez, clonar estilos, aplicaciones de terceros, imagen a vídeo y generar fondos; fotos dentro de marcos.

## Dependencias

FD-32 a FD-37, D-116; specs 084 (modelos, original), 081 (entrada sobre el lienzo). Motores existentes: `gafira-raster::effects` y `gafira-segment`. Sin dependencias externas nuevas.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S85-01 | Modelo. | `ImageNode.look`: ajustes, hasta 8 zonas con su máscara, filtro con intensidad y sombra; se valida, se guarda, se deshace por control y sin nada no se escribe. La máscara es un recurso del documento. |
| S85-02 | Aplicación. | Al decodificar (lienzo, miniatura y exportación) con el mismo cálculo: ajustes, zonas, filtro y sombra, en ese orden. Sin cambios, los píxeles son idénticos. Fuera de una zona, idénticos. |
| S85-03 | Filtros. | Seis grupos de seis con nombres propios; cada uno con miniatura de la imagen seleccionada; intensidad 0–100; «Quitar». |
| S85-04 | Sombras. | Ninguna, Brillo, Paralela, Contorno, Curva, Página flotante, En ángulo y Suelo, con color, opacidad, desenfoque, distancia, ángulo y tamaño. La caja de la imagen no cambia: la sombra sobresale. |
| S85-05 | Zonas. | Primer plano y Fondo con el modelo de quitar fondo; Elemento, con un clic; Área, con pincel sumar/restar. La zona se ve en el menta de la marca sobre la imagen; «Desmarcar» y borrar zona. Si no es fiable, lo dice. |
| S85-06 | Panel. | Botón en Propiedades y en la barra; páginas Inicio, Ajustar, Filtros y Sombras con volver y cerrar; controles con su número, doble clic restablece, Restablecer por grupo, Comparar. Nada pegado a los bordes. |
| S85-07 | Fluidez. | La vista previa se calcula en otro hilo a tamaño reducido; el control responde en menos de 100 ms (CAL-05). |
| S85-08 | Exportar. | Todos los formatos muestran la imagen editada, la sombra incluida. |

## TDD

`gafira-doc/tests/image_look.rs`, `gafira-edit`, `gafira-raster` (look, sombras, filtros), `gafira-render`, `gafira-engine`, `gafira-io-pptx` y `gafira-ui`.

## Evidencia

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` y `cargo test --workspace --locked`: en verde, 3600 tests. V5, V12, V13, ARQ-06, ARQ-07 y ARQ-107, en verde; fallan desde antes V14 (`DuplicatePage`, `ReorderPage`) y `forbid(unsafe_code)` en `gafira-photos`.
- Medido en release (i7-12700H), sin contar la lectura del archivo: vista previa mientras se mueve un control (480 px), 52 ms con un ajuste y 246 ms con seis ajustes, filtro y sombra; las 36 miniaturas, 390 ms (130 ms por cada 12). A tamaño completo, 12 Mpx: 2,7 s con un ajuste y 13 s con todo lo anterior; solo ocurre al exportar. Con muchos ajustes a la vez la vista previa pasa de 100 ms, pero se calcula en otro hilo y el control no se detiene.
- Probado en la app (Linux, datos temporales): botón en la barra y en Propiedades, filtros con miniaturas, seis de las siete sombras en el lienzo (Brillo, solo en su miniatura), Elemento, Área con y sin detección automática, Primer plano y Fondo con el modelo, ajustes por zona, deshacer por paso. Sin probar a mano: Comparar (tampoco tiene test), y Ajuste automático, Edición del color y el pincel Restar, cuyos cálculos sí tienen tests. No probado: Windows, macOS, ni la exportación abierta en otros programas.

## Riesgos y decisiones abiertas

- Al usar la imagen como fondo de página o dentro de un marco se pierden sus ajustes (fuera de alcance).
- Elemento distingue por color: sobre fotos de bajo contraste puede no acertar y lo avisa; se corrige con el pincel.
- Los valores de filtros y sombras son una primera propuesta, sujeta a la revisión visual del titular (ítem 133 de 07).
