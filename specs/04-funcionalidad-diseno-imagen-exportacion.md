# 04 — Funcionalidad de diseño, imagen y exportación

| Campo | Valor |
|---|---|
| Documento | Especificación 04 |
| Estado | Aprobado (D-83, 2026-11-16) |
| Alcance | Funciones del Editor de diseño: lienzo, posición y Capas, Plantillas, Piezas, Dibujo, Biblioteca, imagen, Aislar sujeto, texto, Identidad visual, Cambiar tamaño, Exportar, Versiones, autoguardado y recuperación |
| Depende de | `00-decisiones.md` (D-01 a D-86, prevalecen), 01 (RF, RNF, CAL), 02 (crates, traits, ARQ, ADR; §3.1 y §3.2 para `Page` y `TextNode`, §7.1 para `.gaf`), 03 (nombres del glosario §2, componentes, DIS) |
| Convenciones | `FD-nn` = ficha de función. Cada ficha indica propósito, comportamiento y reglas, entrada en la UI (nombres del glosario de 03), crates y traits (02), formatos, presupuesto de rendimiento y criterios de aceptación, con trazo a `RF-nn` (01). **PROPUESTA**, **confirmada (D-71)**, **confirmada (D-80)** y **VERIFICAR** como en 01. Las cifras PROPUESTA añadidas a FD-04, FD-07, FD-35, FD-41 y FD-44 se confirmaron en bloque (D-34); D-71 confirma las demás, salvo las fórmulas de FD-33 y el catálogo de FD-34 (revisión visual pendiente: 07, ítem 133; D-78; no bloquea esta spec, D-86). D-80 confirma las decisiones PROPUESTA, salvo el campo `template` del manifiesto (FD-16, §15), que fija D-81, y las fórmulas de FD-33 (D-78) |

**Formato de ficha.** Cada función se describe con: *Propósito* · *Comportamiento y reglas* · *UI* (dónde se usa) · *Técnica* (crates y traits de 02) · *Formatos* (si aplica) · *Presupuesto* (referencia a CAL de 01 o cifra propia, confirmada por D-71) · *Aceptación* (criterios verificables) · *RF*.

**Reglas generales de todas las funciones**

- **G-1**: toda modificación del documento es un `Command` y entra en el historial (ARQ-30, ARQ-33); los gestos continuos generan una sola entrada.
- **G-2**: toda operación mayor de 100 ms se ejecuta fuera del hilo de UI, es cancelable en ≤ 100 ms (ARQ-43, CAL-27) y muestra progreso.
- **G-3**: los ajustes y efectos son no destructivos: el píxel original nunca se modifica (02 §3.2).
- **G-4**: ninguna función de edición, importación o exportación exige red; la única conexión de la app es la de la licencia (ARQ-04, D-01). Licencia y activación: ver spec 06 (pendiente, se redacta al final).
- **G-5**: toda función es alcanzable por Paleta de comandos y por teclado (RF-13, DIS-30).
- **G-6**: los textos de interfaz, mensajes y errores usan el glosario y el microcopy de 03 (§2, §12).

---

## 1. Visión general de las funciones

| Sección | Funciones | RF |
|---|---|---|
| §2 Lienzo y páginas | FD-01..FD-09 | RF-06..RF-09, RF-12 |
| §3 Posición y Capas | FD-10..FD-13 | RF-10, RF-11 |
| §4 Plantillas | FD-14..FD-16 | RF-15 |
| §5 Piezas | FD-17..FD-23 | RF-16 |
| §6 Dibujo | FD-24 | RF-17 |
| §7 Biblioteca | FD-25..FD-30 | RF-18, RF-55 |
| §8 Imagen | FD-31..FD-37 | RF-19..RF-21 |
| §9 Aislar sujeto | FD-38..FD-40 | RF-22, RF-23 |
| §10 Texto | FD-41..FD-49 | RF-24..RF-27 |
| §11 Identidad visual | FD-50..FD-52 | RF-28, RF-29 |
| §12 Cambiar tamaño | FD-53, FD-54 | RF-30 |
| §13 Exportar, Versiones y recuperación | FD-55..FD-63 | RF-31..RF-35, RF-62, RF-02..RF-04 |
| §14 Opcional desactivable | FD-64 | RF-23 |

---

## 2. Lienzo y páginas

**FD-01 · Tamaño, unidades y orientación** (RF-06)
- *Propósito*: crear y cambiar el tamaño del lienzo de cualquier proyecto.
- *Comportamiento y reglas*: tamaños predefinidos (Destinos de FD-53) y personalizados con Ancho y Alto en **px, pulgada (in), mm o cm**; la unidad base interna es el píxel lógico y la conversión usa un DPI por proyecto (por defecto 96; 300 para Impresión). Orientación vertical/horizontal intercambia ancho y alto. Límites: 1–16 384 px por lado (confirmada, D-71; acorde con la textura máxima de la GPU y la exportación por tiles). Al cambiar la unidad, los valores se convierten sin cambiar el tamaño real. Cambiar el tamaño de una página existente no escala el contenido (para eso, FD-54).
- *UI*: Archivo → Nuevo; botón Tamaño de la Barra superior; Inicio («Tamaño propio»).
- *Técnica*: `gafira-geom` (`Unit`, conversión por DPI), `Page.size: PageSize` (02 §3.1); `Command` `SetProperty<PageSize>`.
- *Presupuesto*: aplicar < 100 ms (CAL-05).
- *Aceptación*: (a) 210×297 mm a 300 DPI da 2480×3508 px; (b) alternar px/mm/cm/in no cambia el tamaño físico más de 0,01 px; (c) valores fuera de 1–16 384 px se rechazan con el mensaje inline de 03 §8; (d) prueba de propiedad: `unit → px → unit` es la identidad con error < 1e-6.

**FD-02 · Páginas múltiples** (RF-06)
- *Propósito*: trabajar con varias páginas en un mismo proyecto.
- *Comportamiento y reglas*: las páginas se apilan en vertical con una cabecera por página: título editable y acciones **Subir, Bajar, Ocultar, Bloquear, Duplicar, Eliminar**. Una página oculta no se exporta ni se muestra; una bloqueada no admite edición. Eliminar es reversible (Deshacer). Cada página puede tener su propio tamaño. La Barra inferior muestra «n de N» y la rejilla de Páginas. Nueva página con `Ctrl/Cmd+Enter`. Las **Notas** de la Barra inferior son un texto plano por página (`Page.notes`, 02 §3.1) que no se exporta en ningún formato de imagen, video, SVG ni PPTX (ARQ-15).
- *UI*: cabecera de página; Barra inferior (Páginas, Notas).
- *Técnica*: `Page` (02 §3.1); comandos `InsertPage`, `RemovePage`, `ReorderPage`, `DuplicatePage` en `gafira-edit`.
- *Presupuesto*: abrir un proyecto de 20 páginas < 3 s (CAL-09); duplicar una página de 100 capas < 100 ms.
- *Aceptación*: (a) Subir/Bajar cambia el orden y se refleja en la exportación por rango; (b) Ocultar excluye la página de PNG/PDF; (c) Duplicar produce nuevos `NodeId` sin colisiones (validado por ARQ-13); (d) cada acción se deshace en una entrada.

**FD-03 · Reglas y Guías** (RF-07)
- *Propósito*: alinear con precisión usando referencias visuales.
- *Comportamiento y reglas*: **Reglas** horizontal y vertical en la unidad del proyecto, con el origen en la esquina de la página activa. Las **Guías** se arrastran desde la regla; un doble clic sobre la guía permite escribir su posición numérica; se pueden bloquear y borrar arrastrándolas fuera. Las guías son por página y se guardan en el documento (`Page.guides`). Una guía tiene eje (vertical u horizontal), posición y bloqueo. La posición se mide desde la esquina superior izquierda de la página, en la unidad en que se escribió, y se guarda sin redondear. Puede quedar fuera de la página, como un nodo (02 §3.2). No se exportan. Visibilidad con `Ctrl+Shift+R` (Reglas) y `Ctrl+;` (Guías).
- *UI*: Ver → Reglas, Guías; arrastre desde las reglas.
- *Técnica*: `gafira-layout` (operaciones sobre guías), overlay de `Tool::overlay` (02 §6.1: no invalida tiles). Los datos están en `Page.guides` (02 §3.1). `Guide` y `GuideSet` se definen en `gafira-doc` (ADR 0008).
- *Presupuesto*: arrastrar una guía a 60 fps (CAL-02).
- *Aceptación*: (a) la posición escrita se guarda sin redondear y, convertida a la unidad de la regla, difiere ≤ 0,01 unidades de lo escrito con cualquier DPI válido (1–10 000); (b) las guías sobreviven a guardar y abrir; (c) el Imán (FD-06) se engancha a ellas dentro de su umbral; (d) una posición NaN o `±∞` se rechaza y `Page.guides` no cambia (ARQ-110).

**FD-04 · Generador de guías** (RF-07)
- *Propósito*: crear rejillas de guías sin dibujarlas una a una.
- *Comportamiento y reglas*: ajustes predefinidos «12 columnas» (12 columnas, sin filas), «3 columnas» (3 columnas, sin filas), «Rejilla 3×3» (3 columnas y 3 filas), todos con espacio de 24 px y margen de 24 px (confirmada, D-71), y «Personalizada» (columnas, filas, espacio y margen). La **vista previa** se actualiza en vivo sobre la página al cambiar cada valor; «Aplicar» crea las guías como un único `Command` y «Reemplazar guías existentes» es una casilla.
  - **Cálculo** (D-15): en cada eje con `n ≥ 1` divisiones, el área útil mide `A = lado − 2 × margen` y cada columna o fila mide `c = (A − (n − 1) × espacio) / n`. Se generan 2 guías de margen, en `margen` y en `lado − margen`, y 2 guías por cada espacio, en `margen + i × c + (i − 1) × espacio` y en `margen + i × (c + espacio)`, para `i = 1..n−1`. El total por eje es `2n`. Las columnas usan el ancho y las filas el alto. Con `n = 0`, ese eje no genera guías.
  - **Reglas**: las posiciones no se redondean. Con margen 0, las 2 guías de margen se generan igual, sobre los bordes de la página. Dos guías de un eje en la misma posición exacta se emiten una vez (confirmada, D-71): pasa con espacio 0. La salida da primero las verticales y luego las horizontales, cada grupo en orden ascendente.
  - **Validación**: `c ≥ 1 px`, el mismo mínimo que el lado de una página (`PageSize::MIN_PX`). Si no se cumple, hay error y ninguna guía. Espacio y margen son finitos y ≥ 0; columnas y filas son enteros ≥ 0. El margen es una entrada del generador: no lee ni cambia `Page.margins`. El DPI que convierte las unidades también es una entrada.
- *UI*: Ver → Generador de guías (diálogo con vista previa).
- *Técnica*: `gafira-layout::guides::generate(spec, page)` es función pura; se aplica con `SetProperty` y `Property::Guides`.
- *Presupuesto*: vista previa < 16 ms por cambio de valor (02 §6.7, `bench_guides_generate`).
- *Aceptación*: (a) «12 columnas» con espacio de 24 px y margen de 24 px en un lado de 1080 px da 24 guías verticales, 22 de columna y 2 de margen, con columnas de 64 px: 24, 88, 112, 176, 200 … 968, 992, 1056. Con margen 0 las columnas miden 68 px y las primeras guías son 0, 68 y 92. Con margen 40, `12 × c + 11 × 24 = 1000` con error ≤ 1e-9 y `c` no se redondea; (b) cancelar no modifica el documento; (c) aplicar y deshacer restaura exactamente el conjunto anterior; (d) 2 columnas, espacio 8 y margen 0 en un lado de 10 px dan `c = 1` y generan guías; en un lado de 9,999 px dan error y ninguna guía; (e) «Rejilla 3×3» en 1080×1080 da 12 guías, con 24, 352, 376, 704, 728 y 1056 en cada eje; (f) espacio o margen negativos dan error; espacio 0 con 3 columnas y margen 0 en 900 px da 4 guías (0, 300, 600, 900).

**FD-05 · Márgenes, Sangrado y Zona de seguridad** (RF-07)
- *Propósito*: preparar piezas para impresión y para plataformas que recortan bordes.
- *Comportamiento y reglas*: **Márgenes** por página (`Page.margins: Margins`: cuatro lados, ligados por defecto con `linked`, o independientes; 0 por defecto). **Sangrado** de 0–10 mm (`Page.bleed: Length`, el mismo en los cuatro lados; por defecto 0; 3 mm en Impresión) que se dibuja como borde exterior (`canvas.bleed`) y se incluye al exportar PDF con marcas de corte. **Zona de seguridad**: área interior (por defecto 5 % por lado, D-38; `safe_rect` recibe el porcentaje como entrada) y en video vertical zonas que cubren las interfaces de plataformas (confirmada, D-80; valores VERIFICAR por plataforma). Un contenido que sobresale de la Zona de seguridad dispara el aviso «Parte del contenido queda fuera de la zona de seguridad.» (no bloquea).
  - **Cálculo**: `bleed_rect` amplía el rectángulo de la página por el sangrado en los cuatro lados. `safe_rect` lo reduce un porcentaje de su ancho a izquierda y derecha, y de su alto arriba y abajo. El rectángulo de márgenes lo reduce por cada margen. El DPI que convierte mm a px es una entrada.
  - **`overflow(bounds, rect)`**: da el exceso ≥ 0 por lado e indica si la caja queda totalmente fuera. Un borde sobre el límite no sobresale. La caja está totalmente fuera si no comparte área con `rect`: `bounds.x0 ≥ rect.x1`, `bounds.x1 ≤ rect.x0`, y lo mismo en `y`. El aviso aparece si algún exceso es `> 0`.
  - **Validación**: dan error un sangrado fuera de 0–10 mm, unos márgenes que quitan algo y dejan el rectángulo sin área, y un porcentaje fuera de `[0, 50)`. Una caja de tamaño 0 con márgenes 0 es válida. La regla por eje y el redondeo están en 02 §4.17.2.
- *UI*: Ver → Márgenes; Cambiar tamaño (Impresión); Exportar → Avanzado.
- *Técnica*: `gafira-layout` (`bleed_rect`, `safe_rect`, `overflow`), `Page.bleed`, `Page.margins`.
- *Presupuesto*: cálculo < 1 ms por página (02 §6.7, `bench_layout_rects`).
- *Aceptación*: (a) con sangrado de 3 mm, el PDF exportado tiene `TrimBox` y `BleedBox` correctos (validación estructural); (b) en 1080×1080 con zona del 5 % (de 54 a 1026), un elemento de 100×100 en x = 926 no avisa y en x = 926,01 avisa con exceso de 0,01 ±1e-9 a la derecha; (c) los márgenes se respetan en la plantilla al Adaptar (FD-54); (d) 3 mm a 300 DPI amplían 35,43 ±0,01 px por lado; sangrado 0 da el rectángulo de la página; 10 mm valen y 10,01 mm dan error; (e) `safe_rect` del 5 % en 1080×1920 da de (54, 96) a (1026, 1824); (f) márgenes izquierdo y derecho que suman el ancho, con suma > 0, dan error; una caja de ancho 0 con márgenes 0 da la misma caja; una caja que toca `rect` solo por un borde está totalmente fuera; una caja de tamaño 0 dentro de `rect` tiene exceso 0.

**FD-06 · Imán (snap)** (RF-08)
- *Propósito*: alinear con precisión al arrastrar sin medir.
- *Comportamiento y reglas*: Imán es una función propia, activa por defecto (D-28) y activable con `Ctrl+Shift+'`. Se engancha a: Guías, bordes y centro de la página, Márgenes, Cuadrícula, y bordes y centros de otros elementos, con **umbral de 6 px de pantalla** (configurable de 2 a 16 px; D-28). Al engancharse dibuja una línea guía temporal con el valor de distancia a los vecinos; mantener `Alt` lo desactiva mientras se arrastra. El Imán no cambia el modelo: solo corrige la posición final del `Command`.
  - **Regla** (D-16): cada eje se resuelve por separado. En `x` se comparan el borde izquierdo, el centro y el borde derecho de la caja en movimiento con los destinos verticales; en `y`, el borde superior, el centro y el inferior con los horizontales. Gana el destino más cercano con distancia ≤ umbral. En un empate exacto, la prioridad es Guía > Margen > Página > Elemento > Cuadrícula. Si sigue el empate, gana la menor coordenada de destino y, después, la referencia izquierda o superior antes que el centro, y el centro antes que la derecha o inferior.
  - **Unidades**: `umbral en página = umbral en px / zoom`. El zoom es una entrada, en píxeles de pantalla por unidad de página (1,0 = 100 %; rango de FD-08). Las guías en otra unidad se convierten con el DPI, que también es una entrada.
  - **Cajas**: todas son rectángulos alineados a los ejes en coordenadas de página. Un nodo girado usa su caja alineada a los ejes (D-28).
  - **Anidados**: un nodo dentro de un grupo atrae. Un nodo bajo un antecesor oculto o bloqueado no atrae. Un grupo atrae por la unión de sus hijos.
  - **Quién no atrae**: la selección que se mueve y sus descendientes, un grupo que contiene algo de la selección (sus demás hijos sí atraen), los elementos ocultos o bloqueados, las Guías si su visibilidad está apagada y la Cuadrícula si `visible` o `snap` son falsos. La Cuadrícula atrae a las líneas que FD-07 muestra con el zoom actual.
  - **Resultado**: el desplazamiento y el destino elegido en cada eje. Sin destino, el desplazamiento de ese eje es 0.
  - **Giro**: con `Shift`, el ángulo va al múltiplo de 15° más cercano; a medio camino, al mayor.
- *UI*: Ver → Imán; Ajustes → General (umbral).
- *Técnica*: `gafira-layout::snap`; resultados aplicados por el `Command` de mover; overlay por `Tool::overlay`.
- *Presupuesto*: cálculo de snap < 2 ms con 1 000 nodos (02 §6.7, `bench_snap_1000`); el arrastre mantiene 60 fps (CAL-02).
- *Aceptación*: (a) arrastrar un elemento a 5 px de una Guía, con zoom 100 % y umbral 6, deja su borde sobre ella con error ≤ 1e-9; (b) con `Alt` no se engancha; (c) la función respeta la visibilidad: elementos ocultos o bloqueados no atraen, ni la selección movida ni sus descendientes, ni un nodo bajo un antecesor oculto o bloqueado; un nodo dentro de un grupo visible sí atrae, y el grupo atrae por la unión de sus hijos; un grupo que contiene algo de la selección no atrae, y sus demás hijos sí; (d) prueba de propiedad: el resultado nunca desplaza más que el umbral en ningún eje; (e) a distancia 6 engancha y a 6,01 no; con zoom 200 %, a distancia 3 engancha y a 3,01 no; (f) con una Guía y el borde de página a la misma distancia gana la Guía, y cada par vecino de la prioridad se prueba igual; con la Cuadrícula a 2 y una Guía a 3 gana la Cuadrícula, porque está más cerca; (g) cada eje es independiente: `x` puede engancharse a una Guía e `y` a un elemento en el mismo movimiento; (h) sin destinos, el desplazamiento es (0, 0); (i) con `Shift`, 7,4° da 0° y 7,5° da 15°.

**FD-07 · Cuadrícula** (RF-08)
- *Propósito*: referencia regular de fondo para composición.
- *Comportamiento y reglas*: Cuadrícula es una función propia, activable con `Ctrl+'`. Paso configurable (1–500 en la unidad del paso; por defecto 10 px), subdivisiones (1–10; por defecto 1, confirmada, D-71) y estilo (líneas o puntos; por defecto líneas). Es independiente de las Guías, vive en la página (`Page.grid: GridSettings { visible, step, subdivisions, style, snap }`, 02 §3.1) y no se exporta (ARQ-15). Por defecto, `visible` es falso y `snap` verdadero (confirmada, D-71).
  - **Líneas**: el origen es la esquina superior izquierda de la página. La **cuadrícula mayor** tiene líneas cada `paso` y la **menor** cada `paso / subdivisiones`.
  - **Qué se muestra**: a zoom < 25 % solo la cuadrícula mayor. A 25 % o más, también la menor. Si la separación en pantalla de un nivel es < 6 px (confirmada, D-71), ese nivel no se muestra. Si tampoco cabe la mayor, se usa el menor múltiplo `paso × 2^k` con separación ≥ 6 px.
  - **Imán**: con Imán activo, cada eje se engancha a la línea mostrada más cercana, así que los elementos caen en las intersecciones (regla de FD-06).
- *UI*: Ver → Cuadrícula; Barra contextual del lienzo.
- *Técnica*: `gafira-layout::grid`; dibujo como overlay.
- *Presupuesto*: dibujo < 1 ms (no invalida tiles; 02 §6.7, `bench_grid_lines`).
- *Aceptación*: (a) el paso se respeta en cualquier unidad: las líneas caen en `k × paso` convertido con el DPI, ±0,01 px; (b) activar y desactivar no cambia el documento exportado; (c) con cualquier zoom de 5 % a 3200 % y un *viewport* de hasta 3840×2160 px no se generan más de 2 000 líneas; (d) con paso 10 px y 5 subdivisiones hay líneas cada 40 px a 24,9 %, cada 10 px y no cada 2 px a 100 %, y cada 2 px a 500 %; (e) paso 0,99 o 500,01, subdivisiones 0 u 11, y zoom fuera de 5–3200 % dan error.

**FD-08 · Zoom y vista** (RF-09)
- *Propósito*: acercar y alejar con precisión desde 5 % a 3200 %.
- *Comportamiento y reglas*: slider, campo numérico y desplegable (Ajustar a la ventana, Ajustar a la selección, Ajustar al ancho, 50 %, 100 %, 200 %); `Ctrl+=`, `Ctrl+-`, `Ctrl+0`, `Ctrl+1`; rueda con `Ctrl/Cmd`; gesto de pellizco en trackpad. El zoom es inmediato con refinamiento progresivo (02 §6.2): se muestra el mejor mip disponible y el vector se re-rasteriza al nivel exacto tras 150 ms sin cambios.
- *UI*: Barra inferior (Zoom); Ver → Zoom.
- *Técnica*: `gafira-render` (tiles de 256 px, mipmaps), `RenderBackend`.
- *Presupuesto*: rango 5–3200 % (CAL-12); respuesta al gesto < 16 ms (CAL-01).
- *Aceptación*: (a) los límites 5 % y 3200 % se alcanzan y no se superan; (b) a 3200 % un círculo vectorial no muestra pixelado (golden); (c) «Ajustar a la selección» deja la selección visible con 24 px de margen.

**FD-09 · Recorte visual de elementos fuera de la página** (RF-12)
- *Propósito*: permitir trabajar con elementos que sobresalen sin perderlos.
- *Comportamiento y reglas*: el contenido fuera del área de la página no se pinta en la exportación (salvo el área de Sangrado en el PDF de Impresión) y se atenúa en el lienzo; reaparece al moverlo dentro. Es no destructivo: los nodos conservan su geometría. La caja de selección muestra una etiqueta con cuánto sobresale («+24 px a la derecha»). Los elementos totalmente fuera se listan en Capas con un icono de aviso.
- *UI*: lienzo, Capas.
- *Técnica*: recorte al renderizar por `RenderGraph`; `layout::overflow(bounds, page)` (02 §3.2).
- *Presupuesto*: sin coste medible (CAL-02).
- *Aceptación*: (a) la exportación PNG no contiene píxeles fuera de la página; (b) al mover el elemento dentro, aparece completo; (c) el documento guardado conserva el elemento intacto; (d) en una página de 1080 px, un elemento con x = 1000 y ancho 104 da `overflow` de 24 px a la derecha y 0 en el resto, y con x = 1080 está totalmente fuera.

---

## 3. Posición y Capas

**FD-10 · Selección y transformaciones numéricas** (RF-10)
- *Propósito*: situar y dimensionar con exactitud.
- *Comportamiento y reglas*: selección con clic, `Shift+clic`, marco y `Ctrl/Cmd+A`; doble clic entra en grupos. En la pestaña **Posición** del Inspector: **X, Y, Ancho, Alto, Giro y Opacidad** como campos numéricos con arrastre (03 §8) en la unidad del proyecto; **bloqueo de proporción** (candado); con selección múltiple se edita el cuadro envolvente. Las flechas mueven 1 unidad (10 con `Shift`). Las asas del lienzo y los campos producen los mismos `Command` (`SetProperty<Transform2D>`) con `merge_key`.
- *UI*: Inspector → Posición; Barra contextual.
- *Técnica*: `gafira-edit` (`Selection`, `HitTester`, `SetProperty<T>`), `gafira-geom` (`Affine`).
- *Presupuesto*: selección < 16 ms (CAL-01); arrastre a 60 fps (CAL-02).
- *Aceptación*: (a) con el candado activo, cambiar el Ancho cambia el Alto manteniendo la relación (error < 0,01 px); (b) un arrastre de 300 eventos añade una sola entrada al historial (ARQ-33); (c) alinear, distribuir y ajustar 20 objetos solo con atajos y campos numéricos es posible (criterio del PRD RF-02).

**FD-11 · Alinear y distribuir** (RF-10)
- *Propósito*: ordenar elementos en un solo gesto.
- *Comportamiento y reglas*: seis acciones de **Alinear** (izquierda, centro, derecha, arriba, medio, abajo) respecto de la página (un elemento) o de la caja de la selección (dos o más); **distribuir** horizontal y vertical con espacio igual entre elementos o entre centros (el modo es una entrada; por defecto, espacio igual). Los elementos bloqueados no se mueven. Atajos (D-88): `Alt+A/H/D` (izquierda, centro, derecha) y `Alt+W/V/S` (arriba, medio, abajo).
  - **Cajas**: rectángulos alineados a los ejes en coordenadas de página; un nodo girado usa su caja alineada a los ejes (D-28).
  - **Alinear con bloqueados**: la caja de la selección incluye los elementos bloqueados (D-29). Alinear no cambia el otro eje.
  - **Distribuir** (D-17, D-28): solo cuentan los elementos libres. Se ordenan por su posición en el eje: borde inicial con espacio igual y centro con centros; un empate conserva el orden de entrada. El primero y el último no se mueven. Con menos de 3 libres no hace nada. Un elemento libre puede quedar sobre uno bloqueado.
  - **Espacio igual**: `espacio = (inicio del último − fin del primero − suma de los tamaños intermedios) / (n − 1)`. Si los elementos no caben, `espacio` es negativo y se solapan por igual; no es un error.
  - **Entre centros**: los centros quedan a la misma distancia entre el centro del primero y el del último.
- *UI*: Inspector → Posición → Alinear; Barra contextual; menú contextual.
- *Técnica*: `gafira-layout::align`, `gafira-layout::distribute` (devuelven resultados; el `Command` los aplica).
- *Presupuesto*: < 100 ms con 100 elementos (CAL-05).
- *Aceptación*: (a) alinear a la izquierda deja todos los bordes izquierdos libres dentro de ±0,01 unidades y no cambia `y`; (b) distribuir produce espacios iguales ±0,01 y el primero y el último no se mueven; (c) una sola acción de Deshacer revierte toda la operación; (d) con 4 elementos, 1 de ellos bloqueado, se distribuyen los 3 libres y el bloqueado no se mueve; con 3 elementos y 1 bloqueado, nada cambia; (e) tres cajas de ancho 100 en un tramo de 250 dan espacio −25, sin error; (f) con un elemento, se alinea a la página; una caja girada 45° se alinea por su caja alineada a los ejes; con la selección vacía o un solo elemento bloqueado, nada cambia.

**FD-12 · Orden de apilado y Capas** (RF-11)
- *Propósito*: controlar qué queda delante y gestionar la estructura.
- *Comportamiento y reglas*: acciones **Traer adelante, Enviar atrás, Al frente, Al fondo**; las no aplicables se atenúan (por ejemplo, «Al frente» en el elemento superior). La pestaña **Capas** lista los elementos con arrastre para reordenar, visibilidad y bloqueo; la selección está sincronizada con el lienzo; doble clic renombra. Cada elemento tiene **modo de mezcla** (normal, multiplicar, trama, superponer, oscurecer, aclarar y otros estándar; 16 modos en el MVP, D-27) y **Opacidad**. Alcance: Capas muestra la página activa, con opción «Todas las páginas».
- *UI*: Inspector → Capas y Posición.
- *Técnica*: `NodeKind`, `ElementBase.blend`, `Reorder`/`Reparent` (02 §4.3); modos de mezcla en `gafira-raster` (ARQ-119) y WGSL.
- *Presupuesto*: reordenar < 100 ms; las Capas con 100 elementos se dibujan sin superar 16 ms de frame (CAL-02).
- *Aceptación*: (a) el orden de la lista equivale al orden de pintado; (b) arrastrar una fila muestra la línea de inserción y es un solo `Command`; (c) los 16 modos coinciden entre CPU y GPU en los golden (≤ 2/255); la mitad CPU es el criterio de ARQ-119. Con entrada de 8 bits, la GPU despremultiplica con el redondeo de ARQ-117, como la CPU; con un `c / a` exacto, diez de los 16 modos se separan de la CPU más de 1/255, hasta 85/255 en ColorBurn (D-57).

**FD-13 · Agrupar, bloquear, ocultar y Duplicar** (RF-10)
- *Propósito*: tratar varios elementos como uno y evitar cambios accidentales.
- *Comportamiento y reglas*: **Agrupar** (`Ctrl+G`) crea un `GroupNode`; Desagrupar (`Ctrl+Shift+G`). Los hijos conservan su posición en espacio de página y la caja del grupo es la unión de la de sus hijos (02 §3.2, D-35). El Inspector no ofrece sesgo en grupos. Bloquear y Ocultar afectan a la selección y se ven en Capas. **Duplicar** (`Ctrl/Cmd+D` o `Alt+arrastrar`) crea una copia desplazada y con nombre «{nombre} copia». Los elementos de la Identidad visual pueden bloquearse como grupo (RF-29).
- *UI*: Barra contextual, menú contextual, Capas.
- *Técnica*: `GroupNode`, `InsertNodes`, `Reparent` en `gafira-edit`.
- *Presupuesto*: < 100 ms con 100 elementos.
- *Aceptación*: (a) agrupar y desagrupar conservan posiciones absolutas; (b) un elemento bloqueado no responde al arrastre ni a las flechas; (c) duplicar genera `NodeId` únicos.

---

## 4. Plantillas

**FD-14 · Biblioteca local de Plantillas y búsqueda** (RF-15)
- *Propósito*: encontrar un punto de partida adecuado en segundos.
- *Comportamiento y reglas*: la pestaña **Plantillas** muestra una cuadrícula de miniaturas (carga perezosa) filtrada **por el formato actual** por defecto. Búsqueda por texto y filtros por formato, estilo, color, orientación, tema y duración (video). Cada miniatura tiene metadatos accesibles: tipo, formato, estilo, color principal, fuentes, licencia y elementos reemplazables (requisito del PRD). Las plantillas son locales: empaquetadas con la app (`BundledProvider`) y propias (`UserLibraryProvider`). Un recurso sin licencia registrada no se lista (02 §4.11). No hay descargas ni catálogo remoto.
- *UI*: Panel lateral → Plantillas; Inicio («Explorar plantillas»).
- *Técnica*: `AssetProvider` (`kinds() = [Template]`), búsqueda federada (Composite) en `gafira-assets`; índice local (`LibraryRepository`).
- *Presupuesto*: búsqueda < 100 ms con 5 000 plantillas (confirmada, D-71; CAL-05).
- *Aceptación*: (a) abrir la pestaña muestra primero las plantillas del formato actual; (b) cada plantilla de `assets/` tiene licencia en `LICENSES.toml` (ARQ-75); (c) la búsqueda funciona sin red (ARQ-04).

**FD-15 · Aplicar una plantilla** (RF-15)
- *Propósito*: partir de un diseño existente y personalizarlo.
- *Comportamiento y reglas*: un clic aplica la plantilla **añadiendo una página nueva** con textos y elementos totalmente editables (no se reemplaza la página actual, salvo opción «Reemplazar esta página»). Si el tamaño de la plantilla difiere del de la página, se ofrece «Adaptar al tamaño actual» (usa FD-54). Las fuentes ausentes se sustituyen con aviso (FD-48). El aplicado es una transacción: un solo Deshacer lo revierte.
- *UI*: Plantillas; Inicio.
- *Técnica*: `Command` `ApplyTemplate` (02 §4.3), `materialize` del `AssetProvider`.
- *Presupuesto*: aplicar una plantilla de 50 elementos < 300 ms hasta ser editable (confirmada, D-71).
- *Aceptación*: (a) tras aplicar, todos los textos se pueden editar y los elementos mover; (b) un solo Deshacer elimina la página añadida; (c) las fuentes de la plantilla ausentes generan el aviso «Falta la fuente “{nombre}”…».

**FD-16 · Guardar como plantilla propia** (RF-15)
- *Propósito*: reutilizar diseños propios.
- *Comportamiento y reglas*: Archivo → Guardar como plantilla (o menú contextual sobre la selección) pide Nombre, Etiquetas y Estilo; guarda el diseño o la selección como un `.gaf` marcado como plantilla (`manifest.json`: objeto opcional `template` con `tags`, `pageFormat` (id de formato de inicio de 03 §6.2), `style` y `color` `#RRGGBB`; D-81, 02 §7.1) en la biblioteca local, con miniatura. Las imágenes se incrustan; los elementos de texto se marcan como reemplazables. Se pueden renombrar, editar y borrar desde la pestaña Plantillas.
- *UI*: Archivo → Guardar como plantilla; menú contextual; Plantillas → «Mías».
- *Técnica*: `gafira-format` (`ProjectRepository`), `UserLibraryProvider`.
- *Presupuesto*: guardar < 500 ms para 20 páginas.
- *Aceptación*: (a) la plantilla guardada aparece en «Mías» y se aplica en cualquier proyecto; (b) no contiene rutas absolutas (ARQ-56); (c) abrirla como proyecto funciona.

---

## 5. Piezas (biblioteca de elementos)

La pestaña **Piezas** reúne: Formas, Líneas, Marcos, Iconos, Códigos QR, Tablas, Gráficos e Ilustraciones. Inserción con un clic (en el centro de la vista) o arrastrando. Hay una búsqueda por palabra y los usados recientemente aparecen primero. Todas las piezas incluidas tienen licencia registrada (ARQ-75).

**FD-17 · Formas y Líneas** (RF-16)
- *Propósito*: construir composiciones con geometría simple.
- *Comportamiento y reglas*: formas: rectángulo (radio de esquina editable), elipse, polígono (3–12 lados), estrella (3–12 puntas) y ruta libre; **Líneas** con grosor, estilo (continua, discontinua, punteada), remates y flechas. Relleno sólido, degradado lineal o radial, o sin relleno; contorno con color y grosor. Las formas son vectoriales y editables nodo a nodo (doble clic). Booleanas (unir, restar, intersectar) en Fase 2 (confirmada, D-80).
- *UI*: Piezas → Formas, Líneas; herramientas Forma (R, O) y Línea (L).
- *Técnica*: `ShapeNode`, `gafira-pathops` (contorno de trazo, simplificación), `gafira-geom` (kurbo).
- *Presupuesto*: insertar < 100 ms; arrastrar a 60 fps (CAL-02).
- *Aceptación*: (a) un rectángulo a 3200 % no muestra pixelado; (b) las flechas se mantienen proporcionales al cambiar el grosor; (c) exportado a SVG, la forma conserva sus rutas.

**FD-18 · Marcos con recorte por forma** (RF-16)
- *Propósito*: colocar una imagen dentro de una forma.
- *Comportamiento y reglas*: un **Marco** (cuadrado, círculo, corazón, estrella, rombo, película, borde irregular y más) recorta el contenido soltado encima. El contenido tiene `offset`, `scale` y ajuste (Rellenar, Ajustar, Estirar, Ninguno); doble clic permite reencuadrar la imagen dentro del Marco. Reemplazar la imagen conserva la forma. Quitar el contenido devuelve el Marco vacío con aviso visual.
- *UI*: Piezas → Marcos; arrastrar una imagen desde Biblioteca sobre un Marco.
- *Técnica*: `FrameNode` (`clip_shape`, `content`, `fit`), `Command` `SetFrameContent` (02 §3.2).
- *Presupuesto*: soltar una imagen en un Marco < 100 ms para imágenes en caché.
- *Aceptación*: (a) la imagen soltada queda recortada por la forma; (b) reemplazar la imagen mantiene forma y posición del Marco; (c) un Marco sin contenido no se exporta como vacío visible salvo opción «Incluir marcos vacíos».

**FD-19 · Iconos e Ilustraciones** (RF-16)
- *Propósito*: enriquecer el diseño con gráficos vectoriales listos.
- *Comportamiento y reglas*: Los Iconos y las Ilustraciones incluidos son SVG con licencia permisiva registrada; se insertan como `Group` de `ShapeNode`, escalan sin pérdida y permiten **cambiar color** por muestra de la Identidad. Se pueden importar SVG propios (FD-31). El tamaño de insertado es el 25 % de la página (confirmada, D-71).
- *UI*: Piezas → Iconos, Ilustraciones.
- *Técnica*: `gafira-io-svg` (usvg), `AssetProvider` `BundledProvider`; `assets/LICENSES.toml`.
- *Presupuesto*: insertar < 100 ms.
- *Aceptación*: (a) cambiar el color actualiza todo el icono; (b) los iconos incluidos pasan `xtask check-licenses`; (c) la exportación SVG no incluye referencias externas.

**FD-20 · Códigos QR** (RF-16)
- *Propósito*: generar códigos QR sin conexión.
- *Comportamiento y reglas*: se introduce un texto o URL; opciones: nivel de corrección (L, M, Q, H; por defecto M), color de primer plano y de fondo, margen. El código se dibuja como vectores y se regenera al editar el contenido. Advertencia si el contraste primer plano/fondo es < 3:1 («Este código puede no leerse bien.»). El contenido nunca sale del equipo.
- *UI*: Piezas → Códigos QR.
- *Técnica*: nodo `Extension` con `type_id = "gafira.qr"` (ADR-02, `NodeExtension` interno); generación con crate permisivo (p. ej. `qrcode` 0.14.1, MIT OR Apache-2.0: verificado (2026-11-16, crates.io y `cargo-deny` 0.20.2 con el `deny.toml` del repositorio)) o implementación propia.
- *Presupuesto*: generar < 50 ms para 300 caracteres.
- *Aceptación*: (a) el código generado se decodifica correctamente con un lector de referencia en la prueba (VERIFICAR herramienta); (b) cambiar el nivel de corrección cambia la densidad; (c) el nodo sobrevive a abrir sin el registro (ARQ-11).

**FD-21 · Tablas** (RF-16)
- *Propósito*: presentar datos tabulares dentro del diseño.
- *Comportamiento y reglas*: tablas de 1–50 filas y 1–20 columnas; edición de celdas con texto; fila de encabezado; fila alterna; redimensionar columnas y filas arrastrando; estilos que toman colores de la Identidad. Combinar celdas en Fase 2 (confirmada, D-80). Pegar desde portapapeles (texto con tabuladores) rellena celdas.
- *UI*: Piezas → Tablas.
- *Técnica*: `TableNode` (02 §3.2), `gafira-text` para cada celda.
- *Presupuesto*: edición de una celda < 50 ms (CAL-03).
- *Aceptación*: (a) pegar una matriz 3×3 la rellena; (b) cambiar el color de la Identidad actualiza el encabezado; (c) exporta como vectores en PDF y SVG.

**FD-22 · Gráficos** (RF-16)
- *Propósito*: visualizar datos simples.
- *Comportamiento y reglas*: barras, líneas, circular y anillo; editor de datos en una tabla; títulos, etiquetas, leyenda y paleta de la Identidad; los gráficos se redibujan al editar datos. Todos los colores llevan etiqueta o patrón además del color (DIS-33).
- *UI*: Piezas → Gráficos.
- *Técnica*: nodo `Extension` `type_id = "gafira.chart"`; `gafira-text`, `gafira-geom`.
- *Presupuesto*: redibujar < 100 ms con 100 puntos.
- *Aceptación*: (a) editar un valor actualiza el gráfico; (b) la exportación vectorial conserva la etiqueta de cada serie; (c) un gráfico abierto sin el registro se conserva intacto (ARQ-11).

**FD-23 · Nota adhesiva** (RF-16)
- *Propósito*: anotar sobre el diseño sin que forme parte del resultado.
- *Comportamiento y reglas*: nota con texto y color de una paleta de 6; se arrastra y se redimensiona; **no se exporta** salvo la opción «Incluir notas» de Exportar; se lista en Capas con icono propio.
- *UI*: herramienta Nota adhesiva (N); Piezas.
- *Técnica*: `StickyNode`.
- *Presupuesto*: insertar < 100 ms.
- *Aceptación*: (a) una exportación por defecto no incluye notas; (b) con «Incluir notas» aparecen en el PDF; (c) las notas se pueden ocultar todas con un control en Capas.

---

## 6. Herramientas de dibujo

**FD-24 · Lápiz, Marcador, Resaltador y Borrador** (RF-17)
- *Propósito*: dibujar a mano alzada con trazos vectoriales editables.
- *Comportamiento y reglas*: pestaña **Dibujo** y herramientas flotantes: **Lápiz** (trazo fino opaco), **Marcador** (trazo grueso opaco), **Resaltador** (translúcido con mezcla «multiplicar»), **Borrador** (por defecto elimina el trazo entero que toca; con `Shift` lo divide). Selector de color (colores del documento y de la Identidad) y de grosor (1–100 px). El trazo se suaviza (Catmull-Rom → Bézier con simplificación) y se almacena como `ShapeNode`; cada trazo termina al soltar el puntero y es una entrada de historial. Soporte de lápiz con presión en Fase 2 (confirmada, D-80).
- *UI*: Panel lateral → Dibujo; herramientas P, M, E (Resaltador: Shift+H).
- *Técnica*: `Tool` (máquina de estados, 02 §4.2), `gafira-pathops` (simplificación), `ShapeNode`.
- *Presupuesto*: latencia de trazo < 16 ms por evento (CAL-01).
- *Aceptación*: (a) un trazo de 500 puntos se simplifica a ≤ 80 nodos con error < 0,5 px; (b) el Resaltador no tapa el contenido inferior (golden); (c) `Esc` cancela el trazo en curso; (d) cada trazo se deshace de una vez.

---

## 7. Biblioteca (archivos propios)

**FD-25 · Importar y organizar archivos** (RF-18)
- *Propósito*: reunir en un solo lugar imágenes, video, audio y diseños propios.
- *Comportamiento y reglas*: pestañas internas **Imágenes, Video, Audio, Diseños**. Se importa arrastrando archivos o carpetas (importación masiva) o con «Importar». Las miniaturas y los metadatos se generan en segundo plano; los archivos se guardan por contenido (BLAKE3) y se incrustan en el proyecto al usarse. La biblioteca vive en el directorio de datos de la app, es local y puede exportarse como carpeta.
- *UI*: Panel lateral → Biblioteca; Archivo → Importar (`Ctrl+Shift+I`).
- *Técnica*: `ContentStore`, `LibraryRepository`, `Importer` (02 §4.5) en `gafira-assets`.
- *Presupuesto*: importar 100 imágenes de 12 MP con miniaturas sin bloquear la UI; miniatura de una imagen < 50 ms (confirmada, D-71).
- *Aceptación*: (a) arrastrar 50 archivos no bloquea la interfaz (ARQ-41); (b) un archivo no compatible muestra el mensaje de 03 §12; (c) los recursos duplicados por hash se guardan una sola vez (02 §3.2).

**FD-26 · Búsqueda por etiqueta y color** (RF-18)
- *Propósito*: encontrar archivos rápido.
- *Comportamiento y reglas*: búsqueda por nombre, **etiqueta** (con autocompletado), **color dominante** (selector de muestra; se calculan 5 colores dominantes por imagen), tipo, tamaño, duración y licencia. Se pueden combinar filtros. La búsqueda semántica es opcional y solo local (Fase 3, confirmada, D-80).
- *UI*: Biblioteca → barra de búsqueda y filtros.
- *Técnica*: `LibraryRepository` (índice `rusqlite` o `redb`, VERIFICAR; 02 §9.2), cálculo de paleta en `gafira-raster` (ARQ-122), con Lab y ΔE2000 de `gafira-color`.
- *Presupuesto*: < 100 ms con 10 000 archivos (confirmada, D-71).
- *Aceptación*: (a) buscar el color de una muestra devuelve imágenes con ese color dominante con ΔE2000 < 15 (confirmada, D-71); (b) combinar etiqueta y tipo reduce el resultado; (c) sin red.

**FD-27 · Colecciones** (RF-18)
- *Propósito*: agrupar archivos por proyecto, cliente o tema.
- *Comportamiento y reglas*: una **Colección** es un conjunto con nombre; un archivo puede estar en varias; se crean y editan arrastrando y desde el menú contextual; las colecciones se pueden exportar como carpeta.
- *UI*: Biblioteca → Colecciones.
- *Técnica*: `LibraryRepository`.
- *Presupuesto*: < 100 ms.
- *Aceptación*: (a) un archivo en dos colecciones aparece en ambas; (b) eliminar la colección no borra los archivos.

**FD-28 · Detección de duplicados** (RF-18)
- *Propósito*: evitar archivos repetidos.
- *Comportamiento y reglas*: al importar, igual hash BLAKE3 se considera duplicado exacto y se ofrece «Usar el existente»; imágenes parecidas se detectan con un hash perceptual (dHash, confirmada, D-71) y se listan para revisar sin borrar nada automáticamente.
- *UI*: Biblioteca → «Duplicados».
- *Técnica*: `blake3`; hash perceptual en `gafira-raster` (ARQ-122).
- *Presupuesto*: hash de 12 MP < 100 ms (02 §6.7, `bench_dhash_12mp`).
- *Aceptación*: (a) importar dos veces el mismo archivo no crea dos recursos; (b) una versión al 50 % de una imagen aparece como parecida (distancia de Hamming ≤ 10) si los dos lados de la original miden ≥ 128 px y la mitad se hace por media 2×2, o ≥ 640 px y la mitad se hace por bilineal o por vecino más cercano (ARQ-122, D-86); por debajo de esos tamaños no hay cota; (c) nada se borra sin confirmar.

**FD-29 · Versiones y licencias de archivos** (RF-18)
- *Propósito*: conservar el historial de un archivo y saber con qué licencia se puede usar.
- *Comportamiento y reglas*: «Reemplazar archivo» crea una **versión** nueva del recurso y los diseños que lo usan se actualizan (con Deshacer); se pueden restaurar versiones anteriores. Cada recurso tiene un campo de **licencia** (Propia, Permisiva, Dominio público, Con atribución, Desconocida) y notas; «Desconocida» muestra un aviso al exportar si el recurso está en uso.
- *UI*: Biblioteca → detalle del archivo.
- *Técnica*: `AssetTable`, `ContentStore`.
- *Presupuesto*: reemplazar < 100 ms.
- *Aceptación*: (a) restaurar una versión devuelve los bytes originales (hash idéntico); (b) el aviso de licencia aparece solo cuando corresponde.

**FD-30 · Vínculos ausentes y relocalización** (RF-18, RF-55) — Relocalizar medios es de Fase 2
- *Propósito*: no producir resultados rotos si falta un recurso.
- *Comportamiento y reglas*: en el MVP los recursos se incrustan en el `.gaf` (`AssetStorage::Embedded`); si uno falta o está dañado, se muestra un marcador visible y la exportación advierte antes de generar (RF-33). En **Fase 2**, con **Relocalizar medios** (Archivo → Relocalizar medios), los videos grandes pueden vincularse (`manifest.assets[...].linked { rel_path, bytes, blake3 }`, 02 §7.1): un vínculo ausente emite el evento `LinksMissing`, se muestra como marcador y «Buscar archivo» relocaliza uno o una carpeta entera, confirmando por hash. No se guardan rutas absolutas (ARQ-56).
- *UI*: marcador sobre el elemento; diálogo de Exportar.
- *Técnica*: `AssetTable`, `ContentStore`, `AssetService::relocate` (02 §7.1).
- *Presupuesto*: relocalizar una carpeta de 1 000 archivos < 2 s (confirmada, D-71).
- *Aceptación*: (a) con un recurso ausente la exportación advierte antes de empezar; (b) tras relocalizar, el proyecto se abre completo; (c) 0 rutas absolutas en los JSON (ARQ-56).

---

## 8. Imagen

Todos los ajustes de esta sección se aplican como una pila no destructiva de `EffectInstance` (02 §4.4) sobre el elemento seleccionado, con parámetros editables y eliminables después. La pestaña **Imagen** del Inspector y la Barra contextual los exponen. Los parámetros se generan desde `ParamSchema` y se ejecutan sobre el mip del viewport en vista previa y a resolución completa en segundo plano.

**FD-31 · Importación de imágenes** (RF-19)
- *Propósito*: traer imágenes de uso común al diseño.
- *Comportamiento y reglas*: se importan por arrastre, pegado, Archivo → Importar o desde Biblioteca. El perfil de color embebido (ICC) se respeta y se convierte a sRGB de trabajo; EXIF de orientación se aplica. Un TIFF de varias páginas importa la primera página (con opción de elegir). Un GIF animado se importa como imagen fija o, a elección, como clip de video. Un SVG se importa como vectores editables (scripts y referencias externas se ignoran). Imágenes mayores de 16 384 px por lado se reducen con aviso (confirmada, D-71).
- *UI*: Biblioteca; Archivo → Importar; arrastrar y soltar.
- *Técnica*: `Importer` (`FormatRegistry::resolve_import`, 02 §4.5) en `gafira-io-image` y `gafira-io-svg`; `gafira-color` (`ColorTransform`, moxcms).
- *Formatos*: **MVP**: PNG, JPEG, WebP, GIF, TIFF, BMP, QOI, SVG. **Fase 2**: HEIC (solo lectura con el decodificador del SO, feature `heic-os`, D-59; en Linux solo si el sistema lo ofrece) y PDF (primera página o páginas elegidas; hayro 0.9.0, Apache-2.0 OR MIT: verificado (2026-11-16, crates.io y `cargo-deny` 0.20.2 con el `deny.toml` del repositorio)). Sin decodificador del SO, el archivo HEIC se rechaza con un mensaje que explica por qué. En el MVP el PRD pide JPG/PNG/WebP/SVG/HEIC/TIFF: HEIC se difiere por patentes y dependencia del SO; Gafira no incluye decodificador propio de HEIC (D-59, ADR-07).
- *Presupuesto*: decodificar un JPEG de 12 MP y mostrarlo < 500 ms (confirmada, D-71); sin bloquear la UI (ARQ-41).
- *Aceptación*: (a) cada formato del MVP importa un archivo de prueba y el golden coincide; (b) los fuzz targets de cada importador corren 10 min por PR sin fallos (ARQ-21); (c) un archivo corrupto muestra un error y no un cierre (ARQ-54).

**FD-32 · Zona de edición** (RF-20)
- *Propósito*: decidir en qué parte de la imagen actúa un ajuste.
- *Comportamiento y reglas*: selector con seis opciones: **Todo**, **Elemento** (el objeto detectado), **Mano alzada** (se pinta el área con un pincel), **Primer plano**, **Fondo** y **Texto** (detecta regiones de texto raster). Todas generan una máscara que modula el ajuste y es editable. «Elemento», «Primer plano» y «Fondo» usan el `ClassicSegmenter` (FD-38); «Elemento» añade una pista de prioridad central (D-23). Si el resultado no es fiable (02 ARQ-131), la opción se atenúa con el motivo «No se detectó un resultado fiable en esta imagen.». Esta atenuación solo afecta a las zonas que segmentan, «Elemento», «Primer plano» y «Fondo»: «Todo» nunca se atenúa por baja fiabilidad, y `reliable` se mantiene tal como lo calcula ARQ-131. **Texto** pasa a Fase 2 (D-23): se muestra atenuada con el motivo «Disponible en una versión futura.».
- *UI*: Inspector → Imagen → Zona de edición.
- *Técnica*: `Mask::Alpha` / `Painted`, `Segmenter`, `EffectInstance` con máscara.
- *Presupuesto*: la máscara aproximada < 5 s en CPU para 4000 px (CAL-06), refinamiento en segundo plano; el efecto con máscara ya calculada < 300 ms (CAL-04).
- *Aceptación*: (a) con «Fondo» el ajuste no afecta al primer plano en el golden; (b) la máscara se puede pintar con Pincel Quitar y Pincel Devolver; (c) «Restablecer» elimina el ajuste y su máscara; (d) «Texto» aparece atenuada con su motivo y no genera máscara; (e) `ClassicSegmenter` cumple ARQ-130 y ARQ-131; (f) en una imagen uniforme, «Elemento», «Primer plano» y «Fondo» se atenúan con el motivo, mientras que «Todo» sigue sin atenuar y sin motivo aunque `reliable` sea falso.

**FD-33 · Ajustes de imagen: Ajuste automático, Balance de blancos, Luz, Color y Textura** (RF-20)
- *Propósito*: corregir y estilizar sin salir del editor.
- *Comportamiento y reglas*: cada grupo es un efecto con sliders enlazados a campos numéricos (03 §8), salvo Invertir, que es un interruptor; doble clic en un slider lo restablece; Zona de edición se aplica a todos.

| Grupo | Parámetros | Rango | Reglas |
|---|---|---|---|
| **Ajuste automático** | Intensidad | 0–100 (por defecto 60) | Un clic calcula luz y color con histograma y balance de grises; los valores resultantes quedan en los sliders de Luz y Color y pueden afinarse |
| **Balance de blancos** | Temperatura, Matiz | −100–100 | Operan en luz lineal; cuentagotas sobre un gris neutro calcula ambos |
| **Luz** | Brillo, Contraste, Luces, Sombras, Blancos, Negros | −100–100 | Contraste con curva en S; Luces y Sombras recuperan detalle sin aplanar |
| **Color** | Tono, Saturación, Vitalidad, Invertir; edición por colores detectados (hasta 8 muestras) | Tono −180°–180°; Invertir es un interruptor (`invert`, apagado por defecto; D-46); resto −100–100 | La edición por color detectado mueve solo ese color dentro de una tolerancia ajustable |
| **Textura** | Nitidez, Claridad, Viñeta | 0–100 (viñeta −100–100) | Nitidez con máscara de bordes; la viñeta es centrada y ajustable |

**Fórmulas de FD-33** (PROPUESTA: valores provisionales que fijan las pruebas, pendientes de la revisión visual del titular, ítem 133 de 07, D-78; esa revisión no bloquea esta spec, D-86). Todas cumplen que el valor por defecto no cambia ningún píxel, y ninguna toca el alfa.

Notación: `c` es un canal en luz lineal sin premultiplicar; `v` es `c` codificado con la curva sRGB y recortado a `[0, 1]`; `k = valor / 100`; `Y = 0,2126 R + 0,7152 G + 0,0722 B`; `w(a, b, x)` es *smoothstep* de `a` a `b`.

| Parámetro | Fórmula |
|---|---|
| Temperatura `T`, Matiz `M` | `R × 2^(T/200)`, `G × 2^(−M/200)`, `B × 2^(−T/200)` |
| Cuentagotas | Sobre un gris elegido: `T = 100 × log2(B / R)` y `M = 200 × log2(G / √(R × B))`, recortados a ±100. Con un canal 0, el ajuste no cambia |
| Brillo | `c × 2^k` |
| Contraste | `v' = v^g / (v^g + (1 − v)^g)`, con `g = 2^k` |
| Luces, Sombras | `c × 2^(k_L × w(0,5; 1; v_Y) + k_S × (1 − w(0; 0,5; v_Y)))`, con `v_Y` = `Y` codificada |
| Blancos, Negros | `v' = (v − n) / (b − n)`, con `b = 1 − 0,25 × k_B` y `n = −0,25 × k_N` |
| Tono | Matriz `hue-rotate` de W3C *Filter Effects* sobre `c` |
| Saturación | Matriz `saturate` de W3C *Filter Effects* con `s = 1 + k` |
| Vitalidad | La misma matriz con `s = 1 + k × (1 − (max − min))`, con `max` y `min` de `R`, `G`, `B` |
| Invertir | `v' = 1 − v` (fijada por D-46) |
| Color detectado | Peso `1 − w(τ/2, τ, ΔE)`, con `ΔE` el ΔE2000 frente a la muestra y `τ` la tolerancia; el peso modula Tono y Saturación |
| Nitidez | `c + 1,5 × k × e × (c − G)`, con `G` desenfoque de radio 2 px y `e = w(0,02; 0,1; |Y − Y_G|)` |
| Claridad | Igual que Nitidez, con radio 40 px, peso 0,5 y solo sobre `Y`, sin máscara `e` |
| Viñeta | `c × 2^(k × w(0,5; 1; d))`, con `d` la distancia al centro dividida por media diagonal |
| Ajuste automático | Se calcula sobre el original, sin la pila de ajustes. `n` y `b` son los percentiles 0,5 y 99,5 de `v_Y`, de los que salen Negros y Blancos. Brillo lleva la mediana de `Y` a 0,18. `T` y `M` salen del cuentagotas con la media de `R`, `G`, `B`. Una imagen negra deja Brillo, `T` y `M` en 0. El resultado se multiplica por Intensidad / 100 y queda en los sliders |

FD-33 no tiene slider de Desenfoque en el MVP (D-33): el desenfoque solo se usa dentro de Sombras, Nitidez y Claridad.

Orden de cálculo: Balance de blancos, Luz (Brillo, Contraste, Luces y Sombras, Blancos y Negros), Color (Tono, Saturación, Vitalidad, Invertir, colores detectados) y Textura (Nitidez, Claridad, Viñeta).


- *UI*: Inspector → Imagen; Barra contextual (Ajuste automático, Zona de edición).
- *Técnica*: `Effect` + `GpuEffect` (WGSL) en `gafira-raster::effects` (02 §4.4); cálculo en f32 de luz lineal; golden CPU y comparación GPU↔CPU ≤ 2/255.
- *Presupuesto*: vista previa de cualquier ajuste < 300 ms con 4000 px (CAL-04); el cambio de un slider mantiene la respuesta de interacción < 100 ms (CAL-05).
- *Aceptación*: (a) cada slider en su valor por defecto y el interruptor Invertir apagado son la identidad (diferencia 0); (b) el ajuste de GPU coincide con CPU en ≤ 2/255 en el 99,5 % de los píxeles; (c) los ajustes se pueden desactivar uno a uno (ojo) sin perder valores; (d) `Ajuste automático` es idempotente: se calcula sobre el original, así que aplicarlo dos veces da los mismos valores y no empeora el resultado más de 1/255; (e) valores analíticos de la tabla, ±1e-5: Brillo +100 lleva 0,25 a 0,5; Contraste deja `v = 0,5` en 0,5 con cualquier valor y con +100 lleva `v = 0,75` a 0,9; Blancos +100 lleva `v = 0,75` a 1; Saturación −100 da `R = G = B`; Temperatura +100 multiplica `R` por √2 y divide `B` por √2; (f) un píxel con alfa 0 sigue en 0 y el alfa no cambia con ningún ajuste; Nitidez y Claridad no cambian una imagen plana; Viñeta no cambia el centro; Invertir dos veces da la imagen original ≤ 1/255; (g) tras el cuentagotas sobre un gris dentro del rango, `R`, `G` y `B` de ese gris coinciden ≤ 1/255; (h) un parámetro no finito o fuera de rango da error y no escribe en destino.

**FD-34 · Filtros** (RF-21)
- *Propósito*: aplicar estilos listos con vista previa.
- *Comportamiento y reglas*: rejilla de filtros con **vista previa sobre la propia imagen** (miniaturas del elemento actual); intensidad 0–100; los filtros son presets de parámetros (`ParamValues` guardados, no código; 02 §4.4) y pueden crearse y guardarse como propios. Los radios escalan con el tamaño de la imagen (ARQ-120), de modo que una miniatura representa el resultado final. En Fase 2 se admiten LUT `.cube` como filtros.
  - **Primer corte** (D-40): se entrega el mecanismo y un preset de prueba que no se muestra a la persona. El catálogo real, con sus nombres y valores, llega con la revisión visual del ítem 133 de 07 (D-78).
  - **Preset**: id estable, nombre (`MessageKey`) y lista ordenada de pares (id de efecto, `ParamValues`). Los efectos se aplican en ese orden. Un parámetro que el preset no nombra queda en su valor por defecto. El preset se valida con el `ParamSchema` de cada efecto. El formato con que se guarda un filtro propio va con el criterio (c).
  - **Intensidad** (D-40): con `i` en 0–100, extremos incluidos, un parámetro `Float` vale `d × (1 − i/100) + p × i/100`, con `d` su valor por defecto y `p` el del preset. Un `Int` usa esa fórmula, evaluada en `f64`, y redondea al entero más cercano; medio se aleja de 0. Un `Color` se interpola en luz lineal con la regla de `Color` de 02 §3.4, con `t = i/100`. `Bool` y `Choice` valen `d` con `i = 0` y `p` con `i > 0` (D-41). Una intensidad NaN, `±∞` o fuera de rango da `Err` antes de escribir en destino (ARQ-110).
  - **Miniatura**: es el mismo filtro aplicado a una copia reducida, con `scale` igual a la razón de reducción (ARQ-120).
- *UI*: Inspector → Imagen → Filtros.
- *Técnica*: `Effect` con presets en datos; `LutTransform` (Fase 2).
- *Presupuesto*: generar la rejilla de 12 miniaturas < 300 ms (CAL-04); aplicar < 300 ms.
- *Aceptación*: (a) con el preset de prueba, que usa Contraste y Saturación, y una imagen de bloques constantes de 2×2 px, la miniatura sobre el nivel 1 de mip (ARQ-121) con `scale` 0,5 coincide con el nivel 1 del resultado a tamaño completo en ≤ 1/255 por canal; (b) intensidad 0 equivale a sin filtro, byte a byte, e intensidad 100 da los parámetros del preset, bit a bit; con `d` 0 y `p` 40, intensidad 25 da 10; un `Int` con `d` 0 e intensidad 50 da 3 con `p` 5, −3 con `p` −5 y 2 con `p` 4; un `Color` de negro a blanco, opacos, con intensidad 50 da 0,5 en `r`, `g` y `b` en luz lineal; `Bool` y `Choice` dan `d` con intensidad 0 y `p` con intensidad 0,01 y 100; intensidad NaN, `±∞`, −0,01 o 100,01 da `Err` y deja el destino igual; un preset con un valor fuera de su rango da `Err`; (c) un filtro propio se guarda y reaplica.

**FD-35 · Sombras** (RF-21)
- *Propósito*: dar profundidad a imágenes, formas y texto.
- *Comportamiento y reglas*: tipos **Paralela** (desplazamiento −500–500 px por eje, desenfoque 0–100 px, color, opacidad 0–100 %), **Brillo** (halo difuso, radio 0–100 px) y **Contorno** (trazo exterior con grosor 1–100 px) (rangos confirmados, D-71); color de la muestra o de la Identidad. Se pueden apilar varias. El desenfoque sigue ARQ-120. El desplazamiento escala con la imagen y se redondea a píxeles enteros, también según ARQ-120. El elemento se pinta sobre su sombra con el modo `Normal` de ARQ-119, con la sombra como fondo y el elemento como fuente, en luz lineal también en 8 bits (D-56). **Curva** (sombra deformada bajo el elemento) pasa a Fase 2 (D-27). **Brillo** y **Contorno** salen del primer corte hasta que se aprueben sus fórmulas (D-42).
- *UI*: Inspector → Imagen → Sombras; Efectos de texto.
- *Técnica*: `Effect` de sombra en `gafira-raster` (desenfoque gaussiano separable).
- *Presupuesto*: < 100 ms con radio ≤ 40 px en 1080p (02 §6.7, `bench_shadow_1080`).
- *Aceptación*: (a) la sombra sigue el canal alfa del elemento: con desenfoque 0, el alfa de la sombra es el del elemento desplazado por `opacidad`, exacto; (b) la exportación SVG la rasteriza o la reproduce con filtro, sin pérdida visible > 2/255 (VERIFICAR modo); (c) apilar tres sombras mantiene el arrastre a 60 fps; (d) en Paralela, opacidad 0 no cambia la imagen; un parámetro fuera de rango da `Err` y sus extremos valen: desplazamiento ±500 y desenfoque 0 y 100 pasan; ±500,01 y 100,01 dan `Err`; (e) en `Rgba8`, cada píxel de la salida es, bit a bit, el `Normal` de ARQ-119 en 8 bits con el píxel de sombra como fondo y el del elemento como fuente; por ejemplo, un elemento negro con alfa 128, premultiplicado `(0, 0, 0, 128)`, sobre su sombra de color rojo opaco con desenfoque 0, desplazamiento 0 y opacidad 100, es decir `(128, 0, 0, 128)`, da `(117, 0, 0, 192)`, y en `RgbaF32`, `(0; 0; 0; 0,5)` da `(0,25; 0; 0; 0,75)`; (f) la misma imagen en `Rgba8` y en `RgbaF32`, con los mismos parámetros, da el mismo resultado a ≤ 2/255 por canal en todo píxel cuyo alfa de sombra en 8 bits sea ≥ 16; por debajo de 16 no hay cota, porque una sombra casi transparente no conserva su color en 8 bits (D-56). El alfa de sombra en 8 bits es el byte `⌊cobertura × opacidad × alfa del color + 0,5⌋` que se compone bajo el elemento, con la cobertura en `[0, 255]` y la opacidad en `[0, 1]`. La entrada `RgbaF32` es la de 8 bits despremultiplicada según ARQ-117, decodificada a luz lineal y premultiplicada, y su salida se lleva a 8 bits como la referencia de ARQ-119. La prueba incluye desenfoque, alfa bajo del elemento y color de sombra translúcido. Brillo y Contorno tendrán sus criterios con sus fórmulas (D-42).

**FD-36 · Bordes y esquinas** (RF-21)
- *Propósito*: enmarcar y redondear.
- *Comportamiento y reglas*: contorno con color, grosor y estilo (continuo, discontinuo, punteado); **radio de esquina** global o por esquina; el borde se aplica a la forma recortada de un Marco. Los estilos se guardan como parte de **Estilos** de la Identidad.
- *UI*: Inspector → Imagen → Bordes.
- *Técnica*: `ShapeNode` / `FrameNode`, `gafira-pathops` (contorno de trazo).
- *Presupuesto*: < 100 ms.
- *Aceptación*: (a) un radio mayor que la mitad del lado se limita al máximo posible sin deformar; (b) el borde sigue la forma del Marco; (c) se actualiza al cambiar el Estilo.

**FD-37 · Comparar y Restablecer** (RF-21)
- *Propósito*: juzgar un ajuste y deshacerlo por completo.
- *Comportamiento y reglas*: **Comparar** (mantener pulsado, o tecla `\`) muestra el original; **Restablecer** elimina todos los ajustes del elemento (con Deshacer) y existe por grupo (Luz, Color…). El historial por elemento se ve en Capas (último ajuste, desactivar).
- *UI*: Inspector → Imagen (Comparar, Restablecer); Barra contextual.
- *Técnica*: pila de `EffectInstance`; vista del original sin pila.
- *Presupuesto*: alternar < 16 ms (CAL-01).
- *Aceptación*: (a) mantener Comparar muestra el original con diferencia 0; (b) Restablecer devuelve el elemento al estado inicial (hash del render idéntico); (c) un Deshacer restaura los ajustes.

---

## 9. Aislar sujeto

**FD-38 · Aislar sujeto** (RF-22)
- *Propósito*: separar el sujeto de una imagen sin modelo de IA y sin conexión.
- *Comportamiento y reglas*: un clic sobre una imagen (Barra contextual → «Aislar sujeto») ejecuta el `ClassicSegmenter` (GrabCut con GMM y *max-flow* sobre superpíxeles SLIC; 02 §4.7). Trabaja sobre una copia reducida a 2048 px de lado largo (una imagen menor no se amplía), usa solo color en los GMM (D-45) y escala con *guided filter* al tamaño completo. Ese filtro es el refinado de bordes, el *matting* de esta función: no hay una pasada aparte (02 §4.7, ARQ-130). Muestra progreso y una **estimación de tiempo** en CPU (requisito del PRD), que calcula `gafira-engine` a partir de la fracción de progreso (02 ARQ-112); es cancelable en ≤ 100 ms. El resultado es una **máscara no destructiva** (`Mask::Alpha`) que se muestra sobre un fondo de cuadros; el píxel original no cambia y se puede quitar la máscara con Restablecer. Si hay pistas del usuario (trazo sobre el sujeto o el fondo), se usan como semillas. Sin modelos por defecto (ARQ-76).
- *UI*: Barra contextual y Inspector → Imagen → «Aislar sujeto»; menú contextual.
- *Técnica*: `Segmenter` (`ClassicSegmenter`, `SegHints`, `Segmentation`, `AlphaMask`; 02 ARQ-130) en `gafira-segment`, `Mask`, pool de render (`rayon`).
- *Presupuesto*: imagen de 4000 px **< 5 s en CPU** (CAL-06); **< 2 s con GPU** (CAL-07, VERIFICAR); la UI nunca se bloquea. Falta medir estas cifras con la copia de 2048 px en el hardware de D-03 (ítem 134 de 07, D-78).
- *Aceptación*: (a) las 16 escenas sintéticas del conjunto fijo de `gafira-testkit` dan IoU ≥ 0,90 por escena (prueba bloqueante, D-20, 02 ARQ-132); las 12 escenas nuevas de 02 ARQ-132 solo informan de su recuento (aviso, D-45); el IoU sobre fotos reales es solo un aviso hasta que el titular aporte un conjunto con licencia; (b) `bench_segment_4000` < 5 s en el hardware de gama media; (c) cancelar devuelve el control en ≤ 100 ms y deja el documento intacto (prueba por informes de progreso, 02 ARQ-112; la cifra se mide en `bench_cancel_segment`); (d) la imagen original se puede recuperar quitando la máscara; (e) un borde en rampa da alfa graduado, como fija el criterio de 02 ARQ-130.

**FD-39 · Retocar borde (Pincel Quitar y Pincel Devolver)** (RF-22)
- *Propósito*: corregir la máscara a mano.
- *Comportamiento y reglas*: tras Aislar sujeto aparece la barra de retoque con **Pincel Quitar** (borra de la máscara; `B`) y **Pincel Devolver** (restaura; `Shift+B`), tamaño 1–500 px (`[` y `]`), dureza 0–100 %, vista previa del resultado y **Comparar**. Cada trazo es una entrada de historial y puede deshacerse. La máscara se guarda como tiles con copia al escribir (02 §3.2) y persiste en `masks/` del `.gaf`.
- *UI*: barra de retoque sobre el lienzo; herramientas B y Shift+B.
- *Técnica*: `Tool` Pincel de máscara (02 §4.2), `Command` `PaintMask`, que cambia una `TileSetRef` por otra; la copia al escribir de los tiles `Arc<Tile>` ocurre fuera del comando (ADR 0008).
- *Presupuesto*: latencia de trazo < 16 ms (CAL-01).
- *Aceptación*: (a) un trazo solo copia los tiles tocados (prueba de memoria); (b) Quitar y Devolver son inversos en el golden con dureza 100 %; (c) la máscara sobrevive a guardar y abrir.

**FD-40 · Segmentadores opcionales** (RF-22, RF-23)
- *Propósito*: admitir modelos de segmentación instalables por la persona.
- *Comportamiento y reglas*: el trait `Segmenter` permite añadir `OnnxSegmenter` (Fase 3, feature `onnx`). Los modelos se instalan como paquete separado con su licencia declarada en el manifiesto; nunca se incluyen en el instalador; Ajustes → Funciones locales opcionales los muestra desactivados por defecto. La UI elige la estrategia por `requirements()` y la preferencia de la persona.
- *UI*: Ajustes → Funciones locales opcionales.
- *Técnica*: `Segmenter`, `Registry<dyn Segmenter>` (02 §4.7, §11.1).
- *Presupuesto*: la misma cifra que FD-38 si se activa un segmentador por GPU.
- *Aceptación*: (a) el instalador no contiene `.onnx` (ARQ-76); (b) sin modelo instalado, la función no aparece activa; (c) cada paquete de modelo muestra su licencia antes de activarse.

---

## 10. Texto

Todo texto es un `TextNode` maquetado por `gafira-text` (shaping y bidi con las bibliotecas de 02 §9.2, licencias VERIFICADAS) con fuentes embebidas Inter y Manrope y las fuentes del sistema. La edición es directa sobre el lienzo con la Barra contextual y la pestaña **Texto** del Inspector.

**FD-41 · Texto básico, estilos y listas** (RF-24)
- *Propósito*: componer texto cómodo y exacto.
- *Comportamiento y reglas*: fuente, tamaño (1–1000 pt), peso, **B/I/U/tachado**, mayúsculas, alineación (izquierda, centro, derecha, justificada), anclaje vertical del cuadro, **interletrado** (−100–500, en milésimas de em), **interlineado** (0,5–4,0), **kerning** y ligaduras activables, y **Listas** con viñetas y numeradas (con sangría por nivel). Estilos reutilizables (Título, Subtítulo, Cuerpo, Pie, Llamada) enlazados a **Estilos** de la Identidad; cambiar un estilo actualiza todo. Doble clic crea o edita texto.
  - **Tamaño**: se escribe en pt; `px = pt × DPI / 72`, con el DPI como entrada.
  - **Interlineado**: sigue el modelo de `line-height` de CSS. Un tramo tiene ascendente `A`, descendente `D`, en positivo, y espacio entre líneas `G` de su fuente a su tamaño; su altura natural es `A + D + G`. Con `L = interlineado × (A + D + G)`, el tramo sube `A' = A + (L − A − D) / 2` y baja `D' = D + (L − A − D) / 2`. La línea sube el mayor `A'` y baja el mayor `D'` de sus tramos, y la distancia entre dos líneas base es la bajada de la primera más la subida de la segunda. Si las fuentes de una línea no se anidan, esa distancia no es proporcional al interlineado. Las métricas salen de `OS/2` si la fuente activa `USE_TYPO_METRICS` y, si no, de `hhea`.
  - **Interletrado**: añade `interletrado / 1000 × tamaño` tras cada clúster de grafemas, salvo el último de la línea. El espacio que cuelga no cuenta como último ni lleva interletrado. Si es distinto de 0 se apagan las ligaduras opcionales, como en CSS; las obligatorias de una escritura se mantienen.
  - **Saltos**: una palabra más ancha que la caja se parte por clústeres. Un solo clúster más ancho que la caja se deja en su línea y cuenta como desborde.
  - **Párrafos bidi** (UAX #9 P1): cada salto forzado cierra un párrafo bidi y abre otro, también al final del texto. Son salto forzado U+000A, U+000D, la pareja U+000D U+000A, que cuenta como uno, U+2028 y U+2029. El salto pertenece al párrafo que cierra. U+2028 abre párrafo aunque UAX #9 no lo clasifique como separador de párrafo. U+000B, U+000C, U+0085 y U+001C a U+001E no son salto forzado y no abren párrafo.
  - **Dirección del párrafo** (UAX #9 P2 y P3): cada párrafo toma su dirección de su primer carácter fuerte (clases L, AL o R), sin contar el interior de un aislado y sin heredar del párrafo anterior. AL o R dan derecha a izquierda y L, izquierda a derecha. Un párrafo vacío o sin carácter fuerte es de izquierda a derecha. Todas las líneas de un párrafo tienen su dirección. En esta ficha, «párrafo» y «nivel bidi del párrafo» son siempre los del párrafo bidi de cada línea.
  - **Alineación por párrafo**: Izquierda, Centro y Derecha son físicas: se refieren a los bordes de la caja y no dependen de la dirección. El espacio que cuelga queda donde termina su párrafo: a la derecha en un párrafo de izquierda a derecha y a la izquierda en uno de derecha a izquierda, donde con Izquierda tiene x negativa. En Justificado, la línea que no se estira empieza donde empieza su párrafo: en el borde izquierdo en uno de izquierda a derecha y en el borde derecho en uno de derecha a izquierda.
  - **Espacio que cuelga**: el espacio en blanco al final de una línea, si tiene el nivel bidi del párrafo, cuelga en toda alineación, también en la última línea y antes de un salto forzado, y no cuenta para alinear.
  - **Límite aceptado del espacio final de otro nivel**: la biblioteca de maquetación (02 §9.2) no aplica la regla L1 de UAX #9 a los tramos. El espacio en blanco al final lógico de una línea, dentro de un tramo de nivel bidi distinto al del párrafo, se queda en su posición bidi, dentro de la línea. Las reglas de salto, interletrado, desborde y alineación lo tratan como el espacio que cuelga. Tiene dos efectos visibles: a la izquierda, el contenido puede empezar a un espacio del borde; en cualquier alineación, los clústeres que no son espacio en blanco pueden pasar de la caja hasta el avance de ese espacio, sin desborde (02 ARQ-116), y esa línea no se centra ni se alinea a la derecha. Estos dos efectos son un límite documentado, no un defecto.
  - **Justificado**: reparte el sobrante entre palabras. El espacio en blanco al inicio de una línea no recibe reparto y es contenido de la línea, así que la línea, con ese espacio incluido, termina en el borde; en un párrafo de derecha a izquierda, la tinta puede terminar a un espacio del borde. La última línea de un párrafo y la línea con salto forzado no se estiran, ni la línea de una sola palabra.
  - **Mayúsculas**: conversión Unicode completa sin idioma (`ß` da `SS`; ARQ-107). Los rangos de la salida indican el texto de entrada (02 ARQ-116).
  - **Subrayado y tachado**: posición y grosor salen de las tablas `post` y `OS/2`; con grosor 0 o ausente, 0,05 em (confirmada, D-71).
  - **Listas**: numeradas con `1.` y con viñetas `•`, `◦` y `▪` por nivel, en ciclo; hasta 4 niveles; sangría de `(nivel − 1) × 1,5 em`, con el primer nivel en el borde (confirmada, D-71). Tras el marcador va un espacio y el texto cuelga: todas las líneas de un elemento empiezan en la sangría más el marcador más ancho de ese nivel en la lista más ese espacio, así que `9.` y `10.` alinean su texto. Cada nivel cuenta solo sus elementos numerados: una viñeta no suma ni reinicia la cuenta, y un nivel reinicia tras un elemento de nivel menor (confirmada, D-71).
  - **Límite aceptado del colgado con un DPI extremo**: el colgado se mide en `f32`. Si el avance de la línea formada por el marcador, su espacio y una letra, con su interletrado, pasa de `f32::MAX` (unos 3,4e38 px), el colgado no está garantizado: el resultado puede ser `Err` de no finito u `Ok` con un colgado menor que el debido y el texto sobre el marcador, sin `panic` ni un no finito en la salida (ARQ-110). Con Inter, `1.`, 12 pt e interletrado 0, ese avance es 1,522 em y el punto es un DPI de unos 1,34e39. A 1,7e39 da `Ok` con un colgado de 1,97e38 px contra 2,77e38 debidos, y a 2e39, 2,32e38 contra 3,25e38. Queda fuera del dominio de un documento, cuyo DPI válido es 1–10 000 (FD-03). El motor no comprueba un rango de DPI por este límite y ninguna prueba fija el resultado por encima de él. Es un límite documentado, no un defecto.
  - **Fuentes de reserva** (D-19): quien llama da una lista ordenada de fuentes en bytes, con al menos una; una lista vacía da `Err`, porque el texto se perdería sin aviso. Cada clúster usa la primera fuente que cubre todos sus puntos de código. Si ninguna lo cubre, usa la fuente que cubre más puntos de código del clúster, y en un empate la primera de la lista. Los puntos sin cobertura usan su `.notdef` y la salida lo indica; no es un error.
  - **Emoji** (D-19, D-30): en el MVP son contornos de esas fuentes de reserva; las pruebas usan un subconjunto OFL monocromo. Una fuente que solo tiene emoji en color no cubre el clúster. El emoji en color es de Fase 2.
  - **Hinting**: posiciones y avances no dependen del hinting, que solo se usa en la vista previa (02 §6.2). Los contornos reciben el modo como entrada y los golden van sin hinting.
- *UI*: herramienta Texto (T); Panel lateral → Texto; Inspector → Texto.
- *Técnica*: `gafira-text` (`shape`, `layout_box`), `TextNode`, `StyleRef`; `Command` `SetProperty` con `merge_key` para sliders.
- *Presupuesto*: escribir un carácter < 16 ms en cuadros de hasta 5 000 caracteres (02 §6.7, `bench_text_type_5000`); abrir un documento de 100 capas con texto < 500 ms (CAL-10, texto maquetado de forma perezosa).
- *Aceptación*: (a) el texto con tildes, bidi y emojis se maqueta correctamente en golden de geometría, sin píxeles: orden visual según el algoritmo bidi de Unicode, salvo el límite del espacio final de otro nivel, fijado en una prueba (`abc ابج` da `a`, `b`, `c`, espacio, `ج`, `ب`, `ا` de izquierda a derecha), y clústeres correctos, con `👩‍💻` como 1 clúster y, con el subconjunto de emoji como reserva, `😀` con un contorno real no vacío de esa fuente; (b) cambiar un Estilo actualiza todos los textos vinculados con un solo `Command`; (c) interletrado 0 e interlineado 1,0 son la identidad: las posiciones coinciden bit a bit con las de un texto sin esos ajustes; (d) los extremos de tamaño, interletrado e interlineado valen, y 0,99 y 1000,01 pt, −100,01 y 500,01, 0,49 y 4,01 dan error; (e) una línea de `N` clústeres, sin contar el espacio que cuelga, con interletrado `t` mide lo mismo que sin él más `(N − 1) × t / 1000 × tamaño`, ±0,01 px, salvo una línea donde se forma una ligadura con `t` 0, que cambia sus clústeres, y `"ab  "` a la derecha con `t` 500 termina `b` en el borde; en Manrope, porque Inter no tiene `liga`, con `t` distinto de 0 no se forma la ligadura opcional que sí se forma con 0; (f) en líneas de una sola fuente, con interlineado 2,0 la distancia entre líneas base es el doble que con 1,0, ±0,01 px; con Inter y Noto Emoji en cada línea, a 100 px, la distancia es 61,74, 121,29 y 241,99 ±0,01 px con 0,5, 1,0 y 2,0; (g) con Inter como primaria y la fuente OFL de árabe o hebreo como reserva, un texto mixto usa cada fuente en sus clústeres; sin reserva, usa `.notdef` y lo indica; si dos fuentes cubren parte de un clúster, `.notdef` sale de la que cubre más, y en un empate de la primera; (h) `straße` en mayúsculas da `STRASSE` con cualquier configuración regional; (i) una línea justificada termina en el borde de la caja, ±0,01 px, y la última línea del párrafo no se estira; (j) una lista de 3 niveles pone el marcador del nivel 1 en el borde y sangra 1,5 em por nivel, ±0,01 px, y numera 1, 2, 3 en el mismo nivel; numerado, numerado, viñeta y numerado del mismo nivel dan `1.`, `2.`, `•` y `3.`; `9.` y `10.` empiezan su texto en la misma x, y la línea partida de un elemento también, ±0,01 px; con Inter, `1.` y 12 pt, el colgado es proporcional al DPI, ±0,01 %, hasta 1,3e39 DPI, por debajo del límite aceptado del colgado; (k) en una caja de 100 px, `"ab  "` a la derecha termina `b` en el borde, ±0,01 px, y centrado queda como `"ab"`; (l) posiciones y avances son iguales bit a bit con cualquier modo de hinting; (m) con Inter y la fuente OFL de árabe como reserva, a 96 DPI, alineado a la izquierda e interletrado 0, `abc ابج دهو def` fija el límite del espacio final de otro nivel: a 12 pt en una caja de 40 px, la segunda línea pone ese espacio en x = 0 y su contenido empieza en 4,50, sin desborde; a 9 pt en una caja de 39,3 px, la primera línea termina su contenido en 41,56, 2,26 px fuera de la caja, sin desborde. A 9 pt en la misma caja, `abc ابج def`, cuyo espacio final tiene el nivel del párrafo, termina su contenido en 38,19 y deja ese espacio detrás. Con `ابج\n` delante de cada texto, sus líneas dan las mismas medidas. Medidas ±0,01 px; (n) con Inter y la fuente OFL de árabe como reserva, a 96 DPI, 12 pt, interletrado 0 y una caja de 100 px, `ابج\nabc ابج def` da dos párrafos: la línea 1 es de derecha a izquierda y la línea 2 se ordena de izquierda a derecha, con `a`, `b`, `c`, espacio, `ج`, `ب`, `ا`, espacio, `d`, `e`, `f`, en las mismas x que ese texto solo. Da lo mismo con `\r`, `\r\n`, U+2028 y U+2029 en lugar de `\n`; con U+0085 sigue siendo un solo párrafo de derecha a izquierda. Justificado: `ابج\nab cd` termina la línea 1 en el borde derecho y empieza `a` en x = 0; `abc\nابج` termina la línea 2 en el borde derecho; `ابج\n123` empieza `1` en x = 0, porque ese párrafo no tiene carácter fuerte. Con Izquierda, los espacios de `ابج\nab  ` quedan a la derecha de `b`, y los de `abc\nابج  `, en x negativa, con la tinta desde x = 0. Medidas ±0,01 px.

**FD-42 · Color y degradado de texto** (RF-24)
- *Propósito*: colorear con coherencia.
- *Comportamiento y reglas*: selector con búsqueda por nombre o HEX, **cuentagotas**, colores del documento, colores de la Identidad visual, sólidos y degradados predefinidos; degradados lineal y radial con paradas editables. Los colores de la Identidad se enlazan (`ColorRef::Swatch`) y siguen sus cambios. Advertencia de contraste si el texto resulta < 4,5:1 contra el fondo detectado (no bloquea).
- *UI*: Barra contextual; Inspector → Texto.
- *Técnica*: `gafira-color` (muestras, degradados), `Interpolate` para `GradientStops` (02 §3.4).
- *Presupuesto*: cambio de color < 50 ms (CAL-03).
- *Aceptación*: (a) cambiar una muestra de la Identidad cambia todos los textos enlazados en < 50 ms; (b) el cuentagotas toma el color con error ΔE2000 < 1,0 en el golden; (c) la advertencia aparece con ratio < 4,5:1.

**FD-43 · Efectos de texto** (RF-25)
- *Propósito*: dar carácter gráfico al texto.
- *Comportamiento y reglas*: **Contorno** (grosor, color, posición exterior/interior), **Sombra** (FD-35), brillo, eco, fondo (cuadro de color con relleno y radio), «sin relleno» (solo contorno), desalineado y neón (presets). Se combinan con degradado. Los efectos son estilos con datos y no destruyen el texto: el texto sigue siendo editable y seleccionable.
- *UI*: Panel lateral → Texto → Efectos de texto; Inspector → Texto.
- *Técnica*: `gafira-text` (contornos), `gafira-pathops` (trazo), `Effect`.
- *Presupuesto*: aplicar < 100 ms (CAL-05).
- *Aceptación*: (a) un contorno de 4 px no cambia el layout del texto; (b) el efecto se conserva al exportar SVG (como trazos y filtros) y PDF (VERIFICAR efectos complejos: se aplanan si no son representables); (c) el texto exportado a PDF sigue siendo seleccionable cuando el efecto lo permite (RF-05 del PRD).

**FD-44 · Curvar** (RF-25)
- *Propósito*: texto sobre arco o curva.
- *Comportamiento y reglas*: slider **Curvar** de −100 a 100 (0 = recto) para arcos; en Fase 2, texto sobre una forma vectorial elegida y texto vertical (confirmada, D-80). El texto sigue editable; la curva afecta a la geometría de pintado, no al contenido.
  - **Escala** (D-26): `θ = valor × 1,8°`, de modo que ±100 son ±180° de arco. Un valor positivo da un arco abierto hacia abajo (∩) y uno negativo, abierto hacia arriba (∪) (confirmada, D-71).
  - **Geometría**: `L` es el avance de la línea sin el espacio que cuelga, y `s` es la posición en ese contenido: cuenta desde su inicio y no incluye el avance de ese espacio. Vale también para el espacio final de otro nivel, aunque quede dentro de la línea (FD-41). Ese espacio no tiene lugar en el arco. Una sola línea tiene un arco de longitud `L`, así que `radio = L / |θ|` con `θ` en radianes. El arco se centra en el ancho de la caja. Cada glifo se coloca por el centro de su avance y gira según la tangente. Una línea con `L = 0` no cambia.
  - **Varias líneas** (confirmada, D-71): usan arcos concéntricos, con un solo centro y el mismo `θ`. Sean `σ` el signo de `θ` y, para las líneas con `L_i > 0`, `L_i` su avance y `b_i` su línea base, con y hacia abajo; `b_ref` es la línea base de una de ellas, y el resultado no depende de cuál. El radio de referencia es `R_ref = máx_i(L_i / |θ| + σ(b_i − b_ref))`, y cada línea tiene `R_i = R_ref − σ(b_i − b_ref)`. El centro está en `y = b_ref + σ R_ref`. Así, `R_i × |θ| ≥ L_i`: ninguna línea queda por debajo de su propio avance y todo radio es positivo. La línea que da el máximo fija el radio y tiene `R_i = L_i / |θ|`; una sola línea queda como arriba. Cada línea cubre todo `θ`, con su propio `s / L_i`. Un arco de muchas líneas puede pasar del ancho de la caja.
- *UI*: Inspector → Texto → Curvar.
- *Técnica*: `gafira-text` (texto en curva y conversión a contornos), `gafira-pathops`.
- *Presupuesto*: actualizar el slider < 16 ms por evento con 100 caracteres (02 §6.7, `bench_text_curve_100`).
- *Aceptación*: (a) Curvar 0 equivale al texto recto (diferencia 0); (b) la exportación SVG convierte a contornos y mantiene la forma; (c) al cambiar el texto se rehace la curva; (d) con ±100, en una sola línea o en la línea que fija el radio, la cuerda del arco mide `2L / π`, ±0,01 px; en toda línea, un glifo cuyo centro está en la posición `s` gira `(s / L − 0,5) × θ` con el `L` de su línea, ±1e-6 rad; con +50 y −50 las rotaciones son opuestas; (e) ±100,01 dan error; una línea vacía o solo de espacio que cuelga no cambia nada; con 0,01 toda posición es finita; (f) con 600 caracteres, 12 pt y 96 DPI en una caja de 300 px, con Curvar 100 ninguna línea tiene radio ≤ 0; ahí y con −100, toda línea cumple `R_i × |θ| ≥ L_i`, ±0,01 px, todas comparten centro y una cumple la igualdad; (g) con Curvar 100 en la misma caja, `Hello ` y `Hello` dan a sus cinco glifos las mismas posiciones, ±0,01 px, y los mismos giros, ±1e-6 rad; lo mismo con un texto árabe de una línea, con y sin espacio final.

**FD-45 · Columnas** (RF-26)
- *Propósito*: repartir un texto largo en varias columnas dentro de un cuadro.
- *Comportamiento y reglas*: función propia. `TextNode.columns: TextColumns { count, gap, balance }` (02 §3.2): **1–8 columnas** (`count = 1` es sin columnas), espacio entre columnas `gap` (0–200 unidades), opción `balance` de columnas equilibradas y reparto automático por altura del cuadro; las columnas son de igual ancho: `(ancho − (count − 1) × gap) / count`, que debe ser ≥ 1 px. Solo aplica con tamaño de cuadro fijo o de alto automático. Salto de columna manual: Fase 2 (D-24). Funciona con Listas y con justificado. Si el contenido no cabe, el cuadro muestra un indicador de desborde (no se pierde texto).
  - **Reparto**: con `balance` apagado y tamaño fijo, cada columna se llena hasta el alto del cuadro y sigue en la siguiente. Con `balance`, el texto se reparte para minimizar la diferencia de alturas. Con alto automático, las columnas siempre se equilibran y el alto del cuadro sale del resultado.
  - **Tamaño automático de ancho**: las columnas no aplican y `count` cuenta como 1.
  - **Una columna**: `gap` se valida igual y no tiene efecto. En un cuadro fijo, el indicador de desborde aparece si el texto es más alto que el cuadro, aunque `layout_box` no lo marque: (e) fija la maquetación, no ese indicador.
- *UI*: Inspector → Texto → Columnas.
- *Técnica*: `gafira-text::layout_box` con `TextColumns` (campo `columns` de `TextNode`, 02 §3.2).
- *Presupuesto*: re-maquetar 5 000 caracteres en 4 columnas < 16 ms (02 §6.7, `bench_text_columns_5000`).
- *Aceptación*: (a) con 3 columnas y texto de 3 000 caracteres, la altura de las columnas difiere ≤ 1 línea al equilibrar; (b) sin criterio en el MVP: el salto manual es de Fase 2 (D-24); (c) el indicador de desborde aparece cuando corresponde; (d) `count` 0 o 9, `gap` negativo o mayor que 200, también con `count = 1`, y un ancho de columna < 1 px dan error; (e) `count = 1` da la misma maquetación bit a bit que sin columnas, con cualquier `gap` válido.

**FD-46 · Ajustar al cuadro** (RF-26)
- *Propósito*: que el texto llene su cuadro sin desbordar.
- *Comportamiento y reglas*: función propia. Con `TextNode.autofit = Autofit::FitBox { min_size, max_size }` (02 §3.2), `gafira-text` busca por bisección en el intervalo (por defecto 6–400 pt), sobre una rejilla formada por los múltiplos de 0,25 pt, `min_size` y `max_size`. Busca un tamaño `r` que cabe y cuyo siguiente en la rejilla no cabe, o `r = max_size`. El tamaño resultante es un **valor derivado**: no se guarda como dato del usuario y es determinista para las mismas fuentes. Si no cabe ni al mínimo, `r = min_size` y muestra el indicador de desborde. Se desactiva al cambiar el tamaño a mano.
  - **Cabe**: la altura maquetada es ≤ el alto del cuadro (la igualdad cabe) y no hay desborde de clúster (FD-41). El ajuste de palabras no es monótono, así que `r` es un máximo local de la rejilla, no siempre el mayor absoluto.
  - **Tramos de varios tamaños**: `r` es el tamaño del tramo mayor; los demás escalan en la misma proporción.
  - **Tamaño del cuadro**: solo aplica con tamaño fijo; con ancho o alto automático el tamaño no cambia.
  - **Fijar tipografía** (D-25): con `type_lock.size` activo (FD-47), el tamaño no cambia y el interruptor se atenúa con el motivo «Con el tamaño fijado no se puede ajustar al cuadro.».
- *UI*: Inspector → Texto → Ajustar al cuadro (interruptor).
- *Técnica*: `gafira-text` (medición y bisección); el cambio de modo es un `Command` `SetProperty<Autofit>`; el tamaño derivado no genera `Command`.
- *Presupuesto*: < 16 ms por recálculo en cuadros de 500 caracteres (02 §6.7, `bench_text_fit_500`).
- *Aceptación*: (a) prueba de propiedad: `r` cabe y el siguiente de la rejilla no cabe, salvo `r = max_size`; (b) con texto vacío, no cambia ni hay desborde; (c) respeta el mínimo y el máximo: `r` está en el intervalo, un texto que no cabe da `min_size` con desborde y uno que siempre cabe da `max_size`; (d) con `type_lock.size`, el tamaño no cambia y el interruptor se atenúa con el motivo; (e) `min_size = max_size` da ese tamaño; `min_size > max_size` o fuera de 1–1000 dan error; (f) con dos tramos de 10 y 20 pt, el resultado conserva la relación 1:2.

**FD-47 · Fijar tipografía** (RF-26)
- *Propósito*: evitar que fuente, tamaño o estilo cambien por accidente.
- *Comportamiento y reglas*: función propia. `TextNode.type_lock: TypeLock { font, size, style }` (02 §3.2) marca qué propiedades se protegen: **fuente**, **tamaño** y **estilo** (peso, cursiva e interletrado). Los comandos que cambian estilos de forma masiva —`SetSwatch` sobre fuentes de la identidad, Aplicar identidad (FD-52), Adaptar este diseño y Duplicar y adaptar con reflujo (`ResizeDesign`, FD-54) y la sustitución masiva de fuentes— no modifican las propiedades marcadas. Las **ediciones directas** del usuario sobre ese texto sí se aplican. Se indica con un candado en la Barra contextual. Se puede fijar un solo texto o una selección. Fijar no impide editar el contenido.
- *UI*: Inspector → Texto → Fijar tipografía; Capas (icono).
- *Técnica*: campo `TextNode.type_lock` (02 §3.2), respetado por `SetSwatch`, `ApplyIdentity` y `ResizeDesign`.
- *Presupuesto*: sin coste medible.
- *Aceptación*: (a) aplicar una identidad no cambia la fuente de un texto fijado; (b) adaptar el diseño ajusta el cuadro pero no la tipografía; (c) una edición directa de la fuente sobre un texto fijado sí se aplica; (d) quitar el candado devuelve el comportamiento normal.

**FD-48 · Fuentes: detección, sustitución y licencias** (RF-27)
- *Propósito*: abrir proyectos sin romper el diseño cuando falta una fuente.
- *Comportamiento y reglas*: Gafira detecta las fuentes del sistema en segundo plano (nunca en el arranque; 02 §6.7). Si falta una fuente al abrir, propone **sustitutos compatibles** (misma clase y métricas similares), conserva el layout y **recuerda la fuente original** para volver a usarla si se instala. «Sustitución masiva» cambia una fuente por otra en todo el proyecto. Fuentes del usuario: advertencia de licencia y compatibilidad al usarlas («Esta fuente no está incluida. Comprueba que su licencia permite tu uso.»); una fuente del usuario se incrusta en el proyecto según su licencia (bits `fsType` de OpenType) y según el perfil del `.gaf` (D-67); si la licencia no lo permite, se vincula por nombre y, al abrir el proyecto en un equipo que no la tiene, se avisa de la fuente que falta (D-60). Qué hace cada perfil del `.gaf` con una fuente incrustable se fija con la §17 de la spec 008.
- *UI*: Texto → Fuentes; diálogo al abrir; Archivo → Empaquetar fuentes (confirmada, D-80).
- *Técnica*: `gafira-platform` (enumeración), `gafira-text` (métricas y clase de la fuente), `gafira-engine` (elige el sustituto), `FontCollector` (Visitor, 02 §4.1).
- *Presupuesto*: la detección completa no bloquea el arranque (CAL-08).
- *Aceptación*: (a) abrir un proyecto con una fuente ausente propone sustituto y no desplaza más de 2 % las cajas de texto; (b) al instalar la fuente original, se restaura sin edición; (c) las fuentes de egui por defecto no se incluyen (ARQ-62).

**FD-49 · Buscar y reemplazar texto** (RF-27)
- *Propósito*: cambiar textos repetidos en todo el proyecto.
- *Comportamiento y reglas*: `Ctrl/Cmd+F`; búsqueda en todas las páginas con opciones de mayúsculas y palabra completa; «Reemplazar» y «Reemplazar todo» son un solo `Command`; muestra el número de coincidencias.
- *UI*: Edición → Buscar y reemplazar.
- *Técnica*: `NodeVisitor` de búsqueda; `Command` `ReplaceText`.
- *Presupuesto*: < 100 ms con 20 páginas.
- *Aceptación*: (a) reemplazar todo cambia todas las coincidencias y un Deshacer las revierte; (b) las coincidencias en páginas ocultas se señalan.

---

## 11. Identidad visual

**FD-50 · Identidad visual** (RF-28)
- *Propósito*: guardar y reutilizar la identidad de una marca, cliente o proyecto.
- *Comportamiento y reglas*: una **Identidad** contiene logos, colores con nombre, tipografías por rol (titular, cuerpo), Plantillas propias, gráficos, iconos, notas de voz y **Estilos**. Se gestiona en la pantalla Identidad visual (03 §7.7) y se usa desde la pestaña **Identidad** del Panel lateral: clic en un color lo aplica; sus tipografías aparecen primero en la pestaña Texto. Importar un logo valida formato (SVG preferido, PNG con transparencia).
- *UI*: pantalla Identidad visual; Panel lateral → Identidad.
- *Técnica*: `VisualIdentity`, `IdentityProvider` (02 §4.11), `IdentityService`.
- *Presupuesto*: abrir la pantalla < 100 ms (CAL-05).
- *Aceptación*: (a) crear una identidad, añadir 5 colores y 2 fuentes persiste tras guardar y abrir; (b) las fuentes de la identidad se listan primero; (c) un logo de formato no admitido muestra el mensaje estándar.

**FD-51 · Estilos (variables de estilo)** (RF-28)
- *Propósito*: cambiar un valor una vez y que se refleje en todo el diseño.
- *Comportamiento y reglas*: **Estilos** con nombre para color, texto y efecto (por ejemplo «Color principal», «Título»). Los elementos se enlazan por referencia (`ColorRef::Swatch`, `StyleRef`). Cambiar un estilo actualiza todos los elementos enlazados con un único `Command` (`SetSwatch`). Se puede desvincular un elemento. Los estilos se exportan con la Identidad.
- *UI*: Identidad visual → Estilos; Barra contextual (icono de enlace).
- *Técnica*: `StyleSheet`, `StyleRef`, `SetSwatch` (02 §3.2, §4.3).
- *Presupuesto*: cambio de color enlazado < 50 ms con 100 elementos (CAL-03).
- *Aceptación*: (a) cambiar un color actualiza todos sus usos y se deshace de una vez; (b) un elemento desvinculado no cambia; (c) los estilos sobreviven a guardar y abrir.

**FD-52 · Varias identidades y Aplicar** (RF-29)
- *Propósito*: cambiar la identidad de un diseño con un clic y deshacerlo.
- *Comportamiento y reglas*: selector de **Identidades** (varias por proyecto o biblioteca); **Aplicar a este diseño** reemplaza colores y fuentes mapeadas conservando la jerarquía y las posiciones; los textos con Fijar tipografía no cambian (FD-47); los elementos bloqueados de la identidad se respetan. Es una transacción que se deshace de una vez. **Validación**: informe de contraste (< 4,5:1 para texto), color fuera de la paleta y fuentes ausentes, sin bloquear. Opción de **bloquear elementos de identidad** (logos, colores) para evitar cambios accidentales.
- *UI*: Identidad visual → «Aplicar a este diseño»; selector de identidad en la pestaña Identidad.
- *Técnica*: `Command` `ApplyIdentity` (composite), `IdentityService`; contraste con `gafira-color` (ARQ-61).
- *Presupuesto*: aplicar en un diseño de 100 elementos < 300 ms.
- *Aceptación*: (a) aplicar y deshacer devuelven el estado inicial exacto (hash del documento); (b) el informe de contraste coincide con el cálculo WCAG; (c) los textos con Fijar tipografía no cambian.

---

## 12. Cambiar tamaño

**FD-53 · Cambiar tamaño: Destinos y vista previa** (RF-30)
- *Propósito*: preparar el mismo diseño para varios destinos. Llega en un change posterior a `gafira-text` (D-14).
- *Comportamiento y reglas*: el panel (03 §7.6) ofrece búsqueda, sugerencias (según el formato actual) y **Destinos**: Redes, Impresión, Presentaciones, Video, Web y Personalizado; selección múltiple de tamaños; **tamaño personalizado** (Ancho, Alto, unidades, candado de proporción, orientación). La vista previa muestra, por cada destino, cómo quedaría la composición antes de aplicar. Los tamaños exactos en píxeles están en la tabla siguiente. **Las medidas de plataformas cambian: VERIFICAR contra la plataforma vigente al publicar.**

| Destino | Tamaño | Px (ancho × alto) |
|---|---|---|
| Redes | Publicación cuadrada 1:1 | 1080 × 1080 |
| Redes | Publicación vertical 4:5 | 1080 × 1350 |
| Redes | Historia, Reel o video corto 9:16 | 1080 × 1920 |
| Redes | Publicación horizontal 1,91:1 | 1080 × 566 |
| Redes | Portada de red profesional | 1584 × 396 |
| Redes | Encabezado de perfil 3:1 | 1500 × 500 |
| Impresión (300 ppp) | A4 (210 × 297 mm) | 2480 × 3508 |
| Impresión (300 ppp) | A5 (148 × 210 mm) | 1748 × 2480 |
| Impresión (300 ppp) | A3 (297 × 420 mm) | 3508 × 4961 |
| Impresión (300 ppp) | Carta (8,5 × 11 in) | 2550 × 3300 |
| Impresión (300 ppp) | Tarjeta de visita (3,5 × 2 in) | 1050 × 600 |
| Impresión (300 ppp) | Postal (10 × 15 cm) | 1181 × 1772 |
| Presentaciones | Panorámica 16:9 | 1920 × 1080 |
| Presentaciones | Estándar 4:3 | 1440 × 1080 |
| Video | Horizontal 16:9 (FHD) | 1920 × 1080 |
| Video | Vertical 9:16 | 1080 × 1920 |
| Video | Cuadrado 1:1 | 1080 × 1080 |
| Video | Vertical 4:5 | 1080 × 1350 |
| Video | Ultra panorámico 21:9 | 2560 × 1080 |
| Video | 4K UHD 16:9 | 3840 × 2160 |
| Web | Imagen para compartir (1,91:1) | 1200 × 630 |
| Web | Encabezado de página | 1920 × 600 |
| Web | Icono cuadrado | 512 × 512 |

- *UI*: Barra superior → Tamaño; Archivo → Nuevo; Panel Cambiar tamaño.
- *Técnica*: `gafira-layout` (reflujo, `ResizeDesign`), `gafira-render` para la vista previa; lista de Destinos como datos (`assets/sizes.toml`).
- *Presupuesto*: la vista previa de 5 destinos < 500 ms (confirmada, D-71); aplicar < 300 ms.
- *Aceptación*: (a) cada tamaño de la tabla se crea con las dimensiones exactas; (b) la vista previa coincide con el resultado final en ≤ 1/255; (c) el tamaño personalizado respeta unidades y proporción.

**FD-54 · Duplicar y adaptar / Adaptar este diseño** (RF-30)
- *Propósito*: producir versiones en otros tamaños conservando la jerarquía visual. Llega en un change posterior a `gafira-text` (D-14).
- *Comportamiento y reglas*: **Duplicar y adaptar** crea una página nueva por cada destino elegido y conserva el original; **Adaptar este diseño** cambia el tamaño de la página actual (Deshacer lo revierte). El reflujo: escala uniforme de la composición al área útil, reposiciona los elementos por anclajes (esquinas, centro, bordes) y proporciones, mantiene el orden de lectura, respeta Márgenes y Zona de seguridad, reajusta cuadros de texto (con Ajustar al cuadro) y no toca tipografías fijadas. Tras adaptar, el ajuste manual es posible. Criterio del PRD: pasar de 1080×1920 a 1080×1080 conserva textos, imágenes principales y jerarquía.
- *UI*: Cambiar tamaño (los dos botones).
- *Técnica*: `Command` `ResizeDesign` (composite), que recibe el resultado de `gafira-layout::reflow`. `reflow` recibe una función de medida inyectada, porque `gafira-layout` no depende de `gafira-text` (ADR 0007).
- *Presupuesto*: adaptar un diseño de 50 elementos a 5 destinos < 1 s (confirmada, D-71).
- *Aceptación*: (a) en el conjunto de prueba (20 plantillas), ningún texto queda fuera de la Zona de seguridad tras adaptar 1080×1920 → 1080×1080; (b) el orden de lectura se conserva; (c) Duplicar y adaptar no modifica el original (hash idéntico); (d) un solo Deshacer revierte la adaptación.

---

## 13. Exportar, Versiones, autoguardado y recuperación

La pantalla **Exportar** (03 §7.5) genera su panel de opciones desde `Exporter::options_schema` (02 §4.5) y habilita solo los formatos que el documento admite (`ExportCaps`). Las exportaciones se encolan en la **Cola de exportación** (02 §6.4): el trabajo sigue si la persona edita otra cosa, se escribe en un archivo temporal y se renombra al terminar. Al acabar: «Tu diseño está listo para compartir» y «Mostrar en carpeta». «Recordar estos ajustes» guarda las opciones por formato.

**Reglas de códecs (D-02, D-07).** Cuando el documento contiene video, Exportar ofrece los formatos de la spec 05 (FV-12): **WebM y MP4 con AV1 + Opus por defecto** (libres de regalías); MP4 H.264/AAC solo si el sistema operativo lo permite (Media Foundation, VideoToolbox o VA-API) y, si no, Gafira avisa y propone AV1 + Opus. No hay salida HEVC ni MP3. Los formatos de imagen de esta sección (PNG, JPEG, WebP, GIF animado, SVG, PDF, y PPTX en Fase 2) no cambian. No existe ninguna opción que consulte la red.

**FD-55 · Exportar imagen: PNG, JPEG, WebP, GIF y SVG** (RF-31)
- *Propósito*: obtener archivos para compartir o continuar en otro programa.
- *Comportamiento y reglas*: 

| Formato | Opciones | Reglas |
|---|---|---|
| **PNG** | **Escala** (0,5×, 1×, 2×, 3×, 4× o tamaño en px; muestra la resolución resultante); **Calidad**: Comprimir (rápido), Alta o «Limitar tamaño» (objetivo en MB); **Fondo transparente** | Perfil sRGB incrustado; con fondo transparente se conserva el canal alfa; límite de 16 384 px por lado (confirmada, D-71), exportación por tiles |
| **JPEG** | Calidad 1–100 (por defecto 90), submuestreo (4:4:4 o 4:2:0), progresivo | Sin transparencia: se compone sobre el color de fondo de la página (blanco por defecto) |
| **WebP** | Con pérdida (calidad 1–100) o sin pérdida; transparencia | Misma escala que PNG |
| **GIF animado** | Fotogramas por segundo (1–30), bucle (infinito o n), paleta (hasta 256 colores) y tramado | Cada página con `duration` es un cuadro; o el rango de una secuencia (05). Aviso de peso por encima de 15 MB |
| **SVG** | Texto como contornos (por defecto) o como texto; imágenes incrustadas | Los efectos no representables se rasterizan con aviso; sin referencias externas |

- *UI*: Exportar → Formato.
- *Técnica*: `Exporter` en `gafira-io-image` (PNG, JPEG, WebP, GIF) y `gafira-io-svg`; render en `RenderQuality::Export` (f32 en luz lineal, CPU por defecto; 02 §6.2), con el bucle de frame secuencial de hoy y el paralelismo por tiles del lado de `gafira-render` (02 §6.7).
- *Formatos*: PNG, JPEG, WebP, GIF animado y SVG en el MVP.
- *Presupuesto*: **PNG de 1080p < 2 s** (CAL-11); JPEG y WebP de 1080p < 3 s (confirmada, D-71); exportación en segundo plano sin bloquear la UI.
- *Aceptación*: (a) `bench_export_png_1080` < 2 s; (b) un PNG con fondo transparente tiene alfa 0 donde no hay contenido (comparación de píxeles); (c) la vista previa y la exportación difieren ≤ 1/255 en el 99,9 % de los píxeles (ARQ-42, CAL-34); (d) cancelar devuelve el control en ≤ 100 ms sin dejar archivo parcial (ARQ-43, ARQ-50).

**FD-56 · Exportar PDF (RGB y CMYK)** (RF-32)
- *Propósito*: entregar documentos para pantalla o impresión.
- *Comportamiento y reglas*: preajustes **Digital** (imágenes a 150 ppp efectivos como máximo, compresión JPEG de calidad 85) e **Impresión** (imágenes a 300 ppp, **Sangrado** y **marcas de corte** activados). Opciones: **perfil de color RGB** (MVP) o **CMYK** (Fase 2, con perfil ICC y conversión `RGB → CMYK`); **Sangrado y marcas de corte** (FD-05); **Aplanar** (rasteriza transparencias y efectos no representables); **Incluir notas** (FD-23); **orden de lectura accesible** (PDF etiquetado, texto alternativo por imagen y orden de Capas como orden de lectura); **Proteger con contraseña** (contraseña de apertura y de permisos, cifrado AES-256; VERIFICAR soporte en la biblioteca elegida o crate permisivo). El texto se exporta como **texto seleccionable** con subconjuntos de fuente incrustados cuando la licencia de la fuente lo permite (bit de incrustación `fsType`); si no, se convierte a contornos con aviso. Varias páginas en un solo PDF. PDF/X queda en VERIFICAR (C-17 de 01).
- *UI*: Exportar → PDF → Avanzado.
- *Técnica*: `gafira-io-pdf` (krilla 0.8.2, MIT OR Apache-2.0: verificado (2026-11-16, crates.io y `cargo-deny` 0.20.2 con el `deny.toml` del repositorio); alternativa `pdf-writer`, MIT OR Apache-2.0), `ColorTransform` con `IccTransform` (moxcms) para CMYK.
- *Formatos*: PDF RGB en MVP; PDF CMYK en Fase 2.
- *Presupuesto*: PDF de 20 páginas < 5 s (confirmada, D-71); progreso por página.
- *Aceptación*: (a) con Sangrado de 3 mm el PDF contiene `TrimBox` y `BleedBox` correctos y marcas de corte fuera del sangrado (validación estructural); (b) el texto del PDF es seleccionable (extracción devuelve el contenido); (c) el PDF con contraseña no abre sin ella (prueba con lector de referencia, VERIFICAR); (d) el golden de estructura no cambia sin aprobación.

**FD-57 · Alcance de la exportación: página, selección o rango** (RF-31)
- *Propósito*: exportar solo lo necesario.
- *Comportamiento y reglas*: «Todas las páginas», «Página actual», «Selección» (exporta los elementos seleccionados recortados a su caja) y «Rango» (por ejemplo `1-3, 5`). Varios archivos se guardan en una carpeta con un patrón de nombre configurable (`{proyecto}-{n}`); el PDF puede ser un único archivo multipágina. Las páginas ocultas se omiten (FD-02). El diálogo muestra el número de archivos y el tamaño estimado.
- *UI*: Exportar → Páginas.
- *Técnica*: `ExportRequest::builder()` (02 §5.3), `ExportJob`.
- *Presupuesto*: el cálculo del tamaño estimado < 100 ms.
- *Aceptación*: (a) el rango `1-3, 5` exporta exactamente las páginas 1, 2, 3 y 5; (b) la selección exporta solo los elementos elegidos sin fondo de página; (c) los nombres no colisionan (sufijo numérico).

**FD-58 · Validación previa y escritura segura** (RF-33)
- *Propósito*: no producir archivos corruptos ni resultados inesperados.
- *Comportamiento y reglas*: antes de encolar, `Exporter::validate` comprueba: fuentes ausentes, recursos ausentes, resolución efectiva baja en Impresión (< 150 ppp), texto desbordado, contenido fuera de la Zona de seguridad, páginas vacías, recursos con licencia «Desconocida», perfil de color incompatible y límite de tamaño. Los **errores** (recurso ausente, tamaño inválido) impiden exportar; las **advertencias** se muestran y permiten continuar. Se escribe a `*.tmp`, se verifica la integridad (relectura del encabezado y tamaño) y se renombra. Cada exportación registra ruta, configuración, duración, tamaño y errores en el registro local de exportaciones. Reintentos automáticos solo ante errores transitorios (02 §6.4).
- *UI*: diálogo Exportar (lista de avisos antes del botón).
- *Técnica*: `Exporter::validate`, cola de exportación (02 §6.4), `FileSystem` (escritura atómica, ARQ-50).
- *Presupuesto*: validación < 300 ms para 20 páginas.
- *Aceptación*: (a) si falta un recurso, la exportación **no empieza** y el mensaje lo nombra (criterio del PRD RF-09); (b) matriz `export_matrix` (≥ 200 combinaciones) con ≥ 98 % de éxito (CAL-17); (c) un corte de energía simulado durante la escritura no deja un archivo final parcial (ARQ-50).

**FD-59 · Historial: Deshacer y Rehacer** (RF-04)
- *Propósito*: probar sin miedo.
- *Comportamiento y reglas*: Deshacer y Rehacer (`Ctrl+Z`, `Ctrl+Shift+Z`) con botones visibles y tooltip «Deshacer {acción}». El historial es híbrido (instantánea `Arc<Document>` + `Command`, ADR-03) con presupuesto de 512 MB (confirmada, D-71) y compactación de entradas antiguas. Los gestos continuos se fusionan (ARQ-33). Una lista **Historial** (Edición → Historial) permite saltar a cualquier estado (confirmada, D-80). El historial no se guarda en el archivo y se pierde al cerrar (las Versiones sí persisten, FD-60).
- *UI*: Barra superior (↶ ↷); Edición → Historial.
- *Técnica*: `History`, `Transaction` (`gafira-edit`), ADR-03.
- *Presupuesto*: Deshacer/Rehacer < 16 ms p99 en 100 capas (CAL-28).
- *Aceptación*: (a) 1 000 deshacer consecutivos < 16 ms p99 (ARQ-32); (b) tras compactar, Deshacer sigue funcionando por `revert`; (c) el historial respeta el presupuesto de memoria (CAL-29).

**FD-60 · Versiones nombradas** (RF-04)
- *Propósito*: guardar hitos del trabajo y volver a ellos.
- *Comportamiento y reglas*: Archivo → Versiones (o botón **Versiones** de la Barra superior) lista las versiones. La persona guarda **versiones nombradas** («Guardar versión…»); además Gafira crea **una versión automática** al guardar si la última automática tiene más de 30 min, y conserva las **20 más recientes** (cifras confirmadas en 02 §7.1, D-71). Las versiones **nombradas nunca se borran solas**; solo las borra el usuario. **Restaurar** aplica el `Command` `RestoreVersion` (deshacible); antes de restaurar se guarda una versión automática del estado actual para no perder nada. Cada versión guarda los JSON de ese momento (`document.json`, `pages/*.json`, `sequences/*.json`) y comparte los recursos por hash BLAKE3 con `assets/` y `masks/` (D-68), por lo que ocupa poco; el recolector de recursos no borra uno que use alguna versión. Se pueden renombrar y borrar. Son distintas de los journals de autoguardado (FD-61).
- *UI*: Barra superior → Versiones; Archivo → Versiones.
- *Técnica*: `ProjectRepository` (`gafira-format`); almacenamiento en `versions/index.json` (`[{ id, name, kind: "named"|"auto", created, app, schema }]`) y `versions/<version_id>/` dentro del `.gaf` (02 §7.1); `Command` `RestoreVersion`.
- *Presupuesto*: guardar una versión < 200 ms con 20 páginas; abrir la lista < 100 ms.
- *Aceptación*: (a) restaurar una versión y deshacer vuelve al estado previo (hash del documento); (b) 20 versiones de un proyecto de 20 páginas añaden menos de 10 MB (confirmada, D-71); (c) las versiones sobreviven a abrir con una versión futura de la app (round-trip, ARQ-11).

**FD-61 · Autoguardado y Recuperación tras fallo** (RF-03)
- *Propósito*: no perder nunca trabajo.
- *Comportamiento y reglas*: autoguardado cada **10 s** por defecto (configurable de 5 a 15 s en Ajustes → Guardado), solo si hay cambios, en un journal de **3 ranuras rotativas** (02 §7.2), sobre una instantánea `Arc` en el hilo de E/S sin bloquear la UI. El estado visible es «Cambios guardados.» / «Guardando…» / error con acción. Al arrancar tras un cierre inesperado, la pantalla Recuperación tras fallo (03 §7.9) ofrece **Recuperar**, **Abrir el último guardado** y **Descartar**. Un journal corrupto cae a la ranura anterior (checksum BLAKE3). Bloqueo de archivo: si otro proceso tiene el proyecto abierto, se ofrece solo lectura o copia. Los archivos de esquema futuro se abren en solo lectura (ARQ-55).
- *UI*: Barra superior (Estado de guardado); Recuperación tras fallo; Ajustes → Guardado.
- *Técnica*: `ProjectService::recover`, `gafira-format` (journal, `.lock`), `gafira-platform` (`rename` atómico).
- *Presupuesto*: cada autoguardado < 200 ms en el proyecto de 20 páginas (CAL-19); pérdida máxima ≤ 15 s (CAL-18); recuperación del proyecto de 20 páginas < 3 s.
- *Aceptación*: (a) 500 *kills* aleatorios (`SIGKILL`/`TerminateProcess`) dejan siempre un proyecto recuperable con pérdida ≤ intervalo (ARQ-51, ARQ-52); (b) un journal corrupto usa el slot anterior; (c) el diálogo no aparece sin recuperación disponible.

**FD-62 · Exportar PPTX** (RF-62) — Fase 2
- *Propósito*: entregar una presentación que se abre en programas de presentaciones comunes con la fidelidad visual exacta del diseño.
- *Comportamiento y reglas*: cada página se convierte en **una diapositiva con una sola imagen de alta resolución** del tamaño del proyecto (D-07). **No hay objetos editables**: ni cuadros de texto, ni formas, ni imágenes sueltas. Las páginas ocultas se omiten (FD-02), y las **Notas** de la página no se exportan por defecto (ARQ-15); si el exportador ofrece incluirlas como notas del orador, lo declara en su `ExportCaps` y la opción está desactivada. La resolución de la imagen se elige en Exportar (por defecto 2× del tamaño de la página, con límite de 16 384 px por lado, confirmada, D-71). Sin fuentes que incrustar. Escritor propio de ZIP + XML (OOXML) con imágenes PNG o JPEG, sin dependencias externas.
- *UI*: Exportar → PPTX.
- *Técnica*: `Exporter` en el crate `gafira-io-pptx` (Fase 2, feature opcional; 02 §11.2); reutiliza el render de exportación (`RenderQuality::Export`).
- *Presupuesto*: 20 diapositivas < 5 s (confirmada, D-71).
- *Aceptación*: (a) el archivo abre sin error de reparación en al menos tres programas de presentaciones de uso común (VERIFICAR conjunto); (b) cada diapositiva contiene exactamente una imagen y ningún otro objeto; (c) la imagen coincide con la exportación PNG de la misma página en ≤ 1/255 (ARQ-42); (d) notas y cuadrícula no aparecen (ARQ-15).

**FD-63 · Guardar, abrir y proyecto portable** (RF-01, RF-02, RF-35)
- *Propósito*: abrir, guardar y mover proyectos sin pérdida.
- *Comportamiento y reglas*: Guardar (`Ctrl+S`), Guardar como y **Guardar copia** escriben `.gaf` de forma atómica (archivo temporal, `fsync`, renombrado; `.bak` de una generación; 02 §7.2). Cada proyecto lleva un UUID que se genera al crearlo y se guarda en el `.gaf`; la recuperación lo usa, así que sobrevive a renombrar o mover el archivo. «Guardar copia» asigna un UUID nuevo a la copia; «Guardar como» conserva el del proyecto (D-63, 02 §7.1). **Exportar proyecto editable con recursos**: genera un `.gaf` con el perfil `portable-full` (D-67, spec 008 §3), con todos los recursos incrustados y sin vínculos externos para migrar a otro equipo; incluye las fuentes del usuario que su licencia permite incrustar (D-60). Es un guardado, no una exportación: escribe solo el `.gaf` del usuario y sus recursos, y como Guardar, Guardar como y «Guardar copia» está disponible sin licencia y no depende de `can_export` (D-55, 02 ARQ-95). Abrir valida el esquema, ejecuta migraciones por paso y abre en solo lectura un esquema futuro. Al abrir se muestra la primera página en cuanto está lista (carga perezosa por página). Proyectos: crear, abrir, **duplicar, renombrar, archivar y eliminar** desde Inicio → Proyectos (archivar mueve a una carpeta de archivados; eliminar pide confirmación y ofrece enviar a la papelera del SO).
- *UI*: Archivo; Inicio → Proyectos; Barra superior.
- *Técnica*: `ProjectService` (`new_document`, `open`, `save`, `save_as`, `recover`, `close`; 02 §5.4), `ProjectRepository`, migraciones `migrate_n_to_n1` (ARQ-53).
- *Formatos*: `.gaf` (ZIP, `manifest.json` con `format: "gafira-project"`, `formatVersion`, `schemaVersion`, `profile`, `features` y `packageCompleteness`, en camelCase; `document.json` más un JSON por página y por secuencia; recursos `assets/<blake3>.<ext>`; ADR-04, D-68).
- *Presupuesto*: abrir 20 páginas < 3 s (CAL-09); abrir 100 capas < 500 ms (CAL-10); guardar un proyecto de 20 páginas < 2 s (confirmada, D-71).
- *Aceptación*: (a) la serialización es determinista (ARQ-10); (b) un proyecto copiado a otro equipo se abre idéntico sin recursos ausentes; (c) un esquema o un contenedor futuro, o una capacidad requerida desconocida (D-68), abre en solo lectura: el documento se edita en memoria, «Guardar» se rechaza con error de solo lectura antes de tocar el archivo y los cambios solo se conservan con «Guardar como» (ARQ-55, D-76); (d) un fallo a mitad de guardado deja el archivo anterior o el nuevo íntegros (ARQ-50); (e) con la licencia en `Unlicensed`, `Expired` o `Invalid`, Guardar, Guardar como, «Guardar copia» y «Exportar proyecto editable con recursos» escriben su `.gaf` sin aviso de licencia, y el archivo reabierto contiene la última edición (D-55, 02 ARQ-95).

---

## 14. Funciones opcionales y desactivables (fuera del MVP)

**FD-64 · Reescritura de texto con modelo local** (RF-23) — Fase 3, opcional, fuera del MVP
- *Propósito*: sugerir otra redacción de un texto, solo si la persona lo pide.
- *Comportamiento y reglas*: está **desactivada por defecto** (Ajustes → Funciones locales opcionales). Requiere instalar aparte un paquete de modelo con licencia declarada y verificada; nunca se incluye en el instalador (ARQ-76). Solo actúa sobre el texto seleccionado y tras una acción explícita; muestra la propuesta junto al original con «Usar» y «Descartar»; nunca inserta contenido sin confirmación y nunca se abre sola. Todo se procesa en el equipo, sin red. Se puede desactivar y desinstalar en cualquier momento.
- *UI*: Barra contextual → «Reescribir» (solo si está activada); Ajustes → Funciones locales opcionales.
- *Técnica*: trait `TextAssistant` en `gafira-ports` (02 §4.15; opcional, local, desactivable y fuera del MVP), implementación opcional aislada tras una feature; `TextAssistantRegistry` vacío en el MVP (ARQ-26).
- *Presupuesto*: **< 4 s** (CAL-31).
- *Aceptación*: (a) con la función desactivada, ningún elemento de la interfaz la menciona ni consume recursos; (b) el binario base no contiene pesos de modelo (ARQ-76); (c) una prueba con red bloqueada funciona igual (ARQ-04); (d) ninguna propuesta se inserta sin confirmación.

---

## 15. Extensiones del modelo requeridas por esta especificación

Los campos y puntos de extensión que las funciones necesitan **ya están incorporados en 02**. La tabla indica dónde.

| Elemento | Necesidad | Estado en 02 |
|---|---|---|
| `Page.grid` y `Page.notes` | FD-02, FD-07 | Incorporados: `Page { grid: GridSettings, notes: String, … }` (§3.1) |
| `TextNode` | FD-45, FD-46, FD-47 | Incorporados: `columns: TextColumns`, `autofit: Autofit`, `type_lock: TypeLock` (§3.2) |
| `manifest.json` | FD-16 | Incorporado: objeto opcional `template { tags, pageFormat, style, color }` (D-81, §7.1); un lector que no lo conoce lo ignora |
| Almacén de Versiones | FD-60 | Incorporado: `versions/index.json` y `versions/<version_id>/` (§7.1) |
| QR y Gráficos | FD-20, FD-22 | `NodeExtension` internos `gafira.qr` y `gafira.chart` registrados en el binario (ADR-02) |
| `TextAssistant` | FD-64 | Incorporado: trait en `gafira-ports` (§4.15, Fase 3) |
| Exportador PPTX | FD-62 | Incorporado: crate `gafira-io-pptx` (Fase 2, §11.2) |
| Medios vinculados | FD-30 | Incorporados: `manifest.assets[...].linked` y evento `LinksMissing` (§7.1) |
| Destinos de tamaño | FD-53 | `assets/sizes.toml` con licencia registrada (ARQ-75) |

## 16. Matriz de trazabilidad RF → función

| RF | Funciones | RF | Funciones |
|---|---|---|---|
| RF-01 | FD-63 | RF-19 | FD-31 |
| RF-02 | FD-63 | RF-20 | FD-32, FD-33 |
| RF-03 | FD-61 | RF-21 | FD-34..FD-37 |
| RF-04 | FD-59, FD-60 | RF-22 | FD-38, FD-39, FD-40 |
| RF-05 | G-4 (todas) | RF-23 | FD-40, FD-64 |
| RF-06 | FD-01, FD-02 | RF-24 | FD-41, FD-42 |
| RF-07 | FD-03, FD-04, FD-05 | RF-25 | FD-43, FD-44 |
| RF-08 | FD-06, FD-07 | RF-26 | FD-45, FD-46, FD-47 |
| RF-09 | FD-08 | RF-27 | FD-48, FD-49 |
| RF-10 | FD-10, FD-11, FD-13 | RF-28 | FD-50, FD-51 |
| RF-11 | FD-12 | RF-29 | FD-52 |
| RF-12 | FD-09 | RF-30 | FD-53, FD-54 |
| RF-13 | G-5 (Paleta de comandos, spec 03 §9.3) | RF-31 | FD-55, FD-57 |
| RF-14 | spec 03 §3, §4 | RF-32 | FD-56 |
| RF-15 | FD-14, FD-15, FD-16 | RF-33 | FD-58 |
| RF-16 | FD-17..FD-23 | RF-34 | FD-58 (cola básica), spec 05 §B |
| RF-17 | FD-24 | RF-35 | FD-63 |
| RF-18 | FD-25..FD-30 | RF-62 | FD-62 |
| RF-55 | FD-30 | RF-63 | spec 06 (pendiente, se redacta al final) |

## 17. Resumen de formatos de esta especificación

| Dirección | Formato | Fase | Crate | Clase legal |
|---|---|---|---|---|
| Importar | PNG, JPEG, WebP, GIF, TIFF, BMP, QOI | MVP | `gafira-io-image` | Libre de regalías; crates MIT OR Apache-2.0 (`image` 0.25.10 con `png`, `zune-jpeg`, `image-webp`, `gif`, `tiff`, `qoi`): verificado (2026-11-16, crates.io y `cargo-deny` 0.20.2 con el `deny.toml` del repositorio) |
| Importar | SVG | MVP | `gafira-io-svg` | Libre |
| Importar | HEIC (solo lectura, solo con el decodificador del SO; D-59; en Linux solo si el sistema lo ofrece) | Fase 2 | feature `heic-os` | Licenciado por el SO |
| Importar | PDF | Fase 2 | `gafira-io-pdf` o hayro (hayro 0.9.0, Apache-2.0 OR MIT: verificado (2026-11-16, crates.io)) | Libre (VERIFICAR) |
| Importar y exportar | `.gaf` | MVP | `gafira-format` | Propio |
| Exportar | PNG, JPEG, WebP, GIF animado | MVP | `gafira-io-image` | Libre |
| Exportar | SVG | MVP | `gafira-io-svg` | Libre |
| Exportar | PDF RGB | MVP | `gafira-io-pdf` | Libre |
| Exportar | PDF CMYK | Fase 2 | `gafira-io-pdf` + moxcms | Libre |
| Exportar | PPTX (una imagen por diapositiva, sin objetos editables) | Fase 2 | `gafira-io-pptx` | Propio |

## 18. Preguntas abiertas y riesgos específicos

| ID | Tema | Estado |
|---|---|---|
| Q-08 | Alcance y fase de PPTX | Cerrada: D-07 (Fase 2, una imagen por diapositiva; FD-62) |
| Q-10 | Empaquetar fuentes del usuario en `.gaf` | Cerrada: D-60 (según su licencia, bits `fsType`, y según el perfil del `.gaf`; FD-48, FD-63) |
| Q-13 | Importación de AVI | Cerrada: D-07 (Fase 2, solo códecs libres; ver 05) |
| Q-17 | Importar HEIC (y HEVC) vía API del sistema | Cerrada: D-59 (solo importación con el decodificador del SO; FD-31) |
| Q-34 | Aspecto de las fórmulas de FD-33 y catálogo de filtros de FD-34 (D-40) | Cerrada como pregunta: D-78 (pasa a tarea, la revisión visual del ítem 133 de 07) |
| Q-35, Q-36, Q-39, Q-40, Q-41, Q-42 | Decisiones de capa 2 | Cerradas: D-29, D-30, D-33, D-34, D-41 y D-42 |
| R-04 | Aislar sujeto < 2 s con GPU | VERIFICAR (CAL-07) |
| R-A | Cifrado AES-256 de PDF con biblioteca permisiva | VERIFICAR (FD-56) |
| R-B | Licencia de krilla, hayro y qrcode | Cerrada: krilla 0.8.2, hayro 0.9.0 y qrcode 0.14.1 son MIT OR Apache-2.0 (verificado el 2026-11-16, crates.io y `cargo-deny` 0.20.2 con el `deny.toml` del repositorio; 02 §9.2) |
| R-C | Medidas de Destinos por plataforma | VERIFICAR al publicar cada versión (FD-53) |
| R-D | Calidad del reflujo al Adaptar con diseños atípicos | Mitigar con el conjunto de 20 plantillas y ajuste manual (FD-54) |
