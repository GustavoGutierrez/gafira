# Glosario

| Término | Significado |
|---|---|
| ADR | Registro de decisión de arquitectura en la spec 02 (por ejemplo ADR-01 granularidad de crates, ADR-03 historial) |
| ARQ-nn | Requisito de arquitectura con criterio de aceptación, en la spec 02 |
| Arena | El `BTreeMap<NodeId, Arc<Node>>` de cada página que es dueño de los nodos de la página |
| JSON canónico | La forma compacta determinista que escribe `Document::to_json` |
| Command | Una edición deshacible de un documento; el trait `Command` de `gafira-edit` |
| Copia al escribir | Mutación mediante `Arc::make_mut`, que copia solo los elementos compartidos a lo largo del camino editado |
| D-nn | Una decisión del titular en `docs/specs/00-decisiones.md`; prevalece sobre las demás specs |
| Entrega | Una porción revisable de un change (`D1`, `D2a`...), construida con TDD estricto |
| DirtySet | Las páginas y nodos que un comando cambia, usado para invalidar cachés |
| `DocMut` | La vista de edición exclusiva de un `Document`, obtenida con un `EditToken` |
| Crate de dominio | Un crate de las capas 0 y 1 con un `clippy.toml` y una fila V5 |
| `EditToken` | La prueba no copiable de que quien la posee puede editar documentos |
| Gesto | Un arrastre u otra acción del usuario, nombrado por un `GestureId`, cuyos comandos pueden fusionarse |
| Archivo golden | Una salida esperada almacenada que se compara byte a byte en las pruebas |
| Arquitectura hexagonal | Puertos y adaptadores: el dominio es puro y los adaptadores implementan puertos |
| SDD ligero | El flujo spec, apply, verify, archive de este repositorio |
| MergeKey | La clave bajo la que dos comandos pueden fusionarse en una sola entrada de historial |
| Mutante | Un pequeño cambio deliberado de código hecho por `cargo-mutants`; un mutante superviviente indica una laguna de pruebas |
| OpenSpec | La organización de specs principales y carpetas de changes bajo `openspec/` |
| P1 a P6 | Las pasadas ordenadas de `Document::validate` |
| Planificado | Especificado pero aún no implementado en el repositorio |
| Puerto | Un trait definido por la capa de aplicación que implementan los adaptadores (**Planificado**, `gafira-ports`) |
| Transaction | Un comando compuesto: muchos comandos, una entrada de historial, todo o nada |
| V5, V12, V13 | Pasos de política de la CI: tabla de capas, sincronía de `clippy.toml`, confinamiento de `EditToken` |
| Forma en disco | La forma JSON de un tipo guardado |
