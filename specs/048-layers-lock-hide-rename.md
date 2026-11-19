# 048 — Bloquear, ocultar, renombrar y arrastrar en Capas

> **Estado:** in-progress — 2026-11-19 — empezada; S48-04 por D-91.

## Objetivo

Que el usuario controle la estructura de la página desde Capas (FD-12, FD-13, RF-10, RF-11): ocultar y bloquear cada elemento, renombrarlo y reordenarlo arrastrando la fila. Que el lienzo respete el bloqueo: un elemento bloqueado no se mueve, no se redimensiona y no se gira por accidente.

## Alcance / no alcance

Alcance (`gafira-ui`, sin cambio de formato ni de `gafira-edit`):

- Botones de ojo (Ocultar/Mostrar) y candado (Bloquear/Desbloquear) en cada fila de Capas, con `SetProperty` y `Property::Visible` y `Property::Locked`.
- `Ctrl+Shift+H` oculta o muestra la selección y `Ctrl+Shift+L` la bloquea o desbloquea (03 §11), cada una en un solo paso de deshacer.
- Doble clic en el nombre de la fila para renombrar con `Property::Name`. Enter o perder el foco confirma, Esc cancela, y un nombre vacío vuelve al nombre por tipo (`None`).
- Arrastrar una fila entre sus hermanas (mismo padre) muestra una línea de inserción de 2 px (03 «Lista con arrastre») y aplica un solo `Reorder`. `Alt+↑/↓` hace lo mismo con el elemento seleccionado.
- Fila oculta o bloqueada atenuada (03 «Lista con arrastre»), y los hijos de un grupo oculto o bloqueado también.
- Iconos Phosphor nuevos: `eye`, `eye-slash`, `lock-simple` y `lock-simple-open` (D-87, ARQ-75).
- Textos es/en del catálogo.

No alcance: arrastrar una fila a otro grupo o fuera de él (`Reparent`), el menú contextual (03 §11), Capas de «Todas las páginas», el modo de mezcla, el bloqueo y la ocultación de páginas (FD-02), y el aviso de elementos fuera de la página (FD-09).

## Dependencias

- `gafira-edit`: `SetProperty` con `Property::{Visible, Locked, Name}`, `Reorder` y `Transaction` (spec 040). Ya existen y tienen tests.
- `gafira-ui`: `canvas::layers` y `canvas::shapes` (024, 043), `group::{draws, items, pick}` (043), `Selection` (040), el Imán (044) y alinear y distribuir (042). El Imán, alinear y distribuir ya leen `locked`.
- Decisiones: D-17 y D-29 (bloqueados al distribuir y alinear), D-28, D-35, D-87.
- Lo oculto ya no se dibuja, no se toca y no se exporta (`shapes`, `draws`, `gafira-render::graph`). Esta spec no cambia eso.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S48-01 | Ocultar y mostrar (FD-13). | El ojo de una fila cambia `visible` en un paso de deshacer. Un elemento oculto no se dibuja ni se toca en el lienzo, y deshacer lo devuelve. Ocultar algo seleccionado lo quita de la selección. |
| S48-02 | Bloquear y desbloquear (FD-13). | El candado cambia `locked` en un paso de deshacer y la fila se atenúa. |
| S48-03 | Atajos sobre la selección (03 §11). | `Ctrl+Shift+H` y `Ctrl+Shift+L` actúan sobre toda la selección en un solo `Transaction`. Si algún elemento está visible o libre, todos pasan a oculto o bloqueado. Si no, todos pasan a visible o libre. |
| S48-04 | Lo bloqueado en el lienzo (FD-13 b, D-91). | Un elemento bloqueado, o bajo un grupo bloqueado, no se elige con clic ni con marco: el clic pasa a lo que está debajo. `Ctrl+A` no lo incluye. Sí se selecciona desde Capas, pero no muestra tiradores ni asa de giro, y no se mueve con arrastre ni con flechas. Girar, voltear y borrar desde el menú siguen funcionando (como alinear con D-29). |
| S48-05 | Renombrar (FD-12). | Doble clic en la fila abre un campo con el nombre. Enter o perder el foco aplica un `SetProperty` `Name`, Esc no cambia nada y un texto vacío o solo con espacios guarda `None`. El nombre se recorta y queda en el `.gaf`. |
| S48-06 | Reordenar arrastrando (FD-12 b). | Arrastrar una fila sobre sus hermanas muestra la línea de inserción. Soltarla aplica un solo `Reorder` al índice que marca la línea. Soltarla en su propio sitio o fuera de su padre no cambia nada. `Alt+↑` sube el elemento seleccionado un paso y `Alt+↓` lo baja. |
| S48-07 | Lista y textos. | Las filas ocultas o bloqueadas y las que están bajo un grupo oculto o bloqueado se atenúan. Ojo y candado tienen nombre accesible y tooltip en es/en. |

## TDD

Casos que deben fallar primero (lógica pura de `gafira-ui` con una regla invariante):

1. `s48_04_locked_is_not_picked`: un clic sobre un rectángulo bloqueado que tapa otro elige el de debajo. Sobre un bloqueado sin nada debajo no elige nada. Un hijo de un grupo bloqueado tampoco se elige.
2. `s48_04_marquee_and_select_all_skip_locked`: el marco y `Ctrl+A` dejan fuera lo bloqueado y lo que está bajo un antecesor bloqueado.
3. `s48_04_locked_does_not_move`: un arrastre y las flechas sobre una selección que mezcla bloqueados y libres solo mueven los libres. Sin libres, no hay comando.
4. `s48_03_toggle_rule`: con una selección mixta, el atajo lleva todos a oculto o bloqueado. Con todos ocultos o bloqueados, los lleva a visible o libre. Es un solo paso de deshacer.
5. `s48_05_name_normalised`: `"  Logo "` da `Some("Logo")`, y `""` y `"   "` dan `None`. Un nombre igual al actual no da comando.
6. `s48_06_drop_index`: con 4 hermanas, soltar la fila del índice 0 bajo la de delante da índice 3. Soltarla en su propio hueco no da comando, y un índice fuera de rango no se produce.

Integración (`editor::tests`): ojo, candado, renombrar y arrastrar sobre un documento real, cada uno con deshacer y rehacer, y guardar y abrir el `.gaf` conserva `visible`, `locked` y el nombre.

## Evidencia esperada

- Los tests de arriba en verde, con el ID en el nombre.
- Mutantes a mano (opcional) sobre la regla del bloqueo y el índice de soltar.
- Los gates de `AGENTS.md`: fmt, clippy y `cargo test --workspace --locked`, con el recuento de tests.
- Prueba manual en Linux: ocultar, bloquear, renombrar y arrastrar, y comprobar que lo bloqueado no se mueve.

## Riesgos y decisiones abiertas

- S48-04 la confirmó el titular como D-91 (2026-11-19): lo bloqueado no se elige en el lienzo.
- `Ctrl+Shift+H` no choca con `Mayús+H` (voltear, spec 041). Hay que comprobar que egui distingue los dos.
- Arrastrar en egui dentro de un `ScrollArea` puede robar el gesto del scroll. Si pasa, se arrastra solo desde el asa ⋮⋮ de la fila.
