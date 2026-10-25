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
- **Procedencia de la imagen** (lectura técnica del 24 de octubre de 2026, no es asesoría legal): el titular generó `assets/splash_screen.jpg` con ChatGPT Plus (generación de imágenes de OpenAI). Base legal: Términos de uso de OpenAI (vigentes desde el 1 de enero de 2026, https://openai.com/policies/row-terms-of-use/), apartado «Ownership of content», que cede al usuario todos los derechos sobre el *Output* y permite el uso comercial sin atribución ni regalías. Condiciones: no presentarla como obra humana, así que **Acerca de / créditos** la muestran como «generada con IA (OpenAI)»; sin garantía de unicidad ni de no infracción; probablemente no sea protegible por derechos de autor (U.S. Copyright Office, 2025), por lo que no da exclusividad. La copia del repositorio no lleva metadatos C2PA. Cuando se introduzca `assets/LICENSES.toml` (ARQ-75) se registrará su entrada con los campos: origen, herramienta, base de licencia, aviso de IA, nota de derechos de autor, procedencia y comprobación de similitud.
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
- Aplicado a 01, 03 y 06 (ver «Efecto sobre las especificaciones») cuando se promuevan.

## D-13 `gafira-cli validate` pasa a `gafira-format`

Fecha: 24 de octubre de 2026.

- **Decisión**: el subcomando `gafira-cli validate` se implementa en el change `gafira-format`, no antes. Confirma la pregunta 5 de la propuesta archivada de C2b (`domain-doc`): sin lector de archivos `.gafira` no hay nada que validar desde la línea de comandos.
- **Archivos archivados**: no se editan; esta decisión es el registro de la confirmación.
- Aplicado al change `gafira-format` cuando se cree.

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
4. Marca (D-04): el coordinador hace una búsqueda preliminar (en curso); un abogado debe confirmarla antes del lanzamiento.
5. ~~Verificar con `cargo-deny` las licencias marcadas VERIFICAR (incluido el decodificador MP3)~~ (**resuelto** el 24 de octubre de 2026: pasan 72 de 76; `zip`, `ureq`, `directories` y `ort` se corrigen con features o cambiando de crate, ver 02 §9.2; las patentes de MP3 siguen en VERIFICAR).
6. ~~Registrar el origen, la autoría y la licencia de `assets/splash_screen.jpg` en `assets/LICENSES.toml` (D-09)~~ (**resuelto** el 24 de octubre de 2026: generada con ChatGPT Plus; los Términos de uso de OpenAI ceden al titular los derechos sobre el *Output* y permiten el uso comercial sin atribución ni regalías; se muestra como «generada con IA (OpenAI)» en Acerca de / créditos; sin garantía de unicidad ni de no infracción y probablemente sin derechos de autor exclusivos; ver D-09). D-10 cubre solo el audio de `assets/musics/`. Quedan como VERIFICAR para el titular: (a) búsqueda inversa de imagen (Google Lens, TinEye) y revisión visual de logos, textos o firmas; (b) conservar fuera del repositorio la descarga original con C2PA como prueba de procedencia; (c) anotar la fecha exacta de generación, hoy desconocida; (d) crear la entrada en `assets/LICENSES.toml` al introducir ese archivo (ARQ-75).
7. ~~Aprobar el nombre «Pantalla de arranque» / «Startup screen» y las cifras de D-09~~ (**aprobado** el 24 de octubre de 2026; ver D-09).
8. D-10: ~~revisar las 9 pistas con voz detectada y volver a comprobar `underscore-05`, `-06` y `-10`~~ (**hecho**: las 9 regeneradas y comprobadas, 40/40 sin voz); queda al implementar la pestaña Sonidos convertir el audio, registrarlo en `assets/LICENSES.toml` y `THIRD_PARTY_NOTICES.md` y mostrar «MiniMax-Music3» y el aviso de contenido generado por IA.
