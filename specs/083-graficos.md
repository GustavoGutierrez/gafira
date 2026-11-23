# 083 — Gráficos

> **Estado:** in-progress — 2026-11-22 — implementada y probada; pendiente de la prueba del titular. Pedida por el titular con capturas de referencia.

## Objetivo

Elementos → Gráficos: gráficos prediseñados que se insertan con un clic o un arrastre, se editan con sus datos en el panel y en una tabla ampliada, se personalizan (colores incluidos) y se exportan como vectores con texto real.

## Alcance / no alcance

Alcance: 28 tipos en ocho grupos; panel con «Ver todo», búsqueda y «Crear con mis datos»; editor de datos en el panel y ampliado; importar Excel o CSV (lector de la spec 082); elegir qué columna va en las etiquetas y cuáles en los valores; consolidar; colores, texto y opciones por gráfico; convertir una tabla del lienzo en gráfico; exportación vectorial.

No alcance: barras animadas (el editor aún no anima elementos en el lienzo); gráfico nativo de PowerPoint (va como formas y textos editables); doble eje; mapas; colores de la Identidad que se actualizan solos.

## Dependencias

FD-22 y RF-16; D-102 y D-105 (Elementos, clic y arrastre); D-114 (modelo); DIS-33; specs 081 (valores, dibujo del texto) y 082 (lector). Crate de dominio nuevo `gafira-chart` (fila V5: `gafira-geom gafira-color gafira-doc gafira-time gafira-table`); `gafira-render` y `gafira-io-pptx` dependen de él. Sin dependencias externas nuevas.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S83-01 | Modelo. | `NodeKind::Chart` con datos como texto (la primera fila nombra las columnas), límites (201 filas, 20 columnas) y opciones; se guarda, se abre, se valida y se deshace. |
| S83-02 | Datos. | Una columna es de números si al menos la mitad de sus celdas con algo lo son; lo que no es número no se dibuja y el editor lo marca. Etiquetas y valores se eligen o salen solos. Consolidar suma, promedia, cuenta o toma el menor o el mayor de las filas con la misma etiqueta. |
| S83-03 | Dibujo. | Cada tipo da piezas (trazados rellenos y textos) dentro de su caja; con datos vacíos, negativos, enormes o una caja mínima no falla y nada es infinito. Ángulos, áreas y largos son proporcionales a los valores. |
| S83-04 | Ejes. | Marcas en números redondos que cubren los datos; las barras y áreas parten de cero; etiquetas que no caben se acortan o se saltan. |
| S83-05 | Color. | Paleta Gafira por defecto (separación entre vecinos validada para daltonismo), paletas estándar y cada color editable. Con dos o más series hay leyenda; las líneas y la dispersión llevan además una forma por serie (DIS-33). El texto sobre un color contrasta. |
| S83-06 | Panel. | Grupos con «Ver todo», búsqueda y vistas previas; clic y arrastre (D-105); «Crear con mis datos» abre Excel o CSV (elige hoja si hay varias). |
| S83-07 | Editar. | Doble clic en el gráfico abre el panel Gráfico: tipo, pestañas Datos y Personalizar, importar, intercambiar filas y columnas, vaciar. «Ampliar» abre la tabla a pantalla con vista previa. Cada cambio se ve al momento y es un paso de deshacer por campo. |
| S83-08 | Barra. | Colores, fuente, tamaño, negrita y cursiva sobre el gráfico seleccionado. |
| S83-09 | Exportar. | PDF y SVG: trazados y texto seleccionable, sin imágenes. PowerPoint: formas y cuadros de texto. Los formatos de píxeles, igual que el lienzo. |
| S83-10 | Tabla a gráfico. | «Crear gráfico» en el menú de una tabla inserta un gráfico con sus datos. |
| S83-11 | Presupuesto. | Redibujar con 100 puntos, menos de 100 ms (FD-22). |

## TDD

`gafira-doc/tests/chart.rs`, `gafira-edit`, `gafira-chart/tests/{data,scene}.rs`, `gafira-render`, `gafira-io-pptx`, `gafira-ui`.

## Evidencia

- `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` y `cargo test --workspace --locked`: en verde, 3536 tests. V5, V12, V13, ARQ-06, ARQ-07 y ARQ-107, en verde. Fallan desde antes V14 (`DuplicatePage`, `ReorderPage`) y `forbid(unsafe_code)` en `gafira-photos`.
- Paleta Gafira comprobada con un validador de paletas: separación mínima entre vecinos ΔE 8,3 con daltonismo y 27,3 en visión normal. Coral, menta y ámbar quedan por debajo de 3:1 sobre blanco, por eso las etiquetas y la leyenda van en tinta.
- Los 31 gráficos del panel exportados juntos desde el motor: PDF con `pdftotext` legible, fuente incrustada y `pdfimages` vacío; SVG sin imágenes; PowerPoint abierto con LibreOffice como formas y textos.
- Rendimiento (release, 100 puntos con su número): 9 a 10 ms por dibujo (FD-22 pide menos de 100).
- Probado en la app: panel, insertar, doble clic, escribir y añadir filas, celda que no es número, ampliar, intercambiar, cambiar de tipo, personalizar. Los diálogos de archivo («Importar datos» y «Crear con mis datos») no se probaron a mano; comparten el lector de la spec 082.

## Riesgos y decisiones abiertas

- Un número ambiguo (`1.234`) no se adivina: la celda se marca (regla de la spec 081).
- El texto del gráfico mantiene su tamaño al cambiar la caja; los textos se acortan si no caben.
