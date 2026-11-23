# 047 — Alt redimensiona desde el centro, Ctrl suelta el Imán

> **Estado:** done — 2026-11-17 — S47-01..S47-07 cubiertos por `gafira-ui` (`resize::tests` s47, 5; `magnet::tests` s47, 1; `editor::tests` s47, 1; S44-13 pasa a `Ctrl`); 24 mutantes a mano muertos; 3156 tests.

## Objetivo

Resolver el choque que dejó abierto la 045 (decisión del titular, D-90): `Alt` mantenido al arrastrar un tirador redimensiona desde el centro (03 §9.1), y el Imán se suelta con `Ctrl` (`Cmd` en macOS) en lugar de `Alt`. Así `Alt` ya no hace dos cosas a la vez, y soltar el Imán no depende de una tecla que algunos escritorios Linux usan para mover la ventana.

## Alcance / no alcance

Alcance:

- Desde el centro (`Alt` mantenido mientras se arrastra un tirador):
  - el centro del elemento se queda en su sitio de la página y los lados opuestos se mueven a la vez y lo mismo: el lado arrastrado avanza `d` y el opuesto `d` hacia el otro lado, así que la dimensión cambia `2d`;
  - se combina con la 045 (D-89): una esquina conserva la proporción por defecto, desde el centro; `Shift` la libera; `Alt+Shift` es libre y desde el centro; el lado que más crece en proporción manda, como en la 045;
  - un tirador de lado con `Alt` cambia solo esa dimensión, simétrica respecto del centro;
  - lo girado, volteado o sesgado redimensiona en sus propios ejes (041): el arrastre se pasa a los ejes locales y el centro en la página no cambia, ni el giro;
  - grupos (043) por igual: la caja del grupo crece desde su centro y sus hijos la siguen. La selección múltiple no tiene tiradores de redimensionar (los tiradores solo agarran con un elemento o un grupo, 040/045), así que no aplica;
  - mínimo y más allá del centro: ningún lado baja de 1 px. Si el borde arrastrado pasa el centro, el elemento se queda en el mínimo centrado (1 px en esa dimensión; con proporción, la caja más pequeña de la misma proporción); **no se voltea**, como en la 045.
- Imán desde el centro (044): se engancha el borde arrastrado (el que manda) y el opuesto lo refleja respecto del centro. Libre: cada borde arrastrado se engancha en su eje y el opuesto se mueve lo mismo hacia el otro lado; si así un lado bajaría de 1 px, ese enganche se descarta. Con proporción: de los dos bordes de la esquina se engancha el más cerca de su destino (`x` en un empate) y el otro lado se deduce de la proporción, desde el centro. La guía inteligente marca el borde que enganchó. Lo girado no se engancha, como en la 044.
- `Ctrl` (`Cmd` en macOS; `modifiers.command` de egui, como el resto de atajos del editor) mantenido mientras se mueve o se redimensiona suelta el Imán; `Alt` ya no lo hace. `Alt+Ctrl` al redimensionar: desde el centro y sin Imán.
  - Durante el arrastre solo cuenta el estado de los modificadores: mantener `Ctrl` no dispara atajos (los atajos con `Ctrl` necesitan una tecla) ni copia (la ventana convierte `Ctrl+C/X/V` en eventos de portapapeles solo al pulsar la letra, 043).
  - `Ctrl+clic` en Capas (040) sigue igual: es otro panel y el lienzo no mira `Ctrl` al pulsar.
- La etiqueta de tamaño de la 045 se muestra igual al redimensionar desde el centro (su dibujo no depende del modo).
- D-90 entra en `specs/00-decisiones.md`; 03 §9.1 y 04 FD-06 dejan de decir que `Alt` suelta el Imán; las notas de `Alt` de la 044 y la 045 remiten a D-90.

No alcance:

- `Alt+arrastrar` para duplicar (03 §9.1 «Duplicar»): no está hecho y no cambia; con D-90 `Alt` sobre un tirador es desde el centro, y sobre el cuerpo del elemento sigue libre para duplicar en otra spec.
- Voltear al pasar el centro o la esquina opuesta.
- Redimensionar la caja de una selección múltiple.
- Un `Alt` pulsado antes de hacer clic en escritorios Linux que lo usan para mover la ventana: el sistema gana (044); pulsado ya durante el arrastre, funciona.

## Dependencias

- Specs 022 (tiradores), 041 (ejes locales), 043 (grupos), 044 (Imán, `snap_resize`) y 045 (proporción, `resize::resized`, `scaled`, etiqueta).
- Sin crates, dependencias ni capas nuevas; `Cargo.lock` no cambia; sin cambio de formato.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S47-01 | Desde el centro, libre. | 200 × 100 en (100, 100), centro (200, 150): el lado derecho +20 da 240 × 100 en (80, 100); el izquierdo −20 lo mismo; el inferior +10 da 200 × 120 en (100, 90); la esquina abajo-derecha libre (+20, +10) da 240 × 120 en (80, 90); a 200 % la mitad de px de pantalla da lo mismo; el centro no cambia. |
| S47-02 | Desde el centro, con proporción. | Abajo-derecha (+50, +5) da 300 × 150 en (50, 75); arriba-izquierda (−50, −5) igual; arriba-derecha (0, −25) da 300 × 150 (manda el alto); sin arrastre, el original. |
| S47-03 | Mínimo y más allá del centro. | El lado derecho −150 (pasa el centro) da 1 × 100 en (199.5, 100), sin voltear; la esquina con proporción más allá del centro da 2 × 1 en (199, 149.5); libre, 1 × 1 en (199.5, 149.5); `scaled_centred` por 0,5 da 100 × 50 en (150, 125) y un factor que deja un lado bajo 1 px no da caja. |
| S47-04 | Girado. | A 90°, el lado derecho local con (0, +50) en la página da 300 × 100 con el mismo centro de página y el mismo giro; la esquina con proporción da 300 × 150 con el mismo centro (±1e-9). |
| S47-05 | Imán desde el centro. | Lado derecho libre a 2 px del borde izquierdo de otro elemento (320): 240 × 100 en (80, 100) con la guía en 320; lado izquierdo a 3 px del borde de página: 400 de ancho en x = 0; esquina con proporción, el borde derecho a 2 px del centro de la página y el inferior a 4 de otro elemento: manda el derecho, 600 × 300 en (−100, 0) y guía solo en x; un enganche que dejaría un lado bajo 1 px reflejado se descarta; sin destino, apagado o girado, igual. |
| S47-06 | Modificadores. | `Alt` → desde el centro; `Ctrl` (`command`) → sin Imán; `Alt+Ctrl` → las dos; `Shift` → solo `shift`; `Cmd` de macOS (`mac_cmd` con `command`) suelta el Imán; `Ctrl` sin `command` (el `Ctrl` de macOS) no; ninguno → nada. |
| S47-07 | Por el editor. | En el cuadrado de 360 en (360, 360) a 100 %: `Alt` + esquina (+30, +5) da 420 × 420 en (330, 330) en un paso de deshacer; `Alt+Shift` (+30, +15) da 420 × 390 en (330, 345); `Alt` + lado derecho +37 engancha a la elipse (760): 440 de ancho en x = 320 con la guía en 760; con `Alt+Ctrl`, 434 en x = 323 y sin guía; con `Ctrl` sin `Alt`, 397 en x = 360. Un fotograma con `Ctrl` mantenido durante el arrastre no copia ni cambia el documento, y el arrastre sigue. Un grupo con `Alt` en una esquina crece desde su centro. S44-13 suelta el Imán con `Ctrl`. |

## TDD

Casos que fallaron primero (los 7 a la vez, cada uno en su primera aserción y por la razón esperada):

1. `resize::tests` S47-01 a S47-04 y S47-06: escritos con `resized_centred` que devolvía el original, `scaled_centred` que devolvía `None` y `drag_keys` que devolvía los modificadores vacíos.
2. `magnet::tests` S47-05: escrito con `snap_resize_centred` que devolvía lo que recibía.
3. `editor::tests` S47-07: escrito con `Asked.centre` sin conectar.

Tras implementarlos pasaron los 7 sin tocar los tests, y siguieron en verde S45, S44, S43, S41 y S22 (89 tests).

## Evidencia

- Mutantes a mano, todos muertos (cada uno aplicado solo y con `cargo test -p gafira-ui --lib --locked -- s47 s45 s44 s43 s41 s22`):
  1. `Alt` no va desde el centro en `drag_keys` (S47-06, S47-07);
  2. el Imán se suelta con `ctrl` en vez de `command`, así que `Cmd` no vale en macOS (S47-06, S47-07);
  3. `Alt` suelta el Imán, como antes de D-90 (S47-06, S47-07);
  4. el ancho crece una vez el arrastre, no dos (S47-01 a S47-04, S47-07);
  5. el alto crece una vez el arrastre, no dos (S47-01, S47-02, S47-04, S47-07);
  6. ninguna esquina conserva desde el centro (S47-02 a S47-04, S47-07);
  7. manda el lado que menos crece (S47-02, S47-04, S47-07);
  8. sin el mínimo de 1 px con proporción (S47-03);
  9. el lado libre se voltea al pasar el centro (S47-03);
  10. la caja anclada a la izquierda en vez de al centro (S47-01 a S47-05, S47-07);
  11. la caja anclada arriba en vez de al centro (S47-01 a S47-05, S47-07);
  12. lo girado sin pasar el arrastre a sus ejes (S45-04, S47-04);
  13. `scaled_centred` sin la comprobación de 1 px (S47-03);
  14. el Imán desde el centro sin reflejar el enganche (S47-05, S47-07);
  15. un enganche reflejado que deja un lado bajo 1 px se aplica (S47-05);
  16. con proporción manda el borde más lejano (S47-05);
  17. lo girado se engancha (S47-05);
  18. el enganche libre no se aplica (S47-05, S47-07);
  19. con proporción se engancha borde a borde, sin deducir el otro lado (S47-05);
  20. el editor ignora `Alt` (S47-07);
  21. desde el centro se engancha como un redimensionado normal (S47-07);
  22. `Ctrl` no suelta el Imán (S44-13, S47-07);
  23. `Asked::hold_keys` pone `Alt` en el Imán (S47-07);
  24. `Asked::hold_keys` no pasa `Alt` (S47-07).
  El 23 sobrevivió en la primera versión, cuando el lienzo copiaba las teclas en `Asked` dentro del cierre de la interfaz, que ningún test alcanza sin ventana; se movió a `Asked::hold_keys`, que S47-07 prueba.
- `cargo fmt --all --check`: limpio.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: limpio.
- `cargo test --workspace --locked`: 3156 tests, 0 fallos.
- `gafira-testkit`, `gafira-ports`, dependencias, capas, `ci.yml` y el manual técnico no cambiaron (pasos 4 a 7 no aplican; en `gafira-layout` solo cambió un comentario).
- `python3 tools/sync_agents.py --check`: 0.
- La etiqueta de tamaño: su dibujo (`size_label`) solo mira el tirador agarrado, el origen y la colocación actual, no el modo, así que se muestra igual desde el centro; S47-07 dibuja un fotograma con el tirador agarrado, pero no comprueba los píxeles de la etiqueta.
- Sin prueba manual en la ventana: `Alt` y `Ctrl` se han probado por `drag_keys` y `Asked::hold_keys`, no con un teclado real en Linux, Windows ni macOS.

## Riesgos y decisiones abiertas

- Ninguna decisión abierta: D-90 cierra la de la 045.
- En escritorios Linux que mueven la ventana con `Alt+arrastre` (Xfce, Cinnamon, KDE), pulsar `Alt` antes de hacer clic en el tirador lo gana el sistema; hay que pulsarlo ya agarrado. Soltar el Imán ya no depende de esa tecla.
- En Windows, AltGr llega como `Ctrl+Alt`: con AltGr mantenido se redimensiona desde el centro y sin Imán. Es coherente con D-90 y no dispara ningún atajo.
- `Alt+arrastrar` para duplicar (03 §9.1) sigue pendiente; sobre el cuerpo del elemento `Alt` no hace nada hoy, y sobre un tirador es desde el centro.
- El Imán desde el centro engancha solo el borde arrastrado; el reflejado no busca destino propio.
