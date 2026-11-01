# Especificación de migración a ODD ligero (Outcome-Driven Development)

**Tipo:** instrucciones ejecutables para un agente de programación sobre un repositorio existente  
**Objetivo:** sustituir o simplificar un flujo SDD documentalmente pesado por ODD ligero, preservando agentes, capacidades y verificaciones útiles.  
**Prioridad:** entregar software funcional, probado y verificable con el mínimo trabajo documental necesario.  
**Estado:** propuesta para evaluación y aplicación *in situ*. No presupone que el repositorio usa una estructura concreta.

---

## 1. Instrucción principal al agente

> Inspecciona primero el repositorio, sus instrucciones, sus skills, subagentes, comandos y configuración SDD existente. Diseña e implementa la migración incremental a **ODD ligero**, sin eliminar capacidades ni cambiar comportamientos de producción. Conserva especialmente la función del agente `verify` (o su equivalente). No crees una segunda infraestructura paralela si puedes adaptar la actual. Comprueba la migración mediante un cambio piloto real o aislado y presenta evidencia verificable.

**No interpretes ODD como un estándar externo con comandos fijos:** es la metodología definida por este documento. Adapta nombres, rutas y mecanismos a las convenciones reales del proyecto y del agente que se utiliza (Alisio, Claude Code, OpenCode u otro). No des por existente un ejecutor, runtime de skills o almacenamiento de estado; primero identifícalo.

## 2. Metas y no metas

### Metas

- Minimizar el tiempo desde solicitud hasta código validado, no solo el tiempo de redactar specs.
- Tener, como **orientación**, de 3 a 5 **especificaciones de capacidad** para un producto de alcance manejable; no imponer un máximo rígido a productos grandes.
- Reutilizar y actualizar la spec existente antes de crear otra.
- Descomponer la ejecución en tareas pequeñas **sin convertir cada tarea en un nuevo documento o `change/`**.
- Mantener contratos, decisiones relevantes, criterios de aceptación y pruebas.
- Reutilizar agentes especializados y `verify`; suprimir pasos redundantes, no controles importantes.
- Hacer el flujo reanudable tras interrupciones, evitando replantear lo ya acordado.
- Usar lectura selectiva para reducir tokens y escrituras innecesarias.

### No metas

- No reescribir la aplicación ni reorganizar `src/` por el mero cambio metodológico.
- No prometer un ahorro porcentual de tokens o velocidad sin medición.
- No eliminar pruebas, auditorías de seguridad necesarias ni aprobaciones obligatorias.
- No reescribir históricos de Git o destruir artefactos SDD anteriores.
- No adoptar automáticamente LeanSpec, OpenSpec o Spec Kit como dependencia.

## 3. Auditoría obligatoria previa

Antes de modificar archivos, entregar un inventario conciso de:

1. Archivos de instrucciones: `AGENTS.md`, `CLAUDE.md`, reglas del IDE, `CONTRIBUTING.md`, instrucciones en subcarpetas y precedencia real.
2. Agentes actuales: orquestador, autor de hard spec, autor Gherkin, implementador TDD, juez, `verify`, mutation tester y equivalentes **solo si existen**; rutas, disparadores, entrada, salida y permisos.
3. Skills/comandos: ubicaciones, esquema de metadatos, nombres, consumidores, herramientas y dependencias.
4. Flujo SDD vigente: PRP, hard spec, Gherkin, TDD, judge, mutation, BDD u otras fases existentes; qué se ejecuta de verdad, qué es opcional y qué repite trabajo.
5. Carpeta de documentación: specs, changes, plans, tareas, ADR, decisiones, duplicados y políticas de archivo.
6. Mecanismo de ejecución: delegación a subagentes, CLI, hooks, scripts, CI, sistema de estados (si existe), permisos y aislamiento.
7. Pruebas existentes: unitarias, integración, E2E, lint, tipos, seguridad; configuración y coste.

**Salida de la auditoría:** tabla «componente actual → mantener/adaptar/fusionar/deprecar → motivo → impacto → verificación». Si hay ambigüedades, elige la modificación reversible más pequeña y documenta la suposición. No sobrescribas configuraciones sin leerlas.

## 4. Modelo de ODD ligero

**Principio:** un *outcome* es un resultado observable y verificable para el usuario o para el sistema; una *spec* describe una capacidad estable; una *task* es trabajo ejecutable. Son cosas distintas.

```text
Solicitud / outcome
    ↓
Identificar capacidad y spec vigente
    ↓
Actualizar solo contratos/aceptación afectados
    ↓
Seleccionar siguiente lote ejecutable pequeño
    ↓
Implementar → pruebas → verify
    ↺ corregir hasta pasar
    ↓
Actualizar estado y entregar evidencia
```

### Regla de granularidad

- **Spec nueva** si nace una capacidad con límites propios, contratos relevantes o ciclo de vida distinto.
- **Spec existente** si se mejora, corrige o amplía una capacidad ya cubierta.
- **Tarea** si puede implementarse y comprobarse de forma acotada; no requiere carpeta de cambio propia.
- **ADR** solo ante decisión arquitectónica transversal o difícil de revertir.
- **Plan detallado** solo cuando hay dependencias, migraciones o riesgos que lo justifican.

**No imponer 5 specs a toda costa:** preservar cohesión y una navegación sencilla. Usar 3–5 specs iniciales como presupuesto de organización, no como límite de complejidad. Evitar tanto cientos de cambios microscópicos como un `spec.md` inmanejable.

### Política de documentación

**Obligatorio por capacidad:** propósito, alcance, requisitos observables, contratos/invariantes, criterios de aceptación verificables y dependencias importantes.  
**Opcional según riesgo:** diseño técnico, diagrama, ADR, matriz de amenazas, Gherkin extenso, plan de migración.  
**No crear por defecto:** `proposal.md`, `design.md`, `tasks.md`, `changes/<micro-feature>/`, copias delta, resúmenes repetitivos, bitácoras narrativas por cada ejecución.

La tarea activa puede vivir en el sistema de issues, un archivo compacto de estado o en la memoria transaccional ya existente. **No asumir SQLite:** reutilizar el sistema del proyecto; si no hay ninguno, empezar con un archivo pequeño en lugar de introducir una base de datos.

## 5. Estructura sugerida (ajustar al repositorio)

```text
AGENTS.md
.odd/                         # opcional: solo si no hay mecanismo equivalente
  state.json                   # progreso mínimo y reanudación; no duplicar issues
  metrics.jsonl                # opcional para comparar ejecuciones
specs/
  PRODUCT.md                   # contexto y límites comunes
  01-capacidad-a.md
  02-capacidad-b.md
  03-capacidad-c.md
  ...
docs/adr/                    # solo decisiones relevantes, si ya existe convención
```

Para **Gafira** como ejemplo orientativo: `01-editor-lienzo.md`, `02-elementos-composicion.md`, `03-proyectos-persistencia.md`, `04-plantillas-recursos.md`, `05-exportacion-plataforma.md`; y `PRODUCT.md` para arquitectura compartida. Los nombres y la partición deben ajustarse a los límites reales del código. No migrar la documentación histórica a la fuerza.

### Plantilla mínima de spec

```markdown
---
id: editor-lienzo
status: active
owner: editor
---
# Editor y lienzo
## Resultado esperado
[Capacidad observable]
## Alcance / fuera de alcance
[Qué incluye y qué excluye]
## Contratos e invariantes
[Restricciones, interfaces, datos, seguridad]
## Comportamientos requeridos
[Requisitos concretos y comprobables]
## Criterios de aceptación
- [ ] [Comportamiento y evidencia esperada]
## Dependencias / decisiones
[Enlaces; no repetir otros documentos]
```

Si la spec ya existe con otro formato útil, **editarla en vez de convertirla automáticamente**.

## 6. Cambios en AGENTS.md

Inspecciona `AGENTS.md` y preserva todas las políticas de seguridad, entorno, arquitectura, estilo, herramientas y aprobaciones. **Agregar o integrar** una sección como esta, adaptada a los comandos reales:

```markdown
## Metodología de desarrollo: ODD ligero

- Prioriza resultados verificables y cambios de código sobre artefactos ceremoniales.
- Lee primero el código y la spec de capacidad relevante; no cargues todas las specs sin necesidad.
- Una solicitud se asocia a una spec existente salvo que introduzca una capacidad genuinamente nueva.
- No generes un change, propuesta, diseño, plan ni spec por cada archivo, endpoint o subtarea.
- Antes de implementar, garantiza criterios de aceptación y restricciones suficientes; aclara solo bloqueos reales.
- Ejecuta lotes pequeños: prueba (cuando aplique) → implementación → pruebas → verificación.
- Invoca `verify` para la validación independiente del resultado y corrige los hallazgos.
- Actualiza specs solo cuando cambien requisitos, interfaces o decisiones duraderas; no por cada movimiento interno.
- Registra avance de modo reanudable en el mecanismo de estado existente sin duplicar información.
- Ante cambios riesgosos (auth, pagos, permisos, migraciones, pérdida de datos, seguridad), eleva el rigor de diseño y verificación.
- No declares «completo» sin evidencia de criterios de aceptación y pruebas ejecutadas.
- No elimines instrucciones de mayor precedencia; respeta la configuración propia del runtime.
```

**Nota:** `AGENTS.md` es orientación para agentes que la leen; los comandos y subagentes no aparecen mágicamente por añadir texto. Instala/adapta cada skill o comando en el directorio correcto de la plataforma real. Si `CLAUDE.md` u otro archivo manda, utiliza el mecanismo de importación o documentación correspondiente para evitar instrucciones contradictorias.

## 7. Skills propuestas

Crear la menor cantidad necesaria y reutilizar las existentes:

| Skill | Responsabilidad | ¿Por qué? | Resultado |
|---|---|---|---|
| `odd-spec` | Localiza/crea/actualiza la spec de capacidad; define criterios verificables | Evitar nueva carpeta y cuatro artefactos por cada cambio | Diff pequeño a una spec, o sin cambio si ya cubre la solicitud |
| `odd-run` | Orquesta el lote ejecutable y delega implementación/TDD | Pasar pronto de requisito a código, conservando progreso | Código, tests, estado, evidencias |
| `odd-verify` **o reutilizar `verify`** | Evalúa aceptación, pruebas, regresiones y riesgos | Mantener una puerta de calidad independiente | PASS / FAIL / BLOCKED con evidencias |

**No crear `odd-verify` si ya existe `verify` apto:** integrarlo como dependencia de `odd-run`. `odd` puede ser un comando de entrada opcional que compone `odd-spec` y `odd-run`, sin duplicar lógica. Las rutas de skills dependen del host; detectarlas antes de crear archivos.

### Contrato de `odd-spec`

**Entrada:** objetivo, código relevante, specs existentes, límites y riesgos.  
**Proceso:** seleccionar spec afectada, inspeccionar contratos, escribir solo los requisitos que falten, definir aceptación observable; no crear por default una propuesta o diseño.  
**Salida compacta:** `spec_id`, `outcome`, `acceptance_criteria`, `constraints`, `affected_areas`, `risks`, `next_action`.  
**Criterio de terminación:** suficiente precisión para escribir pruebas y comenzar a implementar.

### Contrato de `odd-run`

**Entrada:** spec y outcome, estado reanudable, herramientas disponibles.  
**Proceso:** inspeccionar código; seleccionar lote pequeño; asignar implementador existente; ejecutar pruebas; invocar `verify`; corregir; actualizar estado; repetir.  
**Salida:** cambios concretos, criterios cumplidos, comandos de pruebas y resultados, riesgos pendientes, siguiente lote.  
**Límite:** no planificar todo el proyecto a nivel de archivo antes de comenzar; no crear subagentes por tarea pequeña si la delegación añade más fricción que valor.

### Contrato de `verify` preservado

El agente `verify` existente debe ser **independiente del implementador** en cuanto al criterio de aceptación, sin depender de que quien escribió el código declare que pasó. Debe:

1. Comparar implementación contra criterios de aceptación y contratos.
2. Ejecutar o comprobar evidencia de tests, tipos, lint y pruebas relevantes según el riesgo.
3. Analizar regresiones, escenarios negativos y seguridad cuando aplique.
4. Emitir `PASS`, `FAIL` o `BLOCKED`, con pruebas de soporte y defectos reproducibles.
5. Devolver los fallos a `odd-run` sin abrir nuevas specs por cada defecto.
6. Escalar a revisión humana cuando una verificación crítica no pueda realizarse.

No exigir que `verify` ejecute siempre la suite completa de E2E para cambios triviales, pero sí cuando los riesgos o dependencias lo justifiquen.

## 8. Agentes: mantener, fusionar o hacer condicionales

| Rol actual (si existe) | Acción recomendada | Motivo |
|---|---|---|
| Orchestrator | **Mantener/adaptar** para clasificación y coordinación ODD | Un punto de decisión; evita pipelines automáticos para tareas simples |
| Hard spec author / PRP author | **Fusionar funcionalmente en `odd-spec`** o reutilizarlo | Un contrato compacto por capacidad |
| Gherkin author | **Invocar bajo demanda** (BDD, flujos complejos) | Evita Gherkin ceremonial; conserva escenarios valiosos |
| TDD implementer | **Mantener** | Sigue siendo el ejecutor principal |
| `verify` | **Mantener y fortalecer** | Control de aceptación y regresiones |
| Judge | **Fusionar con `verify` cuando se solapen**; conservar separado si evalúa propiedades diferentes | Elimina doble revisión sin perder independencia necesaria |
| Mutation tester | **Opcional por riesgo / criticidad** | Evitar coste fijo en cada cambio |

**Importante:** no eliminar ni renombrar agentes que otros flujos invocan hasta localizar todas las referencias y dejar compatibilidad o migración explícita. Separar roles lógicos no obliga a iniciar procesos de agente adicionales.

## 9. Política de enrutamiento por riesgo

| Nivel | Ejemplos | Flujo mínimo |
|---|---|---|
| L0 — trivial | Texto, estilos sin cambio conductual | Inspección → cambio → verificación apropiada; spec solo si cambia contrato |
| L1 — rutinario | Bug acotado, ajuste de interacción | Criterio de aceptación → TDD/implementación → `verify` |
| L2 — funcional | Nueva herramienta de lienzo, endpoint, persistencia | Actualizar spec existente → lotes TDD → `verify` + integración |
| L3 — crítico | Migración de datos, seguridad, pagos, formato de proyecto | Spec + diseño/ADR si procede → plan de migración → pruebas amplias → `verify` y aprobación donde aplique |

El nivel lo determina **impacto + incertidumbre + reversibilidad**, no el número de archivos. El orquestador puede elevar el rigor si aparecen hallazgos nuevos.

## 10. Ejecución y estado reanudable

La unidad mínima de estado es un *outcome* con identificador estable:

```json
{
  "outcome_id": "editor-multiselect",
  "spec_id": "editor-lienzo",
  "status": "implementing",
  "criteria": [
    {"id": "AC-01", "status": "passed", "evidence": "tests/selection.test.ts"},
    {"id": "AC-02", "status": "pending"}
  ],
  "next_action": "Implementar agrupación y prueba de undo/redo"
}
```

Usar el issue tracker, la memoria del agente o una base de datos existente **si ya proporciona estas propiedades**. Si no, archivo mínimo atómico. Evitar duplicar estado en checkboxes Markdown, JSON, SQLite e issues a la vez. La spec define el contrato; el estado transitorio pertenece al ejecutor.

**Reanudación:** verificar branch, diff, tests y estado real antes de continuar; nunca confiar ciegamente en un checklist obsoleto. Ser idempotente: ejecutar dos veces un paso no debe duplicar specs ni sobrescribir código correcto.

## 11. Guardrails de desempeño y calidad

- Leer solo la spec pertinente y dependencias directas; ampliar contexto cuando haya evidencias de impacto transversal.
- Evitar reformular la misma solicitud en múltiples documentos.
- Hacer edición incremental; no regenerar la spec completa por cada tarea.
- No medir productividad por líneas escritas ni cantidad de documentos.
- No ejecutar todas las verificaciones caras tras cada línea; sí las necesarias antes de declarar terminada una capacidad.
- Conservar checkpoints de Git y cambios pequeños revisables; no realizar commits destructivos ni force push sin autorización.
- Garantizar compatibilidad con pipeline CI y reglas de permisos/herramientas ya existentes.
- Reconocer bloqueos reales (dependencias, secretos, servicios externos, aprobaciones) y no inventar evidencia.

## 12. Plan de migración incremental y reversible

### Fase A — inventario y línea base

1. Auditar instrucciones, agentes, skills, comandos y rutas de specs.
2. Identificar duplicidades y puntos donde se crean artefactos innecesarios.
3. Elegir una funcionalidad piloto representativa y registrar datos de referencia si están disponibles.
4. Proponer mapeo preciso de archivos a modificar y de mecanismos a conservar.

### Fase B — introducir ODD sin romper SDD

1. Añadir reglas de ODD a `AGENTS.md` o punto de instrucciones equivalente.
2. Crear/adaptar `odd-spec` y `odd-run` según el mecanismo nativo del proyecto.
3. Conectar `odd-run` con el implementador y el `verify` existentes.
4. Añadir enrutamiento L0–L3 al orquestador y mantener una opción de retorno a SDD.
5. Implementar almacenamiento mínimo de progreso **solo si falta**.

### Fase C — piloto

Ejecutar una solicitud acotada real con:

- Una única spec de capacidad ya existente o nueva.
- Un conjunto de criterios de aceptación.
- Tareas pequeñas sin carpetas `changes/` individuales.
- Tests y veredicto independiente de `verify`.
- Evidencia de que la continuación no vuelve a redactar el plan completo.

### Fase D — consolidación

1. Ajustar instrucciones según errores del piloto.
2. Marcar comandos SDD antiguos como legacy/deprecados solo después de probar equivalencias.
3. Añadir aliases de compatibilidad si hay usuarios o automatizaciones que los llaman.
4. Documentar el rollback y mantener historia Git.
5. No eliminar archivos históricos hasta que se autorice.

## 13. Comandos expuestos (conceptuales)

Los siguientes nombres son una **interfaz deseada**, no comandos existentes garantizados:

```text
/odd:spec   "Agregar agrupación de elementos en Gafira"
/odd:run    editor-lienzo --outcome agrupacion
/odd:verify editor-lienzo --outcome agrupacion
/odd        "Agregar agrupación de elementos en Gafira"  # opcional: flujo completo
```

Implementarlos como skills, slash commands, CLI o intents del orquestador según las capacidades reales. No crear capas de wrappers sin valor. Permitir ejecutar el flujo completo desde lenguaje natural.

## 14. Criterios de aceptación de la migración

- [ ] Se inventarían todos los agentes/skills/instrucciones involucrados y se identifica su función actual.
- [ ] `AGENTS.md` (o equivalente) refleja ODD sin borrar políticas ni contradecir instrucciones prioritarias.
- [ ] Una solicitud rutinaria actualiza una sola spec pertinente, o ninguna si la spec ya cubre el trabajo.
- [ ] No se crean carpetas `changes/` ni documentos de propuesta/diseño/tareas por defecto.
- [ ] La ejecución produce cambios de código y pruebas, no solo documentación.
- [ ] `verify` se ejecuta en los cambios que requieren validación y emite resultado comprobable.
- [ ] Las tareas se pueden reanudar sin duplicar artefactos.
- [ ] Los cambios de alto riesgo conservan diseño, revisión y verificaciones adecuadas.
- [ ] Los comandos antiguos críticos siguen funcionando o disponen de migración documentada.
- [ ] Se registra antes/después y se evita afirmar mejoras no medidas.

### Métricas recomendadas

| Métrica | Cómo medir | Qué se busca |
|---|---|---|
| Tiempo hasta primer cambio funcional | Inicio de solicitud → primer cambio ejecutable válido | Reducir |
| Tiempo hasta entrega verificada | Inicio → `verify: PASS` | Reducir sin bajar calidad |
| Tokens totales | Uso del proveedor/agente hasta PASS | Reducir |
| Tokens de documentación | Estimación por artefactos/llamadas si hay telemetría | Reducir |
| Artefactos generados | Archivos de metodología creados por outcome | Reducir |
| Reintentos/defectos | Fallos y correcciones hasta PASS | No empeorar |
| Regresiones | Fallos encontrados después de integrar | No aumentar |

Medir una funcionalidad comparable al flujo antiguo, con el mismo modelo, herramientas y expectativas de prueba siempre que sea posible. Si no hay línea base fiable, dejar explícito «no medido».

## 15. Qué se gana y qué se pierde

| Ganancias esperadas (hipótesis a validar) | Costes / riesgos asumidos | Mitigación |
|---|---|---|
| Menos documentos y escrituras | Menos historial formal de decisiones menores | Git + ADR cuando corresponde |
| Menos contexto redundante | Specs demasiado amplias o ambiguas | Contratos claros, lectura selectiva y criterios de aceptación |
| Inicio de implementación más temprano | Omisión de análisis necesario en cambios críticos | Router de riesgo L0–L3 |
| Reutilización de agentes existentes | Requerimiento de adaptar integración/orquestación | Migración reversible y piloto |
| Progreso reanudable sin planes repetitivos | Riesgo de desincronización con estado real | Validar diff y tests en cada reanudación |
| Menos fragmentación de capacidades | Riesgo de specs gigantes | División por dominio coherente, no por tarea |

## 16. Entregables exigidos al agente que aplique esta especificación

1. **Diagnóstico:** inventario y mapa de impacto de archivos/roles reales.
2. **Cambios implementados:** diff de instrucciones, skills, comandos y/o orquestación pertinentes.
3. **Compatibilidad:** lista de flujos anteriores conservados y deprecaciones explícitas.
4. **Piloto verificable:** spec afectada, cambio de código, tests y resultado de `verify`.
5. **Comparación:** métricas antes/después o justificación de ausencia de datos.
6. **Rollback:** instrucciones para volver al flujo anterior sin pérdida de datos.
7. **Resumen final breve:** qué cambió, qué ganó, qué no se pudo probar y riesgos pendientes.

### Condición de aceptación final

> La migración solo está terminada cuando el agente puede tomar una solicitud de funcionalidad, identificar o actualizar una spec de capacidad, implementar código con tests y obtener un veredicto reproducible de `verify`, **sin fabricar un conjunto de documentos por cada subtarea**.

---

**Nota de implementación:** el diseño favorece rapidez de extremo a extremo, no «cero documentación». El criterio primordial es software correcto con el menor coste total de planificación, implementación y validación.
