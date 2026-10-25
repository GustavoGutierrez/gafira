# Especificaciones de producto

Aquí solo viven los documentos **en curso**. Todo lo demás espera en `backlog/`. Al completar el documento en curso, se promueve el siguiente desde el backlog.

Ciclo de vida: `backlog/` -> en curso (`docs/specs/`) -> `archive/` (completado).

El skill `product-specs` (`.agents/skills/product-specs/`) describe el ciclo de vida, el formato de los documentos y cómo derivar changes de OpenSpec.

## Estado

| Doc | Estado | Detalle |
|---|---|---|
| `02-arquitectura-software.md` | En curso | Hecho: C1 `workspace-scaffold` (ARQ-01 parcial, ARQ-02, ARQ-03, ARQ-07, ARQ-70, ARQ-71, ARQ-80 parcial: la puerta de merge está diferida por el 403 de GitHub), C2a `domain-color` (ARQ-02, ARQ-03, ARQ-10), C2b `domain-doc` (ARQ-03, ARQ-10, ARQ-11, ARQ-12, ARQ-13, ARQ-30) (archivados). Activo: C3 `edit-history` (propuesta cerrada en `openspec/changes/edit-history/`, diez entregas D1-D10). Luego `gafira-format`, render, UI... |
| `00-decisiones.md` | Vigente (no ejecutable) | Registro de decisiones del titular; prevalece sobre 01-05. No se promueve ni se archiva. |
| `backlog/01-producto-y-calidad.md` | Backlog | Producto, calidad y criterios de aceptación. |
| `backlog/03-diseno-visual-e-interaccion.md` | Backlog | Tokens, modo oscuro, glosario ES+EN, wireframes. |
| `backlog/04-funcionalidad-diseno-imagen-exportacion.md` | Backlog | Diseño, imagen y exportación. |
| `backlog/05-funcionalidad-video-y-distribucion.md` | Backlog | Video y distribución. |
| `backlog/06-licencia-y-activacion.md` | Backlog | Licencia y activación. |
| `archive/` | Archivados | Ninguno todavía. |

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
