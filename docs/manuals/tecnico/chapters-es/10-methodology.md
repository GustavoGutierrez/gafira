# Metodología de desarrollo

## Estado

| Área | Estado | Fuente |
|---|---|---|
| Flujo simple de specs: specs numeradas, banner de estado, el agente principal implementa | **Implementado** | `docs/adr/0018-simple-spec-flow.md`, `AGENTS.md`, skill `gafira-specs` |
| TDD para la lógica crítica, pruebas de integración para el resto | **Implementado** | `AGENTS.md` «Rules» (Testing), skill `gafira-specs` |
| Fuente única de agentes y skills, mantenida por `tools/sync_agents.py` | **Implementado** | `AGENTS.md` «Agent and skill sync», `.agents/README.md` |
| Flujo multiagente anterior (SDD ligero con ODD lite) | **Retirado** por el ADR 0018; el registro se conserva en solo lectura | `specs/done/legacy/` |

## Por qué el flujo es simple

El flujo anterior pasaba cada solicitud por un coordinador y hasta ocho agentes del repositorio, con niveles de riesgo, modos de flujo, Slice cards, un archivo de estado de outcomes y rondas de verify acotadas. Encontró defectos reales, pero su ceremonia costaba a menudo más que el trabajo. Mantener coherentes los agentes, sus tres réplicas por runtime y el archivo de estado era un trabajo en sí mismo.

El ADR 0018 lo reemplaza. El agente principal implementa las specs directamente, con pruebas y las puertas de `AGENTS.md`. Los controles que quedan son las puertas, los pasos de política de la CI y la regla de prueba primero para la lógica crítica. Desaparecen las rondas de verificación independiente y las pruebas de mutación obligatorias. Se descartaron dos alternativas: reducir los agentes pero mantener las reglas de enrutamiento, porque la mayor parte del coste venía de esas reglas, y borrar los registros antiguos, porque los comentarios del código y los ADR todavía los citan.

## Specs y carpetas

El banner de estado de la spec y `specs/README.md` son el estado; git es el historial.

| Lugar | Contenido |
|---|---|
| `specs/backlog/` | Specs no iniciadas: una idea, una pregunta abierta, una decisión pendiente |
| `specs/` | Specs activas, en progreso o en ejecución estable; también los documentos de referencia de producto |
| `specs/done/` | Specs completadas y verificadas; no se reabren en el sitio |
| `specs/README.md` | El índice: specs activas, cerradas y en backlog, y el siguiente ID de spec libre |
| `specs/00-..07-*.md` | Documentos de referencia de producto (decisiones del titular, producto, arquitectura, funcionalidades, deuda abierta), no specs de ciclo de vida |
| `specs/done/legacy/` | Registro del flujo anterior; solo lectura |

Las decisiones del titular en `specs/00-decisiones.md` (`D-nn`) prevalecen sobre los demás documentos y no se reabren sin el titular. Una spec nueva cita los IDs de producto que implementa (`ARQ-12`, `FD-03`, `D-08`) y sus secciones.

## ID de spec y banner de estado

Un ID de spec tiene tres dígitos, con ceros a la izquierda, y se asigna al crear la spec: `008`, `009`, etc. Los IDs menores que `008` pertenecen a los documentos de referencia de producto. El nombre del archivo es `NNN-kebab-case.md`, el título empieza por el ID, y el ID no cambia cuando la spec se mueve entre carpetas (`git mv`, nunca una copia). Por ejemplo, la spec `009` (el `SettingsService` del engine) está en `specs/done/009-engine-settings-service.md`.

Toda spec empieza con un banner de estado justo debajo del título:

```markdown
> **Estado:** backlog | in-progress | done — YYYY-MM-DD — evidence or blocker.
```

El banner acompaña a la carpeta: `backlog` en `specs/backlog/`, `in-progress` en `specs/`, `done` en `specs/done/`. Una spec bloqueada se queda en `specs/` con su bloqueo, o vuelve a `specs/backlog/` si el trabajo se detiene. Un alcance nuevo para una spec cerrada recibe un ID de spec nuevo. El cuerpo de las specs se redacta en español.

## Definition of Ready

Una spec sale de `specs/backlog/` solo cuando tiene todo lo siguiente:

- Problema y resultado esperado.
- Alcance y no objetivos.
- Dependencias y supuestos (crates, capas, ADR, decisiones del titular).
- Requisitos funcionales con IDs, propios o los IDs de producto que implementa.
- Criterios de aceptación comprobables y la evidencia esperada para darla por terminada.
- Casos de TDD para su lógica crítica: las pruebas que deben fallar primero.
- Riesgos y decisiones abiertas.

## Ciclo de vida

1. Capturar: crear la spec en `specs/backlog/` con el siguiente ID de spec libre y el banner de estado, y añadirla a `specs/README.md`.
2. Refinar: completar la Definition of Ready. La spec sigue en backlog hasta que el titular decide empezar.
3. Empezar: crear una rama desde `develop`, mover la spec a `specs/` con `git mv`, fijar `in-progress` y escribir primero los casos de TDD.
4. Ejecutar: implementar por unidades de trabajo, un conventional commit cada una.
5. Verificar: ejecutar las pruebas y las puertas que pide la spec (véase [Puertas antes de terminar](#puertas-antes-de-terminar)).
6. Cerrar: actualizar el banner y la evidencia, mover la spec a `specs/done/` con `git mv`, actualizar el índice y fusionar en `develop`.
7. Revisar referencias: tras cualquier movimiento, buscar la ruta antigua con `git grep` y corregir los enlaces.

Una spec está terminada solo cuando todos los criterios de aceptación están comprobados, sus pruebas nuevas existen y pasan, y no se oculta ningún bloqueo conocido. Las limitaciones y el trabajo pendiente se registran en la spec.

## TDD para la lógica crítica

La lógica crítica es el código en el que un defecto corrompe datos o rompe un invariante. En Gafira abarca el modelo de documento, la carga y el guardado de JSON y su validación, los ids y la arena, los comandos de edición y el historial, el modelo de tiempo y el timecode, las conversiones de color, la geometría, las transformaciones y las operaciones de path, la interpolación de animaciones, los límites de tamaño y DPI, el parsing de formatos de archivo y toda regla de spec con un invariante.

La lógica crítica sigue rojo, verde, refactorización. Primero se escribe la prueba que falla, se ejecuta y se confirma que falla por el motivo esperado; después se implementa lo mínimo para que pase y se limpia el código con las pruebas en verde. Una corrección de errores en lógica crítica empieza con una prueba de regresión. El resto del código se valida con pruebas de integración por funcionalidad en el `tests/` de cada crate; no hay capa de extremo a extremo ni umbral de cobertura.

Las pruebas cubren el contrato público y nombran el ID de requisito que demuestran en su nombre o en un comentario de una línea, de modo que `git grep ARQ-nn` rastrea la cobertura. `cargo-mutants` es opcional, solo para la lógica crítica, y no es una puerta de la CI (véase el [capítulo de herramientas](09-tooling-and-quality.md)). Una spec solo de documentación no puede declarar TDD de código; necesita una relectura estructural y la comprobación de enlaces.

## Puertas antes de terminar

Durante el trabajo, las puertas se limitan al crate tocado (`-p <crate>`), y a `--workspace` cuando el cambio toca un crate del que dependen otros, un `Cargo.toml`, un `Cargo.lock` o `ci.yml` (ADR 0017). La suite completa se ejecuta una vez al cerrar la spec. La sección «Before calling something done» de `AGENTS.md` enumera las puertas:

| Puerta | Cuándo |
|---|---|
| `cargo fmt --check`, `clippy` del workspace con `-D warnings`, `cargo test --workspace --locked` | Siempre |
| `clippy` y pruebas de `gafira-testkit` con `--features ports` | Cambió `gafira-testkit` o `gafira-ports` |
| `cargo deny check` | Cambiaron las dependencias (si está instalado; la CI lo ejecuta) |
| Pasos de política de `ci.yml` (V5, V12, ARQ-107, V13, V14) | Cambió un crate, sus capas o `ci.yml` |
| `python3 docs/manuals/tecnico/manual.py check` (y `--lang es`) | Cambió el manual técnico |
| `python3 tools/sync_agents.py --check` | Siempre |

Los comandos exactos, sus resultados y la evidencia de la spec se registran en la spec. El capítulo de herramientas describe cada puerta.

## Ramas, commits y fusiones

- `main` contiene el código publicado y estable; `develop` es la rama de integración (ADR 0003). El trabajo se hace en una rama de vida corta creada desde `develop` (`feat/<spec>-...`, `fix/...`, `docs/...`, `chore/...`).
- Si el trabajo pasa sus puertas, el agente que lo hace fusiona la rama en `develop` con `git merge --no-ff`, empuja `develop` y borra la rama.
- El único pull request es `develop` -> `main`. Se abre solo cuando el titular lo pide, o con su visto bueno en un punto digno de versión, y lo fusiona el titular. Las versiones y etiquetas se crean desde `main`, y después `develop` se sincroniza con `main`.
- Los commits son conventional commits, una unidad de trabajo revisable cada uno, sin líneas de atribución de IA.
- El agente se detiene y consulta al titular ante un bloqueo real: una decisión de producto abierta, un cambio destructivo o que rompe el formato, o un cambio que va mucho más allá del alcance de la spec.

## Agentes, skills y el script de sincronización

`.agents/` es la única fuente de verdad de los agentes (`.agents/agents/`) y las skills (`.agents/skills/`). Claude Code, OpenCode y Pi leen los mismos archivos mediante enlaces simbólicos relativos: `.claude/`, `.opencode/` y `.pi/` contienen cada uno un enlace `agents` y otro `skills` hacia `.agents/`. Los agentes y las skills no se editan a través de esos enlaces.

El script de sincronización `tools/sync_agents.py` crea los enlaces que faltan y repara los incorrectos o absolutos. Tras cualquier cambio en `.agents/`, se ejecuta y después se confirma que su modo de comprobación pasa:

```bash
python3 tools/sync_agents.py
python3 tools/sync_agents.py --check
```

`--check` no cambia nada; sale con 1 si hay deriva y con 2 si la fuente no es válida o hay un conflicto. El frontmatter compartido de los agentes se limita a `name`, `description`, `mode: subagent` y `subagent_mode`; los modelos y demás ajustes por runtime viven en la configuración de cada runtime (`.agents/README.md`).

`manual-writer` es el único agente del repositorio; actualiza este manual cuando el titular lo pide. Las skills se referencian por nombre y se resuelven mediante `.atl/skill-registry.md` (ignorado por git) con la skill `skill-registry`.

## Reglas de código e idioma

- Rust edición 2024, toolchain 1.93.1; `unsafe` está prohibido; `#[allow]` no se usa para silenciar lints.
- Los crates de dominio usan `BTreeMap` y `BTreeSet`, nunca `HashMap` ni `HashSet`.
- El código, los comentarios, los commits y los artefactos generados están en inglés. Las specs de producto bajo `specs/` están en español. El texto de la interfaz de usuario es en español e inglés (D-08).
- Sin rutas absolutas ni de usuario en archivos versionados. Las skills se referencian solo por nombre.

## Documentación

Este manual vive en `docs/manuals/tecnico/` y se mantiene con `manual.py` (véase el README de la carpeta). Un cambio que altere una API pública, una regla de capas, una puerta o el proceso actualiza el capítulo correspondiente, en ambos idiomas y en la misma rama, y marca como **Planificado** lo que aún no esté construido.

## Historial

Antes del 2026-11-16 el repositorio seguía un flujo multiagente: SDD ligero con ODD lite (ADR 0001, 0002, 0004 y 0011). El ADR 0018 reemplaza esos ADR, modifica el ADR 0003 en cuanto a quién fusiona y mantiene el alcance de puertas por crate del ADR 0017. El registro de ese flujo se trasladó sin cambios a `specs/done/legacy/`: los changes cerrados en `history/`, las specs por capacidad en `capabilities/`, la carpeta abierta del change `gafira-engine` en `changes/`, y el índice anterior con su lista numerada de pendientes en `README.md`. Ese registro se lee y nunca se reescribe. `specs/README.md` muestra `gafira-engine` como en progreso; retomarlo supone abrir una spec con un ID de spec nuevo que cite la carpeta. El [glosario](11-glossary.md) conserva los términos de ese flujo en su sección «Términos históricos».
