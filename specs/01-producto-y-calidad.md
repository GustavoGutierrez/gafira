# 01 — Producto, alcance y criterios de calidad de Gafira

| Campo | Valor |
|---|---|
| Documento | Especificación 01 de 05 |
| Estado | Borrador para revisión |
| Alcance | Visión, público, posicionamiento, alcance por fase, requisitos funcionales (RF) y no funcionales (RNF), criterios de calidad y velocidad (CAL), métricas, restricciones, modelo de funciones, riesgos, preguntas abiertas y resolución de contradicciones |
| Fuentes | `00-decisiones.md` (decisiones D-01 a D-09 del titular, prevalecen sobre el PRD y sobre cualquier texto contrario), `docs/research/PRD.md`, `docs/research/spec_software.md`, `docs/manuals/marca/manual-gafira.md`, `02-arquitectura-software.md` |
| Specs relacionadas | 02 (arquitectura: `ARQ-xx`, `ADR-xx`), 03 (diseño: `DIS-xx` y glosario de interfaz), 04 (diseño, imagen y exportación), 05 (video y distribución) |
| Convenciones | `RF-xx` requisito funcional; `RNF-xx` requisito no funcional; `CAL-xx` criterio de calidad medible; **FIJADA** = cifra fijada por el titular (D-03); **PROPUESTA** = cifra o decisión no presente en las fuentes; **VERIFICAR** = dato por confirmar antes de vender. Los nombres de interfaz son los del glosario de la spec 03 (§2) |

---

## 1. Visión

Gafira es un editor de escritorio local, rápido y estable para crear piezas gráficas y videos. Reúne tres capas en una sola aplicación y un solo archivo de proyecto:

1. **Diseño gráfico rápido**: lienzo, Plantillas, Piezas, texto, imagen, Identidad visual y Exportar.
2. **Video básico**: Línea de tiempo, cortes, Transiciones, Subtítulos, audio y formatos verticales y horizontales.
3. **Video avanzado mínimo**: multicapa, Fotogramas clave, Máscaras, Corrección de color, audio avanzado, Copias ligeras y exportación con control técnico.

**Eslogan**: «Haz visibles tus ideas.» (en inglés, «Bring your ideas to life.»). **Idiomas**: español e inglés, según el idioma del sistema (D-08, §7.11). **Subtítulo**: «Un espacio creativo donde las ideas toman forma.» **Principio de producto**: «La herramienta no debe ser más difícil que la idea que el usuario quiere comunicar.» **Principio rector de marca**: el contenido del usuario es el protagonista.

Gafira funciona sin conexión: los proyectos, cachés, vistas previas y renders se ejecutan en el equipo de la persona. No existen servidores, versión web ni nube de Gafira (RNF-04, ARQ-04). La **única conexión de red permitida** es la de la licencia (activar, validar o desactivar; D-01). Licencia y activación: ver spec 06 (pendiente, se redacta al final).

## 2. Público y posicionamiento

### 2.1 Público

Creadores de contenido que producen piezas gráficas y videos con frecuencia y necesitan control sin aprender un programa profesional completo.

| Perfil | Necesidad principal | Qué espera de Gafira |
|---|---|---|
| Creador o creadora independiente (redes, video corto) | Publicar a menudo con identidad reconocible | Plantillas, Cambiar tamaño a varios Destinos, exportación vertical en minutos |
| Diseñador/a freelance o de estudio pequeño | Control fino y entregables fiables | Capas, Guías, Imán, PDF de impresión, Identidad visual por cliente |
| Comunicador/a de pyme o de organización | Piezas coherentes sin depender de un equipo | Identidad visual reutilizable, Plantillas propias, trabajo sin conexión |
| Editor/a de video aficionado/a o semiprofesional | Cortes limpios y acabado de color y sonido | Línea de tiempo, Fotogramas clave, Corrección de color, Mezclador |
| Docente o divulgador/a | Materiales claros con buen texto y subtítulos | Texto con Columnas y Ajustar al cuadro, Subtítulos, exportación PDF accesible |

### 2.2 Posicionamiento

**«Diseño simple con control creativo.»** Arquetipos de marca: Creador y Guía. Valores: simplicidad útil, libertad creativa, autonomía, claridad, inclusión y confianza.

| Frente a… | Gafira ofrece |
|---|---|
| Editores en línea basados en plantillas | Trabajo local sin conexión, proyecto portable y control profesional (capas, máscaras, curvas, color) |
| Suites profesionales de escritorio | Una interfaz guiada (Elegir → Personalizar → Compartir) y un solo programa para diseño y video |
| Herramientas gratuitas dispersas | Un único formato `.gafira`, una única Identidad visual y una sola cola de exportación |

## 3. Propuesta de valor

| Problema habitual del creador | Respuesta de Gafira | Requisitos |
|---|---|---|
| Precio y condiciones confusos | Una sola licencia, sin prueba gratuita (D-12): todo el producto en un único plan, sin módulos ni funciones sueltas por separado (D-01, §11). Esto **sustituye** el principio del PRD «sin funciones esenciales retenidas tras un plan de pago» | §11, RF-63 |
| Retrasos, cierres y pérdida de trabajo | Motor local, autoguardado transaccional, recuperación tras fallo y Copias ligeras | RF-03, RF-53, CAL-18 |
| Exportaciones fallidas o archivos corruptos | Validación previa, escritura atómica, reintentos y alternativa de códec | RF-33, CAL-17 |
| Poco control profesional | Capas, Máscaras, Fotogramas clave, Corrección de color, audio avanzado y exportación técnica | RF-45..RF-54 |
| Resultados homogéneos | Plantillas como punto de partida: todo es editable; Estilos e Identidad visual propios | RF-15, RF-28 |
| Inteligencia artificial intrusiva | Aislar sujeto sin modelo (técnica clásica); IA opcional, local y desactivable, fuera del MVP | RF-22, RF-23 |
| Interfaz confusa | Flujo guiado, Paleta de comandos (Ctrl/Cmd+K), tooltips con atajo y teclado completo | RF-13, RNF-05 |
| Portabilidad deficiente | `.gafira` abierto y documentado, formatos estándar de entrada y salida | RF-02, RF-35 |
| Soporte percibido como débil | Diagnóstico integrado, registros locales exportables y reporte de errores con datos técnicos que la persona decide enviar | RNF-04, RNF-06 |

## 4. Principios de producto

| # | Principio | Implicación |
|---|---|---|
| PP-1 | **Local primero.** Todo funciona sin red. | Ninguna función de edición, importación o exportación abre conexiones; la única conexión es la de la licencia (ARQ-04, ARQ-91). |
| PP-2 | **Simple al principio, profundo al seleccionar.** | Divulgación progresiva en lugar de «modo simple» y «modo experto» separados: Barra contextual → Inspector → Paleta de comandos. |
| PP-3 | **Nada destructivo.** | Edición no destructiva y Deshacer en todo (RNF-03, RNF-05). |
| PP-4 | **El usuario manda sobre sus datos.** | Formato abierto, sin cuentas, sin telemetría automática, sin IA activa por defecto; solo el identificador de dispositivo de la licencia sale del equipo y se declara en `PRIVACY.md`. |
| PP-5 | **Calidad medible.** | Cada cifra de la §7 tiene benchmark y umbral en CI (ARQ-40). |
| PP-6 | **Vendible sin sorpresas.** | Solo dependencias y assets permisivos; patentes de códecs aisladas por feature (ARQ-70..76, ADR-07). |
| PP-7 | **Extensible por registro.** | Formatos, efectos, herramientas y códecs se añaden implementando un trait (ARQ-20). |

## 5. Alcance por fase

Las fases siguen el PRD (§6) y la hoja de ruta técnica de 02 (§11.2). Cuando difieren, se explica en la §13.

| Área | MVP (diseño + video básico) | Fase 2 (control profesional) | Fase 3 (diferenciación) |
|---|---|---|---|
| Arranque | Pantalla de arranque (D-09, RF-67): ilustración incrustada y reemplazable, logo y eslogan sobre panel claro, barra de carga real; visible al menos 1,2 s | — | — |
| Proyecto | `.gafira`, autoguardado, recuperación, Versiones, Historial | Relocalización de medios (RF-55) | — |
| Lienzo | Tamaños, unidades, páginas, Reglas, Guías, Generador de guías, Imán, Cuadrícula, Zona de seguridad, Sangrado, Zoom 5–3200 % | Distribuir con reglas avanzadas | — |
| Plantillas y Piezas | Plantillas locales, Piezas (formas, líneas, Marcos, Iconos, Códigos QR, Tablas, Gráficos, Ilustraciones), Dibujo, Biblioteca | Plantillas de video y componentes reutilizables | Biblioteca avanzada con búsqueda semántica local (opcional) |
| Imagen | Ajustes de imagen, Filtros, Aislar sujeto (clásico), importación PNG/JPEG/WebP/GIF/TIFF/BMP/QOI/SVG | HEIC (solo lectura, vía SO; depende de Q-17), importar PDF, LUT en imagen | Segmentadores ONNX opcionales |
| Texto | Texto completo, Efectos de texto, Columnas, Ajustar al cuadro, Fijar tipografía, Curvar | Texto sobre forma vectorial y vertical | Reescritura de texto con modelo local (opcional, desactivable) |
| Identidad | Identidad visual, Estilos, Cambiar tamaño | — | Perfiles de identidad múltiples avanzados |
| Exportar imagen | PNG, JPEG, WebP, GIF animado, SVG, PDF RGB | PDF CMYK, PPTX (una imagen por diapositiva, sin objetos editables; `gafira-io-pptx`, D-07) | Lotes con CSV |
| Video básico | Línea de tiempo, cortes, velocidad, Invertir, Congelar fotograma, Transiciones, escenas, texto animado, Subtítulos manuales, Sonidos, volumen y fundidos, vista previa con scrubbing, Cola de exportación básica | Voz en off (PROPUESTA), marcadores de compás (PROPUESTA) | — |
| Video avanzado | Modelo y motor listos (02, §3.4) | Multicapa y Agrupar en secuencia, Fotogramas clave con Editor de curvas, Máscaras, Clave de color, Seguimiento, Estabilizar, Corrección de color y Medidores, Mezclador completo (con Puerta de ruido `NoiseGate` y *time-stretch* `TimeStretch`), SRT/VTT, Copias ligeras (gestión), control de códec | Optimización GPU. El render distribuido en la red local queda **en suspenso** (abriría una conexión distinta de la de licencia, D-01; Q-18) |
| Exportar video | **WebM y MP4 con AV1 + Opus por defecto**; MP4 H.264/AAC solo vía API del sistema (`gafira-codec-os`); secuencia PNG; **extracción de fotograma (PNG) y de audio (WAV)**; sin salida HEVC ni MP3 (D-02, D-07) | WebM VP9, extracción de fotograma en JPEG y de audio en FLAC u Opus, control de bitrate, fps y rango de color | — |
| Extensiones | — | — | Plugins (`PluginHost`), automatizaciones locales |
| Distribución | MSI (Windows), `.deb`, `.rpm` y `.tar.gz` (Linux x86_64), DMG firmado y notarizado (macOS aarch64); sin actualizador (D-05, D-06) | — | — |
| Idioma | **Español e inglés** con catálogos completos, idioma por defecto según el sistema (D-08), selector en Ajustes → General, cambio sin reiniciar, glosario y microcopy en los dos idiomas, plantillas empaquetadas con idioma declarado (`gafira-i18n`) | — | Otros idiomas, si el titular los decide (Q-23) |

### 5.1 Fase final: licencia y distribución comercial

La licencia queda **fuera del alcance de las primeras fases de desarrollo** y no bloquea el MVP técnico de diseño y video. Se entrega en una **fase final, «Licencia y distribución comercial»**, que reúne: el crate `gafira-license` y la integración con el `license-hub` (servicio externo ya existente, reutilizado tal cual), el modo sin licencia, los estados de licencia, la interfaz de activación, `PRIVACY.md` con el identificador de dispositivo, el acuerdo de licencia basado en el de Tracewright y la lista previa a la venta (05, Parte C). Las decisiones de D-01 y D-12 ya están fijadas (todo de pago, modo sin licencia, una sola licencia, única conexión de red = licencia); su diseño detallado se redacta al final. **Licencia y activación: ver spec 06 (pendiente, se redacta al final).**

## 6. Fuera de alcance

Excluido explícitamente (PRD §8 y decisiones del proyecto):

- Colaboración en tiempo real, comentarios, aprobaciones y calendario editorial.
- Publicación directa o integraciones con redes sociales.
- Generación de contenido por IA (textos, diseños, imágenes o video).
- Versión web, aplicación móvil, WASM, macOS x86_64, Windows arm64 (ARQ-03), Linux aarch64, AppImage y Flatpak (D-05).
- Comprobación de versiones nuevas, actualizador y firma de manifiestos de actualización (D-06): el usuario descarga cada versión desde la web.
- Encoders propios de H.264, HEVC o AAC, salida HEVC y salida MP3 (D-02, D-07).
- Servidores de Gafira, sincronización, cuentas en la nube, bibliotecas de plantillas remotas y marketplace público. (El `license-hub` es un servicio externo ya existente, no un servidor de la app.)
- Telemetría que envíe datos o contenido del usuario.
- Binarios externos en el instalador, incluido FFmpeg (ARQ-72).
- Exportación de AVI, ProRes, EPS, DXF, EMF/WMF (no previstos en 02). La importación de AVI es de Fase 2 y solo con códecs libres (D-07).

---

## 7. Requisitos funcionales

Cada RF traza al requisito del PRD (RF-nn del PRD, sección 3) y a la arquitectura (ARQ/ADR de 02). «Detalle» indica la spec donde se define el comportamiento, con criterios de aceptación. Los RF se numeran de forma continua para que las specs 04 y 05 puedan citarlos; los RF-01..RF-10 del PRD se citan como «PRD RF-nn».

### 7.1 Proyectos y robustez (PRD RF-01, RF-10)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-01 | Crear, abrir, duplicar, renombrar, archivar y eliminar proyectos desde Inicio y Archivo; proyectos recientes con miniatura. | RF-01 | MVP | ADR-04 | 03 §7.2, 04 §13 |
| RF-02 | Un único formato abierto y documentado, `.gafira` (ZIP + `manifest.json` versionado), con migraciones por paso y archivos de esquema futuro en solo lectura. | RF-01, RNF-06 | MVP | ADR-04, ARQ-53, ARQ-55 | 04 §13 |
| RF-03 | Autoguardado cada 5–15 s (10 por defecto), guardado atómico y Recuperación tras fallo con pérdida ≤ 15 s. | RF-01, RF-10 | MVP | ARQ-50..52 | 04 §13, 03 §7.9 |
| RF-04 | Deshacer y Rehacer amplios, fusión de gestos, Versiones nombradas y automáticas (carpeta `versions/` del `.gafira`; una automática al guardar si la última tiene más de 30 min, las 20 más recientes; las nombradas no se borran solas). | §2.1 Historial | MVP | ADR-03, ARQ-30, ARQ-32, ARQ-33, 02 §7.1 | 04 §13 |
| RF-05 | Todas las funciones de edición, importación y exportación funcionan sin conexión; la única conexión de red es la de la licencia (D-01). | RF-01, RNF-03 | MVP | ARQ-04, ARQ-91 | 04, 05 |

### 7.2 Lienzo y editor de diseño (PRD RF-02)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-06 | Lienzo con tamaños predefinidos y personalizados, unidades px/pulgada/mm/cm, orientación y varias páginas apiladas con acciones por página. | §2.1 Lienzo | MVP | `gafira-geom`, ADR-02 | 04 §2 |
| RF-07 | Reglas, Guías arrastrables, Generador de guías con vista previa, Márgenes, Sangrado y Zona de seguridad. | RF-02 | MVP | `gafira-layout` | 04 §2 |
| RF-08 | **Imán** y **Cuadrícula** como funciones propias, con umbral y paso configurables. | RF-02, §2.1 | MVP | `gafira-layout` | 04 §2 |
| RF-09 | Zoom de 5 % a 3200 % con ajuste a selección, página o ancho. | RF-02 | MVP | §6.1 de 02 (tiles, mips) | 04 §2 |
| RF-10 | Selección múltiple, campos numéricos exactos (X, Y, Ancho, Alto, Giro, Opacidad), proporción bloqueable, Alinear, distribuir, Agrupar, bloquear, ocultar y Duplicar. | RF-02 | MVP | ARQ-30, ARQ-33 | 04 §3 |
| RF-11 | Capas: lista con arrastre, orden de apilado, visibilidad, bloqueo y modos de mezcla. | RF-02, RF-04 | MVP | `NodeKind`, `BlendMode` | 04 §3 |
| RF-12 | Los elementos fuera de la página se recortan al verlos y al exportar sin perderse; la selección indica cuánto sobresalen. | RF-02 | MVP | 02 §3.2 | 04 §2 |
| RF-13 | Atajos configurables y Paleta de comandos (Ctrl/Cmd+K) con todas las acciones. | RF-02, RNF-05 | MVP | `Tool`, `Command` | 03 §9 |
| RF-14 | Modo claro, oscuro o del sistema, con tokens validados (AA). | RF-02, RNF-01 | MVP | ARQ-60, ARQ-61 | 03 §3, §4 |

### 7.3 Plantillas, Piezas y Biblioteca (PRD RF-03)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-15 | Plantillas locales con búsqueda y filtros (formato, estilo, color, orientación, duración); aplicar añade una página con elementos editables; Guardar como plantilla propia. | RF-03 | MVP | `AssetProvider` | 04 §4 |
| RF-16 | Piezas: formas, líneas, Marcos con recorte por forma, Iconos, Códigos QR, Tablas, Gráficos, Ilustraciones y Nota adhesiva. | §2.1 Elementos | MVP | `FrameNode`, `TableNode`, `StickyNode` | 04 §5 |
| RF-17 | Herramientas de Dibujo: Lápiz, Marcador, Resaltador y Borrador, con color y grosor. | §2.1 | MVP | `Tool` | 04 §6 |
| RF-18 | Biblioteca de archivos propios: búsqueda por etiqueta y color, Colecciones, detección de duplicados, versiones, licencias, importación masiva y aviso de vínculo ausente. | RF-03 | MVP | `ContentStore`, ARQ-56 | 04 §7 |

*Nota (PRD §2.1 y §3 RF-03, D-08)*: el PRD pide filtrar las plantillas por idioma. Las plantillas empaquetadas **declaran su idioma** en su índice (`language = "es" | "en" | "neutral"`, 02 §14.5); la pestaña Plantillas muestra por defecto las del idioma de la interfaz más las neutras y ofrece el filtro Idioma junto a los de RF-15. Las plantillas propias se marcan `neutral` salvo que la persona elija otro. La spec 04 (§4) aún no recoge este filtro.

### 7.4 Imagen (PRD RF-04)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-19 | Importar PNG, JPEG, WebP, GIF, TIFF, BMP, QOI y SVG; HEIC (lectura vía SO, `heic-os`, sujeto a Q-17; sin soporte en Linux) y PDF en Fase 2. | §2.1 Imágenes | MVP / F2 | ARQ-21, `heic-os` | 04 §8 |
| RF-20 | Ajustes de imagen no destructivos con Zona de edición: Ajuste automático, Balance de blancos, Luz, Color, Textura. | RF-04 | MVP | `Effect`, ARQ-20 | 04 §8 |
| RF-21 | Filtros con vista previa, Sombras, Bordes y esquinas, Comparar con el original y Restablecer. | RF-04 | MVP | `Effect` | 04 §8 |
| RF-22 | **Aislar sujeto** sin modelo (técnica clásica), máscara no destructiva y Retocar borde con Pincel Quitar y Pincel Devolver. | RF-04 | MVP | `Segmenter`, `ClassicSegmenter` | 04 §9 |
| RF-23 | Segmentadores y reescritura de texto con modelo local: opcionales, desactivables, instalados aparte; nunca en el instalador base. | RF-04, §8 | F3 | `onnx`, ARQ-76 | 04 §14 |

### 7.5 Texto (PRD RF-05)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-24 | Texto con fuente, tamaño, color (incluye degradados y colores de la Identidad), B/I/U/tachado, mayúsculas, alineación, Listas, interletrado, interlineado y kerning. | RF-05 | MVP | `gafira-text` | 04 §10 |
| RF-25 | Efectos de texto: Contorno, Sombra, Degradado, fondo, brillo, eco y Curvar. | RF-05, §2.1 | MVP | `gafira-text`, `gafira-pathops` | 04 §10 |
| RF-26 | **Columnas**, **Ajustar al cuadro** y **Fijar tipografía** como funciones propias (`TextNode.columns`, `autofit`, `type_lock`). | RF-05 | MVP | `gafira-text`, 02 §3.2 | 04 §10 |
| RF-27 | Detección de fuentes del sistema, sustitución con aviso y fuente original recordada, Buscar y reemplazar, aviso de licencia de fuentes no incluidas. | RF-05 | MVP | `gafira-platform`, Q-10 | 04 §10 |

### 7.6 Identidad visual y Cambiar tamaño (PRD RF-06, RF-03)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-28 | Identidad visual: logos, colores, Tipografías, Plantillas propias, Estilos y notas de voz; los colores enlazados se actualizan en todo el diseño con un `Command`. | RF-06 | MVP | `VisualIdentity`, `SetSwatch` | 04 §11 |
| RF-29 | Varias identidades; aplicar con un clic y deshacer en un paso; bloqueo opcional de elementos; validación de contraste. | RF-06 | MVP | ARQ-61 | 04 §11 |
| RF-30 | Cambiar tamaño con Destinos y vista previa; **Duplicar y adaptar** y **Adaptar este diseño**; el reflujo conserva la jerarquía. | RF-03 | MVP | `ResizeDesign`, `gafira-layout` | 04 §12 |

### 7.7 Exportación (PRD RF-09)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-31 | Exportar imagen: PNG (escala, calidad, fondo transparente), JPEG, WebP, GIF animado y SVG; por página, selección o rango. | RF-09 | MVP | `Exporter` | 04 §13 |
| RF-32 | Exportar PDF RGB (Digital e Impresión, Sangrado y marcas de corte, aplanar, orden de lectura accesible, contraseña); PDF CMYK en Fase 2. | RF-09, §2.1 | MVP / F2 | `gafira-io-pdf`, `ColorTransform` | 04 §13 |
| RF-33 | Validación previa (fuentes, recursos, resolución, códec) y escritura atómica; registro de cada exportación. | RF-09 | MVP | §6.4 de 02, ARQ-50 | 04 §13, 05 |
| RF-34 | Cola de exportación: básica (MVP) y con prioridades, pausa y reintentos (Fase 2). | RF-09 | MVP / F2 | §6.4 de 02, ARQ-43 | 05 §B |
| RF-35 | Exportar el proyecto editable con sus recursos para migrar a otro equipo. | RF-09 | MVP | `.gafira` | 04 §13 |

### 7.8 Video básico (PRD RF-07)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-36 | Línea de tiempo con pistas de Video, Superposiciones, Texto y Audio, zoom, marcadores y Cabezal. | RF-07 | MVP | `Sequence`, `Track`, `Clip` | 05 §A |
| RF-37 | Importar MP4/MOV, WebM/MKV, GIF, WAV, FLAC y Ogg; MP3 **solo importación** con decodificador permisivo (VERIFICAR licencia, D-07); H.264/AAC solo vía API del sistema (D-02); AVI en Fase 2 con códecs libres (D-07); con clase legal por códec. La importación de HEVC queda fuera del MVP (Q-17). | §2.2 | MVP / F2 | ADR-07, `Container`, `Codec` | 05 §A |
| RF-38 | Dividir, Recortar, Duplicar, reordenar, Velocidad, Invertir y Congelar fotograma, todo no destructivo. | RF-07 | MVP | `SplitClip`, `TrimClip` | 05 §A |
| RF-39 | Nueva escena y Transiciones (Corte, Fundido, Deslizar, Zoom, Borrado, Disolvencia) en la unión de clips, con el trait `Transition` (dos entradas, `TransitionInstance` en `Clip.transitions`). | RF-07 | MVP | `Clip.transitions`, `Transition`, ARQ-25 | 05 §A |
| RF-40 | Texto animado, títulos y tercios inferiores; presets de Movimiento y Ruta de movimiento. | RF-07 | MVP | `Animatable<T>` | 05 §A |
| RF-41 | Subtítulos manuales con editor de tiempo y estilos; automáticos solo como opción local futura (VERIFICAR). | RF-07 | MVP | `Track::Caption` | 05 §A |
| RF-42 | Sonidos (música y efectos con vista previa), volumen, fundidos, silenciar, Separar audio, Nivelar y sincronización básica. | RF-07 | MVP | `AudioProcessor` | 05 §A |
| RF-43 | Vista previa en tiempo real y scrubbing sin bloquear la interfaz. | RF-07 | MVP | §6.5 de 02, ARQ-41 | 05 §A |
| RF-44 | Exportar video con preajustes 9:16, 1:1, 4:5, 16:9 y 21:9, por defecto en AV1 + Opus (WebM o MP4); H.264/AAC solo vía API del sistema. **Extracción de fotograma (PNG) y de audio (WAV) en el MVP.** | §2.2 | MVP | ADR-07, `gafira-mux` | 05 §A |

### 7.9 Video avanzado mínimo (PRD RF-08)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-45 | Multicapa y Agrupar en secuencia (anidación reutilizable). | RF-08 | F2 | `ClipSource::Scene` | 05 §B |
| RF-46 | Fotogramas clave con Editor de curvas (lineal, ease-in/out/in-out, Bézier) para posición, escala, rotación, opacidad, recorte, volumen y efectos; copiar y pegar animaciones. | RF-08 | F2 | `Animatable<T>`, ARQ-12 | 05 §B |
| RF-47 | Máscaras animables: Rectángulo, Elipse, Polígono, Vector y Texto. | RF-08 | F2 | `Mask` | 05 §B |
| RF-48 | Clave de color con tolerancia, supresión de derrame y ajuste de bordes. | RF-08 | F2 | `Effect` | 05 §B |
| RF-49 | Seguimiento básico y Estabilizar (opcional). | RF-08 | F2 | `Effect` | 05 §B |
| RF-50 | Corrección de color: LUT `.cube`, Curvas RGB, Ruedas, Balance de blancos, Medidores y comparación antes/después. | RF-08 | F2 | `LutTransform`, `ColorTransform` | 05 §B |
| RF-51 | Audio avanzado: Mezclador de hasta 16 pistas, Ecualizador de 3 bandas (procesadores `Eq3Band` y `Pan`: MVP en `gafira-audio`, D-22; su interfaz, F2), Puerta de ruido (`NoiseGate`), Compresor, Limitador, Reducir ruido, *time-stretch* (`TimeStretch`), Nivelar y medidor de sonoridad EBU R128. | RF-08 | F2 | ARQ-23, 02 §4.9 | 05 §B |
| RF-52 | Importar y exportar SRT y VTT. | §2.3 | F2 | `gafira-captions` | 05 §B |
| RF-53 | Copias ligeras: generación automática y manual para material de alta resolución; la exportación usa siempre el original. | RF-08 | MVP (motor) / F2 (gestión) | §6.5 de 02 | 05 §B |
| RF-54 | Exportación con control de códec, bitrate, fps, resolución, rango de color, audio y metadatos. | RF-08, RF-09 | F2 | `EncoderConfig` | 05 §B |
| RF-55 | Relocalizar medios cuando cambia una carpeta (medios vinculados: `manifest.assets[...].linked {rel_path, bytes, blake3}`; evento `LinksMissing`). | RF-08 | F2 | ARQ-56, 02 §7.1 | 05 §B, 04 §7 |

### 7.10 Distribución y extensión (PRD RNF-01, RNF-06)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-56 | Instaladores: MSI (Windows), `.deb`, `.rpm` y `.tar.gz` portable (solo Linux x86_64; sin AppImage, Flatpak ni aarch64; D-05), DMG firmado y notarizado (macOS aarch64). | RNF-01 | MVP | ARQ-03, ARQ-80, ARQ-83, ARQ-84 | 05 §C |
| RF-57 | Asociación del tipo `.gafira` con icono y apertura al hacer doble clic. | RNF-01 | MVP | — | 05 §C |
| RF-58 | Avisos de terceros (`THIRD_PARTY_NOTICES.md`), SBOM y auditoría de licencias en cada versión. | RNF-06 | MVP | ARQ-70..76 | 05 §C |
| RF-59 | **Cancelado (D-06).** La app no comprueba versiones nuevas ni contiene actualizador; el usuario descarga cada versión desde la web. El ID se conserva porque otras specs lo citan. | RNF-01 | — | ARQ-78 | 05 §C |
| RF-60 | Plugins con API estable (efectos, importadores, proveedores, comandos). | RNF-06 | F3 | `PluginHost`, ARQ-24 | 05 §C |
| RF-61 | Exportación por lotes con datos de un CSV (variables de contenido). | PRD Fase 3 | F3 | `gafira-cli` | 05 §C |
| RF-62 | Exportar a PPTX: una diapositiva por página como imagen de alta resolución, sin objetos editables (D-07). | — (no figura en el PRD) | F2 | `gafira-io-pptx`, `Exporter` | 04 §13 |
| RF-63 | Licencia y activación: todo de pago y una sola licencia, con modo sin licencia en lugar de prueba gratuita (exportación bloqueada y funciones desactivadas; D-12); activar, validar y desactivar es la única conexión de red (D-01). Licencia y activación: ver spec 06 (pendiente, se redacta al final). | — (sustituye un principio del PRD) | Fase final (§5.1) | §4.14 y §13 de 02, ARQ-90..96 | spec 06 |

### 7.11 Idioma (D-08)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-64 | **Idiomas publicados: español e inglés.** Toda la interfaz (menús, pestañas, paneles, herramientas, mensajes, tooltips, nombres accesibles, Paleta de comandos), el glosario y el microcopy existen en los dos idiomas con correspondencia 1:1; números, fechas, tiempos relativos y unidades se formatean según el idioma. El contenido de la persona no se traduce. | — (decisión del titular; el PRD pide filtrar plantillas por idioma) | MVP | `gafira-i18n`, ADR-09, ARQ-97, ARQ-98, ARQ-102, ARQ-105, ARQ-106 | 03 §2.8, §12, §16; 02 §14 |
| RF-65 | **Idioma por defecto según el sistema**, con la regla exacta de D-08: cualquier variante de español (`es`, `es-MX`, `es-CO`, `es-ES`…) → español; inglés (`en`, `en-US`, `en-GB`…) → inglés; **cualquier otro idioma, o ningún dato del SO → inglés**. Se detecta en Windows, macOS y Linux (variables de locale) y el primer arranque no pregunta el idioma. | — | MVP | `SystemLocaleSource` (`gafira-platform`), `D08LocaleResolver`, ARQ-100 | 02 §14.3, §14.4; 03 §16.2, DIS-74 |
| RF-66 | **Selector manual** en Ajustes → General con las opciones `Sistema` (por defecto), `Español` y `English`; la elección se guarda en los ajustes, sobrevive a reiniciar y se aplica al momento, sin reiniciar ni cerrar el proyecto. | — | MVP | `LanguagePreference`, `LocaleChanged`, ARQ-101 | 03 §16.1, DIS-73, DIS-75 |

### 7.12 Arranque (D-09)

| ID | Requisito | PRD | Fase | ARQ | Detalle |
|---|---|---|---|---|---|
| RF-67 | **Pantalla de arranque.** Al iniciar, Gafira muestra brevemente una pantalla con la ilustración `assets/splash_screen.jpg` de fondo (recorte «cubrir») y, sobre un panel inferior de blanco cálido, el logo horizontal, el eslogan («Haz visibles tus ideas.» / «Bring your ideas to life.», según el idioma de D-08), la versión y una barra de carga coral que refleja el progreso real. Se cierra al terminar la carga necesaria para Inicio, con un mínimo de 1,2 s; avisa si la carga supera 5 s (PROPUESTA) y ofrece «Reintentar» si el arranque falla. Respeta reducir movimiento y no roba el foco. **La imagen es reemplazable**: se sustituye el archivo (mismo nombre, JPEG o PNG) y se recompila, sin tocar código; va incrustada en el binario y no depende de rutas del usuario. | — (decisión del titular; sin RF en el PRD) | MVP | ARQ-65..ARQ-69, ADR-10 | 03 §7.10, DIS-80..DIS-87 |

---

## 8. Requisitos no funcionales

| ID | Requisito | PRD | ARQ | Criterios de calidad |
|---|---|---|---|---|
| RNF-01 | **Plataforma.** Windows 10/11 x86_64 (MSI), Linux x86_64 (`.deb`, `.rpm` y `.tar.gz`) y macOS 12+ aarch64 (DMG). Pantallas HiDPI, modo oscuro y teclado completo. Sin web ni otras plataformas. Varios monitores: la ventana principal es única; paneles flotantes en un segundo monitor son PROPUESTA de Fase 2. | RNF-01 | ARQ-03, ARQ-80 | — |
| RNF-02 | **Rendimiento.** Se cumplen las cifras de la §9; el hilo de UI nunca bloquea más de 16 ms por trabajo del engine; la GPU es opcional y existe siempre alternativa en CPU con estimación de tiempo visible. | RNF-02 | ARQ-40..44, ARQ-22, ARQ-65, ARQ-68 | CAL-01..CAL-16, CAL-38, CAL-39 |
| RNF-03 | **Confiabilidad.** Autoguardado con recuperación del 100 %, guardado atómico, copia `.bak` de una generación, ninguna operación destructiva sin Deshacer, verificación de integridad en exportaciones. | RNF-03 | ARQ-50..57 | CAL-17, CAL-18 |
| RNF-04 | **Seguridad y privacidad.** Procesamiento 100 % local; la única conexión de red es la de la licencia, que envía solo los datos que declara `PRIVACY.md` (identificador de dispositivo incluido; D-01); sin cuentas, nube ni telemetría automática; el registro de actividad es local y auditable; permisos mínimos del SO; los modelos de IA, si existen, se instalan aparte y se desactivan por defecto. | RNF-04 | ARQ-04, ARQ-24, ARQ-76, ARQ-91 | CAL-21 |
| RNF-05 | **Usabilidad.** Primera pieza publicable en < 10 min; toda función accesible por Paleta de comandos o atajo; paneles redimensionables y plegables; toda acción destructiva es reversible o pide confirmación. | RNF-05 | ARQ-63, ARQ-64 | CAL-20, CAL-33 |
| RNF-06 | **Mantenibilidad.** Arquitectura por crates y capas, formato versionado y documentado, pruebas automáticas de render, exportación, recuperación y rendimiento. | RNF-06 | ARQ-01, ARQ-05, ARQ-80..83 | — |
| RNF-07 | **Vendibilidad.** Solo licencias permisivas (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode; OFL para fuentes). Sin GPL, LGPL ni AGPL en el código distribuido (las bibliotecas LGPL del sistema solo se enlazan de forma dinámica y no se empaquetan, ARQ-79), sin FFmpeg ni ejecutables de terceros en el instalador. Patentes de códecs aisladas por feature. | Restricción del proyecto | ARQ-70..76, ADR-07 | — |
| RNF-08 | **Accesibilidad.** Contraste AA, foco visible, AccessKit, reducción de movimiento, color nunca como única señal. | RNF-01, RNF-05 | ARQ-60..64 | CAL-33 |
| RNF-09 | **Tamaño y recursos.** Instalador, binario y memoria dentro de los presupuestos de la §9. | — (vacío del PRD) | ARQ-83, ARQ-44 | CAL-22..CAL-26 |
| RNF-10 | **Calidad de salida.** La vista previa y la exportación comparten el grafo; el render de exportación es determinista (CPU por defecto). | RNF-03 | ARQ-42 | CAL-34 |
| RNF-11 | **Modelo comercial.** Producto propietario, todo de pago y una sola licencia, con modo sin licencia (D-12); sin módulos de pago separados (D-01). Se implementa en la fase final (§5.1) y no condiciona el MVP técnico. | Decisión del titular | ARQ-90..96 | — |
| RNF-12 | **Cobertura de traducción del 100 %.** Cada clave de mensaje, nombre del glosario y texto de microcopy existe en español y en inglés con los mismos parámetros y las formas de plural que exige cada idioma; una versión publicada nunca muestra una clave ni un texto en el otro idioma (respaldo: idioma pedido → inglés). | Decisión del titular (D-08) | ARQ-97, ARQ-98, ARQ-99 | CAL-37 |
| RNF-13 | **Cambio de idioma sin reiniciar.** Ningún cambio de idioma exige reiniciar la app ni cerrar o recargar el proyecto; se conservan selección, historial y estado de los paneles, y el cambio completo se ve en ≤ 100 ms. | Decisión del titular (D-08) | ARQ-101 | CAL-36 |

---

## 9. Criterios de calidad y velocidad

### 9.1 Hardware de referencia (FIJADA, D-03)

Los criterios de velocidad se miden en tres perfiles de referencia **fijados por el titular** (00-decisiones, D-03; tabla idéntica a 02 §6.6). Cierran el vacío del PRD («gama media» y «GPU moderna»); la pregunta Q-07 está cerrada.

| Perfil | CPU | RAM | GPU / VRAM | Disco | Pantalla |
|---|---|---|---|---|---|
| Gama media (Windows y Linux) | Intel Core i5 de 10.ª generación o AMD Ryzen 5 3600 | 8 GB | GPU integrada | SSD SATA | 1920×1080 |
| GPU (cifras con GPU, medidas aparte) | El mismo equipo de gama media | 8 GB | NVIDIA GTX 1650 de 4 GB | SSD SATA | 1920×1080 |
| macOS de referencia | MacBook Air M1 | 8 GB unificada | Integrada M1 | SSD | Retina |

Salvo indicación, «gama media» significa el primer perfil y «GPU» el segundo.

### 9.2 Método de medición común

- Cada criterio tiene un benchmark en `gafira-cli bench` o `criterion` y un umbral en `perf-budgets.toml` (ARQ-40). El job nocturno en la máquina de referencia (*self-hosted*) falla si una métrica supera su umbral o empeora más de un 10 % frente a la base.
- Los percentiles se calculan sobre ≥ 300 muestras tras calentar; se reporta p50, p95 y p99. Los documentos de prueba son sintéticos y reproducibles (`gafira-testkit`: 100 capas, 20 páginas, video de 3 pistas, Reel de 60 s).
- Las cifras de la tabla están **fijadas** por el titular (D-03: «las demás cifras propuestas en 02 pasan de PROPUESTA a fijadas»). Los criterios marcados **VERIFICAR** requieren un *spike* técnico antes de comprometerlos.
- La medición de la experiencia (CAL-20) es una prueba de usabilidad con personas, no una prueba de CI.

### 9.3 Tabla de criterios

| ID | Criterio | Cifra | Origen | Hardware | Método de medición | Trazo |
|---|---|---|---|---|---|---|
| CAL-01 | Selección de un objeto | < 16 ms (p99, 1 000 nodos) | spec técnica | Gama media | `bench_select_hit` | RF-10, ARQ-40 |
| CAL-02 | Arrastre de objetos | 60 fps (p95 ≤ 16,7 ms, 100 capas) | spec técnica | Gama media | `bench_drag_100_layers`, 300 frames | RF-10, DIS-60 |
| CAL-03 | Cambio de color | < 50 ms (p99) | spec técnica | Gama media | `bench_recolor` | RF-24, RF-28 |
| CAL-04 | Aplicar un filtro (vista previa) | < 300 ms | spec técnica | Gama media | `bench_filter_4000px_preview` | RF-21 |
| CAL-05 | Interacciones locales | < 100 ms (p99) | spec técnica | Gama media | `bench_interactions` | RF-13, DIS-62 |
| CAL-06 | Aislar sujeto de una imagen de 4000 px en CPU | < 5 s | PRD y spec técnica | Gama media | `bench_segment_4000` | RF-22 |
| CAL-07 | Aislar sujeto de 4000 px con GPU | < 2 s (VERIFICAR viabilidad con la técnica clásica) | spec técnica | GPU | `bench_segment_4000` | RF-22, R-04 de 02 |
| CAL-08 | Arranque hasta el primer frame interactivo de Inicio, incluida la Pantalla de arranque | < 5 s (objetivo interno < 2 s, FIJADA) | PRD | Gama media | `gafira --bench-startup` | RNF-02, RF-67 |
| CAL-09 | Abrir un proyecto de 20 páginas | < 3 s hasta la primera página interactiva | PRD | Gama media | `bench_open_20_pages` | RF-01 |
| CAL-10 | Abrir un documento de 100 capas | < 500 ms | spec técnica | Gama media | `bench_open_100_layers` | RF-02 |
| CAL-11 | Exportar PNG de 1080p | < 2 s | spec técnica | Gama media | `bench_export_png_1080` | RF-31 |
| CAL-12 | Rango de zoom | 5 %–3200 %, vector sin pixelado a 3200 % | PRD | Gama media | Prueba funcional y golden | RF-09 |
| CAL-13 | Vista previa de video de 60 s con 3 pistas | Frames de UI p99 < 16,7 ms durante la reproducción | PRD | Gama media | `bench_playback_3tracks` | RF-43 |
| CAL-14 | Scrubbing 1080p | ≥ 24 actualizaciones/s; búsqueda en caché < 50 ms, sin caché < 250 ms (FIJADA) | PRD (cifra: FIJADA) | Gama media | `bench_scrub_1080` | RF-43 |
| CAL-15 | Mezclador de 16 pistas con EQ de 3 bandas | < 25 % de un núcleo a 48 kHz (FIJADA) | PRD (cifra de CPU: FIJADA) | Gama media | `bench_mix_16` | RF-51, ARQ-23 |
| CAL-16 | Exportar un Reel de 60 s a 1080p | < 180 s (requiere ≥ 10 fps de extremo a extremo a 30 fps) | PRD | Gama media con el códec del SO; con AV1 por software (salida por defecto, D-02) solo si cumple (R-03 de 02) | `bench_export_reel_60s` | RF-44 |
| CAL-17 | Exportaciones exitosas sin intervención | ≥ 98 % (meta interna 100 %) | PRD | Todos | Suite `export_matrix` (≥ 200 combinaciones) | RF-33 |
| CAL-18 | Pérdida máxima por fallo y recuperación | ≤ 15 s y 100 % recuperables | PRD | Todos | 500 *kills* aleatorios (ARQ-51, ARQ-52) | RF-03 |
| CAL-19 | Intervalo de autoguardado | 5–15 s configurable, 10 s por defecto (FIJADA el valor por defecto); cada autoguardado < 200 ms en el proyecto de 20 páginas (FIJADA) | PRD | Gama media | Traza de `tracing` | RF-03 |
| CAL-20 | Primera pieza publicable | < 10 min | PRD | Persona sin experiencia previa | Prueba de usabilidad con 5 personas (plantilla → editar texto → exportar) | RNF-05 |
| CAL-21 | Quejas de lag | < 5 % de sesiones | PRD | Todos | Informe local opcional y exportable por la persona; sin envío automático (§13) | RNF-04 |
| CAL-22 | Límite de memoria y VRAM configurables | Por defecto 50 % de la RAM y 70 % de la VRAM (FIJADA); no superar el límite más de un 10 % | PRD | Todos | Prueba con límite de 1 GB (ARQ-44) | RNF-09 |
| CAL-23 | Tamaño del instalador | ≤ 60 MB por plataforma (FIJADA) | D-03 | — | `xtask size` (ARQ-83) | RNF-09 |
| CAL-24 | Tamaño del binario | ≤ 40 MB tras `strip` (FIJADA) | D-03 | — | `xtask size` | RNF-09 |
| CAL-25 | RAM en reposo con documento vacío | ≤ 300 MB (FIJADA) | D-03 | Gama media | Medición del proceso | RNF-09 |
| CAL-26 | RAM con 20 páginas e imágenes | ≤ 1,5 GB (FIJADA) | D-03 | Gama media | Medición del proceso | RNF-09 |
| CAL-27 | Respuesta de cancelación | ≤ 100 ms (exportación 4K, segmentación) | 02 | Todos | ARQ-43 | RF-34 |
| CAL-28 | Deshacer y rehacer | < 16 ms (p99, 100 capas) | 02 | Gama media | ARQ-32 | RF-04 |
| CAL-29 | Historial | Presupuesto de 512 MB (FIJADA) con compactación | 02 | Todos | Prueba de historial largo | RF-04 |
| CAL-30 | Frecuencias de fotograma de proyecto | 23,976, 24, 25, 29,97, 30, 50 y 60; vista previa a fps del proyecto (60 en GPU) | 02 (FIJADA) | GPU para 60 | `bench_playback_3tracks` | RF-36 |
| CAL-31 | Reescritura de texto con modelo local (opcional) | < 4 s (sin prueba en el MVP) | spec técnica | GPU | Solo cuando exista (F3) | RF-23 |
| CAL-32 | Indexado de keyframes al importar | ≤ 1 s por minuto de material 1080p (FIJADA) | 02 | Gama media | `bench_index_keyframes` | RF-37 |
| CAL-33 | Contraste y accesibilidad | AA en todos los pares de tokens; 0 widgets sin nombre accesible | manual y PRD | Todos | DIS-10, DIS-32 | RNF-08 |
| CAL-34 | Coherencia vista previa y exportación | ≤ 1/255 por canal en el 99,9 % de los píxeles | 02 | CPU de referencia | ARQ-42 | RNF-10 |
| CAL-35 | Estabilidad del hilo de UI | 0 trabajos > 16 ms fuera de diálogos del SO | 02 | Gama media | ARQ-41 | RNF-02 |
| CAL-36 | Cambio de idioma con un proyecto abierto | ≤ 100 ms (p99) hasta el primer frame completo en el idioma nuevo, sin reiniciar | D-08 (cifra PROPUESTA, igual que el cambio de modo, DIS-64) | Gama media | `bench_locale_switch` (ARQ-101, DIS-75) | RF-66, RNF-13 |
| CAL-37 | Cobertura de traducción | 100 % de claves en `es` y `en`; 0 truncados sin tooltip en las 8 pantallas en `es`, `en` y seudo-localización | D-08 | Todos | `xtask check-i18n` (ARQ-98), capturas de DIS-76 (ARQ-103) | RF-64, RNF-12 |
| CAL-38 | Primer frame de la Pantalla de arranque desde que se lanza el proceso | < 300 ms | D-09 (cifra PROPUESTA) | Gama media y macOS de referencia | `gafira --bench-startup` (`splash_first_frame_ms`, ARQ-65) | RF-67, RNF-02 |
| CAL-39 | Duración visible de la Pantalla de arranque | ≥ 1,2 s (D-09) y ≤ el mayor entre 1,2 s y el fin de la carga necesaria para Inicio, más 1 frame; aviso de carga lenta a los 5 s (PROPUESTA) | D-09 | Gama media | `gafira --bench-startup` (`splash_visible_ms`) y pruebas con arranque simulado (03 DIS-82, ARQ-68) | RF-67 |

- **CAL-A**: ninguna cifra de la tabla se relaja sin un cambio documentado de esta spec y una nueva decisión del titular (D-03). *Aceptación*: `perf-budgets.toml` contiene una entrada por cada CAL medible y un revisor del repositorio lo valida en cada cambio de umbral.

---

## 10. Métricas de éxito y restricciones

### 10.1 Métricas de éxito

| Métrica | Meta | Origen | Cómo se mide sin servidores |
|---|---|---|---|
| Tiempo hasta la primera pieza publicable | < 10 min | PRD | Prueba de usabilidad (CAL-20) en cada versión mayor |
| Proyectos recuperados tras fallo | 100 % | PRD | 500 *kills* aleatorios en CI (CAL-18) |
| Pérdida máxima de trabajo por fallo | ≤ 15 s | PRD | Ídem |
| Exportaciones exitosas sin intervención | ≥ 98 % | PRD | `export_matrix` en CI (CAL-17) |
| Reel de 60 s a 1080p | < 3 min | PRD | `bench_export_reel_60s` (CAL-16) |
| Quejas de lag | < 5 % de sesiones | PRD | Informe local de frames lentos que la persona puede exportar y compartir de forma voluntaria; no hay envío automático (CAL-21) |
| Adopción de funciones avanzadas | 30 % de usuarios activos mensuales usan Fotogramas clave, Máscaras o Corrección de color | PRD | **No medible de forma automática** sin telemetría. Se sustituye por encuesta voluntaria dentro de la app (PROPUESTA) y por conteos locales que la persona puede exportar. Ver §13 |

### 10.2 Restricciones

| Restricción | Detalle | Fuente |
|---|---|---|
| Plataformas | Solo Windows 10/11 x86_64 (MSI), Linux x86_64 (`.deb`, `.rpm` y `.tar.gz`) y macOS 12+ aarch64 (DMG). Sin Linux aarch64, AppImage ni Flatpak (D-05) | D-05, ARQ-03, ARQ-84 |
| Local | Sin servidores, web ni nube; la única conexión de red es la de la licencia (D-01); sin comprobación de versiones (D-06) | ARQ-04, ARQ-78, ARQ-91 |
| Licencias | Dependencias y assets con licencias permisivas; `cargo-deny` en CI; sin dependencias git | ARQ-70, ARQ-71 |
| Venta | Gafira debe poder venderse sin restricciones de licencia de terceros. Licencia propia: propietaria y de código cerrado, basada en el contrato de Tracewright (`LICENSE` y `LICENCIA.md`, prevalece el español) con tres adaptaciones; todo de pago, modo sin licencia y una sola licencia | D-01, D-12 |
| Binarios | Ningún ejecutable de terceros en el instalador | ARQ-72 |
| Actualizaciones | La app no comprueba versiones nuevas; el usuario descarga cada versión desde la web | D-06, ARQ-78 |
| Patentes | Exportación por defecto AV1 + Opus; H.264/AAC solo vía API del sistema; sin encoders propios de H.264, HEVC ni AAC; sin salida HEVC ni MP3; `encumbered-sw` desactivada | D-02, ADR-07 |
| Marca | Colores, logo, tipografías, eslogan y tono del manual; tokens de Gafira prevalecen | Manual de marca |
| Nombres | Los nombres de interfaz son los del glosario de 03, en español y en inglés, y no copian los de otros productos | Spec 03, DIS-01..03, DIS-71, DIS-72 |
| Idioma | Español e inglés; por defecto según el sistema (cualquier variante de español → español; cualquier otro idioma → inglés); selector en Ajustes → General; las specs siguen en español | D-08, RF-64..RF-66 |
| IA | Opcional, local, desactivable y fuera del MVP | PRD §4 y §8 |

---

## 11. Modelo comercial: todo de pago con modo sin licencia

**Decisiones D-01 y D-12 (00-decisiones.md).** Gafira es de pago, con **una sola licencia**. No hay prueba gratuita: sin licencia, la app funciona en **modo sin licencia** (D-12). Esto **sustituye** el principio del PRD «sin funciones esenciales retenidas tras un plan de pago» y el modelo anterior de «núcleo gratuito y módulos avanzados de pago». Se eliminan el reparto núcleo/avanzado, los módulos de pago separados y los requisitos que dependían de funciones gratuitas.

### 11.1 Principios

1. **Una licencia, un binario.** El mismo ejecutable contiene todas las funciones de todas las fases; no hay módulos que se activen por separado.
2. **Modo sin licencia** (D-12). La exportación está bloqueada y algunas funciones están desactivadas. La lista de funciones bloqueadas se define en la spec 06 (pendiente 1 del titular). Hasta entonces, solo la exportación está bloqueada: con `Unlicensed` o `Expired` se puede editar, recuperar un journal, guardar y usar «Guardar copia» (D-55, 02 ARQ-95). Una licencia activada lo desbloquea todo.
3. **Sin marcas de agua** en las exportaciones (PROPUESTA).
4. **Datos del usuario nunca retenidos.** Un proyecto del usuario siempre se puede abrir, ver y recuperar; sin licencia rige el modo sin licencia (D-12), que sustituye la propuesta de solo lectura de 02. La lista de funciones bloqueadas se define en la spec 06 (Q-20).
5. **Única conexión de red = licencia** (activar, validar o desactivar). `PRIVACY.md` declara el identificador de dispositivo que se envía al activar y no puede afirmar que la app no transmite datos.
6. **Transparencia.** Precio, renovación (si existe) y cancelación se muestran antes de pagar (precio por definir, tentativo 99 USD, PROPUESTA; pendiente 1 del titular).

### 11.2 Alcance y fase

La licencia **no bloquea el MVP técnico** de diseño y video: se entrega en la fase final «Licencia y distribución comercial» (§5.1). El contrato, el cliente, los estados de licencia y la interfaz de activación se detallan en la spec 06. **Licencia y activación: ver spec 06 (pendiente, se redacta al final).** El servicio `license-hub` es externo, ya existe y se reutiliza tal cual; no se construye ningún servidor de licencias en este proyecto.

---

## 12. Riesgos y preguntas abiertas

### 12.1 Riesgos de producto

Los riesgos técnicos R-01..R-14 están en 02 (§12.1). Aquí se añaden los de producto.

| ID | Riesgo | Mitigación |
|---|---|---|
| P-01 | **Alcance.** Tres capas en un solo MVP (diseño, video básico y motor para el avanzado) es mucho trabajo para un equipo pequeño. | Entregas incrementales dentro del MVP: diseño estático primero, luego video básico (02 §11.2). Cada incremento es utilizable. |
| P-02 | **Exportar H.264 puede exponer a patentes** y no funciona en Linux sin VA-API. | D-02, ADR-07: solo vía API del sistema; AV1 + Opus es la salida por defecto y siempre está disponible. |
| P-03 | **Contenido incluido** (plantillas, ilustraciones, fotos, música, sonidos) sin licencia clara impide vender. | Solo contenido propio o con licencia permisiva registrada (ARQ-75); paquete de contenido separado si crece el instalador (R-09). |
| P-04 | **Calidad percibida de la técnica clásica de Aislar sujeto** frente a modelos de IA. | Retocar borde con pincel; prueba bloqueante con IoU ≥ 0,90 en 16 escenas sintéticas con máscara exacta (D-20), sin generalización demostrada: 9 de 12 en escenas nuevas, que son un aviso (D-45); el IoU sobre fotos reales es un aviso hasta que el titular aporte un conjunto con licencia; Segmentadores ONNX opcionales en Fase 3. |
| P-05 | **Accesibilidad de egui** insuficiente en el lienzo. | AccessKit con árbol propio; plan B iced (R-10 de 02). |
| P-06 | **Marca y nombres.** «Gafira» (y «Gefira», por parecido) podrían coincidir con marcas registradas; el glosario está aprobado (D-04). | Búsqueda del titular en USPTO, TMview y WIPO antes de publicar (§12.4; spec 05, Parte C). |
| P-07 | **Funciones bloqueadas y precio sin definir.** | Pendiente 1 del titular antes de la comunicación comercial; la licencia se diseña en la spec 06. |
| P-08 | **Medición de métricas sin telemetría** (adopción, lag). | Informe local voluntario y encuesta opcional (§10.1). |
| P-09 | **Diferencias de render entre GPU y drivers.** | CPU como referencia de exportación; tolerancias golden (R-07 de 02). |

### 12.2 Preguntas cerradas

Resueltas por el titular en `00-decisiones.md`. Se conservan los IDs porque otras specs los citan.

| ID | Pregunta | Resolución |
|---|---|---|
| Q-01 | Licencia y modelo comercial. | Cerrada: D-01 y D-12 (propietaria, todo de pago, modo sin licencia, una sola licencia). |
| Q-02 | ¿Basta delegar H.264/HEVC/AAC en el SO? | Cerrada: D-02 (H.264/AAC solo vía API del sistema; HEVC sin salida; AV1 + Opus por defecto). El dictamen legal sigue como VERIFICAR (R-01 de 02). |
| Q-03 | Linux sin VA-API. | Cerrada: D-02 (sin decodificador de software; se avisa y se propone AV1). |
| Q-04 | ¿Linux aarch64? | Cerrada: D-05 (no; solo x86_64). |
| Q-05 | Formatos de paquete Linux. | Cerrada: D-05 (`.deb`, `.rpm` y `.tar.gz`; sin AppImage ni Flatpak). |
| Q-06 | Actualizaciones firmadas. | Cerrada: D-06 (la app no comprueba versiones). |
| Q-07 | Hardware de referencia y cifras. | Cerrada: D-03 (cifras fijadas, §9). |
| Q-08 | PPTX. | Cerrada: D-07 (Fase 2, una imagen por diapositiva). |
| Q-09 | MP3 como importación. | Cerrada: D-07 (solo importación, decodificador permisivo, VERIFICAR licencia). |
| Q-11 | Reparto núcleo/avanzado. | Cerrada: D-01 (no hay módulos de pago separados). |
| Q-12 | Activación de módulos avanzados. | Cerrada: D-01 (activación con `gafira-license` y `license-hub`; detalle en la spec 06). |
| Q-13 | Importación de AVI y de AAC/M4A. | Cerrada: D-07 (AVI en Fase 2 solo con códecs libres) y D-02 (AAC solo vía SO). |
| Q-16 | Aprobación del glosario de 03. | Cerrada: D-04 (aprobado sin cambios); queda la búsqueda de marca del titular (§12.4). |
| Q-14 | ¿Idioma del producto: solo español o también inglés al lanzar? | Cerrada: D-08 (español e inglés; por defecto según el sistema, cualquier otro idioma → inglés; selector en Ajustes → General). Requisitos RF-64..RF-66, RNF-12, RNF-13; arquitectura en 02 §14; diseño en 03 §2.8 y §16. |
| Q-19 | Mecanismo de la prueba gratuita. | Cerrada: D-12 (no hay prueba gratuita). |

### 12.3 Preguntas abiertas

Incluye las que siguen abiertas en 02 (§12.3) y las propias de este documento.

| ID | Pregunta | Responsable | Bloquea |
|---|---|---|---|
| Q-10 | ¿Se permite empaquetar fuentes del usuario dentro de `.gafira` según su licencia? ¿Cómo se informa? | Legal + producto | RF-27 |
| Q-15 | ¿Se confirma la voz en off y los marcadores de compás como Fase 2? | Producto | Spec 05 |
| Q-17 | ¿Se admite importar HEVC (y HEIC, `heic-os`) mediante la API del sistema, o queda prohibido en toda vía? D-02 prohíbe «HEVC» sin distinguir importación de salida. | Titular + legal | RF-19, RF-37 |
| Q-18 | ¿Se mantiene el render distribuido en la red local (Fase 3)? Está en suspenso: abriría una conexión distinta de la de licencia, prohibida por D-01. | Titular | Fase 3 |
| Q-20 | ¿Qué puede hacer el usuario sin licencia válida? D-12 sustituye la propuesta de solo lectura de 02: rige el modo sin licencia. Queda la lista de funciones bloqueadas; hasta entonces solo la exportación está bloqueada (D-55). | Titular | Spec 06, RF-63 |
| Q-23 | ¿Qué idiomas se añaden después de español e inglés, y cuándo? | Titular | Fase 3 (02 §14.8) |
| Q-24 | ¿Números y fechas según la región del sistema dentro del idioma (es-MX `1,234.5`, es-ES `1234,5`) o un formato fijo por idioma? Propuesta de 02 §14.4: según la región. | Titular | RF-64 |
| Q-25 | Aprobación de los nombres ingleses del glosario (03 §2.8) y del microcopy inglés que no sale del manual (03 §12), y búsqueda de marca de los nombres ingleses propios (*Pieces*, *Magnet*, *Light copies*…). | Titular | RF-64, DIS-71, DIS-72 |
| Q-26 | ¿Se confirma que `gafira-cli` imprime en inglés por defecto y solo cambia con `--lang` (02 ARQ-104)? ¿Y que el instalador, los paquetes y el `.desktop` de Linux también se ofrecen en los dos idiomas (la spec 05 FV-30 dice solo español)? | Titular | RF-56, RF-64 |
| Q-34 | Aspecto de las fórmulas de 04 FD-33 y catálogo de filtros de 04 FD-34 (02 §12.3, D-40). Q-35..Q-45 están cerradas (D-29 a D-34 y D-41 a D-45). | Titular | No bloquea la especificación |

### 12.4 Pendientes del titular

Lista de `00-decisiones.md`, con el ajuste posterior del titular sobre el `license-hub`:

1. Definir la lista de funciones bloqueadas sin licencia (D-12) y el precio.
2. Adaptar `LICENSE` y `LICENCIA.md` (contrato de Tracewright con las tres adaptaciones de D-01) y escribir el `PRIVACY.md` de Gafira, con el identificador de dispositivo.
3. `license-hub`: el servicio ya está desplegado y se reutiliza tal cual; su lista de seguridad y su documentación quedan fuera de estas specs. (00-decisiones lo listaba como «completar y desplegar».)
4. Hacer la búsqueda de marca de D-04 (lista de comprobación siguiente).
5. Verificar con `cargo-deny` las licencias marcadas VERIFICAR (incluido el decodificador MP3).
6. Aportar un conjunto de fotos reales con licencia para `gafira-segment` antes de publicar (D-20).

**Lista de comprobación de marca (D-04).** Sin abogado; registrar fecha, término, resultado y captura de cada búsqueda antes de publicar.

| # | Base de datos | Términos a buscar | Resultado |
|---|---|---|---|
| 1 | USPTO (Estados Unidos) | «Gafira» y «Gefira» (por parecido), clases de software y de diseño | Pendiente |
| 2 | TMview (Unión Europea y oficinas asociadas) | «Gafira» y «Gefira» | Pendiente |
| 3 | WIPO Global Brand Database (Madrid y nacionales) | «Gafira» y «Gefira» | Pendiente |

Dato previo: una búsqueda web general encontró una empresa suiza de TI llamada «Gefira», con otra ortografía.

---

## 13. Resolución de contradicciones entre PRD, spec técnica y manual

Decisiones ya tomadas por el proyecto; no se reabren en las specs 02 a 05.

| # | Contradicción | Fuentes | Resolución |
|---|---|---|---|
| C-01 | **Web frente a escritorio.** El manual describe uso en web (dominio de aplicación web y favicon). | Manual vs PRD | Gafira es **escritorio local**. Se ignoran las referencias web del manual. No hay favicon; hay icono de aplicación (03 §5.3). |
| C-02 | **Sincronización y nube opcional**, biblioteca remota de plantillas y cifrado de sincronización. | PRD RF-01, RF-03, RNF-04 | Fuera de alcance (§6). Plantillas y recursos son locales. |
| C-03 | **IA.** La spec técnica incluye reescritura de texto y separación del fondo con modelo; el PRD la deja opcional. | spec técnica vs PRD | IA opcional, local, desactivable y fuera del MVP (RF-23). Aislar sujeto usa técnica clásica sin modelo (RF-22) con el trait `Segmenter` para añadir modelos después. |
| C-04 | **Formato de proyecto único.** `.designpkg` (PRD) frente a `.design` con SQLite (spec técnica). | PRD vs spec técnica | **`.gafira`**: ZIP con `manifest.json` versionado (ADR-04, RF-02). |
| C-05 | **Tokens de marca.** El manual define coral y grafito; la spec técnica propone violeta `#8B3DFF`. | Manual vs spec técnica | Prevalecen los tokens de Gafira (spec 03 §3). El violeta se descarta. |
| C-06 | **Modo oscuro** exigido por el PRD sin paleta en el manual. | PRD vs manual | Se **deriva** de la paleta y se valida con AA (spec 03 §3 y §4); es PROPUESTA hasta aprobación de marca. |
| C-07 | **Códecs.** El PRD pide MP4/H.264 y WebM/VP9 en el MVP. | PRD vs 02 | D-02: salida por defecto AV1 + Opus (WebM y MP4); H.264/AAC solo vía API del sistema; sin salida HEVC. VP9 llega en Fase 2 (ADR-07). |
| C-08 | **Importaciones de video y audio.** El PRD lista AVI, AAC, M4A y MP3. | PRD vs 02 | AAC y M4A vía SO (D-02); MP3 solo importación con decodificador permisivo, VERIFICAR licencia (D-07); AVI en Fase 2 solo con códecs libres (D-07). |
| C-09 | **Plataformas.** El PRD pide macOS 12+ y «Linux principales» sin arquitectura. | PRD vs decisión | macOS solo aarch64; Linux solo x86_64 con `.deb`, `.rpm` y `.tar.gz` (D-05). |
| C-10 | **Cola de render.** El PRD la coloca en el MVP; 02 la hace avanzada en Fase 2. | PRD vs 02 | Cola básica (un trabajo a la vez, cancelar, reintentos transitorios) en MVP; prioridades y pausa en Fase 2 (RF-34). |
| C-11 | **Proxies.** El PRD los pone en Fase 2; 02 los usa en el motor del MVP. | PRD vs 02 | Motor de Copias ligeras en MVP para cumplir el scrubbing 1080p; gestión manual en Fase 2 (RF-53). |
| C-12 | **Aislar sujeto en GPU.** El PRD pide separar el fondo en < 5 s con GPU dedicada; la spec técnica pide < 5 s CPU y < 2 s GPU. | PRD vs spec técnica | CPU < 5 s es obligatorio; GPU < 2 s es VERIFICAR (CAL-06, CAL-07). |
| C-13 | **Telemetría.** El PRD admite telemetría local opcional de fallos; algunas métricas requieren datos de uso. | PRD | Solo local y opcional, sin envío automático. Las métricas de adopción y lag se obtienen por informe voluntario (§10.1). |
| C-14 | **Actualizador con verificación de firma** (PRD). | PRD vs decisión | Eliminado (D-06): la app no comprueba versiones; los paquetes siguen firmados por plataforma. RF-59 cancelado. |
| C-15 | **Modo simple y modo experto separados** (PRD §4). | PRD | Divulgación progresiva en una sola interfaz (PP-2), sin dos modos. |
| C-16 | **Nombre de la función de marca.** El PRD y el producto explorado la designan con un nombre en inglés de otro producto. | PRD | Se llama **Identidad visual** (glosario de 03). |
| C-17 | **PDF/X y formatos de impresión.** El PRD menciona PDF/X. | PRD vs 02 | PDF RGB en MVP; CMYK en Fase 2; PDF/X en VERIFICAR (no previsto en 02). |
| C-18 | **Varios monitores.** El PRD lo cita en RNF-01. | PRD | Ventana única; paneles flotantes como PROPUESTA de Fase 2. |
| C-19 | **Funciones esenciales sin retener tras un plan de pago** (principio del PRD). | PRD vs D-01 | D-01 y D-12 lo sustituyen: todo de pago, modo sin licencia y una sola licencia (§11). |
| C-20 | **Idioma del producto.** El manual fija español e inglés como idiomas prioritarios y da el eslogan inglés; el PRD no define idioma de interfaz; 03 decía «la traducción al inglés (si se hace)» y 05 FV-30 limita el instalador al español «según Q-14». | Manual vs PRD vs 03/05 | D-08: español e inglés, por defecto según el sistema y cualquier otro idioma → inglés (§7.11). 03 ya está alineada; 05 FV-30 queda pendiente de actualizar (Q-26). |
| C-21 | **Plantillas por idioma.** El PRD pide una biblioteca «local y remota» filtrable por idioma. | PRD vs §6 | Solo plantillas locales (§6); cada plantilla empaquetada declara su idioma y la pestaña Plantillas filtra por él (nota de §7.3). |
| C-22 | **Logo del splash.** El manual cita el «símbolo aislado» como uso correcto en un splash (§7.9, uso 3) y prohíbe el logo sobre una fotografía agitada sin superficie de contraste (§7.10); 03 §5.1 preveía un logo apilado sobre `bg` sin imagen. | Manual vs D-09 vs 03 | D-09: logo horizontal `gafira-logo-primary.svg` sobre un panel `#FAFAF7` (uso correcto 1 y 6 del manual), nunca sobre la ilustración; 03 §5.1 y §7.10 ya están alineadas (RF-67). |

---

## 14. Trazabilidad inversa PRD → RF

| PRD | RF de este documento |
|---|---|
| RF-01 Gestión de proyectos | RF-01..RF-05, RF-35 |
| RF-02 Editor de diseño | RF-06..RF-14 |
| RF-03 Plantillas y assets | RF-15, RF-16, RF-18, RF-30 (filtro por idioma: nota de §7.3, RF-64) |
| RF-04 Imagen y corrección visual | RF-19..RF-23, RF-11 |
| RF-05 Texto y tipografía | RF-24..RF-27 |
| RF-06 Identidad visual (marca) | RF-28, RF-29 |
| RF-07 Video básico | RF-36..RF-44 |
| RF-08 Video avanzado | RF-45..RF-55 |
| RF-09 Exportación y render | RF-31..RF-35, RF-44, RF-54 |
| RF-10 Rendimiento y estabilidad | RF-03, RNF-02, RNF-03, CAL-01..CAL-35 |
| RNF-01..RNF-06 | RNF-01..RNF-06 (misma numeración) y RNF-07..RNF-11 (nuevos) |
| Distribución, avisos y licencia (decisiones D-01, D-05, D-06) | RF-56..RF-59 (RF-59 cancelado), RF-62, RF-63 |
| Idioma (decisión D-08; sin RF en el PRD) | RF-64..RF-66, RNF-12, RNF-13, CAL-36, CAL-37 → 02 ARQ-97..ARQ-107, ADR-09 → 03 DIS-01, DIS-71..DIS-79 |
| Pantalla de arranque (decisión D-09; sin RF en el PRD) | RF-67, CAL-08 (incluye la Pantalla de arranque), CAL-38, CAL-39 → 02 ARQ-65..ARQ-69, ADR-10 → 03 §7.10, DIS-80..DIS-87. La imagen es reemplazable sin tocar código (03 §7.10.10) |
