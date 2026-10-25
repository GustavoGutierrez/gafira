# Product document skeleton (Spanish)

Copy the structure; fill from the real sources. Match the header fields of the nearest existing document (`docs/specs/02-arquitectura-software.md`, `docs/specs/backlog/03-diseno-visual-e-interaccion.md`).

```markdown
# NN — <Nombre del documento> de Gafira

| Campo | Valor |
|---|---|
| Documento | Especificación NN de 05 |
| Estado | Borrador para revisión |
| Alcance | <qué cubre, en una frase> |
| Depende de | `00-decisiones.md` (D-01 a D-nn, prevalecen), <otras specs y secciones> |
| Lo usan | <documentos que citan este> |
| Convenciones | `XXX-nn` = <significado> con criterio de aceptación. **FIJADA** = cifra fijada por el titular (00-decisiones). **PROPUESTA** = decisión no presente en las fuentes ni fijada todavía. **VERIFICAR** = hecho por confirmar antes de vender |
| Numeración | Los IDs no se renumeran. Huecos reservados: <lista> |

<Párrafo breve: qué es y qué principio lo guía.>

---

## 1. <Sección>

### 1.1 <Subsección>

| ID | Requisito | Criterio de aceptación |
|---|---|---|
| XXX-01 | <Un enunciado verificable.> | <Prueba, comando o medición que lo comprueba.> |

## N. Riesgos y preguntas abiertas
## N+1. Trazabilidad (hacia el PRD o hacia los RF)
```

Checklist before committing a product document edit:
- New IDs use the next free number; no ID renumbered or reused.
- Every ID has one statement and a way to check it.
- Markers (PROPUESTA, VERIFICAR, FIJADA) match their definition in the header; FIJADA only if 00 fixes it.
- No contradiction with 00; cross-references (`02 §x`) resolve.
- Reverse trace tables updated.
