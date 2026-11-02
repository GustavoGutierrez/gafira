# 02 — Arquitectura del software de Gafira

| Campo | Valor |
|---|---|
| Documento | Especificación 02 de 05 |
| Estado | Borrador para revisión |
| Alcance | Crates, capas, traits, patrones, modelo de documento, render, persistencia, UI, dependencias y licencias, pruebas, hoja de ruta técnica |
| Fuentes | `specs/00-decisiones.md` (decisiones D-01 a D-45 del titular, prevalecen), `docs/research/PRD.md`, `docs/research/spec_software.md`, `docs/manuals/marca/manual-gafira.md`, handoff consolidado |
| Convenciones | `ARQ-xx` = requisito de arquitectura con criterio de aceptación. `ADR-xx` = decisión con alternativas descartadas. **FIJADA** = cifra fijada por el titular (00-decisiones, D-03). **PROPUESTA** = cifra o decisión no presente en las fuentes ni fijada todavía. **VERIFICAR** = hecho que debe confirmarse (normalmente con `cargo-deny` o asesoría legal) antes de vender |
| Numeración ARQ | Los IDs no se renumeran porque otras specs los citan. Los huecos están **reservados**: ARQ-09, ARQ-27..ARQ-29, ARQ-34..ARQ-39, ARQ-45..ARQ-49 y ARQ-85..ARQ-89. ARQ-97..ARQ-99, antes reservados, y ARQ-100..ARQ-107 se usan para el idioma (§14, D-08); ARQ-108..ARQ-109 quedan reservados. ARQ-65..ARQ-69, antes reservados, se usan para la Pantalla de arranque (§8.4, D-09). ARQ-16..ARQ-19, antes reservados, se usan para la animación (§3.4). ARQ-110..ARQ-133 son los contratos de los motores de capa 2 (§4.17) |

Gafira es un editor de escritorio local para diseño gráfico, imagen y video. Su principio de producto, «La herramienta no debe ser más difícil que la idea que el usuario quiere comunicar», se traduce en arquitectura así: el núcleo es pequeño, predecible y rápido. Cada capacidad nueva entra por un punto de extensión y no modifica el núcleo.

---

## 1. Principios y atributos de calidad

### 1.1 Atributos de calidad

| Atributo | Significado operativo en Gafira | Requisitos |
|---|---|---|
| Extensibilidad | Los formatos, efectos, herramientas, códecs, segmentadores, proveedores de recursos y backends de render nuevos se añaden registrando una implementación de un trait. No se edita `match` alguno del núcleo. | ARQ-10, ARQ-20..ARQ-31 |
| Adaptabilidad | La misma aplicación se adapta al hardware: GPU o CPU, códecs del sistema o libres de regalías, proxies según la RAM, límites configurables de memoria y VRAM. | ARQ-40..ARQ-47 |
| Rendimiento | Cumple las cifras del PRD y de la spec técnica (tabla en §6.7). Ninguna operación pesada corre en el hilo de UI. La Pantalla de arranque se pinta pronto y no bloquea la carga (D-09). | ARQ-40..ARQ-47, ARQ-65, ARQ-68 |
| Robustez | Pérdida máxima por fallo de 15 s, recuperación del 100 %, guardado atómico, degradación elegante ante fallos de GPU, códec o plugin. | ARQ-50..ARQ-57 |
| Portabilidad | **Solo** Windows 10/11 x86_64 (MSI), Linux x86_64 (`.deb`, `.rpm` y `.tar.gz`) y macOS 12+ aarch64. Sin web/wasm, sin macOS x86_64, sin Windows arm64, sin Linux en otras arquitecturas, sin móvil (D-05). | ARQ-03, ARQ-80, ARQ-84 |
| Vendibilidad | Todas las dependencias y assets distribuidos tienen licencias permisivas. No hay GPL, LGPL, AGPL ni licencias no comerciales. El instalador no incluye binarios externos (FFmpeg incluido). Gafira es software propietario de pago con prueba gratuita y una única licencia (D-01). | ARQ-70..ARQ-79, ARQ-90..ARQ-96 |
| Localización | La interfaz se publica en español e inglés (D-08). El dominio y los motores solo conocen claves de mensaje; los textos, plurales y formatos de número, fecha y unidad los resuelve `gafira-i18n`. El idioma cambia sin reiniciar. | ARQ-97..ARQ-107 |
| Privacidad local | La app trabaja sin conexión. La **única** conexión de red es activar, validar o desactivar la licencia (D-01, D-06). Sin telemetría ni comprobación de versiones. La IA es opcional, local, desactivable y queda fuera del MVP. | ARQ-04, ARQ-91 |

### 1.2 Arquitectura hexagonal

Gafira sigue el patrón de puertos y adaptadores:

- **Dominio**: modelo de documento, comandos, animación, geometría, color y tiempo. Son funciones puras sin I/O.
- **Motores de cómputo**: geometría booleana, maquetación de texto, procesamiento raster, DSP de audio y segmentación. Son puros: reciben bytes o buffers y nunca abren archivos ni hilos del sistema por su cuenta.
- **Aplicación**: sesión, servicios (Facade), historial, cola de exportación, grafo de render y orquestación de autoguardado. Habla con el exterior solo a través de **puertos** (traits) definidos en `gafira-ports`.
- **Adaptadores**: formatos de archivo, códecs y contenedores, backends de render CPU y GPU, UI egui, licencia (`gafira-license`, el único adaptador con red), idioma (`gafira-i18n`: catálogos de mensajes y formatos por idioma) y plataforma (sistema de archivos, bloqueos, audio del sistema, aceleración por hardware, notificaciones, idioma del sistema).
- **Binarios**: componen todo (*composition root*) e inyectan las implementaciones en los registros.

```text
              ┌──────────────────────────── binarios ────────────────────────────┐
              │      gafira (app de escritorio)        gafira-cli (lotes/pruebas)│
              └───────────────┬───────────────────────────────┬──────────────────┘
                              │ composición e inyección        │
 ┌──────────── adaptadores ───┴───────────────────────────────┴──────────────────┐
 │ gafira-ui  gafira-theme  gafira-render-cpu  gafira-render-gpu  gafira-format  │
 │ gafira-io-image  gafira-io-svg  gafira-io-pdf  gafira-mux  gafira-codec-free  │
 │ gafira-codec-os  gafira-assets  gafira-platform  gafira-license  gafira-plugin│
 │ gafira-captions  gafira-io-pptx (Fase 2)  gafira-i18n                         │
 └───────────────────────────────────┬───────────────────────────────────────────┘
                                     │ implementan puertos
 ┌──────────── aplicación ───────────┴───────────────────────────────────────────┐
 │                gafira-engine (Facade, servicios, bus, colas)                  │
 │                gafira-render (grafo, cachés, planificador)   gafira-media     │
 │                gafira-ports (traits de puertos y registros)                   │
 └───────────────────────────────────┬───────────────────────────────────────────┘
 ┌──────────── motores de cómputo ───┴───────────────────────────────────────────┐
 │  gafira-pathops   gafira-text   gafira-raster   gafira-audio   gafira-layout  │
 │  gafira-segment                                                               │
 └───────────────────────────────────┬───────────────────────────────────────────┘
 ┌──────────── dominio ──────────────┴───────────────────────────────────────────┐
 │  gafira-doc   gafira-edit   gafira-anim   gafira-geom   gafira-color          │
 │  gafira-time                                                                  │
 └───────────────────────────────────────────────────────────────────────────────┘
```

### 1.3 Requisitos de principio

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-01 | **Regla de dependencias**: un crate solo depende de crates de su misma capa o de capas inferiores. El dominio no depende de motores, aplicación ni adaptadores. | `cargo xtask check-layers` lee `cargo metadata`, compara con la tabla de capas de §2.1 y falla en CI si hay una arista prohibida. |
| ARQ-02 | El dominio y los motores no hacen I/O: no usan `std::fs`, `std::net`, `std::thread::spawn`, `std::time::SystemTime::now` ni `std::env`. | Un lint de CI (`clippy::disallowed_methods` y `disallowed_types` en `clippy.toml`, aplicado a los crates de las capas 0–2) da 0 avisos. |
| ARQ-03 | Las únicas plataformas objetivo son `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu` y `aarch64-apple-darwin`. | La matriz de CI contiene exactamente esos tres *targets*. `gafira-platform` emite `compile_error!` en cualquier otro. |
| ARQ-04 | Ninguna función de edición, importación o exportación abre conexiones de red. La única conexión de la app es la de licencias (§13). No hay telemetría ni comprobación de versiones. | Una prueba E2E con red bloqueada (sandbox del SO) completa crear → editar → exportar PNG/PDF/MP4 con la licencia en estado `Trial`, `Active` u `OfflineGrace`. `cargo tree -i <cliente HTTP> -p gafira` muestra un único camino: `gafira-license` con la feature `online`. Ningún otro crate del workspace depende de un cliente HTTP, de sockets ni de DNS. |
| ARQ-05 | Las capas superiores ven a las inferiores solo por su API pública. Los detalles internos son `pub(crate)`. | `cargo public-api` se ejecuta en CI para `gafira-doc`, `gafira-ports` y `gafira-engine`, y los cambios de API pública requieren revisión. |

---

## 2. Workspace de crates

### 2.1 Lista de crates

Prefijo `gafira-`. El prefijo evita colisiones en crates.io si en el futuro se publica algún crate de forma aislada, y hace evidente la frontera del producto en `cargo tree`. Edición Rust 2024 y MSRV 1.93 (`rust-toolchain.toml` fija 1.93.1).

| Capa | Crate | Responsabilidad | NO debe | Fase |
|---|---|---|---|---|
| 0 Dominio | `gafira-geom` | Puntos, vectores, `Affine`, `Rect`, `IRect`, rutas Bézier (sobre `kurbo`), unidades (px, mm, cm, in, pt) y conversión por DPI. | Rasterizar, conocer nodos. | MVP |
| 0 Dominio | `gafira-color` | Tipos de color (sRGB, lineal, Display-P3, CMYK nominal, Lab), conversión, degradados, `ColorTransform`, perfiles ICC (sobre `moxcms`), muestras de paleta. | Leer archivos ICC del disco (recibe bytes). | MVP |
| 0 Dominio | `gafira-time` | Tiempo racional en *flicks* (1/705 600 000 s; divide exactamente 24, 25, 30, 48, 50, 60, 120 fps y 44,1/48/96 kHz), `TimeRange`, timecode y frecuencias de fotograma. | Usar relojes del sistema. | MVP |
| 1 Dominio | `gafira-anim` | `Animatable<T>`, keyframes, curvas (Hold, lineal, ease, Bézier cúbica), trait `Interpolate`, muestreo determinista. | Conocer nodos concretos. | MVP |
| 1 Dominio | `gafira-doc` | Modelo de documento unificado (§3): `Project`, `Page`, `Sequence`, `Track`, `Clip`, `Node`, `ElementBase`, estilos, identidad visual, `AssetRef`, `NodeVisitor`, validación de invariantes, esquema serde. Módulo `msg`: tipos neutros de mensaje (`MessageKey`, `Message`, `MsgArg`, trait `ToMessage`) que las capas superiores usan para hablar por claves (§14.2). Módulo `contract`: tipos compartidos por los motores (§4.17, ADR 0005). | Renderizar, decodificar recursos, leer archivos. Conocer idiomas o textos traducidos. | MVP |
| 1 Dominio | `gafira-edit` | Trait `Command`, comandos del núcleo, `History` (Command + Memento), `Selection`, transacciones y fusión de gestos. | Conocer la UI o los formatos. | MVP |
| 2 Motores | `gafira-pathops` | Booleanas de rutas, contorno de trazo, simplificación, *hit testing* geométrico. | Pintar píxeles. | MVP |
| 2 Motores | `gafira-text` | Shaping, bidi, saltos de línea, maquetación de cajas, interletrado, ligaduras, texto en curva y conversión a contornos. Las fuentes llegan como bytes. | Enumerar fuentes del sistema (eso es `gafira-platform`). | MVP |
| 2 Motores | `gafira-layout` | Guías, rejillas de columnas, ajuste magnético (*snap*), alineación y distribución, cálculo de sangrado y zona segura, reflujo al cambiar de tamaño. | Mutar el documento (devuelve resultados que un `Command` aplica). | MVP |
| 2 Motores | `gafira-raster` | Buffers de imagen (u8/f16/f32, premultiplicados), `AlphaMask`, tiles, mipmaps, fórmulas de los modos de mezcla, trait `Effect` y efectos del núcleo (ajustes, desenfoque, nitidez, viñeta, sombras), trait `Transition` y transiciones del núcleo (§4.13), paleta dominante y hash perceptual, scopes (Fase 2). | Decodificar formatos de archivo. | MVP |
| 2 Motores | `gafira-audio` | Grafo de mezcla, trait `AudioProcessor`, ganancia, fundidos, EQ de 3 bandas, panorama, nivelación (EBU R128), remuestreo. Fase 2: puerta de ruido, compresor, limitador, reducción de ruido y *time-stretch* con «mantener tono». | Abrir dispositivos de audio. | MVP |
| 2 Motores | `gafira-segment` | Trait `Segmenter` e implementación clásica sin ML (GrabCut con GMM y *max-flow* sobre superpíxeles SLIC, refinado de bordes con *guided filter*). Feature `onnx` opcional (Fase 3). | Incluir pesos de modelos por defecto. | MVP |
| 3 Aplicación | `gafira-ports` | Traits de puertos y registros: `Importer`, `Exporter`, `RenderBackend`, `Container`, `Codec`, `Decoder`, `Encoder`, `AssetProvider`, `ProjectRepository`, `ContentStore`, `FileSystem`, `Clock`, `Notifier`, `PluginHost`, `LicenseVerifier` (§4.14), `TextAssistant` (§4.15, opcional), los puertos de idioma `Translator`, `LocaleResolver` y `SystemLocaleSource` con los tipos `Locale`, `LanguagePreference` y `FormatLocale` (§4.16) y `Registry<T>`. | Implementar adaptadores. | MVP |
| 3 Aplicación | `gafira-render` | Construcción del grafo de render (Visitor), caché por contenido y por tiles, planificador con prioridades, presupuesto de frame, selección y degradación de backend. | Hablar con wgpu directamente. | MVP |
| 3 Aplicación | `gafira-media` | Pipeline de medios: índice de keyframes, caché de fotogramas, proxies, sincronía A/V, reloj de reproducción y orquestación de exportación de video. | Implementar códecs concretos. | MVP |
| 3 Aplicación | `gafira-engine` | `Session`, servicios Facade (§5.4), bus de eventos, cola de exportación, autoguardado, recuperación, Versiones, límites de memoria, gestión de trabajos y cancelación, y `LicenseService` (consulta `LicenseVerifier`, §13). | Depender de egui. Abrir conexiones de red. | MVP |
| 4 Adaptadores | `gafira-format` | Lectura y escritura de `.gafira` (§7), carpeta `versions/`, medios vinculados, migraciones por paso, guardado atómico. Implementa `ProjectRepository`. | Decodificar imágenes. Guardar rutas absolutas. | MVP |
| 4 Adaptadores | `gafira-assets` | Almacén direccionado por contenido (BLAKE3), índice de biblioteca local, miniaturas, plantillas empaquetadas. Implementa `ContentStore` y `AssetProvider`. | Acceder a la red. | MVP |
| 4 Adaptadores | `gafira-io-image` | `Importer`/`Exporter` raster: PNG, JPEG, WebP, GIF (incluido el animado), TIFF, BMP, QOI. | Contener lógica de documento. | MVP |
| 4 Adaptadores | `gafira-io-svg` | Importación SVG (usvg) y exportación SVG propia (texto como contornos o como `<text>`). | — | MVP |
| 4 Adaptadores | `gafira-io-pdf` | Exportación PDF (krilla), preajustes Digital e Impresión, sangrado y marcas de corte. CMYK en Fase 2. | Importar PDF en el MVP. | MVP |
| 4 Adaptadores | `gafira-io-pptx` | `Exporter` PPTX: una diapositiva por página con una imagen de alta resolución, sin objetos editables (D-07). Escribe el paquete OOXML con `zip` (solo la feature `deflate`) y un escritor XML permisivo (`quick-xml`, MIT; §9.2). | Depender de otras bibliotecas externas. Importar PPTX. | Fase 2 |
| 4 Adaptadores | `gafira-mux` | Contenedores en Rust propio: MP4/ISO-BMFF, WebM/Matroska, Ogg, WAV y MP3 elemental (solo lectura). AVI solo lectura en Fase 2 (D-07). Implementa `Container`. | Codificar o decodificar elementales. Escribir MP3 o AVI. | MVP |
| 4 Adaptadores | `gafira-codec-free` | Códecs libres de regalías: AV1 (codificación y decodificación), Opus (codificación y decodificación), FLAC, Vorbis (decodificación), MP3 (**solo decodificación**, con `nanomp3`, MIT OR Apache-2.0; §9.2), PCM, MJPEG intra para proxies y para AVI (Fase 2), VP9 (Fase 2). | Incluir encoders H.264, HEVC o AAC; codificar MP3; incluir decodificadores con patentes activas. | MVP |
| 4 Adaptadores | `gafira-codec-os` | Códecs del sistema operativo por FFI: Media Foundation (Windows), VideoToolbox/AudioToolbox (macOS), VA-API (Linux, cargado con `libloading` cuando el equipo la tenga): **H.264 y AAC** (D-02). | Incluirse sin la feature `os-codecs`. Contener implementaciones propias de códecs. Exportar HEVC. | MVP |
| 4 Adaptadores | `gafira-render-cpu` | `RenderBackend` CPU de referencia (tiny-skia y compositor f32 propio). Es el oráculo de las pruebas golden. | Usar la GPU. | MVP |
| 4 Adaptadores | `gafira-render-gpu` | `RenderBackend` GPU (wgpu: compositor y efectos en WGSL; rutas vectoriales con vello). | Ser obligatorio: siempre existe la alternativa CPU. | MVP |
| 4 Adaptadores | `gafira-platform` | Rutas de datos, bloqueos de archivo, `rename` atómico por SO, fuentes del sistema, preferencia de reducción de movimiento, **idioma y formato regional del sistema** (`SystemLocaleSource`, §14.3), dispositivo de audio (cpal), notificaciones del SO, información de hardware, manejador de fallos. | Contener lógica de negocio. Decidir el idioma de la interfaz (solo informa de lo que dice el SO; la regla de D-08 está en `gafira-i18n`). | MVP |
| 4 Adaptadores | `gafira-i18n` | Idioma de la interfaz (§14): catálogos de mensajes español e inglés embebidos, resolución del idioma según D-08 (`D08LocaleResolver`), traducción por clave con parámetros y plurales (`CatalogTranslator`), formato de números, fechas, tiempos relativos y unidades por idioma (ICU4X), cadena de respaldo y seudo-localización para pruebas. Implementa `Translator` y `LocaleResolver`. | Depender de egui, `gafira-engine` o `gafira-ui`. Leer catálogos del disco en release. Detectar el idioma del SO (eso es `gafira-platform`). Guardar estado global mutable. | MVP |
| 4 Adaptadores | `gafira-theme` | Carga y validación de tokens de diseño desde datos (`gafira.tokens.json`, cuyo contenido normativo está en 03 §3) y tipografías embebidas (Inter, Manrope). | Depender de egui (expone tipos neutros). Definir valores de tokens en código. | MVP |
| 4 Adaptadores | `gafira-license` | Licencia y activación (§13). Núcleo puro: verificación sin conexión del token Ed25519 con clave pública fija en el ejecutable y máquina de estados. Feature `online`: cliente de activación, validación y desactivación. Feature `credentials`: almacén del secreto del dispositivo en el llavero del SO. Implementa `LicenseVerifier`. | Descargar la clave pública. Enviar datos distintos de los de licencia. Contener código del servidor. | MVP |
| 4 Adaptadores | `gafira-ui` | Interfaz egui/eframe: shell, paneles, lienzo, timeline, herramientas (`Tool`), widgets accesibles y traducción de eventos. Pantalla de arranque con sus recursos incrustados (§8.4, D-09). Todo texto visible sale de `Translator` por clave (ARQ-97). | Mutar el documento sin `Command`, o hacer I/O directo. Contener texto literal visible. | MVP |
| 4 Adaptadores | `gafira-captions` | Lectura y escritura de SRT y WebVTT, estilos de subtítulo. | — | Fase 2 |
| 4 Adaptadores | `gafira-plugin` | `PluginHost` con wasmi (sandbox, combustible, capacidades), manifiesto de extensión. | Dar acceso a red o disco no concedido. | Fase 3 |
| Soporte | `gafira-testkit` | Fixtures, generadores `proptest`, comparación golden, documentos sintéticos (100 capas, 20 páginas, video de 3 pistas), escenas sintéticas con máscara exacta (ARQ-132). Solo `dev-dependency`. | Entrar en binarios de release. | MVP |
| Soporte | `xtask` | Comprobación de capas, empaquetado (MSI; `.deb`, `.rpm` y `.tar.gz` para Linux x86_64; DMG), generación de iconos, auditoría de licencias y generación de `THIRD_PARTY_NOTICES.md`. | Distribuirse. | MVP |
| Binario | `gafira` (`apps/gafira`) | App de escritorio: composición, registros e inyección. | Contener lógica. | MVP |
| Binario | `gafira-cli` (`apps/gafira-cli`) | Exportación por lotes (`gafira-cli export proyecto.gafira --format png --pages 1-5`), validación de proyectos, *benchmarks* de escenario (`gafira-cli bench`), regeneración de golden. Mensajes en inglés por defecto; `--lang` para cambiarlo (ARQ-104). | Depender de `gafira-ui`. | MVP |

### 2.2 Grafo de dependencias (acíclico por capas)

```text
gafira-time ─┐
gafira-geom ─┼─► gafira-anim ─► gafira-doc ─► gafira-edit
gafira-color ┘                     │
                                   ▼
        gafira-pathops  gafira-text  gafira-layout  gafira-raster  gafira-audio  gafira-segment
                 (cada motor puede depender de geom, color y doc; audio, también de time; ADR 0007.
                  Únicas aristas entre motores: text → pathops y segment → raster.
                  Sin layout → text, testkit → raster, doc → time, audio → anim ni raster → anim.
                  Aquí a → b significa que a depende de b. Hoy gafira-doc solo depende de geom y color)
                                   │
                                   ▼
                              gafira-ports  ◄── todos los adaptadores
                                   │
                 gafira-render ◄───┼───► gafira-media
                                   ▼
                              gafira-engine
                                   │
            ┌──────────┬───────────┼──────────┬──────────────┐
         gafira-ui  gafira-format gafira-io-*  gafira-codec-* gafira-render-{cpu,gpu} gafira-license gafira-i18n ...
            │                                                   (adaptadores → ports + dominio + motores;
            ▼                                                    nunca → engine, salvo gafira-ui;
                                                                 gafira-i18n → ports + gafira-doc::msg, sin egui)
          apps/gafira, apps/gafira-cli (dependen de todo lo necesario)
```

Reglas adicionales:

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-06 | Los adaptadores dependen de `gafira-ports` y de las capas 0–2, **nunca** de `gafira-engine`. Única excepción: `gafira-ui`, que consume la Facade del engine. | `check-layers` falla si `gafira-format`, `gafira-io-*`, `gafira-codec-*`, `gafira-render-*`, `gafira-assets`, `gafira-license` o `gafira-i18n` declaran `gafira-engine`. |
| ARQ-07 | `gafira-cli` no depende de `gafira-ui`, `eframe`, `winit` ni `wgpu` en su build mínima. | `cargo tree -p gafira-cli --no-default-features` no contiene esos crates. |
| ARQ-08 | Las features que traen exposición a patentes, librerías nativas o red (`os-codecs`, `onnx`, `heic-os`, `online`, `credentials`) son *opt-in* por crate y se activan solo en los binarios. No existen las features `update-check` ni `encumbered-sw` con código (D-02, D-06). | `cargo tree -e features` del dominio y los motores no contiene esas features. Una búsqueda en los `Cargo.toml` no encuentra `update-check`. |
| ARQ-09 | Reservado. | — |

### 2.3 Cuándo dividir en crates y cuándo no (ADR-01)

**ADR-01: granularidad de crates.**
Se crea un crate nuevo solo si cumple al menos uno de estos criterios:

1. **Frontera de capa**: separa dominio de I/O (por ejemplo, `gafira-doc` frente a `gafira-format`).
2. **Feature opcional o exposición legal**: aísla código con patentes o nativo (`gafira-codec-os`, la feature `onnx` de `gafira-segment`) para poder excluirlo del build.
3. **Compilación incremental**: código grande y estable que cambia poco (códecs, contenedores) y que, separado, evita recompilar el resto.
4. **Reutilización por varios consumidores** con API estrecha (`gafira-text` lo usan render y exportación SVG/PDF).

No se divide cuando solo hay organización interna: se usan módulos.

*Alternativas descartadas*:

- **Monocrate**: compila peor en incremental, no impide dependencias ilegales entre capas y no permite excluir códigos con patentes.
- **Un crate por códec** (~45, como en una referencia): demasiado coste de mantenimiento para el alcance de Gafira. Se agrupan por clase legal (`free` frente a `os`).

---

## 3. Modelo de documento unificado

### 3.1 Estructura

```text
Project
 ├─ meta: ProjectMeta { id, name, created, modified, app_version, color_profile }
 ├─ identities: Vec<VisualIdentity>          (identidad visual: logos, colores, fuentes, estilos, plantillas propias)
 ├─ styles: StyleSheet                        (estilos de texto, de relleno y de efecto con nombre)
 ├─ assets: AssetTable                        (AssetRef → ContentHash + metadatos + AssetStorage::{Embedded | Linked})
 ├─ pages: Vec<Page>                          (lienzos; en video, cada escena es una Page con duración)
 │    └─ Page { id, name, size: PageSize, bleed: Length, margins: Margins, guides: GuideSet, grid: GridSettings, notes: String,
 │              background, duration: Option<Flicks>, locked, hidden, roots: Vec<NodeId>, nodes: NodeArena }
 │         GridSettings { visible: bool, step: Length, subdivisions: u8, style: GridStyle, snap: bool }   (Cuadrícula de 03 §2.6)
         Margins { top, right, bottom, left: Length, linked: bool }
         GuideSet = lista de Guide { axis: GuideAxis, position: Length, locked: bool }; GuideAxis = Vertical | Horizontal
 └─ sequences: Vec<Sequence>                  (timelines)
      └─ Sequence { id, fps: FrameRate, size, sample_rate, tracks: Vec<Track>, markers }
           └─ Track { id, kind: Video|Overlay|Audio|Caption, muted, locked, solo, clips: Vec<Clip> }
                └─ Clip { id, source: ClipSource, place: TimeRange, source_in: Flicks,
                          speed, volume: Animatable<f32>, transitions, effects, base: ElementBase }
                     ClipSource = Scene(PageId) | Media(AssetRef) | Nodes(Vec<NodeId>) | Audio(AssetRef)
```

- **Diseño estático**: un `Project` con `pages` y sin `sequences`.
- **Video básico**: una `Sequence` cuya pista principal coloca `Scene(PageId)` y clips de medios en orden. La acción de «agregar escena» crea una `Page` y un `Clip`.
- **Video avanzado**: varias pistas, keyframes, máscaras y capas de ajuste. Es el mismo modelo con más datos, no otro modelo.

`NodeArena` es un `HashMap<NodeId, Arc<Node>>` con hijos por `NodeId`. Así se comparte estructura entre versiones (§5.1) y se hace *lookup* O(1).

- **Cuadrícula** (`Page::grid`): rejilla de fondo con paso configurable (`step` en su propia unidad, de 1 a 500; `subdivisions` internas, de 1 a 10) y estilo `Lines | Dots` (04 FD-07). Es un dato de la página, se guarda con ella y la usa `gafira-layout::snap` cuando el Imán está activo. Nunca se exporta.
- **Datos de maquetación de la página** (ADR 0008): `bleed` vale para los cuatro lados, 0 mm por defecto. `Margins` es 0 px en los cuatro lados y `linked` es verdadero por defecto. Son tipos de `gafira-doc`; `gafira-layout` opera sobre ellos.
- **Validación al cargar** (`Document::validate`): posiciones de guía y márgenes finitos, márgenes ≥ 0, sangrado de 0 a 10 mm, `step` de 1 a 500 en su unidad y `subdivisions` de 1 a 10. Nada se recorta. Unos márgenes que dejan la página sin área no son un error de carga, porque la página puede cambiar de tamaño; lo informa `gafira-layout` (04 FD-05).
- **Campos nuevos** (D-36): el esquema 1 sigue abierto hasta que la primera build que escriba `.gafira` llegue a un usuario. Desde entonces, un campo nuevo con datos de usuario sube el esquema (ADR 0006).
- **Notas** (`Page::notes`): texto plano por página que se edita en la Barra inferior (03 §2.6). No se renderiza en el lienzo ni en ninguna exportación de imagen o video. Si un exportador ofrece incluirlas (por ejemplo, como anotaciones de PDF), lo declara en su `ExportCaps` y la opción está desactivada por defecto.

### 3.2 `ElementBase` y nodos

```rust
pub struct ElementBase {
    pub id: NodeId,                       // u64 único por proyecto, estable al serializar
    pub name: Option<String>,
    pub transform: Animatable<Transform2D>, // x, y, width, height, rotation_deg, skew, flip
    pub opacity: Animatable<f32>,
    pub blend: BlendMode,
    pub visible: bool,
    pub locked: bool,
    pub timing: Option<TimeRange>,         // aparición dentro de la página o escena
    pub mask: Option<Mask>,                // no destructiva
    pub effects: Vec<EffectInstance>,      // pila no destructiva (Strategy por id)
    pub style: Option<StyleRef>,
    pub anim_presets: Vec<AnimPresetRef>,  // presets de entrada, énfasis y salida
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeKind {
    Group(GroupNode),
    Frame(FrameNode),        // contenedor con recorte: forma + contenido con offset/escala
    Shape(ShapeNode),        // rectángulo, elipse, polígono, estrella, ruta, línea
    Text(TextNode),
    Image(ImageNode),        // AssetRef + recorte + ajustes
    Video(VideoNode),        // AssetRef + source_in/out dentro de una escena
    Adjustment(AdjustmentNode), // capa de ajuste: efectos aplicados a lo que hay debajo
    Sticky(StickyNode),      // nota adhesiva
    Table(TableNode),
    Extension(ExtensionNode),// punto abierto: { type_id, schema_version, payload: serde_json::Value }
}

pub struct Node { pub base: ElementBase, pub kind: NodeKind, pub children: Vec<NodeId> }

pub struct TextNode {
    pub content: RichText,               // párrafos con tramos de estilo (fuente, tamaño, color, efectos)
    pub sizing: TextSizing,              // AutoWidth | AutoHeight | Fixed (caja de tamaño fijo)
    pub columns: TextColumns,            // { count: u8 (1 = sin columnas), gap: Length, balance: bool }
    pub autofit: Autofit,                // Off | FitBox { min_size: f32, max_size: f32 } («Ajustar al cuadro»)
    pub type_lock: TypeLock,             // { font: bool, size: bool, style: bool } («Fijar tipografía»)
    pub align: TextAlign, pub vertical_anchor: VAnchor,
    pub tracking: Animatable<f32>, pub line_height: f32,
    pub path: Option<TextPath>,          // texto en curva («Curvar»)
}
```

- **Tipos de texto** (ADR 0008; llegan a `gafira-doc` con `TextNode`): `RichText` es una lista de párrafos, cada uno con tramos (texto y estilo: fuente, tamaño, color, efectos) y un nivel de lista opcional. `TextSizing` es `AutoWidth | AutoHeight | Fixed`. `TextAlign` es `Left | Center | Right | Justify`. `VAnchor` es `Top | Middle | Bottom`. `TextPath` tiene en el MVP la variante `Arc { bend: f32 }`, con `bend` en −100..100 (04 FD-44); la forma vectorial de Fase 2 añade otra. `StyleRef` referencia por nombre un estilo de `StyleSheet`.
- **Columnas**: `gafira-text` reparte el flujo en `count` columnas de igual ancho dentro de la caja, separadas por `gap`. Con `balance`, las columnas terminan a alturas similares. Solo aplica con `sizing = Fixed` o `AutoHeight` (reglas en 04 FD-45).
- **Ajustar al cuadro**: con `Autofit::FitBox`, `gafira-text` busca por bisección, en `[min_size, max_size]`, un tamaño con el que el texto cabe y con el que el siguiente de la rejilla ya no cabe (regla exacta en 04 FD-46). El tamaño resultante es un valor derivado (no se guarda como dato del usuario) y es determinista para las mismas fuentes.
- **Fijar tipografía**: los comandos que cambian estilos de forma masiva (`SetSwatch` sobre fuentes de la identidad, aplicar una identidad o plantilla, `ResizeDesign` con reflujo) no modifican las propiedades marcadas en `type_lock`. Las ediciones directas del usuario sobre ese texto sí se aplican. `type_lock.size` prevalece sobre `FitBox`: el tamaño no cambia (D-25).

- **Marco con contenido recortado** (`FrameNode`): tiene `clip_shape: ShapeGeometry` y `content: Option<NodeId>` con `content_offset`, `content_scale` y `fit: Fill|Fit|Stretch|None`. Al soltar una imagen sobre un marco se aplica un `Command` que asigna el contenido. Reemplazarla conserva la forma.
- **Grupos en espacio de página** (D-35, ADR 0009): la `Placement` de todo nodo está en espacio de página, también la de los hijos de un grupo. La `Placement` de un grupo no mueve a sus hijos. La caja de un grupo es la unión de las cajas de sus hijos; solo un grupo vacío usa su propia caja. Mover un grupo mueve cada descendiente, y girarlo o redimensionarlo reescribe la `Placement` de cada uno.
- **Fuera de página**: los nodos fuera del área se conservan y solo se recortan al renderizar. La selección informa del exceso con `layout::overflow(bounds, page)`.
- **Máscaras no destructivas**: `Mask { source: MaskSource::{Node(NodeId) | Alpha(AssetRef) | Painted(TileSetRef)}, mode: Alpha|Luma, invert, feather: Animatable<f32> }`. Al aislar un sujeto se genera una `Mask::Alpha` editable con pincel Borrar/Restaurar (tiles con COW). El píxel original nunca se modifica.
- **Capas de ajuste**: `AdjustmentNode` aplica su `effects` al compuesto de los hermanos inferiores, acotado por su máscara.
- **Estilos e identidad visual**: `StyleRef` apunta a un estilo con nombre. `VisualIdentity { id, name, colors: Vec<NamedColor>, fonts: Vec<FontRole>, logos: Vec<AssetRef>, templates: Vec<AssetRef>, graphics, voice_notes }`. Los colores pueden referenciar una muestra (`ColorRef::Swatch(identity, key)`): al cambiar la muestra cambia todo el documento con un solo `Command`.
- **Recursos direccionados por contenido**: `AssetRef(ContentHash)`, donde `ContentHash` es BLAKE3 de 32 bytes. Si dos inserciones del mismo archivo producen el mismo hash, el recurso se guarda una sola vez.
- **Medios vinculados**: cada entrada de `AssetTable` tiene `storage: AssetStorage::Embedded` (bytes dentro del `.gafira`, el valor por defecto) o `AssetStorage::Linked { rel_path, bytes, blake3 }` para videos grandes que se referencian por ruta en lugar de copiarse (§7.1). La identidad del recurso sigue siendo su hash, de modo que relocalizarlo no cambia ningún nodo ni clip.

### 3.3 Enum cerrado frente a trait objects (ADR-02)

**ADR-02: modelo de datos cerrado y extensión abierta por registro.**
`NodeKind` es un **enum cerrado** serializable. La extensibilidad llega por `NodeKind::Extension`, cuyo comportamiento (bounds, render, inspector) aporta un `NodeExtension` registrado en `Registry<dyn NodeExtension>` por `type_id`.

- *A favor*: serialización con serde trivial y estable, `match` exhaustivo que obliga a tratar cada tipo nuevo del núcleo, comparación estructural para pruebas, `Clone` barato y sin `dyn` en el camino crítico.
- *Extensiones desconocidas*: si el registro no tiene el `type_id`, el nodo se conserva intacto al guardar (*round-trip* del `payload`) y se renderiza como un marcador con su caja. Nunca se pierden datos.
- *Descartado*:
  - `Box<dyn Node>` con `typetag`: serialización frágil, registros globales con `inventory` que no funcionan bien en todos los linkers, y peor depuración.
  - Modelo ECS: excesivo para un árbol de documento y complica el orden z y el undo.

### 3.4 Animación: `Animatable<T>`

```rust
pub trait Interpolate: Clone + PartialEq + Send + Sync + 'static {
    fn interpolate(&self, to: &Self, t: f32) -> Self;
    fn is_finite(&self) -> bool; // todos los componentes finitos (ARQ-18, ARQ-19)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)] // al leer: forma de transporte sin validar y validación después (ARQ-18)
pub enum Animatable<T: Interpolate> {
    Static(T),
    Keyed(KeyTrack<T>),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyTrack<T> { keys: Vec<Keyframe<T>> } // privado: ordenadas por tiempo; invariante validada (ARQ-18)

impl<T: Interpolate> KeyTrack<T> {
    pub fn new(keys: Vec<Keyframe<T>>) -> Result<Self, KeyTrackError> { /* valida, no ordena ni repara */ }
    pub fn keys(&self) -> &[Keyframe<T>] { /* solo lectura */ }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Keyframe<T> { pub at: Flicks, pub value: T, pub out: Easing }

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Easing {
    Hold, Linear, EaseIn, EaseOut, EaseInOut,
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
}

impl<T: Interpolate> Animatable<T> {
    pub fn sample(&self, t: Flicks) -> T { /* búsqueda binaria + easing; O(log n) */ }
}
```

Implementan `Interpolate`: `f32`, `Vec2`, `Transform2D` (descompuesto: traslación, escala, rotación por el camino corto), `Color` (en espacio lineal) y `GradientStops`. Los presets de animación (entrada, énfasis, salida, trayectoria) se **compilan** a `KeyTrack` al aplicarse. La capa avanzada de video edita esos mismos keyframes con curvas.

**Muestreo (ARQ-16).** `n` es el número de keys; `keys` está ordenada por `at` (ARQ-18).

| Caso | Resultado de `sample(t)` |
|---|---|
| `Static(v)` | `v`, para cualquier `t`. |
| `t ≤ keys[0].at` | `keys[0].value`. Antes del primer key se mantiene su valor. |
| `t ≥ keys[n−1].at` | `keys[n−1].value`. El `out` del último key no se usa. |
| `t == keys[i].at` | `keys[i].value`, sin interpolar. |
| `keys[i].at < t < keys[i+1].at` | Progreso `u = (t − keys[i].at) / (keys[i+1].at − keys[i].at)`, en `[0, 1]`. Se aplica la curva del key **anterior**: `e = keys[i].out(u)`. Resultado: `keys[i].value.interpolate(&keys[i+1].value, e)`. |
| Lo mismo con `keys[i].out = Hold` | `keys[i].value` para todo `t < keys[i+1].at`. El valor salta en `keys[i+1].at`. |

- `sample` es total: ninguna pista válida y ningún `t` representable en `Flicks` (incluidos los extremos de `i64`) producen pánico ni desbordamiento.
- Dentro de un tramo, `u` no decrece cuando `t` crece.
- Un `KeyTrack` de un solo key devuelve ese valor para cualquier `t`.

**Curvas (ARQ-17).** Todas las curvas cumplen `e(0) = 0` y `e(1) = 1`. Las tres con nombre son las de CSS Easing Functions Level 1.

| `Easing` | Definición de `e(u)` |
|---|---|
| `Hold` | Escalón: `e(u) = 0` para `u < 1` y `e(1) = 1`. En la tabla de muestreo, el valor del key anterior hasta el siguiente key; `sample` no evalúa esta curva. |
| `Linear` | `e(u) = u`. |
| `EaseIn` | `CubicBezier { x1: 0.42, y1: 0.0, x2: 1.0, y2: 1.0 }` (CSS `ease-in`). |
| `EaseOut` | `CubicBezier { x1: 0.0, y1: 0.0, x2: 0.58, y2: 1.0 }` (CSS `ease-out`). |
| `EaseInOut` | `CubicBezier { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 }` (CSS `ease-in-out`). |
| `CubicBezier { x1, y1, x2, y2 }` | Bézier cúbica de puntos `P0 = (0, 0)`, `P1 = (x1, y1)`, `P2 = (x2, y2)`, `P3 = (1, 1)`, como `cubic-bezier()` de CSS: `e(u) = y(s)`, donde `s ∈ [0, 1]` cumple `x(s) = u`. `x1` y `x2` están en `[0, 1]`, de modo que `x(s)` es monótona y `s` es único. `y1` e `y2` admiten cualquier valor finito: la curva puede salirse de `[0, 1]` (rebote). |

- **Precisión**: `|e(u) − y(s*)| ≤ 1e-5 · max(1, |y1|, |y2|)` (PROPUESTA), donde `s*` es la raíz exacta de `x(s) = u`.
- **Acotación**: `e(u)` queda en `[min(0, y1, y2), max(1, y1, y2)]`.
- **Entrada fuera de rango**: con `u < 0` la curva da `e(0) = 0`, con `u > 1` da `e(1) = 1` (incluidos `±∞`), y con `u` NaN da `0`. `sample` solo pide `u ∈ [0, 1]`; esta regla hace la curva total. Se recorta la entrada `u`, no la salida: una curva con rebote sigue dando valores fuera de `[0, 1]` dentro del rango.

**Interpolación (ARQ-19).** Reglas comunes a todo `Interpolate`:

- `interpolate(a, b, 0.0) == a` e `interpolate(a, b, 1.0) == b` (igualdad de `PartialEq`). Los dos extremos se devuelven tal cual, sin pasar por la descomposición ni por la conversión, porque recomponer un valor descompuesto solo se acerca a él (≈ 1e-15 en `Transform2D`).
- Los valores son finitos: un componente NaN o `±∞` en el `value` de un key hace inválida la pista (ARQ-18). `is_finite` da `true` si y solo si todos los componentes del valor son finitos; todo tipo que implementa `Interpolate` lo implementa (`Color`, `Transform2D` y `GradientStops` incluidos), y es la única vía por la que `KeyTrack` comprueba la finitud de un `T` genérico. Un `Static` con un componente no finito queda fuera de contrato: JSON no representa esos números (`serde_json` escribe `null`) y ese archivo no se vuelve a leer.
- Con `t` fuera de `[0, 1]` (curvas con rebote) se extrapola. `Interpolate` no recorta: el consumidor recorta si lo necesita (por ejemplo, la opacidad al renderizar).
- **Rango de contrato de `f32` y `Vec2`** (Q-32): la fórmula es `a + (b − a) · t`. La propiedad «entre `a` y `b`» para `t` en `[0, 1]` se garantiza cuando `b − a` es finito en cada componente, en su tipo (`f32` o `f64`). Con extremos finitos cuya diferencia desborda (por ejemplo `3e38` y `−3e38` en `f32`), el resultado intermedio puede ser `±∞` y queda fuera de contrato. Los valores reales de un documento quedan muy lejos de ese límite. Un resultado no finito de la extrapolación (`t` fuera de `[0, 1]`) también queda fuera de contrato.

| Tipo | Regla |
|---|---|
| `f32` | `a + (b − a) · t`, con los extremos exactos. |
| `Vec2` (`f64` por componente) | La misma regla por componente en `f64`. `t` pasa de `f32` a `f64` sin pérdida. |
| `Color` | Ambos extremos se pasan a luz lineal y se interpolan `r`, `g`, `b` linealmente. El resultado no depende de las primarias lineales elegidas, porque el cambio entre ellas es lineal. Si los dos extremos tienen el mismo espacio, el resultado se expresa en ese espacio; si no, en sRGB lineal. **Alfa premultiplicado** (Q-27, como CSS Color 4): se interpolan `r·a`, `g·a`, `b·a` y `a` linealmente, y el resultado se divide por su `a`. Ese `a` es el alfa del resultado en `f32`, `a_a + (a_b − a_a) · t` con la regla de `f32` (bit a bit, Q-29), y es el mismo divisor que usa la referencia de ARQ-19. Así un fundido hacia transparente no oscurece ni tiñe el color, igual que la composición premultiplicada de `gafira-raster`. **Alfa resultante ≤ 0** (Q-31): no se premultiplica; `r`, `g`, `b` se interpolan en recto (`c_a + (c_b − c_a) · t` en luz lineal) y `a` queda sin recortar. Con `t` en `[0, 1]` y alfas no negativos solo ocurre si los dos alfas son 0; con una curva con rebote, `a` puede salir negativo. El consumidor recorta `a` al componer. |
| `Transform2D` | Se descompone cada extremo en traslación `(tx, ty)`, escala `sx > 0` en x, escala `sy` en y con el signo del determinante, sesgo `k` y rotación `θ`, de modo que la parte lineal es `[sx·cos θ, sx·sin θ, sy·(k·cos θ − sin θ), sy·(k·sin θ + cos θ)]` y el determinante es `sx·sy`. Cada componente se interpola linealmente, y la rotación por el camino corto: `θ` es el ángulo de la primera columna `(a, b)`, `atan2(b, a)` en `[−π, π]`; `Δθ = θ_b − θ_a`, se le resta `2π` si `Δθ > π` y se le suma `2π` si `Δθ < −π`, de modo que `|Δθ| ≤ π`. **Empate** (`|Δθ| = π` exacto): `Δθ` conserva el signo de `θ_b − θ_a`, sin sumar ni restar `2π`. Ocurre con entradas corrientes, por ejemplo de la identidad (`θ = 0`) a `scale_xy(−1, 1)` (`θ = π`, `Δθ = +π`), y con todo volteo que la descomposición lee como media vuelta. Con esta regla, el camino de `b` a `a` es el de `a` a `b` recorrido al revés: `interpolate(b, a, 1 − t)` coincide con `interpolate(a, b, t)`. **Comportamiento conocido**: el signo del cero cuenta; una primera columna `(−1, −0)` tiene `θ = −π` y gira en el sentido contrario a `(−1, 0)`. Un volteo (determinante negativo) es siempre `sy < 0`. Entre extremos con el mismo signo de determinante, todo resultado intermedio conserva ese signo, porque `sx` y `sy` se interpolan entre valores del mismo signo. No se usa la descomposición de «Interpolation of Matrices» 2D de CSS Transforms Module Level 1 tal cual: su matriz de sesgo residual deja que el determinante cruce 0 dentro del camino (0,8 % de los pares aleatorios del mismo signo, spike de `anim-core`). |
| `Transform2D`, extremo degenerado (Q-28) | **Prueba de degeneración** (Q-33): sea la parte lineal `[a, b, c, d]`, con columnas `c1 = (a, b)` y `c2 = (c, d)`, y `det = a·d − b·c`, todo evaluado en `f64`. Un extremo de coeficientes finitos es degenerado si `det` o `|c1| · |c2|` no es finito, o si `|det| ≤ 1e-12 · |c1| · |c2|`. El cociente `|det| / (|c1| · |c2|)` es el seno del ángulo entre las dos columnas: no depende de la escala, de modo que `scale(1e-9)` no es degenerado, y un extremo de rango 1 cuyo determinante solo se aparta de 0 por redondeo sí lo es. Un determinante 0 exacto es siempre degenerado. **Comportamiento conocido** (Q-33): un extremo justo por encima del umbral se descompone con normalidad y su sesgo `k` crece como el inverso de ese cociente, de modo que los resultados intermedios pueden tener coeficientes grandes. Un extremo degenerado (por ejemplo la escala 0 de un preset «aparecer desde 0») no se descompone: toma `θ` y `k` del otro extremo, y solo se interpolan la traslación y la escala por eje. Su escala por eje es la diagonal de su parte lineal expresada en el marco `θ`, `k` prestado (Q-30). Así, de la escala 0 a `B`, la parte lineal es `t ·` la de `B`; y de `scaleX(0)` a la identidad, `t = 0,5` da `scale(0,5, 1)`. **Comportamiento conocido** (Q-30): un extremo de rango 1 que no encaja en ese marco (por ejemplo la parte lineal `[1, 2, 2, 4]`) salta justo después de `t = 0` (o justo antes de `t = 1`), porque la parte fuera de la diagonal se pierde; el extremo exacto se sigue devolviendo en `t = 0` y `t = 1`. Ningún preset produce esos extremos: solo producen escalas 0 por eje. Si los dos extremos son degenerados, se interpolan linealmente los 6 coeficientes. **Coeficiente no finito**: un coeficiente NaN o `±∞` hace inválida la pista (ARQ-18), de modo que `sample` nunca lo pasa a `interpolate`. Llamada directamente, `Transform2D::interpolate` es total y no hace pánico: con `t = 0` o `t = 1` devuelve el extremo (regla común), y con cualquier otro `t` interpola linealmente los 6 coeficientes (`a + (b − a) · t` en `f64`), sea cual sea el otro extremo. El extremo no finito no se descompone, no presta su marco `θ`, `k` ni toma el del otro extremo. Un coeficiente del resultado es finito si los dos coeficientes de los que sale son finitos. |
| `GradientStops` | Llega con el tipo de degradado de `gafira-color`, fuera del bloque `gafira-anim`. Su regla se especifica con ese tipo. |

**Pista y formato (ARQ-18).**

- **Invariante de `KeyTrack`**: al menos un key; `at` estrictamente creciente (no hay dos keys en el mismo instante, porque `sample` en ese instante sería ambiguo, ARQ-12); cada `out` es válido (`CubicBezier` con `x1` y `x2` en `[0, 1]` y los cuatro valores finitos); cada `value` es finito en todos sus componentes (`Interpolate::is_finite`, ARQ-19).
- Construir una pista desde una lista de keys (`KeyTrack::new`) devuelve un error que indica la regla incumplida y el índice del key. La pista no se ordena ni se corrige en silencio. Ninguna operación pública deja una pista inválida: la lista de keys no es un campo público y solo se lee con `keys()`.
- Deserializar una pista inválida falla. Nunca produce un `KeyTrack` inválido ni lo repara (ARQ-13).
- **Lectura de `Animatable<T>`**: el JSON se lee primero en una forma de transporte sin validar (`Static` o `Keyed` con keys sin validar) y la pista se valida después. Así el error de una pista inválida leída como `Animatable<T>` nombra la regla y el índice del key, igual que al leer un `KeyTrack` directamente. Un `#[serde(untagged)]` derivado que valida dentro de cada intento pierde esa información: solo informa de que ninguna variante encaja. Un valor de tipo equivocado (por ejemplo, una cadena en un campo `f32`) sí puede dar ese error genérico, porque no incumple ninguna regla de la pista.
- **Formato serde** (`#[serde(untagged)]`): `Static(v)` se escribe como el valor `v` sin envoltorio. `Keyed` se escribe como `{"keys":[{"at":<ticks>,"value":<T>,"out":<Easing>}, …]}`, con `at` como entero de *flicks*. `Easing` usa la representación por defecto de serde del enum tal como se declara: `"Hold"`, `"Linear"`, `"EaseIn"`, `"EaseOut"`, `"EaseInOut"` y `{"CubicBezier":{"x1":…,"y1":…,"x2":…,"y2":…}}`.
- **Compatibilidad**: un campo que pasa de `T` a `Animatable<T>` lee los archivos ya guardados, cuyo valor desnudo se lee como `Static`.
- **Ambigüedad residual**: un objeto escrito a mano con los campos de `T` y además un campo `keys` (por ejemplo `{"x":1,"y":2,"keys":[]}` para `Vec2`) se lee como `Static` y `keys` se ignora. El serializador nunca escribe esa forma.

**Determinismo entre plataformas (ARQ-12, Q-29).** En la misma plataforma, `sample` es determinista bit a bit para todos los tipos. Entre las 3 plataformas de §10.1:

| Tipo | Garantía | Motivo |
|---|---|---|
| `f32`, `Vec2` y las curvas de `Easing` | Bit a bit. | Solo usan aritmética IEEE (suma, resta, producto, división, raíz). El compilador no fusiona `a·b + c` en FMA si el código no usa `mul_add`: comprobado en el ensamblador de x86_64, también con `+fma` y `target-cpu=native`. En aarch64 no se ha comprobado en ensamblador (VERIFICAR); la prueba golden de ARQ-12 lo cubre en el *runner* `aarch64-apple-darwin`. |
| `Color` | `r`, `g`, `b`: diferencia absoluta ≤ `4 · ε_f32 · max(1, abs(c))` por canal, con `ε_f32 = 2^−23` (PROPUESTA). `a`: bit a bit. | La conversión entre espacios usa `powf` (curva de transferencia de `gafira-color`), que llama a la `libm` de la plataforma, de precisión no especificada. Los canales son `f32`: una tolerancia de 1e-9 relativa estaría por debajo de su épsilon (≈ 1,19e-7). El alfa solo usa aritmética IEEE. |
| `Transform2D` | diferencia absoluta ≤ `1e-9 · max(1, abs(c))` por coeficiente (`f64`, PROPUESTA). | Usa funciones trigonométricas (`sin`, `cos`, `atan2`) de la `libm` de la plataforma. |

- En `t` igual al `at` de un key, fuera del rango de keys y con `Hold`, el resultado es el `value` del key: bit a bit en todas las plataformas para todos los tipos.

Fuera de este bloque: los presets de animación, `GradientStops`, la edición de curvas en la UI y el muestreo de nodos concretos (lo hacen `gafira-doc` y `gafira-render`).

### 3.5 Serialización

- `serde` + `serde_json` para el documento, con campos `#[serde(default)]` para compatibilidad hacia delante y `deny_unknown_fields` **desactivado** en los nodos para tolerar versiones futuras.
- Cada `Page` y cada `Sequence` se serializa en su propio archivo dentro del contenedor (§7.1). Así la carga es perezosa y el guardado incremental.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-10 | El documento se serializa de forma determinista: el mismo documento produce los mismos bytes. | Una prueba de propiedad dice que `serialize(deserialize(serialize(d))) == serialize(d)` para 10 000 documentos generados. Los mapas usan `BTreeMap` u orden estable. |
| ARQ-11 | Los nodos `Extension` desconocidos sobreviven a abrir y guardar. | Prueba golden: un proyecto con extensión ficticia, abierto sin el registro, se guarda con su `payload` intacto byte a byte. |
| ARQ-12 | `Animatable::sample` es O(log n) en el número de keys y determinista: la misma pista y el mismo `t` dan el mismo resultado bit a bit en la misma plataforma, y entre plataformas cumple la tabla de determinismo de §3.4 (Q-29). | Un *benchmark* de 10 000 keyframes da < 1 µs por muestra en el hardware de referencia (FIJADA). Esa medición se hace con los *benchmarks* de ARQ-40 (`bench_anim_sample_10k`, §6.7); mientras tanto, `gafira-anim` prueba la estructura: con un `T` de prueba que cuenta llamadas, cada `sample` llama a `interpolate` como mucho una vez, y ninguna en el `at` de un key, fuera del rango de keys o con `Hold`. Pruebas de propiedad: `sample` en `at` de cada key devuelve su `value`, y dos llamadas con la misma pista y el mismo `t` dan resultados idénticos bit a bit. Prueba golden entre plataformas: un archivo de pistas de los cuatro tipos de §3.4 y valores de `t`, con los resultados esperados guardados, se muestrea en las 3 plataformas de CI (§10.1); `f32`, `Vec2` y las curvas coinciden bit a bit, y `Color` y `Transform2D` quedan dentro de las tolerancias de §3.4 (PROPUESTA). |
| ARQ-13 | Las invariantes del documento (ids únicos, hijos existentes, sin ciclos, keys ordenadas, `TimeRange` válidos) se validan tras cada carga y en `debug_assert` tras cada `Command`. | Un fuzzing de `gafira-format` no produce documentos inválidos aceptados. |
| ARQ-14 | «Fijar tipografía» protege las propiedades marcadas frente a cambios masivos, y «Ajustar al cuadro» es determinista. | Prueba de propiedad: tras `SetSwatch` de fuente, aplicar identidad y `ResizeDesign`, un `TextNode` con `type_lock` conserva fuente, tamaño y estilo bloqueados. Prueba golden: `FitBox` da el mismo tamaño en las 3 plataformas con las fuentes embebidas. |
| ARQ-15 | Las `notes` y la `grid` de una página nunca aparecen en las exportaciones de imagen, video, SVG ni PPTX. | Prueba golden: un documento con notas y cuadrícula visibles exporta píxeles idénticos al mismo documento sin ellas. |
| ARQ-16 | `sample` sigue la tabla de muestreo de §3.4: fuera del rango de keys mantiene el primer o el último valor, en el `at` de un key devuelve su valor, entre dos keys aplica la curva `out` del key anterior y con `Hold` mantiene el valor anterior hasta el siguiente key. Es total. | Pruebas unitarias, una por fila de la tabla, con una pista `f32` de 3 keys (incluido `t` un *flick* antes y después de cada key). Pruebas de propiedad sobre pistas generadas: `sample(t)` con `t ≤ keys[0].at` es `keys[0].value` y con `t ≥ keys[n−1].at` es `keys[n−1].value`; con `out = Linear`, el resultado está entre los dos valores vecinos (con la tolerancia de ARQ-19); con `out = Hold`, el resultado es `keys[i].value` para todo `t < keys[i+1].at`; `sample` no hace pánico con `t` y `at` en `i64::MIN` e `i64::MAX`. |
| ARQ-17 | Las curvas de `Easing` son las de §3.4: `Linear` es la identidad y `EaseIn`, `EaseOut` y `EaseInOut` son las curvas CSS `ease-in`, `ease-out` y `ease-in-out` como `CubicBezier`. `CubicBezier` se resuelve con la precisión de §3.4. `Hold` es el escalón de §3.4, y toda curva recorta su entrada `u` a `[0, 1]` (NaN da 0). | Pruebas unitarias: cada curva con nombre da el mismo `e(u)` que su `CubicBezier` equivalente en 1 001 valores de `u`, bit a bit. Pruebas de propiedad con `x1`, `x2` en `[0, 1]` e `y1`, `y2` en `[−2, 3]`: `e(0) = 0` y `e(1) = 1` exactos; `|e(u) − y(s*)| ≤ 1e-5 · max(1, |y1|, |y2|)` (PROPUESTA) frente a una referencia en `f64` por bisección; `e(u)` queda en `[min(0, y1, y2), max(1, y1, y2)]`. Pruebas unitarias: `Hold` da 0 en `u = 0` y `u = 0,5`, y 1 en `u = 1`; cada curva da 0 con `u = −1`, `−∞` y NaN, y 1 con `u = 2` y `+∞`. |
| ARQ-18 | `KeyTrack` cumple su invariante (al menos un key, `at` estrictamente creciente, `out` válido, `value` finito) por construcción y al deserializar, y `Animatable<T>` tiene el formato serde de §3.4. | Pruebas unitarias: construir con 0 keys, con dos `at` iguales, con `at` decreciente, con `CubicBezier` con `x1` o `x2` fuera de `[0, 1]` o con un valor no finito, y con un `value` con un componente no finito, devuelve un error que nombra la regla y el índice; el mismo JSON (salvo los no finitos, que JSON no representa), leído como `Animatable<T>` (no solo como `KeyTrack`), falla con un error que nombra la misma regla y el mismo índice. Prueba de propiedad: `deserialize(serialize(a)) == a` y conserva la variante (`Static` o `Keyed`) para los `T` de §3.4 con valores finitos. Prueba golden: `Static(1.5_f32)` se escribe `1.5`, y un JSON con el valor desnudo de un campo `T` se lee como `Static`. |
| ARQ-19 | `f32`, `Vec2`, `Color` y `Transform2D` implementan `Interpolate` con las reglas de §3.4. | Pruebas de propiedad: para cada tipo, `t = 0` devuelve `a` y `t = 1` devuelve `b`. `f32` y `Vec2`: para `t ∈ [0, 1]`, cada componente está entre `a` y `b` con una tolerancia de `ε · max(|a|, |b|)`, donde `ε` es el épsilon de máquina de su tipo (`f32` o `f64`), sobre el rango de contrato de §3.4 (Q-32): los generadores producen componentes en `[−MAX/2, MAX/2]` de su tipo, de modo que `b − a` es finito. Pruebas unitarias de `is_finite`, para cada tipo: `false` con un componente NaN, `+∞` o `−∞`, y `true` con todos finitos. `Color`: tras convertir a sRGB lineal, cada canal `r`, `g`, `b` difiere de la interpolación premultiplicada de referencia en `f64` en ≤ 1e-5 (PROPUESTA), donde la referencia premultiplica e interpola `r·a`, `g·a`, `b·a` en `f64` y divide por el alfa del resultado en `f32` (`a_a + (a_b − a_a) · t` de §3.4, pasado a `f64` sin pérdida), no por un alfa recalculado en `f64`; `a` es la interpolación lineal de los dos alfas, y el espacio del resultado sigue la regla de §3.4; prueba unitaria: de rojo opaco a negro transparente (`a = 0`), todo resultado con `a > 0` tiene el tono rojo de partida en luz lineal (`g = b = 0`, `r` del rojo). `Color` con alfa resultante ≤ 0 (Q-31), pruebas unitarias: de rojo transparente a azul transparente (los dos con `a = 0`, mismo espacio), `t = 0,5` da `a = 0` y `r`, `g`, `b` iguales a la interpolación en recto de referencia en `f64` (≤ 1e-5, PROPUESTA), todos finitos (sin NaN); de un color con `a = 0,5` a otro con `a = 0` y `t = 1,5`, el resultado tiene `a = −0,25` (sin recortar) y `r`, `g`, `b` iguales a la extrapolación en recto de referencia en `f64` (≤ 1e-5, PROPUESTA), sin canales invertidos de signo por la división. `Transform2D`, pruebas unitarias: dos traslaciones interpolan solo la traslación; de una rotación de 350° a una de 10°, `t = 0,5` da 0° y no 180°; empate en `|Δθ| = π`, con traslación nula y `|obtenido − esperado| ≤ 1e-9 · max(1, |esperado|)` por coeficiente (PROPUESTA): de la identidad a `scale_xy(−1, 1)`, `t = 0,5` da `[0, 1, 0, 0, 0, 0]` (giro de `+π/2`; el sentido contrario daría `[0, −1, 0, 0, 0, 0]`) y `t = 0,25` da `[√2/2, √2/2, −√2/4, √2/4, 0, 0]`; de `scale_xy(−1, 1)` a la identidad, `t = 0,5` da también `[0, 1, 0, 0, 0, 0]` y `t = 0,75` da `[√2/2, √2/2, −√2/4, √2/4, 0, 0]`; coeficiente no finito: con la parte lineal identidad, `tx` NaN o `+∞` y `ty = 4`, hacia una transformación descomponible con rotación y sesgo (por ejemplo `θ = 2`, `k = 0,8`, `sx = 2`, `sy = 3`, traslación `(1, −2)`) y desde ella, `t = 0,5` no hace pánico y cada coeficiente salvo `tx` es la interpolación lineal de los suyos, con la misma tolerancia; dos escalas interpolan la escala por eje; de la identidad a un sesgo `k` el resultado es continuo: entre muestras separadas por `Δt = 1e-3`, cada coeficiente cambia ≤ `1e-2 · max(1, |k|)` (PROPUESTA); de la escala 0 a `B`, la parte lineal es `t ·` la de `B` y la traslación se interpola; de `scaleX(0)` a la identidad, `t = 0,5` da `scale(0,5, 1)`; comportamiento conocido de rango 1 (Q-30): de la parte lineal `[1, 2, 2, 4]` con traslación nula a la identidad, `t = 0` devuelve el extremo bit a bit, `t = 0,5` da la parte lineal `[1, 0, 0, 2,5]` y `t = 1e-6` da `[1, 0, 0, 4]` con una diferencia ≤ 1e-5 por coeficiente (el salto documentado; tolerancias PROPUESTA); entre dos extremos degenerados, cada uno de los 6 coeficientes es la interpolación lineal de los suyos; prueba de degeneración (Q-33), siempre con traslación nula, en `t = 0,5` y hacia la identidad salvo que se indique otro extremo, con `|obtenido − esperado| ≤ 1e-9 · max(1, |esperado|)` por coeficiente, como la tolerancia de `Transform2D` de la tabla de determinismo (PROPUESTA; un coeficiente esperado 0 admite hasta 1e-9): la parte lineal `[0,1, 0,3, 0,7, 2,1]` (rango 1, `det` ≈ 2,8e-17 por redondeo) es degenerada y da `[0,55, 0, 0, 1,55]`, igual que la de determinante 0 exacto `[1,1, 2,2, 3,3, 6,6]` da `[1,05, 0, 0, 3,8]`; la parte lineal `[1e200, 1e200, 1e200, 2e200]` (coeficientes finitos, `det` no finito en `f64`) es degenerada y da `[5e199, 0, 0, 1e200]`, con todos los coeficientes finitos y el mismo resultado en las 3 plataformas; dos ejemplos distinguen el umbral relativo de uno absoluto, con `rotation(θ)` = `[cos θ, sin θ, −sin θ, cos θ]`: la parte lineal `[1e6, 0, 2e6 · cos 1e-13, 2e6 · sin 1e-13]` (`det` ≈ 0,2, seno entre columnas 1e-13) es degenerada y da `[500000,5, 0, 0, (1 + 2e-7) / 2]` (un umbral absoluto la descompondría, con sesgo `k` ≈ 1e13); la parte lineal `1e-7 · rotation(0,3)` (`det` ≈ 1e-14, seno entre columnas 1) hacia `rotation(2,0)` no es degenerada y da `0,50000005 · rotation(1,15)` (un umbral absoluto la trataría como degenerada y daría ≈ `0,49999999 · rotation(2,0)`). `Transform2D`, prueba de propiedad: entre extremos aleatorios con el mismo signo de determinante (`abs(det) > 1e-3`, coeficientes en `[−3, 3]`), todo resultado con `t` en `0,01..0,99` tiene ese signo; en `t = 0` y `t = 1`, también con extremos degenerados, devuelve el extremo bit a bit. |

---

## 4. Traits y polimorfismo

Convención para todos los registros:

```rust
pub struct Registry<T: ?Sized> { items: Vec<Arc<T>>, by_id: HashMap<&'static str, usize> }
impl<T: ?Sized + Describe> Registry<T> {
    pub fn register(&mut self, item: Arc<T>) -> Result<(), RegistryError>; // id duplicado → error
    pub fn get(&self, id: &str) -> Option<&Arc<T>>;
    pub fn iter(&self) -> impl Iterator<Item = &Arc<T>>;
}
pub trait Describe { fn descriptor(&self) -> &Descriptor; } // id estable, nombre, versión
```

`Descriptor` tiene `id`, `name` (`MessageKey`) y `version`. Las capacidades no van en él: cada trait expone las suyas en forma tipada (`Effect::gpu`, `Segmenter::requirements`, `RenderBackend::caps`, `Exporter::capabilities`). Aceptación: `Descriptor` se construye con esos tres campos, y ningún consumidor lee capacidades de él.

Los registros se construyen en el *composition root* (binarios). Están prohibidos los registros globales mutables (`static mut`, `inventory`) (ver ARQ-31).

### 4.1 `Node` y `NodeVisitor` (Visitor)

```rust
pub trait NodeBehavior {
    fn base(&self) -> &ElementBase;
    fn local_bounds(&self, cx: &MeasureCx, t: Flicks) -> Rect;
    fn hit_test(&self, cx: &MeasureCx, p: Point, t: Flicks) -> bool;
}

pub trait NodeVisitor {
    type Out;
    fn visit_group(&mut self, n: &Node, g: &GroupNode) -> Self::Out;
    fn visit_frame(&mut self, n: &Node, f: &FrameNode) -> Self::Out;
    fn visit_shape(&mut self, n: &Node, s: &ShapeNode) -> Self::Out;
    fn visit_text(&mut self, n: &Node, t: &TextNode) -> Self::Out;
    fn visit_image(&mut self, n: &Node, i: &ImageNode) -> Self::Out;
    fn visit_video(&mut self, n: &Node, v: &VideoNode) -> Self::Out;
    fn visit_adjustment(&mut self, n: &Node, a: &AdjustmentNode) -> Self::Out;
    fn visit_sticky(&mut self, n: &Node, s: &StickyNode) -> Self::Out;
    fn visit_table(&mut self, n: &Node, t: &TableNode) -> Self::Out;
    fn visit_extension(&mut self, n: &Node, e: &ExtensionNode) -> Self::Out;
}
pub fn walk<V: NodeVisitor>(doc: &PageView, v: &mut V, order: WalkOrder) -> Vec<V::Out>;
```

- **Patrón**: Visitor sobre un enum cerrado. Separa las operaciones (render, exportación SVG/PDF, recolección de fuentes, GC de recursos, *hit testing*, accesibilidad del lienzo) de los datos.
- **Implementan**: `RenderGraphBuilder` (`gafira-render`), `SvgWriter` (`gafira-io-svg`), `PdfWriter` (`gafira-io-pdf`), `FontCollector` y `AssetCollector` (`gafira-engine`), `HitTester` (`gafira-edit`).
- **Cómo añadir una operación nueva**:
  1. Crear un struct que implemente `NodeVisitor` en el crate que la necesita.
  2. Invocar `walk`.
  3. No se toca `gafira-doc`.
- **Cómo añadir un tipo de nodo de extensión**:
  1. Implementar `NodeExtension` (bounds, `render_into(&mut RenderGraphBuilder)`, esquema del inspector).
  2. Registrarlo.
  3. Nunca se añade una variante a `NodeKind` desde fuera del núcleo.

### 4.2 `Tool` (patrón State)

```rust
pub trait Tool: Send {
    fn id(&self) -> ToolId;
    fn cursor(&self, cx: &ToolCx) -> CursorIcon;
    fn on_event(&mut self, ev: &CanvasEvent, cx: &mut ToolCx) -> ToolOutcome;
    fn overlay(&self, cx: &ToolCx, out: &mut OverlayList);   // asas, guías, previsualización
    fn cancel(&mut self, cx: &mut ToolCx);                    // Esc: revierte la vista previa
}
pub enum ToolOutcome {
    Idle,
    Preview(Box<dyn Command>),      // se aplica sobre una copia ligera; no entra en historial
    Commit(Box<dyn Command>),       // entra en historial (con merge_key si es gesto continuo)
    SwitchTo(ToolId),
}
```

- **Patrón**: State. Cada herramienta es una máquina de estados interna (por ejemplo, `Select { Idle, Pressing, DraggingMove, DraggingHandle(h), Marquee }`). La `ToolBox` delega en la herramienta activa.
- **Implementan** en `gafira-ui::tools`: seleccionar, mano, zoom, texto, formas, líneas, dibujo (esfero, marcador, resaltador, borrador), nota adhesiva, pincel de máscara Borrar/Restaurar, cuentagotas, recorte, y las herramientas de timeline (cuchilla, deslizar, recortar borde).
- **Por qué**: las interacciones complejas sin `bool` dispersos, `Esc` uniforme y herramientas que se prueban sin UI con `CanvasEvent` sintéticos.
- **Cómo añadir una herramienta**:
  1. Implementar `Tool`, que solo produce `Command`s y nunca muta el documento.
  2. Registrarla en `ToolRegistry` con atajo e icono.
  3. Escribir pruebas con secuencias de eventos.
  4. El núcleo no cambia.

### 4.3 `Command` (patrón Command)

```rust
pub trait Command: std::any::Any + Send + Sync + std::fmt::Debug {
    fn label(&self) -> CommandLabel;                          // = MessageKey (gafira-doc::msg) para «Deshacer {x}» (§14)
    fn apply(&mut self, doc: &mut DocMut<'_>) -> Result<Applied, CommandError>;
    fn revert(&mut self, doc: &mut DocMut<'_>) -> Result<(), CommandError>;
    fn merge_key(&self) -> Option<MergeKey> { None }          // p. ej. (Move, node_ids, gesture_id)
    fn merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>> { Err(next) }
    fn touches(&self) -> DirtySet;                            // nodos/páginas afectados (invalidación)
}
```

- **Capacidad de mutación**: `apply` y `revert` reciben `&mut DocMut<'_>`, la vista mutable de `gafira-doc` que solo se obtiene con el `EditToken`; `History` es quien lo usa. Todo comando, del núcleo o de una extensión, muta el documento por esta vía. `Any` permite que `merge` recupere el tipo concreto de `next`: primero comprueba por referencia que es de su propio tipo y solo entonces convierte la caja; si no lo es, devuelve `Err(next)`.
- **Patrón**: Command, con apply, revert y merge. `Transaction` (Composite) agrupa varios comandos en una sola entrada de historial.
- **Implementan** los comandos del núcleo en la raíz de `gafira-edit`: `SetProperty` (un enum `Property` con los campos de nodo y de forma, los de página y el DPI del documento), `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent`, `InsertPage`, `RemovePage`, `SetFrameContent`, `PaintMask`, `ApplyTemplate`, `ResizeDesign`, `SplitClip`, `TrimClip`, `MoveClip`, `SetKeyframe` y `SetSwatch`. Los que dependen de datos que aún no existen (desde `SetFrameContent` hasta `SetSwatch`) llegan con esos datos. `PaintMask` cambia una `TileSetRef` por otra y `ResizeDesign` recibe el resultado de `gafira-layout::reflow`: un comando de capa 1 no llama a un motor (ADR 0008). Los comandos no comprueban `locked`: lo deciden las herramientas. Las extensiones aportan los suyos.
- **Cómo añadir un comando**:
  1. Struct con los datos mínimos para aplicar y revertir.
  2. `touches` cubre todo cambio: una página cuando cambian sus campos o sus `roots`, un nodo cuando cambia su registro o sus `children`; un cambio del DPI no lista nada y quien dependa del DPI compara `Document::dpi`.
  3. Prueba de propiedad con `gafira-testkit::prop_command` (apply → revert → igualdad con la instantánea).
  4. Exponerlo en el servicio correspondiente.

### 4.4 `Effect` (Strategy + Registry)

```rust
pub trait Effect: Describe + Send + Sync {
    fn params(&self) -> &ParamSchema;                          // tipos, rangos, defaults, animables
    fn roi(&self, cx: &EffectCx, p: &ParamValues, out: IRect) -> IRect;    // región de entrada necesaria; `p` ya resuelto y `cx.scale` finito (ARQ-110)
    fn cost_hint(&self, cx: &EffectCx, p: &ParamValues, px: u64) -> Cost;  // para planificación y presupuesto, sin tiempo (§4.17); `p` ya resuelto y `cx.scale` finito (ARQ-110)
    fn run_cpu(&self, cx: &EffectCx, p: &ParamValues, src: ImageView<'_>, dst: ImageViewMut<'_>)
        -> Result<(), EffectError>;
    fn gpu(&self) -> Option<&dyn GpuEffect> { None }           // variante WGSL opcional
}
pub trait GpuEffect: Send + Sync { fn wgsl(&self) -> &'static str; fn bind(&self, p: &ParamValues) -> UniformBlock; }
```

- **Patrón**: Strategy, porque el efecto es intercambiable por id en `EffectInstance { effect_id, params: AnimatableParams }`, combinado con Registry. Los filtros con nombre (presets) son datos (`ParamValues` guardados), no código.
- **Implementan**: `gafira-raster::effects` (ajustes de iluminación, color, balance de blancos, textura, desenfoque, sombras, contorno, viñeta, LUT 3D en Fase 2, *chroma key* en Fase 2) y plugins (Fase 3).
- **Cómo añadir un efecto**:
  1. Implementar `Effect` (y opcionalmente `GpuEffect`).
  2. Añadir una prueba golden CPU y, si hay GPU, otra que compare GPU con CPU dentro de la tolerancia (§10).
  3. Añadir una prueba de que `params()` pasa `ParamSchema::validate_definition`.
  4. Registrar.
  5. El inspector se genera solo desde `ParamSchema`.

### 4.5 `Importer` y `Exporter` (Strategy + Factory/Registry)

```rust
pub struct FormatDescriptor {
    pub id: &'static str, pub extensions: &'static [&'static str],
    pub mime: &'static [&'static str], pub signatures: &'static [ByteSignature],
    pub kind: FormatKind,          // Raster | Vector | Document | Video | Audio | Captions | Project
    pub legal: LegalClass,         // RoyaltyFree | OsLicensed | Encumbered
}
pub trait Importer: Send + Sync {
    fn descriptor(&self) -> &FormatDescriptor;
    fn sniff(&self, head: &[u8]) -> Confidence;
    fn import(&self, src: &mut dyn ReadSeek, opt: &ImportOptions, cx: &mut ImportCx<'_>)
        -> Result<Imported, ImportError>;      // Imported = Asset | Nodes | Pages
}
pub trait Exporter: Send + Sync {
    fn descriptor(&self) -> &FormatDescriptor;
    fn capabilities(&self) -> ExportCaps;       // transparencia, multipágina, animación, CMYK, sangrado…
    fn options_schema(&self) -> &ParamSchema;   // genera el panel de opciones de exportación
    fn validate(&self, req: &ExportRequest) -> Vec<ExportIssue>;
    fn export(&self, job: &ExportJob<'_>, sink: &mut dyn ExportSink,
              progress: &dyn Progress, cancel: &CancelToken) -> Result<ExportReport, ExportError>;
}
```

- **Descubrimiento**: `FormatRegistry::resolve_import(path, head)` filtra por extensión, ordena por `sniff`, prueba el de mayor confianza y, si falla, el siguiente. Para exportar, la UI lista `exporters().filter(caps ⊇ requeridas por el documento)`.
- **Implementan**: `gafira-io-image`, `gafira-io-svg`, `gafira-io-pdf`, `gafira-format` (el propio `.gafira` también es un `Importer` y `Exporter` para «guardar copia») y los adaptadores de video y audio en `gafira-media` (que combinan `Container` y `Codec`).
- **Cómo añadir un formato**:
  1. Implementar `Importer` o `Exporter` en un crate adaptador.
  2. Añadir el fuzz target del importador (§10).
  3. Añadir golden del exportador y una prueba de que `options_schema()` pasa `ParamSchema::validate_definition`.
  4. Registrar en el binario.
  5. Si `legal != RoyaltyFree`, ponerlo detrás de una feature.

### 4.6 `RenderBackend` (Strategy + Adapter, degradación automática)

```rust
pub trait RenderBackend: Send {
    fn caps(&self) -> BackendCaps;               // max_texture, formatos, f16/f32, efectos GPU
    fn begin_frame(&mut self, target: &TargetDesc) -> Result<FrameCx, RenderError>;
    fn execute(&mut self, cx: &mut FrameCx, plan: &RenderPlan, cache: &mut TileCache,
               deadline: Deadline, cancel: &CancelToken) -> Result<RenderReport, RenderError>;
    fn read_back(&mut self, cx: &FrameCx, region: IRect) -> Result<ImageBuf, RenderError>;
    fn memory_usage(&self) -> MemoryUsage;
    fn trim(&mut self, budget: Bytes);           // libera caché al superar límite
}
```

- **Patrón**: Strategy, porque CPU y GPU son intercambiables, y Adapter, porque cada backend adapta tiny-skia/vello/wgpu al plan común. `BackendSelector` aplica la degradación: GPU → CPU ante `DeviceLost`, `OutOfMemory`, adaptador ausente o lista de bloqueo de drivers. Tras un fallo durante la inicialización de la GPU, el siguiente arranque abre en modo CPU y lo notifica.
- **Implementan**: `gafira-render-cpu` (referencia) y `gafira-render-gpu`.
- **Cómo añadir un backend**:
  1. Implementar el trait.
  2. Pasar la suite golden con la tolerancia definida.
  3. Registrarlo con su prioridad en `BackendSelector`.

### 4.7 `Segmenter` (aislar sujeto)

```rust
pub trait Segmenter: Describe + Send + Sync {
    fn requirements(&self) -> SegReq;            // CPU/GPU, RAM estimada, tamaño máx. de trabajo
    fn segment(&self, img: ImageView<'_>, hints: &SegHints,  // zona y trazos de semilla; la zona «Elemento» es la prioridad central (ARQ-130)
               progress: &dyn Progress, cancel: &CancelToken) -> Result<Segmentation, SegError>;
}
pub struct Segmentation { pub mask: AlphaMask, pub reliable: bool }  // ARQ-130, ARQ-131
```

- **Implementación MVP**: `ClassicSegmenter`, que recibe el pool de `rayon` en su constructor, de modo que el trait no depende de `rayon`. Reduce a un lado largo de 2048 px, aplica SLIC, GMM de primer plano y fondo con solo color (D-45), *max-flow* (GrabCut iterativo) y escala con un *guided filter* sobre la imagen completa. Ese filtro es el refinado de bordes, el *matting* del que hablan estas specs: no hay una pasada aparte (ARQ-130). Paraleliza con rayon.
- **Fase 3**: `OnnxSegmenter` tras la feature `onnx`. Los pesos los instala el usuario como paquete separado, con licencia declarada en su manifiesto y verificada por el propio paquete. Nunca se incluyen en el instalador base.
- **Patrón**: Strategy + Registry. La UI elige la estrategia por `requirements` y la preferencia del usuario.
- **Cómo añadir un segmentador**:
  1. Implementar el trait.
  2. Validar con el conjunto fijo de escenas sintéticas de `gafira-testkit` (IoU ≥ 0,90 frente a máscaras exactas, prueba bloqueante, D-20, ARQ-132). El IoU sobre fotos reales es solo un aviso hasta que exista un conjunto con licencia.
  3. Registrar.

### 4.8 Video: `Container`, `Codec`, `Decoder`, `Encoder`

```rust
pub trait Container: Describe + Send + Sync {
    fn probe(&self, head: &[u8]) -> Confidence;
    fn open(&self, src: Box<dyn ReadSeek + Send>) -> Result<Box<dyn Demuxer>, MediaError>;
    fn create(&self, dst: Box<dyn WriteSeek + Send>, cfg: &MuxConfig) -> Result<Box<dyn Muxer>, MediaError>;
}
pub trait Demuxer: Send {
    fn streams(&self) -> &[StreamInfo];
    fn read_packet(&mut self) -> Result<Option<Packet>, MediaError>;
    fn seek(&mut self, t: Flicks, mode: SeekMode) -> Result<(), MediaError>;  // al keyframe previo
}
pub trait Muxer: Send {
    fn add_stream(&mut self, s: &StreamInfo) -> Result<StreamIdx, MediaError>;
    fn write_packet(&mut self, p: &Packet) -> Result<(), MediaError>;
    fn finish(self: Box<Self>) -> Result<MuxReport, MediaError>;
}
pub trait Codec: Describe + Send + Sync {
    fn codec_id(&self) -> CodecId;                  // Av1, Vp9, H264, Opus, Aac, Flac, Vorbis, Mp3, Pcm, Mjpeg…
                                                    // (Hevc existe solo como identificador para avisar de que no se admite)
    fn caps(&self) -> CodecCaps;                    // decode/encode, hw, perfiles, bit depth, legal: LegalClass
    fn decoder(&self, p: &CodecParams) -> Result<Box<dyn Decoder>, MediaError>;
    fn encoder(&self, c: &EncoderConfig) -> Result<Box<dyn Encoder>, MediaError>;
}
pub trait Decoder: Send {
    fn send(&mut self, p: Option<&Packet>) -> Result<(), MediaError>;   // None = flush
    fn receive(&mut self) -> Result<Option<Frame>, MediaError>;        // Frame = Video(VideoFrame) | Audio(AudioBuf)
}
pub trait Encoder: Send {
    fn send(&mut self, f: Option<&Frame>) -> Result<(), MediaError>;
    fn receive(&mut self) -> Result<Option<Packet>, MediaError>;
}
```

- **Patrón**: Strategy + Abstract Factory (`Codec` crea `Decoder` y `Encoder` compatibles) + Registry con política. `CodecRegistry::best_decoder(id)` prefiere hardware del sistema y luego software libre de regalías. No existe una tercera vía de software restringido (D-02). `LegalClass` filtra según las features de build. Para exportar, el valor por defecto es AV1 + Opus (WebM o MP4); H.264/AAC solo se ofrece si `gafira-codec-os` encuentra la API del sistema.
- **Implementan**: `gafira-mux` (contenedores), `gafira-codec-free` y `gafira-codec-os`.
- **Cómo añadir un códec**:
  1. Implementar `Codec`, `Decoder` y `Encoder` en el crate de su clase legal. Un códec con patentes activas (H.264, HEVC, AAC) nunca se implementa en Gafira: solo se adapta la API del sistema en `gafira-codec-os`.
  2. Añadir una prueba de *round-trip* (PSNR ≥ 40 dB a calidad alta, FIJADA) y conformidad con los vectores de prueba del estándar.
  3. Añadir el fuzz del decodificador.
  4. Registrar con su `LegalClass`.

### 4.9 `AudioProcessor` (cadena de efectos, Pipeline)

```rust
pub trait AudioProcessor: Describe + Send {
    fn params(&self) -> &ParamSchema;                          // sin valor por defecto (§4.17.5)
    fn prepare(&mut self, sample_rate: u32, max_block: usize, channels: u16);
    fn process(&mut self, block: &mut AudioBlock<'_>, params: &ParamSnapshot); // sin asignaciones ni bloqueos
    fn latency(&self) -> u32 { 0 }
    fn tail(&self) -> u32 { 0 }
    fn reset(&mut self);
}
```

- **Patrón**: Pipeline/Chain of Responsibility. Cada pista es una `ProcessorChain` (ganancia → EQ de 3 bandas → puerta de ruido → compresor → limitador → panorama, en el orden de 05 FV-20; en el MVP solo ganancia, EQ y panorama). `LoudnessNormalizer` aplica la ganancia de Nivelar, aparte del Volumen, en clip, pista o bus maestro (D-31). Los parámetros llegan por un *ring buffer* sin bloqueo (*lock-free*) desde la UI.
- **Implementa**: `gafira-audio`: `Gain`, `Fade`, `Eq3Band`, `Pan` y `LoudnessNormalizer` (MVP); `NoiseGate` (puerta de ruido: umbral, ataque, retención, liberación), `Compressor`, `Limiter`, `NoiseReducer` y `TimeStretch` (Fase 2).
- ***Time-stretch***: «mantener tono» al cambiar la velocidad de un clip (05 FV-04) es un `AudioProcessor` con latencia declarada (`latency()`) que cambia la duración sin cambiar el tono. Como su salida no tiene la misma longitud que la entrada, `gafira-media` lo aplica antes de la mezcla, al leer el clip, y no dentro de la cadena de la pista. Algoritmo propio en Rust (por ejemplo, WSOLA o vocoder de fase) o crate permisivo (VERIFICAR).
- **Cómo añadir un procesador**:
  1. Implementar el trait sin asignar en `process`. Se verifica con una guardia de asignación: `assert_no_alloc`, solo como `dev-dependency` (§9.2, D-18), que aborta el proceso si `process` asigna; no hace `panic`. Solo actúa en builds de depuración. El control negativo que demuestra que la guardia detecta una asignación vive en `gafira-testkit` y corre en un subproceso. `unsafe` sigue prohibido en el código propio.
  2. Añadir prueba de respuesta (impulso o barrido) contra la referencia.
  3. Añadir una prueba de que `params()` pasa `ParamSchema::validate_definition`.
  4. Registrar.

### 4.10 `ColorTransform`

```rust
pub trait ColorTransform: Send + Sync {
    fn src(&self) -> ColorSpaceId;
    fn dst(&self) -> ColorSpaceId;
    fn apply_f32(&self, px: &mut [f32], layout: PixelLayout) -> Result<(), PixelBufferError>;
    fn bake_lut3d(&self, size: u32) -> Option<Lut3d> { None }   // para GPU
}
```

- **Errores** (D-11): `apply_f32` devuelve `Err(PixelBufferError::LengthNotMultiple { len, channels })` si `px.len()` no es múltiplo de los canales de `layout`, y deja el búfer sin modificar.

- **Patrón**: Strategy. Implementan: matrices y TRC analíticas (sRGB↔lineal↔P3), `IccTransform` (moxcms; RGB→CMYK para PDF de impresión en Fase 2) y `LutTransform` (LUT `.cube`, Fase 2).
- **Cómo añadir una transformación**:
  1. Implementar el trait.
  2. Probar el error ΔE2000 < 1,0 frente a la referencia en una carta de 1 000 colores (FIJADA).

### 4.11 `AssetProvider` (plantillas y biblioteca local)

```rust
pub trait AssetProvider: Describe + Send + Sync {
    fn kinds(&self) -> &[AssetKind];       // Template, Element, Photo, Video, Audio, Font, Icon, AnimPreset
    fn search(&self, q: &AssetQuery) -> Result<AssetPage, AssetError>;  // texto, etiquetas, color, formato
    fn thumbnail(&self, id: &ProviderAssetId, px: u32) -> Result<ImageBuf, AssetError>;
    fn materialize(&self, id: &ProviderAssetId, store: &dyn ContentStore) -> Result<Materialized, AssetError>;
}
```

- **Patrón**: Strategy + Registry, con una búsqueda federada (Composite) que combina proveedores.
- **Implementan** en `gafira-assets`: `BundledProvider` (plantillas y elementos incluidos, todos con licencia propia o permisiva registrada en `assets/LICENSES.toml`), `UserLibraryProvider` (archivos propios del usuario) y `IdentityProvider` (recursos de las identidades visuales). Los plugins pueden añadir proveedores locales en Fase 3.
- **Cómo añadir un proveedor**:
  1. Implementar el trait.
  2. Declarar las licencias de cada recurso en su índice. Un recurso sin licencia declarada no se lista.
  3. Registrar.

### 4.12 `PluginHost` (Fase 3)

```rust
pub trait PluginHost: Send {
    fn load(&mut self, pkg: &PluginPackage) -> Result<PluginHandle, PluginError>; // valida firma y manifiesto
    fn contributions(&self, h: PluginHandle) -> &Contributions;  // efectos, importadores, proveedores, comandos
    fn call(&mut self, h: PluginHandle, req: PluginRequest, fuel: Fuel) -> Result<PluginReply, PluginError>;
    fn unload(&mut self, h: PluginHandle);
}
```

- **Patrón**: Adapter + Proxy. Cada contribución del plugin se envuelve en un adaptador que implementa el trait nativo (`Effect`, `Importer`, etc.) y delega en el host.
- **Implementación**: wasmi (MIT/Apache-2.0) con límite de combustible y memoria. Las capacidades se declaran en el manifiesto y no hay acceso a red. Al disco solo accede por *handles* que concede el usuario.
- **Cómo añadir un tipo de contribución**:
  1. Definir la ABI en `gafira-plugin::abi` (versión semántica).
  2. Escribir el adaptador hacia el trait nativo. Si la contribución trae un `ParamSchema`, el adaptador lo comprueba con `ParamSchema::validate_definition` una vez, antes de registrarla; si falla, el plugin se desactiva y el documento queda intacto (§7.3).
  3. Añadir pruebas de aislamiento (bucle infinito → `FuelExhausted`; acceso no concedido → error) y una de un `ParamSchema` no válido.

### 4.13 `Transition` (efecto de dos entradas)

```rust
pub trait Transition: Describe + Send + Sync {
    fn params(&self) -> &ParamSchema;                  // dirección, color, suavidad…; la duración es del clip
    fn handles(&self) -> HandleReq;                    // margen de fotogramas que pide a cada clip (Both | From | To | None)
    fn run_cpu(&self, cx: &EffectCx, p: &ParamValues, progress: f32,   // 0,0 = solo «desde»; 1,0 = solo «hacia»
               from: ImageView<'_>, to: ImageView<'_>, dst: ImageViewMut<'_>) -> Result<(), EffectError>;
    fn gpu(&self) -> Option<&dyn GpuTransition> { None }               // variante WGSL opcional
    fn audio(&self) -> AudioCrossfade { AudioCrossfade::EqualPower }   // fundido cruzado del audio asociado
}
pub trait GpuTransition: Send + Sync { fn wgsl(&self) -> &'static str; fn bind(&self, p: &ParamValues, progress: f32) -> UniformBlock; }

pub struct TransitionInstance {                        // dato del documento, en Clip.transitions
    pub transition_id: String, pub params: AnimatableParams,
    pub duration: Flicks, pub align: TransitionAlign,  // Centered | Start | End respecto al corte
    pub easing: Easing,                                // curva de `progress`
}
```

- **Patrón**: Strategy + Registry, igual que `Effect`, pero con dos entradas. `RenderGraphBuilder` inserta un nodo `Transition { from, to }` en el grafo cuando el tiempo `t` cae dentro de la unión de dos clips adyacentes; `progress` se muestrea con `easing`. Si un clip no tiene margen de fotogramas, el builder repite el fotograma del borde y la sesión emite un aviso (05 FV-06).
- **Implementan** en `gafira-raster::transitions` las transiciones del catálogo de 03 §2.6 y 05 FV-06: `Cut` (Corte), `Fade` (Fundido a color), `Dissolve` (Disolvencia), `Slide` (Deslizar, 4 direcciones), `Zoom` y `Wipe` (Borrado, 4 direcciones y circular). Los plugins pueden aportar más en Fase 3 (solo CPU).
- **Fotograma entero**: `Transition` no tiene `roi` y `run_cpu` recibe fotogramas enteros, nunca tiles.
- **Registro**: `TransitionRegistry`, construido en el *composition root*.
- **Parámetros** (PROPUESTA): forman el `ParamSchema` de cada transición, y ninguno es animable. Corte y Disolvencia tienen un esquema vacío. Las etiquetas de las opciones son `transition.direction.<id>`. Negro y blanco son valores de `color`, no otra opción. Fundido convierte `color` una vez por llamada, y conserva su alfa: a bytes sRGB premultiplicados en `Rgba8` y a sRGB lineal premultiplicado en `RgbaF32`.

| Transición | Id | Tipo | Por defecto | Etiqueta |
|---|---|---|---|---|
| Fundido | `color` | `Color` | Negro opaco | `transition.fade.color` |
| Deslizar | `direction` | `Choice`: `left`, `right`, `up`, `down` | `left` | `transition.slide.direction` |
| Zoom | `direction` | `Choice`: `in`, `out` | `in` | `transition.zoom.direction` |
| Borrado | `direction` | `Choice`: `right`, `left`, `down`, `up`, `circular` | `right` | `transition.wipe.direction` |
| Borrado | `softness` | `Float` en `[0, 1]` | 0 | `transition.wipe.softness` |

  Identidad y márgenes (PROPUESTA). `transition_id` se guarda en `TransitionInstance` y es igual a `Descriptor.id`; `version` empieza en 1 y sube cuando cambia la salida. Ningún valor de `handles` depende de `TransitionAlign`, porque `handles` no la recibe.

| Transición | `transition_id` | Clave de nombre | `handles` |
|---|---|---|---|
| Corte | `transition.cut` | `glossary.transition_cut` | `None` |
| Fundido | `transition.fade` | `glossary.transition_fade` | `Both` |
| Disolvencia | `transition.dissolve` | `glossary.transition_dissolve` | `Both` |
| Deslizar | `transition.slide` | `glossary.transition_slide` | `Both` |
| Zoom | `transition.zoom` | `glossary.transition_zoom` | `Both` |
| Borrado | `transition.wipe` | `glossary.transition_wipe` | `Both` |

  Aceptación: por transición, `descriptor().id`, `descriptor().name` y `handles()` son los de la tabla. `params()` pasa `ParamSchema::validate_definition` y da los ids, tipos, valores por defecto y etiquetas de la tabla, con `animatable` falso; Corte y Disolvencia dan un esquema vacío; Fundido a blanco opaco con `p` 0,5 da 255 en todos los canales de `Rgba8` y 1,0 en `RgbaF32`; `direction` `diagonal` da `Err`.
- **Cómo añadir una transición**:
  1. Implementar `Transition` (y opcionalmente `GpuTransition`) en `gafira-raster::transitions` o en un crate adaptador.
  2. Declarar `handles` y los parámetros en `ParamSchema`, con una prueba de que `params()` pasa `ParamSchema::validate_definition`; el Inspector → Clip → Transición se genera solo.
  3. Añadir pruebas golden en `progress` = 0, 0,5 y 1. En 0 la salida es idéntica a «desde» y en 1 idéntica a «hacia» (reglas del núcleo en ARQ-118).
  4. Si tiene variante GPU, compararla con la CPU dentro de la tolerancia de §10.
  5. Registrar en `TransitionRegistry` con su nombre del glosario y su miniatura de vista previa. El núcleo no cambia.

### 4.14 `LicenseVerifier` (puerto de licencia)

```rust
pub trait LicenseVerifier: Send + Sync {
    fn status(&self, now: UnixTime) -> LicenseStatus;                 // sin red: lee y verifica el token guardado
    fn activate(&self, key: &LicenseKey, label: Option<&str>) -> Result<LicenseStatus, LicenseError>; // red
    fn validate(&self, now: UnixTime) -> Result<LicenseStatus, LicenseError>;                      // red
    fn deactivate(&self) -> Result<(), LicenseError>;                                              // red
}

pub enum LicenseStatus {
    Trial { ends_at: UnixTime },                           // prueba gratuita en curso
    Active { expires_at: UnixTime, activation_id: String },// token válido verificado sin conexión
    OfflineGrace { token_expires_at: UnixTime, retry_after: Duration }, // sin conexión; el token sigue vigente
    Expired,                                               // prueba o token vencidos
    Unlicensed,                                            // nunca activada y sin prueba
    Invalid { reason: ReasonCode },                        // rechazada por el servidor o token alterado
}
impl LicenseStatus { pub fn can_edit(&self) -> bool { matches!(self, Self::Trial{..} | Self::Active{..} | Self::OfflineGrace{..}) } }
```

- **Patrón**: Port/Adapter. `gafira-ports` define el trait y `gafira-license` lo implementa (§13). `status` es puro y recibe la hora del puerto `Clock` (ARQ-02). Los métodos de red son bloqueantes y el engine los ejecuta en el hilo de E/S, nunca en el de UI.
- **Consumidor**: `LicenseService` en `gafira-engine` publica `EngineEvent::LicenseChanged(LicenseStatus)` y la UI muestra la pantalla de activación y el estado en Ajustes (03).
- **Dobles de prueba**: `FakeLicenseVerifier` en `gafira-testkit` con cada estado, para que las pruebas E2E no dependan de la red.

### 4.15 `TextAssistant` (opcional, local y desactivable)

```rust
pub trait TextAssistant: Describe + Send + Sync {
    fn requirements(&self) -> AssistReq;                   // RAM, CPU/GPU, tamaño del modelo instalado
    fn rewrite(&self, text: &str, req: &RewriteRequest,    // tono, longitud, idioma
               progress: &dyn Progress, cancel: &CancelToken) -> Result<Vec<Suggestion>, AssistError>;
}
```

- **Alcance**: función «Reescribir» del glosario (03 §2.6). Es **opcional, local y desactivable**, y queda **fuera del MVP** (Fase 3), como pide el PRD.
- **Reglas**: ninguna implementación se registra por defecto ni se incluye en el instalador base. El modelo lo instala el usuario como paquete aparte con licencia declarada en su manifiesto (igual que `OnnxSegmenter`, ARQ-76). La implementación corre en el proceso, sin red. Las sugerencias nunca se aplican solas: el usuario elige una y se aplica con un `Command`. Ajustes → Funciones locales opcionales la mantiene desactivada por defecto.
- **Cómo añadir un asistente**: implementar el trait, declarar la licencia del modelo, pasar la prueba de red bloqueada y registrar en `TextAssistantRegistry`.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-20 | Toda capacidad de las §4.4–§4.13 y §4.15 se añade sin modificar `gafira-doc`, `gafira-edit` ni `gafira-engine`. | Prueba arquitectónica: el crate `gafira-testkit/examples/extension-demo` registra un efecto, un exportador y un proveedor nuevos sin tocar el núcleo, y compila en CI. |
| ARQ-21 | Los importadores no causan `panic` con entradas arbitrarias. | Los objetivos `cargo fuzz` de cada importador corren 10 min por PR y 8 h por semana sin fallos. |
| ARQ-22 | La GPU degrada a CPU sin perder el documento. | Una prueba con inyección de `DeviceLost` en mitad de un render termina el frame en CPU y la sesión sigue editable. |
| ARQ-23 | El hilo de audio no asigna ni bloquea. | En build de depuración, la prueba con la guardia de asignación (§4.9, D-18) pasa con 16 pistas activas y bloques de 512 muestras. Una asignación aborta el proceso, así que el control negativo de `gafira-testkit` corre en un subproceso y comprueba ese fallo. |
| ARQ-24 | Un plugin no puede colgar la app. | Un plugin con bucle infinito se interrumpe en ≤ 100 ms por combustible y la UI sigue respondiendo. |
| ARQ-25 | Una transición nueva se añade sin modificar el núcleo y respeta sus extremos. | `extension-demo` registra una `Transition` nueva y compila sin tocar `gafira-doc`, `gafira-edit` ni `gafira-engine`. Para cada transición registrada, la prueba golden da salida idéntica a «desde» en `progress = 0` y a «hacia» en `progress = 1`. |
| ARQ-26 | `TextAssistant` es opcional, local, desactivado por defecto y no forma parte del MVP. | El build del MVP no registra ninguna implementación (`TextAssistantRegistry` vacío). Con una implementación de prueba registrada, la prueba con red bloqueada pasa y la función está desactivada hasta que el usuario la activa en Ajustes. |
| ARQ-27..ARQ-29 | Reservados. | — |

### 4.16 `Translator`, `LocaleResolver` y `SystemLocaleSource` (puertos de idioma)

```rust
// gafira-ports::i18n
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Locale { Es, En }                                   // idiomas publicados (D-08); ampliable (Q-23)

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum LanguagePreference { #[default] System, Es, En }    // Ajustes → General: Sistema | Español | English

pub struct SystemLocale { pub ui: Option<String>, pub formats: Option<String> } // etiquetas tal como las da el SO
pub trait SystemLocaleSource: Send + Sync { fn system_locale(&self) -> SystemLocale; } // gafira-platform

pub struct ResolvedLocale { pub locale: Locale, pub formats: FormatLocale, pub source: LocaleSource }
pub enum LocaleSource { Manual, System { tag: String }, Fallback }       // Fallback = SO sin dato o no soportado
pub trait LocaleResolver: Send + Sync {                                   // gafira-i18n (D08LocaleResolver)
    fn resolve(&self, pref: LanguagePreference) -> ResolvedLocale;
}

pub trait Translator: Send + Sync {                                       // gafira-i18n (CatalogTranslator)
    fn text(&self, locale: Locale, msg: &Message) -> Cow<'_, str>;        // clave + parámetros + plural
    fn has(&self, locale: Locale, key: MessageKey) -> bool;
    fn number(&self, f: &FormatLocale, v: f64, style: NumberStyle) -> String;   // 1,2 / 1.2; %; cifras tabulares
    fn date(&self, f: &FormatLocale, t: UnixTime, style: DateStyle) -> String;  // «12 mar» / «Mar 12»
    fn relative(&self, f: &FormatLocale, delta: Duration) -> String;            // «hace 2 h» / «2 h ago»
    fn quantity(&self, f: &FormatLocale, v: f64, unit: Unit) -> String;         // MB, px, mm, s, fps, dB, LUFS
}
```

- **Patrón**: Port/Adapter + Strategy. Los tipos viven en `gafira-ports` para que el engine, la UI y la CLI los usen sin depender del adaptador; `gafira-i18n` implementa `Translator` y `LocaleResolver`, y `gafira-platform` implementa `SystemLocaleSource`. El *composition root* los inyecta. El detalle, el ADR y los requisitos están en §14.
- **Sin estado global**: el idioma activo es un `Locale` (`Copy`) que guarda quien pinta (la UI o la CLI) y que se pasa en cada llamada. No hay `set_locale` global (ARQ-31).
- **Dobles de prueba**: `FakeSystemLocaleSource` (etiquetas fijas) en `gafira-testkit`; `PseudoTranslator` (feature `pseudo` de `gafira-i18n`, solo desarrollo y pruebas, §14.6).

### 4.17 Contratos de los motores de capa 2

Esta sección hace ejecutable el primer corte de `gafira-layout`, `gafira-text`, `gafira-raster`, `gafira-audio` y `gafira-segment`. Las fichas de 04 fijan lo que ve el usuario; aquí están las reglas comunes y lo que no tiene ficha en el MVP. Cada crate tiene una tabla con lo que entra en su primer corte y una lista de casos numéricos.

Orden de ejecución sugerido: `gafira-layout`, `gafira-text`, `gafira-raster`, `gafira-audio` y `gafira-segment`. Las únicas aristas entre motores son `text → pathops` y `segment → raster` (§2.2, ADR 0007).

Dónde vive cada tipo compartido (ADR 0005, ADR 0008):

| Tipos | Crate |
|---|---|
| `Effect`, `Transition` | `gafira-raster` |
| `Describe`, `Descriptor`, `ParamSchema`, `ParamValues`, `CancelToken`, `Progress`, `AudioCrossfade` | `gafira_doc::contract` |
| `IRect` | `gafira-geom` |
| `ImageView`, `ImageViewMut`, `EffectCx`, `Cost`, `HandleReq`, `GpuEffect`, `GpuTransition`, `UniformBlock`, `Tile`, `TileSet`, `AlphaMask` | `gafira-raster` |
| `AudioBlock`, `ParamSnapshot` | `gafira-audio` |
| `Segmentation`, `SegHints`, `SegReq`, `SegError` | `gafira-segment` |
| `Registry<T>` | `gafira-ports` |

`Cost` da unidades de trabajo abstractas, calculadas con los píxeles y el tamaño del núcleo, sin tiempo (ARQ-02). Aceptación: con los mismos parámetros y el mismo `cx`, `cost_hint` no da menos trabajo para el doble de píxeles.

Las cifras PROPUESTA de esta sección están confirmadas en bloque por D-34 y pueden cambiar con mediciones.

#### 4.17.1 Reglas comunes

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-110 | **Entrada no finita.** Toda función pública de un motor de capa 2 comprueba sus números antes de calcular. Un id que el `ParamSchema` del motor no conoce no es uno de sus números: se conserva intacto y el motor nunca lo lee. Un NaN, un `±∞` o un resultado intermedio no finito dan un `Err` tipado que nombra el campo. No hay `panic`, salida parcial ni un no finito en la salida. Los constructores y los datos planos no validan, como en `gafira-geom` y `gafira-doc`. Excepciones: `AudioProcessor::process` y `mix` (ARQ-123, ARQ-129), y cuatro funciones sin `Result`: `Effect::roi`, `Effect::cost_hint`, `GpuEffect::bind` y `GpuTransition::bind`. Estas cuatro exigen valores que ya pasaron `ParamSchema::resolve`. `GpuTransition::bind` exige además `progress` finito y ya recortado (ARQ-118). `Effect::roi` y `Effect::cost_hint` exigen además `cx.scale` finito, sin otro rango (§4.17.1, «Rangos»). Lo garantiza quien llama. Con otra entrada no hay `panic` y el resultado no está especificado. | Prueba de tabla por función pública: NaN, `+∞` y `−∞` en cada parámetro numérico dan `Err` con ese campo, y el búfer de destino, si lo hay, queda idéntico byte a byte. Un id desconocido con NaN o `±∞` en `ParamValues` no da `Err`, y la salida es idéntica byte a byte a la que se obtiene sin ese id. Prueba de propiedad: con entrada finita dentro del dominio, toda salida numérica es finita. Por efecto y transición, con valores de `resolve`, cada parámetro en su mínimo y en su máximo: `roi`, `cost_hint` y `bind` no hacen `panic`. `cost_hint` con `px = u64::MAX` y `roi` con `out` en los extremos de `i32` no desbordan, tampoco con `cx.scale = f32::MAX`. Con valores sin `resolve`, NaN incluido, tampoco hacen `panic`, ni `roi` y `cost_hint` con `cx.scale` NaN o `±∞`, y ninguna prueba fija el valor devuelto. |
| ARQ-111 | **Determinismo.** La misma entrada produce los mismos bytes. El resultado no depende del número de hilos ni de ninguna fuente de entropía. Un generador pseudoaleatorio, si existe, usa una semilla fija. | Cada crate ejecuta dos veces cada operación de su primer corte y compara los bytes. Los crates con `rayon` comparan además pools de 1, 2 y 4 hilos. |
| ARQ-112 | **Cancelación y progreso sin reloj.** Una operación es larga si su firma recibe `CancelToken` y `Progress`. En capa 2, la única es `Segmenter::segment` (§4.7). `AudioProcessor`, `Effect`, `Transition`, la sonoridad (ARQ-126), Nivelar (ARQ-127) y el remuestreo (ARQ-128) no reciben token y quedan fuera de esta fila; `gafira-engine` cancelará esos tres últimos entre bloques más adelante (D-43). Una operación larga emite `Progress` con una fracción no decreciente en `[0, 1]`, que solo llega a `1,0` al terminar bien. La fracción sigue un modelo de trabajo, las unidades hechas sobre las previstas, y no la parte del tiempo transcurrido. Comprueba el token al menos una vez por informe. Al ver la cancelación devuelve `Err` de cancelado, no entrega salida y no emite más progreso. La estimación de tiempo no se calcula en capa 2 (ARQ-02): la calcula `gafira-engine` con su `Clock` a partir de la fracción. | Prueba con un doble de `Progress` que cancela en el informe `k` (`k` = 1, el del medio y el penúltimo): la llamada devuelve cancelado y hay como mucho 1 informe después del `k`. Sobre la entrada de referencia, la fracción sube ≤ 0,02 entre dos informes seguidos; ese límite se comprueba sobre las fracciones emitidas, no sobre el tiempo de cada tramo. Con un pool de 1 hilo, dos ejecuciones dan la misma secuencia de fracciones. La entrada de referencia de `segment` es la escena 01 de ARQ-132, de fondo plano y sujeto centrado, a 1024×768 px; ese tamaño solo vale para esta fila. Sin medidas de tiempo en `cargo test`. La cifra de ≤ 100 ms (ARQ-43) se mide en §6.7. |
| ARQ-113 | **Golden de capa 2.** Un criterio de píxel usa primero un valor analítico. Un golden guardado se genera en `x86_64-unknown-linux-gnu`, usa un formato sin pérdida y solo se regenera con una acción explícita: ninguna prueba reescribe su referencia. La tolerancia es la de §10. | Prueba del comparador: una referencia con 1 byte alterado falla en la misma plataforma. Entre plataformas, con 1 000 píxeles, 1 píxel a 2/255 pasa (99,9 %), 2 píxeles a 2/255 fallan, y todos a 1/255 pasan. |
| ARQ-114 | **Presupuestos.** Las cifras de capa 2 viven en §6.7 y las mide un *benchmark* (ARQ-40). No son una puerta de `cargo test`. | Cada cifra de §6.7 citada por una ficha de capa 2 tiene un *benchmark* con el nombre de esa fila. |

**Rangos.** Un motor comprueba solo los rangos que indican sus filas de §4.17 o sus filas `FD-`. Un valor finito fuera de un rango sin fila, como `skew = 90` en `node_bounds`, no se vuelve a comprobar: queda fuera del dominio de ARQ-110 y, si da un resultado finito, ese valor no está especificado a propósito. Un no finito, de entrada o calculado, sigue dando `Err` (ARQ-110). Aceptación: `node_bounds` de una caja de 100 px con `skew = 90` no da `Err` de rango, y ninguna prueba fija el valor devuelto.

#### 4.17.2 `gafira-layout`

| Primer corte (D-14) | Fuera del corte |
|---|---|
| FD-03 (a, c, d), FD-04 (a, d, e, f), FD-05 (b, d, e, f), FD-06 (a a i), FD-07 (a, c, d, e), FD-09 (d), FD-11 (a, b, d, e, f) | Datos y comandos en otros crates: FD-03 (b), FD-04 (b, c), FD-11 (c). Exportación o interfaz: FD-05 (a), FD-07 (b), FD-09 (a, b, c). Con FD-54: FD-05 (c). Change posterior a `gafira-text`: FD-53, FD-54. |

| Caso numérico | Regla | ID |
|---|---|---|
| NaN o `±∞` en cualquier entrada | `Err` de toda la llamada; ningún resultado parcial. Se comprueban todas las entradas, también las que no intervienen en el cálculo. En `node_bounds`, son entrada los campos numéricos del `Placement` del nodo pedido y de todos sus descendientes, grupos con hijos incluidos. La caja de un grupo con hijos se comprueba, pero no se usa para calcular (ADR 0009): si es finita, no cambia el resultado. Aceptación: un grupo con hijos válidos y NaN en la `x` de su propio `Placement` da `Err` con ese campo; con una `x` finita y una caja desfasada da la unión de sus hijos; un NaN en un nodo fuera del subárbol pedido no cambia el resultado. | ARQ-110 |
| Suma que desborda (`x + w` con valores enormes) u otro resultado calculado no finito | `Err` de toda la llamada por resultado no finito, también si es intermedio y el valor devuelto sería finito. Aceptación: `overflow` de `Rect(MAX, 0, MAX, 1)` sobre `Rect(−MAX, 0, 0, 1)` da `Err`, porque `−MAX − MAX` es `−∞`; el rectángulo de márgenes de `Rect(−MAX, 0, MAX, 100)` da `Err`, porque su ancho es `+∞`. | ARQ-110 |
| Caja con `x1 < x0` o `y1 < y0` | `Err`. | FD-05, FD-06, FD-11 |
| Caja de tamaño 0 | Válida: `gafira-doc` admite `w = 0` y `h = 0`. Sus bordes y su centro coinciden. | FD-05, FD-06, FD-11 |
| Rectángulo sin área | Se decide por eje. El rectángulo de márgenes (`margin_rect`) da `Err` si los dos márgenes del eje suman `> 0` y `≥` el tamaño de la caja; con suma 0 conserva la caja en ese eje, también con tamaño 0. Si la suma o el tamaño no son finitos, se aplica «Suma que desborda». `margin_rect` y `safe_rect` dan `Err` si un eje de tamaño `> 0` queda con tamaño ≤ 0 tras redondear. Aceptación: en un ancho de 100, márgenes 50 y 50 dan `Err`, y 49,99 y 50 no; `Rect(0, 0, 0, 100)` con márgenes 0 devuelve la misma caja, y con margen izquierdo 5 da `Err`; en `Rect(1e16, 0, 1e16 + 4, 100)`, un margen izquierdo de 3 y un `safe_rect` del 49 % dan `Err`. | FD-05 |
| Umbral del Imán | Finito y > 0, en px de pantalla; `≤ 0` da `Err` de fuera de rango que nombra el umbral. La distancia es el valor absoluto de `destino − referencia` y se compara en `f64` con `umbral / zoom` (FD-06). La distancia igual a ese cociente engancha; por encima, no. El rango 2–16 lo aplica Ajustes: `snap` no lo comprueba. Un cociente no finito sigue la regla «Suma que desborda». Aceptación: umbral 0 y −1 dan ese `Err`; 0,5 y 20 no lo dan; a zoom 0,9, una distancia igual a `6 / 0,9` calculado en `f64` engancha con umbral 6; con el Imán activo, umbral `1e308` a zoom 0,05 da `Err` de no finito. | FD-06 |
| Zoom | Finito en `[0,05; 32]`; fuera, `Err`. Aceptación en `snap`: 0,05 y 32 no dan `Err`; 0,049 y 33 dan `Err` de fuera de rango con el campo `zoom`, también con el Imán inactivo o sin ningún destino dentro del umbral. | FD-06, FD-07 |
| Empate del Imán | Comparación exacta de `f64`, sin tolerancia. | FD-06 |
| Entradas del Imán | `snap` no lee el documento. Sus entradas son: la caja en movimiento, la caja de la página, el rectángulo de márgenes, que es opcional, y las cajas de los elementos que atraen, como `Rect` en px de página; las Guías y su visibilidad; `GridSettings`; si el Imán está activo; el zoom, el umbral y el DPI. Una página cuyos márgenes dan el `Err` de la fila «Rectángulo sin área» no tiene rectángulo de márgenes: los Márgenes no atraen, `snap` no da `Err` por ello y los demás destinos siguen igual. Con el Imán apagado o `Alt` pulsado, el Imán está inactivo: `snap` hace solo las comprobaciones de entrada y de rango y da desplazamiento (0, 0), sin destino. No calcula nada más: ni centros, ni el cociente del umbral, ni Guías en px, ni distancias, así que ninguno de esos resultados da `Err`, aunque desborde. El paso en pantalla sí se calcula, porque es parte de una comprobación de rango (fila «Destino en la Cuadrícula»). Con la visibilidad de las Guías apagada, las Guías no atraen, pero se comprueban. Los destinos de cada eje son las Guías de ese eje, convertidas a px con el DPI; los dos bordes del rectángulo de márgenes, si lo hay; los dos bordes y el centro de la página y de cada caja de elemento; y las líneas de la Cuadrícula (fila «Destino en la Cuadrícula»). El centro de una caja es `(x0 + x1) / 2`, y lo mismo en `y`. La lista de cajas de elementos puede estar vacía. El resultado de cada eje es `destino − referencia` y el destino elegido, dado por su coordenada y su tipo; dos destinos con la misma coordenada y el mismo tipo son uno. `bench_snap_1000` mide `snap` con 1 000 cajas de elementos. Aceptación: con el Imán inactivo, una Guía a 1 px da (0, 0), y con NaN en esa Guía da `Err`; con el Imán inactivo dan (0, 0), sin `Err`, una caja en movimiento o de elemento de tamaño 0 en `x = 1e308`, cuyo centro desborda, el umbral `1e308` a zoom 0,05 y una caja en movimiento de `x0 = −MAX` a `x1 = 0` con una Guía en `x = MAX`, y con el Imán activo cada una de esas entradas da `Err` de no finito; con la visibilidad apagada, esa Guía no se engancha, y con NaN da `Err`; una caja de elemento con NaN o con `x1 < x0` da `Err` con ese campo; con márgenes 0, un borde a 1 px del borde de la página da el tipo Margen (D-16); sin rectángulo de márgenes, ese mismo borde da el tipo Página, sin `Err`, y un borde a 1 px de una Guía se engancha a ella; una caja en movimiento de `x0 = −MAX` a `x1 = 0` con una Guía en `x = MAX` da `Err` de no finito, porque la distancia desde `x0` es `+∞`. | FD-06 |
| Elementos que atraen | FD-06 (c) se prueba en `gafira-layout`: a partir de la página y de la selección que se mueve, el crate da las cajas de los elementos que atraen, y esas cajas son la entrada de `snap`. Cada caja es la de `node_bounds`. No atraen la selección y sus descendientes, los antecesores de la selección, los nodos ocultos o bloqueados y todo nodo bajo un antecesor oculto o bloqueado. Un nodo dentro de un grupo atrae, y el grupo también, por la unión de sus hijos, salvo que contenga algo de la selección a cualquier profundidad. Sus demás hijos siguen atrayendo. Son entrada los nodos que atraen y todos sus descendientes, ocultos y bloqueados incluidos, como en `node_bounds` (fila «NaN o `±∞` en cualquier entrada»): un NaN en un descendiente de un grupo que atrae da `Err` con ese campo, aunque ese descendiente no atraiga. Queda fuera solo el nodo que no está en el subárbol de ningún nodo que atrae: un NaN en él no cambia el resultado. Un id de la selección que no está en la página da el `Err` de `node_bounds`. Aceptación: los casos de FD-06 (c), cada uno con un nodo distinto y comprobando las cajas devueltas; con la selección vacía atraen todos los nodos visibles y libres; con un grupo A que contiene un grupo B y un nodo C, y B contiene los nodos D y E, si se mueve D, A y B no están entre las cajas devueltas, y C y E sí; con NaN en D o en el `Placement` de A, el resultado no cambia. En ese mismo árbol, con E oculto o bloqueado y NaN en su `x`: si se mueve C, B atrae y el resultado es `Err` con ese campo, aunque E no atraiga; si se mueve D, ni A ni B atraen, E queda fuera y el resultado no cambia. | FD-06 |
| Destino en la Cuadrícula | La Cuadrícula atrae solo si `visible` y `snap` son verdaderos. Por cada referencia de la caja en movimiento, su destino es la línea mostrada más cercana a esa referencia. Las líneas son las de FD-07 con ese zoom: el mismo nivel que elige `grid_lines` (mayor y menor, solo mayor o `paso × 2^k`) y las mismas posiciones `f64`. Una línea mayor es exactamente `k × paso`, nunca `k × paso / subdivisiones`; una menor, `k × paso / subdivisiones`. Rige la fila «Líneas de `grid_lines`»: `k` es entero, también negativo, y las líneas continúan fuera de la página. Este cálculo no tiene entrada de vista ni la limita a 3840×2160 en pantalla, y por eso `snap` no da el `Err` con el campo `view`. Si dos líneas equidistan de la referencia, compiten las dos y decide la regla de empate de FD-06. Un paso o unas subdivisiones fuera de rango, o un paso que en pantalla vale 0, dan el mismo `Err` que en `grid_lines`, también si la Cuadrícula no atrae. El valor del paso en pantalla es parte de esa comprobación de rango: se calcula también con el Imán inactivo, y si desborda da `Err` de no finito. Aceptación, con caja de tamaño 0, página de 1000×1000 y umbral 6: paso 10 px, zoom 1 y referencia 14 dan destino 10; referencia 15 da 10; referencia −14 da −10. Paso 10 px y 5 subdivisiones: a zoom 1, referencia 13 da 10 y no 12; a zoom 5, referencia 3,2 da 4; a zoom 0,249, referencia 25 da 40. Paso 1 px, zoom 32, página de 16 384×16 384 y referencia 16 000,01 dan 16 000, sin `Err`. Paso 4,8 px, 3 subdivisiones, zoom 4 y referencia 52,81 dan un destino con los mismos bits que `11 × 4,8`. Con `visible` o `snap` falsos, referencia 14 no se engancha a la Cuadrícula, y paso 500,01, subdivisiones 0 o un paso NaN dan `Err`. Con el Imán inactivo, paso 500 in, DPI `3e305` y zoom 32 dan `Err` de no finito, porque el paso en px es finito y en pantalla es `+∞`. Prueba de propiedad: si una vista válida de `grid_lines` alrededor de la referencia contiene una línea a cada lado, el destino tiene los mismos bits que la línea más cercana de esa salida; en un empate, la menor. | FD-06, FD-07 |
| Giro del Imán | Función aparte de `snap`, con una sola entrada: el ángulo en grados, finito y sin rango. Da el múltiplo de 15 más cercano, sin normalizar a una vuelta, comparando las dos distancias exactas en `f64`. A medio camino da el mayor en valor numérico, también con ángulos negativos. Aceptación: 7,4 da 0; 7,5 da 15; 7,499999999999999 da 0; −7,5 da 0; −22,5 da −15; 367 da 360; NaN y `±∞` dan `Err` con ese campo. | FD-06 |
| Ancho de columna | `c ≥ 1 px`; `c = 1` vale. | FD-04 |
| Lado de página de `generate` | El ancho y el alto de `page` son `≤ PageSize::MAX_PX` (16 384 px); el máximo vale. Por encima, `Err` de fuera de rango con el campo `page` y ninguna guía, también si el eje tiene `n = 0`. Esta comprobación va después de las de no finito (ARQ-110) y de la regla «DPI», y antes de la regla «Ancho de columna»: en un lado válido, un `n` enorme da el `Err` de `c ≥ 1 px`, con el campo `columns` o `rows`; en un lado fuera de rango, da el `Err` de `page`. Con `c ≥ 1 px`, `n ≤ 16 384`, así que cada eje da como mucho 32 768 guías. `generate` da las posiciones en px, sea cual sea la unidad de `gap` y `margin`. Aceptación: con espacio y margen 0, 16 384 columnas en un ancho de 16 384 px dan `c = 1` y 16 385 guías verticales, porque las posiciones iguales se emiten una vez; un ancho de 16 384,01 px con 1 columna da `Err` con `page`; un alto de 16 385 px con 1 columna y 0 filas da `Err` con `page`; `columns = u32::MAX` da `Err` con `columns` en un ancho de 16 384 px y con `page` en uno de 16 385 px; a 96 DPI, 2 columnas y 0 filas con `gap` y `margin` de 1 in en un ancho de 960 px dan guías verticales en 96, 432, 528 y 864 px. Prueba de propiedad: en todo `Ok`, cada eje tiene como mucho 32 768 guías. | FD-04 |
| Vista de `grid_lines` | `view` es una región en coordenadas de página, en px, y puede salir de la página (fila «Líneas de `grid_lines`»); su tamaño en pantalla es `ancho × zoom` por `alto × zoom`. Cada eje se compara por separado, en `f64`, con una tolerancia `ε = 1e-6` px de pantalla que absorbe el redondeo de una vista calculada como `pantalla / zoom`: ancho `≤ 3840 + ε` y alto `≤ 2160 + ε`, el dominio de FD-07 (c); exactamente 3840×2160 vale. Por encima, `Err` de fuera de rango con el campo `view` y ninguna línea. Esta comprobación va después de las de no finito (ARQ-110), de la regla «DPI» y de la regla «Zoom». Un producto no finito sigue la regla «Suma que desborda». El límite de 2 000 líneas de FD-07 (c) se mantiene en este dominio y `ε` no añade ninguna: con la separación mínima de 6 px salen 641 líneas verticales y 361 horizontales, 1 002 en total. Aceptación: a zoom 2, una vista de 1920×1080 no da ese `Err`, y una de 1920,01×1080 o de 1920×1080,01 lo da; a zoom 0,5, una de 7680×4320 no lo da; a zoom 1, una de 2160×3840 lo da; a zoom 1, una de 3840,0000009×2160 o de 3840×2160,0000009 no lo da, y una de 3840,0000011×2160 o de 3840×2160,0000011 lo da; a zoom 0,9, una vista de `3840 / 0,9` por `2160 / 0,9`, calculada en `f64`, no lo da, ni con esquina en (0, 0) ni en (1000,5, 1000,5); con zoom 33 y una vista de 1000×1000, el `Err` nombra `zoom`; paso 6 px, 1 subdivisión y zoom 1 en una vista de (0, 0) a (3840, 2160) dan 641 verticales y 361 horizontales, y en una de (0, 0) a (3840,0000009, 2160,0000009), las mismas. Prueba de propiedad: con cualquier zoom válido y `x0`, `y0` en `[−16 384, 16 384]`, la vista de `(x0, y0)` a `(x0 + 3840 / zoom, y0 + 2160 / zoom)` no da ese `Err`. | FD-07 |
| Líneas de `grid_lines` | `view` puede salir de la página por cualquier lado. Las líneas continúan fuera de ella en `k × paso` y, cuando se muestra la cuadrícula menor, en `k × paso / subdivisiones`, con `k` entero, negativos incluidos, y el origen en la esquina superior izquierda de la página. Si solo cabe `paso × 2^k` (FD-07), rige lo mismo con ese múltiplo como paso. Una línea en un múltiplo entero del paso usado es mayor, también con `k` negativo. Los dos bordes de `view` están incluidos. Todas las posiciones de la salida están en px de página, sea cual sea la unidad del paso. Aceptación: paso 10 px, 5 subdivisiones y zoom 5 en una vista de (−10, 0) a (10, 10) dan verticales de −10 a 10 cada 2 px, y solo −10, 0 y 10 son mayores; paso 1 in a 96 DPI y zoom 1 en una vista de (−96, 0) a (96, 10) dan verticales en −96, 0 y 96. | FD-07 |
| Aviso de zona de seguridad | Exceso `> 0`, sin tolerancia; el borde sobre el límite no avisa. | FD-05 |
| Porcentaje de zona de seguridad | Entrada; por defecto 5 % (D-38). En `[0, 50)`; 50 deja un rectángulo sin área y da `Err`. | FD-05 |
| Sangrado | En `[0, 10]` mm, extremos incluidos. | FD-05 |
| Márgenes | Cada lado `≥ 0`, la misma regla que al cargar (§3). Un lado por debajo de 0 da `Err` de fuera de rango que nombra ese lado. Aceptación: `margin_rect` con margen superior −1 da `Err` de fuera de rango con el campo `margins.top`; con margen superior 0 no da ese `Err`. | FD-05 |
| DPI de `bleed_rect`, `margin_rect`, `place_guide`, `generate`, `grid_lines` y `snap` | `dpi > 0`. `dpi ≤ 0` da `Err` de fuera de rango con el campo `dpi`, también si las longitudes ya están en px o si ningún eje usa el DPI. El orden es el mismo en las seis funciones: primero las comprobaciones de no finito de todas las entradas y de caja con `x1 < x0` o `y1 < y0`; después esta regla; después el resto de reglas de rango y los resultados calculados. Aceptación: `dpi = 0` y `dpi = −96` dan ese `Err` en las seis funciones, con longitudes en px; `generate` con 0 columnas y 0 filas lo da; `snap` sin Guías y con la Cuadrícula apagada lo da; con `dpi = 0`, un NaN en `gap` o `margin` de `generate`, en la posición de `place_guide`, en el paso de `grid_lines`, en `bleed` de `bleed_rect` o en un margen de `margin_rect` o, en `snap`, en una Guía, en el paso o en una caja da `Err` de no finito con ese campo; con `dpi = 0`, una caja de `snap` con `x1 < x0` da el `Err` de caja; con `dpi = 0`, `generate` con espacio −1 o con un ancho de 16 385 px, `grid_lines` con zoom 33 y `snap` con zoom 33 o umbral 0 dan el `Err` de `dpi`. | FD-03, FD-04, FD-05, FD-06, FD-07 |
| Entradas de alinear y distribuir | Ninguna de las dos lee el documento. `align` recibe la acción, la caja de la página y las cajas de la selección en orden, como `Rect` en px de página, cada una con si está bloqueada. `distribute` recibe el eje, el modo y esas mismas cajas; no usa la página. Ambas dan un desplazamiento por caja, en el orden de entrada, y es 0 en cada caja que no se mueve. Alinear usa la página solo si la selección tiene exactamente un elemento y está libre. Con dos o más, la referencia es la unión de todas las cajas, bloqueadas incluidas (D-29), aunque solo haya uno libre; si todas están bloqueadas, nada cambia. El centro es el de «Entradas del Imán». El orden de distribuir compara `f64` de forma exacta, así que `−0,0` y `0,0` empatan. Las reglas de no finito y de caja se aplican a todas las cajas y a la página, también con la selección vacía o si la caja no interviene. Aceptación: dos cajas bloqueadas dan 0 en las dos; una libre y una bloqueada se alinean a su unión y no a la página; con la selección vacía, NaN en la página da `Err`; un NaN en una caja bloqueada da `Err` en `align` y en `distribute`; el resultado de una entrada de 4 cajas tiene 4 desplazamientos en ese orden, también cuando distribuir no hace nada. | FD-11 |
| Tolerancia de las pruebas | Posiciones generadas y resultados de alinear y distribuir: ±0,01 unidades (FD-11) y 1e-9 cuando la ficha lo indica. | FD-04, FD-06, FD-11 |

#### 4.17.3 `gafira-text`

| Primer corte | Fuera del corte |
|---|---|
| FD-41 (a, c a m), FD-43 (a), FD-44 (a, c, d, e), FD-45 (a, c, d, e), FD-46 (a, b, c, e, f), ARQ-14 (`FitBox`), ARQ-115, ARQ-116 | Comandos y estilos: FD-41 (b), FD-47, FD-49. Interfaz: FD-42, FD-46 (d). Exportación: FD-43 (b, c), FD-44 (b). `gafira-engine` y `gafira-platform`: FD-48. |

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-115 | **Fuentes de prueba** (D-19): las pruebas usan subconjuntos de Inter y Manrope, una fuente OFL con árabe o hebreo y un subconjunto OFL monocromo de emoji (D-30). La prueba de ligadura opcional de FD-41 (e) usa Manrope, porque Inter no tiene `liga`. Toda fuente de prueba tiene entrada en `assets/LICENSES.toml` (ARQ-75), esté donde esté. | Una prueba o comprobación de CI recorre las fuentes de prueba y falla si alguna no tiene entrada de licencia. |
| ARQ-116 | **Salida de la maquetación.** La maquetación da, sin píxeles: las líneas con su base y su avance, y cada clúster de grafemas con su rango en bytes en el texto de entrada, su posición y su avance. Con mayúsculas, el rango sigue siendo el de la entrada: los clústeres que salen de un mismo carácter comparten su rango. Indica además el desborde, que sale de comparar el avance con el ancho de la caja, sin contar el espacio en blanco que cuelga al final de una línea, y los glifos sin cobertura, es decir, los `.notdef` de 04 FD-41. Así FV-07 anima letra a letra y FV-08 cuenta líneas y caracteres por línea. Un salto de línea final abre una línea más, vacía, y un texto vacío es una línea vacía, como en un editor. | Con Inter: `""` da 1 línea vacía, sin desborde; `"abc\n"` y `"abc\r\n"` dan 2 líneas, la segunda vacía; `"\n"` da 2 líneas vacías; `"a\n\nb"` da 3 líneas. Un texto de 3 líneas forzadas da 3 líneas; sin mayúsculas, concatenar los clústeres en orden lógico da el texto de entrada; `straße` en mayúsculas da 7 clústeres, los dos `S` con el rango 4..6 y `E` con 6..7; `é` compuesta (U+0065 U+0301) es 1 clúster; un texto que excede una caja `Fixed` indica desborde y conserva todo el texto. |

| Caso numérico | Regla | ID |
|---|---|---|
| NaN o `±∞` en tamaños, medidas o caja | `Err`. | ARQ-110 |
| Tamaño | `[1, 1000]` pt, extremos incluidos; fuera, `Err`. | FD-41 |
| Interletrado | `[−100, 500]` milésimas de em, extremos incluidos. | FD-41 |
| Interlineado | `[0,5; 4,0]`, extremos incluidos. | FD-41 |
| Fuente sin métricas | Altura natural ≤ 0 en una fuente que da forma a algún clúster, o en la primera de la lista con texto vacío: `Err` de fuente inválida. Una fuente que nada usa no se comprueba. Aceptación: con una fuente de métricas 0, sola, `"abc"` y `""` dan ese `Err`; detrás de Inter, `"abc"` da `Ok`. | FD-41 |
| Lista de fuentes vacía | `Err` sin fuente, también con texto vacío, tras las comprobaciones de ARQ-110. Aceptación: `"abc"` y `""` con la lista vacía dan ese `Err`, distinto del de fuente inválida; con NaN en el DPI, el `Err` es el de no finito. | FD-41 |
| Caja de tamaño 0 | Sin error; hay desborde si el texto tiene algún clúster que no sea espacio en blanco (`White_Space` de Unicode). Aceptación: `" "`, `"   "`, `"\t"`, `"\n"` y `"\r\n"` no dan desborde; `"a"` y `" a"` lo dan. | FD-41, FD-45 |
| Columnas | `count` en 1–8; `gap` en `[0, 200]`; ancho de columna ≥ 1 px. | FD-45 |
| `FitBox` | `min_size ≤ max_size`, los dos en `[1, 1000]`; el límite «cabe» incluye la igualdad; rejilla de 0,25 pt. | FD-46 |
| Curvar | `[−100, 100]`, extremos incluidos; 0 no calcula radio; avance 0 no hace nada. | FD-44 |
| Tolerancia de las pruebas | Identidades, bit a bit; medidas en px, ±0,01. | FD-41, FD-44 |

#### 4.17.4 `gafira-raster`

| Primer corte | Fuera del corte |
|---|---|
| ARQ-25 (extremos), ARQ-117 a ARQ-122, FD-12 (c, mitad CPU, que es ARQ-119), FD-26 (paleta), FD-28 (b, hash), FD-33 (a, d a h), FD-34 (a, b), FD-35 (a, d) | GPU y escenario de `extension-demo`: FD-33 (b), FD-12 (c, mitad GPU), FV-06 (b). Interfaz: FD-32, FD-33 (c), FD-37. Exportación y rendimiento de lienzo: FD-35 (b, c). Sin fórmula aprobada: Brillo y Contorno de FD-35 (D-42). Scopes, *chroma key*, LUT y seguimiento: Fase 2. |

Requisitos previos fuera del crate: Lab y ΔE2000 en `gafira-color` (ARQ-122), que hoy no existen.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-117 | **Búferes.** Los píxeles son de 8 bits o `f32`, premultiplicados. Premultiplicar en 8 bits es `(c × a + 127) / 255` con división entera. Despremultiplicar da `(0, 0, 0, 0)` con `a = 0`, y en otro caso `min(255, redondeo(c × 255 / a))`. Las operaciones de color trabajan sin premultiplicar y conservan el alfa. La variante `f16` no entra en el primer corte: solo la usa la GPU (§6.2). | Prueba exhaustiva de los 65 536 pares `(c, a)` en 8 bits: el resultado premultiplicado es ≤ `a` y coincide con el redondeo exacto de `c × a / 255`. Con `a = 255`, premultiplicar y despremultiplicar es la identidad. Con `a = 0`, la salida es 0. Un búfer con longitud distinta de `ancho × alto × canales` da `Err`. |
| ARQ-118 | **Transiciones del núcleo.** `progress` finito fuera de `[0, 1]` se recorta a ese rango; NaN y `±∞` dan `Err`. Con `progress` 0, la salida es la copia exacta de «desde»; con 1, de «hacia». «Desde», «hacia» y destino tienen las mismas dimensiones y el mismo formato, `Rgba8` o `RgbaF32`; si no, `Err`. Ninguna transición convierte formatos, y `Alpha8` no se admite. **Corte**: «desde» si `p < 0,5` y «hacia» si `p ≥ 0,5`. **Disolvencia**: `desde × (1 − p) + hacia × p`. **Fundido**: de «desde» al color con `2p` si `p < 0,5` y del color a «hacia» con `2p − 1` si no. **Deslizar**: «hacia» entra sobre «desde», que no se mueve, con desplazamiento `d = redondeo((1 − p) × lado)`, en `f64` y con medio alejándose de 0 (PROPUESTA). `direction` nombra hacia dónde viaja el borde, con `y` hacia abajo, y `lado` es el ancho en `left` y `right` y el alto en `up` y `down`. Con `left`, «hacia» entra por la derecha: cada píxel `x ≥ d` es `hacia(x − d)`; con `right`, por la izquierda; con `up`, por abajo; con `down`, por arriba. Reemplaza píxeles, no compone: un «hacia» translúcido no salta en `p = 1`. **Zoom**: mezcla como Disolvencia, con la escala lineal en `p` (PROPUESTA). `in` (acercar): «desde» a `1 + p` y «hacia» a `(1 + p) / 2`. `out` (alejar): «desde» a `1 − p / 2` y «hacia» a `2 − p`. El pivote es el centro `(c_x, c_y) = (W / 2, H / 2)`. Un píxel de pantalla muestrea `s_x = c_x + (x + 0,5 − c_x) / escala − 0,5`, y lo mismo en `y`, con filtro bilineal sobre canales premultiplicados. Fuera de la imagen, el muestreo toma el píxel del borde: la regla de negro transparente de ARQ-120 no aplica aquí, para que dos clips opacos sigan opacos. **Borrado**: `u` es la posición del centro del píxel en `[0, 1]` en la dirección elegida (`right`: `(x + 0,5) / W`; `left`: `1 − (x + 0,5) / W`; `down`: `(y + 0,5) / H`; `up`: `1 − (y + 0,5) / H`), o en `circular` la distancia al centro `(W / 2, H / 2)` dividida por media diagonal `√(W² + H²) / 2` (PROPUESTA); con suavidad `s` en `[0, 1]`, la máscara es `limita((p × (1 + s) − u) / s, 0, 1)`; con `s = 0`, es 1 si `u < p` y 0 si no. La máscara es el peso de «hacia». | Por transición y por formato, `Rgba8` y `RgbaF32`: con `p` 0 y 1, igualdad byte a byte, también con suavidad 1 en Borrado. `p` −0,5 da lo mismo que 0 y 1,5 lo mismo que 1. Valores analíticos: Disolvencia a 0,25 sobre 0 y 1 da 0,25; Fundido a 0,5 da el color exacto; Corte a 0,4999 da «desde» y a 0,5 da «hacia». Deslizar sobre 10×4 px a 0,25: `d = 8`; en `left`, las columnas 8 y 9 son las columnas 0 y 1 de «hacia» y el resto es «desde»; en `right`, las columnas 0 y 1 son las columnas 8 y 9 de «hacia»; en `up`, con `d = 3`, la fila 3 es la fila 0 de «hacia»; en `down`, la fila 0 es la fila 3 de «hacia»; un «hacia» de alfa 0 borra «desde» donde entra. Borrado con `s = 0` y `p` 0,5 sobre 4×2 px: `right` da «hacia» en las columnas 0 y 1; `left`, en las columnas 2 y 3; `down`, en la fila 0; `up`, en la fila 1. Zoom: dos imágenes opacas dan alfa opaco en todo píxel y todo `p`; `out` con «desde» `a`, «hacia» `b` y `p` coincide ≤ 1/255 con `in` con «desde» `b`, «hacia» `a` y `1 − p`; dos imágenes constantes del mismo color dan ese color. Borrado circular con `p` 1 no deja ningún píxel de «desde», esquinas incluidas. Imagen de tamaño 0: `Ok` sin cambios. Los `Err` se dan antes de escribir: `progress` NaN nombra `progress`; tamaños distintos, `size_mismatch` con el campo `to` o `dst`; formatos distintos, `format_mismatch` con ese campo; `Alpha8`, `unsupported_format`. Con `p` 0,0001 y 0,9999, Disolvencia, Fundido y Borrado con `s = 1` quedan a ≤ 1/255 de su extremo. |
| ARQ-119 | **Mezcla y composición.** `gafira-raster` contiene las fórmulas de los 16 modos de `BlendMode::ALL` (D-27), según W3C *Compositing and Blending Level 1*, con composición *source-over* premultiplicada. `gafira-render-cpu` las usa, no las redefine. Antes de un modo distinto de `Normal`, los canales `f32` se recortan a `[0, 1]`, salvo con fuente de alfa 0: en todo modo, esa fuente deja el fondo igual bit a bit, sin recorte. | Por modo, tabla de al menos 256 ternas (fondo, fuente, alfa) frente a una referencia en `f64` de la fórmula W3C: error ≤ 1e-5 en `f32` y ≤ 1/255 en 8 bits. Por modo: fuente con alfa 0, sea cual sea su color, deja el fondo igual bit a bit, también con el fondo `(1,5; 0,5; 0,2; 1)` y con `−0,0`; un NaN en esa fuente o ese fondo sigue dando `Err`; fondo con alfa 0 da la fuente; `Normal` con alfa 1 da la fuente. |
| ARQ-120 | **Desenfoque, bordes y escala.** El desenfoque es gaussiano con `σ = radio / 2`, como `blur()` de CSS, y radio 0 es la identidad. Fuera de la imagen, los píxeles son negro transparente. Los radios se expresan en px lógicos y se multiplican por la `scale` del contexto del efecto. Los desplazamientos de sombra (04 FD-35) también son px lógicos: se multiplican por esa `scale` y se redondean a píxeles enteros, y medio se aleja de 0. No hay desplazamiento subpíxel. `roi` devuelve la región de entrada necesaria con la `scale` del `cx` que recibe. | Un impulso da una respuesta gaussiana con `σ` medida dentro de ±2 %. Una imagen constante opaca conserva su valor a más de `3σ` del borde. Con `scale` 1, los desplazamientos 0,5, −0,5 y −1,5 mueven 1, −1 y −2 px; con `scale` 0,5, 3 y −3 mueven 2 y −2 px. Una imagen suave y su mitad con `scale` 0,5 coinciden ≤ 1/255 tras reescalar, con radio ≥ 16 px a `scale` 1 y a más de `3σ` del borde de la imagen. Suave significa sin bordes duros: por ejemplo, un gradiente de baja frecuencia, con ≤ 1/255 por canal entre píxeles vecinos. Si hay desplazamiento de sombra, su valor escalado es entero en los dos tamaños; con `scale` 1 y 0,5, eso pide desplazamientos pares. La cota no se aplica a bordes duros con radios pequeños, donde ninguna gaussiana puede cumplirla. En el desenfoque de radio `r`, `roi` con `scale` 2 contiene `out` ampliado `⌈2·r⌉` px por cada lado. Ejecutar por tiles con `roi` coincide con la imagen entera ≤ 1/255 en todos los píxeles, con `scale` 1, 0,5 y 2. |
| ARQ-121 | **Tiles y mipmaps.** Los tiles miden 256×256 px (FIJADA, §6.1). Cada nivel de mip divide cada lado por 2 con redondeo hacia abajo, con mínimo 1. El nivel promedia cajas de 2×2 en luz lineal con alfa premultiplicado. Ningún píxel se pierde: en un lado impar mayor que 1, la última fila o columna se une a la última caja, que cubre 3 en vez de 2. Un lado de 1 no se divide. Al promediar, cada píxel pesa por el área de la imagen original que cubre, así que todo píxel de un nivel es la media de los píxeles originales que cubre, con el mismo peso para cada uno. Hay `⌊log2(max(ancho, alto))⌋ + 1` niveles. | Imagen de 1000×600: 10 niveles, el último de 1×1. Imagen constante: todos los niveles iguales. Ajedrez sRGB 0 y 255 opaco: el nivel 1 da 188 ±1. El nivel de un píxel `a = 0` no aporta color. Una imagen de 257×1 se reparte en 2 tiles. Lado impar, `Alpha8`: 3×1 y 1×3 con 0, 0 y 255 dan 85 ±1; 3×3 con la última fila y la última columna a 255 da 142 ±1; el nivel 1 de 5×1 con 0, 0, 0, 255 y 255 da 0 y 170 ±1. `Rgba8` opaco: 3×1 con sRGB 0, 0 y 255 da 156 ±1. 1000×600 opaco, con 512 columnas a 0 y 488 a 255: el nivel de 1×1 da 185 ±2 en `Rgba8`, cerca de la media exacta de 185,49, y no 0 ni 156. Una imagen de 1×1 tiene 1 nivel, igual a la entrada. |
| ARQ-122 | **Paleta y hash perceptual.** La paleta da hasta 5 colores dominantes, ordenados por área, e ignora los píxeles con alfa 0. El dHash (PROPUESTA) da 64 bits, y dos imágenes son parecidas si su distancia de Hamming es ≤ 10 (PROPUESTA). Lab, ΔE2000 y ΔE76 los aporta `gafira-color`. Lab sigue W3C *CSS Color Module Level 4* (§19): blanco D50 de cromaticidad `(0,3457; 0,3585)`, matriz Bradford de D65 a D50, `ε = 216/24389` y `κ = 24389/27`. ΔE2000 es CIEDE2000 con `kL = kC = kH = 1`. ΔE76 es la distancia euclídea en Lab (CSS Color 4 §20.1). `gafira-color` es capa 0: son datos planos, no valida y ARQ-110 no le aplica; una entrada no finita no está especificada ahí, y el motor que llama comprueba antes sus números. | Imagen sintética de 5 colores planos con áreas conocidas: los 5 colores con ΔE2000 < 1, en orden de área. Imagen de 2 colores: 2 entradas. Imagen transparente: lista vacía. Una imagen y su versión al 50 %: distancia ≤ 10; dos imágenes de ruido con semillas distintas: > 10. `delta_e2000` coincide ±1e-4 con los 34 pares de la tabla I de Sharma, Wu y Dalal (*Color Research and Application*, 30(1), 2005, pp. 21-30), que publica 4 decimales. Lab desde sRGB lineal: blanco `(1, 1, 1)` da `L` 100, `a` 0 y `b` 0, ±1e-3; negro da 0, 0 y 0, ±1e-3; azul `(0, 0, 1)` da 29,567, 68,298 y −112,0294, y amarillo `(1, 1, 0)` da 97,607, −15,753 y 93,388, ±0,02 (valores de CSS Color 4 §9.1; el código de su §19 los reproduce con una diferencia de hasta 0,011). `delta_e76` de `(50, 0, 0)` y `(53, 4, 0)` da 5 ±1e-12. |

| Caso numérico | Regla | ID |
|---|---|---|
| Píxel `f32` NaN o `±∞` | `Err` antes de escribir en destino. | ARQ-110 |
| Parámetro no finito o fuera de rango | `Err` antes de escribir en destino. | ARQ-110, FD-33 |
| `progress` finito fuera de `[0, 1]` | Se recorta: las curvas con rebote lo producen (§3.4). | ARQ-118 |
| Imagen de tamaño 0 | `Ok` sin cambios. | ARQ-118 |
| Producto `ancho × alto × canales` que desborda | `Err`, sin reservar memoria. | ARQ-117 |
| Canal `f32` fuera de `[0, 1]` | Se admite, salvo en modos de mezcla y curvas que lo recortan. Una fuente de alfa 0 no recorta el fondo. | ARQ-119, FD-33 |
| División por alfa 0 | No ocurre: el resultado es 0. | ARQ-117 |
| Canal 0 en un cociente (cuentagotas) | El ajuste no cambia. | FD-33 |
| Tolerancia de las pruebas | Identidades, diferencia 0; fórmulas `f32`, 1e-5; 8 bits, 1/255. | ARQ-118, ARQ-119 |

#### 4.17.5 `gafira-audio`

| Primer corte | Fuera del corte |
|---|---|
| ARQ-23, ARQ-123 a ARQ-129, CAL-15 (`bench_mix_16`), FV-04 (b, longitud de audio), FV-10 (a, b, d), FV-20 (c, d) | `gafira-media` y exportación: FV-10 (c), FV-12. Interfaz del Mezclador: FV-20. Fase 2: limitador (FV-20 e), puerta de ruido, compresor, reducción de ruido, *time-stretch* y medidores en vivo. |

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-123 | **`process` es total.** `AudioProcessor::process` no devuelve error. Una muestra NaN o `±∞` cuenta como 0, un parámetro no finito se ignora y queda el último válido, y un parámetro fuera de rango se recorta. Ninguna salida es no finita ni subnormal. Si `prepare` recibió `sample_rate`, `channels` o `max_block` iguales a 0, si el bloque está vacío (`frames() == 0`), sea cual sea el `prepare`, si supera `max_block`, si sus canales tienen longitudes distintas o si su número difiere del de `prepare`, `process` no procesa, no lee ningún parámetro y su estado no cambia: solo limpia el bloque, con NaN, `±∞` y subnormales a 0. `process` no recorta a ±1: una muestra puede superar 0 dBFS en la mezcla. | Por procesador: un bloque con NaN, `+∞` y `−∞` da salida finita y los bloques siguientes también. Tras 1 s de señal a −6 dBFS y 1 s de ceros, ninguna muestra es subnormal. Con `prepare` en 0, con un bloque mayor que `max_block`, con canales de 4 y 3 muestras y con 1 canal tras `prepare` con 2, y NaN, `+∞`, `−∞` y un subnormal en el bloque: esas muestras dan 0 y las demás quedan idénticas, y el siguiente bloque válido da lo mismo que si ese no hubiera llegado. Bloque vacío, sin canales o con canales de 0 muestras, con `volume` 50, `duration` 0,001 o `pan` −1: tras un `prepare` válido, el bloque siguiente sin parámetros sale × 1,0 en `Gain` y `Fade` e igual en `Pan`, como si el vacío no hubiera llegado. Lo mismo si llega tras cada `prepare` en 0 (`max_block`, `channels` o ambos) y antes de un `prepare` válido. |
| ARQ-124 | **Ganancia, fundido y panorama.** Ganancia: `10^(dB / 20)`; −∞ dB es 0 y 200 % es 2,0. Un cambio de ganancia se aplica con una rampa lineal de 10 ms (PROPUESTA). El primer bloque procesado tras `prepare` o `reset` no tiene valor previo: su valor rige desde la muestra 0, sin rampa. Si ese bloque no trae valor, rige el último válido o el valor por defecto, y un valor posterior es un cambio. Un bloque que `process` no procesa (ARQ-123), como todo bloque vacío, no es ese primer bloque. Fundido de clip: rampa lineal de ganancia de 0 a 1 o de 1 a 0 (PROPUESTA); duración 0 no cambia nada. La entrada y la salida son dos instancias de `Fade`, cada una con su duración y su `ParamSnapshot`, fuera de la `ProcessorChain` de la pista (§4.9). Cuentan muestras desde `prepare` o `reset`, y `gafira-media` llama a `reset` en la primera muestra de la rampa: la entrada empieza en la primera muestra del clip y la salida termina en la última. El primer bloque procesado tras `prepare` o `reset`, que nunca está vacío, fija la longitud `L` de la rampa con el último valor válido de `duration`, contando el de ese bloque. `L` rige hasta el siguiente `prepare` o `reset`, para que la ganancia siga continua (05 FV-10). Un valor válido en un bloque procesado posterior no cambia ninguna muestra hasta entonces, pero pasa a ser el último válido. Al terminar, `Fade` mantiene su valor final. Una rampa de `L = redondeo(duración × fs)` muestras vale `n / L` en la muestra `n` al entrar y `1 − n / L` al salir. Fundido cruzado `EqualPower`: `cos(π p / 2)` y `sin(π p / 2)`. Panorama en bloques estéreo, con `p` en `[−1, 1]`, `θ = (p + 1) × π / 4`: `L × min(1, √2 cos θ)` y `R × min(1, √2 sin θ)` (ley de −3 dB normalizada a 0 dB en el centro, PROPUESTA). En otros bloques, `Pan` no cambia nada. | +6 dB da 1,9953 ±1e-4. Un salto de ganancia de 0 a 1 a 48 kHz cambia ≤ 1/480 + 1e-6 por muestra. Con `volume` 0 en el primer bloque, toda muestra sale 0 desde la muestra 0; con 50, la muestra 0 sale × 0,5; tras `reset`, lo mismo. Un primer bloque sin `volume` y un segundo con 0 dan rampa. Un bloque vacío sin `volume`, seguido de uno con 0, da 0 desde la muestra 0, sin rampa; un bloque vacío con 0, seguido de uno sin `volume`, deja las muestras × 1,0; tras un bloque con `volume` 100, un bloque vacío con 0 y uno sin `volume` dan × 1,0, sin rampa. Un fundido de 2 s a 48 kHz sube 1/96 000 + 1e-6 por muestra como máximo, de 0 a 1. Con esa rampa, la entrada vale 0 en la muestra 0 y 1 desde la 96 000, y la salida vale 1 en la 0 y 0 desde la 96 000. Cambiar `duration` a mitad de rampa no cambia ninguna muestra hasta el siguiente `reset`; pasar de duración 0 a 2 s sin `reset` tampoco. A 48 kHz, un bloque vacío sin `duration`, seguido de uno con 2, da la entrada de 2 s desde la muestra 0. A 48 kHz, con `duration` 0,02 en el primer bloque, 0,005 en un bloque a mitad de rampa, `reset` y un bloque sin `duration` o con NaN, la rampa sube 1/240 ±1e-6 por muestra; si ese 0,005 llega en un bloque vacío, sube 1/960 ±1e-6; si el bloque tras `reset` trae 0,04, sube 1/1920 ±1e-6. `EqualPower`: `g_a² + g_b² = 1`, ±1e-6, en todo `p`. Panorama: `p = 0` deja el bloque igual, ±1e-6; `p = −1` da `R` ≤ 1e-6 y `L` igual; `p = 0,5` da `L × 0,5412` ±1e-4 y `R` igual. |
| ARQ-125 | **`Eq3Band`** (D-22): tres biquads del *Audio EQ Cookbook* de RBJ: estante de graves en 100 Hz con `S = 1`, campana en 1 kHz con `Q = 1` y estante de agudos en 8 kHz con `S = 1` (PROPUESTA), ±15 dB. La curva teórica de 05 FV-20 es la respuesta digital de esos biquads. Con ganancia 0, la banda no cambia la señal. Una banda con frecuencia ≥ `fs / 2` se omite. Una banda está activa si no se omite y su ganancia vigente, el último valor válido o, sin él, el valor por defecto, no es 0. `tail()` es `ceil(0,04 × fs)` *frames* (PROPUESTA) con alguna banda activa y 0 si no; ese cálculo es exacto, sin redondeo de coma flotante. Un cambio de ganancia de banda rige desde el siguiente bloque procesado, el que trae el valor, desde su muestra 0 y sin rampa, y el historial de los filtros continúa: la rampa de ARQ-124 es solo de `Gain`. | A 44,1, 48 y 96 kHz, con cada banda a ±15 dB y 0 dB: la respuesta medida con barrido o impulso coincide con `H(e^jω)` en ±0,5 dB de 20 Hz a 20 kHz. Con todas a 0 dB, la salida es idéntica bit a bit. Todos los polos tienen módulo < 1. A `fs = 16 000 Hz`, la banda de 8 kHz se omite. `tail()` con una banda activa: 1 764 a 44 100 Hz, 1 920 a 48 000 Hz, 3 840 a 96 000 Hz, 1 765 a 44 101 Hz y 171 798 692 a `fs = u32::MAX`, sin `panic`. Da 0 sin bandas activas: con todas a 0 dB o −0 dB, con `sample_rate` 0 y con solo `high` a +6 dB a 16 000 Hz; esta última da 641 a 16 001 Hz. El valor se conserva tras `reset` y tras `prepare` a la misma `fs`, y un bloque rechazado no lo cambia. A 44,1, 48 y 96 kHz, tras un impulso de amplitud 1, con toda combinación de −15, 0 y +15 dB con alguna banda activa: desde el *frame* `tail()`, toda muestra tiene módulo < 1e-6 (−120 dB). Un cambio de 0 a +15 dB en `mid` sobre un tono: la salida del bloque que lo trae coincide ±1e-6 con los biquads nuevos partiendo del historial de entrada y salida del bloque anterior. |
| ARQ-126 | **Sonoridad integrada.** Sigue ITU-R BS.1770-4 y EBU R128: ponderación K, bloques de 400 ms con solape del 75 %, puerta absoluta de −70 LUFS y relativa de −10 LU, y pesos 1,0 para L, R y C, 1,41 para Ls y Rs, y sin LFE. Admite 1, 2 y 6 canales; otro número da `Err`. Con 6 canales, el orden es L, R, C, LFE, Ls, Rs, con índices 0 a 5 (PROPUESTA); un adaptador con otro orden, como Opus (RFC 7845 §5.1.1.2), reordena antes de llamar. En otras frecuencias, la ponderación K coincide con la de 48 kHz en ±0,1 dB hasta 16 kHz. Una señal sin bloques válidos, por silencio o por durar menos de 400 ms, da un resultado «sin medida», no −∞. | Señales sintéticas de los casos 1 a 5 de EBU Tech 3341, sin guardar sus archivos: ±0,1 LU. Seno estéreo de 997 Hz a −23 dBFS: −23,0 ±0,1 LUFS a 44,1, 48 y 96 kHz. Silencio y una señal de 399 ms: «sin medida». 3 canales: `Err`. Con 6 canales y un seno de 997 Hz a −20 dBFS en un solo canal: índices 0, 1 o 2, −23,0 ±0,1 LUFS; índices 4 o 5, −21,5 ±0,1 LUFS; índice 3, «sin medida». |
| ARQ-127 | **Nivelar** (D-21, D-31). Nivelar tiene su propia ganancia, aparte del Volumen del clip, y la aplica `LoudnessNormalizer`. La ganancia pedida es `objetivo − sonoridad medida`. El tope es `−1 dBTP − pico real medido`. La ganancia aplicada es la menor de las dos, y el resultado indica si se alcanzó el objetivo; la igualdad lo alcanza. El pico real sigue el anexo 2 de BS.1770 con sobremuestreo ×4 a 48 kHz o menos. La ganancia es constante en toda la señal. Sin medida, la ganancia es 0 y no se alcanza el objetivo. | Señal de −20 LUFS con pico holgado y objetivo −14: −14 ±0,5 LU y objetivo alcanzado. Señal de −30 LUFS y −3 dBTP con objetivo −14: ganancia +2 dB ±0,01, pico real ≤ −1 dBTP, −28 ±0,5 LU y objetivo no alcanzado. Silencio: ganancia 0 y objetivo no alcanzado. Una ganancia de +16 dB se aplica con el Volumen al 100 %, sin cambiarlo. |
| ARQ-128 | **Remuestreo.** Admite una razón constante entre cualquier par de frecuencias de `SampleRate`, con Velocidad de 0,1 a 10. La longitud de salida es `redondeo(n × fs_salida / (fs_entrada × velocidad))`, evaluada en `f64`: esa evaluación es normativa también en los empates. Banda pasante ±0,1 dB hasta 0,9 veces el Nyquist menor y atenuación ≥ 80 dB en banda eliminada, que empieza en 1,0 vez el Nyquist menor (PROPUESTA). La latencia se declara. | 44 100 muestras de 44,1 a 48 kHz dan 48 000; 96 000 muestras a 2,0× dan 48 000. Seno de 1 kHz de 44,1 a 48 kHz: SNR ≥ 80 dB. Un impulso sale en la latencia declarada, ±1 muestra. Entrada vacía: salida vacía. Velocidad 0, negativa, 0,09 o 10,1: `Err`. |
| ARQ-129 | **Cadena, mezcla y desfase.** El `tail()` de un procesador son los *frames* que su salida sigue resonando tras acabar la entrada. `ProcessorChain` aplica los procesadores en orden, su `latency()` es la suma y su `tail()` es la suma saturada. La mezcla suma las muestras de las pistas no silenciadas. `mix` es total, como `process` (ARQ-123): no devuelve error, una muestra NaN o `±∞` cuenta como 0, la salida es finita y no subnormal, y no recorta a ±1. Una pista con menos canales o muestras que el destino aporta lo que tiene. No hay límite de pistas en el motor: 16 es la cifra del Mezclador. Un desfase en *flicks* pasa a muestras con `redondeo(desfase × fs / 705 600 000)`, medio se aleja de 0, en aritmética entera sin desbordamiento. | Una cadena con latencias 3 y 5 declara 8. Con colas 7 y 11 declara 18; con colas `u32::MAX` y 1, `u32::MAX`, sin `panic`. Sumar 16 pistas coincide con la suma manual ±1e-6. Una pista silenciada no aporta. `mix` con NaN, `+∞` y `−∞` en una pista da la suma de las demás muestras; dos pistas a `f32::MAX` dan una salida finita; una pista de 2 canales sobre un destino de 3 aporta a los 2 primeros. +1 ms a 44,1 kHz da 44 muestras y a 48 kHz, 48. Con `i64::MAX` *flicks*, sin `panic`. |

Parámetros de los procesadores, que forman su `ParamSchema` y son los ids de `ParamSnapshot`. Todos son `Float`. La unidad es solo documentación: `ParamDef` no tiene campo de unidad. `ProcessorChain` no tiene parámetros propios: su esquema es vacío y cada etapa conserva el suyo.

| Procesador | Id | Unidad | Rango | Por defecto | Animable | Etiqueta |
|---|---|---|---|---|---|---|
| `Gain` | `volume` | % | `[0, 200]` | 100 | Sí | `audio.gain.volume` |
| `Fade` | `duration` | s | `[0, f64::MAX]`: todo valor finito ≥ 0 | 0 | No | `audio.fade.duration` |
| `Pan` | `pan` | Sin unidad | `[−1, 1]` | 0 | No | `audio.pan.pan` |
| `Eq3Band` | `low`, `mid`, `high` (PROPUESTA) | dB | `[−15, 15]` | 0 (PROPUESTA) | No (PROPUESTA) | `audio.eq3band.low`, `.mid`, `.high` (PROPUESTA) |
| `LoudnessNormalizer` | `gain` (PROPUESTA) | dB | `[−f64::MAX, f64::MAX]` (PROPUESTA) | 0 | No | `audio.loudness_normalizer.gain` (PROPUESTA) |

Aceptación: por procesador, llamando por `dyn AudioProcessor`, `params()` pasa `ParamSchema::validate_definition` y su id, rango, valor por defecto, animable y etiqueta son los de la tabla; `ProcessorChain` da un esquema vacío. `volume` 250 se recorta a 200, `pan` 2 a 1, `duration` −1 a 0, y una banda de `Eq3Band` 16 a 15 y −16 a −15 (ARQ-123); un valor en cada extremo no cambia; un id distinto de estos no cambia la salida.

| Caso numérico | Regla | ID |
|---|---|---|
| NaN o `±∞` en funciones con `Result` | `Err`. | ARQ-110 |
| NaN o `±∞` en `process` | Muestra a 0; parámetro ignorado. | ARQ-123 |
| Subnormales | Ninguna salida subnormal. | ARQ-123 |
| NaN o `±∞` en `mix` | Muestra a 0. | ARQ-110, ARQ-129 |
| Desbordamiento de `f32` en la mezcla | Sin recorte a ±1; la salida sigue finita. | ARQ-123, ARQ-129 |
| Desbordamiento entero en desfases | Aritmética sin desbordamiento. | ARQ-129 |
| Tamaño 0 | `prepare` en 0: se limpia y no se procesa. Bloque vacío: se limpia y no se procesa. | ARQ-123 |
| Forma del bloque | Mayor que `max_block`, canales desiguales o número distinto del de `prepare`: se limpia y no se procesa. | ARQ-123 |
| Tope de pico | Pico resultante ≤ −1 dBTP; igualdad admitida. | ARQ-127 |
| Sonoridad sin bloques | «Sin medida», nunca −∞. | ARQ-126 |
| Tolerancias | ±0,5 LU en Nivelar; ±0,1 LU en el medidor; ±0,5 dB en EQ; 1 muestra en desfases. | ARQ-125 a ARQ-129 |

#### 4.17.6 `gafira-segment`

| Primer corte | Fuera del corte |
|---|---|
| ARQ-130, ARQ-131, ARQ-132, ARQ-133, ARQ-43 (segmentación), CAL-06 (`bench_segment_4000`), FD-32 (e), FD-38 (a, b, c) | Interfaz y comandos: FD-32 (a a d, f), FD-38 (d), FD-39. Fase 3: FD-40, `onnx`. GPU: CAL-07. |

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-130 | **Contrato de `Segmenter`.** Trabaja sobre una copia de lado largo 2048 px (D-45); una imagen con lado largo ≤ 2048 no se amplía. Los GMM usan solo color, sin término de posición (D-45); la prioridad central de «Elemento» es una pista aparte (D-23) y no entra en ellos. Devuelve una `Segmentation` cuya `mask` es una `AlphaMask` de 8 bits con el ancho y el alto de la entrada. `SegHints` expresa la zona (todo, primer plano, fondo o elemento) y los trazos de semilla de primer plano y de fondo. La prioridad central de «Elemento» (D-23) no es un dato aparte de `SegHints`: elegir esa zona es la pista. Las semillas se refieren al objeto: «Primer plano» y «Elemento» las aplican tal cual, «Fondo» es el complemento exacto de «Primer plano» con las mismas semillas, y «Todo» no segmenta, ignora las semillas y da una máscara sólida de 255. Si un píxel queda bajo semillas de los dos tipos, gana la de primer plano. El refinado de bordes es el *guided filter* de §4.7: da alfa graduado en los bordes, sin una pasada de *matting* aparte. El color de `Rgba8` y `RgbaF32` se lee como está guardado, premultiplicado (ARQ-117), es decir, sobre negro; el alfa de la entrada no interviene aparte en la máscara. `Alpha8` no tiene color y da `Err` (`SegError::UnsupportedFormat`). «4000 px» significa lado largo. | Entradas de 4000×3000, 2048×1536, 800×600 y 1×1: la máscara tiene sus dimensiones. Ancho o alto 0: `Err`. `Alpha8` con ancho y alto > 0, en cualquier zona: `Err` (`UnsupportedFormat`). En «Primer plano» y «Elemento», un píxel bajo una semilla de primer plano da alfa ≥ 243 y bajo una de fondo, ≤ 12; un píxel bajo semillas de los dos tipos da ≥ 243; dos píxeles contiguos con semillas de distinto tipo cumplen cada uno su cota. «Fondo» es el complemento exacto de «Primer plano» con la misma imagen y las mismas semillas: cada píxel vale `255 − alfa`, sin tolerancia, así que da ≤ 12 bajo una semilla de primer plano y ≥ 243 bajo una de fondo. «Todo», con semillas o sin ellas, da 255 en todos los píxeles; una semilla con un número no finito sigue dando `Err` (ARQ-110). Con dos objetos y sin trazos, «Elemento» conserva el del centro y descarta el otro, aunque el otro sea mayor, y «Primer plano» conserva los dos. Un `SegHints` escrito solo con la zona y los trazos, sin valores por defecto, compila. Sujeto gris 220 sobre fondo gris 40 con un borde en rampa de 16 px, 800×600, «Primer plano» sin trazos: al menos un píxel de la rampa tiene 0 < alfa < 255. Dos entradas con el mismo RGB guardado y distinto alfa dan la misma `mask`, bit a bit; `reliable` puede diferir, porque ARQ-131 pondera por el alfa. Sin pistas, dos superpíxeles del mismo color medio en posiciones distintas reciben, bit a bit, el mismo coste frente a cada GMM. |
| ARQ-131 | **Fiabilidad** (D-23). `Segmentation.reliable` juzga la máscara del sujeto en toda zona que segmenta: en «Primer plano» y «Elemento» es la `mask` devuelta; en «Fondo» es la de «Primer plano» con la misma entrada, no su complemento. «Fondo» da así el mismo valor que «Primer plano» con la misma imagen y las mismas semillas. «Todo» no segmenta y se juzga sobre su `mask` sólida. Es verdadero si el primer plano, los píxeles de esa máscara con alfa ≥ 128 como en ARQ-132, ocupa entre el 1 % y el 95 % de la imagen, extremos incluidos, y la distancia ΔE76 entre los colores medios de primer plano y fondo es ≥ 5 (PROPUESTA). Esos colores se promedian en luz lineal, ponderados por el alfa de la entrada, y después se pasan a Lab (D-44). Un píxel con alfa 0 no aporta color, y un grupo sin alfa no tiene color medio, así que el resultado no es fiable. Un resultado no fiable no es un error: es `Ok` con `reliable` falso y conserva su máscara. | Imagen uniforme, ruido uniforme sin sujeto y una escena con sujeto del 0,5 %: `Ok` con `reliable` falso y máscara de las dimensiones de la entrada. Ruido uniforme significa que cada píxel se sortea aparte, con semilla fija. Un ruido por bloques es otra entrada: con bloques de 64 px hay regiones que separar, el resultado puede ser fiable y esta fila no lo prohíbe. Regla con máscara y colores dados: 1 % y 95 % son fiables, y 0,99 % y 95,01 % no; ΔE76 5 es fiable, y 4,999 no. Si no hay píxeles de primer plano o de fondo, no es fiable, sin NaN. Un primer plano con la mitad de los píxeles negros y la mitad blancos tiene un color medio de `L` 76,07 ±0,01, no 50. Con alfa de entrada, un píxel blanco de alfa 1 y uno negro de alfa 0 dan `L` 100 ±0,01; uno blanco de alfa 0,25 y uno negro de alfa 0,75 dan `L` 57,08 ±0,01; un fondo con todos sus píxeles de alfa 0 da no fiable, sin NaN. Las escenas del conjunto fijo de ARQ-132: fiable. Con la misma imagen y las mismas semillas, «Fondo» y «Primer plano» dan el mismo `reliable`: los dos fiable con un sujeto del 2 %, cuya máscara de «Fondo» cubre el 98 %; los dos no fiable con un sujeto del 0,5 % y con uno del 96 %, cuya máscara de «Fondo» cubre el 4 %. «Todo»: `reliable` falso. |
| ARQ-132 | **Puerta de calidad** (D-20). `gafira-testkit` genera dos conjuntos deterministas de escenas sintéticas con máscara exacta conocida, de 4000×3000 px (Q-49): el conjunto fijo de 16 escenas, que son las definiciones de XP5 de la tabla de abajo, las mismas con las que se alcanzó D-45, y 12 escenas nuevas. Rango de composición de los dos conjuntos: fondo plano, degradado y texturizado; textura por píxel; sujeto centrado y desplazado; formas cóncavas y con agujero; contraste bajo; bordes duros y suaves; sujeto entre el 2 % y el 50 % del área; colores medios exactos de sujeto y fondo a ΔE76 ≥ 6. Textura por píxel significa que cada píxel de fondo sortea su ruido aparte, sin celdas. Borde suave significa una rampa lineal de 16 px entre sujeto y fondo (PROPUESTA); su máscara exacta son los píxeles con al menos la mitad de color de sujeto. Borde duro significa que no hay rampa. Contraste bajo significa sujeto y fondo planos a ΔE76 entre 6 y 10, extremos incluidos (PROPUESTA). Ese margen sale de la puerta: con IoU 0,90, una máscara imperfecta acerca los colores medios hasta un 10,6 % con sujeto ≤ 50 %, y 6 × 0,894 > 5 conserva la fiabilidad de ARQ-131. Las escenas texturizadas conservan ese mínimo y no tienen otro propio, pero un fondo texturizado a ΔE76 6 exacto queda fuera del conjunto (D-45): en ellas, ΔE76 > 6. El conjunto fijo queda dentro del rango sin cubrirlo entero: sus bordes son duros, y sus degradados y texturas tienen ΔE76 objetivo 25. Las 12 escenas nuevas cubren el rango entero, con degradado y textura hasta unos 7, es decir, ΔE76 entre 7 y 8. Sus recetas son distintas de las del conjunto fijo: salen de `gafira-testkit` con otra semilla, no son las mismas formas con otros colores y nunca se usan para ajustar parámetros. La prueba bloqueante exige IoU ≥ 0,90 por escena del conjunto fijo, no en media, binarizando con alfa ≥ 128. Con las dos máscaras vacías, el IoU es 1. Las 12 escenas nuevas corren como aviso: informan de cuántas llegan a IoU ≥ 0,90 y no bloquean (D-45). Por tanto, D-20 se cumple en las escenas de la puerta y no está demostrado que generalice: el resultado conocido es 9 de 12 en las 12 escenas nuevas de XP5 (D-45). Esa cifra no es un umbral ni el recuento del conjunto nuevo de esta fila, que no son esas escenas. Sobre fotos reales, el IoU es solo un aviso hasta que el titular aporte un conjunto con licencia (pendiente 9). | Una prueba recorre los dos conjuntos a 4000×3000 y comprueba con su máscara exacta el área del sujeto y ΔE76, y que las texturizadas tienen ΔE76 > 6. Conjunto fijo: cada escena coincide con su fila de la tabla en fondo, forma, posición y ΔE76 objetivo; la 15 ocupa el 2,4 % ±0,1 y la 16, el 45 % ±0,1; las escenas 11, 12 y 14 son las de contraste bajo, y sujeto y fondo son planos a ΔE76 entre 6 y 10. Cada conjunto tiene al menos una escena de cada clase: fondo plano, degradado y texturizado, sujeto centrado y desplazado, forma cóncava, forma con agujero y contraste bajo. Escenas nuevas: al menos una con área ≤ 5 % y una con área ≥ 40 %; al menos una degradada y una texturizada con ΔE76 entre 7 y 8, extremos incluidos; al menos una de borde duro y una de borde suave, y esta tiene píxeles con un color estrictamente entre los de sujeto y fondo; ninguna máscara exacta llega a IoU ≥ 0,90 con la de una escena del conjunto fijo. Textura por píxel, en los dos conjuntos: al menos la mitad de los pares de píxeles de fondo contiguos en horizontal tienen distinto color; una textura de celdas de 12 px no cumple. La prueba de escenas falla si una sola de las 16 queda por debajo de 0,90. Prueba del cálculo de IoU: 0,90 pasa y 0,8999 falla. Aviso: la semilla de las 12 escenas nuevas es distinta de la del ruido del conjunto fijo, y un segmentador que falla en todas informa `0/12` sin que la ejecución falle. Mientras no exista el conjunto real, ninguna prueba bloqueante depende de él. |
| ARQ-133 | **Corte de `ClassicSegmenter`.** La arista entre dos superpíxeles contiguos pesa `gamma` por la longitud de su borde compartido por el término de color. Esa longitud es el número de pares de píxeles 4-vecinos de la copia de trabajo con un píxel en cada superpíxel. Un superpíxel con al menos la mitad de sus píxeles en la banda de borde queda fijado como fondo. Las semillas van primero: un superpíxel con semilla sigue su semilla (ARQ-130), aunque esté en la banda. La regularización de GMM es de unos 25 y `gamma`, de unos 500. «Unos» significa ±20 % (PROPUESTA): de 20 a 30 y de 400 a 600. Esos dos valores solo se ajustan con las escenas de la puerta. | Con dos pares de superpíxeles de los mismos colores, el par con el doble de borde compartido tiene el doble de capacidad, salvo el redondeo de la capacidad. Un superpíxel de `2n` píxeles con `n` en la banda queda fijado como fondo; con `n − 1`, no. Una semilla de primer plano sobre un superpíxel de la banda lo deja en primer plano, con alfa ≥ 243 bajo la semilla. Sin semillas, ningún superpíxel fijado acaba en primer plano. Si todos quedan fijados como fondo, el resultado es `Ok`, sin píxeles de primer plano y no fiable. Una prueba comprueba que la regularización y `gamma` están en sus tramos, extremos incluidos. Una semilla sigue dirigiendo el corte en imágenes alargadas, en las dos orientaciones. |

Conjunto fijo de ARQ-132. Las medidas van en unidades del alto, con un ancho de 4/3. Un disco es `(cx, cy, r)` y una elipse, `(cx, cy, rx, ry)`. El sujeto es plano, en Lab `(55, 8, −12)` más un desplazamiento de dirección `(0,6; 0,5; −0,6)`, salvo la escena 14, de dirección `(0; 0,7; 0,7)`, calibrado para quedar en el ΔE76 objetivo o justo por encima. El fondo parte del mismo Lab. El degradado suma `20 × (u/ancho − 0,5)` a `L` y `8 × (v − 0,5)` a `a`. La textura suma a `L` `6 × sen(60u) × sen(50v)`, y a cada canal de 8 bits, un ruido por píxel de ±10. Todos los bordes son duros.

| Escena | Fondo | Forma | Posición | ΔE76 objetivo |
|---|---|---|---|---|
| 01 | Plano | Elipse `(0,667; 0,5; 0,34; 0,25)` | Centrada | 40 |
| 02 | Plano | Rectángulo redondeado, centro `(0,35; 0,62)`, semilados 0,2 y 0,15, radio 0,05 | Desplazada | 30 |
| 03 | Degradado | Disco `(0,667; 0,5; 0,32)` | Centrada | 25 |
| 04 | Degradado | Estrella de 5 puntas `r ≤ 0,22 × (1 + 0,35 cos 5θ)`, centro `(0,95; 0,4)` | Desplazada | 25 |
| 05 | Texturizado | Disco `(0,667; 0,5; 0,28)` | Centrada | 25 |
| 06 | Texturizado | Elipse `(1,0; 0,6; 0,25; 0,17)` | Desplazada | 25 |
| 07 | Plano | Media luna: disco `(0,667; 0,5; 0,33)` menos disco `(0,8; 0,5; 0,27)` | Centrada | 30 |
| 08 | Degradado | L: `x` 0,35 a 0,62 con `y` 0,2 a 0,8, más `x` 0,35 a 1,0 con `y` 0,58 a 0,8 | Centrada | 25 |
| 09 | Plano | Anillo: disco `(0,667; 0,5; 0,34)` menos disco `(0,667; 0,5; 0,17)` | Centrada | 30 |
| 10 | Texturizado | Rectángulo `x` 0,3 a 1,0, `y` 0,2 a 0,8, menos discos `(0,5; 0,5; 0,1)` y `(0,8; 0,5; 0,1)` | Centrada | 25 |
| 11 | Plano | La de la escena 01 | Centrada | 6, contraste bajo |
| 12 | Plano | Estrella como la 04, de `r` 0,22, centro `(0,9; 0,55)` | Desplazada | 8, contraste bajo |
| 13 | Plano | Anillo: disco `(0,5; 0,5; 0,26)` menos disco `(0,5; 0,5; 0,13)` | Desplazada | 10 |
| 14 | Plano | La de la escena 07 | Centrada | 6, contraste bajo |
| 15 | Plano | Disco `(0,667; 0,5; 0,10)` | Centrada | 30 |
| 16 | Plano | Elipse `(0,667; 0,5; 0,53; 0,36)` | Centrada | 30 |

La escena 13 no cuenta como contraste bajo: al cuantizar a 8 bits su ΔE76 exacto es 10,54, por encima de 10. El objetivo no es una cota de la escena; las cotas son las de ARQ-132.

Parámetros de partida de `ClassicSegmenter` (PROPUESTA; se pueden cambiar sin tocar esta spec mientras se cumplan ARQ-132 y CAL-06, salvo la regularización de GMM y `gamma`, que fija ARQ-133): unos 1 500 superpíxeles SLIC, 5 componentes por GMM, hasta 5 iteraciones de GrabCut, *guided filter* de radio 8 px, inicialización por una banda de borde, una regularización de GMM de unos 25 y `gamma` de unos 500. Estos tres últimos valores se ajustaron sobre las escenas de la puerta, así que pasar ARQ-132 con ellos no prueba que generalicen (D-45: 9 de 12 en las escenas nuevas de XP5). El tamaño de 2048 px y el uso de solo color no son parámetros de partida: los fija D-45.

Coste de la copia de 2048 px frente a la de 1024: 2048×1536 son 3 145 728 píxeles y 1024×768 son 786 432, cuatro veces más en cada paso que recorre la copia. Con 1024, el prototipo hizo 4000×3000 en 0,3 a 1,6 s con un pico de 585 MB, en un equipo que no es el de D-03, y la estimación inferida para D-03 era de 1,3 a 3 s. Si todo escalara por cuatro, darían 5,2 a 12 s y 2,3 GB, por encima de CAL-06 y de CAL-26, y esa cota era alta, porque el escalado final no depende de la copia. Con 2048, lo medido en SG3 queda muy por debajo, también en un equipo que no es el de D-03: 1055 ms con 1 hilo y un pico de *heap* de 35 a 303 MB. Son cifras indicativas hasta medir en el hardware de D-03. Los presupuestos no cambian (Q-48).

Hueco conocido y aceptado (D-45, Q-45): un fondo texturizado a ΔE76 6 queda fuera de la puerta. Pasa con `gamma` 50 y falla con 500. Las escenas texturizadas de la puerta conservan el mínimo de ARQ-132.

| Caso numérico | Regla | ID |
|---|---|---|
| Píxel `f32` NaN o `±∞` | `Err`. | ARQ-110 |
| Imagen de tamaño 0 | `Err`. | ARQ-130 |
| Imagen de 1×1 o uniforme | `Ok`, no fiable. | ARQ-131 |
| Tamaño de trabajo | Lado largo > 2048: reduce; = 2048 o menos: no. | ARQ-130 |
| Semillas de los dos tipos en un píxel | Gana la de primer plano. | ARQ-130 |
| Umbral de IoU | `≥ 0,90`, igualdad incluida. | ARQ-132 |
| Límites de área | 1 % y 95 % incluidos. | ARQ-131 |
| Superpíxel con la mitad justa de píxeles en la banda | Queda fijado como fondo. | ARQ-133 |
| Superpíxel de la banda con semilla | Manda la semilla. | ARQ-133 |
| Todos los superpíxeles fijados como fondo | `Ok`, sin primer plano, no fiable. | ARQ-131, ARQ-133 |
| Sujeto que toca el borde de la imagen, sin semillas | Lo que cae en superpíxeles fijados queda como fondo. Una semilla de primer plano lo recupera. Sin cota de IoU. | ARQ-133 |
| Cancelación | Sin reloj en las pruebas unitarias (ARQ-112). | ARQ-112 |

---

## 5. Patrones transversales

### 5.1 Command + Memento: historial (ADR-03)

**ADR-03: historial híbrido.**
`Document` es un árbol de `Arc` con compartición estructural propia: `Arc<Page>`, `NodeArena` con `Arc<Node>` y `Arc::make_mut` a lo largo de la ruta modificada.

- Cada entrada de historial guarda una **instantánea `Arc<Document>`** (Memento, O(1) en tiempo y O(nodos tocados) en memoria) y el `Command` que la produjo.
- **Deshacer** restaura la instantánea, lo que es infalible. `revert` se usa cuando, por el presupuesto de memoria del historial (por defecto 512 MiB = 536 870 912 bytes, FIJADA), las instantáneas antiguas se compactan y solo quedan los comandos.
- Los tiles de máscaras pintadas y de píxeles son `Arc<Tile>`: un trazo de pincel solo copia los tiles tocados.
- **Fusión de gestos**: los comandos con igual `merge_key` dentro del mismo gesto (puntero presionado) o en menos de 500 ms (teclas de flecha, sliders) se fusionan en una entrada. `History` nunca lee el reloj: quien llama pasa en cada `execute` un instante monótono (`Duration`), que el motor obtiene de su reloj inyectado.
- Las instantáneas inmutables permiten que render, autoguardado y exportación lean versiones coherentes desde otros hilos sin bloqueos.
- *Descartado*:
  - `im`/`imbl`/`rpds`: son MPL-2.0 y la política de §9 los evita.
  - Solo comandos (*command sourcing* puro): deshacer frágil si un `revert` tiene un error.
  - Solo instantáneas completas: consumo de memoria inaceptable con imágenes.

### 5.2 Observer / bus de eventos

```rust
pub enum EngineEvent {
    DocumentChanged { version: DocVersion, dirty: DirtySet },
    SelectionChanged(SelectionSnapshot),
    SaveState(SaveState),                     // Guardando | Guardado(t) | Error(e)
    JobProgress { job: JobId, fraction: f32, eta: Option<Duration> },
    JobFinished { job: JobId, result: JobResult },
    BackendChanged(BackendKind), MemoryPressure(Level), RecoveryAvailable(Vec<RecoveryEntry>),
    LicenseChanged(LicenseStatus), LinksMissing(Vec<AssetRef>),   // §13 y medios vinculados ausentes (§7.1)
    LocaleChanged { preference: LanguagePreference, resolved: ResolvedLocale }, // §14: Ajustes → General → Idioma
    StartupProgress { step: StartupStep, fraction: f32 },  // §8.4: barra de la Pantalla de arranque (D-09)
    StartupFinished(Result<(), Message>),                  // §8.4: cierra la Pantalla de arranque o muestra el error
}
```

`StartupProgress` y `StartupFinished` los publica la secuencia de arranque (§8.4). El *composition root* crea el canal **antes** que el engine, entrega el receptor a la Pantalla de arranque y pasa el emisor a la secuencia, que corre fuera del hilo principal; así la barra recibe eventos desde el primer paso.

`LocaleChanged` lo publica `SettingsService` al arrancar (primera resolución) y cada vez que la persona cambia el idioma. La UI lo drena como cualquier otro evento, cambia su `Locale` activo y pide un repintado; no reinicia nada (ARQ-101).

El engine publica por `crossbeam_channel` (sin límite para eventos de estado, con coalescencia de `JobProgress` a 10 Hz). La UI drena la cola al inicio de cada frame. Un `Waker` inyectado (que llama a `egui::Context::request_repaint`) despierta la UI cuando llega un evento. El dominio no conoce a los observadores.

### 5.3 Otros patrones

| Patrón | Dónde | Por qué |
|---|---|---|
| Builder | `ExportRequest::builder()`, `ProjectBuilder` (en testkit y en plantillas), `EncoderConfig::builder()` | Construcciones con muchos opcionales validadas en `build()`. |
| Facade | `gafira-engine::Gafira` con los servicios `ProjectService`, `EditService`, `SelectionService`, `AssetService`, `IdentityService`, `MediaService`, `ExportService`, `SettingsService`, `LicenseService` | La UI y la CLI usan una API estable y pequeña. Los detalles internos cambian sin romper la UI. |
| Repository | `ProjectRepository` (`.gafira`), `ContentStore` (recursos), `LibraryRepository` (índice de la biblioteca local), `SettingsRepository` | Aísla la persistencia y permite pruebas en memoria (`InMemoryRepository` en testkit). |
| DI por traits | Los servicios reciben `Arc<dyn Port>` en su constructor. El *composition root* está en los binarios, sin frameworks ni contenedores mágicos. | Pruebas con dobles y binarios con distintas combinaciones (CLI sin GPU). |
| Strategy + Registry | Efectos, formatos, códecs, backends, segmentadores, proveedores, herramientas | Extensión sin tocar el núcleo. |
| Visitor | Recorridos de documento (§4.1) | Operaciones nuevas sin modificar los datos. |
| Adapter | Backends de render, códecs del SO, plugins | Encapsula APIs externas tras traits propios. |
| Pipeline | Render (§6), cadena de audio, exportación de video (decodificar → componer → codificar → *mux*) | Etapas con canales acotados que aplican contrapresión. |
| Composite | `Transaction`, grupos de nodos, búsqueda federada | Tratamiento uniforme de uno o muchos. |

### 5.4 Facade de aplicación (firma resumida)

```rust
pub struct Gafira { /* servicios y puertos inyectados */ }
impl Gafira {
    pub fn new(ports: Ports, registries: Registries, cfg: EngineConfig) -> Result<Self, EngineError>;
    pub fn projects(&self) -> &ProjectService;   // new, open, save, save_as, recover, close,
                                                 // save_version, list_versions, restore_version
    pub fn edit(&mut self) -> &mut EditService;  // execute(cmd), undo, redo, begin/end_gesture
    pub fn assets(&self) -> &AssetService;       // import (embebido o vinculado), relocate(folder)
    pub fn export(&self) -> &ExportService;      // enqueue(req) -> JobId, cancel, list
    pub fn media(&self) -> &MediaService;        // play, pause, seek, scrub, proxies
    pub fn license(&self) -> &LicenseService;    // status, activate, validate, deactivate (§13)
    pub fn settings(&self) -> &SettingsService;  // get/set; set_language(pref) → persiste y publica LocaleChanged (§14)
    pub fn events(&self) -> &Receiver<EngineEvent>;
}
```

### 5.5 Antipatrones prohibidos

- Mutar el documento fuera de un `Command`, por ejemplo desde un widget.
- `Arc<Mutex<Document>>` compartido. Se usan instantáneas inmutables más mutación en un único dueño.
- Trabajo pesado (decodificación, render completo, E/S) en el hilo de UI.
- `unwrap`/`expect` en rutas de datos del usuario. `panic` como control de flujo.
- Singletons y registros globales mutables. Un *service locator* accesible desde cualquier parte.
- Una «clase dios» `Editor` con toda la lógica. Herencia simulada con `Deref`.
- *Strings* mágicos para tipos de nodo fuera de `Extension`. Rutas absolutas guardadas en el proyecto.
- Dependencias circulares disimuladas con *callbacks* globales. Features que cambian la semántica del formato de archivo.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-30 | Todo cambio de documento pasa por `History::execute` de `gafira-edit` (envuelto después por `EditService::execute`). | `Document` solo se muta con la capacidad `EditToken`, que solo usa `gafira-edit`; el CI lo vigila (V13). Un test de compilación (`trybuild`) muestra que una mutación desde `gafira-ui` no compila. |
| ARQ-31 | No hay estado global mutable para los registros (§4). | Una búsqueda en CI de `static mut`, `lazy_static!` e `inventory::` en crates no de prueba da 0 coincidencias (excepto la `OnceLock` de configuración de logging). |
| ARQ-32 | Deshacer y rehacer son O(1) para el usuario. | Con un documento de 100 capas, 1 000 deshacer seguidos tardan < 16 ms cada uno en el percentil 99. Esa medición se hace con los *benchmarks* de ARQ-40; mientras tanto, `gafira-edit` prueba la estructura: deshacer y rehacer de una entrada con instantánea no llaman ni a `apply` ni a `revert` (solo a `label()`, para devolver la etiqueta) y solo instalan el `Arc` guardado. |
| ARQ-33 | Un gesto continuo produce una sola entrada de historial. | Prueba: un arrastre de 300 eventos de movimiento deja `history.len()` aumentado en 1. |
| ARQ-34..ARQ-39 | Reservados. | — |

---

## 6. Pipeline de render y rendimiento

### 6.1 Grafo de render

```text
Document snapshot (Arc) ──► RenderGraphBuilder (Visitor) ──► RenderGraph (DAG)
   nodos: Source{Vector|Text|Image|VideoFrame} → Transform → Effect* → Mask → Blend → Composite
        ──► Planner: recorta a viewport/tiles, resuelve LOD (mip), consulta TileCache
        ──► RenderPlan (lista de tareas por tile, ordenada por prioridad)
        ──► RenderBackend.execute(deadline) ──► Overlay (selección, guías) ──► presentación egui
```

- **Claves de caché por contenido**: cada nodo del grafo tiene `key = hash(tipo, parámetros muestreados en t, claves de entrada, LOD)`. Si un parámetro no cambia, la clave tampoco y el resultado se reutiliza. Esto cubre el movimiento de un elemento sin re-rasterizarlo: solo cambia la composición.
- **Tiles**: 256×256 px por nivel de zoom en potencias de 2 (FIJADA). El zoom de 5 % a 3200 % usa el mip más cercano y se refina en segundo plano. El vector se re-rasteriza en el nivel exacto cuando la vista se estabiliza (150 ms sin cambios, FIJADA).
- **Invalidación**: `Command::touches` → `DirtySet` → rectángulos sucios en coordenadas de página → tiles afectados.
- **Overlay** de selección, asas, guías y *snap*: se dibuja con el pintor de egui encima del lienzo y nunca invalida tiles del documento.

### 6.2 Presupuesto de frame y vista previa frente a exportación

- **Presupuesto**: 16 ms por frame de UI. Reparto orientativo (FIJADA): ≤ 4 ms UI egui, ≤ 2 ms construcción y planificación del grafo, ≤ 8 ms render visible, 2 ms de margen.
- Si el render no termina dentro del `Deadline`, se presenta el mejor mip disponible y se continúa en el siguiente frame (refinamiento progresivo). La UI nunca espera.
- `RenderQuality::Preview`: 8 bits sRGB premultiplicado o f16 en GPU, efectos a resolución del viewport, proxies de video, texto con *hinting* de pantalla.
- `RenderQuality::Export`: f32 en luz lineal, resolución completa, medios originales y CPU por defecto (determinista). La GPU es opcional para exportar video si pasa la verificación golden.
- El grafo es el mismo en ambos modos y solo cambian los parámetros. Así se evita que la vista previa y el resultado diverjan.

### 6.3 Concurrencia

| Hilo / pool | Función | Prioridad |
|---|---|---|
| Principal (UI) | egui, `EditService` (mutación vía Arc COW), drenado de eventos | Máxima, nunca bloquea > 1 ms |
| Pool de render (`rayon::ThreadPool` dedicado, n−1 núcleos) | Tiles, efectos CPU, segmentación | Cola con 4 niveles: `Interactive` > `Visible` > `Prefetch` > `Background` |
| Decodificación (1 hilo por flujo activo, máx. 8) | Demux y decodificación hacia la caché de fotogramas | Por proximidad al cabezal |
| Audio (callback de cpal) | Mezcla en tiempo real desde *ring buffers* | Tiempo real del SO |
| E/S | Guardado, autoguardado, importación de recursos, generación de proxies | Normal |
| Exportación | 1 trabajo a la vez por defecto (configurable), usa el pool de render en `Background` | Baja mientras el usuario interactúa |

Reglas:

- **Canales**: `crossbeam_channel` acotados en las etapas del pipeline, para aplicar contrapresión.
- **Cancelación**: `CancelToken` (generación `AtomicU64`). Se comprueba por tile, por fotograma y por iteración de algoritmo. Una nueva interacción invalida la generación previa del viewport.
- **Paralelismo de datos**: `rayon` solo dentro de los motores puros, que reciben el pool por parámetro o en su constructor y no usan el pool global. Los efectos corren en un solo hilo por llamada: el paralelismo por tiles es de `gafira-render`.

### 6.4 Cola de exportación

- Los trabajos `ExportJob` se persisten en `datos_app/export-queue.json` y sobreviven a un fallo: al reabrir, se ofrece reanudarlos.
- Estados: `Queued → Preparing → Rendering(p) → Encoding(p) → Finalizing → Done | Failed(e) | Cancelled`.
- Se escribe a un archivo temporal y se renombra al terminar. Nunca queda un archivo final parcial.
- **Reintentos**: hasta 2 automáticos solo para errores transitorios (E/S temporal, `DeviceLost` → reintento en CPU, códec del sistema no disponible → propuesta de AV1 + Opus con aviso, porque no existe un códec de software de la misma clase legal, D-02). Los errores de validación no se reintentan: se muestran antes de encolar.
- **Notificación**: un aviso en la app con el microcopy de la marca («Tu diseño está listo para compartir»), una notificación del SO opcional y la acción «Mostrar en carpeta».

### 6.5 Video: scrubbing, caché de fotogramas y proxies

- **Al importar**: se indexan los keyframes en segundo plano (≤ 1 s por minuto de material 1080p, FIJADA) y se genera una tira de miniaturas para la timeline.
- **Caché de fotogramas**: LRU por `(asset, índice de fotograma, nivel de proxy)`, con presupuesto de RAM (por defecto 1 GB o el 15 % de la RAM, el menor; FIJADA) y prefetch en la dirección del movimiento del cabezal.
- **Proxies**: se generan automáticamente para material de más de 1080p o códecs costosos, en MJPEG intra a 540p o 720p, en la caché del usuario (fuera del `.gafira`). Al exportar siempre se usa el original.
- **Scrubbing**: búsqueda al keyframe previo y decodificación hacia delante en el hilo de decodificación. Mientras tanto se muestra el fotograma en caché más cercano. Con proxies todo es intra, así que la búsqueda es inmediata.
- **Reproducción**: el reloj maestro es el de audio. El video descarta fotogramas si se retrasa y nunca bloquea el audio.

### 6.6 Hardware de referencia (FIJADA, D-03)

| Perfil | CPU | RAM | GPU / VRAM | Disco | Pantalla |
|---|---|---|---|---|---|
| Gama media (Windows/Linux) | Intel Core i5 de 10.ª generación o AMD Ryzen 5 3600 | 8 GB | GPU integrada | SSD SATA | 1920×1080 |
| GPU (cifras con GPU, medidas aparte) | El mismo equipo de gama media | 8 GB | NVIDIA GTX 1650 de 4 GB | SSD SATA | 1920×1080 |
| macOS de referencia | MacBook Air M1 | 8 GB unificada | Integrada M1 | SSD | Retina |

«Gama media» y «GPU moderna» en las cifras del PRD y de la spec técnica se refieren a estos perfiles. Las columnas de disco y pantalla se mantienen como condición de medida.

Otras cifras (FIJADAS, D-03):

- Instalador ≤ 60 MB por plataforma; binario ≤ 40 MB tras `strip`.
- RAM en reposo con un documento vacío ≤ 300 MB. Con un documento de 20 páginas e imágenes ≤ 1,5 GB.
- Límite de memoria configurable (por defecto el 50 % de la RAM física). Límite de VRAM configurable (por defecto el 70 % de la VRAM reportada).
- Vista previa de video a la fps del proyecto (24/25/30, y 60 en el perfil con GPU).
- Fps de proyecto admitidas: 23,976, 24, 25, 29,97, 30, 50 y 60.

### 6.7 Mapa de cifras → técnica → prueba

| Cifra (fuente) | Técnica que la logra | Prueba que la verifica (falla CI si se supera) |
|---|---|---|
| Selección < 16 ms (spec) | Índice espacial R-tree por página (`rstar`, MIT OR Apache-2.0) y bounds cacheados; selección = cambio de `Selection` sin re-render de tiles | `bench_select_hit` con 1 000 nodos, p99 < 16 ms |
| Arrastre a 60 fps (spec) | Elementos arrastrados como capa compuesta cacheada; solo se recompone; overlay en egui | `bench_drag_100_layers`: 300 frames, p95 ≤ 16,7 ms |
| Cambio de color < 50 ms (spec) | Invalidación por `touches` y re-rasterización solo de los tiles del nodo | `bench_recolor`, p99 < 50 ms |
| Filtro < 300 ms (spec) | Efecto sobre el mip del viewport (vista previa) en GPU o rayon; resolución completa en segundo plano | `bench_filter_4000px_preview` < 300 ms |
| Interacciones locales < 100 ms (spec) | Todo comando del núcleo es O(nodos tocados); trabajo pesado asíncrono | Suite `bench_interactions`: cada acción < 100 ms p99 |
| Muestreo de animación < 1 µs con 10 000 keyframes (ARQ-12, FIJADA) | Búsqueda binaria sobre los `at` ordenados (ARQ-18) y como mucho una evaluación de curva y una interpolación por muestra (ARQ-16) | `bench_anim_sample_10k`: < 1 µs por muestra en la máquina de referencia |
| Aislar sujeto 4000 px < 5 s CPU, < 2 s GPU (spec/PRD) | Segmentación en 2048 px (D-45) y escalado guiado; SLIC y GMM paralelos; variante GPU del *guided filter* | `bench_segment_4000` en la máquina de referencia: CPU < 5 s; GPU < 2 s (VERIFICAR viabilidad de la vía GPU con la técnica clásica, R-04) |
| Arranque < 5 s gama media (PRD) | Inicialización perezosa de la GPU y de los registros pesados; fuentes embebidas sin escanear el sistema al arrancar (escaneo en segundo plano); LTO; la Pantalla de arranque solo espera los pasos bloqueantes y los diferibles siguen tras cerrarla (§8.4) | `gafira --bench-startup` hasta el primer frame interactivo de Inicio < 5 s (objetivo interno < 2 s, FIJADA), incluida la Pantalla de arranque |
| Primer frame de la Pantalla de arranque < 300 ms (PROPUESTA, D-09) | Ventana y primer frame antes de cualquier otro paso; logo pequeño incrustado; decodificación y reescalado de la imagen en un hilo propio (ARQ-65) | `gafira --bench-startup` informa de `splash_first_frame_ms` < 300 ms en gama media |
| Pantalla de arranque visible ≥ 1,2 s (D-09) y aviso de carga lenta a los 5 s (PROPUESTA) | Temporizador de la Pantalla de arranque sobre el reloj inyectado (`Clock`); cierre con `StartupFinished` | `splash_visible_ms` ≥ 1200 y ≤ `max(1200, pasos bloqueantes) + 1 frame`; pruebas con arranque simulado (03 DIS-82) |
| Abrir proyecto de 20 páginas < 3 s (PRD) | Manifiesto y JSON por página; render de la página visible primero; miniaturas guardadas en el archivo; recursos bajo demanda | `bench_open_20_pages` < 3 s hasta la primera página interactiva |
| Abrir documento de 100 capas < 500 ms (spec) | Deserialización serde directa, sin pasos intermedios; textos maquetados de forma perezosa | `bench_open_100_layers` < 500 ms |
| Exportar PNG 1080p < 2 s (spec) | Render CPU en tiles paralelos; codificador PNG con compresión rápida por defecto | `bench_export_png_1080` < 2 s |
| Zoom 5 %–3200 % (PRD) | Mipmaps y re-rasterización vectorial al nivel exacto | Prueba funcional en los límites y golden a 3200 % sin pixelado del vector |
| Video de 60 s con 3 pistas sin bloquear la UI (PRD) | Decodificación en hilos propios; caché de fotogramas; reloj de audio | `bench_playback_3tracks`: frames de UI p99 < 16,7 ms durante la reproducción |
| Scrubbing 1080p fluido (PRD) | Proxies intra, prefetch y fotograma más cercano en caché | `bench_scrub_1080`: ≥ 24 actualizaciones/s; búsqueda en caché < 50 ms, sin caché < 250 ms (FIJADA) |
| Mezclador de 16 pistas, EQ de 3 bandas (PRD) | Cadena `AudioProcessor` sin asignaciones; bloques de 512 muestras | `bench_mix_16`: < 25 % de un núcleo a 48 kHz; prueba de respuesta del EQ |
| Cancelar una segmentación ≤ 100 ms (ARQ-43) | Comprobación del token por unidad de trabajo (ARQ-112) | `bench_cancel_segment`: tramo más largo entre dos comprobaciones del token ≤ 100 ms |
| Sombra < 100 ms con radio ≤ 40 px en 1080p (04 FD-35) | Desenfoque gaussiano separable | `bench_shadow_1080`: < 100 ms |
| Hash perceptual de 12 MP < 100 ms (04 FD-28) | La fija el diseño del change | `bench_dhash_12mp`: < 100 ms |
| Escribir un carácter < 16 ms en cuadros de hasta 5 000 caracteres (04 FD-41) | La fija el diseño del change | `bench_text_type_5000`: p99 < 16 ms |
| Actualizar Curvar < 16 ms con 100 caracteres (04 FD-44) | La fija el diseño del change | `bench_text_curve_100`: p99 < 16 ms |
| Re-maquetar 5 000 caracteres en 4 columnas < 16 ms (04 FD-45) | La fija el diseño del change | `bench_text_columns_5000`: p99 < 16 ms |
| Ajustar al cuadro < 16 ms con 500 caracteres (04 FD-46) | Bisección sobre la rejilla de tamaños | `bench_text_fit_500`: p99 < 16 ms |
| Vista previa del Generador de guías < 16 ms (04 FD-04) | Función pura sin tocar el documento | `bench_guides_generate`: < 16 ms por cambio de valor |
| Márgenes, sangrado y zona de seguridad < 1 ms por página (04 FD-05) | Función pura | `bench_layout_rects`: < 1 ms |
| Imán < 2 ms con 1 000 nodos (04 FD-06) | Búsqueda del destino más cercano por eje | `bench_snap_1000`: p99 < 2 ms |
| Cuadrícula < 1 ms (04 FD-07) | Solo se calculan las líneas del *viewport*; se dibuja como overlay | `bench_grid_lines`: < 1 ms |
| Reel de 60 s 1080p en < 3 min (PRD) | Exportación encadenada en paralelo (composición y codificación en hilos distintos); codificador por hardware del SO o AV1 en preset rápido | `bench_export_reel_60s`: < 180 s (requiere ≥ 10 fps de extremo a extremo a 30 fps) |
| Exportaciones sin intervención ≥ 98 % (PRD) | Validación previa, reintentos transitorios, alternativa de códec, escritura atómica | Suite `export_matrix` (≥ 200 combinaciones formato × opción × documento) con ≥ 98 % de éxito; meta interna 100 % |
| Autoguardado cada 5–15 s, pérdida ≤ 15 s, recuperación 100 % (PRD) | §7.2 | Prueba de *kill* aleatorio (`SIGKILL` / `TerminateProcess`) 500 veces: siempre recuperable y pérdida ≤ intervalo configurado |
| Primera pieza publicable < 10 min (PRD) | Plantillas locales indexadas, exportación con valores por defecto | Prueba de usabilidad (spec 01), no de CI |
| Quejas de lag < 5 % de sesiones (PRD) | Todo lo anterior; registro **local** de frames lentos visible solo para el usuario (no es telemetría: nunca sale del equipo) | Informe local opcional; sin envío automático |
| Límite configurable de memoria y VRAM (PRD) | `MemoryGovernor` en el engine: cachés con `trim(budget)` y evento `MemoryPressure` | Prueba con un límite de 1 GB: el proceso no lo supera más de un 10 % al abrir el documento de 20 páginas |
| Reescritura de texto < 4 s (spec) | Solo con IA local opcional; fuera del MVP | Sin prueba en el MVP |

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-40 | Cada cifra de §6.7 tiene un benchmark con umbral en `perf-budgets.toml`. | El job nocturno en la máquina de referencia falla si una métrica empeora por encima de su umbral o un 10 % respecto a la base. |
| ARQ-41 | El hilo de UI nunca bloquea más de 16 ms por trabajo del engine. | Instrumentación `tracing`: en la suite E2E, 0 spans de más de 16 ms en el hilo principal fuera de diálogos del SO. |
| ARQ-42 | La vista previa y la exportación comparten el grafo. | La prueba golden compara la vista previa al 100 % en CPU con la exportación con diferencia ≤ 1/255 por canal en el 99,9 % de los píxeles. |
| ARQ-43 | La cancelación responde en ≤ 100 ms. | Cancelar una exportación 4K o una segmentación devuelve el control en ≤ 100 ms. En `gafira-segment` no hay reloj, ni en sus *benchmarks* (ARQ-02): `bench_cancel_segment` mide el tramo de trabajo más largo entre dos comprobaciones del token con una imagen de 4000 px, y ese tiempo acota la respuesta por arriba. |
| ARQ-44 | Las cachés respetan el `MemoryGovernor`. | Ver la prueba del límite de memoria en §6.7. |
| ARQ-45..ARQ-49 | Reservados. | — |

---

## 7. Persistencia y robustez

### 7.1 Formato de proyecto `.gafira` (ADR-04)

**ADR-04: un único formato `.gafira`.**
Es un contenedor ZIP con un manifiesto JSON versionado. Sustituye a las propuestas `.designpkg` y `.design/SQLite`.

```text
proyecto.gafira (ZIP)
├─ mimetype                         "application/x-gafira+zip" (primera entrada, STORE, sin comprimir)
├─ manifest.json                    { "format": "gafira.project", "schema": 1, "app": "0.1.0",
│                                     "project_id", "created", "modified", "pages": [...ids, orden],
│                                     "sequences": [...], "assets": { "<blake3>": {mime, bytes, name,
│                                       "linked": {rel_path, bytes, blake3}?} },   (presente solo si el medio está vinculado)
│                                     "versions": [...ids], "fonts_used": [...], "features_required": [] }
├─ document.json                    Project sin páginas ni secuencias (meta, estilos, identidades, tabla de assets)
├─ pages/<page_id>.json             una página por archivo (carga perezosa, guardado incremental)
├─ sequences/<seq_id>.json
├─ assets/<blake3-hex>.<ext>        recursos embebidos, STORE si ya están comprimidos (JPEG/PNG/MP4), DEFLATE si no
│                                   (los medios vinculados NO tienen entrada aquí)
├─ masks/<blake3-hex>.tiles         máscaras pintadas: tiles con zstd (crate BSD-3-Clause; la `libzstd` en C, §9.2)
├─ thumbs/<page_id>.webp            miniaturas de 512 px (apertura rápida y vista del explorador)
├─ versions/index.json              [{ id, name, kind: "named"|"auto", created, app, schema }]
├─ versions/<version_id>/           una versión: document.json, pages/*.json y sequences/*.json de ese momento;
│                                   los recursos se comparten por hash con assets/ y masks/ (no se duplican)
└─ extensions/<type_id>/...         datos opacos de extensiones (se conservan)
```

- **Versiones** (menú Archivo → Guardar versión y Versiones, 03 §2.2). Una **versión nombrada** la crea el usuario y nunca se borra de forma automática. Una **versión automática** se crea al guardar si la última automática tiene más de 30 min; se conservan las 20 más recientes y se descartan las más antiguas (cifras PROPUESTA). Restaurar una versión es un `Command` (`RestoreVersion`) que se puede deshacer. El recolector de recursos no borra un recurso que use alguna versión. Las versiones son distintas de los journals de autoguardado, que viven fuera del archivo (§7.2).
- **Medios vinculados** (Fase 2, con Relocalizar medios, 01 RF-55 y 05 FV-25). Por defecto todos los medios se embeben. Los videos de más de 500 MB (PROPUESTA, de 05) pueden vincularse: el manifiesto guarda `rel_path` relativo a la carpeta del `.gafira`, el tamaño y el hash BLAKE3 completo, nunca una ruta absoluta (ARQ-56). La última ruta absoluta conocida puede guardarse en `datos_app/links/<project_id>.json`, fuera del proyecto, solo como pista. **Relocalizar medios**: el usuario elige una carpeta; `AssetService::relocate` la indexa, busca por nombre, filtra por tamaño y confirma por hash; reconecta todos los coincidentes con un solo `Command` y rechaza con aviso los que tienen el mismo nombre y otro hash. Un vínculo ausente emite `LinksMissing`, se muestra como marcador y la exportación avisa antes de empezar. Proxies y miniaturas de un medio vinculado van a la caché del usuario. El campo `linked` existe desde `schema` 1, para que activar la función en Fase 2 no exija migración.

- **Migraciones por paso**: `schema` es un entero. Las funciones `migrate_n_to_n1(serde_json::Value) -> Result<Value, MigrationError>` se encadenan y cada una tiene su propio golden de entrada y salida.
- Al abrir un archivo de un `schema` futuro: modo solo lectura con aviso y nunca se sobrescribe.
- Los proxies, cachés y el historial **no** se guardan en el archivo.
- Las fuentes no se incrustan por defecto. Si la licencia OFL de una fuente lo permite, «Empaquetar fuentes» las incluye en `assets/` (PROPUESTA).
- *Descartado*:
  - SQLite como formato de proyecto: binario opaco, más difícil de migrar y de inspeccionar, y no aporta frente a JSON por página.
  - Carpeta de proyecto: frágil al copiar o enviar.
  - JSON único: no admite carga perezosa.

### 7.2 Guardado atómico, autoguardado y recuperación

- **Guardado atómico**:
  1. Escribir `nombre.gafira.tmp-<rand>` en el mismo directorio.
  2. `fsync` del archivo.
  3. En Unix, `rename` y `fsync` del directorio. En Windows, `ReplaceFileW` (conserva ACL), con `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` como alternativa.
  4. El archivo anterior se conserva como `.gafira.bak` (1 generación, FIJADA).
- **Guardado incremental**: los recursos ya presentes con el mismo hash se copian del ZIP anterior sin recomprimirlos. Solo se reescriben los JSON de páginas sucias.
- **Autoguardado rotativo**: cada N s con N configurable entre 5 y 15 (por defecto 10, FIJADA), solo si hay cambios. Se escribe en `datos_app/recovery/<project_id>/slot-{0,1,2}.gafira-journal` en rotación. El journal guarda solo JSON (documento y páginas sucias) más referencias a recursos ya volcados en el `ContentStore` local, de modo que cada autoguardado dura menos de 200 ms en el documento de 20 páginas (FIJADA). Se ejecuta en el hilo de E/S sobre una instantánea `Arc`, sin bloquear.
- **Recuperación**: al arrancar, se buscan journals cuyo bloqueo pertenece a un PID que ya no existe y se emite `RecoveryAvailable`. La UI ofrece «Recuperar», «Abrir el último guardado» o «Descartar». Un journal se valida con checksum BLAKE3 y, si está corrupto, se usa el slot anterior.
- **Bloqueos de archivo**: un archivo auxiliar `nombre.gafira.lock` con `{pid, host, usuario, inicio}` más un bloqueo consultivo del SO (`fs4`, MIT OR Apache-2.0). Si el proyecto ya está abierto en otra instancia o equipo, se ofrece abrir en solo lectura o una copia.
- **Unidades de red y almacenamiento en la nube local**: se detectan cuando es posible y se recomienda trabajar sobre una copia local.

### 7.3 Errores y degradación

```rust
#[derive(Debug)]
pub enum FormatError {
    Corrupt(String),
    TooNew { found: u32, supported: u32 },
    Io(std::io::Error),
    Migration(MigrationError),
}
// `Display` (solo para el log), `std::error::Error` y `ToMessage` se escriben a mano (ADR 0012).
```

- Un enum de error por crate, escrito a mano y sin `thiserror` (ADR 0012) (`DocError`, `CommandError`, `FormatError`, `ImportError`, `ExportError`, `MediaError`, `RenderError`, `EffectError`, `SegError`, `PluginError`). Excepción: un tipo que comparten varios crates puede tener su propio enum, como `BufferError` en `gafira-raster` (`ImageView`, `AlphaMask`). El enum del crate lo envuelve en una variante con `From`, y su `to_message` delega, como con `Cancelled`: `EffectError` envuelve `BufferError` y `ParamError`, y añade `size_mismatch`, `format_mismatch` y `unsupported_format`, con las claves `error.raster.<variante>`. Los formatos viajan en `Text` y los tamaños en `Count`. `anyhow` solo en los binarios y en `xtask`.
- `gafira-engine` traduce los errores a `UserError { kind, message: Message, detail, actions: Vec<Message> }`. Cada enum de error implementa `ToMessage` (`gafira-doc::msg`, §14.2) y devuelve una **clave con parámetros** (por ejemplo `error.format.too_new { found, supported }`), nunca un texto ya traducido (ARQ-102). La UI la traduce en el idioma activo; los mensajes visibles siguen el tono de marca («No pudimos completar esta acción. Inténtalo de nuevo.» / «We couldn't complete this action. Try again.») y el detalle técnico va al log local.
- El texto de `Display` es **solo para el log local**, nunca se muestra en la interfaz y no se traduce. A partir de D-08 se escribe en inglés para que los registros sean iguales con cualquier idioma de interfaz (PROPUESTA; el extracto anterior conserva su texto original como ejemplo de estructura).
- **Degradación**:

| Fallo | Comportamiento |
|---|---|
| GPU | Pasa a CPU. |
| Códec de hardware | Usa software o la alternativa libre. |
| Fuente ausente | Sustituta con aviso y la fuente original recordada. |
| Recurso ausente | Marcador visible y acción de relocalizar. |
| Plugin | Se desactiva el plugin y el documento sigue intacto. |
| Memoria | Se recortan cachés y se activan proxies. |

- **Pánicos**: `panic = "unwind"` en release (ADR-05). Los trabajos de fondo se ejecutan bajo `catch_unwind`, de modo que un pánico en un importador o efecto se convierte en un error del trabajo y no en un cierre. Un *hook* de pánico guarda el journal de emergencia y un informe local.

**ADR-05: `panic = "unwind"`.**
*A favor*: aislar fallos de trabajos (robustez > ~3–5 % de tamaño, estimación). *Descartado*: `abort`, que cerraría la app ante el fallo de cualquier importador.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-50 | El guardado es atómico. | La inyección de fallos (kill en cada `write`/`fsync`/`rename` de una traza simulada con `FileSystem` falso) deja siempre el archivo anterior o el nuevo íntegros. |
| ARQ-51 | La pérdida máxima es ≤ intervalo de autoguardado (≤ 15 s). | Prueba de *kill* aleatorio (§6.7). |
| ARQ-52 | La recuperación es del 100 % de los journals válidos. | 500/500 recuperaciones en la prueba de *kill*. |
| ARQ-53 | Las migraciones están encadenadas y probadas. | Cada versión de `schema` tiene un fixture que abre en la versión actual con un golden de salida. |
| ARQ-54 | Un error en un trabajo de fondo no cierra la app. | La prueba con un efecto que provoca `panic` deja la sesión viva y el trabajo marcado como `Failed`. |
| ARQ-55 | Los archivos de esquema futuro no se sobrescriben. | Prueba: abrir con `schema = actual + 1` → solo lectura y «Guardar» deshabilitado. |
| ARQ-56 | El proyecto no guarda rutas absolutas. | Una búsqueda en los JSON de los fixtures (incluidos los de medios vinculados y `versions/`) no encuentra rutas absolutas. Los recursos se referencian por hash y los vínculos por `rel_path`. |
| ARQ-57 | Hay bloqueo multiinstancia. | Una segunda instancia que abre el mismo archivo recibe la opción de solo lectura. |
| ARQ-58 | Las versiones se guardan dentro del `.gafira` sin duplicar recursos y se restauran sin pérdida. | Prueba: crear 5 versiones nombradas de un proyecto con 100 MB de recursos aumenta el archivo en < 1 % de esos 100 MB más el JSON; restaurar cada versión reproduce su documento byte a byte (serialización determinista, ARQ-10); deshacer la restauración devuelve el estado previo. |
| ARQ-59 | Relocalizar medios reconecta por hash y nunca acepta un archivo distinto. | Prueba de 05 FV-25: mover la carpeta de medios y relocalizar reconecta el 100 % de los coincidentes; un archivo con el mismo nombre y otro hash se rechaza con aviso. |

---

## 8. Interfaz de usuario

### 8.1 ADR-06: egui/eframe

**Decisión**: egui/eframe (MIT/Apache-2.0, VERIFICADA según el handoff; versión orientativa 0.36) con backend `wgpu`, el mismo dispositivo que `gafira-render-gpu` (textura del lienzo compartida, sin copias). Si no hay GPU, se usa el renderizador software de eframe o `glow` (VERIFICAR la disponibilidad en la versión elegida).

*Motivos*:

- Las cuatro apps de referencia lo usan con éxito en editores complejos.
- El modo inmediato encaja con el lienzo y la timeline de alta frecuencia.
- Integra AccessKit para lectores de pantalla.
- Licencia permisiva.
- Funciona en las tres plataformas sin webview.

*Descartadas*:

| Alternativa | Motivo |
|---|---|
| Qt (qmetaobject/cxx-qt) | LGPL o comercial; complica la vendibilidad y el empaquetado. |
| GTK4 (gtk-rs) | LGPL y apariencia ajena en Windows y macOS. |
| Slint | Licencias GPLv3, *royalty-free* con condiciones o comercial: no es permisiva sin condiciones. |
| Tauri, Electron o cualquier webview | Obligan a tecnología web, contradicen «sin web» y añaden peso. |
| iced | MIT y viable, pero con menos madurez en lienzos interactivos y accesibilidad en la versión evaluada; queda como plan B. |
| Xilem/Floem | Inmaduros para producción. |

### 8.2 Tokens de marca desde datos

**Contenido normativo**: la estructura y los valores de `gafira.tokens.json` (marca, modos claro y oscuro, tipografía, espacio, radios, tamaños, bordes, sombras, movimiento y capas z) están definidos en **03 §3**, que prevalece. Esta spec no repite el JSON; solo fija el mecanismo:

- **Carga**: `gafira-theme` lee `assets/theme/gafira.tokens.json`, embebido en el binario con `include_bytes!`. Solo en builds de desarrollo se puede sustituir con la variable `GAFIRA_THEME_PATH`; en el perfil `dist` esa variable se ignora.
- **Modelo**: el JSON se deserializa a tipos neutros (`ThemeTokens`, `ModeTokens`, `TypeScale`, `MotionTokens`…) sin dependencia de egui. `gafira-ui` los traduce a `egui::Style`/`Visuals` en `theme.rs`, el único lugar donde se crean colores de egui (ARQ-60).
- **Validación al cargar** (falla el arranque en desarrollo y la prueba de CI; en release se usa el archivo embebido, ya validado):
  1. `schema` conocido; si es más nuevo, error.
  2. Referencias del tipo `{brand.ink}` resueltas y sin ciclos.
  3. Cada token que usa `gafira-ui` existe en los dos modos (`light` y `dark`) o declara su alternativa.
  4. Contraste WCAG calculado con la fórmula de 03 §4 para los pares declarados en 03 §4, con mínimo 4,5:1 (texto) y 3:1 (texto grande y componentes), y rechazo de los pares prohibidos (texto blanco pequeño sobre coral, 2,80:1).
  5. `motion.reduced` presente con duraciones 0.
- **Modo**: Claro, Oscuro o Sistema (Ajustes → Apariencia); el cambio de modo no reinicia la app.
- **Tipografías**: Inter y Manrope (OFL-1.1) embebidas como bytes con su `OFL.txt` incluido en la distribución y en «Acerca de → Licencias». **Cobertura de los dos idiomas (D-08)**: ambas familias incluyen los subconjuntos *latin* y *latin-ext*, que contienen todos los caracteres del español y del inglés (á é í ó ú ü ñ y sus mayúsculas, ¿ ¡, « », comillas tipográficas “ ” ‘ ’, raya —, guion –, puntos suspensivos …, ×, °, %). Se considera confirmado según el juego de caracteres publicado de cada familia y se **comprueba en CI** sobre los archivos embebidos (ARQ-105), porque un subconjunto mal generado podría perder glifos. Se **desactivan las fuentes por defecto de egui/epaint** (feature `default_fonts`), porque pueden incluir fuentes con Ubuntu Font Licence (VERIFICAR) y no forman parte de la marca. Para símbolos y escrituras no latinas se usan las fuentes del sistema como alternativa por `gafira-platform` (sin redistribuirlas).
- **Logo e iconos de la app**: se generan en build con `xtask` a partir de los SVG de `docs/manuals/marca/assets`. El wordmark es vectorial y nunca se reescribe con una fuente. Se producen `.ico` multirresolución (16–256), `.icns` (16–1024) y PNG para Linux (16–512 en *hicolor*). En la UI el símbolo mide ≥ 24×24 px.
- **Iconografía de interfaz**: propia, de trazo simple con grosor uniforme. Si se parte de Lucide (ISC) o Feather (MIT), se incluye su licencia.

### 8.3 Accesibilidad y movimiento

- **Teclado completo**: orden de foco por panel, `Tab`/`Shift+Tab`, atajos registrados en el `CommandPalette` (Ctrl/Cmd+K) y foco visible con el token `focus.ring` (spec 03). Mover y redimensionar elementos con flechas y modificadores genera `Command`s fusionables.
- **AccessKit**: nombres y roles en todos los widgets. El lienzo expone un árbol accesible de nodos (nombre, tipo, posición). Los nombres accesibles salen del mismo `Translator` que el texto visible, y el nodo raíz declara el idioma activo (`es` o `en`) para que el lector de pantalla use la voz correcta; las opciones «Español» y «English» del selector de idioma declaran el suyo propio (03 §16). Que la integración de egui con AccessKit transmita la propiedad de idioma es **VERIFICAR** en la versión elegida.
- **Reducción de movimiento**: `gafira-platform::prefers_reduced_motion()` lee la preferencia del SO (Windows `SPI_GETCLIENTAREAANIMATION`; macOS `accessibilityDisplayShouldReduceMotion`; Linux `org.gnome.desktop.interface enable-animations` vía zbus, con alternativa en la configuración propia). Si está activa, las transiciones de la UI pasan a 0 ms. Las animaciones del *contenido* del usuario no se ven afectadas.
- El color nunca es la única señal: los estados llevan icono o texto.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-60 | Ningún color literal en `gafira-ui`. | Un lint de CI (búsqueda de `Color32::from_rgb` fuera de `theme.rs`) da 0 coincidencias. |
| ARQ-61 | Los tokens se validan con contraste AA para el texto. | La prueba de `gafira-theme` falla si un par texto/fondo declarado tiene < 4,5:1 (< 3:1 en texto grande). |
| ARQ-62 | Las fuentes de egui por defecto no se incluyen. | `cargo tree -e features -p gafira` no muestra `default_fonts`. Una inspección del binario no encuentra los nombres de esas fuentes. |
| ARQ-63 | Todo el flujo Elegir → Personalizar → Compartir es operable solo con teclado. | La prueba E2E de teclado (egui_kittest, licencia sin medir: VERIFICAR) completa plantilla → editar texto → exportar PNG. |
| ARQ-64 | Se respeta la reducción de movimiento. | Con la preferencia activa, la duración medida de las transiciones de la UI es 0. |

### 8.4 Pantalla de arranque (D-09)

El diseño visible está en 03 §7.10. Aquí se fija dónde vive, cómo se incrustan y decodifican sus recursos y cómo informa del progreso.

- **Dónde vive**: módulo `splash` de `gafira-ui`, dibujado con egui como cualquier otra vista. `apps/gafira` solo lo compone: crea el canal de eventos, abre la Pantalla de arranque y lanza la secuencia de arranque en otro hilo. `gafira-ui` no lee el disco para ella: todos sus recursos están en el binario.
- **Ventana**: una ventana propia sin decoraciones (*viewport* de egui de 800×450 px lógicos, 03 §7.10.2) y la ventana principal se crea oculta y se muestra al cerrar la Pantalla de arranque. Que eframe 0.36 permita crear la ventana raíz oculta y una ventana secundaria sin decoraciones, sin activarla, en las tres plataformas es **VERIFICAR** (en Wayland la posición la decide el compositor). Respaldo: dibujar la Pantalla de arranque dentro de la ventana principal, a su tamaño, con las reglas de recorte de 03 §7.10.3.
- **Recursos incrustados** (ADR-10): un `build.rs` de `gafira-ui` busca `assets/splash_screen.jpg` y `assets/splash_screen.png` en la raíz del workspace, exige que exista exactamente uno, lo valida (cabecera, dimensiones, proporción, tamaño y decodificación completa) y lo copia a `OUT_DIR` con su formato anotado; el crate lo incluye con `include_bytes!` desde `OUT_DIR`. El `build.rs` declara las dos rutas candidatas con `cargo:rerun-if-changed`, así que sustituir el archivo basta para que la siguiente compilación lo recoja. El logo `gafira-logo-primary.svg` se rasteriza en build con `xtask gen-brand` (como los demás logos, §8.2) a 200 px de ancho visible en 1×, 1,5×, 2×, 2,5× y 3× (PNG) y se incrusta igual; a la vista se dibuja la versión más cercana igual o mayor que el factor de escala del monitor.
- **Decodificación sin bloquear el hilo principal**: el hilo de la imagen arranca al empezar `main`, antes de crear la ventana; decodifica el JPEG o el PNG y lo reescala a los px físicos exactos de su zona (filtro Catmull-Rom o Lanczos3 de `image::imageops`). El primer frame se pinta sin esperarlo: si la imagen aún no está, su zona se pinta con `splash.panel` y la imagen se sube como textura en el frame en que llega. El logo (PNG de pocos KB) se decodifica en el hilo principal antes del primer frame (< 5 ms medido en CI; si supera 16 ms, ARQ-41 falla). Si la imagen no decodifica en ejecución (no debería ocurrir tras ARQ-69), la Pantalla de arranque se muestra sin imagen y se registra el error; el arranque no falla por ello.
- **Secuencia de arranque y pasos**: `StartupStep` enumera los pasos con un peso fijo; `fraction` es la suma de pesos completados más la fracción del paso en curso, sobre el total, y nunca decrece.

| Paso (`StartupStep`) | Tipo | Peso (PROPUESTA) |
|---|---|---|
| `Settings`: leer ajustes, preferencia de idioma y de movimiento | Bloqueante, antes del primer frame (la Pantalla de arranque los necesita) | 5 |
| `Locale`: resolver el idioma (D-08) y cargar el catálogo | Bloqueante, antes del primer frame | 5 |
| `Theme`: tokens embebidos y fuentes Inter y Manrope | Bloqueante, antes del primer frame | 10 |
| `License`: leer y verificar el token guardado, sin red (§13) | Bloqueante | 5 |
| `Registries`: formatos, efectos, herramientas, códecs | Bloqueante | 25 |
| `Engine`: crear la Facade y los servicios | Bloqueante | 20 |
| `Recovery`: buscar trabajo sin guardar (§7.2) | Bloqueante | 10 |
| `MainWindow`: primer frame de Inicio preparado | Bloqueante | 20 |
| `Templates`, `RecentThumbnails`, `SystemFonts`, `GpuCanvas` | Diferible: corre tras cerrar la Pantalla de arranque | 0 (no cuenta en la barra) |

- **Reloj y tiempos**: los tiempos de 03 §7.10.5 (mínimo 1,2 s, carga lenta a los 5 s, error tras 20 s sin progreso) se miden con el puerto `Clock`, para probarlos con un reloj simulado. Son constantes de `gafira-ui` (`SplashTimings`); solo el mínimo de 1,2 s es decisión del titular (D-09).
- **Presupuesto de arranque**: primer frame de la Pantalla de arranque < 300 ms (PROPUESTA) y primer frame interactivo de Inicio < 5 s (FIJADA; objetivo interno < 2 s), contando los 1,2 s mínimos. Riesgo: la creación del dispositivo `wgpu` de eframe ocurre antes del primer frame y puede tardar más de 300 ms en algunos equipos (R-19).
- **Presupuesto de tamaño**: el JPEG actual ocupa 296 847 bytes (0,28 MiB), el 0,74 % de los 40 MB del binario (FIJADA, D-03); los PNG del logo suman unas decenas de KB (estimación; se mide con `cargo bloat`). **No hace falta recomprimirlo**: ahorrar 100–150 KB bajando la calidad es un 0,3 % del presupuesto y degradaría las texturas finas de la ilustración, y un JPEG ya comprimido no se reduce más dentro del instalador. Memoria: la decodificación usa 6,0 MiB temporales (1672×941×4 bytes) más la textura a tamaño físico (1,4 MiB al 100 %, 5,5 MiB al 200 %); todo se libera al cerrar la Pantalla de arranque y no cuenta para los ≤ 300 MB en reposo.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-65 | **La Pantalla de arranque vive en `gafira-ui` y no bloquea el hilo principal.** Se dibuja con egui; la ventana y su primer frame van antes de cualquier paso diferible; la decodificación y el reescalado de la imagen corren en un hilo propio; la carga sigue en segundo plano mientras se muestra. | `gafira --bench-startup` informa `splash_first_frame_ms` < 300 ms (PROPUESTA) en gama media; `tracing` no registra spans > 16 ms en el hilo principal durante el arranque (ARQ-41); una búsqueda en CI encuentra el `include_bytes!` del recurso solo en `gafira-ui`; con la ventana secundaria deshabilitada por configuración de prueba, el respaldo dentro de la ventana principal pasa las capturas de 03 DIS-81. |
| ARQ-66 | **Decodificación con crates permisivos ya admitidos.** La imagen se decodifica con `image` (MIT/Apache-2.0, VERIFICADA) con solo las features `jpeg` y `png`; el JPEG lo decodifica `zune-jpeg` (MIT/Apache-2.0/Zlib, VERIFICADA), que `image` usa por debajo en sus versiones recientes (VERIFICAR la versión elegida y que decodifica JPEG progresivo, como el actual). Es la misma versión de `image` que `gafira-io-image`. | `cargo deny check` en verde; `cargo tree -d` no muestra dos versiones de `image` ni de `zune-jpeg`; una prueba decodifica el JPEG progresivo actual y un PNG de prueba. |
| ARQ-67 | **Recursos incrustados en el binario por la build** (ADR-10). Exactamente uno de `assets/splash_screen.jpg` o `assets/splash_screen.png`; el `build.rs` falla si no hay ninguno, si hay los dos, si la cabecera o la decodificación fallan o si se incumplen los límites de 03 §7.10.10; sustituir el archivo se recoge en la siguiente compilación. | Prueba de build en CI con cinco *fixtures* (JPEG válido, PNG válido, ausente, duplicado, truncado): solo los válidos compilan; en release, `strings` o una búsqueda de la firma JPEG/PNG en el binario encuentran el recurso y ningún acceso a `assets/` en tiempo de ejecución (prueba con `assets/` renombrada). |
| ARQ-68 | **Evento `StartupProgress` real.** La barra se alimenta solo de `StartupProgress` y `StartupFinished` (§5.2); `fraction` es monótona en [0, 1] y llega a 1 antes de `StartupFinished(Ok)`; los pasos diferibles no retrasan el cierre. | Prueba de la secuencia de arranque con un `Clock` simulado y puertos falsos: las fracciones publicadas no decrecen, terminan en 1,0 y cubren todos los pasos bloqueantes; un paso diferible bloqueado 10 s no retrasa `StartupFinished`; un fallo en un paso bloqueante publica `StartupFinished(Err)` con un `Message` sin rutas ni códigos (03 DIS-41). |
| ARQ-69 | **Prueba del recurso y presupuesto de tamaño.** Una prueba de `gafira-ui` decodifica el recurso incrustado y comprueba dimensiones (de 1600×900 a 2880×1620 px), proporción (1,5–2,1:1) y tamaño (≤ 2 MB, PROPUESTA); el binario sigue ≤ 40 MB. | `cargo test -p gafira-ui splash_asset` en verde; falla con un recurso corrupto o fuera de límites; `xtask size` (ARQ-83) atribuye a la Pantalla de arranque ≤ 2,2 MB incluido el logo y el binario `dist` no supera 40 MB. |

**ADR-10: recursos de la Pantalla de arranque incrustados por la build.**

**Decisión**: un `build.rs` en `gafira-ui` localiza, valida y copia a `OUT_DIR` la imagen `assets/splash_screen.{jpg,png}`, y el crate la incluye con `include_bytes!`. El mismo mecanismo `include_bytes!` ya se usa para los tokens (§8.2).

*Motivos*: D-09 exige que la imagen vaya en el binario y que se cambie sustituyendo el archivo, sin tocar código. Un `include_bytes!` con ruta fija no admitiría las dos extensiones ni podría validar el archivo antes de compilar; el `build.rs` hace las dos cosas, falla con un mensaje claro y avisa a Cargo de que vuelva a compilar si el archivo cambia. No añade dependencias en tiempo de ejecución; en build usa la misma `image` (como `build-dependency`).

*Descartadas*:

| Alternativa | Motivo |
|---|---|
| Leer la imagen de disco junto al ejecutable | Depende de la ruta de instalación y del paquete de cada plataforma; D-09 lo prohíbe. |
| Crate de incrustación de directorios (p. ej. `include_dir`) | Dependencia nueva para un solo archivo; tampoco valida la imagen. |
| Guardar la imagen ya decodificada (RGBA) | 6,0 MiB frente a 0,28 MiB en el binario; no compensa los pocos ms de decodificación, que además van en otro hilo. |
| Generar código Rust con `xtask` | Paso manual adicional al cambiar la imagen; contradice «sustituir y recompilar». |

---

## 9. Política de dependencias y licencias

### 9.1 Reglas

- Licencias permitidas (allowlist de `cargo-deny`): `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-3.0`, `Unicode-DFS-2016`, `BSL-1.0`, `CC0-1.0`, `MIT-0`, `0BSD`. `OFL-1.1` se permite solo para fuentes y nunca para código. **Excepción por crate**: `assert_no_alloc` (BSD-1-Clause, D-37), con una entrada `[[licenses.exceptions]]` y su motivo en `deny.toml`; BSD-1-Clause no entra en la lista general.
- Prohibidas: `GPL-*`, `LGPL-*`, `AGPL-*`, `SSPL`, `CC-BY-NC*`, `BUSL`, cualquier licencia no comercial y licencias desconocidas.
- `MPL-2.0`: **denegada por defecto**. Una excepción requiere un ADR, aislamiento en un crate opcional y la publicación del código fuente de los archivos MPL modificados. Se evitan `symphonia`, `imbl`, `im` y `rpds` (alternativas en la tabla).
- Sin dependencias git: nada de `storytold/craft-libs` ni forks sin publicar. Solo crates.io.
- Sin binarios externos en el instalador: ni FFmpeg, ni `ffmpeg-sidecar`, ni ejecutables de terceros. Las librerías C con licencia permisiva compiladas estáticamente (por ejemplo libdav1d, BSD-2) se aceptan con VERIFICAR.
- **Bibliotecas del sistema con licencia LGPL**: solo se aceptan si son parte del SO del usuario, se enlazan de forma **dinámica** y **nunca se empaquetan** en el instalador. Caso conocido: `cpal` en Linux enlaza `libasound` (ALSA, LGPL-2.1) del sistema. Es un riesgo **VERIFICAR** con asesoría legal (R-06). Mitigación propuesta: enlace dinámico a la `libasound.so.2` del sistema (dependencia del paquete `.deb`/`.rpm`, no un archivo incluido), `xtask audit-bundle` falla si una `.so` LGPL aparece en el paquete y, como alternativa si el dictamen lo exige, un backend de audio propio por PipeWire/PulseAudio.
- Toda dependencia nueva que llega a un binario pasa `cargo deny check licenses bans sources advisories` en CI. `cargo-deny` no comprueba la licencia de las dependencias solo de desarrollo de crates privados, como `png` en `gafira-testkit`, `proptest`, `criterion`, `trybuild` y `assert_no_alloc`: su licencia se comprueba a mano al añadirlas y se anota en §9.2. Toda dependencia se registra en `THIRD_PARTY_NOTICES.md` (generado con `cargo about`, licencia de la herramienta sin medir: VERIFICAR, que solo se usa en build). Es el **único nombre** del archivo de avisos de terceros en todas las specs y en el instalador.

```toml
# deny.toml (extracto)
[licenses]
allow = ["MIT","Apache-2.0","Apache-2.0 WITH LLVM-exception","BSD-2-Clause","BSD-3-Clause",
         "ISC","Zlib","Unicode-3.0","Unicode-DFS-2016","BSL-1.0","CC0-1.0","MIT-0","0BSD"]
confidence-threshold = 0.93
[bans]
multiple-versions = "warn"
deny = [ { name = "ffmpeg-sidecar" }, { name = "ffmpeg-next" }, { name = "ffmpeg-sys-next" },
         { name = "symphonia" }, { name = "imbl" }, { name = "im" }, { name = "rpds" },
         { name = "gstreamer" }, { name = "openssl-sys" },
         # D-02 y D-07: encoders con patentes o LGPL (nombres de crate VERIFICAR al implementar)
         { name = "openh264" }, { name = "x264" }, { name = "mp3lame-encoder" },
         # D-08: GNU gettext (LGPL/GPL) y su envoltorio no se usan para el idioma (ADR-09)
         { name = "gettext-rs" }, { name = "gettext-sys" } ]
[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-git = []
```

### 9.2 Tabla de dependencias propuestas

Las versiones son orientativas: se fijan en `Cargo.lock` en el momento de implementar. «VERIFICADA» significa verificada localmente según el handoff; «VERIFICADA (`cargo-deny`)» significa medida con `cargo-deny` 0.20.2 y el `deny.toml` del repositorio (proyectos de prueba, 24 de octubre de 2026): 72 de los 76 crates marcados antes VERIFICAR pasan con sus features por defecto; `zip`, `ureq`, `directories` y `ort` no, y se corrigen en la tabla con features o con otro crate. Quedan VERIFICAR las licencias de las bibliotecas en C, que `cargo-deny` no lee.

| Crate | Versión orient. | Licencia | Estado | Uso | Alternativa permisiva |
|---|---|---|---|---|---|
| kurbo | última | MIT/Apache-2.0 | VERIFICADA | Geometría y Bézier | lyon_geom |
| tiny-skia | última | BSD-3-Clause | VERIFICADA | Rasterización CPU | vello_cpu |
| usvg / resvg | última | Apache-2.0/MIT | VERIFICADA | Importar SVG, iconos en build | — |
| moxcms | última | BSD-3/Apache-2.0 | VERIFICADA | ICC y CMYK | lcms2 (MIT, C) |
| image, png, gif, tiff, image-webp | última | MIT/Apache-2.0 | VERIFICADA | Códecs raster; también decodificar la imagen de la Pantalla de arranque en `gafira-ui` (solo features `jpeg` y `png`) y validarla en su `build.rs` (§8.4, ADR-10) | — |
| zune-jpeg | última | MIT/Apache/Zlib | VERIFICADA | Decodificar JPEG | jpeg-decoder |
| fax, weezl | última | MIT/Apache | VERIFICADA | TIFF y LZW | — |
| egui, eframe | 0.36 | MIT/Apache-2.0 | VERIFICADA | UI | iced |
| wgpu, winit | alineada con egui-wgpu | MIT/Apache-2.0 | VERIFICADA | GPU y ventanas | — |
| rfd, arboard | última | MIT (/Apache) | VERIFICADA | Diálogos y portapapeles | — |
| libloading | última | ISC | VERIFICADA | Cargar VA-API y códecs del SO | — |
| zbus | última | MIT | VERIFICADA | D-Bus (preferencias y notificaciones en Linux) | — |
| icu_segmenter | última | Unicode-3.0 | VERIFICADA | Saltos de línea y palabra | unicode-linebreak |
| icu_locale_core, icu_plurals, icu_decimal, icu_datetime (ICU4X 2.x, con `icu_provider` y datos compilados solo para `es` y `en`) | 2.x | Unicode-3.0 (también en sus dependencias transitivas, p. ej. `zerovec`, `yoke`, `writeable`, `tinystr`) | VERIFICADA (`cargo-deny`) | `gafira-i18n`: análisis de etiquetas de idioma, categorías de plural CLDR, formato de números, fechas y tiempos relativos (ADR-09) | Formateadores propios para `es` y `en` |
| toml | última | MIT/Apache-2.0 | VERIFICADA (`cargo-deny`) | **Solo en build** (`xtask gen-i18n`): lee `assets/i18n/*.toml` y genera las tablas embebidas; no entra en el binario | serde_json (catálogos JSON) |
| fluent-bundle + fluent-syntax + intl_pluralrules + unic-langid | última | Apache-2.0/MIT; la dependencia `self_cell` declara `Apache-2.0 OR GPL-2.0-only` y se usa bajo Apache-2.0 | VERIFICADA (`cargo-deny`) | **No se usa**: alternativa de ADR-09 si la gramática de idiomas futuros lo exige | — |
| sys-locale | última | MIT/Apache-2.0 | VERIFICADA (`cargo-deny`) | **No se usa**: alternativa a la detección propia de `gafira-platform` (§14.3) | `windows` / `objc2` / variables de entorno |
| vello | compatible con wgpu | Apache-2.0/MIT | VERIFICADA (`cargo-deny`) | Vectores en GPU | rasterizar en CPU y subir |
| vello_cpu | última | Apache-2.0/MIT | VERIFICADA (`cargo-deny`) | Alternativa CPU | tiny-skia |
| parley + skrifa + harfrust | última | Apache-2.0/MIT | VERIFICADA (`cargo-deny`). `parley` con `default-features = false, features = ["std"]` | Maquetación y shaping de texto | cosmic-text (MIT/Apache) |
| linesweeper | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Booleanas de rutas | implementación propia |
| krilla | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Exportar PDF | pdf-writer (MIT/Apache) |
| hayro | última | Apache-2.0/MIT | VERIFICADA (`cargo-deny`) | Importar PDF (Fase 2) | — |
| serde, serde_json | 1.x | MIT/Apache | VERIFICADA (`cargo-deny`) | Serialización | — |
| zip | última | MIT | VERIFICADA (`cargo-deny`). Con `default-features = false, features = ["deflate"]`: las features por defecto traen `libbz2-rs-sys` (`bzip2-1.0.6`) | Contenedor `.gafira` | rawzip / propio |
| miniz_oxide / flate2 | última | MIT/Apache/Zlib | VERIFICADA (`cargo-deny`) | DEFLATE | — |
| zstd | última | BSD-3-Clause (crate `zstd`); la `libzstd` en C que compila `zstd-sys` no la lee `cargo-deny`: VERIFICAR | VERIFICAR (solo la parte C) | Tiles de máscara | lz4_flex (MIT) |
| blake3 | 1.x | CC0/Apache-2.0 | VERIFICADA (`cargo-deny`) | Hash de contenido | — |
| thiserror | 2.x | MIT/Apache | EVITAR | Ningún crate propio la usa: los errores se escriben a mano (ADR 0012) | — |
| tracing, tracing-subscriber | última | MIT | VERIFICADA (`cargo-deny`) | Logs y spans locales | log |
| crossbeam-channel | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Canales | std::sync::mpsc |
| rayon | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Paralelismo de datos | — |
| rtrb | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Ring buffer de audio | propio |
| rstar | última | MIT OR Apache-2.0 | VERIFICADA (`cargo-deny`) | Índice espacial | propio |
| cpal | última | Apache-2.0 | VERIFICADA (`cargo-deny`) para los crates. **Riesgo**: en Linux enlaza `libasound` (LGPL-2.1) del sistema; solo enlace dinámico, nunca empaquetada (§9.1, R-06, ARQ-79) | Salida de audio | backend PipeWire/PulseAudio propio |
| rav1e | última | BSD-2-Clause | VERIFICADA (`cargo-deny`) | Codificar AV1 | — |
| rav1d / dav1d | última | BSD-2-Clause | VERIFICADA (`cargo-deny`) para el crate Rust; `dav1d` espera una `libdav1d` del sistema y su licencia en C no la lee `cargo-deny`: VERIFICAR | Decodificar AV1 | — |
| Opus (crate a elegir sobre libopus) | — | BSD-3 (libopus) | VERIFICAR: el crate compila C y `cargo-deny` no lee la licencia de `libopus` | Codificar y decodificar Opus | — |
| claxon / flacenc | última | Apache-2.0 | VERIFICADA (`cargo-deny`) | FLAC | — |
| lewton | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Decodificar Vorbis | — |
| hound | última | Apache-2.0 | VERIFICADA (`cargo-deny`) | WAV | propio |
| Decodificador MP3: `nanomp3` (elegido); `minimp3` (respaldo) | última | `nanomp3`: MIT OR Apache-2.0, Rust puro, `forbid(unsafe_code)`. `minimp3`: MIT, C incluido en el crate (CC0) | VERIFICADA (`cargo-deny`) para los crates. Patentes de MP3: VERIFICAR (no comprobadas) | **Solo** importar MP3 (patentes centrales caducadas: UE 2012, EE. UU. abril de 2017, D-07). Sin codificador MP3 | códec del SO |
| Escritor XML (p. ej. `quick-xml`) | última | MIT | VERIFICADA (`cargo-deny`) | `gafira-io-pptx` (Fase 2) | plantillas de texto propias |
| ed25519-dalek | 2.x/3.x | BSD-3-Clause | VERIFICADA (`cargo-deny`) | Verificar el token de licencia | — |
| base64, serde_json, uuid, getrandom | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Token, identificador y secreto del dispositivo | — |
| Cliente HTTP con TLS: `ureq` con `default-features = false, features = ["rustls-no-provider"]` (alternativa: `reqwest` con `rustls`) | última | MIT/Apache; `ring` Apache-2.0 AND ISC. Las features por defecto o `rustls` traen `webpki-roots` (CDLA-Permissive-2.0, fuera de la allowlist): usar las raíces del SO | VERIFICADA (`cargo-deny`) con esas features | Solo `gafira-license` con la feature `online` | `reqwest` |
| keyring | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Secreto del dispositivo en el llavero del SO (feature `credentials`) | archivo cifrado propio |
| windows / objc2 | última | MIT/Apache | VERIFICADA (`cargo-deny`) | FFI de Media Foundation y VideoToolbox | — |
| fs4 | última | MIT/Apache | VERIFICADA (`cargo-deny`) | Bloqueos de archivo | fd-lock |
| etcetera (en lugar de `directories`) | última | MIT OR Apache-2.0 | VERIFICADA (`cargo-deny`). `directories` y `dirs` traen `option-ext` (MPL-2.0); alternativa `app_dirs2` | Rutas de datos | — |
| rusqlite (bundled) | última | MIT (SQLite de dominio público) | VERIFICADA (`cargo-deny`) para el crate. Compila SQLite en C; `cargo-deny` no lee esa licencia: VERIFICAR | Índice de biblioteca local | redb (MIT/Apache) |
| wasmi | última | MIT/Apache-2.0 | VERIFICADA (`cargo-deny`) | Plugins (Fase 3) | — |
| ort + ONNX Runtime | — | MIT/Apache (runtime MIT) | VERIFICADA (`cargo-deny`) solo con `default-features = false, features = ["load-dynamic","std"]` (las features por defecto traen `openssl-sys`, prohibido, y descargan binarios nativos). `ort` es una versión candidata (*release candidate*) y la biblioteca ONNX Runtime se entrega aparte (modelo opcional, fuera del instalador base) | Segmentador ML opcional (Fase 3, paquete aparte) | candle (VERIFICAR) |
| proptest, criterion, insta | dev | MIT/Apache | VERIFICADA (`cargo-deny`) | Pruebas | — |
| trybuild | dev | MIT/Apache | VERIFICADA (`cargo-deny`) | Pruebas de compilación de ARQ-30 en `gafira-edit` | — |
| assert_no_alloc | 1.1.2 | BSD-1-Clause | VERIFICADA a mano, con excepción por crate (§9.1, D-37). Solo `dev-dependency`; `cargo-deny` no la comprueba | Guardia de asignación (ARQ-23) | — |
| png | 0.18 | MIT/Apache-2.0 | Pasa `cargo deny` (spike E5); ver la nota de dependencias de desarrollo de §9.1 | Golden de píxeles, solo en `gafira-testkit` (ADR 0010) | — |
| Fuentes de prueba: subconjuntos de Inter, Manrope, Noto Sans Arabic y Noto Emoji | — | OFL-1.1, sin nombre de fuente reservado | VERIFICADA (spike E4 de `layout-core`). Se registran en `assets/LICENSES.toml` (ARQ-115) | Pruebas de `gafira-text` | — |
| half | — | MIT OR Apache-2.0 | Diferido: llega con `gafira-render-gpu`, porque ARQ-117 deja `f16` fuera | Búferes `f16` | — |
| Remuestreador | — | Propio | Sinc con ventana de Kaiser, sin tercero (spike E3) | ARQ-128 | — |
| **Evitadas** | | | | | |
| symphonia | — | MPL-2.0 | EVITAR | — | claxon, lewton, hound, códecs del SO |
| im / imbl / rpds | — | MPL-2.0 | EVITAR | — | `Arc` + `make_mut` propio (ADR-03) |
| ffmpeg-sidecar / ffmpeg-next | — | Exige FFmpeg (GPL/LGPL) | PROHIBIDA | — | `gafira-mux` + `gafira-codec-*` |
| LAME y cualquier codificador MP3 | — | LGPL | PROHIBIDA | Gafira no exporta MP3 (D-07) | Opus, FLAC, WAV |
| GNU gettext, `gettext-rs`, `gettext-sys` y las herramientas `msgfmt`/`xgettext` | — | LGPL (biblioteca) y GPL (herramientas) | PROHIBIDA | No se usan ni en build ni en el binario (D-08, ADR-09) | Catálogos TOML propios + ICU4X |

Notas de la medición de licencias (`cargo-deny` solo lee los metadatos de los crates, no las licencias de las bibliotecas en C que estos compilan):

- `zstd` (`zstd-sys`), Opus y `rusqlite` con `bundled` compilan código C; sus licencias (BSD-3-Clause, BSD-3-Clause y dominio público) siguen en VERIFICAR hasta leerlas en las fuentes.
- `dav1d` espera una `libdav1d` del sistema. Eso choca con «sin binarios externos» (§9.1) salvo que se compile de forma estática.
- `cpal` en Linux enlaza `libasound` (LGPL-2.1) de forma dinámica (ya recogido en R-06 y ARQ-79).
- `ort` es una versión candidata (*release candidate*) y se usa con `load-dynamic`: la biblioteca ONNX Runtime se entrega aparte con el modelo opcional, coherente con «no entra en el instalador base».
- No se añade CDLA-Permissive-2.0 a la allowlist: se mantiene la regla de licencias solo permisivas.
| rust-i18n | — | MIT (VERIFICAR) | EVITAR | Fija el idioma en estado global (`set_locale`), contrario a ARQ-31 | `gafira-i18n` |
| Encoders H.264/HEVC/AAC propios o de terceros (p. ej. openh264, x264, x265, fdk-aac) | — | Patentes y, en varios casos, GPL | PROHIBIDA | H.264/AAC solo vía SO (D-02) | AV1 + Opus |

### 9.3 Patentes de códecs (ADR-07)

**ADR-07: estrategia de códecs por clase legal (actualizada con D-02 y D-07).**

- **Exportación por defecto**: WebM y MP4 con **AV1 + Opus**, libres de regalías (`gafira-codec-free`, siempre incluido). AV1 es libre de regalías para las patentes de los miembros de AOMedia. Existe un pool de Sisvel orientado a dispositivos de consumo y una demanda de Dolby contra Snap (marzo de 2026): el riesgo es bajo pero no nulo (VERIFICAR con asesoría legal).
- **Libres de regalías** (`gafira-codec-free`): AV1, VP9 (Fase 2), Opus, FLAC, Vorbis (decodificación), PCM y MJPEG. Contenedores WebM/MKV, MP4 (ISO-BMFF admite AV1 y Opus; la compatibilidad con plataformas de publicación es VERIFICAR, R-11), Ogg y WAV.
- **MP3**: **solo importación**, con un decodificador de licencia permisiva (`nanomp3`, §9.2). Sus patentes centrales caducaron (UE en 2012; EE. UU. en abril de 2017). **Sin salida MP3**: el codificador habitual (LAME) es LGPL.
- **AVI**: **solo importación en Fase 2** y solo con códecs libres (MJPEG, sin comprimir, PCM). Sin exportación.
- **H.264/AAC (MP4)**: **solo mediante las API del sistema** (`gafira-codec-os`, feature `os-codecs`): Media Foundation en Windows, VideoToolbox/AudioToolbox en macOS y VA-API en Linux cuando el equipo la tenga. Gafira no incluye ningún encoder ni decoder propio de H.264, HEVC o AAC. Motivo (fuentes públicas): el pool de H.264 cubre productos que incorporan encoders o decoders, y una app que llama al sistema operativo no distribuye ninguno. Si eso basta en cada mercado sigue siendo **VERIFICAR con asesoría legal** (R-01). La última patente de H.264 en EE. UU. caduca el 29 de noviembre de 2027 según fuentes secundarias (VERIFICAR), fecha en la que conviene revisar esta decisión.
  - Si la API del sistema no está disponible (por ejemplo, Linux sin VA-API), Gafira avisa y propone AV1 + Opus. No hay alternativa de software para H.264.
- **HEVC**: **prohibido como salida** en cualquier vía. Su importación queda fuera del MVP; si se admite vía la API del sistema es una pregunta abierta para el titular (Q-17).
- **Feature `encumbered-sw`**: permanece **desactivada y sin código**. No existe ninguna implementación detrás de ese nombre; añadirla exigiría un ADR nuevo, un dictamen legal y una licencia de patentes.
- **HEIC/HEIF**: solo lectura vía el SO (WIC con la extensión HEVC en Windows; ImageIO en macOS), feature `heic-os`, Fase 2, condicionada a la respuesta de Q-17. En Linux no se soporta.
- La UI muestra las opciones libres de regalías como recomendadas (redacción final en la spec 03). Cuando el códec del SO falta, propone la alternativa sin bloquear.

*Descartado*:

- Incluir FFmpeg (GPL/LGPL y patentes).
- Encoders o decoders H.264, HEVC o AAC propios o de terceros en el binario: la exposición a patentes no desaparece por implementarlos uno mismo (D-02).
- Exportar MP3 (codificador LGPL) o HEVC (patentes).

### 9.4 Licencia propia de Gafira

Gafira es **propietaria y de código cerrado** (D-01). El contrato reutiliza el de Tracewright (`LICENSE` en inglés y `LICENCIA.md` en español; prevalece el español) adaptado a Gafira: el usuario es titular de los diseños, imágenes y videos que crea, y puede usar dentro de sus creaciones las plantillas, fuentes y recursos incluidos. El modelo es **todo de pago con prueba gratuita** y **una sola licencia**: no hay núcleo gratuito ni módulos avanzados de pago separados, y el mismo binario contiene todas las funciones. La activación y sus requisitos están en §13. La arquitectura garantiza que ninguna dependencia condiciona esta licencia.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-70 | `cargo deny check` pasa en todos los builds de release. | Job de CI obligatorio que bloquea el merge. |
| ARQ-71 | No hay dependencias git. | `[sources] allow-git = []` y job verde. |
| ARQ-72 | Ningún ejecutable de terceros en el instalador. | `xtask audit-bundle` lista los ejecutables y librerías del paquete: solo `gafira` y `gafira-cli`. Las librerías dinámicas solo pueden ser del SO. |
| ARQ-73 | Se distribuyen las licencias de terceros. | El instalador contiene `THIRD_PARTY_NOTICES.md` y los `OFL.txt` de las fuentes. «Acerca de → Licencias» lo muestra. |
| ARQ-74 | Los códecs con patentes son opcionales por feature y la exportación por defecto es AV1 + Opus. | `cargo build --no-default-features --features free-only` compila y exporta WebM y MP4 (AV1/Opus). En el build completo, el preajuste por defecto de Exportar video es AV1 + Opus. |
| ARQ-75 | Todo asset incluido tiene una licencia registrada. | `assets/LICENSES.toml` cubre el 100 % de los archivos de `assets/`, comprobado por `xtask`. Hasta que exista `xtask`, el archivo puede ser parcial, con dos entradas obligatorias desde su primer commit: las fuentes de prueba (ARQ-115) y `assets/splash_screen.jpg` (D-09). Toda entrada tiene los campos de 05 FV-38, y quitar uno de ellos hace fallar la comprobación; la de `splash_screen.jpg` tiene además los de D-09. |
| ARQ-76 | No se incluyen modelos de ML en el instalador base. | `audit-bundle` no encuentra archivos `.onnx`, `.safetensors` ni `.bin` de pesos. |
| ARQ-77 | El binario no contiene encoders ni decoders propios de H.264, HEVC o AAC, no exporta HEVC ni MP3, y `encumbered-sw` no tiene código. | `cargo deny` bloquea los crates vetados de §9.1. Una prueba recorre `CodecRegistry` del build completo: ningún `Encoder` con `CodecId::Hevc` o `Mp3`, y los `CodecId::H264`/`Aac` solo los aporta `gafira-codec-os`. Una búsqueda de `encumbered-sw` en el código solo la encuentra en documentación. |
| ARQ-78 | La app no comprueba versiones nuevas (D-06). | No existe la feature `update-check` ni código de firma de manifiestos de actualización. La prueba de red de ARQ-91 no registra ninguna conexión fuera de las de licencia. |
| ARQ-79 | Las bibliotecas LGPL del sistema solo se enlazan de forma dinámica y nunca se empaquetan. | `xtask audit-bundle` lista las librerías de cada paquete (MSI, `.deb`, `.rpm`, `.tar.gz`, DMG) y falla si encuentra `libasound` u otra biblioteca LGPL. Los paquetes Linux declaran `libasound2` (o equivalente) como dependencia del sistema. |

---

## 10. Estrategia de pruebas y calidad

| Nivel | Qué | Herramienta | Umbral |
|---|---|---|---|
| Unitarias de dominio | `gafira-geom`, `-color`, `-time`, `-anim`, `-doc`, `-edit` sin I/O | `cargo test` | Cobertura de líneas ≥ 85 % en el dominio (FIJADA) con `cargo-llvm-cov` |
| Propiedades | Para cada `Command`: `apply; revert` ≡ instantánea previa; `apply; revert; apply` ≡ `apply`; la fusión equivale a aplicar la secuencia; serialización *round-trip*; muestreo de animación | `proptest` y generadores de `gafira-testkit` | 1 000 casos por comando en PR y 100 000 por noche |
| Golden de imagen | Render CPU por tipo de nodo, efecto, modo de mezcla y texto; GPU frente a CPU | `gafira-testkit::golden` | CPU: idéntico byte a byte en la misma plataforma; entre plataformas ≤ 1/255 en el 99,9 % de los píxeles. GPU: ≤ 2/255 en el 99,5 % |
| Golden de formato | Migraciones, exportadores SVG/PDF (estructura normalizada), MP4/WebM (tabla de *boxes* o elementos) | `insta` | Sin diferencias no aprobadas |
| Fuzzing | Cada `Importer`, `Demuxer`, `Decoder` y el lector `.gafira` | `cargo fuzz` (libFuzzer) | 10 min por objetivo en PR (los afectados); 8 h por semana; 0 fallos |
| Benchmarks | Cada cifra de §6.7 | `criterion` y `gafira-cli bench` | Se rompe CI si se supera el umbral de `perf-budgets.toml` (en la máquina de referencia nocturna) o hay una regresión > 10 % |
| E2E | Flujos completos sin UI (Facade) y con UI (`egui_kittest`, VERIFICAR) | `cargo test --test e2e` | Flujos del MVP en verde en las 3 plataformas |
| Robustez | Kill aleatorio, inyección de fallos de E/S y GPU, límites de memoria | Arneses de testkit | §6.7 y §7 |
| Localización | Cobertura de claves es/en, parámetros y plurales; regla de D-08; formatos por idioma; seudo-localización con capturas; texto literal en la UI | `xtask check-i18n`, `xtask check-ui-literals`, `cargo test -p gafira-i18n`, E2E con `PseudoTranslator` | 100 % de claves en los dos idiomas; 0 literales; 0 truncados sin tooltip (§14.7) |

### 10.1 CI

- **Matriz**: `x86_64-pc-windows-msvc` (Windows Server 2022), `x86_64-unknown-linux-gnu` (Ubuntu LTS) y `aarch64-apple-darwin` (macOS 14 arm64).
- **Pasos por PR**: `fmt --check`, `clippy -D warnings`, `check-layers`, `deny`, `check-i18n`, `check-ui-literals`, `check-glossary`, pruebas, fuzz corto y benchmarks de humo.
- **Por la noche**: benchmarks completos en la máquina de referencia (*self-hosted*), fuzz largo, paquetes firmados y capturas golden en español, inglés y seudo-localización (§14.6).
- **Empaquetado**:
  - Windows: MSI con WiX v5, CRT estático y firma Authenticode. WiX se usa solo en build; su licencia MS-RL no se distribuye (VERIFICAR).
  - Linux (solo `x86_64`, D-05): `.deb`, `.rpm` y `.tar.gz` portable. Las bibliotecas del sistema (Vulkan, X11/Wayland, ALSA/PipeWire) no se empaquetan (ARQ-79).
    - **Sin AppImage**: su *runtime* enlaza libfuse con licencia LGPL-2.1, lo que obligaría a adjuntar el texto de esa licencia y una oferta de código fuente con cada copia.
    - **Sin Flatpak** y **sin aarch64** en Linux.
  - macOS: `.dmg` firmado y notarizado, solo aarch64, con `LSMinimumSystemVersion` 12.0.
- **Actualizaciones** (D-06): la app **no comprueba** versiones nuevas y no contiene actualizador ni firma de manifiestos de actualización. El usuario descarga cada versión desde la web. Los paquetes siguen firmados por plataforma (Authenticode, notarización de Apple; firma de `.deb`/`.rpm`, VERIFICAR herramienta).

### 10.2 Perfiles de compilación

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 4
panic = "unwind"          # ADR-05
debug = "line-tables-only"
split-debuginfo = "packed" # símbolos aparte para informes de fallo locales
strip = "symbols"

[profile.dist]            # el que se empaqueta
inherits = "release"
lto = "fat"
codegen-units = 1

[profile.dev.package."*"]
opt-level = 2             # dependencias (códecs, imagen) usables en desarrollo
```

- **Presupuesto de tamaño**: el job `xtask size` falla si el binario `dist` supera 40 MB o el instalador supera 60 MB (FIJADA, D-03). `cargo bloat` informa de las 20 mayores contribuciones en cada release.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-80 | CI verde en las 3 plataformas es condición de merge. | Reglas de protección de rama. |
| ARQ-81 | Cada `Command` del núcleo tiene una prueba de propiedad. | `xtask` comprueba que cada tipo que implementa `Command` en `gafira-edit` aparece en `prop_commands.rs`. Hasta que exista `xtask`, el paso V14 de la CI hace la misma comprobación. |
| ARQ-82 | Cada `Importer` tiene un objetivo de fuzz. | `xtask` compara el registro de importadores con `fuzz/fuzz_targets`. |
| ARQ-83 | Se respeta el presupuesto de tamaño. | Job `xtask size`. |
| ARQ-84 | Los paquetes de Linux son exactamente `.deb`, `.rpm` y `.tar.gz` para `x86_64`. | `xtask package linux` produce esos tres artefactos y ningún otro; la CI no tiene *targets* Linux distintos de `x86_64-unknown-linux-gnu`. |
| ARQ-85..ARQ-89 | Reservados. | — |

---

## 11. Mapa de extensibilidad y hoja de ruta técnica

### 11.1 Puntos de extensión

| Punto de extensión | Trait | Registro | Interno (MVP) | Plugin (Fase 3) |
|---|---|---|---|---|
| Tipo de nodo | `NodeExtension` | `NodeRegistry` | — | Sí |
| Operación sobre el documento | `NodeVisitor` | — (código) | Sí | No |
| Herramienta | `Tool` | `ToolRegistry` | Sí | Limitado |
| Comando | `Command` | — | Sí | Vía API de comandos |
| Efecto o filtro | `Effect` / `GpuEffect` | `EffectRegistry` | Sí | Sí (solo CPU) |
| Transición entre clips | `Transition` / `GpuTransition` | `TransitionRegistry` | Sí | Sí (solo CPU) |
| Formato | `Importer` / `Exporter` | `FormatRegistry` | Sí | Sí (bytes → bytes) |
| Contenedor y códec | `Container` / `Codec` | `MediaRegistry` | Sí | No |
| Backend de render | `RenderBackend` | `BackendSelector` | Sí | No |
| Segmentador | `Segmenter` | `SegmenterRegistry` | Sí | Sí (paquete con modelo) |
| Procesador de audio | `AudioProcessor` | `AudioRegistry` | Sí | Fase 3+ |
| Transformación de color | `ColorTransform` | `ColorRegistry` | Sí | LUT como datos |
| Recursos | `AssetProvider` | `AssetRegistry` | Sí | Sí |
| Asistente de texto (opcional, local) | `TextAssistant` | `TextAssistantRegistry` | No (vacío en el MVP) | Fase 3, paquete con modelo |
| Licencia (puerto, no extensión) | `LicenseVerifier` | — (una implementación inyectada) | Sí (`gafira-license`) | No |
| Idioma (puerto, no extensión) | `Translator`, `LocaleResolver`, `SystemLocaleSource` | — (una implementación inyectada) | Sí (`gafira-i18n`, `gafira-platform`); un idioma nuevo = catálogo + variante de `Locale` + glosario (§14.8) | No (los plugins de Fase 3 aportan claves con sus textos en los dos idiomas) |

### 11.2 Fases

| Fase | Crates entregados | Traits activos | Formatos de importación | Formatos de exportación |
|---|---|---|---|---|
| **MVP** (diseño + video básico) | geom, color, time, anim, doc, edit, pathops, text, layout, raster, audio, segment (clásico), ports, render, media, engine, format, assets, io-image, io-svg, io-pdf, mux, codec-free, codec-os, render-cpu, render-gpu, platform, theme, i18n (español e inglés, D-08), license, ui (con la Pantalla de arranque, D-09), testkit, apps | `NodeVisitor`, `Tool`, `Command`, `Effect`, `Transition`, `Importer`, `Exporter`, `RenderBackend`, `Segmenter`, `Container`, `Codec`, `Decoder`, `Encoder`, `AudioProcessor` (ganancia, fundidos, EQ de 3 bandas, panorama, nivelación), `ColorTransform` (sRGB, P3), `AssetProvider`, `ProjectRepository`, `LicenseVerifier`, `Translator`, `LocaleResolver`, `SystemLocaleSource` | `.gafira` (con `versions/`); PNG, JPEG, WebP, GIF, TIFF, BMP, QOI (`gafira-io-image`); SVG (`gafira-io-svg`); MP4/MOV y WebM/MKV (`gafira-mux`) con H.264/AAC vía API del sistema (`gafira-codec-os`) y AV1/Opus/Vorbis/FLAC (`gafira-codec-free`); WAV, FLAC, Ogg y MP3 (**solo importación**, decodificador permisivo VERIFICAR) | `.gafira`; PNG (transparencia, escala), JPEG, WebP, GIF animado (`gafira-io-image`); SVG (`gafira-io-svg`); PDF RGB Digital/Impresión con sangrado y marcas de corte (`gafira-io-pdf`); **WebM y MP4 AV1 + Opus por defecto** (`gafira-codec-free`); MP4 H.264/AAC 480p–4K solo si la API del sistema está disponible (`gafira-codec-os` + `gafira-mux`); secuencia PNG; WAV; **extracción de fotograma (PNG) y de audio (WAV)** |
| **Fase 2** (control profesional) | captions, io-pptx, `scopes` en raster, `heic-os` (según Q-17), VP9 en codec-free, CMYK en io-pdf, AVI de solo lectura en mux | Keyframes con curvas en la UI, máscaras de video, *chroma key*, LUT (`LutTransform`), curvas y ruedas de color, puerta de ruido, compresor, limitador, reducción de ruido, *time-stretch*, cola de render avanzada, control de códec, bitrate y fps, medios vinculados y Relocalizar medios | + AVI con MJPEG, sin comprimir o PCM (D-07); HEIC (SO, solo lectura, según Q-17); PDF (hayro, VERIFICAR); SRT y VTT (`gafira-captions`); LUT `.cube` | + PDF CMYK (moxcms); SRT y VTT; WebM VP9; extracción de fotograma en JPEG y de audio en FLAC u Opus; **PPTX** con una imagen de alta resolución por diapositiva y sin objetos editables (`gafira-io-pptx`, D-07) |
| **Fase 3** (plugins, lotes) | plugin, `onnx` en segment, lotes extendidos en cli | `PluginHost`, `NodeExtension` por plugin, `OnnxSegmenter`, `TextAssistant` (opcional, local, desactivable) | Formatos de plugins | Formatos de plugins; exportación por lotes con plantillas de datos (CSV). El render distribuido en la red local del usuario queda **en suspenso**: abriría una conexión de red distinta de la de licencia, prohibida por D-01 (Q-18) |

---

## 12. Riesgos y preguntas abiertas

### 12.1 Riesgos

| ID | Riesgo | Tipo | Mitigación |
|---|---|---|---|
| R-01 | El uso de H.264/AAC vía las API del sistema puede no cubrir las obligaciones de patentes del distribuidor en algún mercado. | Patentes | Dictamen legal antes de vender (D-02). La feature `os-codecs` se puede apagar y AV1 + Opus es siempre la salida por defecto. Revisar al caducar la última patente de H.264 en EE. UU. (29 de noviembre de 2027, VERIFICAR). |
| R-02 | Linux sin VA-API no puede importar ni exportar H.264, que es el formato habitual de los móviles. | Patentes / funcional | Decidido en D-02: no hay alternativa de software. Mensaje claro, propuesta de AV1 + Opus para exportar y guía para transcodificar fuera de la app. |
| R-03 | La codificación AV1 por software puede no alcanzar ≥ 10 fps a 1080p en gama media. | Rendimiento | Preset rápido de rav1e, *tiles* paralelos; benchmark temprano (*spike* en la semana 1). Si no se alcanza, la cifra del Reel solo se garantiza con el códec del SO. Como AV1 es la salida por defecto (D-02), este riesgo sube de prioridad. |
| R-04 | Aislar sujeto en < 2 s en GPU con técnica clásica puede no ser viable. | Rendimiento | La cifra de CPU (< 5 s) es la obligatoria en el MVP; la de GPU se valida con un *spike*. |
| R-05 | Las licencias de crates (vello, parley, krilla, linesweeper, cpal, rav1e, `nanomp3`, cliente HTTP y `ring`…) ya pasan `cargo-deny` (72 de 76 con features por defecto; los otros 4 se corrigen en §9.2), pero pueden cambiar o traer dependencias transitivas no permitidas en versiones futuras; las bibliotecas en C quedan VERIFICAR. | Licencia | `cargo-deny` en CI y alternativas listadas en §9.2. |
| R-06 | `cpal` en Linux enlaza `libasound` (LGPL-2.1), biblioteca del sistema. | Licencia | **VERIFICAR** con asesoría legal. Mitigación propuesta: enlace dinámico a la biblioteca del sistema, nunca empaquetarla (ARQ-79, §9.1). Alternativa si el dictamen lo exige: backend propio por PipeWire/PulseAudio. |
| R-07 | Las diferencias de render GPU entre drivers rompen los golden. | Calidad | Tolerancias, lista de bloqueo de drivers y CPU como referencia de exportación. |
| R-08 | Las tipografías, plantillas e iconos incluidos sin licencia clara invalidan la venta. | Licencia | ARQ-75. |
| R-09 | El tamaño del instalador supera 60 MB por las plantillas incluidas. | Distribución | Plantillas comprimidas; paquete de contenido opcional descargable manualmente desde la web (la app no lo descarga, D-01). |
| R-10 | egui no cubre algún requisito de accesibilidad (lectores de pantalla en el lienzo). | Accesibilidad | AccessKit con árbol propio; iced como plan B (ADR-06). |
| R-11 | El MP4 con AV1/Opus puede no aceptarse en algunas plataformas de publicación. | Compatibilidad | VERIFICAR; documentar y recomendar WebM o el códec del SO. |
| R-12 | El `license-hub` no está listo para vender: su README indica que no está desplegado (aunque otra sección del mismo README dice lo contrario) y tiene una lista de seguridad pendiente. | Comercial / seguridad | Completar la lista de seguridad y desplegarlo antes de vender (D-01, pendiente 3 del titular). VERIFICAR el estado real. La app no depende de él para editar mientras el token sea válido. |
| R-13 | Una prueba gratuita local se puede reiniciar o alargar manipulando el reloj o reinstalando. | Comercial | Comprobación monotónica de la fecha y registro en el llavero del SO; alternativa con prueba firmada por el servidor (Q-19). |
| R-14 | En Linux el llavero del SO (Secret Service por D-Bus) puede no estar disponible. | Funcional | Alternativa de archivo local con permisos restringidos y aviso (VERIFICAR); el token es verificable aunque el secreto falte, pero validar o desactivar exige el secreto. |
| R-15 | La diferencia de longitud entre español e inglés trunca etiquetas en el Riel de pestañas, la Barra contextual o el Inspector. | Calidad / UI | Diseño para expansión (03 §16.4), seudo-localización con +30–100 % (§14.6) y capturas golden en los dos idiomas; prueba de 0 truncados sin tooltip (DIS-76). |
| R-16 | Los datos de ICU4X o una dependencia transitiva añaden una licencia no permitida o inflan el binario por encima de 40 MB. | Licencia / distribución | Datos compilados solo para `es` y `en`; `cargo deny` y `cargo bloat` en CI; presupuesto propio de `gafira-i18n` (ARQ-106); alternativa: formateadores propios para los dos idiomas (ADR-09). |
| R-17 | Un nombre inglés del glosario coincide con el nombre exclusivo de otro producto o con una marca registrada. | Marca | Los nombres ingleses son PROPUESTA (03 §2.8) hasta la aprobación del titular; revisión de DIS-03 y DIS-72 y búsqueda de marca antes de publicar (Q-25). |
| R-18 | Un mensaje sin traducir llega a una versión publicada (por ejemplo, la clave de un plugin o de un error raro). | Calidad | Cobertura del 100 % como condición de merge (ARQ-98); en release el respaldo nunca muestra una clave (ARQ-99). |
| R-19 | La creación del dispositivo `wgpu` y de la ventana de eframe, previa al primer frame, supera los 300 ms de la Pantalla de arranque en algunos equipos o drivers; o eframe no permite la ventana secundaria sin decoraciones ni la ventana principal oculta (D-09, §8.4). | Rendimiento / UI | Medir `splash_first_frame_ms` en los tres perfiles de referencia (VERIFICAR); respaldo `glow` o software de eframe (ADR-06) y respaldo de dibujo dentro de la ventana principal (ARQ-65); la cifra de 300 ms es PROPUESTA y se ajusta tras medir. |
| R-20 | El origen o la licencia de la ilustración `assets/splash_screen.jpg` no permiten distribuirla en un producto de pago. | Licencia / marca | Registro en `assets/LICENSES.toml` antes de publicar (ARQ-75, pendiente 6 de 00-decisiones); si no se puede acreditar, sustituirla siguiendo 03 §7.10.10. |

### 12.2 Preguntas cerradas

Resueltas por el titular. Se conservan los IDs porque otras specs los citan. Q-01..Q-16 constan en `00-decisiones.md`; Q-27..Q-33 se resolvieron en el cambio `anim-core` y su regla está en §3.4. Q-35..Q-45 constan en `00-decisiones.md` (D-29 a D-34 y D-41 a D-45). Q-49 se cerró en la verificación de SG3 de `gafira-segment`.

| ID | Pregunta | Resolución |
|---|---|---|
| Q-01 | Licencia y modelo comercial. | D-01: propietaria, todo de pago con prueba gratuita, una sola licencia (§9.4, §13). |
| Q-02 | ¿Basta delegar H.264/HEVC/AAC en el SO? | D-02: H.264/AAC solo vía API del sistema; HEVC sin salida; AV1 + Opus por defecto. El dictamen legal sigue pendiente (R-01). |
| Q-03 | Linux sin VA-API e importación de H.264. | D-02: no hay decodificador de software; Gafira avisa y propone AV1. |
| Q-04 | ¿Linux aarch64? | D-05: no; solo x86_64. |
| Q-05 | Formatos de paquete Linux. | D-05: `.deb`, `.rpm` y `.tar.gz`; sin AppImage ni Flatpak. |
| Q-06 | Actualizaciones firmadas. | D-06: la app no comprueba versiones; sin `update-check`. |
| Q-07 | Hardware de referencia y cifras. | D-03: fijados (§6.6). |
| Q-08 | PPTX. | D-07: Fase 2, una imagen por diapositiva (`gafira-io-pptx`). |
| Q-09 | MP3 como importación. | D-07: solo importación con decodificador permisivo (`nanomp3`, §9.2; patentes VERIFICAR). |
| Q-11 | Reparto núcleo/avanzado (01). | D-01: no hay módulos de pago separados. |
| Q-12 | Activación de los módulos avanzados (01). | D-01: activación con `gafira-license` y `license-hub` (§13). |
| Q-13 | Importación de AVI y de AAC/M4A más allá del SO (01). | D-07: AVI en Fase 2 solo con códecs libres; D-02: AAC solo vía SO. |
| Q-16 | Aprobación del glosario de 03 (01). | D-04: aprobado; queda la búsqueda de marca del titular. |
| Q-14 | Idioma del producto (01). | D-08: español e inglés; idioma por defecto según el sistema (cualquier variante de español → español; cualquier otro → inglés); selector en Ajustes → General. Arquitectura en §14 (`gafira-i18n`, ADR-09). |
| Q-27 | ¿Cómo se interpola el alfa de `Color` (ARQ-19)? | Titular: A, alfa premultiplicado, como CSS Color 4 (§3.4, ARQ-19). |
| Q-28 | ¿Qué hace `Transform2D::interpolate` con un extremo no descomponible (determinante 0 o coeficientes no finitos)? | Titular: B. El extremo degenerado toma la rotación y el sesgo del otro extremo y solo se interpolan traslación y escala; si los dos son degenerados, se interpolan los 6 coeficientes linealmente; un coeficiente no finito hace inválida la pista (§3.4, ARQ-18, ARQ-19). La escala del extremo degenerado se fija en Q-30. |
| Q-29 | ¿`sample` da resultados idénticos bit a bit en las 3 plataformas (ARQ-12)? | Titular: A, enmendada tras el spike de `anim-core`. Bit a bit para `f32`, `Vec2` y las curvas (aritmética IEEE; sin FMA comprobado en el ensamblador de x86_64, aarch64 sin comprobar). `Color` y `Transform2D` con tolerancia, porque usan `powf` y funciones trigonométricas de la `libm` de la plataforma: `4 · ε_f32 · max(1, abs(c))` por canal en `Color` y `1e-9 · max(1, abs(c))` por coeficiente en `Transform2D` (PROPUESTA; §3.4, ARQ-12). |
| Q-30 | ¿Qué escala por eje tiene un extremo degenerado de `Transform2D` (ARQ-19, Q-28 B)? | Titular: A. La diagonal de su parte lineal expresada en el marco de rotación y sesgo del otro extremo. Un extremo de rango 1 que no encaja en ese marco (por ejemplo `[1, 2, 2, 4]`) salta justo después de `t = 0` (o antes de `t = 1`); se acepta como comportamiento conocido y documentado, porque los presets solo producen escalas 0 por eje (§3.4, ARQ-19). |
| Q-31 | ¿Qué da `Color::interpolate` cuando el alfa resultante es ≤ 0 (ARQ-19, Q-27 A)? | Titular: A. Con `a ≤ 0` se interpolan `r`, `g`, `b` en recto, sin premultiplicar, y `a` queda sin recortar; el consumidor recorta `a` al componer (§3.4, ARQ-19). |
| Q-32 | ¿Qué hace `f32::interpolate` (y `Vec2`, en `f64`) con extremos finitos cuya diferencia desborda, por ejemplo de `3e38` a `−3e38` con `t = 0,5` (ARQ-19)? | Titular: A. Se mantiene `a + (b − a) · t`. El rango de contrato es `b − a` finito en cada componente; fuera de él el resultado puede ser `±∞` y queda fuera de contrato. Las pruebas de propiedad usan el dominio `[−MAX/2, MAX/2]` de su tipo. No cambian el código ni los resultados bit a bit (Q-29) (§3.4, ARQ-19). |
| Q-33 | ¿Cómo se decide en coma flotante que un extremo de `Transform2D` es degenerado (§3.4, ARQ-19, Q-28 B)? | Titular: B, umbral relativo. Con la parte lineal `[a, b, c, d]`, columnas `c1 = (a, b)` y `c2 = (c, d)` y `det = a·d − b·c` en `f64`, un extremo de coeficientes finitos es degenerado si `det` o `|c1| · |c2|` no es finito, o si `|det| ≤ 1e-12 · |c1| · |c2|`. No depende de la escala, trata como rango 1 lo que solo lo es por redondeo y da el mismo resultado en las 3 plataformas (Q-29). Justo por encima del umbral el sesgo crece como el inverso del cociente; se acepta como comportamiento conocido. Ningún preset cambia: todos producen ceros exactos (§3.4, ARQ-19). |
| Q-35 | Al alinear una selección con elementos bloqueados, ¿la caja de referencia los incluye (04 FD-11)? | D-29: sí. |
| Q-36 | ¿Cómo se prueba el emoji de 04 FD-41 (a), si las fuentes de D-19 no lo tienen? | D-30: se añade un subconjunto OFL monocromo y se comprueban contornos reales. |
| Q-37 | ¿Cómo aplica Nivelar más de +6 dB si el Volumen de clip llega a 200 % (05 FV-10)? | D-31: con una ganancia propia, aparte del Volumen, aplicada por `LoudnessNormalizer`. El tope de −1 dBTP de D-21 sigue vigente. |
| Q-38 | ¿Dónde está Nivelar en el MVP, si la interfaz del Mezclador es de Fase 2? | D-32: en Inspector → Clip. |
| Q-39 | ¿FD-33 tiene un slider de Desenfoque? | D-33: no en el MVP. |
| Q-40 | ¿Se confirman en bloque las cifras PROPUESTA nuevas de capa 2? | D-34: sí, y pueden cambiar con mediciones. Conservan su marca. |
| Q-41 | ¿Cómo actúa la Intensidad de 04 FD-34 en parámetros `Int`, `Bool`, `Choice` y `Color`? | D-41: `Int` mezcla y redondea, medio se aleja de 0; `Color` mezcla en luz lineal; `Bool` y `Choice` saltan al preset por encima de 0. |
| Q-42 | ¿Qué fórmula y qué criterio tienen Brillo y Contorno de 04 FD-35, o salen del primer corte de `gafira-raster`? | D-42: salen del corte hasta que se aprueben sus fórmulas. |
| Q-43 | ¿Medir la sonoridad, Nivelar y remuestrear un clip largo se pueden cancelar y dan progreso? | D-43: quedan fuera de ARQ-112 en este corte; `gafira-engine` los cancela entre bloques más adelante. |
| Q-44 | ¿En qué espacio se calculan los colores medios de ARQ-131? | D-44: en luz lineal, y después se pasan a Lab. |
| Q-45 | ¿Se acepta dejar fuera de ARQ-132 el fondo texturizado a ΔE76 6, que pasa con `gamma` 50 y falla con 500? ¿Qué ΔE76 mínimo tienen entonces las escenas texturizadas? | D-45: sí, queda fuera, y las escenas texturizadas de la puerta conservan el mínimo de ARQ-132. |
| Q-49 | ¿De qué tamaño son las escenas de ARQ-132? | 4000×3000 px, lado largo > 2048, de modo que la copia reduce y D-45 se ejercita. La entrada de referencia de ARQ-112 sigue a 1024×768. |

### 12.3 Preguntas abiertas

| ID | Pregunta | Responsable |
|---|---|---|
| Q-10 | ¿Se permite empaquetar fuentes del usuario dentro de `.gafira` según su licencia? ¿Cómo se informa? | Legal / producto |
| Q-17 | ¿Se admite **importar** HEVC (y HEIC) mediante la API del sistema, o queda prohibido en toda vía? D-02 prohíbe «HEVC» sin distinguir importación de exportación; esta spec lo excluye del MVP y prohíbe su salida. | Titular + legal |
| Q-18 | ¿Se mantiene el render distribuido en la red local (Fase 3)? Abriría conexiones distintas de la de licencia, prohibidas por D-01. | Titular |
| Q-19 | ¿Cómo se implementa la prueba gratuita: local desde el primer arranque (sin red, más fácil de alargar) o con un token de prueba firmado por el `license-hub` (exige conexión al empezar y un endpoint que el servicio no tiene, VERIFICAR)? Duración y límites: pendientes de D-01. | Titular |
| Q-20 | ¿Qué puede hacer el usuario sin licencia válida (prueba vencida)? Esta spec propone un modo de solo lectura que permite abrir, ver, recuperar y guardar copia, sin editar ni exportar (ARQ-95). | Titular |
| Q-23 | ¿Qué idiomas se añaden después de español e inglés, y cuándo? La arquitectura los admite (§14.8), pero cada uno exige catálogo completo, glosario 1:1, revisión de expansión y quizá ADR-09 revisado (Fluent si su gramática lo pide). | Titular |
| Q-24 | ¿Los números y fechas siguen la región del sistema dentro del idioma (es-MX escribe `1,234.5`; es-ES escribe `1234,5`) o un formato fijo por idioma? Esta spec propone seguir la región cuando su idioma coincide con el de la interfaz (§14.4). | Titular |
| Q-25 | Aprobación de los nombres ingleses del glosario (03 §2.8) y del microcopy inglés (03 §12), y búsqueda de marca de los nombres ingleses propios. | Titular |
| Q-26 | ¿Se confirma que `gafira-cli` imprime en inglés por defecto aunque la app esté en español (ARQ-104)? ¿Y que el instalador, los paquetes y el `.desktop` de Linux se ofrecen en los dos idiomas (05 FV-30 dice solo español)? | Titular |
| Q-34 | ¿Se aprueba el aspecto de las fórmulas de 04 FD-33? Son PROPUESTA y las pruebas fijan su valor, no su aspecto. El titular las revisará sobre imágenes de muestra cuando exista el corte de efectos, y con ellas dará el catálogo de filtros de 04 FD-34 (D-40). No bloquea la especificación. | Titular |
| Q-46 | Si los fundidos de entrada y salida de un clip suman más que el clip, ¿se solapan multiplicando sus ganancias o las asas no pueden cruzarse (05 FV-10)? Ninguna spec lo dice. | Titular |
| Q-47 | ¿Tiene duración un Corte? 05 FV-06 lo llama instantáneo, pero da a toda transición de 0,1 a 5 s, y con alineación al inicio o al final mostraría fotogramas más allá del clip, aunque su `handles` es `None`. Recomendación: Corte ignora la duración y la alineación, y el cambio ocurre en el corte. | Titular |
| Q-48 | ¿Se cumplen CAL-06, CAL-26 y ARQ-43 en el hardware de D-03 con la copia de 2048 px de D-45? Las medidas de SG3 son indicativas, porque se tomaron en un i7-12700H con carga: CAL-06, 1055 ms con 1 hilo, margen de 4,7 veces; ARQ-43, tramo más largo de 43 ms con 1 hilo, margen de 2,3 veces; CAL-26, pico de *heap* de 35 a 303 MB de 1 a 20 hilos, que es solo la parte del segmentador. Se cierra con `bench_segment_4000` < 5 s y `bench_cancel_segment` ≤ 100 ms en la gama media de D-03, y CAL-26 medida en el proceso con 20 páginas e imágenes. Si alguna falla, el titular decide. Las cifras siguen vigentes. | Titular, tras medir |

---

## 13. Licenciamiento y activación (`gafira-license`)

### 13.1 Alcance y servicio externo

- **Modelo** (D-01): Gafira es de pago con prueba gratuita y una sola licencia. El mismo binario contiene todas las funciones; la licencia solo decide si se puede editar y exportar.
- **Servicio**: la activación usa el **`license-hub`** del titular, un servicio externo (Cloudflare Workers + D1; proveedor Gumroad o clave propia; de 1 a 10 dispositivos por licencia). **No se despliega con la app**, no forma parte del workspace de Gafira ni del instalador. Gafira solo contiene el cliente. La URL base, el `iss` esperado y el identificador de aplicación (`"gafira"`, PROPUESTA) se fijan en el build; Gafira se registra en el servicio como una aplicación más.
- **Única conexión de red**: activar, validar o desactivar la licencia. Nada más sale del equipo (ARQ-91).

### 13.2 Diseño del crate

`gafira-license` reutiliza el **diseño** de `tracer-license` (Tracewright), sin copiar su código:

| Módulo | Responsabilidad | Feature |
|---|---|---|
| `token` | `verify_token`: firma Ed25519, `iss`, `aud`, dispositivo, `exp` e `iat` con tolerancia de reloj de 60 s. | — (puro) |
| `claims` | `ActivationClaims { kind, iss, aud, sub, jti, device, iat, exp }` y `VerifiedToken`. | — |
| `state` | Máquina de estados pura (§13.3). No usa red ni reloj: recibe `now`. | — |
| `trial` | Estado de la prueba gratuita (mecanismo según Q-19). | — |
| `device` | Identificador de dispositivo aleatorio (UUID v4, **no** derivado del hardware) y secreto de 32 bytes aleatorios en base64url. | — |
| `store` | Trait `SecretStore` y `MemoryStore` (pruebas). `KeyringStore` en el llavero del SO. | `credentials` |
| `client` | `LicenseClient`: `activate`, `validate` y `deactivate` (tres POST), tiempo límite de 5 s y sin reintentos automáticos; la política de reintento es del llamador. | `online` |
| `adapter` | Implementa `LicenseVerifier` (§4.14) sobre los módulos anteriores. | — |

- **Clave pública fija**: la clave Ed25519 de producción es una constante del ejecutable. Nunca se descarga ni se lee del disco: aceptar la clave que publica el servidor permitiría a cualquiera que responda esa petición emitir licencias válidas. Una prueba comprueba que la constante coincide con la clave SPKI publicada.
- **Features**: `online` y `credentials` están desactivadas por defecto en el crate y se activan en el binario `gafira`. `gafira-cli` solo lee el token guardado y verifica su estado sin red (PROPUESTA); la activación se hace desde la app.
- **Hilos**: las llamadas de red son bloqueantes y se ejecutan en el hilo de E/S del engine.

### 13.3 Estados de licencia

| `LicenseStatus` | Cuándo | Editar y exportar |
|---|---|---|
| `Trial { ends_at }` | Prueba gratuita en curso | Sí |
| `Active { expires_at, activation_id }` | Token válido verificado sin conexión | Sí |
| `OfflineGrace { token_expires_at, retry_after }` | El servidor no responde, pero el token sigue vigente | Sí, con aviso discreto |
| `Expired` | Prueba o token vencidos | No (ARQ-95) |
| `Unlicensed` | Sin activación ni prueba | No (ARQ-95) |
| `Invalid { reason }` | Rechazo explícito del servidor o token alterado | No (ARQ-95) |

- **Invariante**: un fallo de red **nunca** invalida un token válido. `Active` pasa a `OfflineGrace` y sigue siendo usable hasta `exp`. Solo un rechazo explícito del servidor o el vencimiento quitan el permiso.
- **Gracia sin conexión**: la duración del token (`tokenTtlSeconds` del servicio) define cuánto tiempo puede trabajar el equipo sin conexión. Propuesta: 30 días. La app intenta validar sola como máximo una vez al día y solo cuando quedan menos de 7 días, y ofrece «Validar ahora» en Ajustes (cifras PROPUESTA).
- **Sin licencia válida**: la app abre, muestra, recupera y permite «Guardar copia» de los proyectos; no permite editar ni exportar. Los datos del usuario nunca quedan retenidos (PROPUESTA, Q-20).

### 13.4 Datos enviados y almacenamiento

- **Se envía** solo al activar, validar o desactivar: identificador de aplicación, clave de licencia, identificador del dispositivo, secreto del dispositivo (el servidor guarda solo su HMAC), sistema operativo y, si el usuario lo escribe, un nombre para el equipo. Nunca documentos, recursos, rutas, uso ni métricas. No hay telemetría.
- **`PRIVACY.md`** debe declarar el identificador de dispositivo que se envía al activar y no puede afirmar que la app «no transmite datos» (D-01).
- **Almacenamiento**: el secreto del dispositivo y el token se guardan en el llavero del SO (`credentials`). Nunca se escriben en un `.gafira`, en journals ni en logs.

### 13.5 ADR-08: licencia por token verificable sin conexión

**Decisión**: activación en línea contra el `license-hub` una vez por dispositivo y verificación local del token Ed25519 en cada arranque, sin red.

*Descartado*:

- Comprobar la licencia en línea en cada arranque: contradice el trabajo sin conexión.
- Archivo de licencia firmado sin activación: no permite limitar dispositivos ni desactivar uno perdido.
- Servidor de licencias propio dentro del producto: ya existe el `license-hub` y la app no debe incluir servidores.

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-90 | El token se verifica sin conexión con una clave pública fija. | Pruebas: un token firmado con otra clave, de otro dispositivo, con `aud` o `iss` distintos o con claims alterados se rechaza. `status` hace 0 llamadas de red (cliente HTTP falso con contador). La constante de clave coincide con la SPKI publicada. |
| ARQ-91 | La única conexión de red es la de licencia. | Prueba E2E con monitor de red del sandbox: en una sesión de activar → editar → exportar → desactivar, todas las conexiones van al host configurado del servicio y solo durante esas tres operaciones; con la licencia `Active`, editar y exportar abren 0 conexiones. |
| ARQ-92 | Un fallo de red no invalida un token válido. | Prueba de propiedad: para todo token vigente y todo error de red (tiempo agotado, DNS, 5xx), el estado resultante es `OfflineGrace` y `can_edit()` es verdadero hasta `exp`. |
| ARQ-93 | Las transiciones de estado son completas y deterministas. | Prueba de tabla: cada par (estado, evento) de §13.3 da el estado esperado con un `Clock` falso. |
| ARQ-94 | El secreto del dispositivo y el token no se filtran. | Búsqueda en logs, journals, `.gafira` e informes de fallo de la suite E2E: 0 apariciones del secreto o del token. |
| ARQ-95 | Sin licencia válida, el usuario conserva el acceso a sus proyectos. | Con `Expired`, la prueba abre un proyecto, lo muestra, recupera un journal y guarda una copia; editar y exportar están deshabilitados con un mensaje que lleva a activar. |
| ARQ-96 | El `license-hub` no se despliega con la app. | `xtask audit-bundle` no encuentra código del servicio (scripts del Worker, configuración de despliegue, migraciones SQL) ni secretos de administración en los paquetes; las cadenas del binario solo contienen la clave **pública**. |
| ARQ-97..ARQ-99 | Usados por el idioma: ver §14.7. | — |

---

## 14. Idioma e internacionalización (`gafira-i18n`, D-08)

### 14.1 Alcance y lugar en las capas

D-08 fija que Gafira se publica en **español e inglés**, que el idioma por defecto sale del sistema (cualquier variante de español → español; inglés → inglés; **cualquier otro idioma → inglés**) y que la persona puede cambiarlo en Ajustes → General. La documentación de las specs sigue en español.

| Pieza | Crate (capa) | Qué hace | Qué NO hace |
|---|---|---|---|
| `MessageKey`, `Message`, `MsgArg`, `ToMessage`, macro `msg!` | `gafira-doc::msg` (1, dominio) | Representa un mensaje como clave más parámetros tipados. `CommandLabel` es un alias de `MessageKey`. | Contener textos en un idioma, conocer `Locale` o formatear números. |
| Errores de motores y adaptadores | cada crate | Implementan `ToMessage` y devuelven claves con parámetros (ARQ-102). | Construir frases para la persona. |
| `Locale`, `LanguagePreference`, `ResolvedLocale`, `FormatLocale`, `Translator`, `LocaleResolver`, `SystemLocaleSource` | `gafira-ports::i18n` (3) | Contrato común para engine, UI y CLI (§4.16). | Implementar nada. |
| `SystemLocaleSource` | `gafira-platform` (4) | Lee el idioma de la interfaz y el formato regional del SO (§14.3). | Aplicar la regla de D-08. |
| `gafira-i18n` | adaptador (4) | `D08LocaleResolver` (§14.4), `CatalogTranslator` con catálogos embebidos, plurales y formatos (§14.5), respaldo y seudo-localización (§14.6). | Depender de egui, `gafira-engine`, `gafira-ui` o `gafira-platform`; leer el disco en release; estado global. |
| `SettingsService` | `gafira-engine` (3) | Guarda `LanguagePreference` en los ajustes, pide la resolución al `LocaleResolver` inyectado y publica `LocaleChanged` (§5.2). | Traducir textos. |
| UI y CLI | `gafira-ui`, `gafira-cli` | Guardan el `Locale` activo y piden cada texto al `Translator`. | Escribir texto visible literal (ARQ-97). |

```text
gafira-doc::msg ──► gafira-ports::i18n ──► gafira-i18n (ICU4X, catálogos)      gafira-platform (SO)
                           │                     ▲  implementa                       ▲ implementa
                           ▼                     │  Translator, LocaleResolver       │ SystemLocaleSource
                     gafira-engine (SettingsService → LocaleChanged)                 │
                           ▼                                                         │
             gafira-ui / gafira-cli ◄── apps/gafira, apps/gafira-cli: composición e inyección
```

`gafira-i18n` depende solo de `gafira-ports`, `gafira-doc` y de los crates de ICU4X de §9.2. Ningún crate de las capas 0–2 depende de `gafira-i18n` (ARQ-107).

### 14.2 Mensajes por clave en el dominio

```rust
// gafira-doc::msg — tipos neutros: no hay idioma ni texto traducido
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MessageKey(pub &'static str);            // «export.done», «error.format.too_new», «glossary.isolate_subject»
pub type CommandLabel = MessageKey;                 // Command::label (§4.3)

#[derive(Clone, Debug, PartialEq)]
pub enum MsgArg {
    Text(String),              // contenido de la persona (nombre de archivo, de fuente…) o identificador técnico estable (ADR 0012): nunca se traduce
    Count(u64),                // elige la forma de plural
    Int(i64), Decimal(f64),
    Quantity(f64, Unit),       // Unit: Px, Mm, Cm, In, Pt, Percent, Bytes, Seconds, Fps, Db, Lufs, Degrees
    Timestamp(i64),            // segundos Unix; se formatea como fecha u hora
    Elapsed(i64),              // segundos; se formatea como tiempo relativo («hace 2 h» / «2 h ago»)
    Key(MessageKey),           // un nombre del glosario dentro de otra frase («Deshacer {action}»)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message { pub key: MessageKey, pub args: Vec<(&'static str, MsgArg)> }
pub trait ToMessage { fn to_message(&self) -> Message; }

// Forma de crear un Message: xtask check-i18n extrae los literales de msg! y de MessageKey("…") y los compara con los catálogos.
// msg!("export.pages_selected", count = Count(n))
// El literal MessageKey("…") sigue siendo válido para etiquetas constantes (CommandLabel, Descriptor, ParamSchema).
```

- **Convención de claves**: ASCII en `snake_case` con puntos, `área.objeto[.estado]`: 2 o 3 segmentos, cada uno `[a-z][a-z0-9_]*`. Los nombres del glosario usan `glossary.<id>` y son los mismos en los dos idiomas (03 §2.8). Aceptación: valen `edit.set_property`, `error.format.too_new` y `a.b2`; no valen `a`, `.a`, `a.`, `a..b`, `a.b.c.d`, `A.b`, `a.2b` ni `a.é`; `msg!` con una clave no válida no compila.
- **Todo lo que se muestra lleva clave**: además del texto de `gafira-ui`, los `Descriptor` de los registros (§4), las etiquetas de `ParamSchema` de efectos y transiciones, las opciones de `Exporter::options_schema`, los `Destinos` de `assets/sizes.toml` y las contribuciones de plugins (Fase 3) declaran `MessageKey`, no `&str`.
- **Qué no se traduce**: el contenido de la persona (textos, nombres de proyecto, capa, archivo o fuente), los nombres de formato (PNG, MP4, AV1), la marca «Gafira», el texto de los logs, los nombres de subcomandos y opciones de la CLI y las claves del formato `.gafira`. Los nombres por defecto de los elementos nuevos («Página 1» / «Page 1») se crean en el idioma activo y desde ese momento son contenido de la persona: cambiar de idioma no los renombra.

### 14.3 Detección del idioma del sistema (`gafira-platform`)

| SO | Idioma de la interfaz (`SystemLocale.ui`) | Formato regional (`SystemLocale.formats`) | Medio |
|---|---|---|---|
| Windows 10/11 | Primer elemento de `GetUserPreferredUILanguages(MUI_LANGUAGE_NAME)` (idioma de visualización, p. ej. `es-MX`) | `GetUserDefaultLocaleName` (p. ej. `es-CO`) | Crate `windows` (ya previsto en §9.2) |
| macOS 12+ | Primer elemento de `CFLocaleCopyPreferredLanguages` (p. ej. `es-419`, `en-GB`) | Identificador de `CFLocaleCopyCurrent` (p. ej. `es_MX`) | `objc2` y su enlace de Core Foundation (nombre exacto del crate VERIFICAR) |
| Linux x86_64 | Primer elemento de `LANGUAGE` (lista separada por `:`) si la configuración efectiva no es `C`/`POSIX`; si no, el primero no vacío de `LC_ALL`, `LC_MESSAGES` y `LANG` | El primero no vacío de `LC_ALL`, `LC_NUMERIC` y `LANG` | `std::env` (permitido en adaptadores; prohibido en las capas 0–2 por ARQ-02) |

- `gafira-platform` devuelve las etiquetas tal como las da el SO, sin interpretarlas. Si una llamada falla o no hay dato, devuelve `None` y el resolvedor aplica el respaldo inglés.
- Se lee al arrancar y cada vez que la persona vuelve a elegir «Sistema». Un cambio de idioma del SO con la app abierta se aplica en el siguiente arranque (PROPUESTA).
- Solo cuenta el **primer** idioma preferido del SO: si la lista es `fr:es`, el idioma del sistema es francés y la regla da inglés, tal como dice D-08.

### 14.4 Regla de resolución (`D08LocaleResolver`)

```rust
// gafira-i18n
pub struct D08LocaleResolver { system: Arc<dyn SystemLocaleSource> }

impl LocaleResolver for D08LocaleResolver {
    fn resolve(&self, pref: LanguagePreference) -> ResolvedLocale {
        let sys = self.system.system_locale();
        let (locale, source) = match pref {
            LanguagePreference::Es => (Locale::Es, LocaleSource::Manual),
            LanguagePreference::En => (Locale::En, LocaleSource::Manual),
            LanguagePreference::System => match sys.ui.as_deref().and_then(primary_language) {
                Some(lang) if lang == "es" => (Locale::Es, LocaleSource::System { tag: sys.ui.clone().unwrap_or_default() }),
                Some(_)                    => (Locale::En, LocaleSource::System { tag: sys.ui.clone().unwrap_or_default() }), // en y cualquier otro
                None                       => (Locale::En, LocaleSource::Fallback),
            },
        };
        ResolvedLocale { locale, formats: format_locale(locale, sys.formats.as_deref()), source }
    }
}

/// Normaliza «es_CO.UTF-8», «es_ES@euro», «ES-mx», «es-419» → Some("es"); «C», «POSIX», «C.UTF-8», «» → None.
/// Sustituye `_` por `-`, quita `.codeset` y `@modificador` y analiza con `icu_locale_core::LanguageIdentifier`.
fn primary_language(tag: &str) -> Option<String>;

/// Región del formato regional solo si su idioma coincide con la interfaz (es-MX → `1,234.5`); si no, la de CLDR del idioma.
fn format_locale(locale: Locale, formats: Option<&str>) -> FormatLocale;

pub struct FormatLocale { pub locale: Locale, pub region: Option<Region> } // en gafira-ports::i18n
```

| Etiqueta del sistema | Resultado con «Sistema» |
|---|---|
| `es`, `es-ES`, `es-MX`, `es-CO`, `es-AR`, `es-419`, `es_CL.UTF-8`, `es_ES@euro`, `ES-mx` | Español (`System`) |
| `en`, `en-US`, `en-GB`, `en-AU`, `en_IN.UTF-8` | English (`System`) |
| `fr-FR`, `pt-BR`, `de-DE`, `it`, `zh-Hans-CN`, `ja`, y también `ca-ES`, `eu-ES` y `gl-ES` (catalán, euskera y gallego **no** son variantes del español) | English (`System`) |
| Sin dato, ilegible, `C`, `POSIX`, `C.UTF-8` | English (`Fallback`) |

- Con `LanguagePreference::Es` o `En` el sistema se ignora para el idioma de la interfaz (solo aporta la región de formato si coincide).
- **Persistencia**: `SettingsRepository` guarda `"language": "system" | "es" | "en"` (por defecto `"system"`; un valor desconocido se lee como `"system"`). Ajustes muestra las opciones «Sistema», «Español» y «English» (03 §16.1).
- **Formatos por región** (PROPUESTA, Q-24): con interfaz en español y formato regional `es-MX`, los números se escriben `1,234.5`; con `es-ES`, `1234,5` y `12.345,5`; con interfaz en inglés y formato regional francés, se usa el de CLDR para `en`.

### 14.5 Traducción, catálogos y formatos (`CatalogTranslator`)

- **Catálogos**: `assets/i18n/es.toml` y `assets/i18n/en.toml` (se pueden partir por área en `assets/i18n/<idioma>/<área>.toml`) y `assets/i18n/glosario.toml`, que reúne los nombres del glosario en los dos idiomas (03 §2.8). Son datos con licencia propia registrada en `assets/LICENSES.toml` (ARQ-75).

```toml
# assets/i18n/es.toml
[export]
done = "Tu diseño está listo para compartir"
pages_selected = { one = "{count} página seleccionada", many = "{count} de páginas seleccionadas", other = "{count} páginas seleccionadas" }

# assets/i18n/en.toml
[export]
done = "Your design is ready to share"
pages_selected = { one = "{count} page selected", other = "{count} pages selected" }
```

- **Generación en build**: `xtask gen-i18n` valida los TOML (ARQ-98) y genera tablas estáticas ordenadas (`&'static [(&'static str, Entry)]`, búsqueda binaria) que `gafira-i18n` incluye con `include!`. Los dos idiomas van siempre embebidos; en release no se lee ningún catálogo del disco. Solo en builds de desarrollo la variable `GAFIRA_I18N_PATH` recarga los TOML en caliente (feature `dev-reload`); el perfil `dist` la ignora, igual que `GAFIRA_THEME_PATH` (§8.2).
- **Sintaxis**: texto con parámetros `{nombre}` y, para mensajes con `Count`, una variante por categoría de plural de CLDR. No hay lógica dentro del catálogo. Los parámetros numéricos y de fecha los formatea el `Translator` según el `FormatLocale`; un parámetro `Key` se traduce en el mismo idioma (un nivel).
- **Plurales**: categorías cardinales de `icu_plurals`. Inglés exige `one` y `other`; español exige `one`, `many` y `other` (CLDR reciente; VERIFICAR con la versión de datos usada). Si falta una categoría, se usa `other` y `check-i18n` falla.
- **Formatos** (ICU4X; valores de CLDR, comprobados con pruebas golden, ARQ-106):

| Concepto | Español (`es`) | Inglés (`en`) | Fuente |
|---|---|---|---|
| Decimal y miles | `1,5` · `1234,5` · `12.345,5` | `1.5` · `1,234.5` · `12,345.5` | `icu_decimal` |
| Porcentaje | `100 %` (espacio fino no separable) | `100%` | `icu_decimal` + patrón del catálogo |
| Fecha corta | `12 mar 2026` | `Mar 12, 2026` | `icu_datetime` |
| Hora | `14:05` | `2:05 PM` | `icu_datetime` |
| Tiempo relativo | `hace 2 h` · `ayer` | `2 h ago` · `yesterday` | Mensajes del catálogo con plural (el formateador relativo de ICU4X no es estable, VERIFICAR) |
| Tamaño de archivo | `1,2 MB` | `1.2 MB` | `quantity` |
| Dimensiones y unidades | `1080 × 1080 px` · `0,5 s` · `−14 LUFS` | `1080 × 1080 px` · `0.5 s` · `−14 LUFS` | `quantity` |
| Código de tiempo | `00:12:04` | `00:12:04` | Igual en los dos idiomas |

- **Entrada numérica**: los campos numéricos (03 DIS-20) aceptan el separador decimal del `FormatLocale` activo y, además, el punto (PROPUESTA).
- **Plantillas empaquetadas**: su índice declara `language = "es" | "en" | "neutral"`; la pestaña Plantillas las filtra por idioma (01 §7.3).

### 14.6 Respaldo y seudo-localización

**Cadena de respaldo** de `CatalogTranslator::text(locale, msg)`:

1. Entrada de `locale`.
2. Entrada en inglés.
3. En desarrollo (`debug_assertions` o feature `pseudo`): la clave visible como `⟦export.done⟧` y un `warn` en el log. En release (`dist`): la entrada en el otro idioma publicado y, si tampoco existe (caso imposible tras ARQ-98, salvo un plugin mal empaquetado), el texto neutro `…` con un `warn`. **En release nunca se muestra una clave.**

**Seudo-localización** (`PseudoTranslator`, feature `pseudo`, solo desarrollo y pruebas; nunca en `dist`):

- Toma el texto inglés, sustituye letras por variantes acentuadas (`Export` → `Ëxpörţ`), conserva los `{parámetros}`, lo encierra entre `⟦ ⟧` y lo alarga con `·` según su longitud: hasta 10 caracteres +100 %, de 11 a 20 +60 %, de 21 a 50 +40 % y más de 50 +30 % (PROPUESTA, cifras habituales en guías de localización).
- Revela texto literal (lo que aparece sin `⟦ ⟧`), concatenaciones (`⟦…⟧⟦…⟧`), truncados y caracteres no cubiertos por las fuentes.
- Se activa con `GAFIRA_LOCALE=pseudo` en builds de desarrollo y en la suite E2E; el perfil `dist` ignora la variable.

### 14.7 Requisitos

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| ARQ-97 | **Sin texto literal visible en la UI.** Todo texto visible y todo nombre accesible de `gafira-ui` sale del `Translator` por clave. | (1) Los widgets de `gafira-ui` solo aceptan `UiText`, cuyo constructor es privado del módulo `text.rs` y solo lo crea el `Translator`; un test `trybuild` muestra que pasar `"Hola"` a esos widgets no compila. (2) `clippy.toml` declara `disallowed_methods` para `egui::Ui::{label, button, heading, small, checkbox, radio, selectable_label, hyperlink_to}`, `egui::RichText::new` y `egui::WidgetText::from` fuera de `text.rs`: 0 avisos. (3) `xtask check-ui-literals` (análisis con `syn`) encuentra 0 literales con letras en llamadas de UI fuera de `assets/i18n/literal-allowlist.toml` (símbolos como `×` o `•` y la marca «Gafira»). |
| ARQ-98 | **Cobertura del 100 % en español e inglés.** | `xtask check-i18n` extrae todas las claves (`msg!`, `Command::label`, `Descriptor`, `ParamSchema` con `ParamDef.label` y `ParamChoice.label`, `options_schema`, `sizes.toml`, glosario) y falla si una clave falta o está vacía en `es` o en `en`, si los dos idiomas usan conjuntos de `{parámetros}` distintos o si un plural no tiene las categorías CLDR que exige su idioma. Una prueba de `gafira-i18n` repite la comprobación sobre las tablas generadas. Es condición de merge (ARQ-80). Las claves huérfanas dan aviso. |
| ARQ-99 | **Cadena de respaldo**: idioma pedido → inglés → clave visible solo en desarrollo. | Pruebas con un catálogo de prueba incompleto: falta en `es` → texto inglés; falta en los dos → `⟦clave⟧` en desarrollo y, compilado con la configuración de `dist`, el texto del otro idioma o `…`; ninguna salida de release coincide con el patrón de clave `^[a-z0-9_]+(\.[a-z0-9_]+)+$`. |
| ARQ-100 | **Regla de D-08 exacta.** | Prueba de tabla con `FakeSystemLocaleSource` y todas las etiquetas de §14.4 (≥ 30): cualquier `es*` → `Es`; `en*` y cualquier otro idioma → `En`; sin dato o `C`/`POSIX` → `En` con `Fallback`; `Es` y `En` manuales ignoran el sistema. En Linux, una prueba en subproceso cubre la precedencia `LANGUAGE` → `LC_ALL` → `LC_MESSAGES` → `LANG`. En cada SO de la CI, `gafira-platform` devuelve una etiqueta no vacía. |
| ARQ-101 | **Cambio de idioma sin reiniciar**, en ≤ 100 ms. | Prueba E2E: cambiar Ajustes → General → Idioma con un proyecto abierto deja todo el texto visible, los menús y los nombres de AccessKit en el idioma nuevo en el siguiente frame, sin cerrar el proyecto ni tocar el historial (`history.len()` igual) ni la selección. `bench_locale_switch`: del cambio al primer frame completo en el idioma nuevo ≤ 100 ms (p99) en el hardware de gama media (§6.6). |
| ARQ-102 | **Los errores de capas inferiores viajan como claves con parámetros.** | Cada enum de error (§7.3) implementa `ToMessage`; una prueba recorre sus variantes (fixtures por variante) y comprueba que la clave existe en los dos catálogos y que la UI muestra el texto del catálogo, nunca el `Display`. Un lint de `gafira-ui` prohíbe `to_string()` y `format!` sobre tipos de error: 0 coincidencias. `UserError` no tiene campos `String` visibles. **Cumplimiento**: hoy `gafira-doc::msg` solo tiene `MessageKey` y ningún error implementa `ToMessage`. Los tipos llegan con el change `engine-contracts`, que adapta `LayoutError` y `PathOpError`. Los demás crates cumplen al crearse, o con el change que los adapte. |
| ARQ-103 | **Seudo-localización en las pruebas de expansión.** | La suite E2E recorre las 8 pantallas (03 DIS-18) con `PseudoTranslator`: 0 truncados sin tooltip, 0 solapes y 0 textos sin `⟦ ⟧`. Capturas golden nocturnas en `es`, `en` y seudo. `cargo tree -e features -p gafira` del perfil `dist` no contiene la feature `pseudo`. |
| ARQ-104 | **`gafira-cli` imprime en inglés por defecto** y respeta otro idioma solo si se pide. | Con `LANG=es_ES.UTF-8`, `gafira-cli export` imprime en inglés; con `--lang es` (o `GAFIRA_CLI_LANG=es`), en español; `--lang system` aplica la regla de D-08 y `--lang app` la preferencia guardada por la app. La salida `--json` contiene claves y valores, no texto traducido, y es idéntica con cualquier `--lang`. Subcomandos, opciones y códigos de salida no cambian con el idioma. |
| ARQ-105 | **Las fuentes cubren los dos idiomas.** | Una prueba de `gafira-theme` lee la tabla `cmap` de Inter y Manrope embebidas y comprueba todos los caracteres de `es.toml`, `en.toml` y `glosario.toml` más ¿ ¡ ñ Ñ á é í ó ú ü Á É Í Ó Ú Ü « » “ ” ‘ ’ — – … × °: 0 ausentes. |
| ARQ-106 | **Formatos por idioma correctos y dentro del presupuesto de tamaño.** | Pruebas golden de `number`, `date`, `relative` y `quantity` para `es`, `es-MX` y `en` con los valores de la tabla de §14.5. `cargo bloat` atribuye a `gafira-i18n`, sus datos ICU4X y los catálogos ≤ 2 MB (PROPUESTA) dentro de los 40 MB del binario (ARQ-83). |
| ARQ-107 | **El dominio no conoce idiomas.** | `check-layers`: `gafira-i18n` solo depende de `gafira-ports`, `gafira-doc` y crates ICU4X; ningún crate de las capas 0–2 depende de `gafira-i18n` ni de `icu_decimal`, `icu_datetime` o `icu_plurals` (`icu_segmenter` en `gafira-text` sigue permitido). Una búsqueda de `Locale` en las capas 0–2 da 0 coincidencias y `gafira-doc::msg` no contiene literales distintos de claves. Mientras no exista `check-layers`, un paso de CI hace las comprobaciones de dependencias y de `Locale` de esta fila sobre las dependencias normales de las capas 0–2. Control negativo: añadir `icu_plurals` a un crate de esas capas hace fallar el paso. |

*Decisión sobre la CLI (ARQ-104)*: la CLI sirve para lotes, CI y pruebas golden. Una salida en inglés fijo hace que los scripts, los logs y las comparaciones no dependan del equipo que los ejecuta, y el inglés es además el idioma de respaldo de D-08. La regla de D-08 se aplica a la interfaz de la app; la CLI la aplica solo con `--lang system`. Confirmación del titular: Q-26.

### 14.8 ADR-09: catálogos TOML propios compilados + ICU4X

**Decisión**: los mensajes se escriben en **catálogos TOML propios** (§14.5) que `xtask` valida y compila a tablas estáticas embebidas; el análisis de etiquetas de idioma, los plurales y los formatos de número y fecha usan **ICU4X** (`icu_locale_core`, `icu_plurals`, `icu_decimal`, `icu_datetime`) con datos compilados solo para `es` y `en`.

| Criterio | A. TOML propio + ICU4X (**elegida**) | B. Fluent (`fluent-bundle`) | C. ICU4X MessageFormat 2 | D. GNU gettext | E. `rust-i18n` |
|---|---|---|---|---|---|
| Licencia | Unicode-3.0, ya en la allowlist (ICU4X ya entra por `icu_segmenter`); `toml` solo en build | Apache-2.0/MIT (VERIFICAR); `self_cell` es `Apache-2.0 OR GPL-2.0-only`, usable bajo Apache-2.0 (VERIFICAR) | Unicode-3.0 | LGPL/GPL: **prohibida** (§9.1) | MIT (VERIFICAR) |
| Plurales | CLDR (`icu_plurals`) | CLDR (`intl_pluralrules`) | CLDR | Sí | Limitados |
| Números y fechas | `icu_decimal`, `icu_datetime` | `NUMBER()` básico; las fechas exigen ICU4X igualmente | Integrados | De libc | No |
| Claves comprobadas antes de ejecutar | Sí (`msg!` + `check-i18n` + tablas generadas) | No (se resuelven en ejecución) | No | No | Sí (macros) |
| Tamaño en el binario | Catálogos ~0,2–0,4 MB + datos ICU4X `es`/`en` < 1 MB (estimación; se mide con ARQ-106) | Parser y *bundles* de Fluent además de ICU4X para fechas | Similar a A | — | Pequeño |
| Cambio de idioma sin reiniciar (≤ 100 ms) | Trivial: `Locale` es `Copy` y las dos tablas están en memoria | Posible con dos *bundles* | Posible | Global (`setlocale`) | Global (`set_locale`), contrario a ARQ-31 |
| Gramática compleja (género, selectores) | Solo plurales y parámetros | Completa | Completa | Parcial | Parcial |
| Madurez | ICU4X 2.x estable; formato propio mínimo | Estable | MessageFormat 2 experimental en ICU4X (VERIFICAR) | Madura | Media |

*Motivos*: con dos idiomas de gramática parecida bastan parámetros y plurales; las specs ya citaban `assets/i18n/es.toml` y `glosario.toml` (03 DIS-01, DIS-41); no se añade ninguna familia de licencias; la comprobación en build sostiene ARQ-97 y ARQ-98; en release no hay parser de catálogos. El cambio de idioma es inmediato porque egui repinta todo el texto en cada frame: solo cambian el `Locale` activo y las cachés de maquetación de texto.

*Descartadas*:

- **B. Fluent**: buena opción y **plan B** si un idioma futuro necesita género o selectores (Q-23). Hoy añade un parser en ejecución, no comprueba claves en build y no resuelve las fechas.
- **C. MessageFormat 2 de ICU4X**: experimental en la versión evaluada.
- **D. gettext**: biblioteca LGPL y herramientas GPL; prohibida (D-08 y §9.1).
- **E. `rust-i18n`**: guarda el idioma en estado global.
- **JSON propio**: equivalente a A, pero sin comentarios y más incómodo para quien traduce.

*Consecuencias*: quien traduce edita TOML; `check-i18n` es obligatorio en CI; un idioma nuevo exige su catálogo completo, sus nombres del glosario 1:1, una variante de `Locale`, sus datos ICU4X y una nueva revisión de expansión (03 §16.4).

---

## Anexo A. Índice de decisiones

| ADR | Decisión |
|---|---|
| ADR-01 | Granularidad de crates por frontera de capa, feature legal, compilación y reutilización |
| ADR-02 | `NodeKind` enum cerrado serializable + `Extension` abierta por registro |
| ADR-03 | Historial híbrido Memento (`Arc` COW propio) + `Command` con `revert`; sin crates MPL |
| ADR-04 | Formato único `.gafira` = ZIP + `manifest.json` versionado + JSON por página + recursos BLAKE3 + `versions/` + medios vinculados |
| ADR-05 | `panic = "unwind"` con `catch_unwind` en trabajos |
| ADR-06 | UI con egui/eframe sobre wgpu; tokens de marca desde datos (normativos en 03 §3); sin las fuentes por defecto de egui |
| ADR-07 | Códecs por clase legal: AV1 + Opus por defecto; H.264/AAC solo vía API del sistema; sin encoders propios con patentes; sin salida HEVC ni MP3; `encumbered-sw` sin código |
| ADR-08 | Licencia por token Ed25519 verificable sin conexión; activación contra el `license-hub` externo; única conexión de red |
| ADR-09 | Idioma (D-08): catálogos TOML propios validados y compilados en build + ICU4X (Unicode-3.0) para etiquetas, plurales, números y fechas; Fluent como plan B; sin gettext |
| ADR-10 | Pantalla de arranque (D-09): imagen `assets/splash_screen.{jpg,png}` validada por `build.rs` e incrustada con `include_bytes!` en `gafira-ui`; sin lectura de disco en ejecución |
