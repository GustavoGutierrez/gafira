# 043 — Agrupar, duplicar, copiar y pegar

> **Estado:** done — 2026-11-17 — S43-01..S43-19 cubiertos por `gafira-ui` (`group::tests` s43, 12; `clipboard::tests` s43, 5; `canvas::tests` s43, 3; `arrange::tests` s43, 1; `editor::tests` s43, 6) y `gafira-format` (`gaf` s43, 1); 30 mutantes a mano muertos; 3117 tests.

## Objetivo

Tratar varios elementos como uno y repetirlos sin rehacerlos (FD-13: «**Agrupar** (`Ctrl+G`) crea un `GroupNode`; Desagrupar (`Ctrl+Shift+G`). Los hijos conservan su posición en espacio de página y la caja del grupo es la unión de la de sus hijos»; «**Duplicar** (`Ctrl/Cmd+D`) crea una copia desplazada y con nombre «{nombre} copia»»; 03 §9.1: «doble clic entra en un grupo; `Esc` sale»; 03 §9.2: Cortar, Copiar, Pegar, Duplicar, Agrupar y Desagrupar). El modelo y el `.gaf` ya tenían el nodo `group` (spec 019); hasta ahora la interfaz no podía crearlo, el lienzo no lo seleccionaba y Ctrl+A lo saltaba (spec 040).

## Alcance / no alcance

Alcance:

- Modelo y formato: sin cambios. `NodeKind::Group` existe desde la spec 019 y el `.gaf` guarda los grupos con sus listas `children`; `schemaVersion` sigue en 1. Los hijos de un grupo están en espacio de página (D-35): la colocación de un grupo no los mueve y su caja es la unión de las de sus hijos.
- Agrupar (Ctrl+G, Edición → Agrupar), con uno o más elementos:
  - un `GroupNode` nuevo entra en la lista del elemento de más adelante, justo encima de él, y los seleccionados pasan a él en su orden de apilado (no en el de selección); el grupo queda seleccionado;
  - ningún elemento se mueve: su colocación no cambia ni un bit, también la de los girados; la colocación propia del grupo es la caja alineada a los ejes de sus hijos (D-28);
  - es un `Transaction` de `InsertNodes` y `Reparent`: un paso de deshacer. El id del grupo es el siguiente del documento (`InsertNodes` numera en preorden desde él).
- Desagrupar (Ctrl+Mayús+G, Edición → Desagrupar), con uno o más grupos seleccionados:
  - los hijos de cada grupo ocupan su sitio en la lista de su padre, en su orden y en su sitio de la página, y el grupo vacío se quita; un grupo dentro de otro desagrupa dentro de él;
  - quedan seleccionados los hijos liberados y los seleccionados que no eran grupos;
  - todo es un `Transaction` (`Reparent` y `RemoveNodes` por grupo): un paso de deshacer;
  - la opacidad y el modo de mezcla propios del grupo se pierden: los hijos conservan los suyos.
- Grupos en el lienzo:
  - un clic sobre algo dibujado selecciona el hijo del ámbito que lo contiene: en la página, el grupo raíz; dentro de un grupo, su hijo (a cualquier profundidad);
  - doble clic sobre un grupo entra en él y selecciona el hijo bajo el puntero; otro doble clic entra un nivel más;
  - Esc dentro de un grupo selecciona el grupo y sale a su padre; fuera de un grupo vacía la selección como antes;
  - un clic fuera del grupo en que se entró, en otra página o en el lienzo vacío sale de él;
  - el grupo seleccionado se dibuja con su caja recta alrededor de sus hijos, con los ocho tiradores y el asa de giro si es el único seleccionado. Los bordes de selección se dibujan ahora encima de todos los elementos de la página;
  - la opacidad de un grupo atenúa a sus hijos en el lienzo (se multiplica; el render compone la capa, así que dos hijos que se solapan pueden verse algo distintos que al exportar);
  - Ctrl+A y el marco seleccionan los hijos del ámbito que dibujan algo: en la página, los grupos ya cuentan (040 los saltaba); los ocultos y los grupos vacíos no.
- Acciones sobre un grupo seleccionado: cada una reescribe la colocación de cada nodo bajo él y vuelve a ajustar la de cada grupo a la caja de sus hijos, en un paso de deshacer:
  - mover (arrastre, alinear, distribuir): todos se desplazan igual;
  - redimensionar (tiradores o Ancho/Alto en el Inspector): cada nodo conserva su sitio relativo en la caja y escala con ella; un hijo girado escala en sus ejes y conserva su giro (exacto con escala uniforme o giros de 90°; con escala no uniforme no se deforma en paralelogramo);
  - girar (asa, Edición, Inspector): cada nodo gira alrededor del centro de la caja del grupo y suma el giro (normalizado a `[0, 360)`); con Mayús, el giro se ajusta a 15° como en 041;
  - voltear (Mayús+H, Mayús+V): cada nodo se refleja por el eje del centro de la caja;
  - orden de apilado, eliminar y opacidad, como un elemento.
- Inspector con un grupo: «Grupo», X, Y, Ancho y Alto de su caja, los botones de girar y voltear, Alinear y Orden, y Opacidad. Sin Giro (su caja siempre es recta), relleno ni contorno. Con varios seleccionados y algún grupo, sin relleno.
- Capas: lista en árbol. Un grupo con hijos tiene un triángulo para abrir o cerrar («Abrir grupo», «Cerrar grupo»); sus hijos van debajo, 16 px más adentro, delante primero. Un clic en un hijo lo selecciona y entra en su grupo en el lienzo; Subir y Bajar actúan en la lista de sus hermanos.
- Orden de apilado (spec 042) entre hermanos: las cuatro acciones actúan en la lista que comparte la selección (la página o un grupo); con elementos de listas distintas se desactivan.
- Duplicar (Ctrl+D, Edición → Duplicar):
  - copia cada seleccionado con todo lo que tiene debajo, con ids nuevos (FD-13 (c)), desplazada **10 px** a la derecha y abajo (`COPY_OFFSET`);
  - las copias van juntas justo encima del seleccionado de más adelante, en su orden, y quedan seleccionadas;
  - una copia de un elemento con nombre se llama «{nombre} copia» («{name} copy» en inglés, `editor.copy_name`); sin nombre sigue sin nombre (Capas muestra su tipo);
  - un `InsertNodes`: un paso de deshacer.
- Copiar, Cortar y Pegar (Ctrl+C, Ctrl+X, Ctrl+V y Edición):
  - el portapapeles de la aplicación guarda los subárboles seleccionados sin ids, de atrás adelante (un nodo dentro de un grupo seleccionado va con el grupo), y las entradas de la tabla de assets de sus imágenes. Los bytes de una imagen no se copian: viven una vez en el almacén de contenido de la sesión, por su hash;
  - vive en el shell, así que sobrevive a abrir o crear otro proyecto en la misma sesión;
  - Copiar pone además un texto plano en el portapapeles del sistema: las palabras de cada texto copiado o, si no tiene texto, su nombre o su tipo, una línea por elemento;
  - Cortar copia y quita la selección en un `RemoveNodes`: un paso de deshacer;
  - Pegar inserta encima de todo en la página de la selección, o en la primera, y dentro del grupo en que se entró si lo hay; lo pegado queda seleccionado;
  - sitio: el mismo si cae en la página y no tapa exactamente un elemento de la misma caja; encima de su original (o de una copia anterior) se desplaza 10 px, y otros 10 px por cada copia ya en ese sitio; si cae fuera de la página, se centra en ella;
  - en otro documento de la sesión trae las entradas de assets que le faltan, en el mismo `Transaction` que la inserción: un paso de deshacer que también las quita. Si los bytes no estuvieran en el almacén (no ocurre en una sesión: el almacén es uno), la imagen se pega y se dibuja como hoy un asset que falta (el recuadro `surface.sunken`).
- Atajos (03 §9.2) y su relación con el portapapeles del sistema:
  - la ventana (`egui-winit`) convierte Ctrl+C, Ctrl+X y Ctrl+V en eventos de portapapeles y no en teclas, también con Alt o Mayús pulsados. Por eso Copiar y Cortar se leen de esos eventos, y Pegar del evento que trae el texto del sistema: pega el portapapeles de la aplicación solo si ese texto es el que puso la última copia. Otro texto (copiado en otra aplicación) no se pega;
  - si el portapapeles del sistema está vacío la ventana no envía nada, así que Ctrl+V no pega; Edición → Pegar pega siempre el portapapeles de la aplicación;
  - Ctrl+Mayús+V («Pegar sin formato») llega como el mismo evento y pega igual: los nodos no tienen formato aparte;
  - Ctrl+Alt+C (Alinear al centro, spec 042) también llega como copia. Hasta ahora no alineaba en la ventana real (los tests de 042 enviaban la tecla); ahora una copia con Alt alinea al centro y no copia;
  - Ctrl+G, Ctrl+Mayús+G y Ctrl+D son teclas normales y solo cuentan con esos modificadores exactos. Ninguno choca con los atajos de 03 §9.2 ni con los ya conectados; todos se ignoran mientras se escribe en un campo.
- Menú Edición: Cortar, Copiar, Pegar y Duplicar (con su atajo) antes de Eliminar; Agrupar y Desagrupar (con su atajo) antes de Organizar. Cortar, Copiar y Duplicar se desactivan sin selección; Pegar, sin nada copiado; Agrupar, sin selección; Desagrupar, sin un grupo seleccionado.

No alcance:

- guías, imán y ajuste al mover; componentes o símbolos;
- Alt+arrastrar para duplicar (03 §9.1) y el menú contextual (03 §9.4);
- pegar imágenes o texto de otras aplicaciones desde el portapapeles del sistema;
- arrastrar filas en Capas para cambiar de grupo; renombrar con doble clic;
- bloquear y ocultar (resto de FD-13) y el sesgo de los grupos.

## Dependencias

- Specs 019 (`.gaf`), 024 (Capas), 029 (imágenes y assets), 040 (selección, `Transaction` que se fusiona), 041 (girar y voltear) y 042 (alinear y orden).
- `gafira-edit` sin cambios: `InsertNodes` (subárboles, ids en preorden), `Reparent` (conserva el orden del documento), `RemoveNodes`, `Reorder`, `InsertAsset` y `Transaction`.
- `gafira-layout::node_bounds` para las cajas de grupo. Sin crates nuevos ni cambios de capas: la tabla V5 no cambia.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S43-01 | Plan de agrupar. | Con tres raíces, agrupar la 3 y la 1 pone el grupo en el índice 3 (encima de la 3) con los nodos `[1, 3]`; la caja incluye la caja alineada de la 3 girada 90° (x 10..35, y 5..45) y es recta. Sin nodos o con uno que no existe, nada. Dentro de un grupo, el nuevo grupo queda en él. |
| S43-02 | Agrupar en un paso. | Las raíces pasan a `[2, grupo]` y el grupo tiene `[1, 3]`; las colocaciones son iguales bit a bit; una entrada; un deshacer devuelve la página igual. |
| S43-03 | Desagrupar. | Dos grupos a la vez: las raíces quedan `[1, 2, 3, 6, 5]`, los grupos desaparecen, las colocaciones (también la girada 45°) no cambian; una entrada y un deshacer la devuelven igual. Un grupo anidado desagrupa en su padre; un elemento no es un grupo. |
| S43-04 | Ámbito, clic y Ctrl+A. | `pick` sube al hijo del ámbito a cualquier profundidad y fuera de él da nada; doble clic entra un nivel y selecciona el hijo; un clic fuera sale a la raíz. `items` lista los hijos del ámbito que dibujan algo (sin ocultos ni grupos vacíos); Ctrl+A en la página incluye los grupos. |
| S43-05 | Caja de un grupo. | Es la unión de sus hijos sin mirar su propia colocación; la de un elemento es su colocación. `siblings` da la lista común o nada con dos padres. |
| S43-06 | Transformar un grupo. | Mover desplaza todo y reajusta los grupos anidados; cada nodo aparece una vez, padres primero. Redimensionar de 100×100 a 200×100 escala posición y tamaño; un hijo girado 90° crece en su eje local y conserva el giro. Girar 90° lleva el centro (10, 5) a (95, 10) y suma el giro (−90 da 270). Voltear refleja el centro por el eje del grupo y el volteo local. |
| S43-07 | Subárbol como `NodeSpec`. | Copia valores e hijos anidados sin ids; desplazar mueve todas las colocaciones; los ids nuevos de las raíces siguen el preorden (10 y 15 con un grupo de cinco nodos delante). |
| S43-08 | Copiar. | Subárboles de atrás adelante, un hijo de un grupo copiado va con él, con la entrada del asset de la imagen; nada sin selección, nodo o página. Texto plano: «Logo», «Hola» y «Rectángulo». |
| S43-09 | Duplicar. | Ids nuevos encima del de más adelante (`[1, 4, copia, copia, 5]`), desplazados 10 px con sus hijos, «Logo copia» y sin nombre si no lo tenía; cuatro nodos nuevos; una entrada y un deshacer. |
| S43-10 | Pegar. | Sobre su original se desplaza 10 px y la siguiente vez 20; en una página libre queda igual; fuera de la página se centra. En otro documento trae la entrada del asset y un deshacer la quita; en el mismo no añade ninguna. |
| S43-11 | Capas y opacidad. | Un grupo abierto lista a su hijo debajo con profundidad 1, su índice y su padre; cerrado no. La opacidad 0,5 del grupo atenúa al hijo a 0,5 y el hijo conserva la suya. |
| S43-12 | Alinear un grupo. | Alinear a la izquierda un grupo y un elemento mueve el grupo y sus hijos −200 px sin cambiar `y`. |
| S43-13 | Ctrl+G y Ctrl+Mayús+G. | Agrupa dos de tres por teclado, el grupo queda donde estaba el de más adelante y seleccionado (también en el fotograma siguiente); desagrupa y selecciona los hijos; nada se mueve; una entrada cada uno. |
| S43-14 | Un grupo como uno. | Arrastrar mueve sus dos hijos (10, 4) en un paso y no al tercero; girar 90° gira los dos; eliminar quita el grupo con sus hijos y un deshacer los trae; el Ancho del Inspector ensancha la caja. |
| S43-15 | Ctrl+D. | Duplica dos elementos con ids nuevos, a +10 px, seleccionados, «Logo copia», encima del de más adelante y en un paso. |
| S43-16 | Copiar, cortar y pegar. | Ctrl+C pone «Rectángulo» en el portapapeles del sistema; un pegado con otro texto no pega; con ese texto pega a +10 px en un paso; Edición → Pegar pega a +20; Ctrl+X quita en un paso y un deshacer lo trae; otro proyecto de la sesión pega lo copiado. |
| S43-17 | Teclado en un grupo. | Dentro del grupo Ctrl+A selecciona sus hijos; Esc selecciona el grupo y sale; otro Esc vacía. Ctrl+Alt+C (que llega como copia) alinea al centro y no copia. |
| S43-18 | Acciones activas. | Agrupar necesita selección y Desagrupar un grupo; el orden se calcula entre los hijos del grupo y se desactiva con elementos de dos listas. |
| S43-19 | `.gaf`. | Grupos anidados con nombre, opacidad y un hijo girado vuelven iguales por el `.gaf`, con sus listas de hijos, y `schemaVersion` sigue en 1. |

## TDD

Casos que fallaron primero:

1. `group::tests` (S43-01 a S43-07): escritos con las funciones de `group.rs` vacías (devolvían nada o su entrada); los 11 fallaron por sus aserciones. Después se añadió S43-04 (doble clic) con `click`.
2. `clipboard::tests` (S43-08 a S43-10): escritos con `clipboard.rs` vacío; los 5 fallaron.
3. `canvas::tests` S43-11 (Capas en árbol, opacidad heredada) y S43-04 (Ctrl+A con grupos) se escribieron con el cambio de `layers` y `shapes`; `s40_02_select_all_takes_the_drawn_roots` cambió de nombre y de expectativa, porque 040 dejaba fuera los grupos «todavía».
4. `arrange::tests` S43-12 y los tests del editor (S43-13 a S43-18) se escribieron con la interfaz conectada; su capacidad de fallar la prueban los mutantes de abajo.
5. S43-19 es de caracterización: el formato ya tenía grupos y pasó al escribirlo.

## Evidencia

- Mutantes a mano (cada uno aplicado solo y con `cargo test -p gafira-ui --lib -- s43 s42 s41 s40 s24`):
  1. el grupo nuevo debajo del elemento de más adelante en vez de encima (S43-01);
  2. los nodos en el orden de selección y no en el de apilado (S43-01, S43-02);
  3. la caja del grupo con las colocaciones sin girar (S43-01);
  4. los hijos desagrupados detrás del sitio del grupo (S43-03);
  5. desagrupar varios grupos planificando sobre la página sin los pasos anteriores (S43-03);
  6. `pick` que ignora el ámbito (S43-04);
  7. el doble clic que no entra en el grupo (S43-04);
  8. `items` con ocultos y grupos vacíos (S43-04, también Ctrl+A en el lienzo);
  9. redimensionar con el ancho escalado en ejes de página (S43-06);
  10. girar alrededor del centro en sentido contrario (S43-06);
  11. voltear sin reflejar el centro (S43-06);
  12. los grupos sin reajustar a sus hijos (S43-06);
  13. los ids de las raíces sin contar sus hijos (S43-07);
  14. desplazar sin llegar a los hijos (S43-07, S43-09, S43-10);
  15. el hijo de un grupo copiado dos veces (S43-08);
  16. duplicar sin desplazar (S43-09, S43-15);
  17. duplicar sin «{nombre} copia» (S43-09, S43-15);
  18. pegar exactamente encima del original (S43-10, S43-16);
  19. pegar fuera de la página (S43-10);
  20. pegar con un `InsertAsset` del asset que ya está (S43-10, por la etiqueta del comando);
  21. pegar en otro documento sin la entrada del asset (S43-10);
  22. Ctrl+Alt+C que copia en vez de alinear (S43-17);
  23. cualquier texto pegado pega el portapapeles (S43-16);
  24. arrastrar un grupo sin sus hijos (S43-14);
  25. girar un grupo sin sus hijos (S43-14);
  26. el orden de apilado calculado sobre las raíces (S43-18);
  27. Esc que vacía dentro de un grupo (S43-17);
  28. Ctrl+A que ignora el grupo en que se entró (S43-17);
  29. alinear un grupo sin sus hijos (S43-12);
  30. un grupo que no atenúa a sus hijos (S43-11).

  El 20 sobrevivió a la primera pasada: el historial deshace con su copia del documento y no notaba el `InsertAsset` sobrante. S43-10 comprueba ahora que en el mismo documento el pegado es solo la inserción.
- `cargo fmt --all --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 3117 tests, 0 fallos.
- `gafira-testkit` y `gafira-ports` no cambiaron; tampoco las dependencias, las capas ni `ci.yml`, así que no aplican los pasos 4 a 6 de «Before calling something done».
- `python3 tools/sync_agents.py --check`: 0.
- Sin prueba manual en la ventana: el doble clic se cubre con `group::click` (puro), no con un fotograma; el resto, con fotogramas del editor y con `apply`.

## Riesgos y decisiones abiertas

- Redimensionar un grupo con un hijo girado de forma no uniforme conserva el giro del hijo y escala sus lados a lo largo de sus ejes: no se deforma en un paralelogramo (que pediría sesgo). Es exacto con escala uniforme o con giros múltiplos de 90°.
- Un grupo no tiene giro propio: su caja es siempre recta alrededor de sus hijos (D-35), así que el Inspector no muestra Giro para un grupo y, tras girarlo, su caja crece alrededor de los hijos girados.
- Desagrupar pierde la opacidad y el modo de mezcla del grupo. Pasarlos a los hijos (multiplicar la opacidad) es una alternativa si el titular lo prefiere.
- La opacidad de un grupo en el lienzo se multiplica por hijo; el render compone el grupo como capa. Con hijos que se solapan y un grupo translúcido, el lienzo puede verse algo distinto de la exportación.
- Ctrl+V depende del texto del portapapeles del sistema: si otra aplicación lo cambió, no pega lo copiado en Gafira (Edición → Pegar sí). Pegar imágenes o texto de fuera queda para otra spec.
- El portapapeles de la aplicación se pierde al cerrar Gafira; no se escribe en disco.
- Ctrl+Alt+C no alineaba en la ventana real desde la spec 042 (la ventana lo convierte en copia); queda corregido aquí.
