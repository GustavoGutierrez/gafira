# 041 — Girar y voltear

> **Estado:** done — 2026-11-17 — S41-01..S41-20 cubiertos por `gafira-ui` (`rotation::tests` s41, 10; `canvas::s41_tests`, 6; `editor::tests` s41, 7), `gafira-format` (`gaf` s41, 1) y `gafira-io-image` (`png` s41, 1; `jpeg_webp` s41, 1); 21 mutantes a mano muertos; 3074 tests.

## Objetivo

Girar y voltear los elementos del lienzo (03 §9.1: «8 asas y asa de giro; `Shift` al girar ajusta a 15°»; 03 §9.2: «Girar 15° · Shift+giro»; 03 §7.3 y FD-10: «Giro» en la pestaña Posición del Inspector; 03 §8: «girar, voltear (2)»). El modelo ya tenía el giro y el volteo; hasta ahora el lienzo los ignoraba y nada en la interfaz los cambiaba.

## Alcance / no alcance

Alcance:

- Modelo y formato: sin cambios.
  - `Placement` ya lleva `rotation_deg` (grados, en el sentido de las agujas del reloj sobre la página con y hacia abajo), `skew` y `flip { x, y }`. `to_transform` aplica, de derecha a izquierda, el volteo local, el sesgo y el giro alrededor del centro de la caja.
  - Los tres campos son opcionales al leer (`#[serde(default)]`) y se escriben siempre. Un documento anterior carga sin giro ni volteo; `schemaVersion` sigue en 1 y el `.gaf` no cambia.
  - `validate()` ya exige un giro finito y no lo limita a una vuelta: el documento guarda el valor tal cual.
- Normalización (`gafira-ui`, `rotation.rs`):
  - todo giro que escribe la interfaz (lienzo, Inspector, menú Edición) se normaliza a `[0, 360)`: `-0.0` y un valor que redondea a 360 dan `+0.0`;
  - con Mayús se ajusta al múltiplo de 15° más cercano con `gafira_layout::snap_rotation` (FD-06 (i): a medio camino, al mayor; 7,4° da 0° y 7,5° da 15°) y después se normaliza (352,5° da 0°).
- Voltear:
  - es un espejo en el eje de la página alrededor del centro del elemento, sea cual sea su giro;
  - como el volteo local va antes del giro y del sesgo, el espejo de la página es el volteo local de ese eje con el giro y el sesgo cambiados de signo (`F·R(θ)·S(k) = R(-θ)·S(-k)·F`);
  - voltear dos veces devuelve el elemento igual.
- Girar 90° a la izquierda o a la derecha: suma ∓90° al giro; la caja no se mueve.
- Dibujo en el lienzo, con la misma transformación que el render (`scale · placement`, D-35) llevada a la página:
  - un elemento sin giro, sesgo ni volteo se dibuja como antes;
  - una forma girada es un polígono convexo sobre su contorno (rectángulo, radio limitado a la mitad del lado corto como en el render, elipse), con el contorno centrado escalado por el zoom;
  - una imagen es un cuadrilátero con textura: su textura sigue la caja local, así que el volteo la refleja; sin textura, un polígono `surface.sunken`;
  - un texto girado o volteado se compone como antes en su caja sin girar y sus vértices de glifo pasan por la transformación del elemento, así que gira y se refleja. Las coordenadas del atlas de fuentes se normalizan con su tamaño del fotograma; si el atlas crece después, se pide otro fotograma.
- Selección en el lienzo:
  - el clic lleva el puntero a la caja local de cada elemento: se selecciona lo que se ve girado, y la elipse cuenta por su curva;
  - el marco y la caja de la selección múltiple usan la caja alineada a los ejes de la caja girada (D-28, FD-06 «Cajas»);
  - el borde de selección y los ocho tiradores siguen la caja girada.
- Asa de giro:
  - un círculo 24 px (un objetivo `size.target.min` de 03) por fuera del centro del borde superior local, en la dirección «arriba» del elemento, unido a ese borde por una línea de 1 px; gira con el elemento;
  - su zona de 24 px toca la del tirador superior central sin solaparse; gana el más cercano;
  - el arrastre suma el cambio del ángulo del puntero alrededor del centro; con Mayús ajusta a 15°; todo el arrastre es un paso de deshacer (gesto, ARQ-33);
  - como el resto de tiradores, el agarre se decide donde se pulsó (`press_origin`) y solo con un elemento seleccionado.
- Redimensionar un elemento girado: en sus ejes locales. El desplazamiento del puntero se lleva a la caja local, la caja crece allí con el mínimo de 1 px, y la nueva colocación conserva el giro y deja el borde opuesto donde estaba en la página. Con un elemento volteado, su borde derecho local está a la izquierda en pantalla y su tirador se arrastra hacia allí.
- Inspector:
  - «Giro» en la pestaña Posición, en grados con un decimal (`unit.deg`), normalizado; cada cambio es un `SetProperty` de colocación, como X, Y, Ancho y Alto;
  - debajo, cuatro botones de icono: Girar 90° a la izquierda y a la derecha, Voltear horizontalmente y verticalmente (con su atajo en el tooltip);
  - con varios elementos, los cuatro botones aparecen bajo «N elementos» y actúan sobre todos.
- Menú Edición: «Girar 90° a la izquierda», «Girar 90° a la derecha», «Voltear horizontalmente» (Mayús+H) y «Voltear verticalmente» (Mayús+V); activos con una selección.
- Atajos: Mayús+H y Mayús+V, solo con Mayús (Ctrl+Mayús+H no voltea) y nunca mientras se escribe en un campo.
- Selección múltiple: cada acción gira o voltea cada elemento alrededor de su propio centro, con un `SetProperty` por elemento dentro de una `Transaction`. La acción abre y cierra su propio gesto, así que es un paso de deshacer y nunca se fusiona con la siguiente (dos giros seguidos son dos pasos).
- Iconos: `arrow-counter-clockwise`, `arrow-clockwise`, `flip-horizontal` y `flip-vertical` de Phosphor 2.1.1 (peso Regular, sin modificar, del mismo paquete que la spec 037, sha256 del tarball igual), con su entrada en `assets/LICENSES.toml` (ARQ-75).
- Exportación: PNG, JPG, WebP y PDF dibujan con el grafo de `gafira-render`, que ya aplicaba `to_transform` a cada nodo (forma, texto e imagen); el PDF lleva la misma imagen de página (`page_pixels` y `PageRenderers`). No cambia código de render.

No alcance:

- girar o voltear una selección múltiple alrededor de su caja común; redimensionarla por su caja;
- Mayús para conservar la proporción y Alt para redimensionar desde el centro (03 §9.1) al redimensionar;
- editar el sesgo desde la interfaz (se conserva y se respeta al dibujar, voltear y redimensionar);
- alinear y distribuir, grupos, copiar y pegar;
- tiradores dibujados girados: los cuadrados se colocan sobre la caja girada pero siguen alineados a la pantalla.

## Dependencias

- Specs 020 (selección y arrastre), 022 (tiradores), 027 (texto), 029 (imágenes), 037 (iconos), 038 (contorno y radio) y 040 (selección múltiple, `Transaction` que se fusiona por gesto).
- `gafira-ui` gana la dependencia de `gafira-layout` (`snap_rotation`) y `proptest` en desarrollo; los dos ya estaban en el workspace. `gafira-ui` no tiene fila V5 y ninguna capa cambia.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S41-01 | Normalización. | 360 y -360 dan 0, 450 da 90, -90 da 270, -720,5 da 359,5; `-0.0` da `+0.0` y `-1e-20` da 0, nunca 360. NaN e infinito vuelven sin cambio (el comando los rechaza). |
| S41-02 | Ajuste a 15°. | 7,4 → 0, 7,5 → 15, -7,5 → 0, -8 → 345, 352,5 → 0, 400 → 45. |
| S41-03 | Girar 90°. | La caja y su centro no se mueven; 300 + 90 da 30 y 0 - 90 da 270. |
| S41-04 | Voltear. | Prueba de propiedad: con cualquier giro, sesgo y volteo, el elemento volteado es el espejo del original en el eje de la página por su centro (±1e-6), con el giro en `[0, 360)` y la caja igual. Dos volteos devuelven el original; sin `-0.0`. |
| S41-05 | Paridad con el render. | La transformación del lienzo es `scale · to_transform` del render llevada a la página; la caja girada 90° de 40 × 10 en (10, 20) queda en x 125..135, y 55..95 con la página en (100, 50). |
| S41-06 | Del puntero a la caja local. | Un punto vuelve a la caja local a través del giro y el volteo; una caja de ancho 0 sigue siendo invertible; una transformación degenerada o no finita no tiene inversa. |
| S41-07 | Posición del asa de giro. | 24 px por encima del centro superior a zoom 2; con 90° queda a la derecha del elemento. |
| S41-08 | Arrastre del asa de giro. | De arriba a la derecha del centro suma 90°; volver más allá del inicio da 280°; con Mayús 35° da 30°; la caja no cambia. |
| S41-09 | Elemento recto. | Recto solo sin giro (360 cuenta como 0), sin sesgo y sin volteo. |
| S41-10 | Clic y marco. | Un clic en la barra girada la selecciona y en su caja sin girar no; la elipse girada se toca por su curva. El marco cuenta la caja alineada de la caja girada. |
| S41-11 | Tiradores girados. | El tirador derecho de un elemento girado 90° queda abajo; el asa de giro es uno de los agarres; gana el más cercano. |
| S41-12 | Redimensionar girado. | Arrastrar el borde derecho local (abajo en pantalla) 20 px a zoom 2 alarga 10 px; el borde izquierdo local queda en el mismo punto de la página; con volteo horizontal el borde derecho local se arrastra hacia la izquierda; el mínimo de 1 px se mantiene. |
| S41-13 | Forma girada en el lienzo. | Un polígono cerrado con las esquinas de la caja girada, relleno y contorno escalado por el zoom; los puntos de la elipse están sobre su curva girada; las esquinas redondeadas no tocan la esquina de la caja. |
| S41-14 | Imagen y texto girados. | La textura de la imagen sigue el volteo y el giro; el texto volteado refleja cada vértice por el centro de la caja, con las coordenadas del atlas en `[0, 1]`, y recto coincide con donde se dibujan sus galleys. |
| S41-15 | Gesto del asa de giro. | Tres fotogramas de arrastre giran 90° alrededor del centro en un solo paso de deshacer; un deshacer vuelve al inicio; con Mayús 35° da 30°. |
| S41-16 | Girar y voltear la selección. | Dos «Girar 90° a la derecha» y un «Voltear horizontalmente» sobre dos elementos dejan 180° y `flip.x`, cada uno con su caja; son tres pasos de deshacer, y cada deshacer restaura los dos. |
| S41-17 | Atajos. | Mayús+H y Mayús+V voltean la selección; Ctrl+Mayús+H no; los no seleccionados no cambian. |
| S41-18 | Redimensionar girado por el editor. | Arrastrar el tirador derecho de un elemento girado 90° alarga en su eje en un paso de deshacer y conserva el giro. |
| S41-19 | `.gaf`. | Giro 292,5 y volteo en x e y vuelven iguales por el `.gaf`; una colocación sin ellos carga y se guarda con giro 0 y sin volteo. |
| S41-20 | Exportación. | PNG: la imagen volteada tiene el azul a la izquierda; girada 90° alrededor de su centro, la mitad izquierda (roja) queda arriba y su caja sin girar queda blanca; girada y volteada, el azul arriba. JPG y WebP: una barra girada 90° está de pie y su caja sin girar es blanca. |

## TDD

Casos que fallaron primero:

1. `rotation::tests` (S41-01 a S41-09): escritos con las funciones de `rotation.rs` vacías (devolvían su entrada, la identidad o `None`); los 10 fallaron por sus aserciones.
2. `canvas::s41_tests`: S41-11, S41-13 y S41-14 no compilaban sin `Grip`, `handle_point`, `grip_hit`, `painted_turned` ni `image_mesh` (13 errores). S41-10 (clic y marco) y S41-12 (redimensionar) compilaban contra el código anterior y fallaron por sus aserciones: el clic caía en la caja sin girar y el redimensionado movía el borde en ejes de página.
3. Los tests del editor (S41-14 texto, S41-15 a S41-18) se escribieron con la interfaz ya conectada; su capacidad de fallar la prueban los mutantes de abajo.
4. S41-19 y S41-20 son de caracterización: pasaron al escribirlos porque el modelo, el formato y el render ya tenían el giro y el volteo. Quedan como guarda de la paridad entre lienzo y exportación.

## Evidencia

- Mutantes a mano, todos muertos (cada uno aplicado solo y con `cargo test -p gafira-ui --lib -- s41 s40 s22 s20`):
  1. `normalized` sin la guarda de la vuelta completa (S41-01);
  2. `normalized` que conserva `-0.0` (S41-01, S41-04);
  3. voltear sin cambiar el signo del giro (S41-04, también el fotograma del editor);
  4. voltear sin cambiar el signo del sesgo (S41-04);
  5. Mayús que no ajusta (S41-08, S41-15);
  6. el ángulo del arrastre con el signo cambiado (S41-08, S41-15);
  7. el hueco del asa de giro sin dividir por el zoom (S41-07);
  8. «recto» que ignora el volteo (S41-09, S41-12);
  9. el clic que ignora el giro (S41-10);
  10. el marco con la caja sin girar (S41-10);
  11. redimensionar girado con el desplazamiento de página como local (S41-12, S41-18);
  12. redimensionar girado sin devolver el borde opuesto a su sitio (S41-12);
  13. sin asa de giro entre los agarres (S41-11);
  14. la malla de la imagen sin la transformación (S41-14);
  15. la forma girada pintada sin su transformación (S41-13);
  16. la malla del texto sin normalizar las coordenadas del atlas (S41-14);
  17. la malla del texto sin la transformación (S41-14);
  18. girar y voltear sin gesto propio: dos giros seguidos se fusionan (S41-16);
  19. Mayús+H también con Ctrl (S41-17);
  20. voltear antes de girar en una misma acción (fotograma del editor);
  21. el arrastre del asa de giro que ignora el desplazamiento del puntero (S41-15).
- `cargo fmt --all --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 3074 tests, 0 fallos.
- `gafira-testkit` y `gafira-ports` no cambiaron (paso 4 no aplica). Sin crates de terceros nuevos: `Cargo.lock` solo gana las aristas `gafira-ui -> gafira-layout` y `gafira-ui -> proptest` (desarrollo), así que `cargo deny` no aplica.
- Pasos de política de `ci.yml` en local (ARQ-07, `forbid(unsafe_code)`, V5, ARQ-06, V12, ARQ-107 x2, V13, V14, testkit `ports`): todos en verde.
- `python3 tools/sync_agents.py --check`: 0.
- Sin prueba manual en la ventana: el dibujo girado se cubre con las funciones puras (`painted_turned`, `image_mesh`, `text_mesh`) y con un fotograma completo del editor con forma, elipse y texto girados y volteados; el arrastre del asa, con los tests de `apply`.

## Riesgos y decisiones abiertas

- El giro se normaliza a `[0, 360)` en la interfaz, pero el documento guarda lo que recibe: un archivo escrito por otra herramienta puede traer 720° o -90° y se dibuja igual. El Inspector lo muestra normalizado y lo normaliza al primer cambio.
- Mayús+H choca con el atajo del Resaltador de la pestaña Dibujo en 03 §2 («Lápiz, Marcador, Resaltador: P, M, Shift+H»). Dibujo no existe todavía; cuando llegue, el titular decide cuál cambia (03 §9.2 no lista atajos de voltear).
- Girar una selección múltiple gira cada elemento alrededor de su propio centro, no del centro común, así que los elementos no cambian de sitio entre sí. Si el titular prefiere el centro común, es una spec nueva junto con el cuadro envolvente de FD-10.
- Un elemento volteado verticalmente tiene su «arriba» local abajo en pantalla, así que su asa de giro queda debajo. Es coherente con su caja local; se puede revisar con el titular.
- El texto girado deja la alineación a píxeles de egui; puede verse algo menos nítido que el recto. La exportación usa `gafira-text`, no egui, así que puede diferir en el suavizado (como ya pasaba con el texto recto).
- El tamaño del atlas de fuentes se lee durante el dibujo del lienzo. Si un panel dibujado después añade glifos y el atlas crece, el texto girado se ve mal un fotograma; el siguiente lo corrige porque se pide otro.
- Una versión anterior de Gafira ya leía y escribía el giro y el volteo, así que no hay riesgo de compatibilidad.
