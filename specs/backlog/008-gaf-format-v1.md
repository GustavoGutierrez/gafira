# 008 — GAF — Especificación del formato de proyecto de Gafira

> **Estado:** backlog — 2026-11-16 — propuesta técnica v1.0 del titular; pendiente de las decisiones de §17 y de la orden de empezar.

**Documento:** GAF-FORMAT-SPEC-001  
**Estado:** Propuesta técnica para implementación (v1.0)  
**Fecha:** 2026-11-05  
**Producto:** Gafira — editor visual y editor básico de video  
**Extensión:** `.gaf`  
**Tipo MIME propuesto:** `application/vnd.gafira.project+zip` (identificador propio, no registrado oficialmente)

> **Principio fundamental:** el proyecto editable es un documento estructurado, no una exportación renderizada. `.gaf` es una instantánea portátil de ese documento y sus dependencias, mientras que el editor web usa el mismo modelo lógico sin crear un ZIP en cada guardado.

## 1. Objetivo y alcance

Establecer un único formato nativo para conservar, transferir, restaurar y versionar diseños de Gafira que incluyan composiciones gráficas, múltiples páginas, textos, formas, vectores, imágenes, plantillas, animaciones y edición básica de video y audio.

El formato DEBE permitir que se abra un proyecto en otro equipo sin perder los elementos editables si se exportó en modo portátil completo. DEBE distinguir recursos originales, objetos del documento, medios proxy/caché y resultados finales exportados.

**Alcance inicial:** documentos estáticos multipágina; escenas con duración; secuencias audiovisuales; clips de video y audio; títulos; pistas; cortes; recortes; velocidad constante; volumen, fundidos y transiciones básicas; animaciones mediante keyframes; recursos externos o integrados; miniaturas; empaquetado; validación; migraciones. Funciones avanzadas (composición 3D, color grading profesional, multicámara, plugins arbitrarios, LUTs, VFR editable a nivel de fotograma) quedan fuera de v1, aunque el modelo debe permitir extensiones futuras.

### 1.1 Semántica normativa

- **DEBE / NO DEBE:** requisito obligatorio de interoperabilidad o seguridad.
- **DEBERÍA:** recomendación fuerte, con excepciones justificadas.
- **PUEDE:** capacidad opcional.
- **Perfil:** conjunto identificado de características compatibles; un consumidor debe advertir ante una característica desconocida.

## 2. Decisiones arquitectónicas (ADR resumidas)

| ID | Decisión | Justificación |
|---|---|---|
| GAF-ADR-01 | `.gaf` será un contenedor ZIP con manifest y documento JSON versionados. | Inspección, portabilidad y tooling multiplataforma. |
| GAF-ADR-02 | El **modelo de documento es la fuente de verdad**; renderizados y exports son derivados. | Edición sin pérdida de estructura. |
| GAF-ADR-03 | En la nube, documento/metadata/revisiones y blobs se almacenan separadamente. | Autosave eficiente y deduplicación. |
| GAF-ADR-04 | Exportación portátil completa incrusta todos los recursos redistribuibles necesarios, incluidos videos/audio originales. | Reapertura offline sin rutas rotas. |
| GAF-ADR-05 | Un perfil ligero puede almacenar referencias externas con identidad, integridad y capacidad de revinculación. | Evitar ZIP de muchos GB cuando se prefiera. |
| GAF-ADR-06 | Las rutas absolutas no son identificadores canónicos de assets. | Robustez entre OS y equipos. |
| GAF-ADR-07 | Video/audio se conservan **sin transcodificar** por defecto; proxies no reemplazan los masters. | Calidad y eficiencia. |
| GAF-ADR-08 | Los tiempos de edición se representan mediante enteros de ticks y una base de tiempo racional; no floats para el timeline. | Evitar deriva y facilitar 29.97/23.976 fps. |
| GAF-ADR-09 | Todo recurso tendrá ID estable, MIME, longitud y hash SHA-256 verificados cuando corresponda. | Identidad, deduplicación e integridad. |
| GAF-ADR-10 | Versiones de esquema, capacidades y migraciones explícitas. | Compatibilidad futura. |
| GAF-ADR-11 | Se hace guardado seguro y validación estricta de entradas comprimidas. | Prevención de corrupción y entradas hostiles. |
| GAF-ADR-12 | No se adjuntan exportaciones MP4/PNG/PDF automáticamente. | No inflar el proyecto con artefactos reproducibles. |

## 3. Perfiles de archivo y comportamiento esperado

**`portable-full` (predeterminado al «Exportar proyecto»):** incrusta todas las fuentes necesarias y legalmente redistribuibles, incluyendo fuentes tipográficas cuando la licencia permita su incrustación; si alguna dependencia no puede empaquetarse DEBE generar una advertencia explícita y marcar el proyecto como no completamente autónomo, o cancelar la exportación si el usuario exige autosuficiencia.

**`linked-light`:** incrusta documentos, edición y recursos pequeños configurables; permite referencias a archivos locales y recursos remotos. No promete uso offline. Siempre conserva metadatos de detección y revinculación.

**`hybrid`:** permite inclusión por recurso, p. ej. imágenes sí y video original externo; marca dependencias faltantes.

**`cloud-snapshot`:** instantánea lógica de una revisión; usada en la nube, no necesariamente un ZIP. Para descarga se convierte a uno de los perfiles anteriores.

La UI DEBE mostrar, antes de guardar, tamaño estimado, número de assets vinculados, dependencias no disponibles y si el proyecto podrá abrirse sin conexión.

## 4. Diseño físico del paquete

```text
mi-proyecto.gaf                           # ZIP / ZIP64 cuando proceda
├── mimetype                               # texto ASCII, sin compresión
├── manifest.json                          # tabla de contenido/recursos/capacidades
├── document.json                          # documento editable completo
├── assets/
│   ├── images/<sha256>.png
│   ├── images/<sha256>.jpg
│   ├── video/<sha256>.mp4
│   ├── audio/<sha256>.wav
│   ├── vectors/<sha256>.svg
│   └── fonts/<sha256>.woff2               # solo si licencia lo permite
├── previews/
│   ├── cover.webp
│   ├── pages/<page-id>.webp
│   └── video/<sequence-id>.jpg
├── proxies/                                # opcional, derivados regenerables
│   ├── video/<asset-id>.mp4
│   └── audio/<asset-id>.waveform.bin
├── versions/                               # versiones del proyecto, dentro del archivo (D-67, 02 §7.1)
│   ├── index.json                         # lista de versiones nombradas y automáticas
│   └── <version-id>/                      # documento de ese momento; recursos compartidos por hash con assets/
└── metadata/
    └── provenance.json                    # opcional, sin secretos
```

- `mimetype` DEBE ser la primera entrada ZIP, sin compresión, con el texto exacto `application/vnd.gafira.project+zip` en ASCII UTF-8 sin BOM y sin salto final.
- `manifest.json` y `document.json` son obligatorios; orden del resto de entradas no significativo.
- Todas las rutas internas usan `/`, codificación UTF-8, rutas relativas; NO DEBEN contener `..`, barra inicial, letras de unidad o segmentos vacíos sospechosos.
- El lector DEBE soportar ZIP estándar con `STORE` y `DEFLATE`, y ZIP64 para archivos grandes. Métodos de compresión adicionales no son obligatorios para v1.
- Archivos multimedia ya comprimidos (MP4, WebM, JPEG, PNG, etc.) DEBERÍAN almacenarse con `STORE` si DEFLATE no reduce su tamaño.
- Identificadores basados en hash del contenido para blobs; nombres lógicos/visualización no dependen del nombre físico. Dos assets lógicos pueden referenciar el mismo blob.
- No asumir que los paths dentro del ZIP son URLs; usar `assetId` y el manifest como única resolución autorizada.

## 5. Manifest y metadatos

`manifest.json` es la tabla de recursos y la declaración de compatibilidad. Campos obligatorios:

- `format`: literal `gafira-project`.
- `formatVersion`: versión semántica del formato contenedor (inicial `1.0.0`).
- `schemaVersion`: versión del documento (inicial `1`).
- `projectId`, `createdAt`, `updatedAt`, `generator`.
  - *Nota (D-63)*: `projectId` es el UUID del proyecto que se genera al crearlo y que 02 §7.1 ya guarda en el manifiesto como `project_id`. Es ese mismo UUID, porque la recuperación lo usa (las versiones viven dentro del `.gaf`, D-67); «Guardar copia» asigna uno nuevo y «Guardar como» conserva el del proyecto. Falta decidir si importar un `.gaf` conserva su `projectId` o asigna uno nuevo.
- `profile`: `portable-full | linked-light | hybrid`.
- `document`: ruta y hash del documento.
- `features.required[]` y `features.optional[]`: capacidades identificadas (ej. `video.timeline.v1`).
- `resources[]`: `id`, `kind`, `mimeType`, `byteLength`, `sha256` (si se materializó), `storage`, `path` y/o `external`, `originalFilename` opcional, `metadata`.
- `packageCompleteness`: `complete | partial`; `missingResources[]` para recursos no empaquetados ni accesibles en exportación.

```json
{
  "format": "gafira-project",
  "formatVersion": "1.0.0",
  "schemaVersion": 1,
  "projectId": "project-01",
  "createdAt": "2026-11-05T12:00:00Z",
  "updatedAt": "2026-11-05T12:05:00Z",
  "generator": { "app": "Gafira", "version": "0.1.0" },
  "profile": "portable-full",
  "features": {
    "required": ["design.pages.v1", "video.timeline.v1", "animation.keyframes.v1"],
    "optional": ["previews.v1"]
  },
  "document": { "path": "document.json", "sha256": "<sha256-real>" },
  "packageCompleteness": "complete",
  "missingResources": [],
  "resources": [
    {
      "id": "asset-video-01",
      "kind": "video",
      "mimeType": "video/mp4",
      "byteLength": 12345678,
      "sha256": "<sha256-real>",
      "storage": "embedded",
      "path": "assets/video/<sha256-real>.mp4",
      "originalFilename": "clip-original.mp4",
      "metadata": {
        "width": 1920, "height": 1080,
        "durationTicks": "900000", "timebase": { "num": 1, "den": 90000 },
        "videoCodec": "avc1", "hasAudio": true
      }
    }
  ]
}
```

Los hashes ilustrativos de los ejemplos NO son válidos para un paquete real. Se calculan sobre los bytes definitivos; `document.json` no contiene el hash del manifest para evitar dependencia circular.

### 5.1 Recursos externos

`storage` admite `embedded`, `external-local`, `external-url`, `cloud-managed` (este último es una referencia lógica de la plataforma; no un enlace público durable). Un recurso local puede incluir un `relativePath` relativo al directorio del `.gaf`, y un `originalPathHint` **opcional y privado** para diagnóstico, nunca una dependencia exclusiva. No incluir tokens ni URLs firmadas expuestas o expiring en el archivo. Las URL remotas deben poder resolverse mediante autenticación del usuario y controles de permisos externos al archivo.

En modo portable, todas las dependencias necesarias se incrustan o se declaran no empaquetables. La resolución de un recurso es: contenido interno válido → recurso vinculado permitido y validado → solicitud de revincular → placeholder explícito (nunca sustitución silenciosa).

## 6. Modelo lógico común para web y `.gaf`

El documento DEBE ser independiente del motor de renderizado (Canvas 2D, SVG, WebGL, WebGPU o backend de video). El esquema compartido identifica:

1. **Proyecto:** `id`, `name`, configuración, perfiles de color y metadata.
2. **Páginas/escenas:** dimensiones, fondo, duración opcional, elementos ordenados y secuencias asociadas.
3. **Elementos:** `id`, `type`, `parentId`, `zIndex/order`, propiedades de posición/transformación, visibilidad, bloqueo, composición, estilos y referencias a assets.
4. **Assets:** recursos inmutables con versiones y metadatos; ubicados fuera del árbol de objetos.
5. **Secuencias de video:** formato audiovisual, pistas, clips, transiciones, marcadores y keyframes.
6. **Animaciones:** propiedades animables y curvas interpoladas sobre un tiempo local explícito.
7. **Extensiones:** espacios de nombres para datos desconocidos y características opcionales.

Los elementos soportados inicialmente son `shape`, `path`, `text`, `image`, `group`, `mask`, `audio`, `video`, `frame` y `component-instance` (este último opcional). Deben preservar posición/tamaño, pivote, transformada afín, opacidad, recorte, filtros y blending, cuando el motor los soporte.

Para texto: contenido, spans, familias y sustituciones de tipografía, métricas y estilos, idioma, alineación, espaciado. Para SVG externos: conservar fuente original y representación editable cuando sea compatible; **no interpretar scripts ni contenido activo** del SVG.

### 6.1 Unidades, geometría y color

- Coordenadas de diseño expresadas en unidades lógicas del lienzo (ej. px de diseño), no píxeles de pantalla del dispositivo.
- Dimensiones y matrices numéricas documentadas; valores finitos y límites de validación.
- Orden de apilamiento explícito; la posición visual no depende del orden de claves JSON.
- Color con formato definido (mínimo `srgb` RGBA); soporte de perfiles ICC como extensión futura y referencia opcional a perfil embebido.
- Las transformaciones no deben rasterizar ni mutar destructivamente los recursos originales.

## 7. Especificación de video y audio

### 7.1 Principios

El proyecto guarda las **decisiones de edición** y referencias a los masters; no guarda el render MP4 como sustituto de la timeline. Un proyecto puede tener páginas estáticas y secuencias audiovisuales. Una página/escena puede servir como composición gráfica superpuesta a un video.

**Entidad `sequence`:** `id`, `name`, `width`, `height`, `pixelAspectRatio`, `fps` racional (`num`/`den`), `durationTicks`, `timebase`, `background`, `audioSampleRate`, `channels`, `tracks[]`, `markers[]`.

**Entidad `track`:** `id`, `kind` (`video|audio|overlay|caption`), `name`, `enabled`, `locked`, `muted`, `solo` (cuando aplique), `order`, `items[]`. Se admiten solapes en pistas distintas y, cuando la semántica lo indique, composiciones sobre una misma pista; no inferir automáticamente qué elemento gana.

**Entidad `clip`:** `id`, `assetId` o `sceneId`, `timelineStartTicks`, `timelineDurationTicks`, `sourceInTicks`, `sourceOutTicks`, `sourceTimebase`, `playbackRate` racional, `reverse` opcional, `transform`, `crop`, `opacity`, `volumeDb` y `effects[]`. `timelineDurationTicks` determina duración de la instancia en la secuencia, distinta de la duración total del medio.

**Entidades adicionales:** `transition`, `keyframeTrack`, `marker`, `caption`, `audioEnvelope`, `generator` (fondo, título, color).

### 7.2 Semántica temporal precisa

- Todos los tiempos en el documento se expresan como enteros **decimales serializados en string** (`"12345"`) para que JS no pierda precisión cuando exceden `Number.MAX_SAFE_INTEGER`.
- `timebase = {"num": 1, "den": 90000}` significa 90 000 ticks/segundo (valor ilustrativo, no requisito universal). La secuencia declara su propia base temporal y cada source puede declarar otra.
- `fps = {"num": 30000, "den": 1001}` representa aproximadamente 29,97 cuadros/segundo. El FPS controla el muestreo/render, no convierte tiempos a floats.
- Tiempo absoluto: `segundos = ticks × timebase.num / timebase.den`; toda conversión se realiza racionalmente, con redondeo documentado en límites.
- Intervalos de clips y marcas son semiabiertos `[start, start+duration)`; `durationTicks` es no negativo y `sourceOutTicks > sourceInTicks` salvo tipos de elemento sin fuente.
- Para velocidad positiva constante: duración editorial = duración fuente / velocidad, tras conversión de bases temporales y reglas de redondeo. `reverse` cambia el sentido del muestreo; no invierte la posición temporal del clip.
- Video VFR puede importarse y conservarse como fuente VFR; el editor decodifica según timestamps de presentación y el render muestrea según FPS de salida. No asumir que frame index = timestamp.
- Los keyframes se definen con dominio temporal explícito (`sequence`, `clip` o `source`), propiedad (ruta tipada) e interpolación (`hold`, `linear`, `bezier`).
- Audio se mezcla por tiempo real de muestra; `audioSampleRate` determina la renderización, no implica recomprimir el master al abrir.

### 7.3 Edición básica soportada

| Capacidad | Persistencia |
|---|---|
| Cortar, dividir, recortar | Clips individuales con rangos de origen y posición temporal. |
| Reordenar y superponer | Pistas ordenadas y tiempos de inicio. |
| Redimensionar/reencuadrar/rotar | Transformación y crop no destructivos. |
| Velocidad constante y reversa | `playbackRate` y `reverse`, con validación de soporte. |
| Transiciones básicas | Objetos con tipo, participantes, inicio, duración y parámetros. |
| Fundidos de audio y volumen | Envolventes/keyframes en dB o ganancia, especificado por propiedad. |
| Títulos, subtítulos y stickers | Elementos gráficos y pistas caption con intervalos. |
| Animación de opacidad/posición/escala | Keyframes independientes del motor. |
| Silenciar/separar audio | Estado del clip y vínculos estables a pistas/segmentos de audio. |
| Música de fondo y voice-over | Assets de audio y clips temporizados. |
| Miniaturas, waveform, proxies | Caché opcional regenerable y enlazado al hash del master. |
| Exportar video | Perfil de render externo al modelo, sin integrar MP4 final automáticamente. |

### 7.4 Proxies, calidad y portabilidad

- Master siempre autoritativo. Proxy tiene `derivedFromAssetId`, `sourceSha256`, parámetros de generación y se invalida si cambia el origen.
- Proxies/waveforms/thumbs se excluyen por defecto del ZIP portable para controlar el tamaño, salvo opción «incluir proxies para rendimiento».
- El paquete portátil completo **incluye master de cada clip utilizado** y audio asociado siempre que permisos/licencias lo permitan. Esto puede producir archivos grandes (GB); usar ZIP64, copia/streaming sin cargarlo todo en RAM y progreso de escritura.
- `linked-light` y `hybrid` pueden incluir únicamente `assetId`, hash, nombre original y pistas de ubicación/revinculación.
- Los codecs soportados dependen de navegador, SO, infraestructura y capacidades de decodificación; el formato GAF no impone un codec universal. El importador DEBE reportar cuando una fuente no puede decodificarse. Una versión proxy compatible PUEDE facilitar previsualización sin sustituir el original.
- Una exportación final de MP4/WebM es un artefacto derivado; metadatos de exportación pueden guardarse como preset sin incrustar el archivo final.

### 7.5 Esquema conceptual de secuencia

```json
{
  "id": "seq-main",
  "name": "Reel tropical",
  "width": 1080,
  "height": 1920,
  "fps": { "num": 30000, "den": 1001 },
  "timebase": { "num": 1, "den": 90000 },
  "durationTicks": "900000",
  "audioSampleRate": 48000,
  "channels": 2,
  "tracks": [
    {
      "id": "video-track-1",
      "kind": "video",
      "order": 0,
      "items": [
        {
          "type": "clip",
          "id": "clip-01",
          "assetId": "asset-video-01",
          "timelineStartTicks": "0",
          "timelineDurationTicks": "450000",
          "sourceInTicks": "90000",
          "sourceOutTicks": "540000",
          "sourceTimebase": { "num": 1, "den": 90000 },
          "playbackRate": { "num": 1, "den": 1 },
          "reverse": false,
          "transform": { "x": 0, "y": 0, "scaleX": 1, "scaleY": 1, "rotationDeg": 0 },
          "effects": [],
          "keyframes": []
        }
      ]
    },
    {
      "id": "audio-track-1",
      "kind": "audio",
      "order": 1,
      "items": [
        {
          "type": "clip",
          "id": "music-01",
          "assetId": "asset-audio-01",
          "timelineStartTicks": "0",
          "timelineDurationTicks": "900000",
          "sourceInTicks": "0",
          "sourceOutTicks": "900000",
          "sourceTimebase": { "num": 1, "den": 90000 },
          "playbackRate": { "num": 1, "den": 1 },
          "volumeDb": -6,
          "effects": [],
          "keyframes": []
        }
      ]
    }
  ],
  "markers": []
}
```

La secuencia anterior ilustra objetos independientes y referencias. En un validador real DEBE verificarse que los recursos referenciados existan y que duración y rango fuente sean consistentes con el medio y la velocidad.

### 7.6 Transiciones y composición de escena

Una transición tendrá `id`, `kind` (ej. `crossfade`, `dip-to-black`, `wipe`), `fromClipId`, `toClipId`, `startTicks`, `durationTicks`, `params`. Para evitar ambigüedad, su intervalo DEBE pertenecer al área de solape efectivo o a una región explícitamente definida por la transición; el render no debe inventar fotogramas ausentes. Implementaciones con modelos de transición diferentes deberían migrarlos a una representación común.

Una escena gráfica se puede referenciar desde un clip `sceneId`, con `localTime` y duración, de forma que títulos/animaciones se dibujen en la misma timeline. Objetos de diseño estáticos también pueden tener `visibleFromTicks` y `visibleUntilTicks` en contextos audiovisuales.

## 8. Ejemplo simplificado de `document.json`

```json
{
  "schemaVersion": 1,
  "id": "project-01",
  "name": "Campaña Gafira",
  "colorSpace": "srgb",
  "pages": [
    {
      "id": "page-01",
      "name": "Portada",
      "width": 1080,
      "height": 1350,
      "unit": "px",
      "background": { "type": "solid", "color": "#F6F1E7" },
      "elements": [
        {
          "id": "image-01",
          "type": "image",
          "assetId": "asset-image-01",
          "x": 80, "y": 60, "width": 920, "height": 950,
          "rotationDeg": 0, "opacity": 1, "visible": true
        },
        {
          "id": "title-01",
          "type": "text",
          "text": "NATURALEZA",
          "x": 120, "y": 1060,
          "style": { "fontFamily": "Inter", "fontSize": 72, "fill": "#25443C" }
        }
      ]
    }
  ],
  "sequences": [
    {
      "id": "seq-01",
      "name": "Video promocional",
      "width": 1080, "height": 1920,
      "fps": { "num": 30, "den": 1 },
      "timebase": { "num": 1, "den": 90000 },
      "durationTicks": "270000",
      "audioSampleRate": 48000,
      "channels": 2,
      "tracks": [],
      "markers": []
    }
  ],
  "metadata": { "tags": ["social", "campaign"] }
}
```

El ejemplo no sustituye JSON Schema formal; los componentes de implementación DEBEN compartir tipos y validadores. El manifest registra todos los `assetId` referenciados.

## 9. Guardado, apertura y recuperación

### 9.1 Guardado en nube

- Mantener estado normalizado por documento, páginas, objetos, secuencias y assets.
- Autosave de revisiones o parches incrementales, con control de concurrencia (revisionId/baseRevision), confirmación de persistencia y reintento idempotente.
- Enviar blobs por separado, por contenido hash, mediante carga multipart/resumable para video grande; no serializar ni subir cada video en cada cambio de texto.
- Mantener snapshots periódicos y log de operaciones con retención acotada; no confundir historial/deshacer con snapshot portable.
- PostgreSQL (metadata, revisiones, relaciones, permisos) + object storage como R2 (masters, derivados) es una arquitectura sugerida, no una dependencia del formato.
- IndexedDB PUEDE conservar borradores y cola offline; sincronización con detección y resolución de conflictos.

### 9.2 Exportación `.gaf`

1. Congelar una revisión consistente del documento; recopilar su **cierre transitivo** de assets (clips, escenas, grupos, fuentes autorizadas, miniaturas seleccionadas).
2. Validar referencias, permisos/licencias, disponibilidad y estimar tamaño.
3. Construir manifest y serializar `document.json` determinísticamente; crear hashes sobre bytes serializados.
4. Escribir ZIP en streaming a temporal, preferir `STORE` para multimedia ya comprimida; registrar progreso y soporte de cancelación.
5. Verificar integridad, rutas, conteos, ZIP64 y estado de `packageCompleteness`.
6. Completar escritura y reemplazar destino de forma segura cuando el sistema lo permita; evitar sobrescribir un archivo válido antes de confirmar el nuevo.

### 9.3 Importación `.gaf`

1. Detectar tipo y magic ZIP; validar `mimetype` y tamaño declarado.
2. Inspeccionar directorio central, topología, entradas duplicadas, rutas inválidas, symlinks, métodos de compresión y límites (ver sección 12).
3. Validar manifest, versión/capacidades y esquema JSON; calcular/verificar hashes al abrir o materializar recursos.
4. Resolver IDs/referencias y crear placeholders explícitos para medios ausentes.
5. Cargar previews y datos mínimos primero; decodificar video/audio bajo demanda y construir proxies si conviene.
6. Abrir en modo seguro de solo lectura si la versión es futura o faltan features obligatorias; NO guardar encima de un archivo que no puede interpretarse íntegramente.

### 9.4 Cambio de versión

- `formatVersion` controla contenedor/manifest y `schemaVersion` el documento.
- Migraciones explícitas, idempotentes cuando proceda, desde N a N+1; crear backup antes de conversión irreversible.
- Preservar bloques de extensiones desconocidas si la transformación lo permite, sin ejecutarlas; rechazar características requeridas desconocidas.
- Añadir fixtures de versiones anteriores a la suite de compatibilidad.

## 10. Seguridad, privacidad y licencias

- Rechazar **ZIP Slip**, rutas absolutas, symlinks y hardlinks no autorizados, entradas duplicadas/colisionantes (incluida comparación case-fold cuando se materialice sobre FS case-insensitive).
- Límites configurables: tamaño comprimido, tamaño total descomprimido, ratio de compresión, número de entradas, profundidad de paths, longitud JSON, número de nodos, tracks, clips, fotogramas clave y duración declarada. Abortos controlados antes de asignar memoria masiva.
- JSON parseado con esquemas estrictos y límites; no evaluar código, expresiones ni scripts dentro de proyectos.
- No procesar HTML/SVG sin sanitización robusta; tratar fuentes y multimedia como entradas no confiables de decodificadores aislados o actualizados.
- Proteger contra SSRF y exfiltración en referencias externas: restringir protocolos/hosts autorizados y pedir consentimiento antes de descargar recursos remotos.
- No guardar tokens, contraseñas, cookies ni URLs firmadas de corta duración. Considerar **omitir rutas locales absolutas** por privacidad salvo aceptación explícita.
- Incrustar tipografías, música, videos, stock y otros medios únicamente conforme a licencias y permisos. Si no se pueden redistribuir, advertir y proporcionar un mecanismo de resolución con licencia en la cuenta receptora.
- Autoría, atribuciones y procedencia se conservan en metadata, pero no sustituyen una licencia.
- ZIP no implica cifrado. Cifrado y firma son extensiones futuras explícitas; no prometer confidencialidad ni autenticidad en v1.

## 11. Rendimiento

- Usar streaming y acceso aleatorio al directorio ZIP; evitar cargar assets de GB en memoria.
- Imágenes/video se decodifican a demanda; cachés de RAM/IndexedDB controladas por presupuesto.
- Proxies de video a resolución baja y waveform de audio por hash del original; reconstrucción asíncrona sin bloquear apertura.
- Deduplicar blobs por SHA-256 y mantener referencias lógicas múltiples.
- No recomprimir multimedia sin ganancia y no transcodificar masters de manera automática.
- En proyectos enormes, ofrecer modo `linked-light` y alertar sobre tamaño/coste de portabilidad.
- Al modificar solo metadatos, permitir reusar bytes de blobs sin redecodificarlos; aunque se regenere el ZIP, no se necesita reencodificar video ni imágenes.

## 12. Límites y políticas de validación sugeridos

Los valores concretos deben ser configurables según despliegue/plataforma; como política **inicial sugerida** para importación web (no límites intrínsecos de `.gaf`):

- Manifest individual hasta 16 MiB y documento hasta 128 MiB; permitir configuración server-side para proyectos grandes.
- Máximo inicial de 100 000 entradas; límite de nombre de ruta 512 caracteres y profundidad 16.
- Ratio de expansión sospechoso >100:1 exige inspección/rechazo según tipo de archivo y política; nunca confiar solo en el ratio.
- Control de tamaño total expandido y tamaño individual antes de crear buffers. Un video de varios GB puede ser legítimo y no debe descargarse entero a RAM.
- Verificar coherencia de duración, FPS/base temporal, divisiones por cero, rangos, IDs duplicados, rutas y hashes.

Estas cifras son parámetros operativos de ejemplo y DEBEN ajustarse con pruebas de rendimiento, carga y seguridad.

## 13. Compatibilidad e interoperabilidad

- Exportación de imagen (PNG, JPEG, WebP, SVG según capacidades) y documentos (PDF) son productos finales, no sustitutos de `.gaf`.
- Exportación de video (ej. MP4/WebM) renderiza pistas, efectos, títulos y audio; el archivo final se produce separado del paquete.
- Export/import de OpenTimelineIO puede ofrecerse mediante adaptador para pistas, clips, referencias y transiciones, **sin asumir equivalencia completa** (especialmente efectos y composición gráfica).
- El motor de UI no debe determinar el esquema; documentar semánticas y pruebas de render reproducible por fixture.
- El soporte de codec se negocia por plataforma, se informa al usuario y no debe confundirse con soporte del formato de proyecto.

## 14. Roadmap de implementación

### Fase 1 — Fundamentos del documento
- `@gafira/document-schema` o paquete equivalente: tipos TypeScript, Zod/JSON Schema, identificadores y validadores.
- Modelo normalizado para páginas, grupos, textos, vectores, assets y elementos editables.
- Manifest, ZIP import/export, hashes, perfiles `portable-full` y `linked-light`.
- Pruebas round-trip con imágenes y documentos multipágina.

### Fase 2 — Video esencial
- Secuencia y pistas, clips con rangos de origen y destino, tiempos racionales, recorte/división, audio, títulos y keyframes.
- Decodificador/adaptador por capacidad (WebCodecs si está disponible y es adecuado; fallback según plataforma).
- Proxies, miniaturas y waveform regenerables.
- Exportación del documento audiovisual a archivo final por pipeline separado.

### Fase 3 — Escala y experiencia
- Autosave incremental, revisión/conflictos, recuperación offline, deduplicación de assets.
- Empaquetado streaming ZIP64, validación intensiva, reconexión de medios y perfiles híbridos.
- Migraciones y compatibilidad entre versiones.

### Fase 4 — Intercambio y funciones avanzadas
- Adaptador OTIO, presets de render, recursos compartidos entre documentos.
- Mejoras en color, efectos, subtítulos, componentes reutilizables y gestión de licencias.

## 15. Criterios de aceptación verificables

1. Un `.gaf` de diseño estático empaquetado en `portable-full` abre offline en otro equipo preservando páginas, elementos y estilos editables.
2. Un `.gaf` con video/audio empaquetado abre offline y preserva clips, recortes, pistas, sincronización, efectos admitidos y keyframes sin recomprimir originales durante el guardado.
3. Exportar e importar conserva IDs, orden de capas, tiempo racional, assets y estados de selección/edición salvo transformaciones de migración documentadas.
4. El guardado no incluye por defecto render finales ni caches regenerables.
5. Un master de video de varios GB se empaqueta/desempaqueta mediante streaming/ZIP64 sin requerir RAM proporcional al tamaño del master.
6. En modo vinculado, mover o borrar un asset produce señal clara de recurso faltante y opciones de revinculación, sin sustituir silenciosamente.
7. Un `.gaf` manipulado con `../`, rutas absolutas, symlinks, duplicados, hashes erróneos o expansión exagerada es rechazado de forma controlada.
8. El lector detecta versiones futuras/capacidades obligatorias no soportadas y evita sobrescribir o truncar información desconocida.
9. Una edición de texto no obliga a retransmitir blobs de imágenes, video ni audio a la nube.
10. Los medios externos restringidos por licencia no se empaquetan clandestinamente: la UI comunica las limitaciones antes de exportar.
11. Los proyectos con `fps=30000/1001` y `fps=24000/1001` no presentan deriva acumulativa por uso de tiempo flotante.
12. Los resultados de exportación audiovisuales son independientes del documento editable, con presets reproducibles.

## 16. Casos de prueba mínimos

- Diseño de 1 página con fotografía, texto y máscara; round-trip.
- Diseño de 50 páginas con 1 imagen reutilizada en varias; deduplicación.
- Video vertical de 30 segundos: dos clips, corte, crossfade, texto animado, música y fade-out; round-trip y comparación de duración.
- Video VFR + pista 48 kHz + secuencia 29,97 fps; muestreo y sincronización.
- Dos referencias al mismo master con tramos de origen diferentes.
- Master MP4 grande (ZIP64), cancelación, archivo temporal y recuperación del destino previo.
- Fuente sin autorización de incrustación; exportación informa `packageCompleteness=partial`.
- Recurso remoto inaccesible; placeholder + revinculación sin alterar timing.
- MIME falso, extensión falsa, hash corrupto, ZIP bomb, path traversal, JSON profundo, ids duplicados.
- Documento nuevo abierto con lector anterior: modo solo lectura/mensaje de incompatibilidad.

## 17. Decisiones deliberadamente pendientes

Las siguientes no impiden implementar v1, pero exigen ADR antes de ampliar capacidades:

- Lista formal de codecs objetivo por navegador/OS/servicio de render.
- Compatibilidad exacta con subtítulos (WebVTT, SRT, TTML) y su traducción al modelo nativo.
- Estrategia de edición colaborativa (operaciones, CRDT u otro mecanismo) independiente del contenedor.
- Perfil de exportación encriptada/firma y distribución comercial de plantillas.
- Perfiles de color de producción y HDR.
- Semántica avanzada para remapeo de velocidad variable y efectos temporales.
- Qué hace cada perfil (§3) con una fuente del usuario incrustable (por ejemplo, si `linked-light` la vincula y `portable-full` la empaqueta en `fonts/`). *Nota (D-60)*: la regla de licencia ya está decidida (incrustar según los bits `fsType` de OpenType; si no se permite, vincular por nombre y avisar de la fuente que falta al abrir); esta decisión solo fija el comportamiento por perfil (§3, §4; D-67: hay un solo formato).

## 18. Referencias técnicas

1. PKWARE APPNOTE (formato ZIP/ZIP64): https://support.pkware.com/pkzip/appnote
2. Common ZIP Specification (robustez e interoperabilidad): https://commonzip.org/spec/
3. W3C WebCodecs (API del navegador; no define codecs universalmente soportados): https://www.w3.org/TR/webcodecs/
4. OpenTimelineIO (modelo de clips, pistas, transiciones y referencias): https://opentimelineio.readthedocs.io/en/latest/

---

**Resumen de la decisión:** `.gaf` debe persistir **la edición y sus recursos**, no únicamente el resultado rasterizado o un MP4. El documento estructurado es la fuente de verdad; los videos y audios originales se empaquetan en el perfil portátil, se pueden vincular en el perfil ligero, y los renders/proxies son derivados. La edición en nube conserva este mismo esquema lógico sin crear un ZIP en cada autosave.
