# 03 — Diseño visual, navegación e interacción de Gafira

| Campo | Valor |
|---|---|
| Documento | Especificación 03 |
| Estado | Borrador para revisión |
| Alcance | Sistema de diseño (tokens, contraste, modo oscuro), uso del logo en la app, navegación y pantallas (incluidas la activación y los estados de licencia, y la **Pantalla de arranque**, §7.10, D-09), componentes, interacción, accesibilidad, movimiento, microcopy, iconografía, **glosario de interfaz** en español e inglés e **idioma** (§16, D-08) |
| Depende de | `00-decisiones.md` (D-01 a D-09, prevalecen), `02-arquitectura-software.md` (ADR-06, §8: `gafira-theme`, `gafira-ui`, ARQ-60..64; §8.4: Pantalla de arranque, ARQ-65..69, ADR-10; §4.14 y §13: licencia, ARQ-90..96; §4.16 y §14: idioma, `gafira-i18n`, ADR-09, ARQ-97..107) |
| Lo usan | `01-producto-y-calidad.md`, `04-funcionalidad-diseno-imagen-exportacion.md`, `05-funcionalidad-video-y-distribucion.md` (todos usan los nombres del §2 de forma idéntica) |
| Convenciones | `DIS-xx` = requisito de diseño con criterio de aceptación. **PROPUESTA** = decisión no presente en el manual de marca. **VERIFICAR** = dato por confirmar. **FIJADA** = cifra fijada por el titular (00-decisiones, D-03). Las decisiones del titular se citan como D-01..D-09 de `00-decisiones.md`; las propuestas de esta spec llevan el prefijo `PD-` (§15) para no confundirse con ellas |

Gafira es un editor de escritorio local: trabaja sin conexión y su única conexión de red es la de la licencia (D-01). Su principio rector es que el contenido del usuario es el protagonista; su principio de producto, que «La herramienta no debe ser más difícil que la idea que el usuario quiere comunicar». El diseño de la interfaz se limita a lo necesario para que el lienzo y la línea de tiempo respiren: superficies claras, un solo color de acción, iconos de trazo simple y movimiento breve. Gafira es de pago con prueba gratuita y una sola licencia (D-01): la interfaz no tiene funciones bloqueadas por módulo. Licencia y activación: ver spec 06 (pendiente, se redacta al final).

---

## 1. Principios de diseño

| # | Principio | Consecuencia verificable |
|---|---|---|
| P1 | **El contenido manda.** La interfaz es neutra y deja el color al trabajo del usuario. | Coral solo en la acción principal de cada pantalla y en el indicador activo; ningún fondo de color saturado detrás del lienzo. |
| P2 | **Un camino claro: Elegir → Personalizar → Compartir.** | La primera pieza publicable se logra en < 10 min (cifra del PRD, spec 01) y cada etapa tiene una sola acción principal visible. |
| P3 | **Control sin ruido.** Lo simple se ve; lo avanzado aparece al seleccionar. | Los controles avanzados viven en la barra contextual, el Inspector o la Paleta de comandos, no en la barra superior. |
| P4 | **Todo es operable con teclado y tiene nombre accesible.** | DIS-30..DIS-34. |
| P5 | **Nada se pierde.** Cada acción se puede deshacer y el estado de guardado siempre es visible. | Botones de Deshacer y Rehacer visibles; «Cambios guardados.» en la barra superior. |
| P6 | **Color nunca como única señal.** | Cada estado de color lleva icono o texto (DIS-33). |
| P7 | **La marca habla poco y claro.** Frases breves de acción, sin superlativos. | Microcopy del §12; palabras vetadas por el manual («revolucionario», «el mejor», «sin límites») no aparecen. |

Las proporciones orientativas del manual (60 % claros, 25 % grafito, 10 % coral, 5 % verde agua) se aplican al conjunto de la ventana: áreas claras (panel, lienzo, barra inferior), grafito (barra superior, riel de pestañas), coral (acción principal y activo) y verde agua (confirmaciones y acentos de ilustración).

---

## 2. Glosario de interfaz

Este glosario es **normativo**. Define el nombre propio de cada pestaña, menú, panel y herramienta de Gafira. Las specs 01, 04 y 05 usan estos nombres tal cual y no los renombran. La columna «Equivalente genérico» sirve para localizar el concepto en cualquier otro programa; no es texto de interfaz. Gafira se publica en español e inglés (D-08): los nombres en español de §2.1–§2.6 están **aprobados** (D-04) y su versión inglesa, con correspondencia 1:1 fila por fila, está en §2.8 como **PROPUESTA** pendiente de aprobación del titular (Q-25). Los dos idiomas comparten la misma clave (`glossary.<id>`, 02 §14.2).

### 2.1 Pantallas y estructura

| Nombre en Gafira | Tipo | Equivalente genérico |
|---|---|---|
| Inicio | Pantalla | Pantalla de bienvenida / proyectos recientes |
| Pantalla de arranque | Pantalla transitoria al iniciar la app, sin controles salvo en caso de error (nombre PROPUESTA, D-09; §7.10) | Splash screen |
| Editor de diseño | Pantalla | Editor de lienzo |
| Editor de video | Pantalla | Editor con timeline |
| Exportar | Diálogo y pantalla | Exportación |
| Cambiar tamaño | Panel | Redimensionar lienzo |
| Identidad visual | Pantalla/pestaña | Guía de estilo de marca |
| Ajustes | Pantalla | Preferencias |
| Recuperación tras fallo | Diálogo al arrancar | Restaurar sesión |
| Paleta de comandos | Superposición (Ctrl/Cmd+K) | Búsqueda de acciones |
| Barra superior | Región | Barra de título y menús |
| Riel de pestañas | Región | Barra lateral de pestañas |
| Panel lateral | Región | Panel de biblioteca |
| Inspector | Región (derecha) | Panel de propiedades |
| Lienzo | Región | Área de trabajo |
| Barra contextual | Flotante sobre la selección | Barra de formato contextual |
| Barra inferior | Región | Barra de estado y páginas |
| Línea de tiempo | Región (video) | Timeline |
| Pistas | Texto, Superposiciones, Video, Audio y Subtítulos (en la Línea de tiempo) | Tracks |
| Cabezal y Marcadores | Posición de lectura y marcas con nombre en la Línea de tiempo | Playhead / markers |

### 2.2 Barra superior y menús

| Nombre en Gafira | Contenido | Equivalente genérico |
|---|---|---|
| Inicio (botón con isotipo) | Vuelve a la pantalla Inicio | Home |
| Archivo | Nuevo, Abrir, Abrir reciente, Guardar, Guardar como, Guardar copia, Guardar versión, Guardar como plantilla, Importar, Relocalizar medios (Fase 2), Empaquetar fuentes, Exportar por lotes (Fase 3), Versiones, Ajustes, Ayuda | File |
| Tamaño | Abre Cambiar tamaño | Resize |
| Edición | Deshacer, Rehacer, Historial, Copiar, Pegar, Duplicar, Eliminar, Seleccionar todo, Buscar y reemplazar | Edit |
| Ver | Reglas, Guías, Cuadrícula, Imán, Márgenes, Zoom, Modo claro/oscuro | View |
| Estado de guardado | «Cambios guardados.» / «Guardando…» / error | Save status |
| Cola (botón, en el Editor de video) | Abre la Cola de exportación | Render queue |
| Exportar (botón coral) | Abre Exportar | Export / Share |
| Versiones | Lista de versiones nombradas y automáticas | Version history |

### 2.3 Pestañas del Panel lateral

| Nombre en Gafira | Pantalla | Qué contiene | Equivalente genérico |
|---|---|---|---|
| Plantillas | Diseño y Video | Búsqueda, plantillas empaquetadas y propias | Templates |
| Piezas | Diseño y Video | Formas, Líneas, Marcos, Iconos, Códigos QR, Tablas, Gráficos, Ilustraciones | Elements / shapes library |
| Texto | Diseño y Video | Estilos de texto, combinaciones, Efectos de texto | Text |
| Identidad | Diseño y Video | Colores, Tipografías, Logos y Estilos de la Identidad visual activa | Brand panel |
| Biblioteca | Diseño y Video | Archivos propios: imágenes, video, audio, diseños, Colecciones | Uploads / media library |
| Dibujo | Diseño | Lápiz, Marcador, Resaltador, Borrador, color y grosor | Draw |
| Proyectos | Diseño y Video | Proyectos recientes y Colecciones de proyectos | Projects |
| Medios | Video | Clips y escenas del proyecto, importar | Media bin |
| Sonidos | Video | Música y efectos de sonido con vista previa | Audio library |
| Movimiento | Diseño y Video | Entradas, salidas, énfasis y Ruta de movimiento | Animations |
| Subtítulos | Video | Subtítulos manuales; SRT/VTT (Fase 2) | Captions |
| Extensiones | Diseño y Video | Complementos instalados (Fase 3) | Plugins |

### 2.4 Inspector (pestañas a la derecha)

| Nombre en Gafira | Contenido | Equivalente genérico |
|---|---|---|
| Posición | X, Y, Ancho, Alto, Giro, Bloqueo de proporción, Alinear, Orden de apilado | Arrange / Transform |
| Capas | Lista de Capas con arrastre, visibilidad, bloqueo | Layers |
| Imagen | Zona de edición, Ajuste automático, Balance de blancos, Luz, Color, Textura, Filtros, Sombras, Bordes, Comparar, Restablecer, Aislar sujeto | Image adjustments |
| Texto (contextual) | Fuente, tamaño, interletrado, interlineado, Columnas, Ajustar al cuadro, Fijar tipografía | Typography |
| Clip | Velocidad, Invertir, Congelar fotograma, Volumen, Fundidos, Recortar encuadre | Clip properties |
| Fotogramas clave | Parámetros animables y Editor de curvas (Fase 2) | Keyframes |
| Corrección de color | Curvas RGB, Ruedas, LUT, Balance de blancos, Medidores (Fase 2) | Color grading |
| Mezclador | Una tira por pista de audio (hasta 16), Ecualizador, Puerta de ruido, Compresor, Limitador, Reducir ruido y medidor de Sonoridad (Fase 2) | Audio mixer |
| Máscaras | Rectángulo, Elipse, Polígono, Vector, Texto (Fase 2) | Masks |

### 2.5 Herramientas (Dibujo y edición)

| Nombre en Gafira | Atajo | Equivalente genérico |
|---|---|---|
| Seleccionar | V | Selection tool |
| Mano | H (o Espacio) | Pan |
| Zoom | Z | Zoom |
| Texto | T | Text tool |
| Forma | R (rectángulo), O (elipse) | Shape |
| Línea | L | Line |
| Lápiz, Marcador, Resaltador | P, M, Shift+H | Pen / marker / highlighter |
| Borrador | E | Eraser |
| Nota adhesiva | N | Sticky note |
| Recortar | C | Crop |
| Cuentagotas | I | Eyedropper |
| Pincel Quitar / Pincel Devolver | B / Shift+B | Mask brush erase / restore |
| Dividir (video) | S | Razor / split |
| Mover borde (video) | W | Trim / ripple |

### 2.6 Funciones y acciones con nombre propio

| Nombre en Gafira | Qué hace | Equivalente genérico |
|---|---|---|
| Aislar sujeto | Separa el sujeto del fondo con máscara no destructiva y retoque con pincel | Background removal |
| Retocar borde | Pincel Quitar / Devolver sobre la máscara | Mask refinement |
| Zona de edición | Selector de dónde actúa un ajuste: Todo, Elemento, Mano alzada, Primer plano, Fondo, Texto | Selection scope |
| Ajuste automático | Corrige luz y color de un solo clic | Auto-enhance |
| Imán | Atrae elementos a guías, bordes, centros y cuadrícula | Snap |
| Cuadrícula | Rejilla de fondo con paso configurable | Grid |
| Generador de guías | Crea guías por columnas, filas y rejilla con vista previa | Guide generator |
| Zona de seguridad | Área interior donde el contenido no se corta ni queda tapado | Safe area / danger zone |
| Sangrado | Margen exterior de impresión | Bleed |
| Columnas | Reparte un texto en varias columnas | Text columns |
| Ajustar al cuadro | El texto cambia de tamaño para llenar su cuadro | Autofit |
| Fijar tipografía | Protege fuente, tamaño y estilo frente a cambios de identidad o de tamaño | Type lock |
| Curvar | Texto sobre una curva | Warp text on path |
| Duplicar y adaptar | Crea una copia en otro tamaño y conserva el original | Resize copy |
| Adaptar este diseño | Cambia el tamaño del diseño actual | Resize in place |
| Destinos | Categorías de tamaño: Redes, Impresión, Presentaciones, Video, Web, Personalizado | Size presets |
| Estilos | Variables de estilo con nombre (color, texto, efecto) | Style variables |
| Nueva escena | Añade una escena entre clips o al final | Add scene |
| Transición | Efecto en la unión de dos clips: Corte, Fundido, Deslizar, Zoom, Borrado, Disolvencia | Transition |
| Ruta de movimiento | Animación por trayectoria arrastrando el elemento | Motion path |
| Separar audio | Extrae el audio de un clip a su propia pista | Detach audio |
| Nivelar | Normaliza el volumen a una sonoridad objetivo | Normalize |
| Clave de color | Hace transparente un color de fondo | Chroma key |
| Seguimiento | Sigue un punto o zona en el video | Motion tracking |
| Estabilizar | Reduce el temblor de la cámara | Stabilization |
| Medidores | Forma de onda, Vectorscopio, Histograma, Parada RGB | Scopes |
| Copias ligeras | Versiones de baja resolución para editar con fluidez | Proxies |
| Cola de exportación | Lista de trabajos de exportación con prioridad y reintentos | Render queue |
| Agrupar en secuencia | Anida clips en una secuencia reutilizable | Nested sequence |
| Versiones | Instantáneas nombradas del proyecto | Version history |
| Recuperar / Abrir el último guardado / Descartar | Opciones de Recuperación tras fallo | Restore / open last / discard |
| Hoja de fotogramas | Vista de todas las propiedades animadas en filas, junto al Editor de curvas | Dope sheet |
| Capa de ajuste | Capa cuyos efectos se aplican a todo lo que hay debajo | Adjustment layer |
| Marcadores de compás | Marcas automáticas sobre el ritmo de la música para atraer cortes (Fase 2) | Beat markers |
| Voz en off | Grabación de voz desde el micrófono hacia una pista de Audio (Fase 2) | Voice-over |
| Notas | Texto por página en la Barra inferior; no se exporta | Page notes |
| Reescribir | Sugerencia local de otra redacción de un texto; opcional y desactivable (Fase 3) | Text rewrite |
| Relocalizar medios (Fase 2) | Reconecta, con una sola acción, los medios vinculados que cambiaron de carpeta o de disco | Relink media |

**Licencia y activación (diferido).** Los nombres de interfaz de la pantalla de activación, de la sección Licencia de Ajustes y de los estados de licencia se añadirán a este glosario cuando se redacte la spec 06. Licencia y activación: ver spec 06 (pendiente, se redacta al final).

### 2.7 Reglas del glosario

- **DIS-01**: en **cada idioma publicado (español e inglés)**, ningún texto de interfaz, documentación o microcopy usa un nombre distinto del glosario de ese idioma para la misma función, y nunca se mezclan los dos idiomas en una misma pantalla. *Aceptación*: una prueba de `gafira-ui` extrae las etiquetas i18n de `es.toml` y `en.toml` y las compara con las columnas `es` y `en` de `assets/i18n/glosario.toml`; `cargo xtask check-glossary` falla ante una etiqueta no registrada o duplicada en cualquiera de los dos idiomas.
- **DIS-02**: los términos genéricos ampliamente difundidos (Capas, Exportar, Deshacer, Guías, Reglas, Zoom, Plantillas, Opacidad, Recortar, Duplicar, Alinear, Agrupar) se usan sin reinterpretarlos.
- **DIS-03**: ningún nombre del glosario coincide con un nombre propio de otro producto. *Aceptación*: revisión de los nombres antes de cada versión y registro de la revisión en el repositorio; la consulta de marcas es un punto de la lista previa a la venta (spec 05, Parte C).
- **DIS-71** (glosario inglés 1:1): cada fila de §2.1–§2.6 tiene exactamente una fila en §2.8 con su nombre inglés, y un nombre inglés solo se repite donde el español repite el mismo nombre para el mismo concepto (por ejemplo, Texto/Text como pestaña, pestaña del Inspector y herramienta). *Aceptación*: `assets/i18n/glosario.toml` tiene, por cada `id`, los campos `es` y `en` no vacíos; `xtask check-glossary` falla si falta uno, si dos `id` con `es` distinto comparten el mismo `en` o si el número de entradas difiere del número de filas de §2.1–§2.6 (102 en esta versión, con «Pantalla de arranque», D-09).
- **DIS-72** (nombres ingleses propios o genéricos): DIS-03 se aplica también a los nombres ingleses. Son términos genéricos ampliamente usados por muchos programas (*Layers*, *Export*, *Crop*, *Timeline*) o nombres propios de Gafira traducidos de forma literal (*Pieces*, *Magnet*, *Lock type*, *Light copies*); nunca el nombre exclusivo de una función de otro producto. *Aceptación*: la revisión de DIS-03 cubre las dos columnas y queda registrada; hasta que el titular apruebe §2.8 (Q-25) los nombres ingleses se marcan PROPUESTA en `glosario.toml` (`status = "proposed"`).

### 2.8 Versión inglesa del glosario (PROPUESTA, D-08)

> **Nota (D-77)**: el titular aprobó estos nombres ingleses y el microcopy inglés de §12 tal como están, a reserva de la búsqueda de marca de los nombres ingleses propios (R-17 de 02). Q-25 queda cerrada; las marcas PROPUESTA se quedan mientras esta spec siga en backlog.

Correspondencia 1:1 con §2.1–§2.6, en el mismo orden. La columna «Equivalente genérico» de esas tablas sigue siendo la referencia para localizar el concepto y no es texto de interfaz. Los atajos de §2.5 no cambian con el idioma (DIS-77).

#### 2.8.1 Pantallas y estructura (§2.1)

| Nombre en Gafira (es, aprobado) | Nombre en inglés (PROPUESTA) |
|---|---|
| Inicio | Home |
| Pantalla de arranque | Startup screen |
| Editor de diseño | Design editor |
| Editor de video | Video editor |
| Exportar | Export |
| Cambiar tamaño | Resize |
| Identidad visual | Visual identity |
| Ajustes | Settings |
| Recuperación tras fallo | Crash recovery |
| Paleta de comandos | Command palette |
| Barra superior | Top bar |
| Riel de pestañas | Tab rail |
| Panel lateral | Side panel |
| Inspector | Inspector |
| Lienzo | Canvas |
| Barra contextual | Context bar |
| Barra inferior | Bottom bar |
| Línea de tiempo | Timeline |
| Pistas | Tracks (Text, Overlays, Video, Audio and Captions) |
| Cabezal y Marcadores | Playhead and Markers |

#### 2.8.2 Barra superior y menús (§2.2)

| Nombre en Gafira (es, aprobado) | Nombre en inglés (PROPUESTA) | Contenido en inglés (PROPUESTA) |
|---|---|---|
| Inicio (botón con isotipo) | Home (symbol button) | Returns to the Home screen |
| Archivo | File | New, Open, Open recent, Save, Save as, Save a copy, Save version, Save as template, Import, Relink media (Phase 2), Package fonts, Batch export (Phase 3), Versions, Settings, Help |
| Tamaño | Size | Opens Resize |
| Edición | Edit | Undo, Redo, History, Copy, Paste, Duplicate, Delete, Select all, Find and replace |
| Ver | View | Rulers, Guides, Grid, Magnet, Margins, Zoom, Light/dark mode |
| Estado de guardado | Save status | “Changes saved.” / “Saving…” / error |
| Cola (botón, en el Editor de video) | Queue (button, in the Video editor) | Opens the Export queue |
| Exportar (botón coral) | Export (coral button) | Opens Export |
| Versiones | Versions | List of named and automatic versions |

#### 2.8.3 Pestañas del Panel lateral (§2.3)

| Nombre en Gafira (es, aprobado) | Nombre en inglés (PROPUESTA) | Contenido en inglés (PROPUESTA) |
|---|---|---|
| Plantillas | Templates | Search, bundled and own templates |
| Piezas | Pieces | Shapes, Lines, Frames, Icons, QR codes, Tables, Charts, Illustrations |
| Texto | Text | Text styles, pairings, Text effects |
| Identidad | Identity | Colors, Fonts, Logos and Styles of the active Visual identity |
| Biblioteca | Library | Own files: images, video, audio, designs, Collections |
| Dibujo | Drawing | Pencil, Marker, Highlighter, Eraser, color and thickness |
| Proyectos | Projects | Recent projects and project Collections |
| Medios | Media | Project clips and scenes, import |
| Sonidos | Sounds | Music and sound effects with preview |
| Movimiento | Motion | Entrances, exits, emphasis and Motion path |
| Subtítulos | Captions | Manual captions; SRT/VTT (Phase 2) |
| Extensiones | Extensions | Installed add-ons (Phase 3) |

#### 2.8.4 Inspector (§2.4)

| Nombre en Gafira (es, aprobado) | Nombre en inglés (PROPUESTA) | Contenido en inglés (PROPUESTA) |
|---|---|---|
| Posición | Position | X, Y, Width, Height, Rotation, Lock proportions, Align, Stacking order |
| Capas | Layers | Layer list with drag, visibility and lock |
| Imagen | Image | Edit area, Auto adjust, White balance, Light, Color, Texture, Filters, Shadows, Borders, Compare, Reset, Isolate subject |
| Texto (contextual) | Text (contextual) | Font, size, letter spacing, line spacing, Columns, Fit to box, Lock type |
| Clip | Clip | Speed, Reverse, Freeze frame, Volume, Fades, Crop framing |
| Fotogramas clave | Keyframes | Animatable parameters and Curve editor (Phase 2) |
| Corrección de color | Color correction | RGB curves, Wheels, LUT, White balance, Scopes (Phase 2) |
| Mezclador | Mixer | One strip per audio track (up to 16), Equalizer, Noise gate, Compressor, Limiter, Reduce noise and Loudness meter (Phase 2) |
| Máscaras | Masks | Rectangle, Ellipse, Polygon, Vector, Text (Phase 2) |

#### 2.8.5 Herramientas (§2.5)

| Nombre en Gafira (es, aprobado) | Nombre en inglés (PROPUESTA) | Atajo (igual en los dos idiomas) |
|---|---|---|
| Seleccionar | Select | V |
| Mano | Hand | H (or Space) |
| Zoom | Zoom | Z |
| Texto | Text | T |
| Forma | Shape | R (rectangle), O (ellipse) |
| Línea | Line | L |
| Lápiz, Marcador, Resaltador | Pencil, Marker, Highlighter | P, M, Shift+H |
| Borrador | Eraser | E |
| Nota adhesiva | Sticky note | N |
| Recortar | Crop | C |
| Cuentagotas | Eyedropper | I |
| Pincel Quitar / Pincel Devolver | Remove brush / Restore brush | B / Shift+B |
| Dividir (video) | Split (video) | S |
| Mover borde (video) | Move edge (video) | W |

#### 2.8.6 Funciones y acciones con nombre propio (§2.6)

| Nombre en Gafira (es, aprobado) | Nombre en inglés (PROPUESTA) |
|---|---|
| Aislar sujeto | Isolate subject |
| Retocar borde | Refine edge |
| Zona de edición | Edit area (All, Object, Freehand, Foreground, Background, Text) |
| Ajuste automático | Auto adjust |
| Imán | Magnet |
| Cuadrícula | Grid |
| Generador de guías | Guide builder |
| Zona de seguridad | Safe zone |
| Sangrado | Bleed |
| Columnas | Columns |
| Ajustar al cuadro | Fit to box |
| Fijar tipografía | Lock type |
| Curvar | Curve |
| Duplicar y adaptar | Duplicate and adapt |
| Adaptar este diseño | Adapt this design |
| Destinos | Destinations (Social, Print, Presentations, Video, Web, Custom) |
| Estilos | Styles |
| Nueva escena | New scene |
| Transición | Transition (Cut, Fade, Slide, Zoom, Wipe, Dissolve) |
| Ruta de movimiento | Motion path |
| Separar audio | Detach audio |
| Nivelar | Level |
| Clave de color | Color key |
| Seguimiento | Tracking |
| Estabilizar | Stabilize |
| Medidores | Scopes (Waveform, Vectorscope, Histogram, RGB parade) |
| Copias ligeras | Light copies |
| Cola de exportación | Export queue |
| Agrupar en secuencia | Group into sequence |
| Versiones | Versions |
| Recuperar / Abrir el último guardado / Descartar | Recover / Open last saved / Discard |
| Hoja de fotogramas | Frame sheet |
| Capa de ajuste | Adjustment layer |
| Marcadores de compás | Beat markers |
| Voz en off | Voice-over |
| Notas | Notes |
| Reescribir | Rewrite |
| Relocalizar medios (Fase 2) | Relink media (Phase 2) |

*Criterios usados*: un nombre por concepto y sin sinónimos; sustantivos para pestañas y paneles y verbos para las acciones, como en español; los nombres propios de Gafira se traducen de forma literal y clara (*Pieces*, *Magnet*, *Light copies*, *Frame sheet*); los conceptos que todos los programas llaman igual usan ese término genérico (*Layers*, *Keyframes*, *Scopes*, *Adjustment layer*, *Relink media*). La licencia y la activación se añadirán a las dos versiones del glosario cuando se redacte la spec 06.

---

## 3. Tokens de diseño

`gafira-theme` carga y valida `assets/theme/gafira.tokens.json` (02, §8.2). Los valores de marca (grafito, coral, blanco cálido, verde agua, gris azulado, radios, escala tipográfica y unidad de 4 px) provienen del manual. Los estados, el modo oscuro, los colores semánticos, las sombras y la escala de movimiento son **PROPUESTA** derivada de la paleta y validada con el cálculo del §4.

*Nota (D-82)*: el titular aprobó el modo oscuro derivado (cierra 01 C-06 y la parte de modo oscuro de PD-01); la marca PROPUESTA se queda mientras esta spec siga en backlog.

```json
{
  "schema": 1,
  "name": "gafira",
  "brand": {
    "ink": "#18243B", "coral": "#FF6B57", "canvas": "#FAFAF7", "mint": "#3FBFA9", "mist": "#E7EAF2"
  },
  "modes": {
    "light": {
      "bg": "#FAFAF7", "surface": "#FFFFFF", "surface.raised": "#FFFFFF", "surface.sunken": "#E7EAF2", "workspace": "#E7EAF2",
      "border": "#C3CADB", "border.strong": "#6B7790",
      "text": "#18243B", "text.secondary": "#46526B", "text.muted": "#5A667E", "text.disabled": "#8E97AA",
      "primary.bg": "#FF6B57", "primary.bg.hover": "#FF7E6C", "primary.bg.pressed": "#F2563F", "primary.fg": "#18243B",
      "secondary.bg": "#E7EAF2", "secondary.bg.hover": "#DADFEB", "secondary.bg.pressed": "#CBD2E2", "secondary.fg": "#18243B",
      "accent": "#3FBFA9", "accent.fg": "#18243B",
      "focus.ring": "#18243B", "focus.halo": "#FAFAF7",
      "selection.bg": "#CFEFE8", "link": "#1B4FA0",
      "error.fg": "#B3261E", "error.bg": "#FBE9E7", "error.border": "#B3261E",
      "success.fg": "#17695A", "success.bg": "#E3F5F1", "success.border": "#17695A",
      "warning.fg": "#7A4F00", "warning.bg": "#FFF3D6",
      "danger.bg": "#B3261E", "danger.bg.hover": "#9C2019", "danger.fg": "#FFFFFF",
      "topbar.bg": "#18243B", "topbar.fg": "#FAFAF7", "topbar.fg.muted": "#C3CADB", "topbar.hover": "#2A3C5E",
      "canvas.selection": "#18243B", "canvas.selection.halo": "#FFFFFF",
      "canvas.guide": "#17695A", "canvas.bleed": "#B3261E", "canvas.safe": "#17695A"
    },
    "dark": {
      "bg": "#0F1828", "surface": "#18243B", "surface.raised": "#223250", "surface.sunken": "#0A1120", "workspace": "#0A1120",
      "border": "#33456A", "border.strong": "#8593B2",
      "text": "#FAFAF7", "text.secondary": "#C3CADB", "text.muted": "#A4AEC4", "text.disabled": "#6F7B96",
      "primary.bg": "#FF6B57", "primary.bg.hover": "#FF7E6C", "primary.bg.pressed": "#F2563F", "primary.fg": "#18243B",
      "secondary.bg": "#2A3C5E", "secondary.bg.hover": "#33496F", "secondary.bg.pressed": "#3C5480", "secondary.fg": "#FAFAF7",
      "accent": "#3FBFA9", "accent.fg": "#18243B",
      "focus.ring": "#3FBFA9", "focus.halo": "#0F1828",
      "selection.bg": "#1F4A52", "link": "#8FB4FF",
      "error.fg": "#FFB4AB", "error.bg": "#4A1F24", "error.border": "#FFB4AB",
      "success.fg": "#7BE0CB", "success.bg": "#15382F", "success.border": "#7BE0CB",
      "warning.fg": "#FFD27A", "warning.bg": "#3E3112",
      "danger.bg": "#FFB4AB", "danger.bg.hover": "#FFC7C0", "danger.fg": "#18243B",
      "topbar.bg": "#0A1120", "topbar.fg": "#FAFAF7", "topbar.fg.muted": "#C3CADB", "topbar.hover": "#1B2A45",
      "canvas.selection": "#3FBFA9", "canvas.selection.halo": "#0F1828",
      "canvas.guide": "#3FBFA9", "canvas.bleed": "#FFB4AB", "canvas.safe": "#3FBFA9"
    }
  },
  "splash": {
    "mode": "light",
    "panel": "{modes.light.bg}",
    "progress.track": "{brand.ink}",
    "progress.fill": "{brand.coral}",
    "window": [800, 450], "panel.height": 128, "panel.padding": 32, "logo.width": 200,
    "progress.height": 4, "image.min_visible_height": 96
  },
  "contrast": {
    "min": { "text": 4.5, "ui": 3.0, "exempt": 0 },
    "pairs": {
      "light": [
        ["text", "bg", "text"],
        ["text", "surface", "text"],
        ["text", "surface.sunken", "text"],
        ["text.secondary", "bg", "text"],
        ["text.secondary", "surface", "text"],
        ["text.muted", "bg", "text"],
        ["text.muted", "surface", "text"],
        ["text.disabled", "surface", "exempt"],
        ["primary.fg", "primary.bg", "text"],
        ["primary.fg", "primary.bg.hover", "text"],
        ["primary.fg", "primary.bg.pressed", "text"],
        ["text", "secondary.bg", "text"],
        ["text", "secondary.bg.hover", "text"],
        ["text", "secondary.bg.pressed", "text"],
        ["accent.fg", "accent", "text"],
        ["focus.ring", "bg", "ui"],
        ["focus.ring", "surface", "ui"],
        ["border.strong", "bg", "ui"],
        ["border.strong", "surface", "ui"],
        ["error.fg", "error.bg", "text"],
        ["error.fg", "bg", "text"],
        ["error.fg", "surface", "text"],
        ["success.fg", "success.bg", "text"],
        ["success.fg", "bg", "text"],
        ["warning.fg", "warning.bg", "text"],
        ["link", "bg", "text"],
        ["link", "surface", "text"],
        ["text", "selection.bg", "text"],
        ["danger.fg", "danger.bg", "text"],
        ["danger.fg", "danger.bg.hover", "text"],
        ["topbar.fg", "topbar.bg", "text"],
        ["topbar.fg.muted", "topbar.bg", "text"],
        ["topbar.fg.muted", "topbar.hover", "text"],
        ["primary.bg", "topbar.bg", "ui"],
        ["accent", "topbar.bg", "ui"],
        ["text", "workspace", "text"],
        ["text", "surface.raised", "text"],
        ["text.muted", "surface.raised", "text"],
        ["border.strong", "surface.raised", "ui"],
        ["focus.ring", "surface.raised", "ui"],
        ["link", "surface.raised", "text"]
      ],
      "dark": [
        ["text", "bg", "text"],
        ["text", "surface", "text"],
        ["text", "surface.raised", "text"],
        ["text.secondary", "bg", "text"],
        ["text.secondary", "surface", "text"],
        ["text.muted", "bg", "text"],
        ["text.muted", "surface", "text"],
        ["text.disabled", "surface", "exempt"],
        ["primary.fg", "primary.bg", "text"],
        ["primary.fg", "primary.bg.hover", "text"],
        ["primary.fg", "primary.bg.pressed", "text"],
        ["text", "secondary.bg", "text"],
        ["text", "secondary.bg.hover", "text"],
        ["text", "secondary.bg.pressed", "text"],
        ["accent.fg", "accent", "text"],
        ["focus.ring", "bg", "ui"],
        ["focus.ring", "surface", "ui"],
        ["border.strong", "bg", "ui"],
        ["border.strong", "surface", "ui"],
        ["error.fg", "error.bg", "text"],
        ["error.fg", "bg", "text"],
        ["error.fg", "surface", "text"],
        ["success.fg", "success.bg", "text"],
        ["success.fg", "bg", "text"],
        ["warning.fg", "warning.bg", "text"],
        ["link", "bg", "text"],
        ["link", "surface", "text"],
        ["text", "selection.bg", "text"],
        ["danger.fg", "danger.bg", "text"],
        ["danger.fg", "danger.bg.hover", "text"],
        ["topbar.fg", "topbar.bg", "text"],
        ["topbar.fg.muted", "topbar.bg", "text"],
        ["topbar.fg.muted", "topbar.hover", "text"],
        ["primary.bg", "topbar.bg", "ui"],
        ["accent", "topbar.bg", "ui"],
        ["text", "workspace", "text"],
        ["accent", "surface", "ui"],
        ["text.muted", "surface.raised", "text"],
        ["border.strong", "surface.raised", "ui"],
        ["focus.ring", "surface.raised", "ui"],
        ["link", "surface.raised", "text"]
      ],
      "splash": [
        ["progress.fill", "progress.track", "ui"],
        ["progress.track", "panel", "ui"]
      ]
    },
    "rejected": [
      ["#FFFFFF", "#FF6B57"], ["#FF6B57", "#FAFAF7"], ["#3FBFA9", "#FAFAF7"],
      ["#3FBFA9", "#FFFFFF"], ["#FF6B57", "#FFFFFF"], ["#18243B", "#0F1828"]
    ]
  },
  "type": {
    "family": { "display": "Manrope", "ui": "Inter" },
    "scale": {
      "display": { "family": "display", "size": 56, "line": 64, "weight": 700 },
      "h1": { "family": "display", "size": 40, "line": 48, "weight": 700 },
      "h2": { "family": "display", "size": 30, "line": 38, "weight": 700 },
      "h3": { "family": "display", "size": 24, "line": 32, "weight": 650 },
      "body-lg": { "family": "ui", "size": 18, "line": 29, "weight": 400 },
      "body": { "family": "ui", "size": 16, "line": 26, "weight": 400 },
      "body-sm": { "family": "ui", "size": 14, "line": 22, "weight": 400 },
      "caption": { "family": "ui", "size": 12, "line": 18, "weight": 500 },
      "ui-label": { "family": "ui", "size": 13, "line": 20, "weight": 500 },
      "mono-num": { "family": "ui", "size": 13, "line": 20, "weight": 500, "features": ["tnum"] }
    }
  },
  "space": { "unit": 4, "scale": [0, 4, 8, 12, 16, 20, 24, 32, 40, 48, 64] },
  "radius": { "sm": 8, "md": 12, "lg": 20, "pill": 999 },
  "size": {
    "control": { "sm": 28, "md": 32, "lg": 40 },
    "icon": { "sm": 16, "md": 20, "lg": 24 },
    "target.min": 24,
    "panel.left": 320, "panel.right": 296, "rail": 64, "topbar": 48, "bottombar": 40, "timeline.min": 220
  },
  "border": { "width": 1, "focus": 2, "focus.offset": 2 },
  "shadow": {
    "0": "none",
    "1": "0 1px 2px rgba(24,36,59,0.12)",
    "2": "0 4px 12px rgba(24,36,59,0.16)",
    "3": "0 12px 32px rgba(24,36,59,0.24)"
  },
  "motion": {
    "fast": 120, "base": 160, "slow": 220,
    "easing": { "standard": [0.2, 0, 0, 1], "enter": [0, 0, 0, 1], "exit": [0.4, 0, 1, 1] },
    "reduced": { "fast": 0, "base": 0, "slow": 0 }
  },
  "z": { "canvas": 0, "overlay": 10, "contextbar": 20, "panel": 30, "popover": 40, "dialog": 50, "toast": 60, "tooltip": 70 }
}
```

Notas de uso:

- **`surface.raised`** existe en los dos modos y lo usan Diálogo, Popover y menús; en modo claro coincide con `surface`, por lo que se distingue por borde y sombra y no por color. Sus pares de contraste están en las tablas del §4.
- **`splash`** (D-09, §7.10) agrupa los tokens de la Pantalla de arranque. No depende del modo de la app: `"mode": "light"` indica que su panel y sus textos usan siempre los tokens de `modes.light` (texto `text`, versión `text.muted`, error `error.fg`, botones `primary.*` y `secondary.*`, anillo `focus.ring`), porque el logo principal es grafito y no puede ir sobre un fondo oscuro. `progress.track` y `progress.fill` son la pista grafito y el relleno coral de la barra de carga. Las medidas (`window`, `panel.height`, `logo.width`…) están en px lógicos. Sus pares de contraste propios están en `contrast.pairs.splash` y los demás se reutilizan de `contrast.pairs.light` (§4).
- Las sombras del modo oscuro usan `rgba(0,0,0,0.45)` en lugar de las anteriores y se acompañan de un borde `border` de 1 px, porque la sombra es poco visible sobre fondos oscuros.
- `gafira-theme` resuelve referencias del tipo `{brand.ink}` (02, §8.2) y rechaza el archivo si falta un token obligatorio o si un par declarado en `contrast.pairs` (sección del JSON de arriba, con los mínimos de `contrast.min`) incumple el mínimo (DIS-10). Los pares de `contrast.rejected` no deben aparecer en `pairs`.
- Los controles miden 32 px por defecto (28 compacto, 40 en diálogos y Inicio). El área de clic mínima es 24×24 px (WCAG 2.2, criterio 2.5.8). Los iconos de 20 px llevan una zona de clic de 32 px.
- Unidad de espacio 4 px; se prefieren múltiplos de 8. Rejilla de 12 columnas en Inicio y en diálogos anchos.
- **DIS-04** (colores en código): ningún `Color32::from_rgb` fuera de `theme.rs` (ARQ-60). *Aceptación*: lint de CI con 0 coincidencias.
- **DIS-05** (tipografías): Inter y Manrope se embeben con su `OFL.txt` (02, §8.2). Los números de campos numéricos usan cifras tabulares (`tnum`) para que no «salten» al arrastrar.

---

## 4. Contraste y accesibilidad cromática

**Método.** Los ratios se calcularon con la fórmula de WCAG 2.x: luminancia relativa `L = 0,2126·R + 0,7152·G + 0,0722·B` con canales sRGB linealizados, y `ratio = (L1 + 0,05) / (L2 + 0,05)`. El cálculo se hizo con un script de Python (sin redondeos intermedios) y se reproduce como prueba en `gafira-theme` (ARQ-61). Mínimos aplicados: **4,5:1** para texto normal, **3:1** para texto grande (≥ 24 px, o ≥ 18,66 px en negrita) y para componentes gráficos (bordes de campos, anillo de foco, iconos que transmiten información).

**Validación de referencia.** El script reproduce los ratios del manual: grafito/blanco cálido 14,83:1; blanco/grafito 15,51:1; grafito/coral 5,53:1; blanco/coral 2,80:1; grafito/verde agua 6,82:1. Esto confirma que el cálculo coincide con el de la marca.

**Resultado.** Los 84 pares declarados (41 en modo claro, 41 en oscuro y 2 de la Pantalla de arranque) se listan en las tablas siguientes y coinciden con `contrast.pairs` del JSON del §3: los 82 con mínimo cumplen y los 2 restantes (texto deshabilitado, exento por WCAG 1.4.3) son informativos. Los pares de `surface.raised` y los 2 de `splash` (D-09) son los únicos añadidos respecto de la versión anterior. El modo oscuro se **deriva** de la paleta: el fondo `bg` es el grafito oscurecido, `surface` es el propio grafito (`#18243B`), `surface.raised` es el grafito aclarado (en modo claro vale `#FFFFFF`, igual que `surface`, y la elevación la dan el borde `border` y la sombra 2), el texto principal es el blanco cálido, y el coral y el verde agua se mantienen sin cambio de matiz. Los colores semánticos (error, éxito, aviso, enlace) no existen en la paleta del manual: se derivan y se marcan como **PROPUESTA**.

#### Modo claro

| # | Par | Primer plano | Fondo | Ratio | Mínimo | Resultado |
|---|---|---|---|---|---|---|
| 1 | Texto principal | `text` #18243B | `bg` #FAFAF7 | 14.83:1 | 4.5:1 | Cumple |
| 2 | Texto principal | `text` #18243B | `surface` #FFFFFF | 15.51:1 | 4.5:1 | Cumple |
| 3 | Texto principal | `text` #18243B | `surface.sunken` #E7EAF2 | 12.88:1 | 4.5:1 | Cumple |
| 4 | Texto secundario | `text.secondary` #46526B | `bg` #FAFAF7 | 7.49:1 | 4.5:1 | Cumple |
| 5 | Texto secundario | `text.secondary` #46526B | `surface` #FFFFFF | 7.84:1 | 4.5:1 | Cumple |
| 6 | Texto atenuado | `text.muted` #5A667E | `bg` #FAFAF7 | 5.52:1 | 4.5:1 | Cumple |
| 7 | Texto atenuado | `text.muted` #5A667E | `surface` #FFFFFF | 5.77:1 | 4.5:1 | Cumple |
| 8 | Texto deshabilitado (exento) | `text.disabled` #8E97AA | `surface` #FFFFFF | 2.93:1 | exento | Informativo |
| 9 | Botón primario | `primary.fg` #18243B | `primary.bg` #FF6B57 | 5.53:1 | 4.5:1 | Cumple |
| 10 | Botón primario hover | `primary.fg` #18243B | `primary.bg.hover` #FF7E6C | 6.24:1 | 4.5:1 | Cumple |
| 11 | Botón primario pressed | `primary.fg` #18243B | `primary.bg.pressed` #F2563F | 4.56:1 | 4.5:1 | Cumple |
| 12 | Botón secundario | `text` #18243B | `secondary.bg` #E7EAF2 | 12.88:1 | 4.5:1 | Cumple |
| 13 | Botón secundario hover | `text` #18243B | `secondary.bg.hover` #DADFEB | 11.62:1 | 4.5:1 | Cumple |
| 14 | Botón secundario pressed | `text` #18243B | `secondary.bg.pressed` #CBD2E2 | 10.23:1 | 4.5:1 | Cumple |
| 15 | Acento (texto sobre mint) | `accent.fg` #18243B | `accent` #3FBFA9 | 6.82:1 | 4.5:1 | Cumple |
| 16 | Anillo de foco vs fondo | `focus.ring` #18243B | `bg` #FAFAF7 | 14.83:1 | 3.0:1 | Cumple |
| 17 | Anillo de foco vs superficie | `focus.ring` #18243B | `surface` #FFFFFF | 15.51:1 | 3.0:1 | Cumple |
| 18 | Borde de campo (componente UI) | `border.strong` #6B7790 | `bg` #FAFAF7 | 4.30:1 | 3.0:1 | Cumple |
| 19 | Borde de campo vs superficie | `border.strong` #6B7790 | `surface` #FFFFFF | 4.50:1 | 3.0:1 | Cumple |
| 20 | Error texto | `error.fg` #B3261E | `error.bg` #FBE9E7 | 5.58:1 | 4.5:1 | Cumple |
| 21 | Error texto sobre fondo | `error.fg` #B3261E | `bg` #FAFAF7 | 6.25:1 | 4.5:1 | Cumple |
| 22 | Error texto sobre superficie | `error.fg` #B3261E | `surface` #FFFFFF | 6.54:1 | 4.5:1 | Cumple |
| 23 | Éxito texto | `success.fg` #17695A | `success.bg` #E3F5F1 | 5.80:1 | 4.5:1 | Cumple |
| 24 | Éxito texto sobre fondo | `success.fg` #17695A | `bg` #FAFAF7 | 6.27:1 | 4.5:1 | Cumple |
| 25 | Aviso texto | `warning.fg` #7A4F00 | `warning.bg` #FFF3D6 | 6.46:1 | 4.5:1 | Cumple |
| 26 | Enlace | `link` #1B4FA0 | `bg` #FAFAF7 | 7.51:1 | 4.5:1 | Cumple |
| 27 | Enlace sobre superficie | `link` #1B4FA0 | `surface` #FFFFFF | 7.86:1 | 4.5:1 | Cumple |
| 28 | Texto sobre selección | `text` #18243B | `selection.bg` #CFEFE8 | 12.67:1 | 4.5:1 | Cumple |
| 29 | Acción destructiva | `danger.fg` #FFFFFF | `danger.bg` #B3261E | 6.54:1 | 4.5:1 | Cumple |
| 30 | Acción destructiva hover | `danger.fg` #FFFFFF | `danger.bg.hover` #9C2019 | 7.97:1 | 4.5:1 | Cumple |
| 31 | Barra superior: texto | `topbar.fg` #FAFAF7 | `topbar.bg` #18243B | 14.83:1 | 4.5:1 | Cumple |
| 32 | Barra superior: texto atenuado | `topbar.fg.muted` #C3CADB | `topbar.bg` #18243B | 9.44:1 | 4.5:1 | Cumple |
| 33 | Barra superior: texto atenuado en hover | `topbar.fg.muted` #C3CADB | `topbar.hover` #2A3C5E | 6.71:1 | 4.5:1 | Cumple |
| 34 | Barra superior: indicador coral (gráfico) | `primary.bg` #FF6B57 | `topbar.bg` #18243B | 5.53:1 | 3.0:1 | Cumple |
| 35 | Barra superior: acento mint (gráfico) | `accent` #3FBFA9 | `topbar.bg` #18243B | 6.82:1 | 3.0:1 | Cumple |
| 36 | Área de trabajo: texto | `text` #18243B | `workspace` #E7EAF2 | 12.88:1 | 4.5:1 | Cumple |
| 37 | Superficie elevada: texto (diálogos, popovers) | `text` #18243B | `surface.raised` #FFFFFF | 15.51:1 | 4.5:1 | Cumple |
| 38 | Superficie elevada: texto atenuado | `text.muted` #5A667E | `surface.raised` #FFFFFF | 5.77:1 | 4.5:1 | Cumple |
| 39 | Superficie elevada: borde de campo (componente UI) | `border.strong` #6B7790 | `surface.raised` #FFFFFF | 4.50:1 | 3.0:1 | Cumple |
| 40 | Superficie elevada: anillo de foco | `focus.ring` #18243B | `surface.raised` #FFFFFF | 15.51:1 | 3.0:1 | Cumple |
| 41 | Superficie elevada: enlace | `link` #1B4FA0 | `surface.raised` #FFFFFF | 7.86:1 | 4.5:1 | Cumple |

#### Modo oscuro

| # | Par | Primer plano | Fondo | Ratio | Mínimo | Resultado |
|---|---|---|---|---|---|---|
| 1 | Texto principal | `text` #FAFAF7 | `bg` #0F1828 | 17.00:1 | 4.5:1 | Cumple |
| 2 | Texto principal | `text` #FAFAF7 | `surface` #18243B | 14.83:1 | 4.5:1 | Cumple |
| 3 | Texto principal | `text` #FAFAF7 | `surface.raised` #223250 | 12.24:1 | 4.5:1 | Cumple |
| 4 | Texto secundario | `text.secondary` #C3CADB | `bg` #0F1828 | 10.83:1 | 4.5:1 | Cumple |
| 5 | Texto secundario | `text.secondary` #C3CADB | `surface` #18243B | 9.44:1 | 4.5:1 | Cumple |
| 6 | Texto atenuado | `text.muted` #A4AEC4 | `bg` #0F1828 | 7.98:1 | 4.5:1 | Cumple |
| 7 | Texto atenuado | `text.muted` #A4AEC4 | `surface` #18243B | 6.96:1 | 4.5:1 | Cumple |
| 8 | Texto deshabilitado (exento) | `text.disabled` #6F7B96 | `surface` #18243B | 3.65:1 | exento | Informativo |
| 9 | Botón primario | `primary.fg` #18243B | `primary.bg` #FF6B57 | 5.53:1 | 4.5:1 | Cumple |
| 10 | Botón primario hover | `primary.fg` #18243B | `primary.bg.hover` #FF7E6C | 6.24:1 | 4.5:1 | Cumple |
| 11 | Botón primario pressed | `primary.fg` #18243B | `primary.bg.pressed` #F2563F | 4.56:1 | 4.5:1 | Cumple |
| 12 | Botón secundario | `text` #FAFAF7 | `secondary.bg` #2A3C5E | 10.53:1 | 4.5:1 | Cumple |
| 13 | Botón secundario hover | `text` #FAFAF7 | `secondary.bg.hover` #33496F | 8.64:1 | 4.5:1 | Cumple |
| 14 | Botón secundario pressed | `text` #FAFAF7 | `secondary.bg.pressed` #3C5480 | 7.24:1 | 4.5:1 | Cumple |
| 15 | Acento (texto sobre mint) | `accent.fg` #18243B | `accent` #3FBFA9 | 6.82:1 | 4.5:1 | Cumple |
| 16 | Anillo de foco vs fondo | `focus.ring` #3FBFA9 | `bg` #0F1828 | 7.82:1 | 3.0:1 | Cumple |
| 17 | Anillo de foco vs superficie | `focus.ring` #3FBFA9 | `surface` #18243B | 6.82:1 | 3.0:1 | Cumple |
| 18 | Borde de campo (componente UI) | `border.strong` #8593B2 | `bg` #0F1828 | 5.77:1 | 3.0:1 | Cumple |
| 19 | Borde de campo vs superficie | `border.strong` #8593B2 | `surface` #18243B | 5.03:1 | 3.0:1 | Cumple |
| 20 | Error texto | `error.fg` #FFB4AB | `error.bg` #4A1F24 | 8.18:1 | 4.5:1 | Cumple |
| 21 | Error texto sobre fondo | `error.fg` #FFB4AB | `bg` #0F1828 | 10.47:1 | 4.5:1 | Cumple |
| 22 | Error texto sobre superficie | `error.fg` #FFB4AB | `surface` #18243B | 9.13:1 | 4.5:1 | Cumple |
| 23 | Éxito texto | `success.fg` #7BE0CB | `success.bg` #15382F | 8.16:1 | 4.5:1 | Cumple |
| 24 | Éxito texto sobre fondo | `success.fg` #7BE0CB | `bg` #0F1828 | 11.31:1 | 4.5:1 | Cumple |
| 25 | Aviso texto | `warning.fg` #FFD27A | `warning.bg` #3E3112 | 8.92:1 | 4.5:1 | Cumple |
| 26 | Enlace | `link` #8FB4FF | `bg` #0F1828 | 8.58:1 | 4.5:1 | Cumple |
| 27 | Enlace sobre superficie | `link` #8FB4FF | `surface` #18243B | 7.49:1 | 4.5:1 | Cumple |
| 28 | Texto sobre selección | `text` #FAFAF7 | `selection.bg` #1F4A52 | 9.30:1 | 4.5:1 | Cumple |
| 29 | Acción destructiva | `danger.fg` #18243B | `danger.bg` #FFB4AB | 9.13:1 | 4.5:1 | Cumple |
| 30 | Acción destructiva hover | `danger.fg` #18243B | `danger.bg.hover` #FFC7C0 | 10.47:1 | 4.5:1 | Cumple |
| 31 | Barra superior: texto | `topbar.fg` #FAFAF7 | `topbar.bg` #0A1120 | 18.03:1 | 4.5:1 | Cumple |
| 32 | Barra superior: texto atenuado | `topbar.fg.muted` #C3CADB | `topbar.bg` #0A1120 | 11.48:1 | 4.5:1 | Cumple |
| 33 | Barra superior: texto atenuado en hover | `topbar.fg.muted` #C3CADB | `topbar.hover` #1B2A45 | 8.74:1 | 4.5:1 | Cumple |
| 34 | Barra superior: indicador coral (gráfico) | `primary.bg` #FF6B57 | `topbar.bg` #0A1120 | 6.73:1 | 3.0:1 | Cumple |
| 35 | Barra superior: acento mint (gráfico) | `accent` #3FBFA9 | `topbar.bg` #0A1120 | 8.29:1 | 3.0:1 | Cumple |
| 36 | Área de trabajo: texto | `text` #FAFAF7 | `workspace` #0A1120 | 18.03:1 | 4.5:1 | Cumple |
| 37 | Icono acento (mint) vs superficie (gráfico) | `accent` #3FBFA9 | `surface` #18243B | 6.82:1 | 3.0:1 | Cumple |
| 38 | Superficie elevada: texto atenuado | `text.muted` #A4AEC4 | `surface.raised` #223250 | 5.75:1 | 4.5:1 | Cumple |
| 39 | Superficie elevada: borde de campo (componente UI) | `border.strong` #8593B2 | `surface.raised` #223250 | 4.15:1 | 3.0:1 | Cumple |
| 40 | Superficie elevada: anillo de foco | `focus.ring` #3FBFA9 | `surface.raised` #223250 | 5.63:1 | 3.0:1 | Cumple |
| 41 | Superficie elevada: enlace | `link` #8FB4FF | `surface.raised` #223250 | 6.18:1 | 4.5:1 | Cumple |

#### Pantalla de arranque (D-09, §7.10)

El panel de la Pantalla de arranque usa los tokens del modo claro en cualquier modo de la app. Sus textos y controles reutilizan pares ya declarados en `contrast.pairs.light`; solo la barra de carga añade pares (`contrast.pairs.splash`). Cálculo con el mismo script de Python de este apartado (24 de octubre de 2026).

| # | Par | Primer plano | Fondo | Ratio | Mínimo | Resultado |
|---|---|---|---|---|---|---|
| S1 | Barra de carga: relleno sobre pista (gráfico) | `progress.fill` #FF6B57 | `progress.track` #18243B | 5.53:1 | 3.0:1 | Cumple |
| S2 | Barra de carga: pista sobre panel (gráfico) | `progress.track` #18243B | `panel` #FAFAF7 | 14.83:1 | 3.0:1 | Cumple |
| — | Eslogan (`h3`, texto grande) | `text` #18243B | `panel` = `bg` #FAFAF7 | 14.83:1 | 3.0:1 (se exige 4.5:1) | Cumple (= par claro 1) |
| — | Versión y mensaje de carga lenta (`caption`, `body-sm`) | `text.muted` #5A667E / `text.secondary` #46526B | `panel` #FAFAF7 | 5.52:1 / 7.49:1 | 4.5:1 | Cumple (= pares claros 6 y 4) |
| — | Mensaje de error | `error.fg` #B3261E | `panel` #FAFAF7 | 6.25:1 | 4.5:1 | Cumple (= par claro 21) |
| — | Botón «Reintentar» | `primary.fg` #18243B | `primary.bg` #FF6B57 | 5.53:1 | 4.5:1 | Cumple (= par claro 9) |
| — | Anillo de foco en el estado de error | `focus.ring` #18243B | `panel` #FAFAF7 | 14.83:1 | 3.0:1 | Cumple (= par claro 16) |

*Combinaciones descartadas al diseñar la barra*: coral sobre `bg` #FAFAF7 (2.68:1), sobre `secondary.bg` #E7EAF2 (2.33:1) o sobre `border` #C3CADB (1.71:1) no llegan a 3:1, por eso la pista es grafito. Con la app en modo oscuro tampoco valen sus tokens sobre el panel claro: `focus.ring` oscuro #3FBFA9 da 2.17:1 y `error.fg` oscuro #FFB4AB da 1.62:1; por eso el panel fija `modes.light`. El borde superior e inferior del relleno coral toca el panel (2.68:1), pero no transmite información: el progreso se lee en el límite entre relleno y pista (S1).

#### Pares rechazados (no se usan)

| Par rechazado | Ratio | Regla derivada |
|---|---|---|
| Texto blanco `#FFFFFF` sobre coral `#FF6B57` | 2,80:1 | El botón principal usa texto grafito (decisión del manual). |
| Coral `#FF6B57` como texto sobre `#FAFAF7` | 2,68:1 | El coral no se usa como color de texto sobre claro. |
| Verde agua `#3FBFA9` como texto sobre `#FAFAF7` | 2,17:1 | El verde agua solo es relleno (con texto grafito encima) o acento gráfico sobre oscuro. |
| Verde agua `#3FBFA9` como icono sobre blanco | 2,27:1 | En modo claro los iconos de estado usan `success.fg` (`#17695A`, 6,27:1 sobre `#FAFAF7`). |
| Coral como único indicador (borde o icono) sobre blanco | 2,80:1 | El estado activo en superficies claras usa grafito más un subrayado o relleno; el coral solo indica «acción principal» o «activo en la barra superior grafito» (5,53:1). |
| Grafito `#18243B` sobre `#0F1828` | 1,15:1 | Regla del manual: no se usa grafito sobre fondos oscuros. |

- **DIS-10**: cada par texto/fondo de los tokens cumple su mínimo. *Aceptación*: la prueba `gafira-theme::contrast` calcula todos los pares de la sección `contrast.pairs` del JSON del §3 (84 pares: 41 claros, 41 oscuros y 2 de `splash`; los mismos de las tablas anteriores), 0 fallos, y comprueba que los pares de `contrast.rejected` no superan el mínimo de su uso (ARQ-61).
- **DIS-11**: el anillo de foco tiene ≥ 3:1 contra el fondo adyacente en ambos modos (14,83:1 claro; 7,82:1 oscuro).
- **DIS-12**: los colores de estado de los objetos del lienzo (selección, guías, sangrado) se dibujan con doble trazo (color + halo del fondo) para ser visibles sobre cualquier contenido del usuario.

---

## 5. Logo e icono de la aplicación

Se respetan las reglas del manual: isotipo de tres piezas (base grafito, panel coral superior, panel verde agua lateral); área de protección X = 1/5 del ancho del isotipo; el wordmark «Gafira» es vectorial y nunca se reescribe con una fuente; no se estira, rota, recolorea, ni se le aplican sombras, contornos o degradados; no se coloca grafito sobre fondos oscuros; no hay marcas de agua permanentes; no hay logos enormes en el editor.

### 5.1 Uso en la aplicación

| Lugar | Variante (archivo existente) | Tamaño | Fondo |
|---|---|---|---|
| Barra superior (botón Inicio) | Símbolo `gafira-logo-symbol.svg` sobre pastilla `canvas` | 32×32 px (mínimo 24×24) | La barra superior es grafito: el símbolo va sobre una pastilla `#FAFAF7` de 40×40 px con radio `sm`, para no poner grafito sobre oscuro. |
| Inicio | Horizontal `gafira-logo-primary.svg`; en modo oscuro `gafira-logo-dark-background.svg` | ≥ 160 px de ancho (recomendado 192) | Claro: `bg`. Oscuro: `bg` oscuro. |
| Acerca de | Apilado `gafira-logo-stacked.svg` | ≥ 120 px de ancho (recomendado 160) | `surface`. Muestra versión, licencias de terceros y OFL de las fuentes. |
| Instalador MSI (WiX) | Horizontal sobre `canvas`; banner de 493×58 px y fondo de diálogo de 493×312 px | Logo ≥ 160 px | `#FAFAF7`, sin degradados. |
| Imagen de DMG | Apilado ≥ 120 px sobre `canvas` | Ventana 660×400 pt | `#FAFAF7`. |
| Pantalla de arranque (D-09, §7.10) | Horizontal `gafira-logo-primary.svg` (isotipo y texto «Gafira»), siempre sobre el panel, **nunca sobre la imagen de fondo**; barra de carga real, sin progreso falso | 200 px de ancho visible del arte (mínimo 160) | Panel `splash.panel` (`bg` del modo claro, `#FAFAF7`) en cualquier modo, sobre la ilustración `assets/splash_screen.jpg`. |
| Exportaciones | **Nunca** se inserta el logo de Gafira ni marca de agua | — | — |

### 5.2 Variantes que faltan (el manual las cita, el repositorio no las contiene)

`gafira-logo-symbol-dark` (símbolo para fondos oscuros), `gafira-logo-mono-navy` y `gafira-logo-mono-white` (una tinta) y las variantes monocromas del símbolo se **crean** a partir de los SVG existentes sin modificar la geometría. `xtask gen-brand` las produce y falla si la geometría difiere del original. Los tokens JSON y el icono de aplicación se especifican aquí (§3 y §5.3).

### 5.3 Icono de la aplicación

Composición: baldosa de esquinas redondeadas `#FAFAF7` con el símbolo centrado ocupando el 62 % del lado (el área de protección X queda dentro de la baldosa). La baldosa evita el grafito sobre fondo oscuro del dock o de la barra de tareas en modo oscuro. Sin sombras añadidas en el archivo fuente; las sombras del SO las pone el sistema.

| Plataforma | Archivo | Tamaños | Generación |
|---|---|---|---|
| Windows | `gafira.ico` (multi-resolución) | 16, 24, 32, 48, 64, 128, 256 px | `xtask gen-icons` rasteriza el SVG por tamaño; los de 16–32 px usan una versión con trazos simplificados (más grueso) si el símbolo pierde legibilidad (PROPUESTA) |
| macOS | `gafira.icns` | 16, 32, 64, 128, 256, 512, 1024 px (con variantes @2x) | Cuerpo de 824×824 px dentro del lienzo de 1024 px (plantilla de iconos de macOS; VERIFICAR contra la guía vigente) |
| Linux | PNG en `hicolor` | 16, 24, 32, 48, 64, 128, 256, 512 px | + `gafira.svg` en `scalable/apps` |
| Asociación `.gaf` | `gafira-doc.ico` / `.icns` / PNG | Los mismos | Símbolo sobre una hoja con esquina doblada; el texto «.gaf» no se rotula en el icono |

- **DIS-13**: ningún icono de aplicación se genera con herramientas externas manuales; todos salen de `xtask gen-icons` a partir de los SVG de la marca. *Aceptación*: reconstruir iconos desde cero produce archivos idénticos (hash estable) y los tamaños de la tabla existen.
- **DIS-14**: el símbolo en la interfaz nunca mide menos de 24×24 px ni más de 48×48 px en el editor. *Aceptación*: prueba de capturas golden de la barra superior.

---

## 6. Navegación y flujo

### 6.1 Mapa de navegación

```text
                      ┌──────────────────────────┐
   (arranque) ───────►│ Recuperación tras fallo  │── Recuperar / Último guardado / Descartar
        │ (si hay     └────────────┬─────────────┘
        │  recuperación)           ▼
        └──────────────►  INICIO  ─────────────────────────────┐
                           │  │  │                              │
        Empezar a diseñar  │  │  └─ Proyectos recientes ────────┤
        Explorar plantillas│  └──── Abrir archivo .gaf       │
                           ▼                                    ▼
                 ┌───────────────────┐   Nueva escena   ┌───────────────────┐
                 │ EDITOR DE DISEÑO  │◄────────────────►│  EDITOR DE VIDEO  │
                 └───┬─────┬─────┬───┘   (misma sesión)  └───┬─────┬─────┬───┘
   Tamaño ───────────┘     │     └── Identidad visual ──┐    │     │     └── Cola de exportación
   Cambiar tamaño          │                            │    │     │
                           ▼                            ▼    ▼     ▼
                        EXPORTAR  ◄─────────────────────────────────┘
        Ajustes ◄─ Archivo ─► Versiones           Paleta de comandos (Ctrl/Cmd+K, desde cualquier pantalla)
```

- **Arranque.** Toda ejecución empieza en la **Pantalla de arranque** (§7.10, D-09). Cuando se cierra aparece Inicio y, si hay trabajo que recuperar, Recuperación tras fallo sobre Inicio. La Pantalla de arranque no tiene navegación propia.
- **Un documento, dos editores.** El Editor de diseño y el Editor de video comparten el mismo `Project` (02, §3.1). Pasar de uno a otro cambia la región inferior (Barra inferior de páginas ↔ Línea de tiempo) y el Inspector; no cambia el archivo.
- **Identidad visual** se abre como pantalla completa desde la pestaña Identidad o desde Archivo → Ajustes; es el único modo con navegación propia a media pantalla.
- **Atrás** siempre existe: `Esc` cierra la superposición activa; el botón Inicio vuelve a Inicio preguntando «¿Guardar los cambios?» solo si hay cambios sin guardar.

### 6.2 Flujo Elegir → Personalizar → Compartir

| Etapa | Qué ve la persona | Acción principal | Atajo | Resultado |
|---|---|---|---|---|
| **Elegir** | Inicio con «¿Qué quieres crear hoy?» y formatos (Redes, Presentación, Impreso, Video vertical, Video horizontal, Tamaño propio) más Plantillas | **Empezar a diseñar** (coral) y «Explorar plantillas» (secundario) | `Ctrl/Cmd+N` | Un proyecto con una página del tamaño elegido, o una plantilla aplicada |
| **Personalizar** | Editor con lienzo, Panel lateral, Barra contextual e Inspector | Editar el contenido (la acción principal es «Exportar», siempre visible) | `Ctrl/Cmd+K` | Documento editado con cambios guardados |
| **Compartir** | Diálogo Exportar con vista previa y ajustes por uso | **Exportar** (coral) | `Ctrl/Cmd+E` | Archivo en disco y aviso «Tu diseño está listo para compartir» con «Mostrar en carpeta» |

- **DIS-15**: cada etapa tiene una sola acción coral en pantalla. *Aceptación*: una prueba de captura por pantalla cuenta botones con `primary.bg`: exactamente 1 por vista (excepto diálogos de confirmación, donde también es 1).
- **DIS-16**: la primera pieza publicable (plantilla → editar texto → exportar PNG) se completa en < 10 min en la prueba de usabilidad con 5 personas sin experiencia previa (cifra del PRD; método en spec 01).

---

## 7. Layout y pantallas

### 7.1 Estructura de ventana

Ventana mínima 1280×720 px (PROPUESTA); se adapta hasta 3840×2160 y a escalas de 100–200 % (HiDPI). Regiones y medidas (tokens `size.*`):

| Región | Medida | Fondo | Ritmo visual |
|---|---|---|---|
| Barra superior | 48 px de alto | `topbar.bg` (grafito) | Grafito: nombre del proyecto, Estado de guardado, Deshacer/Rehacer, Exportar (coral) |
| Riel de pestañas | 64 px de ancho | `topbar.bg` | Iconos de 24 px con etiqueta de 12 px; activo con indicador coral de 3 px |
| Panel lateral | 320 px (redimensionable 280–480) | `bg` | Claro; contenido en tarjetas `surface` con radio `md` |
| Lienzo | resto | `workspace` (gris azulado); páginas en `#FFFFFF` por defecto con sombra 1 | El color del usuario es el protagonista |
| Inspector | 296 px (plegable) | `surface` | Pestañas Posición, Capas, Imagen… |
| Barra contextual | Flotante, 40 px de alto, a 12 px sobre la selección | `surface` + sombra 2, radio `md` | Se reubica debajo si no cabe arriba |
| Barra inferior | 40 px | `bg` | Notas, Zoom, Páginas |

Reglas de adaptación: por debajo de 1440 px el Inspector se pliega a pestañas flotantes; por debajo de 1280 px el Panel lateral se oculta tras el riel (PROPUESTA). Los paneles recuerdan su estado entre sesiones (`SettingsService`).

### 7.2 Inicio

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ [◧ Gafira]   Inicio    Proyectos    Plantillas                       (? Ctrl+K)   [Ajustes]│  ← barra grafito
├──────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│   Haz visibles tus ideas.                                                                │
│   ¿Qué quieres crear hoy?                                                                │
│                                                                                          │
│   ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐                       │
│   │ Redes  │ │Present.│ │Impreso │ │ Video  │ │ Video  │ │Tamaño  │                       │
│   │ 1080²  │ │ 16:9   │ │ A4     │ │ 9:16   │ │ 16:9   │ │ propio │                       │
│   └────────┘ └────────┘ └────────┘ └────────┘ └────────┘ └────────┘                       │
│                                                                                          │
│   [ Empezar a diseñar ]  (coral)      Explorar plantillas  (secundario)                   │
│                                                                                          │
│   Recientes                                                                   Ver todos  │
│   ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐     Tu próxima idea comienza aquí  │
│   │ miniatura│ │ miniatura│ │ miniatura│ │ miniatura│     (si no hay proyectos)          │
│   │ Nombre   │ │ Nombre   │ │ Nombre   │ │ Nombre   │                                    │
│   │ hace 2 h │ │ ayer     │ │ 12 mar   │ │ 3 mar    │                                    │
│   └──────────┘ └──────────┘ └──────────┘ └──────────┘                                    │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

- El eslogan usa `h1` Manrope (en la interfaz inglesa, «Bring your ideas to life.», §12); el logo horizontal va en la esquina o centrado (≥ 160 px, §5.1).
- Las miniaturas salen de `thumbs/<page_id>.webp` (02, §7.1), sin abrir el proyecto. Abrir un proyecto de 20 páginas tarda < 3 s (cifra del PRD).
- **Arranque**: Inicio es interactivo en < 5 s en el hardware de referencia de 02 §6.6 (cifra del PRD; objetivo interno < 2 s, FIJADA, D-03), contando la Pantalla de arranque que lo precede (§7.10: primer frame < 300 ms, visible al menos 1,2 s). El scrubbing de video mantiene ≥ 24 actualizaciones por segundo (FIJADA).

### 7.3 Editor de diseño

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│[◧] Archivo Edición Ver  Tamaño │ Mi cartel ✎ │ ↶ ↷ │ Cambios guardados. │   Versiones [Exportar]│
├────┬─────────────────┬──────────────────────────────────────────────┬────────────────────┤
│ ▦  │ ? Buscar plant. │  0     200    400    600    800   (Reglas)   │ Posición Capas Imagen│
│Plan│ ┌─────┐ ┌─────┐ │ 0┌────────────────────────────┐             │ X [  120 ] Y [ 240 ]│
│ ◇  │ │tpl 1│ │tpl 2│ │  │  Página 1   ⌃ ⌄ ◉ ▣ ⧉ ✕   │             │ An [ 640 ] Al [ 360]│
│Piez│ └─────┘ └─────┘ │  │  ┌────────┐                │             │ ▣ proporción       │
│ T  │ ┌─────┐ ┌─────┐ │  │  │ Barra  │ Título         │             │ Giro [ 0° ]         │
│Text│ │tpl 3│ │tpl 4│ │  │  │contex. │                │             │ Alinear ▤▥▦ ▧▨▩     │
│ ◉  │ └─────┘ └─────┘ │  └────────────────────────────┘             │ Orden: ▲ ▼ ⤒ ⤓     │
│Iden│  Plantillas     │  ┌────────────────────────────┐             │ Opacidad ───●── 100 │
│ ⬒  │                 │  │  Página 2                  │             │                     │
│Bibl│                 │  └────────────────────────────┘             │                     │
│ ✎  │                 │                                              │                     │
│Dibu│                 │                                              │                     │
├────┴─────────────────┴──────────────────────────────────────────────┴────────────────────┤
│ Notas │                                    │ ─ ───●──── + 100 % ▾ │ ▭ 1 de 2 ▾ │ ⊞ Páginas│
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

- Las páginas se apilan en vertical con cabecera por página: título editable y acciones Subir, Bajar, Ocultar, Bloquear, Duplicar y Eliminar.
- La **Barra contextual** muestra, según la selección: fuente/tamaño/color (Texto); Zona de edición y Ajuste automático (imagen); relleno y contorno (Forma); Recortar; Duplicar; Eliminar; «⋯ Más».
- El Zoom llega a 5–3200 % (cifra del PRD) con slider, campo numérico y desplegable (Ajustar, 50 %, 100 %, 200 %).

### 7.4 Editor de video

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│[◧] Archivo Edición Ver │ Reel mayo ✎ │ ↶ ↷ │ Cambios guardados. │ Cola ▾ │ Versiones [Exportar]│
├────┬─────────────────┬─────────────────────────────────────────┬─────────────────────────┤
│Medi│  Medios         │            Vista previa 9:16            │ Clip  Fotogramas  Color │
│Soni│  ┌──┐ ┌──┐ ┌──┐ │           ┌──────────┐                  │ Velocidad ──●── 1,0×    │
│Mov.│  │m1│ │m2│ │m3│ │           │          │                  │ Volumen ───●─── 80      │
│Text│  └──┘ └──┘ └──┘ │           │  ▶ 00:12 │                  │ Fundido ent. [0,5] s    │
│Subt│  Importar…      │           └──────────┘                  │ Invertir  Congelar      │
├────┴─────────────────┴─────────────────────────────────────────┴─────────────────────────┤
│ ▶ ⏮ ⏭  00:12:04 / 01:00:00   ? − ───●─── + (zoom de la Línea de tiempo)   Mostrar todas │
│ regla │0s    5s    10s   15s   20s   25s   30s                                            │
│ Texto │      [ Título ──────]        [ Tercio inferior ───]                               │
│ Superp│            [ logo ────────────────]                                               │
│ Video │[ clip 1 ─────────]◇[ clip 2 ──────────]+[ clip 3 ───────]  ⎸cabezal               │
│ Audio │[ ░░ música ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]                    │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

- `◇` entre dos clips es el control **Transición**; `+` es **Nueva escena**. El Cabezal se arrastra o se mueve con `←/→` (un fotograma) y `Shift+←/→` (un segundo).
- La Línea de tiempo mide ≥ 220 px y se puede ampliar arrastrando su borde superior. Las pistas siguen el modelo de 02 (§3.1): Texto, Superposiciones, Video, Audio y Subtítulos.
- Los diseños de interacción de la Parte B (curvas, máscaras, Medidores, Mezclador) están en la spec 05.

### 7.5 Exportar

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ Exportar                                                          [✕]   │
├───────────────────────────────┬─────────────────────────────────────────┤
│  Vista previa                 │ Formato   [ PNG ▾ ]  Imagen sin pérdida  │
│  ┌─────────────────────┐      │ Páginas   (•) Todas ( ) Selección ( ) 1-3│
│  │                     │      │ Escala    [ 1× ▾ ] → 1080 × 1080 px      │
│  │   (página actual)   │      │ Calidad   Comprimir ─●── Alta            │
│  │                     │      │ [✓] Fondo transparente                   │
│  └─────────────────────┘      │ ▸ Avanzado: sangrado y marcas de corte,  │
│  Tamaño estimado: 1,2 MB      │   aplanar, accesible, contraseña         │
│  ◀ 1 / 3 ▶                    │ [✓] Recordar estos ajustes               │
├───────────────────────────────┴─────────────────────────────────────────┤
│ Guardar en: …/Documentos/Gafira ▾  (Examinar)     [ Cancelar ] [Exportar]│
└─────────────────────────────────────────────────────────────────────────┘
```

- Las opciones se generan desde `Exporter::options_schema` (02, §4.5): el diálogo no codifica formatos a mano. Opciones que el documento no admite se muestran deshabilitadas con la razón en un tooltip.
- Al terminar: «Tu diseño está listo para compartir» y el botón «Mostrar en carpeta». En video, el trabajo pasa a la Cola de exportación.

### 7.6 Cambiar tamaño

```text
┌───────────────────────────────────────────────────┐
│ Cambiar tamaño                                    │
│ ? Buscar tamaño…                                   │
│ Destinos:  Redes │ Impresión │ Presentaciones │   │
│            Video │ Web │ Personalizado           │
│ ┌──────────────┐ ┌──────────────┐ ┌─────────────┐│
│ │ Publicación  │ │ Historia     │ │ Banner      ││
│ │ 1080 × 1350  │ │ 1080 × 1920  │ │ 1200 × 630  ││
│ │ ☑            │ │ ☐            │ │ ☐           ││
│ └──────────────┘ └──────────────┘ └─────────────┘│
│ Personalizado: Ancho [  ] Alto [  ] [px ▾] ▣ ⟲⟳  │
│ Vista previa por destino: ▭ ▭ ▭ (elementos movidos)│
│ [ Duplicar y adaptar ]        [ Adaptar este diseño ]│
└───────────────────────────────────────────────────┘
```

### 7.7 Identidad visual

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ Identidad visual     Identidad activa: [ Estudio Aurora ▾ ]  [ + Nueva identidad ]        │
├───────────┬──────────────────────────────────────────────────────────────────────────────┤
│ Logos     │ Colores   ■ Principal  ■ Acento  ■ Neutro  ■ Fondo        [ + Añadir color ]  │
│ Colores   │ Tipografías  Titular: Manrope ▾   Cuerpo: Inter ▾                             │
│ Tipografías│ Estilos  «Título», «Cuerpo», «Botón»   (cambiar uno actualiza todo el diseño)│
│ Plantillas│ Logos     [ arrastra tus logos aquí ]                                         │
│ Estilos   │ Plantillas propias  [ miniaturas ]                                            │
│ Voz       │                                              [ Aplicar a este diseño ]        │
└───────────┴──────────────────────────────────────────────────────────────────────────────┘
```

- Cada identidad es un objeto `VisualIdentity` (02, §3.2). Un cambio de color de muestra se aplica con un único `Command` (`SetSwatch`). La pestaña Texto muestra las Tipografías de la identidad activa.

### 7.8 Ajustes

Secciones (lista a la izquierda, contenido a la derecha): **General** (Idioma: Sistema, Español o English, detalle en §16; carpeta de proyectos), **Apariencia** (Claro, Oscuro, Sistema), **Rendimiento** (GPU o CPU; límite de memoria y de VRAM; Copias ligeras automáticas), **Guardado** (autoguardado entre 5 y 15 s, por defecto 10; Versiones), **Atajos** (tabla editable con búsqueda y detección de conflictos), **Accesibilidad** (reducir movimiento: Sistema/Sí/No; tamaño de interfaz 100–200 %), **Privacidad** («Tus diseños y archivos no salen de tu equipo. La única conexión de Gafira es la de tu licencia.»), **Licencia** (estado y gestión de la licencia; diseño diferido: ver spec 06, pendiente, se redacta al final), **Funciones locales opcionales** (segmentadores y reescritura de texto desactivados por defecto), **Acerca de** (versión, logo apilado, licencias de terceros y OFL).

### 7.9 Recuperación tras fallo

```text
┌────────────────────────────────────────────────┐
│ Encontramos trabajo sin guardar                │
│ «Mi cartel» · última copia hace 8 s            │
│ [ Recuperar ]  [ Abrir el último guardado ]  [ Descartar ] │
└────────────────────────────────────────────────┘
```

- Se muestra al arrancar solo si hay recuperación (02, §7.2). Con varios proyectos, lista con miniatura y hora. El primer botón (coral) es **Recuperar**. Descartar pide confirmación.
- **DIS-17**: la recuperación muestra el resultado en < 3 s para un proyecto de 20 páginas.

- **DIS-18** (pantallas): las ocho pantallas existen con los nombres del §2 y navegación por teclado completa. *Aceptación*: prueba E2E de teclado que recorre cada una (ARQ-63). La Pantalla de arranque (§7.10) no cuenta entre las ocho porque no tiene controles salvo en el estado de error; sus pruebas son DIS-80..DIS-87.

### 7.10 Pantalla de arranque (D-09)

Pantalla breve que aparece al iniciar Gafira, antes de Inicio. Muestra la marca y el progreso real de la carga. Su nombre de interfaz es «Pantalla de arranque» / «Startup screen» (PROPUESTA, §2.1 y §2.8.1; no se llama «Inicio» porque Inicio es otra pantalla). La arquitectura está en 02 §8.4 (ARQ-65..ARQ-69, ADR-10).

#### 7.10.1 Wireframe (tamaño por defecto, 800×450 px lógicos)

```text
 ◄─────────────────────────── 800 px lógicos ───────────────────────────►
┌────────────────────────────────────────────────────────────────────────┐ ▲
│                                                                        │ │
│   Imagen de fondo a sangre (assets/splash_screen.jpg)                  │ │
│   recorte «cubrir»; el centro de la imagen va al centro de esta zona   │ │ 322 px
│   nada de logo, texto ni controles sobre la imagen                     │ │ de imagen
│                                                                        │ │ visible
│                                                                        │ ▼
├────────────────────────────────────────────────────────────────────────┤ ▲
│  ┌┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┐                                              │ │
│  ┊ X  [◧ Gafira] 200 px ┊                     Haz visibles tus ideas.  │ │ 128 px
│  └┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┘                                              │ │ panel
│  ━━━━━━━━━━━━━━━━━━━━━━──────────────────────────────    Versión 1.0.0 │ │
│                                                                        │ ▼
└────────────────────────────────────────────────────────────────────────┘
   ┄ área de protección X del logo     ━ relleno coral     ─ pista grafito
```

#### 7.10.2 Ventana, dimensiones y HiDPI

- **Ventana propia** sin decoraciones ni bordes, no redimensionable, de **800×450 px lógicos** (16:9, token `splash.window`, PROPUESTA), centrada en el monitor donde se abrirá la ventana principal (el último usado según los ajustes o, si no hay dato, el principal). Aparece en la barra de tareas o en el Dock con el icono y el nombre «Gafira», para que la persona sepa que la app está arrancando. No está siempre encima de otras ventanas.
- **Tamaño de interfaz**: el tamaño lógico se multiplica por el tamaño de interfaz de Ajustes → Accesibilidad (100–200 %, DIS-35). Si el resultado supera el 80 % del área de trabajo del monitor, se reduce con la misma proporción hasta caber, con un mínimo de 480×270 px lógicos.
- **HiDPI**: todas las medidas son px lógicos; los px físicos son las medidas por el factor de escala del monitor. La imagen se reescala una sola vez, en segundo plano, a los px físicos exactos del área que cubre (filtro de calidad, 02 ARQ-65) y se dibuja 1:1, sin reescalado en la GPU. Si la ventana pasa a un monitor con otro factor, se reescala de nuevo en segundo plano y mientras tanto se dibuja la versión anterior.
- **Cobertura de la imagen actual** (1672×941 px) a 800×450 lógicos: al 100 % se reduce al 48 %; al 150 %, al 72 %; al 200 % (1600×900 px físicos), al 96 %; al 250 % se amplía un 20 % y al 300 % un 43 %, con pérdida de nitidez visible. Por eso la resolución recomendada para una imagen nueva es mayor (§7.10.10).
- **Sin ventana propia**: si la plataforma no permite una ventana sin decoraciones o centrada (en Wayland la posición la decide el compositor, VERIFICAR), o si falla su creación, la Pantalla de arranque se dibuja dentro de la ventana principal, a su tamaño, con las mismas reglas de recorte y composición. 02 ARQ-65 fija este respaldo.

#### 7.10.3 Regla de recorte y ajuste («cubrir»)

Con la ventana de `W×H` px físicos, el panel de `P` px físicos y la imagen de `w×h` px:

1. **Escala**: `k = max(W / w, H / h)`. La imagen siempre cubre toda la ventana, sin bandas vacías y sin deformarse.
2. **Horizontal**: el centro de la imagen coincide con el centro de la ventana; se recorta lo mismo a izquierda y derecha: `x = (W − w·k) / 2`.
3. **Vertical**: el centro de la imagen se coloca en el centro de la **zona visible** sobre el panel (`Hv = H − P`), sin dejar huecos: `y = clamp(Hv / 2 − h·k / 2, H − h·k, 0)`. Así el motivo central de la ilustración queda a la vista y no tapado por el panel.
4. **Punto focal fijo**: el centro geométrico de la imagen. No hay archivo de configuración del punto focal; quien cambie la imagen debe centrar en ella lo importante (§7.10.10).
5. **Ventana muy ancha** (proporción > 16:9): la escala la fija el ancho y se recorta arriba y abajo; con la regla 3 queda a la vista la franja central de la imagen.
6. **Ventana muy estrecha** (proporción < 16:9, por ejemplo con el respaldo dentro de la ventana principal): la escala la fija el alto y se recortan los laterales por igual; en la imagen actual el guacamayo (a la izquierda) puede quedar fuera y se mantienen las montañas del centro.
7. **Zona visible insuficiente**: si `Hv` es menor que `splash.image.min_visible_height` (96 px lógicos) o el ancho es menor que 360 px lógicos, la imagen no se dibuja y el panel ocupa toda la ventana con su contenido centrado.
8. **Composición del panel según el ancho**: a partir de 640 px lógicos, logo a la izquierda y eslogan a la derecha (wireframe). Por debajo de 640 px, o si el eslogan no cabe en dos líneas junto al logo (por ejemplo en seudo-localización), el eslogan pasa debajo del logo, alineado a la izquierda, y el panel crece a 168 px.

#### 7.10.4 Composición del panel

| Elemento | Especificación | Tokens |
|---|---|---|
| Panel | Banda inferior de ancho completo y `splash.panel.height` (128 px); sin borde, sombra ni radio; margen interior `splash.panel.padding` (32 px) a los lados, 20 px arriba y abajo | Fondo `splash.panel` (`bg` del modo claro) |
| Logo | Horizontal principal `gafira-logo-primary.svg` (isotipo y texto «Gafira», `docs/manuals/marca/assets/`), sin recolorear ni deformar. Ancho visible del arte `splash.logo.width` = 200 px (mínimo del manual: 160 px), unos 61 px de alto; arriba a la izquierda del panel | — (arte de marca, no se tiñe) |
| Área de protección | X = 1/5 del ancho visible del isotipo. El isotipo ocupa el 27,9 % del arte medido en el SVG (357 de 1280 unidades), es decir 55,8 px a 200 px de ancho: X = 11,2 px, que se redondea a **12 px** por cada lado. Ningún texto, borde, barra ni píxel de la imagen entra en esa zona | — |
| Eslogan | «Haz visibles tus ideas.» / «Bring your ideas to life.» en Manrope, estilo `h3` (24/32 px, peso 650), alineado a la derecha y centrado en vertical con el logo; hasta dos líneas | `text` |
| Barra de carga | Fina, de `splash.progress.height` (4 px) y radio `pill`; va de 32 px desde la izquierda hasta 16 px antes de la versión; su borde inferior queda a 20 px del borde del panel | Pista `splash.progress.track`, relleno `splash.progress.fill` |
| Versión | «Versión {versión}» / «Version {version}» en `caption` (12/18 px), alineada a la derecha y centrada con la barra | `text.muted` |
| Mensaje de carga lenta | Sustituye a la versión (§7.10.5) | `text.secondary`, `body-sm` |
| Estado de error | Icono de error de 20 px y mensaje en lugar de la barra; botones «Reintentar» (primario) y «Cerrar» (secundario) de 32 px a la derecha (§7.10.5) | `error.fg`, `primary.*`, `secondary.*`, `focus.ring` |

- Todos los colores son tokens de §3 y se resuelven en `theme.rs` (DIS-04): `splash.*` y, para textos y botones, los de `modes.light`, también con la app en modo oscuro.
- Ningún elemento usa coral salvo la barra y, solo en el estado de error, el botón «Reintentar». La barra es un indicador de estado, no una acción, por lo que el estado de carga tiene **cero** botones coral; DIS-15 se cumple en el estado de error con uno.
- Sin degradados, sombras, contornos ni texturas sobre el logo; sin velos sobre la imagen; el logo nunca se coloca sobre la imagen (manual §7.10).

#### 7.10.5 Estados y tiempos

| Estado | Cuándo | Qué se ve |
|---|---|---|
| **Cargando** | Desde el primer frame (< 300 ms tras lanzar el proceso, PROPUESTA) | Imagen, panel, logo, eslogan, barra con el progreso real y versión. Si la imagen aún no está decodificada, su zona se pinta con `splash.panel` y la imagen aparece en cuanto está lista |
| **Lista, en espera mínima** | El arranque termina antes de 1,2 s desde el primer frame | La barra llega al 100 % y se mantiene hasta cumplir **1,2 s** (D-09) |
| **Cierre** | El arranque terminó y pasaron al menos 1,2 s | Se muestra la ventana principal (Inicio y, si procede, Recuperación tras fallo) y la Pantalla de arranque se funde y se destruye (§7.10.6) |
| **Carga lenta** | A los **5 s** desde el primer frame (PROPUESTA) sin haber terminado los pasos necesarios para mostrar Inicio | Igual que «Cargando», con el mensaje «Esto está tardando más de lo normal. Seguimos cargando.» en lugar de la versión, y un único anuncio cortés para lectores de pantalla |
| **Error de arranque** | El arranque informa de un fallo, o pasan 20 s (PROPUESTA) sin ningún evento de progreso | Icono y mensaje «No pudimos abrir Gafira. Inténtalo de nuevo.»; botones «Reintentar» (repite el arranque desde el principio) y «Cerrar» (sale). El detalle técnico va al registro local (02 §7.3), nunca a la pantalla (DIS-41) |

- **Qué espera y qué no**: la Pantalla de arranque solo espera los pasos **bloqueantes**, los imprescindibles para pintar Inicio (ajustes, idioma, tokens y fuentes, estado de la licencia leído sin red, registros, ventana principal). Los pasos **diferibles** (índice de plantillas, miniaturas de recientes, escaneo de fuentes del sistema, preparación del render GPU del lienzo) siguen en segundo plano tras cerrarla, e Inicio muestra sus marcadores de carga. Así la duración en el caso normal es `max(1,2 s, pasos bloqueantes)`, dentro del < 5 s de CAL-08. La clasificación de pasos está en 02 §8.4.
- **Progreso real**: la barra solo avanza con los eventos `StartupProgress` del arranque (02 ARQ-68); nunca avanza por tiempo, nunca retrocede y no tiene modo indeterminado animado. Antes del primer evento está en 0 %.
- **Cierre de la persona**: cerrar con el atajo del SO (Alt+F4, Cmd+Q) cancela el arranque y sale sin dejar archivos a medias.

#### 7.10.6 Movimiento y reducción de movimiento

| Animación | Sin reducción de movimiento | Con reducción de movimiento (§11) |
|---|---|---|
| Aparición de la ventana | Inmediata, sin fundido (no se retrasa la primera imagen de la marca) | Inmediata |
| Aparición de la imagen si llega después del primer frame | Fundido de opacidad `motion.fast` (120 ms), curva `enter` | Inmediata |
| Avance de la barra hasta el nuevo valor | Interpolación `motion.fast` (120 ms), curva `standard` | Salto directo al valor |
| Paso a carga lenta o a error | Cambio de opacidad `motion.base` (160 ms) | Inmediato |
| Cierre | Fundido de salida de la ventana `motion.base` (160 ms), curva `exit`, si la plataforma admite opacidad de ventana (VERIFICAR por plataforma); si no, cierre inmediato | Cierre inmediato, sin fundido |

- No hay zoom lento, desplazamiento ni paralaje sobre la imagen, ni nada que parpadee. Ninguna transición supera 220 ms (DIS-40).
- La preferencia se lee antes del primer frame: `gafira-platform::prefers_reduced_motion()` y, si existe, la opción de Ajustes → Accesibilidad (02 §8.3).

#### 7.10.7 Accesibilidad

- **Foco**: en los estados de carga la Pantalla de arranque no tiene elementos enfocables, no pide el foco del sistema al crearse (la ventana se crea sin activarse, VERIFICAR por plataforma) y no lo quita a otra app si la persona cambió de ventana mientras carga: en ese caso la ventana principal se abre sin robar el foco y pide atención en la barra de tareas o el Dock. Solo el estado de error toma el foco, porque exige una decisión: el foco va a «Reintentar» y `Tab` alterna con «Cerrar».
- **Árbol de AccessKit**: ventana con nombre «Gafira»; logo con rol de imagen y nombre accesible «Gafira»; eslogan como texto; barra con rol de indicador de progreso, nombre «Cargando Gafira» / «Loading Gafira» y valor de 0 a 100; versión como texto. El nodo raíz declara el idioma resuelto (DIS-78).
- **Texto alternativo de la imagen de fondo**: es **decorativa** y se excluye del árbol accesible (sin nombre), porque no aporta información y cambia cuando se sustituye el archivo. Por eso cambiar la imagen no exige cambiar ningún texto. El texto alternativo que transmite la marca es el del logo («Gafira»).
- **Anuncios** (región en vivo): «Cargando Gafira.» una vez al aparecer (cortés); el mensaje de carga lenta una vez (cortés); el mensaje de error una vez (asertivo). Los porcentajes de la barra no se anuncian. Al cerrar no se anuncia nada: Inicio anuncia su propio título.
- **Contraste**: todos los pares cumplen AA (§4, tabla «Pantalla de arranque»). El color nunca es la única señal: el error lleva icono y texto (DIS-33).

#### 7.10.8 Idioma

- El eslogan, el nombre accesible de la barra, la versión y los mensajes salen del catálogo de `gafira-i18n` (02 ARQ-97) en el idioma **resuelto antes del primer frame** con la regla de D-08 (§16.2): la preferencia guardada o, con «Sistema», español para cualquier variante de español e inglés en los demás casos.
- Textos: «Haz visibles tus ideas.» / «Bring your ideas to life.» (manual §6.1, sin cambios) y los mensajes de §12. Nunca se mezclan los dos idiomas (DIS-01). Solo se traduce la palabra «Versión» / «Version»; el número de versión no se localiza.

#### 7.10.9 Microcopy de marca

Tono del §12: frases breves, sin exclamaciones ni superlativos. El eslogan es el principal del manual; no se usa «Tus ideas toman forma.», reservado a la carga del proyecto (§12). Las filas nuevas (Pantalla de arranque) están en la tabla del §12 con su versión inglesa.

#### 7.10.10 Cómo cambiar la imagen

1. **Archivo**: sustituir `assets/splash_screen.jpg` por la imagen nueva con el **mismo nombre base**: `assets/splash_screen.jpg` o `assets/splash_screen.png`. Debe existir **exactamente uno** de los dos; si hay dos o ninguno, la compilación falla con un mensaje que lo explica (02 ARQ-67).
2. **Formatos**: JPEG (base o progresivo, 8 bits, RGB o YCbCr; no CMYK) o PNG (8 bits por canal, RGB o RGBA; la transparencia se compone sobre `splash.panel`). La imagen se trata como sRGB: convertirla a sRGB antes de guardarla, porque los perfiles ICC incrustados se ignoran.
3. **Proporción**: **16:9** recomendada (la actual es 1672×941, 1,777:1). La compilación avisa si la proporción está fuera de 1,7–1,85:1 (más recorte del previsto) y falla si está fuera de 1,5–2,1:1.
4. **Resolución**: mínimo **1600×900 px** (cubre 800×450 al 200 % sin ampliar); recomendada **2000×1125 px** o más (cubre el 250 %); máximo **2880×1620 px** (su decodificación ocupa unos 17,8 MiB de memoria temporal, que se libera al cerrar la pantalla).
5. **Tamaño del archivo**: hasta **1 MB** recomendado y **2 MB** como límite que hace fallar la compilación (PROPUESTA, 02 ARQ-69). Un JPEG de calidad 85–90 a 2000×1125 suele quedar por debajo de 1 MB.
6. **Composición y legibilidad**:
   - La franja inferior (128 de 450 px, el 28 %) queda tapada por el panel: no poner allí nada importante.
   - Centrar el motivo principal en la zona visible: el punto focal es el centro de la imagen y los laterales pueden recortarse en ventanas estrechas.
   - **El logo nunca va sobre la imagen**: no hace falta dejar hueco para él, pero tampoco se debe incluir en la ilustración ningún logo, texto ni marca de agua (el logo ya está en el panel).
   - Sin texto dentro de la imagen: no se traduciría y no se leería en todos los tamaños.
   - Sin destellos ni patrones de alto contraste que parpadeen al reescalar.
7. **Licencia**: actualizar la entrada de la imagen en `assets/LICENSES.toml` (origen, autoría, licencia y hash; ARQ-75). Sin entrada válida, `xtask` falla y el archivo no se empaqueta.
8. **Comprobación**: recompilar (`cargo build`), ejecutar las pruebas de la Pantalla de arranque y aprobar las nuevas capturas golden (DIS-81, DIS-67). No se toca código ni el catálogo de textos.

#### 7.10.11 Requisitos

- **DIS-80** (composición y marca): la Pantalla de arranque muestra el logo `gafira-logo-primary.svg` con 200 px de ancho visible, su área de protección de 12 px sin invadir, el eslogan en `h3` Manrope, la barra y la versión sobre el panel `splash.panel`, con el layout de §7.10.4; ningún píxel del logo ni de su área de protección se dibuja sobre la imagen; todos los colores salen de tokens; en el estado de carga hay 0 botones coral. *Aceptación*: capturas golden en claro y oscuro del sistema (deben ser idénticas) a 100, 150 y 200 %; una prueba geométrica comprueba que el rectángulo del logo más X queda dentro del panel y que ningún otro elemento lo cruza; DIS-04 da 0 colores literales.
- **DIS-81** (recorte y HiDPI): la imagen cubre siempre toda la ventana sin deformarse, con la regla de §7.10.3. *Aceptación*: prueba unitaria de la función de recorte con ventanas de 800×450, 1280×400, 480×600, 2000×1125 y 300×200 px (este último, sin imagen) y factores de 1, 1,5, 2 y 2,5; resultado: 0 bandas vacías, desviación de proporción < 0,5 %, centro según la regla 3; capturas golden en 800×450 a 100 y 200 % y en el respaldo dentro de la ventana principal a 1280×720.
- **DIS-82** (tiempos): primer frame < 300 ms desde el lanzamiento (PROPUESTA) en gama media; visible al menos 1,2 s desde el primer frame (D-09); se cierra en ≤ 1 frame tras cumplir las dos condiciones de cierre; carga lenta a los 5 s y error tras 20 s sin progreso (PROPUESTA). *Aceptación*: `gafira --bench-startup` informa de `splash_first_frame_ms` y `splash_visible_ms` (CAL-38, CAL-39); pruebas con un `StartupProgress` simulado: arranque de 100 ms → visible 1,2 s ± 1 frame; arranque de 3 s → visible 3 s ± 1 frame; arranque bloqueado a 6 s → mensaje de carga lenta a los 5 s; sin eventos durante 20 s → estado de error.
- **DIS-83** (barra de carga real): la barra solo refleja `StartupProgress`, es monótona, empieza en 0 % y llega al 100 % antes de cerrarse; no hay animación indeterminada ni avance por tiempo. Contraste: S1 ≥ 3:1 y S2 ≥ 3:1 (§4). *Aceptación*: prueba con una secuencia de eventos desordenada (0,2; 0,5; 0,4; 1,0): la barra muestra 0,2; 0,5; 0,5; 1,0; con eventos congelados, el valor dibujado no cambia en 3 s; DIS-10 cubre los pares S1 y S2.
- **DIS-84** (movimiento): las animaciones son las de §7.10.6 y ninguna supera 220 ms; con reducción de movimiento todas duran 0 ms y no hay fundidos. *Aceptación*: la prueba de DIS-40 registra las transiciones de la Pantalla de arranque con y sin la preferencia (ARQ-64).
- **DIS-85** (accesibilidad): en los estados de carga la Pantalla de arranque no tiene nodos enfocables ni pide el foco; la imagen de fondo no está en el árbol accesible; el logo se llama «Gafira»; la barra tiene rol de progreso y valor; se emiten exactamente los anuncios de §7.10.7; en el estado de error el foco está en «Reintentar». *Aceptación*: prueba con inspector de accesibilidad en los cuatro estados; una prueba E2E abre otra ventana durante el arranque y comprueba que, al terminar, Gafira no le quita el foco (VERIFICAR la posibilidad por plataforma).
- **DIS-86** (idioma): el eslogan y los textos de la Pantalla de arranque están en el idioma resuelto por D-08 desde el primer frame. *Aceptación*: prueba con perfil vacío y SO en `es-MX`, `en-GB` y `fr-FR`: el primer frame muestra «Haz visibles tus ideas.», «Bring your ideas to life.» y «Bring your ideas to life.»; con la preferencia guardada `es` y SO en `en-US`, español; 0 claves visibles; capturas en seudo-localización sin truncados (DIS-76).
- **DIS-87** (imagen reemplazable): cambiar la imagen solo requiere sustituir el archivo, actualizar su entrada en `assets/LICENSES.toml` y recompilar; la compilación falla si el archivo falta, está duplicado (`.jpg` y `.png`), está corrupto o incumple los límites de §7.10.10. *Aceptación*: pruebas de la build de 02 ARQ-67 y ARQ-69 con cinco archivos de prueba (válido JPEG, válido PNG, ausente, duplicado, truncado): solo los dos válidos compilan; ningún archivo de código cambia entre las dos compilaciones válidas.

---

## 8. Catálogo de componentes

Estados comunes: **reposo, hover, foco, pressed, disabled, error, success, seleccionado**. Todos los tokens salen del §3; ningún componente define colores propios. Toda interacción con puntero tiene equivalente de teclado (§10).

### 8.1 Componentes y estados

| Componente | Reposo | Hover | Foco | Pressed | Disabled | Error / éxito |
|---|---|---|---|---|---|---|
| **Botón primario** (una por vista) | `primary.bg` + `primary.fg`, radio `md`, 32/40 px | `primary.bg.hover` | Anillo 2 px `focus.ring` con halo y separación de 2 px | `primary.bg.pressed` | `secondary.bg` + `text.disabled`, sin sombra | Error: no aplica; en carga muestra rueda + «Exportando…» |
| **Botón secundario** | `secondary.bg` + `text` | `secondary.bg.hover` | Igual | `secondary.bg.pressed` | `text.disabled` | — |
| **Botón de texto / icono** | Sin fondo; icono de 20 px en zona de 32 px | `secondary.bg` | Igual | `secondary.bg.hover` | `text.disabled` | — |
| **Botón destructivo** | `danger.bg` + `danger.fg` | `danger.bg.hover` | Igual | Oscurece 8 % | `text.disabled` | Siempre con diálogo de confirmación |
| **Campo numérico con arrastre** | `surface`, borde `border.strong`, cifras tabulares | Cursor ↔ sobre la etiqueta | Borde `focus.ring` 2 px y selección del número | Arrastre horizontal ajusta el valor | `text.disabled`, sin cursor | Error: borde `error.border` + icono + mensaje bajo el campo («Introduce un valor entre 1 y 20000.») |
| **Slider enlazado a campo** | Pista `secondary.bg`, relleno `text`, tirador 16 px `surface` con borde | Tirador crece a 18 px | Anillo de foco en el tirador | Muestra el valor en una burbuja | Pista y tirador atenuados | — |
| **Selector (desplegable)** | Campo con chevrón | `secondary.bg` en la opción | Anillo | Abre lista (máx. 8 visibles, buscador si > 12) | Atenuado | Error igual que el campo |
| **Selector de color** | Muestra + valor HEX | Tooltip con nombre | Anillo | Abre popover | Atenuado | HEX inválido: error inline |
| **Interruptor / casilla / opción** | 20 px, marca con icono | `secondary.bg` | Anillo | Marca animada 120 ms | Atenuado | — |
| **Panel (acordeón)** | Cabecera 32 px con chevrón y título `ui-label` | `secondary.bg` | Anillo | Despliega 160 ms | — | — |
| **Popover** | `surface.raised`, sombra 2, radio `md`, 8 px de margen | — | Foco atrapado; `Esc` cierra | — | — | — |
| **Diálogo** | `surface.raised`, sombra 3, radio `lg`; velo `text` al 40 % | — | Foco atrapado; primer foco en la acción segura | — | — | Pie con «Cancelar» y acción; destructivo nombra la acción («Eliminar página») |
| **Toast** | `surface`, sombra 2, 4 s (error: hasta cerrar), abajo a la izquierda | Pausa el temporizador | `Alt+T` enfoca el último | — | — | Éxito `success.*` con icono de visto; error `error.*` con icono y «Reintentar» |
| **Tooltip** | `text` sobre `surface` invertido, 500 ms de retardo, 12 px | — | Aparece al enfocar con teclado | — | — | Incluye el atajo («Dividir · S») |
| **Lista con arrastre** (Capas, pistas, páginas) | Fila 32 px con asa ⋮⋮, visibilidad, bloqueo | `secondary.bg` | Anillo en la fila; `Alt+↑/↓` reordena | Línea de inserción coral de 2 px al arrastrar | Fila bloqueada atenuada | — |
| **Chip** | Radio `pill`, 24 px, `secondary.bg` | `secondary.bg.hover` | Anillo | `secondary.bg.pressed` | Atenuado | Seleccionado: borde `text` 1 px + icono de visto (no solo color) |
| **Pestaña del riel** | Icono 24 px + etiqueta, `topbar.fg.muted` | `topbar.hover` | Anillo | — | — | Activo: `topbar.fg` + indicador coral |
| **Miniatura (plantilla, medio)** | Radio `md`, borde `border` | Sombra 1 + acciones | Anillo | — | — | Etiqueta accesible con tipo, formato, estilo y color |
| **Barra de progreso** | Pista `secondary.bg`, relleno `accent` + texto con porcentaje | — | — | — | — | Error: relleno `error.fg` + mensaje |

### 8.2 Reglas de los componentes

- **DIS-20**: el **campo numérico con arrastre** admite escribir, arrastrar sobre su etiqueta, rueda con foco, `↑/↓` (paso 1), `Shift+↑/↓` (paso 10) y `Alt+↑/↓` (paso 0,1). Genera un `Command` con `merge_key` para que el gesto sea una sola entrada de historial (02, ARQ-33).
- **DIS-21**: el **slider enlazado** y su campo muestran siempre el mismo valor, sin retraso visible; doble clic restablece el valor por defecto.
- **DIS-22**: los **diálogos de confirmación** destructivos nombran lo que se elimina y ofrecen Deshacer inmediato siempre que sea posible, en vez de confirmar dos veces.
- **DIS-23**: los **toasts** no se apilan más de 3; no interrumpen la escritura ni roban el foco.

---

## 9. Interacción

### 9.1 Selección y manipulación directa

- **Seleccionar**: clic; `Shift+clic` suma; arrastrar en vacío traza un marco de selección; `Ctrl/Cmd+A` selecciona todo; doble clic entra en un grupo o en un texto; `Esc` sale. La selección responde en < 16 ms (cifra de la spec técnica).
- **Mover**: arrastre a 60 fps con 100 capas (cifra de la spec técnica); `Imán` atrae a guías, bordes de página, centros, márgenes y cuadrícula (umbral 6 px de pantalla, PROPUESTA) y muestra la distancia a los vecinos; `Alt` desactiva el Imán durante el arrastre.
- **Redimensionar y girar**: 8 asas y asa de giro; `Shift` conserva la proporción, `Alt` redimensiona desde el centro, `Shift` al girar ajusta a 15°. La caja de selección indica cuánto sobresale de la página (02, §3.2, `layout::overflow`).
- **Recortar**: doble clic sobre una imagen o tecla `C` entra en el modo de recorte con asas visibles.
- **Duplicar**: `Alt+arrastrar` o `Ctrl/Cmd+D`.
- **Todo es deshacer/rehacer**: botones ↶ y ↷ en la Barra superior con tooltip «Deshacer {acción}» (etiqueta `Command::label`, 02 §4.3); deshacer tarda < 16 ms (ARQ-32).

### 9.2 Atajos de teclado

`Ctrl` en Windows y Linux es `Cmd` en macOS. Los atajos se pueden reasignar en Ajustes → Atajos (con detección de conflictos).

| Grupo | Acción | Atajo |
|---|---|---|
| Archivo | Nuevo · Abrir · Guardar · Guardar como · Exportar | Ctrl+N · Ctrl+O · Ctrl+S · Ctrl+Shift+S · Ctrl+E |
| Archivo | Importar · Ajustes · Cerrar proyecto | Ctrl+Shift+I · Ctrl+, · Ctrl+W |
| Edición | Deshacer · Rehacer | Ctrl+Z · Ctrl+Shift+Z (Windows/Linux también Ctrl+Y) |
| Edición | Cortar · Copiar · Pegar · Pegar sin formato | Ctrl+X · Ctrl+C · Ctrl+V · Ctrl+Shift+V |
| Edición | Duplicar · Eliminar · Seleccionar todo · Buscar y reemplazar | Ctrl+D · Supr/Retroceso · Ctrl+A · Ctrl+F |
| Edición | Mover 1 px · 10 px · Girar 15° | Flechas · Shift+flechas · Shift+giro |
| Capas | Agrupar · Desagrupar | Ctrl+G · Ctrl+Shift+G |
| Capas | Traer adelante · Enviar atrás · Al frente · Al fondo | Ctrl+] · Ctrl+[ · Ctrl+Shift+] · Ctrl+Shift+[ |
| Capas | Bloquear · Ocultar · Alinear a la izquierda / centro / derecha | Ctrl+Shift+L · Ctrl+Shift+H · Ctrl+Alt+L / C / R |
| Ver | Acercar · Alejar · Ajustar a la ventana · 100 % | Ctrl+= · Ctrl+- · Ctrl+0 · Ctrl+1 |
| Ver | Reglas · Guías · Cuadrícula · Imán | Ctrl+Shift+R · Ctrl+; · Ctrl+' · Ctrl+Shift+' |
| Ver | Modo claro/oscuro · Panel lateral · Inspector | Ctrl+Shift+D · Ctrl+\ · Ctrl+Shift+\ |
| Herramientas | Seleccionar · Mano (mantener) · Zoom | V · Espacio · Z |
| Herramientas | Texto · Forma · Elipse · Línea · Nota adhesiva | T · R · O · L · N |
| Herramientas | Lápiz · Marcador · Borrador · Cuentagotas · Recortar | P · M · E · I · C |
| Herramientas | Pincel Quitar · Pincel Devolver · tamaño del pincel | B · Shift+B · [ y ] |
| Texto | Negrita · Cursiva · Subrayado · Alinear izq./centro/der. | Ctrl+B · Ctrl+I · Ctrl+U · Ctrl+Shift+L/E/R (en edición de texto) |
| Páginas | Página anterior · siguiente · Nueva página | Re Pág · Av Pág · Ctrl+Enter |
| Video | Reproducir/pausar · Fotograma ± · Segundo ± | Espacio · ←/→ · Shift+←/→ |
| Video | Dividir en el Cabezal · Eliminar y cerrar hueco · Mover borde | S · Shift+Supr · W |
| Video | Marcar entrada · Marcar salida · Ir al inicio · Ir al final | I · O · Inicio · Fin |
| Video | Nueva escena · Añadir Transición en la unión · Silenciar pista | Ctrl+Alt+N · Ctrl+T · Ctrl+Shift+M |
| Global | Paleta de comandos · Ayuda · Cerrar superposición | Ctrl+K · F1 · Esc |

*Notas*: en el Editor de video `I` y `O` marcan entrada y salida; en el Editor de diseño `I` es Cuentagotas. Los atajos dependen del editor activo y la Paleta de comandos muestra el atajo vigente de cada acción. Los conflictos con atajos del SO se resuelven a favor del SO (VERIFICAR en cada plataforma).

- **DIS-77** (atajos independientes del idioma): la tabla anterior es la misma en español y en inglés; un atajo nunca se deriva de la inicial del nombre traducido, no hay mnemónicos por idioma (letras subrayadas en menús) y cambiar de idioma no cambia ningún atajo ni las reasignaciones de la persona. Los nombres de teclas que se muestran sí se traducen («Supr»/«Delete», «Espacio»/«Space», «Re Pág»/«Page Up»). *Aceptación*: la prueba de DIS-24 se ejecuta en `es` y en `en` con el mismo resultado; el registro de comandos no contiene atajos por idioma.

- **DIS-24**: todos los atajos de la tabla funcionan en las 3 plataformas. *Aceptación*: prueba de teclado sintética (`egui_kittest`, VERIFICAR) por atajo y por editor; la tabla se genera desde el registro de comandos, no se mantiene a mano.

### 9.3 Paleta de comandos

- Se abre con `Ctrl/Cmd+K` desde cualquier pantalla, en **< 100 ms**. Es una búsqueda difusa sobre todas las acciones registradas en `CommandPalette`, con atajo, categoría y estado (deshabilitada con motivo).
- Primer resultado preseleccionado; `↑/↓` navega; `Enter` ejecuta; `Esc` cierra. Admite verbos del tono de marca («crear página», «cambiar tamaño», «exportar PNG»).
- Muestra «Usados recientemente» al abrirse. Las acciones nunca ejecutan nada destructivo sin confirmación.

### 9.4 Menú contextual

Clic derecho o tecla de menú sobre un objeto: Copiar, Pegar, Duplicar, Eliminar, Agrupar/Desagrupar, Orden de apilado (submenú), Alinear (submenú), Bloquear, Ocultar, Aislar sujeto (imágenes), Reemplazar imagen, Guardar como plantilla (selección), Añadir a Identidad (color o logo). Sobre el lienzo vacío: Pegar, Zoom, Reglas, Guías, Imán, Cuadrícula. En la Línea de tiempo: Dividir, Eliminar, Separar audio, Velocidad, Transición.

### 9.5 Arrastrar y soltar

| Origen | Destino | Resultado |
|---|---|---|
| Archivo del sistema (imagen, SVG, video, audio, `.gaf`) | Lienzo, Línea de tiempo o Biblioteca | Importa y coloca; un `.gaf` se abre en Inicio |
| Imagen | Marco | Recorta por la forma del Marco y conserva su forma al reemplazar |
| Color de la Identidad | Elemento | Aplica la muestra enlazada (cambia con la identidad) |
| Plantilla | Lienzo | Añade una página nueva con sus elementos editables |
| Fila de Capas / pista / página | Misma lista | Reordena con línea de inserción |
| Clip | Línea de tiempo | Mueve con Imán a cortes, cabezal y marcadores |

- **DIS-25**: todo arrastre cancela con `Esc` y restaura el estado previo; el resultado es una entrada de historial.

---

## 10. Accesibilidad

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| DIS-30 | **Teclado completo**: orden de foco lógico por región (Barra superior → Riel → Panel lateral → Lienzo → Inspector → Barra inferior); `F6`/`Shift+F6` cambian de región. | La prueba E2E recorre las 8 pantallas solo con teclado (ARQ-63). |
| DIS-31 | **Foco visible** en todo control: anillo de 2 px con halo y separación de 2 px. | Captura golden por componente con foco; ratio ≥ 3:1 (DIS-11). |
| DIS-32 | **AccessKit**: nombre, rol y estado en cada widget; el Lienzo expone un árbol de objetos (nombre, tipo, posición) que se recorre con `Tab` dentro del lienzo. | Prueba con inspector de accesibilidad: 0 widgets sin nombre. |
| DIS-33 | **El color nunca es la única señal**: errores con icono y texto, estados activos con forma (subrayado, relleno, marca), selección con borde + etiqueta. | Revisión por captura en escala de grises: todo estado sigue siendo distinguible. |
| DIS-34 | **Contraste AA** de texto y componentes (DIS-10, DIS-11). | Prueba de `gafira-theme`. |
| DIS-35 | **Tamaño de interfaz** 100–200 % sin pérdida de funciones; texto mínimo de 12 px a 100 %. | Capturas a 100, 150 y 200 %: 0 truncados sin tooltip. |
| DIS-36 | **Reducción de movimiento** (§11) y alternativa estática de cualquier animación de interfaz. | ARQ-64. |
| DIS-37 | **Orden de lectura accesible** al exportar PDF (spec 04) y texto alternativo editable por imagen. | Un PDF exportado pasa la validación de etiquetas (VERIFICAR herramienta). |
| DIS-78 | **Idioma del texto accesible**: el nodo raíz del árbol de AccessKit declara el idioma activo (`es` o `en`) y cambia con él; las opciones «Español» y «English» del selector de idioma declaran el suyo propio; los nombres accesibles salen del mismo catálogo que el texto visible (02 ARQ-97). El texto de la persona en el Lienzo no recibe etiqueta de idioma salvo que se añada una propiedad de idioma al documento (PROPUESTA, PD-13). | Prueba con inspector de accesibilidad en los dos idiomas: idioma del nodo raíz correcto, 0 nombres en el otro idioma y 0 claves visibles. Que egui transmita la propiedad de idioma a AccessKit es VERIFICAR (02 §8.3). |

---

## 11. Movimiento

- **Escala**: 120 ms (`fast`: hover, foco, casillas), 160 ms (`base`: paneles, popovers, toasts), 220 ms (`slow`: diálogos, cambio de pantalla). Curvas `standard` (0,2; 0; 0; 1), `enter` y `exit` (tokens `motion.easing`).
- **Qué se anima**: opacidad y desplazamiento ≤ 8 px. No se anima el contenido del usuario ni el lienzo (el zoom es inmediato con refinamiento progresivo, 02 §6.2).
- **Reducción de movimiento**: si `gafira-platform::prefers_reduced_motion()` es verdadero, o Ajustes → Accesibilidad lo fuerza, todas las duraciones pasan a 0 ms (`motion.reduced`) y los desplazamientos se sustituyen por un cambio inmediato de opacidad. Las animaciones del *contenido* del usuario no se alteran (02 §8.3).
- **Pantalla de arranque** (D-09): sus animaciones (aparición de la imagen, avance de la barra y fundido de salida de 160 ms) y su comportamiento con reducción de movimiento están en §7.10.6 (DIS-84).
- **DIS-40**: ninguna transición supera 220 ms. *Aceptación*: la prueba de `gafira-ui` mide la duración de cada transición registrada (≤ 220 ms; 0 con reducción de movimiento, ARQ-64).

---

## 12. Estados vacíos, errores y microcopy

Tono de Gafira: frases breves de acción en segunda persona, verbos como crear, diseñar, cambiar, mover, compartir y exportar; sin signos de exclamación ni superlativos. Un error dice qué pasó y qué hacer; el detalle técnico va al registro local (02 §7.3). En inglés (D-08) se aplica el mismo tono: frases breves de acción en segunda persona (*create, design, change, move, share, export*), contracciones naturales (*can’t*, *we’re*), sin exclamaciones ni superlativos y sin traducir palabra por palabra cuando el inglés tiene una forma más natural; el eslogan inglés es el del manual («Bring your ideas to life.»).

| Situación | Texto en español (aprobado) | Texto en inglés (PROPUESTA) |
|---|---|---|
| Eslogan | «Haz visibles tus ideas.» · «Un espacio creativo donde las ideas toman forma.» | «Bring your ideas to life.» · «A creative space where ideas take shape.» |
| Botón principal de Inicio | «Empezar a diseñar» | «Start designing» |
| Botón secundario de Inicio | «Explorar plantillas» | «Browse templates» |
| Bienvenida | «¿Qué quieres crear hoy?» | «What do you want to create today?» |
| Inicio sin proyectos | «Tu próxima idea comienza aquí» | «Your next idea starts here» |
| Guardado | «Cambios guardados.» · «Guardando…» | «Changes saved.» · «Saving…» |
| Exportación terminada | «Tu diseño está listo para compartir» + «Mostrar en carpeta» | «Your design is ready to share» + «Show in folder» |
| Error genérico | «No pudimos completar esta acción. Inténtalo de nuevo.» | «We couldn’t complete this action. Try again.» |
| Vacío de Capas | «Aún no hay capas. Añade una pieza o un texto.» | «No layers yet. Add a piece or some text.» |
| Vacío de Biblioteca | «Arrastra aquí tus imágenes, video o audio.» | «Drag your images, video or audio here.» |
| Vacío de búsqueda | «No encontramos resultados para “{término}”. Prueba con otra palabra.» | «No results for “{term}”. Try another word.» |
| Vacío de Línea de tiempo | «Suelta un clip aquí para empezar.» | «Drop a clip here to start.» |
| Vacío de Plantillas propias | «Guarda un diseño como plantilla para reutilizarlo.» | «Save a design as a template to reuse it.» |
| Archivo no compatible | «No podemos abrir este archivo. Formatos admitidos: PNG, JPEG, WebP, GIF, TIFF, BMP, QOI, SVG, MP4, MOV, WebM, MKV, WAV, FLAC, Ogg, MP3.» | «We can’t open this file. Supported formats: PNG, JPEG, WebP, GIF, TIFF, BMP, QOI, SVG, MP4, MOV, WebM, MKV, WAV, FLAC, Ogg, MP3.» |
| Códec no disponible en el sistema | «Este sistema no puede exportar H.264. Exporta en WebM o MP4 con AV1, compatibles en todos los equipos.» | «This system can’t export H.264. Export as WebM or MP4 with AV1, which work on every computer.» |
| Fuente ausente | «Falta la fuente “{nombre}”. Usamos otra y recordamos la original.» | «The font “{name}” is missing. We’re using another one and keeping the original in mind.» |
| Recurso ausente | «No encontramos “{archivo}”. Búscalo de nuevo.» + «Buscar archivo» | «We can’t find “{file}”. Look for it again.» + «Find file» |
| Archivo de versión futura | «Este proyecto es de una versión más nueva. Puedes verlo, pero no guardarlo aquí.» | «This project is from a newer version. You can view it, but not save it here.» |
| Proyecto abierto en otra instancia | «Este proyecto ya está abierto. Ábrelo solo para ver o haz una copia.» | «This project is already open. Open it view-only or make a copy.» |
| GPU no disponible | «Gafira usa el procesador para dibujar. Todo sigue funcionando.» | «Gafira is using the processor to draw. Everything still works.» |
| Memoria baja | «Liberamos memoria para seguir editando con fluidez.» | «We freed up memory so you can keep editing smoothly.» |
| Fuera de la zona de seguridad | «Parte del contenido queda fuera de la zona de seguridad.» | «Part of the content is outside the safe zone.» |
| Aviso de eliminación | «¿Eliminar la página 3? Puedes deshacerlo.» | «Delete page 3? You can undo it.» |
| Recuperación | «Encontramos trabajo sin guardar.» | «We found unsaved work.» |
| Recuperación (detalle) | «“{proyecto}” · última copia {hace}» | «“{project}” · last copy {ago}» |
| Privacidad (Ajustes) | «Tus diseños y archivos no salen de tu equipo. La única conexión de Gafira es la de tu licencia.» | «Your designs and files never leave your computer. Gafira only connects to check your license.» |
| Ayuda del selector de idioma (§16.1) | «Con Sistema, Gafira usa español si tu equipo está en cualquier variante de español, e inglés en los demás casos. El cambio se aplica al momento.» | «With System, Gafira uses Spanish if your computer is set to any variant of Spanish, and English otherwise. The change applies right away.» |
| Idioma cambiado (lector de pantalla) | «Idioma: Español.» | «Language: English.» |
| Pantalla de arranque: eslogan (§7.10) | «Haz visibles tus ideas.» | «Bring your ideas to life.» (texto del manual, no PROPUESTA) |
| Pantalla de arranque: barra (nombre accesible) y anuncio al aparecer | «Cargando Gafira» · «Cargando Gafira.» | «Loading Gafira» · «Loading Gafira.» |
| Pantalla de arranque: versión | «Versión {versión}» | «Version {version}» |
| Pantalla de arranque: carga lenta | «Esto está tardando más de lo normal. Seguimos cargando.» | «This is taking longer than usual. We’re still loading.» |
| Pantalla de arranque: error | «No pudimos abrir Gafira. Inténtalo de nuevo.» + «Reintentar» · «Cerrar» | «We couldn’t open Gafira. Try again.» + «Try again» · «Close» |
| Pantalla de arranque: logo (nombre accesible) | «Gafira» | «Gafira» |

**Mensajes complementarios del manual** (manual §6.3; el texto inglés es el del manual y no es PROPUESTA; la ubicación sí lo es):

| Contexto | Español | Inglés | Dónde se usa (PROPUESTA) |
|---|---|---|---|
| Producto | «Diseña sin complicarte.» | «Design without the hassle.» | Acerca de, junto al logo apilado |
| Inspiración | «Tus ideas toman forma.» | «Your ideas take shape.» | Pantalla de carga del proyecto |
| Acción | «Empieza a crear.» | «Start creating.» | Inicio sin proyectos, bajo «Tu próxima idea comienza aquí» |
| Libertad | «Diseña a tu manera.» | «Design your way.» | Cabecera de Identidad visual vacía |
| Plantillas | «Empieza con una idea.» | «Start with an idea.» | Cabecera de la pestaña Plantillas |

- **DIS-41**: todo mensaje de error visible tiene un título breve, una acción y no muestra códigos ni rutas internas, en los dos idiomas. *Aceptación*: revisión de la tabla `message_key` → texto en `assets/i18n/es.toml` y `assets/i18n/en.toml`; ninguna cadena de los dos catálogos contiene «Exception», «panic», «0x» ni rutas absolutas.
- **DIS-42**: el microcopy del manual aparece sin cambios en los dos idiomas (manual §6.1–§6.3); cualquier texto nuevo se añade a la tabla anterior, con su versión en español y en inglés, en el mismo cambio.
- **DIS-79** (microcopy inglés completo): cada fila de las tablas de esta sección tiene texto en los dos idiomas, con los mismos `{parámetros}` y el tono de marca. Los textos ingleses que no salen del manual son PROPUESTA hasta la aprobación del titular (Q-25). *Aceptación*: `xtask check-i18n` (02 ARQ-98) no encuentra claves de microcopy sin `en`; una revisión de estilo confirma 0 signos de exclamación y 0 palabras vetadas («revolutionary», «the best», «unlimited», y sus equivalentes en español).

---

## 13. Iconografía

- **Estilo**: trazo simple de grosor uniforme sobre rejilla de 24 px (trazo de 1,75 px a 24 px; 1,5 px a 16 px y 20 px, PROPUESTA), terminaciones y uniones redondeadas, esquinas moderadas, relleno solo para estados activos. Solo `text`/`topbar.fg` (los iconos no llevan color propio salvo estados semánticos). Si el significado es ambiguo, el icono lleva etiqueta o tooltip.
- **Origen**: propio (dibujado en la rejilla) o, si se parte de una colección, **Lucide (ISC)** o **Feather (MIT)** con su aviso en `THIRD_PARTY_NOTICES.md` (VERIFICAR la licencia vigente de cada versión antes de adoptarla). Cada icono se registra en `assets/LICENSES.toml` (ARQ-75). Los SVG se convierten en geometría de trazo en tiempo de compilación; no se usan fuentes de iconos.
- **Lista de iconos necesarios** (≈ 110):

| Grupo | Iconos |
|---|---|
| Navegación y estructura | inicio, menú, pestañas del riel (plantillas, piezas, texto, identidad, biblioteca, dibujo, proyectos, medios, sonidos, movimiento, subtítulos, extensiones), ajustes, ayuda, buscar, paleta de comandos, cerrar, atrás, más, plegar, desplegar |
| Archivo y estado | nuevo, abrir, guardar, guardar copia, importar, exportar, versiones, guardado, guardando, error, éxito, aviso, información, candado, ojo, ojo tachado, papelera |
| Edición | deshacer, rehacer, copiar, pegar, duplicar, eliminar, agrupar, desagrupar, reordenar (asa), buscar y reemplazar |
| Herramientas | seleccionar, mano, zoom, texto, forma, línea, lápiz, marcador, resaltador, borrador, nota adhesiva, recortar, cuentagotas, pincel Quitar, pincel Devolver |
| Posición y capas | alinear (6), distribuir (2), traer adelante, enviar atrás, al frente, al fondo, proporción bloqueada/libre, girar, voltear (2) |
| Lienzo | reglas, guías, cuadrícula, Imán, márgenes, sangrado, zona de seguridad, páginas, página nueva, orientación |
| Imagen | zona de edición, ajuste automático, balance de blancos, luz, color, textura, filtros, sombra, borde, comparar, restablecer, aislar sujeto |
| Texto | negrita, cursiva, subrayado, tachado, mayúsculas, alinear (4), lista, columnas, ajustar al cuadro, fijar tipografía, curvar, interletrado, interlineado |
| Video y audio | reproducir, pausar, fotograma ±, inicio, fin, dividir, mover borde, velocidad, invertir, congelar fotograma, transición, nueva escena, volumen, silenciar, separar audio, nivelar, fotograma clave, curva, máscara, clave de color, seguimiento, estabilizar, medidores, mezclador, copia ligera, cola |
| Exportación | formato imagen, PDF, video, audio, GIF, SVG, carpeta, contraseña |

- **DIS-50**: todos los iconos usan la misma rejilla y trazo. *Aceptación*: `xtask check-icons` verifica rejilla de 24 px, trazo y márgenes; el 100 % de los iconos de la lista existe y tiene licencia registrada.

---

## 14. Criterios de aceptación visuales y de rendimiento de UI

| ID | Criterio | Umbral | Cómo se mide |
|---|---|---|---|
| DIS-60 | Fluidez de la interfaz en uso normal | 60 fps; p95 de frame ≤ 16,7 ms | `bench_ui_idle` y `bench_drag_100_layers`, 300 frames (02 §6.7) |
| DIS-61 | Respuesta a la selección | < 16 ms (p99) | `bench_select_hit` con 1 000 nodos |
| DIS-62 | Interacciones locales (abrir menú, cambiar pestaña, abrir Paleta de comandos, aplicar color) | < 100 ms (p99) | `bench_interactions` |
| DIS-63 | Cambio de color | < 50 ms | `bench_recolor` |
| DIS-64 | Cambio de modo claro/oscuro sin reiniciar | < 100 ms | Medición del primer frame tras el cambio |
| — | Pantalla de arranque: primer frame y duración visible | Primer frame < 300 ms (PROPUESTA); visible ≥ 1,2 s (D-09); carga lenta a los 5 s (PROPUESTA) | DIS-82 (§7.10.11), `gafira --bench-startup` (CAL-38, CAL-39) |
| DIS-75 | Cambio de idioma (Español ↔ English) sin reiniciar, con el proyecto abierto, sin perder selección ni historial | ≤ 100 ms (p99) hasta el primer frame completo en el idioma nuevo | `bench_locale_switch` (02 ARQ-101) |
| DIS-65 | Ningún trabajo del hilo de UI > 16 ms fuera de diálogos del SO | 0 spans | `tracing` en la suite E2E (ARQ-41) |
| DIS-66 | Cancelación de arrastre, exportación o segmentación | ≤ 100 ms | ARQ-43 |
| DIS-67 | Capturas golden por pantalla y componente en ambos modos | Diferencia ≤ 1/255 en el 99,9 % de los píxeles entre ejecuciones de la misma plataforma | `gafira-testkit::golden` |
| DIS-68 | Tokens y contraste | 0 fallos | DIS-10 |
| DIS-69 | Colores literales en código | 0 coincidencias | DIS-04 |
| DIS-70 | Uso del logo | Mínimos y áreas de protección respetados; sin variantes alteradas; en la Pantalla de arranque, nunca sobre la imagen | DIS-13, DIS-14, DIS-80 |
---

## 15. Propuestas y preguntas abiertas de esta especificación

| ID | Elemento | Estado |
|---|---|---|
| PD-01 | Colores semánticos (error, éxito, aviso, enlace), todo el modo oscuro y `surface.raised` en modo claro | PROPUESTA; requieren aprobación de marca |
| PD-02 | Estados hover, pressed y foco de los componentes | PROPUESTA |
| PD-03 | Pastilla clara para el símbolo sobre la barra superior grafito | PROPUESTA (alternativa: variante `symbol-dark` del manual) |
| PD-04 | Icono de aplicación con baldosa `#FAFAF7` y símbolo al 62 % | PROPUESTA |
| PD-05 | Ventana mínima 1280×720 y breakpoints de paneles | PROPUESTA |
| PD-06 | Nombres del glosario (§2) | **Aprobados** (00-decisiones, D-04); queda la búsqueda de «Gafira» y «Gefira» en USPTO, TMview y WIPO, a cargo del titular (spec 05, Parte C) |
| PD-07 | Colección de iconos (propia, Lucide ISC o Feather MIT) | VERIFICAR licencia y decidir |
| PD-08 | Atajos fuera de los universales (Ctrl+Alt+N, Ctrl+T, etc.) | VERIFICAR contra conflictos del SO |
| PD-09 | Variantes monocromas del logo (`mono-navy`, `mono-white`, `symbol-dark`) | Por crear; no existen en el repositorio |
| PD-10 | Pantallas y estados de la licencia (activación, prueba, solo lectura) | Diferido: se diseñan en la spec 06 (pendiente, se redacta al final); propuesta previa de 02: Q-19 y Q-20 |
| PD-11 | Nombres ingleses del glosario (§2.8) | PROPUESTA (D-08); requieren aprobación del titular y revisión de marca (Q-25, DIS-72) |
| PD-12 | Microcopy inglés que no sale del manual (§12) y ubicación de los mensajes complementarios | PROPUESTA (Q-25) |
| PD-13 | Selector de idioma con opciones de radio, sin pregunta de idioma en el primer arranque, y propiedad de idioma opcional en el texto de la persona para AccessKit | PROPUESTA |
| PD-14 | Factores de expansión de la seudo-localización (+100 % a +30 %) | PROPUESTA (02 §14.6) |
| PD-15 | Nombre «Pantalla de arranque» / «Startup screen» (§2.1, §2.8.1) | PROPUESTA (D-09); requiere aprobación del titular |
| PD-16 | Pantalla de arranque: ventana propia de 800×450, panel de 128 px, logo de 200 px, pista grafito para la barra coral, panel siempre en modo claro, imagen decorativa sin texto alternativo | PROPUESTA (D-09) |
| PD-17 | Pantalla de arranque: primer frame < 300 ms, carga lenta a los 5 s, error tras 20 s sin progreso, límites de la imagen (1600×900 a 2880×1620 px; 1 MB recomendado, 2 MB máximo) | PROPUESTA (D-09); la duración mínima de 1,2 s es decisión del titular |

---

## 16. Idioma (D-08)

Gafira se publica en **español e inglés**. El idioma por defecto sale del sistema: cualquier variante de español (`es`, `es-MX`, `es-CO`, `es-ES`…) usa español; inglés (`en`, `en-US`, `en-GB`…) usa inglés; **cualquier otro idioma usa inglés**. La persona lo cambia en Ajustes → General. La arquitectura está en 02 §14 (`gafira-i18n`); aquí se fija lo que se ve y cómo se comporta.

### 16.1 Selector en Ajustes → General

```text
┌─ Ajustes ──────────────────────────────────────────────────────────────────────────┐
│ General          │ Idioma                                                          │
│ Apariencia       │ (•) Sistema · ahora: Español (es-MX)                            │
│ Rendimiento      │ ( ) Español                                                     │
│ Guardado         │ ( ) English                                                     │
│ Atajos           │ Con Sistema, Gafira usa español si tu equipo está en cualquier  │
│ Accesibilidad    │ variante de español, e inglés en los demás casos. El cambio se  │
│ Privacidad       │ aplica al momento.                                              │
│ Licencia         │                                                                 │
│ Funciones locales│ Carpeta de proyectos   [ …/Documentos/Gafira ]   (Examinar)     │
│ Acerca de        │                                                                 │
└──────────────────┴─────────────────────────────────────────────────────────────────┘
```

| Opción | Etiqueta en interfaz española | Etiqueta en interfaz inglesa | Valor guardado | Comportamiento |
|---|---|---|---|---|
| Sistema (**por defecto**) | «Sistema · ahora: {idioma} ({etiqueta})» | «System · now: {language} ({tag})» | `system` | Aplica la regla de D-08 a la etiqueta del SO (02 §14.4). Si el SO no da idioma, muestra «Sistema · ahora: English» / «System · now: English». |
| Español | «Español» | «Español» | `es` | Interfaz en español, sea cual sea el idioma del SO. |
| English | «English» | «English» | `en` | Interfaz en inglés, sea cual sea el idioma del SO. |

- Los nombres de los idiomas se escriben **siempre en su propio idioma** («Español», «English»), para que cualquiera encuentre el suyo aunque la interfaz esté en el otro. «Sistema» sí se traduce («System»).
- Control: grupo de opciones (componente «Interruptor / casilla / opción» de §8.1) con foco por teclado, `↑/↓` para elegir; el cambio se aplica al seleccionar, sin botón «Aplicar» y sin reiniciar (DIS-75). El texto de ayuda va bajo el grupo con `text.secondary` y `body-sm`.
- Tras el cambio, el foco se queda en la opción elegida (ya traducida) y el lector de pantalla anuncia «Idioma: Español.» / «Language: English.» (§12).
- La Paleta de comandos ofrece la acción «Cambiar idioma» / «Change language», que abre esta sección.

- **DIS-73** (selector de idioma): Ajustes → General tiene un único selector de idioma con las tres opciones de la tabla, en ese orden, con `Sistema` por defecto y el texto de ayuda de §12. *Aceptación*: prueba E2E: (a) con `FakeSystemLocaleSource` en `es-CO`, `en-GB`, `fr-FR` y sin dato, la opción Sistema muestra Español, English, English y English; (b) elegir Español o English persiste `es` o `en` y sobrevive a reiniciar; (c) las etiquetas «Español» y «English» son idénticas en los dos idiomas de interfaz.

### 16.2 Primer arranque

1. Sin ajustes guardados, la preferencia es `Sistema` y se resuelve con la regla de D-08 antes de pintar el primer frame (la resolución no añade tiempo medible al arranque de 02 §6.6).
2. **No se pregunta el idioma**: Inicio aparece directamente en el idioma resuelto (principio P2: un camino claro, sin pasos previos).
3. Si el idioma del SO no es español ni inglés, Inicio aparece en inglés y no se muestra ningún aviso; Ajustes → General indica «System · now: English».
4. Los proyectos no guardan el idioma de la interfaz: el mismo `.gaf` se abre igual con cualquier idioma, y los nombres ya creados («Página 1») no se traducen (02 §14.2).

- **DIS-74** (primer arranque): el primer arranque nunca muestra una pregunta de idioma ni texto en un idioma distinto del resuelto. *Aceptación*: prueba E2E con perfil de ajustes vacío y SO en `es-ES`, `en-US` y `de-DE`: la primera captura de Inicio está entera en español, inglés e inglés respectivamente y no hay diálogos.

### 16.3 Qué cambia con el idioma y qué no

| Cambia | No cambia |
|---|---|
| Todo el texto de la interfaz, menús, tooltips, mensajes, nombres accesibles y la Paleta de comandos | El contenido de la persona: textos, nombres de proyectos, capas, páginas, archivos y fuentes |
| Formato de números, fechas, horas, tiempos relativos y unidades («1,2 MB» / «1.2 MB», «hace 2 h» / «2 h ago», «100 %» / «100%»; 02 §14.5) | Los atajos de teclado (DIS-77) y los códigos de tiempo (`00:12:04`) |
| Los nombres de teclas mostrados («Supr» / «Delete») | Los nombres de formatos (PNG, MP4, AV1) y la marca «Gafira» |
| El filtro por defecto de Plantillas (idioma de la interfaz más plantillas neutras; 01 §7.3) | El archivo `.gaf` y sus claves |

Los campos numéricos escriben y leen el separador decimal del idioma activo (coma en español, punto en inglés) y aceptan además el punto (PROPUESTA, 02 §14.5).

### 16.4 Diseño para la expansión de texto

El español suele ocupar más que el inglés (a menudo un 15–30 % más en etiquetas cortas), y los futuros idiomas pueden ocupar más aún. Reglas:

- Ningún control tiene un ancho fijo calculado para el texto de un idioma: botones, pestañas del Inspector y opciones miden según su etiqueta, con los mínimos de §3 (32 px de alto y 24×24 px de área de clic).
- **Riel de pestañas** (64 px): la etiqueta de 12 px puede ocupar **dos líneas**; si aun así no cabe, se reduce el interletrado hasta el mínimo del token y, como último recurso, el nombre completo queda en el tooltip y en el nombre accesible. Nunca se abrevia a mano en el catálogo.
- **Barra contextual** e **Inspector**: si las acciones no caben, las sobrantes pasan a «⋯ Más» / «⋯ More» en orden inverso de prioridad; no se reduce el tamaño de letra.
- Diálogos y mensajes: el texto se ajusta en varias líneas; los botones del pie se apilan en vertical si no caben en una fila.
- Ninguna frase se construye concatenando fragmentos traducidos: cada frase completa es una clave con parámetros (02 §14.2), porque el orden de las palabras cambia entre idiomas.
- Las pruebas usan **seudo-localización** (02 §14.6): texto inglés con acentos, entre `⟦ ⟧` y alargado entre +30 % y +100 % según su longitud (PD-14).

- **DIS-76** (expansión): ninguna pantalla trunca texto sin tooltip ni solapa controles en español, en inglés o en seudo-localización, a 100, 150 y 200 % de tamaño de interfaz. *Aceptación*: la suite de capturas de las 8 pantallas (DIS-18) se ejecuta en `es`, `en` y seudo, con los tres tamaños de DIS-35: 0 truncados sin tooltip, 0 solapes y 0 textos sin `⟦ ⟧` en seudo (02 ARQ-103).

### 16.5 Accesibilidad y atajos

- **Idioma del texto accesible**: DIS-78 (§10).
- **Atajos**: DIS-77 (§9.2). La tabla de §9.2 es única para los dos idiomas; solo se traducen los nombres de las acciones y de las teclas.
- **Contraste y tamaño**: los textos ingleses y españoles usan los mismos tokens; DIS-10, DIS-11 y DIS-35 se verifican en los dos idiomas.

### 16.6 Requisitos de esta sección

Índice (cada requisito se define una sola vez, en la sección indicada):

| Sección | Requisitos | Resumen |
|---|---|---|
| §2.7 | DIS-01 (ampliado), DIS-71, DIS-72 | Nombres del glosario en los dos idiomas; glosario inglés 1:1; nombres ingleses propios o genéricos |
| §9.2 | DIS-77 | Atajos independientes del idioma |
| §10 | DIS-78 | Idioma del texto accesible |
| §12 | DIS-41, DIS-42 (ampliados), DIS-79 | Errores y microcopy en los dos idiomas |
| §14 | DIS-75 | Cambio de idioma ≤ 100 ms sin reiniciar |
| §16.1 | DIS-73 | Selector de idioma |
| §16.2 | DIS-74 | Primer arranque sin pregunta |
| §16.4 | DIS-76 | Expansión de texto |
| §7.10.11 | DIS-86 | Eslogan y textos de la Pantalla de arranque en el idioma resuelto desde el primer frame (D-09) |
