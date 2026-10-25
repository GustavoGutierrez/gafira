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
| `gafira-doc` | 1 | `gafira-geom`, `gafira-color` | Modelo de documento, JSON, validación, capacidad de edición |
| `gafira-edit` | 1 | `gafira-geom`, `gafira-color`, `gafira-doc` | Contrato `Command`, `Transaction`, `DirtySet` |
| `gafira-cli` (`apps/`) | binario | ninguna | `--help`, `--version` |

Dependencias externas declaradas en el workspace: `kurbo` (geometría, con `serde`), `serde`, `serde_json` (`float_roundtrip`) y `proptest` para pruebas.

## Crates planificados

La especificación enumera más crates que aún no están en el repositorio. Están **Planificados** y se nombran aquí solo para ubicar a los existentes:

- Capas 0 y 1: `gafira-time`, `gafira-anim`.
- Capa 2 (motores): `gafira-pathops`, `gafira-text`, `gafira-layout`, `gafira-raster`, `gafira-audio`, `gafira-segment`.
- Capa 3 (aplicación): `gafira-ports`, `gafira-render`, `gafira-media`, `gafira-engine`.
- Capa 4 (adaptadores): `gafira-format`, `gafira-assets`, `gafira-io-*`, `gafira-mux`, `gafira-codec-*`, `gafira-render-cpu`, `gafira-render-gpu`, `gafira-platform`, `gafira-i18n`, `gafira-theme`, `gafira-license`, `gafira-ui`, `gafira-captions`, `gafira-plugin`.
- Soporte: `gafira-testkit`, `xtask`.

La tabla autoritativa, con la responsabilidad y las prohibiciones de cada crate, es la sección 2.1 de `docs/specs/02-arquitectura-software.md`.

Un crate nuevo debe cumplir un criterio de ADR-01: una frontera de capa, una feature opcional o exposición legal, tiempo de compilación incremental, o reutilización por varios consumidores. La mera organización interna es un módulo, no un crate.

## Reglas de dependencias permitidas (V5)

La CI aplica la tabla de capas en el job `policy`, paso "Domain crates respect the layer table (V5)". El paso contiene una tabla `crate  allowed-internal-deps` y, para cada fila, lee `cargo tree -p <crate> -e normal,build --target all`:

| Crate | Dependencias internas permitidas |
|---|---|
| `gafira-geom` | ninguna |
| `gafira-color` | ninguna |
| `gafira-doc` | `gafira-geom`, `gafira-color` |
| `gafira-edit` | `gafira-geom`, `gafira-color`, `gafira-doc` |

El paso falla cuando:

- un crate depende, directa o transitivamente por aristas normales o de build, de un crate `gafira-*` que no está en su fila;
- el árbol contiene un crate externo prohibido: `egui`, `eframe`, `winit`, `wgpu`, `tokio`, `mio`, `socket2`, `reqwest` o `ureq`;
- `cargo tree` no devuelve nada para un crate listado;
- un crate tiene su propio `clippy.toml` (la marca de un crate de dominio) pero ninguna fila en la tabla.

Añadir un crate de dominio implica añadir su fila y una copia de `clippy.toml`; el paso en sí no cambia.

## Otras reglas aplicadas

| Regla | Mecanismo |
|---|---|
| ARQ-02, sin I/O en crates de dominio | Un `clippy.toml` por crate de dominio con `disallowed-methods` y `disallowed-types` (`std::fs`, `std::net`, hilos, relojes, `std::env`, procesos) |
| ARQ-10, orden determinista | `HashMap` y `HashSet` son tipos prohibidos en crates de dominio; se usan `BTreeMap` y `BTreeSet` |
| V12, copias de política sincronizadas | El `clippy.toml` de `gafira-geom` es el canónico; las demás copias deben ser idénticas byte a byte (`diff`) |
| ARQ-07, CLI sin UI | `cargo tree -p gafira-cli --no-default-features` no debe contener `gafira-ui`, `eframe`, `winit` ni `wgpu` |
| Sin `unsafe` | `unsafe_code = "deny"` en los lints del workspace, más `#![forbid(unsafe_code)]` en cada `lib.rs` y `main.rs`, verificado por la CI |
| ARQ-30 provisional, capacidad de edición | V13: `EditToken` solo puede nombrarse en `gafira-doc`, `gafira-edit` y `gafira-testkit`, y `gafira-edit` no debe reexportarlo |

## La capacidad de edición dentro de las capas

Leer un `Document` es libre; mutarlo exige un `EditToken`. Así todo cambio pasa por comandos, que viven en `gafira-edit`. Los capítulos de documento y de edición describen el mecanismo.
