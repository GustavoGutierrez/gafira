# Decisiones de producto de Gafira

Registro de las decisiones del titular que resuelven las preguntas abiertas de las especificaciones 01 a 05. Fecha: 24 de octubre de 2026. Este documento prevalece sobre cualquier texto contrario de 01 a 05 hasta que estas se actualicen.

Aviso: las referencias legales de este documento son lectura técnica de fuentes públicas, no asesoría legal. Las cifras y fechas marcadas VERIFICAR no se comprobaron en una fuente primaria.

## D-01 Licencia, modelo comercial y activación (Q-01, Q-11, Q-12)

- **Licencia**: propietaria, código cerrado. Se reutiliza el contrato de Tracewright (`LICENSE` en inglés y `LICENCIA.md` en español, prevalece el español) con tres adaptaciones:
  1. Nombre del producto: Gafira.
  2. Cláusula «Sus resultados»: el usuario es titular de los diseños, imágenes y videos que cree, no solo de archivos vectoriales.
  3. Cláusula nueva: permiso para usar las plantillas, fuentes y recursos incluidos dentro de las creaciones del usuario.
- **Modelo**: todo de pago, ~~con prueba gratuita (igual que Tracewright)~~ (**sustituido en parte por D-12**: no hay prueba gratuita; se usa el modo sin licencia). Esto **sustituye** el principio del PRD «sin funciones esenciales retenidas tras un plan de pago». ~~La duración y los límites de la prueba están por definir.~~ (Ya no aplica: ver D-12.)
- **Activación**: se reutilizan la crate `tracer-license` y el servicio `license-hub` (Cloudflare Workers + D1, proveedor Gumroad o clave propia, de 1 a 10 dispositivos por licencia, token Ed25519 verificable sin conexión con clave pública fija en el ejecutable).
- **Actualización del 24-oct-2026 (titular)**: el `license-hub` **ya está desplegado** y se reutiliza tal cual; no se construye otro servidor de licencias. Su README está desactualizado y lo corregirá el titular fuera de estas specs. El diseño detallado de licencia y activación **se escribe en una spec 06 al final** y queda fuera de las primeras fases de desarrollo.
- **Consecuencias**:
  - «Local» significa que la app trabaja sin conexión. La única conexión permitida es activar, validar o desactivar la licencia.
  - `PRIVACY.md` de Gafira debe declarar el identificador de dispositivo que se envía al activar. No puede copiar la frase «no transmite datos».
  - Antes de vender, completar la lista de seguridad del `license-hub` y desplegarlo (su README indica que no está desplegado).
  - Los módulos «avanzados de pago» de las specs pasan a ser parte de una única licencia.

## D-02 Códecs y patentes (Q-02, Q-03, Q-09)

- **Exportación por defecto**: WebM y MP4 con AV1 + Opus (libres de regalías).
- **H.264/AAC (MP4)**: solo mediante las API del sistema: Media Foundation (Windows), VideoToolbox (macOS) y VA-API en Linux cuando el equipo la tenga. Si no existe, Gafira avisa y propone AV1.
- **Prohibido**: encoders propios H.264, HEVC o AAC en el binario; HEVC; salida MP3. La feature `encumbered-sw` permanece desactivada.
- **Motivo (fuentes públicas)**: el pool H.264 (Via LA) cubre productos que incorporan encoders o decoders; una app que llama al sistema operativo no distribuye uno. AV1 es libre de regalías para las patentes de los miembros de AOMedia; existe un pool de Sisvel (dispositivos de consumo) y una demanda de Dolby contra Snap (marzo de 2026), por lo que el riesgo es bajo pero no nulo.
- **Revisar**: la última patente de H.264 en EE. UU. caduca el 29 de noviembre de 2027 según fuentes secundarias (VERIFICAR).

## D-03 Hardware de referencia y límites (Q-07)

- Windows y Linux: Intel i5 de 10.ª generación o Ryzen 5 3600, 8 GB de RAM, GPU integrada. Las cifras con GPU se miden aparte en una GTX 1650 de 4 GB.
- macOS: MacBook Air M1 de 8 GB.
- Límites: instalador ≤ 60 MB; binario ≤ 40 MB; RAM en reposo ≤ 300 MB y ≤ 1,5 GB con 20 páginas; scrubbing ≥ 24 actualizaciones por segundo.
- Las demás cifras propuestas en 02 pasan de PROPUESTA a **fijadas**.

## D-04 Nombres de interfaz y nombre Gafira (Q-16)

- El glosario de la spec 03 (sección 2) queda **aprobado** sin cambios.
- **Tarea del titular (sin abogado)**: buscar «Gafira» (y «Gefira», por parecido) en USPTO, TMview (UE) y WIPO Global Brand Database, y registrar el resultado antes de publicar. Una búsqueda web general encontró una empresa suiza de TI llamada «Gefira», con otra ortografía.
- **Búsqueda preliminar (24 de octubre de 2026)** (no es asesoría legal): fuentes consultadas: USPTO, TMview (incluidos SIC, IMPI, INPI AR, OEPM, Chile, Brasil y Portugal), WIPO Global Brand Database y comprobaciones web, de tiendas de apps, de GitHub y de dominios.
  - **Resultado**: no hay una marca vigente idéntica o casi idéntica a «Gafira» o «Gefira» en las clases 9, 41 y 42. Riesgo **bajo-medio**, por lo siguiente:
    - GAFIRA idéntica registrada en el Benelux (BOIP 1539112, clases 8 y 11, herramientas para el cabello, 2025-12-16).
    - AFIRA en EE. UU. (serie 99570447, pendiente, clases 9 y 42, software de cumplimiento comercial) y afira en Brasil (INPI 942171330, pendiente, clase 9): una letra de diferencia.
    - GAIRA en Colombia (SIC, clases 9, 41 y 42, registrada).
    - «Gefira» sin registro: empresas de TI en Suiza y EE. UU.
    - El titular registró gafira.com el 2026-10-24 (Cloudflare) para alojar la página de presentación del producto cuando exista la interfaz; gafira.app, .io, .co y .dev parecían libres (el titular puede reservarlos más adelante).
    - Connotación: en árabe, غافرة significa «la que perdona» (connotación religiosa).
  - **Siguiente paso**: ~~elegir el primer país de registro y~~ encargar a un abogado una búsqueda de disponibilidad que incluya AFIRA, afira y GAIRA. La elección del primer país de registro se aplaza (D-84).

## D-05 Linux (Q-04, Q-05)

- Arquitectura: solo x86_64.
- Paquetes: `.deb`, `.rpm` y `.tar.gz` portable.
- Quedan **fuera** del MVP: AppImage (su runtime enlaza libfuse con LGPL-2.1, lo que exigiría adjuntar el texto de licencia y una oferta de código fuente), Flatpak y aarch64.

## D-06 Actualizaciones (Q-06)

- La app **no comprueba** versiones nuevas. El usuario descarga cada versión desde la web.
- Desaparecen la feature `update-check` y la firma de manifiestos de actualización.
- La activación de licencia es la única conexión de red de la app (ver D-01).

## D-07 Formatos de audio, video y presentación (Q-08, Q-09, Q-13)

- **MP3**: solo importación, con decodificador permisivo (VERIFICAR licencia). Sin exportación (el codificador habitual, LAME, es LGPL). Las patentes centrales de MP3 caducaron (UE en 2012; EE. UU. en abril de 2017).
- **AVI**: importación en Fase 2, solo con códecs libres (MJPEG, sin comprimir, PCM). Sin exportación.
- **PPTX**: exportación en Fase 2, una diapositiva por página como imagen de alta resolución, sin objetos editables.

## D-08 Idioma del producto (Q-14)

- Gafira se publica en **español e inglés**.
- **Idioma por defecto según el sistema**: cualquier variante de español (`es`, `es-MX`, `es-CO`, `es-ES`…) usa español; inglés (`en`, `en-US`, `en-GB`…) usa inglés; **cualquier otro idioma usa inglés**.
- El usuario puede cambiarlo a mano en Ajustes → General.
- **Consecuencias**:
  - El glosario de interfaz de 03 §2 necesita su versión inglesa con correspondencia 1:1 (DIS-01 se amplía a los dos idiomas).
  - El microcopy y el eslogan tienen versión inglesa («Bring your ideas to life.»).
  - Hace falta un sistema de traducción: catálogos de mensajes con claves, plurales, formatos de fecha y número. `CommandLabel` ya es una clave i18n en 02. Se sugiere una crate `gafira-i18n`, con licencia permisiva a verificar (no usar herramientas GPL como gettext).
  - La detección del idioma del sistema va por plataforma en `gafira-platform` (Windows, macOS y variables de entorno de locale en Linux), con inglés como respaldo.
  - La documentación de las specs sigue en español.
- Aplicado a la spec 02 (nuevo crate y requisito), 03 (glosario inglés, Ajustes) y 01 (cierra Q-14) cuando se actualicen las specs.

## D-09 Pantalla de arranque (splash)

Fecha: 24 de octubre de 2026.

- **Qué es**: una pantalla breve que aparece al iniciar la aplicación, con la marca y un indicador de carga. Su nombre de interfaz es **«Pantalla de arranque»** (**FIJADA**, aprobada por el titular el 24 de octubre de 2026; evita chocar con «Inicio», que es otra pantalla).
- **Imagen**: `assets/splash_screen.jpg` (JPEG de 1672×941 px, 296 847 bytes; ilustración de un guacamayo, montañas y mar en coral, grafito y dorado). Va **incrustada en el binario** y no depende de rutas del usuario.
- **Imagen reemplazable**: se cambia sustituyendo ese archivo por otro con el mismo nombre base (`splash_screen.jpg` o `splash_screen.png`) y recompilando, sin tocar código. La build valida el archivo y falla si falta, si hay dos o si está corrupto (02 ARQ-67, ARQ-69).
- **Cifras**: **FIJADAS** por el titular el 24 de octubre de 2026: máximo de 5 s, primer frame en menos de 300 ms y ventana de 800×450.
- **Diseño**: la imagen cubre la ventana de fondo (recorte «cubrir», regla en 03 §7.10). El **logo horizontal `gafira-logo-primary.svg`** (logotipo con el isotipo y el texto «Gafira», `docs/manuals/marca/assets/`) y el eslogan **«Haz visibles tus ideas.»** (en inglés **«Bring your ideas to life.»**, según el idioma resuelto por D-08) van sobre un **panel de blanco cálido `#FAFAF7`** en la parte inferior. Una **barra de carga fina** usa el coral `#FF6B57` y refleja el progreso real del arranque.
- **Comportamiento**: se cierra cuando la app termina de cargar lo necesario para mostrar Inicio, con una **duración mínima de 1,2 s**. Máximo: 5 s (**FIJADA**); si la carga se alarga, la pantalla avisa de que sigue cargando y, si el arranque se detiene, muestra un error con «Reintentar». Respeta **reducir movimiento**: sin animaciones ni fundidos si está activo; si no, fundido de salida de 160 ms (escala de movimiento 120/160/220 ms).
- **Motivo**: el manual de marca prohíbe superponer el logo «sobre una fotografía agitada sin una superficie de contraste» (§7.10) y pide colocarlo «dentro de una zona lisa de contraste adecuado y con espacio de protección» (§7.9, uso 6). La ilustración es muy recargada, así que el logo nunca va sobre ella: va sobre el panel claro, que es el uso correcto 1 del manual (horizontal principal sobre blanco cálido).
- **Consecuencias**:
  - El manual cita el «símbolo aislado» para el splash (§7.9, uso 3). D-09 elige el logo horizontal sobre el panel claro; las dos formas son usos correctos y prevalece D-09 (01 §13, C-22).
  - El panel de la Pantalla de arranque usa siempre los tokens del modo claro, también con la app en modo oscuro, porque el logo principal es grafito y no puede ir sobre fondo oscuro.
  - La barra coral va sobre una pista grafito: el coral sobre blanco cálido solo da 2,68:1 y no llega al 3:1 que exige un componente gráfico (03 §4).
  - La imagen se registra en `assets/LICENSES.toml` (ARQ-75) como cualquier asset. Su origen está resuelto (ver «Procedencia de la imagen»); la comprobación de similitud sigue pendiente (VERIFICAR).
  - El instalador MSI y la imagen del DMG **no** usan esta ilustración (05 sin cambios): siguen con el logo sobre `#FAFAF7`.
- **Procedencia de la imagen** (lectura técnica del 24 de octubre de 2026, no es asesoría legal): el titular generó `assets/splash_screen.jpg` con ChatGPT Plus (generación de imágenes de OpenAI). Fecha de generación: la noche del 23 de octubre de 2026. Base legal: Términos de uso de OpenAI (vigentes desde el 1 de enero de 2026, https://openai.com/policies/row-terms-of-use/), apartado «Ownership of content», que cede al usuario todos los derechos sobre el *Output* y permite el uso comercial sin atribución ni regalías. Condiciones: no presentarla como obra humana, así que **Acerca de / créditos** la muestran como «generada con IA (OpenAI)»; sin garantía de unicidad ni de no infracción; probablemente no sea protegible por derechos de autor (U.S. Copyright Office, 2025), por lo que no da exclusividad. La copia del repositorio no lleva metadatos C2PA. Cuando se introduzca `assets/LICENSES.toml` (ARQ-75) se registrará su entrada con los campos: origen, herramienta, base de licencia, aviso de IA, nota de derechos de autor, procedencia y comprobación de similitud.
- Aplicado a 01 (RF-67, CAL-38, CAL-39), 02 (ARQ-65..ARQ-69, ADR-10, evento `StartupProgress`) y 03 (§7.10, DIS-80..DIS-87, glosario, tokens y contraste).

## D-10 Excepción de licencia para el audio generado localmente

Fecha: 24 de octubre de 2026.

- **Decisión**: se admite una **excepción a la regla «solo assets propios o con licencia permisiva»** (ARQ-75, FV-09, P-03). Vale **solo en este repositorio**, **solo para los 40 WAV** de `assets/musics/` y **solo mientras la licencia del modelo permita distribuirlos o regalarlos**. No sienta precedente para otros assets.
- **Origen**: el titular los generó en local el 24 de octubre de 2026 con su herramienta `minimax-music3`. Usa el runtime audio.cpp (Apache-2.0) con un único modelo, `minimax-music3`, en el port GGUF `audio-cpp/MiniMax-Music3-GGUF` (mezcla q4_0) del modelo `MiniMaxAI/MiniMax-Music3`. No se usó ACE-Step. Formato: WAV de 44,1 kHz, 16 bits, estéreo (~201 MB).
- **Licencia**: «MiniMax-Music3 Community License» (https://huggingface.co/MiniMaxAI/MiniMax-Music3/blob/main/LICENSE). El port GGUF publica el mismo texto, byte a byte.
- **Qué permite**: la licencia **no reclama ni restringe los resultados** y **no prohíbe distribuirlos**. Su cláusula de exención de garantía contempla expresamente «distributing any of the Software or outputs». Concede uso, copia y distribución «free of charge», incluido el uso comercial con las condiciones de abajo.
- **Obligaciones** (texto de la licencia):

| Cláusula | Qué exige | Qué hace Gafira |
|---|---|---|
| §3.1 | «prominently display “MiniMax-Music3” on the user interface of commercial product or service that uses the Software» | Lectura prudente: **aplica**. Mostrar «MiniMax-Music3» de forma visible en la interfaz (pestaña Sonidos y/o Acerca de) |
| Anexo A, punto 11 | No difundir contenido en un entorno público sin indicar «clearly and prominently» que es **generado por máquina** | Indicar en la interfaz y en `THIRD_PARTY_NOTICES.md` que la música es generada por IA |
| Anexo A (lista completa) | Cumplir la política de uso aceptable; «MiniMax reserves the right to update» | Revisar el anexo antes de cada versión que distribuya el audio |
| §3.2 | Autorización previa por escrito si los ingresos anuales superan 20 millones de USD (api@minimax.io) | No aplica hoy; revisar si el negocio crece |
| Exención de garantía | El usuario asume los riesgos de distribuir los resultados y **indemniza a MiniMax** por incumplir la licencia | Riesgo del titular, sin garantía de MiniMax |
| §1 | Conservar el aviso de copyright y de permiso | Incluir el texto de la licencia en `THIRD_PARTY_NOTICES.md` |

- **Salvedad de derechos de autor** (contexto general, no es asesoría legal): el audio puramente generado por IA puede **no ser protegible por copyright** en algunas jurisdicciones. Gafira no podría impedir que otros lo copien, y no debe prometer exclusividad sobre estas pistas.
- **Almacenamiento**: el audio **queda fuera de git por ahora** (`.gitignore` ignora `assets/musics/*` salvo `assets/musics/LICENSE.md` y `assets/musics/README.md`). Al implementar la pestaña Sonidos (05, FV-09) se **convierten a un formato comprimido**, se versionan, y se registran en `assets/LICENSES.toml` y `THIRD_PARTY_NOTICES.md` con esta excepción como justificación.
- **Voces detectadas**: **resuelto**. La verificación de voz del lote marcó palabras en 9 pistas de la primera tanda: `bgm-04`, `underscore-02`, `underscore-04`, `underscore-05`, `underscore-06`, `underscore-10`, `soundtrack-08`, `ambient-06`, `ambient-10`. Las 9 se regeneraron con otra semilla y se volvieron a comprobar. La comprobación transcribe cada pista con tres pistas de idioma distintas (en, zh, es) y exige que **las tres devuelvan el mismo texto**: el ASR alucina frases sobre una cama instrumental y esa alucinación cambia con el idioma pedido, así que un texto idéntico en los tres es la señal de voz real. Estado final: **40/40 sin voz**, con tres rondas independientes por pista; Silero VAD coincidió en todos los casos.
- **Condición de caducidad**: si MiniMax cambia la licencia o el anexo de modo que impida distribuir los resultados, esta excepción **deja de valer** y se retira el audio.
- Aplicado a 05 (pestaña Sonidos, FV-09) cuando se implemente; sin cambios en las demás specs.

## D-11 `ColorTransform::apply_f32` devuelve `Result`

Fecha: 24 de octubre de 2026.

- **Decisión**: `ColorTransform::apply_f32` devuelve `Result<(), PixelBufferError>` en lugar de `()` (02 §4.10). Si la longitud del búfer no es múltiplo de los canales del `PixelLayout`, devuelve `Err(PixelBufferError::LengthNotMultiple { len, channels })` y deja el búfer **sin modificar**. Un búfer vacío es válido y devuelve `Ok(())`.
- **Motivo**: un error explícito evita el fallo silencioso (omitir el resto del búfer lo corrompe sin avisar) y evita un `panic`, que choca con la política futura de no entrar en pánico. Al devolver el error antes de tocar nada, el búfer del llamante queda intacto.
- **Efecto sobre 02 §4.10**: cambia la firma de `apply_f32`. `bake_lut3d` y `Lut3d` quedan fuera del trait hasta que exista su consumidor de GPU; se podrá añadir después un `bake_lut3d` con implementación por defecto sin romper a los implementadores.
- Aplicado a 02 (§4.10).

## D-12 Modelo de licencia sin prueba gratuita

Fecha: 24 de octubre de 2026.

- **Decisión**: **no hay prueba gratuita**. Gafira se puede usar sin licencia, pero en **modo sin licencia** la exportación está bloqueada y algunas funciones están desactivadas (ejemplo del titular: «quitar fondo», es decir, Aislar sujeto). Con una licencia activada se desbloquea todo.
- **Lista de funciones bloqueadas**: **pendiente**. Se define en la fase final de licencia y activación (spec 06). Hasta entonces, solo está fijado que la exportación está bloqueada sin licencia.
- **Precio**: se define **cuando el software esté terminado**. Cifra tentativa: **99 USD** (PROPUESTA, no fijada).
- **Qué sustituye de D-01**: la parte «con prueba gratuita (igual que Tracewright)» del **Modelo** y la frase «La duración y los límites de la prueba están por definir». Siguen vigentes de D-01: licencia propietaria, «todo de pago» (una única licencia), la activación con `tracer-license` y `license-hub`, y la sustitución del principio del PRD «sin funciones esenciales retenidas tras un plan de pago».
- **Consecuencias**:
  - No existen duración, límites ni estado de «prueba». Los estados de licencia son, como mínimo, sin licencia y con licencia activada.
  - El modo sin licencia no exige conexión: la app sigue trabajando sin red (D-01). La conexión sigue siendo solo para activar, validar o desactivar.
  - La interfaz debe indicar qué funciones están bloqueadas y cómo activar la licencia.
- Aplicado a 01, 03 y 06 (ver «Efecto sobre las especificaciones») cuando se promuevan. Aplicado a 02 (§4.14, §13, Q-19 cerrada) y 05 (Q-19 cerrada).

## D-13 `gafira-cli validate` pasa a `gafira-format`

Fecha: 24 de octubre de 2026.

- **Decisión**: el subcomando `gafira-cli validate` se implementa en el change `gafira-format`, no antes. Confirma la pregunta 5 de la propuesta archivada de C2b (`domain-doc`): sin lector de archivos `.gafira` no hay nada que validar desde la línea de comandos.
- **Archivos archivados**: no se editan; esta decisión es el registro de la confirmación.
- Aplicado al change `gafira-format` cuando se cree.

## D-14 Alcance del primer change de `gafira-layout`

Fecha: 30 de octubre de 2026.

- **Decisión**: el primer change de `gafira-layout` cubre solo las primitivas: FD-03, FD-04, FD-05, FD-06, FD-07, FD-09 y FD-11.
- **Consecuencia**: FD-53 y FD-54 (reflujo y catálogo de tamaños) pasan a un change posterior a `gafira-text`.
- Aplicado a 04.

## D-15 Guías generadas por FD-04

Fecha: 30 de octubre de 2026.

- **Decisión**: el Generador de guías crea dos guías por cada espacio entre columnas. «12 columnas» con espacio de 24 px da 22 guías de borde de columna y 2 de margen: 24 en total.
- Aplicado a 04 (FD-04, criterio a).

## D-16 Prioridad del Imán

Fecha: 30 de octubre de 2026.

- **Decisión**: el Imán resuelve cada eje por separado y gana el destino más cercano dentro del umbral. En un empate exacto, la prioridad es guía > margen > página > elemento > cuadrícula.
- Aplicado a 04 (FD-06).

## D-17 Distribuir con elementos bloqueados

Fecha: 30 de octubre de 2026.

- **Decisión**: distribuir ignora los elementos bloqueados. Solo se distribuyen los libres, entre el primero y el último libre, que no se mueven. Con menos de 3 elementos libres no hace nada. Un elemento libre puede quedar sobre uno bloqueado.
- Aplicado a 04 (FD-11).

## D-18 Guardia de asignación de ARQ-23

Fecha: 30 de octubre de 2026.

- **Decisión**: la guardia de asignación de ARQ-23 es un crate externo, solo como `dev-dependency`. `unsafe` sigue prohibido en el código propio.
- **Licencia**: se añade a 02 §9.2 como VERIFICAR. Si ningún crate pasa la comprobación de licencia, la decisión vuelve al titular.
- Aplicado a 02 (§4.9, §9.2, ARQ-23).

## D-19 Fuentes de prueba de texto y emoji

Fecha: 30 de octubre de 2026.

- **Decisión**: las pruebas de texto usan subconjuntos de Inter y Manrope y una fuente OFL con árabe o hebreo. Toda fuente se registra con su licencia (ARQ-75).
- **Emoji**: en el MVP son contornos que salen de fuentes de reserva que aporta quien llama. El emoji en color pasa a Fase 2.
- Aplicado a 02 (§4.17) y 04 (FD-41).

## D-20 Puerta de calidad de `gafira-segment`

Fecha: 30 de octubre de 2026.

- **Decisión**: las pruebas bloqueantes usan escenas sintéticas con máscaras exactas conocidas. El IoU sobre fotos reales es solo un aviso hasta que el titular aporte un conjunto con licencia.
- **Consecuencia**: se resuelve el choque entre FIJADA y PROPUESTA de 02, 01 y 04, y se añade el pendiente 9 del titular.
- Aplicado a 01 (P-04), 02 (§4.7, ARQ-132) y 04 (FD-38).

## D-21 Nivelar sin limitador

Fecha: 30 de octubre de 2026.

- **Decisión**: Nivelar limita la ganancia para que el pico real quede en −1 dBTP o por debajo, e informa de que no se alcanzó el objetivo.
- Aplicado a 02 (ARQ-127) y 05 (FV-10).

## D-22 `Eq3Band` y `Pan` en el MVP

Fecha: 30 de octubre de 2026.

- **Decisión**: `Eq3Band` y `Pan` son del MVP en `gafira-audio`, como dice 02.
- Aplicado a 01 (RF-51), 02 (§11.2) y 05 (FV-20).

## D-23 Zona de edición: «Texto» y «Elemento»

Fecha: 30 de octubre de 2026.

- **Decisión**: «Texto» pasa a Fase 2 y se muestra atenuada con el motivo. «Elemento» usa `ClassicSegmenter` con una pista de prioridad central y se atenúa con el motivo si el resultado no es fiable.
- Aplicado a 02 (ARQ-130, ARQ-131) y 04 (FD-32).

## D-24 Salto de columna manual

Fecha: 30 de octubre de 2026.

- **Decisión**: el salto de columna manual pasa a Fase 2 y FD-45(b) sale del MVP.
- Aplicado a 04 (FD-45).

## D-25 `type_lock.size` y Ajustar al cuadro

Fecha: 30 de octubre de 2026.

- **Decisión**: `type_lock.size` prevalece sobre `FitBox`. El tamaño no cambia y el interruptor «Ajustar al cuadro» se atenúa con el motivo.
- Aplicado a 02 (§3.2) y 04 (FD-46).

## D-26 Escala de Curvar

Fecha: 30 de octubre de 2026.

- **Decisión**: el rango ±100 de Curvar corresponde de forma lineal a ±180° de arco.
- Aplicado a 04 (FD-44).

## D-27 Modos de mezcla y sombra «Curva»

Fecha: 30 de octubre de 2026.

- **Decisión**: el MVP tiene 16 modos de mezcla. La sombra «Curva» pasa a Fase 2.
- Aplicado a 04 (FD-12, FD-35).

## D-28 Valores por defecto de `gafira-layout`

Fecha: 30 de octubre de 2026.

- **Decisión**: el titular acepta tres propuestas:
  - Distribuir ordena los elementos por su posición en el eje.
  - Los nodos girados usan su caja alineada a los ejes.
  - El Imán está activo por defecto, con umbral de 6 px y rango de 2 a 16.
- Aplicado a 04 (FD-06, FD-11).

## D-29 Alinear con elementos bloqueados

Fecha: 30 de octubre de 2026.

- **Decisión**: los elementos bloqueados cuentan en la caja de referencia al alinear una selección (Q-35).
- Aplicado a 04 (FD-11).

## D-30 Fuente de prueba de emoji

Fecha: 30 de octubre de 2026.

- **Decisión**: las fuentes de prueba de texto añaden un subconjunto OFL monocromo de emoji, registrado con su licencia (ARQ-75). FD-41(a) comprueba contornos reales de emoji (Q-36).
- Amplía D-19; el resto de D-19 sigue vigente.
- Aplicado a 02 (ARQ-115) y 04 (FD-41).

## D-31 Ganancia propia de Nivelar

Fecha: 30 de octubre de 2026.

- **Decisión**: Nivelar tiene su propia ganancia, aparte del Volumen del clip, y la aplica `LoudnessNormalizer` (Q-37). El tope de −1 dBTP de D-21 sigue vigente.
- Aplicado a 02 (§4.9, ARQ-127) y 05 (FV-10).

## D-32 Lugar de Nivelar en el MVP

Fecha: 30 de octubre de 2026.

- **Decisión**: en el MVP, Nivelar está en Inspector → Clip (Q-38).
- Aplicado a 05 (FV-10, FV-20).

## D-33 Sin slider de Desenfoque en FD-33

Fecha: 30 de octubre de 2026.

- **Decisión**: FD-33 no tiene slider de Desenfoque en el MVP, ni en Textura ni en otro grupo (Q-39). El desenfoque solo se usa dentro de otros efectos.
- Aplicado a 04 (FD-33).

## D-34 Cifras PROPUESTA nuevas de capa 2

Fecha: 30 de octubre de 2026.

- **Decisión**: las cifras PROPUESTA añadidas el 30 de octubre de 2026 en 02 §4.17 y en 04 FD-04, FD-07, FD-35, FD-41 y FD-44 quedan confirmadas en bloque (Q-40). Pueden cambiar más adelante con mediciones.
- **Marcas**: esas cifras conservan su marca PROPUESTA; no se reetiquetan una a una.
- **Quedan fuera**: las fórmulas de FD-33 (Q-34) y las cifras que ya eran PROPUESTA antes de esa fecha, como la Zona de seguridad del 5 % de FD-05.
- Aplicado a 02 (§4.17, §12.2) y 04.

## D-35 Hijos de grupo en espacio de página

Fecha: 30 de octubre de 2026.

- **Decisión**: la `Placement` de todo nodo está en espacio de página, también la de los hijos de un grupo (ADR 0009).
- **Revisión**: esta decisión se revisa frente al diseño de video antes de la primera build que guarde archivos. Un cambio posterior exige una migración.
- Aplicado a 02 (§3.2) y 04 (FD-06, FD-13).

## D-36 Fin del esquema 1 abierto

Fecha: 30 de octubre de 2026.

- **Decisión**: el esquema 1 sigue abierto hasta que la primera build que escriba `.gafira` llegue a un usuario (ADR 0006). Desde entonces, un campo nuevo con datos de usuario sube el esquema.
- Aplicado a 02 (§3.1).

## D-37 Excepción de licencia de `assert_no_alloc`

Fecha: 30 de octubre de 2026.

- **Decisión**: `assert_no_alloc` tiene licencia BSD-1-Clause, que no está en la lista permitida. Se admite con una excepción por crate, con su motivo, en `deny.toml`.
- **Efecto sobre D-18**: resuelve la comprobación de licencia que D-18 dejó como VERIFICAR. `unsafe` sigue prohibido en el código propio.
- Aplicado a 02 (§9.1, §9.2).

## D-38 Zona de seguridad por defecto

Fecha: 30 de octubre de 2026.

- **Decisión**: la Zona de seguridad por defecto es del 5 % por lado. Las zonas de video vertical por plataforma siguen en VERIFICAR.
- Aplicado a 02 (§4.17.2) y 04 (FD-05).

## D-39 ADR 0005 a 0010

Fecha: 30 de octubre de 2026.

- **Decisión**: el titular acepta los ADR 0005 a 0010 (`docs/adr/`).

## D-40 Filtros en el primer corte de `gafira-raster`

Fecha: 1 de noviembre de 2026.

- **Decisión**: el primer corte entrega el mecanismo de filtros de FD-34 y un solo preset de prueba (opción a de la pregunta O3 de `specs/done/legacy/history/2026-11-02-layer-2/design.md`). El catálogo real llega más adelante, con Q-34.
- **Intensidad**: es una mezcla lineal de cada parámetro desde su valor por defecto hasta el valor del preset.
- Aplicado a 02 (§12.3) y 04 (FD-34).

## D-41 Intensidad de filtros por tipo de parámetro

Fecha: 1 de noviembre de 2026.

- **Decisión**: la Intensidad de FD-34 trata cada tipo de parámetro así (Q-41): `Int` mezcla como `Float` y redondea, y medio se aleja de 0; `Color` mezcla en luz lineal; `Bool` y `Choice` saltan al valor del preset por encima de intensidad 0.
- Amplía D-40; el resto de D-40 sigue vigente.
- Aplicado a 02 (§12.2) y 04 (FD-34).

## D-42 Brillo y Contorno fuera del primer corte

Fecha: 1 de noviembre de 2026.

- **Decisión**: los tipos Brillo y Contorno de FD-35 salen del primer corte de `gafira-raster` hasta que se aprueben sus fórmulas (Q-42). Paralela sigue en el corte.
- Aplicado a 02 (§4.17.4, §12.2) y 04 (FD-35).

## D-43 Cancelación de sonoridad, Nivelar y remuestreo

Fecha: 1 de noviembre de 2026.

- **Decisión**: medir la sonoridad, Nivelar y remuestrear quedan fuera de ARQ-112 en este corte (Q-43). `gafira-engine` los cancelará entre bloques más adelante.
- Aplicado a 02 (ARQ-112, §12.2).

## D-44 Colores medios de la fiabilidad

Fecha: 1 de noviembre de 2026.

- **Decisión**: los colores medios de ARQ-131 se promedian en luz lineal y después se pasan a Lab (Q-44).
- Aplicado a 02 (ARQ-131, §12.2).

## D-45 `ClassicSegmenter` y alcance de la puerta de calidad

Fecha: 2 de noviembre de 2026.

- **Decisión**: `ClassicSegmenter` usa solo color, sin término de posición en el GMM, y una copia de trabajo de lado largo 2048 px en lugar de 1024.
- **Puerta**: el conjunto fijo de 16 escenas deterministas sigue siendo la prueba bloqueante, con IoU ≥ 0,90 por escena. Un conjunto de 12 escenas nuevas, con otra semilla y nunca usado para ajustar parámetros, corre como aviso, no bloquea e informa de su recuento.
- **Alcance de D-20**: D-20 se cumple en las escenas de la puerta y no está demostrado que generalice. El resultado conocido en las escenas nuevas es 9 de 12.
- **Q-45**: el fondo texturizado a ΔE76 6 queda fuera de la puerta. Las escenas texturizadas de la puerta conservan el mínimo de ΔE76 de ARQ-132.
- **Nota (verificación de SG3, `gafira-segment`)**: el 9 de 12 es el resultado de las 12 escenas nuevas de XP5. Las 16 escenas de la puerta son las definiciones de XP5; el conjunto nuevo de 02 ARQ-132 es otro y da su propio recuento.
- Amplía D-20; el resto de D-20 sigue vigente. La prioridad central de «Elemento» (D-23) no cambia.
- Aplicado a 01 (P-04), 02 (§4.7, ARQ-130, ARQ-131, ARQ-132, §6.7, §12.2, §12.3) y 04 (FD-38).

## D-46 Invertir como interruptor

Fecha: 2 de noviembre de 2026.

- **Decisión**: Invertir de FD-33 es un interruptor (`invert`, apagado por defecto), no un slider. Su fórmula es `v' = 1 − v`.
- Aplicado a 04 (FD-33).

## D-47 Colores minoritarios de la banda y fiabilidad

Fecha: 2 de noviembre de 2026.

- **Decisión**: se mantiene la regla de `ClassicSegmenter` que aparta de los GMM los colores minoritarios de la banda de borde, y `reliable` es falso cuando esa regla aparta un color real (hallazgo SG3-W7).
- **Motivo**: la regla conserva un sujeto que toca el borde. A cambio, en un fondo de dos colores, el menor puede pasar al sujeto.
- **Consecuencia**: la máscara no cambia. El ruido aislado de la banda no cuenta, así que el umbral de «color real» es una cifra PROPUESTA de 02 ARQ-131, no de esta decisión.
- Aplicado a 02 (§4.17.6, ARQ-131, ARQ-133).

## D-48 Espacio de mezcla de las transiciones

Fecha: 2 de noviembre de 2026.

- **Decisión**: las transiciones mezclan en luz lineal (O5 (a)). `Rgba8` se decodifica a luz lineal, se mezcla y se codifica. `RgbaF32` se mezcla tal como está guardado.
- **Motivo**: la vista previa y la exportación comparten el grafo (02 §6.2), y la exportación trabaja en luz lineal.
- **Coste conocido**: una Disolvencia `Rgba8` en CPU a 1080p tarda de 154 a 156 ms, unas 19 veces los 8 ms de render de 02 §6.2 (XP13: 5 ejecuciones, `--release`, `p` 0,37). `RgbaF32` y los extremos no pagan ese coste. Los remedios del diseño de capa 2 (P15, punto 8) son estimaciones, no medidas, y no entran en esta decisión.
- **Alcance**: transiciones. Los modos de mezcla de ARQ-119 los fija D-53.
- Este O5 es el del diseño de capa 2, no el O5 de audio de `spike-layer2.md`.
- Aplicado a 02 (§6.2, ARQ-118, §12.3).

## D-49 Las asas de fundido no se cruzan

Fecha: 2 de noviembre de 2026.

- **Decisión**: las asas de fundido de entrada y salida de un clip no pueden cruzarse: la suma de sus duraciones nunca supera la duración del clip (Q-46). Recortar un clip por debajo de esa suma reduce los dos fundidos en proporción.
- Aplicado a 02 (ARQ-124, §12.2, §12.3) y 05 (FV-10).

## D-50 Corte sin duración

Fecha: 2 de noviembre de 2026.

- **Decisión**: un Corte ignora la duración y la alineación, y el cambio ocurre en el corte (Q-47).
- Aplicado a 02 (§4.13, ARQ-118, §12.2, §12.3) y 05 (FV-06).

## D-51 Fundidos al acortar un clip por otras vías

Fecha: 2 de noviembre de 2026.

- **Decisión**: Velocidad, Dividir y Congelar fotograma también reducen los fundidos en proporción cuando dejan un clip más corto que su suma (Q-50). Es la regla de D-49 para recortar.
- Amplía D-49.
- Aplicado a 02 (§12.2, §12.3) y 05 (FV-10).

## D-52 Alargar no devuelve los fundidos; arrastre desde el inicio del gesto

Fecha: 2 de noviembre de 2026.

- **Decisión**: alargar un clip no devuelve los fundidos que se redujeron; solo Deshacer los restaura. En un arrastre, la reducción se calcula desde las duraciones del inicio del gesto (Q-51).
- Amplía D-49.
- Aplicado a 02 (§12.2, §12.3) y 05 (FV-10).

## D-53 Modos de mezcla en luz lineal

Fecha: 2 de noviembre de 2026.

- **Decisión**: los modos de mezcla entre capas operan en luz lineal, también en `Rgba8`, igual que las transiciones de D-48 (Q-52). Incluye los 16 modos de D-27, `Normal` incluido.
- **Consecuencia**: los modos se ven distintos de los programas que mezclan valores sRGB codificados. La vista previa `Rgba8` paga la conversión por píxel, como en D-48; ese coste no está medido.
- Amplía D-48.
- Aplicado a 02 (ARQ-119, §12.2, §12.3).

## D-54 Fundidos al Dividir y Congelar fotograma

Fecha: 2 de noviembre de 2026.

- **Decisión**: tras Dividir o Congelar fotograma, la primera parte conserva el fundido de entrada, la última conserva el de salida, y en el punto de división no hay fundido (Q-53). Así dividir y volver a unir no pierde nada (05 FV-03 (a)).
- Amplía D-51.
- Aplicado a 02 (§12.2) y 05 (FV-10).

## D-55 Guardar sin licencia mientras falta la lista de D-12

Fecha: 2 de noviembre de 2026.

- **Decisión**: mientras la lista de funciones bloqueadas de D-12 siga pendiente, un usuario en `Unlicensed` o `Expired` puede editar, recuperar un journal, guardar con normalidad (Guardar, `Ctrl+S`) y usar «Guardar copia». Solo la exportación está bloqueada (opción (b) de la pregunta del hueco S10 de `gafira-ports`).
- **Precisiones** (respuestas del titular a las preguntas de D-55):
  1. La licencia nunca bloquea guardar: `Invalid` también guarda, igual que `Unlicensed` y `Expired`.
  2. Guardar como y el autoguardado en el journal también están permitidos sin licencia.
  3. «Exportar proyecto editable con recursos» cuenta como guardar, no como exportar: solo escribe el `.gafira` del usuario y sus recursos. Está permitido sin licencia y `can_export` no lo controla.
- **Guardar**, en esta decisión, es Guardar (`Ctrl+S`), Guardar como, «Guardar copia», el autoguardado en el journal y «Exportar proyecto editable con recursos». Exportar es todo lo demás que escribe una salida (PNG, PDF, MP4 y los demás formatos de exportación).
- **Consecuencia**: `LicenseStatus` conserva un único predicado, `can_export`. No hay `can_save`: guardar no depende de la licencia en ningún estado.
- Amplía D-12. Sustituye la propuesta de solo lectura de 02 (Q-20).
- Aplicado a 01 (§11, Q-20), 02 (ARQ-04, §4.14, §13.1, §13.3, ARQ-95, Q-20) y 04 (FD-63).

## D-56 Sombra en luz lineal

Fecha: 2 de noviembre de 2026.

- **Decisión**: el efecto Sombra pinta el elemento sobre su sombra con el modo `Normal` de ARQ-119, en luz lineal también en `Rgba8`, como D-53 (aviso W1 de la verificación de D-53). Así la sombra en 8 bits y en `f32` sigue la misma regla.
- **Precisión** (titular, tras medir): las dos coinciden a ≤ 2/255, la cota de ARQ-120 para sombras translúcidas tintadas, en los píxeles con alfa de sombra en 8 bits ≥ 16. Por debajo no hay cota: una sombra de alfa cercano a 1 no conserva su color en 8 bits, y la diferencia medida llega a 6,4/255. Con 16, el peor caso medido es 1,88/255, un margen de 0,12/255; con 14 sería 1,99/255, sin margen (04 FD-35 (f)).
- **Consecuencia**: en 8 bits cambian los píxeles donde un elemento translúcido deja ver su sombra, que hasta ahora se componían sobre los valores sRGB codificados.
- Amplía D-53.
- Aplicado a 02 (ARQ-120, §4.17.4) y 04 (FD-35).

## D-57 Referencia de 8 bits de los modos de mezcla

Fecha: 2 de noviembre de 2026.

- **Decisión**: la referencia en `f64` de ARQ-119 para 8 bits despremultiplica con el redondeo de 8 bits de ARQ-117, como hace el código (aviso W2 de la verificación de D-53).
- **Motivo**: frente a una referencia con `c / a` exacto, diez modos se separan más de 1/255, también con alfas altos: ColorBurn (hasta 85/255), ColorDodge, Hue, Overlay, Difference (2,05/255), Saturation (1,28/255), SoftLight (1,05/255), Luminosity (2,15/255), Color (1,24/255) y HardLight (1,69/255).
- **Consecuencia**: una ruta GPU que reciba 8 bits debe despremultiplicar con el mismo redondeo para quedar a ≤ 2/255 de la CPU (04 FD-12 (c)).
- Precisa D-53.
- Aplicado a 02 (ARQ-119) y 04 (FD-12).

## Efecto sobre las especificaciones

| Spec | Qué cambiar |
|---|---|
| 01 | Quitar el principio de no retener funciones esenciales; reflejar D-01, D-03, D-07; D-12: sustituir cualquier mención de prueba gratuita por el modo sin licencia (exportación bloqueada y funciones desactivadas), precio tentativo PROPUESTA; cerrar Q-01, Q-02, Q-03, Q-04, Q-05, Q-06, Q-07, Q-08, Q-09, Q-11, Q-12, Q-13, Q-16. D-09: RF-67 (Pantalla de arranque), fila de alcance del MVP, CAL-38 y CAL-39, contradicción C-22 y trazabilidad |
| 02 | D-11: §4.10, `apply_f32` devuelve `Result<(), PixelBufferError>`. D-01 (conexión de licencia), D-02, D-05, D-06 (quitar `update-check`), D-07; corregir las incoherencias listadas en el informe de redacción. D-09: §8.4, ARQ-65..ARQ-69 (antes reservados), ADR-10, eventos de arranque en §5.2, filas en §6.7 y §11.2 |
| 03 | Pantalla de activación (sin «prueba»; D-12: modo sin licencia con exportación y funciones bloqueadas, avisos de qué está bloqueado), estados de licencia; sin pantalla de actualizaciones; modo de licencia en Ajustes. D-09: §7.10 Pantalla de arranque, DIS-80..DIS-87, fila del glosario (§2.1 y §2.8.1), tokens `splash` (§3), pares de contraste (§4), logo en §5.1, microcopy (§12) y PD-15..PD-17 |
| 04 | PPTX en Fase 2 como imágenes. D-09: sin cambios |
| 05 | Linux x86_64 con `.deb`, `.rpm`, `.tar.gz`; sin AppImage ni aarch64; sin actualizador; códecs según D-02; MP3 y AVI según D-07; aviso de marca (D-04). D-09: sin cambios en el instalador; al actualizar 05, la frase «el primer arranque muestra Inicio» (Parte C) debe decir que antes aparece la Pantalla de arranque |
| 06 | D-12 (al promoverla): la licencia y activación se diseñan sin prueba gratuita ni su duración o límites; definen el modo sin licencia (exportación bloqueada), la lista final de funciones bloqueadas y los estados de licencia |

## Pendientes del titular

1. Definir la lista de funciones bloqueadas sin licencia (D-12) y fijar el precio final (tentativo 99 USD, PROPUESTA).
2. `LICENSE`: **hecho** (aviso propietario mínimo en la raíz). El EULA (`LICENCIA.md`) y el `PRIVACY.md` se escriben en la fase final de licencia (06).
3. Completar la lista de seguridad del `license-hub` y desplegarlo: va a la fase final de licencia (06).
4. Marca (D-04): la búsqueda preliminar está **hecha** (24 de octubre de 2026; riesgo bajo-medio, ver D-04). Pendientes: la búsqueda de disponibilidad de un abogado (incluidas AFIRA, afira y GAIRA) antes del lanzamiento y, opcionalmente, reservar gafira.app, .io, .co y .dev (gafira.com ya está registrado por el titular para la página de presentación). La elección del primer país de registro queda **aplazada** (D-84).
5. ~~Verificar con `cargo-deny` las licencias marcadas VERIFICAR (incluido el decodificador MP3)~~ (**resuelto** el 24 de octubre de 2026: pasan 72 de 76; `zip`, `ureq`, `directories` y `ort` se corrigen con features o cambiando de crate, ver 02 §9.2; las patentes de MP3 siguen en VERIFICAR).
6. ~~Registrar el origen, la autoría y la licencia de `assets/splash_screen.jpg` en `assets/LICENSES.toml` (D-09)~~ (**resuelto** el 24 de octubre de 2026: generada con ChatGPT Plus; los Términos de uso de OpenAI ceden al titular los derechos sobre el *Output* y permiten el uso comercial sin atribución ni regalías; se muestra como «generada con IA (OpenAI)» en Acerca de / créditos; sin garantía de unicidad ni de no infracción y probablemente sin derechos de autor exclusivos; ver D-09). D-10 cubre solo el audio de `assets/musics/`. Quedan como VERIFICAR para el titular: (a) búsqueda inversa de imagen (Google Lens, TinEye) y revisión visual de logos, textos o firmas; (b) conservar fuera del repositorio la descarga original con C2PA como prueba de procedencia; (c) ~~anotar la fecha de generación~~ (**resuelto**: la noche del 23 de octubre de 2026); (d) crear la entrada en `assets/LICENSES.toml` al introducir ese archivo (ARQ-75).
7. ~~Aprobar el nombre «Pantalla de arranque» / «Startup screen» y las cifras de D-09~~ (**aprobado** el 24 de octubre de 2026; ver D-09).
8. D-10: ~~revisar las 9 pistas con voz detectada y volver a comprobar `underscore-05`, `-06` y `-10`~~ (**hecho**: las 9 regeneradas y comprobadas, 40/40 sin voz); queda al implementar la pestaña Sonidos convertir el audio, registrarlo en `assets/LICENSES.toml` y `THIRD_PARTY_NOTICES.md` y mostrar «MiniMax-Music3» y el aviso de contenido generado por IA.
9. D-20: aportar un conjunto de fotos reales con licencia y máscaras de referencia para `gafira-segment` antes de publicar. Hasta entonces, el IoU sobre fotos reales es solo un aviso.

## D-58 Licencia opcional por build

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): el licenciamiento se puede **habilitar o deshabilitar** sin cambiar el engine ni la UI, porque es posible que Gafira se publique sin licencia. El composition root elige qué `LicenseVerifier` inyecta: el adaptador de `gafira-license` (licencia habilitada) o `NoLicensing` de `gafira-ports` (licencia deshabilitada).
- **Licencia deshabilitada**: el estado es `LicenseStatus::NotRequired`; `can_export` es verdadero, no hay pantalla de activación ni conexión de red, y activar, validar o desactivar devuelven `LicenseError::Disabled`.
- **Licencia habilitada**: sigue rigiendo D-12 y D-55 sin cambios.
- Amplía D-12. Aplicado en la spec 010, en 02 (§4.14, §13.1, §13.3) y en 01 (§11.1).

## D-59 Importar HEVC y HEIC por la API del sistema (Q-17)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): Gafira **importa** HEVC y HEIC solo a través del decodificador del sistema operativo (macOS y Windows; en Linux, cuando el sistema lo ofrezca). El binario no incluye un decodificador propio de HEVC ni HEIC y **nunca exporta** HEVC.
- **Consecuencia**: precisa D-02, que prohíbe HEVC en la salida y en el código propio, no la importación por el SO. Si el SO no tiene decodificador, el archivo se rechaza con un mensaje que explica por qué.
- Cierra Q-17. Aplicar a 01 (RF-19, RF-37), 04 (FD-31) y 05.

## D-60 Fuentes del usuario dentro del proyecto (Q-10)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): una fuente del usuario se **incrusta según su licencia** (bits `fsType` de OpenType) y **según el formato de archivo**, tanto en `.gafira` como en `.gaf` (spec 008). Si la licencia no permite incrustarla, el proyecto la vincula por nombre y, al abrirlo en un equipo que no la tiene, avisa de la fuente que falta.
- **Pendiente**: qué hace cada formato con una fuente incrustable (por ejemplo, si `.gafira` la vincula y `.gaf` la empaqueta) se fija con las decisiones de la §17 de la spec 008.
- Cierra Q-10 en lo que toca a la regla de licencia. Aplicar a 01 (RF-27), 04 (FD-48, FD-63) y 02 §7.1.

## D-61 Idioma de la CLI, el instalador y los paquetes (Q-26)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): `gafira-cli` escribe en inglés por defecto y solo cambia con `--lang` (ARQ-104), para que su salida sea estable en scripts. El instalador, los paquetes y el `.desktop` de Linux se ofrecen en **español e inglés**, según el idioma del sistema (D-08).
- Cierra Q-26. Aplicar a 01 (RF-56, RF-64) y 05 (FV-30, que ya no dice «solo español» pero debe fijar los dos idiomas).

## D-62 Primera versión sin licencia (D-12, D-58)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): la primera versión se publica **sin licencia**: el composition root inyecta `NoLicensing` (D-58), todo está habilitado y no hay conexión de red.
- **Consecuencia**: `gafira-license`, la lista de funciones bloqueadas y el precio de D-12 quedan **aplazados** hasta que el titular decida cobrar; no bloquean el lanzamiento. ARQ-90 a ARQ-96 se aplican solo a un build con licencia habilitada.
- Precisa D-12 y D-58.

## D-63 Identidad del proyecto para la recuperación (SP5)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): cada proyecto lleva un **UUID** que se genera al crearlo y se guarda en el `.gafira` (campo nuevo de `schema` 1, que D-36 mantiene abierto). `recovery/<project_id>/` y la carpeta de versiones usan ese UUID, así que la recuperación sobrevive a renombrar o mover el archivo. (Precisado por D-67: el archivo es un `.gaf` y las versiones viven dentro de él, no en una carpeta aparte.)
- **Copias**: «Guardar copia» asigna un UUID nuevo a la copia; «Guardar como» conserva el del proyecto.
- Cierra SP5 (02 §5.6) y desbloquea E4. Sustituye la clave provisional de P13.

## D-64 Formato de números y fechas por región (Q-24)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): los números y las fechas siguen la **región del sistema cuando su idioma coincide con el de la interfaz** (es-MX → `1,234.5`; es-ES → `1234,5`); si no coincide, se usa el formato CLDR del idioma de la interfaz. Confirma la propuesta de 02 §14.4.
- Cierra Q-24.

## D-65 El tiempo vive en el documento (Q-54, Q-55)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): se confirma la opción (a): `gafira-doc` depende de `gafira-time` y los tipos de tiempo del documento viven ahí, como ya hace el código.
- Cierra Q-54 y Q-55.

## D-66 Mutantes críticos de `gafira-text` y `gafira-raster` en paralelo (07, ítems 74 y 75)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): los 98 mutantes críticos vivos se atacan **ahora, en paralelo** al engine, en una spec propia que solo añade tests (sin cambiar la API pública).

## D-67 Extensión `.gaf` y un solo formato nativo

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): la extensión del archivo de proyecto es **`.gaf`**, como la define la spec 008 (ZIP con manifiesto y JSON, tipo MIME `application/vnd.gafira.project+zip`). El nombre `.gafira` para el archivo de proyecto se retira en todas partes.
- **Nombres derivados**: `nombre.gaf.bak` (1 generación), `nombre.gaf.lock` y las ranuras del journal de recuperación `slot-{0,1,2}.gaf-journal`.
- **Versiones**: viven **dentro del `.gaf`** (`versions/` del contenedor, como ya dispone 02 §7.1), no en una carpeta externa. La carpeta de recuperación `datos_app/recovery/<project_id>/` sigue fuera del archivo y se nombra con el UUID.
- **Un solo formato nativo**: «Guardar», «Guardar como» y «Guardar copia» escriben un `.gaf`; «Exportar proyecto editable con recursos» escribe un `.gaf` con el perfil `portable-full` (spec 008 §3). Donde un texto distinga un formato que vincula de otro que empaqueta, se lee como perfiles del mismo `.gaf`.
- Precisa D-63 (las versiones no van en una carpeta aparte). Donde D-13, D-36, D-55, D-60 y D-63 dicen `.gafira`, léase `.gaf`; en D-60, «según el formato de archivo» se lee «según el perfil del `.gaf`».
- Aplicado a 01, 02 (ADR-04, §7.1, §7.2), 04, 05 (FV-31), 03, la spec 008 (§4, §5, §17), la spec 012 (nota), el ADR 0006 (nota) y el manual técnico.

## D-68 Disposición híbrida del `.gaf`

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): el contenedor conserva la base de 02 §7.1 y le suma los campos de manifiesto de la spec 008:
  - **De 02 §7.1**: hashes de contenido BLAKE3 (ya en el código: `ContentHash`); `document.json` más un JSON por página (`pages/<id>.json`) y por secuencia (`sequences/<id>.json`), para la carga perezosa y el guardado incremental; recursos planos como `assets/<blake3>.<ext>`; versiones dentro, en `versions/`.
  - **De 008 §5**: `profile` (`portable-full` | `linked-light` | `hybrid`), `formatVersion` (versión semántica del contenedor, empieza en `1.0.0`), `schemaVersion`, `features` (`required` y `optional`) y `packageCompleteness`.
- **Grafía**: las claves de `manifest.json` van en **camelCase**, como en 008 (`projectId`, `formatVersion`, `schemaVersion`, `features`, `profile`, `packageCompleteness`, `generator`), y `format` vale `"gafira-project"`. El JSON del documento (`document.json`, `pages/*.json`, `sequences/*.json`) conserva el **snake_case** de `gafira-doc` (`project_id`, etc.): el código manda.
- **Integridad**: donde 008 dice SHA-256, se lee BLAKE3.
- **Carpetas**: `thumbs/` de 02 y `previews/pages/` de 008 son lo mismo (miniatura por página); se queda el nombre `thumbs/`, porque el código y las specs lo citan. Los **proxies nunca van en el archivo** (regla de 02): se retira la carpeta opcional `proxies/` de 008 y su opción «incluir proxies para rendimiento».
- **Queda para la spec del formato** (`gafira-format`): la forma de la tabla de recursos (`assets` por hash de 02 frente a `resources[]` con `storage` de 008), el campo `originalPathHint` de 008 §5.1 frente a ARQ-56, dónde van la portada y las miniaturas de secuencia de 008 (`previews/cover.webp`, `previews/video/`), `masks/` y `extensions/` (solo en 02) y `metadata/provenance.json` (solo en 008).
- Precisa D-67. Aplicado a 02 (ADR-04, §7.1, anexo A), 04 (FD-60, FD-63), 05 (FV-25) y la spec 008 (§2, §4, §5, §7.4, §8, §11).

## D-69 El tiempo en disco son flicks

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): los tiempos se guardan como **enteros de flicks**, igual que en memoria (`gafira-time`, D-65). Un flick es 1/705 600 000 s y divide exactamente los fotogramas de 30000/1001 y 24000/1001 fps, así que se cumple el criterio de 008 de no acumular deriva (§15, criterio 11).
- **Consecuencia**: se sustituyen los *ticks* y la `timebase` de 008 (GAF-ADR-08, §7) por flicks más la cadencia racional de la secuencia (`fps` = `num`/`den`). Los flicks se escriben como enteros JSON, como hace `Flicks` con serde; 2^53 flicks son unos 147 días, dentro del rango entero seguro de JSON.
- Amplía D-65. Aplicado a la spec 008 (§2, §5, §7, §8, §12, §15).

## D-70 Qué bloquea la primera versión gratuita (D-62)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): la primera versión es una **descarga gratuita**. Las licencias de terceros, las patentes, la marca y los avisos de terceros bloquean **cualquier** versión, gratuita o de pago. La activación, el EULA de venta, el precio y el identificador de dispositivo de `PRIVACY.md` esperan a que haya venta.
- **Consecuencia**: Q-20 (qué bloquea el modo sin licencia) queda **aplazada** con la licencia (D-62). El modelo de precios de D-01 y D-12 no cambia; solo se aplaza.
- Precisa D-62. Aplicado a 01 (§3, §5.1, §11, Q-20), 02 (Q-20) y 05 (C11, Q-20).

## D-71 Cifras PROPUESTA confirmadas en bloque

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): toda cifra marcada PROPUESTA en 01, 02, 04 y 05 queda **confirmada** como valor provisional que una medición puede cambiar, como en D-34.
- **Quedan fuera** y siguen abiertas: el aspecto de FD-33 y FD-34 (Q-34), el glosario y el microcopy en inglés (Q-25) y el modo oscuro derivado (C-06). No se tocan las cifras de 03 (en backlog) ni las de la spec 008.
- **Marcas**: a diferencia de D-34, las cifras se reetiquetan: PROPUESTA pasa a «confirmada (D-71)», para que lo que sigue en PROPUESTA sea solo lo abierto.
- Amplía D-34. Aplicado a 01, 02, 04 y 05.

## D-72 Voz en off y marcadores de compás en Fase 2 (Q-15)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): la **voz en off** grabada desde el micrófono y los **marcadores de compás** automáticos se quedan en **Fase 2**, como ya dicen 01 §5 y 05 FV-10.
- **Consecuencia**: Q-15 queda cerrada. Las marcas PROPUESTA de esas dos funciones pasan a «confirmada (D-72)»; el algoritmo de los marcadores de compás sigue en VERIFICAR.
- Aplicado a 01 (§5, Q-15) y 05 (FV-10, Q-15).

## D-73 Sin render distribuido en la red local (Q-18)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): se **descarta** el render distribuido en la red local. Gafira no abre ninguna conexión de red salvo la de la licencia futura (D-01, ARQ-04).
- **Consecuencia**: Q-18 queda cerrada. Ninguna fase planifica render en la red local; el texto que lo dejaba en suspenso pasa a descartado.
- Confirma D-01. Aplicado a 01 (§5, Q-18) y 02 (§11.2, Q-18).

## D-74 Sin tercer idioma por ahora (Q-23)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): no se planifica un tercer idioma; la pregunta se **aplaza** hasta después del lanzamiento. La arquitectura ya admite añadir idiomas (02 §14.8).
- **Consecuencia**: Q-23 queda aplazada. El producto se publica en español e inglés (D-08).
- Aplicado a 01 (§5, Q-23) y 02 (§4.16, Q-23).

## D-75 Huecos SP1, SP2, SP3 y SP8 del engine (02 §5.6)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): se acepta lo que hace el código.
  - **SP1**: `Gafira::new` solo falla con `EngineError::InvalidConfig` o con un error de un puerto (`crates/gafira-engine/src/engine.rs`). Hoy `InvalidConfig` lo devuelve `EngineConfig::new` y la construcción del engine no falla.
  - **SP2**: los tipos de carga del bus (`SaveState`, `JobResult`, `RecoveryEntry`, `PressureLevel`, `StartupStep`) los define el crate `gafira-engine`.
  - **SP3**: el engine construye `UserError { kind, message, detail, actions }`. `kind` es un enum cerrado; `detail` solo va al log local; `actions` son claves de mensaje. `CommandError` (`gafira-edit`) y `ValidationError` (`gafira-doc`) implementan `ToMessage` cuando llegue `E11`.
  - **SP8**: los niveles son `PressureLevel { Normal, Critical }`, como en el código (`crates/gafira-engine/src/governor.rs`). El límite del 70 % de la VRAM lo aplica el adaptador del backend de render en GPU, no el governor del engine.
- **Consecuencia**: SP1, SP2, SP3 y SP8 pasan a cerradas; `E11` ya no está bloqueada por SP3. SP7 sigue abierta.
- Aplicado a 02 (§5.6) y a `specs/README.md`.

## D-76 La sesión de solo lectura se puede editar en memoria (SP7, 02 §5.6)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): una sesión de solo lectura, abierta porque el archivo tiene un esquema futuro o un contenedor más nuevo o una capacidad requerida desconocida (D-68), **se puede editar en memoria**. «Guardar» se rechaza con un error de solo lectura antes de cualquier operación de archivo, y los cambios solo se conservan con «Guardar como», como hace el código (ARQ-55, 02 §7.1).
- **Consecuencia**: SP7 queda cerrada y 02 §5.6 no tiene huecos abiertos; `E3c` deja de ser provisional por SP7.
- Confirma ARQ-55 y precisa D-68. Aplicado a 02 (§5.6, ARQ-55), 04 (FD-63) y a `specs/README.md`.

## D-77 Glosario y microcopy en inglés aprobados (Q-25)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): los nombres ingleses del glosario (03 §2.8) y el microcopy en inglés (03 §12) se **aprueban** tal como están, a reserva de la búsqueda de marca de los nombres ingleses propios (R-17).
- **Consecuencia**: Q-25 queda cerrada. La búsqueda de marca de los nombres ingleses pasa a los pendientes del titular (01 §12.4) y a la lista previa a publicar (05 C11, punto 13). Las marcas PROPUESTA de 03 §2.8 se quedan mientras 03 siga en backlog; una nota al inicio de §2.8 remite a esta decisión.
- Aplicado a 01 (§12.3, §12.4), 02 (R-17, §12.3), 03 (nota en §2.8) y 05 (C11).

## D-78 Q-34 y Q-48 pasan a tareas de la deuda abierta

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): Q-34 y Q-48 salen de las tablas de preguntas abiertas y pasan a tareas de `specs/07-deuda-abierta.md`:
  - (a) **Ítem 133**: revisión visual de las fórmulas de FD-33 y del catálogo de filtros de FD-34 (antes Q-34).
  - (b) **Ítem 134**: medición de la segmentación en el hardware de referencia de D-03 (antes Q-48).
- **Consecuencia**: Q-34 y Q-48 quedan cerradas como preguntas. Las fórmulas de FD-33 siguen en PROPUESTA y las cifras de Q-48 siguen vigentes hasta que esas tareas se hagan; si una medición falla, el titular decide. El ítem 55 de 07 queda sin trabajo propio: lo recoge el 134.
- Aplicado a 01 (§12.3), 02 (§4.17, §12.3), 04 (convenciones, FD-33, FD-34, FD-38, §18) y 07 (§0, §1.1, §1.2).

## D-79 Firma, calidad, instalación, distribución y accesibilidad bloquean toda versión (05 C11)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): los puntos 15 (firma de código) y 19 a 22 (calidad, instalación, distribución y accesibilidad) de 05 C11 bloquean **cualquier** versión, también la primera, gratuita. Motivo: SmartScreen (Windows) y Gatekeeper (macOS) bloquean las aplicaciones sin firmar.
- **Consecuencia**: esos puntos pasan a «Toda versión» y C11 ya no tiene puntos sin clasificar.
- Amplía D-70. Aplicado a 05 (C11).

## D-80 Decisiones PROPUESTA confirmadas en bloque

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): toda decisión de producto o de diseño marcada PROPUESTA en 01, 02, 04 y 05 queda **confirmada** tal como está escrita. Las cifras ya las confirmó D-71.
- **Quedan fuera** y siguen abiertas: el campo `template` del manifiesto (02 §7.1, hueco abierto; 04 FD-16 y §15) y el modo oscuro derivado (01 C-06). Lo aplazado sigue aplazado: Q-20 y el precio (D-70) y Q-23 (D-74). Las fórmulas de FD-33 siguen en PROPUESTA hasta la revisión visual del ítem 133 de 07 (D-78).
- **Marcas**: como en D-71, PROPUESTA pasa a «confirmada (D-80)», para que lo que sigue en PROPUESTA sea solo lo abierto. No cambia el contenido de ninguna decisión.
- Amplía D-71. Aplicado a 01 (convenciones, RNF-01, §10.1, §11.1, C-18), 02 (convenciones, §4.17.5, §7.3, §13.1, §13.2, §14.3, §14.5), 04 (convenciones, FD-05, FD-17, FD-21, FD-24, FD-26, FD-44, FD-48, FD-59) y 05 (convenciones, FV-15, FV-18, FV-19, C1, C4, C7, C10).

## D-81 Campo `template` del manifiesto (04 FD-16 y §15; 02 §7.1)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): un `.gaf` es una **plantilla** cuando su `manifest.json` tiene el objeto opcional `template = { "tags": [string], "pageFormat": string, "style": string, "color": "#RRGGBB" }`. `pageFormat` es el id de un formato de inicio de 03 §6.2: `social`, `presentation`, `print`, `video_vertical`, `video_horizontal` o `custom`. Un lector que no conoce el campo lo ignora.
- **Consecuencia**: se cierran el hueco abierto de 02 §7.1 y la PROPUESTA de 04 FD-16 y §15, que D-80 dejó fuera. «Formato» es el de página, no el de archivo, y un `.gaf` sin `template` es un proyecto normal.
- Aplicado a 02 (§7.1), 04 (convenciones, FD-16, §15) y a la spec 008 (§5).

## D-82 Modo oscuro derivado aprobado (01 C-06)

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): la paleta oscura derivada de `assets/theme/gafira.tokens.json` (03 §3), validada por los 84 pares de contraste de la spec 014, queda **aprobada** como modo oscuro. Se puede ajustar más adelante sin reabrir 01.
- **Consecuencia**: C-06 queda cerrada, y con ella lo que D-80 dejó fuera sobre el modo oscuro. La marca PROPUESTA del modo oscuro en 03 se queda mientras 03 siga en backlog; una nota en §3 remite a esta decisión.
- Aplicado a 01 (C-06) y 03 (nota en §3).

## D-83 Cuándo un documento de referencia está aprobado

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): un documento de referencia de producto (01, 02, 04 y 05) está **«Aprobado»** cuando no tiene ninguna decisión del titular abierta. Los puntos VERIFICAR (licencias, patentes, dictámenes legales, comprobaciones de plataforma) y los *spikes* no lo bloquean, siempre que cada uno esté registrado como tarea en `specs/07-deuda-abierta.md` o en 05 §C11. Las decisiones aplazadas (Q-20 y el precio, D-70; Q-23, D-74) tampoco lo bloquean, porque están aplazadas de forma explícita.
- **Consecuencia**: los VERIFICAR y *spikes* de 01, 02, 04 y 05 que no tenían seguimiento pasan a 07 como ítems 138 a 153; los ítems 133 a 137 de 07 y 05 §C11 ya cubrían el resto. Al aplicarla, el 16 de noviembre de 2026, ningún documento queda aprobado, porque cada uno conserva una decisión del titular abierta:
  - 01: el primer país de registro de la marca (§12.4, punto 4; pendiente 4 del titular, D-04; ítem 145 de 07).
  - 02 y 05: la herramienta de MSI, entre pagar la Open Source Maintenance Fee de WiX v6 o posterior y usar otra herramienta (02 §10.1; 05 FV-30 y R-F; ítem 139 de 07).
  - 04: el aspecto de las fórmulas de FD-33 y el catálogo de FD-34, que siguen en PROPUESTA (D-78, D-80; ítem 133 de 07), y el tamaño mínimo de FD-28 (b) (ítem 60 de 07).
- Cuando el titular cierre la decisión que bloquea un documento, su fila `Estado` pasa a «Aprobado (D-83, fecha)»; hasta entonces se queda como está. Aplicado a 07 (ítems 138 a 153).

## D-84 Primer país de registro de la marca, aplazado

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): la elección del primer país de registro de la marca se **aplaza** («por ahora no se hace, después»). La búsqueda de disponibilidad con abogado y la de los nombres ingleses del glosario siguen como tareas (ítem 145 de 07; 05 C11, puntos 12 y 13).
- **Consecuencia**: es una decisión aplazada de forma explícita, así que, por D-83, no bloquea 01. Con ella, 01 ya no tiene ninguna decisión del titular abierta y pasa a «Aprobado (D-83, 2026-11-16)».
- Precisa D-04. Aplicado a 00 (D-04, pendiente 4), 01 (estado, §12.4, punto 4) y 07 (§0, ítem 145).

## D-85 El MSI no se hace con WiX

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular): el MSI de Windows **no** se construye con WiX. Su licencia, MS-RL, no está en la allowlist, y desde v6 el uso con ingresos exige la Open Source Maintenance Fee, que es de pago. Se usa una herramienta de empaquetado con licencia permisiva; son candidatas `cargo-packager` y `msitools` (`wixl`).
- **Consecuencia**: la licencia de la herramienta elegida se comprueba al implementar el empaquetado de Windows; el ítem 139 de 07 deja de ser una decisión del titular y pasa a ser esa comprobación técnica. Con ella, 02 y 05 ya no tienen ninguna decisión del titular abierta y pasan a «Aprobado (D-83, 2026-11-16)».
- Aplicado a 02 (estado, §10.1), 03 (§5.1), 05 (estado, FV-30, FV-40, C11 punto 2, R-F) y 07 (§0, ítem 139).

## D-86 Tamaño mínimo de FD-28 (b) y revisión visual de FD-33 y FD-34

Fecha: 16 de noviembre de 2026.

- **Decisión** (titular):
  - (a) 04 FD-28 (b) toma el mínimo que ya fija ARQ-122: una imagen y su versión al 50 % son parecidas (distancia de Hamming ≤ 10) si los dos lados de la original miden ≥ 128 px y la mitad se hace por media 2×2, o ≥ 640 px y la mitad se hace por bilineal o por vecino más cercano. Por debajo de esos tamaños no hay cota.
  - (b) El aspecto de las fórmulas de FD-33 y el catálogo de FD-34 son la tarea de revisión visual del ítem 133 de 07, que **no** bloquea 04. Las fórmulas siguen en PROPUESTA, como valores provisionales que fijan las pruebas, hasta esa revisión.
- **Consecuencia**: el ítem 60 de 07 queda resuelto, y el 133 sigue abierto como tarea sin bloquear 04. Con ello, 04 ya no tiene ninguna decisión del titular abierta y pasa a «Aprobado (D-83, 2026-11-16)».
- Precisa D-78 y D-83. Aplicado a 04 (estado, convenciones, FD-28, FD-33) y 07 (§0, §1.2, §1.3, ítems 60 y 133).

## D-87 Colección de iconos: Phosphor

Fecha: 17 de noviembre de 2026.

- **Decisión** (titular): la colección de iconos de partida es **Phosphor** (licencia MIT), no Lucide ni Feather. Se usa el peso «Regular»: 1,5 px de trazo a 24 px, con terminaciones y uniones redondeadas. Para los estados activos, que 03 §13 rellena, se usa el peso «Fill». Los iconos que falten se dibujan en la misma rejilla.
- **Consecuencia**: cierra PD-07 de 03. La licencia de la versión adoptada se comprueba al importarla, y cada icono va a `assets/LICENSES.toml` con su aviso en `THIRD_PARTY_NOTICES.md` (ARQ-75). Los SVG se convierten en geometría en tiempo de compilación; no se usan fuentes de iconos.
- Precisa 03 §13 («Origen»). Aplicado a 03 (§13 y PD-07).

## D-88 Atajos de Alinear con Alt

Fecha: 17 de noviembre de 2026.

- **Decisión** (titular): los atajos de Alinear dejan `Ctrl+Alt+L/C/R`, porque `Ctrl+Alt+L` bloquea la sesión en algunos escritorios Linux. Pasan a `Alt` con una letra, y las seis acciones tienen atajo: `Alt+A` izquierda, `Alt+H` centro horizontal, `Alt+D` derecha, `Alt+W` arriba, `Alt+V` medio vertical y `Alt+S` abajo.
- **Consecuencia**: cuentan solo con `Alt` exacto. AltGr no los dispara: en Windows llega como `Ctrl+Alt` y en Linux no llega como `Alt`, así que los caracteres que el teclado español escribe con AltGr no alinean. No chocan con los atajos de 03 §9.2 (`Shift+H` y `Shift+V` voltean; `W` del Editor de video no lleva modificador). `Ctrl+Alt+C` vuelve a ser una copia, como la convierte la ventana. Se aplica en la spec 044.
- Precisa 03 §9.2 y 04 FD-11. Aplicado a 03 (§9.2) y 04 (FD-11).

## D-89 Las esquinas conservan la proporción

Fecha: 17 de noviembre de 2026.

- **Decisión** (titular): al redimensionar, los cuatro tiradores de esquina conservan la proporción del elemento por defecto, con la esquina opuesta fija; mantener `Shift` mientras se arrastra la libera. Los tiradores de lado siguen cambiando una sola dimensión. Vale para formas, imágenes, cajas de texto y grupos, y para lo girado en sus propios ejes.
- **Consecuencia**: invierte lo que decía 03 §9.1 («`Shift` conserva la proporción»). El Imán engancha el borde que manda y el otro lado sigue la proporción. El tamaño mínimo (1 px) y arrastrar más allá de la esquina opuesta no rompen la proporción. Se aplica en la spec 045.
- Precisa 03 §9.1. Aplicado a 03 (§9.1 y §9.2).

## D-90 Alt redimensiona desde el centro; Ctrl suelta el Imán

Fecha: 17 de noviembre de 2026.

- **Decisión** (titular): mantener `Alt` mientras se arrastra un tirador redimensiona desde el centro: el centro se queda fijo y los lados opuestos se mueven a la vez y lo mismo. Se combina con D-89: una esquina conserva la proporción por defecto, desde el centro; `Shift` la libera; `Alt+Shift` es libre y desde el centro. Un lado con `Alt` cambia esa dimensión simétrica. Vale para lo girado en sus propios ejes y para los grupos. Mantener `Ctrl` (`Cmd` en macOS) mientras se mueve o se redimensiona suelta el Imán; `Alt` ya no lo hace.
- **Consecuencia**: resuelve el choque que dejó abierto la 045 (`Alt` desde el centro y `Alt` sin Imán) y saca el Imán de una tecla que algunos escritorios Linux usan para mover la ventana. El tamaño mínimo (1 px) se mantiene: más allá del centro el elemento se queda en el mínimo centrado, sin voltearse. El Imán engancha el borde arrastrado y refleja el opuesto. Durante el arrastre solo cuenta el estado de `Ctrl`: no dispara atajos ni copia. Se aplica en la spec 047.
- Precisa 03 §9.1 y 04 FD-06. Aplicado a 03 (§9.1) y 04 (FD-06).

## D-91 Lo bloqueado no se elige en el lienzo

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular): un elemento bloqueado, o bajo un grupo bloqueado, no se elige en el lienzo: el clic pasa a lo que está debajo, y el marco y `Ctrl/Cmd+A` lo saltan. Se selecciona solo desde Capas. Seleccionado así, no muestra tiradores ni asa de giro y no se mueve con arrastre ni con flechas; girar, voltear, alinear (D-29) y eliminar desde los menús siguen actuando sobre él.
- **Consecuencia**: un fondo bloqueado deja de tapar los clics sobre la página. Se aplica en la spec 048.
- Precisa 04 FD-13. Aplicado a 04 (FD-13).
- **Sustituida en parte** por D-110: un clic sí elige lo bloqueado.

## D-92 Lo que sobresale de una página se recorta en el lienzo

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular, con una captura de referencia): el lienzo dibuja cada elemento recortado a su página; la parte que sobresale no se ve. El marco de selección, sus tiradores y su asa de giro se dibujan completos por encima de todas las páginas, y se pueden tomar también fuera de la página. Arrastrar un elemento y soltarlo sobre otra página lo pasa a esa página.
- **Consecuencia**: sustituye en el lienzo el «se atenúa» de FD-09; el documento sigue guardando la geometría completa y la exportación sigue recortando a la página. Se aplica en la spec 051.
- Precisa 04 FD-09. Aplicado a 04 (FD-09).

## D-93 Lo que se suelta fuera de su página se elimina

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular): al soltar un arrastre de movimiento, un elemento cuya caja no comparte área con su página (ni la toca por dentro) se elimina, para que no quede contenido oculto que ocupe espacio. Mover y eliminar son un solo paso de deshacer. Soltarlo sobre otra página lo pasa a esa página (D-92).
- **Consecuencia**: sustituye el «los elementos totalmente fuera se listan en Capas con un icono de aviso» de FD-09 para lo que se suelta fuera; un documento que ya traiga elementos totalmente fuera los conserva hasta que se muevan. Se aplica en la spec 051.
- Precisa 04 FD-09. Aplicado a 04 (FD-09).

## D-94 «Piezas» pasa a llamarse «Formas»

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular): la pestaña del riel y la biblioteca de elementos que el glosario llama «Piezas» se llama «Formas» («Shapes» en inglés).
- **Consecuencia**: cambia el nombre normativo de D-04 en el glosario de 03 (§2.1 y §2.8). Las demás menciones a «Piezas» en 03 y 04 se leen como «Formas»; se actualizan cuando se edite cada sección. Se aplica en la spec 052.
- Precisa D-04 y 03 §2.1, §2.8. Aplicado a 03 (glosario).

## D-95 Las formas nuevas son trazados vectoriales en el documento

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular): los polígonos, estrellas, flechas, nubes, corazones y demás formas de la biblioteca se guardan como una geometría nueva de trazado vectorial (`ShapeGeometry::Path`), que se rellena, se contornea, se escala y se gira como un rectángulo o una elipse. Se descartaron las formas paramétricas (solo cubren polígonos y estrellas) y las imágenes SVG (no se les puede cambiar color ni contorno).
- **Consecuencia**: cambio aditivo del esquema 1 del documento: los `.gaf` existentes se abren igual y un documento sin trazados escribe el mismo JSON, pero una versión anterior de Gafira no abre un archivo que use trazados. Se aplica en la spec 054.

## D-96 Las formas pueden llevar texto, y el texto se escribe en el lienzo

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular, con una captura de referencia): toda forma puede llevar un texto dentro o quedar vacía; el texto se alinea a la izquierda, al centro o a la derecha y arriba, al medio o abajo (por defecto, centrado y al medio). Todo texto, el de un elemento de texto y el de una forma, se escribe directamente en el lienzo con un doble clic, sin ir al panel. La negrita y la cursiva vienen en una spec posterior.
- **Consecuencia**: cambio aditivo del esquema 1 (`ShapeNode.text`, escrito solo cuando existe): una forma sin texto conserva sus bytes; una versión anterior de Gafira no abre un archivo con texto en una forma. Se aplica en la spec 055.

## D-97 «Identidad» pasa a llamarse «Marca»

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular): la pestaña del riel y el concepto que el glosario llama «Identidad» e «Identidad visual» se llaman «Marca» («Brand» en inglés), un nombre más reconocible.
- **Consecuencia**: cambia los nombres normativos de D-04 en el glosario de 03 (§2.1 y §2.8). Las claves internas y los ids de requisitos no cambian; las demás menciones se leen como «Marca» y se actualizan al editar cada sección. Se aplica en la spec 057.
- Precisa D-04 y 03 §2.1, §2.8. Aplicado a 03 (glosario).

## D-98 Las líneas son un elemento propio que se conecta a las formas

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular, con capturas de referencia): una línea es un tipo nuevo de elemento con dos extremos, una punta por extremo, color y grosor. Cada extremo puede quedar conectado a una forma en uno de cinco puntos (arriba, derecha, abajo, izquierda y centro); su posición se calcula desde la forma al dibujar y al exportar, así que la línea la sigue sin despegarse. Al arrastrar un extremo cerca de una forma se ven sus puntos y el extremo se engancha como un imán. Se descartó guardar la línea como un trazado con puntas (obliga a reescribir cada línea al mover una forma).
- **Consecuencia**: cambio aditivo del esquema 1 (`NodeKind::Line`); una versión anterior de Gafira no abre un archivo con líneas. Una conexión a una forma borrada no invalida el documento: el extremo vuelve a su punto guardado. Se aplica en la spec 058.

## D-99 Las guías son comunes a todas las páginas

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular): las guías no son de cada página sino del documento: una guía creada, movida o borrada en una página aparece, se mueve o se borra en todas, en las mismas posiciones.
- **Consecuencia**: sustituye «Las guías son por página» de FD-03. Sin cambio de formato: cada página sigue guardando `Page.guides` y la interfaz las mantiene iguales en todas (un cambio se escribe en todas en un paso de deshacer; una página nueva nace con las de la primera). Se aplica en la spec 060.
- Precisa 04 FD-03. Aplicado a 04 (FD-03).

## D-100 Biblioteca de fuentes incrustadas, con negrita y cursiva

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular): Gafira incrusta una selección curada de familias de licencia libre (SIL OFL) con normal, negrita, cursiva y negrita cursiva donde existan, y un texto puede cambiar de fuente desde un panel «Fuente». Se descartaron una selección mínima y las fuentes del sistema (un diseño abierto en otro equipo cambiaría de fuente).
- **Consecuencia**: el estilo de un tramo gana `bold` e `italic` (cambio aditivo del esquema 1: un texto sin ellos conserva sus bytes; una versión anterior no abre un archivo que los use). Playfair Display y Merriweather se cambian por EB Garamond y Source Serif 4 por sus nombres reservados. La app crece unos 12 MB. Se aplica en la spec 062.

## D-101 El relleno de una forma puede ser un degradado

Fecha: 19 de noviembre de 2026.

- **Decisión** (titular, con capturas de referencia): el relleno de una forma es un color sólido o un degradado de dos o más colores, lineal en cualquier dirección o circular, elegido en el Inspector con las pestañas «Color sólido» y «Degradado».
- **Consecuencia**: cambio aditivo del esquema 1 (`ShapeNode.gradient`, que rellena en lugar de `fill` cuando existe): una forma sin degradado conserva sus bytes; una versión anterior no abre un archivo con degradados. El contrato del render gana `RenderOp::FillGradient`. Se aplica en la spec 063.

## D-102 «Formas» pasa a llamarse «Elementos», con categorías

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular, con capturas de referencia): la pestaña del riel que el glosario llama «Formas» se llama «Elementos» («Elements» en inglés). Su panel muestra una cuadrícula de categorías (las de 03 §2.3: Formas, Líneas, Marcos, Iconos, Códigos QR, Tablas, Gráficos, Ilustraciones); al pulsar una se entra en ella, con una flecha junto al título para volver. Volver a pulsar la pestaña activa del riel oculta el panel lateral, en todas las pestañas. Las tarjetas de categoría siguen la guía de marca: iconos de trazo simple en grafito sobre tarjetas claras apiladas (03 §13, P1); se descartaron iconos a color.
- **Consecuencia**: cambia el nombre normativo de D-94 en el glosario de 03 (§2.3 y §2.8); «Formas» queda como categoría dentro de Elementos. Las claves internas no cambian. Se aplica en la spec 064.
- Precisa D-94 y 03 §2.3, §2.8, §13. Aplicado a 03 (glosario).

## D-103 «Luces y efectos»: Brillo y Destello de lente

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular, con una imagen de referencia): Elementos gana la categoría «Luces y efectos» («Lights & effects») con dos elementos personalizables: «Brillo» («Sparkle»), una estrella de luz con halo interior blanco, rayos (número, grosor, intensidad y ángulo) y halo exterior de color opcional; y «Destello de lente» («Lens flare»), el mismo brillo más el reflejo de una cámara que cruza la página desde la luz y se recalcula al moverla. Los nombres buscan que los entienda quien no es diseñador. Se descartó «Pasar por detrás» (reordenar objetos dentro de una forma): la categoría es solo de luces.
- **Origen**: el comportamiento se tomó del manual público de un programa existente; su licencia prohíbe la ingeniería inversa, así que no se analizó su binario y la implementación es propia, con técnicas gráficas estándar.
- **Consecuencia**: cambio aditivo del esquema 1 (`ShapeNode.effect`; una forma sin efecto conserva sus bytes; una versión anterior no abre un archivo que lo use). La luz se dibuja como imagen, así que el contrato del render no cambia. Se aplica en la spec 065.
- Precisa D-102 y 03 §2.3.

## D-104 Tres luces más: Destello de cine, Rayos de sol y Luces bokeh

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular, pidiendo innovar sobre D-103): «Luces y efectos» gana «Destello de cine» («Cinematic streak»), la línea de luz de una lente anamórfica a lo largo de la página; «Rayos de sol» («Sun rays»), haces de luz que salen de la luz y cruzan la página; y «Luces bokeh» («Bokeh lights»), círculos de luz desenfocados repartidos por la caja de la forma. Usan los mismos ajustes que Brillo y Destello de lente (número, grosor o tamaño, intensidad, ángulo y color); el Inspector nombra cada ajuste según la luz.
- **Consecuencia**: tres variantes más de `ShapeEffect` (aditivo, esquema 1). Se aplica en la spec 065.
- Precisa D-103.

## D-105 Todo lo que sale del panel Elementos se arrastra al lienzo

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular): cada elemento del panel Elementos, de cualquier categoría presente o futura, se añade de dos formas, como las formas (spec 052): un clic lo pone en el centro de la página y un arrastre lo lleva al lienzo y lo deja centrado donde se suelta, seleccionado, en un solo paso de deshacer.
- **Consecuencia**: una categoría nueva de Elementos no está terminada sin arrastrar y soltar. Aplicado a Formas (spec 052) y a «Luces y efectos» (spec 065).
- Precisa D-102.

## D-106 «Líneas» sale de Elementos; entra «Foto collage»

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular): la categoría «Líneas» deja Elementos (las líneas siguen en la biblioteca de Formas) y su lugar lo ocupa «Foto collage» («Photo collage»): composiciones de varias fotos que funcionan como los Marcos (FD-18), cada hueco recibe una foto que queda recortada por él, pero con la estructura de un collage.
- **Consecuencia**: cambia la lista de categorías de D-102 y del glosario de 03 (§2.3, §2.8). Foto collage depende del mecanismo de Marcos (FD-18), aún no implementado; hasta entonces su vista dice «Disponible pronto.».
- Precisa D-102 y 03 §2.3.

## D-107 La foto de un marco vive dentro del marco

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular): el contenido de un marco (FD-18) es una referencia a la imagen guardada en el propio marco (`FrameContent { asset, natural }`), no un nodo hijo. Reemplazar la foto es un solo comando con un paso de deshacer, y la foto no aparece suelta en Capas ni se selecciona aparte del marco.
- **Consecuencia**: precisa el `content: Option<NodeId>` de 02 §3.2. Se aplica en la spec 066.
- Precisa 02 §3.2 y FD-18.

## D-108 Fondo de la página

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular, con capturas de referencia): con la página o el lienzo seleccionados (nada seleccionado), el Inspector muestra solo «Fondo de la página»: un color (de una paleta, del selector, o buscado por nombre en español o inglés —«azul», «blue»— o por código hexadecimal —«#00c4cc»—), un degradado predefinido, un patrón (puntos, cuadrícula, rayas, cuadros, zigzag o cruces) con su color y su fondo, o una imagen subida que rellena, encaja o se repite en mosaico; y «Aplicar a todas las páginas».
- **Consecuencia**: cambio aditivo del esquema 1 (`Page.background`; una página sin fondo conserva sus bytes; una versión anterior no abre un archivo que lo use). Se aplica en la spec 069.

## D-109 Redimensionar una caja de texto

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular, con una captura): una caja de texto tiene tiradores en sus cuatro esquinas y en el centro de sus lados, no en el centro de arriba ni de abajo. Una esquina escala la caja y el tamaño de letra en la misma proporción (la letra más pequeña no baja de 6 px); un lado cambia solo el ancho, nunca por debajo de lo que necesita una letra (1,2 veces la letra más grande).
- **Consecuencia**: 03 DIS-89. Aplicado en `gafira-ui` (`textbox.rs`).

## D-110 Un clic elige lo bloqueado para desbloquearlo

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular, con dos capturas): un clic sobre un elemento bloqueado lo selecciona (o el grupo bloqueado de más arriba), y la barra sobre la selección muestra solo el candado cerrado, resaltado, con «Desbloquear» al pasar el puntero. Sigue sin moverse ni mostrar tiradores. Un arrastre que empieza sobre él dibuja el marco de selección, y el marco y `Ctrl/Cmd+A` siguen saltándolo, así que un fondo bloqueado no estorba.
- **Consecuencia**: sustituye en D-91 «el clic pasa a lo que está debajo» y «se selecciona solo desde Capas»; lo demás de D-91 sigue. Se aplica en la spec 070.

## D-111 Fotos remotas: la segunda conexión de red

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular): Elementos → Fotos busca e inserta fotos de catálogos públicos. Es la segunda conexión de red de la app, acotada: solo la abre el panel Fotos al buscar, cargar miniaturas o insertar una foto, desde el adaptador `gafira-photos`, con su cliente HTTP (`ureq`, TLS con `rustls` y el almacén de certificados del sistema) tras la feature `online`, activada solo en el binario. Editar, guardar y exportar siguen sin red: una foto insertada se guarda en el almacén del proyecto como cualquier imagen.
- **Fuentes**: Openverse y Wikimedia Commons, sin clave de API (el titular pidió servicios gratuitos sin clave). Solo licencias que permiten uso comercial y modificación (CC0, dominio público, CC BY y CC BY-SA). Pixabay, Unsplash y Pexels quedan para cuando haya intermediario o clave (spec 071 §7). Ninguna clave en el código, el ejecutable ni el `.gaf`.
- **Privacidad**: las peticiones llevan un `User-Agent` genérico de Gafira, sin datos de la persona; la consulta no se guarda en disco.
- **Consecuencia**: precisa ARQ-04 y D-01: «la única conexión es la de licencias» pasa a «licencias y el panel Fotos». Se aplica en la spec 071.
- **Ampliación** (titular, 2026-11-21): Elementos → Ilustraciones usa la misma conexión con Iconify (`api.iconify.design`), sin clave, solo con colecciones MIT o Apache 2.0 (spec 072).

## D-112 PDF con vectores, texto real y contraseña

Fecha: 21 de noviembre de 2026.

- **Decisión** (titular): el PDF se escribe por defecto con vectores y texto real con su fuente incrustada; «Aplanar PDF» conserva el PDF de imagen. La contraseña cifra con AES-256 mediante `lopdf` 0.47 (MIT, sin sus funciones por defecto) sobre el PDF que escribe krilla, que no cifra; la clave del archivo sale de `getrandom`. El perfil CMYK es una conversión directa sin perfil ICC: los perfiles de imprenta (FOGRA, GRACoL) tienen licencias que no se han revisado.
- **Consecuencia**: `RenderOp::Text` y `PageRenderer::vectors` en `gafira-ports`; `ParamKind::Text` en el contrato de opciones. Se aplica en la spec 073.

## D-113 El código QR es un tipo de nodo propio

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular): un código QR es `NodeKind::Qr` (`QrNode`), como los marcos y las líneas, en lugar del nodo `Extension` con `type_id = "gafira.qr"` que pedía FD-20. Así el documento valida el logotipo, la exportación lo decodifica y editarlo es un paso de deshacer.
- **Consecuencia**: cambio aditivo del esquema 1; una versión anterior no abre un archivo con códigos QR. La técnica y FD-20 (c) quedan sustituidas. Los Gráficos (FD-22) siguen como estaban. Se aplica en la spec 080.
- Precisa ADR-02 y FD-20.

## D-114 El gráfico es un tipo de nodo propio

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular): un gráfico es `NodeKind::Chart` (`ChartNode`), como los QR (D-113) y las tablas, en lugar del nodo `Extension` con `type_id = "gafira.chart"` de FD-22: la infraestructura de extensiones no existe y el nodo propio se valida, se deshace y se exporta como vectores.
- **Consecuencia**: cambio aditivo del esquema 1; una versión anterior no abre un archivo con gráficos. La técnica y FD-22 (c) quedan sustituidas. Se aplica en la spec 083.
- Precisa ADR-02, FD-22 y D-113.

## D-115 Modelos de imagen integrados y descargables

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular): «Mejorar resolución» y «Quitar fondo» usan modelos locales. El paquete incluye los dos más ligeros (`u2netp` y `realesr-general-x4v3`, menos de 10 MB entre ambos); los mayores y la aceleración por GPU se descargan solo cuando la persona lo pide y acepta, con su peso y su licencia a la vista.
- **Consecuencia**: precisa ARQ-76, RNF-04 y FD-40 («sin modelos en el instalador» pasa a «sin modelos de más de 10 MB en total»), y ARQ-04 y D-111: es la tercera conexión de red, acotada al botón de descargar del panel de modelos, tras la feature `online`. Las imágenes no salen nunca de la máquina. Solo modelos con licencia permisiva y procedencia comprobada. Se aplica en la spec 084.

## D-116 Edición de imagen completa

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular): «Editar imagen» reúne ajustes, zonas, filtros y sombras. Se entregan el catálogo de filtros con nombres propios (D-40 lo aplazaba), y las sombras Brillo, Contorno y Curva (D-42 y D-27 las aplazaban) más Página flotante, En ángulo y Suelo. Primer plano y Fondo usan el modelo de quitar fondo (D-115) en lugar del `ClassicSegmenter`; Elemento se elige con un clic.
- **Consecuencia**: precisa FD-32, FD-34, FD-35, D-27, D-40 y D-42; sus fórmulas y valores quedan sujetos a la revisión visual del titular. Cambio aditivo del esquema 1. Se aplica en la spec 085.

## D-117 Modelos para reconstruir el fondo

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular, que pidió elegir por licencia): se usan los dos modelos propuestos, porque ambos tienen licencia permisiva verificada en su origen: MI-GAN (código y pesos MIT, `LICENSE-WEIGHTS` de su repositorio) como ligero y LaMa (Apache-2.0) como el de calidad. Son descargas opcionales: no entran en el límite de 10 MB integrados de D-115.
- **Consecuencia**: precisa D-115 con una tarea nueva. Se aplica en la spec 086.

## D-118 AVIF sin código nativo

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular, que pidió importar y exportar AVIF): se decodifica con `re_rav1d` (BSD-2-Clause, sin ensamblador, para no exigir `nasm`) y se codifica con `ravif` (BSD-3-Clause). El contenedor se lee con código propio: las alternativas son MPL-2.0 (`avif-parse`) o AGPL (`rav1d-safe`), fuera de la lista de licencias permitidas. AV1 es libre de regalías.
- **Consecuencia**: dos dependencias nuevas de la capa 4; WebP se importa con `image-webp`, ya en el árbol. Se aplica en la spec 087.

## D-119 Recorte de imágenes y Elementos sin «Iconos»

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular): una imagen se recorta sin destruirla, en rectángulo, redondeado, círculo o a mano, libre o con proporción. La categoría «Iconos», que no tenía contenido, sale de Elementos. Marca y Plantillas van al final de la barra, apagadas hasta que se implementen.
- **Consecuencia**: cambio aditivo del esquema 1. Se aplica en la spec 088.

## D-120 Efectos de texto

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular): un texto seleccionado tiene «Efectos»: Paralela, Brillo, Eco, Contorno, Fondo, Desalineado, Sin relleno, Neón y Distorsión, cada uno con sus parámetros; Curvar; y estilos listos con nombres propios.
- **Consecuencia**: precisa FD-43 y FD-35 (en texto); cambio aditivo del esquema 1. Se aplica en la spec 089.

## D-121 Publicación de binarios y coste de Actions

Fecha: 22 de noviembre de 2026.

- **Decisión** (titular): cada versión se publica con binarios de Linux y Windows, con instalador de Windows que registra `.gaf` y su icono; macOS cuando se pueda. Actions gasta lo mínimo.
- **Consecuencia**: una etiqueta compila y publica sin repetir pruebas; CI corre en el PR a `main`, con pruebas en Linux y solo clippy en Windows; las de Windows y macOS, a petición. Se aplica en la spec 090.

## D-122 Inicio nuevo y plantillas

Fecha: 23 de noviembre de 2026.

- **Decisión** (titular, con capturas de referencia): Inicio pasa a ser una portada con título, pestañas Inicio y Plantillas, un buscador de tipos de diseño y plantillas, accesos a las categorías de «Crear un diseño», recientes y plantillas por tipo. «Crear un diseño» se conserva, se abre desde esos accesos en su categoría y tiene vuelta a Inicio y a Plantillas. Plantillas es una pantalla propia, filtrable y con carga progresiva; una plantilla abre un diseño nuevo, editable por completo, que se guarda sin la marca de plantilla.
- **Formato**: una plantilla es un `.gaf` con el `template` de D-81 y su miniatura; no hay extensión aparte. Las plantillas van en `assets/templates/<categoría>/`, con nombres de archivo en inglés (`corner-coffee.es.gaf`); su nombre visible es la etiqueta `title:` y su idioma la etiqueta `lang:`. La búsqueda encuentra una plantilla con palabras de cualquiera de los dos idiomas. Las plantillas incluidas usan solo los colores de la marca.
- **Editor**: la pestaña Plantillas del riel ofrece las plantillas de proporción parecida a la página activa; una plantilla reemplaza el contenido de esa página y añade sus demás páginas detrás, en un paso que se deshace.
- **Detalle**: pulsar una plantilla en Inicio o en Plantillas abre un modal con su imagen, nombre, tipo y tamaño, «Personalizar esta plantilla» y plantillas similares; pulsar una similar la muestra en el mismo modal. Sin opciones de papel.
- **Consecuencia**: se activan la pestaña Plantillas de la barra superior y la del riel (cambia lo dicho de Plantillas en D-119). Se aplica en la spec 091.

## D-123 Barra superior y Ajustes

Fecha: 23 de noviembre de 2026.

- **Decisión** (titular): la barra superior de las pantallas de inicio muestra el logo horizontal de fondo oscuro en lugar del texto «Gafira», sin Proyectos, y con iconos en Inicio, Plantillas y Ajustes. Ajustes se rediseña: cada sección y opción mejora en diseño y uso, y el tema se elige con una miniatura de cada uno. Ajustes debe reflejar lo que Gafira hace hoy.
- **Consecuencia**: se aplica en la spec 092; el texto de Privacidad deja de decir que Gafira no se conecta a internet.
