# Metodología de desarrollo

## SDD ligero

Las specs ya dicen qué construir, así que no hay fases separadas de propuesta, diseño o tareas. El flujo es:

```
spec -> (explorer, only when a claim is risky) -> tdd-apply -> mutants + verify -> archive
```

| Paso | Agente (`.agents/agents/`) | Salida |
|---|---|---|
| Medir afirmaciones riesgosas en un directorio temporal y contrastar cifras entre documentos | `explorer` | `openspec/changes/<change>/spike.md` con comando, salida y veredicto |
| Construir una entrega con TDD estricto | `tdd-apply` | Commits y `openspec/changes/<change>/apply-progress.md` |
| Probar por mutación las líneas cambiadas; reportar mutantes supervivientes (solo lectura) | `mutants` | Un informe en el directorio temporal de la sesión, fuera del repositorio |
| Comprobar las puertas y cada requisito y escenario | `verify` | `openspec/changes/<change>/verify-report.md`, PASS o FAIL |
| Cerrar el change tras PASS y con todas las entregas hechas | `archive` | Delta specs fusionadas en `openspec/specs/`; el change pasa a `openspec/changes/archive/` |

Ante un FAIL, o cuando quedan entregas, el flujo vuelve a `tdd-apply`.

El agente `coordinator` ejecuta todo el flujo: lee el estado del repositorio, elige el siguiente paso, delega en los agentes anteriores con un solo escritor a la vez, rechaza un «hecho» sin evidencia y se detiene ante decisiones del titular y fusiones. Los subagentes no pueden lanzar subagentes, así que el coordinador debe ser el agente del hilo principal. Solo se escribe una nota de diseño breve cuando una spec deja abierta una decisión real.

## TDD estricto

RED (escribir la prueba, ejecutarla, verla fallar), GREEN (código mínimo), REFACTOR (las pruebas siguen en verde). Un commit por unidad de trabajo revisable. Cada commit pasa por sí solo las tres puertas locales (véase el capítulo de herramientas).

## Specs

| Lugar | Contenido |
|---|---|
| `docs/specs/00-decisiones.md` | Decisiones del titular (D-nn). Prevalecen sobre las demás specs y no se reabren sin el titular |
| `docs/specs/NN-*.md` | El documento en curso; los completados pasan a `docs/specs/archive/` con `git mv`, los que esperan están en `docs/specs/backlog/` |
| `openspec/specs/<capability>/spec.md` | Specs principales por capacidad, la fuente de verdad, completadas al archivar |
| `openspec/changes/<change>/` | `proposal.md`, `design.md`, `tasks.md` (opcionales), `specs/<capability>/spec.md` (delta specs), `apply-progress.md`, `verify-report.md` |

Formato de spec: palabras clave de RFC 2119; escenarios como Given/When/Then; cada requisito etiquetado con su entrega (`D1`, `D2a`...) y cada escenario con su tipo (unit, property, integration, command, manual). Las delta specs usan `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements` (con Reason y nota de Migration) y `## RENAMED Requirements`. El archivado es el único paso que edita las specs principales, y avisa antes de cualquier delta destructiva.

Cadena de changes hasta ahora: `workspace-scaffold`, `domain-color` y `domain-doc` están cerrados (los dos últimos en `openspec/changes/archive/`); `edit-history` está abierto.

## Commits, ramas y pull requests

- Conventional commits; sin líneas de atribución de IA.
- Desarrollo basado en tronco con ramas de vida corta. Cuando una entrega pasa verify, el coordinador empuja la rama y abre un pull request contra `main` (o contra la rama de la entrega anterior aún sin fusionar). El titular fusiona, en orden, de abajo hacia arriba, con commits de fusión. Un pull request cuya base ya se fusionó se redirige primero a `main`.
- El estado vive en la carpeta del change y en git, no en una herramienta de memoria externa.

## Reglas de código e idioma

- Rust edición 2024, toolchain 1.93.1; `unsafe` está prohibido; `#[allow]` no se usa para silenciar lints.
- Los crates de dominio usan `BTreeMap` y `BTreeSet`, nunca `HashMap` ni `HashSet`.
- Los artefactos (código, comentarios, documentos, specs, commits) están en inglés. El texto de la interfaz de usuario es en español e inglés (D-08). Los documentos de producto bajo `docs/specs/` están en español.
- Sin rutas absolutas ni de usuario en archivos versionados. Las skills se referencian solo por nombre.

## Documentación

Este manual vive en `docs/manuals/tecnico/` y se mantiene con `manual.py` (véase el README de la carpeta). Existe en inglés (fuente) y en español, con la misma numeración y los mismos nombres de archivo. Un cambio que altere una API pública, una regla de capas, una puerta o el proceso actualiza el capítulo correspondiente, en ambos idiomas, en el mismo pull request, y marca como **Planificado** lo que aún no esté construido.
