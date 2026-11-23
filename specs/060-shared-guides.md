# 060 — Guías comunes a todas las páginas y arrastre limpio

> **Estado:** in-progress — 2026-11-19 — pedida por el titular; D-99.

## Objetivo

Que las guías sean las mismas en todas las páginas del documento (D-99): crear, mover o borrar una guía en una página la crea, la mueve o la borra en todas. Y que, al arrastrar una guía, deje de verse en su sitio original mientras se arrastra.

## Alcance / no alcance

Alcance (`gafira-ui`, sin cambio de formato: cada página sigue guardando `Page.guides`):

- Un cambio de guías se aplica a todas las páginas en un `Transaction` de un paso de deshacer.
- Una página nueva (Añadir página en la cabecera o en la Barra inferior) nace con las guías de la primera página, en el mismo paso; Duplicar ya las copia.
- Durante el arrastre, la guía tomada no se dibuja en su posición de origen; solo se ve la que se mueve.

No alcance: un campo `guides` del documento (cambio de formato), guías en otra unidad.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S60-01 | Guías comunes (D-99). | Crear, mover o borrar una guía en una página deja el mismo juego en todas; un paso de deshacer lo devuelve en todas. |
| S60-02 | Página nueva. | Una página añadida nace con las guías de la primera, en el mismo paso de deshacer. |
| S60-03 | Arrastre limpio. | La guía tomada no se dibuja en su origen mientras se arrastra. |

## TDD

1. `s60_01_guides_are_shared` y `s60_02_a_new_page_takes_the_guides` (`gafira-ui`, integración).

## Evidencia esperada

Tests en verde, gates y prueba del titular.
