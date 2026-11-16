# 034 — Pantalla de Ajustes: General, Apariencia, Privacidad y Acerca de

> **Estado:** done — 2026-11-16 — S34-01..S34-04 cubiertos por `gafira-ports` (`tests/settings.rs`), `gafira-engine` (`settings::tests`) y `gafira-ui` (`settings::tests` 2); 2921 tests.

## Objetivo

Abrir Ajustes desde Inicio y desde Archivo y cambiar el idioma y la apariencia al momento, como piden 03 §7.8 y §16.1 (DIS-73, DIS-75).

## Alcance / no alcance

Alcance:

- `gafira-ports`: `Appearance::System` (`"system"`), además de claro y oscuro.
- `gafira-engine`: `SettingsService::preview(language)`: a qué idioma resuelve una preferencia ahora, sin guardar ni publicar.
- `gafira-ui`: pantalla Ajustes con secciones a la izquierda:
  - **General**: Idioma con Sistema («Sistema · ahora: Español (es-MX)»), Español y English, en ese orden, con el texto de ayuda; el cambio se guarda y la interfaz cambia de idioma en el mismo cuadro.
  - **Apariencia**: Claro, Oscuro y Sistema (sigue el modo del sistema operativo si egui lo conoce; si no, claro).
  - **Privacidad**: «Tus diseños y archivos no salen de tu equipo. Gafira no se conecta a internet.» (03 §7.8 menciona la conexión de la licencia; con D-62 la primera versión no la tiene).
  - **Acerca de**: versión y tipografías incluidas con su licencia OFL.
  - «Volver» y `Esc` cierran Ajustes y vuelven a la pantalla en la que se estaba.

No alcance: Rendimiento, Guardado, Atajos, Accesibilidad, Licencia y Funciones locales (03 §7.8); carpeta de proyectos; la acción «Cambiar idioma» de la Paleta de comandos; el anuncio para lectores de pantalla.

## Requisitos

| ID | Requisito | Criterio de aceptación |
| --- | --- | --- |
| S34-01 | Apariencia «Sistema». | `"system"` va y vuelve en el archivo de ajustes. |
| S34-02 | Vista previa del idioma. | `preview` resuelve sin cambiar los ajustes, sin guardar y sin publicar. |
| S34-03 | Etiqueta de Sistema. | «Sistema · ahora: Español (es-MX)»; en la interfaz inglesa «System · now: Español (es-MX)»; sin idioma del sistema, English (en). |
| S34-04 | Modo de Sistema. | Sigue el sistema; desconocido se lee claro; Claro y Oscuro no dependen del sistema. |
