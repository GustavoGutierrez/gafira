# 017 — Primer shell de escritorio (`gafira-ui` y `apps/gafira`)

> **Estado:** done — 2026-11-16 — primera entrega de la UI de 03 (en backlog); S17-01..S17-04 cubiertos por `gafira-ui` (12 tests); comprobado a mano: la app abre en Linux (X11), muestra la Pantalla de arranque, Inicio y el Editor de diseño en español e inglés (`--lang`). Sin engine todavía.

## Objetivo

Poder abrir Gafira en el escritorio y ver sus pantallas con la marca real: los tokens de `gafira-theme` (spec 014) y los textos de `gafira-i18n` (spec 015), siguiendo la estructura de 03 §6.1, §7.1, §7.2, §7.3 y §7.10.

## Alcance / no alcance

Alcance:

- `crates/gafira-ui` (capa 4, egui/eframe 0.33 con `glow`, `x11`, `wayland` y `accesskit`; sin las fuentes por defecto de egui):
  - Tema: colores por token, Inter para la interfaz y Manrope para los títulos, radios y tamaños de 03 §3.
  - Pantalla de arranque dibujada dentro de la ventana principal (respaldo de 03 §7.10.2): imagen con la regla «cubrir» de §7.10.3, panel con símbolo, nombre, eslogan, barra de progreso y versión; visible al menos 1,2 s (D-09).
  - Inicio (§7.2): barra grafito, eslogan, «¿Qué quieres crear hoy?», seis formatos seleccionables, «Empezar a diseñar» (coral, también `Ctrl/Cmd+N`), «Explorar plantillas», Recientes con su estado vacío.
  - Editor de diseño (§7.1, §7.3): barra superior, riel de pestañas con indicador coral, Panel lateral redimensionable (280–480 px), lienzo con la página del formato elegido ajustada a la vista (zoom con `Ctrl` + rueda), Inspector y Barra inferior con zoom y páginas. «Inicio» vuelve a Inicio.
  - Todo texto visible sale de los catálogos por clave (ARQ-97); claves nuevas `app`, `splash`, `home`, `nav`, `menu`, `editor`, `tab`, `inspector`, `bottom` en es y en.
- `apps/gafira`: raíz de composición; resuelve el idioma con `D08LocaleResolver` sobre `LC_ALL`/`LC_MESSAGES`/`LANG` y acepta `--lang es|en`.

No alcance (siguientes entregas de la UI):

- Conectar el engine (`Gafira` y sus servicios) con adaptadores reales de plataforma (`FileSystem`, `Clock`, `IdSource`, `ProjectRepository`): nuevo, abrir, guardar, deshacer.
- Ventana propia sin bordes para la Pantalla de arranque; progreso real de carga.
- Menús con contenido, Paleta de comandos, Exportar, Ajustes, Editor de video, Recuperación tras fallo, modo oscuro conmutable, iconografía (§13), logo horizontal (§5.2: falta el archivo).
- Pruebas de captura (DIS-15) y de accesibilidad.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S17-01 | La Pantalla de arranque sigue D-09 y §7.10.3. | La barra se llena en 1,2 s; la imagen cubre la ventana sin deformarse, centrada en horizontal y sobre la zona encima del panel, sin dejar hueco abajo. |
| S17-02 | El tema sale de los tokens. | Todo token de color que nombra la UI existe en los dos modos; los pasos tipográficos existen; los títulos usan Manrope y la interfaz Inter; un token ausente se ve en magenta. |
| S17-03 | Los textos salen de los catálogos (ARQ-97). | Toda clave que muestra la UI existe en es y en; la misma clave da el texto de cada idioma. |
| S17-04 | La página del formato elegido entra en el lienzo. | Cada formato tiene su tamaño (A4 a 96 dpi, 9:16, 16:9...); el zoom de ajuste deja el margen, no amplía más del 100 % y no baja del 5 %. |

## Notas

- Las fuentes son los subconjuntos de prueba de la spec 014; con ellas los títulos se ven con el peso por defecto de la fuente variable. Los archivos de producto son una decisión pendiente.
- `Rgb8` a `Color32` y los tamaños de token a `f32` se convierten sin `as` (las reglas de clippy del workspace).
