# 035 — Exportar en segundo plano, con progreso y cancelación

> **Estado:** done — 2026-11-17 — S35-01..S35-06 cubiertos por `gafira-engine` (`export::tests` s35, 4) y `gafira-ui` (`export::tests` s35, 4); 2 mutantes a mano muertos; 2964 tests.

## Objetivo

Exportar nunca congela la ventana. Con la spec 030 el PDF de una página con una imagen grande tardaba varios segundos en el hilo de la ventana. El sistema operativo mostraba entonces «La aplicación no responde» y la persona la cerraba. Ahora la exportación corre en su propio hilo, con una tarjeta de progreso y «Cancelar» (02 §6.4, 03 §8.1).

## Alcance / no alcance

Alcance:

- `gafira-engine`:
  - `ExportService::prepare(request, to)` hace en el hilo de quien llama todas las comprobaciones de `export`: licencia, documento, formato y hallazgos de `validate`. No dibuja nada ni abre archivos. Devuelve un `PreparedExport` que es `Send + 'static` y contiene una instantánea del documento y los puertos.
  - `PreparedExport::run(progress, cancel)` decodifica las imágenes, dibuja y escribe por los temporales.
  - La cancelación se mira antes y entre las imágenes, en el exportador y después de él, antes de mover nada.
  - `export` es `prepare` seguido de `run`.
  - El engine no crea hilos (ARQ-02, P13): los crea la app.
- `gafira-ui`:
  - «Exportar» muestra el diálogo de guardado y luego `prepare`; si sale bien, `run` va a un hilo `gafira-export`.
  - Mientras corre, la ventana sigue dibujando y respondiendo, y una tarjeta no modal abajo a la derecha muestra «Exportando…», la barra con el porcentaje y «Cancelar».
  - Al terminar se muestra uno de estos avisos:
    - «Tu diseño está listo para compartir»;
    - «Exportación cancelada.»;
    - el mensaje de error.
  - Un hilo que termina sin respuesta (por ejemplo, el pánico de un renderer) es un error, nunca el cierre de la ventana.
  - Una segunda exportación mientras corre la primera avisa «Ya hay una exportación en curso».
  - Al cerrar la ventana se cancela la exportación y se espera al hilo, así no queda ningún temporal.

No alcance:

- la cola persistente de exportaciones y sus reintentos (02 §6.4);
- varias exportaciones a la vez;
- el diálogo de guardado no bloqueante (`rfd` asíncrono);
- guardar el proyecto en segundo plano, aunque la miniatura de la spec 033 se dibuja al guardar.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S35-01 | Comprobar antes de correr. | `prepare` rechaza igual que `export` (licencia, formato) sin dibujar ni abrir archivos; el resultado es `Send + 'static`. |
| S35-02 | Instantánea. | Lo que se edita después de `prepare` no entra en la exportación. |
| S35-03 | Cancelar no deja archivos. | Cancelado antes de correr, no dibuja y no deja archivos; cancelado después de que el exportador escribiera, no mueve nada y borra el temporal. |
| S35-04 | Fuera del hilo de la ventana. | `spawn` vuelve enseguida; el progreso llega a 1, siempre dentro de [0, 1]; la ventana se despierta desde otro hilo; el PDF aparece. |
| S35-05 | «Cancelar». | Termina como cancelada y deja la carpeta vacía. |
| S35-06 | Un pánico no cierra la app. | Un hilo que termina sin respuesta se lee como «No pudimos exportar tu diseño». |

## Evidencia

- `cargo test --workspace --locked`: 2964 tests.
- Mutantes a mano: sin la comprobación posterior al exportador muere S35-03 (cancelado después de escribir); sin la comprobación previa a las imágenes muere S35-03 (cancelado antes).
- Reproducción del fallo: el PDF de Redes con una foto de 3000 × 2000 tardaba 5,9 s en el hilo de la ventana en la compilación de desarrollo.
