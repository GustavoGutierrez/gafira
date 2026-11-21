# 064 — Panel «Elementos» con categorías y panel lateral ocultable

> **Estado:** in-progress — 2026-11-21 — pedida por el titular con capturas de referencia; D-102.

## Objetivo

Que la pestaña «Formas» del riel se llame «Elementos» y abra una cuadrícula de categorías desde la que se entra en cada biblioteca, empezando por Formas, y que el panel lateral se pueda ocultar pulsando otra vez la pestaña activa del riel (D-102).

## Alcance / no alcance

Alcance: el nombre de la pestaña (es/en) y el glosario de 03; la cuadrícula de categorías del panel con su animación al pasar el puntero; la vista de una categoría con título y flecha para volver; ocultar y mostrar el panel lateral desde el riel, en todas las pestañas; los iconos Phosphor nuevos que usan las tarjetas.

No alcance: las bibliotecas de Líneas, Marcos, Iconos, Códigos QR, Tablas, Gráficos e Ilustraciones (se ven en la cuadrícula y su vista dice que llegarán pronto); la búsqueda de elementos, «Usado recientemente» y «Recomendado»; renombrar claves internas (`tab.pieces`, `Icon::Pieces`, `pieces.*`), que siguen igual.

## Dependencias

- D-102 (decisión), D-94 (nombre anterior), D-87 y 03 §13 (iconos Phosphor de trazo, Regular y Fill), 03 §1 P1 (interfaz neutra), 03 §8.1 (riel).
- Specs 052 y 053: la biblioteca de Formas que se muestra dentro de la categoría Formas.
- `gafira-ui` solamente; tokens de movimiento de `gafira-theme` (`motion.base`, `motion.reduced`).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S64-01 | Nombre (D-102). | La pestaña dice «Elementos» en español y «Elements» en inglés; el glosario de 03 (§2.3, §2.8) lo recoge. |
| S64-02 | Cuadrícula de categorías. | Al abrir Elementos se ven, en tres columnas y en el orden de 03 §2.3, las tarjetas de las categorías: Formas, Foto collage (D-106, en lugar de Líneas), Marcos, Iconos, Códigos QR, Tablas, Gráficos, Ilustraciones y Luces y efectos (spec 065). Cada tarjeta es una pila de tarjetas claras (`surface`, `surface.sunken`, borde `border`) con un icono Phosphor Regular en `text` y su nombre debajo en `caption`. Sin coral ni colores de ilustración (P1). |
| S64-03 | Animación al pasar el puntero. | Al pasar el puntero, la tarjeta delantera se inclina y sube y las traseras se abren en abanico, en `motion.base`; al salir vuelven igual. Cuando exista el ajuste de movimiento reducido (ARQ-64), el cambio será instantáneo. La tarjeta muestra la mano y tiene nombre accesible. |
| S64-04 | Entrar en una categoría. | Al pulsar una tarjeta el panel muestra la categoría: arriba, una flecha «Volver a Elementos» a la izquierda del título de la categoría. Formas muestra la biblioteca de las specs 052/053 (clic y arrastre siguen igual); las demás dicen «Disponible pronto.». |
| S64-05 | Volver. | La flecha vuelve a la cuadrícula. |
| S64-06 | Ocultar el panel lateral. | Pulsar la pestaña activa del riel con el panel visible lo oculta, también dentro de una categoría de Elementos; pulsarla de nuevo lo muestra en la misma vista. Pulsar otra pestaña muestra el panel con esa pestaña. La pestaña activa sigue marcada con el panel oculto. El panel «Fuente» (spec 062) se muestra aunque el panel esté oculto. |

### Regla para todas las categorías (D-105)

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S64-07 | Arrastrar y soltar. | Todo elemento del panel Elementos, de cualquier categoría presente o futura, se añade con un clic (centro de la página) o arrastrándolo al lienzo (centrado donde se suelta, seleccionado, un paso de deshacer). Una categoría nueva no se da por terminada sin ello. |

## Casos TDD

Lógica con invariante (estado del riel y navegación), en `gafira-ui`:

1. `s64_06_clicking_the_active_tab_hides_and_shows_the_panel`.
2. `s64_06_clicking_another_tab_shows_the_panel_on_it`.
3. `s64_04_s64_05_a_category_is_entered_and_left`.
4. `s64_06_hiding_keeps_the_category_and_reopening_returns_to_it`.

## Evidencia esperada

Tests anteriores en verde; catálogos y glosario cambiados; iconos nuevos registrados en `assets/LICENSES.toml` con su hash; gates de `AGENTS.md`; prueba del titular en la app.

## Riesgos y decisiones abiertas

- Las categorías sin biblioteca se ven pero llevan a «Disponible pronto.»; cada una tendrá su spec.
- Nombres largos («Ilustraciones», «Códigos QR») en tres columnas a 280 px de panel: se recortan con puntos suspensivos y el nombre completo va en el texto de ayuda.
