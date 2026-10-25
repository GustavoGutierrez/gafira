# Modelo de edición, comandos e historial

## Estado

| Parte | Estado |
|---|---|
| Trait `Command`, `Applied`, `CommandError`, `DirtySet`, `MergeKey`, `GestureId` | Implementado en `gafira-edit` |
| `Transaction` (Composite) | Implementado |
| `History` (deshacer, rehacer, instantáneas, presupuesto de memoria, fusión de gestos), comandos del núcleo, `debug_assert` tras cada comando, `gafira-testkit`, `trybuild` | **Planificado**: change abierto `openspec/changes/edit-history` |
| `EditService`, `Session`, `SelectionService`, bus de eventos | **Planificado**: capa 3, `gafira-engine` |
| `Selection`, `HitTester`, `Tool` | **Planificado** |

El crate es código de dominio de la capa 1: sin I/O, sin relojes, sin `unsafe`. Depende de `gafira-geom`, `gafira-color` y `gafira-doc`. Sus elementos públicos son `Command`, `Applied`, `GestureId`, `MergeKey`, `DirtySet`, `CommandError` y `Transaction`. No debe reexportar públicamente `EditToken` (CI V13).

## El contrato Command

Todo cambio de documento es un comando (patrón Command, spec 02 sección 4.3). `Command: Any + Send + Sync + Debug`:

| Método | Contrato |
|---|---|
| `label() -> CommandLabel` | Clave de mensaje mostrada como «Deshacer {label}»; sin texto, solo una clave |
| `apply(&mut self, &mut DocMut) -> Result<Applied, CommandError>` | Aplica el cambio. Devuelve `Changed` cuando el comando es un paso deshacible y `Unchanged` cuando el documento ya tenía el resultado. Un `apply` fallido DEBE dejar el documento igual a su estado previo (ARQ-13) |
| `revert(&mut self, &mut DocMut) -> Result<(), CommandError>` | Restaura el estado anterior al último `apply` exitoso |
| `merge_key() -> Option<MergeKey>` | Por defecto `None`; la clave bajo la que este comando puede fusionarse con uno posterior |
| `merge(&mut self, next: Box<dyn Command>) -> Result<(), Box<dyn Command>>` | Por defecto devuelve `next` intacto. Una sobrescritura recupera el tipo concreto sondeándolo primero por referencia (`(&*next as &dyn Any).is::<Own>()`), porque un downcast fallido por valor no puede devolver la caja |
| `touches() -> DirtySet` | Páginas y nodos que este comando cambia |

Los comandos reciben `&mut DocMut<'_>`, la vista de capacidad de `gafira-doc`. No guardan un `EditToken` por sí mismos; lo guarda el dueño del documento.

### Tipos de apoyo

- `Applied`: `Changed` o `Unchanged`.
- `GestureId(pub u64)`: nombra un arrastre u otro gesto de usuario para que sus comandos puedan fusionarse.
- `MergeKey { kind: &'static str, page: Option<PageId>, nodes: Vec<NodeId>, gesture: Option<GestureId> }`. Dos comandos se fusionan solo cuando sus claves son iguales; `History` decide cuándo (mismo gesto, o dentro de la ventana de fusión).
- `DirtySet { pages: BTreeSet<PageId>, nodes: BTreeSet<NodeId> }` con `union`. Una página figura cuando cambian sus propios campos o sus `roots`; un nodo, cuando cambian su registro o sus `children`. Su objetivo es invalidar cachés en la capa de render.
- `CommandError` (no exhaustivo): `MissingPage`, `MissingNode`, `NotAContainer`, `IndexOutOfRange { index, len }`, `WouldCreateCycle`, `InvalidValue(&'static str)`, `InvalidDocument(ValidationError)`, `IdSpaceExhausted`, `NotApplied`. Es `Clone`, `PartialEq` e implementa `std::error::Error`.

## Transaction

`Transaction::new(label, commands)` es un comando Composite: muchos comandos, una entrada de historial, todo o nada (ADR-03).

- `apply` ejecuta los hijos en orden. Si un hijo falla, los hijos ya aplicados se revierten en orden inverso y se devuelve el error del hijo que falló, de modo que el documento termina como empezó. El resultado es `Changed` si algún hijo cambió el documento.
- `revert` revierte los hijos en orden inverso.
- `touches` es la unión de los conjuntos de los hijos.

## Planificado: comandos del núcleo

El change `edit-history` planifica, para el modelo que existe hoy (`Group`, `Shape`, `Extension`):

- `SetProperty`: campos de nodo (`name`, `placement`, `opacity`, `blend`, `visible`, `locked`), `fill` y `geometry` de las formas, campos de página (`name`, `size`, `locked`, `hidden`) y el DPI del documento.
- `InsertNodes`, `RemoveNodes`, `Reorder`, `Reparent` (dentro de una página), `InsertPage`, `RemovePage`.

Los comandos para tipos o datos que aún no existen (clips, fotogramas clave, máscaras, plantillas, versiones) quedan fuera de alcance hasta que exista su modelo. Mover nodos entre páginas y aplicar bloqueos en los comandos también quedan fuera de alcance de ese change.

## Planificado: History

`History` está especificado como Command más Memento (ADR-03). El diseño en `openspec/changes/edit-history` planifica:

- Es dueño del `Arc<Document>` de la sesión; operaciones `execute`, `undo`, `redo` y `preview`.
- Una instantánea por entrada. Como `Document` comparte páginas y nodos mediante `Arc`, una instantánea cuesta solo el camino cambiado.
- Un cambio nuevo vacía la pila de rehacer.
- Fusión de gestos: `GestureId` explícito más una ventana de 500 ms sobre un tiempo aportado por quien llama. El crate no lee ningún reloj (ARQ-02), así que el tiempo se pasa desde fuera.
- Un presupuesto de memoria, por defecto 512 MiB (536 870 912 bytes), con compactación a solo comandos y `revert` para las entradas compactadas.
- `debug_assert!(doc.validate().is_ok())` tras cada comando aplicado, deshacer y rehacer (ARQ-13); los comandos comprueban sus propias entradas en todas las compilaciones.
- El historial nunca se guarda con el documento.

Son decisiones de diseño de un change abierto y aún pueden cambiar; trate la carpeta del change como fuente de verdad hasta que se archive.
