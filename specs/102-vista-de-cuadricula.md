# 102 — Vista de cuadrícula de las páginas

> **Estado:** in-progress — 2026-11-25 — implementada y probada en Linux (miniaturas, «+» entre páginas, arrastrar para reordenar con hueco, selección, barra de acciones, salida a la página elegida, menú «⋯» de cada página); pendiente del titular.

## Problema y resultado esperado

Con muchas páginas no había una vista de conjunto para ordenarlas. El resultado: un botón «Vista de cuadrícula» en la barra inferior muestra todas las páginas como miniaturas en filas, donde se eligen, se reordenan arrastrando, se añaden, duplican, eliminan y ocultan.

## Alcance

- Botón en la barra inferior, junto a «Presentar en pantalla completa»; marcado mientras la cuadrícula está abierta. Pulsarlo otra vez vuelve al lienzo en la página seleccionada.
- Cada página es una miniatura con su propia forma (proporción de la página) y su número debajo; una página oculta se ve atenuada con el ojo tachado.
- Clic: elige una página; Ctrl+clic añade o quita; Mayús+clic elige el tramo desde la última; doble clic vuelve al lienzo en esa página. Ctrl+A elige todas y Supr elimina las elegidas.
- Arrastrar una página: las demás le hacen sitio, un hueco con su forma y una línea muestran dónde caerá, y la página sigue al puntero con borde de selección; al soltar se mueve (un paso de deshacer).
- Un «+» aparece solo cuando el puntero está en el espacio entre dos páginas de una fila y añade una página ahí; una tarjeta «+» al final añade una al final.
- Al pasar el puntero por una página aparece un botón «⋯»; su menú muestra el título de la página (editable en el propio menú) y su tamaño, y ofrece duplicar, eliminar, agregar una página después, ocultar o mostrar, y bloquear o desbloquear la página.
- Barra superior: «Seleccionar todo», añadir página tras la selección, duplicar, eliminar (siempre queda al menos una página) y ocultar o mostrar.

## Fuera de alcance

Cambiar el tamaño de las miniaturas y arrastrar varias páginas a la vez. Del menú de referencia quedan fuera las funciones que Gafira aún no tiene en una página: copiar y pegar páginas, copiar su estilo, transiciones, comentarios, descargar una página, enlace a la página, notas y redimensionar.

## Requisitos

- **S102-01** Botón que abre y cierra la cuadrícula; al cerrarla el lienzo muestra la página elegida.
- **S102-02** Miniaturas con la forma de cada página, redibujadas cuando la página cambia (una por fotograma).
- **S102-03** Selección con clic, Ctrl y Mayús.
- **S102-04** Reordenar arrastrando, con hueco e indicador del destino.
- **S102-05** «+» entre dos páginas solo al acercar el puntero, y «+» al final.
- **S102-06** Duplicar, eliminar y ocultar las elegidas, cada acción un paso de deshacer.
- **S102-07** Menú «⋯» por página, visible al pasar el puntero: título, tamaño, duplicar, eliminar, agregar después, ocultar y bloquear.

## Pruebas

- `s102_03_pages_are_picked` y `s102_04_the_drop_index_follows_the_pointer` (`gafira-ui`).
- Revisión en la app (Linux): cuatro páginas de colores, «+» con su tooltip entre las páginas 1 y 2, arrastre de la página 3 a la posición 2 con hueco y línea, salida al lienzo en la página elegida; «⋯» al pasar el puntero, menú de la página y «Duplicar la página».

## Implementación

`crates/gafira-ui/src/page_grid.rs` (estado, dibujo, selección, arrastre, barra) y en `editor.rs` el botón, `grid_view`, `toggle_grid` y `grid_asks`, que usan las órdenes existentes `ReorderPage`, `InsertPage`, `DuplicatePage`, `RemovePage` y `PageHidden`. Las miniaturas usan la vista previa de exportación del motor.
