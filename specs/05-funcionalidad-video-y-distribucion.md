# 05 — Funcionalidad de video y distribución

| Campo | Valor |
|---|---|
| Documento | Especificación 05 |
| Estado | Borrador para revisión |
| Alcance | **Parte A** video básico · **Parte B** video avanzado mínimo (con diseño de interacción) · **Parte C** empaquetado y distribución |
| Depende de | `00-decisiones.md` (D-01 a D-07, prevalecen), 01 (RF, RNF, CAL), 02 (crates, traits, ARQ, ADR; en especial §3.4, §4.8, §4.9, §4.13, §6.4, §6.5, §7.1, §9.3, §10), 03 (glosario §2, componentes, wireframe del Editor de video §7.4) |
| Convenciones | `FV-nn` = ficha de función de video y distribución. Cada ficha indica propósito, comportamiento y reglas, UI (nombres del glosario de 03), técnica (crates y traits de 02), presupuesto, aceptación y `RF-nn` de 01. **PROPUESTA** y **VERIFICAR** como en 01 |

**Reglas generales de video** (se suman a G-1..G-6 de la spec 04)

- **V-1**: todas las ediciones son **no destructivas**: los archivos originales no se modifican nunca (RF-38).
- **V-2**: el tiempo interno se mide en *flicks* (`gafira-time`, 1/705 600 000 s); la interfaz muestra timecode `HH:MM:SS:FF` y segundos. Los cortes caen siempre en un fotograma exacto de la secuencia.
- **V-3**: las exportaciones usan los medios **originales**, nunca las Copias ligeras (02 §6.5).
- **V-4**: el audio manda el reloj de reproducción; el video descarta fotogramas si se retrasa y nunca bloquea el audio (02 §6.5).
- **V-5**: cada códec tiene una **clase legal** (`LegalClass`: Libre de regalías, Licenciado por el SO, Restringido) visible en el diálogo de importación y de exportación (ADR-07).
- **V-6**: si un códec del SO falta, se propone la alternativa libre de regalías (AV1 + Opus) sin bloquear el trabajo (02 §9.3, D-02).
- **V-7**: la salida por defecto es **AV1 + Opus** (WebM o MP4). H.264/AAC solo existe mediante `gafira-codec-os` (API del sistema); no hay encoders propios de H.264, HEVC ni AAC, no hay salida HEVC ni MP3 y la feature `encumbered-sw` permanece desactivada (D-02, D-07).
- **V-8**: ninguna función de video abre conexiones de red; la única conexión de la app es la de la licencia (ARQ-04). Licencia y activación: ver spec 06 (pendiente, se redacta al final).

---

# Parte A — Video básico

El Editor de video comparte el `Project` con el Editor de diseño (02 §3.1). Una `Sequence` tiene `fps`, `size` y `sample_rate`; sus pistas son `Track { kind: Video | Overlay | Audio | Caption }`. La interfaz presenta cinco carriles: **Texto**, **Superposiciones**, **Video**, **Audio** y **Subtítulos**. Texto y Superposiciones son pistas `Overlay` (la primera se crea con etiqueta «Texto» y es la predeterminada para cuadros de texto); no se añade un `TrackKind` nuevo.

## A1. Línea de tiempo y pistas

**FV-01 · Línea de tiempo** (RF-36)
- *Propósito*: organizar clips en el tiempo de forma precisa y legible.
- *Comportamiento y reglas*: la **Línea de tiempo** (03 §7.4) tiene regla de segundos/fotogramas, **Cabezal** arrastrable, zoom horizontal (slider y `Ctrl+rueda`, de un fotograma a toda la secuencia), «Mostrar todas las pistas» y marcadores (`M`) con nombre y color. Pistas: añadir, renombrar, silenciar, bloquear, ocultar y reordenar por arrastre; pista **Video** (principal: ripple por defecto), **Superposiciones** y **Texto** (encima, en capas), **Audio** (hasta 16 pistas) y **Subtítulos**. El **Imán** (FD-06 aplicado al tiempo) atrae clips al Cabezal, a otros cortes, a marcadores y a fotogramas; `Alt` lo desactiva. Selección con clic, marco y `Shift`; mover con arrastre o con `Alt+←/→` (un fotograma). Duración editable por clip con campo numérico. Frecuencias de proyecto admitidas: 23,976, 24, 25, 29,97, 30, 50 y 60 fps. Navegación completa por teclado (03 §9.2).
- *UI*: Editor de video → Línea de tiempo.
- *Técnica*: `Sequence`, `Track`, `Clip` (02 §3.1), comandos `MoveClip`, `TrimClip`, `SplitClip`; `gafira-time`; render de miniaturas de la tira en segundo plano.
- *Presupuesto*: arrastrar clips a 60 fps con 3 pistas y 20 clips (CAL-02); zoom de la línea sin saltos > 16 ms (CAL-01).
- *Aceptación*: (a) con Imán activo un clip arrastrado a 6 px del Cabezal queda exactamente en él; (b) mover un clip es una sola entrada de historial; (c) el cambio de fps de proyecto reajusta los cortes al fotograma más cercano sin perder clips; (d) 16 pistas de audio se pueden crear y la 17.ª se rechaza con mensaje.

## A2. Importación y clases legales

**FV-02 · Importar video y audio** (RF-37)
- *Propósito*: abrir el material habitual de cámaras, móviles y bibliotecas de sonido.
- *Comportamiento y reglas*: se importa por arrastre, Biblioteca o Archivo → Importar. El **sondeo** (`Container::probe`) identifica el contenedor y los flujos; se indexan los keyframes en segundo plano (CAL-32) y se crea la tira de miniaturas. Un archivo con un códec no disponible se **importa como marcador** con el motivo y la alternativa («Este sistema no puede leer H.264. Convierte el archivo a MP4 con AV1 o instala los códecs del sistema.»; sin decodificador de software de respaldo, D-02). Cada flujo muestra su clase legal.

| Contenedor | Códec de video | Códec de audio | Clase legal | Fase | Crate |
|---|---|---|---|---|---|
| MP4 / MOV | H.264 | AAC | Licenciado por el SO (`os-codecs`) | MVP (Windows y macOS; Linux solo con VA-API, D-02) | `gafira-mux` + `gafira-codec-os` |
| MP4 / MOV | HEVC | AAC | Licenciado por el SO (D-59) | Fuera del MVP; solo importación con el decodificador del SO (macOS y Windows; en Linux, si el sistema lo ofrece); sin él, el archivo se rechaza explicando por qué. Nunca como salida (D-02, D-59) | `gafira-codec-os` |
| MP4 / MOV | AV1 | Opus | Libre de regalías | MVP | `gafira-mux` + `gafira-codec-free` |
| WebM / MKV | AV1 | Opus, Vorbis, FLAC | Libre de regalías | MVP | `gafira-mux` + `gafira-codec-free` |
| WebM / MKV | VP9 | Opus, Vorbis | Libre de regalías | **Fase 2** | `gafira-codec-free` |
| MKV | H.264 | AAC | Licenciado por el SO | MVP (igual que MP4; HEVC como en MP4, D-59) | `gafira-codec-os` |
| GIF animado | — | — | Libre | MVP (como imagen animada) | `gafira-io-image` |
| WAV / PCM | — | PCM | Libre | MVP | `gafira-mux` |
| FLAC, Ogg (Vorbis/Opus) | — | FLAC, Vorbis, Opus | Libre de regalías | MVP | `gafira-codec-free` |
| MP3 | — | MP3 | Libre de patentes vigentes; **solo importación** (D-07) | MVP | decodificador permisivo, licencia VERIFICAR (02 §9.2) |
| M4A | — | AAC | Licenciado por el SO | MVP | `gafira-codec-os` |
| AVI | MJPEG, sin comprimir | PCM | Libre de regalías; **solo importación**, sin exportación (D-07) | **Fase 2** | `gafira-mux` + `gafira-codec-free` |
| VP8, ProRes | — | — | No previsto | — | — |

- *UI*: Biblioteca → Video y Audio; diálogo de importación.
- *Técnica*: `Container`, `Demuxer`, `Codec`, `Decoder` (02 §4.8); `CodecRegistry::best_decoder` (orden: hardware; códec del SO sin hardware; libre de regalías; `Encumbered` nunca; 02 §4.8); `LegalClass`.
- *Presupuesto*: indexar ≤ 1 s por minuto de material 1080p (CAL-32); importar un clip de 1 GB sin bloquear la UI (ARQ-41).
- *Aceptación*: (a) el sondeo identifica cada fixture de la tabla; (b) los fuzz targets de cada `Demuxer` y `Decoder` pasan 10 min por PR (ARQ-21, ARQ-82); (c) un clip con códec ausente se importa como marcador y el proyecto no falla; (d) el build `free-only` importa AV1, Opus, Vorbis, FLAC y WAV (ARQ-74); (e) MP3 se importa y no existe ninguna opción de exportarlo (ARQ-77).

## A3. Edición de clips

**FV-03 · Dividir, Recortar, Duplicar y reordenar** (RF-38)
- *Propósito*: hacer los cortes básicos de forma rápida y exacta.
- *Comportamiento y reglas*: **Dividir** en el Cabezal (`S`) crea dos clips adyacentes sin pérdida; **Recortar** arrastrando los bordes (modo Mover borde, `W`) con ripple opcional; **Duplicar** (`Ctrl/Cmd+D`) coloca la copia al final del clip; **reordenar** arrastrando en la pista principal (con ripple); eliminar (`Supr`) deja hueco y **Eliminar y cerrar hueco** (`Shift+Supr`) lo cierra. Marcar entrada/salida (`I`, `O`) define el rango de un clip. Todo es no destructivo: el recorte cambia `source_in`/`place`, y se puede ampliar de nuevo hasta el límite del material.
- *UI*: Línea de tiempo, Barra contextual, menú contextual.
- *Técnica*: `SplitClip`, `TrimClip`, `MoveClip` en `gafira-edit::cmd` (02 §4.3); `merge_key` para arrastres.
- *Presupuesto*: dividir/recortar < 100 ms con 100 clips (CAL-05).
- *Aceptación*: (a) dividir y volver a unir produce una reproducción idéntica (hash de fotogramas); (b) recortar y ampliar recupera exactamente los fotogramas originales; (c) las propiedades de prueba `apply; revert` pasan para cada comando (ARQ-81).

**FV-04 · Velocidad, Invertir y Congelar fotograma** (RF-38)
- *Propósito*: cambiar el ritmo de un clip.
- *Comportamiento y reglas*: **Velocidad** de 0,1× a 10× con slider y campo (por defecto 1,0×); al cambiar, la duración del clip se ajusta y el audio se remuestrea (cambia el tono; «mantener tono» con `TimeStretch` de `gafira-audio` es de Fase 2). En cámara lenta los fotogramas se repiten o se mezclan (opción). **Invertir** reproduce el clip hacia atrás (usa decodificación por grupos de imágenes y una Copia ligera intra; avisa si el clip excede 60 s, PROPUESTA). **Congelar fotograma** divide el clip en el Cabezal e inserta un fotograma estático de 3 s (editable). Los tres son metadatos del clip.
- *UI*: Inspector → Clip (Velocidad, Invertir, Congelar fotograma).
- *Técnica*: `Clip.speed`, `gafira-media` (caché y Copia ligera), `gafira-audio` (remuestreo); comando `SetProperty`.
- *Presupuesto*: aplicar < 100 ms; reproducción invertida a la fps del proyecto en el perfil de gama media con Copia ligera.
- *Aceptación*: (a) velocidad 2,0× reduce la duración a la mitad ± 1 fotograma; (b) el audio a 2,0× conserva la sincronía (longitud de salida de 02 ARQ-128); (c) Invertir dos veces equivale al original (hash de fotogramas); (d) Congelar fotograma deja un clip de 3 s con el fotograma exacto del Cabezal.

**FV-05 · Escenas (Nueva escena)** (RF-39)
- *Propósito*: estructurar un video por bloques de diseño.
- *Comportamiento y reglas*: **Nueva escena** (`Ctrl+Alt+N`) inserta entre clips o al final una `Page` con duración (por defecto 3 s, editable) y la coloca como `ClipSource::Scene(PageId)`. Una escena se edita con el Editor de diseño (texto, formas, imágenes, Identidad) y se anima con Movimiento. Se puede aplicar una plantilla de video (Fase 2 para biblioteca ampliada; en el MVP, plantillas de escena sencillas).
- *UI*: botón `+` entre clips; Barra inferior → Escenas.
- *Técnica*: `Page.duration`, `ClipSource::Scene` (02 §3.1).
- *Presupuesto*: crear y mostrar una escena < 100 ms.
- *Aceptación*: (a) la escena aparece en la pista Video con su duración; (b) editar la escena actualiza la vista previa al instante; (c) borrar la escena elimina su clip y es reversible.

**FV-06 · Transiciones** (RF-39)
- *Propósito*: unir clips con un efecto controlado.
- *Comportamiento y reglas*: el control `◇` en la unión de dos clips abre el catálogo **Transición**: **Corte** (instantáneo), **Fundido** (a negro, blanco o color), **Disolvencia** (mezcla cruzada), **Deslizar** (cuatro direcciones), **Zoom** (acercar o alejar con mezcla) y **Borrado** (cuatro direcciones y circular, con suavidad). Las demás transiciones tienen duración 0,1–5 s (por defecto 0,5 s) y alineación (centrada en el corte, al inicio o al final). Corte ignora la duración y la alineación: el cambio ocurre en el corte (D-50). Arrastrar el borde de la transición cambia la duración. Si faltan fotogramas de margen en los clips, se repite el fotograma de borde y se avisa. Vista previa al pasar el cursor sobre cada opción.
- *UI*: `◇` en la unión; Inspector → Clip → Transición; menú contextual (Añadir Transición, `Ctrl+T`).
- *Técnica*: trait `Transition` (02 §4.13): efecto de dos entradas con `params`, `handles`, `run_cpu(cx, p, progress, from, to, dst)` y variante opcional `GpuTransition` (WGSL); el dato del documento es `TransitionInstance { transition_id, params, duration, align, easing }` dentro de `Clip.transitions` (02 §3.1). Se implementan en `gafira-raster::transitions` (Corte, Fundido, Disolvencia, Deslizar, Zoom y Borrado) y se registran en `TransitionRegistry`; añadir una transición no toca el núcleo (ARQ-25).
- *Presupuesto*: vista previa de una transición a la fps del proyecto en gama media (CAL-13).
- *Aceptación*: (a) con `progress` = 0 el resultado equivale al clip A y con `progress` = 1 al clip B (diferencia 0; ARQ-25); (b) cada transición coincide entre CPU y GPU (≤ 2/255); (c) cambiar la duración mantiene los cortes y es deshacible; (d) con Corte, en las tres alineaciones, con duración 0,1 s y 5 s y sin fotogramas de margen en los clips, el fotograma anterior al corte es el de A y el fotograma del corte es el de B (diferencia 0), no hay aviso de margen y ninguna muestra de audio mezcla A y B.

## A4. Texto, animación y subtítulos

**FV-07 · Texto animado, títulos, tercios inferiores y Movimiento** (RF-40)
- *Propósito*: añadir textos y gráficos animados sin conocimientos de animación.
- *Comportamiento y reglas*: los textos son `TextNode` (spec 04 §10) sobre las pistas Texto o Superposiciones con duración editable. La pestaña **Movimiento** ofrece presets de **entrada**, **énfasis** y **salida** (por ejemplo, aparecer, deslizar, subir, ampliar, rebotar, escribir letra a letra) por categoría, con vista previa al pasar el cursor, duración y demora; y la **Ruta de movimiento** (se arrastra el elemento por el lienzo para dibujar la trayectoria). Los presets se **compilan a `KeyTrack`** (02 §3.4), por lo que se pueden afinar después con Fotogramas clave (Parte B). **Títulos** y **tercios inferiores** son plantillas de texto y forma con estilos de la Identidad (se pueden guardar como plantillas propias, RF-15).
- *UI*: Panel lateral → Movimiento y Texto; Inspector → Clip → Movimiento; lienzo (Ruta de movimiento).
- *Técnica*: `Animatable<T>`, `anim_presets: Vec<AnimPresetRef>` (02 §3.2), `AnimPreset` como datos; `gafira-text`.
- *Presupuesto*: vista previa del preset a la fps del proyecto; evaluar animaciones en < 1 µs por muestra (ARQ-12).
- *Aceptación*: (a) aplicar un preset de entrada produce keyframes editables; (b) la Ruta de movimiento se deshace en una entrada; (c) la animación es determinista: la exportación coincide con la vista previa en ≤ 1/255 (ARQ-42); (d) con «Reducir movimiento» del SO, las animaciones del **contenido** del usuario no se alteran (02 §8.3).

**FV-08 · Subtítulos manuales** (RF-41)
- *Propósito*: añadir subtítulos legibles y sincronizados a mano.
- *Comportamiento y reglas*: la pista **Subtítulos** (`Track::Caption`) contiene cuadros con texto, inicio y fin. Se crean con `Ctrl+Enter` en el Cabezal, se editan en una lista con editor de tiempo (campos numéricos y arrastre en la línea) y heredan un **estilo global** (fuente, tamaño, color, fondo, contorno, posición) con estilos prediseñados que se aplican a todos o a los clips elegidos. Avisos no bloqueantes: más de 2 líneas o 42 caracteres por línea, duración menor de 1 s, solapes. Importar y exportar SRT/VTT es Fase 2 (FV-21). **Subtítulos automáticos**: solo como opción **local, opcional y desactivable** de una fase futura (VERIFICAR modelo de voz con licencia permisiva); no forman parte del MVP.
- *UI*: Panel lateral → Subtítulos; pista Subtítulos.
- *Técnica*: `Track::Caption`, `Clip` con `Nodes`, `gafira-text`; `gafira-captions` en Fase 2.
- *Presupuesto*: editar un subtítulo < 16 ms por pulsación; la lista con 500 subtítulos sin retraso perceptible (CAL-05).
- *Aceptación*: (a) los subtítulos se renderizan en la vista previa y en la exportación (golden); (b) cambiar el estilo global actualiza todos con un `Command`; (c) los avisos aparecen con 3 líneas.

## A5. Audio básico

**FV-09 · Sonidos: música y efectos** (RF-42)
- *Propósito*: añadir música y efectos con licencia segura.
- *Comportamiento y reglas*: la pestaña **Sonidos** es una biblioteca **local** de música (por ánimo, ritmo y duración) y de efectos, con **vista previa antes de aplicar** (clic para escuchar, doble clic para insertar en la pista de Audio en el Cabezal). Solo se incluye contenido propio o con licencia permisiva registrada en `assets/LICENSES.toml` (ARQ-75); sin licencia no se lista. El contenido va en formatos libres (Opus, FLAC o WAV). No hay descargas de catálogo. Se puede importar música propia a la Biblioteca.
- *UI*: Panel lateral → Sonidos.
- *Técnica*: `AssetProvider` (`kinds = [Audio]`), `gafira-assets`, decodificación en `gafira-codec-free`.
- *Presupuesto*: la vista previa de una pista suena en < 200 ms desde el clic (PROPUESTA).
- *Aceptación*: (a) el 100 % del contenido de `assets/audio` tiene licencia registrada; (b) la vista previa se detiene al cambiar de pista; (c) no hay solicitudes de red.

**FV-10 · Volumen, fundidos, silenciar, Separar audio, Nivelar y sincronización básica** (RF-42)
- *Propósito*: lograr un sonido equilibrado sin herramientas complejas.
- *Comportamiento y reglas*:
  - **Volumen** por clip (0–200 %, con gráfica de volumen editable en el clip) y por pista; **Silenciar** (`Ctrl+Shift+M`).
  - **Fundidos** de entrada y salida, cada uno con su duración en segundos y su asa en el clip: rampa lineal de ganancia (02 ARQ-124). Las asas no se cruzan: la suma de las dos duraciones nunca supera la duración del clip, y al arrastrar un asa o escribir su duración, el valor se limita a lo que deja la otra (D-49). Recortar el clip, acortar su Duración, subir su Velocidad, Dividirlo o Congelar fotograma, si dejan un clip por debajo de la suma de sus fundidos, reducen las dos en proporción: cada una se multiplica por `duración nueva / suma` (D-49, D-51). Tras Dividir o Congelar fotograma, la primera parte conserva el fundido de entrada, la última el de salida, y en el punto de división no hay fundido; la regla de proporción se aplica después a cada parte (D-54). Alargar el clip no devuelve los fundidos reducidos; solo Deshacer los restaura (D-52). En un arrastre, la reducción se calcula desde las duraciones del inicio del gesto: con suma inicial `S` y duración actual `D`, los fundidos son los iniciales si `D ≥ S` y los iniciales por `D / S` si no (D-52). Una sola entrada de historial deshace el cambio del clip y los fundidos. Ningún redondeo deja la suma por encima del clip.
  - **Separar audio**: extrae el audio de un clip de video a su propia pista enlazada (mover juntos, desenlazar opcional).
  - **Nivelar**: lleva el clip, la pista o todo a una sonoridad objetivo con una ganancia propia, aparte del Volumen (D-31) (por defecto −14 LUFS; opciones −16 y −23 LUFS). Limita la ganancia para que el pico real quede en −1 dBTP o por debajo e informa si no alcanzó el objetivo (D-21, 02 ARQ-127).
  - **Sincronización básica**: forma de onda visible en los clips, **desplazar** el audio con ajuste fino (± ms y por fotograma) y **marcadores** (`M`) a los que se pueden atraer los cortes con el Imán. Marcadores de compás automáticos: Fase 2 (PROPUESTA, VERIFICAR algoritmo).
  - **Voz en off** grabada desde el micrófono: Fase 2 (PROPUESTA, Q-15); requiere permiso del SO.
- *UI*: Inspector → Clip (Volumen, Fundidos, Nivelar; D-32); clic derecho (Separar audio).
- *Técnica*: `AudioProcessor` (ganancia, fundido, nivelación EBU R128; 02 §4.9); formas de onda precalculadas por `gafira-media`.
- *Presupuesto*: la reproducción no asigna ni bloquea en el hilo de audio (ARQ-23); la forma de onda de un clip de 10 min se calcula en segundo plano en < 5 s (PROPUESTA).
- *Aceptación*: (a) un fundido de 2 s lleva la ganancia de 0 a 1, o de 1 a 0, de forma continua (prueba de respuesta), y la entrada y la salida tienen duraciones independientes; (b) Nivelar lleva un audio de prueba con pico holgado a ±0,5 LU del objetivo; si el tope de pico lo impide, deja el pico real ≤ −1 dBTP e informa de que no lo alcanzó (D-21); (c) Separar audio conserva la sincronía exacta (diferencia 0 muestras); (d) el desfase fino de 1 ms se aplica con precisión de 1 muestra (02 ARQ-129); (e) en un clip de 10 s con entrada de 3 s, el asa de salida no pasa de 7 s; (f) en un clip de 10 s con entrada de 2 s y salida de 6 s, recortar a 8 s no cambia los fundidos, recortar a 6 s da 1,5 s y 4,5 s, y recortar a 0 da 0 y 0; con entrada 0 y salida 6 s, recortar a 3 s da 0 y 3 s; (g) deshacer ese recorte, en un solo paso, restaura exactamente la duración del clip y las de los dos fundidos; (h) en un clip de 10 s a 1,0× con fundidos de 2 s y 6 s, Velocidad 2,0× da un clip de 5 s con 1,25 s y 3,75 s, y volver a 1,0× deja 1,25 s y 3,75 s; (i) tras recortar a 6 s (1,5 s y 4,5 s), ampliar el clip a 10 s deja 1,5 s y 4,5 s; (j) en un solo arrastre desde 10 s, con fundidos de 2 s y 6 s, ir a 4 s y soltar en 9 s deja 2 s y 6 s, e ir a 4 s y soltar en 6 s deja 1,5 s y 4,5 s; (k) prueba de propiedad: tras Dividir o Congelar fotograma, en cada clip resultante la suma de sus fundidos es ≤ su duración; (l) un clip de 10 s con fundidos de 2 s y 6 s, dividido a los 3 s, da una parte de 3 s con entrada 2 s y salida 0, y otra de 7 s con entrada 0 y salida 6 s; dividido a 1 s, la primera parte queda con entrada 1 s y salida 0; (m) Congelar fotograma a los 3 s en ese clip da 3 s con entrada 2 s y salida 0, el fotograma estático sin fundidos, y 7 s con entrada 0 y salida 6 s; (n) dividir a los 3 s y volver a unir da las mismas muestras de audio que el clip original (diferencia 0).

## A6. Vista previa y exportación

**FV-11 · Vista previa en tiempo real y scrubbing** (RF-43)
- *Propósito*: ver y oír el resultado mientras se edita, sin esperas.
- *Comportamiento y reglas*: reproducción a 0,5×, 1× y 2× y fotograma a fotograma (`←/→`); arrastrar el Cabezal (**scrubbing**) muestra el fotograma más cercano de la caché mientras se decodifica el exacto. El reloj maestro es el audio; el video descarta fotogramas si se retrasa más de 40 ms (la misma tolerancia que FV-12 (b) fija para el archivo exportado) y nunca bloquea el audio. Indexado de keyframes al importar, caché de fotogramas con presupuesto de RAM (1 GB o el 15 % de la RAM, el menor; PROPUESTA) y prefetch en la dirección del movimiento. Copias ligeras automáticas para material de más de 1080p o códecs costosos (FV-22). La calidad de la vista previa es `Preview` (02 §6.2); se puede elegir 100 %, 50 % o 25 % de resolución.
- *UI*: vista previa y Línea de tiempo; control de calidad bajo la vista previa.
- *Técnica*: `gafira-media` (caché, reloj de reproducción, sincronía A/V), `MediaService::{play, pause, seek, scrub}`, `Decoder` por flujo (máx. 8 hilos, 02 §6.3).
- *Presupuesto*: 60 s con 3 pistas sin bloquear la UI: frames de UI p99 < 16,7 ms (CAL-13); **scrubbing 1080p ≥ 24 actualizaciones/s**, búsqueda en caché < 50 ms y sin caché < 250 ms (CAL-14, PROPUESTA); la reproducción mantiene la fps del proyecto en gama media a 1080p.
- *Aceptación*: (a) `bench_playback_3tracks` y `bench_scrub_1080` dentro del umbral; (b) tras 10 min de reproducción continua no se degrada la fluidez ni crece la memoria más de un 10 % (ARQ-44); (c) el audio nunca se corta al saltar fotogramas; (d) con el audio y el video desalineados 39 ms no se descarta ningún fotograma, y con 40 ms o más se descarta el fotograma que llega tarde sin que el audio se corte (02 §6.5, ARQ-147).

**FV-12 · Exportar video, extracción de fotograma y de audio** (RF-44, RF-34, RF-33)
- *Propósito*: obtener un archivo listo para publicar.
- *Comportamiento y reglas*: el diálogo **Exportar** (03 §7.5) en modo video ofrece **preajustes por proporción** y resolución, y formatos según la clase legal de cada códec. Se exporta toda la secuencia o un rango de entrada/salida. Antes de encolar se valida: recursos y fuentes ausentes, audio silenciado, resolución inconsistente, códec no disponible (propone alternativa). El trabajo entra en la **Cola de exportación** básica (un trabajo a la vez, progreso con estimación, **cancelar** y reintentos automáticos solo por errores transitorios; 02 §6.4).

| Preajuste | 480p | 720p | 1080p | 4K |
|---|---|---|---|---|
| **16:9** horizontal | 854 × 480 | 1280 × 720 | 1920 × 1080 | 3840 × 2160 |
| **9:16** vertical | 480 × 854 | 720 × 1280 | 1080 × 1920 | 2160 × 3840 |
| **1:1** cuadrado | 480 × 480 | 720 × 720 | 1080 × 1080 | 2160 × 2160 |
| **4:5** vertical | 480 × 600 | 720 × 900 | 1080 × 1350 | 2160 × 2700 |
| **21:9** ultra panorámico | 1120 × 480 | 1680 × 720 | 2560 × 1080 | 5120 × 2160 (solo si el hardware lo admite) |

| Formato | Video | Audio | Clase legal | Fase |
|---|---|---|---|---|
| **MP4 (H.264/AAC)** | H.264 | AAC | Licenciado por el SO (solo API del sistema) | MVP (Windows y macOS; Linux con VA-API; D-02) |
| **MP4 (AV1/Opus)** | AV1 | Opus | Libre de regalías | MVP, **por defecto** (compatibilidad en plataformas de publicación: VERIFICAR, R-11 de 02) |
| **WebM (AV1/Opus)** | AV1 | Opus | Libre de regalías | MVP, **por defecto** |
| **WebM (VP9/Opus)** | VP9 | Opus | Libre de regalías | Fase 2 |
| **Secuencia PNG** | PNG por fotograma | — | Libre | MVP |
| **Extracción de fotograma** | PNG del fotograma en el Cabezal | — | Libre | **MVP** (JPEG: Fase 2) |
| **Extracción de audio** | — | WAV | Libre | **MVP** (FLAC y Opus: Fase 2) |

El diálogo muestra «Recomendado para máxima compatibilidad legal» junto a las opciones libres de regalías (02 §9.3) y preselecciona AV1 + Opus. En Linux sin VA-API, o en cualquier equipo cuyo sistema no ofrezca el codificador, la opción H.264 se atenúa con la razón y se propone WebM o MP4 con AV1 + Opus. No se ofrece HEVC ni MP3 como salida. La **extracción de fotograma (PNG)** y la **extracción de audio (WAV)** forman parte del MVP y se lanzan desde Exportar → Video o desde el menú contextual del Cabezal.
- *UI*: Exportar → Video; Barra superior (Exportar); Cola de exportación.
- *Técnica*: `ExportJob` + `Exporter` de video en `gafira-media` (decodificar → componer → codificar → *mux*, con canales acotados, 02 §5.3); `Muxer` (`gafira-mux`), `Encoder` (`gafira-codec-free` o `gafira-codec-os`); render en `RenderQuality::Export` con CPU por defecto.
- *Presupuesto*: **Reel de 60 s a 1080p < 180 s** (CAL-16; ≥ 10 fps de extremo a extremo a 30 fps, con el códec del SO; con AV1 por software solo si el *spike* lo permite, R-03 de 02); cancelación ≤ 100 ms (CAL-27); extracción de un fotograma PNG < 2 s (CAL-11).
- *Aceptación*: (a) `bench_export_reel_60s` < 180 s en el perfil de referencia; (b) el MP4 y el WebM exportados reproducen en un reproductor de referencia con video y audio sincronizados (diferencia A/V < 40 ms); (c) la exportación sin el códec del SO ofrece AV1 + Opus sin error y no existe código de encoder H.264 propio (ARQ-77); (d) cancelar no deja archivo parcial; (e) la suite `export_matrix` incluye ≥ 40 combinaciones de video y alcanza ≥ 98 % de éxito (CAL-17); (f) exportar un fotograma (PNG) coincide con la vista previa en ≤ 1/255 y extraer el audio produce un WAV de la misma duración que la secuencia ± 1 muestra.

---

# Parte B — Video avanzado mínimo

La Parte B no tiene una referencia visual previa: el diseño de interacción es propio. Las funciones son de **Fase 2**, salvo indicación. El modelo y el motor ya existen en el MVP (`Animatable<T>`, `Mask`, `AdjustmentNode`, `Effect`, 02 §3.2, §3.4). Todos los paneles siguen los componentes y tokens de la spec 03 y se abren en el **Inspector** o en la zona inferior de la **Línea de tiempo**.

## B1. Multicapa y Agrupar en secuencia

**FV-13 · Multicapa y Agrupar en secuencia** (RF-45)
- *Propósito*: componer varias capas de video y reutilizar bloques completos.
- *Comportamiento y reglas*: las pistas **Video** y **Superposiciones** se apilan (la superior se pinta encima) con **Opacidad** y **modo de mezcla** por clip. Una **capa de ajuste** (`AdjustmentNode`) aplica efectos a todas las capas inferiores dentro de su duración. **Agrupar en secuencia** (`Ctrl+Alt+G`) convierte los clips elegidos en una secuencia anidada (`ClipSource::Scene`/`Nodes`) que se ve como un solo clip, se puede abrir con doble clic, reutilizar varias veces (intros, outros, tercios inferiores) y actualizar en un solo lugar. Los cambios se propagan a todas las instancias; «Desvincular» crea una copia independiente. Hasta 4 niveles de anidación (PROPUESTA).
- *UI*: Línea de tiempo (menú contextual → Agrupar en secuencia); Inspector → Clip (Opacidad, Mezcla).
- *Técnica*: `Sequence`, `ClipSource`, `AdjustmentNode`, `RenderGraphBuilder` (Visitor, 02 §4.1, §6.1); caché por contenido en `gafira-render`.
- *Presupuesto*: 3 capas de video 1080p + 1 ajuste a la fps del proyecto en GPU; en CPU con Copias ligeras. Vista previa sin bloquear la UI (CAL-13).
- *Aceptación*: (a) la capa superior tapa a las inferiores según su opacidad (golden); (b) editar una secuencia anidada actualiza todas sus instancias; (c) los ciclos de anidación se rechazan (ARQ-13); (d) Deshacer revierte la agrupación completa.

## B2. Fotogramas clave y Editor de curvas

**FV-14 · Fotogramas clave con Editor de curvas** (RF-46)
- *Propósito*: animar cualquier parámetro con control fino.
- *Comportamiento y reglas*: cada parámetro animable (posición, escala, rotación, **opacidad**, **recorte**, **volumen** y parámetros de efectos) tiene un interruptor de animación `◇`. Con él activo, cambiar el valor en el Cabezal crea un fotograma clave. La pestaña **Fotogramas clave** lista los parámetros del clip con una mini línea de tiempo; el **Editor de curvas** muestra valor frente a tiempo.
  - **Interpolación por segmento** (`Keyframe.out`): **Mantener** (`Hold`), **Lineal**, **Ease-in**, **Ease-out**, **Ease-in-out** y **Bézier** con dos asas (`CubicBezier x1,y1,x2,y2`).
  - Operaciones: mover, selección múltiple y marco, **escalar el tiempo** de la selección, copiar y pegar animaciones entre parámetros y clips, invertir, eliminar, ajustar al fotograma, atajos `Alt+←/→` (un fotograma) y `Alt+↑/↓` (valor fino).
  - La vista **Hoja de fotogramas** (todas las propiedades en filas) y **Editor de curvas** se alternan con `Tab`.
  - Los presets de Movimiento (FV-07) se editan aquí como cualquier otro fotograma clave.
- *UI*: Inspector → Fotogramas clave; zona inferior de la Línea de tiempo.

```text
┌─ Fotogramas clave ───────────────────────────────────── [Hoja de fotogramas] [Curvas] ────────────────┐
│ Parámetros              │  00:00    00:01    00:02    00:03    00:04                       │
│ ◆ Posición   X  120 ◇   │   ◆━━━━━━━━━━━◆━━━━━━━━━━━━━━━━━◆                              │
│ ◆ Posición   Y  340 ◇   │   ◆━━━━━━━━━━━━━━━━◆                                           │
│ ◆ Escala        100 ◇   │         ◆━━━━━━◆                                               │
│ ◇ Rotación        0 ◇   │                                                                │
│ ◆ Opacidad      100 ◇   │   ◆━━━━━◆                                                      │
├──────────────────────────────────────────────────────────────────────────────────────────┤
│ Curvas — Opacidad     ▲ 100 ┤        ╭───────●                                           │
│                             │      ╱╱                    Interpolación del segmento:     │
│                         50 ┤    ╱╱       (asas Bézier)    [Mantener][Lineal][Ease-in]     │
│                             │  ╱╱                        [Ease-out][Ease-in-out][Bézier] │
│                          0 ┼●─────────────────────► t     x1[0,42] y1[0] x2[0,58] y2[1] │
│  ⟲ Restablecer curva   ⧉ Copiar animación   ⧉ Pegar animación   ⌖ Ajustar al fotograma   │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

- *Técnica*: `Animatable<T>`, `KeyTrack<T>`, `Easing`, `Interpolate` (02 §3.4); comando `SetKeyframe`; muestreo O(log n) (ARQ-12). El Editor de curvas es un widget de `gafira-ui` (render con el pintor de egui).
- *Presupuesto*: evaluar todos los parámetros de un clip por fotograma < 1 ms; arrastrar un fotograma clave con la vista previa actualizada a 60 fps (CAL-02).
- *Aceptación*: (a) `sample` en el tiempo de cada key devuelve su valor (prueba de propiedad, ARQ-12); (b) una curva Bézier con asas (0,42; 0; 0,58; 1) coincide con la referencia CSS `ease-in-out` en ≤ 0,001; (c) copiar y pegar una animación a otro clip conserva tiempos relativos; (d) mover varios keys es una sola entrada de historial.

## B3. Máscaras

**FV-15 · Máscaras** (RF-47)
- *Propósito*: mostrar u ocultar zonas de un clip o capa.
- *Comportamiento y reglas*: panel **Máscaras** del Inspector con las formas **Rectángulo**, **Elipse**, **Polígono** y **Vector** (dibujo con nodos Bézier editables) y **Texto** (el contorno de un texto como máscara). Cada máscara tiene **Suavizado** (`feather`), **Expansión**, **Opacidad** e **Invertir**; varias máscaras se combinan (**Sumar**, **Restar**, **Intersectar**; PROPUESTA). Las máscaras son **animables**: posición, escala, rotación, suavizado y opacidad con Fotogramas clave; la animación de los vértices de un Vector requiere la extensión de la §C10 (PROPUESTA). Una máscara con **Seguimiento** (FV-17) acompaña al objeto.
- *UI*: Inspector → Máscaras; lienzo (asas de forma y nodos).

```text
┌─ Máscaras ─────────────────────────────────────────┐
│ [+ Rectángulo] [+ Elipse] [+ Polígono] [+ Vector] [+ Texto] │
│ ▸ Máscara 1  (Elipse)     ◉  Sumar ▾   [Invertir ☐]  │
│     Suavizado  ──●────  24 px   ◇                      │
│     Expansión  ───●───  0 px    ◇                      │
│     Opacidad   ─────●─ 100 %    ◇                      │
│ ▸ Máscara 2  (Vector)     ◉  Restar ▾  [Invertir ☑]   │
│ Vista: (•) Resultado  ( ) Contorno  ( ) Canal alfa     │
└────────────────────────────────────────────────────────┘
```

- *Técnica*: `Mask { source: Node | Alpha | Painted, mode, invert, feather: Animatable<f32> }` (02 §3.2); `gafira-pathops` (contornos); `gafira-render` (máscaras en el grafo).
- *Presupuesto*: dibujar y mover una máscara con la vista previa a 60 fps (CAL-02); evaluar 4 máscaras en 1080p < 4 ms en GPU.
- *Aceptación*: (a) la máscara Elipse con Invertir da el complemento exacto (golden); (b) la máscara de Texto se actualiza al editar el texto; (c) CPU y GPU coinciden en ≤ 2/255; (d) animar el suavizado mediante keys produce el valor esperado en cada fotograma.

## B4. Clave de color, Seguimiento y Estabilizar

**FV-16 · Clave de color** (RF-48)
- *Propósito*: reemplazar un fondo de color uniforme (pantalla verde o azul).
- *Comportamiento y reglas*: efecto **Clave de color** con cuentagotas sobre el video, **Tolerancia**, **Suavizado de borde**, **Supresión de derrame** (quita el reflejo del color de fondo en el sujeto), **Contraer o expandir** el borde y **Desenfoque de borde**. Tres vistas: Resultado, **Matte** (canal alfa en escala de grises) y Original. Los parámetros son animables.
- *UI*: Inspector → Clip → Efectos → Clave de color.
- *Técnica*: `Effect` + `GpuEffect` (WGSL) en `gafira-raster::effects` (02 §4.4); distancia en YCbCr/Lab con umbral suave y *despill*.
- *Presupuesto*: vista previa a 30 fps en 1080p con GPU; en CPU ≥ 15 fps con Copia ligera (PROPUESTA).
- *Aceptación*: (a) sobre un fixture de pantalla verde, el *matte* tiene IoU ≥ 0,95 respecto a la verdad (PROPUESTA); (b) la supresión de derrame reduce el tinte verde en bordes (ΔE medido) sin afectar otros colores; (c) CPU y GPU en ≤ 2/255.

**FV-17 · Seguimiento (motion tracking)** (RF-49)
- *Propósito*: pegar texto, logos o máscaras a un objeto que se mueve.
- *Comportamiento y reglas*: herramienta **Seguimiento**: se coloca un recuadro sobre el punto o la zona (con recuadro de búsqueda) y se analiza hacia delante o hacia atrás. Modos: **Posición** (un punto), **Posición y escala** y **Posición, escala y giro** (dos puntos). El resultado son **Fotogramas clave** de posición/escala/giro que se aplican a un texto, logo, imagen o máscara destino y se pueden editar. Cuando pierde el objeto, se detiene y pide corrección manual en ese fotograma. Cancelable en ≤ 100 ms.
- *UI*: Inspector → Clip → Seguimiento («Analizar», «Aplicar a…»).
- *Técnica*: algoritmo propio (plantilla piramidal con refinado de subpíxel / Lucas–Kanade) en `gafira-raster`; resultado como `KeyTrack`; ejecución en el pool `Background` (02 §6.3).
- *Presupuesto*: ≥ 10 fps de análisis en 1080p en gama media (PROPUESTA); la UI no se bloquea.
- *Aceptación*: (a) en un fixture sintético con movimiento conocido, el error medio de posición < 1 px; (b) un objeto que sale de cuadro detiene el análisis con aviso; (c) el elemento seguido se mantiene sobre el objeto en el golden de 5 s.

**FV-18 · Estabilizar** (RF-49)
- *Propósito*: reducir el temblor de la cámara.
- *Comportamiento y reglas*: botón **Estabilizar** en Inspector → Clip. Modos **Suave** (traslación) y **Completo** (traslación, giro y escala); **Intensidad** 0–100 y opción de **Recortar automáticamente** (amplía lo mínimo para ocultar bordes) o **Rellenar bordes**. El análisis calcula la trayectoria de la cámara y la suaviza (filtro gaussiano); es no destructivo (se guardan datos de transformación y se puede desactivar). Corrección de *rolling shutter*: Fase 3 (PROPUESTA).
- *UI*: Inspector → Clip → Estabilizar.
- *Técnica*: estimación de movimiento en `gafira-raster` y aplicación como `Transform2D` por fotograma en `gafira-render`.
- *Presupuesto*: análisis ≥ 10 fps en 1080p en gama media (PROPUESTA).
- *Aceptación*: (a) en un fixture con temblor sintético, la desviación de la trayectoria baja ≥ 70 % (PROPUESTA); (b) «Desactivar» devuelve el clip original sin diferencia; (c) con Recortar automáticamente no quedan bordes negros en el golden.

## B5. Corrección de color

**FV-19 · Corrección de color y Medidores** (RF-50)
- *Propósito*: igualar, corregir y dar estilo al color del video con control profesional.
- *Comportamiento y reglas*: el panel **Corrección de color** (Inspector → Clip, o sobre una capa de ajuste para toda una escena) agrupa, en orden de procesamiento, cinco módulos que se pueden activar y desactivar por separado:
  1. **Balance de blancos**: Temperatura y Matiz, cuentagotas sobre un gris neutro.
  2. **Ruedas**: **Sombras, Medios y Luces** (elevar, gamma y ganancia) más una rueda **Global**; cada rueda tiene un disco de matiz/saturación y un deslizador de luminancia; doble clic restablece.
  3. **Curvas RGB**: curvas **Maestra, Rojo, Verde y Azul** con puntos editables (interpolación monótona, hasta 16 puntos por curva); clic añade, arrastre mueve, `Supr` quita.
  4. **LUT**: carga de archivos `.cube` (1D y 3D) con **Intensidad** 0–100 y orden «antes o después» de las curvas; las LUT incluidas son propias o con licencia permisiva registrada (ARQ-75).
  5. **Saturación y tono** globales.
  - **Medidores**: **Forma de onda** (luma y RGB), **Vectorscopio**, **Histograma** y **Parada RGB** en un panel acoplable; se actualizan en vivo al mover el Cabezal o un control, calculados sobre el fotograma de vista previa a resolución reducida.
  - **Comparación antes/después**: vista dividida (vertical u horizontal, con asa arrastrable) o alternar con `\`; **Restablecer** por módulo y total.
  - Espacio de trabajo: luz lineal f32; se asume Rec. 709/sRGB con rango limitado en video; **HDR queda fuera de alcance** (PROPUESTA).

```text
┌─ Corrección de color ───────────────────────────────────┬─ Medidores ──────────────────────┐
│ ☑ Balance de blancos  Temperatura ──●─ +12   Matiz ─●── 0 ⌖ │ [Forma de onda][Vectorscopio]      │
│ ☑ Ruedas   Sombras ◎   Medios ◎   Luces ◎   Global ◎        │ [Histograma][Parada RGB]            │
│              ▮ 0         ▮ 0        ▮ 0        ▮ 0          │ 100 ┤ ▁▂▃▅▆▇▆▅▃▂▁▁▂▃                │
│ ☑ Curvas   [Maestra][Rojo][Verde][Azul]                     │  50 ┤  ▂▃▄▅▅▅▄▃▂                     │
│            ╱╱╱  (curva con puntos)                          │   0 ┴──────────────────           │
│ ☑ LUT      [ Cargar .cube… ]  Intensidad ───●── 80          │                                    │
│ Vista: [ Antes | Después ]  [ Dividida ▾ ]   ⟲ Restablecer  │ Escala: 0–100 %  ☑ Actualizar en vivo│
└─────────────────────────────────────────────────────────────┴────────────────────────────────────┘
```

- *UI*: Inspector → Corrección de color; panel acoplable Medidores.
- *Técnica*: `LutTransform` y `ColorTransform` (02 §4.10), `Effect`/`GpuEffect` de ruedas y curvas (bake a LUT 3D para GPU, `bake_lut3d`), `gafira-raster` (scopes, 02 §2.1: Fase 2).
- *Presupuesto*: aplicar el módulo completo a 1080p en vista previa a la fps del proyecto en GPU; Medidores ≥ 15 actualizaciones/s en gama media (PROPUESTA); cambio de un control < 100 ms (CAL-05).
- *Aceptación*: (a) con todos los módulos en su valor por defecto la salida es idéntica a la entrada; (b) una LUT identidad `.cube` no cambia la imagen (≤ 1/255); (c) el error ΔE2000 de una LUT cargada frente a la referencia < 1,0 (ARQ de `ColorTransform`, PROPUESTA); (d) la forma de onda de un degradado conocido coincide con la referencia analítica; (e) CPU y GPU coinciden en ≤ 2/255.

## B6. Audio avanzado

**FV-20 · Mezclador y audio avanzado** (RF-51)
- *Propósito*: mezclar y mejorar el audio como en una mesa de sonido compacta.
- *Comportamiento y reglas*: `Eq3Band` y `Pan` son procesadores del MVP en `gafira-audio` (D-22); la interfaz del Mezclador es de Fase 2. El **Mezclador** (Inspector → Mezclador, o panel inferior) muestra una tira por pista de audio, **hasta 16 visibles**, más el bus maestro. Cada tira: **fader** (−∞ a +12 dB), **panorama**, **Silenciar**, **Solo**, **medidor de picos y RMS**, y una cadena de procesadores en orden fijo: ganancia → **Ecualizador de 3 bandas** → **Puerta de ruido** → **Compresor** → **Limitador** → panorama.
  - **Ecualizador de 3 bandas**: graves (estante, 100 Hz), medios (campana, 1 kHz) y agudos (estante, 8 kHz), ±15 dB, con curva de respuesta dibujada en vivo (diseño en 02 ARQ-125).
  - **Compresor**: umbral, relación, ataque, liberación y ganancia de compensación, con medidor de reducción.
  - **Puerta de ruido** (`NoiseGate`, Fase 2): umbral, ataque, retención y liberación.
  - **Limitador**: techo (por defecto −1 dBTP) y liberación.
  - **Reducir ruido** con previsualización A/B: aprende el perfil del ruido de un fragmento seleccionado y aplica una reducción espectral de 0 a 30 dB.
  - **Nivelar** (normalización de sonoridad) y **medidor de sonoridad**: LUFS momentáneo, a corto plazo e integrado, rango de sonoridad (LRA) y pico real (dBTP) según EBU R128 / ITU-R BS.1770 (parámetros en 02 ARQ-126). Nivelar es del MVP en Inspector → Clip (FV-10, D-32); los medidores en vivo son de Fase 2. Objetivos de Nivelar: −14, −16 y −23 LUFS.

```text
┌─ Mezclador ──────────────────────────────────────────────────────────────────────────────┐
│  A1 Voz    │ A2 Música  │ A3 Efectos │  ...  hasta A16  │  Maestro                        │
│  ▮▮▮ ▮▮    │ ▮▮ ▮       │ ▮          │                  │  ▮▮▮▮ ▮▮▮                       │
│  Pan ──●── │ Pan ──●──  │ Pan ──●──  │                  │  Sonoridad  -14,2 LUFS          │
│  ┌──┐      │ ┌──┐       │ ┌──┐       │                  │  Corto plazo -13,8   LRA 6,1 LU │
│  │▒▒│ -3 dB│ │▒ │ -9 dB │ │▒▒│ -6 dB │                  │  Pico real   -1,3 dBTP          │
│  └──┘      │ └──┘       │ └──┘       │                  │  [ Nivelar a -14 LUFS ]         │
│  [S][M]    │ [S][M]     │ [S][M]     │                  │                                 │
│  ▸ Ecualizador (3 bandas)  ▸ Puerta  ▸ Compresor  ▸ Limitador  ▸ Reducir ruido             │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

- *UI*: Mezclador; Inspector → Clip (Volumen).
- *Técnica*: `AudioProcessor` + `ProcessorChain` (02 §4.9) en `gafira-audio` (sin asignaciones ni bloqueos, bloques de 512 muestras, parámetros por ring buffer `rtrb`); medidor EBU R128 propio; `NoiseGate`, `Compressor`, `Limiter`, `NoiseReducer` y `TimeStretch` son procesadores de `gafira-audio` de Fase 2 registrados en `AudioRegistry` (02 §4.9, §11.1).
- *Presupuesto*: **mezcla de 16 pistas con EQ de 3 bandas < 25 % de un núcleo a 48 kHz** (CAL-15); latencia de monitorización < 20 ms (PROPUESTA).
- *Aceptación*: (a) `bench_mix_16` dentro del umbral; (b) la guardia de asignación no detecta asignaciones en el hilo de audio con 16 pistas activas (ARQ-23); (c) la respuesta del EQ a un barrido coincide con la curva teórica, que es la respuesta digital del diseño (02 ARQ-125), en ±0,5 dB; (d) Nivelar cumple FV-10 (b); (e) con un tono a 0 dBFS, el limitador no supera el techo en más de 0,1 dB.

## B7. Subtítulos SRT y VTT

**FV-21 · SRT y VTT** (RF-52)
- *Propósito*: intercambiar subtítulos con otros programas y plataformas.
- *Comportamiento y reglas*: **Importar** SRT y WebVTT a la pista Subtítulos (codificación UTF-8, con o sin BOM; si no es UTF-8, ofrecer elegir codificación) y **exportar** a SRT o VTT en UTF-8; se pueden exportar como archivo aparte además del video. Se conservan tiempos al fotograma, saltos de línea y etiquetas básicas (negrita, cursiva) y, en VTT, las posiciones básicas; lo demás se descarta con aviso. Opción de desplazar todos los tiempos (± ms) y de dividir/combinar cuadros. Los subtítulos no se queman en el video salvo que la persona active «Incrustar en el video».
- *UI*: Panel lateral → Subtítulos → Importar, Exportar.
- *Técnica*: `gafira-captions` (SRT y WebVTT, estilos).
- *Presupuesto*: importar 1 000 cuadros < 200 ms.
- *Aceptación*: (a) un SRT de prueba importado y exportado coincide byte a byte tras normalizar saltos de línea; (b) fuzz del lector de SRT/VTT sin fallos (ARQ-21); (c) los tiempos conservan precisión de 1 ms.

## B8. Copias ligeras

**FV-22 · Copias ligeras (proxies)** (RF-53)
- *Propósito*: editar con fluidez material 4K y 6K en equipos modestos.
- *Comportamiento y reglas*: se generan **automáticamente** para clips de más de 1080p o con códecs costosos (por ejemplo HEVC de 10 bits) y se pueden crear o borrar a mano (menú contextual de Medios → «Copia ligera»). Formato: MJPEG intra a 540p o 720p, almacenada en la **caché del usuario**, fuera del `.gafira`; límite de caché configurable (por defecto 10 GB, PROPUESTA) con descarte de los menos usados. Un distintivo «Copia ligera» indica cuándo se usa; **la exportación siempre usa el original** (V-3). La generación corre en segundo plano con prioridad `Background`, con progreso y cancelación. El **motor** existe en el MVP y la **gestión manual** en Fase 2 (C-11 de 01).
- *UI*: Medios; Ajustes → Rendimiento (Copias ligeras automáticas); distintivo en el clip.
- *Técnica*: `gafira-media` (proxies, caché), `Codec` MJPEG de `gafira-codec-free`.
- *Presupuesto*: generar a ≥ 1× el tiempo real en 1080p en gama media (PROPUESTA); con Copia ligera, scrubbing ≥ 24 actualizaciones/s en 4K (CAL-14).
- *Aceptación*: (a) un proyecto con video 4K, música, voz en off, LUT, Fotogramas clave y Máscara se edita con Copias ligeras sin bloquear la UI y se exporta a 1080p con originales (criterio del PRD RF-08); (b) la exportación nunca referencia una Copia ligera (prueba de grafo); (c) borrar la caché no pierde datos del proyecto.

## B9. Cola de exportación

**FV-23 · Cola de exportación con prioridades y reintentos** (RF-34)
- *Propósito*: renderizar varios trabajos en segundo plano sin perder el control.
- *Comportamiento y reglas*: la **Cola de exportación** (botón «Cola ▾» de la Barra superior) lista los trabajos con estado (`Queued → Preparing → Rendering(p) → Encoding(p) → Finalizing → Done | Failed(e) | Cancelled`), progreso, tiempo restante estimado, prioridad (**Alta, Normal, Baja**; se reordena arrastrando), **Pausar**, **Reanudar**, **Cancelar**, **Reintentar** y «Mostrar en carpeta». Se ejecuta **un trabajo a la vez** por defecto (configurable). Los trabajos usan una **instantánea del proyecto**: se puede seguir editando o abrir otro proyecto mientras se renderiza. Los trabajos se guardan en disco y se ofrecen para reanudar tras un fallo. **Reintentos**: hasta 2 automáticos solo para errores transitorios (E/S, `DeviceLost` → CPU, códec de hardware no disponible → códec de software de la misma clase legal o alternativa libre, con aviso). **Notificación** al terminar: aviso en la app («Tu diseño está listo para compartir») y notificación del SO opcional. Cada trabajo registra ruta, configuración, duración, tamaño y errores.
- *UI*: Barra superior → Cola; panel de la cola.
- *Técnica*: `ExportService::{enqueue, cancel, list}` (02 §5.4), `ExportRequest` persistido en `export-queue.json` (`ExportJob` es la vista de una ejecución y no se persiste, 02 §6.4), `CancelToken`.
- *Presupuesto*: cancelar en ≤ 100 ms (CAL-27); la UI sigue fluida durante un render largo (CAL-35); el trabajo se escribe a archivo temporal y se renombra.
- *Aceptación*: (a) matar el proceso en medio de un trabajo y reabrir ofrece reanudarlo; (b) un error transitorio simulado dispara un reintento y registra el evento; (c) con prioridad Alta un trabajo adelanta a uno en espera; (d) la suite `export_matrix` mantiene ≥ 98 % de éxito (CAL-17).

## B10. Exportación con control técnico

**FV-24 · Exportación con control de códec, bitrate, fps, resolución, rango de color, audio y metadatos** (RF-54)
- *Propósito*: producir exactamente el archivo que exige una entrega.
- *Comportamiento y reglas*: el panel **Avanzado** de Exportar → Video expone: **códec** (los disponibles según clase legal; por defecto AV1 + Opus), **bitrate** (modo calidad constante, bitrate objetivo con VBR, o máximo), **fps** (23,976 a 60; conversión por duplicado o mezcla de fotogramas), **resolución** personalizada (múltiplos de 2), **rango de color** (limitado o completo; Rec. 709), **audio** (códec, frecuencia de 44,1 o 48 kHz, canales, bitrate de 96 a 320 kb/s) y **metadatos** (título, autor, derechos, etiquetas; opción «Quitar metadatos»). AV1: velocidad del codificador (preset 0–10) y número de hilos. H.264 por el SO: solo los parámetros que el sistema exponga (VERIFICAR por plataforma); no hay HEVC como salida. Se muestra una **estimación de tamaño y de tiempo** y se avisa de combinaciones incompatibles antes de encolar. WebM VP9: Fase 2.
- *UI*: Exportar → Video → Avanzado; guardado de **preajustes propios**.
- *Técnica*: `EncoderConfig::builder()` (02 §5.3), `Exporter::options_schema`, `Encoder`, `Muxer`.
- *Presupuesto*: validar combinaciones < 100 ms; exportar según CAL-16 en el preajuste por defecto.
- *Aceptación*: (a) el archivo exportado refleja el códec, fps, resolución y rango de color pedidos (inspección con el `Demuxer` propio); (b) una combinación inválida se rechaza antes de encolar con mensaje claro; (c) «Quitar metadatos» no deja campos de autoría en el archivo.

## B11. Relocalización de medios

**FV-25 · Relocalizar medios** (RF-55) — Fase 2
- *Propósito*: reconectar medios que cambiaron de carpeta o disco.
- *Comportamiento y reglas*: los medios se incrustan en el `.gafira` por defecto; los videos grandes (> 500 MB, PROPUESTA) pueden **vincularse** en lugar de incrustarse (`manifest.assets[...].linked { rel_path, bytes, blake3 }`: ruta relativa a la carpeta del `.gafira`, tamaño y hash BLAKE3; sin rutas absolutas, ARQ-56). **Relocalizar medios** es una función de **Fase 2** (el campo `linked` existe desde `schema` 1, 02 §7.1). Si un vínculo falta (evento `LinksMissing`), la Línea de tiempo muestra el clip en rojo con un icono y el diálogo **Buscar archivo**; al elegir una carpeta, Gafira busca por nombre y verifica por tamaño y hash y reconecta **todos** los que coincidan. La exportación se bloquea o advierte antes de generar (RF-33).
- *UI*: clip con vínculo ausente; Archivo → Relocalizar medios.
- *Técnica*: `AssetTable` con `AssetStorage::Linked`, `AssetService::relocate` (indexa la carpeta, filtra por nombre y tamaño y confirma por hash; un solo `Command` reconecta todos los coincidentes) y `ContentStore` (02 §3.2, §7.1).
- *Presupuesto*: indexar una carpeta de 1 000 archivos < 2 s (PROPUESTA).
- *Aceptación*: (a) mover la carpeta de medios y relocalizar reconecta el 100 % de los coincidentes; (b) un archivo con el mismo nombre pero distinto hash se rechaza con aviso; (c) el proyecto no guarda rutas absolutas.

---

# Parte C — Empaquetado y distribución

Plataformas únicas (ARQ-03): `x86_64-pc-windows-msvc` (Windows 10/11), `x86_64-unknown-linux-gnu` (**solo Linux x86_64**, D-05) y `aarch64-apple-darwin` (macOS 12+). Paquetes: MSI, `.deb`, `.rpm` y `.tar.gz` portable, y DMG. No hay AppImage, Flatpak, Linux aarch64 ni actualizador (D-05, D-06). El instalador contiene solo `gafira` y `gafira-cli` y los recursos propios (ARQ-72). Presupuesto: instalador ≤ 60 MB y binario ≤ 40 MB por plataforma (CAL-23, CAL-24). Perfil de compilación `dist` (02 §10.2: LTO completo, `codegen-units = 1`, `panic = "unwind"`).

## C1. Windows: instalador MSI

**FV-30 · MSI con WiX v5** (RF-56)
- *Propósito*: instalar, actualizar y desinstalar Gafira con el estándar de Windows Installer.
- *Comportamiento y reglas*:
  - **Herramienta**: WiX v5 se usa **solo en el build**; no se distribuye. Su licencia (MS-RL según 02) está en VERIFICAR antes de depender del proceso de empaquetado.
  - **Contenido**: `gafira.exe`, `gafira-cli.exe` (opcional, en el PATH como característica), `THIRD_PARTY_NOTICES.md`, licencias OFL de las fuentes y recursos propios; **CRT estático** (sin redistribuible de Visual C++).
  - **Instalación por equipo** en `C:\Program Files\Gafira` con permisos de administrador (por usuario sin administrador: PROPUESTA). `UpgradeCode` fijo y `MajorUpgrade` para actualizar en sitio sin duplicar.
  - **Accesos directos**: Menú Inicio (siempre) y Escritorio (casilla, desactivada por defecto). Entrada en «Aplicaciones instaladas» con el icono de Gafira, versión, editor y enlaces de soporte.
  - **Asociación `.gafira`** (FV-31).
  - **Desinstalación limpia**: elimina `Program Files\Gafira`, accesos directos, claves de registro y asociaciones. Los **datos de la persona** (proyectos, ajustes, copias de recuperación, biblioteca) **no se borran** por defecto; una casilla de la desinstalación («Eliminar también mis ajustes y copias de recuperación») los elimina, sin tocar los proyectos `.gafira` guardados por la persona.
  - **Firma**: Authenticode con certificado de firma de código y sello de tiempo; instalador y ejecutables firmados. La reputación en el filtro de SmartScreen puede tardar en consolidarse (VERIFICAR; certificado de validación extendida como opción).
  - **Idioma**: el instalador se ofrece en español e inglés, según el idioma del sistema (D-08, D-61).
  - Versiones mínimas: Windows 10 (build mínima por definir, VERIFICAR contra los requisitos de wgpu/Media Foundation) y Windows 11.
- *UI*: asistente del instalador (FV-40); Menú Inicio.
- *Técnica*: `xtask package windows` (WiX v5 + `signtool`); `xtask audit-bundle` (ARQ-72).
- *Presupuesto*: instalador ≤ 60 MB (CAL-23); instalación en < 60 s en SSD (PROPUESTA).
- *Aceptación*: (a) instalar, abrir un `.gafira` con doble clic y desinstalar en una máquina limpia deja 0 archivos en `Program Files\Gafira` y 0 claves de Gafira en el registro (comprobado con diff del sistema); (b) `Get-AuthenticodeSignature` reporta firma válida en el MSI y los `.exe`; (c) actualizar de la versión n−1 a n conserva ajustes y asociaciones; (d) `audit-bundle` solo lista `gafira` y `gafira-cli` como ejecutables (ARQ-72).

## C2. Asociación del tipo de archivo `.gafira`

**FV-31 · Asociación `.gafira`** (RF-57)
- *Propósito*: abrir proyectos con doble clic y reconocerlos en el explorador.
- *Comportamiento y reglas*:
  - **Tipo MIME** `application/x-gafira+zip` (el `mimetype` del contenedor, 02 §7.1); extensión `.gafira`; **icono de documento** propio (símbolo sobre una hoja, 03 §5.3); verbo «Abrir» con `gafira "%1"` (Windows), `gafira %f` (Linux) y el manejador del paquete (macOS).
  - Windows: `ProgId` y `Applications` registrados por el MSI. Linux: archivo `gafira.xml` de MIME en `shared-mime-info` y `MimeType=application/x-gafira+zip;` en el `.desktop`. macOS: `CFBundleDocumentTypes` y una UTI exportada (`<id-de-organización>.gafira.project`, el identificador depende de Q-01) conforme a `public.zip-archive`.
  - La app abre el archivo recibido por argumento y, si ya hay una instancia, lo entrega a ella (bloqueo multiinstancia, ARQ-57).
- *UI*: explorador de archivos del SO; Ajustes → General (restablecer asociación).
- *Técnica*: `gafira-platform` (argumentos, instancia única), plantillas de empaquetado de `xtask`.
- *Presupuesto*: abrir al hacer doble clic hasta la primera página interactiva < 3 s para el proyecto de 20 páginas (CAL-09).
- *Aceptación*: (a) en las tres plataformas, doble clic abre el proyecto; (b) el icono del tipo se muestra en el explorador; (c) abrir un segundo archivo con la app abierta lo abre en la misma instancia o en solo lectura según corresponda.

## C3. Linux x86_64: `.deb`, `.rpm` y `.tar.gz`

**FV-32 · AppImage — excluido (D-05)** (RF-56)
- *Decisión*: AppImage queda **fuera** del MVP. Su *runtime* enlaza libfuse con licencia LGPL-2.1, lo que obligaría a adjuntar el texto de esa licencia y una oferta de código fuente con cada copia. Tampoco hay Flatpak ni Linux aarch64. El ID se conserva porque otras specs lo citan. Esta ficha solo explica la exclusión; no hay comportamiento que implementar.
- *Aceptación*: `xtask package linux` no produce ningún AppImage y la CI no tiene *targets* Linux distintos de `x86_64-unknown-linux-gnu` (ARQ-84).

**FV-33 · Paquetes `.deb`, `.rpm` y `.tar.gz`** (RF-56)
- *Propósito*: instalación estándar en Debian, Ubuntu, Fedora, openSUSE y derivadas, y una opción portable para el resto.
- *Comportamiento y reglas*:
  - **Contenido común**: `gafira` y `gafira-cli` en `/usr/bin` (en el `.tar.gz`, en `bin/`), recursos en `/usr/share/gafira`, archivo `.desktop` con nombre y descripción en español e inglés (D-61), iconos `hicolor` (16–512 px, 03 §5.3), MIME de `.gafira` (`gafira.xml` en `shared-mime-info`, `MimeType=application/x-gafira+zip;`), `THIRD_PARTY_NOTICES.md` y los `OFL.txt` de las fuentes.
  - **`.deb`**: `gafira_<versión>_amd64.deb`, con `copyright` que recoge las licencias de terceros. **`.rpm`**: `gafira-<versión>.x86_64.rpm`. Ambos se generan con **nfpm** (herramienta solo de build; **licencia VERIFICAR** antes de depender de ella).
  - **`.tar.gz` portable**: `gafira-<versión>-linux-x86_64.tar.gz`, se descomprime y se ejecuta sin instalar; incluye un script opcional de integración con el escritorio.
  - **Dependencias del sistema** (solo bibliotecas del SO, nunca empaquetadas): por ejemplo `libc6`, `libvulkan1`, `libxkbcommon0`, `libwayland-client0`, `libasound2` (audio) y `xdg-desktop-portal` (diálogos de archivo); los nombres equivalentes en el `.rpm`. `libasound` es LGPL-2.1 y se enlaza de forma **dinámica** como dependencia del paquete, no como archivo incluido (ARQ-79, R-06 de 02, VERIFICAR con asesoría legal).
  - Se construye sobre una base con la **glibc más antigua soportada** (propuesta: la de la LTS de Ubuntu más antigua en soporte, VERIFICAR).
  - **Firma**: los paquetes se firman por plataforma (firma de `.deb`/`.rpm`, VERIFICAR herramienta). No hay repositorio ni actualizador (D-06): la persona descarga cada versión desde la web.
- *UI*: gestor de paquetes; Menú de aplicaciones.
- *Técnica*: `xtask package linux` (nfpm para `.deb` y `.rpm`; `tar` para el portable); `xtask audit-bundle` (ARQ-72, ARQ-79, ARQ-84).
- *Presupuesto*: ≤ 60 MB por paquete (CAL-23).
- *Aceptación*: (a) `xtask package linux` produce exactamente `.deb`, `.rpm` y `.tar.gz` para `x86_64` y ningún otro artefacto (ARQ-84); (b) `dpkg -i` y `rpm -i` instalan y `dpkg -r`/`purge` y `rpm -e` eliminan todos los archivos del paquete; (c) `audit-bundle` no encuentra FFmpeg, ejecutables de terceros ni `libasound` u otra `.so` LGPL dentro de ningún paquete; (d) el icono y la asociación `.gafira` aparecen tras instalar; (e) el `.tar.gz` se ejecuta sin instalar en dos distribuciones de referencia (VERIFICAR cuáles); (f) el diálogo de abrir archivos funciona en X11 y Wayland.

## C4. macOS aarch64: DMG

**FV-34 · DMG, firma, notarización y entitlements** (RF-56)
- *Propósito*: distribuir Gafira de forma que macOS la abra sin avisos de seguridad.
- *Comportamiento y reglas*:
  - **Paquete**: `Gafira.app` solo `aarch64`, `LSMinimumSystemVersion` 12.0, icono `.icns` (03 §5.3), tipos de documento de `.gafira`, categoría Gráficos y diseño.
  - **DMG** con ventana de 660×400 pt, fondo `#FAFAF7` con el logo apilado (≥ 120 px) y el alias a Aplicaciones.
  - **Firma** con certificado *Developer ID Application*, *hardened runtime* y sello de tiempo (`codesign --options runtime --timestamp`); el DMG también va firmado.
  - **Notarización** con `notarytool` y *staple* del ticket; verificación con `spctl --assess` y `stapler validate`. Requiere cuenta del programa de desarrolladores de Apple (coste y titularidad: Q-01).
  - **Entitlements mínimos**: por defecto **ninguno** que relaje el *hardened runtime* (sin JIT ni validación de bibliotecas desactivada: `wasmi` es un intérprete, no JIT). Cuando exista la **voz en off** (Fase 2): `com.apple.security.device.audio-input` y `NSMicrophoneUsageDescription` con un texto claro en el tono de marca. Sin App Sandbox en la distribución directa (PROPUESTA; decidir si se publica en una tienda: fuera de alcance).
  - **Sin red salvo la licencia**: no existe `update-check` (D-06); como la distribución directa no usa App Sandbox, la conexión de la licencia no exige un entitlement de red adicional (VERIFICAR con `codesign -d --entitlements`).
- *UI*: ventana del DMG; primer arranque (Gatekeeper sin avisos).
- *Técnica*: `xtask package dmg`, `codesign`, `notarytool`, `stapler` (herramientas del SO y de Apple en el build).
- *Presupuesto*: DMG ≤ 60 MB (CAL-23).
- *Aceptación*: (a) en una Mac M1 limpia, abrir el DMG y ejecutar Gafira no muestra avisos de desarrollador no identificado (`spctl` acepta); (b) `codesign --verify --deep --strict` pasa; (c) `codesign -d --entitlements` lista solo los previstos; (d) `lipo -archs` devuelve únicamente `arm64`.

## C5. Integración continua en los tres sistemas

**FV-35 · CI** (RF-56, ARQ-80)
- *Propósito*: garantizar que cada cambio compila, pasa pruebas y empaqueta en las tres plataformas.
- *Comportamiento y reglas*: matriz exacta: Windows Server 2022 (`x86_64-pc-windows-msvc`), Ubuntu LTS (`x86_64-unknown-linux-gnu`) y macOS 14 arm64 (`aarch64-apple-darwin`) (02 §10.1). **Por cada PR**: `fmt --check`, `clippy -D warnings`, `check-layers`, `cargo deny check licenses bans sources advisories`, pruebas, fuzz corto de los objetivos afectados y *benchmarks* de humo; build `free-only` y build por defecto. **Nocturno**: *benchmarks* completos en la máquina de referencia (*self-hosted*), fuzz largo (8 h por semana) y paquetes firmados de prueba. **Release** (por etiqueta): paquetes `dist`, firma y notarización con secretos protegidos solo en ese flujo, `audit-bundle`, SBOM, avisos de terceros, sumas de verificación SHA-256 y prueba de instalación y desinstalación en máquina limpia. La CI es un servicio de construcción, no un servicio de Gafira: la app no depende de él en ejecución.
- *UI*: no aplica.
- *Técnica*: `xtask` (`check-layers`, `package`, `gen-icons`, `audit-bundle`, `size`); `perf-budgets.toml` (ARQ-40).
- *Presupuesto*: PR verde en < 45 min (PROPUESTA).
- *Aceptación*: (a) la matriz contiene exactamente los 3 *targets* (ARQ-03); (b) un PR no se puede fusionar con CI roja (ARQ-80); (c) un release sin firma o sin `audit-bundle` verde falla.

## C6. Avisos de terceros, SBOM y auditoría de licencias

**FV-36 · `THIRD_PARTY_NOTICES.md` y SBOM** (RF-58)
- *Propósito*: cumplir las obligaciones de atribución y poder demostrar qué contiene cada versión.
- *Comportamiento y reglas*: en cada release se genera **`THIRD_PARTY_NOTICES.md`** (único nombre del archivo de avisos en todas las specs y en el instalador; la lista de dependencias de Rust con nombre, versión, licencia y texto completo, y las licencias OFL de las fuentes y de los assets que lo exijan); se incluye en el instalador y se muestra en **Ajustes → Acerca de → Licencias**. Se produce además un **inventario de software (SBOM)** en formato CycloneDX o SPDX (VERIFICAR herramienta y licencia, solo de build), por plataforma y por conjunto de *features*, firmado y adjunto a la versión; el SBOM del build `free-only` se publica por separado. La herramienta de generación es de uso solo en build (02 §9.1).
- *UI*: Ajustes → Acerca de → Licencias.
- *Técnica*: `cargo about` o equivalente (VERIFICAR licencia), `xtask audit-bundle`; ARQ-73.
- *Presupuesto*: generación < 2 min en CI.
- *Aceptación*: (a) el archivo de avisos contiene el 100 % de las dependencias resueltas por `cargo metadata` para ese build; (b) el instalador incluye el archivo y «Acerca de» lo muestra; (c) el SBOM se valida contra el esquema del formato elegido.

**FV-37 · `cargo-deny` y política de dependencias en release** (RF-58)
- *Propósito*: impedir que entre una dependencia no permitida.
- *Comportamiento y reglas*: `deny.toml` de 02 §9.1: lista de licencias permitidas (MIT, Apache-2.0 incluida la excepción LLVM, BSD-2/3, ISC, Zlib, Unicode-3.0 y Unicode-DFS-2016, BSL-1.0, CC0-1.0, MIT-0, 0BSD), **prohibidas** GPL, LGPL, AGPL, SSPL, licencias no comerciales y desconocidas, `MPL-2.0` denegada por defecto (excepción solo con ADR y aislamiento), **sin dependencias git**, y lista de crates vetados (`ffmpeg-sidecar`, `ffmpeg-next`, `ffmpeg-sys-next`, `symphonia`, `imbl`, `im`, `rpds`, `gstreamer`, `openssl-sys` y, por D-02 y D-07, los encoders `openh264`, `x264` y `mp3lame-encoder`; nombres VERIFICAR al implementar). Se añade el control de **avisos de seguridad** (RustSec) en CI. Todo crate marcado VERIFICAR en 02 §9.2 (incluido el decodificador MP3, el cliente HTTP y `ring`) se verifica con `cargo-deny` y se resuelve (licencia confirmada o alternativa) antes de la primera versión comercial.
- *UI*: no aplica.
- *Técnica*: `cargo deny check licenses bans sources advisories`.
- *Presupuesto*: < 2 min por ejecución.
- *Aceptación*: (a) un crate con licencia GPL de prueba hace fallar la CI; (b) el listado de licencias del build `dist` solo contiene licencias de la lista; (c) 0 dependencias git (ARQ-71).

## C7. Reglas de assets y fuentes OFL

**FV-38 · Assets y fuentes** (RF-58)
- *Propósito*: garantizar que todo contenido incluido puede venderse.
- *Comportamiento y reglas*:
  - **Registro**: cada archivo de `assets/` tiene una entrada en `assets/LICENSES.toml` con ruta, hash BLAKE3, licencia (identificador SPDX o «Propia»), autoría, origen, si exige atribución y si fue modificado (ARQ-75). Una entrada puede añadir campos: una fuente de prueba, con `modificado` verdadero, añade repositorio, commit y comando de subconjunto; `assets/splash_screen.jpg` añade los de D-09 y su licencia es «Propia», por la cesión que recoge D-09. Sin entrada, el archivo **no se empaqueta** ni se lista en la app.
  - **Licencias admitidas para contenido**: propia (obra creada para Gafira con cesión de derechos), CC0, MIT, Apache-2.0, BSD, ISC, Zlib. **No se admiten** licencias con atribución obligatoria no resuelta (por ejemplo, CC BY), «no comercial» ni «sin derivados» salvo decisión explícita de la persona titular (PROPUESTA).
  - **Fuentes**: solo **OFL-1.1** o permisivas; Inter y Manrope (marca) se embeben con su `OFL.txt` y aviso de copyright; las fuentes OFL se pueden empaquetar con el programa pero **no venderse por separado**; respetar los nombres reservados de las fuentes. Se desactivan las fuentes por defecto de egui (ARQ-62). Las fuentes del sistema no se redistribuyen.
  - **Música y sonidos**: solo propios o CC0 (el audio es el contenido de mayor riesgo; cada pista con comprobante de origen).
  - **Plantillas e ilustraciones**: propias o con licencia permisiva registrada; los textos de ejemplo y fotos de las plantillas también se registran.
  - **Marca Gafira**: el logo y los nombres del glosario son de la titularidad de Gafira; los assets de otros proyectos (por ejemplo, la marca de las apps de referencia) **no** se reutilizan.
  - **Modelos de ML**: no se incluyen en el instalador base (ARQ-76).
- *UI*: Acerca de → Licencias (contenido incluido).
- *Técnica*: `xtask check-licenses` (ARQ-75), `audit-bundle` (ARQ-76).
- *Presupuesto*: comprobación < 1 min.
- *Aceptación*: (a) `LICENSES.toml` cubre el 100 % de `assets/` (ARQ-75); (b) un archivo sin entrada hace fallar `xtask`; (c) `audit-bundle` no encuentra `.onnx`, `.safetensors` ni pesos (ARQ-76); (d) los `OFL.txt` de Inter y Manrope están en el instalador.

## C8. Sin actualizador (D-06)

**FV-39 · Sin comprobación de versiones nuevas** (RF-59, cancelado) — D-06
- *Decisión*: la app **no comprueba** versiones nuevas y no contiene actualizador ni firma de manifiestos de actualización. La persona descarga cada versión desde la web. Desaparece la feature `update-check`. El ID se conserva porque otras specs lo citan.
- *Qué sí se firma*: los paquetes por plataforma (Authenticode, notarización de Apple, firma de `.deb`/`.rpm`) y se publican con su suma SHA-256 (FV-35).
- *Red*: la única conexión de la app es la de la licencia (ARQ-04, ARQ-91).
- *Aceptación*: no existe código ni feature `update-check` (ARQ-78); la prueba de red de ARQ-91 no registra conexiones fuera de las de licencia; Ajustes → Privacidad no contiene ninguna opción de actualizaciones.

## C9. Pantalla del instalador con la marca

**FV-40 · Instalador con la marca Gafira** (RF-56)
- *Propósito*: que la primera impresión siga la marca.
- *Comportamiento y reglas*: el asistente usa fondo `#FAFAF7`, el logo horizontal de Gafira (≥ 160 px, área de protección X respetada), botón principal **coral con texto grafito** y texto grafito; sin degradados ni marcas de agua. Tipografía: la del sistema (los instaladores nativos no incrustan fuentes; Inter y Manrope se usan en la app, no en el MSI). Medidas del MSI: banner 493×58 px y fondo de diálogo 493×312 px. Contraste AA en todos los textos (03 §4). Flujo y textos (tono de marca):

```text
┌─ Instalar Gafira ──────────────────────────────────────────┐
│ [logo Gafira]                                              │
│  Haz visibles tus ideas.                                   │
│  Un espacio creativo donde las ideas toman forma.          │
│                                    [ Siguiente ]  [Cancelar]│
├────────────────────────────────────────────────────────────┤
│ 1 Bienvenida → 2 Licencia → 3 Carpeta → 4 Opciones →       │
│ 5 Instalando → 6 Listo                                     │
│ Opciones: [✓] Acceso directo en el Menú Inicio             │
│           [ ] Acceso directo en el Escritorio              │
│           [✓] Abrir archivos .gafira con Gafira            │
│ Listo: «Gafira está lista.»        [ Empezar a diseñar ]   │
└────────────────────────────────────────────────────────────┘
```

  - Pantalla del acuerdo de licencia con el EULA basado en el de Tracewright (ver spec 06, pendiente) y enlace a **`THIRD_PARTY_NOTICES.md`**.
  - DMG: fondo `#FAFAF7`, logo apilado y flecha hacia Aplicaciones. `.deb`, `.rpm` y `.tar.gz`: sin asistente; el primer arranque muestra la Pantalla de arranque y después Inicio.
- *UI*: asistente del MSI; ventana del DMG.
- *Técnica*: temas de WiX (`WixUI` personalizado) y plantilla del DMG en `xtask`.
- *Presupuesto*: sin impacto en el tamaño (< 1 MB de recursos gráficos).
- *Aceptación*: (a) captura del asistente: 1 sola acción coral por pantalla (DIS-15); (b) contraste ≥ 4,5:1 en cada texto; (c) el logo mide ≥ 160 px y no está deformado ni recoloreado (DIS-70).

## C10. Extensiones del modelo y de 02 requeridas por esta especificación

| Elemento | Necesidad | Propuesta |
|---|---|---|
| `Transition` | FV-06: las transiciones tienen dos entradas | **Incorporado en 02 §4.13**: trait `Transition`, `GpuTransition`, `TransitionInstance` y `TransitionRegistry` |
| Animación de vértices de un Vector | FV-15 | `Interpolate` para rutas de igual topología (PROPUESTA, Fase 2) |
| Medios vinculados | FV-25 | **Incorporado en 02 §7.1**: `manifest.assets[...].linked { rel_path, bytes, blake3 }` y evento `LinksMissing` |
| *Time-stretch* y mantener tono | FV-04 | **Incorporado en 02 §4.9**: `TimeStretch` (Fase 2) |
| Puerta de ruido | FV-20 | **Incorporado en 02 §4.9**: `NoiseGate` (Fase 2) |
| `TrackKind::Overlay` para Texto | FV-01 | Convención de interfaz; sin cambio de tipo |
| Instalador | FV-30, FV-33, FV-34 | Soporte de `xtask package` para MSI, `.deb`, `.rpm`, `.tar.gz` y DMG (sin AppImage, D-05) |

## C11. Lista de comprobación previa a vender

Cada punto tiene propietario y evidencia. Ninguna versión comercial se publica con un punto abierto de las categorías de licencias, patentes, marcas o documentos. Esta lista pertenece a la **fase final «Licencia y distribución comercial»** (01 §5.1) y no bloquea el MVP técnico.

| # | Categoría | Comprobación | Evidencia | Resp. |
|---|---|---|---|---|
| 1 | Licencias | `cargo deny check licenses bans sources advisories` verde en el build de release; 0 GPL/LGPL/AGPL/SSPL/no comerciales; 0 dependencias git | Informe de CI | Ingeniería |
| 2 | Licencias | Verificación con `cargo-deny` de todas las licencias marcadas VERIFICAR en 02 §9.2 (vello, parley, krilla, linesweeper, cpal, rav1e, wasmi, decodificador MP3, cliente HTTP, `ring`…) y de las herramientas de build (nfpm, WiX, `cargo about`) | Tabla actualizada | Ingeniería |
| 3 | Licencias | `cpal`/`libasound` (LGPL-2.1, enlace dinámico, no empaquetada) evaluado con asesoría legal; alternativa decidida (R-06 de 02) | Dictamen | Legal |
| 4 | Licencias | `THIRD_PARTY_NOTICES.md` completo en el instalador y en «Acerca de» | Archivo + captura | Ingeniería |
| 5 | Licencias | `assets/LICENSES.toml` cubre el 100 % del contenido; música y sonidos con comprobante de origen | `xtask check-licenses` | Contenido |
| 6 | Licencias | OFL.txt de Inter y Manrope incluidos; copyright y nombres reservados respetados | Instalador | Marca |
| 7 | Licencias | Sin ejecutables de terceros ni FFmpeg; sin pesos de ML; sin `.so` LGPL en ningún paquete | `audit-bundle` | Ingeniería |
| 8 | Patentes | H.264/AAC solo vía API del sistema, sin encoders propios de H.264, HEVC ni AAC; dictamen sobre si cubre al distribuidor en cada mercado; revisar al caducar la última patente de H.264 en EE. UU. (29 de noviembre de 2027, VERIFICAR) | Dictamen | Legal |
| 9 | Patentes | AV1 (patentes de AOMedia; pool de Sisvel y demanda de Dolby contra Snap, riesgo bajo pero no nulo), Opus, Vorbis, FLAC: revisión de obligaciones | Memo legal | Legal |
| 10 | Patentes | Importar HEVC y HEIC solo con el decodificador del SO (D-59, cierra Q-17); MP3 solo importación con decodificador de licencia permisiva | Registro de decisiones | Legal |
| 11 | Patentes | Build `free-only` verificado de punta a punta (AV1 + Opus) | CI | Ingeniería |
| 12 | Marcas | Búsqueda de «Gafira» y «Gefira» en USPTO, TMview y WIPO Global Brand Database, con resultado registrado antes de publicar (D-04; lista de 01 §12.4) | Informe con capturas | Titular |
| 13 | Marcas | Nombres de interfaz del glosario (aprobados en D-04) sin coincidencias con marcas ajenas | Informe | Titular |
| 14 | Marcas | Ningún asset ni nombre de otros productos en el instalador o la documentación | Revisión de texto | Marca |
| 15 | Firma | Certificado de firma de código de Windows, Developer ID de Apple y claves de firma de paquetes Linux disponibles y custodiados (sin claves de actualización: D-06) | Registro de claves | Producto |
| 16 | Documentos | EULA basado en el contrato de Tracewright (`LICENSE` en inglés y `LICENCIA.md` en español, prevalece el español) con las tres adaptaciones de D-01: nombre del producto, «Sus resultados» (diseños, imágenes y videos del usuario) y permiso de uso de plantillas, fuentes y recursos incluidos | Documentos | Titular |
| 17 | Documentos | `PRIVACY.md` de Gafira que declara el identificador de dispositivo enviado al activar; no puede copiar la frase «no transmite datos» (D-01) | Documento | Titular |
| 18 | Licencia | Activación contra el `license-hub` (servicio externo ya existente, reutilizado tal cual) integrada y probada con `gafira-license`. Licencia y activación: ver spec 06 (pendiente, se redacta al final) | Prueba E2E de ARQ-91 | Ingeniería |
| 19 | Calidad | Presupuestos de rendimiento y de tamaño en verde; `export_matrix` ≥ 98 %; 500 *kills* recuperados | Informe nocturno | Calidad |
| 20 | Instalación | Instalar, abrir un `.gafira` y desinstalar en máquinas limpias de las 3 plataformas (MSI, `.deb`, `.rpm`, `.tar.gz`, DMG) | Registro de pruebas | Calidad |
| 21 | Distribución | Sumas SHA-256 y SBOM publicados; paquetes firmados y notarizados | Release | Ingeniería |
| 22 | Accesibilidad | Contraste AA, teclado completo y AccessKit comprobados (DIS-10, DIS-30..37) | Informe | Diseño |

## C12. Extensión y lotes (Fase 3)

**FV-42 · Plugins** (RF-60) — Fase 3
- *Propósito*: ampliar Gafira sin tocar el núcleo.
- *Comportamiento y reglas*: pestaña **Extensiones** (instalar, activar, desactivar, quitar). Un plugin es un paquete firmado con manifiesto y **capacidades declaradas** (sin red; disco solo mediante permisos que concede la persona); sus contribuciones (efectos solo CPU, importadores, exportadores, proveedores de recursos y comandos) se envuelven en adaptadores de los traits nativos. Un plugin con bucle infinito se interrumpe en ≤ 100 ms por combustible y se desactiva sin afectar al documento.
- *UI*: Panel lateral → Extensiones; Ajustes → Extensiones.
- *Técnica*: `PluginHost` con `wasmi` (VERIFICAR licencia; 02 §4.12), `gafira-plugin`.
- *Presupuesto*: cargar un plugin < 200 ms (PROPUESTA).
- *Aceptación*: ARQ-24 (≤ 100 ms), ARQ-20 (el demo de extensión compila), acceso no concedido devuelve error.

**FV-43 · Lotes con datos de un CSV** (RF-61) — Fase 3
- *Propósito*: generar muchas piezas a partir de un diseño y una tabla.
- *Comportamiento y reglas*: `gafira-cli batch plantilla.gafira --data datos.csv --format png --out carpeta/` sustituye variables de contenido (`{{nombre}}`, `{{imagen}}`) en textos e imágenes y exporta una pieza por fila; también desde la app (Archivo → Exportar por lotes). Valida columnas y recursos antes de empezar y escribe un informe por fila.
- *UI*: Archivo → Exportar por lotes; `gafira-cli`.
- *Técnica*: `gafira-cli` (02 §2.1), `ExportService`, `Exporter`.
- *Presupuesto*: 100 piezas PNG de 1080p < 3 min en gama media (PROPUESTA).
- *Aceptación*: (a) una fila inválida se informa y no detiene el lote; (b) los resultados coinciden con la exportación manual de la misma pieza (≤ 1/255); (c) sin red.

## C13. Trazabilidad RF → función y preguntas abiertas

| RF | Funciones | RF | Funciones |
|---|---|---|---|
| RF-33 | FV-12 | RF-47 | FV-15 |
| RF-34 | FV-12, FV-23 | RF-48 | FV-16 |
| RF-36 | FV-01 | RF-49 | FV-17, FV-18 |
| RF-37 | FV-02 | RF-50 | FV-19 |
| RF-38 | FV-03, FV-04 | RF-51 | FV-20 |
| RF-39 | FV-05, FV-06 | RF-52 | FV-21 |
| RF-40 | FV-07 | RF-53 | FV-22 |
| RF-41 | FV-08 | RF-54 | FV-24 |
| RF-42 | FV-09, FV-10 | RF-55 | FV-25 |
| RF-43 | FV-11 | RF-56 | FV-30, FV-32, FV-33, FV-34, FV-35, FV-40 |
| RF-44 | FV-12 | RF-57 | FV-31 |
| RF-45 | FV-13 | RF-58 | FV-36, FV-37, FV-38 |
| RF-46 | FV-14 | RF-59 (cancelado) | FV-39 |
| RF-60 | FV-42 | RF-61 | FV-43 |
| RF-62 | spec 04 FD-62 | RF-63 | spec 06 (pendiente) |

| ID | Pregunta o riesgo | Estado |
|---|---|---|
| Q-02, Q-03, Q-04, Q-05, Q-06, Q-09, Q-12, Q-13 | Códecs vía SO, Linux, actualizaciones, MP3, activación y AVI | Cerradas (D-01, D-02, D-05, D-06, D-07; 01 §12.2) |
| Q-17 | Importar HEVC y HEIC vía API del sistema | Cerrada: D-59 (solo importación con el decodificador del SO; nunca salida HEVC) |
| Q-15 | Voz en off y marcadores de compás en Fase 2 | Abierta |
| Q-19 | Prueba gratuita | Cerrada: D-12 (no hay prueba gratuita; modo sin licencia) |
| Q-20 | Modo sin licencia válida | D-12 sustituye la propuesta de solo lectura; la lista de funciones bloqueadas se define en la spec 06 |
| R-03 (02) | AV1 por software ≥ 10 fps a 1080p (ahora es la salida por defecto) | *Spike* en la semana 1 |
| R-06 (02) | `cpal` y `libasound` (LGPL, dinámico) | VERIFICAR, legal |
| R-F | Licencia de WiX v5, nfpm y de las herramientas de empaquetado | VERIFICAR |
