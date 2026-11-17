# 038 — Contorno y radio de esquina de las formas

> **Estado:** done — 2026-11-17 — S38-01..S38-08 cubiertos por `gafira-doc` (`node::tests` s38, 2; `validate::tests` s38, 1), `gafira-format` (`gaf` s38, 1), `gafira-edit` (`set_property::tests` s38, 2; `budget::tests` s38, 1), `gafira-render` (`graph::tests` s38, 5), `gafira-render-cpu` (`s38_outline`, 2) y `gafira-ui` (`canvas::tests` s38, 1; `editor::tests` s38, 1); 12 mutantes a mano muertos; 2987 tests.

## Objetivo

Las formas tienen contorno y los rectángulos esquinas redondeadas (FD-17: «rectángulo (radio de esquina editable)», «contorno con color y grosor»; 03 §7.3, relleno y contorno). Hasta ahora `ShapeNode` solo tenía relleno.

## Alcance / no alcance

Alcance:

- `gafira-doc`:
  - `ShapeStroke { color, width }` y `ShapeNode.stroke: Option<ShapeStroke>`.
  - El contorno va centrado en el borde: la mitad del grosor cae dentro de la caja y la otra mitad fuera. Ni 02 ni 04 fijan la posición del contorno de una forma (FD-25 la fija solo para el texto).
  - `ShapeNode.corner_radius: f64`, en px. Solo lo usa el rectángulo; la elipse lo ignora.
  - `validate()` comprueba los canales y el alfa del color del contorno (como el relleno), el grosor en `0..=1000` px (`ShapeStroke::MAX_WIDTH`) y el radio `>= 0`; todos finitos.
  - El radio no se limita al guardar. Se dibuja como mucho a la mitad del lado corto (FD-36 a), y el valor guardado se conserva.
- `gafira-edit`:
  - `Property::Stroke(Option<ShapeStroke>)` y `Property::CornerRadius(f64)` con los mismos rangos que `validate()`, deshacer y fusión por gesto.
  - Sobre un nodo que no es forma se rechazan sin cambio parcial.
  - Los dos campos van dentro del registro del nodo, así que la estimación de memoria del historial no necesita un término nuevo.
- `gafira-render`:
  - La forma rellena su trazado y encima, en la misma capa, el contorno de trazo de `gafira-pathops` (centrado, esquinas en inglete).
  - El rectángulo con radio es un `RoundedRect` de kurbo con el radio limitado.
  - Un grosor 0 no dibuja contorno, y una forma sin relleno ni contorno no dibuja nada.
  - El contorno (color y grosor) y el radio dibujado entran en la clave de contenido solo cuando existen. Así la clave de una forma anterior no cambia.
  - La exportación PNG y PDF y las miniaturas pasan por el grafo, así que los heredan.
  - `gafira-render` gana la dependencia `gafira-pathops`, que la tabla V5 ya permitía.
- `gafira-ui`:
  - El Inspector muestra «Contorno» en las formas: color, «Sin contorno» y «Grosor» en px. Un contorno nuevo empieza con 2 px.
  - Los rectángulos muestran además «Radio de esquina», de 0 a la mitad del lado corto.
  - Cada cambio es un `SetProperty`, es decir, un paso de deshacer.
  - El lienzo pinta las formas con `canvas::painted`: el contorno centrado, escalado por el zoom y con la opacidad del elemento, y el radio limitado como en el render.

No alcance:

- estilo del contorno (discontinuo, punteado), posición interior/exterior y radio por esquina (FD-36);
- contorno en texto (FD-25) e imágenes (Inspector → Imagen → Bordes, FD-36);
- el contorno en la barra contextual (03 §7.3), que todavía no existe;
- un arrastre del grosor o del radio en un solo paso de deshacer: como X, Y, Ancho y Alto, cada cambio del campo es un paso.

## Formato y compatibilidad

- Los dos campos son opcionales y aditivos:
  - `stroke` se escribe solo si existe y `corner_radius` solo si no es `+0.0` (`-0.0` sí se escribe y vuelve igual).
  - Una forma sin ellos conserva sus bytes anteriores (el test de forma en la conexión de `node.rs` no cambió).
  - Un documento anterior carga sin contorno y con radio 0.
- `schemaVersion` sigue en 1, igual que con `TextNode` (027) e `ImageNode` (029). El `.gaf` no cambia: el documento va en su JSON.
- Riesgo: una versión anterior de Gafira ignora los campos desconocidos. Si abre un archivo nuevo y lo guarda, pierde el contorno y el radio sin aviso. Para evitarlo habría que subir el schema o declarar una capacidad en `features`; queda para cuando haya versiones publicadas que convivan.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S38-01 | Campos aditivos. | Contorno y radio vuelven iguales por JSON y por `.gaf`. Una forma sin ellos no los escribe, y una forma antigua carga con `stroke: None` y radio `+0.0`. Un contorno sin color o sin grosor no carga. |
| S38-02 | Validación. | Grosor `0` y `1000` pasan; `1000,5`, `-0,5` y NaN fallan con `StrokeWidth`. El alfa del contorno fuera de `[0, 1]` falla con `StrokeAlpha`, un canal NaN con `StrokeChannel`, y un radio `-1e-9` o infinito con `CornerRadius`. El relleno se informa antes que el contorno. |
| S38-03 | Comandos. | `Stroke` y `CornerRadius` se aplican, quedan sin cambio con el mismo valor, se deshacen y tienen su clave de fusión. Los valores que `validate()` rechaza fallan sin cambiar nada, y sobre un grupo fallan. Una forma con contorno cuesta en el historial lo mismo que una sin él. |
| S38-04 | Contorno en el grafo y en píxeles. | Operaciones `push, fill, fill, pop`, con el contorno encima y de su color. La caja del contorno es la de la forma ampliada en la mitad del grosor, y su centro queda vacío. Un contorno solo también dibuja; grosor 0 sin relleno no dibuja nada. NaN es `NonFinite` (`stroke`, `corner_radius`). En CPU, los píxeles a 1,5 px a cada lado del borde son del color del contorno. |
| S38-05 | Radio. | La esquina de un rectángulo con radio queda vacía. Un radio mayor que la mitad del lado corto se dibuja igual que esa mitad, sin deformar (FD-36 a). La elipse lo ignora y el contorno sigue la curva, también en píxeles. |
| S38-06 | Clave de contenido. | Una forma sin contorno ni radio tiene la clave de antes. El grosor, el color del contorno y el radio la cambian. Dos radios que se dibujan igual dan la misma clave. |
| S38-07 | Lienzo. | `painted` escala el grosor por el zoom, aplica la opacidad y usa `StrokeKind::Middle`. Limita el radio a la mitad del lado corto en pantalla; en la elipse lleva el contorno y no el radio. |
| S38-08 | Inspector por el engine. | Poner contorno y radio y quitar el contorno pasa por el engine; cada uno se deshace en un paso. |

## Evidencia

- Rojo verificado:
  - los tests de `node` y `validate` no compilaban sin `ShapeStroke`, y con los tipos las filas de S38-02 devolvían `Ok`;
  - `set_property` no compilaba sin las variantes, y sin las comprobaciones fallaba S38-03 (valores inválidos);
  - los 5 tests de `graph` fallaban (sin contorno, sin radio, clave sin cambios). La clave de referencia de S38-06 sí coincidía, así que el fallo era de la función que faltaba;
  - el test de `canvas` no compilaba sin `painted` ni los campos.
- Mutantes a mano, todos muertos:
  1. sin el tope del grosor (`validate`);
  2. el radio sin la regla `>= 0` (`validate`);
  3. `+0.0` comparado por valor en vez de por bits al escribir (muere S38-01 con `-0.0`);
  4. sin el tope del grosor (`check` del comando);
  5. sin `>= 0` en el radio (`check` del comando);
  6. `write` que no guarda el contorno;
  7. sin limitar el radio (muere S38-06: kurbo ya limita el trazado, solo la clave los distingue);
  8. el contorno fuera de la clave;
  9. el contorno debajo del relleno;
  10. el radio 0 en la clave (cambia la clave de las formas anteriores);
  11. el lienzo sin el límite de la mitad del lado;
  12. el lienzo sin la opacidad en el contorno.
  - Equivalente, ejecutado y vivo, no contado: quitar el filtro `width > 0` del render; `gafira-pathops` ya rechaza un grosor 0 y el contorno se omite igual.
- `cargo fmt --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 2987 tests, 0 fallos.
- `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` y `cargo test -p gafira-testkit --features ports --locked`: en verde (el testkit solo cambió sus literales de `ShapeNode`).
- Pasos de política de `ci.yml` en local (ARQ-07, `forbid(unsafe_code)`, V5, ARQ-06, V12, ARQ-107 x2, V13, V14): todos en verde. La tabla V5 no cambió: ya listaba `gafira-pathops` para `gafira-render`.
- Sin crates de terceros nuevos: `Cargo.lock` solo gana la arista `gafira-render -> gafira-pathops`.
- `python3 tools/sync_agents.py --check`: 0.

## Riesgos y decisiones abiertas

- El riesgo de compatibilidad hacia atrás está descrito en «Formato y compatibilidad».
- Si `gafira-pathops` rechaza el trazado de una forma con coordenadas por encima de `1e9` px, el contorno se omite y la forma conserva su relleno. No hay error de exportación.
- egui guarda el radio de esquina en px de pantalla enteros, hasta 255. Con un zoom muy grande, el lienzo redondea menos que la exportación.
- La elipse con contorno usa el trazo de egui en el lienzo y el de `gafira-pathops` al exportar; pueden diferir en el suavizado, no en la geometría.
