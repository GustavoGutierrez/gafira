# Especificaciones de producto

Aquí solo viven los documentos **en curso**. Todo lo demás espera en `backlog/`. Al completar el documento en curso, se promueve el siguiente desde el backlog.

Ciclo de vida: `backlog/` -> en curso (`docs/specs/`) -> `archive/` (completado).

El skill `product-specs` (`.agents/skills/product-specs/`) describe el ciclo de vida, el formato de los documentos y cómo derivar changes de OpenSpec.

## Estado

| Doc | Estado | Detalle |
|---|---|---|
| `02-arquitectura-software.md` | En curso | Hecho: C1 `workspace-scaffold` (ARQ-01 parcial, ARQ-02, ARQ-03, ARQ-07, ARQ-70, ARQ-71, ARQ-80 parcial: la puerta de merge está diferida por el 403 de GitHub), C2a `domain-color` (ARQ-02, ARQ-03, ARQ-10), C2b `domain-doc` (ARQ-03, ARQ-10, ARQ-11, ARQ-13, ARQ-30) (archivados), C3 `edit-history` (`gafira-edit`, entregas D1-D11, archivado en `openspec/changes/archive/2026-10-25-edit-history/`), C2c `domain-time` (`gafira-time`, capa 0, entregas D0-D4, archivado en `openspec/changes/archive/2026-10-25-domain-time/`). C4 `engine-pathops` (`gafira-pathops`, capa 2, entregas D1-D4, archivado en `openspec/changes/archive/2026-10-26-engine-pathops/`). C5 `anim-core` (`gafira-anim`, capa 1, entregas D1, D2a, D2b, D3a, D3b, D3c, archivado en `openspec/changes/archive/2026-10-30-anim-core/`), cubre ARQ-12 parcial (estructural; el benchmark queda con ARQ-40), ARQ-16, ARQ-17, ARQ-18, ARQ-19. Activo: el resto del trabajo de 02. Siguiente, de abajo arriba por capas (§2.1): capa 1 `gafira-anim` hecha; empezar por la capa 2: `gafira-text`, `gafira-layout`, `gafira-raster`, `gafira-audio`, `gafira-segment`; capa 3 `gafira-ports` y el resto; capa 4 adaptadores (`gafira-format` necesita `gafira-ports`). |
| `00-decisiones.md` | Vigente (no ejecutable) | Registro de decisiones del titular; prevalece sobre 01-05. No se promueve ni se archiva. |
| `backlog/01-producto-y-calidad.md` | Backlog | Producto, calidad y criterios de aceptación. |
| `backlog/03-diseno-visual-e-interaccion.md` | Backlog | Tokens, modo oscuro, glosario ES+EN, wireframes. |
| `backlog/04-funcionalidad-diseno-imagen-exportacion.md` | Backlog | Diseño, imagen y exportación. |
| `backlog/05-funcionalidad-video-y-distribucion.md` | Backlog | Video y distribución. |
| `backlog/06-licencia-y-activacion.md` | Backlog | Licencia y activación. |
| `archive/` | Archivados | Ninguno todavía. |

## Cómo continuar el documento en curso

Desde 2026-10-29 los documentos se ejecutan por la vía rápida (ADR 0002): sin propuesta ni delta specs, con una Slice card en `openspec/changes/<change>/apply-progress.md` que cita los IDs del documento. Los changes ya archivados (delta specs en `openspec/specs/`) no se tocan.

1. Arrancar `claude --agent coordinator` en `main` y pedir: "ejecuta docs/specs/02-arquitectura-software.md".
2. El coordinador calcula los IDs pendientes (los que ningún change ni test nombra todavía):
   `comm -23 <(grep -oE 'ARQ-[0-9]+' docs/specs/02-arquitectura-software.md | sort -u) <(git grep -hoE 'ARQ-[0-9]+' -- openspec crates apps | sort -u)`
3. Elige el siguiente bloque de abajo arriba por capas (§2.1 de 02): la capa más baja con crates pendientes cuyas dependencias ya existan (próximo: capa 2), escribe su Slice card y sigue con `tdd-apply` -> `verify`; la mutación corre una vez (T4) al cerrar.
4. Cada change cerrado actualiza "Detalle" arriba. Cuando no quedan IDs pendientes, el documento se archiva (sección de abajo).

## Pending follow-ups

Recorded 2026-10-26; items 6-9 recorded 2026-10-30. Open items that belong to no active change. Remove an item when done.

1. **Flaky test** `crates/gafira-testkit/tests/harness.rs::each_broken_law_is_named_with_a_counterexample`. Why: it fails about half the runs with `PROPTEST_CASES=1` (passes unset); accepted as out of scope in the archived change `testkit-oracle` (decision C6). Start: a small test-only change (for example make the broken-law case deterministic or pin its seed/case count), then run it repeatedly with `PROPTEST_CASES=1`.
2. **Re-enable CI.** Why: workflow `CI` was disabled manually because jobs failed to start (billing). Start: `gh workflow enable 395407504`, then observe the first runs. Unobserved since D5 (owner accepted the risk, decision W2=B): the Windows and macOS matrix and the trybuild `.stderr` fixtures (ARQ-30, R3). Fix whatever fails.
3. **Spec gaps recorded at archive** (`openspec/changes/archive/2026-10-26-engine-pathops/`, crate `gafira-pathops`). Start: decide each with the owner, then specify via `specifier` and test first.
   - A path with no initial `MoveTo`: the spec is silent (it is not a valid `BezPath`).
   - Infinite distance on an empty path and the wording of the "path has" error (accepted suggestions S3/S4 of `engine-pathops`).
   - `simplify` collapses `Z Z` (coordinator choice, not in the spec).
4. **Owner-only items**, all tracked in `00-decisiones.md`, section "Pendientes del titular" (not duplicated here): locked-feature list and final price (D-12, item 1); EULA (`LICENCIA.md`) and `PRIVACY.md` (item 2, phase 06); `license-hub` security list and deploy (item 3, R-12 in 02); trademark: lawyer availability search, first registration country, optional domains (item 4, D-04); splash reverse-image search, original C2PA download kept outside the repo, `assets/LICENSES.toml` entry (item 6, D-09); convert the audio, register it in `assets/LICENSES.toml` and `THIRD_PARTY_NOTICES.md`, show the AI-content notice (item 8, D-10).
5. **`02-arquitectura-software.md`, `gafira-time` row** (layer 0 table). Why: it lists the exact-divisor rates (24, 25, 30, 48, 50, 60, 120 fps) but omits the NTSC rates (23.976, 29.97) that the project frame rates include; incomplete, not false. Start: add them to that row (docs-only change).
6. **Coste del solver de Bézier frente al 1 µs de ARQ-12.** Why: el solver de bisección simple cuesta unos 650 ns por muestra `CubicBezier`, y una muestra sobre una pista de 10 000 claves unos 720 ns en un i7-12700H, frente al límite fijado de 1 µs de ARQ-12 (D-03) en el hardware de referencia, más lento (Core i5 de 10.ª generación). Start: medirlo con el benchmark de ARQ-40; si no se cumple, acelerar el solver manteniendo acotado el error en `s`.
7. **CI: grep contra `mul_add` en `gafira-anim`.** Why: el determinismo entre plataformas depende de no usar `mul_add`; hoy solo se comprueba a mano. Start: añadir al CI un paso que falle si aparece `mul_add` en `crates/gafira-anim`.
8. **Test puntual de ARQ-17 con el valor exacto 0.125.** Why: el test de referencia de ARQ-17 usa la misma bisección f64 que el solver, así que comparte su error (unos 4.7e-6 en la raíz triple) y no lo detecta. Start: añadir `CubicBezier { x1: 1, y1: -1, x2: 0, y2: 1 }.apply(0.5)` comparado con 0.125 y tolerancia 1e-5.
9. **Golden cross-plataforma de `gafira-anim` (ARQ-12) solo en Linux.** Why: `tests/golden/sample.json` solo se ejecuta en local, en Linux, mientras CI está desactivado; Windows y macOS no lo comprueban. Start: cubierto por el follow-up 2 (re-activar CI); no duplicar.

## Promover un documento del backlog

1. `git mv docs/specs/backlog/<NN-doc>.md docs/specs/`
2. Actualizar la tabla de arriba y las referencias de ruta (`git grep "backlog/<NN>"`).
3. Hacerlo en su propio commit.

## Archivar un documento completado

Criterio: cada requisito del documento está cubierto por changes de OpenSpec archivados y la tabla de estado lo dice. `00-decisiones.md` no se archiva nunca.

1. `git mv docs/specs/<NN-doc>.md docs/specs/archive/` (mismo nombre de archivo).
2. Actualizar la tabla de arriba y las referencias de ruta (`git grep "<NN-doc>"`).
3. Promover el siguiente documento desde `backlog/` (sección anterior).
4. Hacerlo en su propio commit.

Si un change de 02 necesita un documento del backlog (p. ej. la UI necesita 03), se promueve en ese momento.

Las referencias cruzadas tipo "01 §x" siguen valiendo: los nombres de archivo no cambian.
