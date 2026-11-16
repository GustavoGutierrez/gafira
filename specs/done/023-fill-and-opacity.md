# 023 — Relleno y opacidad en el Inspector

> **Estado:** done — 2026-11-16 — S23-01..S23-02 cubiertos por `gafira-ui` (`canvas::tests` 1, `editor::tests` 1); 2806 tests.

## Objetivo

Cambiar el color de relleno y la opacidad de la forma seleccionada desde el Inspector (03 §2.4), con deshacer.

## Alcance / no alcance

Alcance (`gafira-ui`): selector de color sRGB para el relleno (`Property::Fill`), botón «Sin relleno», deslizador de opacidad 0–100 % (`Property::Opacity`) con la unidad del catálogo (ARQ-97); el lienzo pinta el relleno con la opacidad del elemento.

No alcance: colores de la Identidad visual y paletas del documento, contorno, degradados, modos de fusión, cuentagotas, espacios de color distintos de sRGB.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S23-01 | Lo que pinta una forma. | Sin relleno no pinta nada; con relleno pinta su color; la opacidad escala el alfa (0,5 → 128). |
| S23-02 | Por el engine, con deshacer. | Cambiar el relleno, la opacidad y quitar el relleno llegan al documento; deshacer devuelve el relleno anterior. |
