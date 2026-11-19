# 049 — Reglas fuera del lienzo y Barra inferior

> **Estado:** done — 2026-11-19 — S49-01..S49-05 cubiertos por `gafira-ui` (`ruler::tests` s49, 2; `editor::tests` s49, 1) y revisados por el titular en la app; 3180 tests.

## Objetivo

Que las Reglas se vean y funcionen como las de un editor de diseño (FD-03, 03 §7.3): fuera del lienzo, con marcas finas entre los números, números verticales legibles y la extensión de la selección marcada en las dos reglas. Que la Barra inferior siga 03 §7.3 y FD-08: Notas con icono, zoom con slider, «n de N» con la página real y Páginas con icono.

## Alcance / no alcance

Alcance (`gafira-ui`):

- Las Reglas ocupan una franja de 20 px arriba y a la izquierda. El lienzo empieza después de ellas y no queda tapado. La esquina se rellena.
- Números en las marcas mayores (1, 2 o 5 × 10ⁿ px, como en la spec 044) y marcas menores entre ellas: 10, 5 o 2 divisiones, la mayor que deje al menos 5 px de pantalla entre marcas. Las mayores ocupan todo el alto de la regla y las menores, un cuarto; la del medio, la mitad.
- En la regla vertical los números van girados 90°, leídos de abajo arriba.
- La caja de la selección se marca en las dos reglas con una banda `selection.bg`.
- Barra inferior, de izquierda a derecha: Notas (icono `note-pencil`, atenuada hasta que exista la función), y a la derecha Imán, Alejar, el slider de zoom (5–3200 %, logarítmico), Acercar, el menú del porcentaje, Páginas (icono `squares-four`), «n de N» y «Añadir página».
- «n de N» cuenta desde la página de la selección, o la primera.
- Iconos Phosphor 2.1.1 nuevos: `note-pencil` y `squares-four`.

No alcance: las Notas por página (FD-02), la rejilla de Páginas, el campo numérico del zoom, «Ajustar a la selección», las reglas en otras unidades que px y la línea del puntero sobre la regla.

## Dependencias

- Specs 037 (zoom e iconos), 044 (reglas y guías) y 017 (Barra inferior).
- 03 §7.3 (maqueta de la Barra inferior) y FD-03, FD-08.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S49-01 | Reglas fuera del lienzo. | Con las Reglas visibles, el área del lienzo empieza 20 px a la derecha y 20 px abajo. Sin ellas, ocupa todo. Crear, mover y borrar guías sigue igual (spec 044). |
| S49-02 | Marcas menores. | Con zoom 100 % y paso 100 px hay 10 divisiones de 10 px. Con 5 % y paso 2000 px las divisiones son de 200 px. Ninguna marca menor cae sobre una mayor y ninguna queda a menos de 5 px de pantalla de su vecina. |
| S49-03 | Números verticales. | En la regla vertical, el texto se dibuja girado −90°. |
| S49-04 | Selección en las reglas. | La banda va de la x mínima a la máxima de las cajas de la selección (y en y en la vertical). Sin selección, no hay banda. |
| S49-05 | Barra inferior. | El orden de 03 §7.3, el slider da `ZoomAsk::To` con el valor elegido entre 5 % y 3200 %, y «n de N» da la página de la selección. |

## TDD

Casos que deben fallar primero:

1. `s49_02_minor_ticks`: los casos de S49-02.
2. `s49_04_selection_band`: la banda de varias cajas es la unión de sus extensiones en cada eje; sin cajas es `None`.
3. `s49_05_page_number`: la página de la selección da su índice + 1. Sin selección, 1. Sin páginas, 0.

## Evidencia

TDD: `minor_ticks`, `band` y `page_number` se escribieron vacíos (sin marcas, `None` y 0). Los 3 tests fallaron por sus aserciones y pasaron con la implementación.

Implementación (`gafira-ui`):

- `ruler::{minor_ticks, band}`: 10, 5 o 2 divisiones por paso, al menos 5 px de pantalla entre marcas. En la práctica siempre son 10, porque el paso mide 64–160 px en pantalla.
- El lienzo va en un `UiBuilder` con el área sin la franja de las reglas. Las reglas se dibujan en la franja exterior.
- `draw_ruler`: marcas mayores de lado a lado con el número al lado (10 px), menores a un cuarto y la del medio a la mitad, banda `selection.bg` de la selección, números girados −90° en la vertical, todo recortado fuera de la esquina.
- Barra inferior: Notas atenuada con `note-pencil`; a la derecha Imán, Alejar, slider logarítmico 5–3200 % (nombre accesible «Zoom»), Acercar, menú del porcentaje, `squares-four` Páginas, «n de N» y «Añadir página», con separadores de 1 px en `border`.
- `page_number` corrige «n de N», que antes siempre decía 1.
- Iconos Phosphor 2.1.1 `note-pencil` y `squares-four`, sin modificar, del tarball comprobado (sha256 `313332be…baadd`), con su BLAKE3 en `assets/LICENSES.toml`.

Comandos:

- `cargo fmt --check`: ok.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: sin avisos.
- `cargo test --workspace --locked --no-fail-fast`: 3180 passed, 0 failed.
- `python3 tools/sync_agents.py --check`: 0.
- Prueba manual: el titular abrió la app en Linux y aprobó reglas y barra (2026-11-19).

## Riesgos y decisiones abiertas

- Notas queda visible pero atenuada hasta que exista la función: lo pide 03 §7.3.
