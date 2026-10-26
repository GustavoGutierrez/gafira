# Metodología de desarrollo

## Estado

| Área | Estado | Fuente |
|---|---|---|
| Modos de flujo, packs, puertas G0 a G5, profundidades de verify V0 a V3, niveles de mutación T0 a T4 | **Implementado** (decisión aceptada; las definiciones de los agentes la siguen) | `docs/adr/0001-flow-modes-and-packs.md`, `AGENTS.md`, `.agents/agents/coordinator.md` |
| `verification.json` | **Implementado** como piloto; el punto de revisión es el archivado del siguiente change que lo use | ADR 0001 registro de decisiones 5, `.agents/agents/verify.md` |
| Archivado de una ejecución ligera sin fusión de deltas | **Implementado** | ADR 0001 registro de decisiones 6, `.agents/agents/archive.md` |

## SDD ligero

Las specs ya dicen qué construir, así que por defecto no hay fases separadas de propuesta, diseño o tareas. Dos etapas previas son opcionales: `specifier`, cuando falta la spec o es ambigua, y `architect`, cuando hace falta una decisión de diseño real o un desglose en entregas. El flujo completo es:

```
(specify) -> spec -> (architect) -> (explorer, only when a claim is risky) -> tdd-apply -> mutants + verify -> archive
```

El modo de una solicitud (sección siguiente) decide qué etapas se ejecutan. Los agentes están en `.agents/agents/`.

| Paso | Agente | Salida |
|---|---|---|
| Convertir una solicitud en specs: requisitos, escenarios, criterios de aceptación, no objetivos, preguntas abiertas | `specifier` | Delta specs en `openspec/changes/<change>/specs/` |
| Decidir el cómo cuando una spec deja abierta una decisión de diseño; dividir el trabajo en entregas con una tabla de dependencias | `architect` | `openspec/changes/<change>/design.md`, ADR transversales en `docs/adr/` |
| Medir afirmaciones riesgosas en un directorio temporal y contrastar cifras entre documentos | `explorer` | `openspec/changes/<change>/spike.md` con comando, salida y veredicto |
| Construir una entrega con TDD estricto | `tdd-apply` | Commits y `openspec/changes/<change>/apply-progress.md` |
| Probar por mutación el código según el nivel de la ejecución; reportar mutantes supervivientes (solo lectura) | `mutants` | Un informe en el directorio temporal de la sesión, fuera del repositorio |
| Comprobar las puertas y cada criterio de aceptación, requisito y escenario | `verify` | `verify-report.md` y `verification.json` en la carpeta del change, PASS o FAIL |
| Cerrar el change tras PASS y con todas las entregas hechas | `archive` | Delta specs fusionadas en `openspec/specs/`; el change pasa a `openspec/changes/archive/` |
| Editar el manual técnico, primero en inglés y con espejo en español | `manual-writer` | Capítulos en ambos idiomas, evidencia de `manual.py check` y `build` |

`mutants` y `verify` se ejecutan en paralelo sobre la misma entrega. `verify` ejecuta las puertas locales, asocia cada criterio a una prueba o un comando, lee el informe de mutantes y escribe un veredicto con hallazgos numerados (críticos, advertencias, sugerencias). Un FAIL abre una ronda de corrección: `tdd-apply` escribe primero las pruebas y registra la ronda en `apply-progress.md`, después `verify` se ejecuta de nuevo.

## Modos de flujo y packs

El `coordinator` clasifica cada solicitud antes de enrutarla. Aplica los mínimos obligatorios, después el valor por defecto del pack y, por último, la instrucción explícita del titular. Solo ejecuta los agentes y las comprobaciones del modo resultante. Decisión: `docs/adr/0001-flow-modes-and-packs.md` (aceptada).

### Modos

| | Light | Standard | Full |
|---|---|---|---|
| Agentes | `coordinator`, `tdd-apply`, `verify` | añade `specifier`, `explorer` opcional, `mutants`, `archive` | añade `architect`; `mutants` y `verify` por entrega |
| Fuente de la aceptación | Bloque Light spec escrito por el coordinador | Delta specs de `specifier` | Delta specs y `design.md` con la tabla de entregas |
| Puertas | G0, G3, G4 | G0, G1, G3, G4, G5 | G0 a G5 |
| Nivel de mutación por defecto | T1 o T2 según el pack | T2 o T3 según el pack | T2 por entrega, T4 al archivar |
| Profundidad de verify por defecto | V1 o V2 según el pack | V2 | V3 para la primera entrega de un crate o capacidad nuevos, V2 después |

### Mínimos obligatorios

La primera fila que coincide fija el modo mínimo y siempre prevalece el modo más alto. El coordinador nunca baja un modo por debajo de un mínimo. Solo el titular puede bajar el valor por defecto de un pack.

| Señal | Modo mínimo |
|---|---|
| Los criterios requieren una decisión del titular que `docs/specs/00-decisiones.md` no da | Detenerse y preguntar al titular |
| Crate nuevo, arista de dependencia, fila de la tabla de capas V5, versión de formato serializado, decisión de diseño transversal o varias entregas ordenadas | Full |
| Cambio de API pública o de formato en un crate existente; una regla nueva sobre código crítico; una solicitud tan ambigua que los criterios inventarían comportamiento | Standard |
| Previsión multiplicada por 1,4 por encima de unas 300 líneas cambiadas | Standard |
| Previsión multiplicada por 1,4 por encima de unas 700 líneas, o entregas en paralelo | Full |

Una solicitud que no coincide con ninguna fila de pack, o con dos, se trata como la más pesada.

### Packs

Un pack fija el modo, los agentes, el nivel de mutación y la profundidad de verify para un tipo de solicitud.

| Tipo de solicitud | Modo | Nivel | Verify |
|---|---|---|---|
| Solo documentación | Light | T0 | V0 |
| Cambio del manual | Light, `manual-writer` | T0 | V0 (`manual.py check` y `build`) |
| Cambio de skill o de definición de agente | Light, un agente de propósito general en un worktree, después `verify` | T0 | V1 |
| Actualización de dependencia o de toolchain | Light | T0 (T2 sobre el diff derivado si cambió código) | V1 (V2 si cambió código) |
| Corrección de error | Light si las specs fijan el comportamiento y la causa es local; si no, standard | T2 (T3 en un archivo crítico) | V2, prueba reproductora en RED primero |
| Refactor pequeño sin cambio de comportamiento | Light | T2 (T3 en un archivo crítico) | V2 y diff de elementos públicos vacío |
| Función nueva en un crate de dominio existente | Standard | T3 | V2 |
| Capacidad o crate nuevo | Full | T2 por entrega, T4 al archivar | V3, después V2 |
| Trabajo solo de spec o de spec de producto | Standard sin código, `specifier` | T0 | V0 |
| Release o versionado | Light | T0 | V1 |
| Investigación o spike | Light, solo `explorer` | T0 | Ninguna; la salida es `spike.md` |
| Hotfix de CI o de herramientas | Light | T0 | V1 |
| Refuerzo solo de pruebas o ronda de corrección tras un FAIL | Hereda el modo del change | Mismo nivel, `--iterate` permitido | V2 solo sobre los hallazgos |

Un cambio en una fila V5 o en un pin de acción dentro de un hotfix de CI es un mínimo full. El titular revisa todo pull request que cambie definiciones de agentes o skills, porque cambia el propio proceso.

### Registro y anulación

El coordinador escribe una línea en la cabecera de `apply-progress.md` (dentro del bloque Light spec en modo light):

```
Flow: <mode> | pack: <pack> | tier: T<n> | verify: V<n> | decided-by: coordinator|owner | reason: <one line>
```

Cada cambio de modo se añade como `Flow-change: light -> standard (trigger: ambiguity, after verify round 1)`. El coordinador comunica al titular el modo elegido en su primer informe.

La instrucción explícita del titular prevalece sobre cualquier otra regla («run it full», «run it light», «skip mutation», «T4 now»). El coordinador registra `decided-by: owner`. Si la instrucción baja de un mínimo, el coordinador expone el riesgo una vez y después obedece, y G0 se mantiene en su forma mínima. Una anulación se aplica solo al change en curso. El coordinador puede subir el modo de inmediato. Solo puede bajarlo en un límite de fase, antes de que empiece trabajo del modo más pesado, y nunca por debajo de un mínimo.

## Puertas

El coordinador comprueba cada puerta antes de la fase siguiente. Una puerta fallida devuelve el trabajo a la etapa responsable.

| Puerta | Se supera cuando | Modos |
|---|---|---|
| G0 | Existe un artefacto confirmado (bloque Light spec o delta spec) antes del primer commit de implementación, con alcance, no objetivos, un criterio comprobable y tipado por cada cambio de comportamiento y la línea `Flow:` | Todos |
| G1 | Tras `specifier`: sin preguntas abiertas, cada requisito con un escenario tipado, decisiones del titular respetadas | Standard, full |
| G2 | Tras `architect`: tabla de entregas con dependencias, cada entrega cerca o por debajo de unas 700 líneas | Full |
| G3 | Tras `tdd-apply`: `status: done` con commits, salida de las puertas, líneas medidas, evidencia RED por criterio, desviaciones | Todos los modos con código |
| G4 | Tras `verify` y `mutants`: cero CRITICAL, umbrales del nivel cumplidos, `verification.json` escrito | Todos los modos con V1 o superior |
| G5 | Antes de push, pull request y archivado: PASS en todas las entregas, T4 ejecutado en cada crate que toca el change | Standard, full |

El coordinador no puede dispensar G0. Si el titular pide no tener spec, el coordinador escribe igualmente una Light spec de tres líneas.

## Modo light

En modo light el coordinador escribe un solo artefacto: un bloque Light spec de unas 40 líneas como máximo al principio de `openspec/changes/<change>/apply-progress.md`. Lo confirma como `docs(<change>): light spec` antes de cualquier ejecución de `tdd-apply`. El bloque contiene la línea `Flow:`, el motivo, el alcance, los no objetivos, los criterios de aceptación tipados, un plan de 7 pasos como máximo, el riesgo y la reversibilidad, y las decisiones del titular afectadas.

El coordinador sigue sin escribir código de producción ni pruebas, sin editar delta specs, specs principales o de producto ni ADR, sin decidir asuntos exclusivos del titular, sin inventar comportamiento y sin aprobar su propio trabajo. Solo modifica el bloque tras empezar la implementación en un commit aparte `docs(<change>): light spec amend` que indica el motivo. La restricción a un único archivo es una regla, no un límite de la herramienta. `verify` la hace cumplir: comprueba que el commit de la Light spec toca solo ese archivo y que ningún commit del implementador cambió el bloque.

Una ejecución light no tiene delta spec. `archive` mueve la carpeta del change a `openspec/changes/archive/` sin fusión de deltas.

### Escalada

El coordinador comprueba estos desencadenantes tras cada retorno de un agente. Subir el modo es inmediato y se registra con `Flow-change:`.

| Desencadenante | Acción |
|---|---|
| Un criterio de aceptación necesita un comportamiento que ninguna spec fija (`blocked` o hallazgo de clase `ambiguity`) | Standard: `specifier` escribe la delta spec |
| Cambio de API pública o de formato serializado en el diff | Standard |
| Una regla nueva sobre código crítico | Standard, nivel subido a T3 |
| Tamaño medido por encima de unas 300 líneas, o previsión multiplicada por 1,4 por encima de unas 300 | Standard; por encima de unas 700 o con varias entregas: full |
| Cuestión de arquitectura (arista de dependencia, fila V5, crate nuevo, capas) | Full: `architect` |
| Se toca una decisión del titular | Detenerse y preguntar al titular |
| Segundo FAIL de cualquier clase en la misma entrega | Subir un modo e informar al titular; sin tercera ronda light |

## Profundidades de verify

La profundidad fija cuánto trabajo independiente hace `verify`. `verify` nunca edita código fuente y nunca se fía del resumen del implementador.

| Profundidad | Qué se ejecuta |
|---|---|
| V0 | Sin agente. El coordinador ejecuta las comprobaciones deterministas del pack (enlaces, `manual.py check`, `cargo fmt --check`, `git diff --stat`) y las reporta tal cual |
| V1 | Puertas en HEAD; cada criterio de aceptación asociado a una prueba o un comando; PASS o FAIL |
| V2 | V1, más comprobaciones de sondeo derivadas de los criterios antes de leer las pruebas del implementador, mutantes manuales (5 como máximo, 3 en modo light) sobre una exportación temporal, hallazgos clasificados por `class:` |
| V3 | V2, más puertas por commit sobre exportaciones limpias, ejecuciones negativas de la política de CI y la matriz completa de requisitos y escenarios |

### verification.json

`verify` escribe `openspec/changes/<change>/verification.json` junto a `verify-report.md` en toda profundidad superior a V0. Contiene un objeto por ronda de verify en un array `rounds`: ronda, commit verificado, modo, nivel, profundidad, veredicto, `next`, recuentos, hallazgos (id, severidad, `class`, referencia, resumen), códigos de salida de las puertas, recuentos de mutación y tiempo, mutantes manuales, criterios cubiertos y tamaño medido. El coordinador lee este archivo, no el informe largo, para enrutar. Si ambos difieren, prevalece el informe y la ronda se repite.

El archivo es un piloto. Si el enrutado no lo lee tras archivar el siguiente change que lo use, se elimina.

## Niveles de mutación

Los niveles fijan cuánto código mutan `mutants` y `verify`. El [capítulo de herramientas](09-tooling-and-quality.md) indica los comandos.

| Nivel | Alcance | Se aplica a |
|---|---|---|
| T0 | Ninguno | No se toca código Rust |
| T1 | Solo mutantes manuales de `verify`, de 3 a 5 como máximo | Cambios mínimos en código no crítico |
| T2 | `cargo mutants --in-diff`: líneas cambiadas | Toda entrega que cambia Rust |
| T3 | `--file` por cada archivo fuente no de prueba cambiado | Valor por defecto de standard; entregas que editan un archivo crítico existente |
| T4 | Crate completo | Archivado, en cada crate que toca el change, en todos los modos |

El coordinador sube un nivel cuando una entrega edita un archivo crítico existente o añade una regla de un tipo nuevo. Solo el titular baja un nivel. `--iterate` se permite únicamente en rondas de corrección solo de pruebas con el mismo conjunto de archivos, nunca en la ronda que cierra una entrega ni en T4. En modo light `mutants` se ejecuta solo desde T2.

## Control del ciclo

El coordinador controla el ciclo desde la solicitud hasta el archivado. Cada agente devuelve su resultado al coordinador y ninguno llama a otro; el campo `next` de un resultado es una recomendación. Los subagentes no pueden lanzar subagentes, así que el coordinador debe ser el agente del hilo principal.

`verify` asigna una clase a cada hallazgo CRITICAL y WARNING. El coordinador enruta por clase y nunca corrige un hallazgo por sí mismo.

| Clase | Significado | Ruta |
|---|---|---|
| `implementation` | Falta de código o de prueba, fallo de una puerta, mutante crítico superviviente, tamaño | `tdd-apply`, después `verify` y `mutants` |
| `architecture` | Descomposición, decisión o ADR erróneos | `architect`, después `specifier` si cambian los requisitos, después `tdd-apply` |
| `ambiguity` | Requisito ausente o contradictorio | `specifier`; las preguntas abiertas van al titular |
| `claim` | Afirmación riesgosa sin demostrar | `explorer` |

Un informe mixto se enruta primero hacia arriba: ambigüedad, después arquitectura, después implementación.

### Límites del bucle

- Tras dos FAIL consecutivos de la misma clase en una entrega, el coordinador se detiene y pregunta al titular, con una pregunta y una recomendación.
- Una entrega tiene como máximo tres rondas de verify entre todas las clases. Una cuarta ronda es una decisión del titular.
- En modo light, el segundo FAIL de cualquier clase sube el modo en su lugar.
- El coordinador nunca aprueba por sí mismo un FAIL, dispensa un hallazgo ni sube un tope en silencio. Los asuntos exclusivos del titular (decisiones de `docs/specs/00-decisiones.md`, fusiones, precios, licencia, alcance) nunca se enrutan a un agente.

Estos límites son señales para detenerse y preguntar, no bloqueos estrictos. El techo de una entrega es de unas 700 líneas; superarlo requiere la aprobación explícita del titular.

## TDD estricto

RED (escribir la prueba, ejecutarla, verla fallar), GREEN (código mínimo), REFACTOR (las pruebas siguen en verde). Un commit por unidad de trabajo revisable. Cada commit pasa por sí solo las tres puertas locales (véase el capítulo de herramientas). Cuando una prueba pasa de inmediato porque el código ya existe, el RED se demuestra con un mutante (véase la sección de pruebas de mutación del capítulo de herramientas).

## Specs

| Lugar | Contenido |
|---|---|
| `docs/specs/00-decisiones.md` | Decisiones del titular (D-nn). Prevalecen sobre las demás specs y no se reabren sin el titular |
| `docs/specs/NN-*.md` | El documento en curso; los completados pasan a `docs/specs/archive/` con `git mv`, los que esperan están en `docs/specs/backlog/` |
| `openspec/specs/<capability>/spec.md` | Specs principales por capacidad, la fuente de verdad, completadas al archivar |
| `openspec/changes/<change>/` | `proposal.md`, `design.md`, `tasks.md` (opcionales), `specs/<capability>/spec.md` (delta specs; ninguna en una ejecución light), `apply-progress.md`, `verify-report.md`, `verification.json` |

Formato de spec: palabras clave de RFC 2119; escenarios como Given/When/Then; cada requisito etiquetado con su entrega (`D1`, `D2a`...) y cada escenario con su tipo (unit, property, integration, command, manual). Las delta specs usan `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements` (con Reason y nota de Migration) y `## RENAMED Requirements`. El archivado es el único paso que edita las specs principales, y avisa antes de cualquier delta destructiva.

Cadena de changes hasta ahora: seis changes están cerrados y archivados en `openspec/changes/archive/`, y no hay ninguno abierto. En orden: `workspace-scaffold`, `domain-color` y `domain-doc` (2026-10-24), `domain-time` y `edit-history` (2026-10-25), y `engine-pathops` (2026-10-26). El change `edit-history` incluye todas sus entregas, de D1 a D10, con el paso de CI V14.

## Commits, ramas y pull requests

- Conventional commits; sin líneas de atribución de IA.
- Desarrollo basado en tronco con ramas de vida corta. Cuando una entrega pasa verify, el coordinador empuja la rama y abre un pull request contra `main` (o contra la rama de la entrega anterior aún sin fusionar). El titular fusiona, en orden, de abajo hacia arriba, con commits de fusión. Un pull request cuya base ya se fusionó se redirige primero a `main`.
- El estado vive en la carpeta del change y en git, no en una herramienta de memoria externa.
- La CI está desactivada manualmente en GitHub por ahora (facturación), así que las puertas de cada entrega se ejecutan en local antes del pull request; véase el capítulo de herramientas.

## Reglas de código e idioma

- Rust edición 2024, toolchain 1.93.1; `unsafe` está prohibido; `#[allow]` no se usa para silenciar lints.
- Los crates de dominio usan `BTreeMap` y `BTreeSet`, nunca `HashMap` ni `HashSet`.
- Los artefactos (código, comentarios, documentos, specs, commits) están en inglés. El texto de la interfaz de usuario es en español e inglés (D-08). Los documentos de producto bajo `docs/specs/` están en español.
- Sin rutas absolutas ni de usuario en archivos versionados. Las skills se referencian solo por nombre.

## Documentación

Este manual vive en `docs/manuals/tecnico/` y se mantiene con `manual.py` (véase el README de la carpeta). Un cambio que altere una API pública, una regla de capas, una puerta o el proceso actualiza el capítulo correspondiente, en ambos idiomas, en el mismo pull request, y marca como **Planificado** lo que aún no esté construido.
