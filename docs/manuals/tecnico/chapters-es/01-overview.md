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
| Capas 0 y 1: `gafira-geom`, `gafira-color`, `gafira-time`, `gafira-anim`, `gafira-doc`, `gafira-edit` | Implementado |
| Capa 2: `gafira-pathops`, `gafira-text`, `gafira-layout`, `gafira-raster`, `gafira-audio`, `gafira-segment` | Implementado |
| Capas 3 y 4: `gafira-ports`, `gafira-render`, `gafira-media`, `gafira-render-cpu` | Implementado; el grafo de render solo emite rellenos y capas, y el backend de CPU devuelve un error para imágenes, máscaras, efectos y transiciones (`crates/gafira-render-cpu/src/backend.rs`) |
| `gafira-testkit` (generador de documentos, arnés de propiedades de comandos) | Implementado (solo soporte de pruebas) |
| `gafira-cli` | Solo `--help` y `--version` |
| `gafira-engine`, `gafira-format`, `gafira-ui`, códecs, licencias, empaquetado | **Planificado** (el capítulo de arquitectura tiene el inventario completo) |

La tabla agrupa los crates por capa; el capítulo de arquitectura lista el inventario completo de crates.

Versión del workspace: `0.1.0-beta.2`. Edición 2024, toolchain `1.93.1` fijado en `rust-toolchain.toml`.

## Mapa del repositorio

| Ruta | Contenido |
|---|---|
| `crates/` | Diecisiete crates de biblioteca en las capas 0 a 4, incluido el crate de soporte de pruebas `gafira-testkit`; el capítulo de arquitectura tiene el inventario completo |
| `apps/gafira-cli/` | Binario de línea de comandos |
| `specs/` | Specs numeradas (activas, `backlog/`, `done/`; índice en `specs/README.md`) y los documentos de producto y arquitectura; `00-decisiones.md` recoge las decisiones del titular |
| `docs/manuals/` | Manual de marca (`marca/`) y este manual técnico (`tecnico/`) |
| `specs/done/legacy/` | Registro del flujo anterior: specs por capacidad, carpetas de change y changes cerrados |
| `.agents/agents/` | El agente `manual-writer` (`.claude/agents` es un enlace simbólico) |
| `.github/workflows/ci.yml` | CI: pruebas, comprobaciones de política, `cargo-deny` |
| `deny.toml`, `.cargo/mutants.toml` | Política de licencias y fuentes; ajustes de pruebas de mutación (el perfil `mutants` está en `Cargo.toml`) |
| `docs/adr/` | Registros de decisiones de arquitectura, incluidas las del flujo (ADR 0018 flujo simple de specs, que reemplaza a ADR 0001, 0002, 0004 y 0011) |

## Orden de lectura

1. Arquitectura y capas, para saber dónde puede vivir cada código.
2. Los capítulos de crates, de abajo hacia arriba: geometría, color, documento, edición.
3. Herramientas y metodología, antes de enviar un cambio. Las specs se ejecutan por el flujo de `AGENTS.md` y el skill `gafira-specs` (ADR 0018); el capítulo de metodología todavía describe el flujo anterior.
