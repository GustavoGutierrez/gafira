# 044 — Guías, Imán y mover con flechas

> **Estado:** done — 2026-11-17 — S44-01..S44-19 cubiertos por `gafira-ui` (`magnet::tests` s44, 7; `ruler::tests` s44, 7; `editor::tests` s44, 7), `gafira-ports` (`settings` s44, 1) y `gafira-format` (`gaf` s44, 1); 23 mutantes a mano muertos y 1 equivalente; 3140 tests.

## Objetivo

Colocar con precisión sin medir: el Imán (FD-06, D-16, D-28; 03 §9.1: «`Imán` atrae a guías, bordes de página, centros […] (umbral 6 px de pantalla) […]; `Alt` desactiva el Imán durante el arrastre»), las Reglas y las Guías (FD-03: «Las Guías se arrastran desde la regla […]; se pueden bloquear y borrar arrastrándolas fuera. Las guías son por página y se guardan en el documento») y mover la selección con las flechas (03 §9.2: «Mover 1 px · 10 px · Flechas · Shift+flechas»). Incluye el cambio de los atajos de Alinear que pidió el titular (D-88).

## Alcance / no alcance

Alcance:

- Atajos de Alinear (D-88, primer commit de la spec): `Alt+A` izquierda, `Alt+H` centro horizontal, `Alt+D` derecha, `Alt+W` arriba, `Alt+V` medio vertical y `Alt+S` abajo, en lugar de `Ctrl+Alt+L/C/R`:
  - cuentan solo con `Alt` exacto. AltGr no los dispara: en Windows llega como `Ctrl+Alt` y en Linux (X11 y Wayland con xkb) llega como nivel 3, no como `Alt`; así los caracteres que el teclado español escribe con AltGr (`@`, `#`, `[`, `{`, `€`…) nunca alinean;
  - en macOS `Option+letra` escribe un carácter (`å`…); la ventana cae entonces en la tecla física, así que `⌥A` alinea igual (VERIFICAR en una ventana real);
  - sin choques: `Shift+H` y `Shift+V` voltean (041), `W` del Editor de video no lleva modificador, y ningún atajo de 03 §9.2 usa `Alt` con esas letras. Nunca mientras se escribe en un campo;
  - el arreglo de la 043 que convertía una copia con `Alt` en «alinear al centro» se quita: `Ctrl+Alt+C` vuelve a ser una copia, como la ventana la entrega;
  - el tooltip y el submenú Organizar muestran el atajo de las seis acciones.
- Imán (FD-06) al mover y al redimensionar, con `gafira_layout::snap` y `gafira_layout::attractors`, que ya existían con sus pruebas:
  - destinos: bordes y centro de la página, bordes y centros de los demás elementos de la página (lo girado por su caja alineada a los ejes, D-28; lo que se mueve y sus descendientes no atraen) y las guías de la página si están visibles. Cada eje por separado; gana el más cercano y, en empate, la prioridad de D-16;
  - umbral de **6 px de pantalla** (`magnet::THRESHOLD_PX`), así que en la página es `6 / zoom`: se siente igual a cualquier zoom;
  - mover: la caja de todo lo que se mueve (la unión de cajas alineadas) se engancha por sus bordes y su centro, y el desplazamiento se suma a cada elemento (un grupo, con lo que tiene);
  - redimensionar: solo los bordes que arrastra el tirador se enganchan; el opuesto no se mueve; un enganche que dejaría un lado por debajo de 1 px no se aplica. Un elemento girado no se engancha al redimensionar: cambia de tamaño en sus ejes (041), que no son los de la página;
  - girar: sigue el ajuste a 15° con `Shift` de la 041;
  - **`Alt` mantenido suelta el Imán** durante el arrastre (03 §9.1, FD-06 b);
  - mientras engancha, una guía inteligente discontinua cruza la página en cada eje enganchado, con el token `canvas.guide` sobre un halo `canvas.selection.halo` (DIS-12); desaparece al soltar;
  - Imán activable en Ver → Imán, con `Ctrl+Shift+'` (03 §9.2) y con un botón con el icono `magnet` en la Barra inferior, encendido sobre `selection.bg` (par verificado `text`/`selection.bg`).
- Reglas y Guías (FD-03):
  - reglas de 20 px arriba y a la izquierda del lienzo, numeradas en px desde la esquina de la página de la selección (o la primera); paso de 1, 2 o 5 × 10ⁿ con al menos 64 px de pantalla entre números;
  - arrastrar desde la regla de arriba crea una guía horizontal, y desde la de la izquierda una vertical, en la página de la fila donde se suelta; soltarla de nuevo sobre la regla no crea nada;
  - arrastrar una guía la mueve (4 px de alcance, la más cercana); soltarla sobre su regla la borra. Las guías bloqueadas no se arrastran. El puntero muestra la flecha de mover sobre una guía o una regla;
  - una guía vertical se dibuja a lo alto de su página y una horizontal a lo ancho del lienzo, con `canvas.guide` sobre halo;
  - las guías viven en `Page.guides` (ya estaban en el modelo y en el `.gaf` desde la 019): crear, mover y borrar son un `SetProperty` de `Property::Guides` con un gesto propio, un paso de deshacer cada uno. Una guía nueva o movida se escribe en px;
  - Ver → Reglas (`Ctrl+Shift+R`) y Ver → Guías (`Ctrl+;`, con o sin `Shift`, porque el teclado español escribe `;` con `Shift`); las guías ocultas no se dibujan, no se arrastran y no atraen. Crear una guía con las guías ocultas las vuelve a mostrar.
- Ver → Reglas, Guías e Imán se recuerdan entre sesiones en `Settings.view` (`gafira-ports`), como el modo oscuro (021): todo encendido por defecto (FD-06: el Imán está activo por defecto); el archivo de ajustes solo escribe `view` cuando algo está apagado, así que uno anterior se lee y se escribe igual.
- Mover con flechas (03 §9.2): 1 px por pulsación y 10 px con `Shift`, para toda la selección (un grupo con lo que tiene). Las pulsaciones a menos de 500 ms unas de otras (mantener la tecla o pulsar seguido) son **un solo paso de deshacer**: son `SetProperty` sin gesto y el historial los fusiona por la ventana de 500 ms de 02 §4.3 («teclas de flecha, sliders»). Con `Ctrl` o `Alt` no mueven, y nunca mientras se escribe en un campo.
- Textos `view.rulers`, `view.guides` y `view.magnet` en `assets/i18n/{es,en}.toml` («Reglas», «Guías», «Imán»; «Rulers», «Guides», «Magnet», 03 §2.8.6).
- Icono `magnet` de Phosphor 2.1.1, peso Regular, sin modificar, del mismo tarball que las specs 037, 041 y 042 (sha256 `313332be…baadd`, comprobado), con su entrada en `assets/LICENSES.toml` (ARQ-75).

No alcance:

- Cuadrícula, Generador de guías (FD-04, FD-07), márgenes (FD-05) como destinos del Imán y escribir la posición de una guía con doble clic;
- las distancias a los vecinos que 03 §9.1 pide junto a la guía inteligente;
- el umbral configurable en Ajustes (FD-06: 2 a 16 px); se queda en 6;
- reglas en la unidad del proyecto: numeran en px;
- bloquear una guía desde la interfaz (el modelo sí tiene `locked` y se respeta).

## Dependencias

- Specs 019 (`.gaf` con `Page.guides`), 021 (ajustes de Ver persistidos), 037 (iconos), 040 (selección, `Transaction`), 041 (giro, ajuste a 15°), 042 (Alinear) y 043 (grupos).
- `gafira-layout::{snap, attractors, node_bounds}` y `gafira-edit::Property::Guides` ya existían con sus pruebas. `gafira-ports` gana `ViewSettings` (aditivo). Sin crates, dependencias ni capas nuevas; `Cargo.lock` no cambia.
- D-16 (prioridad), D-28 (Imán por defecto, umbral 6, cajas alineadas), D-88 (atajos de Alinear).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S44-01 | Alinear con `Alt` (D-88). | `Alt+W`, `Alt+V` y `Alt+S` dejan arriba, en medio y abajo (±1e-9) y un deshacer los devuelve; `Ctrl+Alt` con A/H/D/W/V y la antigua `Ctrl+Alt+L/C/R` no alinean; una copia con `Alt` copia; el tooltip de «Alinear arriba» nombra `Alt+W`. `Alt+D/H/A` (S42-14) y `Alt+H` en un grupo (S43-17) alinean. |
| S44-02 | Cuánto mueve una flecha. | 1 px por eje y 10 con `Shift`; otra tecla, nada. |
| S44-03 | El destino más cercano dentro del umbral. | Un borde a 3 px del de otro elemento va a él; a 7 no; lo que se mueve no se atrae a sí mismo; la caja de la selección es la unión de cajas alineadas (un cuadrado girado 45° mide 50√2 a cada lado del centro). |
| S44-04 | Cada eje por su cuenta. | En el mismo movimiento `x` va a la guía vertical y `y` al centro de la página; una guía en mm atrae en sus px. |
| S44-05 | Umbral en px de pantalla. | A 200 %, 3 px de página enganchan y 3,5 no; a 50 %, 10 enganchan y 12,5 no. |
| S44-06 | Apagado y guías ocultas. | Con el Imán apagado (o `Alt`) nada se engancha; con las guías ocultas la guía no atrae y la página sí. |
| S44-07 | Redimensionar. | El tirador de la esquina lleva el borde derecho a 500 y el inferior a 400 con la esquina opuesta fija; el izquierdo, el borde a 0 sin tocar `y`; uno girado no se engancha. |
| S44-08 | Agarrar una guía. | A 4 px se agarra y a 4,5 no; la más cercana gana; una vertical solo a lo alto de su página; una bloqueada nunca; una guía en mm donde están sus px. |
| S44-09 | Crear, mover y borrar. | Añadir va al final, mover conserva eje y bloqueo, borrar conserva el orden; índices que no existen no cambian nada; el puntero en px de página a cualquier zoom. |
| S44-10 | Reglas y página. | La guía horizontal vuelve a la regla de arriba y la vertical a la de la izquierda; una guía soltada entre dos páginas va a la más cercana. |
| S44-11 | Números de la regla. | Cada 100 px a 100 %, cada 200 a 50 %, cada 20 a 400 %, desde la esquina de la página. |
| S44-12 | Ajustes de Ver. | Encendidos por defecto; un archivo sin `view` los lee encendidos y sin cambios en lo que escribe; apagados vuelven iguales; una clave ausente es encendida. |
| S44-13 | Arrastrar con Imán. | Arrastrar 3 px deja el elemento en el centro de la página con su guía inteligente, que se va al soltar; con `Alt` se mueve 3; 10 px fuera, se mueve; con el Imán apagado no engancha; un paso de deshacer por arrastre. |
| S44-14 | Redimensionar con Imán. | A 200 %, a 3 px de página de la elipse el borde derecho va a ella con el izquierdo fijo; a 3,5 no. |
| S44-15 | Flechas. | Tres flechas y `Shift+abajo` mueven un grupo y otro elemento (3, 10), con los hijos, en un paso; con `Ctrl` o `Alt` no; sin selección no. |
| S44-16 | Guías y deshacer. | Crear y mover son dos pasos, cada deshacer devuelve el conjunto anterior. |
| S44-17 | Atajos de Ver. | `Ctrl+Shift+R`, `Ctrl+;` y `Ctrl+Shift+'` cambian Reglas, Guías e Imán y los ajustes los guardan; `Ctrl+R` solo no. |
| S44-18 | `.gaf`. | Guías en px y en mm, una bloqueada y otra fuera de la página vuelven iguales, sin redondear y en su orden, con `schemaVersion` 1. |
| S44-19 | Fotogramas. | Con reglas, guías y una guía inteligente, y con las guías ocultas, los fotogramas se dibujan y el documento no cambia. |

## TDD

Casos que fallaron primero:

1. `magnet::tests` (S44-02 a S44-07): escritos con `magnet.rs` vacío (cada función devolvía nada o su entrada); los 7 fallaron por sus aserciones. S44-06 volvió a fallar con la primera implementación por un error del test (la guía vertical estaba en el centro de la página, que también atrae); la guía pasó a 600 px.
2. `ruler::tests` (S44-08 a S44-11): escritos con `ruler.rs` vacío; los 7 fallaron.
3. S44-12 (`gafira-ports`) se escribió junto con `ViewSettings` y pasó al compilar.
4. Los tests del editor (S44-01, S44-13 a S44-17, S44-19) se escribieron con la interfaz conectada; su capacidad de fallar la prueban los mutantes de abajo.
5. S44-18 es de caracterización: el formato ya guardaba las guías y pasó al escribirlo.

## Evidencia

- Mutantes a mano (cada uno aplicado solo y con `cargo test -p gafira-ui --lib -- s44 s43 s42 s41 s40 s22`):
  1. el Imán sin mirar si está activo (S44-06, S44-13);
  2. lo que se mueve también atrae (S44-03);
  3. las guías ocultas atraen (S44-06);
  4. el umbral sin dividir por el zoom (S44-05, S44-14, y S40-05 y S43-14);
  5. el tirador izquierdo sin mover `x` (S44-07);
  6. lo girado también se engancha al redimensionar (S44-07);
  7. la caja de la selección como intersección (S44-03);
  8. `Shift` sin efecto en las flechas (S44-02, S44-15);
  9. el desplazamiento del Imán sin sumar al mover (S44-13);
  10. `Alt` sin soltar el Imán (S44-13);
  11. la guía inteligente sin borrar al soltar (S44-13);
  12. cada flecha con un gesto propio, un paso por pulsación (S44-15);
  13. las guías sin gesto propio, que se fusionan (S44-16);
  14. alinear con `Alt` no exacto, que también acepta AltGr de Windows (S44-01);
  15. `Ctrl+R` sin `Shift` cambia las reglas (S44-17);
  16. `Alt+W` alinea abajo (S44-01);
  17. las guías bloqueadas se agarran (S44-08);
  18. una guía vertical se agarra fuera de su página (S44-08);
  19. gana la guía más lejana (S44-08);
  20. mover una guía le quita el bloqueo (S44-09);
  21. borrar siempre la primera guía (S44-09; sobrevivió a la primera versión, que solo borraba la de índice 0, y el test ganó la de índice 1);
  22. una guía soltada siempre en la primera página (S44-10);
  23. la regla sin el paso de 2 (S44-11).

  Equivalente: en `snap_edges`, devolver el desplazamiento de un eje que el tirador no mueve. Ese eje se mide en `0`, que es el borde de la página, así que su desplazamiento siempre es 0.
- `cargo fmt --all --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 3140 tests, 0 fallos.
- `gafira-ports` cambió (paso 4): `cargo clippy -p gafira-testkit --all-targets --features ports --locked -- -D warnings` y `cargo test -p gafira-testkit --features ports --locked`, limpios. Sin dependencias nuevas ni cambios de crate, capas o `ci.yml` (pasos 5 y 6 no aplican); el manual técnico no cambió (paso 7 no aplica).
- `python3 tools/sync_agents.py --check`: 0.
- Icono: el tarball `@phosphor-icons/core-2.1.1.tgz` del registro npm tiene el sha256 de la spec 037; `magnet.svg` se copió sin cambios y su hash BLAKE3 en `assets/LICENSES.toml` lo comprueba S37-04.
- Sin prueba manual en la ventana: el arrastre de guías desde las reglas y su cursor se dibujan en fotogramas (S44-19) y su lógica se prueba pura (`ruler`); el gesto real con el puntero no se ha probado.

## Riesgos y decisiones abiertas

- `Alt` al pulsar el ratón mueve la ventana en algunos escritorios Linux (Xfce, Cinnamon, KDE; GNOME usa Super): si se mantiene antes de pulsar, el sistema gana. Pulsado ya durante el arrastre, suelta el Imán como pide 03.
- `Ctrl+Shift+'`: en el teclado español `Shift+'` escribe `?`, así que este atajo de 03 §9.2 no llega en ese teclado; Ver → Imán y el botón de la Barra inferior sí. VERIFICAR qué atajo prefiere el titular para teclados sin `'` sin modificador.
- La guía inteligente cruza la página entera y no muestra distancias (03 §9.1 lo pide; queda para otra spec).
- Una guía movida o creada se escribe en px aunque la original estuviera en mm (FD-03 guarda «la unidad en que se escribió»; arrastrar no escribe una unidad).
- Las reglas numeran en px y miden desde la página de la selección; con varias páginas, una guía horizontal se suelta en la página de esa fila.
- La ventana de 500 ms del historial también fusiona una flecha con un cambio de colocación escrito en el Inspector justo antes sobre el mismo elemento.
