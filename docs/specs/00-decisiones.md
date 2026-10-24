# Decisiones de producto de Gafira

Registro de las decisiones del titular que resuelven las preguntas abiertas de las especificaciones 01 a 05. Fecha: 24 de octubre de 2026. Este documento prevalece sobre cualquier texto contrario de 01 a 05 hasta que estas se actualicen.

Aviso: las referencias legales de este documento son lectura técnica de fuentes públicas, no asesoría legal. Las cifras y fechas marcadas VERIFICAR no se comprobaron en una fuente primaria.

## D-01 Licencia, modelo comercial y activación (Q-01, Q-11, Q-12)

- **Licencia**: propietaria, código cerrado. Se reutiliza el contrato de Tracewright (`LICENSE` en inglés y `LICENCIA.md` en español, prevalece el español) con tres adaptaciones:
  1. Nombre del producto: Gafira.
  2. Cláusula «Sus resultados»: el usuario es titular de los diseños, imágenes y videos que cree, no solo de archivos vectoriales.
  3. Cláusula nueva: permiso para usar las plantillas, fuentes y recursos incluidos dentro de las creaciones del usuario.
- **Modelo**: todo de pago, con prueba gratuita (igual que Tracewright). Esto **sustituye** el principio del PRD «sin funciones esenciales retenidas tras un plan de pago». La duración y los límites de la prueba están por definir.
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

## Efecto sobre las especificaciones

| Spec | Qué cambiar |
|---|---|
| 01 | Quitar el principio de no retener funciones esenciales; reflejar D-01, D-03, D-07; cerrar Q-01, Q-02, Q-03, Q-04, Q-05, Q-06, Q-07, Q-08, Q-09, Q-11, Q-12, Q-13, Q-16 |
| 02 | D-01 (conexión de licencia), D-02, D-05, D-06 (quitar `update-check`), D-07; corregir las incoherencias listadas en el informe de redacción |
| 03 | Pantalla de activación y prueba, estados de licencia; sin pantalla de actualizaciones; modo de licencia en Ajustes |
| 04 | PPTX en Fase 2 como imágenes |
| 05 | Linux x86_64 con `.deb`, `.rpm`, `.tar.gz`; sin AppImage ni aarch64; sin actualizador; códecs según D-02; MP3 y AVI según D-07; aviso de marca (D-04) |

## Pendientes del titular

1. Definir duración y límites de la prueba gratuita, y el precio.
2. Adaptar `LICENSE` y `LICENCIA.md` y escribir el `PRIVACY.md` de Gafira.
3. Completar la lista de seguridad del `license-hub` y desplegarlo.
4. Hacer la búsqueda de marca de D-04.
5. Verificar con `cargo-deny` las licencias marcadas VERIFICAR (incluido el decodificador MP3).
