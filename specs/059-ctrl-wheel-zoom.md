# 059 — Zoom con Ctrl + rueda, centrado en el puntero

> **Estado:** in-progress — 2026-11-19 — pedida por el titular (FD-08).

## Objetivo

Que Ctrl + rueda del ratón (Cmd en macOS) acerque y aleje el lienzo, y que el punto de la página bajo el puntero se quede bajo el puntero (FD-08). La rueda sola sigue desplazando el lienzo.

## Alcance / no alcance

Alcance (`gafira-ui`): leer el gesto de zoom de egui (`zoom_delta`, que da Ctrl + rueda y el pellizco del touchpad) en lugar del desplazamiento de la rueda, que egui pone a cero con Ctrl; multiplicar el zoom por ese factor dentro de 5–3200 %; y corregir el desplazamiento del lienzo en el frame siguiente para que el punto de página bajo el puntero vuelva a quedar bajo él.

No alcance: el zoom de los botones, el menú y los atajos (siguen centrados como hoy).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S59-01 | Factor. | El zoom nuevo es el actual por el factor del gesto, dentro de 5–3200 %. |
| S59-02 | Centrado en el puntero. | El punto de página que estaba bajo el puntero queda a menos de 1 px de él tras corregir el desplazamiento. |

## TDD

1. `s59_01_zoom_by_factor` y `s59_02_anchored_offset` (`gafira-ui`).

## Evidencia esperada

Tests en verde, gates y prueba del titular con Ctrl + rueda.
