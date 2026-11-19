# 048 — Bloquear, ocultar, renombrar y arrastrar en Capas

> **Estado:** done — 2026-11-19 — S48-01..S48-08 cubiertos por `gafira-ui` (`layers::tests` s48, 7; `editor::tests` s48, 4) y `gafira-format` (`gaf` s48, 1); 3177 tests.

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
| S48-08 | El `.gaf` conserva lo oculto, lo bloqueado y el nombre. | Guardar y abrir deja `visible`, `locked` y `name` igual. |
| S48-07 | Lista y textos. | Las filas ocultas o bloqueadas y las que están bajo un grupo oculto o bloqueado se atenúan. Ojo y candado tienen nombre accesible y tooltip en es/en. |

## TDD

Casos que deben fallar primero (lógica pura de `gafira-ui` con una regla invariante):

1. `s48_04_locked_is_not_picked`: un clic sobre un rectángulo bloqueado que tapa otro elige el de debajo. Sobre un bloqueado sin nada debajo no elige nada. Un hijo de un grupo bloqueado tampoco se elige.
2. `s48_04_marquee_and_select_all_skip_locked`: el marco y `Ctrl+A` dejan fuera lo bloqueado y lo que está bajo un antecesor bloqueado.
3. `s48_04_locked_does_not_move`: un arrastre y las flechas sobre una selección que mezcla bloqueados y libres solo mueven los libres. Sin libres, no hay comando.
4. `s48_03_toggle_rule`: con una selección mixta, el atajo lleva todos a oculto o bloqueado. Con todos ocultos o bloqueados, los lleva a visible o libre. Es un solo paso de deshacer.
5. `s48_05_name_normalised`: `"  Logo "` da `Some("Logo")`, y `""` y `"   "` dan `None`. Un nombre igual al actual no da comando.
6. `s48_06_drop_index`: con 4 hermanas, soltar la fila del índice 0 sobre la de delante da índice 3, y bajo ella, 2. Soltarla en su propio hueco no da comando, y un índice fuera de rango no se produce.

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

## Evidencia

TDD (rojo verificado antes de implementar): los 6 tests de `layers::tests` se escribieron con `layers.rs` vacío (cada función devolvía `false`, su entrada o `None`) y con `canvas::hit`, `in_marquee` y `group::items` sin mirar el bloqueo. Fallaron los 6 por sus aserciones (por ejemplo, el clic elegía el 2 bloqueado en vez del 1, y el marco tomaba `[1, 2, 3]`). Después se añadió `s48_07_dimmed_rows`.

Implementación (`gafira-ui`, sin cambios en `gafira-edit` ni en el formato):

- `layers.rs`: `locked` y `hidden` (propio o de un grupo de arriba), `movable`, `toggled` (S48-03), `renamed` (S48-05) y `dropped` (S48-06).
- `canvas::Shape.locked`, heredado en `canvas::shapes`. `canvas::hit` e `in_marquee` lo saltan y `group::items` (Ctrl+A) solo toma lo que se puede elegir (D-91). Lo bloqueado se sigue dibujando.
- Editor: el arrastre y las flechas solo mueven `layers::movable`. Lo bloqueado no muestra tiradores ni asa de giro. Capas tiene un ojo y un candado por fila, renombrar con doble clic (Enter o perder el foco guarda, Esc cancela), arrastre con línea de inserción de 2 px en `primary.bg`, filas atenuadas, `Ctrl+Mayús+H`/`L` y `Alt+↑/↓`. Cada cambio de Capas lleva su propio gesto, así que dos cambios seguidos no se fusionan en un paso de deshacer (la ventana de 500 ms de `History`).
- Iconos Phosphor 2.1.1, Regular, sin modificar: `eye`, `eye-slash`, `lock-simple` y `lock-simple-open`, del tarball de la spec 037 (sha256 `313332be…baadd`, comprobado), con sus entradas BLAKE3 en `assets/LICENSES.toml` (S37-04).
- Textos `layers.{hide, show, lock, unlock}` en es/en.
- `s41_17` se ajustó: `Ctrl+Mayús+H` sigue sin voltear, pero ahora oculta, así que el test lee el rectángulo por id.

Comandos y resultados:

- `cargo test -p gafira-ui --locked s48`: 11 passed.
- `cargo fmt --check`: ok.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: sin avisos.
- `cargo test --workspace --locked --no-fail-fast`: 3177 passed, 0 failed.
- `python3 tools/sync_agents.py --check`: 0.

Prueba manual en Linux: pendiente del titular (abrir, ocultar, bloquear, renombrar y arrastrar filas).

## Limitaciones y seguimiento

- Lo oculto sale de la selección (poda de la spec 040), así que `Ctrl+Mayús+H` solo oculta: para mostrar algo de nuevo se usa el ojo en Capas.
- Un grupo libre que se mueve arrastra también a sus hijos bloqueados (el grupo se mueve entero, D-35).
- La línea de inserción usa las filas de las hermanas. Si hay un grupo abierto entre ellas, la línea bajo el grupo queda entre él y sus hijos.
- Arrastrar una fila a otro grupo (`Reparent`) y el menú contextual siguen fuera.
- No se corrieron mutantes.

