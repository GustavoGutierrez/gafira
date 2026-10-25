# Especificaciones de producto

Aquí solo viven los documentos **en curso**. Todo lo demás espera en `backlog/`. Al completar el documento en curso, se promueve el siguiente desde el backlog.

## Estado

| Doc | Estado | Detalle |
|---|---|---|
| `02-arquitectura-software.md` | En curso | Hecho: C1 `workspace-scaffold`, C2a `domain-color`, C2b `domain-doc` hasta D5b (D6 en curso). Sigue: C3 `edit-history`, luego `gafira-format`, render, UI... |
| `00-decisiones.md` | Vigente (no ejecutable) | Registro de decisiones del titular; prevalece sobre 01-05. No se promueve ni se archiva. |
| `backlog/01-producto-y-calidad.md` | Backlog | Producto, calidad y criterios de aceptación. |
| `backlog/03-diseno-visual-e-interaccion.md` | Backlog | Tokens, modo oscuro, glosario ES+EN, wireframes. |
| `backlog/04-funcionalidad-diseno-imagen-exportacion.md` | Backlog | Diseño, imagen y exportación. |
| `backlog/05-funcionalidad-video-y-distribucion.md` | Backlog | Video y distribución. |
| `backlog/06-licencia-y-activacion.md` | Backlog | Licencia y activación. |

## Promover un documento del backlog

1. `git mv docs/specs/backlog/<NN-doc>.md docs/specs/`
2. Actualizar la tabla de arriba y las referencias de ruta (`git grep "backlog/<NN>"`).
3. Hacerlo en su propio commit.

Si un change de 02 necesita un documento del backlog (p. ej. la UI necesita 03), se promueve en ese momento.

Las referencias cruzadas tipo "01 §x" siguen valiendo: los nombres de archivo no cambian.
