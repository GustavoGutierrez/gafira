# 013 — Mutantes críticos de `gafira-text` y `gafira-raster` (solo tests)

> **Estado:** done — 2026-11-16 — D-66; ítems 74 y 75 de [07](../07-deuda-abierta.md) (§2.3, bloques BT1-BT6 y BR1-BR10). De 98 críticos: 57 muertos (42 de text, 15 de raster), 34 equivalentes excluidos en `.cargo/mutants.toml` (18 y 16) y 7 sin matar en la frontera exacta de una tolerancia (5 y 2); S13-01..S13-22 cubiertos por 24 tests nuevos; 2691 tests.

## Objetivo

Matar con tests los 98 mutantes críticos vivos del T4 de `gafira-text` (65) y `gafira-raster` (33) que lista 07 §2.3, o dejar escrito por archivo y línea por qué un mutante no se puede matar (D-66).

## Alcance / no alcance

Alcance:

- Tests nuevos en los módulos de tests de cada archivo fuente o en `crates/gafira-text/tests/`, con el estilo del crate.
- Los mutantes equivalentes se excluyen en `.cargo/mutants.toml` con su motivo (`AGENTS.md`, Testing).

No alcance:

- Ningún cambio de API pública ni de comportamiento: si un test destapa un defecto, se para y se abre otra spec.
- El T4 completo de los dos crates (07 §4, tanda D): aquí solo se corren los mutantes de la lista.
- Los 16 no críticos de 07 §2.1 y la suposición de `cost_hint` (07 §2.2).

## Dependencias

- Listas de 07 §2.3, corridas sobre 4a59889. `gafira-text` no ha cambiado desde entonces; `gafira-raster` sí (`blend.rs`, `effects/shadow.rs`, `transitions/`), pero los 33 nombres siguen en `cargo mutants --list`.
- `cargo-mutants` 27.1.0 con `.cargo/mutants.toml`.

## Requisitos

Los mutantes se citan como en 07 (`archivo:línea:col`, rutas desde `crates/<crate>/src/`).

### `gafira-text`

| ID | Requisito | Criterio de aceptación | Mata |
| --- | --- | --- | --- |
| S13-01 | FD-45: `balance` da el óptimo exhaustivo, empates incluidos. | Toda secuencia de hasta 7 líneas de alturas {1; 2; 3,5}, de 2 a 4 columnas, con y sin límite, coincide con una búsqueda exhaustiva; casos de empate a mano. | `columns.rs:223:17`, `229:46`, `229:63`, `230:32`, `230:50` (5) |
| S13-02 | FD-44 (e): la base de referencia es la de la primera línea con contenido. | Con la línea a 1e20 px del borde, sola o tras una línea vacía, el radio es `L / \|θ\|`. | `curve.rs:89:30` (3) |
| S13-03 | ARQ-110: una coordenada no finita de un clúster es `Err` sola. | NaN o infinito en `x` o en la base de una línea recta; un clúster malo en una recta junto a un arco bueno. | `curve.rs:159:45`, `159:64`, `171:37` (3) |
| S13-04 | ARQ-110: el fondo de un ítem, `top + height`, se comprueba. | A 1e38 px dos ítems son finitos y el tercero da `NonFinite { field: "output" }`. | `list.rs:119:60` (2) |
| S13-05 | ARQ-110: un NaN en cualquier punto de una curva del contorno. | `finite` es falso con un NaN en cada punto de `QuadTo` y `CurveTo`, de uno en uno. | `outline.rs:118:47`, `119:51`, `119:68` (3) |
| S13-06 | FD-41 (e): el interletrado de un run en un párrafo posterior es el suyo. | El segundo párrafo de `fit_box` da los avances de los mismos runs solos. | `layout.rs:304:51` (1) |
| S13-07 | FD-41 (e): runs sin interletrado conservan las ligaduras opcionales. | `fi` + `x` en Manrope con `fit_box`: un glifo de ligadura. | `layout.rs:327:37` (`*` por `+`), `327:60` (2) |
| S13-08 | «Fuente sin métricas»: la altura natural es ascendente + descendente + interlineado. | Fuentes con uno o dos de los tres a 0, ascendente igual al descendente o interlineado mayor que ambos: válidas y con esa distancia entre bases. | `layout.rs:443:36`, `443:48` (4) |
| S13-09 | ARQ-116: los glifos de un clúster empiezan uno tras otro. | `x` + U+0301 en Inter: el segundo glifo empieza en el avance de la `x`. | `layout.rs:472:25` (2) |
| S13-10 | FD-41: `Center` y `Right` en un párrafo RTL partido, con el espacio colgante a la izquierda. | Contenido centrado y pegado a la derecha, ±0,01 px, también en líneas de una letra. | `layout.rs:680:65` (`*`), `686:39`, `690:36`, `692:30` (4) |
| S13-11 | FD-41 (i): una línea justificada es contigua. | Cada clúster acaba donde empieza el siguiente y los avances suman el de la línea. | `layout.rs:722:30` (1) |
| S13-12 | Carga de fuentes: el directorio de tablas de una fuente y de cada fuente de una colección. | Fuentes mínimas a mano: una fuente, colección de 44 bytes, colección de dos, índice que no existe, directorio cortado. | `layout.rs:576:25`, `580:17`, `580:21` (6) |
| S13-13 | ARQ-110: `check_output` rechaza cada campo no finito solo. | Ocho campos, uno cada vez. | `layout.rs:760:72`, `764:17`, `765:17`, `767:25`, `768:25`, `769:25` (6) |

### `gafira-raster`

| ID | Requisito | Criterio de aceptación | Mata |
| --- | --- | --- | --- |
| S13-14 | ARQ-119 (W3C ClipColor): un máximo de exactamente 1 no se reescala. | `clip_color` devuelve el color bit a bit. | `blend.rs:81:10` (1) |
| S13-15 | «Imagen de tamaño 0»: un solo lado a 0 da hash 0. | Rgba8 y RgbaF32, (0, 3) y (3, 0). | ninguno: los dos son equivalentes |
| S13-16 | ARQ-122: el cubo son los 5 bits altos de cada canal. | Valores exactos de `bin_of`. | `palette.rs:28:60`, `28:66`, `28:93` (3) |
| S13-17 | FD-26: un cubo se une a su grupo al leerse, y la media se mueve antes del cubo siguiente. | Grises 104, 128 y 152 más tres colores: áreas `[240, 40, 10, 10, 10]`. | `palette.rs:158:29`, `158:31` (`==`), `158:47` (3) |
| S13-18 | ARQ-110: el `Err` nombra el elemento. | `index` 6 al premultiplicar y al desmultiplicar. | `pixel.rs:69:34`, `69:38` (4) |
| S13-19 | ARQ-120: en σ = 0,6 exacto el núcleo es la gaussiana muestreada. | Pesos ±1e-6; justo debajo, los taps de fuera a 0. | `effects/blur.rs:28:14` (1) |
| S13-20 | «Imagen de tamaño 0» con todos los deslizadores de Textura. | `Ok` con un lado a 0, en los dos formatos. | ninguno: `texture.rs:270:19` es equivalente |
| S13-21 | «División por alfa 0»: un alfa difuminado de 0. | Un píxel con el alfa f32 más pequeño queda igual con Nitidez y con Claridad. | `effects/texture.rs:212:12`, `218:12` (2) |
| S13-22 | ARQ-118: en 8 bits el borde suave pesa `from` por `1 − m`. | Blanco a negro, suavidad 1, p = 0,5: bytes `enc(1 − m)` ±1. | `transitions/wipe.rs:158:52` (1) |

## TDD

No hay código nuevo: los tests cubren reglas ya implementadas. Cada test que mata un mutante se comprobó en rojo contra él (cargo-mutants o mutante a mano), no contra el código sin mutar. S13-15 y S13-20 fijan el contrato de tamaño 0 y no matan ninguno: sus mutantes resultaron equivalentes.

## Evidencia

Corridas con `cargo mutants -p <crate> --jobs 8 -o <scratch> --re '^<nombre>$' ...`, un `--re` por cada nombre de 07 §2.3 (la configuración de `.cargo/mutants.toml` pone `--locked` y el perfil `mutants`):

| Crate | Críticos | Muertos | Equivalentes | Sin matar |
| --- | ---: | ---: | ---: | ---: |
| `gafira-text` | 65 | 42 | 18 | 5 |
| `gafira-raster` | 33 | 15 | 16 | 2 |
| Total | 98 | 57 | 34 | 7 |

- `gafira-text`: antes de los tests de columnas y de `layout.rs` quedaban 42 de 65; tras ellos, 24; el último test de alineación mató `layout.rs:680:65` (`*`). Los 23 que quedan son los 18 equivalentes y los 5 de abajo.
- `gafira-raster`: 33 corridos, 14 atrapados, 18 perdidos y 1 timeout, `palette.rs:158:29` (`false`), que se confirmó a mano: con la guarda en `false`, `fd_26_a_bin_joins_its_cluster_as_it_is_read_and_moves_its_mean` falla en el `assert` de las áreas.
- Exclusiones: `cargo mutants --list` pasa de 568 a 550 en `gafira-text` y de 1908 a 1892 en `gafira-raster`; las 34 líneas que desaparecen son exactamente las de la tabla de equivalentes.
- Gates: `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` y `python3 tools/sync_agents.py --check` en verde; `cargo test --workspace --locked`: 2691 tests, 0 fallos (2667 + 24 nuevos: 15 en text, 9 en raster).

## Mutantes equivalentes

Excluidos en `.cargo/mutants.toml`, cada uno con su motivo:

| Mutante | Motivo |
| --- | --- |
| `columns.rs:190:9` (`-` por `+`) | El reparto lleno solo siembra la cota; una semilla mayor poda menos y la búsqueda encuentra el mismo óptimo. |
| `columns.rs:233:58` (`-` por `+`) | Un `first` menor solo ensancha la ventana de candidatos. |
| `columns.rs:236:39` y `236:46` (`-` por `+` y por `/`, 4) | Cambian el orden en que se prueban los candidatos, no el conjunto. |
| `columns.rs:251:64` (`>` por `>=`) | Fines iguales son el mismo reparto con la misma diferencia. |
| `fit.rs:166:22` (`>` por `>=`) | Con `last == 0` el mínimo ya se probó; probarlo otra vez da la misma disposición. |
| `fit.rs:200:21` (`>` por `>=`) | Con `max_size == min_size` la rejilla repite el mismo tamaño. |
| `layout.rs:277:30` (`<` por `<=`) | Con interletrado 0 el tamaño del borde se multiplica por 0. |
| `layout.rs:303:21` (`<` por `<=`) | Un rango vacío no da estilo a ningún texto ni contiene ningún byte. |
| `layout.rs:327:37` (`*` por `/`) | `e / px != 0` vale lo mismo que `e * px != 0` con tamaños finitos y positivos. |
| `layout.rs:556:54` (`+` por `*`) | parley da al menos un byte a cada clúster, así que el `max` devuelve `end` igual. |
| `layout.rs:680:65` (`+` por `-`) | `start - adv <= start + adv` y el `max` siguiente repone el borde derecho. |
| `layout.rs:687:25` (`>` por `>=`) | Holgura 0 da el desplazamiento `-l`, que es el colgante que la línea ya tiene. |
| `layout.rs:708:58` y `708:67` (3) | Una línea con separadores pasa la caja como mucho en `FIT_TOLERANCE` (4e-6 del ancho): la holgura siempre está por encima de −1e-4 del ancho. |
| `dhash.rs:70:23` y `89:23` (2) | Con un lado a 0 no hay píxeles: las sumas son 0 y el hash también. |
| `effects/auto_adjust.rs:115:76` (`/` por `*`) | Temperatura y Matiz solo dependen de las razones entre canales de la media. |
| `effects/auto_adjust.rs:59:52` (`-` por `+` y por `/`, 2) | Los percentiles pedidos llegan a 99,5: el rango nunca pasa de `len - 1`. |
| `effects/blur.rs:11:27` (`\|\|` por `&&`) | NaN o un radio ≤ 0 dan `ceil(1,5 r)` NaN o ≤ 0, que el paso a `usize` deja en 0. |
| `effects/blur.rs:62:15`, `effects/shadow.rs:208:19`, `effects/texture.rs:270:19`, `transitions/guard.rs:87:20` (4) | Con un lado a 0 los planos están vacíos y ningún bucle escribe. |
| `palette.rs:28:38` y `28:72` (`\|` por `^`, 2) | Los tres campos de 5 bits no se solapan. |
| `transitions/fade.rs:90:31` (`<` por `<=`) | En 0,5 exacto las dos ramas dan el color solo. |
| `transitions/wipe.rs:144:38` (`==` y `>=`, 2) y `145:36` | Fuera de la suavidad la fórmula satura al mismo 0 o 1, y ahí `gap` nunca es 0. |

## Mutantes sin matar

No son equivalentes, pero solo cambian el resultado cuando una diferencia cae exactamente en una tolerancia redondeada; un test tendría que construir esa igualdad bit a bit, que no es una regla de la spec (07 §2.3: sin tests forzados):

| Mutante | Frontera |
| --- | --- |
| `columns.rs:223:23` (`>` por `==` y por `>=`) | Diferencia de alturas igual a `best + TIE`. |
| `columns.rs:233:65` (`<` por `<=` y por `==`) | Columna justo en `lower`, que incluye `TIE`. |
| `columns.rs:251:14` (`<` por `<=`) | Diferencia igual a `best − TIE`. |
| `palette.rs:158:31` (`<` por `<=`) | ΔE2000 de exactamente `MERGE_DELTA_E` (10). |
| `palette.rs:232:46` (`<` por `<=`) | Dos pares de grupos con el mismo ΔE2000 exacto. |

## Riesgos y decisiones abiertas

- Las exclusiones de `.cargo/mutants.toml` citan línea y columna: un cambio en esos archivos puede dejarlas sin efecto o apuntar a otro mutante; el siguiente T4 debe revisarlas.
- No se corrió el T4 completo de los crates (07 §4, tanda D); los mutantes que no están en la lista de 07 no se volvieron a probar. En `gafira-raster` la lista viene de 4a59889, anterior a los cambios de `blend.rs`, `effects/shadow.rs` y `transitions/`.
- `cost_hint` sigue como suposición (07 §2.2, ítem 75).
