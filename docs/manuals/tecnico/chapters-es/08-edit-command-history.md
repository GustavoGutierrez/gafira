# Modelo de edición, comandos e historial

## Estado

| Parte | Estado |
|---|---|
| Trait `Command`, `Applied`, `CommandError` (con `RollbackFailed`), `DirtySet`, `MergeKey`, `GestureId` | Implementado en `gafira-edit` |
| `Transaction` (Composite) | Implementado |
| `History`: `Arc<Document>` propio, `execute` atómico, deshacer y rehacer por instantánea, `preview`, fusión de gestos, presupuesto de memoria con compactación | Implementado |
| `SetProperty` (ocho propiedades de nodo), `InsertNodes`, `RemoveNodes` | Implementado |
| `debug_assert` tras cada comando, deshacer y rehacer (ARQ-13) | Implementado en `History` |
| `gafira-testkit` (generador de documentos, `prop_command` con seis leyes) | Implementado |
| Fixtures compile-fail con `trybuild` para ARQ-30 | Implementado |
| `Reorder`, `Reparent` (entrega D9) | **Planificado**: en curso, sin fusionar |
| `InsertPage`, `RemovePage`, destinos de página y DPI de `SetProperty` (entrega D10) y paso de CI V14 | **Planificado** |
| `EditService`, `Session`, `SelectionService`, bus de eventos | **Planificado**: capa 3, `gafira-engine` |
| `Selection`, `HitTester`, `Tool` | **Planificado** |

El crate es código de dominio de la capa 1: sin I/O, sin relojes, sin `unsafe`. Depende de `gafira-geom`, `gafira-color`, `gafira-doc` y, para dimensionar cargas de extensión, de `serde_json`. Sus elementos públicos son `Command`, `Applied`, `GestureId`, `MergeKey`, `DirtySet`, `CommandError`, `Transaction`, `History`, `DEFAULT_BUDGET`, `SetProperty`, `Property`, `InsertNodes`, `NodeSpec` y `RemoveNodes`. No debe reexportar públicamente `EditToken` (CI V13). Las fuentes de verdad de este capítulo son el código bajo `crates/gafira-edit/`, `openspec/specs/` y el change abierto `openspec/changes/edit-history`.

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
- `CommandError` (no exhaustivo): `MissingPage`, `MissingNode`, `NotAContainer`, `IndexOutOfRange { index, len }`, `WouldCreateCycle`, `InvalidValue(&'static str)`, `InvalidDocument(ValidationError)`, `IdSpaceExhausted`, `NotApplied`, `RollbackFailed { apply, revert }`. Es `Clone`, `PartialEq` e implementa `std::error::Error`.

## Transaction

`Transaction::new(label, commands)` es un comando Composite: muchos comandos, una entrada de historial, todo o nada (ADR-03).

- `apply` ejecuta los hijos en orden. Si un hijo falla, los hijos ya aplicados se revierten en orden inverso (se intenta cada reversión, aunque una falle) y se devuelve el error del hijo que falló, de modo que el documento termina como empezó. Si una de esas reversiones también falla, el documento puede no ser igual a su estado previo y el error es `CommandError::RollbackFailed { apply, revert }`, que lleva el error original y el primer error de reversión; `History` aplica sobre una copia, así que simplemente la descarta. El resultado es `Changed` si algún hijo cambió el documento.
- `revert` revierte los hijos en orden inverso y se detiene en su primer error.
- `touches` es la unión de los conjuntos de los hijos.

## History

`History` es Command más Memento (ADR-03) y el único lugar donde los comandos se ejecutan sobre el documento de la sesión. Es dueño de un `Arc<Document>`; no existe un acceso `&mut Document` (un fixture compile-fail lo demuestra).

| Operación | Comportamiento |
|---|---|
| `History::new(doc)`, `History::with_budget(doc, budget)` | Validan `doc` y devuelven `ValidationError` si es inválido. `new` usa `DEFAULT_BUDGET` |
| `execute(cmd, at: Duration)` | Atómico: ejecuta `cmd` sobre una copia que comparte todas las páginas y nodos que no toca. `Changed` instala la copia y registra el comando como paso deshacible; `Unchanged` no registra nada; `Err` deja el documento como estaba. Un cambio registrado vacía la pila de rehacer |
| `undo()`, `redo()` | Devuelven `Result<Option<CommandLabel>, CommandError>`; `None` cuando no hay nada que hacer. Una entrada que conserva sus instantáneas instala el `Arc<Document>` guardado y no llama a ningún método del comando |
| `preview(cmd)` | El documento que produciría `cmd`, sin instalarlo ni tocar ninguna pila |
| `begin_gesture()`, `end_gesture()` | Abren y cierran un gesto; `begin_gesture` devuelve su `GestureId` |
| `document()`, `len()`, `is_empty()`, `can_undo()`, `can_redo()`, `estimated_bytes()` | Estado de solo lectura |

Como `Document` comparte páginas y nodos mediante `Arc`, una instantánea cuesta solo el camino cambiado. Cada entrada guarda una instantánea `before` y una `after`.

### Fusión de gestos

Un comando que cambia el documento se fusiona con la entrada más reciente de la pila de deshacer cuando ambos tienen `merge_key`, las claves son iguales y `Command::merge` lo acepta. El campo `gesture` de la clave decide entonces:

- Con `Some(gesture)`, se fusionan solo mientras ese gesto sea el abierto.
- Con `None`, se fusionan solo cuando `at` está a menos de 500 ms del último comando de la entrada más reciente.

`at` es un `Duration` monótono aportado por quien llama: el crate no lee ningún reloj (ARQ-02). Deshacer y rehacer terminan una racha de fusión. Una entrada fusionada conserva su primera instantánea `before` y toma la `after` más reciente, de modo que un solo deshacer revierte todo el arrastre.

### Presupuesto de memoria y compactación

`DEFAULT_BUDGET` es 512 MiB (536 870 912 bytes); `with_budget` fija otro valor y `usize::MAX` nunca compacta.

- La estimación es por entrada: los bytes de su instantánea `before` que su instantánea `after` no comparte por identidad de `Arc`, con un modelo de tamaños fijo (tamaños de registros más la longitud en heap de cadenas y cargas de extensión), no el heap real. Se guarda por entrada, por lo que `estimated_bytes()` es O(entradas).
- Tras un `execute` que registra o fusiona un cambio, `History` descarta las instantáneas de las entradas más antiguas, empezando por la más antigua, hasta que la suma cabe en el presupuesto. Una entrada compactada conserva su comando y cuenta cero. La entrada más reciente de la pila de deshacer nunca se compacta.
- Deshacer una entrada compactada ejecuta `revert` sobre una copia del documento actual; rehacer vuelve a ejecutar `apply`, y `Unchanged` cuenta como éxito. Con `Err` nada cambia y la entrada vuelve a su pila. Por eso `revert` debe restaurar el estado previo exacto, incluido el contador de ids.

### Validación

`History` ejecuta `debug_assert!(doc.validate().is_ok())` tras cada comando aplicado y tras cada deshacer y rehacer (ARQ-13). Las compilaciones de release no ejecutan `validate()` (es O(n)); cada comando comprueba sus propias entradas en todas las compilaciones. El historial nunca se guarda con el documento.

## Comandos del núcleo

Los comandos actúan sobre una página y no comprueban `locked`: los bloqueos los aplican las herramientas, no los comandos (decisión del propietario, Q12 del change). Cada `apply` exitoso registra lo que `revert` necesita, así que un comando sigue siendo reutilizable tras uno fallido.

### SetProperty

`SetProperty::new(page, nodes, value: Property, gesture: Option<GestureId>)` fija una propiedad en uno o más nodos. `Property` tiene ocho variantes:

| Variante | Campo del nodo | Comprobación |
|---|---|---|
| `Name(Option<String>)` | nombre | ninguna |
| `Placement(Placement)` | ubicación | valores finitos, `w` y `h` no negativos, inclinación estrictamente entre -90 y 90 |
| `Opacity(f32)` | opacidad | en `0.0..=1.0` |
| `Blend(BlendMode)` | modo de mezcla | ninguna |
| `Visible(bool)` | visibilidad | ninguna |
| `Locked(bool)` | indicador de bloqueo | ninguna |
| `Fill(Option<Color>)` | relleno de la forma | color finito, alfa en `0.0..=1.0`; solo formas |
| `Geometry(ShapeGeometry)` | geometría de la forma | solo formas |

Un valor inválido es `InvalidValue`; una página o nodo inexistente es `MissingPage` o `MissingNode`; una propiedad de forma sobre otro tipo de nodo y una lista de destinos vacía son `InvalidValue`. Devuelve `Unchanged` cuando todos los destinos ya tienen el valor. Su clave de fusión es el tipo de propiedad (por ejemplo `set_property.opacity`), la página, los destinos y el gesto; al fusionar conserva los valores previos del primer comando y toma el valor más reciente.

### InsertNodes y NodeSpec

`InsertNodes::new(page, parent, index, specs)` inserta subárboles como raíces de página (`parent` es `None`) o como hijos de un `Group`. Un `NodeSpec` describe un nodo sin ids (nombre, ubicación, opacidad, mezcla, visible, bloqueado, tipo y `children`); solo un `Group` puede tener hijos. Los ids se asignan del contador del documento en `apply`, en preorden. `revert` quita los nodos y baja el contador con `DocMut::restore_next_id`, de modo que un rehacer reutiliza los mismos ids y reconstruye un documento idéntico. Errores: `MissingPage`, `MissingNode`, `NotAContainer`, `IndexOutOfRange`, `IdSpaceExhausted` e `InvalidValue` para una lista vacía o hijos en un nodo que no es grupo.

### RemoveNodes

`RemoveNodes::new(page, nodes)` quita nodos con todos sus descendientes. Un nodo listado junto con uno de sus descendientes cuenta una vez, como el ancestro. Conserva los mismos valores `Arc<Node>` que quitó, y `revert` devuelve cada subárbol a su padre e índice anteriores. Una lista vacía es `InvalidValue`.

### Comandos planificados

- `Reorder` y `Reparent` dentro de una página (entrega D9, en curso): **Planificado**.
- `InsertPage`, `RemovePage`, y los campos de página (`name`, `size`, `locked`, `hidden`) y el DPI del documento como destinos de `SetProperty` (entrega D10): **Planificado**.
- Los comandos para tipos o datos que aún no existen (clips, fotogramas clave, máscaras, plantillas, versiones), mover nodos entre páginas y aplicar bloqueos en los comandos quedan fuera de alcance del change.

## Evidencia del contrato

### Arnés de propiedades (ARQ-81)

`gafira-testkit::prop_command` recibe una función que construye una estrategia de comandos para un documento generado y comprueba seis leyes:

1. `apply; revert` da un documento igual al original.
2. `apply; revert; apply` da un documento igual al del primer `apply`.
3. Tras un `apply` exitoso, `validate()` pasa.
4. `touches` es exacto: toda página o nodo que cambió figura, y todo lo que figura existe.
5. Con `Err`, el documento es igual por puntero al original.
6. Cuando dos comandos se fusionan, aplicar el comando fusionado equivale a aplicar ambos en secuencia.

Ejecuta 1 000 casos salvo que la variable de entorno `PROPTEST_CASES` indique otro valor. `try_prop_command` devuelve un `LawViolation` (número de ley, detalle, entrada minimizada) en lugar de entrar en pánico. `crates/gafira-edit/tests/prop_commands.rs` lo ejecuta para cada comando del núcleo. El paso de CI V14, que comprobaría que cada comando del núcleo tiene una prueba así, está **Planificado** (sustituye a `xtask`, que aún no existe).

### Fixtures compile-fail (ARQ-30)

`crates/gafira-edit/tests/ui.rs` usa `trybuild` con fixtures en `tests/ui/` que actúan como un crate externo a `gafira-edit`. Un fixture debe compilar: un `Command` externo ejecutado mediante `History`. Ocho no deben compilar: obtener `&mut Document` de `History`, asignar a través de `&Document` o de `&Arc<Document>`, llamar a `Document::edit` sin token o sobre una referencia compartida, escribir un campo privado y alcanzar `page_mut` o `node_mut` desde un documento o página compartidos. El constructor de `EditToken` es público pero oculto, así que ningún fixture puede mostrar que construir un token falla; V13 lo cubre. Un cambio en la firma de `Document::edit` exige regenerar las instantáneas `.stderr` con `TRYBUILD=overwrite`. Aún no se ha verificado que las instantáneas sean idénticas en Windows y macOS.

### Pruebas estructurales

- Deshacer y rehacer una entrada que conserva sus instantáneas intercambian un solo `Arc` y no llaman a ningún método del comando, sea cual sea el tamaño del documento (ARQ-32, prueba estructural; el criterio temporizado espera al benchmark de ARQ-40).
- Un arrastre de 300 movimientos copia solo el nodo arrastrado (`Arc::ptr_eq` para todos los demás nodos).
- Una prueba de propiedades muestra que deshacer y rehacer mediante `revert` y `apply` (entradas compactadas) alcanzan los mismos estados que mediante instantáneas; hoy cubre solo `SetProperty`.
