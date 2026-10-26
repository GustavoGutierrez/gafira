# Arquitectura y capas

## Arquitectura hexagonal

Gafira sigue el patrón de puertos y adaptadores (spec 02, sección 1.2). Las dependencias apuntan hacia dentro:

| Capa | Papel | Regla |
|---|---|---|
| Dominio (capas 0 y 1) | Modelo de documento, comandos, geometría, color, tiempo | Funciones puras, sin I/O |
| Motores de cómputo (capa 2) | Operaciones de rutas, maquetación de texto, raster, audio, segmentación | Puros: reciben bytes o buffers y nunca abren archivos ni hilos |
| Aplicación (capa 3) | Sesión, servicios, grafo de render, puertos (traits) | Habla con el exterior solo mediante puertos |
| Adaptadores (capa 4) | Formatos de archivo, códecs, backends de render, UI, licencias, plataforma | Implementan puertos |
| Binarios | `gafira`, `gafira-cli` | Raíz de composición: conectan las implementaciones |

ARQ-01 fija la regla de dependencias: un crate depende solo de crates de su misma capa o de capas inferiores, y el dominio nunca depende de motores, aplicación ni adaptadores.

## Crates del workspace hoy

| Crate | Capa | Dependencias internas | Propósito |
|---|---|---|---|
| `gafira-geom` | 0 | ninguna | Longitudes con unidades y conversión por DPI, transformaciones 2D, tamaños |
| `gafira-color` | 0 | ninguna | Espacios de color, tipos RGBA `f32`, conversiones analíticas |
| `gafira-time` | 0 | ninguna | Tiempo exacto en flicks, frecuencias de fotogramas y de muestreo, rangos de tiempo, timecode |
| `gafira-doc` | 1 | `gafira-geom`, `gafira-color` | Modelo de documento, JSON, validación, capacidad de edición |
| `gafira-edit` | 1 | `gafira-geom`, `gafira-color`, `gafira-doc` | Contrato `Command`, `Transaction`, `DirtySet`, `History`, comandos del núcleo (`SetProperty`, `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent`, `InsertPage`, `RemovePage`) |
| `gafira-pathops` | 2 | `gafira-geom` | Operaciones booleanas de trazados, contorno de trazo, simplificación, detección de impacto |
| `gafira-testkit` | soporte | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit` | Soporte de pruebas: generador acotado de documentos, generadores de corrupción, arnés `prop_command`. Solo se usa como dev-dependency |
| `gafira-cli` (`apps/`) | binario | ninguna | `--help`, `--version` |

Dependencias externas declaradas en el workspace: `kurbo` (geometría, con `serde`), `serde`, `serde_json` (`float_roundtrip`), `linesweeper` (declarada por `gafira-pathops`) y, solo para pruebas, `proptest` y `trybuild`.

`gafira-testkit` es dev-dependency de `gafira-doc` y de `gafira-edit` mientras depende de ambos. Cargo lo acepta porque una arista de dev-dependency no es una arista de compilación, pero el testkit debe usarse solo desde pruebas de integración (`tests/`), donde cada crate tiene una única copia de cada tipo; dentro de las pruebas unitarias de un crate los tipos serían dos copias distintas.

## Crates gafira-time y gafira-pathops

Ambos son crates puros con un `clippy.toml` y una fila V5. Fuentes: `openspec/specs/time-model/`, `openspec/specs/timecode/`, `openspec/specs/path-operations/` y el código de cada crate.

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

Capa 2, depende de `gafira-geom`, `kurbo` y `linesweeper`. Las coordenadas son `f64`; cada operación recibe una tolerancia, con `DEFAULT_TOLERANCE` de `1e-3` por defecto.

| Elemento | Contrato |
|---|---|
| `Path`, `FillRule` | Un trazado de Bézier con regla de relleno par-impar o no cero |
| `union`, `intersection`, `difference`, `xor`, `boolean_op` (`BooleanOp`) | Operaciones booleanas entre dos trazados |
| `stroke_outline` (`StrokeStyle`) | El contorno de un trazado con trazo |
| `simplify` | Simplificación del trazado dentro de una tolerancia |
| `contains` (`Containment`), `distance_to_perimeter` | Detección geométrica de impacto |
| `PathOpError` | Entrada no finita, tolerancia demasiado fina para la entrada y otras entradas inválidas |

## Crates planificados

La especificación enumera más crates que aún no están en el repositorio. Están **Planificados** y se nombran aquí solo para ubicar a los existentes:

- Capas 0 y 1: `gafira-anim`.
- Capa 2 (motores): `gafira-text`, `gafira-layout`, `gafira-raster`, `gafira-audio`, `gafira-segment`.
- Capa 3 (aplicación): `gafira-ports`, `gafira-render`, `gafira-media`, `gafira-engine`.
- Capa 4 (adaptadores): `gafira-format`, `gafira-assets`, `gafira-io-*`, `gafira-mux`, `gafira-codec-*`, `gafira-render-cpu`, `gafira-render-gpu`, `gafira-platform`, `gafira-i18n`, `gafira-theme`, `gafira-license`, `gafira-ui`, `gafira-captions`, `gafira-plugin`.
- Soporte: `xtask`.

La tabla autoritativa, con la responsabilidad y las prohibiciones de cada crate, es la sección 2.1 de `docs/specs/02-arquitectura-software.md`.

Un crate nuevo debe cumplir un criterio de ADR-01: una frontera de capa, una feature opcional o exposición legal, tiempo de compilación incremental, o reutilización por varios consumidores. La mera organización interna es un módulo, no un crate.

## Reglas de dependencias permitidas (V5)

La CI aplica la tabla de capas en el job `policy`, paso "Domain crates respect the layer table (V5)". El paso contiene una tabla `crate  allowed-internal-deps` y, para cada fila, lee `cargo tree -p <crate> -e normal,build --target all`:

| Crate | Dependencias internas permitidas |
|---|---|
| `gafira-geom` | ninguna |
| `gafira-color` | ninguna |
| `gafira-time` | ninguna |
| `gafira-doc` | `gafira-geom`, `gafira-color` |
| `gafira-edit` | `gafira-geom`, `gafira-color`, `gafira-doc` |
| `gafira-testkit` | `gafira-geom`, `gafira-color`, `gafira-doc`, `gafira-edit` |
| `gafira-pathops` | `gafira-geom` |

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
| ARQ-30 provisional, capacidad de edición | V13: `EditToken` solo puede nombrarse en `gafira-doc`, `gafira-edit` y `gafira-testkit`, y `gafira-edit` no debe reexportarlo. Los fixtures compile-fail de `trybuild` en `gafira-edit` muestran que un crate externo no puede mutar un `Document` sin pasar por `History` |
| ARQ-81, cada comando del núcleo tiene una prueba de propiedades | El paso de CI V14 (sustituto de `xtask`) comprueba que cada tipo que implementa `Command` tiene una prueba en `crates/gafira-edit/tests/prop_commands.rs` |

## La capacidad de edición dentro de las capas

Leer un `Document` es libre; mutarlo exige un `EditToken`. Así todo cambio pasa por comandos, que viven en `gafira-edit`. Los capítulos de documento y de edición describen el mecanismo.
