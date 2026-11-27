# 111 — Pestañas de documentos

> **Estado:** in-progress — 2026-11-27 — pedida por el titular, con photocraft como referencia; en implementación.

## Objetivo

Trabajar con varios proyectos abiertos a la vez en el editor, cada uno en su pestaña con el nombre del archivo, y pasar de uno a otro sin cerrar ninguno.

## Alcance / no alcance

Alcance:

- Motor (`gafira-engine`): varias sesiones abiertas. La sesión activa sigue en el hueco de siempre, así que todo lo que edita, exporta o guarda trabaja sobre ella sin cambios; las demás quedan aparcadas con su historial, su archivo, su bloqueo, su autoguardado y sus versiones.
  - Aparcar la activa, activar otra por su clave, listar las abiertas con su nombre, su estado de cambios y su ruta.
  - Cada sesión sabe si tiene cambios sin guardar por sí misma: editar una no marca las otras.
  - Un guardado o un autoguardado que termina con su sesión aparcada se aplica a esa sesión.
  - La recuperación no ofrece los proyectos que están abiertos en otra pestaña.
  - Un archivo ya abierto no se abre dos veces: se activa su pestaña.
- Interfaz:
  - Tira de pestañas bajo la barra superior: nombre del archivo («Sin título» si no tiene), marca de cambios sin guardar, botón de cerrar, botón «+» que lleva a Inicio para crear o abrir.
  - Abrir, abrir un reciente, crear, crear desde plantilla y «Copiar y redimensionar» abren una pestaña nueva y conservan las demás.
  - Cerrar una pestaña con cambios pregunta «¿Guardar los cambios?»; Inicio deja las pestañas abiertas; cerrar la ventana pregunta por cada una con cambios.
  - Cada pestaña conserva su zoom, su desplazamiento, su selección y su panel.
  - Ctrl+Tab y Ctrl+Mayús+Tab cambian de pestaña; Ctrl+W cierra la activa.
  - Las pestañas inactivas sueltan sus texturas de caché.

No alcance: arrastrar pestañas para reordenarlas o sacarlas a otra ventana, pestañas restauradas al volver a abrir Gafira, comparar dos pestañas lado a lado.

## Dependencias

- Sesión y servicios del motor (02 §5.4, P13, P15, decisión O2 del contador de versiones), guardado en hilo (spec 036), bloqueos (spec 028), recuperación (spec 016), versiones (spec 031), shell y editor (`gafira-ui`).

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S111-01 | Varias sesiones | Aparcar la activa y abrir o crear otra deja dos sesiones; activar una por su clave la pone en el hueco activo con su historial intacto |
| S111-02 | Cambios por sesión | Editar la sesión activa no cambia el estado «modificado» de las aparcadas |
| S111-03 | Guardado aparcado | Un guardado o autoguardado que termina con su sesión aparcada se aplica a ella (limpia, ranura avanzada) |
| S111-04 | Recuperación | Los proyectos abiertos en cualquier sesión no se ofrecen como recuperables |
| S111-05 | Archivo ya abierto | Pedir abrir la ruta de una sesión abierta activa esa sesión sin abrirla otra vez |
| S111-06 | Cerrar | Cerrar una sesión suelta su bloqueo y sus ranuras y deja las demás como estaban |
| S111-07 | Tira de pestañas | Una pestaña por sesión con su nombre y su marca de cambios; clic activa, × cierra, «+» lleva a Inicio |
| S111-08 | Estado de vista | Volver a una pestaña la muestra con el zoom, el desplazamiento y la selección que tenía |
| S111-09 | Atajos | Ctrl+Tab, Ctrl+Mayús+Tab y Ctrl+W |

## Pruebas (TDD en el motor)

- `gafira-engine`: `s111_01` a `s111_06` en `project.rs`, primero en rojo.
- `gafira-ui`: integración de la tira, las pestañas nuevas al abrir y los atajos.

## Evidencia para cerrar

- Pruebas `s111_` en verde (`git grep s111_`), gates de `AGENTS.md` y prueba del titular en la app.

## Riesgos y decisiones abiertas

- Memoria: cada historial puede llegar a 512 MB. Las pestañas inactivas sueltan sus texturas; repartir el presupuesto del historial entre pestañas queda para una spec posterior si hace falta.
- El contador de versiones sigue siendo del motor (O2) para nombrar estados; el estado «modificado» pasa a medirse por sesión.
