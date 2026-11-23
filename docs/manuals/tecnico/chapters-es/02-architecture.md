# Arquitectura y capas

## Arquitectura hexagonal

Gafira sigue el patrón de puertos y adaptadores (spec 02, sección 1.2). Las dependencias apuntan hacia dentro:

| Capa | Papel | Regla |
|---|---|---|
| Dominio (capas 0 y 1) | Modelo de documento, comandos, geometría, color, tiempo | Funciones puras, sin I/O |
| Motores de cómputo (capa 2) | Operaciones de rutas, maquetación de texto, raster, audio, segmentación | Puros: reciben bytes o buffers y nunca abren archivos ni hilos |
| Aplicación (capa 3) | Grafo de render, puertos (traits), tubería de medios; `Session` y los servicios Facade están **Planificados** (`gafira-engine`) | Habla con el exterior solo mediante puertos |
| Adaptadores (capa 4) | Formatos de archivo, códecs, backends de render, UI, licencias, plataforma | Implementan puertos |
| Binarios | `gafira-cli`; la aplicación de escritorio `gafira` está **Planificada** | Raíz de composición: conectan las implementaciones |

ARQ-01 fija la regla de dependencias: un crate depende solo de crates de su misma capa o de capas inferiores, y el dominio nunca depende de motores, aplicación ni adaptadores.

## Inventario de crates

El repositorio contiene hoy diecisiete crates de biblioteca en `crates/` y un binario en
`apps/`. La tabla incluye además los veinte crates de la sección 2.1 que solo existen en la
especificación, para que el lector vea cada capa completa. Un crate está **Implementado**
cuando existe en el repositorio y **Planificado** cuando no existe; una fila Implementada
añade una nota de cobertura solo cuando falta parte de la responsabilidad que fija la spec.

| Crate | Capa | Estado | Cubre |
|---|---|---|---|
| `gafira-geom` | 0 | Implementado | Longitudes con unidades y conversión por DPI, transformaciones 2D, `IRect`, tamaños |
| `gafira-color` | 0 | Implementado | Espacios de color y conversiones analíticas; degradados, perfiles ICC y muestras de paleta **Planificados** |
| `gafira-time` | 0 | Implementado | Flicks, frecuencias de fotogramas y de muestreo, rangos de tiempo, timecode |
| `gafira-anim` | 1 | Implementado | `Animatable<T>`, keyframes, curvas hold, lineal, ease y Bézier cúbica |
| `gafira-doc` | 1 | Implementado | Modelo de página, nodo, línea de tiempo y activos, JSON canónico, validación, capacidad de edición; `Project`, estilos e identidad visual **Planificados** |
| `gafira-edit` | 1 | Implementado | `Command`, `Transaction`, `DirtySet`, `History`, los siete comandos del núcleo; `Selection` **Planificado** |
| `gafira-pathops` | 2 | Implementado | Booleanas de trazados, contorno de trazo, simplificación, detección de impacto |
| `gafira-text` | 2 | Implementado | Shaping, bidi, saltos de línea, maquetación de cajas y columnas, texto curvado, contornos |
| `gafira-layout` | 2 | Implementado | Guías, rejillas de columnas, snap, alineación, distribución, sangrado y áreas seguras; reflujo al cambiar de tamaño **Planificado** |
| `gafira-raster` | 2 | Implementado | `ImageBuf`, `ImageView`, máscaras alfa, tiles, mipmaps, mezcla, efectos y transiciones |
| `gafira-audio` | 2 | Implementado | Cadena de `AudioProcessor`, ganancia, fundidos, EQ de 3 bandas, pan, mezcla, remuestreo, sonoridad EBU R128 |
| `gafira-segment` | 2 | Implementado | `Segmenter` y la ruta clásica: GrabCut con GMM, max-flow, SLIC, refinado guiado |
| `gafira-ports` | 3 | Implementado | Traits de puerto, `Registry<T>` y un enum de error por familia (capítulo 12) |
| `gafira-render` | 3 | Implementado | Grafo de render, cachés por contenido y por tiles, planificador, bucle de fotogramas; emite rellenos y capas, las imágenes, máscaras y efectos están **Planificados** |
| `gafira-media` | 3 | Implementado | Índice de keyframes, caché de fotogramas, proxies, reloj de reproducción y sincronía A/V |
| `gafira-engine` | 3 | Planificado | `Session`, servicios Facade, bus de eventos, cola de exportación, autoguardado, gobernador de memoria |
| `gafira-render-cpu` | 4 | Implementado | `CpuBackend`: ejecuta capas y rellenos, las imágenes, máscaras, efectos y transiciones devuelven un error |
| `gafira-format` | 4 | Planificado | Lectura y escritura de `.gaf`, `versions/`, medios vinculados |
| `gafira-assets` | 4 | Planificado | Almacén direccionado por contenido e índice de biblioteca local |
| `gafira-io-image` | 4 | Planificado | Importación y exportación PNG, JPEG, WebP y GIF |
| `gafira-io-svg` | 4 | Planificado | Importación y exportación SVG |
| `gafira-io-pdf` | 4 | Planificado | Exportación PDF con los preajustes Digital e Impresión |
| `gafira-io-pptx` | 4 | Planificado | Exportación PPTX, una diapositiva por página |
| `gafira-mux` | 4 | Planificado | Contenedores MP4, WebM/Matroska, Ogg y WAV en Rust |
| `gafira-codec-free` | 4 | Planificado | AV1 y Opus |
| `gafira-codec-os` | 4 | Planificado | Códecs del sistema operativo por FFI |
| `gafira-render-gpu` | 4 | Planificado | `RenderBackend` de GPU sobre wgpu |
| `gafira-platform` | 4 | Planificado | Rutas de datos, bloqueos de archivo, renombrado atómico, fuentes del sistema |
| `gafira-i18n` | 4 | Planificado | Catálogos de mensajes en español e inglés |
| `gafira-theme` | 4 | Planificado | Tokens de diseño cargados desde datos |
| `gafira-license` | 4 | Planificado | Licencia y activación, verificación sin conexión |
| `gafira-ui` | 4 | Planificado | Shell egui/eframe, paneles, lienzo, timeline, herramientas |
| `gafira-captions` | 4 | Planificado | Lectura y escritura de SRT y WebVTT |
| `gafira-plugin` | 4 | Planificado | `PluginHost` con wasmi, manifiesto de extensión |
| `gafira-testkit` | soporte | Implementado | Fixtures, generadores `proptest`, comparación golden, dobles de puertos; solo dev-dependency |
| `xtask` | soporte | Planificado | Comprobación de capas y empaquetado, hoy un sustituto en la CI |
| `gafira-cli` (`apps/`) | binario | Implementado | `--help` y `--version`; `export` por lotes **Planificado** |
| `gafira` (`apps/`) | binario | Planificado | Aplicación de escritorio: composición, registros e inyección |

Dependencias externas declaradas en `[workspace.dependencies]` (`Cargo.toml`): `blake3`,
`kurbo` (geometría, con `serde`), `serde`, `serde_json` (`float_roundtrip`), `png`, `tiny-skia`
(en `gafira-render-cpu`), `parley` y `skrifa` (en `gafira-text`), `rayon` y, solo para
pruebas, `proptest`, `criterion`, `trybuild` y `assert_no_alloc`. `linesweeper` no está en la
tabla del workspace: lo declara `gafira-pathops` directamente.

`gafira-testkit` es dev-dependency de ocho crates —`gafira-audio`, `gafira-doc`,
`gafira-edit`, `gafira-media`, `gafira-raster`, `gafira-render`, `gafira-segment` y
`gafira-text`— mientras depende de `gafira-geom`, `gafira-color`, `gafira-doc` y
`gafira-edit`; su feature opcional `ports` añade dobles de los traits de puerto, que es la
única arista que la CI comprueba aparte. Cargo acepta el ciclo porque una arista de
dev-dependency no es una arista de compilación, pero el testkit debe usarse solo desde
pruebas de integración (`tests/`), donde cada crate tiene una única copia de cada tipo;
dentro de las pruebas unitarias de un crate los tipos serían dos copias distintas.

## Crates gafira-time y gafira-pathops

Ambos son crates puros con un `clippy.toml` y una fila V5. Fuentes: `specs/done/legacy/capabilities/time-model/`, `specs/done/legacy/capabilities/timecode/`, `specs/done/legacy/capabilities/path-operations/` y el código de cada crate.

### gafira-time

Capa 0, sin dependencias internas (solo `serde`). El tiempo es un número entero con signo de flicks, 1/705 600 000 s. El crate no lee ningún reloj.

| Elemento | Contrato |
|---|---|
| `Flicks`, `FLICKS_PER_SECOND`, `FLICKS_PER_MILLISECOND` | Tiempo exacto; la construcción desde segundos y milisegundos y la aritmética son comprobadas y devuelven `TimeError` ante un desbordamiento |
| `FrameRate` | Racional exacto (`num`, `den`) de al menos un fotograma por segundo; convierte fotogramas en flicks y redondea un tiempo hacia abajo al fotograma que lo contiene |
| `SampleRate` | Frecuencia de audio en hercios; la alineación de muestras con los flicks es exacta |
| `TimeRange` | Rango semiabierto con duración no negativa; `contains`, `intersects`, `intersection` |
| `Timecode`, `TimecodeFormat`, `TimecodeMode` | Etiqueta de fotograma validada sobre flicks; sin descarte de fotogramas y con descarte solo para 29,97 y 59,94; `parse` y `format` son canónicos |

### gafira-pathops

Capa 2, depende de `gafira-geom`, `gafira-doc`, `kurbo` y `linesweeper`. Las coordenadas son `f64`; cada operación recibe una tolerancia, con `DEFAULT_TOLERANCE` de `1e-3` por defecto.

| Elemento | Contrato |
|---|---|
| `Path`, `FillRule` | Un trazado de Bézier con regla de relleno par-impar o no cero |
| `union`, `intersection`, `difference`, `xor`, `boolean_op` (`BooleanOp`) | Operaciones booleanas entre dos trazados |
| `stroke_outline` (`StrokeStyle`) | El contorno de un trazado con trazo |
| `simplify` | Simplificación del trazado dentro de una tolerancia |
| `contains` (`Containment`), `distance_to_perimeter` | Detección geométrica de impacto |
| `PathOpError` | Entrada no finita, tolerancia demasiado fina para la entrada y otras entradas inválidas |

## La tabla de crates de la spec

La sección 2.1 de `specs/02-arquitectura-software.md` es la tabla autoritativa: fija la
responsabilidad y las prohibiciones de cada crate. El inventario anterior repite solo la capa
y una nota breve.

Un crate nuevo debe cumplir un criterio de ADR-01: una frontera de capa, una feature opcional o exposición legal, tiempo de compilación incremental, o reutilización por varios consumidores. La mera organización interna es un módulo, no un crate.

## Reglas de dependencias permitidas (V5)

La CI aplica la tabla de capas en el job `policy`, paso "Domain crates respect the layer table (V5)". El paso contiene una tabla `crate  allowed-internal-deps` y, para cada fila, lee `cargo tree -p <crate> -e normal,build --target all`:

| Crate | Dependencias internas permitidas |
|---|---|
| `gafira-geom` | ninguna |
| `gafira-color` | ninguna |
| `gafira-time` | ninguna |
| `gafira-doc` | `gafira-geom`, `gafira-color`, `gafira-time` |
| `gafira-edit` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-testkit` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit`, `gafira-time` |
| `gafira-pathops` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-layout` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-anim` | `gafira-time`, `gafira-geom`, `gafira-color` |
| `gafira-text` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-pathops`, `gafira-time` |
| `gafira-raster` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-audio` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time` |
| `gafira-segment` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-raster`, `gafira-time` |
| `gafira-ports` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-time`, `gafira-raster` |
| `gafira-render` | `gafira-ports`, `gafira-doc`, `gafira-geom`, `gafira-color`, `gafira-raster`, `gafira-time`, `gafira-anim` |
| `gafira-media` | `gafira-ports`, `gafira-doc`, `gafira-raster`, `gafira-time`, `gafira-geom`, `gafira-color` |
| `gafira-render-cpu` | `gafira-ports`, `gafira-doc`, `gafira-geom`, `gafira-color`, `gafira-raster`, `gafira-time` |

El paso falla cuando:

- un crate depende, directa o transitivamente por aristas normales o de build, de un crate `gafira-*` que no está en su fila;
- el árbol contiene un crate externo prohibido: `egui`, `eframe`, `winit`, `wgpu`, `tokio`, `mio`, `socket2`, `reqwest` o `ureq`;
- `cargo tree` no devuelve nada para un crate listado;
- un crate tiene su propio `clippy.toml` (la marca de un crate de dominio) pero ninguna fila en la tabla.

Añadir un crate de dominio implica añadir su fila y una copia de `clippy.toml`; el paso en sí no cambia. `gafira-testkit` tiene fila pero no `clippy.toml`, así que no es un crate de dominio para V12; su fila mantiene acotadas sus aristas internas.

## Otras reglas aplicadas

| Regla | Mecanismo |
|---|---|
| ARQ-02, sin I/O en crates de dominio | Un `clippy.toml` por crate de dominio con `disallowed-methods` y `disallowed-types` (`std::fs`, `std::net`, hilos, relojes, `std::env`, procesos) |
| ARQ-10, orden determinista | `HashMap` y `HashSet` son tipos prohibidos en crates de dominio; se usan `BTreeMap` y `BTreeSet` |
| V12, copias de política sincronizadas | El `clippy.toml` de `gafira-geom` es el canónico; las demás copias deben ser idénticas byte a byte (`diff`) |
| ARQ-07, CLI sin UI | `cargo tree -p gafira-cli --no-default-features` no debe contener `gafira-ui`, `eframe`, `winit` ni `wgpu` |
| Sin `unsafe` | `unsafe_code = "deny"` en los lints del workspace, más `#![forbid(unsafe_code)]` en cada `lib.rs` y `main.rs`, verificado por la CI |
| ARQ-107, las capas 0 a 2 no conocen idiomas | Un paso de `policy` busca tipos de idioma en esos crates y exime a `gafira-ports`, que define `Locale` a propósito (`.github/workflows/ci.yml:156`) |
| ARQ-30 provisional, capacidad de edición | V13: `EditToken` solo puede nombrarse en `gafira-doc`, `gafira-edit` y `gafira-testkit`, y `gafira-edit` no debe reexportarlo. Los fixtures compile-fail de `trybuild` en `gafira-edit` muestran que un crate externo no puede mutar un `Document` sin pasar por `History` |
| ARQ-81, cada comando del núcleo tiene una prueba de propiedades | El paso de CI V14 (sustituto de `xtask`) comprueba que cada tipo que implementa `Command` tiene una prueba en `crates/gafira-edit/tests/prop_commands.rs` |

## La capacidad de edición dentro de las capas

Leer un `Document` es libre; mutarlo exige un `EditToken`. Así todo cambio pasa por comandos, que viven en `gafira-edit`. Los capítulos de documento y de edición describen el mecanismo.
