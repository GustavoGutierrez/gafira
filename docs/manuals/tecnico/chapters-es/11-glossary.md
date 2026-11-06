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
| D-nn | Una decisión del titular en `specs/00-decisiones.md`; prevalece sobre las demás specs |
| Ronda de corrección | Una vuelta a `tdd-apply` tras un FAIL de verify, con las pruebas primero y una nueva verificación |
| Entrega | Una porción revisable de un change (`D1`, `D2a`...), construida con TDD estricto |
| DirtySet | Las páginas y nodos que un comando cambia, usado para invalidar cachés |
| `DocMut` | La vista de edición exclusiva de un `Document`, obtenida con un `EditToken` |
| Crate de dominio | Un crate de las capas 0 y 1 con un `clippy.toml` y una fila V5 |
| `EditToken` | La prueba no copiable de que quien la posee puede editar documentos |
| Flick | La unidad de tiempo exacta de `gafira-time`: 1/705 600 000 de segundo |
| Gesto | Un arrastre u otra acción del usuario, nombrado por un `GestureId`, cuyos comandos pueden fusionarse |
| Archivo golden | Una salida esperada almacenada que se compara byte a byte en las pruebas |
| Mutación manual | Un cambio deliberado de una línea en código terminado, aplicado a mano para mostrar que una prueba falla; se registra como selección, no como ejecución completa |
| Arquitectura hexagonal | Puertos y adaptadores: el dominio es puro y los adaptadores implementan puertos |
| History | El dueño del `Arc<Document>` de la sesión y de sus pilas de deshacer y rehacer (`gafira-edit`) |
| Ley | Una de las seis propiedades que `prop_command` comprueba para un comando |
| SDD ligero | El flujo spec, apply, verify, archive de este repositorio |
| Carril rápido | La ruta del ADR 0002 (en el trabajo cubierto, la entrada de outcome recoge ahora los campos de la tarjeta): una spec de producto se ejecuta directamente desde sus IDs de requisitos mediante una Slice card, sin propuesta, diseño ni delta specs |
| Clase de hallazgo | El `class:` de un hallazgo de verify (`implementation`, `architecture`, `ambiguity`, `claim`); el coordinador enruta según ella |
| Modo de flujo | Light, standard o full: el conjunto de agentes, puertas y comprobaciones que ejecuta una solicitud; lo elige el coordinador |
| Puerta (G0 a G5) | Una comprobación que hace el coordinador entre fases de un flujo; no es lo mismo que las puertas locales (`fmt`, `clippy`, `test`) |
| Light spec | El bloque de criterios de aceptación que el coordinador escribe al principio de `apply-progress.md` en modo light |
| Slice card | El bloque que el coordinador escribe al principio de `apply-progress.md` en el carril rápido: línea `Flow:`, IDs de producto, alcance, no objetivos, entregas y plan; satisface G0 |
| Nivel de mutación (T0 a T4) | Cuánto código cubre la ejecución de mutación: ninguno, mutantes manuales, diff, archivos cambiados, crate completo |
| Interruptor de mutación | El campo `mutation: off\|archive\|delivery` de la línea `Flow:`; fija cuándo se ejecuta `cargo-mutants` (por defecto `archive`: T4 por crate tocado al cerrar el change) |
| Pack | El modo, los agentes, el valor de `mutation` y la profundidad de verify predefinidos para un tipo de solicitud |
| `verification.json` | **Retirado**: el registro legible por máquina de cada ronda de verify que introdujo el ADR 0001; el ADR 0017 deja `verify-report.md` como el único archivo que escribe `verify` |
| Profundidad de verify (V0 a V3) | Cuánto trabajo independiente hace `verify`, desde solo comprobaciones deterministas hasta la matriz completa |
| MergeKey | La clave bajo la que dos comandos pueden fusionarse en una sola entrada de historial |
| Mutante | Un pequeño cambio deliberado de código hecho por `cargo-mutants`; un mutante superviviente indica una laguna de pruebas |
| NodeSpec | Un nodo descrito sin ids, la entrada de `InsertNodes` |
| BLOCKED | Un veredicto de `verify`: una comprobación crítica no pudo ejecutarse; va al titular, nunca cuenta como PASS y no cuesta una ronda |
| Carpeta de change | La carpeta `specs/changes/<change>/` con los artefactos de un change; se cierra en `specs/history/` |
| ODD lite | Enrutamiento por resultados sobre los modos de flujo (ADR 0011, aceptado): niveles de riesgo, entradas de outcome y BLOCKED |
| Entrada de outcome | El objeto de `.odd/state.json` que registra criterios, estado y siguiente acción de un outcome activo |
| Nivel de riesgo (L0 a L3) | La clase que el coordinador asigna a una solicitud por impacto, incertidumbre y reversibilidad; puede subir el modo de flujo y nunca baja un suelo |
| P1 a P6 | Las pasadas ordenadas de `Document::validate` |
| Planificado | Especificado pero aún no implementado en el repositorio |
| Puerto | Un trait de la capa 3 que implementan los adaptadores; los traits de puerto y sus registros viven en `gafira-ports` |
| Instantánea | El `Arc<Document>` anterior o posterior a una entrada del historial; comparte con sus vecinas las páginas y nodos sin cambios |
| `testkit` | `gafira-testkit`, el crate de soporte de pruebas con el generador de documentos y `prop_command` |
| `trybuild` | La dev-dependency que comprueba que los fixtures no compilen (o compilen); evidencia de ARQ-30 |
| Transaction | Un comando compuesto: muchos comandos, una entrada de historial, todo o nada |
| V14 | Paso de CI que comprueba que cada comando del núcleo tiene una prueba de propiedades (ARQ-81) |
| V5, V12, V13 | Pasos de política de la CI: tabla de capas, sincronía de `clippy.toml`, confinamiento de `EditToken` |
| Forma en disco | La forma JSON de un tipo guardado |
