# 040 — Selección múltiple

> **Estado:** done — 2026-11-17 — S40-01..S40-09 cubiertos por `gafira-edit` (`set_property::tests` s40, 4; `transaction::tests` s40, 1) y `gafira-ui` (`selection::tests` s40, 8; `canvas::tests` s40, 3; `editor::tests` s40, 6); 13 mutantes a mano muertos; 3048 tests.

## Objetivo

Seleccionar varios elementos a la vez y moverlos o eliminarlos en un solo paso (03 §9.1: «`Shift+clic` suma; arrastrar en vacío traza un marco de selección; `Ctrl/Cmd+A` selecciona todo; `Esc` sale»; FD-10: «selección con clic, `Shift+clic`, marco y `Ctrl/Cmd+A`»). Hasta ahora el editor guardaba un solo elemento seleccionado (`Option<(PageId, NodeId)>`, spec 020).

## Alcance / no alcance

Alcance:

- `gafira-ui`, `Selection` (`selection.rs`):
  - nodos de una sola página, sin repetidos y en el orden en que se añadieron;
  - el último añadido es el principal: el Inspector muestra sus valores;
  - `toggle` (Mayús+clic), `add` (Mayús+marco), `of` (marco, Ctrl+A), `retain` (poda) y `clear` (Esc);
  - en otra página, `toggle` y `add` empiezan de nuevo allí; añadir nada no cambia nada.
- Lienzo:
  - clic selecciona uno; Mayús+clic lo suma o lo quita; clic en vacío vacía la selección (con Mayús la conserva);
  - arrastrar sobre un elemento seleccionado mueve toda la selección; sobre otro, primero lo selecciona (con Mayús lo suma);
  - arrastrar en vacío traza un marco (relleno `selection.bg` al 50 % y borde `canvas.selection`) que selecciona los elementos cuya caja toca, en la página donde empezó o, si empezó fuera de las páginas, en la primera que toca; con Mayús los suma;
  - Esc vacía la selección y Ctrl/Cmd+A (también Edición → «Seleccionar todo») selecciona las raíces dibujadas de la página de la selección, o de la primera; los dos se ignoran mientras se escribe en un campo;
  - cada elemento seleccionado lleva su borde; con más de uno se dibuja además la caja que los envuelve (`canvas::bounds`, 4 px por fuera, 1 px `canvas.selection`); los tiradores de tamaño solo aparecen con un elemento.
- Mover: cada fotograma del arrastre es un `SetProperty` de colocación por elemento. Con varios van en una `Transaction` del mismo gesto, así que todo el arrastre es un paso de deshacer (ARQ-33, FD-10 b).
- `gafira-edit`, `Transaction` se fusiona:
  - su clave es `"transaction"` con la página, los nodos en orden y el gesto de sus hijos, cuando todos tienen clave, de la misma página y del mismo gesto; si no, no tiene;
  - absorbe otra `Transaction` con el mismo número de hijos y la misma clave hijo a hijo; cualquier otra cosa se devuelve intacta;
  - un hijo que rechaza una clave igual (rompe el contrato de `MergeKey`) se conserva detrás de los demás, así no se pierde su cambio.
- Eliminar (Supr, Retroceso o Edición → Eliminar): un solo `RemoveNodes` con todos los seleccionados, un paso de deshacer.
- Inspector con más de un elemento:
  - «N elementos» (`inspector.count`, con plural);
  - «Relleno» si todos son formas y «Opacidad» siempre, con el valor del principal; cada cambio es un `SetProperty` sobre todos los nodos, un paso de deshacer;
  - posición, tamaño, contorno, radio y texto siguen siendo de un solo elemento.
- Capas: Ctrl/Cmd+clic o Mayús+clic suma o quita la fila; se resaltan todas las seleccionadas. Subir, Bajar y la papelera de la fila solo aparecen con un elemento seleccionado.
- Poda: en cada fotograma la selección pierde lo que el documento ya no dibuja (un deshacer que quitó nodos), como hacía la guarda `selected_shape` con uno.

No alcance (specs posteriores):

- girar y voltear; redimensionar una selección múltiple por su caja (FD-10 «con selección múltiple se edita el cuadro envolvente»);
- alinear y distribuir (FD-11), agrupar y desagrupar, copiar, pegar y duplicar;
- el marco que selecciona por la curva de una elipse: cuenta su caja;
- seleccionar grupos: no se dibujan ni se seleccionan todavía (no hay forma de crearlos desde la interfaz), así que Ctrl+A los salta;
- el campo X/Y/Ancho/Alto del cuadro envolvente.

## Dependencias

- Specs 020 (selección y arrastre), 022 (tiradores), 023 (relleno y opacidad) y 024 (Capas).
- `gafira-edit`: `Transaction`, `SetProperty` (acepta varios nodos con un mismo valor), `RemoveNodes` (acepta varios nodos), `History` (fusión por gesto).
- Sin crates nuevos ni cambios de capas: la tabla V5 no cambia.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S40-01 | Conjunto de selección. | Vacía: sin página ni principal. `of` quita repetidos y el último es el principal; `single` solo con uno. `toggle` suma, quita y vacía; en otra página empieza allí. `add` une sin repetidos y no cambia nada si no añade nada (tampoco en otra página). `retain` conserva el orden y vacía la página con el último. |
| S40-02 | Marco y seleccionar todo. | El marco toca cajas en cualquier dirección y deja fuera lo oculto; la elipse cuenta por su caja. Elige la página donde empezó o la primera que toca; Mayús suma; sin nada tocado vacía sin Mayús y conserva con Mayús. Ctrl+A toma las raíces dibujadas, atrás primero, sin grupos ni ocultos. |
| S40-03 | Caja de la selección. | Es la unión de las cajas; ninguna caja da `None`. |
| S40-04 | Transacciones que se fusionan. | Tres fotogramas de un gesto son una entrada que deshace al inicio y rehace al final; la transacción fusionada revierte y reaplica por sí sola. La clave une las de los hijos o es `None` (vacía, hijo sin clave, otra página, otro gesto). Lo que no empareja se devuelve intacto y no fusiona nada; un hijo que rechaza se conserva. |
| S40-05 | Mover la selección. | Arrastrar dos elementos los mueve igual (15, 6) a zoom 2 en una sola entrada; un tirador con dos seleccionados mueve y no redimensiona; un deshacer los devuelve a los dos. |
| S40-06 | Eliminar la selección. | Eliminar dos de tres deja el tercero y vacía la selección en una sola entrada; un deshacer los trae de vuelta. |
| S40-07 | Poda tras deshacer. | Deshacer la inserción de un seleccionado lo quita de la selección; deshacer el último la vacía. |
| S40-08 | Inspector múltiple. | «Sin relleno» y opacidad 50 % se aplican a los dos, una entrada cada uno; cada deshacer restaura los dos. |
| S40-09 | Teclado. | Ctrl+A selecciona las dos piezas de la página y se conserva entre fotogramas; Esc vacía la selección. |

## TDD

Casos que fallaron primero:

1. `gafira-edit`, S40-04: la entrada del gesto (`left: 3, right: 1`), la fusión directa y la clave (`None`) fallaban sin `merge_key` ni `merge` en `Transaction`; «lo que no empareja» pasaba ya (la implementación por defecto devuelve todo).
2. `gafira-ui`, S40-01 a S40-03: no compilaban sin `Selection` (`one`, `of`, `toggle`, `add`, `retain`...) ni `in_marquee`, `roots` y `bounds` (28 errores).
3. Los tests de integración del editor (S40-02, S40-05 a S40-09) se escribieron con la interfaz ya conectada; su capacidad de fallar la prueban los mutantes de abajo.

## Evidencia

- Mutantes a mano, todos muertos:
  1. la clave de la transacción sin comparar el gesto;
  2. `merge` que solo mira el número de hijos y no la clave de cada uno;
  3. la clave con solo los nodos del primer hijo;
  4. el hijo que rechaza la fusión se pierde en vez de conservarse;
  5. `toggle` en otra página que suma en vez de empezar de nuevo;
  6. `add` sin quitar repetidos;
  7. el marco que solo cuenta su centro;
  8. el marco que ignora Mayús;
  9. sin poda tras deshacer;
  10. una selección múltiple que redimensiona por el tirador;
  11. los valores del Inspector solo en el principal;
  12. eliminar solo el principal;
  13. varios movimientos sin `Transaction` (solo se mueve uno).
- `cargo fmt --all --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 3048 tests, 0 fallos.
- `gafira-testkit` y `gafira-ports` no cambiaron; tampoco las dependencias, las capas ni `ci.yml`, así que no aplican los pasos 4 a 6 de «Before calling something done».
- `python3 tools/sync_agents.py --check`: 0.
- Sin prueba manual en la ventana: el marco, Mayús y Ctrl+A se cubren con los tests del editor (`frame` y `apply`), no con un recorrido a mano.

## Riesgos y decisiones abiertas

- Con selección múltiple el Inspector no edita el cuadro envolvente (FD-10). Queda para una spec posterior, junto con redimensionar la selección por su caja.
- Una `Transaction` de hijos con clave y sin gesto ahora se fusiona con la siguiente igual dentro de la ventana de 500 ms, como ya hacía un `SetProperty` solo. Hoy ningún flujo del engine o de la interfaz crea esas transacciones fuera del arrastre.
- El marco selecciona elementos de una sola página. Uno que cruza dos páginas toma la página donde empezó.
- El marco usa la caja de la elipse, no su curva; el clic sí respeta la curva.
- La caja envolvente no tiene el doble trazo de DIS-12 (color y halo), igual que el borde de selección actual.
