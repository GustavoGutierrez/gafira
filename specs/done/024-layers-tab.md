# 024 — Pestaña Capas del Inspector

> **Estado:** done — 2026-11-16 — S24-01..S24-02 cubiertos por `gafira-ui` (`canvas::tests` 1, `editor::tests` 1); 2808 tests.

## Objetivo

Ver y ordenar los elementos de una página (03 §2.4, Inspector → Capas): la lista de delante hacia atrás, seleccionar desde ella, y Subir o Bajar el seleccionado con deshacer.

## Alcance / no alcance

Alcance (`gafira-ui`): pestañas del Inspector clicables (Posición, Capas, Imagen); Capas lista los elementos raíz de la página del seleccionado o de la primera, con su nombre o su tipo del catálogo; clic selecciona; Subir y Bajar mueven un paso con `Reorder`.

No alcance: hijos de grupos, arrastrar para reordenar, ocultar y bloquear desde la lista, renombrar, agrupar.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S24-01 | La lista. | Raíces de delante hacia atrás, con su tipo (rectángulo, elipse, grupo) y su índice. |
| S24-02 | Reordenar por el engine. | La pieza más nueva está delante; Subir trae el rectángulo al frente; deshacer lo devuelve atrás. |
