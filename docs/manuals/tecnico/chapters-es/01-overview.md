# Visión general

## Propósito de este manual

Este manual describe cómo está construido Gafira: sus crates, las reglas entre ellos, la API pública de los crates de dominio, el formato de documento, las puertas de calidad y el método de desarrollo. Documenta lo que existe hoy en el repositorio. Lo que está especificado pero aún no implementado se marca como **Planificado**.

La intención de producto (qué y por qué) vive en `specs/`; este manual es su contraparte técnica (cómo está construido). Cuando este manual y una especificación discrepan, prevalecen la especificación y las decisiones del titular en `specs/00-decisiones.md`, y este manual se corrige.

## Qué es Gafira

Gafira es un editor de escritorio local para diseño gráfico, imagen y video, escrito en Rust. El workspace es un workspace de Cargo con crates `gafira-*` y binarios en `apps/`. Sus plataformas objetivo son Windows x86_64, Linux x86_64 y macOS aarch64 (ARQ-03, D-05). El producto es propietario; toda dependencia de terceros debe tener una licencia permisiva (`deny.toml`).

## Estado actual

| Área | Estado |
|---|---|
| Workspace, CI, puerta de licencias | Implementado |
| `gafira-geom` (longitudes, transformaciones) | Implementado |
| `gafira-color` (espacios de color, conversiones) | Implementado |
| `gafira-doc` (modelo de documento, JSON, validación, capacidad de edición) | Implementado |
| `gafira-time` (flicks, frecuencias de fotogramas y de muestreo, rangos de tiempo, timecode) | Implementado |
| `gafira-edit` (`Command`, `Transaction`, `DirtySet`, `History`, `SetProperty`, `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent`, `InsertPage`, `RemovePage`) | Implementado |
| `gafira-pathops` (booleanas de trazados, contorno de trazo, simplificación, detección de impacto) | Implementado |
| `gafira-testkit` (generador de documentos, arnés de propiedades de comandos) | Implementado (solo soporte de pruebas) |
| `gafira-cli` | Solo `--help` y `--version` |
| Formato, render, engine, UI, códecs, licencias | **Planificado** (véase el capítulo de arquitectura) |

Versión del workspace: `0.1.0-beta.1`. Edición 2024, toolchain `1.93.1` fijado en `rust-toolchain.toml`.

## Mapa del repositorio

| Ruta | Contenido |
|---|---|
| `crates/` | Crates de biblioteca (`gafira-geom`, `gafira-color`, `gafira-time`, `gafira-doc`, `gafira-edit`, `gafira-pathops`, `gafira-testkit`) |
| `apps/gafira-cli/` | Binario de línea de comandos |
| `specs/` | Especificaciones de producto y arquitectura; `00-decisiones.md` recoge las decisiones del titular |
| `docs/manuals/` | Manual de marca (`marca/`) y este manual técnico (`tecnico/`) |
| `specs/capabilities/` | Specs principales por capacidad, fusionadas al archivar; los changes de carril rápido no las editan |
| `specs/changes/` | Carpetas de change activas, solo para el trabajo que las necesita; las cerradas pasan a `specs/history/` |
| `.odd/state.json` | Los resultados (outcomes) activos de ODD lite (véase el capítulo de metodología) |
| `.agents/agents/` | Agentes del flujo de desarrollo (`.claude/agents` es un enlace simbólico) |
| `.github/workflows/ci.yml` | CI: pruebas, comprobaciones de política, `cargo-deny` |
| `deny.toml`, `.cargo/mutants.toml` | Política de licencias y fuentes; ajustes de pruebas de mutación (el perfil `mutants` está en `Cargo.toml`) |
| `docs/adr/` | Registros de decisiones de arquitectura, incluidas las del flujo (ADR 0001 modos y packs, ADR 0002 carril rápido, ADR 0011 ODD lite, aceptado) |

## Orden de lectura

1. Arquitectura y capas, para saber dónde puede vivir cada código.
2. Los capítulos de crates, de abajo hacia arriba: geometría, color, documento, edición.
3. Herramientas y metodología, antes de enviar un cambio. Las specs de producto se ejecutan por el flujo que describe el capítulo de metodología (ODD lite y el carril rápido).
