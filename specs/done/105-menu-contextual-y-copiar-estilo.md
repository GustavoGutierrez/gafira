# 105 — Menú contextual de los elementos y copiar estilo

> **Estado:** done — 2026-11-26 — aceptada por el titular en la app; S105-01 a 08 con pruebas (`git grep s105_`).

## Objetivo

Que todo elemento del lienzo tenga clic derecho con las acciones comunes y que se pueda copiar el estilo, los efectos o las propiedades de un elemento y pegarlos en otro con un clic, como un pincel de formato.

## Alcance / no alcance

Alcance:

- Clic derecho sobre un elemento del lienzo: lo selecciona (si no lo estaba) y abre un menú en el puntero.
- Acciones comunes del menú, las mismas que ya tienen el teclado y las barras: copiar, cortar, pegar, duplicar, eliminar, agrupar y desagrupar, orden (al frente, adelante, atrás, al fondo), bloquear y ocultar.
- «Copiar estilo», «Copiar efectos» y «Copiar propiedades»: guardan lo copiado y activan el pincel; el cursor cambia; el siguiente clic sobre otro elemento lo pega en un solo paso de deshacer; Esc o un clic fuera lo cancelan. «Pegar estilo» en el menú aplica lo copiado a la selección.
- En texto, «Conectar datos» con submenú de las columnas de la creación en lote (spec 104 §7.2): «Reemplazar contenido» y «Añadir al final».

No alcance: menú contextual del lienzo vacío y de las páginas (ya existe para páginas), copiar estilo entre tipos distintos salvo opacidad y mezcla, estilos con nombre.

## Dependencias

- Selección (spec 040), comandos `SetProperty` y `Transaction` (`gafira-edit`), portapapeles interno de elementos, efectos de texto (spec 089), relleno y degradado (spec 063), efectos de forma (spec 065), aspecto de imagen (spec 085), datos conectados (spec 104).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S105-01 | Clic derecho sobre un elemento abre su menú | El elemento queda seleccionado y el menú aparece en el puntero; un clic fuera lo cierra |
| S105-02 | Acciones comunes desde el menú | Cada acción hace lo mismo que su atajo y es un paso de deshacer |
| S105-03 | Estilo de texto | Copia fuente, tamaño, color, negrita, cursiva, alineación, interletrado, interlineado, curvatura y degradado; las palabras no cambian |
| S105-04 | Efectos | Copia los efectos de texto, el efecto de una forma o el aspecto de una imagen |
| S105-05 | Propiedades | Copia estilo y efectos, más opacidad, mezcla, relleno, contorno y radio de las formas |
| S105-06 | Pincel | Tras copiar, el cursor cambia y el siguiente clic sobre un elemento pega; Esc cancela |
| S105-07 | Conectar datos | Con datos conectados, el menú de un texto ofrece sus columnas y pone el campo |

## TDD

Casos que deben fallar primero:

1. Extraer el estilo de un texto y aplicarlo a otro conserva las palabras del destino y cambia todos sus tramos (S105-03).
2. Aplicar efectos de texto a una forma no cambia nada; de forma a forma, sí (S105-04).
3. Aplicar propiedades a un elemento de otro tipo solo cambia opacidad y mezcla (S105-05).

## Evidencia esperada

Pruebas `s105_*` de la lógica de estilos y de frames del editor (menú y pincel), gates de `AGENTS.md`.

## Riesgos y decisiones abiertas

- Qué entra en «propiedades» entre tipos distintos: por ahora opacidad y mezcla.

## Implementación (2026-11-26)

- El menú del lienzo reutiliza el de Capas (`layer_menu`): copiar, pegar, duplicar, eliminar, «Capa» (orden), «Alinear a la página», bloquear, ocultar e imagen como fondo; «Cambiar nombre» queda solo en Capas.
- Pincel de formato en `crates/gafira-ui/src/paint.rs` (`Part`, `Copied`, `pasted`): «Copiar estilo», «Copiar efectos», «Copiar propiedades» y «Pegar estilo»; el cursor pasa a «copiar», un aviso arriba explica qué hacer, Esc o un clic en vacío terminan, y cada pegado es una `Transaction` («Pegar estilo»).
- Las pruebas de la lógica se escribieron junto con el código, no antes: no hubo fase roja registrada.

Evidencia: `paint::tests` (S105-03 a 05) y `editor::tests::s105_01_right_click_menu_and_the_format_painter` (clic derecho y clic reales sobre el lienzo, pegado y deshacer).

