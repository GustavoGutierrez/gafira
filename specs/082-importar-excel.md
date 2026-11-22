# 082 — Importar Excel y CSV como tabla

> **Estado:** in-progress — 2026-11-22 — implementada y probada; pendiente de la prueba del titular. Pedida por el titular; el CSV, añadido por el titular el mismo día.

## Objetivo

Que arrastrar, pegar o importar un archivo de Excel o CSV en el lienzo cree una tabla (spec 081) con sus datos y su aspecto; y que, si el libro tiene varias hojas, se elija cuál insertar.

## Alcance / no alcance

Alcance: `.csv` y `.tsv`; `.xlsx` y `.xlsm` (sin macros); texto, números, fechas, porcentajes, monedas, lógicos y errores con el valor guardado de las fórmulas; fuentes, colores (también de tema), fondos, bordes, alineación, combinaciones, anchos y altos; el color de las tablas con formato de Excel.

No alcance: `.xls` antiguo y libros con contraseña (se avisa); fórmulas vivas, gráficos, imágenes, formato condicional y comentarios; filas y columnas ocultas (no se traen); más de 50 filas o 20 columnas (FD-21: se traen las primeras y se avisa); abrir `.ods`.

## Dependencias

FD-21, D-105, specs 067 (soltar y pegar archivos), 068 (Importar archivo) y 081. `gafira-io-xlsx` gana el lector; `roxmltree` (ya en el árbol por `usvg`) pasa a dependencia normal. El archivo es hostil: límites de tamaño y de partes, sin DTD.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S82-01 | Datos. | Cada celda muestra lo que muestra Excel; lo que es número se lee de vuelta como el mismo número (`gafira_table::value`), con coma o punto decimal según el idioma. Las fechas, en el orden de su formato; las de formato local, `AAAA-MM-DD`. |
| S82-02 | Aspecto. | Fuente, tamaño, negrita, cursiva, colores directos, de tema (con matiz) e indexados, fondos, bordes, alineación, combinaciones y tamaños. Sin ningún borde, líneas finas para que se vea la cuadrícula. Una tabla con formato de Excel conserva encabezado y franjas. |
| S82-03 | Área. | Solo lo usado, sin filas ni columnas vacías al principio ni al final, ni ocultas; con un máximo de 50 × 20. |
| S82-04 | Archivo hostil. | Basura, un zip truncado, partes enormes, DTD, direcciones fuera de rango o combinaciones que se pisan dan un error o una tabla válida, nunca un fallo. |
| S82-05 | Entrada. | Soltar en el lienzo (centrada donde se suelta), pegar el archivo copiado e «Importar archivo…». Un paso de deshacer; la tabla, seleccionada y dentro de la página. |
| S82-06 | Varias hojas. | Un diálogo lista las hojas visibles con datos, con su tamaño y una vista previa; «Insertar» pone la elegida. Con una sola hoja entra directamente, salvo que haya que recortarla: entonces el diálogo lo dice antes. |
| S82-07 | Avisos. | `.xls`, contraseña, demasiado grande, dañado o sin datos: un mensaje dice qué pasa y qué hacer. Un archivo que no se admite no se ignora en silencio si es de hoja de cálculo. |
| S82-08 | CSV. | Separador `,` `;` tabulación o `\|` detectado (o dicho con `sep=`), comillas con separadores y saltos dentro, UTF-8 (con o sin BOM), UTF-16 con BOM y Windows-1252. El texto se trae tal cual; los números, a la derecha; una primera fila solo de palabras sobre números es el encabezado. Columnas al ancho de su texto. Mismos límites y avisos que Excel. |

## TDD

`crates/gafira-io-xlsx/tests/read.rs` (`s82_01_*` a `s82_04_*`, `s82_08_*`) y `gafira-ui` (`s82_05_*`, `s82_06_*`).

## Evidencia

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` y `cargo test --workspace --locked`: en verde, 3501 tests.
- `gafira-table` lee ahora `1234.567` (más de tres cifras antes del signo) y `-$30` como números; antes eran texto.
- Un XML muy anidado desbordaba la pila del lector: límite de 256 niveles, con test de regresión (`s82_04_deep_parts`).
- Rendimiento (release): un libro de 50 000 × 10 celdas se lee en 0,32 s.
- Probado en la app con libros de openpyxl, pegando el archivo: diálogo con tres hojas (la vacía y la oculta no salen), hoja de 120 × 30 con aviso e insertada como 50 × 20, CSV en Windows-1252 y aviso con un `.xls`. Soltar desde el gestor de archivos se prueba en `s82_05_*`, no con un arrastre real del sistema.

## Riesgos y decisiones abiertas

- Un entero con separador de miles (`1.234`) se muestra sin él: con él no se distingue de un decimal y dejaría de ser número al exportar o en los gráficos.
- Tres decimales tras hasta tres cifras (`1,234`) tampoco se distinguen de los miles: se añade un cero (`1,2340`).
- Los meses con nombre (`15-ene-24`) se muestran con número.
