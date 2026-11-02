# Metodología de desarrollo

## Estado

| Área | Estado | Fuente |
|---|---|---|
| Modos de flujo, packs, puertas G0 a G5, profundidades de verify V0 a V3, niveles de mutación T0 a T4 | **Implementado** (decisión aceptada; las definiciones de los agentes la siguen) | `docs/adr/0001-flow-modes-and-packs.md`, `AGENTS.md`, `.agents/agents/coordinator.md` |
| Carril rápido: ejecución directa de la spec con Slice card, el interruptor `mutation`, valores por defecto V1/V2 de verify | **Implementado** (decisión aceptada; sustituye partes del ADR 0001) | `docs/adr/0002-fast-lane.md`, `AGENTS.md`, `.agents/agents/coordinator.md` |
| `verification.json` | **Implementado** como piloto; el punto de revisión es el archivado del siguiente change que lo use | ADR 0001 registro de decisiones 5, `.agents/agents/verify.md` |
| Reparto de mutantes manuales entre `tdd-apply` y `verify`, rondas de cierre, cierre de supervivientes de T4 | **Implementado** (decisión aceptada; las definiciones de los agentes la siguen) | `docs/adr/0004-hand-mutant-split-and-verify-rounds.md`, `AGENTS.md`, `.agents/agents/verify.md`, `.agents/agents/tdd-apply.md`, `.agents/agents/coordinator.md` |
| Archivado de una ejecución ligera o de carril rápido sin fusión de deltas | **Implementado** | ADR 0001 registro de decisiones 6, ADR 0002 sección 6, `.agents/agents/archive.md` |
| ODD lite: niveles de riesgo L0 a L3, entradas de outcome en `.odd/state.json`, carpetas solo cuando hacen falta, hallazgos de verify en la entrada, veredicto BLOCKED | **Implementado** (decisión aceptada; los archivos de proceso la siguen) | `docs/adr/0011-odd-lite.md`, `AGENTS.md`, `.agents/agents/coordinator.md`, `.agents/agents/verify.md` |

## SDD ligero

Las specs ya dicen qué construir, así que por defecto no hay fases separadas de propuesta, diseño o tareas. Dos etapas previas son opcionales: `specifier`, cuando falta la spec o es ambigua, y `architect`, cuando hace falta una decisión de diseño real o entregas en paralelo (las entregas en serie listadas en una Slice card no necesitan `architect`). El flujo completo es:

```
(specify) -> spec -> (architect) -> (explorer, only when a claim is risky) -> tdd-apply -> verify -> (mutants T4 per touched crate) -> archive
```

En el trabajo que una spec existente ya cubre, la ruta por defecto registra los criterios de aceptación en una entrada de outcome de `.odd/state.json` (véase [ODD lite](#odd-lite)). `tdd-apply` trabaja directamente a partir de los IDs de requisitos de producto, sin propuesta, sin diseño y sin delta specs. La Slice card del carril rápido (ADR 0002, [Carril rápido](#carril-rápido)) sigue siendo la forma de una carpeta de change en curso. En este flujo, `spec` significa la spec de producto más la entrada de outcome o la Slice card.

El modo de una solicitud (sección siguiente) decide qué etapas se ejecutan. `mutants` se ejecuta según el valor `mutation` de la línea `Flow:`. Los agentes están en `.agents/agents/`.

| Paso | Agente | Salida |
|---|---|---|
| Convertir una solicitud en specs: requisitos, escenarios, criterios de aceptación, no objetivos, preguntas abiertas | `specifier` | Carril rápido: solo ante una ambigüedad reportada, y corrige la spec de producto en `specs/`. En otro caso, delta specs en `specs/changes/<change>/specs/` |
| Decidir el cómo cuando una spec deja abierta una decisión de diseño; dividir el trabajo en entregas con una tabla de dependencias | `architect` | `specs/changes/<change>/design.md`, ADR transversales en `docs/adr/` |
| Medir afirmaciones riesgosas en un directorio temporal y contrastar cifras entre documentos | `explorer` | `specs/changes/<change>/spike.md` con comando, salida y veredicto; sin carpeta, solo el resultado |
| Construir una entrega con TDD estricto | `tdd-apply` | Commits y `specs/changes/<change>/apply-progress.md`; sin carpeta, notas en el cuerpo del último commit del implementador |
| Probar por mutación el código según el valor de `mutation` (por defecto, crate completo al cerrar el change); reportar mutantes supervivientes (solo lectura) | `mutants` | Un informe en el directorio temporal de la sesión, fuera del repositorio |
| Comprobar las puertas y cada criterio de aceptación, requisito y escenario | `verify` | `verify-report.md` y `verification.json` en la carpeta del change (sin carpeta, en el directorio temporal), PASS, FAIL o BLOCKED |
| Cerrar el change tras PASS y con todas las entregas hechas | `archive` | El change pasa a `specs/history/` (sin carpeta, no hay nada que mover y la entrada se elimina); carril rápido: tabla de estado de `specs/README.md` actualizada con los IDs cubiertos; las delta specs, si existen, se fusionan en `specs/capabilities/` |
| Editar el manual técnico, primero en inglés y con espejo en español | `manual-writer` | Capítulos en ambos idiomas, evidencia de `manual.py check` y `build` |

`verify` ejecuta las puertas locales, asocia cada criterio a una prueba o un comando, lee el informe de mutantes cuando hubo una ejecución y escribe un veredicto con hallazgos numerados (críticos, advertencias, sugerencias). Un FAIL abre una ronda de corrección: `tdd-apply` escribe primero las pruebas y registra la ronda en `apply-progress.md`, después `verify` se ejecuta de nuevo. La ronda 1 es la única con sondeos y mutantes manuales independientes; las siguientes son rondas de cierre (véase [Rondas de verify](#rondas-de-verify)).

## ODD lite

ODD lite enruta una solicitud según el riesgo y registra su avance fuera de una carpeta de change. Se apoya en los modos de flujo y no elimina ninguna puerta, mínimo obligatorio ni parada del titular. La decisión es `docs/adr/0011-odd-lite.md`, con estado **aceptado**. Los archivos de proceso (`AGENTS.md`, las definiciones de los agentes) ya la aplican.

Las solicitudes en lenguaje natural siguen siendo el punto de entrada; no hay comandos ODD. El `odd-spec` del brief es `specifier` con el skill `product-specs`, `odd-run` es `coordinator` con `tdd-apply` y `odd-verify` es `verify`. Un outcome es un resultado observable, una spec describe una capacidad estable y una tarea es trabajo ejecutable.

### Niveles de riesgo

El coordinador fija el nivel antes de aplicar los mínimos obligatorios. El nivel sale del impacto, la incertidumbre y la reversibilidad, nunca del número de archivos. El modo es el más pesado entre el mínimo, el nivel y el valor por defecto del pack. Esa regla fija solo el modo: la profundidad de verify y los agentes salen de la fila del pack, y un nivel puede subirlos, nunca bajarlos.

| Nivel | Ejemplos | Modo | Ruta |
|---|---|---|---|
| L0 trivial | Texto, comentarios, documentos, estilo sin cambio de comportamiento | Light | Un escritor (o los agentes del pack), V0 o la profundidad del pack si es mayor, `mutation: off` donde se permite |
| L1 rutinario | Un error acotado, un ajuste de interacción | Light | Criterios, `tdd-apply`, `verify` |
| L2 funcional | Una herramienta o función nueva, una funcionalidad que las specs ya cubren | Standard | Actualizar la spec si cambian los requisitos, lotes en serie, `verify`, T4 al cerrar |
| L3 crítico | Migración de datos, seguridad, licencias, el formato de proyecto | Full | Carpeta con diseño y ADR, pruebas amplias, `verify`, aprobación del titular donde corresponda |

Un nivel nunca baja de un mínimo obligatorio ni rebaja el valor por defecto de un pack. La única excepción es `mutation: off` para trabajo L0 y L1 que no toca código crítico. Los hallazgos nuevos suben el nivel en cualquier momento.

### Entradas de outcome y carpetas de change

El trabajo nuevo sobre una capacidad que una spec existente cubre no necesita carpeta en `specs/changes/`. Sus criterios de aceptación son la entrada del outcome en `.odd/state.json`, o la fila de la spec cuando cambian los requisitos. Una entrada confirmada satisface G0. El id del outcome sustituye a `<change>` en nombres de ramas, ámbitos de commit y entradas de agentes.

| Trabajo | Registro de aceptación | Carpeta |
|---|---|---|
| Capacidad cubierta por una spec (L0 a L2) | Entrada de outcome | Ninguna |
| Ninguna spec lo cubre (herramientas, definiciones de agentes, una capacidad nueva) | Light spec o delta spec | `specs/changes/<change>/` |
| Modo full o L3 | Slice card o delta spec | `specs/changes/<change>/`, con `design.md` y un ADR cuando la decisión es transversal |
| Una carpeta ya en curso | Lo que tenga | Conserva carpeta, tarjeta y diseño hasta el archivado |

Sin carpeta, los demás artefactos cambian de lugar:

- `tdd-apply` registra sus notas en el cuerpo del último commit del implementador de la entrega.
- `verify` escribe `verify-report.md` y `verification.json` en la ruta temporal que le da el coordinador.
- `explorer` devuelve su resultado sin `spike.md`.
- `archive` no tiene nada que mover y se ejecuta solo para IDs cubiertos o seguimientos; la entrada se elimina.

### Estado del outcome

`.odd/state.json` está versionado y es pequeño: `{ "schema": 1, "outcomes": [ ... ] }`, con un objeto por outcome activo. El coordinador lo actualiza en los límites de fase en un commit que toca solo ese archivo, con el outcome como ámbito (`docs(<outcome>): ...`). `tdd-apply` nunca lo edita. La entrada se elimina cuando el outcome se cierra, y git conserva el historial.

| Campo | Contenido |
|---|---|
| `outcome_id` | Id estable en kebab-case; para una carpeta en curso, el nombre del change |
| `spec_id` | Nombre de archivo de la spec sin extensión, o `none` |
| `status` | `specifying`, `implementing`, `verifying` o `blocked` |
| `criteria` | Lista de `{id, status, evidence}`; `status` es `pending`, `passed`, `failed` o `blocked` |
| `next_action` | Una línea |

Cuando la entrada es el artefacto de G0, contiene además `flow` (la línea `Flow:`), `scope`, `non_goals`, `text` y `type` en un criterio que la spec no enuncia, y `verify`.

El campo `verify` es `{round, commit, verdict, next, findings, rounds}`. Los cuatro primeros describen la última ejecución. Los otros dos guardan lo que necesita una sesión posterior, porque el directorio temporal no sobrevive a la sesión.

| Campo | Contenido |
|---|---|
| `findings` | Todo CRITICAL y WARNING de cualquier ejecución, como `{id, severity, class, ref, summary, follow_up, status}`, copiado de `verification.json`; sin SUGGESTION |
| `rounds` | Una fila `{round, verdict, classes}` por ejecución de verify de la entrega actual; una fila BLOCKED se conserva y no cuenta para las paradas |

El coordinador confirma esta actualización en la rama de la entrega antes de enrutar, con PASS, FAIL y BLOCKED por igual. Un WARNING aún abierto cuando una entrega termina en PASS pasa a ser seguimiento y llega a `archive`.

Al reanudar, el coordinador comprueba la rama, `git status`, el diff contra `develop` y las pruebas antes de continuar. La entrada es un puntero: cuando discrepa de git, prevalece git y se corrige la entrada. Una ejecución reanudada no rehace el plan ni duplica una fila de spec o un commit.

### Veredicto BLOCKED

`verify` devuelve BLOCKED cuando una comprobación crítica no pudo ejecutarse: una puerta de la profundidad, la única prueba de un criterio, una comprobación de política de CI aplicable o un informe de T4 sobre código crítico (ADR 0011). BLOCKED va al titular, nunca cuenta como PASS y no cuesta una ronda. La puerta G4 lo rechaza.

## Modos de flujo y packs

El `coordinator` clasifica cada solicitud antes de enrutarla. Fija el nivel de riesgo (véase [ODD lite](#odd-lite)), aplica los mínimos obligatorios, después el valor por defecto del pack y, por último, la instrucción explícita del titular. Solo ejecuta los agentes y las comprobaciones del modo resultante. Decisiones: `docs/adr/0001-flow-modes-and-packs.md` (aceptada, sustituida en parte) y `docs/adr/0002-fast-lane.md` (aceptada).

### Modos

| | Light | Standard | Full |
|---|---|---|---|
| Agentes | `coordinator`, `tdd-apply`, `verify` | añade `explorer` opcional, `mutants` (según `mutation`), `archive`; `specifier` solo ante ambigüedad o sin spec de producto | añade `architect` |
| Fuente de la aceptación | Entrada de outcome en el trabajo cubierto; en otro caso, bloque Light spec escrito por el coordinador | IDs de la spec de producto más entrada de outcome o Slice card (carril rápido), o delta specs | Como standard, más `design.md` con la tabla de entregas |
| Puertas | G0, G3, G4 | G0, G1 (solo si se ejecutó `specifier`), G3, G4, G5 | G0 a G5 |
| `mutation` por defecto | `off` o `archive` según el pack | `archive` (T4 al cerrar) | `archive` (T4 al cerrar) |
| Profundidad de verify por defecto | V0, V1 o V2 según el pack | V2 | V2 (V3 solo a petición del titular) |

### Mínimos obligatorios

La primera fila que coincide fija el modo mínimo y siempre prevalece el modo más alto. El coordinador nunca baja un modo por debajo de un mínimo. Solo el titular puede bajar el valor por defecto de un pack, salvo `mutation: off` en trabajo L0 y L1 que no toca código crítico.

| Señal | Modo mínimo |
|---|---|
| Los criterios requieren una decisión del titular que `specs/00-decisiones.md` no da | Detenerse y preguntar al titular |
| Fila nueva de la tabla de capas V5, arista de dependencia nueva que `specs/02-arquitectura-software.md` no recoge, versión nueva de formato serializado, una decisión de diseño transversal que la spec deja abierta, o entregas en paralelo | Full |
| Crate nuevo que `02` ya describe (lista de crates, capa, dependencias); cambio de API pública o de formato en un crate existente; una regla nueva sobre código crítico; una solicitud tan ambigua que los criterios inventarían comportamiento | Standard |
| Previsión multiplicada por 1,4 por encima de unas 300 líneas cambiadas | Standard |

Unas 700 líneas por entrega siguen siendo una señal para detenerse y preguntar, no un mínimo. Un change puede ejecutar varias entregas en serie sin `architect` cuando la Slice card las enumera. Una solicitud que no coincide con ninguna fila de pack, o con dos, se trata como la más pesada.

### Packs

Un pack fija el modo, los agentes, el valor de `mutation` y la profundidad de verify para un tipo de solicitud. La columna Mutación indica el valor y el nivel.

| Tipo de solicitud | Modo | Mutación | Verify |
|---|---|---|---|
| Solo documentación | Light | `off` (T0) | V0 |
| Cambio del manual | Light, `manual-writer` | `off` (T0) | V0 (`manual.py check` y `build`) |
| Cambio de skill o de definición de agente | Light, un agente de propósito general en un worktree, después `verify` | `off` (T0) | V1 |
| Actualización de dependencia o de toolchain | Light | `off` (T0); `archive` (T4) si cambió código | V1 (V2 si cambió código) |
| Corrección de error | Light si las specs fijan el comportamiento y la causa es local; si no, standard | `archive` (T4) | V2, prueba reproductora en RED primero |
| Refactor pequeño sin cambio de comportamiento | Light | `archive` (T4) | V2 y diff de elementos públicos vacío |
| Función nueva en un crate de dominio existente | Standard (carril rápido cuando una spec de producto la fija) | `archive` (T4) | V2 |
| Porción de spec de producto: capacidad o crate nuevo que `02` describe (carril rápido) | Standard; full solo con un mínimo full | `archive` (T4) | V2 |
| Trabajo solo de spec o de spec de producto | Standard sin código, `specifier` | `off` (T0) | V0 |
| Release o versionado | Light | `off` (T0) | V1 |
| Investigación o spike | Light, solo `explorer` | `off` (T0) | Ninguna; la salida es `spike.md` |
| Hotfix de CI o de herramientas | Light | `off` (T0) | V1 |
| Refuerzo solo de pruebas o ronda de corrección tras un FAIL | Hereda el modo del change | Mismo valor; `--iterate` permitido con `delivery` | V2 solo sobre los hallazgos |

Un cambio en una fila V5 o en un pin de acción dentro de un hotfix de CI es un mínimo full. El titular revisa todo cambio en definiciones de agentes o skills, como tarde en el pull request `develop` -> `main`, porque cambia el propio proceso.

### Registro y anulación

El coordinador escribe una línea en la cabecera de `apply-progress.md` (dentro del bloque Light spec en modo light, dentro de la Slice card en el carril rápido), o en la entrada de outcome cuando no hay carpeta:

```
Flow: <mode> | level: L<n> | pack: <pack> | mutation: off|archive|delivery | tier: T<n> | verify: V<n> | decided-by: coordinator|owner | reason: <one line>
```

Las líneas `Flow:` sin `level` siguen siendo válidas. En el carril rápido la línea va dentro de la Slice card. El campo `tier` indica el nivel por entrega con `mutation: delivery`, `T4` con `archive` y `T0` con `off`.

Cada cambio de modo se añade como `Flow-change: light -> standard (trigger: ambiguity, after verify round 1)`. El coordinador comunica al titular el modo elegido en su primer informe.

La instrucción explícita del titular prevalece sobre cualquier otra regla («run it full», «run it light», «skip mutation», que equivale a `mutation: off`, «mutate every delivery», que equivale a `delivery`, «T4 now», «V3»). El coordinador registra `decided-by: owner`. Si la instrucción baja de un mínimo, el coordinador expone el riesgo una vez y después obedece, y G0 se mantiene en su forma mínima. Una anulación se aplica solo al change en curso. El coordinador puede subir el modo de inmediato. Solo puede bajarlo en un límite de fase, antes de que empiece trabajo del modo más pesado, y nunca por debajo de un mínimo.

## Puertas

El coordinador comprueba cada puerta antes de la fase siguiente. Una puerta fallida devuelve el trabajo a la etapa responsable.

| Puerta | Se supera cuando | Modos |
|---|---|---|
| G0 | Existe un artefacto confirmado (entrada de outcome, bloque Light spec, Slice card con sus IDs de spec de producto, o delta spec) antes del primer commit de implementación, con alcance, no objetivos, un criterio comprobable y tipado por cada cambio de comportamiento y la línea `Flow:` | Todos |
| G1 | Tras `specifier`: sin preguntas abiertas, cada requisito con un escenario tipado, decisiones del titular respetadas | Standard, full |
| G2 | Tras `architect`: tabla de entregas con dependencias, cada entrega cerca o por debajo de unas 700 líneas | Full |
| G3 | Tras `tdd-apply`: `status: done` con commits, salida de las puertas, líneas medidas, evidencia RED por criterio, desviaciones | Todos los modos con código |
| G4 | Tras `verify` (y `mutants` cuando hubo una ejecución): cero CRITICAL, veredicto distinto de BLOCKED, umbrales del nivel cumplidos, `verification.json` escrito | Todos los modos con V1 o superior |
| G5 | Antes de fusionar en `develop` y de archivar: PASS en todas las entregas; con `mutation: archive` o `delivery`, T4 ejecutado en cada crate que toca el change y sin supervivientes críticos | Standard, full |

El coordinador no puede dispensar G0. Si el titular pide no tener spec, el coordinador escribe igualmente una Light spec de tres líneas. En una porción de spec de producto, la Slice card satisface G0.

## Modo light

En modo light el coordinador escribe un solo artefacto. En el trabajo cubierto es la entrada de outcome, confirmada como `docs(<outcome>): outcome entry`. En otro caso es un bloque Light spec de unas 40 líneas como máximo al principio de `specs/changes/<change>/apply-progress.md`, confirmado como `docs(<change>): light spec`. Cualquiera de los dos commits va antes de cualquier ejecución de `tdd-apply`. El bloque contiene la línea `Flow:`, el motivo, el alcance, los no objetivos, los criterios de aceptación tipados, un plan de 7 pasos como máximo, el riesgo y la reversibilidad, y las decisiones del titular afectadas.

El coordinador sigue sin escribir código de producción ni pruebas, sin editar delta specs, specs principales o de producto ni ADR, sin decidir asuntos exclusivos del titular, sin inventar comportamiento y sin aprobar su propio trabajo. Solo modifica el bloque tras empezar la implementación en un commit aparte `docs(<change>): light spec amend` que indica el motivo. La restricción a un único archivo es una regla, no un límite de la herramienta. `verify` la hace cumplir: comprueba que el commit de la Light spec toca solo ese archivo y que ningún commit del implementador cambió el bloque.

Una ejecución light no tiene delta spec. `archive` mueve una carpeta de change a `specs/history/` sin fusión de deltas; sin carpeta, no se mueve nada.

### Escalada

El coordinador comprueba estos desencadenantes tras cada retorno de un agente. Subir el modo es inmediato y se registra con `Flow-change:`.

| Desencadenante | Acción |
|---|---|
| Un criterio de aceptación necesita un comportamiento que ninguna spec fija (`blocked` o hallazgo de clase `ambiguity`) | Standard: `specifier` corrige la spec de producto cuando existe (carril rápido); en otro caso escribe la delta spec |
| Cambio de API pública o de formato serializado en el diff | Standard |
| Una regla nueva sobre código crítico | Standard; el coordinador valora `mutation: delivery` (T3) y expone el motivo |
| Tamaño medido por encima de unas 300 líneas, o previsión multiplicada por 1,4 por encima de unas 300 | Standard; por encima de unas 700: dividir en entregas en serie enumeradas en la Slice card, o `architect` si la división requiere una decisión de diseño o trabajo en paralelo |
| Cuestión de arquitectura (fila V5 nueva, arista de dependencia que `02` no recoge, versión de formato, decisión transversal abierta) | Full: `architect` |
| Se toca una decisión del titular | Detenerse y preguntar al titular |
| Segundo FAIL de cualquier clase en la misma entrega | Subir un modo e informar al titular; sin tercera ronda light |

## Carril rápido

El carril rápido ejecuta directamente una spec de producto (ADR 0002, sección 3). Cuando el titular pide ejecutar un documento de `specs/`, o una sección o un rango de IDs de él, el coordinador escribe una Slice card al principio de `specs/changes/<change>/apply-progress.md`. La confirma como `docs(<change>): slice card` antes de cualquier implementación, y la tarjeta satisface G0. En el trabajo cubierto sin carpeta, los mismos campos van en la entrada de outcome; la tarjeta sigue siendo la forma para L3, el modo full y las carpetas en curso.

```
## Slice card
Flow: <mode> | level: L<n> | pack: <pack> | mutation: archive | tier: T4 | verify: V2 | decided-by: coordinator|owner | reason: <one line>
Product IDs: <ARQ-nn, RF-nn ...> (source: specs/<NN-doc>.md §x)
Scope: <1 to 3 lines>      Non-goals: <list>
Deliveries: <only if more than one: D1 <scope, ~lines>, D2 ...; serial>
Plan: <ordered steps, expected crates and files>
Owner decisions touched: none (checked D-nn: ...)
```

Reglas del carril rápido:

- No hay `proposal.md`, ni `design.md`, ni delta specs. La fuente de aceptación de `tdd-apply` y `verify` son las filas de la spec de producto de los IDs enumerados más la Slice card.
- Cada prueba nombra el ID de requisito que demuestra, en el nombre de la prueba o en un comentario de una línea, de modo que `git grep ARQ-nn` traza la cobertura. `verify` comprueba que cada ID de la Slice card tiene una prueba que lo nombra.
- `specifier` se ejecuta solo cuando `tdd-apply` reporta `blocked` o `verify` reporta un hallazgo de clase `ambiguity`. Corrige la spec de producto en `specs/` (en español), no una delta spec, salvo que el titular pida una.
- El coordinador escribe la Slice card con los mismos límites que la Light spec. Nunca edita la spec de producto, y solo modifica la tarjeta en un commit aparte `docs(<change>): slice card amend` que indica el motivo.
- Un crate nuevo que `02` ya describe es un mínimo standard, no full. El tamaño de las entregas y las entregas en serie siguen los mínimos obligatorios anteriores.
- Las solicitudes que no son porciones de spec de producto mantienen el bloque Light spec.
- `archive` no fusiona deltas. Mueve la carpeta y actualiza la columna «Detalle» de la tabla de estado de `specs/README.md` con los IDs cubiertos. Los changes con delta specs mantienen la fusión del ADR 0001, y `specs/capabilities/` sigue siendo el registro de los changes pasados.

## Profundidades de verify

La profundidad fija cuánto trabajo independiente hace `verify`. `verify` nunca edita código fuente y nunca se fía del resumen del implementador.

| Profundidad | Qué se ejecuta |
|---|---|
| V0 | Sin agente. El coordinador ejecuta las comprobaciones deterministas del pack (enlaces, `manual.py check`, `cargo fmt --check`, `git diff --stat`) y las reporta tal cual |
| V1 | Puertas en HEAD; cada criterio de aceptación asociado a una prueba o un comando; PASS o FAIL |
| V2 | V1, más comprobaciones de sondeo derivadas de los criterios antes de leer las pruebas del implementador, solo en la ronda 1: una comprobación puntual de la lista de mutantes manuales de `tdd-apply` más 2 a 3 mutantes manuales independientes sobre una exportación temporal (véase [Mutantes manuales](#mutantes-manuales)), hallazgos clasificados por `class:` |
| V3 | Solo a petición del titular: V2, más puertas por commit sobre exportaciones limpias, ejecuciones negativas de la política de CI y la matriz completa de requisitos y escenarios |

### Rondas de verify

La ronda 1 es el primer `verify` de una entrega y la única ronda que ejecuta los sondeos y los mutantes manuales independientes. Las rondas 2 y posteriores son rondas de cierre.

| Ronda | Qué hace `verify` |
|---|---|
| 1 | Puertas, mapa de criterios, sondeos y mutantes manuales independientes (V2 y V3) |
| 2+ (cierre) | Solo puertas, comprobaciones de integridad de artefactos y cierre de hallazgos anteriores; sin presupuesto de mutantes independientes |

- Un `verify` posterior a una ronda de corrección, a una ronda de corrección de T4, o el `verify` final del change, es una ronda de cierre, salvo que el coordinador indique que es el primer verify de ese código.
- Una laguna de pruebas nueva hallada después de la ronda 1 es una advertencia (WARNING) de clase `implementation` marcada `follow-up`. No bloquea, en ningún modo ni con ningún valor de `mutation`, y no cuesta una ronda. El coordinador la lista para el titular y `archive` la añade a la lista de seguimiento de `specs/README.md`.
- Un defecto real de código mostrado por un sondeo, un fallo de puerta y un superviviente crítico en el informe más reciente de `mutants` son CRITICAL en cualquier ronda.
- Un superviviente crítico de T4 va a `tdd-apply`, después T4 se ejecuta de nuevo sobre ese crate, y luego un `verify` de ronda de cierre lee el informe nuevo. Solo ese informe nuevo prueba el cierre; un informe antiguo deja el superviviente como CRITICAL.

### verification.json

`verify` escribe `verification.json` junto a `verify-report.md` en toda profundidad superior a V0: en `specs/changes/<change>/`, o en la ruta temporal que le da el coordinador cuando no hay carpeta. Contiene un objeto por ronda de verify en un array `rounds`: ronda, commit verificado, modo, nivel, profundidad, veredicto, `next`, recuentos, hallazgos (id, severidad, `class`, referencia, resumen), códigos de salida de las puertas, recuentos de mutación y tiempo, mutantes manuales, criterios cubiertos y tamaño medido. El coordinador lee este archivo, no el informe largo, para enrutar, y copia los hallazgos en la entrada de outcome cuando no hay carpeta. Si ambos difieren, prevalece el informe y la ronda se repite.

El archivo es un piloto. Si el enrutado no lo lee tras archivar el siguiente change que lo use, se elimina.

## Niveles de mutación

Los niveles fijan cuánto código mutan `mutants` y `verify`. El [capítulo de herramientas](09-tooling-and-quality.md) indica los comandos.

| Nivel | Alcance | Se aplica a |
|---|---|---|
| T0 | Ninguno | No se toca código Rust |
| T1 | Solo mutantes manuales de `verify`: de 2 a 3 mutantes independientes | Cambios mínimos en código no crítico |
| T2 | `cargo mutants --in-diff`: líneas cambiadas | `mutation: delivery`, entregas que cambian Rust |
| T3 | `--file` por cada archivo fuente no de prueba cambiado | `mutation: delivery`; entregas que editan un archivo crítico existente |
| T4 | Crate completo | Cierre del change, antes de archivar, en cada crate que toca el change; el valor por defecto (`mutation: archive`) |

El coordinador sube un nivel cuando una entrega edita un archivo crítico existente o añade una regla de un tipo nuevo. Solo el titular baja un nivel. `--iterate` se permite únicamente en rondas de corrección solo de pruebas con el mismo conjunto de archivos, nunca en la ronda que cierra una entrega ni en T4. En modo light `mutants` se ejecuta solo desde T2.

### Interruptor de mutación

El campo `mutation` de la línea `Flow:` elige cuándo se ejecuta `cargo-mutants` (ADR 0002, sección 1).

| Valor | Efecto |
|---|---|
| `archive` (por defecto) | Sin ejecución por entrega. Una ejecución T4 por crate tocado al cerrar el change, antes de archivar. Un superviviente crítico vuelve a `tdd-apply`, después T4 se ejecuta de nuevo sobre ese crate y luego un `verify` de ronda de cierre lee el informe nuevo |
| `delivery` | T2 o T3 por entrega (`--iterate` en rondas de corrección solo de pruebas), más T4 al cerrar. Solo a petición del titular, o cuando el coordinador señala una regla nueva sobre código crítico |
| `off` | Sin `cargo-mutants`. Los mutantes manuales de `tdd-apply` y `verify` siguen aplicándose |

### Mutantes manuales

Los mutantes manuales muestran que una prueba puede fallar. Su presupuesto se reparte entre los dos agentes, en cualquier modo y con cualquier valor de `mutation`. Ninguna parte sustituye a T4.

| Agente | Trabajo con mutantes manuales |
|---|---|
| `coordinator` | En código crítico, entrega a `tdd-apply` una lista a nivel de regla: cada elemento nombra una regla de la spec y su comportamiento erróneo, sin código |
| `tdd-apply` | Aplica cada elemento, confirma que una prueba falla, lo revierte y registra el elemento y su prueba que lo mata en `apply-progress.md`; un superviviente exige una prueba nueva, un elemento que no se puede aplicar queda `blocked` y `done` exige todos muertos |
| `verify` (V2 y V3, ronda 1) | No repite la lista. Comprueba un elemento aplicándolo y confirmando que falla la prueba registrada, y añade de 2 a 3 mutantes manuales independientes elegidos a partir de las reglas de la spec |

Sin lista registrada no hay comprobación puntual, y los 2 a 3 mutantes independientes siguen aplicándose. El nivel T1 depende solo de ellos. Un mutante manual superviviente es de clase `implementation`, y CRITICAL en código crítico.

## Control del ciclo

El coordinador controla el ciclo desde la solicitud hasta el archivado. Cada agente devuelve su resultado al coordinador y ninguno llama a otro; el campo `next` de un resultado es una recomendación. Los subagentes no pueden lanzar subagentes, así que el coordinador debe ser el agente del hilo principal.

`verify` asigna una clase a cada hallazgo CRITICAL y WARNING. El coordinador enruta por clase y nunca corrige un hallazgo por sí mismo.

| Clase | Significado | Ruta |
|---|---|---|
| `implementation` | Falta de código o de prueba, fallo de una puerta, mutante crítico superviviente, tamaño | `tdd-apply`, después `verify` y `mutants` |
| `architecture` | Descomposición, decisión o ADR erróneos | `architect`, después `specifier` si cambian los requisitos, después `tdd-apply` |
| `ambiguity` | Requisito ausente o contradictorio | `specifier`; las preguntas abiertas van al titular |
| `claim` | Afirmación riesgosa sin demostrar | `explorer` |

Un informe mixto se enruta primero hacia arriba: ambigüedad, después arquitectura, después implementación. Un veredicto BLOCKED no es una clase: va directamente al titular.

### Límites del bucle

- Tras dos FAIL consecutivos de la misma clase en una entrega, el coordinador se detiene y pregunta al titular, con una pregunta y una recomendación.
- Una ejecución BLOCKED no cuesta una ronda; sin carpeta, las paradas leen los `rounds` de la entrada y omiten las filas BLOCKED.
- Una entrega tiene como máximo tres rondas de verify entre todas las clases. Una cuarta ronda es una decisión del titular. Un ciclo de corrección de T4 cuenta como una ronda de verify de la última entrega del change.
- En modo light, el segundo FAIL de cualquier clase sube el modo en su lugar.
- El coordinador nunca aprueba por sí mismo un FAIL, dispensa un hallazgo ni sube un tope en silencio. Los asuntos exclusivos del titular (decisiones de `specs/00-decisiones.md`, fusiones, precios, licencia, alcance) nunca se enrutan a un agente.

Estos límites son señales para detenerse y preguntar, no bloqueos estrictos. El techo de una entrega es de unas 700 líneas; superarlo requiere la aprobación explícita del titular.

## TDD estricto

RED (escribir la prueba, ejecutarla, verla fallar), GREEN (código mínimo), REFACTOR (las pruebas siguen en verde). Un commit por unidad de trabajo revisable. Cada commit pasa por sí solo las tres puertas locales (véase el capítulo de herramientas). Cuando una prueba pasa de inmediato porque el código ya existe, el RED se demuestra con un mutante (véase la sección de pruebas de mutación del capítulo de herramientas).

## Specs

| Lugar | Contenido |
|---|---|
| `specs/00-decisiones.md` | Decisiones del titular (D-nn). Prevalecen sobre las demás specs y no se reabren sin el titular |
| `specs/NN-*.md` | El documento en curso; los completados pasan a `specs/archive/` con `git mv`, los que esperan están en `specs/backlog/` |
| `specs/capabilities/<capability>/spec.md` | Specs principales por capacidad, la fuente de verdad, completadas al archivar |
| `specs/changes/<change>/` | Carpeta de change activa, solo para L3, modo full, trabajo que ninguna spec cubre y carpetas ya en curso: `proposal.md`, `design.md`, `tasks.md` (opcionales), `specs/<capability>/spec.md` (delta specs; ninguna en una ejecución light o de carril rápido), `apply-progress.md` (empieza con la Light spec o la Slice card), `verify-report.md`, `verification.json` |
| `specs/history/<date>-<change>/` | Carpetas de change cerradas |
| `.odd/state.json` | Entradas de outcome de los outcomes activos (véase [ODD lite](#odd-lite)) |

Formato de spec: palabras clave de RFC 2119; escenarios como Given/When/Then; cada requisito etiquetado con su entrega (`D1`, `D2a`...) y cada escenario con su tipo (unit, property, integration, command, manual). Las delta specs usan `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements` (con Reason y nota de Migration) y `## RENAMED Requirements`. El archivado es el único paso que edita las specs principales, solo en changes con delta specs, y avisa antes de cualquier delta destructiva. Los changes de carril rápido no tienen ninguna, así que el trabajo nuevo de carril rápido no edita `specs/capabilities/`.

Los changes cerrados están en `specs/history/<date>-<change>/`, nombrados por la fecha de cierre. Su texto se conserva tal como estaba al cerrarse.

## Commits, ramas y pull requests

- Conventional commits; sin líneas de atribución de IA.
- `main` contiene el código publicado y estable, y `develop`, creada desde `main`, es la rama de integración; son las únicas ramas de vida larga (ADR 0003). Cada change y cada entrega es una rama de vida corta creada desde `develop` (`feat/<change>-<delivery>`, `docs/...`, `chore/...`).
- Cuando una entrega pasa verify, el coordinador fusiona su rama en `develop` en local con `git merge --no-ff`, ejecuta las puertas una vez sobre `develop`, empuja `develop` y borra la rama. No hay pull request por entrega ni por change; las entregas de un change se fusionan en orden, y un FAIL nunca llega a `develop`.
- El único pull request es `develop` -> `main`, y se abre solo cuando el titular lo pide o en un punto digno de versión (una versión o etiqueta, un documento de producto completado, un hito que nombre el titular); el coordinador puede sugerirlo en esos puntos y solo lo abre con el visto bueno del titular. El titular lo revisa y lo fusiona con un commit de fusión; las versiones y etiquetas se crean desde `main` tras esa fusión, y el coordinador sincroniza `develop` con `main` (avance rápido cuando es posible) y la empuja.
- El estado vive en la entrada de outcome, en la carpeta del change cuando la hay, y en git, no en una herramienta de memoria externa.
- La CI está desactivada manualmente en GitHub por ahora (facturación), así que las puertas de cada entrega se ejecutan en local antes de fusionarla en `develop`; véase el capítulo de herramientas.

## Reglas de código e idioma

- Rust edición 2024, toolchain 1.93.1; `unsafe` está prohibido; `#[allow]` no se usa para silenciar lints.
- Los crates de dominio usan `BTreeMap` y `BTreeSet`, nunca `HashMap` ni `HashSet`.
- Los artefactos (código, comentarios, documentos, specs, commits) están en inglés. El texto de la interfaz de usuario es en español e inglés (D-08). Los documentos de producto bajo `specs/` están en español.
- Sin rutas absolutas ni de usuario en archivos versionados. Las skills se referencian solo por nombre.

## Documentación

Este manual vive en `docs/manuals/tecnico/` y se mantiene con `manual.py` (véase el README de la carpeta). Un cambio que altere una API pública, una regla de capas, una puerta o el proceso actualiza el capítulo correspondiente, en ambos idiomas, en el mismo pull request, y marca como **Planificado** lo que aún no esté construido.
