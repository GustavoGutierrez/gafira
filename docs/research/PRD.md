## 1. Visión del producto

Construir una aplicación de escritorio multiplataforma —prioridad Windows, macOS y Linux— orientada a creadores de contenido que necesitan producir piezas gráficas y videos de forma rápida, con control profesional, rendimiento local y salida de alta calidad.

El producto debe cubrir tres capas:

1. **Diseño gráfico rápido:** plantillas, assets, texto, imagen, marca y exportación.
2. **Edición de video básica:** clips, cortes, subtítulos, audio, transiciones y formatos verticales/horizontales.
3. **Edición de video avanzada mínima:** multicapa, keyframes, máscaras, corrección de color, audio avanzado, proxies y exportación controlada.

La propuesta evita el problema de “todo depende de la nube”: los proyectos, cachés, previews y renderizados deben poder ejecutarse localmente, con sincronización opcional.

## 2. Alcance incluido

### 2.1 Diseño gráfico

| Módulo | Funcionalidad obligatoria |
| :-- | :-- |
| Lienzo | Lienzos fijos y personalizados, unidades px/mm/cm/in, orientación vertical/horizontal, páginas múltiples, guías, reglas, snap, grid y zoom preciso. |
| Plantillas | Biblioteca local y remota de plantillas por formato, categoría, idioma y estilo; duplicación, personalización y guardado como plantilla propia. |
| Elementos | Formas, líneas, marcos, iconos, stickers, ilustraciones, QR, tablas, gráficos, mockups y elementos vectoriales. |
| Imágenes | Importar JPG, PNG, WebP, SVG, HEIC, TIFF; recortar, redimensionar, rotar, ajustar exposición, contraste, temperatura, sombras, luces, saturación, nitidez y viñeta. |
| Fondo | Removedor de fondo local, reemplazo de fondo, desenfoque, color sólido, degradado, patrón e imagen. |
| Texto | Texto artístico, estilos, curvas, contorno, sombra, degradado, kerning, interlineado, columnas, listas, ajuste automático y bloqueo tipográfico. |
| Marca | Brand Kit local: logotipos, paletas, tipografías, plantillas de marca, activos reutilizables y variables de estilo. |
| Assets | Biblioteca personal con búsqueda, etiquetas, colecciones, versiones, licencias y detección de duplicados. |
| Adaptación de formatos | Resize inteligente con conservación de jerarquía, ajuste manual posterior y previsualización por destino: post, story, Reel, miniatura, banner, presentación, print. |
| Exportación | PNG, JPG, WebP, SVG, PDF, PDF/X donde aplique, GIF, MP4, WebM y paquetes de proyecto; exportación por página, selección o rango. |
| Historial | Undo/redo ilimitado o amplio, versiones nombradas, autoguardado local y recuperación ante fallos. |

### 2.2 Video básico

Estas funciones deben estar disponibles desde el primer release:

- Línea de tiempo con pistas de video, audio, texto, imágenes y overlays.
- Importación de MP4, MOV, WebM, MKV, AVI, GIF, JPG, PNG y audio MP3, WAV, AAC, M4A y FLAC.
- Corte, división, recorte, duplicación, reordenar, velocidad, reversa y congelar frame.
- Transiciones básicas: corte, fundido, deslizamiento, zoom, wipe y disolvencia.
- Texto animado, subtítulos manuales, títulos inferiores y lower thirds.
- Biblioteca de música, efectos de sonido y plantillas de animación.
- Ajuste de volumen, fade in/out, mute, separación de audio y sincronización básica.
- Previsualización en tiempo real con scrubbing fluido.
- Exportación a MP4/H.264 y WebM/VP9, con presets para 9:16, 1:1, 4:5, 16:9 y 21:9.
- Exportación de frames como imagen y extracción de audio.


### 2.3 Video avanzado mínimo

Estas capacidades diferencian la app de un editor “solo plantillas”:

- Línea de tiempo multicapa con anidación de secuencias o composiciones.
- Keyframes para posición, escala, rotación, opacidad, recorte, volumen y efectos.
- Curvas de animación: lineal, ease-in, ease-out, ease-in-out y curvas personalizadas.
- Máscaras: rectangulares, elípticas, poligonales, por forma vectorial y por texto.
- Chroma key o pantalla verde.
- Motion tracking básico para texto, logos y objetos.
- Estabilización de video opcional.
- Corrección de color: LUTs, curvas RGB, ruedas de color, balance de blancos, scopes y comparación antes/después.
- Audio avanzado: mezclador por pista, ecualizador, compresor, limitador, ruido reducido, normalización y medición de niveles.
- Captura y edición de subtítulos con importación/exportación SRT y VTT.
- Proxies para editar material 4K/6K en hardware modesto.
- Renderizado local con cola de exportación, prioridades, reintentos y notificación al finalizar.
- Exportación con control de códec, bitrate, FPS, resolución, rango de color, audio y metadatos.


## 3. Requerimientos funcionales

### RF-01 — Gestión de proyectos

- El usuario debe crear, abrir, duplicar, renombrar, archivar y eliminar proyectos.
- Cada proyecto debe guardar lienzo, páginas, timeline, assets vinculados, ajustes, versiones y preferencias.
- El proyecto debe utilizar un formato abierto o documentado, por ejemplo un paquete `.designpkg` con manifiesto JSON y recursos referenciados.
- Debe existir autoguardado local cada 5 a 15 segundos o tras cada acción relevante.
- Debe recuperarse el último estado válido tras un cierre inesperado.
- Debe permitirse trabajar sin conexión; la nube es opcional.

**Criterio de aceptación:** si la aplicación se cierra por fallo, al reabrirla recupera el proyecto con una pérdida máxima de 15 segundos de trabajo.

### RF-02 — Editor de diseño

- Selección múltiple, agrupación, desagrupación, alineación, distribución, orden Z, bloqueo, ocultamiento y duplicado.
- Snap inteligente a guías, bordes, centros, márgenes y otros objetos.
- Transformaciones numéricas exactas: X, Y, ancho, alto, rotación, opacidad y relación de aspecto.
- Atajos de teclado configurables y paleta de comandos tipo `Ctrl/Cmd + K`.
- Modo oscuro y claro.
- Zoom desde 5% a 3200%, con ajuste a selección, página o ancho.
- Reglas, guías, grids, márgenes seguros y áreas seguras para video vertical.

**Criterio de aceptación:** un usuario puede alinear, distribuir y ajustar 20 objetos sin usar el mouse para cada acción, mediante atajos y campos numéricos.

### RF-03 — Plantillas y assets

- Plantillas filtrables por formato, tema, duración, idioma, orientación, estilo y nivel de edición.
- Cada plantilla debe indicar fuentes, licencias y elementos reemplazables.
- El usuario puede crear plantillas propias desde cualquier diseño o secuencia de video.
- Biblioteca de assets con búsqueda semántica local opcional, etiquetas, favoritos, colecciones y filtros por tipo, tamaño, duración y licencia.
- Importación masiva por arrastrar y soltar.
- Los assets deben poder vincularse o incrustarse en el proyecto; la app debe advertir si un vínculo externo falta.

**Criterio de aceptación:** una plantilla puede cambiarse de 1080x1920 a 1080x1080 conservando textos, imágenes principales y jerarquía, con ajuste manual posterior.

### RF-04 — Imagen y corrección visual

- Removedor de fondo ejecutado localmente cuando el hardware lo permita.
- Selección por objeto, persona, cielo, fondo o color.
- Ajustes destructivos y no destructivos; los ajustes deben poder editarse o eliminarse después.
- Filtros y LUTs aplicables a imágenes y clips.
- Historial de ediciones por capa.
- Soporte de capas con blend modes: normal, multiply, screen, overlay, darken, lighten y otros modos estándar.

**Criterio de aceptación:** el removedor de fondo procesa una imagen de 4000 px en menos de 5 segundos en un equipo con GPU dedicada moderna, o muestra estimación clara en CPU.

### RF-05 — Texto y tipografía

- Instalación y detección de fuentes locales.
- Sustitución masiva de fuentes cuando falta una tipografía.
- Advertencia de licencia o compatibilidad al usar fuentes no incluidas.
- Estilos de texto reutilizables: título, subtítulo, cuerpo, caption, CTA.
- Ajuste automático de tamaño, interlineado y tracking para evitar desbordes.
- Texto en curva, arco, vertical y siguiendo una forma vectorial.
- Exportación de texto como texto seleccionable en PDF cuando el formato lo permita.

**Criterio de aceptación:** si falta una fuente al abrir un proyecto, la app propone sustitutos compatibles y conserva el layout sin romper el diseño.

### RF-06 — Brand Kit

- Creación de múltiples marcas o perfiles de marca.
- Almacenamiento local de logos, colores, tipografías, plantillas, tono visual y activos aprobados.
- Aplicación de marca con un clic a diseños existentes.
- Validación de contraste, accesibilidad y consistencia de color.
- Bloqueo opcional de elementos de marca para evitar modificaciones accidentales.

**Criterio de aceptación:** al aplicar un Brand Kit, la app reemplaza colores y fuentes mapeadas, conserva jerarquía y permite deshacer la operación completa.

### RF-07 — Editor de video básico

- Timeline con zoom horizontal, snapping a frames, marcadores, rangos de selección y navegación por teclado.
- Operaciones sin destruir el archivo original: todos los cortes deben ser no destructivos.
- Preview con reproducción a 1x, 0.5x, 2x y frame a frame.
- Texto, imágenes, stickers y formas como capas sobre el video.
- Subtítulos manuales con editor de tiempo, estilo global y plantillas.
- Audio con fade, volumen por clip, mute y sincronización visual de formas de onda.
- Exportación con presets por plataforma, pero sin publicación automática.

**Criterio de aceptación:** un video de 60 segundos con 3 pistas, subtítulos, música y transiciones debe previsualizarse sin bloquear la interfaz en un equipo de gama media.

### RF-08 — Editor de video avanzado

- Composiciones anidadas para reutilizar intros, outros, lower thirds y secuencias.
- Keyframes con editor de curvas y copiado/pegado de animaciones.
- Máscaras animables.
- Chroma key con tolerancia, spill suppression y ajuste de bordes.
- Motion tracking para pegar texto, logos o imágenes a un objeto en movimiento.
- Color grading con scopes: waveform, vectorscope, histograma y parade RGB.
- Mezclador de audio con hasta 16 pistas visibles, EQ de 3 bandas mínimo, compresor, gate, limitador y medidor LUFS.
- Reducción de ruido de audio con previsualización.
- Proxies generados automáticamente para clips de alta resolución.
- Relink de medios si se mueve una carpeta.

**Criterio de aceptación:** un proyecto con video 4K, subtítulos, música, voice-over, LUT, keyframes y máscara puede editarse mediante proxies y exportarse a 1080p sin bloquear la UI.

### RF-09 — Exportación y render

- Cola de renderizado con múltiples trabajos, progreso, cancelación, pausa y reintento.
- Renderizado en segundo plano sin impedir editar otro proyecto.
- Presets técnicos y personalizados.
- Exportación por rango de tiempo, página, selección o composición.
- Validación previa: fuentes faltantes, assets ausentes, audio silenciado, resolución inconsistente o espacio de color incompatible.
- Registro de exportación con ruta, configuración, duración, tamaño y errores.
- Opción de exportar proyecto editable y recursos para migrar a otra máquina.

**Criterio de aceptación:** si falta un asset o fuente, la app impide el render o advierte explícitamente antes de generarlo, en lugar de producir un archivo corrupto.

### RF-10 — Rendimiento y estabilidad

- Arquitectura que separe UI, motor de render, cola de exportación y gestión de assets.
- Autoguardado transaccional: nunca sobrescribir el proyecto hasta confirmar que el nuevo estado es válido.
- Caché de previews y miniaturas.
- Límite configurable de memoria y VRAM.
- Recuperación de sesión con snapshot del estado de la aplicación.
- Telemetría local opcional de fallos, sin enviar contenido del usuario por defecto.

**Criterio de aceptación:** la aplicación debe seguir respondiendo durante un render largo y permitir cancelar sin corromper el proyecto.

## 4. Soluciones a los reproches de Canva

| Reprocho detectado | Solución en la propuesta |
| :-- | :-- |
| Funciones esenciales bloqueadas tras paywall | Definir un núcleo gratuito completo: diseño, video básico, removedor de fondo, resize, subtítulos y exportación sin marca de agua. Los módulos avanzados se venden como Pro, no como partes esenciales. |
| Precios, cancelación y reembolsos confusos | Suscripción transparente, precio mostrado con impuestos, cancelación en un clic desde la app, confirmación previa a renovación y política de reembolso visible antes de pagar. |
| Lag, crashes y pérdida de trabajo | Motor local, autoguardado transaccional, snapshots, proxies, límites de memoria y recuperación de sesión. |
| Archivos corruptos o exportaciones fallidas | Validación previa al render, render atómico en archivo temporal, verificación de integridad y reintento automático. |
| Bajo control profesional | Capas, máscaras, keyframes, curvas, color grading, scopes, audio avanzado y exportación técnica. |
| Contenido homogéneo por plantillas | Plantillas como punto de partida, no como límite: estilos editables, variables de marca, componentes reutilizables y modo “blank canvas” potente. |
| Video y animación limitados | Timeline multicapa, keyframes, máscaras, chroma key, tracking, audio profesional y exportación configurable. |
| IA excesiva o invasiva | IA opcional, desactivable y contextual. Ninguna función de IA debe abrirse automáticamente ni insertar contenido sin confirmación. |
| Interfaz confusa | Modo simple y modo experto, búsqueda de comandos, onboarding por tareas, tooltips y atajos consistentes. |
| Experiencia móvil limitada | Enfoque desktop-first, con atajos, paneles redimensionables y soporte para monitores múltiples. |
| Portabilidad deficiente | Proyecto portable, importación/exportación de medios, SVG, PDF, SRT/VTT, LUTs y presets; exportación a formatos estándar. |
| Soporte percibido como débil | Diagnóstico integrado, logs exportables, base de conocimiento local, reporte de bug con datos técnicos y canal claro de soporte. |

## 5. Requerimientos no funcionales

### RNF-01 — Plataforma

- Aplicación de escritorio nativa o híbrida de alto rendimiento para Windows 10/11, macOS 12+ y distribuciones Linux principales.
- Soporte para pantallas HiDPI, modo oscuro, múltiples monitores y accesibilidad de teclado.
- Instalación sin dependencias invisibles; actualizador con verificación de firma.


### RNF-02 — Rendimiento

- Arranque de la aplicación en menos de 5 segundos en hardware de gama media.
- Apertura de un proyecto de diseño de 20 páginas en menos de 3 segundos.
- Scrubbing de un timeline 1080p sin bloqueos perceptibles.
- Uso de GPU para previews, removedor de fondo, tracking y render cuando esté disponible.
- Fallback eficiente a CPU con estimación de tiempo visible.


### RNF-03 — Confiabilidad

- Disponibilidad total del editor sin conexión.
- Autoguardado con recuperación garantizada.
- Ninguna operación destructiva sin undo.
- Backups rotativos automáticos del proyecto.
- Verificación de integridad en exportaciones.


### RNF-04 — Seguridad y privacidad

- Procesamiento local por defecto de imágenes, video, audio y documentos.
- Cifrado del caché y de proyectos sincronizados si se habilita nube.
- No enviar contenido del usuario, biometría ni archivos a servicios externos sin consentimiento explícito.
- Permisos mínimos del sistema operativo.
- Registro de actividad local auditable.


### RNF-05 — Usabilidad

- Un creador nuevo debe producir su primer post o Reel en menos de 10 minutos.
- Un editor experimentado debe acceder a cualquier función mediante paleta de comandos o atajo.
- Los paneles deben ser acoplables, ocultables y reordenables.
- Toda acción destructiva debe ser reversible o pedir confirmación.


### RNF-06 — Mantenibilidad

- Arquitectura modular: diseño, video, audio, assets, export, plantillas y Brand Kit como dominios independientes.
- Plugins con API estable para efectos, exportadores, fuentes y herramientas.
- Formato de proyecto versionado y documentado.
- Tests automatizados de render, exportación, recuperación y rendimiento.


## 6. Priorización por fases

### MVP — Diseño y video básico

- Lienzo, capas, texto, imágenes, formas, assets, plantillas.
- Removedor de fondo, resize, Brand Kit y exportación.
- Timeline básico, cortes, subtítulos, música, transiciones y exportación MP4/WebM.
- Autoguardado, recuperación, historial y cola de render.


### Fase 2 — Control profesional

- Keyframes, curvas, máscaras, chroma key, motion tracking.
- Color grading, scopes, LUTs y audio avanzado.
- Composiciones anidadas, proxies y relink de medios.
- Plantillas de video y componentes reutilizables.


### Fase 3 — Diferenciación

- Plugins, automatizaciones locales, batch export, variables de contenido.
- Biblioteca de assets local avanzada.
- Perfiles de marca múltiples.
- Optimización GPU y render distribuido local opcional.


## 7. Métricas de éxito

| Métrica | Meta |
| :-- | :-- |
| Tiempo hasta primera pieza publicable | Menos de 10 minutos |
| Tasa de proyectos recuperados tras fallo | 100% |
| Pérdida máxima de trabajo por crash | 15 segundos |
| Exportaciones exitosas sin intervención | 98% o más |
| Tiempo de render de Reel de 60 s a 1080p | Menos de 3 minutos en hardware de referencia |
| Quejas por lag en proyectos medianos | Menos del 5% de sesiones |
| Adopción de funciones avanzadas | 30% de usuarios activos mensuales usa keyframes, máscaras o color grading |

## 8. Fuera de alcance explícito

No incluir en esta versión:

- Colaboración en tiempo real.
- Comentarios y aprobaciones de equipo.
- Calendario editorial.
- Programación o publicación directa en redes sociales.
- Integraciones con plataformas sociales.
- Magic Write, generación de textos por IA.
- Magic Design o generación automática de layouts.
- Magic Media o generación de imágenes/video por IA.
- Funciones móviles completas.
- Marketplace público de plantillas de terceros.

El principio rector es: **editor local, rápido, estable, portable y con control profesional**, donde la IA es opcional y las funciones esenciales no quedan rehenes del plan pago.[^1][^2][^3][^4]

<div align="center">⁂</div>

[^1]: https://www.tryorbye.com/products/canva

[^2]: https://www.prezent.ai/blog/canva-review

[^3]: https://www.capterra.com/p/168956/Canva/reviews/

[^4]: https://kompozy.io/reviews/canva

