# Glosario

| Término | Significado |
|---|---|
| ADR | Registro de decisión de arquitectura en la spec 02 (por ejemplo ADR-01 granularidad de crates, ADR-03 historial) |
| ARQ-nn | Requisito de arquitectura con criterio de aceptación, en la spec 02 |
| Presupuesto | El máximo de bytes estimados de instantáneas que un `History` conserva antes de compactar las entradas más antiguas; `DEFAULT_BUDGET` es 512 MiB |
| Arena | El `BTreeMap<NodeId, Arc<Node>>` de cada página que es dueño de los nodos de la página |
| JSON canónico | La forma compacta determinista que escribe `Document::to_json` |
| Compactación | Descartar las instantáneas de las entradas más antiguas del historial que superan el presupuesto; la entrada conserva su comando y se deshace con `revert` |
| Command | Una edición deshacible de un documento; el trait `Command` de `gafira-edit` |
| Copia al escribir | Mutación mediante `Arc::make_mut`, que copia solo los elementos compartidos a lo largo del camino editado |
| Lógica crítica | Código en el que un defecto corrompe datos o rompe un invariante (modelo de documento, JSON, ids, historial de edición, tiempo, color, geometría, límites, parsing de formatos); se construye con la prueba primero (TDD) |
| D-nn | Una decisión del titular en `specs/00-decisiones.md`; prevalece sobre las demás specs |
| Definition of Ready | La lista de secciones que necesita una spec antes de salir de `specs/backlog/`: problema, alcance, dependencias, requisitos, criterios de aceptación, casos de TDD, evidencia, riesgos |
| DirtySet | Las páginas y nodos que un comando cambia, usado para invalidar cachés |
| `DocMut` | La vista de edición exclusiva de un `Document`, obtenida con un `EditToken` |
| Crate de dominio | Un crate de las capas 0 y 1 con un `clippy.toml` y una fila V5 |
| `EditToken` | La prueba no copiable de que quien la posee puede editar documentos |
| Flick | La unidad de tiempo exacta de `gafira-time`: 1/705 600 000 de segundo |
| Puerta | Una comprobación de la sección «Before calling something done» de `AGENTS.md` (`fmt`, `clippy`, pruebas, pasos de política, `manual.py check`, `sync_agents.py --check`) |
| Gesto | Un arrastre u otra acción del usuario, nombrado por un `GestureId`, cuyos comandos pueden fusionarse |
| Archivo golden | Una salida esperada almacenada que se compara byte a byte en las pruebas |
| Mutación manual | Un cambio deliberado de una línea en código terminado, aplicado a mano y después revertido, para mostrar que una prueba puede fallar |
| Arquitectura hexagonal | Puertos y adaptadores: el dominio es puro y los adaptadores implementan puertos |
| History | El dueño del `Arc<Document>` de la sesión y de sus pilas de deshacer y rehacer (`gafira-edit`) |
| Ley | Una de las seis propiedades que `prop_command` comprueba para un comando |
| MergeKey | La clave bajo la que dos comandos pueden fusionarse en una sola entrada de historial |
| Mutante | Un pequeño cambio deliberado de código hecho por `cargo-mutants`; un mutante superviviente indica una laguna de pruebas |
| NodeSpec | Un nodo descrito sin ids, la entrada de `InsertNodes` |
| P1 a P6 | Las pasadas ordenadas de `Document::validate` |
| Documento de referencia de producto | Uno de `specs/00-..07-*.md`: decisiones del titular, producto, arquitectura, funcionalidades o deuda abierta; las specs lo citan, no es una spec de ciclo de vida |
| Planificado | Especificado pero aún no implementado en el repositorio |
| Puerto | Un trait de la capa 3 que implementan los adaptadores; los traits de puerto y sus registros viven en `gafira-ports` |
| Instantánea | El `Arc<Document>` anterior o posterior a una entrada del historial; comparte con sus vecinas las páginas y nodos sin cambios |
| Spec | Un archivo `specs/NNN-name.md` que describe una unidad de trabajo, con banner de estado, requisitos y criterios de aceptación |
| ID de spec | El número de tres dígitos, con ceros a la izquierda, de una spec (`009`); se asigna al crearla y nunca se reutiliza ni cambia |
| Banner de estado | La línea `> **Estado:**` bajo el título de una spec: `backlog`, `in-progress` o `done`, una fecha y la evidencia o el bloqueo |
| Script de sincronización | `tools/sync_agents.py`: enlaza `.claude/`, `.opencode/` y `.pi/` con `.agents/`; `--check` informa de la deriva |
| `testkit` | `gafira-testkit`, el crate de soporte de pruebas con el generador de documentos y `prop_command` |
| `trybuild` | La dev-dependency que comprueba que los fixtures no compilen (o compilen); evidencia de ARQ-30 |
| Transaction | Un comando compuesto: muchos comandos, una entrada de historial, todo o nada |
| V14 | Paso de CI que comprueba que cada comando del núcleo tiene una prueba de propiedades (ARQ-81) |
| V5, V12, V13 | Pasos de política de la CI: tabla de capas, sincronía de `clippy.toml`, confinamiento de `EditToken` |
| Forma en disco | La forma JSON de un tipo guardado |

## Términos históricos

Estos términos pertenecen al flujo multiagente anterior que retiró el ADR 0018. Aparecen en `specs/done/legacy/` y en los ADR reemplazados.

| Término | Significado |
|---|---|
| SDD ligero, ODD lite | El flujo anterior: un coordinador enrutaba cada solicitud por agentes del repositorio (spec, apply, verify, archive), con niveles de riesgo y modos de flujo (ADR 0001, 0002, 0004, 0011) |
| Change, carpeta de change | Una unidad de trabajo del flujo anterior y su carpeta de artefactos (propuesta, diseño, delta specs, informe de verify); las carpetas cerradas están en `specs/done/legacy/history/` |
| Delta spec | Los requisitos que un change añadía, modificaba o eliminaba en una spec por capacidad, fusionados al archivar |
| Entrega | Una porción revisable de un change (`D1`, `D2a`...) |
| Slice card | El bloque que definía el alcance y las entregas de un change de carril rápido |
| Entrada de outcome | El registro de un outcome activo en `.odd/state.json`, un archivo que ya no existe |
| Nivel de mutación (T0 a T4) | Cuánto código cubría una ejecución de mutación, desde nada hasta el crate completo |
