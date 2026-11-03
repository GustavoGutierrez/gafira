# 07 — Deuda abierta de Gafira

| Campo | Valor |
|---|---|
| Documento | Especificación 07 |
| Estado | Vigente (no ejecutable): inventario, sin requisitos propios |
| Alcance | Ítems 39 a 77 de `specs/README.md`, sobrevivientes críticos del T4 de `gafira-text` y `gafira-raster` a 4a59889, y el resto del T4 que se debe |
| Fuentes | `specs/README.md` (sección «Pending follow-ups»), listas crudas de `cargo-mutants`, `AGENTS.md` (críticos y no críticos, mutantes colgantes), 02 §4.17 |
| Convenciones | Este documento no crea IDs ni cambia requisitos o decisiones. Los números de ítem son los del README y no se renumeran. Un desajuste con el README se anota en §1.3 y no se corrige aquí. Las rutas de mutantes son relativas a `crates/<crate>/src/`, salvo en §3 y en los anexos, que parten de `crates/` |
| Copia duradera | Las listas crudas estaban en el scratchpad de una sesión, que es temporal. Este documento es la copia que queda |

## 0. Resumen

| Medida | Cifra |
|---|---|
| Ítems 39 a 77 | 39: 34 abiertos, 2 remiten a otro ítem (70, 71), 3 cerrados o solo registro (51, 64, 65) |
| Sobrevivientes del T4 de `gafira-text` | 66: **65 críticos**, 1 no crítico |
| Sobrevivientes del T4 de `gafira-raster` | 48: **33 críticos**, 15 no críticos |
| Total crítico de los dos crates | **98** |
| Mutantes colgantes confirmados | 8, no son deuda (§3.1) |

La cifra 104 (65 + 39) del informe de mutantes y del README no se sostiene al clasificar la lista cruda de raster; el detalle está en §2.1.

## 1. Seguimientos abiertos (ítems 39 a 77)

La columna «Se cierra cuando» resume el `Start` del README. El texto completo y el motivo siguen allí.

### 1.1 Abiertos

| Ítem | Qué falta | Se cierra cuando |
|---|---|---|
| 39 | Una entrada heredada de un outcome cerrado se toma como activa en una rama sin fusionar (W9 de `odd-lite`). | La regla de resume de `coordinator.md` y `AGENTS.md` trata como heredada una entrada de una rama sin commit propio sobre `develop`. |
| 40 | `prepare_dir` crea carpetas antes de comprobar la ruta resuelta. | La comprobación va antes de `create_dir_all` y un test confirma que un rechazo no deja carpetas. |
| 41 | Timeout por el bloqueo del caché de paquetes de cargo que un re-run atrapa; «Hanging mutants» no lo nombra. | Se decide si la regla lo cubre; si no, se nombra en `AGENTS.md` y en `verify`, o se sube `--minimum-test-timeout`. |
| 42 | 02 sigue con la prueba gratuita y el modo de solo lectura que D-12 sustituyó; Q-19 difiere entre documentos. | 02 pasa al modo sin licencia, Q-19 queda cerrada en 02 y 05 como en 01, y 02 entra en la tabla de D-12. |
| 43 | `Display` de `Cancelled` y `ParamError` sin test. | El titular decide si la spec fija el texto: un test por `Display`, o se acepta la exclusión. |
| 44 | Diez sobrevivientes no críticos de `gafira-testkit`. | Se resuelve con el ítem 38, y se decide test o exclusión para `axis_length` y `failure_persistence`. |
| 45 | ARQ-102 omite 8 enums sin dueño y su «Cumplimiento» está desactualizado. | Cada enum tiene dueño en la fila de 02 y «Cumplimiento» está al día. |
| 46 | El registro de Flow-change de D1 no da pack ni motivo. | El próximo cambio de flujo los registra y se decide si la plantilla los exige. |
| 47 | Los tests de atomicidad de `contract_cancel.rs` pierden potencia en máquinas pequeñas. | El titular aprueba `loom` o acepta el test actual. |
| 48 | El manual técnico omite `gafira-doc` como dependencia de `gafira-pathops`. | La fila y la línea 89 quedan corregidas en los dos idiomas, según el `Cargo.toml`. |
| 49 | T4 de `gafira-audio` sin resolver (§3.2, anexo A). | El titular decide si hay ronda de arreglo; en ese caso, `specifier` y test primero por sobreviviente. |
| 50 | T4 de `gafira-segment` completo, de `gafira-testkit` y de `layer-2-scaffold` (§3.3). | Corre el T4 de `layer-2-scaffold`; los de segment y testkit solo si el titular lo reabre. |
| 52 | V07 marcada como fiable con IoU 0.53. | `specifier` revisa el umbral de fiabilidad, con test primero. |
| 53 | ARQ-43 a 96.8 ms en un hilo con carga, fuera de D-03. | Se mide en hardware D-03 (ítem 55). |
| 54 | Regla de banda stray fuera de ARQ-133. | `specifier` la añade a la spec, test después. Ver §1.3. |
| 55 | Q-48 abierta en hardware D-03. | Se mide en D-03; va al titular solo si `bench_segment_4000` falla. |
| 56 | Redacción desactualizada en 01 P-04 y 04 FD-38. | `specifier` actualiza la redacción. |
| 57 | Decisión sin tomar: el texto `modificado` del splash. | El titular la decide. |
| 58 | W5 a W9 de RS5/RS6: ambigüedades de ARQ-122 y FD-26, huecos de test y cuatro mutantes de cierre no reaplicados. | Huecos de test primero; `specifier` para W5 y W6. |
| 59 | `GpuEffect::bind` no recibe `cx`. | `specifier` decide la firma, con test primero. |
| 60 | Tamaño mínimo de FD-28 (b) sin fijar. | El titular lo fija; después, test primero. Ver §1.3. |
| 61 | W37 (FD-45 «Funciona con Listas» sin construir), W38 (un run vacío mueve el salto de línea) y S19. | `specifier` y test primero para W37 y W38; S19 del mismo informe. |
| 62 | W13 sin registro TDD; unión de párrafos de `RichText` sin cubrir. | Test primero para la unión; el registro se corrige en el próximo cambio. |
| 63 | W22 y W23 sin test en `loudness.rs`; S15 gasta memoria por frame. | Test primero para W22 y W23; S15 con suma corrida o energía por paso. |
| 66 | «S1 con filas de cuelgue de layout», sin origen en los informes leídos. | El coordinador confirma el origen antes de actuar. |
| 67 | Avisos de SC1: `assert_no_alloc` como dependencia normal e `icu_plurals` opcional. | Candidato del informe: `--all-features` en el paso del CI. |
| 68 | Memoria de mipmaps a 16 384 px, sin origen en los informes leídos. | El titular decide si se mide o se acota. |
| 69 | `proptest-regressions/` de `gafira-raster` sin decidir. | Se decide borrar o versionar. Ver §1.3. |
| 72 | Fila de diseño P7 y XP3 sin actualizar tras TX. | P7 y XP3 reflejan lo que entregó TX1; solo docs. |
| 73 | T4 de `gafira-segment` parado por orden del titular (§3.3, anexo B). | El titular decide si se prueba el resto; los sobrevivientes, test primero. |
| 74 | T4 de `gafira-text`: 65 críticos (§2). | El titular decide cuándo; después, `specifier` y test primero. Un T4 del crate sin críticos vivos lo confirma. |
| 75 | T4 de `gafira-raster`: 33 críticos (§2). | Se confirma la suposición de `cost_hint` (§2.2); después, `specifier` y test primero. Un T4 del crate sin críticos vivos lo confirma. |
| 76 | SG3-W8: Q-48 y la cita de §4.17 aún dan 1055 ms y 43 ms. | `specifier` pone las cifras medidas tras el arreglo. |
| 77 | SG3-W9: el stub de test imprime un `WARNING ... 0/12` falso. | Se corrige el stub o la condición en `gate.rs`, test primero. |

### 1.2 Cerrados o sin trabajo propio

- **51**: cerrado. El verify de cierre de SG3 dio PASS a 4a59889; queda como registro.
- **64**: registro. El titular aceptó la ruptura de TDD de W1 en SG1; no queda acción.
- **65**: registro. Aviso de tamaño de `pi-agent-sync`, ya integrado; no queda acción sobre esa entrega.
- **70**: sin trabajo propio. Es el ítem 47.
- **71**: sin trabajo propio. Es el ítem 42.

### 1.3 Desajustes con el README, sin corregir

Comprobados contra el árbol en esta rama. El README sigue mandando hasta que el titular decida.

| Ítem | Desajuste |
|---|---|
| 39 | Lo que pide «incluir `develop` en la comparación» ya está en `AGENTS.md:71` y `coordinator.md:17`. Sigue faltando tratar como heredada la entrada de una rama sin commit propio. `backup/pi-agent-sync-preverify` sigue 3 commits por delante. |
| 42 | Los números de línea de 02 han cambiado: Q-19 está en la 1860 (el ítem dice 1772) y Q-20 en la 1861 (dice 1773). |
| 53 | Da un margen de 2.3x para 96.8 ms. Con 100 ms de límite, ese margen es el de 43 ms (la cifra de Q-48); 96.8 ms deja 1,03x. Los 818 ms del ítem no aparecen en los informes de SG3. |
| 54 | ARQ-133 ya recoge la regla de la banda (D-47, aplicado a 02 §4.17.6, ARQ-131 y ARQ-133). El ítem parece hecho. |
| 56 | El ítem no dice qué redacción está mal. El origen probable es SG3-W1, y P-04 ya da «9 de 12». No se pudo confirmar qué falta. |
| 60 | El mínimo ya está en ARQ-122 desde `1b159f9` (128 px con media 2×2, 640 px con bilineal o vecino). 04 FD-28 (b) sigue sin cifra. |
| 69 | `crates/gafira-raster/proptest-regressions/` no existe en ninguno de los dos worktrees. No queda nada que decidir salvo que exista en otra copia. |
| 49, 50, 73 | El 50 dice que el titular saltó el T4 de segment; el 73 y la tabla de estado hablan de una corrida parcial parada. Es la segunda (§3.3). El 73 no cuenta 18 no viables: 983 + 236 + 34 + 18 = 1271. |
| 75 y tabla de estado | Dan 39 críticos. El mismo ítem cuenta 11 `cost_hint` como no críticos, más `error.rs:40` y `view.rs:211`: 48 − 13 = 35, no 39 (§2.1). |

## 2. Sobrevivientes críticos del T4 a 4a59889

Corridas sobre 4a59889. `gafira-text` y `gafira-raster` no han cambiado desde entonces, así que las líneas valen en esta rama. Por orden del titular, no se corrigen antes de publicar (ítem 74).

Crítico según `AGENTS.md`: validación, ids, transformaciones, límites y toda regla de spec con invariante. No crítico: `Display`, accesores triviales y ayudas de test.

### 2.1 Recuento

| Crate | Lista cruda `missed` | Del re-run de timeouts | Sobrevivientes | No críticos | Críticos |
|---|---|---:|---:|---:|---:|
| `gafira-text` | 66 | 0 | 66 | 1 | **65** |
| `gafira-raster` | 47 | 1 | 48 | 15 | **33** |
| Total | 113 | 1 | 114 | 16 | **98** |

Se clasificó cada línea. El informe daba 65 + 39 = 104. Su propio desglose de raster da 47 − 13 + 1 = 35. Esta lectura baja dos más: `Cut::params` y `Dissolve::params` (`transitions/cut.rs:42`, `transitions/dissolve.rs:48`) solo devuelven `&self.schema`, y ese esquema está vacío, igual que el `Default` que pone el mutante. Son accesores triviales y mutantes equivalentes: ningún test puede matarlos. Si el titular prefiere contarlos como críticos, el total es 100.

La cifra depende de una suposición: si `cost_hint` resultara ser un contrato, los 11 mutantes de §2.2 pasarían a críticos y el total sería 109.

Los no críticos, línea a línea:

| Crate | Archivo:línea:col | Función | Mutación | Motivo |
|---|---|---|---|---|
| `gafira-raster` | `effects/shadow.rs:149:22` | `<impl Effect for ParallelShadow>::cost_hint` | `replace * with +` | suposición, ver §2.2 |
| `gafira-raster` | `effects/shadow.rs:149:59` | `<impl Effect for ParallelShadow>::cost_hint` | `replace + with *` | suposición, ver §2.2 |
| `gafira-raster` | `effects/shadow.rs:151:40` | `<impl Effect for ParallelShadow>::cost_hint` | `replace * with +` | suposición, ver §2.2 |
| `gafira-raster` | `effects/shadow.rs:151:47` | `<impl Effect for ParallelShadow>::cost_hint` | `replace + with *` | suposición, ver §2.2 |
| `gafira-raster` | `effects/texture.rs:241:54` | `<impl Effect for Texture>::cost_hint` | `replace * with +` | suposición, ver §2.2 |
| `gafira-raster` | `effects/texture.rs:241:54` | `<impl Effect for Texture>::cost_hint` | `replace * with /` | suposición, ver §2.2 |
| `gafira-raster` | `effects/texture.rs:241:58` | `<impl Effect for Texture>::cost_hint` | `replace * with +` | suposición, ver §2.2 |
| `gafira-raster` | `effects/texture.rs:241:63` | `<impl Effect for Texture>::cost_hint` | `replace * with +` | suposición, ver §2.2 |
| `gafira-raster` | `effects/texture.rs:241:77` | `<impl Effect for Texture>::cost_hint` | `replace + with *` | suposición, ver §2.2 |
| `gafira-raster` | `effects/texture.rs:243:30` | `<impl Effect for Texture>::cost_hint` | `replace > with >=` | suposición, ver §2.2 |
| `gafira-raster` | `effects/texture.rs:248:28` | `<impl Effect for Texture>::cost_hint` | `replace > with >=` | suposición, ver §2.2 |
| `gafira-raster` | `error.rs:40:9` | `<impl fmt::Display for BufferError>::fmt` | `replace body with Ok(Default::default())` | texto de `Display`, solo para el log (02 §7.3) |
| `gafira-raster` | `transitions/cut.rs:42:9` | `<impl Transition for Cut>::params` | `replace body with Box::leak(Box::new(Default::default()))` | accesor trivial de un esquema vacío; `ParamSchema::new(Vec::new())` es igual a `Default`, así que el mutante es equivalente |
| `gafira-raster` | `transitions/dissolve.rs:48:9` | `<impl Transition for Dissolve>::params` | `replace body with Box::leak(Box::new(Default::default()))` | accesor trivial de un esquema vacío; `ParamSchema::new(Vec::new())` es igual a `Default`, así que el mutante es equivalente |
| `gafira-raster` | `view.rs:211:9` | `AlphaMask::height` | `replace body with 1` | accesor trivial |
| `gafira-text` | `error.rs:32:9` | `<impl fmt::Display for TextError>::fmt` | `replace body with Ok(Default::default())` | texto de `Display`, solo para el log (02 §7.3) |

### 2.2 `cost_hint`: sigue siendo una suposición

02 §4.17 dice que `Cost` da «unidades de trabajo abstractas, calculadas con los píxeles y el tamaño del núcleo, sin tiempo», y su aceptación es una sola: con los mismos parámetros y el mismo `cx`, `cost_hint` no da menos trabajo para el doble de píxeles. Ninguna fila fija una cifra. Los 11 mutantes cambian la fórmula sin romper esa aceptación, y hoy ningún crate fuera de `gafira-raster` lee `cost_hint`.

Por eso se cuentan como no críticos. No está confirmado: 02 §4.4 lo declara «para planificación y presupuesto», y `gafira-engine` todavía no existe. El titular o quien especifique ese consumo debe confirmarlo (ítem 75).

### 2.3 Bloques

Cada bloque es un archivo, o una parte de `layout.rs`, y se puede ejecutar solo. Los tests indicados son el tipo de prueba, no requisitos nuevos: si un mutante resulta equivalente, se documenta por archivo y línea y no se escribe un test forzado.

### BT1. `gafira-text`, `columns.rs` (17)

- Regla: FD-45: con `balance`, el reparto minimiza la diferencia de alturas; `count`, `gap` y ancho de columna de §4.17.3.
- Test que lo mata: propiedad: sobre entradas pequeñas, el resultado coincide con una búsqueda exhaustiva de los cortes; más casos unitarios de empate para `Search::offer` (`d` igual a `best`, con y sin `TIE`).

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `columns.rs:190:9` | `balance` | `replace - with +` |
| `columns.rs:223:17` | `Search<'_>::go` | `replace - with /` |
| `columns.rs:223:23` | `Search<'_>::go` | `replace > with ==` |
| `columns.rs:223:23` | `Search<'_>::go` | `replace > with >=` |
| `columns.rs:229:46` | `Search<'_>::go` | `replace + with *` |
| `columns.rs:229:63` | `Search<'_>::go` | `replace + with *` |
| `columns.rs:230:32` | `Search<'_>::go` | `replace - with /` |
| `columns.rs:230:50` | `Search<'_>::go` | `replace - with /` |
| `columns.rs:233:58` | `Search<'_>::go` | `replace - with +` |
| `columns.rs:233:65` | `Search<'_>::go` | `replace < with <=` |
| `columns.rs:233:65` | `Search<'_>::go` | `replace < with ==` |
| `columns.rs:236:39` | `Search<'_>::go` | `replace - with +` |
| `columns.rs:236:39` | `Search<'_>::go` | `replace - with /` |
| `columns.rs:236:46` | `Search<'_>::go` | `replace - with +` |
| `columns.rs:236:46` | `Search<'_>::go` | `replace - with /` |
| `columns.rs:251:14` | `Search<'_>::offer` | `replace < with <=` |
| `columns.rs:251:64` | `Search<'_>::offer` | `replace > with >=` |

### BT2. `gafira-text`, `curve.rs` (6)

- Regla: FD-44 (avance 0 no hace nada) y ARQ-110 (salida no finita).
- Test que lo mata: unitario con una primera línea sin contenido (`curve.rs:89`); inyección de un no finito por campo, de uno en uno, en punto y arco (`:159`, `:171`).

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `curve.rs:89:30` | `curve_layout` | `replace > with <` |
| `curve.rs:89:30` | `curve_layout` | `replace > with ==` |
| `curve.rs:89:30` | `curve_layout` | `replace > with >=` |
| `curve.rs:159:45` | `curve_layout` | `replace && with \|\|` |
| `curve.rs:159:64` | `curve_layout` | `replace && with \|\|` |
| `curve.rs:171:37` | `curve_layout` | `replace && with \|\|` |

### BT3. `gafira-text`, `fit.rs` (2)

- Regla: FD-46: el límite «cabe» incluye la igualdad; rejilla de 0,25 pt; `min_size ≤ max_size`.
- Test que lo mata: unitario de umbral: en el límite, justo por debajo y justo por encima; `min_size == max_size`.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `fit.rs:166:22` | `fit_box` | `replace > with >=` |
| `fit.rs:200:21` | `grid` | `replace > with >=` |

### BT4a. `gafira-text`, `layout.rs`: interletrado y tamaños por rango (6)

- Regla: FD-41 (interletrado en `[−100, 500]`) y ARQ-116.
- Test que lo mata: unitario con interletrado 0, negativo y positivo, y con runs que no empiezan en el byte 0 ni comparten tamaño.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `layout.rs:277:30` | `layout_with` | `replace < with <=` |
| `layout.rs:303:21` | `layout_with` | `replace < with <=` |
| `layout.rs:304:51` | `layout_with` | `replace - with +` |
| `layout.rs:327:37` | `layout_with` | `replace * with +` |
| `layout.rs:327:37` | `layout_with` | `replace * with /` |
| `layout.rs:327:60` | `layout_with` | `replace != with ==` |

### BT4b. `gafira-text`, `layout.rs`: altura natural y avance (6)

- Regla: ARQ-116 (líneas con base y avance) y la fila «Fuente sin métricas» de §4.17.3.
- Test que lo mata: unitario con métricas conocidas, con `leading` y `descent` distintos de 0: valor exacto de la altura y del avance acumulado, ±0,01 px.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `layout.rs:443:36` | `read_lines` | `replace + with *` |
| `layout.rs:443:36` | `read_lines` | `replace + with -` |
| `layout.rs:443:48` | `read_lines` | `replace + with *` |
| `layout.rs:443:48` | `read_lines` | `replace + with -` |
| `layout.rs:472:25` | `read_lines` | `replace += with *=` |
| `layout.rs:472:25` | `read_lines` | `replace += with -=` |

### BT4c. `gafira-text`, `layout.rs`: rango de entrada (1)

- Regla: ARQ-116: cada clúster da su rango en bytes del texto de entrada.
- Test que lo mata: propiedad: con mayúsculas o texto transformado, el rango vuelve al texto original.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `layout.rs:556:54` | `SourceMap::input_range` | `replace + with *` |

### BT4d. `gafira-text`, `layout.rs`: lectura de tablas de la fuente (6)

- Regla: carga de fuentes: validación de bytes de entrada.
- Test que lo mata: unitario con fuentes mínimas construidas a mano: tabla en primera, intermedia y última posición, y directorio truncado.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `layout.rs:576:25` | `has_table` | `replace + with *` |
| `layout.rs:576:25` | `has_table` | `replace + with -` |
| `layout.rs:580:17` | `has_table` | `replace + with *` |
| `layout.rs:580:17` | `has_table` | `replace + with -` |
| `layout.rs:580:21` | `has_table` | `replace * with +` |
| `layout.rs:580:21` | `has_table` | `replace * with /` |

### BT4e. `gafira-text`, `layout.rs`: alineación (10)

- Regla: alineación del texto (04, FD-41) y ARQ-116.
- Test que lo mata: unitario por alineación con línea de ancho exacto, holgura 0 y línea que sobra; posición exacta ±0,01 px; justificado con y sin separadores.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `layout.rs:680:65` | `align_line` | `replace + with *` |
| `layout.rs:680:65` | `align_line` | `replace + with -` |
| `layout.rs:686:39` | `align_line` | `replace - with +` |
| `layout.rs:687:25` | `align_line` | `replace > with >=` |
| `layout.rs:690:36` | `align_line` | `replace - with +` |
| `layout.rs:692:30` | `align_line` | `replace - with +` |
| `layout.rs:708:58` | `align_line` | `replace > with >=` |
| `layout.rs:708:67` | `align_line` | `replace * with +` |
| `layout.rs:708:67` | `align_line` | `replace * with /` |
| `layout.rs:722:30` | `align_line` | `replace + with -` |

### BT4f. `gafira-text`, `layout.rs`: comprobación de salida (6)

- Regla: ARQ-110: un resultado intermedio no finito da `Err` que nombra el campo.
- Test que lo mata: unitario sobre `check_output` con un solo campo no finito cada vez (avance de línea, de clúster, subrayado, tachado, desplazamiento y grosor). Exige poder llegar a esa función desde un test.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `layout.rs:760:72` | `check_output` | `replace && with \|\|` |
| `layout.rs:764:17` | `check_output` | `replace && with \|\|` |
| `layout.rs:765:17` | `check_output` | `replace && with \|\|` |
| `layout.rs:767:25` | `check_output` | `replace && with \|\|` |
| `layout.rs:768:25` | `check_output` | `replace && with \|\|` |
| `layout.rs:769:25` | `check_output` | `replace && with \|\|` |

### BT5. `gafira-text`, `list.rs` (2)

- Regla: ARQ-110: la suma `top + height` no finita da `Err`.
- Test que lo mata: unitario con `top` y `height` finitos cuya suma desborda.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `list.rs:119:60` | `layout_list` | `replace + with *` |
| `list.rs:119:60` | `layout_list` | `replace + with -` |

### BT6. `gafira-text`, `outline.rs` (3)

- Regla: ARQ-110: contorno con puntos no finitos.
- Test que lo mata: unitario: un NaN en cada punto de `QuadTo` y `CurveTo`, de uno en uno.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `outline.rs:118:47` | `finite` | `replace && with \|\|` |
| `outline.rs:119:51` | `finite` | `replace && with \|\|` |
| `outline.rs:119:68` | `finite` | `replace && with \|\|` |

### BR1. `gafira-raster`, `blend.rs` (1)

- Regla: ARQ-119 (W3C *Compositing and Blending Level 1*).
- Test que lo mata: unitario con `x` en 1.0 exacto y justo por encima.
- Nota: posible mutante equivalente: con `x = 1` el reescalado apenas cambia el valor. Si se confirma, se documenta en vez de escribir un test.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `blend.rs:78:10` | `clip_color` | `replace > with >=` |

### BR2. `gafira-raster`, `dhash.rs` (2)

- Regla: ARQ-122 y la fila «Imagen de tamaño 0» de §4.17.4.
- Test que lo mata: unitario con solo un lado a 0 (`w = 0, h > 0` y al revés).

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `dhash.rs:70:23` | `dhash` | `replace \|\| with &&` |
| `dhash.rs:89:23` | `dhash` | `replace \|\| with &&` |

### BR3. `gafira-raster`, `palette.rs` (10)

- Regla: ARQ-122 y FD-26: hasta 5 colores por área; fusión por ΔE2000.
- Test que lo mata: unitario de `bin_of` con colores que difieren solo en un canal (cubos distintos); umbral de fusión en `MERGE_DELTA_E`, justo debajo y justo encima; `MAX_CLUSTERS` exacto; empate en `closest_pair`.
- Nota: `palette.rs:158:29` llegó como timeout y el re-run lo dio por sobreviviente en 69 s; es el único de la lista que no viene de `missed.txt` de la corrida completa.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `palette.rs:28:38` | `bin_of` | `replace \| with ^` |
| `palette.rs:28:60` | `bin_of` | `replace >> with <<` |
| `palette.rs:28:66` | `bin_of` | `replace << with >>` |
| `palette.rs:28:72` | `bin_of` | `replace \| with ^` |
| `palette.rs:28:93` | `bin_of` | `replace >> with <<` |
| `palette.rs:158:29` | `dominant_colors` | `replace match guard d < MERGE_DELTA_E \|\| clusters.len() >= MAX_CLUSTERS with false` |
| `palette.rs:158:31` | `dominant_colors` | `replace < with <=` |
| `palette.rs:158:31` | `dominant_colors` | `replace < with ==` |
| `palette.rs:158:47` | `dominant_colors` | `replace \|\| with &&` |
| `palette.rs:232:46` | `closest_pair` | `replace < with <=` |

### BR4. `gafira-raster`, `pixel.rs` (4)

- Regla: ARQ-110: el `Err` nombra el elemento.
- Test que lo mata: unitario: NaN en un píxel y canal conocidos; comprueba `index` exacto en `BufferError::NonFinite`.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `pixel.rs:69:34` | `convert` | `replace * with +` |
| `pixel.rs:69:34` | `convert` | `replace * with /` |
| `pixel.rs:69:38` | `convert` | `replace + with *` |
| `pixel.rs:69:38` | `convert` | `replace + with -` |

### BR5. `gafira-raster`, `effects/auto_adjust.rs` (3)

- Regla: Ajuste automático (04, tabla de ajustes).
- Test que lo mata: unitario con histograma conocido: percentil exacto en extremos y media de canal exacta.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `effects/auto_adjust.rs:59:52` | `percentile` | `replace - with +` |
| `effects/auto_adjust.rs:59:52` | `percentile` | `replace - with /` |
| `effects/auto_adjust.rs:115:76` | `auto_adjust` | `replace / with *` |

### BR6. `gafira-raster`, `effects/blur.rs` (3)

- Regla: ARQ-120: `σ = radio / 2`, radio 0 es la identidad; fila «Imagen de tamaño 0».
- Test que lo mata: unitario con radio NaN y radio cuyo cuadrado es 0; `sigma` en `SMALL_SIGMA` exacto; un solo lado a 0.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `effects/blur.rs:11:27` | `half_width` | `replace \|\| with &&` |
| `effects/blur.rs:28:14` | `kernel` | `replace < with <=` |
| `effects/blur.rs:62:15` | `blur_shifted` | `replace \|\| with &&` |

### BR7. `gafira-raster`, `effects/shadow.rs` (1)

- Regla: fila «Imagen de tamaño 0» de §4.17.4.
- Test que lo mata: unitario con un solo lado a 0: `Ok` sin cambios.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `effects/shadow.rs:208:19` | `<impl Effect for ParallelShadow>::run_cpu` | `replace \|\| with &&` |

### BR8. `gafira-raster`, `effects/texture.rs` (3)

- Regla: fila «División por alfa 0» y fila «Imagen de tamaño 0» de §4.17.4.
- Test que lo mata: unitario con plano de alfa 0 (resultado sin NaN) y con un solo lado a 0.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `effects/texture.rs:212:12` | `Planes::straight` | `replace > with >=` |
| `effects/texture.rs:218:12` | `Planes::luma` | `replace > with >=` |
| `effects/texture.rs:270:19` | `<impl Effect for Texture>::run_cpu` | `replace \|\| with &&` |

### BR9. `gafira-raster`, `transitions/fade.rs` y `transitions/guard.rs` (2)

- Regla: ARQ-118 y fila «Imagen de tamaño 0».
- Test que lo mata: unitario con `progress` 0,5 exacto en Fundido y con un solo lado a 0 en la guarda.
- Nota: van juntos porque ninguno de los dos archivos tiene módulo de tests propio.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `transitions/fade.rs:90:31` | `<impl Transition for Fade>::run_cpu` | `replace < with <=` |
| `transitions/guard.rs:87:20` | `plan` | `replace \|\| with &&` |

### BR10. `gafira-raster`, `transitions/wipe.rs` (4)

- Regla: ARQ-118.
- Test que lo mata: unitario con `gap` igual a `softness`, `softness = 0` y `gap = 0`; valor exacto del peso de mezcla, 1/255.

| Archivo:línea:col | Función | Mutación |
|---|---|---|
| `transitions/wipe.rs:144:38` | `<impl Transition for Wipe>::run_cpu` | `replace > with ==` |
| `transitions/wipe.rs:144:38` | `<impl Transition for Wipe>::run_cpu` | `replace > with >=` |
| `transitions/wipe.rs:145:36` | `<impl Transition for Wipe>::run_cpu` | `replace > with >=` |
| `transitions/wipe.rs:158:52` | `<impl Transition for Wipe>::run_cpu` | `replace - with +` |

## 3. Resto del T4

### 3.1 Colgantes confirmados: no son deuda

Ocho mutantes no terminan con el límite largo del re-run (600 s). Según ADR 0004 y `AGENTS.md`, cuentan como detectados.

`gafira-text` (6):

- `gafira-text/src/columns.rs:166:21: replace += with *= in fill`
- `gafira-text/src/fit.rs:171:27: replace > with == in fit_box`
- `gafira-text/src/fit.rs:171:27: replace > with >= in fit_box`
- `gafira-text/src/fit.rs:171:22: replace - with + in fit_box`
- `gafira-text/src/fit.rs:172:37: replace / with % in fit_box`
- `gafira-text/src/fit.rs:172:31: replace + with * in fit_box`

`gafira-raster` (2):

- `gafira-raster/src/mip.rs:60:28: replace == with != in Sums::halve`
- `gafira-raster/src/mip.rs:82:46: replace * with / in Sums::halve`

Los demás timeouts de la corrida completa fueron de carga y el re-run los atrapó: `fit.rs:172:37` (`/` por `*`) en text, `mip.rs:59:19` y `mip.rs:63:29` en raster. `palette.rs:158:29` pasó a sobreviviente (bloque BR3).

### 3.2 `gafira-audio` (ítem 49)

889 mutantes: 835 atrapados, 29 perdidos, 13 no viables y 12 timeouts, de los que quedan 10 tras un re-run con límite de 700 s. El informe da 25 críticos. La lista cruda no los marca y el reparto no se encuentra en `specs/history/2026-11-02-gafira-audio/`, así que el 25 no se puede reproducir línea a línea. Los candidatos a los 4 no críticos son `block.rs:96`, `chain.rs:52`, `processor.rs:27` y `processor.rs:31`, sin confirmar. Los 10 timeouts no están clasificados como colgantes.

La lista cruda sobrevivió: anexo A. El código de `gafira-audio` no ha cambiado desde la corrida.

### 3.3 `gafira-segment`, `gafira-testkit` y `layer-2-scaffold` (ítems 50 y 73)

- **`gafira-segment`**: T4 parcial, parado por orden del titular, antes de D-47. 1271 de 1421 mutantes: 983 atrapados, 236 perdidos, 34 timeouts sin clasificar y 18 no viables. Sin clasificar en críticos y no críticos. La lista cruda sobrevivió: anexo B. D-47 cambió `cut.rs` y `lib.rs`, así que las líneas de esos dos archivos no valen en esta rama. Un T4 completo tras D-47 sustituye a esta lista.
- **`gafira-testkit`**: 776 mutantes encontrados, ninguno probado: la corrida se interrumpió y sus listas están vacías. Solo queda esa cifra. Los diez sobrevivientes del ítem 44 vienen de una corrida anterior, la de `engine-contracts`.
- **`layer-2-scaffold`**: T4 por correr. No hay listas.

## 4. Orden de trabajo

Nada de esto arranca sin la decisión del titular de los ítems 74 y 75. Sigue valiendo un escritor a la vez: lo paralelo de abajo solo significa que los bloques no chocan en archivos, y requiere worktrees separados.

| Tanda | Bloques | Por qué |
|---|---|---|
| A, en paralelo | BT1, BT2, BT3, BT5, BT6 | Cada uno tiene su archivo en `crates/gafira-text/tests/`. |
| A, en paralelo | BR1, BR2, BR3, BR4 | Cada uno usa el módulo de tests de su propio archivo. |
| A, en paralelo | BR9, BR10 | `wipe.rs` tiene sus tests; BR9 puede ir en `transitions/mod.rs`, que BR10 no toca. |
| B, en serie entre sí | BR5, BR6, BR7, BR8 | Comparten `effects/testing.rs` si añaden casos comunes. En paralelo con todo lo que no sea `effects/`. |
| C, en serie entre sí | BT4a a BT4f | Son el mismo archivo fuente y comparten `tests/layout.rs` y `tests/align.rs`. Después de la tanda A de text: si un test destapa un defecto, el arreglo en `layout.rs` afecta a columnas, listas y encaje. |
| D, en serie | T4 de `gafira-text`, T4 de `gafira-raster` | Un crate cada vez: en paralelo vuelven los timeouts de carga (ítem 31). |

Text y raster no dependen entre sí, así que sus bloques nunca chocan. Audio y segment tampoco chocan con ellos, pero esperan a la decisión del titular (ítems 49, 50 y 73). Los ítems de §1.1 que solo tocan documentos no chocan con ningún bloque, aunque cerrar los ítems 74 y 75 edita las mismas líneas del README.

## Anexo A. Lista cruda de `gafira-audio`

Perdidos (29):

```text
gafira-audio/src/block.rs:96:9: replace ParamSnapshot::is_empty -> bool with true
gafira-audio/src/chain.rs:52:9: replace <impl AudioProcessor for ProcessorChain>::params -> &ParamSchema with Box::leak(Box::new(Default::default()))
gafira-audio/src/dsp.rs:122:13: replace += with -= in bessel_i0
gafira-audio/src/dsp.rs:123:17: replace < with == in bessel_i0
gafira-audio/src/dsp.rs:123:17: replace < with <= in bessel_i0
gafira-audio/src/eq.rs:123:28: replace < with <= in History::step
gafira-audio/src/loudness.rs:64:38: replace + with * in up_kernel
gafira-audio/src/loudness.rs:99:72: replace + with * in upsampled
gafira-audio/src/loudness.rs:125:35: replace * with / in k_weighting
gafira-audio/src/loudness.rs:126:26: replace + with - in k_weighting
gafira-audio/src/loudness.rs:125:35: replace * with + in k_weighting
gafira-audio/src/loudness.rs:129:17: replace * with / in k_weighting
gafira-audio/src/loudness.rs:129:52: replace + with - in k_weighting
gafira-audio/src/loudness.rs:189:33: replace * with + in integrated_loudness
gafira-audio/src/loudness.rs:189:33: replace * with / in integrated_loudness
gafira-audio/src/loudness.rs:282:90: replace / with % in interpolation_taps
gafira-audio/src/loudness.rs:282:90: replace / with * in interpolation_taps
gafira-audio/src/loudness.rs:288:14: replace /= with *= in interpolation_taps
gafira-audio/src/loudness.rs:288:14: replace /= with %= in interpolation_taps
gafira-audio/src/loudness.rs:321:37: replace + with * in true_peak
gafira-audio/src/loudness.rs:323:27: replace += with -= in true_peak
gafira-audio/src/offset.rs:16:49: replace < with == in flicks_to_samples
gafira-audio/src/offset.rs:16:49: replace < with > in flicks_to_samples
gafira-audio/src/offset.rs:16:49: replace < with <= in flicks_to_samples
gafira-audio/src/processor.rs:27:9: replace AudioProcessor::latency -> u32 with 1
gafira-audio/src/processor.rs:31:9: replace AudioProcessor::tail -> u32 with 1
gafira-audio/src/resample.rs:62:10: replace < with <= in resampled_len
gafira-audio/src/resample.rs:75:58: replace + with * in kernel
gafira-audio/src/resample.rs:171:9: replace Resampler::trim with ()
```

Timeouts tras el re-run (10):

```text
gafira-audio/src/gain.rs:131:31: replace > with == in <impl AudioProcessor for Gain>::process
gafira-audio/src/loudness.rs:211:20: replace * with + in integrated_loudness
gafira-audio/src/loudness.rs:211:20: replace * with / in integrated_loudness
gafira-audio/src/gain.rs:131:31: replace > with >= in <impl AudioProcessor for Gain>::process
gafira-audio/src/resample.rs:149:9: replace Resampler::position -> f64 with 0.0
gafira-audio/src/resample.rs:149:9: replace Resampler::position -> f64 with 1.0
gafira-audio/src/resample.rs:149:9: replace Resampler::position -> f64 with -1.0
gafira-audio/src/resample.rs:149:36: replace / with % in Resampler::position
gafira-audio/src/resample.rs:149:18: replace * with / in Resampler::position
gafira-audio/src/resample.rs:193:27: replace += with *= in Resampler::process
```

## Anexo B. Lista cruda de `gafira-segment`, previa a D-47

Perdidos (236):

```text
gafira-segment/src/lib.rs:143:56: replace + with * in <impl Segmenter for ClassicSegmenter>::segment
gafira-segment/src/lib.rs:143:61: replace * with + in <impl Segmenter for ClassicSegmenter>::segment
gafira-segment/src/lib.rs:143:61: replace * with / in <impl Segmenter for ClassicSegmenter>::segment
gafira-segment/src/lib.rs:156:17: replace + with * in <impl Segmenter for ClassicSegmenter>::segment
gafira-segment/src/lib.rs:156:17: replace + with - in <impl Segmenter for ClassicSegmenter>::segment
gafira-segment/src/lib.rs:155:17: replace + with - in <impl Segmenter for ClassicSegmenter>::segment
gafira-segment/src/cut.rs:35:26: replace > with >= in Node::held
gafira-segment/src/cut.rs:50:33: replace * with + in planned
gafira-segment/src/cut.rs:50:33: replace * with / in planned
gafira-segment/src/cut.rs:55:5: replace round_units -> u64 with 0
gafira-segment/src/cut.rs:55:21: replace / with % in round_units
gafira-segment/src/cut.rs:55:5: replace round_units -> u64 with 1
gafira-segment/src/cut.rs:55:9: replace * with + in round_units
gafira-segment/src/cut.rs:55:9: replace * with / in round_units
gafira-segment/src/cut.rs:61:11: replace / with % in decode
gafira-segment/src/cut.rs:60:11: replace <= with > in decode
gafira-segment/src/cut.rs:61:11: replace / with * in decode
gafira-segment/src/cut.rs:63:22: replace / with % in decode
gafira-segment/src/cut.rs:63:22: replace / with * in decode
gafira-segment/src/cut.rs:63:13: replace + with - in decode
gafira-segment/src/cut.rs:88:22: replace += with *= in nodes
gafira-segment/src/cut.rs:90:22: replace += with *= in nodes
gafira-segment/src/cut.rs:89:22: replace += with *= in nodes
gafira-segment/src/cut.rs:91:22: replace += with *= in nodes
gafira-segment/src/cut.rs:92:22: replace < with == in nodes
gafira-segment/src/cut.rs:92:22: replace < with <= in nodes
gafira-segment/src/cut.rs:92:46: replace + with - in nodes
gafira-segment/src/cut.rs:92:63: replace + with - in nodes
gafira-segment/src/cut.rs:108:62: replace / with % in nodes
gafira-segment/src/cut.rs:134:73: replace * with + in strays
gafira-segment/src/cut.rs:137:53: replace && with || in strays
gafira-segment/src/cut.rs:164:49: replace && with || in strays
gafira-segment/src/cut.rs:164:43: replace < with <= in strays
gafira-segment/src/cut.rs:164:62: replace < with <= in strays
gafira-segment/src/cut.rs:164:54: replace * with + in strays
gafira-segment/src/cut.rs:164:54: replace * with / in strays
gafira-segment/src/cut.rs:199:8: replace * with / in fixed_point
gafira-segment/src/cut.rs:248:24: replace > with >= in grabcut
gafira-segment/src/cut.rs:252:15: replace += with *= in grabcut
gafira-segment/src/cut.rs:256:28: replace + with - in grabcut
gafira-segment/src/cut.rs:257:36: replace + with * in grabcut
gafira-segment/src/cut.rs:281:14: replace < with <= in grabcut
gafira-segment/src/cut.rs:282:42: replace - with + in grabcut
gafira-segment/src/cut.rs:307:26: replace && with || in central
gafira-segment/src/cut.rs:312:28: replace / with * in central
gafira-segment/src/cut.rs:317:58: replace + with * in central
gafira-segment/src/cut.rs:317:80: replace - with / in central
gafira-segment/src/error.rs:64:5: replace format_name -> &'static str with ""
gafira-segment/src/error.rs:35:9: replace <impl fmt::Display for SegError>::fmt -> fmt::Result with Ok(Default::default())
gafira-segment/src/error.rs:64:5: replace format_name -> &'static str with "xyzzy"
gafira-segment/src/flow.rs:49:25: replace < with <= in Flow::source_side
gafira-segment/src/flow.rs:53:59: replace > with == in Flow::source_side
gafira-segment/src/flow.rs:53:59: replace > with < in Flow::source_side
gafira-segment/src/flow.rs:84:42: replace += with -= in Flow::push
gafira-segment/src/flow.rs:84:33: replace ^ with & in Flow::push
gafira-segment/src/gmm.rs:26:5: replace dist2 -> f64 with 0.0
gafira-segment/src/gmm.rs:26:5: replace dist2 -> f64 with 1.0
gafira-segment/src/gmm.rs:26:5: replace dist2 -> f64 with -1.0
gafira-segment/src/gmm.rs:26:51: replace + with - in dist2
gafira-segment/src/gmm.rs:26:27: replace + with - in dist2
gafira-segment/src/gmm.rs:26:51: replace + with * in dist2
gafira-segment/src/gmm.rs:26:27: replace + with * in dist2
gafira-segment/src/gmm.rs:26:11: replace - with + in dist2
gafira-segment/src/gmm.rs:26:11: replace - with / in dist2
gafira-segment/src/gmm.rs:26:35: replace - with + in dist2
gafira-segment/src/gmm.rs:26:35: replace - with / in dist2
gafira-segment/src/gmm.rs:26:59: replace - with / in dist2
gafira-segment/src/gmm.rs:26:59: replace - with + in dist2
gafira-segment/src/gmm.rs:46:48: replace * with / in Gmm::fit
gafira-segment/src/gmm.rs:46:43: replace + with * in Gmm::fit
gafira-segment/src/gmm.rs:54:26: replace < with == in Gmm::fit
gafira-segment/src/gmm.rs:54:26: replace < with > in Gmm::fit
gafira-segment/src/gmm.rs:54:26: replace < with <= in Gmm::fit
gafira-segment/src/gmm.rs:60:21: replace == with != in Gmm::fit
gafira-segment/src/gmm.rs:67:28: replace += with -= in Gmm::fit
gafira-segment/src/gmm.rs:67:28: replace += with *= in Gmm::fit
gafira-segment/src/gmm.rs:67:42: replace * with + in Gmm::fit
gafira-segment/src/gmm.rs:69:20: replace += with -= in Gmm::fit
gafira-segment/src/gmm.rs:69:20: replace += with *= in Gmm::fit
gafira-segment/src/gmm.rs:72:23: replace > with == in Gmm::fit
gafira-segment/src/gmm.rs:72:23: replace > with < in Gmm::fit
gafira-segment/src/gmm.rs:72:23: replace > with >= in Gmm::fit
gafira-segment/src/gmm.rs:83:31: replace - with + in Gmm::fit
gafira-segment/src/gmm.rs:83:31: replace - with / in Gmm::fit
gafira-segment/src/gmm.rs:83:57: replace - with + in Gmm::fit
gafira-segment/src/gmm.rs:89:48: replace * with + in Gmm::fit
gafira-segment/src/gmm.rs:89:27: replace += with *= in Gmm::fit
gafira-segment/src/gmm.rs:89:41: replace * with + in Gmm::fit
gafira-segment/src/gmm.rs:101:46: replace * with / in Gmm::fit
gafira-segment/src/gmm.rs:115:31: replace - with / in Gmm::cost
gafira-segment/src/gmm.rs:119:21: replace + with - in Gmm::cost
gafira-segment/src/gmm.rs:119:21: replace + with * in Gmm::cost
gafira-segment/src/gmm.rs:118:21: replace + with * in Gmm::cost
gafira-segment/src/gmm.rs:117:32: replace * with + in Gmm::cost
gafira-segment/src/gmm.rs:118:32: replace * with + in Gmm::cost
gafira-segment/src/gmm.rs:120:25: replace * with + in Gmm::cost
gafira-segment/src/gmm.rs:122:29: replace + with - in Gmm::cost
gafira-segment/src/gmm.rs:122:29: replace + with * in Gmm::cost
gafira-segment/src/gmm.rs:121:29: replace + with * in Gmm::cost
gafira-segment/src/gmm.rs:121:29: replace + with - in Gmm::cost
gafira-segment/src/gmm.rs:134:23: replace - with + in invert
gafira-segment/src/gmm.rs:134:18: replace * with + in invert
gafira-segment/src/gmm.rs:134:28: replace * with + in invert
gafira-segment/src/gmm.rs:135:23: replace - with + in invert
gafira-segment/src/gmm.rs:135:18: replace * with + in invert
gafira-segment/src/gmm.rs:135:28: replace * with + in invert
gafira-segment/src/gmm.rs:135:28: replace * with / in invert
gafira-segment/src/gmm.rs:136:23: replace - with + in invert
gafira-segment/src/gmm.rs:136:18: replace * with + in invert
gafira-segment/src/gmm.rs:136:28: replace * with + in invert
gafira-segment/src/gmm.rs:136:28: replace * with / in invert
gafira-segment/src/gmm.rs:137:35: replace + with - in invert
gafira-segment/src/gmm.rs:137:35: replace + with * in invert
gafira-segment/src/gmm.rs:137:24: replace + with - in invert
gafira-segment/src/gmm.rs:137:29: replace * with + in invert
gafira-segment/src/gmm.rs:137:40: replace * with + in invert
gafira-segment/src/gmm.rs:138:23: replace - with + in invert
gafira-segment/src/gmm.rs:138:18: replace * with + in invert
gafira-segment/src/gmm.rs:138:18: replace * with / in invert
gafira-segment/src/gmm.rs:139:23: replace - with / in invert
gafira-segment/src/gmm.rs:138:28: replace * with + in invert
gafira-segment/src/gmm.rs:139:23: replace - with + in invert
gafira-segment/src/gmm.rs:139:18: replace * with + in invert
gafira-segment/src/gmm.rs:139:28: replace * with + in invert
gafira-segment/src/gmm.rs:140:23: replace - with + in invert
gafira-segment/src/gmm.rs:140:18: replace * with + in invert
gafira-segment/src/gmm.rs:140:18: replace * with / in invert
gafira-segment/src/gmm.rs:140:28: replace * with + in invert
gafira-segment/src/hints.rs:46:9: replace Band::area -> u64 with 0
gafira-segment/src/hints.rs:46:9: replace Band::area -> u64 with 1
gafira-segment/src/hints.rs:46:36: replace * with / in Band::area
gafira-segment/src/hints.rs:46:36: replace * with + in Band::area
gafira-segment/src/hints.rs:46:31: replace + with - in Band::area
gafira-segment/src/hints.rs:46:31: replace + with * in Band::area
gafira-segment/src/hints.rs:46:20: replace - with + in Band::area
gafira-segment/src/hints.rs:46:59: replace + with * in Band::area
gafira-segment/src/hints.rs:46:48: replace - with + in Band::area
gafira-segment/src/hints.rs:63:29: replace || with && in clip
gafira-segment/src/hints.rs:63:24: replace < with == in clip
gafira-segment/src/hints.rs:63:24: replace < with <= in clip
gafira-segment/src/hints.rs:63:40: replace > with == in clip
gafira-segment/src/hints.rs:63:40: replace > with >= in clip
gafira-segment/src/hints.rs:67:42: replace > with >= in clip
gafira-segment/src/hints.rs:68:71: replace / with * in clip
gafira-segment/src/hints.rs:69:18: replace > with >= in clip
gafira-segment/src/hints.rs:87:27: replace + with - in clip
gafira-segment/src/hints.rs:87:31: replace * with + in clip
gafira-segment/src/hints.rs:87:44: replace + with - in clip
gafira-segment/src/hints.rs:96:72: replace - with / in range
gafira-segment/src/hints.rs:96:72: replace - with + in range
gafira-segment/src/hints.rs:106:44: replace * with / in SeedStroke::bands
gafira-segment/src/hints.rs:127:44: replace - with + in SeedStroke::bands
gafira-segment/src/hints.rs:127:44: replace - with / in SeedStroke::bands
gafira-segment/src/hints.rs:134:9: replace SeedStroke::cost -> u64 with 0
gafira-segment/src/hints.rs:134:9: replace SeedStroke::cost -> u64 with 1
gafira-segment/src/hints.rs:169:32: replace + with - in SeedStroke::for_covered
gafira-segment/src/hints.rs:169:37: replace * with + in SeedStroke::for_covered
gafira-segment/src/hints.rs:199:9: replace SegHints::stroke_cost -> u64 with 0
gafira-segment/src/hints.rs:199:9: replace SegHints::stroke_cost -> u64 with 1
gafira-segment/src/meter.rs:54:9: replace Meter<'_>::check -> Result<(), Cancelled> with Ok(())
gafira-segment/src/refine.rs:32:68: replace - with + in taps
gafira-segment/src/refine.rs:32:32: replace + with - in taps
gafira-segment/src/refine.rs:33:37: replace - with + in taps
gafira-segment/src/refine.rs:33:37: replace - with / in taps
gafira-segment/src/refine.rs:37:25: replace + with * in taps
gafira-segment/src/refine.rs:60:64: replace - with + in box_mean
gafira-segment/src/refine.rs:60:64: replace - with / in box_mean
gafira-segment/src/refine.rs:66:50: replace * with / in box_mean
gafira-segment/src/refine.rs:83:63: replace - with + in box_mean
gafira-segment/src/refine.rs:83:63: replace - with / in box_mean
gafira-segment/src/refine.rs:111:53: replace * with + in upscale
gafira-segment/src/refine.rs:111:53: replace * with / in upscale
gafira-segment/src/refine.rs:111:72: replace * with + in upscale
gafira-segment/src/refine.rs:111:72: replace * with / in upscale
gafira-segment/src/refine.rs:122:32: replace + with - in upscale
gafira-segment/src/refine.rs:122:42: replace * with + in upscale
gafira-segment/src/refine.rs:122:37: replace - with + in upscale
gafira-segment/src/refine.rs:125:42: replace + with - in upscale
gafira-segment/src/refine.rs:125:47: replace - with + in upscale
gafira-segment/src/refine.rs:142:86: replace + with - in upscale
gafira-segment/src/refine.rs:151:53: replace + with - in upscale
gafira-segment/src/reliable.rs:44:15: replace / with % in srgb_table
gafira-segment/src/reliable.rs:44:15: replace / with * in srgb_table
gafira-segment/src/reliable.rs:67:41: replace / with * in accumulate
gafira-segment/src/reliable.rs:69:58: replace + with * in accumulate
gafira-segment/src/slic.rs:53:31: replace / with * in slic
gafira-segment/src/slic.rs:53:58: replace / with * in slic
gafira-segment/src/slic.rs:54:30: replace * with / in slic
gafira-segment/src/slic.rs:57:47: replace * with + in slic
gafira-segment/src/slic.rs:57:58: replace / with * in slic
gafira-segment/src/slic.rs:57:47: replace * with / in slic
gafira-segment/src/slic.rs:57:40: replace + with - in slic
gafira-segment/src/slic.rs:57:40: replace + with * in slic
gafira-segment/src/slic.rs:57:25: replace % with / in slic
gafira-segment/src/slic.rs:57:25: replace % with + in slic
gafira-segment/src/slic.rs:58:58: replace / with * in slic
gafira-segment/src/slic.rs:58:25: replace / with % in slic
gafira-segment/src/slic.rs:58:25: replace / with * in slic
gafira-segment/src/slic.rs:59:53: replace * with / in slic
gafira-segment/src/slic.rs:59:48: replace - with / in slic
gafira-segment/src/slic.rs:59:48: replace - with + in slic
gafira-segment/src/slic.rs:59:78: replace - with + in slic
gafira-segment/src/slic.rs:59:78: replace - with / in slic
gafira-segment/src/slic.rs:101:42: replace + with * in nearest
gafira-segment/src/slic.rs:102:46: replace + with * in nearest
gafira-segment/src/slic.rs:105:72: replace + with * in nearest
gafira-segment/src/slic.rs:105:46: replace + with - in nearest
gafira-segment/src/slic.rs:105:46: replace + with * in nearest
gafira-segment/src/slic.rs:105:28: replace - with / in nearest
gafira-segment/src/slic.rs:105:54: replace - with / in nearest
gafira-segment/src/slic.rs:105:80: replace - with / in nearest
gafira-segment/src/slic.rs:106:47: replace + with * in nearest
gafira-segment/src/slic.rs:106:32: replace - with / in nearest
gafira-segment/src/slic.rs:107:24: replace + with * in nearest
gafira-segment/src/slic.rs:107:33: replace * with + in nearest
gafira-segment/src/slic.rs:107:33: replace * with / in nearest
gafira-segment/src/slic.rs:108:18: replace < with <= in nearest
gafira-segment/src/slic.rs:131:27: replace * with / in recentre
gafira-segment/src/slic.rs:134:22: replace += with *= in recentre
gafira-segment/src/slic.rs:136:22: replace += with *= in recentre
gafira-segment/src/slic.rs:135:22: replace += with *= in recentre
gafira-segment/src/slic.rs:137:22: replace += with *= in recentre
gafira-segment/src/slic.rs:139:22: replace += with *= in recentre
gafira-segment/src/slic.rs:138:22: replace += with *= in recentre
gafira-segment/src/slic.rs:147:21: replace == with != in recentre
gafira-segment/src/slic.rs:152:36: replace + with - in recentre
gafira-segment/src/slic.rs:153:36: replace + with - in recentre
gafira-segment/src/slic.rs:173:42: replace - with + in connect
gafira-segment/src/slic.rs:180:33: replace > with < in connect
gafira-segment/src/slic.rs:182:25: replace > with < in connect
gafira-segment/src/slic.rs:183:32: replace - with / in connect
gafira-segment/src/slic.rs:190:35: replace - with / in connect
gafira-segment/src/slic.rs:199:25: replace > with < in connect
gafira-segment/src/slic.rs:199:40: replace - with / in connect
gafira-segment/src/slic.rs:210:42: replace < with <= in connect
gafira-segment/src/slic.rs:236:45: replace * with + in adjacency
```

Timeouts sin clasificar (34):

```text
gafira-segment/src/cut.rs:123:25: replace != with == in strays::find
gafira-segment/src/cut.rs:256:28: replace + with * in grabcut
gafira-segment/src/cut.rs:298:25: replace != with == in central::find
gafira-segment/src/flow.rs:10:31: replace << with >>
gafira-segment/src/flow.rs:49:25: replace < with == in Flow::source_side
gafira-segment/src/flow.rs:53:59: replace > with >= in Flow::source_side
gafira-segment/src/flow.rs:64:28: replace > with >= in Flow::levels
gafira-segment/src/flow.rs:64:49: replace < with <= in Flow::levels
gafira-segment/src/flow.rs:65:46: replace + with * in Flow::levels
gafira-segment/src/flow.rs:74:9: replace Flow::push -> i64 with 0
gafira-segment/src/flow.rs:74:9: replace Flow::push -> i64 with 1
gafira-segment/src/flow.rs:74:9: replace Flow::push -> i64 with -1
gafira-segment/src/flow.rs:74:14: replace == with != in Flow::push
gafira-segment/src/flow.rs:77:23: replace < with == in Flow::push
gafira-segment/src/flow.rs:77:23: replace < with > in Flow::push
gafira-segment/src/flow.rs:80:20: replace > with == in Flow::push
gafira-segment/src/flow.rs:80:20: replace > with < in Flow::push
gafira-segment/src/flow.rs:80:20: replace > with >= in Flow::push
gafira-segment/src/flow.rs:80:37: replace == with != in Flow::push
gafira-segment/src/flow.rs:80:49: replace + with - in Flow::push
gafira-segment/src/flow.rs:80:49: replace + with * in Flow::push
gafira-segment/src/flow.rs:82:27: replace > with == in Flow::push
gafira-segment/src/flow.rs:82:27: replace > with < in Flow::push
gafira-segment/src/flow.rs:82:27: replace > with >= in Flow::push
gafira-segment/src/flow.rs:83:38: replace -= with /= in Flow::push
gafira-segment/src/flow.rs:84:33: replace ^ with | in Flow::push
gafira-segment/src/flow.rs:88:21: replace += with *= in Flow::push
gafira-segment/src/refine.rs:110:41: replace / with * in upscale
gafira-segment/src/refine.rs:111:68: replace + with * in upscale
gafira-segment/src/slic.rs:33:20: replace * with + in grid
gafira-segment/src/slic.rs:33:20: replace * with / in grid
gafira-segment/src/slic.rs:86:42: replace / with % in slic
gafira-segment/src/slic.rs:86:33: replace * with + in slic
gafira-segment/src/slic.rs:86:33: replace * with / in slic
```
