# 042 — Alinear, distribuir y orden de apilado

> **Estado:** done — 2026-11-17 — S42-01..S42-16 cubiertos por `gafira-ui` (`arrange::tests` s42, 10; `editor::tests` s42, 6); 18 mutantes a mano muertos; 3090 tests.

## Objetivo

Ordenar los elementos de la selección en un solo gesto: las seis acciones de Alinear y las dos de Distribuir de FD-11 (D-17, D-28, D-29) y las cuatro de Orden de apilado de FD-12 (03 §9.2 «Capas»; 03 §2.4: «Alinear, Orden de apilado» en la pestaña Posición; 03 §8: «alinear (6), distribuir (2), traer adelante, enviar atrás, al frente, al fondo»).

## Alcance / no alcance

Alcance:

- Cajas: cada elemento cuenta por su caja alineada a los ejes en la página (`gafira_layout::node_bounds`), así que uno girado cuenta por la caja de su contorno girado (D-28, FD-11 «Cajas»), como el marco de la spec 041. Mover la colocación mueve esa caja lo mismo, de modo que la acción solo cambia `x` e `y`: el giro, el volteo y el tamaño no cambian.
- Alinear (izquierda, centro, derecha, arriba, medio, abajo), con `gafira_layout::align`:
  - con dos o más elementos, a la caja que los envuelve a todos, bloqueados incluidos (D-29); con uno, a la página (0, 0, ancho, alto en px);
  - cada acción cambia un eje; los bloqueados no se mueven; con un solo elemento bloqueado o sin selección no pasa nada (FD-11 f).
- Distribuir horizontal y verticalmente con espacio igual (el modo por defecto de FD-11), con `gafira_layout::distribute`:
  - solo cuentan los libres (D-17), ordenados por su borde inicial en el eje; el primero y el último no se mueven; con menos de tres libres no pasa nada;
  - si no caben, el espacio es negativo y se solapan por igual (FD-11 e).
- Orden de apilado sobre las raíces de la página (índice 0 es el fondo), para toda la selección y conservando su orden relativo, sea cual sea el orden en que se seleccionó:
  - «Al frente»: los seleccionados encima de todo; «Al fondo»: debajo de todo;
  - «Traer adelante»: cada tramo seleccionado sube un puesto por encima del siguiente no seleccionado; «Enviar atrás»: baja uno por debajo del anterior; un tramo que ya está en el borde se queda.
- Un paso de deshacer por acción:
  - Alinear y Distribuir: un `SetProperty` de colocación por elemento que se mueve, en una `Transaction` con un gesto propio (como girar y voltear en la 041), así que dos acciones seguidas son dos pasos;
  - Orden: la nueva lista se reconstruye con pasos `Reorder` (los mínimos que llevan cada nodo a su índice, en orden) dentro de una `Transaction` con la etiqueta `edit.reorder`;
  - una acción que no cambia nada no deja paso.
- Interfaz:
  - pestaña Posición del Inspector, con uno o varios elementos: «Alinear» con seis botones de icono y los dos de Distribuir, y «Orden» con cuatro (Traer adelante, Enviar atrás, Al frente, Al fondo);
  - Edición → «Organizar», un submenú con las mismas doce acciones y sus atajos;
  - estados: Alinear activo con un elemento o más; Distribuir con tres o más, y apagado su tooltip dice por qué («Distribuir horizontalmente: selecciona tres elementos o más»); cada acción de Orden solo cuando cambiaría las capas (FD-12: «Al frente» se atenúa en el elemento superior);
  - el tooltip nombra el atajo cuando lo hay («Al frente · Ctrl+Shift+]»). egui escribe la tecla de un atajo por su nombre inglés («CloseBracket», «Equals»); ahora se escribe su símbolo en todos los atajos del editor.
- Atajos (03 §9.2, sin choques con los atajos existentes de la ventana):
  - Ctrl+Alt+L, Ctrl+Alt+C y Ctrl+Alt+R: alinear a la izquierda, al centro y a la derecha (solo con Ctrl+Alt exactos);
  - Ctrl+] y Ctrl+[: traer adelante y enviar atrás; con Mayús, al frente y al fondo. Mayús+] llega como `}` en muchos teclados y cuenta igual. Alt no se mira: en teclados como el español los corchetes piden AltGr, que en Windows es Ctrl+Alt;
  - arriba, medio, abajo y Distribuir no tienen atajo en 03 §9.2 y no lo tienen aquí;
  - nunca mientras se escribe en un campo.
- Iconos: `align-left`, `align-center-horizontal`, `align-right`, `align-top`, `align-center-vertical`, `align-bottom`, `columns` y `rows` (Distribuir), `arrow-line-up` y `arrow-line-down` (Al frente, Al fondo) de Phosphor 2.1.1, peso Regular, sin modificar, del mismo paquete que las specs 037 y 041 (sha256 del tarball `313332be…baadd`, igual), con su entrada en `assets/LICENSES.toml` (ARQ-75). Traer adelante y Enviar atrás reutilizan `arrow-up` y `arrow-down` de Capas.
- Textos en `assets/i18n/{es,en}.toml`, sección `arrange`.

No alcance:

- el modo «entre centros» de Distribuir (FD-11 lo deja como entrada; la interfaz usa el de por defecto);
- grupos: el orden actúa sobre las raíces de la página, y un nodo seleccionado dentro de un grupo no se reordena (la interfaz todavía no crea grupos);
- copiar y pegar, guías, Imán y el resto de FD-06;
- la Barra contextual y el menú contextual de 03 (no existen todavía);
- arrastrar filas en Capas (FD-12 b).

## Dependencias

- Specs 024 (Capas, `Reorder`), 037 (iconos), 040 (selección múltiple, `Transaction`) y 041 (gesto propio por acción, caja alineada de lo girado).
- `gafira-layout` ya tenía `align`, `distribute` y `node_bounds` con sus tests (FD-11, D-17, D-28, D-29); `gafira-edit` ya tenía `Reorder`. Ningún crate, dependencia ni capa cambia; `Cargo.lock` no cambia.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S42-01 | Alinear a la selección. | Tres cajas en x 100..360, y 50..410: izquierda deja todos los bordes en 100, centro en 230, derecha en 360, arriba en 50, medio en 230, abajo en 410 (±0,01); solo cambia un eje; el que ya está en su sitio no se devuelve. |
| S42-02 | Un elemento a la página. | Una caja de 40 × 30 en una página de 1000 × 800 queda en x 0, 480, 960 o y 0, 385, 770. |
| S42-03 | Girada (D-28). | Un cuadrado de 100 girado 45° se alinea por su caja (borde en 50, x = 70,71) y conserva el giro; solo, su caja toca el borde derecho de la página. |
| S42-04 | Bloqueados (D-29). | Un bloqueado cuenta en la caja y no se mueve; un solo bloqueado, sin selección o con un nodo ausente, nada cambia. |
| S42-05 | Distribuir. | Cuatro cajas en 0..700 con tamaños 270 quedan con tres huecos de 430/3 (±0,01); el primero y el último no se mueven; el orden es el de la posición, no el de la selección; el otro eje no cambia; ya repartidas, nada cambia. |
| S42-06 | Tres libres (D-17). | Con uno bloqueado de cuatro solo se mueve el libre del medio (a x 450); dos libres no se reparten; tres cajas de 100 en un tramo de 250 se solapan 25 (FD-11 e). |
| S42-07 | Distribuir girada. | Una caja de 100 × 20 girada 90° cuenta con 20 de ancho y queda en x 340; conserva el giro. |
| S42-08 | Al frente y Al fondo. | `[1..5]` con `[4, 2]`: al frente `[1, 3, 5, 2, 4]`, al fondo `[2, 4, 1, 3, 5]`; sin selección o con un nodo que no es raíz, igual. |
| S42-09 | Traer adelante y Enviar atrás. | Un puesto por tramo: `[2]` adelante `[1, 3, 2, 4, 5]`; `[1, 2]` adelante `[3, 1, 2, 4, 5]`; `[3, 5]` adelante `[1, 2, 4, 3, 5]`; `[4, 5]` atrás `[1, 2, 4, 5, 3]`; en el borde, igual. |
| S42-10 | Pasos `Reorder`. | Aplicados uno tras otro reconstruyen el nuevo orden para las cuatro acciones; sin cambio, ningún paso; un intercambio, un paso. |
| S42-11 | Alinear por el editor. | Tres elementos a la izquierda y abajo son dos pasos; cada deshacer devuelve los tres; uno solo a la derecha toca el borde de la página de 1080 px; repetir sin cambio no añade paso. |
| S42-12 | Distribuir por el editor. | Tres elementos quedan con huecos iguales en un paso; un deshacer los devuelve. |
| S42-13 | Orden por el editor. | Al frente y Al fondo de dos elementos conservan su orden relativo en un paso (dos `Reorder` en una `Transaction`); un deshacer devuelve los dos; una acción sin cambio no deja paso. |
| S42-14 | Atajos. | Ctrl+Alt+R deja los bordes derechos en el más a la derecha; Ctrl+Alt+C y L alinean; Ctrl+], Ctrl+[, Ctrl+Mayús+] (también como `}`) y Ctrl+Mayús+[ reordenan; Ctrl+Alt+[ (AltGr) también; `[` sin Ctrl y Ctrl+L no hacen nada. |
| S42-15 | Estados. | Sin selección todo apagado; con tres, Alinear y Distribuir y ningún Orden (no cambiaría nada); el superior solo puede bajar; dos al fondo solo subir; uno en medio, los cuatro. |
| S42-16 | Inspector y tooltips. | Fotogramas con uno y con tres elementos dibujan los botones; «Al frente · Ctrl+Shift+]» con el símbolo, no «CloseBracket»; «Alinear arriba» sin atajo; el tooltip de Distribuir apagado explica por qué. |

## TDD

Casos que fallaron primero:

1. `arrange::tests` (S42-01 a S42-10): escritos con las funciones de `arrange.rs` vacías (devolvían una lista vacía o las raíces sin cambiar); los 10 fallaron por sus aserciones.
2. Los tests del editor (S42-11 a S42-16) se escribieron con la interfaz ya conectada. S42-16 falló primero de verdad: el tooltip decía «Ctrl+Shift+CloseBracket», y de ahí salió `keys_text`. La capacidad de fallar del resto la prueban los mutantes de abajo; tres sobrevivieron a la primera versión de los tests y los tests se reforzaron (ver Evidencia).

## Evidencia

- Mutantes a mano, todos muertos (cada uno aplicado solo y con `cargo test -p gafira-ui --lib -- s42 s41 s40 s24`):
  1. los bloqueados contados como libres (S42-04, S42-06);
  2. devolver también los elementos que no se mueven (S42-01, S42-03 a S42-07);
  3. aplicar solo el desplazamiento en x (S42-01, S42-02, S42-11);
  4. la caja de la página con ancho y alto cambiados (S42-02, S42-03);
  5. distribuir entre centros en vez de con espacio igual (S42-05);
  6. «Al frente» que pone la selección debajo (S42-08, S42-13 a S42-15);
  7. «Traer adelante» recorrido desde el fondo: un tramo no sube junto (S42-09, S42-13, S42-14);
  8. «Enviar atrás» que intercambia vecinos seleccionados (S42-09, S42-15; sobrevivió a la primera versión de S42-09, que ganó el caso «un tramo en el fondo se queda»);
  9. un paso `Reorder` también para el nodo que ya está en su índice (S42-10);
  10. los pasos sin seguir la lista intermedia (S42-10);
  11. Distribuir activo con dos elementos (S42-15);
  12. las acciones de Orden siempre activas (S42-15);
  13. colocar sin gesto propio (S41-16, que comparte `place_in_one_step`);
  14. los pasos de Orden ejecutados uno a uno, sin `Transaction` (S42-13; sobrevivió mientras S42-13 solo probaba un caso de un paso, y ganó «Al fondo» de dos elementos, que son dos pasos);
  15. Mayús ignorada en los corchetes (S42-14);
  16. Ctrl+Alt+R que alinea a la izquierda (S42-14; sobrevivió mientras S42-14 comparaba los bordes derechos entre sí con cajas del mismo ancho, y ahora los compara con el más a la derecha);
  17. corchetes sin Ctrl (S42-14);
  18. el atajo escrito por el nombre de la tecla (S42-16).
- `cargo fmt --all --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 3090 tests, 0 fallos.
- `gafira-testkit` y `gafira-ports` no cambiaron (paso 4 no aplica); sin dependencias nuevas ni cambios de crate, capas o `ci.yml` (pasos 5 y 6 no aplican); el manual técnico no cambió (paso 7 no aplica).
- `python3 tools/sync_agents.py --check`: 0.
- Iconos: el tarball `@phosphor-icons/core-2.1.1.tgz` del registro npm tiene el sha256 de la spec 037; los diez SVG se copiaron sin cambios y sus hashes BLAKE3 en `assets/LICENSES.toml` los comprueba S37-04.
- Sin prueba manual en la ventana: las acciones se cubren con las funciones puras y con el editor por `Asked` y por fotogramas con teclas.

## Riesgos y decisiones abiertas

- Ctrl+Alt+L bloquea la sesión en algunos escritorios Linux (GNOME antiguo, Cinnamon, Xfce) y Ctrl+Alt con letras puede ser AltGr en Windows. Según 03 §9.2 gana el sistema; el menú y los botones siguen sirviendo. Verificar en cada plataforma (VERIFICAR de 03).
- En teclados con los corchetes en AltGr (español), Ctrl+[ se teclea como Ctrl+AltGr+`` ` ``; la ventana no mira Alt para que funcione en Windows (AltGr = Ctrl+Alt). En Linux depende de cómo llegue AltGr a winit; no se ha probado en una ventana real.
- Ctrl+Mayús+[ con Mayús+corchete que el teclado escribe como `{` cuenta igual; otros símbolos que produzca un teclado no se reconocen.
- egui escribe los modificadores en inglés («Ctrl+Shift»), también en español; ya pasaba con los atajos anteriores.
- Al frente y Al fondo de un nodo dentro de un grupo no hacen nada hasta que haya grupos; entonces el titular decide si actúan en su lista de hermanos.
- Alinear un elemento a la página usa la página entera, no los márgenes ni la zona segura (FD-11 no lo pide).
