# 092 — Barra superior y Ajustes

> **Estado:** in-progress — 2026-11-23 — implementada y probada en Linux (D-123); pendiente de la prueba del titular.

## Objetivo

La barra superior de las pantallas de inicio muestra el logo de Gafira y lugares con icono; Ajustes se rehace con secciones con icono y cada opción como tarjeta, con una miniatura de cada tema, y dice lo que Gafira hace hoy.

## Alcance / no alcance

Alcance:

- Barra superior de Inicio, Plantillas y «Crear un diseño»: logo horizontal para fondo oscuro (manual de marca) en lugar del texto, Inicio y Plantillas con su icono, Ajustes a la derecha con un engranaje. Se quita Proyectos. El lugar abierto se marca con un fondo suave y una línea coral.
- Barra de Ajustes: logo, «Volver» con flecha y Ajustes marcado.
- Ajustes: secciones General, Apariencia, Privacidad y Acerca de con icono; título y una línea de ayuda por sección.
- General: el idioma como tres tarjetas (Sistema, Español, English), cada idioma descrito en su propia lengua.
- Apariencia: Claro, Oscuro y Sistema como tarjetas con una miniatura de la interfaz en sus colores (Sistema, mitad clara y mitad oscura).
- Privacidad: lo que se queda en el equipo y cuándo Gafira usa internet (fotos, ilustraciones y modelos, D-111 y D-115); se corrige el texto anterior, que decía que no se conecta.
- Acerca de: logo, eslogan y versión; tipografías de la interfaz con su licencia y el número de familias libres para textos; contenido incluido y sus licencias.
- Iconos Phosphor nuevos (MIT): `gear-six`, `globe`, `shield-check`, `info`, `sun`, `moon`, `monitor`.

No alcance:

- Opciones nuevas de configuración; la barra del editor.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S92-01 | Barra con logo e iconos. | Logo SVG de fondo oscuro a 26 px de alto; Inicio, Plantillas y Ajustes con icono; sin Proyectos; un clic en el logo lleva a Inicio. |
| S92-02 | Ajustes por secciones. | Cada sección con icono, título y ayuda; el contenido en tarjetas. |
| S92-03 | Idioma. | Tres tarjetas; la elegida con borde de acento y marca; el cambio se aplica al momento (DIS-75). |
| S92-04 | Tema. | Tres tarjetas con miniatura de la interfaz en los colores de cada modo, tomados de los tokens. |
| S92-05 | Textos ciertos. | Privacidad dice cuándo se usa internet; Acerca de cita solo licencias comprobadas (OFL, MIT, CC0). |

## TDD

Pantallas sin lógica crítica nueva: se mantienen las pruebas S34-03 y S34-04 y la de claves de textos; se valida en Xephyr.

## Evidencia

- Xephyr con datos temporales: barra con logo, Inicio marcado y Ajustes con engranaje; Ajustes en General (tarjetas de idioma), Apariencia (miniaturas Claro, Oscuro y Sistema), Privacidad y Acerca de.
- `fmt`, `clippy`, la política y la suite completa.

## Riesgos y decisiones abiertas

- La miniatura del tema es un dibujo simplificado del editor, no una captura.
