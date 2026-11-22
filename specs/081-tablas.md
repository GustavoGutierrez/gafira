# 081 — Tablas

> **Estado:** in-progress — 2026-11-22 — implementada y probada; pendiente de la prueba del titular. Parte del árbol de la spec 080.

## Objetivo

Que Elementos → Tablas ofrezca tablas listas por grupos (Comenzar, Presentar, Programar y Monitorear), que se rellenen y den estilo directamente en el lienzo, que se muevan como un objeto completo, que su texto siga siendo texto en el PDF y que se lleven a Excel.

## Alcance / no alcance

Alcance: el nodo `Table`; las operaciones sobre filas, columnas, celdas y bordes; el panel; la edición en el lienzo con menús de fila y columna; la barra de la selección y Propiedades; exportar a PNG, JPEG, WebP, PDF, SVG, PowerPoint (tabla nativa) y Excel; copiar y pegar celdas con Excel; los valores de las celdas como números, que es lo que necesitarán los Gráficos (FD-22).

No alcance: «Convertir en una hoja de cálculo», fórmulas y generar gráficos (FD-22); abrir `.xlsx`; colores de la Identidad que se actualizan solos (FD-21 b: la Identidad aún no existe); estilos por tramo dentro de una celda; imágenes dentro de celdas.

## Dependencias

- FD-21 y RF-16; D-35 (posiciones en espacio de página), D-102 y D-105 (Elementos, clic y arrastre), specs 061, 070, 073, 074 y 075. «Combinar celdas» era de Fase 2 (D-80): entra ahora porque el titular lo pide.
- `gafira-table` (dominio, capa 2; fila V5 y `clippy.toml`): disposición, operaciones, portapapeles y valores. `gafira-io-xlsx` (adaptador, capa 4): el libro de Excel, escrito desde ECMA-376, con piezas adaptadas de un proyecto MIT OR Apache-2.0 (aviso en `THIRD_PARTY_NOTICES.md`). Sin dependencias externas nuevas. `gafira-text` gana `FontStack` y el `FontBook` guarda las fuentes ya preparadas: antes cada texto las registraba todas de nuevo.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S81-01 | Modelo. | `NodeKind::Table`: 1–50 filas, 1–20 columnas (FD-21), anchos y altos positivos, celdas con texto y estilo (fondo, color, fuente, tamaño, negrita, cursiva, alineación horizontal y vertical, cuatro bordes), combinaciones que no se pisan, espaciado de tabla y de celda, fila de encabezado. Ida y vuelta JSON exacta; lo que no cumple no valida. Todo cambio es un paso de deshacer. |
| S81-02 | Filas y columnas. | Añadir antes o después, eliminar, mover, duplicar, uniformar tamaños y ajustar al contenido. La caja crece o encoge lo mismo y la esquina superior izquierda no se mueve, también girada. No se pasa de los límites ni se elimina la última. |
| S81-03 | Tamaños. | Arrastrar el borde de una columna o de una fila cambia solo ese tamaño (mínimo 8 px). Redimensionar la tabla reparte el cambio en proporción. Al escribir, la fila crece para que el texto quepa. |
| S81-04 | Combinar. | Combinar un rango une sus textos; separar devuelve celdas sueltas. Añadir, eliminar o mover filas y columnas mantiene las combinaciones válidas. |
| S81-05 | Bordes. | Todos, contorno, interiores, arriba, horizontales, abajo, izquierda, verticales, derecha y ninguno, con color y grosor, en la selección. El borde que comparten dos celdas es uno solo. |
| S81-06 | Estilos. | Fondo (o sin fondo), color de texto, fuente, tamaño, negrita, cursiva y alineación de celdas, filas, columnas o toda la tabla. «Copiar estilo» y pegarlo en otras celdas. |
| S81-07 | Panel. | Cuatro filas de miniaturas, una por grupo, con «Ver todo» y vuelta. Un clic inserta la tabla centrada y un arrastre la deja donde se suelta (D-105), seleccionada y en un paso de deshacer. Plantillas propias, en el idioma de la interfaz. |
| S81-08 | En el lienzo. | Arrastrar mueve la tabla entera. Un clic sobre una tabla seleccionada elige una celda; Mayús+clic, un rango; doble clic, Intro, F2 o escribir editan. Tab y Mayús+Tab pasan de celda (Tab en la última añade una fila), Intro confirma y baja, las flechas se mueven, Supr vacía, Esc vuelve a la tabla. |
| S81-09 | Fila y columna. | Junto a la fila y sobre la columna activas, un botón «⋯» las selecciona y abre su menú; también el clic derecho. Un «+» en el borde derecho y en el inferior añade columna o fila. |
| S81-10 | Barra. | Sobre la tabla: color de fondo, bordes, espaciado (tabla y celda), color y tamaño de texto, negrita, cursiva y alineación. |
| S81-11 | Portapapeles. | Copiar celdas las pone como texto con tabuladores (Excel las pega en celdas); pegar una matriz 3×3 la rellena y, si hace falta, añade filas y columnas (FD-21 a). Entrecomillado y saltos de línea como Excel. |
| S81-12 | PDF y SVG. | En un PDF no aplanado cada celda es texto real y seleccionable con su fuente; fondos y bordes, vectores (FD-21 c). |
| S81-13 | PowerPoint. | Sale como tabla nativa, con combinaciones, colores, bordes y texto. |
| S81-14 | Excel. | «Excel (.xlsx)» en Exportar (una hoja por tabla del alcance) y «Guardar como Excel…» en el menú. Textos, números como números (un texto ambiguo, como `007` o `1.234`, sigue siendo texto), combinaciones, anchos, altos, fondos, bordes y fuentes. Un texto que empieza por `=` nunca se vuelve fórmula. Sin tablas, lo dice. |
| S81-15 | Datos. | `gafira-table` da las cabeceras, categorías y columnas numéricas de una tabla, para los Gráficos. |

## TDD

`s81_01_*` a `s81_05_*`, `s81_11_*`, `s81_14_*` y `s81_15_*` fallan primero: modelo y validación, operaciones con sus invariantes (propiedad: tras cualquier secuencia la tabla valida), disposición, bordes, portapapeles, valores y partes del libro.

## Evidencia

- Suite completa en verde, con fmt, clippy, V5, V12, V13 y ARQ-107. V14 falla desde antes (`DuplicatePage` y `ReorderPage`), como los avisos de `cargo deny check advisories`.
- Exportación real desde el motor en los siete formatos. PDF: `pdftotext` da el texto por columnas, la fuente va incrustada y no hay imágenes. Excel: LibreOffice y openpyxl leen los números como números y la fila de encabezado inmovilizada. PowerPoint: LibreOffice la abre como tabla.
- Rendimiento (release, 50 × 20 con texto): primer dibujo 97 ms (5,5 s antes de `FontStack`), una letra 0,6 ms y redibujar 40 ms: menos de 50 ms por edición (FD-21).
- Probado en la app: panel, inserción, escribir con Tab e Intro, fila nueva, redimensionar columna, rango, mover, combinar, menús, bordes y espaciado.

## Riesgos y decisiones abiertas

- Una versión anterior de Gafira no abre un archivo con tablas (como con marcos, líneas y QR).
- El PDF lleva el texto y los vectores; no lleva etiquetas de accesibilidad de tabla (PDF/UA).
