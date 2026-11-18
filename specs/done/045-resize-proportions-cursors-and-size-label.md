# 045 — Proporción, cursores y tamaño al redimensionar

> **Estado:** done — 2026-11-17 — S45-01..S45-09 cubiertos por `gafira-ui` (`resize::tests` s45, 7; `magnet::tests` s45, 1; `editor::tests` s45, 1; S22-03 actualizado); 19 mutantes a mano muertos; 3149 tests.

## Objetivo

Que redimensionar con los tiradores sea predecible y legible (petición del titular, D-89; 03 §9.1): las esquinas conservan la proporción y `Shift` la libera, cada tirador muestra su cursor, y una etiqueta junto al puntero da el tamaño en vivo. Se apoya en el Imán de la spec 044.

## Alcance / no alcance

Alcance:

- Proporción (D-89):
  - los cuatro tiradores de esquina conservan la proporción del elemento por defecto, con la esquina opuesta fija; el lado que más crece en proporción manda y el otro le sigue;
  - `Shift` mantenido mientras se arrastra una esquina la libera (el redimensionado libre de la 022);
  - los tiradores de lado cambian una sola dimensión, con o sin `Shift`;
  - formas, imágenes, textos y grupos (la caja de un grupo, 043) por igual; lo girado o volteado conserva la proporción en sus propios ejes (041): la esquina opuesta se queda en su sitio de la página y el giro no cambia;
  - ningún lado baja de 1 px: el factor tiene un mínimo que deja los dos lados en 1 px o más con la misma proporción, así que arrastrar más allá de la esquina opuesta se detiene en la caja más pequeña de esa proporción (no se voltea).
- Imán con proporción (044): en una esquina que conserva la proporción, de los dos bordes que arrastra se engancha uno, el más cerca de su destino (`x` en un empate), y el otro lado se deduce de la proporción desde la esquina fija; la guía inteligente marca solo el borde que enganchó. Lo girado no se engancha, como en la 044.
- Cursor (cursores del sistema de egui, ninguno dibujado):
  - sobre un tirador y mientras se arrastra: la flecha doble cuyo eje está más cerca de la dirección del tirador en pantalla, por sectores de 45°: `ResizeNwSe` (arriba-izquierda y abajo-derecha) y `ResizeNeSw` (arriba-derecha y abajo-izquierda) en las esquinas, `ResizeHorizontal` y `ResizeVertical` en los lados;
  - la dirección es la del tirador en los ejes locales `(±1, ±1)` llevada por el giro, el volteo y el sesgo del elemento, no la diagonal de la caja: una esquina es diagonal sea cual sea la proporción, y un elemento girado 30° muestra en su lado derecho la diagonal más cercana;
  - el asa de giro: `Grab` encima y `Grabbing` al arrastrar (el sistema no tiene un cursor de girar; `Alias` no comunica giro);
  - el cursor encima usa la misma prueba que el agarre (`canvas::grip_hit`: el tirador más cercano a 24 px, `HANDLE_REACH`) y gana al de una guía (044).
- Etiqueta de tamaño mientras se redimensiona:
  - «320 × 180»: ancho × alto en px del documento, redondeados como el Inspector (como mucho un decimal, ninguno si es entero), y en `text.secondary` el cambio con signo («+20 × −11»; sin cambio, «0»);
  - fuente `caption` (Inter 12), `text` sobre `surface.raised` con borde `border` y radio pequeño, como un tooltip de 03 §8.1 (pares verificados de 03 §4);
  - a 16 px del puntero en la dirección del tirador, así que queda fuera del elemento; en un eje en que saldría de la vista del lienzo, al otro lado del puntero;
  - sin palabras: no hay textos de catálogo nuevos.
- 03 §9.1 y §9.2 recogen D-89; D-89 entra en `specs/00-decisiones.md`.

No alcance:

- `Alt` para redimensionar desde el centro (03 §9.1): sigue sin hacerse y hoy `Alt` suelta el Imán (044); el titular decide si `Alt` hace las dos cosas;
- bloqueo de proporción por elemento en el Inspector (FD-10 «candado») y escribir Ancho/Alto con proporción;
- voltear el elemento al pasar la esquina opuesta;
- distancias junto a la etiqueta.

## Dependencias

- Specs 022 (tiradores), 041 (redimensionar lo girado en sus ejes), 043 (grupos) y 044 (Imán, `snap_resize`, `canvas::MIN_SIDE`).
- Sin crates, dependencias ni capas nuevas; `Cargo.lock` no cambia; sin cambio de formato.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S45-01 | Cada esquina conserva 2:1. | 200 × 100 en (100, 100): abajo-derecha (+100, +10) da 300 × 150 en (100, 100); arriba-izquierda (−10, −50) da 300 × 150 en (0, 50); arriba-derecha (+20, 0) da 220 × 110 en (100, 90); abajo-izquierda (0, +50) da 300 × 150 en (0, 100); a 200 % la mitad de px de pantalla da lo mismo. |
| S45-02 | `Shift` y lados. | Con `Shift` la esquina da 300 × 110; un lado cambia una dimensión con o sin conservar. |
| S45-03 | Mínimo y más allá de la esquina opuesta. | Arrastrar hasta pasar la esquina opuesta deja 2 × 1 anclado en la esquina fija (en las dos diagonales); `scaled` por 2 y 0,5 desde la esquina opuesta; un factor que deja un lado bajo 1 px no da caja. |
| S45-04 | Girado. | A 90°, (0, +100) en la página da 300 × 150, conserva el giro y la esquina local (0, 0) queda en el mismo punto de la página (±1e-9). |
| S45-05 | Cursor por sector. | Las cuatro esquinas, los cuatro lados; a 90° el lado derecho es vertical y la esquina arriba-derecha `NwSe`; a 20° horizontal y a 30° `NwSe`; volteado en x la arriba-derecha es `NwSe`; la posición del elemento en pantalla no cambia el cursor. |
| S45-06 | Texto de la etiqueta. | «320», «180.5», «0» (no «-0»), «320 × 180», «+20 × −11», «0 × +0.5». |
| S45-07 | Sitio de la etiqueta. | 16 px hacia fuera del puntero en cada diagonal y en un lado; al borde derecho o inferior de la vista, al otro lado. |
| S45-08 | Imán con proporción. | El borde derecho a 2 px del centro de la página manda: 400 × 200; con el inferior a 1 px de otro elemento manda ese: 394 × 197; arriba-izquierda engancha a 0 con la otra esquina fija; sin destino, apagado o girado, igual. |
| S45-09 | Por el editor. | Una esquina de un cuadrado da 390 × 390 en un paso; mientras se agarra el fotograma pide `ResizeNwSe`; con `Shift` 390 × 375; a 3 px de la elipse, 400 × 400 con la guía inteligente en 760. S22-03 pasa a 40 × 40 (antes 40 × 20, sin proporción). |

## TDD

Casos que fallaron primero:

1. `resize::tests` (S45-01 a S45-07): escritos con `resize.rs` vacío (`resized` era el de la 022 y el resto devolvía nada). Fallaron 6 de 7; S45-02 pasó, porque la esquina con `Shift` y los lados son justo el redimensionado de la 022 (caracterización).
2. `magnet::tests` S45-08: escrito con `snap_resize_kept` vacío; falló. Al implementarlo volvió a fallar por un error del test (el otro elemento medía 10 px y su centro atraía); pasó a 100 px de alto.
3. `editor::tests` S45-09 se escribió con la interfaz conectada; S22-03 falló como se esperaba (40 × 40 en vez de 40 × 20) y se actualizó. La primera versión de S45-09 falló dos veces por el test: el fotograma ajusta el zoom a la ventana (se vuelve a 100 %) y el borde inferior libre a 725 enganchaba al de la elipse en 720 (se arrastra a 735).

## Evidencia

- Mutantes a mano, todos muertos (cada uno aplicado solo y con `cargo test -p gafira-ui --lib -- s45 s44 s43 s41 s22`):
  1. ninguna esquina conserva (S45-01, S45-03 a S45-05, S45-09, S22-03);
  2. manda el lado que menos crece (S45-01, S45-04, S45-09, S22-03);
  3. sin el mínimo de 1 px con proporción (S45-03);
  4. la caja nueva siempre desde la esquina local (0, 0) (S45-01, S45-03, S45-08);
  5. lo girado sin pasar el arrastre a sus ejes (S45-04);
  6. lo girado colocado como si fuera recto (S45-04);
  7. `NeSw` y `NwSe` cambiados (S45-05);
  8. el ángulo sin invertir `y` de pantalla (S45-05, S45-09);
  9. la dirección con la traslación del elemento (S45-05; sobrevivió a la primera versión, sin transformaciones con traslación, y el test ganó un caso desplazado);
  10. «-0» (S45-06);
  11. el cambio negativo con «+» (S45-06);
  12. la etiqueta sin darse la vuelta en el borde de la vista (S45-07);
  13. y 14. la etiqueta hacia atrás en un eje sin dirección (S45-07);
  15. el Imán con proporción elige el borde más lejano (S45-08);
  16. el Imán con proporción sin aplicar el enganche (S45-08, S45-09);
  17. `Shift` conserva en vez de liberar (S45-09, S22-03);
  18. la esquina con proporción enganchada borde a borde, sin deducir el otro lado (S45-09);
  19. el cursor del tirador agarrado sin el giro del elemento (S45-09).
- `cargo fmt --all --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 3149 tests, 0 fallos.
- `gafira-testkit`, `gafira-ports`, dependencias, capas, `ci.yml` y el manual técnico no cambiaron (pasos 4 a 7 no aplican).
- `python3 tools/sync_agents.py --check`: 0.
- Sin prueba manual en la ventana: el cursor y la etiqueta se dibujan en un fotograma con el tirador agarrado (S45-09); el cursor encima de un tirador usa `grip_hit`, ya probado, pero no se ha visto con un puntero real en Linux, Windows ni macOS.

## Riesgos y decisiones abiertas

- `Alt` para redimensionar desde el centro (03 §9.1) choca con `Alt` para soltar el Imán (FD-06, 044); queda para que el titular decida.
- El aspecto de los cursores `ResizeNeSw`/`ResizeNwSe` depende del tema de cursores del sistema (en algunos temas de Linux faltan y se ven como la flecha normal).
- El cursor de giro es `Grab`/`Grabbing`: no hay uno de girar en el sistema. Un cursor dibujado sería otra spec.
- En un elemento girado con sesgo la etiqueta sigue la dirección del tirador en pantalla; con sesgos extremos puede tocar el elemento.
- El tamaño de la etiqueta es el de la colocación local (ancho y alto antes de girar), como el Inspector, no el de la caja alineada.
