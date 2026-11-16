# 013 — Mutantes críticos de `gafira-text` y `gafira-raster` (solo tests)

> **Estado:** in-progress — 2026-11-16 — D-66; ítems 74 y 75 de [07](07-deuda-abierta.md) (§2.3, bloques BT1-BT6 y BR1-BR10).

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

No hay código nuevo: los tests cubren reglas ya implementadas. Cada test se comprueba en rojo contra su mutante (cargo-mutants o mutante a mano), no contra el código sin mutar.

## Evidencia esperada

- `cargo mutants -p <crate> --jobs 8 --re '<nombre>'...` sobre la lista de 07 §2.3, antes y después de los tests.
- Gates de `AGENTS.md` en verde.
- Ítems 74 y 75 de 07 actualizados con las cifras.

## Riesgos y decisiones abiertas

- Algunos mutantes solo cambian el resultado en la frontera exacta de una tolerancia (`TIE` de las columnas, `MERGE_DELTA_E` de la paleta). Se documentan sin un test forzado (07 §2.3).
